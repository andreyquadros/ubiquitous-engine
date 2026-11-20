//! Two products on one machine: only one process runs the guard and the AI check (design
//! section 1.6).
//!
//! The primary rewrites `<data_local_dir>/ubiqx-shield/primary.lock` every
//! [`HEARTBEAT_EVERY_SECS`]. A product writes the lock only while its own module is enabled
//! and its guard loop is running, and deletes it on disable and on quit. A heartbeat from
//! the other product that is at most [`STALE_AFTER_SECS`] old makes this process
//! `Secondary`; a heartbeat more than [`STALE_AFTER_SECS`] in the future is stale too. The
//! standalone app wins a tie.
//!
//! # Protocol
//!
//! Every [`HEARTBEAT_EVERY_SECS`] a product whose module is on reads the lock and asks
//! [`decide`]; as `Primary` it then writes its own heartbeat. With one file there is one
//! last writer, so two products that start in the same moment settle within one interval.
//! A process that already runs the passes asks [`decide_holding`] instead, which is where
//! the standalone app wins the tie.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ubiqx_core::ProductId;

/// Directory (under the user's local data directory) both products share, and the lock
/// file inside it.
pub const SHARED_DIR: &str = "ubiqx-shield";
pub const LOCK_FILE: &str = "primary.lock";
pub const HEARTBEAT_EVERY_SECS: i64 = 30;
pub const STALE_AFTER_SECS: i64 = 90;

/// Content of the lock file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimaryLock {
    pub product: ProductId,
    pub pid: u32,
    pub heartbeat_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// This process runs the passes (and writes the heartbeat).
    Primary,
    /// The other product runs them; this process shows `overall: secondary`.
    Secondary,
}

/// Whether `lock` is a live heartbeat of the product that is not `me`: at most
/// [`STALE_AFTER_SECS`] old, and not more than that in the future (a clock change, a file
/// edited by hand), so a bad timestamp cannot switch both products off.
fn other_is_alive(lock: Option<&PrimaryLock>, me: ProductId, now: DateTime<Utc>) -> bool {
    lock.is_some_and(|lock| {
        let age = (now - lock.heartbeat_at).num_seconds();
        lock.product != me && (-STALE_AFTER_SECS..=STALE_AFTER_SECS).contains(&age)
    })
}

/// The role of product `me` given the lock it read (`None`: no file, or unreadable). A
/// live heartbeat of the other product makes this process `Secondary`; no lock, a stale
/// one or its own makes it `Primary`.
pub fn decide(lock: Option<&PrimaryLock>, me: ProductId, now: DateTime<Utc>) -> Role {
    if other_is_alive(lock, me, now) {
        Role::Secondary
    } else {
        Role::Primary
    }
}

/// The role of a process that was `Primary` at its last heartbeat. Finding the other
/// product's live heartbeat then means both took the role at once: the standalone app
/// keeps it (and overwrites the lock), ubiqX stands down.
pub fn decide_holding(lock: Option<&PrimaryLock>, me: ProductId, now: DateTime<Utc>) -> Role {
    match me {
        ProductId::Shield => Role::Primary,
        ProductId::Ubiqx => decide(lock, me, now),
    }
}

/// Parses the lock file; anything that is not a valid lock is `None`.
pub fn parse_lock(text: &str) -> Option<PrimaryLock> {
    serde_json::from_str(text.trim()).ok()
}

/// The lock file's content for `lock` (one line of JSON with a final newline).
pub fn render_lock(lock: &PrimaryLock) -> String {
    // A struct of an enum, a number and a timestamp always serialises.
    let json = serde_json::to_string(lock).unwrap_or_default();
    format!("{json}\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap()
    }

    fn lock(product: ProductId, age_secs: i64) -> PrimaryLock {
        PrimaryLock {
            product,
            pid: 4242,
            heartbeat_at: now() - Duration::seconds(age_secs),
        }
    }

    #[test]
    fn no_lock_or_my_own_lock_makes_me_primary() {
        for me in ProductId::ALL {
            assert_eq!(decide(None, me, now()), Role::Primary);
            assert_eq!(decide(Some(&lock(me, 0)), me, now()), Role::Primary);
            assert_eq!(decide(Some(&lock(me, 10_000)), me, now()), Role::Primary);
        }
    }

    #[test]
    fn a_live_heartbeat_of_the_other_product_makes_me_secondary() {
        for me in ProductId::ALL {
            for age in [0, 1, 30, 89, 90] {
                assert_eq!(
                    decide(Some(&lock(me.other(), age)), me, now()),
                    Role::Secondary,
                    "{me:?} age {age}"
                );
            }
        }
    }

    #[test]
    fn a_stale_heartbeat_hands_the_role_over() {
        for me in ProductId::ALL {
            for age in [91, 120, 3_600, 86_400 * 30] {
                assert_eq!(
                    decide(Some(&lock(me.other(), age)), me, now()),
                    Role::Primary,
                    "{me:?} age {age}"
                );
            }
        }
    }

    #[test]
    fn a_heartbeat_from_the_future_is_stale_too() {
        for me in ProductId::ALL {
            // A little clock skew between two reads is still alive.
            for ahead in [1, 60, 90] {
                assert_eq!(
                    decide(Some(&lock(me.other(), -ahead)), me, now()),
                    Role::Secondary,
                    "{me:?} ahead {ahead}"
                );
            }
            for ahead in [91, 3_600, 86_400 * 365] {
                assert_eq!(
                    decide(Some(&lock(me.other(), -ahead)), me, now()),
                    Role::Primary,
                    "{me:?} ahead {ahead}"
                );
            }
        }
    }

    #[test]
    fn the_standalone_app_wins_a_tie() {
        // Both wrote a heartbeat in the same interval; each now reads the other's.
        let from_ubiqx = lock(ProductId::Ubiqx, 5);
        let from_shield = lock(ProductId::Shield, 5);
        assert_eq!(
            decide_holding(Some(&from_ubiqx), ProductId::Shield, now()),
            Role::Primary
        );
        assert_eq!(
            decide_holding(Some(&from_shield), ProductId::Ubiqx, now()),
            Role::Secondary
        );
        // Without a tie, holding the role changes nothing.
        assert_eq!(decide_holding(None, ProductId::Ubiqx, now()), Role::Primary);
        assert_eq!(
            decide_holding(Some(&lock(ProductId::Shield, 500)), ProductId::Ubiqx, now()),
            Role::Primary
        );
        assert_eq!(
            decide_holding(Some(&lock(ProductId::Ubiqx, 3)), ProductId::Ubiqx, now()),
            Role::Primary
        );
    }

    #[test]
    fn the_lock_round_trips_as_one_line() {
        for product in ProductId::ALL {
            let original = lock(product, 7);
            let text = render_lock(&original);
            assert!(text.ends_with('\n'));
            assert_eq!(text.lines().count(), 1);
            assert_eq!(parse_lock(&text), Some(original));
        }
        assert_eq!(
            render_lock(&lock(ProductId::Shield, 0)),
            "{\"product\":\"shield\",\"pid\":4242,\"heartbeat_at\":\"2026-09-17T15:00:00Z\"}\n"
        );
    }

    #[test]
    fn anything_that_is_not_a_lock_is_none() {
        for text in [
            "",
            "   ",
            "not json",
            "{}",
            "[]",
            "{\"product\":\"ubiqx\"}",
            "{\"product\":\"other\",\"pid\":1,\"heartbeat_at\":\"2026-09-17T15:00:00Z\"}",
            "{\"product\":\"ubiqx\",\"pid\":-1,\"heartbeat_at\":\"2026-09-17T15:00:00Z\"}",
            "{\"product\":\"ubiqx\",\"pid\":1,\"heartbeat_at\":\"yesterday\"}",
            "{\"product\":\"ubiqx\",\"pid\":1,\"heartbeat_at\":\"2026-09-17T15:00:00Z\"} trailing",
        ] {
            assert_eq!(parse_lock(text), None, "{text:?}");
        }
        // A lock that cannot be read never makes this process stand down.
        assert_eq!(
            decide(parse_lock("garbage").as_ref(), ProductId::Ubiqx, now()),
            Role::Primary
        );
    }
}
