use std::{fs::OpenOptions, io::Write};

use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};

use crate::{
    AppPaths,
    config::AppConfig,
    llm::LlmClient,
    model::AgentSkill,
    storage,
    validator::{self, Issue, Severity},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScanResult {
    pub skill_path: String,
    pub issues: Vec<Issue>,
}

#[derive(Clone, Debug, Default)]
pub struct ScanReport {
    pub results: Vec<ScanResult>,
    pub diagnostics: Vec<storage::LoadDiagnostic>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EvolutionRecord {
    pub timestamp: String,
    pub skill_path: String,
    pub action: String,
    pub mode_reason: String,
    pub model: String,
    pub fixed_issues: Vec<Issue>,
    pub remaining_issues: Vec<Issue>,
    pub error: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleDecision {
    pub should_evolve: bool,
    pub mode: &'static str,
    pub reason: String,
}

#[must_use]
pub fn decide(issues: &[Issue], config: &AppConfig) -> RuleDecision {
    if issues.is_empty() {
        return RuleDecision {
            should_evolve: false,
            mode: "skip",
            reason: "no issues".into(),
        };
    }
    let highest = issues
        .iter()
        .map(|issue| issue.severity)
        .max()
        .unwrap_or(Severity::Low);
    let threshold = match config.evolution_min_severity.to_ascii_uppercase().as_str() {
        "CRITICAL" => Severity::Critical,
        "MEDIUM" => Severity::Medium,
        "LOW" => Severity::Low,
        _ => Severity::High,
    };
    if highest < threshold {
        return RuleDecision {
            should_evolve: true,
            mode: "suggest",
            reason: format!(
                "max severity below threshold ({})",
                config.evolution_min_severity
            ),
        };
    }
    if config.evolution_use_api && !config.openrouter_api_key.trim().is_empty() {
        return RuleDecision {
            should_evolve: true,
            mode: "api",
            reason: "api mode enabled".into(),
        };
    }
    if config.evolution_auto_apply {
        return RuleDecision {
            should_evolve: true,
            mode: "skeleton",
            reason: "auto-apply skeleton fix".into(),
        };
    }
    RuleDecision {
        should_evolve: true,
        mode: "suggest",
        reason: "auto-apply disabled".into(),
    }
}

pub fn scan_all_checked(paths: &AppPaths) -> Result<ScanReport, storage::StorageError> {
    let loaded = storage::list_skills_checked(paths, None)?;
    let results = loaded
        .skills
        .into_iter()
        .filter_map(|skill| {
            let issues = validator::validate(&skill);
            (!issues.is_empty()).then(|| ScanResult {
                skill_path: skill.path.to_string_lossy().into_owned(),
                issues,
            })
        })
        .collect();
    Ok(ScanReport {
        results,
        diagnostics: loaded.diagnostics,
    })
}

fn set_default(mapping: &mut Mapping, key: &str, value: Value) {
    mapping.entry(Value::String(key.into())).or_insert(value);
}

pub fn auto_fix_skeleton(
    skill: &mut AgentSkill,
    paths: &AppPaths,
    persist: bool,
) -> anyhow::Result<(Vec<Issue>, Vec<Issue>)> {
    let before = validator::validate(skill);
    let slug = skill.slug();
    set_default(&mut skill.frontmatter, "name", Value::String(slug.clone()));
    set_default(
        &mut skill.frontmatter,
        "description",
        Value::String(format!(
            "{slug} Agent — 請補充描述（50–300 字），包含啟動時機說明。"
        )),
    );
    set_default(
        &mut skill.frontmatter,
        "license",
        Value::String("MIT".into()),
    );
    set_default(
        &mut skill.frontmatter,
        "allowed-tools",
        Value::String("Read Write".into()),
    );
    set_default(
        &mut skill.frontmatter,
        "compatibility",
        Value::String(
            "Claude Code、VS Code Copilot、GitHub Copilot 及所有相容 Agent Skills 的平台".into(),
        ),
    );
    if skill.metadata_string("author").is_empty() {
        skill.set_metadata_string("author", "agent-manager");
    }
    if skill.metadata_string("version").is_empty() {
        skill.set_metadata_string("version", "1.0.0");
    }
    if skill.metadata_string("category").is_empty() {
        let category = skill
            .path
            .parent()
            .and_then(|path| path.parent())
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        skill.set_metadata_string("category", category);
    }
    if skill.metadata_string("language").is_empty() {
        skill.set_metadata_string("language", "zh-TW");
    }
    let sections = [
        (
            "## 角色設定",
            "你是專業的 Agent，請依據使用者需求提供協助。",
        ),
        ("## 核心能力", "- 核心能力 1\n- 核心能力 2\n- 核心能力 3"),
        ("## 操作流程", "1. 接收輸入\n2. 分析需求\n3. 回應建議"),
        ("## 輸入範例", "```\n請描述您的需求...\n```"),
        ("## 輸出範例", "```\n（Agent 回覆內容）\n```"),
        ("## 邊緣案例處理", "- 輸入不清：要求補充\n- 超出範圍：轉介"),
        (
            "## 變更歷史",
            "| 版本 | 日期 | 內容 | 影響範圍 |\n|------|------|------|----------|\n| v1.0.0 | {{date}} | 初始建立 | — |",
        ),
    ];
    for (heading, template) in sections {
        if !skill.body.contains(heading) {
            skill.body.push_str(&format!(
                "\n\n{heading}\n{}\n",
                template.replace("{{date}}", &Local::now().format("%Y-%m-%d").to_string())
            ));
        }
    }
    if persist {
        storage::save_skill(skill, paths, true)?;
    }
    let after = validator::validate(skill);
    let fixed = before
        .into_iter()
        .filter(|issue| !after.contains(issue))
        .collect();
    Ok((fixed, after))
}

const API_SYSTEM_PROMPT: &str = "你是 Agent Skills (SKILL.md) 規格的資深技術編輯。輸出完整 SKILL.md；保留既有內容，只補齊問題；不要解說或程式碼圍欄。frontmatter 必含 name、description、license、metadata、compatibility、allowed-tools，body 必含角色設定、核心能力、操作流程。";

trait SkillCompleter {
    fn complete_skill(&self, prompt: &str) -> anyhow::Result<(String, String)>;
}

impl SkillCompleter for LlmClient {
    fn complete_skill(&self, prompt: &str) -> anyhow::Result<(String, String)> {
        let reply = self.complete(API_SYSTEM_PROMPT, prompt, None, None, None)?;
        Ok((reply.content, reply.model))
    }
}

fn api_fix_with<C: SkillCompleter>(
    skill: &mut AgentSkill,
    issues: &[Issue],
    paths: &AppPaths,
    config: &AppConfig,
    client: &C,
) -> anyhow::Result<(Vec<Issue>, Vec<Issue>, String)> {
    let before = validator::validate(skill);
    let prompt = format!(
        "目前 SKILL.md：\n{}\n問題：\n{}",
        storage::serialize_skill(skill)?,
        issues
            .iter()
            .map(|issue| format!(
                "- [{}] {}: {}",
                issue.severity.label(),
                issue.field,
                issue.message
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let (content, model) = client.complete_skill(&prompt)?;
    let cleaned = content
        .trim()
        .trim_start_matches("```markdown")
        .trim_start_matches("```md")
        .trim_end_matches("```")
        .trim();
    let candidate = storage::parse_skill(skill.path.clone(), cleaned)?;
    anyhow::ensure!(
        !candidate.frontmatter.is_empty(),
        "LLM 回傳不含有效 frontmatter"
    );
    let after = validator::validate(&candidate);
    if config.evolution_require_validation && !after.is_empty() {
        anyhow::bail!("API 修復後仍有 {} 項驗證問題，已拒絕寫入", after.len());
    }
    if !config.evolution_dry_run {
        storage::save_skill(&candidate, paths, true)?;
    }
    *skill = candidate;
    let fixed = before
        .into_iter()
        .filter(|issue| !after.contains(issue))
        .collect();
    Ok((fixed, after, model))
}

fn fallback_from_disk(
    attempted: &AgentSkill,
    paths: &AppPaths,
    persist: bool,
) -> anyhow::Result<AgentSkill> {
    let mut original = storage::load_skill(&attempted.path)?;
    auto_fix_skeleton(&mut original, paths, persist)?;
    Ok(original)
}

pub fn evolve_once(paths: &AppPaths, config: &AppConfig) -> anyhow::Result<Vec<EvolutionRecord>> {
    let client = if config.evolution_use_api {
        Some(LlmClient::new(config.clone())?)
    } else {
        None
    };
    let mut records = Vec::new();
    let loaded = storage::list_skills_checked(paths, None)?;
    anyhow::ensure!(
        loaded.diagnostics.is_empty(),
        "進化前載入失敗：{} 個檔案無法安全解析",
        loaded.diagnostics.len()
    );
    for mut skill in loaded.skills {
        if records.len() >= config.evolution_max_agents_per_run.max(1) {
            break;
        }
        let issues = validator::validate(&skill);
        let decision = decide(&issues, config);
        if !decision.should_evolve {
            continue;
        }
        let timestamp = Local::now().to_rfc3339();
        let path = skill.path.to_string_lossy().into_owned();
        let mut record = EvolutionRecord {
            timestamp,
            skill_path: path,
            action: decision.mode.into(),
            mode_reason: decision.reason,
            model: String::new(),
            fixed_issues: Vec::new(),
            remaining_issues: issues.clone(),
            error: String::new(),
        };
        match decision.mode {
            "skeleton" => {
                let (fixed, remaining) =
                    auto_fix_skeleton(&mut skill, paths, !config.evolution_dry_run)?;
                record.fixed_issues = fixed;
                record.remaining_issues = remaining;
            }
            "api" => match api_fix_with(
                &mut skill,
                &issues,
                paths,
                config,
                client.as_ref().expect("client configured"),
            ) {
                Ok((fixed, remaining, model)) => {
                    record.fixed_issues = fixed;
                    record.remaining_issues = remaining;
                    record.model = model;
                }
                Err(error) => {
                    record.action = "skeleton".into();
                    record.error = error.to_string();
                    record.mode_reason = format!("api failed, fell back: {error}");
                    skill = fallback_from_disk(&skill, paths, !config.evolution_dry_run)?;
                    record.remaining_issues = validator::validate(&skill);
                    record.fixed_issues = issues
                        .iter()
                        .filter(|issue| !record.remaining_issues.contains(issue))
                        .cloned()
                        .collect();
                }
            },
            _ => {}
        }
        records.push(record);
    }
    if !config.evolution_dry_run {
        append_log(paths, &records)?;
    }
    Ok(records)
}

pub fn append_log(paths: &AppPaths, records: &[EvolutionRecord]) -> anyhow::Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(&paths.agents)?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.evolution_log)?;
    for record in records {
        writeln!(file, "{}", serde_json::to_string(record)?)?;
    }
    Ok(())
}

pub fn read_log(paths: &AppPaths, limit: usize) -> Vec<serde_json::Value> {
    std::fs::read_to_string(&paths.evolution_log)
        .ok()
        .map(|text| {
            text.lines()
                .rev()
                .take(limit)
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    #[test]
    fn evolution_decision_matches_priority() {
        let issue = Issue {
            severity: Severity::High,
            field: "body".into(),
            message: "missing".into(),
            suggestion: String::new(),
        };
        let mut config = AppConfig::default();
        assert_eq!(decide(&[], &config).mode, "skip");
        assert_eq!(
            decide(std::slice::from_ref(&issue), &config).mode,
            "skeleton"
        );
        config.evolution_use_api = true;
        config.openrouter_api_key = "secret".into();
        assert_eq!(decide(&[issue], &config).mode, "api");
    }

    struct FakeCompleter {
        reply: String,
    }

    impl SkillCompleter for FakeCompleter {
        fn complete_skill(&self, _prompt: &str) -> anyhow::Result<(String, String)> {
            Ok((self.reply.clone(), "fake/model".into()))
        }
    }

    fn original_skill(paths: &AppPaths) -> AgentSkill {
        storage::parse_skill(
            paths.agents.join("01-Test/alpha/SKILL.md"),
            "---\nname: original\ndescription: This description is deliberately long enough for validation.\nallowed-tools: Read\nmetadata:\n  version: 1.0.0\n  category: 01-Test\n---\noriginal body\n",
        )
        .unwrap()
    }

    #[test]
    fn invalid_required_validation_api_rewrite_never_writes() {
        let temp = tempdir().unwrap();
        let paths = AppPaths::from_root(temp.path());
        let mut skill = original_skill(&paths);
        storage::save_skill(&skill, &paths, false).unwrap();
        let original_text = std::fs::read_to_string(&skill.path).unwrap();
        let config = AppConfig {
            evolution_require_validation: true,
            ..AppConfig::default()
        };
        let fake = FakeCompleter {
            reply: "---\nname: mutated\ndescription: short\nallowed-tools: Read\nmetadata:\n  version: 1\n  category: Test\n---\ninvalid body\n".into(),
        };

        let issues = validator::validate(&skill);
        let result = api_fix_with(&mut skill, &issues, &paths, &config, &fake);

        assert!(result.is_err());
        assert_eq!(skill.name(), "original");
        assert_eq!(std::fs::read_to_string(&skill.path).unwrap(), original_text);
    }

    #[test]
    fn fallback_reloads_disk_original_not_mutated_candidate() {
        let temp = tempdir().unwrap();
        let paths = AppPaths::from_root(temp.path());
        let original = original_skill(&paths);
        storage::save_skill(&original, &paths, false).unwrap();
        let mut mutated = original.clone();
        mutated.set_string("name", "llm-mutated");
        mutated.body = "llm mutated body".into();

        let fallback = fallback_from_disk(&mutated, &paths, true).unwrap();

        assert_eq!(fallback.name(), "original");
        assert!(fallback.body.contains("original body"));
        assert!(!fallback.body.contains("llm mutated body"));
        assert_eq!(
            storage::load_skill(&original.path).unwrap().name(),
            "original"
        );
    }

    #[test]
    fn dry_run_does_not_change_agent_or_create_evolution_log() {
        let temp = tempdir().unwrap();
        let paths = AppPaths::from_root(temp.path());
        let original = original_skill(&paths);
        storage::save_skill(&original, &paths, false).unwrap();
        let before = std::fs::read_to_string(&original.path).unwrap();
        let config = AppConfig {
            evolution_dry_run: true,
            evolution_max_agents_per_run: 1,
            ..AppConfig::default()
        };

        let records = evolve_once(&paths, &config).unwrap();

        assert!(!records.is_empty());
        assert_eq!(std::fs::read_to_string(&original.path).unwrap(), before);
        assert!(!paths.evolution_log.exists());
    }
}
