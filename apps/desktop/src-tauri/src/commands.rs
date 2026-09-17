//! IPC commands. Every command is async and does its repository work in a blocking task so
//! the main thread and the tokio workers never wait on SQLite.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use ubiqx_core::ports::*;
use ubiqx_core::*;
use ubiqx_engine::{BlockGroup, DashboardData, EngineHandle, PrivateModeDuration, ReclassifyScope};

use crate::AppState;

#[derive(Debug, Serialize)]
pub struct IpcError {
    pub code: String,
    pub message: String,
}

impl From<CoreError> for IpcError {
    fn from(e: CoreError) -> Self {
        let code = match &e {
            CoreError::Storage(_) => "storage",
            CoreError::Platform(_) => "platform",
            CoreError::Permission(_) => "permission",
            CoreError::Ai(_) => "ai",
            CoreError::AiNotConfigured => "ai_not_configured",
            CoreError::RateLimited { .. } => "rate_limited",
            CoreError::Invalid(_) => "invalid",
            CoreError::NotFound(_) => "not_found",
            CoreError::Other(_) => "other",
        };
        Self {
            code: code.into(),
            message: e.to_string(),
        }
    }
}

impl From<String> for IpcError {
    fn from(message: String) -> Self {
        Self {
            code: "other".into(),
            message,
        }
    }
}

type IpcResult<T> = Result<T, IpcError>;

fn parse_date(date: &str) -> IpcResult<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| CoreError::Invalid(format!("invalid date {date}")).into())
}

fn parse_ts(ts: &str) -> IpcResult<DateTime<Utc>> {
    ts.parse::<DateTime<Utc>>()
        .map_err(|_| CoreError::Invalid(format!("invalid timestamp {ts}")).into())
}

/// Runs a blocking closure with a clone of the engine handle.
async fn blocking<T, F>(engine: EngineHandle, f: F) -> IpcResult<T>
where
    T: Send + 'static,
    F: FnOnce(EngineHandle) -> CoreResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || f(engine))
        .await
        .map_err(|e| IpcError::from(e.to_string()))?
        .map_err(IpcError::from)
}

fn engine(state: &State<'_, AppState>) -> EngineHandle {
    state.app.engine.clone()
}

// ------------------------------------------------------------------------------------------
// Dashboard & timeline
// ------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn get_dashboard(state: State<'_, AppState>, date: String) -> IpcResult<DashboardData> {
    let date = parse_date(&date)?;
    blocking(engine(&state), move |e| {
        ubiqx_engine::dashboard(e.state(), date)
    })
    .await
}

#[tauri::command]
pub async fn get_timeline(
    state: State<'_, AppState>,
    date: String,
) -> IpcResult<Vec<ActivityBlock>> {
    let date = parse_date(&date)?;
    blocking(engine(&state), move |e| {
        ubiqx_engine::timeline(e.state(), date)
    })
    .await
}

#[tauri::command]
pub async fn get_review_groups(
    state: State<'_, AppState>,
    date: String,
) -> IpcResult<Vec<BlockGroup>> {
    let date = parse_date(&date)?;
    blocking(engine(&state), move |e| {
        ubiqx_engine::review_groups(e.state(), date)
    })
    .await
}

#[tauri::command]
pub async fn get_ai_sent(
    state: State<'_, AppState>,
    date: String,
) -> IpcResult<Vec<ActivityBlock>> {
    let date = parse_date(&date)?;
    blocking(engine(&state), move |e| {
        ubiqx_engine::ai_sent_blocks(e.state(), date)
    })
    .await
}

// ------------------------------------------------------------------------------------------
// Corrections
// ------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn reclassify(
    state: State<'_, AppState>,
    block_id: String,
    category_id: String,
    scope: ReclassifyScope,
    note: Option<String>,
) -> IpcResult<ubiqx_engine::learning::CorrectionOutcome> {
    blocking(engine(&state), move |e| {
        e.reclassify(&block_id, &category_id, note, scope)
    })
    .await
}

#[tauri::command]
pub async fn reclassify_group(
    state: State<'_, AppState>,
    date: String,
    key: String,
    category_id: String,
) -> IpcResult<ubiqx_engine::learning::CorrectionOutcome> {
    let date = parse_date(&date)?;
    blocking(engine(&state), move |e| {
        let ids = ubiqx_engine::group_block_ids(e.state(), date, &key)?;
        let mut merged: Option<ubiqx_engine::learning::CorrectionOutcome> = None;
        for id in ids {
            let out = e.reclassify(&id, &category_id, None, ReclassifyScope::Block)?;
            match &mut merged {
                None => merged = Some(out),
                Some(m) => {
                    m.block_ids.extend(out.block_ids);
                    m.backfilled += out.backfilled;
                    for s in out.suggestions {
                        if !m
                            .suggestions
                            .iter()
                            .any(|x| x.matcher == s.matcher && x.pattern == s.pattern)
                        {
                            m.suggestions.push(s);
                        }
                    }
                    m.auto_rules.extend(out.auto_rules);
                    m.disabled_rules.extend(out.disabled_rules);
                }
            }
        }
        merged.ok_or_else(|| CoreError::NotFound("group has no blocks".into()))
    })
    .await
}

#[tauri::command]
pub async fn accept_rule_suggestion(
    state: State<'_, AppState>,
    suggestion: RuleSuggestion,
) -> IpcResult<Rule> {
    blocking(engine(&state), move |e| {
        e.accept_rule_suggestion(&suggestion)
    })
    .await
}

#[tauri::command]
pub async fn split_block(
    state: State<'_, AppState>,
    block_id: String,
    at: String,
) -> IpcResult<String> {
    let at = parse_ts(&at)?;
    blocking(engine(&state), move |e| e.split_block(&block_id, at)).await
}

#[tauri::command]
pub async fn add_manual_entry(
    state: State<'_, AppState>,
    started_at: String,
    ended_at: String,
    category_id: String,
    note: Option<String>,
) -> IpcResult<ActivityBlock> {
    let s = parse_ts(&started_at)?;
    let en = parse_ts(&ended_at)?;
    blocking(engine(&state), move |e| {
        e.add_manual_entry(s, en, &category_id, note)
    })
    .await
}

#[tauri::command]
pub async fn classify_now(
    state: State<'_, AppState>,
) -> IpcResult<ubiqx_engine::classify::ClassifyReport> {
    engine(&state).classify_now().await.map_err(IpcError::from)
}

// ------------------------------------------------------------------------------------------
// Categories & rules
// ------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn list_categories(
    state: State<'_, AppState>,
    include_archived: Option<bool>,
) -> IpcResult<Vec<Category>> {
    let inc = include_archived.unwrap_or(false);
    blocking(engine(&state), move |e| {
        e.state().deps.repos.categories.list(inc)
    })
    .await
}

#[tauri::command]
pub async fn save_category(state: State<'_, AppState>, category: Category) -> IpcResult<Category> {
    blocking(engine(&state), move |e| {
        let mut c = category;
        if c.name.trim().is_empty() {
            return Err(CoreError::Invalid("nome obrigatório".into()));
        }
        let repos = &e.state().deps.repos;
        if c.id.trim().is_empty() {
            c.id = new_id();
            c.created_at = e.state().now();
            let n = repos.categories.list(true)?.len() as i32;
            if c.sort_order == 0 {
                c.sort_order = n;
            }
        } else if let Some(existing) = repos.categories.get(&c.id)? {
            if existing.is_system {
                // Only cosmetic fields of system categories can change.
                c.is_system = true;
                c.archived = false;
            }
        }
        if c.color.trim().is_empty() {
            c.color = "#2563EB".into();
        }
        if c.icon.trim().is_empty() {
            c.icon = "folder".into();
        }
        repos.categories.upsert(&c)?;
        Ok(c)
    })
    .await
}

#[tauri::command]
pub async fn delete_category(state: State<'_, AppState>, id: String) -> IpcResult<()> {
    blocking(engine(&state), move |e| {
        let repos = &e.state().deps.repos;
        let Some(c) = repos.categories.get(&id)? else {
            return Ok(());
        };
        if c.is_system {
            return Err(CoreError::Invalid(
                "categorias do sistema não podem ser removidas".into(),
            ));
        }
        repos.categories.delete(&id)
    })
    .await
}

#[tauri::command]
pub async fn list_rules(state: State<'_, AppState>) -> IpcResult<Vec<Rule>> {
    blocking(engine(&state), move |e| e.state().deps.repos.rules.list()).await
}

#[tauri::command]
pub async fn save_rule(state: State<'_, AppState>, rule: Rule) -> IpcResult<Rule> {
    blocking(engine(&state), move |e| {
        let mut r = rule;
        ubiqx_core::rules::validate_rule(r.matcher, &r.pattern).map_err(CoreError::Invalid)?;
        if e.state()
            .deps
            .repos
            .categories
            .get(&r.category_id)?
            .is_none()
        {
            return Err(CoreError::NotFound("categoria".into()));
        }
        if r.id.trim().is_empty() {
            r.id = new_id();
            r.created_at = e.state().now();
        }
        e.state().deps.repos.rules.upsert(&r)?;
        Ok(r)
    })
    .await
}

#[tauri::command]
pub async fn delete_rule(state: State<'_, AppState>, id: String) -> IpcResult<()> {
    blocking(engine(&state), move |e| {
        e.state().deps.repos.rules.delete(&id)
    })
    .await
}

// ------------------------------------------------------------------------------------------
// Reports
// ------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn get_reports(state: State<'_, AppState>, date: String) -> IpcResult<Vec<DailyReport>> {
    let date = parse_date(&date)?;
    blocking(engine(&state), move |e| {
        e.state().deps.repos.reports.list_for_date(date)
    })
    .await
}

#[tauri::command]
pub async fn list_reports_between(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> IpcResult<Vec<DailyReport>> {
    let from = parse_date(&from)?;
    let to = parse_date(&to)?;
    blocking(engine(&state), move |e| {
        e.state().deps.repos.reports.list_between(from, to)
    })
    .await
}

#[tauri::command]
pub async fn generate_report(
    state: State<'_, AppState>,
    date: String,
    category_id: String,
) -> IpcResult<DailyReport> {
    let date = parse_date(&date)?;
    engine(&state)
        .generate_report(date, &category_id)
        .await
        .map_err(IpcError::from)
}

#[tauri::command]
pub async fn update_report(
    state: State<'_, AppState>,
    report: DailyReport,
) -> IpcResult<DailyReport> {
    blocking(engine(&state), move |e| {
        let repos = &e.state().deps.repos;
        let category = repos
            .categories
            .get(&report.category_id)?
            .ok_or_else(|| CoreError::NotFound("categoria".into()))?;
        let mut r = report;
        r.edited = true;
        r.stale = false;
        r.summary_md = ubiqx_core::report::render_summary_md(&r, &category);
        repos.reports.upsert(&r)?;
        Ok(r)
    })
    .await
}

#[tauri::command]
pub async fn get_monthly_report(
    state: State<'_, AppState>,
    category_id: String,
    year: i32,
    month: u32,
) -> IpcResult<String> {
    blocking(engine(&state), move |e| {
        ubiqx_engine::monthly_report_md(e.state(), &category_id, year, month)
    })
    .await
}

// ------------------------------------------------------------------------------------------
// Nudges & advice
// ------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn get_nudges(state: State<'_, AppState>, limit: Option<usize>) -> IpcResult<Vec<Nudge>> {
    let limit = limit.unwrap_or(30);
    blocking(engine(&state), move |e| {
        e.state().deps.repos.nudges.list_recent(limit)
    })
    .await
}

#[tauri::command]
pub async fn mark_nudges_seen(state: State<'_, AppState>) -> IpcResult<()> {
    blocking(engine(&state), move |e| {
        e.state().deps.repos.nudges.mark_all_seen()
    })
    .await
}

#[tauri::command]
pub async fn snooze_nudges(state: State<'_, AppState>, minutes: i64) -> IpcResult<()> {
    blocking(engine(&state), move |e| {
        e.snooze_nudges(minutes.clamp(5, 24 * 60))
    })
    .await
}

#[tauri::command]
pub async fn get_advice(state: State<'_, AppState>, force: Option<bool>) -> IpcResult<Advice> {
    engine(&state)
        .advice(force.unwrap_or(false))
        .await
        .map_err(IpcError::from)
}

// ------------------------------------------------------------------------------------------
// Settings, tracking, permissions
// ------------------------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsView {
    pub settings: Settings,
    pub api_key_configured: bool,
    pub api_key_hint: Option<String>,
    pub permissions: PermissionStatus,
    pub ai_health: AiHealth,
    pub tracker_state: TrackerState,
    pub data_dir: String,
    pub platform: String,
    pub version: String,
}

fn settings_view(app: &AppHandle, e: &EngineHandle) -> CoreResult<SettingsView> {
    let hint = e.api_key_hint()?;
    Ok(SettingsView {
        settings: e.settings(),
        api_key_configured: hint.is_some(),
        api_key_hint: hint,
        permissions: e.permissions(),
        ai_health: e.ai_health(),
        tracker_state: e.tracker_state(),
        data_dir: e.state().deps.data_dir.to_string_lossy().to_string(),
        platform: if cfg!(target_os = "macos") {
            "macos".into()
        } else {
            "other".into()
        },
        version: app.package_info().version.to_string(),
    })
}

#[tauri::command]
pub async fn get_settings(app: AppHandle, state: State<'_, AppState>) -> IpcResult<SettingsView> {
    let e = engine(&state);
    tauri::async_runtime::spawn_blocking(move || settings_view(&app, &e))
        .await
        .map_err(|e| IpcError::from(e.to_string()))?
        .map_err(IpcError::from)
}

#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> IpcResult<SettingsView> {
    let e = engine(&state);
    let launch = settings.launch_at_login;
    let view = tauri::async_runtime::spawn_blocking({
        let app = app.clone();
        move || {
            e.update_settings(settings)?;
            settings_view(&app, &e)
        }
    })
    .await
    .map_err(|e| IpcError::from(e.to_string()))?
    .map_err(IpcError::from)?;
    let autostart = app.autolaunch();
    let r = if launch {
        autostart.enable()
    } else {
        autostart.disable()
    };
    if let Err(err) = r {
        tracing::warn!(error = %err, "autostart update failed");
    }
    Ok(view)
}

#[derive(Debug, Serialize)]
pub struct ApiKeyResult {
    pub valid: bool,
    pub message: String,
}

#[tauri::command]
pub async fn set_api_key(
    state: State<'_, AppState>,
    key: Option<String>,
) -> IpcResult<ApiKeyResult> {
    let e = engine(&state);
    let key = key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty());
    match key {
        None => {
            blocking(e, |e| e.set_api_key(None)).await?;
            Ok(ApiKeyResult {
                valid: true,
                message: "Chave removida.".into(),
            })
        }
        Some(k) => match state.app.validate_api_key(&k).await {
            Ok(()) => {
                blocking(e, move |e| e.set_api_key(Some(&k))).await?;
                Ok(ApiKeyResult {
                    valid: true,
                    message: "Chave válida e guardada no Keychain.".into(),
                })
            }
            Err(CoreError::Ai(msg)) => Ok(ApiKeyResult {
                valid: false,
                message: format!("Chave recusada: {msg}"),
            }),
            Err(err) => Ok(ApiKeyResult {
                valid: false,
                message: format!("Não foi possível validar: {err}"),
            }),
        },
    }
}

#[tauri::command]
pub async fn set_tracking(state: State<'_, AppState>, enabled: bool) -> IpcResult<()> {
    let e = engine(&state);
    blocking(e, move |e| {
        if enabled {
            e.resume()
        } else {
            e.pause()
        }
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn set_private_mode(
    state: State<'_, AppState>,
    duration: PrivateModeDuration,
) -> IpcResult<()> {
    blocking(engine(&state), move |e| e.set_private_mode(duration)).await
}

#[tauri::command]
pub async fn request_permission(state: State<'_, AppState>, kind: PermissionKind) -> IpcResult<()> {
    blocking(engine(&state), move |e| e.request_permission(kind)).await
}

#[tauri::command]
pub async fn restart_app(app: AppHandle) -> IpcResult<()> {
    app.restart();
}

#[tauri::command]
pub async fn delete_all_data(state: State<'_, AppState>) -> IpcResult<()> {
    blocking(engine(&state), |e| e.delete_all_data()).await
}

#[tauri::command]
pub async fn export_data(state: State<'_, AppState>) -> IpcResult<String> {
    blocking(engine(&state), |e| {
        e.export_json().map(|p| p.to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
pub async fn open_external(url: String) -> IpcResult<()> {
    if !(url.starts_with("https://") || url.starts_with("http://") || url.starts_with("file://")) {
        return Err(CoreError::Invalid("url".into()).into());
    }
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| IpcError::from(e.to_string()))
}

#[tauri::command]
pub async fn show_window(app: AppHandle) -> IpcResult<()> {
    crate::show_main_window(&app);
    Ok(())
}

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        get_dashboard,
        get_timeline,
        get_review_groups,
        get_ai_sent,
        reclassify,
        reclassify_group,
        accept_rule_suggestion,
        split_block,
        add_manual_entry,
        classify_now,
        list_categories,
        save_category,
        delete_category,
        list_rules,
        save_rule,
        delete_rule,
        get_reports,
        list_reports_between,
        generate_report,
        update_report,
        get_monthly_report,
        get_nudges,
        mark_nudges_seen,
        snooze_nudges,
        get_advice,
        get_settings,
        update_settings,
        set_api_key,
        set_tracking,
        set_private_mode,
        request_permission,
        restart_app,
        delete_all_data,
        export_data,
        open_external,
        show_window,
    ]
}
