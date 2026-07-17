use chrono::Local;
use serde_yaml::{Mapping, Value};
use unicode_normalization::UnicodeNormalization;

use crate::{
    AppPaths,
    model::AgentSkill,
    storage::{load_skill, new_skill_path},
};

pub const BLANK_TEMPLATE_BODY: &str = r#"# {{name}} Agent

## 角色設定
{{role}}

## 核心能力
{{abilities}}

## 操作流程
1. 接收使用者輸入
2. 分析需求並回應
3. 提供具體可執行建議

## 重要聲明
本 Agent 建議僅供參考，實務應用請依專業判斷。

## 輸入範例
```
{{input_example}}
```

## 輸出範例
```
{{output_example}}
```

## 邊緣案例處理
- 輸入不清楚：要求使用者補充
- 超出專業範圍：轉介至更合適的 Agent

## 變更歷史
| 版本 | 日期 | 內容 | 影響範圍 |
|------|------|------|----------|
| v1.0.0 | {{date}} | 初始建立 | — |
"#;

#[must_use]
pub fn slugify(name: &str) -> String {
    let normalized: String = name.trim().to_lowercase().nfkc().collect();
    let mut slug = String::new();
    let mut dash = false;
    for character in normalized.chars() {
        if character.is_whitespace() || matches!(character, '/' | '\\') {
            if !slug.is_empty() {
                dash = true;
            }
        } else if character.is_alphanumeric()
            || character == '_'
            || character == '-'
            || ('\u{4e00}'..='\u{9fff}').contains(&character)
        {
            if dash && !slug.ends_with('-') {
                slug.push('-');
            }
            dash = false;
            slug.push(character);
        }
    }
    slug.trim_matches('-').to_owned()
}

#[derive(Clone, Debug)]
pub struct TemplateInput {
    pub category: String,
    pub name: String,
    pub description: String,
    pub role: String,
    pub abilities: String,
    pub allowed_tools: String,
    pub input_example: String,
    pub output_example: String,
}

pub fn find_best_template(
    paths: &AppPaths,
    category: &str,
    keyword: &str,
) -> Option<std::path::PathBuf> {
    let mut candidates: Vec<_> = walkdir::WalkDir::new(paths.agents.join(category))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == "SKILL.md")
        .map(|entry| entry.into_path())
        .collect();
    candidates.sort();
    let needle = keyword.to_lowercase();
    if !needle.is_empty()
        && let Some(found) = candidates.iter().find(|path| {
            path.parent()
                .is_some_and(|parent| parent.to_string_lossy().to_lowercase().contains(&needle))
        })
    {
        return Some(found.clone());
    }
    candidates.into_iter().next()
}

pub fn render_from_template(
    paths: &AppPaths,
    input: &TemplateInput,
    template: Option<&std::path::Path>,
) -> anyhow::Result<AgentSkill> {
    let mut base = if let Some(path) = template {
        load_skill(path)?
    } else {
        AgentSkill::new(
            std::path::PathBuf::new(),
            Mapping::new(),
            BLANK_TEMPLATE_BODY.into(),
        )
    };
    let slug = slugify(&input.name);
    anyhow::ensure!(!slug.is_empty(), "Agent 名稱無法轉換為安全 slug");
    base.path = new_skill_path(paths, &input.category, &slug)?;
    base.set_string("name", input.name.clone());
    base.set_string("description", input.description.clone());
    if !base
        .frontmatter
        .contains_key(Value::String("license".into()))
    {
        base.set_string("license", "MIT");
    }
    let author = base.metadata_string("author");
    base.set_metadata_string(
        "author",
        if author.is_empty() {
            "agent-manager"
        } else {
            &author
        },
    );
    base.set_metadata_string("version", "1.0.0");
    base.set_metadata_string("category", input.category.clone());
    if base.metadata_string("language").is_empty() {
        base.set_metadata_string("language", "zh-TW");
    }
    if !base
        .frontmatter
        .contains_key(Value::String("compatibility".into()))
    {
        base.set_string(
            "compatibility",
            "Claude Code、VS Code Copilot、GitHub Copilot 及所有相容 Agent Skills 的平台",
        );
    }
    base.set_string(
        "allowed-tools",
        if input.allowed_tools.trim().is_empty() {
            "Read Write"
        } else {
            &input.allowed_tools
        },
    );
    let replacements = [
        ("name", input.name.as_str()),
        ("description", input.description.as_str()),
        ("category", input.category.as_str()),
        (
            "role",
            if input.role.is_empty() {
                "你是專業 Agent。"
            } else {
                &input.role
            },
        ),
        (
            "abilities",
            if input.abilities.is_empty() {
                "- 核心能力 1\n- 核心能力 2\n- 核心能力 3"
            } else {
                &input.abilities
            },
        ),
        (
            "input_example",
            if input.input_example.is_empty() {
                "請描述您的需求..."
            } else {
                &input.input_example
            },
        ),
        (
            "output_example",
            if input.output_example.is_empty() {
                "（Agent 回覆內容）"
            } else {
                &input.output_example
            },
        ),
    ];
    for (key, value) in replacements {
        base.body = base.body.replace(&format!("{{{{{key}}}}}"), value);
    }
    base.body = base
        .body
        .replace("{{date}}", &Local::now().format("%Y-%m-%d").to_string());
    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn slug_and_template_are_unicode_safe() {
        assert_eq!(slugify("  醫療 / QA Agent! "), "醫療-qa-agent");
        let temp = tempdir().unwrap();
        let paths = AppPaths::from_root(temp.path());
        let skill = render_from_template(
            &paths,
            &TemplateInput {
                category: "01-醫療健康".into(),
                name: "臨床 QA".into(),
                description: "何時需要醫療問答時使用，提供嚴謹而安全的協助。".into(),
                role: String::new(),
                abilities: String::new(),
                allowed_tools: "Read".into(),
                input_example: String::new(),
                output_example: String::new(),
            },
            None,
        )
        .unwrap();
        assert!(skill.path.ends_with("01-醫療健康/臨床-qa/SKILL.md"));
        assert!(skill.body.contains("# 臨床 QA Agent"));
        assert!(!skill.body.contains("{{"));
    }
}
