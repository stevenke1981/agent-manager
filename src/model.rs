use std::path::PathBuf;

use serde_yaml::{Mapping, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct AgentSkill {
    pub path: PathBuf,
    pub frontmatter: Mapping,
    pub body: String,
}

impl AgentSkill {
    #[must_use]
    pub fn new(path: PathBuf, frontmatter: Mapping, body: String) -> Self {
        Self {
            path,
            frontmatter,
            body,
        }
    }

    #[must_use]
    pub fn slug(&self) -> String {
        self.path
            .parent()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn string(&self, key: &str) -> String {
        self.frontmatter
            .get(Value::String(key.to_owned()))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    }

    #[must_use]
    pub fn name(&self) -> String {
        let name = self.string("name");
        if name.is_empty() { self.slug() } else { name }
    }

    #[must_use]
    pub fn description(&self) -> String {
        self.string("description")
    }

    #[must_use]
    pub fn metadata_string(&self, key: &str) -> String {
        self.frontmatter
            .get(Value::String("metadata".to_owned()))
            .and_then(Value::as_mapping)
            .and_then(|mapping| mapping.get(Value::String(key.to_owned())))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    }

    #[must_use]
    pub fn category(&self) -> String {
        self.metadata_string("category")
    }

    pub fn set_string(&mut self, key: &str, value: impl Into<String>) {
        self.frontmatter
            .insert(Value::String(key.to_owned()), Value::String(value.into()));
    }

    pub fn set_metadata_string(&mut self, key: &str, value: impl Into<String>) {
        let metadata_key = Value::String("metadata".to_owned());
        if !matches!(self.frontmatter.get(&metadata_key), Some(Value::Mapping(_))) {
            self.frontmatter
                .insert(metadata_key.clone(), Value::Mapping(Mapping::new()));
        }
        if let Some(Value::Mapping(metadata)) = self.frontmatter.get_mut(&metadata_key) {
            metadata.insert(Value::String(key.to_owned()), Value::String(value.into()));
        }
    }
}
