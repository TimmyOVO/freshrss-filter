use crate::config::OpenAiConfig;
use anyhow::{Result, anyhow};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{instrument, warn};

use std::fmt;

#[derive(Debug, Clone)]
pub struct OpenAiApiError {
    pub status: StatusCode,
    pub body: Value,
}

impl OpenAiApiError {
    pub fn new(status: StatusCode, body: Value) -> Self {
        Self { status, body }
    }
}

impl fmt::Display for OpenAiApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "openai_error: status={} body={}", self.status, self.body)
    }
}

impl std::error::Error for OpenAiApiError {}

#[derive(Clone)]
pub struct OpenAiClient {
    client: Client,
    cfg: OpenAiConfig,
}

#[derive(Debug, Deserialize, Clone)]
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
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_tokens: Option<u32>,
        }
        #[derive(Serialize)]
        struct RespFmt {
            r#type: &'static str,
        }
        #[derive(Serialize)]
        struct Message<'a> {
            role: &'static str,
            content: &'a str,
        }

        let body = ReqBody {
            model: &self.cfg.model,
            response_format: RespFmt {
                r#type: "json_object",
            },
            messages: vec![
                Message {
                    role: "system",
                    content: &self.cfg.system_prompt,
                },
                Message {
                    role: "user",
                    content: text,
                },
            ],
            temperature: self.cfg.temperature,
            max_tokens: self.cfg.max_tokens,
        };

        let url = format!(
            "{}/chat/completions",
            self.cfg.api_base.trim_end_matches('/')
        );
        let resp = self
            .client
            .post(url)
            .bearer_auth(&self.cfg.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let v: Value = resp.json().await?;

        if let Some(err) = v.get("error") {
            return Err(OpenAiApiError::new(status, err.clone()).into());
        }

        if !status.is_success() {
            return Err(OpenAiApiError::new(status, v).into());
        }

        // Extract content
        let raw = v["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("{}");
        let content = strip_code_fences(raw);
        parse_classifier_response(&content, raw)
    }
}

fn parse_classifier_response(content: &str, raw: &str) -> Result<ClassifierResponse> {
    match serde_json::from_str::<ClassifierResponse>(content) {
        Ok(parsed) => Ok(parsed),
        Err(primary_err) => {
            let responses: Vec<ClassifierResponse> =
                serde_json::from_str(content).map_err(|secondary_err| {
                    anyhow!(
                        "parse_classifier_response_failed: {} (array_parse_error: {}) raw={}",
                        primary_err,
                        secondary_err,
                        raw
                    )
                })?;

            if responses.is_empty() {
                return Err(anyhow!(
                    "parse_classifier_response_failed: empty array raw={}",
                    raw
                ));
            }

            warn!(
                error = %primary_err,
                count = responses.len(),
                "classifier_response_array_detected"
            );

            if let Some(best_ad) = responses
                .iter()
                .filter(|r| r.is_ad)
                .max_by(|a, b| a.confidence.total_cmp(&b.confidence))
            {
                return Ok(best_ad.clone());
            }

            responses
                .into_iter()
                .max_by(|a, b| a.confidence.total_cmp(&b.confidence))
                .ok_or_else(|| anyhow!("parse_classifier_response_failed: empty array raw={}", raw))
        }
    }
}

fn strip_code_fences(s: &str) -> String {
    let t = s.trim();
    if t.starts_with("```") {
        // remove first line fence and trailing fence
        let mut lines = t.lines();
        let _first = lines.next();
        let rest: String = lines.collect::<Vec<_>>().join("\n");
        let trimmed = rest.trim_end();
        if trimmed.ends_with("```") {
            return trimmed.trim_end_matches("```").trim().to_string();
        }
        return trimmed.to_string();
    }
    t.to_string()
}
