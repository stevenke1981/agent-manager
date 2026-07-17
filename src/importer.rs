use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};

use serde_yaml::{Mapping, Value};
use walkdir::WalkDir;

use crate::{AppPaths, categories::ensure_category, model::AgentSkill, storage};

pub const CATEGORY_MAP: &[(&str, &str)] = &[
    ("academic", "22-Academic"),
    ("design", "23-Design"),
    ("engineering", "24-Engineering"),
    ("finance", "25-Finance"),
    ("game-development", "26-GameDev"),
    ("integrations", "27-Integrations"),
    ("marketing", "28-Marketing"),
    ("paid-media", "29-PaidMedia"),
    ("product", "30-Product"),
    ("project-management", "31-ProjectMgmt"),
    ("sales", "32-Sales"),
    ("spatial-computing", "33-SpatialComp"),
    ("specialized", "34-Specialized"),
    ("strategy", "35-Strategy"),
    ("support", "36-Support"),
    ("testing", "37-Testing"),
];
const SKIP: &[&str] = &[
    "README.md",
    "CONTRIBUTING.md",
    "SECURITY.md",
    "CONTRIBUTING_zh-CN.md",
    "PULL_REQUEST_TEMPLATE.md",
    "EXECUTIVE-BRIEF.md",
    "QUICKSTART.md",
];

pub fn collect_importable(source: &Path) -> Vec<(PathBuf, &'static str)> {
    let mut files = Vec::new();
    for &(folder, category) in CATEGORY_MAP {
        let root = source.join(folder);
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            let path = entry.into_path();
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
                && path
                    .file_name()
                    .is_some_and(|name| !SKIP.iter().any(|skip| name == *skip))
            {
                files.push((path, category));
            }
        }
    }
    files.sort();
    files
}

#[must_use]
pub fn count_importable(source: &Path) -> usize {
    collect_importable(source).len()
}

fn source_slug(import_root: &Path, source: &Path, category: &str) -> anyhow::Result<String> {
    let folder = CATEGORY_MAP
        .iter()
        .find_map(|(folder, mapped)| (*mapped == category).then_some(*folder))
        .ok_or_else(|| anyhow::anyhow!("unknown import category: {category}"))?;
    let relative = source.strip_prefix(import_root.join(folder))?;
    let without_extension = relative.with_extension("");
    let joined = without_extension
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("-");
    let slug = crate::template::slugify(&joined);
    anyhow::ensure!(!slug.is_empty(), "source path cannot form a safe slug");
    Ok(slug)
}

fn stable_suffix(path: &Path) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in path.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn to_skill(
    import_root: &Path,
    source: &Path,
    category: &str,
    paths: &AppPaths,
) -> anyhow::Result<AgentSkill> {
    let text = std::fs::read_to_string(source)?;
    let parsed = storage::parse_skill(source.to_path_buf(), &text)?;
    let slug = source_slug(import_root, source, category)?;
    let mut fm = Mapping::new();
    for key in ["name", "description", "color", "emoji", "vibe"] {
        if let Some(value) = parsed.frontmatter.get(Value::String(key.into())) {
            fm.insert(Value::String(key.into()), value.clone());
        }
    }
    fm.entry(Value::String("name".into()))
        .or_insert(Value::String(slug.clone()));
    fm.entry(Value::String("description".into()))
        .or_insert(Value::String(String::new()));
    fm.insert(Value::String("license".into()), Value::String("MIT".into()));
    let metadata: BTreeMap<&str, &str> = [
        ("author", "agency-agents"),
        ("version", "1.0"),
        (
            "category",
            category
                .split_once('-')
                .map_or(category, |(_, value)| value),
        ),
        ("language", "en"),
    ]
    .into();
    fm.insert(
        Value::String("metadata".into()),
        serde_yaml::to_value(metadata)?,
    );
    fm.insert(
        Value::String("compatibility".into()),
        Value::String("Claude Code compatible".into()),
    );
    fm.insert(
        Value::String("allowed-tools".into()),
        Value::String("Read Write".into()),
    );
    Ok(AgentSkill::new(
        storage::new_skill_path(paths, category, &slug)?,
        fm,
        parsed.body,
    ))
}

pub fn import_all(source: &Path, paths: &AppPaths, skip_existing: bool) -> (usize, usize) {
    let mut imported = 0;
    let mut skipped = 0;
    let mut batch_targets = HashSet::new();
    for (file, category) in collect_importable(source) {
        match to_skill(source, &file, category, paths) {
            Ok(mut skill) if !batch_targets.insert(skill.path.clone()) => {
                let slug = format!("{}-{}", skill.slug(), &stable_suffix(&file)[..8]);
                match storage::new_skill_path(paths, category, &slug) {
                    Ok(path) if batch_targets.insert(path.clone()) => skill.path = path,
                    _ => {
                        skipped += 1;
                        continue;
                    }
                }
                if skip_existing && skill.path.exists() {
                    skipped += 1;
                    continue;
                }
                let _ = ensure_category(&paths.agents, category);
                if storage::save_skill(&skill, paths, skill.path.exists()).is_ok() {
                    imported += 1;
                } else {
                    skipped += 1;
                }
            }
            Ok(skill) if skip_existing && skill.path.exists() => skipped += 1,
            Ok(skill) => {
                let _ = ensure_category(&paths.agents, category);
                if storage::save_skill(&skill, paths, skill.path.exists()).is_ok() {
                    imported += 1;
                } else {
                    skipped += 1;
                }
            }
            Err(_) => skipped += 1,
        }
    }
    (imported, skipped)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn nested_same_stem_sources_get_stable_unique_slugs() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let paths = AppPaths::from_root(temp.path().join("manager"));
        for (team, body) in [("team-one", "first"), ("team-two", "second")] {
            let file = source.join("academic").join(team).join("agent.md");
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(
                file,
                format!("---\nname: {team}\ndescription: imported\n---\n{body}\n"),
            )
            .unwrap();
        }

        let (imported, skipped) = import_all(&source, &paths, false);

        assert_eq!((imported, skipped), (2, 0));
        let skills = storage::list_skills_checked(&paths, Some("22-Academic"))
            .unwrap()
            .skills;
        assert_eq!(skills.len(), 2);
        assert_ne!(skills[0].slug(), skills[1].slug());
        assert!(skills.iter().any(|skill| skill.body.contains("first")));
        assert!(skills.iter().any(|skill| skill.body.contains("second")));
    }
}
