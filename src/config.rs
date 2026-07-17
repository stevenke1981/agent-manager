use std::{fs, io::Write, path::Path};

use atomicwrites::{AllowOverwrite, AtomicFile};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AppConfig {
    pub openrouter_api_key: String,
    pub openrouter_base_url: String,
    pub primary_model: String,
    pub fallback_model: String,
    pub request_timeout: u64,
    pub max_tokens: u32,
    pub temperature: f32,
    pub evolution_use_api: bool,
    pub evolution_min_severity: String,
    pub evolution_auto_apply: bool,
    pub evolution_require_validation: bool,
    pub evolution_max_agents_per_run: usize,
    pub evolution_dry_run: bool,
    pub custom_models: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            openrouter_api_key: String::new(),
            openrouter_base_url: "https://openrouter.ai/api/v1".into(),
            primary_model: "anthropic/claude-haiku-4.5".into(),
            fallback_model: "openai/gpt-5-mini".into(),
            request_timeout: 60,
            max_tokens: 2048,
            temperature: 0.4,
            evolution_use_api: false,
            evolution_min_severity: "HIGH".into(),
            evolution_auto_apply: true,
            evolution_require_validation: true,
            evolution_max_agents_per_run: 20,
            evolution_dry_run: false,
            custom_models: Vec::new(),
        }
    }
}

pub const DEFAULT_MODELS: &[&str] = &[
    "anthropic/claude-opus-4.7",
    "anthropic/claude-sonnet-4.6",
    "anthropic/claude-haiku-4.5",
    "openai/gpt-5",
    "openai/gpt-5-mini",
    "google/gemini-2.5-pro",
    "google/gemini-2.5-flash",
    "meta-llama/llama-4-maverick",
    "qwen/qwen3-max",
    "deepseek/deepseek-v3.1",
    "x-ai/grok-4",
    "mistralai/mistral-large-2",
];

pub fn load_config(path: &Path) -> AppConfig {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_config(path: &Path, config: &AppConfig) -> anyhow::Result<()> {
    let text = serde_json::to_string_pretty(config)?;
    AtomicFile::new(path, AllowOverwrite).write(|file| file.write_all(text.as_bytes()))?;
    Ok(())
}

#[must_use]
pub fn all_models(config: &AppConfig) -> Vec<String> {
    let mut models = Vec::new();
    for model in DEFAULT_MODELS
        .iter()
        .copied()
        .chain(config.custom_models.iter().map(String::as_str))
    {
        if !model.is_empty() && !models.iter().any(|current| current == model) {
            models.push(model.to_owned());
        }
    }
    models
}
