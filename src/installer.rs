use std::path::Path;

use crate::{
    AppPaths,
    categories::ensure_category,
    model::AgentSkill,
    storage,
    tool_registry::{ToolKind, backup_skills, find_tool, install_skill, install_skills},
};

pub fn install_agents(skills: &[AgentSkill], tool_id: &str, target: &Path) -> (usize, usize) {
    let Some(tool) = find_tool(tool_id) else {
        return (0, skills.len());
    };
    if matches!(tool.kind, ToolKind::Aider | ToolKind::Windsurf) {
        return match install_skills(&tool, skills, target) {
            Ok(_) => (skills.len(), 0),
            Err(_) => (0, skills.len()),
        };
    }
    skills.iter().fold((0, 0), |(success, fail), skill| {
        match install_skill(&tool, skill, target) {
            Ok(_) => (success + 1, fail),
            Err(_) => (success, fail + 1),
        }
    })
}

pub fn backup_from_tool(
    tool_id: &str,
    source: &Path,
    paths: &AppPaths,
    skip_existing: bool,
) -> (usize, usize) {
    let Some(tool) = find_tool(tool_id) else {
        return (0, 0);
    };
    let mut imported = 0;
    let mut skipped = 0;
    for skill in backup_skills(&tool, source, paths) {
        if skip_existing && skill.path.exists() {
            skipped += 1;
            continue;
        }
        if let Some(category) = skill
            .path
            .parent()
            .and_then(|path| path.parent())
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
        {
            let _ = ensure_category(&paths.agents, &category);
        }
        if storage::save_skill(&skill, paths, skill.path.exists()).is_ok() {
            imported += 1;
        } else {
            skipped += 1;
        }
    }
    (imported, skipped)
}
