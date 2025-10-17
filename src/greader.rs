use anyhow::{anyhow, Result};
use reqwest::{Client, Url};
use crate::config::FreshRssConfig;

#[derive(Clone)]
pub struct GReaderClient {
    client: Client,
    base: Url,
    username: String,
    password: String,
    user_agent: String,
}

pub fn build_client(cfg: &FreshRssConfig, username: String, password: String) -> Result<GReaderClient> {
    let base = Url::parse(&cfg.base_url)?;
    let client = Client::builder().user_agent(&cfg.user_agent).build()?;
    Ok(GReaderClient { client, base, username, password, user_agent: cfg.user_agent.clone() })
}

impl GReaderClient {
    pub async fn add_label(&self, item_id: i64, label: &str) -> Result<()> {
        let url = self.base.join("/api/greader.php/reader/api/0/edit-tag")?;
        let tag = format!("user/-/label/{}", label);
        let resp = self.client
            .post(url)
            .basic_auth(&self.username, Some(&self.password))
            .form(&[("i", item_id.to_string()), ("a", tag)])
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("greader_edit_tag_error: {}", resp.status()));
        }
        Ok(())
    }
}

