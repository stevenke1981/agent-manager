use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::Duration,
};

use eframe::egui::{self, RichText};

use crate::{
    AppPaths,
    config::{AppConfig, all_models, load_config, save_config},
    evolution::{self, EvolutionRecord, ScanResult},
    importer, installer,
    llm::{AgentDraft, LlmClient, edit_text_with_ai, generate_agent_draft},
    manager::AgentManager,
    model::AgentSkill,
    storage,
    template::TemplateInput,
    theme,
    tool_registry::tools,
    validator::{self, Issue, Severity},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorkspaceTab {
    Editor,
    Validation,
    Evolution,
    Log,
}

#[derive(Clone, Debug, Default)]
struct EditorState {
    name: String,
    category: String,
    version: String,
    tools: String,
    description: String,
    body: String,
    dirty: bool,
}

impl EditorState {
    fn from_skill(skill: &AgentSkill) -> Self {
        Self {
            name: skill.name(),
            category: skill.category(),
            version: skill.metadata_string("version"),
            tools: skill.string("allowed-tools"),
            description: skill.description(),
            body: skill.body.clone(),
            dirty: false,
        }
    }
    fn apply(&self, skill: &mut AgentSkill) {
        skill.set_string("name", self.name.trim());
        skill.set_string("description", self.description.trim());
        skill.set_string("allowed-tools", self.tools.trim());
        skill.set_metadata_string("category", self.category.trim());
        skill.set_metadata_string(
            "version",
            if self.version.trim().is_empty() {
                "1.0.0"
            } else {
                self.version.trim()
            },
        );
        skill.body = format!("{}\n", self.body.trim_end());
    }
}

#[derive(Clone, Debug)]
struct CreateState {
    category: String,
    name: String,
    description: String,
    role: String,
    abilities: String,
    tools: String,
    template_keyword: String,
    brief: String,
    ai_body: Option<String>,
}
impl Default for CreateState {
    fn default() -> Self {
        Self {
            category: String::new(),
            name: String::new(),
            description: String::new(),
            role: String::new(),
            abilities: String::new(),
            tools: "Read Write".into(),
            template_keyword: String::new(),
            brief: String::new(),
            ai_body: None,
        }
    }
}

#[derive(Clone, Debug)]
struct ImportState {
    source: String,
    skip_existing: bool,
    preview_source: String,
    preview_count: Option<usize>,
}
impl Default for ImportState {
    fn default() -> Self {
        Self {
            source: dirs_home()
                .join("agency-agents-main")
                .to_string_lossy()
                .into_owned(),
            skip_existing: true,
            preview_source: String::new(),
            preview_count: None,
        }
    }
}

#[derive(Clone, Debug)]
struct InstallState {
    tool_id: String,
    target: String,
    backup_mode: bool,
    all_filtered: bool,
    skip_existing: bool,
}
impl Default for InstallState {
    fn default() -> Self {
        let tool = tools().into_iter().next().expect("registry");
        Self {
            tool_id: tool.id.into(),
            target: tool.default_path.to_string_lossy().into_owned(),
            backup_mode: false,
            all_filtered: false,
            skip_existing: true,
        }
    }
}

#[derive(Clone, Debug)]
struct AiEditState {
    scope: String,
    instruction: String,
    preview: String,
    model: String,
    binding: Option<DocumentBinding>,
    stale: bool,
}
impl Default for AiEditState {
    fn default() -> Self {
        Self {
            scope: "body".into(),
            instruction: String::new(),
            preview: String::new(),
            model: String::new(),
            binding: None,
            stale: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DocumentBinding {
    path: PathBuf,
    revision: u64,
}

impl DocumentBinding {
    fn matches(&self, current_path: Option<&PathBuf>, current_revision: u64) -> bool {
        current_path == Some(&self.path) && current_revision == self.revision
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DeletePending {
    path: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TaskKind {
    ReadOnly,
    Mutating,
}

#[derive(Clone, Debug)]
struct BusyState {
    label: String,
    kind: TaskKind,
    document: Option<DocumentBinding>,
}

fn editor_and_save_enabled(busy: Option<&BusyState>, conflict: bool) -> bool {
    !conflict && !busy.is_some_and(|state| state.kind == TaskKind::Mutating)
}

enum TaskResult {
    Scan(Result<evolution::ScanReport, String>),
    Evolve(Result<Vec<EvolutionRecord>, String>),
    Ping(Result<String, String>),
    Import(Result<(usize, usize), String>),
    ImportCount {
        source: String,
        count: usize,
    },
    Install(Result<(usize, usize), String>),
    Backup(Result<(usize, usize), String>),
    Draft(Result<AgentDraft, String>),
    AiEdit {
        binding: DocumentBinding,
        result: Result<(String, String), String>,
    },
}

pub struct AgentManagerApp {
    manager: AgentManager,
    config: AppConfig,
    skills: Vec<AgentSkill>,
    categories: Vec<String>,
    filtered: Vec<usize>,
    selected_category: Option<String>,
    selected: Option<usize>,
    search: String,
    editor: EditorState,
    editor_revision: u64,
    tab: WorkspaceTab,
    issues: Vec<Issue>,
    scan_results: Vec<ScanResult>,
    load_diagnostics: Vec<storage::LoadDiagnostic>,
    evolution_records: Vec<EvolutionRecord>,
    logs: Vec<serde_json::Value>,
    status: String,
    status_error: bool,
    dark: bool,
    busy: Option<BusyState>,
    save_conflict: bool,
    receiver: Option<Receiver<TaskResult>>,
    create_open: bool,
    create: CreateState,
    settings_open: bool,
    import_open: bool,
    import: ImportState,
    install_open: bool,
    install: InstallState,
    ai_edit_open: bool,
    ai_edit: AiEditState,
    delete_pending: Option<DeletePending>,
    evolve_confirm: bool,
}

impl AgentManagerApp {
    pub fn new(context: &eframe::CreationContext<'_>, paths: AppPaths) -> Self {
        theme::setup_fonts(&context.egui_ctx);
        theme::apply(&context.egui_ctx, false);
        let manager = AgentManager::new(paths.clone());
        let config = load_config(&paths.config);
        let mut app = Self {
            manager,
            config,
            skills: Vec::new(),
            categories: Vec::new(),
            filtered: Vec::new(),
            selected_category: None,
            selected: None,
            search: String::new(),
            editor: EditorState::default(),
            editor_revision: 0,
            tab: WorkspaceTab::Editor,
            issues: Vec::new(),
            scan_results: Vec::new(),
            load_diagnostics: Vec::new(),
            evolution_records: Vec::new(),
            logs: Vec::new(),
            status: "載入中".into(),
            status_error: false,
            dark: false,
            busy: None,
            save_conflict: false,
            receiver: None,
            create_open: false,
            create: CreateState::default(),
            settings_open: false,
            import_open: false,
            import: ImportState::default(),
            install_open: false,
            install: InstallState::default(),
            ai_edit_open: false,
            ai_edit: AiEditState::default(),
            delete_pending: None,
            evolve_confirm: false,
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        if self.editor.dirty {
            self.error("有未儲存變更，已略過重新載入；請先儲存或重新選取後再執行");
            return;
        }
        let selected_path = self
            .selected
            .and_then(|index| self.skills.get(index))
            .map(|skill| skill.path.clone());
        let categories = match self.manager.categories() {
            Ok(categories) => categories,
            Err(error) => {
                self.error(format!("類別載入失敗：{error:#}"));
                return;
            }
        };
        let report = match self.manager.list(None) {
            Ok(report) => report,
            Err(error) => {
                self.error(format!("Agent 載入失敗：{error}"));
                return;
            }
        };
        self.categories = categories;
        self.skills = report.skills;
        self.load_diagnostics = report.diagnostics;
        self.selected =
            selected_path.and_then(|path| self.skills.iter().position(|skill| skill.path == path));
        self.refilter();
        if self.load_diagnostics.is_empty() {
            self.info(format!(
                "已載入 {} 個 Agent、{} 個類別",
                self.skills.len(),
                self.categories.len()
            ));
        } else {
            self.error(format!(
                "已載入 {} 個 Agent，但有 {} 個檔案載入失敗（詳見掃描 / 進化）",
                self.skills.len(),
                self.load_diagnostics.len()
            ));
        }
    }
    fn refilter(&mut self) {
        let query = self.search.trim().to_lowercase();
        self.filtered = self
            .skills
            .iter()
            .enumerate()
            .filter(|(_, skill)| {
                self.selected_category.as_ref().is_none_or(|category| {
                    skill
                        .path
                        .parent()
                        .and_then(|p| p.parent())
                        .and_then(|p| p.file_name())
                        .is_some_and(|name| name == category.as_str())
                })
            })
            .filter(|(_, skill)| {
                query.is_empty()
                    || skill.name().to_lowercase().contains(&query)
                    || skill.slug().to_lowercase().contains(&query)
                    || skill.description().to_lowercase().contains(&query)
                    || skill.body.to_lowercase().contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
    }
    fn select(&mut self, index: usize) {
        if self.has_mutating_task() {
            self.error("背景寫入工作進行中；完成前不能切換 Agent");
            return;
        }
        if self.editor.dirty && self.selected != Some(index) {
            self.error("有未儲存變更；請先儲存，才能切換 Agent");
            return;
        }
        self.selected = Some(index);
        if let Some(skill) = self.skills.get(index) {
            self.editor = EditorState::from_skill(skill);
            self.editor_revision = self.editor_revision.wrapping_add(1);
            self.save_conflict = false;
            self.issues = validator::validate(skill);
            self.status = format!("已開啟 {}", skill.path.display());
            self.status_error = false;
        }
    }
    fn error(&mut self, message: impl Into<String>) {
        self.status = message.into();
        self.status_error = true;
    }
    fn info(&mut self, message: impl Into<String>) {
        self.status = message.into();
        self.status_error = false;
    }
    fn has_mutating_task(&self) -> bool {
        self.busy
            .as_ref()
            .is_some_and(|state| state.kind == TaskKind::Mutating)
    }
    fn spawn(
        &mut self,
        label: &str,
        kind: TaskKind,
        task: impl FnOnce() -> TaskResult + Send + 'static,
    ) {
        if self.busy.is_some() {
            self.error("已有背景工作進行中，請稍候");
            return;
        }
        if kind == TaskKind::Mutating && (self.editor.dirty || self.save_conflict) {
            self.error("有未儲存或衝突變更；請先重新載入或儲存，才能啟動寫入工作");
            return;
        }
        let document = self
            .selected
            .and_then(|index| self.skills.get(index))
            .map(|skill| DocumentBinding {
                path: skill.path.clone(),
                revision: self.editor_revision,
            });
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(task());
        });
        self.busy = Some(BusyState {
            label: label.into(),
            kind,
            document,
        });
        self.receiver = Some(rx);
        self.info(format!("{label}…"));
    }
    fn reload_after_mutation(&mut self, state: &BusyState) -> bool {
        let current_path = self
            .selected
            .and_then(|index| self.skills.get(index))
            .map(|skill| &skill.path);
        let matches = match &state.document {
            Some(binding) => binding.matches(current_path, self.editor_revision),
            None => current_path.is_none(),
        };
        if self.editor.dirty || !matches {
            self.save_conflict = true;
            return false;
        }

        let selected_path = state.document.as_ref().map(|binding| binding.path.clone());
        self.refresh();
        if let Some(path) = selected_path {
            let Some(index) = self.skills.iter().position(|skill| skill.path == path) else {
                self.save_conflict = true;
                return false;
            };
            self.selected = Some(index);
            self.editor = EditorState::from_skill(&self.skills[index]);
            self.issues = validator::validate(&self.skills[index]);
            self.editor_revision = self.editor_revision.wrapping_add(1);
        }
        self.save_conflict = false;
        true
    }
    fn poll(&mut self, context: &egui::Context) {
        if self.busy.is_some() {
            context.request_repaint_after(Duration::from_millis(80));
        }
        let received = self.receiver.as_ref().map(Receiver::try_recv);
        if matches!(received, Some(Err(TryRecvError::Disconnected))) {
            let completed = self.busy.take();
            self.receiver = None;
            let conflict = completed
                .as_ref()
                .filter(|state| state.kind == TaskKind::Mutating)
                .is_some_and(|state| !self.reload_after_mutation(state));
            self.error(if conflict {
                "背景工作中斷且文件版本衝突；已禁止儲存，請重新選取 Agent"
            } else {
                "背景工作通道意外中斷；工作未確認完成，請重試"
            });
            return;
        }
        if let Some(Ok(result)) = received {
            let completed = self.busy.take();
            self.receiver = None;
            let conflict = completed
                .as_ref()
                .filter(|state| state.kind == TaskKind::Mutating)
                .is_some_and(|state| !self.reload_after_mutation(state));
            match result {
                TaskResult::Scan(Ok(report)) => {
                    self.scan_results = report.results;
                    self.load_diagnostics = report.diagnostics;
                    self.tab = WorkspaceTab::Evolution;
                    if self.load_diagnostics.is_empty() {
                        self.info(format!(
                            "掃描完成：{} 個 Agent 有問題",
                            self.scan_results.len()
                        ));
                    } else {
                        self.error(format!(
                            "掃描完成：{} 個 Agent 有問題，{} 個檔案載入失敗",
                            self.scan_results.len(),
                            self.load_diagnostics.len()
                        ));
                    }
                }
                TaskResult::Scan(Err(e)) => self.error(e),
                TaskResult::Evolve(Ok(records)) => {
                    self.evolution_records = records;
                    self.tab = WorkspaceTab::Evolution;
                    self.info(format!(
                        "進化完成：處理 {} 個 Agent",
                        self.evolution_records.len()
                    ));
                }
                TaskResult::Evolve(Err(e)) => self.error(e),
                TaskResult::Ping(Ok(message)) => self.info(message),
                TaskResult::Ping(Err(e)) => self.error(e),
                TaskResult::Import(Ok((ok, skip))) => {
                    self.info(format!("匯入完成：新增 {ok}，跳過 {skip}"));
                }
                TaskResult::Import(Err(e)) => self.error(e),
                TaskResult::ImportCount { source, count } => {
                    if self.import.source == source {
                        self.import.preview_source = source;
                        self.import.preview_count = Some(count);
                    }
                }
                TaskResult::Install(Ok((ok, fail))) => {
                    self.info(format!("安裝完成：成功 {ok}，失敗 {fail}"))
                }
                TaskResult::Install(Err(e)) => self.error(e),
                TaskResult::Backup(Ok((ok, skip))) => {
                    self.info(format!("備份完成：新增 {ok}，跳過 {skip}"));
                }
                TaskResult::Backup(Err(e)) => self.error(e),
                TaskResult::Draft(Ok(draft)) => {
                    self.create.description = draft.description;
                    self.create.role = draft.role;
                    self.create.abilities = draft.abilities;
                    self.create.ai_body = Some(draft.body);
                    self.info(format!("{} 已產生 Agent 草稿", draft.model));
                }
                TaskResult::Draft(Err(e)) => self.error(e),
                TaskResult::AiEdit {
                    binding,
                    result: Ok((text, model)),
                } => {
                    let current_path = self
                        .selected
                        .and_then(|index| self.skills.get(index))
                        .map(|skill| &skill.path);
                    self.ai_edit.stale = !binding.matches(current_path, self.editor_revision);
                    self.ai_edit.binding = Some(binding);
                    self.ai_edit.preview = text;
                    self.ai_edit.model = model.clone();
                    if self.ai_edit.stale {
                        self.error(format!(
                            "{model} 已回傳，但目前文件或版本已變更；預覽不可套用"
                        ));
                    } else {
                        self.info(format!("{model} 已產生修改預覽"));
                    }
                }
                TaskResult::AiEdit { result: Err(e), .. } => self.error(e),
            }
            if conflict {
                self.error("背景寫入完成，但文件版本已變更；已禁止儲存，請重新選取 Agent");
            }
        }
    }
    fn save_current(&mut self) {
        if !editor_and_save_enabled(self.busy.as_ref(), self.save_conflict) {
            self.error("背景寫入或版本衝突期間不能儲存；請等待完成或重新選取 Agent");
            return;
        }
        let Some(index) = self.selected else {
            self.error("請先選取 Agent");
            return;
        };
        let Some(existing) = self.skills.get(index).cloned() else {
            return;
        };
        let mut skill = existing;
        self.editor.apply(&mut skill);
        match self.manager.save(&skill) {
            Ok(issues) => {
                self.skills[index] = skill;
                self.editor.dirty = false;
                self.editor_revision = self.editor_revision.wrapping_add(1);
                self.issues = issues;
                self.info(format!("已儲存並備份；{} 項驗證問題", self.issues.len()));
                self.refilter();
            }
            Err(e) => self.error(format!("儲存失敗：{e:#}")),
        }
    }
    fn validate_current(&mut self) {
        let Some(index) = self.selected else {
            self.error("請先選取 Agent");
            return;
        };
        let mut skill = self.skills[index].clone();
        self.editor.apply(&mut skill);
        self.issues = validator::validate(&skill);
        self.tab = WorkspaceTab::Validation;
        self.info(if self.issues.is_empty() {
            "驗證通過".into()
        } else {
            format!("驗證完成：{} 項問題", self.issues.len())
        });
    }
    fn run_scan(&mut self) {
        let paths = self.manager.paths.clone();
        self.spawn("掃描全部", TaskKind::ReadOnly, move || {
            TaskResult::Scan(evolution::scan_all_checked(&paths).map_err(|e| e.to_string()))
        });
    }
    fn run_evolve(&mut self) {
        if self.editor.dirty {
            self.error("有未儲存變更；請先儲存，才能執行進化");
            return;
        }
        let paths = self.manager.paths.clone();
        let config = self.config.clone();
        self.spawn("執行自我進化", TaskKind::Mutating, move || {
            TaskResult::Evolve(evolution::evolve_once(&paths, &config).map_err(|e| e.to_string()))
        });
    }
    fn current_scope(&self) -> Option<String> {
        self.selected.and_then(|i| self.skills.get(i))?;
        Some(match self.ai_edit.scope.as_str() {
            "description" => self.editor.description.clone(),
            "body" => self.editor.body.clone(),
            heading => extract_section(&self.editor.body, heading),
        })
    }

    fn command_bar(&mut self, context: &egui::Context) {
        egui::TopBottomPanel::top("command_bar")
            .exact_height(52.0)
            .show(context, |ui| {
                ui.add_space(theme::SPACE_2);
                ui.horizontal(|ui| {
                    ui.heading("Agent Manager");
                    ui.label(
                        RichText::new("2.0 · Rust 工作台").color(ui.visuals().weak_text_color()),
                    );
                    ui.separator();
                    let editor_actions_enabled =
                        editor_and_save_enabled(self.busy.as_ref(), self.save_conflict);
                    if ui
                        .add_enabled(editor_actions_enabled, egui::Button::new("新增"))
                        .clicked()
                    {
                        if self.editor.dirty {
                            self.error("有未儲存變更；請先儲存，才能新增 Agent");
                        } else {
                            self.create = CreateState::default();
                            self.create.category =
                                self.categories.first().cloned().unwrap_or_default();
                            self.create_open = true;
                        }
                    }
                    if ui
                        .add_enabled(
                            self.selected.is_some() && editor_actions_enabled,
                            egui::Button::new("儲存"),
                        )
                        .clicked()
                    {
                        self.save_current();
                    }
                    if ui
                        .add_enabled(self.selected.is_some(), egui::Button::new("驗證"))
                        .clicked()
                    {
                        self.validate_current();
                    }
                    if ui
                        .add_enabled(
                            self.selected.is_some() && editor_actions_enabled,
                            egui::Button::new("刪除"),
                        )
                        .clicked()
                    {
                        if self.editor.dirty {
                            self.error("有未儲存變更；請先儲存，才能刪除 Agent");
                        } else if let Some(path) = self
                            .selected
                            .and_then(|index| self.skills.get(index))
                            .map(|skill| skill.path.clone())
                        {
                            self.delete_pending = Some(DeletePending { path });
                        }
                    }
                    ui.separator();
                    if ui
                        .add_enabled(self.busy.is_none(), egui::Button::new("掃描"))
                        .clicked()
                    {
                        self.run_scan();
                    }
                    if ui
                        .add_enabled(self.busy.is_none(), egui::Button::new("進化"))
                        .clicked()
                    {
                        if self.editor.dirty {
                            self.error("有未儲存變更；請先儲存，才能執行進化");
                        } else {
                            self.evolve_confirm = true;
                        }
                    }
                    if ui
                        .add_enabled(
                            self.selected.is_some() && self.busy.is_none(),
                            egui::Button::new("AI 修改"),
                        )
                        .clicked()
                    {
                        self.ai_edit = AiEditState::default();
                        self.ai_edit_open = true;
                    }
                    ui.separator();
                    if ui.button("匯入").clicked() {
                        if self.editor.dirty {
                            self.error("有未儲存變更；請先儲存，才能匯入 Agent");
                        } else {
                            self.import_open = true;
                        }
                    }
                    if ui.button("安裝 / 備份").clicked() {
                        self.install_open = true;
                    }
                    if ui.button("設定").clicked() {
                        self.settings_open = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(if self.dark { "亮色" } else { "暗色" }).clicked() {
                            self.dark = !self.dark;
                            theme::apply(context, self.dark);
                        }
                        ui.label(format!("{} Agents", self.skills.len()));
                    });
                });
            });
    }
    fn sidebar(&mut self, context: &egui::Context) {
        egui::SidePanel::left("agent_sidebar")
            .resizable(true)
            .default_width(300.0)
            .min_width(220.0)
            .max_width(440.0)
            .show(context, |ui| {
                ui.add_space(theme::SPACE_2);
                ui.heading("Agents");
                let response = ui.add_sized(
                    [ui.available_width(), theme::CONTROL_HEIGHT],
                    egui::TextEdit::singleline(&mut self.search).hint_text("搜尋名稱、描述或全文…"),
                );
                if response.changed() {
                    self.refilter();
                }
                ui.horizontal(|ui| {
                    let all = self.selected_category.is_none();
                    if ui.selectable_label(all, "全部類別").clicked() {
                        self.selected_category = None;
                        self.refilter();
                    }
                    ui.label(format!("{} 筆", self.filtered.len()));
                });
                egui::ScrollArea::vertical()
                    .id_salt("category_filter_scroll")
                    .max_height(156.0)
                    .show(ui, |ui| {
                        let categories = self.categories.clone();
                        for category in categories {
                            let selected = self.selected_category.as_ref() == Some(&category);
                            if ui
                                .selectable_label(
                                    selected,
                                    format!(
                                        "{}  ({})",
                                        category,
                                        self.skills
                                            .iter()
                                            .filter(|skill| skill
                                                .path
                                                .to_string_lossy()
                                                .contains(&category))
                                            .count()
                                    ),
                                )
                                .clicked()
                            {
                                self.selected_category = Some(category);
                                self.refilter();
                            }
                        }
                    });
                ui.separator();
                let filtered = self.filtered.clone();
                egui::ScrollArea::vertical()
                    .id_salt("agent_virtual_list")
                    .auto_shrink([false; 2])
                    .show_rows(ui, 38.0, filtered.len(), |ui, range| {
                        for position in range {
                            let index = filtered[position];
                            let skill = &self.skills[index];
                            let label = format!("{}\n{}", skill.name(), skill.slug());
                            let selected = self.selected == Some(index);
                            let response = ui
                                .push_id(&skill.path, |ui| {
                                    ui.add_enabled(
                                        !self.has_mutating_task(),
                                        egui::Button::new(label)
                                            .selected(selected)
                                            .min_size([ui.available_width(), 36.0].into()),
                                    )
                                })
                                .inner;
                            if response.clicked() {
                                self.select(index);
                            }
                        }
                    });
            });
    }
    fn workspace(&mut self, context: &egui::Context) {
        egui::CentralPanel::default().show(context, |ui| {
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (WorkspaceTab::Editor, "編輯器"),
                    (WorkspaceTab::Validation, "驗證"),
                    (WorkspaceTab::Evolution, "掃描 / 進化"),
                    (WorkspaceTab::Log, "日誌"),
                ] {
                    if ui.selectable_label(self.tab == tab, label).clicked() {
                        self.tab = tab;
                        if tab == WorkspaceTab::Log {
                            self.logs = evolution::read_log(&self.manager.paths, 200);
                        }
                    }
                }
                if self.editor.dirty {
                    ui.label(RichText::new("未儲存變更").color(theme::WARNING));
                }
            });
            ui.separator();
            match self.tab {
                WorkspaceTab::Editor => self.editor_ui(ui),
                WorkspaceTab::Validation => self.validation_ui(ui),
                WorkspaceTab::Evolution => self.evolution_ui(ui),
                WorkspaceTab::Log => self.log_ui(ui),
            }
        });
    }
    fn editor_ui(&mut self, ui: &mut egui::Ui) {
        if self.selected.is_none() {
            ui.vertical_centered(|ui| {
                ui.add_space(96.0);
                ui.heading("選取或建立 Agent");
                ui.label("從左側清單選取一筆資料，即可編輯 frontmatter 與 Markdown 內容。");
            });
            return;
        }
        let enabled = editor_and_save_enabled(self.busy.as_ref(), self.save_conflict);
        ui.add_enabled_ui(enabled, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("editor_scroll")
                .show(ui, |ui| {
                    ui.heading("Frontmatter");
                    egui::Grid::new("frontmatter_grid")
                        .num_columns(2)
                        .spacing([theme::SPACE_4, theme::SPACE_2])
                        .show(ui, |ui| {
                            for (label, value) in [
                                ("name", &mut self.editor.name),
                                ("metadata.category", &mut self.editor.category),
                                ("metadata.version", &mut self.editor.version),
                                ("allowed-tools", &mut self.editor.tools),
                            ] {
                                ui.label(label);
                                if ui
                                    .add_sized(
                                        [ui.available_width(), theme::CONTROL_HEIGHT],
                                        egui::TextEdit::singleline(value),
                                    )
                                    .changed()
                                {
                                    self.editor.dirty = true;
                                    self.editor_revision = self.editor_revision.wrapping_add(1);
                                }
                                ui.end_row();
                            }
                        });
                    ui.label("description（50–300 字，含啟動時機）");
                    if ui
                        .add_sized(
                            [ui.available_width(), 88.0],
                            egui::TextEdit::multiline(&mut self.editor.description),
                        )
                        .changed()
                    {
                        self.editor.dirty = true;
                        self.editor_revision = self.editor_revision.wrapping_add(1);
                    }
                    ui.add_space(theme::SPACE_4);
                    ui.heading("Markdown 內容");
                    let height = ui.available_height().max(320.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), height],
                            egui::TextEdit::multiline(&mut self.editor.body)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY),
                        )
                        .changed()
                    {
                        self.editor.dirty = true;
                        self.editor_revision = self.editor_revision.wrapping_add(1);
                    }
                });
        });
    }
    fn validation_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("驗證結果");
            if ui.button("重新驗證").clicked() {
                self.validate_current();
            }
        });
        if self.selected.is_none() {
            ui.label("請先選取 Agent");
            return;
        }
        if self.issues.is_empty() {
            ui.colored_label(theme::SUCCESS, "驗證通過，未發現問題。");
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("validation_scroll")
            .show(ui, |ui| {
                for (index, issue) in self.issues.iter().enumerate() {
                    ui.push_id(index, |ui| {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(
                                    severity_color(issue.severity),
                                    issue.severity.label(),
                                );
                                ui.strong(&issue.field);
                            });
                            ui.label(&issue.message);
                            if !issue.suggestion.is_empty() {
                                ui.label(
                                    RichText::new(&issue.suggestion)
                                        .color(ui.visuals().weak_text_color()),
                                );
                            }
                        });
                    });
                }
            });
    }
    fn evolution_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("掃描與自我進化");
            if ui
                .add_enabled(self.busy.is_none(), egui::Button::new("掃描全部"))
                .clicked()
            {
                self.run_scan();
            }
            if ui
                .add_enabled(self.busy.is_none(), egui::Button::new("執行進化"))
                .clicked()
            {
                if self.editor.dirty {
                    self.error("有未儲存變更；請先儲存，才能執行進化");
                } else {
                    self.evolve_confirm = true;
                }
            }
        });
        if self.scan_results.is_empty()
            && self.evolution_records.is_empty()
            && self.load_diagnostics.is_empty()
        {
            ui.label("尚無結果。掃描會以唯讀方式檢查全部 Agent；進化修改前會建立備份。");
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("evolution_scroll")
            .show(ui, |ui| {
                for diagnostic in &self.load_diagnostics {
                    ui.group(|ui| {
                        ui.colored_label(theme::ERROR, "載入失敗");
                        ui.monospace(diagnostic.path.display().to_string());
                        ui.label(&diagnostic.message);
                    });
                }
                for result in &self.scan_results {
                    ui.collapsing(
                        format!("{} · {} 項", result.skill_path, result.issues.len()),
                        |ui| {
                            for issue in &result.issues {
                                ui.horizontal_wrapped(|ui| {
                                    ui.colored_label(
                                        severity_color(issue.severity),
                                        issue.severity.label(),
                                    );
                                    ui.label(&issue.message);
                                });
                            }
                        },
                    );
                }
                for record in &self.evolution_records {
                    ui.group(|ui| {
                        ui.strong(format!("{} · {}", record.action, record.skill_path));
                        ui.label(&record.mode_reason);
                        ui.label(format!(
                            "已修復 {}，剩餘 {}",
                            record.fixed_issues.len(),
                            record.remaining_issues.len()
                        ));
                        if !record.error.is_empty() {
                            ui.colored_label(theme::ERROR, &record.error);
                        }
                    });
                }
            });
    }
    fn log_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("進化日誌");
            if ui.button("重新載入").clicked() {
                self.logs = evolution::read_log(&self.manager.paths, 200);
            }
        });
        if self.logs.is_empty() {
            ui.label("尚無進化日誌。");
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("log_scroll")
            .show(ui, |ui| {
                for (index, record) in self.logs.iter().enumerate() {
                    ui.push_id(index, |ui| {
                        ui.monospace(serde_json::to_string_pretty(record).unwrap_or_default());
                        ui.separator();
                    });
                }
            });
    }

    fn dialogs(&mut self, context: &egui::Context) {
        self.create_dialog(context);
        self.settings_dialog(context);
        self.import_dialog(context);
        self.install_dialog(context);
        self.ai_dialog(context);
        self.confirm_dialogs(context);
    }
    fn create_dialog(&mut self, context: &egui::Context) {
        if !self.create_open {
            return;
        }
        let mut open = true;
        egui::Window::new("新增 Agent")
            .id(egui::Id::new("create_agent_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(640.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(context, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("create_scroll")
                    .max_height(620.0)
                    .show(ui, |ui| {
                        egui::ComboBox::from_label("類別")
                            .selected_text(&self.create.category)
                            .show_ui(ui, |ui| {
                                for category in &self.categories {
                                    ui.selectable_value(
                                        &mut self.create.category,
                                        category.clone(),
                                        category,
                                    );
                                }
                            });
                        field(ui, "Agent 名稱", &mut self.create.name);
                        field(ui, "allowed-tools", &mut self.create.tools);
                        field(
                            ui,
                            "模板關鍵字（可留空）",
                            &mut self.create.template_keyword,
                        );
                        multiline(
                            ui,
                            "description（必填）",
                            &mut self.create.description,
                            72.0,
                        );
                        multiline(ui, "角色設定", &mut self.create.role, 64.0);
                        multiline(ui, "核心能力", &mut self.create.abilities, 80.0);
                        multiline(ui, "AI 提示（一句話需求）", &mut self.create.brief, 64.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(self.busy.is_none(), egui::Button::new("AI 生成內容"))
                                .clicked()
                            {
                                let config = self.config.clone();
                                let (category, name, brief, allowed) = (
                                    self.create.category.clone(),
                                    self.create.name.clone(),
                                    self.create.brief.clone(),
                                    self.create.tools.clone(),
                                );
                                if category.trim().is_empty()
                                    || name.trim().is_empty()
                                    || brief.trim().is_empty()
                                {
                                    self.error("AI 生成需要類別、名稱與 AI 提示");
                                } else {
                                    self.spawn("AI 生成 Agent", TaskKind::ReadOnly, move || {
                                        TaskResult::Draft((|| {
                                            let client = LlmClient::new(config)
                                                .map_err(|e| e.to_string())?;
                                            generate_agent_draft(
                                                &client, &category, &name, &brief, &allowed,
                                            )
                                            .map_err(|e| e.to_string())
                                        })(
                                        ))
                                    });
                                }
                            }
                            if ui
                                .add_enabled(!self.has_mutating_task(), egui::Button::new("建立"))
                                .clicked()
                            {
                                if self.editor.dirty {
                                    self.error("有未儲存變更；請先儲存，才能新增 Agent");
                                } else if self.create.category.trim().is_empty()
                                    || self.create.name.trim().is_empty()
                                    || self.create.description.trim().is_empty()
                                {
                                    self.error("類別、名稱與 description 為必填");
                                } else {
                                    let input = TemplateInput {
                                        category: self.create.category.clone(),
                                        name: self.create.name.clone(),
                                        description: self.create.description.clone(),
                                        role: self.create.role.clone(),
                                        abilities: normalize_abilities(&self.create.abilities),
                                        allowed_tools: self.create.tools.clone(),
                                        input_example: String::new(),
                                        output_example: String::new(),
                                    };
                                    let result = if let Some(body) = self.create.ai_body.clone() {
                                        self.manager.create_from_ai(&input, body)
                                    } else {
                                        self.manager.create(&input, &self.create.template_keyword)
                                    };
                                    match result {
                                        Ok(skill) => {
                                            self.create_open = false;
                                            self.refresh();
                                            if let Some(index) = self
                                                .skills
                                                .iter()
                                                .position(|item| item.path == skill.path)
                                            {
                                                self.select(index);
                                            }
                                            self.info(format!("已建立 {}", skill.path.display()));
                                        }
                                        Err(e) => self.error(format!("建立失敗：{e:#}")),
                                    }
                                }
                            }
                        });
                    });
            });
        self.create_open &= open;
    }
    fn settings_dialog(&mut self, context: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = true;
        egui::Window::new("設定")
            .id(egui::Id::new("settings_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(620.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(context, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("settings_scroll")
                    .max_height(640.0)
                    .show(ui, |ui| {
                        ui.heading("OpenRouter");
                        ui.label("API Key（只儲存在 gitignore 的 .config.json）");
                        ui.add_sized(
                            [ui.available_width(), theme::CONTROL_HEIGHT],
                            egui::TextEdit::singleline(&mut self.config.openrouter_api_key)
                                .password(true),
                        );
                        field(ui, "Base URL", &mut self.config.openrouter_base_url);
                        let models = all_models(&self.config);
                        egui::ComboBox::from_label("主模型")
                            .selected_text(&self.config.primary_model)
                            .show_ui(ui, |ui| {
                                for model in &models {
                                    ui.selectable_value(
                                        &mut self.config.primary_model,
                                        model.clone(),
                                        model,
                                    );
                                }
                            });
                        egui::ComboBox::from_label("備援模型")
                            .selected_text(&self.config.fallback_model)
                            .show_ui(ui, |ui| {
                                for model in &models {
                                    ui.selectable_value(
                                        &mut self.config.fallback_model,
                                        model.clone(),
                                        model,
                                    );
                                }
                            });
                        ui.add(
                            egui::Slider::new(&mut self.config.request_timeout, 5..=300)
                                .text("Timeout 秒"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.config.max_tokens, 128..=32768)
                                .text("Max tokens"),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.config.temperature, 0.0..=1.0)
                                .text("Temperature"),
                        );
                        ui.separator();
                        ui.heading("進化規則");
                        ui.checkbox(
                            &mut self.config.evolution_use_api,
                            "使用 OpenRouter API 重寫",
                        );
                        ui.checkbox(&mut self.config.evolution_auto_apply, "自動套用骨架修復");
                        ui.checkbox(&mut self.config.evolution_require_validation, "修復後驗證");
                        ui.checkbox(&mut self.config.evolution_dry_run, "Dry run（不寫檔）");
                        egui::ComboBox::from_label("最低嚴重度")
                            .selected_text(&self.config.evolution_min_severity)
                            .show_ui(ui, |ui| {
                                for severity in ["CRITICAL", "HIGH", "MEDIUM", "LOW"] {
                                    ui.selectable_value(
                                        &mut self.config.evolution_min_severity,
                                        severity.into(),
                                        severity,
                                    );
                                }
                            });
                        ui.add(
                            egui::Slider::new(
                                &mut self.config.evolution_max_agents_per_run,
                                1..=500,
                            )
                            .text("每輪上限"),
                        );
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(self.busy.is_none(), egui::Button::new("測試連線"))
                                .clicked()
                            {
                                let config = self.config.clone();
                                self.spawn("測試 OpenRouter", TaskKind::ReadOnly, move || {
                                    TaskResult::Ping((|| {
                                        let client =
                                            LlmClient::new(config).map_err(|e| e.to_string())?;
                                        client.ping().map_err(|e| e.to_string())
                                    })())
                                });
                            }
                            if ui.button("儲存設定").clicked() {
                                match save_config(&self.manager.paths.config, &self.config) {
                                    Ok(()) => {
                                        self.settings_open = false;
                                        self.info("設定已儲存");
                                    }
                                    Err(e) => self.error(format!("設定儲存失敗：{e:#}")),
                                }
                            }
                        });
                    });
            });
        self.settings_open &= open;
    }
    fn import_dialog(&mut self, context: &egui::Context) {
        if !self.import_open {
            return;
        }
        let mut open = true;
        egui::Window::new("匯入 Agency Agents")
            .id(egui::Id::new("import_dialog"))
            .open(&mut open)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(context, |ui| {
                field(ui, "來源目錄", &mut self.import.source);
                if ui.button("選擇目錄").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_folder()
                {
                    self.import.source = path.to_string_lossy().into_owned();
                }
                ui.checkbox(&mut self.import.skip_existing, "跳過已存在 Agent");
                let source = PathBuf::from(&self.import.source);
                if self.import.preview_source != self.import.source && self.busy.is_none() {
                    let preview_source = self.import.source.clone();
                    self.import.preview_source = preview_source.clone();
                    self.import.preview_count = None;
                    self.spawn("計算匯入預覽", TaskKind::ReadOnly, move || {
                        let count = importer::count_importable(&PathBuf::from(&preview_source));
                        TaskResult::ImportCount {
                            source: preview_source,
                            count,
                        }
                    });
                }
                if self.import.preview_source == self.import.source
                    && let Some(count) = self.import.preview_count
                {
                    ui.label(format!("可匯入 {count} 筆"));
                } else {
                    ui.label("正在背景計算可匯入數量…");
                }
                if ui
                    .add_enabled(
                        source.exists() && self.busy.is_none(),
                        egui::Button::new("開始匯入"),
                    )
                    .clicked()
                {
                    if self.editor.dirty {
                        self.error("有未儲存變更；請先儲存，才能匯入 Agent");
                    } else {
                        let paths = self.manager.paths.clone();
                        let skip = self.import.skip_existing;
                        self.spawn("匯入 Agent", TaskKind::Mutating, move || {
                            TaskResult::Import(Ok(importer::import_all(&source, &paths, skip)))
                        });
                    }
                }
            });
        self.import_open &= open;
    }
    fn install_dialog(&mut self, context: &egui::Context) {
        if !self.install_open {
            return;
        }
        let mut open = true;
        egui::Window::new("安裝 / 備份工具")
            .id(egui::Id::new("install_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(720.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(context, |ui| {
                let registry = tools();
                let selected_tool = registry
                    .iter()
                    .find(|tool| tool.id == self.install.tool_id)
                    .cloned()
                    .unwrap_or_else(|| registry[0].clone());
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.install.backup_mode, false, "安裝到工具");
                    ui.selectable_value(&mut self.install.backup_mode, true, "從工具備份");
                });
                egui::ComboBox::from_label("工具")
                    .selected_text(selected_tool.name)
                    .show_ui(ui, |ui| {
                        for tool in &registry {
                            if ui
                                .selectable_value(
                                    &mut self.install.tool_id,
                                    tool.id.into(),
                                    format!("{} · {}", tool.name, tool.description),
                                )
                                .clicked()
                            {
                                self.install.target =
                                    tool.default_path.to_string_lossy().into_owned();
                            }
                        }
                    });
                field(
                    ui,
                    if self.install.backup_mode {
                        "來源目錄"
                    } else {
                        "目標目錄"
                    },
                    &mut self.install.target,
                );
                if ui.button("選擇目錄").clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_folder()
                {
                    self.install.target = path.to_string_lossy().into_owned();
                }
                if self.install.backup_mode {
                    ui.checkbox(&mut self.install.skip_existing, "跳過已存在 Agent");
                } else {
                    ui.checkbox(
                        &mut self.install.all_filtered,
                        "安裝目前篩選結果（未勾選則只安裝目前 Agent）",
                    );
                }
                if selected_tool.project_scoped {
                    ui.colored_label(theme::WARNING, "此工具為專案範圍，請指定專案根目錄。");
                }
                ui.label("既有目標檔會先備份至目標內的 .agent-manager-backup/。");
                if ui
                    .add_enabled(
                        self.busy.is_none(),
                        egui::Button::new(if self.install.backup_mode {
                            "開始備份"
                        } else {
                            "開始安裝"
                        }),
                    )
                    .clicked()
                {
                    let target = PathBuf::from(&self.install.target);
                    let tool_id = self.install.tool_id.clone();
                    if self.install.backup_mode {
                        let paths = self.manager.paths.clone();
                        let skip = self.install.skip_existing;
                        self.spawn("從工具備份", TaskKind::Mutating, move || {
                            TaskResult::Backup(Ok(installer::backup_from_tool(
                                &tool_id, &target, &paths, skip,
                            )))
                        });
                    } else {
                        let skills: Vec<_> = if self.install.all_filtered {
                            self.filtered
                                .iter()
                                .filter_map(|&index| self.skills.get(index).cloned())
                                .collect()
                        } else {
                            self.selected
                                .and_then(|index| self.skills.get(index).cloned())
                                .into_iter()
                                .collect()
                        };
                        if skills.is_empty() {
                            self.error("沒有可安裝的 Agent");
                        } else {
                            self.spawn("安裝 Agent", TaskKind::Mutating, move || {
                                TaskResult::Install(Ok(installer::install_agents(
                                    &skills, &tool_id, &target,
                                )))
                            });
                        }
                    }
                }
            });
        self.install_open &= open;
    }
    fn ai_dialog(&mut self, context: &egui::Context) {
        if !self.ai_edit_open {
            return;
        }
        let mut open = true;
        egui::Window::new("AI 局部修改")
            .id(egui::Id::new("ai_edit_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(760.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(context, |ui| {
                egui::ComboBox::from_label("範圍")
                    .selected_text(&self.ai_edit.scope)
                    .show_ui(ui, |ui| {
                        for scope in [
                            "body",
                            "description",
                            "## 角色設定",
                            "## 核心能力",
                            "## 操作流程",
                            "## 輸入範例",
                            "## 輸出範例",
                            "## 邊緣案例處理",
                        ] {
                            ui.selectable_value(&mut self.ai_edit.scope, scope.into(), scope);
                        }
                    });
                multiline(ui, "修改指令", &mut self.ai_edit.instruction, 80.0);
                if ui
                    .add_enabled(self.busy.is_none(), egui::Button::new("生成預覽"))
                    .clicked()
                {
                    if self.ai_edit.instruction.trim().is_empty() {
                        self.error("請輸入修改指令");
                    } else if let Some(current) = self.current_scope() {
                        let Some(path) = self
                            .selected
                            .and_then(|index| self.skills.get(index))
                            .map(|skill| skill.path.clone())
                        else {
                            self.error("請先選取 Agent");
                            return;
                        };
                        let binding = DocumentBinding {
                            path,
                            revision: self.editor_revision,
                        };
                        self.ai_edit.binding = Some(binding.clone());
                        self.ai_edit.stale = false;
                        self.ai_edit.preview.clear();
                        let config = self.config.clone();
                        let instruction = self.ai_edit.instruction.clone();
                        let scope = self.ai_edit.scope.clone();
                        self.spawn("AI 局部修改", TaskKind::ReadOnly, move || {
                            TaskResult::AiEdit {
                                binding,
                                result: (|| {
                                    let client =
                                        LlmClient::new(config).map_err(|e| e.to_string())?;
                                    edit_text_with_ai(&client, &current, &instruction, &scope)
                                        .map_err(|e| e.to_string())
                                })(),
                            }
                        });
                    }
                }
                multiline(ui, "AI 預覽（可再編輯）", &mut self.ai_edit.preview, 300.0);
                if self.ai_edit.stale {
                    ui.colored_label(theme::WARNING, "文件或版本已變更；請重新生成預覽。");
                }
                let current_path = self
                    .selected
                    .and_then(|index| self.skills.get(index))
                    .map(|skill| &skill.path);
                let binding_matches = self
                    .ai_edit
                    .binding
                    .as_ref()
                    .is_some_and(|binding| binding.matches(current_path, self.editor_revision));
                if ui
                    .add_enabled(
                        !self.ai_edit.preview.trim().is_empty()
                            && !self.ai_edit.stale
                            && binding_matches,
                        egui::Button::new("套用並儲存備份"),
                    )
                    .clicked()
                {
                    let preview = self.ai_edit.preview.trim().to_owned();
                    match self.ai_edit.scope.as_str() {
                        "description" => self.editor.description = preview,
                        "body" => self.editor.body = preview,
                        heading => {
                            self.editor.body = replace_section(&self.editor.body, heading, &preview)
                        }
                    }
                    self.editor.dirty = true;
                    self.save_current();
                    self.ai_edit_open = false;
                }
            });
        self.ai_edit_open &= open;
    }
    fn confirm_dialogs(&mut self, context: &egui::Context) {
        if let Some(target) = self
            .delete_pending
            .as_ref()
            .map(|pending| pending.path.clone())
        {
            egui::Modal::new(egui::Id::new("delete_confirm")).show(context, |ui| {
                ui.heading("確認刪除");
                ui.label(format!("將刪除並完整備份：{}", target.display()));
                ui.horizontal(|ui| {
                    if ui.button("取消").clicked() {
                        self.delete_pending = None;
                    }
                    if ui
                        .button(RichText::new("確認刪除").color(theme::ERROR))
                        .clicked()
                    {
                        match self
                            .manager
                            .open(&target)
                            .and_then(|skill| self.manager.delete(&skill))
                        {
                            Ok(backup) => {
                                self.selected = None;
                                self.editor = EditorState::default();
                                self.editor_revision = self.editor_revision.wrapping_add(1);
                                self.delete_pending = None;
                                self.refresh();
                                self.info(format!("已刪除；備份：{}", backup.display()));
                            }
                            Err(e) => {
                                self.delete_pending = None;
                                self.error(format!("刪除失敗：{e:#}"));
                            }
                        }
                    }
                });
            });
        }
        if self.evolve_confirm {
            egui::Window::new("執行自我進化")
                .id(egui::Id::new("evolve_confirm"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(context, |ui| {
                    ui.label(format!(
                        "模式：{}",
                        if self.config.evolution_use_api
                            && !self.config.openrouter_api_key.is_empty()
                        {
                            "API 重寫"
                        } else {
                            "骨架補齊"
                        }
                    ));
                    ui.label(format!(
                        "最低嚴重度：{}；本輪上限：{}",
                        self.config.evolution_min_severity,
                        self.config.evolution_max_agents_per_run
                    ));
                    ui.label(if self.config.evolution_dry_run {
                        "Dry run：不寫入檔案"
                    } else {
                        "寫入前會備份至 .backup/"
                    });
                    ui.horizontal(|ui| {
                        if ui.button("取消").clicked() {
                            self.evolve_confirm = false;
                        }
                        if ui.button("繼續").clicked() {
                            self.evolve_confirm = false;
                            self.run_evolve();
                        }
                    });
                });
        }
    }
}

impl eframe::App for AgentManagerApp {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll(context);
        self.command_bar(context);
        self.sidebar(context);
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(34.0)
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    if let Some(busy) = &self.busy {
                        ui.spinner();
                        ui.label(&busy.label);
                    } else {
                        ui.colored_label(
                            if self.status_error {
                                theme::ERROR
                            } else {
                                ui.visuals().text_color()
                            },
                            &self.status,
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.editor.dirty {
                            ui.colored_label(theme::WARNING, "未儲存");
                        }
                    });
                });
            });
        self.workspace(context);
        self.dialogs(context);
    }
}

pub fn run(paths: AppPaths) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Agent Manager 2.0")
            .with_inner_size([1360.0, 860.0])
            .with_min_inner_size([1024.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Agent Manager 2.0",
        options,
        Box::new(move |context| Ok(Box::new(AgentManagerApp::new(context, paths)))),
    )
}

fn dirs_home() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
fn severity_color(severity: Severity) -> egui::Color32 {
    match severity {
        Severity::Critical | Severity::High => theme::ERROR,
        Severity::Medium => theme::WARNING,
        Severity::Low => egui::Color32::from_rgb(59, 130, 246),
    }
}
fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(label);
    ui.add_sized(
        [ui.available_width(), theme::CONTROL_HEIGHT],
        egui::TextEdit::singleline(value),
    );
}
fn multiline(ui: &mut egui::Ui, label: &str, value: &mut String, height: f32) {
    ui.label(label);
    ui.add_sized(
        [ui.available_width(), height],
        egui::TextEdit::multiline(value),
    );
}
fn normalize_abilities(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            if line.trim_start().starts_with("- ") {
                line.trim().to_owned()
            } else {
                format!("- {}", line.trim())
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn extract_section(body: &str, heading: &str) -> String {
    let Some(start) = body.find(heading) else {
        return format!("{heading}\n（目前無此章節）");
    };
    let tail = &body[start..];
    let end = tail
        .get(heading.len()..)
        .and_then(|rest| rest.find("\n## ").map(|offset| heading.len() + offset))
        .unwrap_or(tail.len());
    tail[..end].trim().to_owned()
}
fn replace_section(body: &str, heading: &str, replacement: &str) -> String {
    if let Some(start) = body.find(heading) {
        let tail = &body[start..];
        let end = tail
            .get(heading.len()..)
            .and_then(|rest| {
                rest.find("\n## ")
                    .map(|offset| start + heading.len() + offset)
            })
            .unwrap_or(body.len());
        format!(
            "{}{}\n{}",
            &body[..start],
            replacement.trim(),
            &body[end..].trim_start_matches('\n')
        )
    } else {
        format!("{}\n\n{}\n", body.trim_end(), replacement.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_result_binding_requires_the_same_path_and_revision() {
        let binding = DocumentBinding {
            path: PathBuf::from("agents/research/example/SKILL.md"),
            revision: 7,
        };

        assert!(binding.matches(Some(&PathBuf::from("agents/research/example/SKILL.md")), 7));
        assert!(!binding.matches(Some(&PathBuf::from("agents/research/other/SKILL.md")), 7));
        assert!(!binding.matches(Some(&PathBuf::from("agents/research/example/SKILL.md")), 8));
    }

    #[test]
    fn delete_pending_keeps_the_original_target_snapshot() {
        let pending = DeletePending {
            path: PathBuf::from("agents/research/original/SKILL.md"),
        };
        let later_selection = PathBuf::from("agents/research/later/SKILL.md");

        assert_ne!(pending.path, later_selection);
        assert_eq!(
            pending.path,
            PathBuf::from("agents/research/original/SKILL.md")
        );
    }

    #[test]
    fn mutating_busy_disables_editor_and_save() {
        let busy = BusyState {
            label: "import".into(),
            kind: TaskKind::Mutating,
            document: None,
        };

        assert!(!editor_and_save_enabled(Some(&busy), false));
        let read_only = BusyState {
            kind: TaskKind::ReadOnly,
            ..busy.clone()
        };
        assert!(editor_and_save_enabled(Some(&read_only), false));
        assert!(editor_and_save_enabled(None, false));
        assert!(!editor_and_save_enabled(None, true));
    }
}
