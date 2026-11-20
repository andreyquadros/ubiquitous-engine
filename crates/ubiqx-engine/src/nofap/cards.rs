//! The cards of the floating window, and what their buttons need.
//!
//! A [`NofapCard`] has no field for a host, a title or an application. What a button needs
//! to act (the application to close, the host to unlearn, the request to confirm) stays
//! here, in memory, keyed by the card id, for ten minutes. The last twenty cards are kept
//! for the window to ask for; an id nobody knows gets the neutral discreet card.

use std::collections::VecDeque;

use chrono::{DateTime, Duration, Utc};
use ubiqx_core::nofap::{CardAction, CardKind, NofapCard, NofapEvent};
use ubiqx_core::Id;
use ubiqx_nofap::episode::CARD_MIN_GAP_SECS;
use ubiqx_nofap::matcher::BlockSource;
use ubiqx_nofap::messages::{self, Notice};

use super::NofapService;

/// Cards kept for [`NofapService::card`].
pub const CARDS_KEPT: usize = 20;
/// How long the engine remembers what a card's buttons need.
pub const CARD_TTL_SECS: i64 = 600;
/// Lifetime of the discreet blocked card and of the soft card on screen.
pub const DISCREET_CLOSE_SECS: u32 = 12;
pub const ASK_CLOSE_SECS: u32 = 60;
/// How long the window is taken to be busy with a flow it hosts itself: the three-minute
/// surf (seven steps of thirty seconds) and the ten minutes away from the screen.
pub const SURF_FLOW_SECS: i64 = 240;
pub const AWAY_FLOW_SECS: i64 = 600;

/// What a method queued while it held the runtime lock, to be sent once it is released.
#[derive(Default)]
pub(crate) struct Outbox {
    pub events: Vec<NofapEvent>,
    /// Cards to show, each with the notification that stands in for it when the window
    /// cannot be shown (`None`: nothing is sent then).
    pub cards: Vec<(NofapCard, Option<Notice>)>,
    /// The status has to be computed again.
    pub status: bool,
}

/// What the buttons of one card need. Never serialised, never logged.
#[derive(Debug, Clone, Default)]
pub(crate) struct CardContext {
    /// The sampler key of the window the card is about.
    pub key: Option<u64>,
    pub app_id: Option<String>,
    pub host: Option<String>,
    pub window_id: Option<u32>,
    /// A browser tab (close the tab) as opposed to another application (quit it).
    pub browser: bool,
    /// What blocked the page, for a block card.
    pub source: Option<BlockSource>,
    /// The site a vision verdict learned, so "Não era isso" can unlearn it.
    pub learned_id: Option<Id>,
    /// The waiting request of a "pending ready" card.
    pub pending_id: Option<Id>,
}

struct Kept {
    card: NofapCard,
    context: CardContext,
}

/// The cards in memory and the rule of when the next one may be shown.
#[derive(Default)]
pub(crate) struct CardStore {
    kept: VecDeque<Kept>,
    shown_at: Option<DateTime<Utc>>,
    /// The window hosts a surf or a time away until then; a new card would replace it.
    flow_until: Option<DateTime<Utc>>,
}

impl CardStore {
    /// At most one card in [`CARD_MIN_GAP_SECS`], and none while a flow runs in the
    /// window. A time that seems to lie in the future (the clock was set back) holds
    /// nothing.
    pub fn may_show(&self, now: DateTime<Utc>) -> bool {
        let recent = self
            .shown_at
            .is_some_and(|at| (0..CARD_MIN_GAP_SECS).contains(&(now - at).num_seconds()));
        let in_flow = self
            .flow_until
            .is_some_and(|until| (1..=AWAY_FLOW_SECS).contains(&(until - now).num_seconds()));
        !recent && !in_flow
    }

    /// Remembers a card that is about to be shown.
    pub fn keep(&mut self, card: NofapCard, context: CardContext, now: DateTime<Utc>) {
        self.shown_at = Some(now);
        self.kept.push_back(Kept { card, context });
        while self.kept.len() > CARDS_KEPT {
            self.kept.pop_front();
        }
    }

    pub fn get(&self, id: &str) -> Option<NofapCard> {
        self.kept
            .iter()
            .find(|kept| kept.card.id == id)
            .map(|kept| kept.card.clone())
    }

    /// The kind of a card and what its buttons need, while the card has not expired.
    pub fn context(&self, id: &str, now: DateTime<Utc>) -> Option<(CardKind, CardContext)> {
        self.kept
            .iter()
            .find(|kept| kept.card.id == id && now < kept.card.expires_at)
            .map(|kept| (kept.card.kind, kept.context.clone()))
    }

    pub fn forget(&mut self, id: &str) {
        self.kept.retain(|kept| kept.card.id != id);
    }

    pub fn start_flow(&mut self, secs: i64, now: DateTime<Utc>) {
        self.flow_until = Some(now + Duration::seconds(secs));
    }

    pub fn end_flow(&mut self) {
        self.flow_until = None;
    }
}

/// A card of `kind` with its message and buttons, valid for ten minutes.
pub(crate) fn card(
    kind: CardKind,
    message_key: &str,
    discreet: bool,
    actions: Vec<CardAction>,
    auto_close_secs: Option<u32>,
    now: DateTime<Utc>,
) -> NofapCard {
    NofapCard {
        id: ubiqx_core::new_id(),
        kind,
        message_key: message_key.to_owned(),
        note_key: None,
        discreet,
        actions,
        auto_close_secs,
        expires_at: now + Duration::seconds(CARD_TTL_SECS),
    }
}

/// What the window gets for an id nobody knows: the neutral discreet card, never a sample
/// and never an error.
pub(crate) fn neutral(id: &str, now: DateTime<Utc>) -> NofapCard {
    NofapCard {
        id: id.to_owned(),
        kind: CardKind::Blocked,
        message_key: messages::rotation_key(true, false, 0).to_owned(),
        note_key: None,
        discreet: true,
        actions: vec![CardAction::Support, CardAction::Ok],
        auto_close_secs: Some(DISCREET_CLOSE_SECS),
        expires_at: now + Duration::seconds(CARD_TTL_SECS),
    }
}

/// The card of a blocked page: discreet by default; the support card with a rotating
/// message when the user asked for detailed messages; `block.repeat` at the third hit of
/// an episode. `extra` is the correction button the hit carries, if any.
pub(crate) fn blocked(
    repeat: bool,
    discreet: bool,
    user_list: bool,
    rotation: usize,
    extra: Option<CardAction>,
    now: DateTime<Utc>,
) -> NofapCard {
    let mut made = if repeat {
        card(
            CardKind::BlockedRepeat,
            "block.repeat",
            false,
            vec![CardAction::Surf, CardAction::Ok],
            None,
            now,
        )
    } else {
        card(
            CardKind::Blocked,
            messages::rotation_key(discreet, user_list, rotation),
            discreet,
            vec![CardAction::Support, CardAction::Ok],
            discreet.then_some(DISCREET_CLOSE_SECS),
            now,
        )
    };
    made.actions.extend(extra);
    made
}

impl NofapService {
    /// Shows a card: the window when the shell has one, otherwise the neutral
    /// notification that stands in for it. The `card` event goes out either way, for a
    /// window that is already open.
    pub(crate) fn present(&self, card: &NofapCard, notice: Option<Notice>) {
        let shown = self.deps.presenter.show(card).unwrap_or(false);
        self.deps.sink.emit(NofapEvent::Card { card: card.clone() });
        if shown {
            return;
        }
        if let Some(notice) = notice {
            let language = self.deps.env.snapshot().language;
            let (title, body) = messages::notification_text(notice, language);
            // A notification that cannot be sent leaves nothing to fall back to.
            let _ = self.deps.notifier.notify(&title, &body);
        }
    }
}
