//! `ubiqx` — headless command-line front end for the engine.
//!
//! Typical uses:
//!
//! ```text
//! ubiqx simulate --minutes 8 --fake-ai      # replay a demo day with fake AI, print a summary
//! ubiqx track                                # run the native tracker in the foreground (macOS)
//! ubiqx status                               # today's totals from the database
//! ubiqx classify                             # force one classification pass
//! ubiqx report 2026-09-17 --category <id>    # generate/print a daily report
//! ubiqx categories                           # list categories
//! ubiqx --provider openai classify           # switch the AI vendor, then run the command
//! ```
//!
//! API keys come from the environment (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `XAI_API_KEY`
//! or their `UBIQX_`-prefixed variants) through the platform secret store.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use clap::{Parser, Subcommand};
use ubiqx_app::ubiqx_platform::mock::Scenario;
use ubiqx_app::{AiBackend, App, AppConfig};
use ubiqx_core::ports::EventSink;
use ubiqx_core::{AiProvider, EngineEvent};

#[derive(Parser)]
#[command(name = "ubiqx", version, about = "ubiqX headless engine")]
struct Cli {
    /// Data directory (default: platform application-support directory).
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    /// Use an in-memory database (nothing is persisted).
    #[arg(long, global = true)]
    ephemeral: bool,
    /// Use deterministic fake AI instead of the vendor APIs.
    #[arg(long, global = true)]
    fake_ai: bool,
    /// Disable remote AI entirely (rules + memory only).
    #[arg(long, global = true)]
    no_ai: bool,
    /// Provedor de IA que responde às chamadas remotas (anthropic, openai ou xai). Fica salvo
    /// nas configurações; a chave vem do ambiente (ANTHROPIC_API_KEY, OPENAI_API_KEY,
    /// XAI_API_KEY).
    #[arg(long, global = true, value_parser = parse_provider, value_name = "PROVEDOR")]
    provider: Option<AiProvider>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Replay the built-in demo day through the whole pipeline and print what happened.
    Simulate {
        /// How long to run (wall-clock seconds); each second replays one minute of the demo day.
        #[arg(long, default_value_t = 60)]
        seconds: u64,
        /// Print engine events as they happen.
        #[arg(long)]
        events: bool,
    },
    /// Run the native tracker in the foreground until Ctrl-C.
    Track {
        #[arg(long)]
        events: bool,
    },
    /// Print today's (or a date's) totals, blocks and AI health.
    Status { date: Option<NaiveDate> },
    /// Force one classification pass.
    Classify,
    /// Generate and print the daily report for a category.
    Report {
        date: Option<NaiveDate>,
        #[arg(long)]
        category: String,
    },
    /// Print the monthly report (Markdown) for a category.
    Monthly {
        #[arg(long)]
        category: String,
        #[arg(long)]
        year: i32,
        #[arg(long)]
        month: u32,
    },
    /// List categories.
    Categories,
    /// Export data as JSON and print the path.
    Export,
}

struct PrintSink(bool);

impl EventSink for PrintSink {
    fn emit(&self, event: EngineEvent) {
        if !self.0 {
            return;
        }
        match &event {
            EngineEvent::BlockClosed { block } => println!(
                "▪ fechado  {}–{}  {:<22} {:<40} {}",
                block.started_at.with_timezone(&Local).format("%H:%M:%S"),
                block.ended_at.with_timezone(&Local).format("%H:%M:%S"),
                block.app_name,
                truncate(&block.title, 40),
                block.category_id.as_deref().unwrap_or("-")
            ),
            EngineEvent::BlocksClassified { block_ids } => {
                println!("✓ classificados {} blocos", block_ids.len())
            }
            EngineEvent::ReportReady { report } => println!(
                "📄 relatório pronto: {} ({} itens)",
                report.category_id,
                report.items.len()
            ),
            EngineEvent::Nudge { nudge } => println!(
                "🤖 UBI [{}]: {} — {}",
                nudge.kind.as_str(),
                nudge.title,
                nudge.message
            ),
            EngineEvent::TrackerState { state } => println!("● tracker: {state:?}"),
            EngineEvent::AiHealth { health } => println!("● IA: {health:?}"),
            EngineEvent::PermissionRequired { permission } => {
                println!("⚠ permissão necessária: {permission}")
            }
            EngineEvent::UpdateAvailable { release } => println!(
                "⬆ nova versão disponível: {} ({}) {}",
                release.version, release.build.sha, release.download_url
            ),
            EngineEvent::BlockOpened { .. } | EngineEvent::ScreenshotTaken { .. } => {}
        }
    }
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n - 1).collect::<String>())
    }
}

fn ai_backend(cli: &Cli) -> AiBackend {
    if cli.no_ai {
        AiBackend::None
    } else if cli.fake_ai {
        AiBackend::Fake
    } else {
        AiBackend::Remote
    }
}

fn parse_provider(s: &str) -> Result<AiProvider, String> {
    AiProvider::parse(s).ok_or_else(|| {
        format!(
            "provedor desconhecido \"{s}\"; use {}",
            AiProvider::ALL
                .iter()
                .map(|p| p.id())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

/// Persists `--provider` (and the models it implies) before the subcommand runs, so this
/// invocation and the next ones use that vendor.
fn apply_provider(app: &App, provider: Option<AiProvider>) -> Result<()> {
    let Some(provider) = provider else {
        return Ok(());
    };
    let mut s = app.engine.settings();
    if s.ai_provider != provider {
        s.ai_provider = provider;
        s.reconcile_models();
        app.engine.update_settings(s).context("save provider")?;
    }
    Ok(())
}

/// Starts the engine and applies the global flags that change persisted settings.
fn start(cli: &Cli, config: AppConfig, sink: Arc<dyn EventSink>) -> Result<App> {
    let app = App::start(config, sink).context("start engine")?;
    apply_provider(&app, cli.provider)?;
    Ok(app)
}

#[tokio::main]
async fn main() -> Result<()> {
    ubiqx_app::init_tracing("warn");
    let cli = Cli::parse();
    let base = AppConfig {
        data_dir: cli.data_dir.clone(),
        in_memory_db: cli.ephemeral,
        scripted_platform: None,
        ai: ai_backend(&cli),
        ..AppConfig::default()
    };

    match &cli.cmd {
        Cmd::Simulate { seconds, events } => {
            let config = AppConfig {
                scripted_platform: Some(Scenario::demo_day_with(1)),
                ..base
            };
            let app = start(&cli, config, Arc::new(PrintSink(*events)))?;
            if ubiqx_app::demo::seed(app.engine.state().deps.repos.categories.as_ref())? {
                println!(
                    "Categorias de demonstração criadas (IFRO, Incubadora, Cidades Inteligentes)."
                );
            }
            // Speed up: 2 s samples so a short run covers a few minutes of "activity".
            let mut s = app.engine.settings();
            // Running the simulation is the consent the desktop collects in onboarding.
            s.onboarding_done = true;
            s.sample_interval_secs = 1;
            s.min_block_secs = 2;
            s.classify_batch_min = 1;
            s.classify_max_wait_secs = 1;
            app.engine.update_settings(s)?;
            println!("Simulando {seconds}s do dia de demonstração…");
            tokio::time::sleep(std::time::Duration::from_secs(*seconds)).await;
            let r = app.engine.classify_now().await?;
            println!("Classificação: {r:?}");
            print_status(&app, Local::now().date_naive())?;
            app.engine.shutdown();
        }
        Cmd::Track { events } => {
            let app = start(&cli, base, Arc::new(PrintSink(*events)))?;
            // Nothing is recorded or sent before consent; invoking `track` is that consent.
            let mut s = app.engine.settings();
            if !s.onboarding_done {
                s.onboarding_done = true;
                app.engine.update_settings(s)?;
            }
            println!(
                "Rastreando… (Ctrl-C para sair). Dados em {}",
                app.data_dir.display()
            );
            tokio::signal::ctrl_c().await?;
            // Waits for the tracker to persist the open block.
            app.engine.shutdown();
        }
        Cmd::Status { date } => {
            let app = start(
                &cli,
                AppConfig {
                    scripted_platform: Some(Scenario::new(vec![])),
                    ..base
                },
                Arc::new(PrintSink(false)),
            )?;
            app.engine.pause();
            print_status(&app, date.unwrap_or_else(|| Local::now().date_naive()))?;
            app.engine.shutdown();
        }
        Cmd::Classify => {
            let app = start(
                &cli,
                AppConfig {
                    scripted_platform: Some(Scenario::new(vec![])),
                    ..base
                },
                Arc::new(PrintSink(true)),
            )?;
            app.engine.pause();
            let r = app.engine.classify_now().await?;
            println!("{r:?}");
            app.engine.shutdown();
        }
        Cmd::Report { date, category } => {
            let app = start(
                &cli,
                AppConfig {
                    scripted_platform: Some(Scenario::new(vec![])),
                    ..base
                },
                Arc::new(PrintSink(false)),
            )?;
            app.engine.pause();
            let r = app
                .engine
                .generate_report(date.unwrap_or_else(|| Local::now().date_naive()), category)
                .await?;
            println!("{}", r.summary_md);
            app.engine.shutdown();
        }
        Cmd::Monthly {
            category,
            year,
            month,
        } => {
            let app = start(
                &cli,
                AppConfig {
                    scripted_platform: Some(Scenario::new(vec![])),
                    ..base
                },
                Arc::new(PrintSink(false)),
            )?;
            app.engine.pause();
            println!(
                "{}",
                ubiqx_engine::monthly_report_md(app.engine.state(), category, *year, *month)?
            );
            app.engine.shutdown();
        }
        Cmd::Categories => {
            let app = start(
                &cli,
                AppConfig {
                    scripted_platform: Some(Scenario::new(vec![])),
                    ..base
                },
                Arc::new(PrintSink(false)),
            )?;
            app.engine.pause();
            for c in app.engine.state().deps.repos.categories.list(true)? {
                println!(
                    "{:<20} {:<24} {} {}",
                    c.id,
                    c.name,
                    c.color,
                    if c.is_system { "(sistema)" } else { "" }
                );
            }
            app.engine.shutdown();
        }
        Cmd::Export => {
            let app = start(
                &cli,
                AppConfig {
                    scripted_platform: Some(Scenario::new(vec![])),
                    ..base
                },
                Arc::new(PrintSink(false)),
            )?;
            app.engine.pause();
            println!("{}", app.engine.export_json()?.display());
            app.engine.shutdown();
        }
    }
    Ok(())
}

fn print_status(app: &App, date: NaiveDate) -> Result<()> {
    let d = ubiqx_engine::dashboard(app.engine.state(), date)?;
    println!(
        "\n== {date} · foco {} · humor {:?} · tracker {:?} · IA {:?}",
        d.stats.focus_score, d.stats.mood, d.tracker_state, d.ai_health
    );
    println!(
        "produtivo {} · distração {} · sem categoria {} · ocioso {}",
        hm(d.stats.productive_secs),
        hm(d.stats.distraction_secs),
        hm(d.stats.uncategorized_secs),
        hm(d.stats.idle_secs)
    );
    println!("-- por categoria");
    for t in &d.totals {
        let name = t
            .category_id
            .as_deref()
            .and_then(|id| {
                d.categories
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.name.clone())
            })
            .unwrap_or_else(|| "Sem categoria".into());
        println!(
            "  {:<24} {:>8}  ({} blocos)",
            name,
            hm(t.secs),
            t.block_count
        );
    }
    println!("-- blocos ({})", d.timeline.len());
    for b in d.timeline.iter().take(60) {
        println!(
            "  {}–{} {:<22} {:<36} {:<18} {:.2} {}",
            b.started_at.with_timezone(&Local).format("%H:%M"),
            b.ended_at.with_timezone(&Local).format("%H:%M"),
            truncate(&b.app_name, 22),
            truncate(&b.title, 36),
            b.category_id.as_deref().unwrap_or("-"),
            b.confidence,
            b.source.map(|s| s.as_str()).unwrap_or("")
        );
    }
    println!(
        "-- uso de IA no mês: {} chamadas, US$ {:.4} de US$ {:.2}",
        d.usage_month.calls, d.usage_month.cost_usd, d.budget_usd
    );
    Ok(())
}

fn hm(secs: i64) -> String {
    format!("{}h{:02}", secs / 3600, (secs % 3600) / 60)
}
