use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    sync::{
        LazyLock,
        atomic::{AtomicU64, Ordering},
    },
};

use atomicwrites::{AllowOverwrite, AtomicFile};
use chrono::Local;
use regex::Regex;
use serde_yaml::Mapping;
use thiserror::Error;
use walkdir::WalkDir;

use crate::{AppPaths, model::AgentSkill};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("無法讀寫 {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} 的 YAML frontmatter 無效: {source}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },
    #[error("拒絕操作 agents/ 以外的路徑: {0}")]
    OutsideAgents(PathBuf),
    #[error("不安全的相對路徑元件：{0}")]
    InvalidComponent(String),
    #[error("Agent 路徑必須恰為 agents/<category>/<slug>/SKILL.md：{0}")]
    InvalidSkillPath(PathBuf),
    #[error("無法序列化 YAML: {0}")]
    Serialize(#[from] serde_yaml::Error),
}

fn io_error(path: &Path, source: std::io::Error) -> StorageError {
    StorageError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn split_frontmatter(text: &str) -> Option<(&str, &str)> {
    static FRONTMATTER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?s)\A---[ \t]*\r?\n(.*?)\r?\n---[ \t]*\r?\n(.*)\z").expect("valid regex")
    });
    let captures = FRONTMATTER.captures(text)?;
    Some((captures.get(1)?.as_str(), captures.get(2)?.as_str()))
}

pub fn parse_skill(path: impl Into<PathBuf>, text: &str) -> Result<AgentSkill, StorageError> {
    let path = path.into();
    if let Some((yaml, body)) = split_frontmatter(text) {
        let frontmatter: Mapping =
            serde_yaml::from_str(yaml).map_err(|source| StorageError::Yaml {
                path: path.clone(),
                source,
            })?;
        Ok(AgentSkill::new(path, frontmatter, body.to_owned()))
    } else {
        Ok(AgentSkill::new(path, Mapping::new(), text.to_owned()))
    }
}

pub fn load_skill(path: &Path) -> Result<AgentSkill, StorageError> {
    let text = fs::read_to_string(path).map_err(|source| io_error(path, source))?;
    parse_skill(path.to_path_buf(), &text)
}

pub fn serialize_skill(skill: &AgentSkill) -> Result<String, StorageError> {
    let mut yaml = serde_yaml::to_string(&skill.frontmatter)?;
    if let Some(rest) = yaml.strip_prefix("---\n") {
        yaml = rest.to_owned();
    }
    Ok(format!("---\n{}---\n{}", yaml, skill.body))
}

pub fn validate_relative_component(component: &str) -> Result<(), StorageError> {
    if component.is_empty()
        || component == "."
        || component == ".."
        || component.contains('/')
        || component.contains('\\')
    {
        return Err(StorageError::InvalidComponent(component.to_owned()));
    }
    let mut components = Path::new(component).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err(StorageError::InvalidComponent(component.to_owned()));
    }
    Ok(())
}

fn validated_skill_parts<'a>(
    path: &'a Path,
    paths: &AppPaths,
) -> Result<(&'a str, &'a str, &'a Path), StorageError> {
    let relative = path
        .strip_prefix(&paths.agents)
        .map_err(|_| StorageError::OutsideAgents(path.to_path_buf()))?;
    let components: Vec<_> = relative.components().collect();
    let [
        Component::Normal(category),
        Component::Normal(slug),
        Component::Normal(file),
    ] = components.as_slice()
    else {
        return Err(StorageError::InvalidSkillPath(path.to_path_buf()));
    };
    if file != &std::ffi::OsStr::new("SKILL.md") {
        return Err(StorageError::InvalidSkillPath(path.to_path_buf()));
    }
    let category = category
        .to_str()
        .ok_or_else(|| StorageError::InvalidSkillPath(path.to_path_buf()))?;
    let slug = slug
        .to_str()
        .ok_or_else(|| StorageError::InvalidSkillPath(path.to_path_buf()))?;
    validate_relative_component(category)?;
    validate_relative_component(slug)?;
    let agent_dir = path
        .parent()
        .ok_or_else(|| StorageError::InvalidSkillPath(path.to_path_buf()))?;

    if agent_dir.exists() {
        let canonical_root = paths
            .agents
            .canonicalize()
            .map_err(|source| io_error(&paths.agents, source))?;
        let canonical_agent = agent_dir
            .canonicalize()
            .map_err(|source| io_error(agent_dir, source))?;
        let is_direct_shape = canonical_agent
            .parent()
            .and_then(Path::parent)
            .is_some_and(|parent| parent == canonical_root);
        if !is_direct_shape || !canonical_agent.starts_with(&canonical_root) {
            return Err(StorageError::InvalidSkillPath(path.to_path_buf()));
        }
    } else if let Some(category_dir) = agent_dir.parent()
        && category_dir.exists()
    {
        let canonical_root = paths
            .agents
            .canonicalize()
            .map_err(|source| io_error(&paths.agents, source))?;
        let canonical_category = category_dir
            .canonicalize()
            .map_err(|source| io_error(category_dir, source))?;
        if canonical_category.parent() != Some(canonical_root.as_path())
            || !canonical_category.starts_with(&canonical_root)
        {
            return Err(StorageError::InvalidSkillPath(path.to_path_buf()));
        }
    }
    Ok((category, slug, agent_dir))
}

static BACKUP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn reserve_backup_root(base: &Path) -> Result<PathBuf, StorageError> {
    fs::create_dir_all(base).map_err(|source| io_error(base, source))?;
    loop {
        let counter = BACKUP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let stamp = Local::now().format("%Y%m%d-%H%M%S-%9f");
        let candidate = base.join(format!("{stamp}-{counter:016x}"));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(source) => return Err(io_error(&candidate, source)),
        }
    }
}

fn backup_existing_file(path: &Path, paths: &AppPaths) -> Result<Option<PathBuf>, StorageError> {
    if !path.exists() {
        return Ok(None);
    }
    validated_skill_parts(path, paths)?;
    let relative = path.strip_prefix(&paths.agents).expect("validated prefix");
    let backup = reserve_backup_root(&paths.backup)?.join(relative);
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
    }
    fs::copy(path, &backup).map_err(|source| io_error(&backup, source))?;
    Ok(Some(backup))
}

pub fn save_skill(
    skill: &AgentSkill,
    paths: &AppPaths,
    backup: bool,
) -> Result<Option<PathBuf>, StorageError> {
    validated_skill_parts(&skill.path, paths)?;
    if let Some(parent) = skill.path.parent() {
        fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
    }
    let backup_path = if backup {
        backup_existing_file(&skill.path, paths)?
    } else {
        None
    };
    let content = serialize_skill(skill)?;
    let atomic = AtomicFile::new(&skill.path, AllowOverwrite);
    atomic
        .write(|file| file.write_all(content.as_bytes()))
        .map_err(|source| io_error(&skill.path, source.into()))?;
    Ok(backup_path)
}

pub fn delete_skill(skill: &AgentSkill, paths: &AppPaths) -> Result<PathBuf, StorageError> {
    let (category, slug, agent_dir) = validated_skill_parts(&skill.path, paths)?;
    if !skill.path.exists() {
        return Err(io_error(
            &skill.path,
            std::io::Error::new(std::io::ErrorKind::NotFound, "SKILL.md 不存在"),
        ));
    }
    let backup_agent_dir = reserve_backup_root(&paths.backup)?
        .join(category)
        .join(slug);
    copy_directory(agent_dir, &backup_agent_dir)?;
    if !backup_agent_dir.join("SKILL.md").is_file() {
        return Err(io_error(
            &backup_agent_dir,
            std::io::Error::other("完整 Agent 備份未包含 SKILL.md"),
        ));
    }
    fs::remove_dir_all(agent_dir).map_err(|source| io_error(agent_dir, source))?;
    Ok(backup_agent_dir)
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), StorageError> {
    for entry in WalkDir::new(source).follow_links(false) {
        let entry =
            entry.map_err(|error| io_error(source, std::io::Error::other(error.to_string())))?;
        if entry.file_type().is_symlink() {
            return Err(io_error(
                entry.path(),
                std::io::Error::other("備份不接受符號連結"),
            ));
        }
        let relative = entry.path().strip_prefix(source).expect("walk root prefix");
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|source| io_error(&target, source))?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
            }
            fs::copy(entry.path(), &target).map_err(|source| io_error(&target, source))?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct LoadDiagnostic {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct SkillLoadReport {
    pub skills: Vec<AgentSkill>,
    pub diagnostics: Vec<LoadDiagnostic>,
}

pub fn list_skills_checked(
    paths: &AppPaths,
    category: Option<&str>,
) -> Result<SkillLoadReport, StorageError> {
    if let Some(category) = category {
        validate_relative_component(category)?;
    }
    let root = category.map_or_else(
        || paths.agents.clone(),
        |category| paths.agents.join(category),
    );
    if !root.exists() {
        return Ok(SkillLoadReport::default());
    }
    let mut report = SkillLoadReport::default();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                report.diagnostics.push(LoadDiagnostic {
                    path: error.path().unwrap_or(&paths.agents).to_path_buf(),
                    message: format!("檔案列舉失敗：{error}"),
                });
                continue;
            }
        };
        if !entry.file_type().is_file() || entry.file_name() != "SKILL.md" {
            continue;
        }
        if let Err(error) = validated_skill_parts(entry.path(), paths) {
            report.diagnostics.push(LoadDiagnostic {
                path: entry.path().to_path_buf(),
                message: error.to_string(),
            });
            continue;
        }
        match load_skill(entry.path()) {
            Ok(skill) => report.skills.push(skill),
            Err(error) => report.diagnostics.push(LoadDiagnostic {
                path: entry.path().to_path_buf(),
                message: error.to_string(),
            }),
        }
    }
    report.skills.sort_by(|a, b| a.path.cmp(&b.path));
    report.diagnostics.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(report)
}

pub fn new_skill_path(
    paths: &AppPaths,
    category: &str,
    slug: &str,
) -> Result<PathBuf, StorageError> {
    validate_relative_component(category)?;
    validate_relative_component(slug)?;
    Ok(paths.agents.join(category).join(slug).join("SKILL.md"))
}

#[cfg(test)]
mod tests {
    use serde_yaml::Value;
    use tempfile::tempdir;

    use super::*;

    const COMPLEX: &str = "---\nname: 測試\ndescription: '含: 冒號 # 符號'\nallowed-tools:\n  - Read\n  - Write\nmetadata:\n  version: 1.2.3\n  category: 03-科技資訊\ncustom:\n  nested:\n    enabled: true\n---\n# Body\n\n內容\n";

    #[test]
    fn parser_roundtrip_preserves_unknown_yaml_semantics_and_body() {
        let skill = parse_skill("SKILL.md", COMPLEX).unwrap();
        assert_eq!(skill.name(), "測試");
        let encoded = serialize_skill(&skill).unwrap();
        let reparsed = parse_skill("SKILL.md", &encoded).unwrap();
        assert_eq!(reparsed.frontmatter, skill.frontmatter);
        assert_eq!(reparsed.body, skill.body);
        assert!(matches!(
            reparsed
                .frontmatter
                .get(Value::String("allowed-tools".into())),
            Some(Value::Sequence(_))
        ));
    }

    #[test]
    fn save_is_atomic_and_creates_timestamped_backup() {
        let temp = tempdir().unwrap();
        let paths = AppPaths::from_root(temp.path());
        let path = new_skill_path(&paths, "01-測試", "alpha").unwrap();
        let mut skill = parse_skill(path.clone(), COMPLEX).unwrap();
        save_skill(&skill, &paths, false).unwrap();
        skill.body = "changed\n".into();
        let backup = save_skill(&skill, &paths, true).unwrap().unwrap();
        assert!(backup.exists());
        assert_eq!(load_skill(&path).unwrap().body, "changed\n");
        assert_eq!(load_skill(&backup).unwrap().body, "# Body\n\n內容\n");
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn rejects_traversal_and_non_agent_delete_shapes() {
        let temp = tempdir().unwrap();
        let paths = AppPaths::from_root(temp.path());
        std::fs::create_dir_all(paths.agents.join("category")).unwrap();
        let outside = temp.path().join("outside");
        std::fs::create_dir_all(&outside).unwrap();
        for path in [
            paths.agents.join("SKILL.md"),
            paths.agents.join("category/SKILL.md"),
            paths.agents.join("../outside/SKILL.md"),
        ] {
            std::fs::write(&path, COMPLEX).unwrap();
            let skill = parse_skill(path, COMPLEX).unwrap();
            assert!(delete_skill(&skill, &paths).is_err());
        }
        for component in ["", ".", "..", "../escape", "a/b", "a\\b"] {
            assert!(
                validate_relative_component(component).is_err(),
                "{component}"
            );
            assert!(new_skill_path(&paths, component, "slug").is_err());
            assert!(new_skill_path(&paths, "01-Test", component).is_err());
        }
        assert!(list_skills_checked(&paths, Some("../outside")).is_err());
    }

    #[test]
    fn delete_backs_up_the_complete_agent_directory() {
        let temp = tempdir().unwrap();
        let paths = AppPaths::from_root(temp.path());
        let path = paths.agents.join("01-Test/alpha/SKILL.md");
        std::fs::create_dir_all(path.parent().unwrap().join("assets")).unwrap();
        std::fs::write(&path, COMPLEX).unwrap();
        std::fs::write(path.parent().unwrap().join("assets/icon.txt"), "asset").unwrap();
        let skill = load_skill(&path).unwrap();

        let backup_agent_dir = delete_skill(&skill, &paths).unwrap();

        assert!(!path.parent().unwrap().exists());
        assert!(backup_agent_dir.join("SKILL.md").exists());
        assert_eq!(
            std::fs::read_to_string(backup_agent_dir.join("assets/icon.txt")).unwrap(),
            "asset"
        );
    }

    #[test]
    fn checked_listing_reports_invalid_yaml_instead_of_dropping_it() {
        let temp = tempdir().unwrap();
        let paths = AppPaths::from_root(temp.path());
        let good = paths.agents.join("01-Test/good/SKILL.md");
        let bad = paths.agents.join("01-Test/bad/SKILL.md");
        std::fs::create_dir_all(good.parent().unwrap()).unwrap();
        std::fs::create_dir_all(bad.parent().unwrap()).unwrap();
        std::fs::write(good, COMPLEX).unwrap();
        std::fs::write(bad, "---\nmetadata: [\n---\nbody").unwrap();

        let report = list_skills_checked(&paths, None).unwrap();

        assert_eq!(report.skills.len(), 1);
        assert_eq!(report.diagnostics.len(), 1);
        assert!(report.diagnostics[0].message.contains("YAML"));
    }
}
