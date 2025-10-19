use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub openai: OpenAiConfig,
    pub freshrss: FreshRssConfig,
    pub scheduler: SchedulerConfig,
    pub database: DatabaseConfig,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    pub api_key: String,
    #[serde(default = "default_api_base")]
    pub api_base: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
    #[serde(default = "default_threshold")]
    pub threshold: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshRssConfig {
    pub base_url: String,
    /// Fever API key (MD5 of username:password or token from FreshRSS settings)
    pub fever_api_key: String,
    #[serde(default = "default_user_agent")]
    pub user_agent: String,
    /// Action when classifying as ad: mark_read | delete (delete currently marks read)
    #[serde(default = "default_delete_mode")]
    pub delete_mode: String,
    /// Optional GReader credentials for labeling
    #[serde(default)]
    pub greader_username: Option<String>,
    #[serde(default)]
    pub greader_password: Option<String>,
    /// Label name to tag ads (auto-created by GReader API)
    #[serde(default = "default_spam_label")]
    pub spam_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Cron string, e.g. "0 */10 * * * *" (every 10 minutes)
    #[serde(default = "default_cron")]
    pub cron: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
}

fn default_api_base() -> String {
    "https://api.openai.com/v1".into()
}
fn default_model() -> String {
    "gpt-4o-mini".into()
}
fn default_system_prompt() -> String {
    "You are a strict classifier. Decide if an RSS item is an advertisement or sponsored content. Reply JSON: {\"is_ad\": boolean, \"confidence\": 0..1, \"reason\": string}.".into()
}
fn default_threshold() -> f32 {
    0.5
}
fn default_user_agent() -> String {
    "freshrss-filter/0.1".into()
}
fn default_delete_mode() -> String {
    "mark_read".into()
}
fn default_spam_label() -> String {
    "Ads".into()
}
fn default_cron() -> String {
    "0 */10 * * * *".into()
}
fn default_db_path() -> String {
    "freshrss-filter.db".into()
}

pub async fn load(custom_path: Option<&Path>) -> Result<Config> {
    use config as cfg;
    let mut builder = cfg::Config::builder();

    if let Some(p) = custom_path {
        builder = builder.add_source(cfg::File::from(p));
    } else {
        builder = builder.add_source(cfg::File::with_name("config").required(false));
    }

    builder = builder.add_source(cfg::Environment::with_prefix("FRF").separator("__"));

    let settings = builder.build()?;
    let mut cfg: Config = settings.try_deserialize()?;

    // Default fill if missing
    if cfg.openai.api_base.is_empty() {
        cfg.openai.api_base = default_api_base();
    }
    if cfg.openai.model.is_empty() {
        cfg.openai.model = default_model();
    }
    if cfg.freshrss.user_agent.is_empty() {
        cfg.freshrss.user_agent = default_user_agent();
    }
    if cfg.scheduler.cron.is_empty() {
        cfg.scheduler.cron = default_cron();
    }
    if cfg.database.path.is_empty() {
        cfg.database.path = default_db_path();
    }

    Ok(cfg)
}

impl Config {
    pub fn with_overrides(mut self, dry_run: bool) -> Self {
        if dry_run {
            self.dry_run = true;
        }
        self
    }
}
