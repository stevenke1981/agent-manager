use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

use atomicwrites::{AllowOverwrite, AtomicFile};
use chrono::Local;
use serde_yaml::{Mapping, Value};
use walkdir::WalkDir;

use crate::{AppPaths, model::AgentSkill, storage};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolKind {
    ClaudeCode,
    CopilotVsCode,
    CopilotCli,
    Antigravity,
    GeminiCli,
    OpenCode,
    Cursor,
    Aider,
    Windsurf,
    OpenClaw,
    Hermes,
    Qwen,
    Kimi,
}

#[derive(Clone, Debug)]
pub struct ToolDef {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub default_path: PathBuf,
    pub project_scoped: bool,
    pub kind: ToolKind,
}

fn home() -> PathBuf {
    env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[must_use]
pub fn tools() -> Vec<ToolDef> {
    let home = home();
    vec![
        ToolDef {
            id: "claude-code",
            name: "Claude Code",
            description: "~/.claude/agents/",
            default_path: home.join(".claude/agents"),
            project_scoped: false,
            kind: ToolKind::ClaudeCode,
        },
        ToolDef {
            id: "copilot-vscode",
            name: "Copilot (VS Code)",
            description: "~/.github/agents/*.md",
            default_path: home.join(".github/agents"),
            project_scoped: false,
            kind: ToolKind::CopilotVsCode,
        },
        ToolDef {
            id: "copilot-cli",
            name: "Copilot (CLI/Desktop)",
            description: "~/.copilot/agents/*.agent.md",
            default_path: home.join(".copilot/agents"),
            project_scoped: false,
            kind: ToolKind::CopilotCli,
        },
        ToolDef {
            id: "antigravity",
            name: "Antigravity",
            description: "~/.gemini/antigravity/skills/",
            default_path: home.join(".gemini/antigravity/skills"),
            project_scoped: false,
            kind: ToolKind::Antigravity,
        },
        ToolDef {
            id: "gemini-cli",
            name: "Gemini CLI",
            description: "~/.gemini/extensions/agency-agents/",
            default_path: home.join(".gemini/extensions/agency-agents"),
            project_scoped: false,
            kind: ToolKind::GeminiCli,
        },
        ToolDef {
            id: "opencode",
            name: "OpenCode",
            description: "~/.config/opencode/agents/",
            default_path: home.join(".config/opencode/agents"),
            project_scoped: false,
            kind: ToolKind::OpenCode,
        },
        ToolDef {
            id: "cursor",
            name: "Cursor",
            description: "<專案>/.cursor/rules/",
            default_path: home.join("Documents"),
            project_scoped: true,
            kind: ToolKind::Cursor,
        },
        ToolDef {
            id: "aider",
            name: "Aider",
            description: "<專案>/CONVENTIONS.md（合併）",
            default_path: home.join("Documents"),
            project_scoped: true,
            kind: ToolKind::Aider,
        },
        ToolDef {
            id: "windsurf",
            name: "Windsurf",
            description: "<專案>/.windsurfrules（合併）",
            default_path: home.join("Documents"),
            project_scoped: true,
            kind: ToolKind::Windsurf,
        },
        ToolDef {
            id: "openclaw",
            name: "OpenClaw",
            description: "~/.openclaw/agency-agents/",
            default_path: home.join(".openclaw/agency-agents"),
            project_scoped: false,
            kind: ToolKind::OpenClaw,
        },
        ToolDef {
            id: "hermes",
            name: "Hermes Agent",
            description: "~/.hermes/skills/",
            default_path: home.join(".hermes/skills"),
            project_scoped: false,
            kind: ToolKind::Hermes,
        },
        ToolDef {
            id: "qwen",
            name: "Qwen CLI",
            description: "<專案>/.qwen/agents/",
            default_path: home.join("Documents"),
            project_scoped: true,
            kind: ToolKind::Qwen,
        },
        ToolDef {
            id: "kimi",
            name: "Kimi CLI",
            description: "~/.config/kimi/agents/",
            default_path: home.join(".config/kimi/agents"),
            project_scoped: false,
            kind: ToolKind::Kimi,
        },
    ]
}

#[must_use]
pub fn find_tool(id: &str) -> Option<ToolDef> {
    tools().into_iter().find(|tool| tool.id == id)
}

fn flat_yaml(fields: &[(&str, String)], body: &str) -> anyhow::Result<String> {
    let mapping: Mapping = fields
        .iter()
        .map(|(key, value)| (Value::String((*key).into()), Value::String(value.clone())))
        .collect();
    let mut yaml = serde_yaml::to_string(&mapping)?;
    if let Some(rest) = yaml.strip_prefix("---\n") {
        yaml = rest.into();
    }
    Ok(format!("---\n{yaml}---\n{body}"))
}

fn copilot_cli_yaml(name: String, description: String, body: &str) -> anyhow::Result<String> {
    let mut mapping = Mapping::new();
    mapping.insert(Value::String("name".into()), Value::String(name));
    mapping.insert(
        Value::String("description".into()),
        Value::String(description),
    );
    mapping.insert(
        Value::String("tools".into()),
        Value::Sequence(
            ["read", "edit", "search", "execute"]
                .into_iter()
                .map(|tool| Value::String(tool.into()))
                .collect(),
        ),
    );
    let mut yaml = serde_yaml::to_string(&mapping)?;
    if let Some(rest) = yaml.strip_prefix("---\n") {
        yaml = rest.into();
    }
    Ok(format!("---\n{yaml}---\n{body}"))
}

fn color_hex(color: &str) -> String {
    if color.starts_with('#') {
        return color.into();
    }
    match color.to_ascii_lowercase().as_str() {
        "red" => "#FF4444",
        "blue" => "#4488FF",
        "green" => "#44FF88",
        "cyan" => "#00FFFF",
        "purple" => "#AA44FF",
        "orange" => "#FF8844",
        "yellow" => "#FFFF44",
        "pink" => "#FF88CC",
        "white" => "#FFFFFF",
        "black" => "#222222",
        "teal" => "#00AAAA",
        "gold" => "#FFD700",
        "silver" => "#C0C0C0",
        _ => "#888888",
    }
    .into()
}

fn backup_target(path: &Path, target_root: &Path) -> anyhow::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let relative = path
        .strip_prefix(target_root)
        .unwrap_or_else(|_| path.file_name().map(Path::new).unwrap_or(path));
    let backup =
        storage::reserve_backup_root(&target_root.join(".agent-manager-backup"))?.join(relative);
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(path, backup)?;
    Ok(())
}

fn upsert_consolidated_block(existing: &str, skill: &AgentSkill) -> String {
    let begin = format!("<!-- agent-manager:{}:begin -->", skill.slug());
    let end = format!("<!-- agent-manager:{}:end -->", skill.slug());
    let block = format!(
        "{begin}\n## {}\n\n{}\n{end}",
        skill.name(),
        skill.body.trim()
    );
    if let Some(start) = existing.find(&begin)
        && let Some(relative_end) = existing[start..].find(&end)
    {
        let after = start + relative_end + end.len();
        return format!("{}{}{}", &existing[..start], block, &existing[after..]);
    }
    if existing.trim().is_empty() {
        format!("{block}\n")
    } else {
        format!("{}\n\n---\n\n{block}\n", existing.trim_end())
    }
}

fn write_file(path: &Path, content: &str, target_root: &Path) -> anyhow::Result<PathBuf> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    backup_target(path, target_root)?;
    AtomicFile::new(path, AllowOverwrite).write(|file| file.write_all(content.as_bytes()))?;
    Ok(path.to_path_buf())
}

#[must_use]
pub fn output_paths(tool: &ToolDef, skill: &AgentSkill, target: &Path) -> Vec<PathBuf> {
    match tool.kind {
        ToolKind::ClaudeCode | ToolKind::CopilotVsCode | ToolKind::OpenCode => {
            vec![target.join(format!("{}.md", skill.slug()))]
        }
        ToolKind::CopilotCli => vec![target.join(format!("{}.agent.md", skill.slug()))],
        ToolKind::Antigravity => vec![
            target
                .join(format!("agency-{}", skill.slug()))
                .join("SKILL.md"),
        ],
        ToolKind::GeminiCli => vec![
            target.join("skills").join(skill.slug()).join("SKILL.md"),
            target.join("gemini-extension.json"),
        ],
        ToolKind::Cursor => vec![
            target
                .join(".cursor/rules")
                .join(format!("{}.mdc", skill.slug())),
        ],
        ToolKind::Aider => vec![target.join("CONVENTIONS.md")],
        ToolKind::Windsurf => vec![target.join(".windsurfrules")],
        ToolKind::OpenClaw => vec![
            target.join(skill.slug()).join("SOUL.md"),
            target.join(skill.slug()).join("IDENTITY.md"),
        ],
        ToolKind::Hermes => vec![target.join(skill.slug()).join("SKILL.md")],
        ToolKind::Qwen => vec![
            target
                .join(".qwen/agents")
                .join(format!("{}.md", skill.slug())),
        ],
        ToolKind::Kimi => vec![
            target.join(skill.slug()).join("system.md"),
            target.join(skill.slug()).join("agent.yaml"),
        ],
    }
}

pub fn install_skills(
    tool: &ToolDef,
    skills: &[AgentSkill],
    target: &Path,
) -> anyhow::Result<Vec<PathBuf>> {
    if !matches!(tool.kind, ToolKind::Aider | ToolKind::Windsurf) {
        let mut installed = Vec::new();
        for skill in skills {
            installed.extend(install_skill(tool, skill, target)?);
        }
        return Ok(installed);
    }
    let Some(first) = skills.first() else {
        return Ok(Vec::new());
    };
    let output = output_paths(tool, first, target)
        .into_iter()
        .next()
        .expect("consolidated tool has one output");
    let existing = match fs::read_to_string(&output) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    let merged = skills.iter().fold(existing.clone(), |content, skill| {
        upsert_consolidated_block(&content, skill)
    });
    if merged != existing {
        write_file(&output, &merged, target)?;
    }
    Ok(vec![output])
}

pub fn install_skill(
    tool: &ToolDef,
    skill: &AgentSkill,
    target: &Path,
) -> anyhow::Result<Vec<PathBuf>> {
    let paths = output_paths(tool, skill, target);
    let name = skill.name();
    let description = skill.description();
    match tool.kind {
        ToolKind::ClaudeCode | ToolKind::CopilotVsCode => {
            write_file(
                &paths[0],
                &flat_yaml(&[("name", name), ("description", description)], &skill.body)?,
                target,
            )?;
        }
        ToolKind::CopilotCli => {
            write_file(
                &paths[0],
                &copilot_cli_yaml(name, description, &skill.body)?,
                target,
            )?;
        }
        ToolKind::Antigravity => {
            write_file(
                &paths[0],
                &flat_yaml(
                    &[
                        ("name", format!("agency-{}", skill.slug())),
                        ("description", description),
                        ("risk", "low".into()),
                        ("source", "community".into()),
                        ("date_added", Local::now().format("%Y-%m-%d").to_string()),
                    ],
                    &skill.body,
                )?,
                target,
            )?;
        }
        ToolKind::GeminiCli => {
            write_file(
                &paths[0],
                &flat_yaml(&[("name", name), ("description", description)], &skill.body)?,
                target,
            )?;
            if !paths[1].exists() {
                write_file(
                    &paths[1],
                    "{\n  \"name\": \"agency-agents\",\n  \"version\": \"1.0.0\",\n  \"description\": \"Agency Agents — installed via Agent Manager\",\n  \"contextFileName\": \"SKILL.md\"\n}\n",
                    target,
                )?;
            }
        }
        ToolKind::OpenCode => {
            let color = color_hex(&skill.string("color"));
            write_file(
                &paths[0],
                &flat_yaml(
                    &[
                        ("name", name),
                        ("description", description),
                        ("mode", "subagent".into()),
                        ("color", color),
                    ],
                    &skill.body,
                )?,
                target,
            )?;
        }
        ToolKind::Cursor => {
            write_file(
                &paths[0],
                &flat_yaml(
                    &[
                        ("description", description),
                        ("globs", String::new()),
                        ("alwaysApply", "false".into()),
                    ],
                    &skill.body,
                )?,
                target,
            )?;
        }
        ToolKind::Aider | ToolKind::Windsurf => {
            let existing = fs::read_to_string(&paths[0]).unwrap_or_default();
            write_file(
                &paths[0],
                &upsert_consolidated_block(&existing, skill),
                target,
            )?;
        }
        ToolKind::OpenClaw => {
            write_file(
                &paths[0],
                &format!("---\nname: {name}\nversion: 1.0\n---\n\n{}", skill.body),
                target,
            )?;
            write_file(&paths[1], &format!("# {name}\n\n{description}\n"), target)?;
        }
        ToolKind::Hermes => {
            write_file(
                &paths[0],
                &flat_yaml(
                    &[
                        ("name", name),
                        ("description", description),
                        ("source", "agent-manager".into()),
                    ],
                    &skill.body,
                )?,
                target,
            )?;
        }
        ToolKind::Qwen => {
            write_file(
                &paths[0],
                &flat_yaml(&[("name", name), ("description", description)], &skill.body)?,
                target,
            )?;
        }
        ToolKind::Kimi => {
            write_file(&paths[0], &skill.body, target)?;
            write_file(
                &paths[1],
                &format!(
                    "version: 1\nagent:\n  name: {}\n  extend: default\n  system_prompt_path: ./system.md\n  tools:\n    - \"kimi_cli.tools.shell:Shell\"\n    - \"kimi_cli.tools.file:ReadFile\"\n    - \"kimi_cli.tools.file:WriteFile\"\n",
                    skill.slug()
                ),
                target,
            )?;
        }
    }
    Ok(paths)
}

fn wrap_backup(
    path: PathBuf,
    source: AgentSkill,
    name: String,
    category: &str,
    author: &str,
    paths: &AppPaths,
) -> AgentSkill {
    let mut fm = Mapping::new();
    for (key, value) in [
        ("name", name),
        ("description", source.description()),
        ("license", "MIT".into()),
        ("compatibility", "Claude Code compatible".into()),
        ("allowed-tools", "Read Write".into()),
    ] {
        fm.insert(Value::String(key.into()), Value::String(value));
    }
    let metadata: Mapping = [
        ("author", author),
        ("version", "1.0"),
        ("category", category),
        ("language", "en"),
    ]
    .into_iter()
    .map(|(key, value)| (Value::String(key.into()), Value::String(value.into())))
    .collect();
    fm.insert(Value::String("metadata".into()), Value::Mapping(metadata));
    for key in ["color", "emoji", "vibe"] {
        if let Some(value) = source.frontmatter.get(Value::String(key.into())) {
            fm.insert(Value::String(key.into()), value.clone());
        }
    }
    AgentSkill::new(paths.agents.join(path), fm, source.body)
}

pub fn backup_skills(tool: &ToolDef, source: &Path, paths: &AppPaths) -> Vec<AgentSkill> {
    let (pattern, category, author) = match tool.kind {
        ToolKind::ClaudeCode => ("md", "38-ClaudeCode", "claude-code-backup"),
        ToolKind::CopilotVsCode => ("md", "39-CopilotVSCode", "copilot-vscode-backup"),
        ToolKind::CopilotCli => ("agent.md", "50-CopilotCLI", "copilot-cli-backup"),
        ToolKind::Antigravity => ("SKILL.md", "40-Antigravity", "antigravity-backup"),
        ToolKind::GeminiCli => ("SKILL.md", "41-GeminiCLI", "gemini-backup"),
        ToolKind::OpenCode => ("md", "42-OpenCode", "opencode-backup"),
        ToolKind::Cursor => ("mdc", "43-Cursor", "cursor-backup"),
        ToolKind::Aider => ("CONVENTIONS.md", "44-Aider", "aider-backup"),
        ToolKind::Windsurf => (".windsurfrules", "45-Windsurf", "windsurf-backup"),
        ToolKind::OpenClaw => ("SOUL.md", "46-OpenClaw", "openclaw-backup"),
        ToolKind::Hermes => ("SKILL.md", "47-Hermes", "hermes-backup"),
        ToolKind::Qwen => ("md", "48-QwenCLI", "qwen-backup"),
        ToolKind::Kimi => ("system.md", "49-KimiCLI", "kimi-backup"),
    };
    let scan_root = match tool.kind {
        ToolKind::Cursor => source.join(".cursor/rules"),
        ToolKind::Qwen => source.join(".qwen/agents"),
        ToolKind::GeminiCli => source.join("skills"),
        _ => source.to_path_buf(),
    };
    if !scan_root.exists() {
        return Vec::new();
    }
    WalkDir::new(scan_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let path = entry.into_path();
            let file = path.file_name()?.to_string_lossy();
            let matches = match pattern {
                "md" => {
                    path.extension().is_some_and(|ext| ext == "md") && !file.ends_with(".agent.md")
                }
                "agent.md" => file.ends_with(".agent.md"),
                ".windsurfrules" => file == ".windsurfrules",
                other if other.contains('.') => file == other,
                other => path.extension().is_some_and(|ext| ext == other),
            };
            if !matches {
                return None;
            }
            let text = fs::read_to_string(&path).ok()?;
            let parsed = storage::parse_skill(path.clone(), &text).ok()?;
            let slug = if file.ends_with(".agent.md") {
                file.trim_end_matches(".agent.md").to_owned()
            } else {
                path.parent()
                    .filter(|_| {
                        matches!(
                            tool.kind,
                            ToolKind::Antigravity
                                | ToolKind::GeminiCli
                                | ToolKind::OpenClaw
                                | ToolKind::Hermes
                                | ToolKind::Kimi
                        )
                    })
                    .and_then(|parent| parent.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| {
                        path.file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    })
            };
            let name = if parsed.name().is_empty() {
                slug.clone()
            } else {
                parsed.name().trim_start_matches("agency-").into()
            };
            Some(wrap_backup(
                PathBuf::from(category).join(slug).join("SKILL.md"),
                parsed,
                name,
                category.split_once('-').map_or(category, |(_, v)| v),
                author,
                paths,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_conversion_covers_project_and_cli_targets() {
        let skill = AgentSkill::new(
            PathBuf::from("agents/01-Test/hello/SKILL.md"),
            Mapping::new(),
            String::new(),
        );
        let cursor = find_tool("cursor").unwrap();
        let cli = find_tool("copilot-cli").unwrap();
        assert!(
            output_paths(&cursor, &skill, Path::new("C:/repo"))[0]
                .ends_with(".cursor/rules/hello.mdc")
        );
        assert!(
            output_paths(&cli, &skill, Path::new("C:/Users/me/.copilot/agents"))[0]
                .ends_with("hello.agent.md")
        );
    }

    #[test]
    fn copilot_install_preserves_tool_list_and_backs_up_existing_target() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path();
        let tool = find_tool("copilot-cli").unwrap();
        let mut frontmatter = Mapping::new();
        frontmatter.insert(Value::String("name".into()), Value::String("Hello".into()));
        frontmatter.insert(
            Value::String("description".into()),
            Value::String("A useful agent".into()),
        );
        let skill = AgentSkill::new(
            PathBuf::from("agents/01-Test/hello/SKILL.md"),
            frontmatter,
            "# Hello\n".into(),
        );
        let output = output_paths(&tool, &skill, target)[0].clone();
        std::fs::create_dir_all(output.parent().unwrap()).unwrap();
        std::fs::write(&output, "old").unwrap();

        install_skill(&tool, &skill, target).unwrap();

        let installed = storage::load_skill(&output).unwrap();
        assert!(matches!(
            installed.frontmatter.get(Value::String("tools".into())),
            Some(Value::Sequence(items)) if items.len() == 4
        ));
        assert!(target.join(".agent-manager-backup").exists());
    }

    #[test]
    fn consolidated_tools_upsert_agent_block_idempotently() {
        for tool_id in ["aider", "windsurf"] {
            let temp = tempfile::tempdir().unwrap();
            let tool = find_tool(tool_id).unwrap();
            let mut frontmatter = Mapping::new();
            frontmatter.insert(Value::String("name".into()), Value::String("Hello".into()));
            let mut skill = AgentSkill::new(
                PathBuf::from("agents/01-Test/hello/SKILL.md"),
                frontmatter,
                "first body\n".into(),
            );
            install_skill(&tool, &skill, temp.path()).unwrap();
            skill.body = "updated body\n".into();
            install_skill(&tool, &skill, temp.path()).unwrap();

            let output = output_paths(&tool, &skill, temp.path())[0].clone();
            let text = std::fs::read_to_string(output).unwrap();
            assert_eq!(text.matches("## Hello").count(), 1, "{tool_id}");
            assert!(!text.contains("first body"), "{tool_id}");
            assert!(text.contains("updated body"), "{tool_id}");
        }
    }

    #[test]
    fn rapid_target_backups_are_unique() {
        let temp = tempfile::tempdir().unwrap();
        let tool = find_tool("copilot-cli").unwrap();
        let skill = AgentSkill::new(
            PathBuf::from("agents/01-Test/hello/SKILL.md"),
            Mapping::new(),
            "body\n".into(),
        );
        let output = output_paths(&tool, &skill, temp.path())[0].clone();
        std::fs::create_dir_all(output.parent().unwrap()).unwrap();
        std::fs::write(&output, "original").unwrap();
        install_skill(&tool, &skill, temp.path()).unwrap();
        install_skill(&tool, &skill, temp.path()).unwrap();

        let backups = std::fs::read_dir(temp.path().join(".agent-manager-backup"))
            .unwrap()
            .count();
        assert_eq!(backups, 2);
    }

    #[test]
    fn consolidated_batch_installs_fifty_skills_with_at_most_one_backup() {
        for tool_id in ["aider", "windsurf"] {
            let temp = tempfile::tempdir().unwrap();
            let tool = find_tool(tool_id).unwrap();
            let skills: Vec<_> = (0..50)
                .map(|index| {
                    let slug = format!("agent-{index:02}");
                    let mut frontmatter = Mapping::new();
                    frontmatter.insert(
                        Value::String("name".into()),
                        Value::String(format!("Agent {index:02}")),
                    );
                    AgentSkill::new(
                        PathBuf::from(format!("agents/01-Test/{slug}/SKILL.md")),
                        frontmatter,
                        format!("body {index}\n"),
                    )
                })
                .collect();
            let output = output_paths(&tool, &skills[0], temp.path())[0].clone();
            std::fs::write(&output, "existing conventions\n").unwrap();

            install_skills(&tool, &skills, temp.path()).unwrap();
            install_skills(&tool, &skills, temp.path()).unwrap();

            let text = std::fs::read_to_string(output).unwrap();
            for index in 0..50 {
                let marker = format!("<!-- agent-manager:agent-{index:02}:begin -->");
                assert_eq!(text.matches(&marker).count(), 1, "{tool_id} {marker}");
            }
            let backup_root = temp.path().join(".agent-manager-backup");
            let backups = std::fs::read_dir(backup_root).unwrap().count();
            assert!(backups <= 1, "{tool_id} created {backups} backups");
        }
    }
}
