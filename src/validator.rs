use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::model::AgentSkill;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub const ALL: [Self; 4] = [Self::Critical, Self::High, Self::Medium, Self::Low];
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Issue {
    pub severity: Severity,
    pub field: String,
    pub message: String,
    pub suggestion: String,
}

impl Issue {
    fn new(severity: Severity, field: &str, message: impl Into<String>, suggestion: &str) -> Self {
        Self {
            severity,
            field: field.into(),
            message: message.into(),
            suggestion: suggestion.into(),
        }
    }
}

pub const REQUIRED_SECTIONS: &[&str] = &["## 角色設定", "## 核心能力", "## 操作流程"];
pub const RECOMMENDED_SECTIONS: &[&str] = &[
    "## 輸入範例",
    "## 輸出範例",
    "## 邊緣案例處理",
    "## 變更歷史",
];

#[must_use]
pub fn validate(skill: &AgentSkill) -> Vec<Issue> {
    let mut issues = Vec::new();
    if skill.path.file_name().is_none_or(|name| name != "SKILL.md") {
        issues.push(Issue::new(
            Severity::Critical,
            "path",
            "檔名應為 SKILL.md",
            "將檔案重新命名為 SKILL.md",
        ));
    }
    for key in ["name", "description", "allowed-tools"] {
        let missing = skill
            .frontmatter
            .get(Value::String(key.into()))
            .is_none_or(|value| match value {
                Value::Null => true,
                Value::String(text) => text.trim().is_empty(),
                Value::Sequence(items) => items.is_empty(),
                _ => false,
            });
        if missing {
            issues.push(Issue::new(
                Severity::Critical,
                key,
                format!("缺少必要 frontmatter 欄位：{key}"),
                &format!("請於 YAML 新增：{key}: ..."),
            ));
        }
    }
    match skill.frontmatter.get(Value::String("metadata".into())) {
        Some(Value::Mapping(metadata)) => {
            for key in ["version", "category"] {
                if metadata
                    .get(Value::String(key.into()))
                    .is_none_or(|value| value.as_str().is_none_or(str::is_empty))
                {
                    issues.push(Issue::new(
                        Severity::High,
                        &format!("metadata.{key}"),
                        format!("缺少 metadata.{key}"),
                        &format!("請於 metadata 區塊新增 {key}"),
                    ));
                }
            }
        }
        _ => issues.push(Issue::new(
            Severity::High,
            "metadata",
            "metadata 應為包含 version、category 的字典",
            "新增 metadata 區塊",
        )),
    }
    let length = skill.description().chars().count();
    if length > 0 && length < 30 {
        issues.push(Issue::new(
            Severity::High,
            "description",
            format!("description 過短（{length} 字），建議 50–300 字"),
            "補充使用時機與核心價值",
        ));
    } else if length > 500 {
        issues.push(Issue::new(
            Severity::Medium,
            "description",
            format!("description 過長（{length} 字），建議 50–300 字"),
            "精簡描述",
        ));
    }
    for section in REQUIRED_SECTIONS {
        if !skill.body.contains(section) {
            issues.push(Issue::new(
                Severity::High,
                "body",
                format!("缺少必要章節：{section}"),
                &format!("新增 Markdown 章節：{section}"),
            ));
        }
    }
    for section in RECOMMENDED_SECTIONS {
        if !skill.body.contains(section) {
            issues.push(Issue::new(
                Severity::Low,
                "body",
                format!("建議補齊章節：{section}"),
                "",
            ));
        }
    }
    if let Some(Value::String(tools)) = skill.frontmatter.get(Value::String("allowed-tools".into()))
        && (tools.contains('*') || tools.to_ascii_lowercase().contains("all"))
    {
        issues.push(Issue::new(
            Severity::Medium,
            "allowed-tools",
            "allowed-tools 過度授權，建議明列工具名稱",
            "套用最小權限",
        ));
    }
    issues
}

#[must_use]
pub fn summary(issues: &[Issue]) -> BTreeMap<&'static str, usize> {
    Severity::ALL
        .into_iter()
        .map(|severity| {
            (
                severity.label(),
                issues
                    .iter()
                    .filter(|issue| issue.severity == severity)
                    .count(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::parse_skill;

    #[test]
    fn reports_severity_and_summary() {
        let skill = parse_skill("wrong.md", "plain body").unwrap();
        let issues = validate(&skill);
        let counts = summary(&issues);
        assert!(counts["CRITICAL"] >= 4);
        assert!(counts["HIGH"] >= 4);
        assert_eq!(counts.values().sum::<usize>(), issues.len());
    }
}
