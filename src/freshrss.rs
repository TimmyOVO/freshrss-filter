use anyhow::{anyhow, Result};
use regex::Regex;
use reqwest::{Client, Url};
use serde::{Deserialize, Deserializer, Serialize};
use tracing::instrument;
use crate::config::FreshRssConfig;

#[derive(Clone)]
pub struct FreshRssClient {
    pub client: Client,
    pub base: Url,
    pub fever_api_key: String,
    pub user_agent: String,
}

pub fn build_client(cfg: &FreshRssConfig) -> Result<FreshRssClient> {
    let base = Url::parse(&cfg.base_url)?;
    let client = Client::builder().user_agent(&cfg.user_agent).build()?;
    Ok(FreshRssClient {
        client,
        base,
        fever_api_key: cfg.fever_api_key.clone(),
        user_agent: cfg.user_agent.clone(),
    })
}

#[derive(Debug, Deserialize)]
pub struct FeverItemsResp {
    pub items: Vec<FeverItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FeverItem {
    #[serde(deserialize_with = "de_i64_from_str_or_int")]
    pub id: i64,
    pub title: String,
    pub url: Option<String>,
    pub author: Option<String>,
    pub html: Option<String>,
    pub content: Option<String>,
    #[serde(default, deserialize_with = "de_opt_i64_from_str_or_int")]
    pub created_on_time: Option<i64>,
}

fn de_i64_from_str_or_int<'de, D>(deserializer: D) -> std::result::Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    struct I64Visitor;
    impl<'de> serde::de::Visitor<'de> for I64Visitor {
        type Value = i64;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an integer or string integer")
        }
        fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E> { Ok(v) }
        fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
        where E: serde::de::Error { Ok(v as i64) }
        fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
        where E: serde::de::Error {
            v.parse::<i64>().map_err(E::custom)
        }
        fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
        where E: serde::de::Error {
            v.parse::<i64>().map_err(E::custom)
        }
    }
    deserializer.deserialize_any(I64Visitor)
}

fn de_opt_i64_from_str_or_int<'de, D>(deserializer: D) -> std::result::Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptI64Visitor;
    impl<'de> serde::de::Visitor<'de> for OptI64Visitor {
        type Value = Option<i64>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an optional integer or string integer")
        }
        fn visit_none<E>(self) -> std::result::Result<Self::Value, E> { Ok(None) }
        fn visit_unit<E>(self) -> std::result::Result<Self::Value, E> { Ok(None) }
        fn visit_some<D2>(self, d: D2) -> std::result::Result<Self::Value, D2::Error>
        where D2: Deserializer<'de> {
            de_i64_from_str_or_int(d).map(Some)
        }
    }
    deserializer.deserialize_option(OptI64Visitor)
}

impl FreshRssClient {
    fn fever_url_with(&self, extras: &str) -> Result<Url> {
        let mut u = self.base.join("/api/fever.php")?;
        let qs = if extras.is_empty() { "api".to_string() } else { format!("api&{}", extras) };
        u.set_query(Some(&qs));
        Ok(u)
    }

    #[instrument(name = "Fetching unread items", skip(self))]
    pub async fn get_unread_item_ids(&self) -> Result<Vec<i64>> {
        let ids_url = self.fever_url_with("unread_item_ids")?;
        let ids_resp = self.client
            .post(ids_url)
            .form(&[("api_key", &self.fever_api_key)])
            .send()
            .await?;
        if !ids_resp.status().is_success() {
            return Err(anyhow!("fever_unread_item_ids_error: {}", ids_resp.status()));
        }
        let ids_json = ids_resp.json::<serde_json::Value>().await?;
        let ids_str = ids_json.get("unread_item_ids").and_then(|v| v.as_str()).unwrap_or("");
        let ids: Vec<i64> = ids_str
            .split(',')
            .filter_map(|s| s.trim().parse::<i64>().ok())
            .collect();
        Ok(ids)
    }

    #[instrument(name = "Fetching item content", skip(self, ids), fields(chunk_size = ids.len()))]
    pub async fn get_items_by_ids(&self, ids: &[i64]) -> Result<Vec<FeverItem>> {
        if ids.is_empty() { return Ok(vec![]); }
        let with_ids = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
        let items_url = self.fever_url_with(&format!("items&with_ids={}", with_ids))?;
        let resp = self.client
            .post(items_url)
            .form(&[("api_key", &self.fever_api_key)])
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(anyhow!("fever_items_error: {}", resp.status()));
        }
        let v = resp.json::<serde_json::Value>().await?;
        let items = serde_json::from_value::<FeverItemsResp>(v.clone())
            .map(|r| r.items)
            .unwrap_or_else(|_| v.get("items").and_then(|i| serde_json::from_value(i.clone()).ok()).unwrap_or_default());
        Ok(items)
    }

    #[instrument(name = "Fetching unread items", skip(self))]
    pub async fn fetch_unread_items(&self) -> Result<Vec<FeverItem>> {
        let ids = self.get_unread_item_ids().await?;
        if ids.is_empty() { return Ok(vec![]); }
        let mut items: Vec<FeverItem> = Vec::new();
        for chunk in ids.chunks(50) {
            let mut got = self.get_items_by_ids(chunk).await?;
            items.append(&mut got);
        }
        Ok(items)
    }

    pub async fn mark_item_read(&self, item_id: i64) -> Result<()> {
        let url = self.fever_url_with(&format!("mark=item&as=read&id={}", item_id))?;
        let resp = self.client
            .post(url)
            .form(&[("api_key", &self.fever_api_key)])
            .send()
            .await?;
        if !resp.status().is_success() { return Err(anyhow!("mark_read_error: {}", resp.status())); }
        Ok(())
    }

    pub async fn delete_item_soft(&self, item_id: i64) -> Result<()> {
        // FreshRSS Fever API has mark as read; real deletion requires admin API.
        self.mark_item_read(item_id).await
    }
}

pub fn item_text(item: &FeverItem) -> String {
    let mut text = String::new();
    text.push_str(&item.title);
    if let Some(a) = &item.author { text.push_str(&format!("\nby {}", a)); }
    if let Some(c) = &item.content { text.push_str(&format!("\n{}", c)); }
    if let Some(h) = &item.html { text.push_str(&format!("\n{}", strip_html(h))); }
    text
}

fn strip_html(html: &str) -> String {
    // basic strip for classifier context
    let re = Regex::new(r"<[^>]+>").unwrap();
    re.replace_all(html, " ").to_string()
}
