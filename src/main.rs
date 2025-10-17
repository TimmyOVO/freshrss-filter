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

    // Simple console countdown for next run based on cron
    let cron_expr = cfg.scheduler.cron.clone();
    let countdown_handle = tokio::spawn(async move {
        use indicatif::{ProgressBar, ProgressStyle};
        use std::time::Duration as StdDuration;
        use chrono::{Local, Timelike, Duration as ChronoDuration};

        fn next_run_in(cron: &str) -> Option<StdDuration> {
            let parts: Vec<&str> = cron.split_whitespace().collect();
            if parts.len() != 6 { return None; }
            let sec = parts[0];
            let min = parts[1];
            let mut t = Local::now();
            let now = t;

            if sec == "0" {
                // advance to next minute boundary
                let add_secs = (60 - t.second()) % 60;
                t = t + ChronoDuration::seconds(add_secs as i64);
                if let Some(tt) = t.with_second(0) { t = tt; }

                if let Some(step_str) = min.strip_prefix("*/") {
                    if let Ok(step) = step_str.parse::<u32>() {
                        // advance to minute divisible by step
                        while t.minute() % step != 0 {
                            t = t + ChronoDuration::minutes(1);
                        }
                    }
                } else if let Ok(target_min) = min.parse::<u32>() {
                    while t.minute() != target_min {
                        t = t + ChronoDuration::minutes(1);
                    }
                } else {
                    // unknown minute field, default next minute
                }

                let dur = t - now;
                return dur.to_std().ok();
            }
            None
        }

        let pb = ProgressBar::new_spinner();
        pb.set_style(ProgressStyle::with_template("{spinner} 下次运行 ~{msg}").unwrap());
        loop {
            let remain = next_run_in(&cron_expr).unwrap_or_else(|| StdDuration::from_secs(600));
            let mins = remain.as_secs() / 60;
            let secs = remain.as_secs() % 60;
            pb.set_message(format!("{:02}分{:02}秒", mins, secs));
            pb.tick();
            tokio::time::sleep(StdDuration::from_secs(1)).await;
        }
    });

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
    countdown_handle.abort();
    Ok(())
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, prelude::*};
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info,freshrss_filter=debug".to_string());
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .compact();
    tracing_subscriber::registry()
        .with(EnvFilter::new(filter))
        .with(fmt_layer)
        .init();
}
