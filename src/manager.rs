use std::path::Path;

use crate::{
    AppPaths, categories,
    model::AgentSkill,
    storage,
    template::{self, TemplateInput},
    validator::{self, Issue},
};

#[derive(Clone)]
pub struct AgentManager {
    pub paths: AppPaths,
}

impl AgentManager {
    #[must_use]
    pub fn new(paths: AppPaths) -> Self {
        Self { paths }
    }
    pub fn categories(&self) -> anyhow::Result<Vec<String>> {
        Ok(categories::list_categories(&self.paths.agents)?)
    }
    pub fn list(
        &self,
        category: Option<&str>,
    ) -> Result<storage::SkillLoadReport, storage::StorageError> {
        storage::list_skills_checked(&self.paths, category)
    }
    pub fn open(&self, path: &Path) -> anyhow::Result<AgentSkill> {
        Ok(storage::load_skill(path)?)
    }
    pub fn save(&self, skill: &AgentSkill) -> anyhow::Result<Vec<Issue>> {
        storage::save_skill(skill, &self.paths, true)?;
        Ok(validator::validate(skill))
    }
    pub fn delete(&self, skill: &AgentSkill) -> anyhow::Result<std::path::PathBuf> {
        Ok(storage::delete_skill(skill, &self.paths)?)
    }
    pub fn create(&self, input: &TemplateInput, keyword: &str) -> anyhow::Result<AgentSkill> {
        let template = template::find_best_template(
            &self.paths,
            &input.category,
            if keyword.is_empty() {
                &input.name
            } else {
                keyword
            },
        );
        let skill = template::render_from_template(&self.paths, input, template.as_deref())?;
        anyhow::ensure!(
            !skill.path.exists(),
            "Agent 已存在：{}",
            skill.path.display()
        );
        storage::save_skill(&skill, &self.paths, false)?;
        Ok(skill)
    }
    pub fn create_from_ai(
        &self,
        input: &TemplateInput,
        body: String,
    ) -> anyhow::Result<AgentSkill> {
        let mut skill = template::render_from_template(&self.paths, input, None)?;
        if !body.trim().is_empty() {
            skill.body = format!("{}\n", body.trim());
        }
        anyhow::ensure!(
            !skill.path.exists(),
            "Agent 已存在：{}",
            skill.path.display()
        );
        storage::save_skill(&skill, &self.paths, false)?;
        Ok(skill)
    }
    pub fn search(
        &self,
        category: Option<&str>,
        query: &str,
    ) -> Result<storage::SkillLoadReport, storage::StorageError> {
        let query = query.trim().to_lowercase();
        let mut report = self.list(category)?;
        report.skills.retain(|skill| {
            query.is_empty()
                || skill.slug().to_lowercase().contains(&query)
                || skill.name().to_lowercase().contains(&query)
                || skill.description().to_lowercase().contains(&query)
                || skill.body.to_lowercase().contains(&query)
        });
        Ok(report)
    }
}
