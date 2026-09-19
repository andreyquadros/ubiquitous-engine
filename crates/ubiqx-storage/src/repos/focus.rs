//! [`FocusRepo`] implementation: blocked targets, interventions and focus sessions.

use chrono::{DateTime, Utc};
use rusqlite::{named_params, params, Connection, Row};
use ubiqx_core::ports::FocusRepo;
use ubiqx_core::{
    CoreResult, FocusSession, FocusTarget, FocusTargetKind, Intervention, InterventionAction,
};

use crate::convert::{count, dt, ms, opt_dt, opt_ms, parse_enum, sql_limit};
use crate::error::{StorageError, StorageResult};
use crate::store::{execute, execute_expecting_row, query_list, query_one, query_opt, SqliteStore};

// ---------------------------------------------------------------------------------------------
// Targets
// ---------------------------------------------------------------------------------------------

macro_rules! target_cols {
    () => {
        "id, kind, name, key, enabled, created_at, last_blocked_at, blocked_count"
    };
}

const SELECT_TARGETS: &str = concat!(
    "SELECT ",
    target_cols!(),
    " FROM focus_targets ORDER BY enabled DESC, lower(name) ASC, id ASC"
);

const SELECT_TARGET: &str = concat!(
    "SELECT ",
    target_cols!(),
    " FROM focus_targets WHERE id = ?1"
);

const SELECT_TARGET_BY_KEY: &str = concat!(
    "SELECT ",
    target_cols!(),
    " FROM focus_targets WHERE kind = ?1 AND key = ?2"
);

/// A second add of the same app/site re-enables the row that is already there; its id, name
/// and counters are kept so the history it accumulated stays attached.
const UPSERT_TARGET: &str = concat!(
    "INSERT INTO focus_targets (",
    target_cols!(),
    ") VALUES (:id, :kind, :name, :key, :enabled, :created_at, :last_blocked_at, \
     :blocked_count) ON CONFLICT(kind, key) DO UPDATE SET enabled = 1"
);

const SET_TARGET_ENABLED: &str = "UPDATE focus_targets SET enabled = ?2 WHERE id = ?1";

const DELETE_TARGET: &str = "DELETE FROM focus_targets WHERE id = ?1";

const TOUCH_BLOCKED: &str = "UPDATE focus_targets SET blocked_count = blocked_count + 1, \
     last_blocked_at = ?2 WHERE id = ?1";

fn row_to_target(row: &Row<'_>) -> StorageResult<FocusTarget> {
    let kind: String = row.get("kind")?;
    Ok(FocusTarget {
        id: row.get("id")?,
        kind: parse_enum("focus target kind", &kind, FocusTargetKind::parse)?,
        name: row.get("name")?,
        key: row.get("key")?,
        enabled: row.get("enabled")?,
        created_at: dt(row.get("created_at")?)?,
        last_blocked_at: opt_dt(row.get("last_blocked_at")?)?,
        blocked_count: row.get("blocked_count")?,
    })
}

fn get_target(conn: &Connection, id: &str) -> StorageResult<FocusTarget> {
    query_opt(conn, SELECT_TARGET, params![id], row_to_target)?
        .ok_or_else(|| StorageError::NotFound(format!("focus target {id}")))
}

// ---------------------------------------------------------------------------------------------
// Interventions
// ---------------------------------------------------------------------------------------------

macro_rules! intervention_cols {
    () => {
        "id, at, target_id, kind, name, key, action, session_id, message"
    };
}

const INSERT_INTERVENTION: &str = concat!(
    "INSERT INTO interventions (",
    intervention_cols!(),
    ") VALUES (:id, :at, :target_id, :kind, :name, :key, :action, :session_id, :message)"
);

const SELECT_INTERVENTION: &str = concat!(
    "SELECT ",
    intervention_cols!(),
    " FROM interventions WHERE id = ?1"
);

const SELECT_INTERVENTIONS: &str = concat!(
    "SELECT ",
    intervention_cols!(),
    " FROM interventions ORDER BY at DESC, id DESC LIMIT ?1"
);

const COUNT_INTERVENTIONS_SINCE: &str = "SELECT COUNT(*) FROM interventions WHERE at >= ?1";

fn row_to_intervention(row: &Row<'_>) -> StorageResult<Intervention> {
    let kind: String = row.get("kind")?;
    let action: String = row.get("action")?;
    Ok(Intervention {
        id: row.get("id")?,
        at: dt(row.get("at")?)?,
        target_id: row.get("target_id")?,
        kind: parse_enum("focus target kind", &kind, FocusTargetKind::parse)?,
        name: row.get("name")?,
        key: row.get("key")?,
        action: parse_enum("intervention action", &action, InterventionAction::parse)?,
        session_id: row.get("session_id")?,
        message: row.get("message")?,
    })
}

// ---------------------------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------------------------

macro_rules! session_cols {
    () => {
        "id, task, started_at, ends_at, ended_at, interventions, hid_windows, ran_shortcut"
    };
}

const INSERT_SESSION: &str = concat!(
    "INSERT INTO focus_sessions (",
    session_cols!(),
    ") VALUES (:id, :task, :started_at, :ends_at, :ended_at, :interventions, :hid_windows, \
     :ran_shortcut)"
);

const UPDATE_SESSION: &str = "UPDATE focus_sessions SET task = :task, started_at = :started_at, \
     ends_at = :ends_at, ended_at = :ended_at, interventions = :interventions, \
     hid_windows = :hid_windows, ran_shortcut = :ran_shortcut WHERE id = :id";

const SELECT_ACTIVE_SESSION: &str = concat!(
    "SELECT ",
    session_cols!(),
    " FROM focus_sessions WHERE ended_at IS NULL ORDER BY started_at DESC, id DESC LIMIT 1"
);

const SELECT_SESSIONS: &str = concat!(
    "SELECT ",
    session_cols!(),
    " FROM focus_sessions ORDER BY started_at DESC, id DESC LIMIT ?1"
);

fn row_to_session(row: &Row<'_>) -> StorageResult<FocusSession> {
    Ok(FocusSession {
        id: row.get("id")?,
        task: row.get("task")?,
        started_at: dt(row.get("started_at")?)?,
        ends_at: dt(row.get("ends_at")?)?,
        ended_at: opt_dt(row.get("ended_at")?)?,
        interventions: row.get("interventions")?,
        hid_windows: row.get("hid_windows")?,
        ran_shortcut: row.get("ran_shortcut")?,
    })
}

impl FocusRepo for SqliteStore {
    fn list_targets(&self) -> CoreResult<Vec<FocusTarget>> {
        self.with(|conn| query_list(conn, SELECT_TARGETS, [], row_to_target))
    }

    fn get_target(&self, id: &str) -> CoreResult<Option<FocusTarget>> {
        self.with(|conn| query_opt(conn, SELECT_TARGET, params![id], row_to_target))
    }

    fn upsert_target(&self, target: &FocusTarget) -> CoreResult<FocusTarget> {
        self.with(|conn| {
            execute(
                conn,
                UPSERT_TARGET,
                named_params! {
                    ":id": target.id,
                    ":kind": target.kind.as_str(),
                    ":name": target.name,
                    ":key": target.key,
                    ":enabled": target.enabled,
                    ":created_at": ms(target.created_at),
                    ":last_blocked_at": opt_ms(target.last_blocked_at),
                    ":blocked_count": target.blocked_count,
                },
            )?;
            query_opt(
                conn,
                SELECT_TARGET_BY_KEY,
                params![target.kind.as_str(), target.key],
                row_to_target,
            )?
            .ok_or_else(|| {
                StorageError::Decode(format!(
                    "focus target {}/{} vanished after upsert",
                    target.kind.as_str(),
                    target.key
                ))
            })
        })
    }

    fn set_target_enabled(&self, id: &str, enabled: bool) -> CoreResult<FocusTarget> {
        self.with(|conn| {
            execute_expecting_row(
                conn,
                SET_TARGET_ENABLED,
                params![id, enabled],
                "focus target",
                id,
            )?;
            get_target(conn, id)
        })
    }

    fn remove_target(&self, id: &str) -> CoreResult<()> {
        self.with(|conn| execute(conn, DELETE_TARGET, params![id]).map(|_| ()))
    }

    fn touch_blocked(&self, id: &str, at: DateTime<Utc>) -> CoreResult<()> {
        self.with(|conn| {
            execute_expecting_row(conn, TOUCH_BLOCKED, params![id, ms(at)], "focus target", id)
        })
    }

    fn insert_intervention(&self, intervention: &Intervention) -> CoreResult<()> {
        self.with(|conn| {
            execute(
                conn,
                INSERT_INTERVENTION,
                named_params! {
                    ":id": intervention.id,
                    ":at": ms(intervention.at),
                    ":target_id": intervention.target_id,
                    ":kind": intervention.kind.as_str(),
                    ":name": intervention.name,
                    ":key": intervention.key,
                    ":action": intervention.action.as_str(),
                    ":session_id": intervention.session_id,
                    ":message": intervention.message,
                },
            )?;
            Ok(())
        })
    }

    fn get_intervention(&self, id: &str) -> CoreResult<Option<Intervention>> {
        self.with(|conn| query_opt(conn, SELECT_INTERVENTION, params![id], row_to_intervention))
    }

    fn list_interventions(&self, limit: usize) -> CoreResult<Vec<Intervention>> {
        self.with(|conn| {
            query_list(
                conn,
                SELECT_INTERVENTIONS,
                params![sql_limit(limit)],
                row_to_intervention,
            )
        })
    }

    fn count_interventions_since(&self, since: DateTime<Utc>) -> CoreResult<u64> {
        self.with(|conn| {
            query_one(conn, COUNT_INTERVENTIONS_SINCE, params![ms(since)], |row| {
                count(row, 0)
            })
        })
    }

    fn insert_session(&self, session: &FocusSession) -> CoreResult<()> {
        self.with(|conn| {
            execute(
                conn,
                INSERT_SESSION,
                named_params! {
                    ":id": session.id,
                    ":task": session.task,
                    ":started_at": ms(session.started_at),
                    ":ends_at": ms(session.ends_at),
                    ":ended_at": opt_ms(session.ended_at),
                    ":interventions": session.interventions,
                    ":hid_windows": session.hid_windows,
                    ":ran_shortcut": session.ran_shortcut,
                },
            )?;
            Ok(())
        })
    }

    fn update_session(&self, session: &FocusSession) -> CoreResult<()> {
        self.with(|conn| {
            execute_expecting_row(
                conn,
                UPDATE_SESSION,
                named_params! {
                    ":id": session.id,
                    ":task": session.task,
                    ":started_at": ms(session.started_at),
                    ":ends_at": ms(session.ends_at),
                    ":ended_at": opt_ms(session.ended_at),
                    ":interventions": session.interventions,
                    ":hid_windows": session.hid_windows,
                    ":ran_shortcut": session.ran_shortcut,
                },
                "focus session",
                &session.id,
            )
        })
    }

    fn active_session(&self) -> CoreResult<Option<FocusSession>> {
        self.with(|conn| query_opt(conn, SELECT_ACTIVE_SESSION, [], row_to_session))
    }

    fn list_sessions(&self, limit: usize) -> CoreResult<Vec<FocusSession>> {
        self.with(|conn| {
            query_list(
                conn,
                SELECT_SESSIONS,
                params![sql_limit(limit)],
                row_to_session,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};
    use ubiqx_core::ports::MaintenanceRepo;
    use ubiqx_core::CoreError;

    use super::*;
    use crate::Db;

    fn store() -> SqliteStore {
        SqliteStore::new(Db::open_in_memory().expect("in-memory database"))
    }

    fn at(hour: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, hour, min, 0).unwrap()
    }

    fn target(id: &str, kind: FocusTargetKind, name: &str, key: &str) -> FocusTarget {
        FocusTarget {
            id: id.into(),
            kind,
            name: name.into(),
            key: key.into(),
            enabled: true,
            created_at: at(9, 0),
            last_blocked_at: None,
            blocked_count: 0,
        }
    }

    fn intervention(id: &str, when: DateTime<Utc>, target_id: Option<&str>) -> Intervention {
        Intervention {
            id: id.into(),
            at: when,
            target_id: target_id.map(String::from),
            kind: FocusTargetKind::Site,
            name: "YouTube".into(),
            key: "youtube.com".into(),
            action: InterventionAction::TabClosed,
            session_id: None,
            message: "Não! Foque na sua produtividade.".into(),
        }
    }

    fn session(id: &str, started: DateTime<Utc>) -> FocusSession {
        FocusSession {
            id: id.into(),
            task: "relatório".into(),
            started_at: started,
            ends_at: started + Duration::minutes(45),
            ended_at: None,
            interventions: 0,
            hid_windows: true,
            ran_shortcut: false,
        }
    }

    #[test]
    fn targets_upsert_by_kind_and_key_and_order() {
        let s = store();
        let repo: &dyn FocusRepo = &s;
        let slack = repo
            .upsert_target(&target(
                "t1",
                FocusTargetKind::App,
                "Slack",
                "com.tinyspeck.slackmacgap",
            ))
            .expect("insert");
        assert_eq!(slack.id, "t1");
        repo.upsert_target(&target(
            "t2",
            FocusTargetKind::Site,
            "YouTube",
            "youtube.com",
        ))
        .expect("insert");
        repo.upsert_target(&target(
            "t3",
            FocusTargetKind::Site,
            "Discord",
            "discord.com",
        ))
        .expect("insert");

        repo.touch_blocked("t2", at(10, 0)).expect("touch");
        repo.touch_blocked("t2", at(10, 5)).expect("touch");
        let disabled = repo.set_target_enabled("t2", false).expect("disable");
        assert!(!disabled.enabled);
        assert_eq!(disabled.blocked_count, 2);
        assert_eq!(disabled.last_blocked_at, Some(at(10, 5)));

        // Enabled first, then by name; the disabled YouTube is last.
        let listed = repo.list_targets().expect("list");
        assert_eq!(
            listed.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["t3", "t1", "t2"]
        );

        // Adding the same site again re-enables the existing row and returns it.
        let again = repo
            .upsert_target(&target(
                "t9",
                FocusTargetKind::Site,
                "YouTube again",
                "youtube.com",
            ))
            .expect("upsert");
        assert_eq!(again.id, "t2");
        assert!(again.enabled);
        assert_eq!(again.name, "YouTube");
        assert_eq!(again.blocked_count, 2);
        assert_eq!(repo.list_targets().expect("list").len(), 3);

        // Same key with another kind is a different target.
        repo.upsert_target(&target("t4", FocusTargetKind::App, "x", "youtube.com"))
            .expect("insert");
        assert_eq!(repo.list_targets().expect("list").len(), 4);

        repo.remove_target("t1").expect("remove");
        assert_eq!(repo.get_target("t1").expect("get"), None);
        repo.remove_target("t1").expect("idempotent");
        assert!(matches!(
            repo.set_target_enabled("t1", true),
            Err(CoreError::NotFound(_))
        ));
        assert!(matches!(
            repo.touch_blocked("t1", at(11, 0)),
            Err(CoreError::NotFound(_))
        ));
    }

    #[test]
    fn interventions_list_newest_first_and_count() {
        let s = store();
        let repo: &dyn FocusRepo = &s;
        repo.upsert_target(&target(
            "t2",
            FocusTargetKind::Site,
            "YouTube",
            "youtube.com",
        ))
        .expect("insert");
        let i1 = intervention("i1", at(9, 0), Some("t2"));
        let i2 = intervention("i2", at(10, 0), None);
        let i3 = intervention("i3", at(11, 0), Some("t2"));
        for i in [&i1, &i2, &i3] {
            repo.insert_intervention(i).expect("insert");
        }
        assert_eq!(
            repo.list_interventions(10).expect("list"),
            vec![i3.clone(), i2.clone(), i1.clone()]
        );
        assert_eq!(repo.list_interventions(1).expect("list"), vec![i3.clone()]);
        assert_eq!(repo.get_intervention("i2").expect("get"), Some(i2));
        assert_eq!(repo.get_intervention("nope").expect("get"), None);
        assert_eq!(repo.count_interventions_since(at(10, 0)).expect("count"), 2);
        assert_eq!(repo.count_interventions_since(at(12, 0)).expect("count"), 0);
        // History survives the target.
        repo.remove_target("t2").expect("remove");
        assert_eq!(repo.list_interventions(10).expect("list").len(), 3);
    }

    #[test]
    fn sessions_active_update_and_list() {
        let s = store();
        let repo: &dyn FocusRepo = &s;
        assert_eq!(repo.active_session().expect("active"), None);
        let mut s1 = session("s1", at(9, 0));
        repo.insert_session(&s1).expect("insert");
        let s2 = session("s2", at(10, 0));
        repo.insert_session(&s2).expect("insert");
        assert_eq!(repo.active_session().expect("active"), Some(s2.clone()));

        s1.ended_at = Some(at(9, 30));
        s1.interventions = 3;
        s1.ran_shortcut = true;
        repo.update_session(&s1).expect("update");
        assert_eq!(
            repo.list_sessions(10).expect("list"),
            vec![s2.clone(), s1.clone()]
        );
        let mut s2_done = s2.clone();
        s2_done.ended_at = Some(at(10, 45));
        repo.update_session(&s2_done).expect("update");
        assert_eq!(repo.active_session().expect("active"), None);
        assert!(matches!(
            repo.update_session(&session("ghost", at(1, 0))),
            Err(CoreError::NotFound(_))
        ));
    }

    #[test]
    fn wipe_keeps_targets_but_clears_interventions_and_sessions() {
        let s = store();
        let repo: &dyn FocusRepo = &s;
        repo.upsert_target(&target(
            "t2",
            FocusTargetKind::Site,
            "YouTube",
            "youtube.com",
        ))
        .expect("insert");
        repo.insert_intervention(&intervention("i1", at(9, 0), Some("t2")))
            .expect("insert");
        repo.insert_session(&session("s1", at(9, 0)))
            .expect("insert");
        MaintenanceRepo::wipe_user_data(&s).expect("wipe");
        assert_eq!(repo.list_targets().expect("list").len(), 1);
        assert!(repo.list_interventions(10).expect("list").is_empty());
        assert!(repo.list_sessions(10).expect("list").is_empty());
        assert_eq!(repo.active_session().expect("active"), None);
    }
}
