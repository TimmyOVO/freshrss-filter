use anyhow::{anyhow, Result};
use tracing::instrument;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::config::OpenAiConfig;

#[derive(Clone)]
pub struct OpenAiClient {
    client: Client,
    cfg: OpenAiConfig,
}

#[derive(Debug, Deserialize)]
pub struct ClassifierResponse {
    pub is_ad: bool,
    pub confidence: f32,
    pub reason: String,
}

impl OpenAiClient {
    pub fn new(cfg: OpenAiConfig) -> Self {
        let client = Client::builder().build().unwrap();
        Self { client, cfg }
    }

    #[instrument(name = "Reviewing content", skip(self, text))]
    pub async fn classify(&self, text: &str) -> Result<ClassifierResponse> {
        #[derive(Serialize)]
        struct ReqBody<'a> {
            model: &'a str,
            response_format: RespFmt,
            messages: Vec<Message<'a>>,
            #[serde(skip_serializing_if = "Option::is_none")] temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")] max_tokens: Option<u32>,
        }
        #[derive(Serialize)]
        struct RespFmt { r#type: &'static str }
        #[derive(Serialize)]
        struct Message<'a> { role: &'static str, content: &'a str }

        let body = ReqBody {
            model: &self.cfg.model,
            response_format: RespFmt { r#type: "json_object" },
            messages: vec![
                Message { role: "system", content: &self.cfg.system_prompt },
                Message { role: "user", content: text },
            ],
            temperature: self.cfg.temperature,
            max_tokens: self.cfg.max_tokens,
        };

        let url = format!("{}/chat/completions", self.cfg.api_base.trim_end_matches('/'));
        let resp = self.client
            .post(url)
            .bearer_auth(&self.cfg.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let v: serde_json::Value = resp.json().await?;
        if let Some(err) = v.get("error") {
            return Err(anyhow!("openai_error: status={} body={}", status, err));
        }

        // Extract content
        let raw = v["choices"][0]["message"]["content"].as_str().unwrap_or("{}");
        let content = strip_code_fences(raw);
        let parsed: ClassifierResponse = serde_json::from_str(&content).map_err(|e| {
            anyhow!("parse_classifier_response_failed: {} raw={}", e, raw)
        })?;
        Ok(parsed)
    }
}

fn strip_code_fences(s: &str) -> String {
    let t = s.trim();
    if t.starts_with("```") {
        // remove first line fence and trailing fence
        let mut lines = t.lines();
        let first = lines.next();
        let rest: String = lines.collect::<Vec<_>>().join("\n");
        let trimmed = rest.trim_end();
        if trimmed.ends_with("```") {
            return trimmed.trim_end_matches("```").trim().to_string();
        }
        return trimmed.to_string();
    }
    t.to_string()
}
