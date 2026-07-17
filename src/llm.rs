use std::time::Duration;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::config::AppConfig;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("尚未設定 OpenRouter API Key")]
    MissingKey,
    #[error("OpenRouter 請求失敗: {0}")]
    Request(#[from] reqwest::Error),
    #[error("OpenRouter 回應格式不符合預期: {0}")]
    Response(String),
    #[error("AI 回傳內容無效: {0}")]
    Content(String),
}

#[derive(Clone, Debug)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub raw: Value,
}

#[derive(Clone)]
pub struct LlmClient {
    config: AppConfig,
    client: reqwest::blocking::Client,
}

impl LlmClient {
    pub fn new(config: AppConfig) -> Result<Self, LlmError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(config.request_timeout.max(1)))
            .build()?;
        Ok(Self { config, client })
    }
    #[must_use]
    pub fn available(&self) -> bool {
        !self.config.openrouter_api_key.trim().is_empty()
    }
    pub fn complete(
        &self,
        system: &str,
        user: &str,
        model: Option<&str>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Result<LlmResponse, LlmError> {
        if !self.available() {
            return Err(LlmError::MissingKey);
        }
        let primary = model.unwrap_or(&self.config.primary_model);
        match self.call(primary, system, user, max_tokens, temperature) {
            Ok(response) => Ok(response),
            Err(primary_error)
                if !self.config.fallback_model.is_empty()
                    && self.config.fallback_model != primary =>
            {
                self.call(
                    &self.config.fallback_model,
                    system,
                    user,
                    max_tokens,
                    temperature,
                )
                .map_err(|fallback| {
                    LlmError::Response(format!(
                        "主模型失敗：{primary_error}；備援模型失敗：{fallback}"
                    ))
                })
            }
            Err(error) => Err(error),
        }
    }
    fn call(
        &self,
        model: &str,
        system: &str,
        user: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Result<LlmResponse, LlmError> {
        let url = format!(
            "{}/chat/completions",
            self.config.openrouter_base_url.trim_end_matches('/')
        );
        let payload = json!({"model": model, "messages": [{"role":"system","content":system},{"role":"user","content":user}], "max_tokens": max_tokens.unwrap_or(self.config.max_tokens), "temperature": temperature.unwrap_or(self.config.temperature)});
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.config.openrouter_api_key)
            .header(
                "HTTP-Referer",
                "https://github.com/luckyegg168/agent-manager",
            )
            .header("X-Title", "Agent Manager")
            .json(&payload)
            .send()?
            .error_for_status()?;
        let raw: Value = response.json()?;
        let content = raw
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .ok_or_else(|| LlmError::Response(raw.to_string()))?
            .to_owned();
        Ok(LlmResponse {
            content,
            model: model.to_owned(),
            raw,
        })
    }
    pub fn ping(&self) -> Result<String, LlmError> {
        let reply = self.complete(
            "You answer with a single word: ok",
            "ping",
            None,
            Some(10),
            Some(0.0),
        )?;
        Ok(format!("OK（模型：{}）", reply.model))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentDraft {
    pub description: String,
    pub role: String,
    pub abilities: String,
    pub body: String,
    #[serde(default)]
    pub model: String,
}

const GENERATE_PROMPT: &str = "你是 Agent Skills (SKILL.md) 規格的資深技術編輯。輸出純 JSON，欄位必須是 description、role、abilities、body。description 為 50–300 字且說明啟動時機；body 必含 ## 角色設定、## 核心能力、## 操作流程、## 輸入範例、## 輸出範例、## 邊緣案例處理、## 變更歷史。不要輸出 frontmatter 或程式碼圍欄。";
const EDIT_PROMPT: &str = "你是 SKILL.md 精準編輯器。只修改指定範圍，輸出修改後的完整範圍內容，不要說明、diff、frontmatter 或程式碼圍欄。description 須為 50–300 字。";

fn strip_fence(text: &str) -> String {
    let open = Regex::new(r"(?s)^```(?:json|markdown|md)?\s*\n").expect("valid regex");
    let close = Regex::new(r"(?s)\n```\s*$").expect("valid regex");
    close
        .replace(&open.replace(text.trim(), ""), "")
        .trim()
        .to_owned()
}

pub fn generate_agent_draft(
    client: &LlmClient,
    category: &str,
    name: &str,
    brief: &str,
    allowed_tools: &str,
) -> Result<AgentDraft, LlmError> {
    let user = format!(
        "類別：{category}\nAgent 名稱：{name}\nallowed-tools：{allowed_tools}\n一句話描述：{brief}\n請輸出 JSON。"
    );
    let reply = client.complete(GENERATE_PROMPT, &user, None, None, Some(0.6))?;
    let mut draft: AgentDraft = serde_json::from_str(&strip_fence(&reply.content))
        .map_err(|error| LlmError::Content(error.to_string()))?;
    if [
        &draft.description,
        &draft.role,
        &draft.abilities,
        &draft.body,
    ]
    .iter()
    .any(|field| field.trim().is_empty())
    {
        return Err(LlmError::Content(
            "缺少 description/role/abilities/body 欄位".into(),
        ));
    }
    draft.model = reply.model;
    Ok(draft)
}

pub fn edit_text_with_ai(
    client: &LlmClient,
    current: &str,
    instruction: &str,
    scope: &str,
) -> Result<(String, String), LlmError> {
    let user = format!(
        "--- 範圍 ---\n{scope}\n--- 修改指令 ---\n{instruction}\n--- 目前內容 ---\n{current}"
    );
    let reply = client.complete(EDIT_PROMPT, &user, None, None, Some(0.3))?;
    Ok((strip_fence(&reply.content), reply.model))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strips_json_and_markdown_fences() {
        assert_eq!(strip_fence("```json\n{\"x\":1}\n```"), "{\"x\":1}");
        assert_eq!(strip_fence("plain"), "plain");
    }
}
