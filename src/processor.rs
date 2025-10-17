use anyhow::Result;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};
use tracing::instrument;
use futures::stream::{self, StreamExt};

use crate::{db::Database, freshrss::{FreshRssClient, item_text}, greader::GReaderClient, openai_client::OpenAiClient, config::Config};

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
    pub fn new(db: Database, fr: FreshRssClient, gr: Option<GReaderClient>, llm: OpenAiClient, cfg: Config, state: ProcessorState) -> Self {
        Self { db, fr, gr, llm, cfg, state }
    }
    
    #[instrument(skip(self), name = "Working")]
    #[instrument(name = "Working", skip(self))]
    pub async fn run_once(&self) -> Result<()> {
        let items = self.fr.fetch_unread_items().await?;
        let total = items.len();

        let concurrency = 5usize;
        let processed = stream::iter(items.into_iter())
            .map(|item| self.handle_item(item))
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;

        let reviewed = processed.into_iter().filter(|r| r.is_ok()).count();
        if let Ok(mut s) = self.state.last_run_status.lock() {
            *s = format!("reviewed_items={}/{}", reviewed, total);
        }
        info!(reviewed, total, "processor_run_once_done");
        Ok(())
    }

    #[instrument(name = "Reviewing content", skip(self, item), fields(item_id = item.id, title = %item.title))]
    async fn handle_item(&self, item: crate::freshrss::FeverItem) -> Result<()> {
        let item_id = item.id.to_string();
        if self.db.has_reviewed(&item_id).await? { return Ok(()); }
        let text = item_text(&item);
        let hash = format!("{:x}", md5::compute(&text));
        let res = self.llm.classify(&text).await?;
        self.db.save_review(&item_id, &hash, res.is_ad, res.confidence, &res.reason).await?;

        if res.is_ad && res.confidence >= self.cfg.openai.threshold {
            if self.cfg.dry_run {
                warn!(id = item.id, "dry_run_ad_detected");
            } else {
                if self.cfg.freshrss.delete_mode == "mark_read" {
                    self.fr.mark_item_read(item.id).await?;
                } else if self.cfg.freshrss.delete_mode == "label" {
                    if let Some(gr) = &self.gr {
                        gr.add_label(item.id, &self.cfg.freshrss.spam_label).await?;
                        self.fr.mark_item_read(item.id).await?;
                    }
                } else {
                    self.fr.delete_item_soft(item.id).await?;
                }
            }
        }
        Ok(())
    }
}
