#![forbid(unsafe_code)]

pub mod app;
pub mod categories;
pub mod config;
pub mod evolution;
pub mod importer;
pub mod installer;
pub mod llm;
pub mod manager;
pub mod model;
pub mod storage;
pub mod template;
pub mod theme;
pub mod tool_registry;
pub mod validator;

use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub root: PathBuf,
    pub agents: PathBuf,
    pub backup: PathBuf,
    pub config: PathBuf,
    pub evolution_log: PathBuf,
}

impl AppPaths {
    #[must_use]
    pub fn from_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let agents = root.join("agents");
        Self {
            backup: root.join(".backup"),
            config: root.join(".config.json"),
            evolution_log: agents.join(".evolution.log"),
            agents,
            root,
        }
    }
}
