use anyhow::Result;
use clap::{ArgAction, Parser};
use std::path::PathBuf;
use tracing::{error, info};

mod config;
mod db;
mod freshrss;
mod greader;
mod openai_client;
mod processor;
mod scheduler;
mod tui_app;

#[derive(Parser, Debug)]
#[command(name = "freshrss-filter")] 
#[command(about = "Classify and remove ads from FreshRSS using LLM", long_about = None)]
struct Cli {
    /// Path to config file (TOML)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Dry run: do not delete/modify items
    #[arg(long, action = ArgAction::SetTrue)]
    dry_run: bool,

    /// Run once and exit (no scheduler, no TUI)
    #[arg(long, action = ArgAction::SetTrue)]
    once: bool,

    /// Verbose logging
    #[arg(short, long, action = ArgAction::Count)]
    verbose: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let cli = Cli::parse();
    let cfg = config::load(cli.config.as_deref()).await?;
    let cfg = cfg.with_overrides(cli.dry_run);

    info!(config = ?cfg, "config_loaded");

    let db = db::Database::new(&cfg.database.path).await?;

    let fr_client = freshrss::build_client(&cfg.freshrss)?;
    let gr_client = if let (Some(u), Some(p)) = (&cfg.freshrss.greader_username, &cfg.freshrss.greader_password) {
        Some(greader::build_client(&cfg.freshrss, u.clone(), p.clone())?)
    } else { None };
    let llm = openai_client::OpenAiClient::new(cfg.openai.clone());

    let shared_state = processor::ProcessorState::default();
    let proc = processor::Processor::new(db.clone(), fr_client, gr_client, llm, cfg.clone(), shared_state.clone());

    if cli.once {
        proc.run_once().await?;
        return Ok(());
    }

    // TUI + Scheduler
    let ui_handle = tokio::spawn(tui_app::run_ui(shared_state.clone()));

    let mut sched = scheduler::Scheduler::new(cfg.scheduler.clone()).await?;
    let proc_clone = proc.clone();
    sched.add_job(move || {
        let value = proc_clone.clone();
        async move {
            let proc = value.clone();
            if let Err(e) = proc.run_once().await {
                error!(?e, "processor_run_once_error");
            }
        }
    }).await?;

    info!("starting_scheduler");
    sched.start().await?;

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    info!("shutting_down");
    sched.shutdown().await;
    ui_handle.abort();
    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, prelude::*};
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info,freshrss_filter=debug".to_string());
    let indicatif_layer = tracing_indicatif::IndicatifLayer::new();
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_writer(indicatif_layer.get_stderr_writer())
        .compact();
    tracing_subscriber::registry()
        .with(EnvFilter::new(filter))
        .with(fmt_layer)
        .with(indicatif_layer)
        .init();
}
