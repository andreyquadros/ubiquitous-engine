//! IPC commands. Every command is async and does its repository work in a blocking task so
//! the main thread and the tokio workers never wait on SQLite.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use ubiqx_core::ports::*;
use ubiqx_core::*;
use ubiqx_engine::{
    BlockGroup, DashboardData, EngineHandle, PrivateModeDuration, ReclassifyScope, ScreenshotData,
};

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
            CoreError::AiRejected(_) => "ai_rejected",
            CoreError::AiRefused => "ai_refused",
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

/// The UI language of the running engine, for the few messages a command writes itself.
fn language(e: &EngineHandle) -> UiLanguage {
    e.settings().ui_language()
}

/// The user-facing messages a command writes itself, in the UI language.
struct Text {
    name_required: &'static str,
    system_category: &'static str,
    category_missing: &'static str,
    key_removed: fn(&str) -> String,
    key_saved: fn(&str) -> String,
    key_rejected: fn(&str) -> String,
    key_unverified: fn(&str) -> String,
    no_key: fn(&str) -> String,
    http_only: &'static str,
}

impl Text {
    fn for_language(lang: UiLanguage) -> Self {
        match lang {
            UiLanguage::PtBr => Self {
                name_required: "nome obrigatório",
                system_category: "categorias do sistema não podem ser removidas",
                category_missing: "categoria",
                key_removed: |p| format!("Chave da {p} removida."),
                key_saved: |p| format!("Chave da {p} válida e guardada no Keychain."),
                key_rejected: |m| format!("Chave recusada: {m}"),
                key_unverified: |e| format!("Não foi possível validar: {e}"),
                no_key: |p| format!("Nenhuma chave salva para {p}. Salve a chave primeiro."),
                http_only: "url: apenas links http(s)",
            },
            UiLanguage::En => Self {
                name_required: "name is required",
                system_category: "system categories cannot be removed",
                category_missing: "category",
                key_removed: |p| format!("{p} key removed."),
                key_saved: |p| format!("{p} key is valid and saved to the Keychain."),
                key_rejected: |m| format!("Key rejected: {m}"),
                key_unverified: |e| format!("Could not validate the key: {e}"),
                no_key: |p| format!("No key saved for {p}. Save the key first."),
                http_only: "url: only http(s) links are allowed",
            },
        }
    }
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

/// The screenshot attached to a block, or `null` when there is none (never captured, or
/// already purged after classification).
#[tauri::command]
pub async fn get_screenshot(
    state: State<'_, AppState>,
    block_id: String,
) -> IpcResult<Option<ScreenshotData>> {
    blocking(engine(&state), move |e| {
        ubiqx_engine::screenshot_for_block(e.state(), &block_id)
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
            let t = Text::for_language(language(&e));
            return Err(CoreError::Invalid(t.name_required.into()));
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
            let t = Text::for_language(language(&e));
            return Err(CoreError::Invalid(t.system_category.into()));
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
            let t = Text::for_language(language(&e));
            return Err(CoreError::NotFound(t.category_missing.into()));
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
        let lang = language(&e);
        let category = repos
            .categories
            .get(&report.category_id)?
            .ok_or_else(|| CoreError::NotFound(Text::for_language(lang).category_missing.into()))?;
        let mut r = report;
        r.edited = true;
        r.stale = false;
        r.summary_md = ubiqx_core::report::render_summary_md(&r, &category, lang);
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

/// One selectable AI vendor, with everything the UI needs to describe it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: AiProvider,
    pub label: String,
    pub console_url: String,
    pub key_prefix: String,
    pub default_models: AiModels,
}

impl ProviderInfo {
    fn of(id: AiProvider) -> Self {
        Self {
            id,
            label: id.label().into(),
            console_url: id.console_url().into(),
            key_prefix: id.key_prefix().into(),
            default_models: AiModels::for_provider(id),
        }
    }
}

/// Whether a vendor has a key in the secret store, and its last characters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyStatus {
    pub provider: AiProvider,
    pub configured: bool,
    pub hint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsView {
    pub settings: Settings,
    /// Whether the SELECTED provider (`settings.ai_provider`) has a key.
    pub api_key_configured: bool,
    /// Hint of the SELECTED provider's key.
    pub api_key_hint: Option<String>,
    /// Key status of every provider, in [`AiProvider::ALL`] order.
    pub api_keys: Vec<ApiKeyStatus>,
    /// Every selectable provider, in [`AiProvider::ALL`] order.
    pub providers: Vec<ProviderInfo>,
    pub permissions: PermissionStatus,
    pub ai_health: AiHealth,
    pub tracker_state: TrackerState,
    pub data_dir: String,
    pub platform: String,
    pub version: String,
}

fn settings_view(app: &AppHandle, e: &EngineHandle) -> CoreResult<SettingsView> {
    let settings = e.settings();
    let api_keys: Vec<ApiKeyStatus> = e
        .api_key_status()?
        .into_iter()
        .map(|(provider, hint)| ApiKeyStatus {
            provider,
            configured: hint.is_some(),
            hint,
        })
        .collect();
    let selected = api_keys
        .iter()
        .find(|k| k.provider == settings.ai_provider)
        .and_then(|k| k.hint.clone());
    Ok(SettingsView {
        settings,
        api_key_configured: selected.is_some(),
        api_key_hint: selected,
        api_keys,
        providers: AiProvider::ALL.into_iter().map(ProviderInfo::of).collect(),
        permissions: e.permissions(),
        ai_health: e.ai_health(),
        tracker_state: e.tracker_state(),
        data_dir: e.state().deps.data_dir.to_string_lossy().to_string(),
        // `macos` | `windows` | `linux` (the frontend keys its OS-specific copy on it).
        platform: std::env::consts::OS.into(),
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
    // The tray and application menus follow the (possibly new) language right away.
    crate::relabel_menus(&app, view.settings.ui_language());
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

/// Validates and stores the API key of `provider` (or deletes it with `key: null`).
#[tauri::command]
pub async fn set_api_key(
    state: State<'_, AppState>,
    provider: AiProvider,
    key: Option<String>,
) -> IpcResult<ApiKeyResult> {
    let e = engine(&state);
    let t = Text::for_language(language(&e));
    let key = key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty());
    match key {
        None => {
            blocking(e, move |e| e.set_api_key(provider, None)).await?;
            Ok(ApiKeyResult {
                valid: true,
                message: (t.key_removed)(provider.label()),
            })
        }
        Some(k) => match state.app.validate_api_key(provider, &k).await {
            Ok(()) => {
                blocking(e, move |e| e.set_api_key(provider, Some(&k))).await?;
                Ok(ApiKeyResult {
                    valid: true,
                    message: (t.key_saved)(provider.label()),
                })
            }
            Err(CoreError::Ai(msg)) => Ok(ApiKeyResult {
                valid: false,
                message: (t.key_rejected)(&msg),
            }),
            // Billing / model rejections: the key is real but the account cannot be used
            // (no credits, disabled key, unknown model). The client already wrote the
            // message in the UI language.
            Err(CoreError::AiRejected(msg)) => Ok(ApiKeyResult {
                valid: false,
                message: msg,
            }),
            Err(err) => Ok(ApiKeyResult {
                valid: false,
                message: (t.key_unverified)(&err.to_string()),
            }),
        },
    }
}

/// Chat-capable model ids the stored key of `provider` can use (sorted).
#[tauri::command]
pub async fn list_models(
    state: State<'_, AppState>,
    provider: AiProvider,
) -> IpcResult<Vec<String>> {
    let t = Text::for_language(language(&engine(&state)));
    state.app.list_models(provider).await.map_err(|e| match e {
        // Shown verbatim in the UI next to the "Listar modelos da conta" button.
        CoreError::AiNotConfigured => IpcError {
            code: "ai_not_configured".into(),
            message: (t.no_key)(provider.label()),
        },
        other => IpcError::from(other),
    })
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
pub async fn open_external(state: State<'_, AppState>, url: String) -> IpcResult<()> {
    // Only web links: `open_url` from Rust bypasses the opener plugin's scope, and a
    // `file://` URL would run local files/apps from anything injected into the webview.
    let parsed = url::Url::parse(url.trim()).map_err(|_| CoreError::Invalid("url".into()))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        let t = Text::for_language(language(&engine(&state)));
        return Err(CoreError::Invalid(t.http_only.into()).into());
    }
    tauri_plugin_opener::open_url(parsed.as_str(), None::<&str>)
        .map_err(|e| IpcError::from(e.to_string()))
}

#[tauri::command]
pub async fn show_window(app: AppHandle) -> IpcResult<()> {
    crate::show_main_window(&app);
    Ok(())
}

// ------------------------------------------------------------------------------------------
// Focus guard
// ------------------------------------------------------------------------------------------

/// Installed applications (sorted by name, cached a minute on the platform side).
#[tauri::command]
pub async fn list_installed_apps(state: State<'_, AppState>) -> IpcResult<Vec<InstalledApp>> {
    blocking(engine(&state), |e| e.installed_apps()).await
}

/// Domains from the user's own blocks, most time first.
#[tauri::command]
pub async fn list_known_domains(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> IpcResult<Vec<KnownDomain>> {
    let limit = limit.unwrap_or(30);
    blocking(engine(&state), move |e| e.known_domains(limit)).await
}

#[tauri::command]
pub async fn list_focus_targets(state: State<'_, AppState>) -> IpcResult<Vec<FocusTarget>> {
    blocking(engine(&state), |e| e.focus_targets()).await
}

/// Adds a blocked app or site; one that exists (same kind and key) is re-enabled and
/// returned. Keys are normalised (`invalid` when nothing is left after that).
#[tauri::command]
pub async fn add_focus_target(
    state: State<'_, AppState>,
    kind: FocusTargetKind,
    name: String,
    key: String,
) -> IpcResult<FocusTarget> {
    blocking(engine(&state), move |e| {
        e.add_focus_target(kind, &name, &key)
    })
    .await
}

#[tauri::command]
pub async fn set_focus_target_enabled(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> IpcResult<FocusTarget> {
    blocking(engine(&state), move |e| {
        e.set_focus_target_enabled(&id, enabled)
    })
    .await
}

#[tauri::command]
pub async fn remove_focus_target(state: State<'_, AppState>, id: String) -> IpcResult<()> {
    blocking(engine(&state), move |e| e.remove_focus_target(&id)).await
}

/// Interventions, newest first.
#[tauri::command]
pub async fn list_interventions(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> IpcResult<Vec<Intervention>> {
    let limit = limit.unwrap_or(30);
    blocking(engine(&state), move |e| e.interventions(limit)).await
}

#[tauri::command]
pub async fn get_focus_status(state: State<'_, AppState>) -> IpcResult<FocusStatus> {
    blocking(engine(&state), |e| e.focus_status()).await
}

/// Starts a focus session (`invalid` for a blank task or minutes outside 5..=240); a running
/// session is ended first.
#[tauri::command]
pub async fn start_focus_session(
    state: State<'_, AppState>,
    task: String,
    minutes: u32,
) -> IpcResult<FocusSession> {
    blocking(engine(&state), move |e| {
        e.start_focus_session(&task, minutes)
    })
    .await
}

/// Ends the running session early; `null` when none runs.
#[tauri::command]
pub async fn stop_focus_session(state: State<'_, AppState>) -> IpcResult<Option<FocusSession>> {
    blocking(engine(&state), |e| e.stop_focus_session()).await
}

/// Shows the intervention window with a sample message; records nothing.
#[tauri::command]
pub async fn test_intervention(state: State<'_, AppState>) -> IpcResult<()> {
    blocking(engine(&state), |e| e.test_intervention()).await
}

// ------------------------------------------------------------------------------------------
// Updates
// ------------------------------------------------------------------------------------------

#[tauri::command]
pub async fn get_update_status(state: State<'_, AppState>) -> IpcResult<UpdateStatus> {
    Ok(engine(&state).update_status())
}

/// Downloads the feed and compares it now, whatever `settings.check_updates` says. A feed
/// that could not be fetched comes back in `last_error`.
#[tauri::command]
pub async fn check_for_updates(state: State<'_, AppState>) -> IpcResult<UpdateStatus> {
    engine(&state)
        .check_for_updates()
        .await
        .map_err(IpcError::from)
}

#[tauri::command]
pub async fn dismiss_update(state: State<'_, AppState>, epoch: i64) -> IpcResult<UpdateStatus> {
    blocking(engine(&state), move |e| e.dismiss_update(epoch)).await
}

/// Opens the DMG of the available build in the browser (the download starts there).
#[tauri::command]
pub async fn open_update(state: State<'_, AppState>) -> IpcResult<()> {
    let url = engine(&state)
        .update_status()
        .available
        .map(|r| r.download_url)
        .ok_or_else(|| CoreError::NotFound("no update available".into()))?;
    crate::open_download(&url).map_err(IpcError::from)
}

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        get_dashboard,
        get_timeline,
        get_review_groups,
        get_screenshot,
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
        list_models,
        set_tracking,
        set_private_mode,
        request_permission,
        restart_app,
        delete_all_data,
        export_data,
        open_external,
        show_window,
        get_update_status,
        check_for_updates,
        dismiss_update,
        open_update,
        list_installed_apps,
        list_known_domains,
        list_focus_targets,
        add_focus_target,
        set_focus_target_enabled,
        remove_focus_target,
        list_interventions,
        get_focus_status,
        start_focus_session,
        stop_focus_session,
        test_intervention,
    ]
}
