use anyhow::Result;
use futures::stream::{self, StreamExt};
use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::sync::{Arc, Mutex};
use tracing::instrument;
use tracing::{info, warn};

use crate::{
    config::Config,
    db::Database,
    freshrss::{FreshRssClient, item_text},
    greader::GReaderClient,
    openai_client::OpenAiClient,
};

#[derive(Clone, Default)]
pub struct ProcessorState {
    pub last_run_status: Arc<Mutex<String>>, // for TUI display
}

#[derive(Clone)]
pub struct Processor {
    db: Database,
    fr: FreshRssClient,
    llm: OpenAiClient,
    gr: Option<GReaderClient>,
    cfg: Config,
    state: ProcessorState,
}

impl Processor {
    pub fn new(
        db: Database,
        fr: FreshRssClient,
        gr: Option<GReaderClient>,
        llm: OpenAiClient,
        cfg: Config,
        state: ProcessorState,
    ) -> Self {
        Self {
            db,
            fr,
            gr,
            llm,
            cfg,
            state,
        }
    }

    #[instrument(skip(self), name = "run_once")]
    pub async fn run_once(&self) -> Result<()> {
        // Setup progress UI
        let mp = MultiProgress::new();
        let fetch_pb = mp.add(ProgressBar::new_spinner());
        fetch_pb.set_style(ProgressStyle::with_template("{spinner} 正在获取未读条目...").unwrap());
        fetch_pb.enable_steady_tick(std::time::Duration::from_millis(120));

        // Fetch items
        let items = self.fr.fetch_unread_items().await?;
        let total = items.len();
        fetch_pb.finish_with_message(format!("已获取 {} 条", total));

        if total == 0 {
            if let Ok(mut s) = self.state.last_run_status.lock() {
                *s = "reviewed_items=0/0".into();
            }
            info!(reviewed = 0, total = 0, "processor_run_once_done");
            return Ok(());
        }

        // Main progress bar
        let total_u64 = total as u64;
        let main_pb = mp.add(ProgressBar::new(total_u64));
        let concurrency = 5usize;
        main_pb.set_prefix(format!("处理中 并发={}", concurrency));
        main_pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{prefix} {pos}/{len} [{bar:40.cyan/blue}] {percent}% | 剩余~{eta} | {msg}",
                )
                .expect("valid template")
                .progress_chars("=>-"),
        );
        main_pb.set_message(format!("剩余: {}", total));

        // Status spinner for current action
        let status_pb = mp.add(ProgressBar::new_spinner());
        status_pb.set_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());
        status_pb.enable_steady_tick(std::time::Duration::from_millis(100));
        status_pb.set_message("正在分类...");

        // Progress bars animate via steady ticks and updates above.

        let main_pb_c = main_pb.clone();
        let status_pb_c = status_pb.clone();

        let processed = stream::iter(items.into_iter())
            .map(move |item| {
                let main_pb = main_pb_c.clone();
                let status_pb = status_pb_c.clone();
                let this = self.clone();
                async move {
                    let title = item.title.clone();
                    let res = this.handle_item(item).await;
                    match &res {
                        Ok(action) => {
                            main_pb.inc(1);
                            let left = (total_u64.saturating_sub(main_pb.position())) as usize;
                            main_pb.set_message(format!("动作: {} | 剩余: {}", action, left));
                            status_pb.set_message(format!("{} · {}", action, truncate(&title, 60)));
                            match action {
                                ProcessAction::Kept => {
                                    let indicator = format!("{}", "[+]".green());
                                    let action_text = format!("{}", action.to_string().green());
                                    main_pb.println(format!(
                                        "{} 处理任务完成: {}, 结果: {}",
                                        indicator,
                                        truncate(&title, 60),
                                        action_text
                                    ));
                                }
                                ProcessAction::MarkedRead | ProcessAction::Labeled => {
                                    let indicator = format!("{}", "[-]".red());
                                    let action_text = format!("{}", action.to_string().red());
                                    main_pb.println(format!(
                                        "{} 处理任务完成: {}, 结果: {}",
                                        indicator,
                                        truncate(&title, 60),
                                        action_text
                                    ));
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            main_pb.inc(1);
                            let left = (total_u64.saturating_sub(main_pb.position())) as usize;
                            main_pb.set_message(format!("动作: 出错 | 剩余: {}", left));
                            status_pb.set_message(format!("出错 · {}", truncate(&title, 60)));
                            let indicator = format!("{}", "[!]".yellow());
                            let error_msg = format!("{}", e.to_string().yellow());
                            main_pb.println(format!("{} 处理任务出错: {}", indicator, error_msg));
                        }
                    }
                    res
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;

        // Aggregate results
        let mut counts = ActionCounts::default();
        for r in &processed {
            if let Ok(a) = r {
                match a {
                    ProcessAction::SkippedExists => counts.skipped_exists += 1,
                    ProcessAction::Kept => counts.kept += 1,
                    ProcessAction::MarkedRead => counts.marked_read += 1,
                    ProcessAction::Labeled => counts.labeled += 1,
                    ProcessAction::Deleted => counts.deleted += 1,
                    ProcessAction::WouldAct => counts.would_act += 1,
                }
            }
        }
        let reviewed = (counts.skipped_exists
            + counts.kept
            + counts.marked_read
            + counts.labeled
            + counts.deleted
            + counts.would_act) as usize;
        if let Ok(mut s) = self.state.last_run_status.lock() {
            *s = format!("reviewed_items={}/{}", reviewed, total);
        }
        main_pb.finish_with_message(format!(
            "完成 {}/{} | 保留={} 已读={} 已打标={} 已删除={} 已存在={} 预演={}",
            reviewed,
            total,
            counts.kept,
            counts.marked_read,
            counts.labeled,
            counts.deleted,
            counts.skipped_exists,
            counts.would_act,
        ));
        status_pb.finish_and_clear();
        info!(reviewed, total, "processor_run_once_done");
        Ok(())
    }

    #[instrument(name = "Reviewing content", skip(self, item), fields(item_id = item.id, title = %item.title))]
    async fn handle_item(&self, item: crate::freshrss::FeverItem) -> Result<ProcessAction> {
        let item_id = item.id.to_string();
        if self.db.has_reviewed(&item_id).await? {
            return Ok(ProcessAction::SkippedExists);
        }
        let text = item_text(&item);
        let hash = format!("{:x}", md5::compute(&text));
        let res = self.llm.classify(&text).await?;
        self.db
            .save_review(&item_id, &hash, res.is_ad, res.confidence, &res.reason)
            .await?;

        if res.is_ad && res.confidence >= self.cfg.openai.threshold {
            if self.cfg.dry_run {
                warn!(id = item.id, "dry_run_ad_detected");
                return Ok(ProcessAction::WouldAct);
            } else {
                if self.cfg.freshrss.delete_mode == "mark_read" {
                    self.fr.mark_item_read(item.id).await?;
                    return Ok(ProcessAction::MarkedRead);
                } else if self.cfg.freshrss.delete_mode == "label" {
                    if let Some(gr) = &self.gr {
                        gr.add_label(item.id, &self.cfg.freshrss.spam_label).await?;
                        self.fr.mark_item_read(item.id).await?;
                        return Ok(ProcessAction::Labeled);
                    }
                } else {
                    self.fr.delete_item_soft(item.id).await?;
                    return Ok(ProcessAction::Deleted);
                }
            }
        }
        Ok(ProcessAction::Kept)
    }
}

#[derive(Debug)]
enum ProcessAction {
    SkippedExists,
    Kept,
    MarkedRead,
    Labeled,
    Deleted,
    WouldAct,
}

impl Display for ProcessAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            ProcessAction::SkippedExists => write!(f, "跳过(已处理)"),
            ProcessAction::Kept => write!(f, "保留"),
            ProcessAction::MarkedRead => write!(f, "标记已读"),
            ProcessAction::Labeled => write!(f, "打标签"),
            ProcessAction::Deleted => write!(f, "删除"),
            ProcessAction::WouldAct => write!(f, "预演(不改动)"),
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
}

#[derive(Default)]
struct ActionCounts {
    skipped_exists: u64,
    kept: u64,
    marked_read: u64,
    labeled: u64,
    deleted: u64,
    would_act: u64,
}
