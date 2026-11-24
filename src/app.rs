use crate::{
    agent, ollama,
    projects::{Library, Project, Reference},
    store,
    types::*,
    workspace::{Workspace, clip},
};
use eframe::egui::{self, Color32, RichText, Stroke};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::Duration,
};

const BG: Color32 = Color32::from_rgb(24, 24, 24);
const PANEL: Color32 = Color32::from_rgb(28, 28, 28);
const SURFACE: Color32 = Color32::from_rgb(44, 44, 44);
const TEXT: Color32 = Color32::from_rgb(238, 238, 238);
const MUTED: Color32 = Color32::from_rgb(155, 155, 155);
const ACCENT: Color32 = Color32::from_rgb(91, 156, 246);
const BORDER: Color32 = Color32::from_rgb(53, 53, 53);
const AMBER: Color32 = Color32::from_rgb(245, 193, 112);

pub struct Desktop {
    settings: Settings,
    session: Session,
    saved: Vec<Session>,
    models: Vec<String>,
    files: Vec<String>,
    file_filter: String,
    input: String,
    status: String,
    connected: bool,
    busy: bool,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    activity: Vec<Activity>,
    approval: Option<Approval>,
    error: Option<String>,
    show_settings: bool,
    preview: Option<(String, String)>,
    ctx: egui::Context,
    library: Library,
    search: String,
    show_sources: bool,
    settings_page: String,
    project_editor: Option<Project>,
    project_is_new: bool,
    reference_kind: String,
    reference_title: String,
    reference_value: String,
    show_reference: bool,
    manage_chat: Option<Session>,
    project_storage_ready: bool,
    source_drawer: bool,
}

impl Desktop {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Option<String>) -> Self {
        theme(&cc.egui_ctx);
        let mut settings = store::load_settings();
        if let Some(path) = initial {
            settings.last_workspace = path;
        }
        let (tx, rx) = mpsc::channel();
        let mut saved = store::sessions();
        let (mut library, mut library_error) = match Library::load() {
            Ok(library) => (library, None),
            Err(error) => (
                Library::default(),
                Some(format!(
                    "Could not load projects: {error}. Existing project data has not been replaced."
                )),
            ),
        };
        if library_error.is_none() {
            library.import_sessions(&mut saved);
            if let Err(error) = library.save() {
                library_error = Some(format!("Could not save project library: {error}"));
            }
        }
        let project_storage_ready = library_error.is_none();
        let mut session = Session::fresh(settings.last_workspace.clone(), settings.model.clone());
        session.project_id = library
            .projects
            .iter()
            .find(|p| crate::projects::same_root(&p.workspace, &session.workspace))
            .map(|p| p.id.clone());
        let mut app = Self {
            session,
            saved,
            settings,
            models: vec![],
            files: vec![],
            file_filter: String::new(),
            input: String::new(),
            status: "Connecting to Ollama…".into(),
            connected: false,
            busy: false,
            tx,
            rx,
            cancel: Arc::new(AtomicBool::new(false)),
            activity: vec![],
            approval: None,
            error: library_error,
            show_settings: false,
            preview: None,
            ctx: cc.egui_ctx.clone(),
            library,
            search: String::new(),
            show_sources: true,
            settings_page: "General".into(),
            project_editor: None,
            project_is_new: false,
            reference_kind: "note".into(),
            reference_title: String::new(),
            reference_value: String::new(),
            show_reference: false,
            manage_chat: None,
            project_storage_ready,
            source_drawer: false,
        };
        app.refresh_files();
        app.refresh_models();
        app
    }

    fn refresh_models(&mut self) {
        self.status = "Connecting to Ollama…".into();
        let endpoint = self.settings.endpoint.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Event::Models(
                ollama::models(&endpoint).map_err(|e| format!("{e:#}")),
            ));
            ctx.request_repaint();
        });
    }
    fn refresh_files(&mut self) {
        self.files = Workspace::new(&self.session.workspace)
            .and_then(|w| w.files(".", 1500))
            .unwrap_or_default();
    }
    fn persist(&mut self) {
        self.session.updated = chrono::Local::now().to_rfc3339();
        if !self.session.entries.is_empty()
            && let Err(error) = store::save_session(&self.session)
        {
            self.error = Some(format!("Could not save this chat locally: {error}"));
        }
        if let Err(error) = store::save_settings(&self.settings) {
            self.error = Some(format!("Could not save settings: {error}"));
        }
        self.saved = store::sessions();
        self.library.import_sessions(&mut self.saved);
    }
    fn new_chat(&mut self) {
        if self.busy {
            return;
        }
        self.persist();
        let project_id = self.session.project_id.clone();
        self.session = Session::fresh(self.session.workspace.clone(), self.settings.model.clone());
        self.session.project_id = project_id;
        self.show_settings = false;
        self.activity.clear();
        self.error = None;
        self.input.clear();
        self.settings.edit_file = None;
        self.status = if self.connected {
            "Ready"
        } else {
            "Ollama is not connected"
        }
        .into();
    }
    fn choose_folder(&mut self) {
        if self.busy {
            return;
        }
        if !self.project_storage_ready {
            self.error=Some("Project storage needs attention. Resolve the project-library error before creating or changing projects.".into());
            return;
        }
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Choose a project folder")
            .pick_folder()
        {
            match Workspace::new(&folder) {
                Ok(workspace) => {
                    let root = workspace.root.to_string_lossy().into_owned();
                    if let Some(project) = self
                        .library
                        .projects
                        .iter()
                        .find(|p| crate::projects::same_root(&p.workspace, &root))
                        .cloned()
                    {
                        self.open_project(&project);
                    } else {
                        self.project_editor = Some(Project {
                            id: uuid::Uuid::new_v4().to_string(),
                            name: workspace
                                .root
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned(),
                            workspace: root,
                            instructions: String::new(),
                            references: vec![],
                        });
                        self.project_is_new = true;
                    }
                }
                Err(error) => self.error = Some(error.to_string()),
            }
        }
    }
    fn send(&mut self) {
        if self.busy || self.input.trim().is_empty() {
            return;
        }
        if !self.connected {
            self.error = Some("Start Ollama, then click Refresh models.".into());
            return;
        }
        if self.settings.mode != "chat" && Workspace::new(&self.session.workspace).is_err() {
            self.error = Some("Choose an existing project folder before starting.".into());
            return;
        }
        if self.settings.mode == "edit" && self.settings.edit_file.is_none() {
            self.error=Some("Choose a file in the sidebar and click Edit this file before sending an edit request.".into());
            return;
        }
        if self.input.len() > 12_000 {
            self.error=Some("The message exceeds 12,000 bytes. Split the task or put reference content in a project file.".into());
            return;
        }
        let content = std::mem::take(&mut self.input).trim().to_string();
        if self.session.entries.is_empty() {
            self.session.title = content.chars().take(55).collect();
        }
        self.session.model = self.settings.model.clone();
        self.session.checkpoint = None;
        self.session.entries.push(ChatEntry {
            role: "user".into(),
            content,
            at: chrono::Local::now().to_rfc3339(),
        });
        self.busy = true;
        self.error = None;
        self.activity.clear();
        self.approval = None;
        self.cancel = Arc::new(AtomicBool::new(false));
        self.persist();
        let settings = self.settings.clone();
        let root = self.session.workspace.clone();
        let history = self.session.entries.clone();
        let tx = self.tx.clone();
        let cancel = self.cancel.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || agent::run(settings, root, history, tx, cancel, ctx));
    }
    fn poll(&mut self) {
        let events: Vec<_> = self.rx.try_iter().collect();
        for event in events {
            match event {
                Event::Status(status) => self.status = status,
                Event::Note(message) => self.activity.push(Activity {
                    title: "Agent".into(),
                    detail: message,
                    state: "Plan".into(),
                }),
                Event::Final(content) => {
                    self.session.entries.push(ChatEntry {
                        role: "assistant".into(),
                        content,
                        at: chrono::Local::now().to_rfc3339(),
                    });
                    self.persist();
                }
                Event::Error(error) => {
                    self.error = Some(error.clone());
                    self.session.entries.push(ChatEntry {
                        role: "error".into(),
                        content: error,
                        at: chrono::Local::now().to_rfc3339(),
                    });
                    self.persist();
                }
                Event::Activity(activity) => {
                    self.activity.push(activity);
                    if self.activity.len() > 150 {
                        self.activity.remove(0);
                    }
                }
                Event::Approval(approval) => self.approval = Some(approval),
                Event::Finished => {
                    self.busy = false;
                    self.approval = None;
                    self.status = if self.cancel.load(Ordering::Relaxed) {
                        "Stopped"
                    } else {
                        "Ready"
                    }
                    .into();
                    self.refresh_files();
                    self.persist();
                }
                Event::Models(result) => match result {
                    Ok(models) => {
                        self.connected = true;
                        self.models = models;
                        if !self.models.contains(&self.settings.model) {
                            self.settings.model = self
                                .models
                                .iter()
                                .find(|m| m.starts_with("vmt-coding-agent"))
                                .or(self.models.first())
                                .cloned()
                                .unwrap_or_default();
                        }
                        self.status = if self.models.is_empty() {
                            "No chat models installed"
                        } else {
                            "Ready"
                        }
                        .into();
                    }
                    Err(error) => {
                        self.connected = false;
                        self.status = "Ollama is offline".into();
                        self.error = Some(error);
                    }
                },
                Event::Paused(checkpoint) => self.session.checkpoint = Some(checkpoint),
            }
        }
    }

    fn open_project(&mut self, project: &Project) {
        if self.busy {
            return;
        }
        self.persist();
        self.session = Session::fresh(project.workspace.clone(), self.settings.model.clone());
        self.session.project_id = Some(project.id.clone());
        self.settings.last_workspace = project.workspace.clone();
        self.settings.edit_file = None;
        self.activity.clear();
        self.input.clear();
        self.error = None;
        self.show_settings = false;
        self.refresh_files();
    }
    fn open_chat(&mut self, session: Session) {
        if self.busy {
            return;
        }
        self.persist();
        self.settings.model = session.model.clone();
        self.settings.last_workspace = session.workspace.clone();
        self.session = session;
        self.activity.clear();
        self.input.clear();
        self.settings.edit_file = None;
        self.error = None;
        self.show_settings = false;
        self.refresh_files();
    }
    fn project(&self) -> Option<&Project> {
        self.library
            .projects
            .iter()
            .find(|p| Some(&p.id) == self.session.project_id.as_ref())
    }
    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new(if self.show_sources { "☰" } else { "☷" }).frame(false))
                .on_hover_text("Toggle sources and activity")
                .clicked()
            {
                if ui.ctx().content_rect().width() <= 1180.0 {
                    self.source_drawer = !self.source_drawer;
                } else {
                    self.show_sources = !self.show_sources;
                }
            }
            let name = self
                .project()
                .map(|p| p.name.as_str())
                .unwrap_or("Local workspace");
            ui.label(RichText::new(format!("▱  {name}")).strong().size(15.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Project details").clicked() {
                    if let Some(project) = self.project().cloned() {
                        self.project_editor = Some(project);
                        self.project_is_new = false;
                    } else {
                        self.choose_folder();
                    }
                }
                ui.label(
                    RichText::new(if self.connected {
                        "● Local"
                    } else {
                        "○ Offline"
                    })
                    .color(MUTED)
                    .small(),
                );
            });
        });
    }
    fn chat_row(&mut self, ui: &mut egui::Ui, chat: Session) {
        let response = ui.add_sized(
            [ui.available_width(), 30.0],
            egui::Button::new(
                RichText::new(format!(
                    "{} {}",
                    if chat.pinned { "•" } else { "◯" },
                    clip(&chat.title, 38)
                ))
                .size(13.0),
            )
            .selected(self.session.id == chat.id)
            .frame(self.session.id == chat.id),
        );
        if response.clicked() {
            self.open_chat(chat.clone());
        }
        response.context_menu(|ui| {
            if ui.button("Rename / move to project…").clicked() {
                self.manage_chat = Some(chat.clone());
                ui.close();
            }
            if ui
                .button(if chat.pinned {
                    "Unpin chat"
                } else {
                    "Pin chat"
                })
                .clicked()
            {
                let mut updated = chat.clone();
                updated.pinned = !updated.pinned;
                self.save_chat(updated);
                ui.close();
            }
            if ui
                .button(if chat.archived {
                    "Restore chat"
                } else {
                    "Archive chat"
                })
                .clicked()
            {
                let mut updated = chat.clone();
                updated.archived = !updated.archived;
                self.save_chat(updated);
                ui.close();
            }
        });
    }
    fn save_chat(&mut self, chat: Session) {
        match store::save_session(&chat) {
            Ok(_) => {
                if self.session.id == chat.id {
                    self.session = chat;
                    self.settings.last_workspace = self.session.workspace.clone();
                    self.settings.edit_file = None;
                    self.activity.clear();
                    self.refresh_files();
                }
                self.saved = store::sessions();
                self.library.import_sessions(&mut self.saved);
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new(if self.show_settings {
                "Settings"
            } else {
                "VMT Agent"
            })
            .size(23.0)
            .strong(),
        );
        ui.add_space(18.0);
        if self.show_settings {
            if ui.button("‹  Back to chats").clicked() {
                self.show_settings = false;
            }
            ui.add_space(18.0);
            ui.label(RichText::new("Personal").color(MUTED));
            for (icon, page) in [
                ("⚙", "General"),
                ("◉", "Models & execution"),
                ("▱", "Projects & sources"),
                ("◷", "Archived chats"),
                ("▣", "Storage"),
            ] {
                if ui
                    .add_sized(
                        [ui.available_width(), 37.0],
                        egui::Button::new(format!("{icon}   {page}"))
                            .selected(self.settings_page == page)
                            .frame(self.settings_page == page),
                    )
                    .clicked()
                {
                    self.settings_page = page.into();
                }
            }
            return;
        }
        ui.add_enabled_ui(!self.busy, |ui| {
            if ui
                .add_sized(
                    [ui.available_width(), 36.0],
                    egui::Button::new("↗  New chat").frame(false),
                )
                .clicked()
            {
                self.new_chat();
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search chats and projects")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(17.0);
            let query = self.search.to_lowercase();
            let matches = |chat: &&Session| {
                !chat.archived && (query.is_empty() || chat.title.to_lowercase().contains(&query))
            };
            let pinned: Vec<_> = self
                .saved
                .iter()
                .filter(matches)
                .filter(|c| c.pinned)
                .cloned()
                .collect();
            egui::ScrollArea::vertical()
                .id_salt("project-chat-list")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if !pinned.is_empty() {
                        ui.label(RichText::new("Pinned").color(MUTED).strong());
                        ui.add_space(5.0);
                        for chat in pinned {
                            self.chat_row(ui, chat);
                        }
                        ui.add_space(18.0);
                    }
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Projects").color(MUTED).strong());
                        if ui
                            .small_button("+")
                            .on_hover_text("Create project from a folder")
                            .clicked()
                        {
                            self.choose_folder();
                        }
                    });
                    ui.add_space(6.0);
                    for project in self.library.projects.clone() {
                        let chats: Vec<_> = self
                            .saved
                            .iter()
                            .filter(matches)
                            .filter(|c| c.project_id.as_deref() == Some(project.id.as_str()))
                            .cloned()
                            .collect();
                        if !query.is_empty()
                            && chats.is_empty()
                            && !project.name.to_lowercase().contains(&query)
                        {
                            continue;
                        }
                        let header = egui::CollapsingHeader::new(
                            RichText::new(format!("▱  {}", project.name)).size(14.0),
                        )
                        .id_salt(&project.id)
                        .default_open(true)
                        .show(ui, |ui| {
                            if ui
                                .add_sized(
                                    [ui.available_width(), 28.0],
                                    egui::Button::new("+ New chat in project").frame(false),
                                )
                                .clicked()
                            {
                                self.open_project(&project);
                            }
                            for chat in chats {
                                self.chat_row(ui, chat);
                            }
                        });
                        header.header_response.context_menu(|ui| {
                            if ui.button("Project details & instructions").clicked() {
                                self.project_editor = Some(project.clone());
                                self.project_is_new = false;
                                ui.close();
                            }
                        });
                        ui.add_space(3.0);
                    }
                    if self.library.projects.is_empty() {
                        ui.label(
                            RichText::new(
                                "Create a project to organize chats, source files and references.",
                            )
                            .color(MUTED)
                            .small(),
                        );
                        if ui.button("+ Create project").clicked() {
                            self.choose_folder();
                        }
                    }
                    ui.add_space(18.0);
                    ui.label(RichText::new("Chats").color(MUTED).strong());
                    ui.add_space(5.0);
                    let chats: Vec<_> = self
                        .saved
                        .iter()
                        .filter(matches)
                        .filter(|c| c.project_id.is_none() && !c.pinned)
                        .cloned()
                        .collect();
                    for chat in chats {
                        self.chat_row(ui, chat);
                    }
                    if ui.button("+ Chat without a project").clicked() {
                        self.persist();
                        self.session = Session::fresh(String::new(), self.settings.model.clone());
                        self.settings.mode = "chat".into();
                        self.input.clear();
                        self.activity.clear();
                        self.files.clear();
                    }
                });
        });
    }
    fn sources_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("source-sidebar")
            .show(ui, |ui| {
                egui::Frame::new()
                    .fill(SURFACE)
                    .corner_radius(18)
                    .inner_margin(18)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            RichText::new(
                                self.project()
                                    .map(|p| p.name.clone())
                                    .unwrap_or("This chat".into()),
                            )
                            .color(MUTED)
                            .strong(),
                        );
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Sources").strong());
                            if ui
                                .add_enabled(
                                    self.project().is_some() && !self.busy,
                                    egui::Button::new("+").frame(false),
                                )
                                .on_hover_text("Add a project reference")
                                .clicked()
                            {
                                self.show_reference = true;
                            }
                        });
                        ui.add_space(8.0);
                        let references = self
                            .project()
                            .map(|p| p.references.clone())
                            .unwrap_or_default();
                        if references.is_empty() {
                            ui.label(
                                RichText::new(
                                    "Add source files, notes or reference links for this project.",
                                )
                                .size(13.0)
                                .color(MUTED),
                            );
                        }
                        for reference in references {
                            ui.horizontal_wrapped(|ui| {
                                let icon = match reference.kind.as_str() {
                                    "file" => "▤",
                                    "url" => "↗",
                                    _ => "≡",
                                };
                                if ui
                                    .add(
                                        egui::Button::new(format!("{icon}  {}", reference.title))
                                            .frame(false),
                                    )
                                    .on_hover_text(&reference.value)
                                    .clicked()
                                {
                                    self.preview = Some((
                                        if reference.kind == "file" {
                                            reference.value.clone()
                                        } else {
                                            format!("Reference · {}", reference.title)
                                        },
                                        if reference.kind == "file" {
                                            Workspace::new(&self.session.workspace)
                                                .and_then(|w| w.read(&reference.value))
                                                .unwrap_or_else(|e| e.to_string())
                                        } else {
                                            reference.value.clone()
                                        },
                                    ));
                                }
                                if ui
                                    .add_enabled(!self.busy, egui::Button::new("×").frame(false))
                                    .on_hover_text("Remove reference from project")
                                    .clicked()
                                {
                                    let mut next = self.library.clone();
                                    if let Some(project) = next
                                        .projects
                                        .iter_mut()
                                        .find(|p| Some(&p.id) == self.session.project_id.as_ref())
                                    {
                                        project.references.retain(|r| r.id != reference.id);
                                    }
                                    match next.save() {
                                        Ok(_) => self.library = next,
                                        Err(e) => self.error = Some(e.to_string()),
                                    }
                                }
                            });
                        }
                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(8.0);
                        egui::CollapsingHeader::new(format!(
                            "Project files ({})",
                            self.files.len()
                        ))
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.file_filter)
                                    .hint_text("Filter files…")
                                    .desired_width(f32::INFINITY),
                            );
                            for file in self
                                .files
                                .clone()
                                .into_iter()
                                .filter(|f| {
                                    self.file_filter.is_empty()
                                        || f.to_lowercase()
                                            .contains(&self.file_filter.to_lowercase())
                                })
                                .take(180)
                            {
                                if ui
                                    .add(egui::Button::new(clip(&file, 36)).frame(false))
                                    .on_hover_text(&file)
                                    .clicked()
                                {
                                    match Workspace::new(&self.session.workspace)
                                        .and_then(|w| w.read(&file))
                                    {
                                        Ok(text) => self.preview = Some((file, text)),
                                        Err(e) => self.error = Some(e.to_string()),
                                    }
                                }
                            }
                        });
                    });
                ui.add_space(22.0);
                self.activity_panel(ui);
            });
    }

    fn activity_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        ui.label(RichText::new("Activity").strong().size(16.0));
        ui.add_space(6.0);
        ui.label(
            RichText::new(&self.status)
                .color(if self.approval.is_some() {
                    AMBER
                } else {
                    MUTED
                })
                .size(12.0),
        );
        ui.add_space(13.0);
        ui.separator();
        ui.add_space(9.0);
        if self.activity.is_empty() {
            ui.label(RichText::new("Follow the work as it happens.").color(TEXT));
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "File reads, searches, proposed changes and command results appear here.",
                )
                .color(MUTED)
                .size(13.0),
            );
            ui.add_space(20.0);
            ui.label(RichText::new("You stay in control").strong());
            ui.label(
                RichText::new("Edits, commands and web requests pause for your review.")
                    .color(MUTED)
                    .size(13.0),
            );
        }
        egui::ScrollArea::vertical()
            .id_salt("activity-log")
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for (index, item) in self.activity.iter().enumerate() {
                    ui.push_id(index, |ui| {
                        let color = match item.state.as_str() {
                            "Failed" => Color32::from_rgb(246, 155, 155),
                            "Complete" => ACCENT,
                            "Running" => AMBER,
                            _ => MUTED,
                        };
                        ui.label(
                            RichText::new(format!("{}  {}", item.state, item.title))
                                .color(color)
                                .size(12.0)
                                .strong(),
                        );
                        egui::CollapsingHeader::new(clip(&item.detail.replace('\n', " "), 95))
                            .id_salt(index)
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&item.detail).monospace().size(11.0),
                                    )
                                    .wrap()
                                    .selectable(true),
                                );
                            });
                        ui.add_space(9.0);
                        ui.separator();
                        ui.add_space(9.0);
                    });
                }
            });
    }

    fn conversation(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("composer")
            .exact_size(if self.session.checkpoint.is_some() {
                268.0
            } else {
                224.0
            })
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(24, 14)),
            )
            .show(ui, |ui| {
                let width = ui.available_width().min(920.0);
                ui.horizontal(|ui| {
                    ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        if let Some(checkpoint) = self.session.checkpoint.clone() {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("Task paused: {}", checkpoint.reason))
                                        .small()
                                        .color(AMBER),
                                );
                                if ui
                                    .add_enabled(!self.busy, egui::Button::new("Continue task"))
                                    .clicked()
                                {
                                    self.input = checkpoint.continuation();
                                    self.send();
                                }
                            });
                            ui.add_space(7.0);
                        }
                        egui::Frame::new()
                            .fill(Color32::from_rgb(49, 49, 49))
                            .corner_radius(22)
                            .inner_margin(16)
                            .show(ui, |ui| {
                                let response = ui.add_enabled(
                                    !self.busy,
                                    egui::TextEdit::multiline(&mut self.input)
                                        .hint_text("Work with your local agent")
                                        .frame(egui::Frame::NONE)
                                        .desired_rows(3)
                                        .desired_width(f32::INFINITY),
                                );
                                let send_key = response.has_focus()
                                    && ui.input(|i| {
                                        i.modifiers.ctrl && i.key_pressed(egui::Key::Enter)
                                    });
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.add_enabled_ui(!self.busy, |ui| {
                                        if ui
                                            .add(egui::Button::new("+").frame(false))
                                            .on_hover_text("Add project source or reference")
                                            .clicked()
                                        {
                                            if self.project().is_some() {
                                                self.show_reference = true;
                                            } else {
                                                self.choose_folder();
                                            }
                                        }
                                        egui::ComboBox::from_id_salt("work-mode")
                                            .selected_text(match self.settings.mode.as_str() {
                                                "chat" => "Chat",
                                                "edit" => "Edit file",
                                                _ => "Agent",
                                            })
                                            .width(65.0)
                                            .show_ui(ui, |ui| {
                                                for (value, label) in [
                                                    ("agent", "Agent"),
                                                    ("chat", "Chat"),
                                                    ("edit", "Edit file"),
                                                ] {
                                                    ui.selectable_value(
                                                        &mut self.settings.mode,
                                                        value.into(),
                                                        label,
                                                    );
                                                }
                                            });
                                        ui.label(
                                            RichText::new(if self.settings.read_only {
                                                "Read only"
                                            } else {
                                                "Review actions"
                                            })
                                            .small()
                                            .color(MUTED),
                                        );
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if self.busy {
                                                if ui.button("■ Stop").clicked() {
                                                    self.cancel.store(true, Ordering::Relaxed);
                                                    self.status = "Stopping…".into();
                                                    if let Some(a) = self.approval.take() {
                                                        let _ = a.reply.send(false);
                                                    }
                                                }
                                                ui.spinner();
                                            } else if ui
                                                .add_enabled(
                                                    !self.input.trim().is_empty(),
                                                    egui::Button::new(
                                                        RichText::new("↑")
                                                            .strong()
                                                            .size(20.0)
                                                            .color(BG),
                                                    )
                                                    .fill(TEXT)
                                                    .corner_radius(20),
                                                )
                                                .on_hover_text("Send message · Ctrl+Enter")
                                                .clicked()
                                                || send_key
                                            {
                                                self.send();
                                            }
                                            ui.add_enabled_ui(!self.busy, |ui| {
                                                egui::ComboBox::from_id_salt("composer-model")
                                                    .selected_text(&self.settings.model)
                                                    .width(140.0)
                                                    .show_ui(ui, |ui| {
                                                        for model in &self.models {
                                                            ui.selectable_value(
                                                                &mut self.settings.model,
                                                                model.clone(),
                                                                model,
                                                            );
                                                        }
                                                    });
                                            });
                                        },
                                    );
                                });
                            });
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(if self.settings.mode == "edit" {
                                    self.settings
                                        .edit_file
                                        .as_deref()
                                        .unwrap_or("Choose a project file, then Edit this file")
                                } else {
                                    &self.status
                                })
                                .size(11.0)
                                .color(MUTED),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} steps / {} min",
                                            self.settings.max_steps, self.settings.task_minutes
                                        ))
                                        .size(11.0)
                                        .color(MUTED),
                                    );
                                },
                            );
                        });
                    });
                });
            });
        egui::ScrollArea::vertical().id_salt(("conversation",&self.session.id)).stick_to_bottom(true).auto_shrink([false,false]).show(ui,|ui| {
            let width = ui.available_width().min(920.0);
            let margin = ((ui.available_width()-width)/2.0).max(0.0);
            ui.horizontal(|ui| {ui.add_space(margin); ui.vertical(|ui| {
                ui.set_width(width); ui.add_space(35.0);
                if self.session.entries.is_empty() {
                    ui.add_space(65.0);
                    ui.label(RichText::new("What are we working on?").size(30.0).strong()); ui.add_space(14.0);
                    ui.label(RichText::new("Your projects. Your models. On this computer.").size(16.0).color(MUTED)); ui.add_space(32.0);
                    for (title,prompt) in [("Explore a project","Inspect this project and explain its architecture and checks. Do not change files or run commands."),("Find a bug","Review the project for concrete bugs. Reference files and explain how to verify each finding. Do not modify files."),("Make a change","Help me implement a change. Inspect the project instructions and main files, then ask what behavior I want to change.")] {
                        if ui.add_sized([width.min(470.0),42.0],egui::Button::new(title).fill(PANEL)).clicked(){self.input=prompt.into();} ui.add_space(9.0);
                    }
                    if self.session.workspace.is_empty() && ui.button("+ Create your first project").clicked(){self.choose_folder();}
                }
                for (index,entry) in self.session.entries.iter().enumerate() {
                    ui.push_id(index,|ui| {
                        let user=entry.role=="user";
                        if user {
                            egui::Frame::new().fill(SURFACE).corner_radius(18).inner_margin(16).show(ui,|ui|{ui.set_max_width(width*0.88);render_message(ui,&entry.content);});
                        } else {
                            ui.label(RichText::new(if entry.role=="error" {"Task needs attention"} else {"VMT Agent"}).strong().small().color(if entry.role=="error"{AMBER}else{MUTED})); ui.add_space(10.0);
                            render_message(ui,&entry.content);
                            ui.add_space(8.0); if ui.small_button("Copy").clicked(){ui.ctx().copy_text(entry.content.clone());}
                        }
                        ui.add_space(30.0);
                    });
                }
                ui.add_space(20.0);
            });});
        });
    }
    fn settings_view(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().id_salt("settings-page").show(ui, |ui| {
            let width = ui.available_width().min(900.0);
            ui.horizontal(|ui| {ui.add_space(((ui.available_width()-width)/2.0).max(0.0)); ui.vertical(|ui| {
                ui.set_width(width); ui.add_space(38.0); ui.heading(RichText::new(&self.settings_page).size(31.0)); ui.add_space(36.0);
                if self.settings_page == "Archived chats" {
                    let chats: Vec<_> = self.saved.iter().filter(|c|c.archived).cloned().collect();
                    if chats.is_empty(){ui.label("No archived chats. Right-click a chat to archive it.");}
                    for chat in chats {ui.horizontal(|ui|{ui.label(&chat.title); if ui.button("Restore").clicked(){let mut chat=chat.clone();chat.archived=false;self.save_chat(chat);}});}
                } else if self.settings_page == "Storage" {
                    ui.label("Local data folder");ui.label(RichText::new(store::data_dir().display().to_string()).monospace());ui.add_space(12.0);
                    ui.label("Chats, projects, references and checkpoints are stored locally as plain JSON. Back up this folder along with your project source. Moving a chat organizes its history and changes the workspace used by future tasks; it never moves source files.");
                } else if self.settings_page == "Projects & sources" {
                    ui.label("Projects connect a folder, instructions, source references and related chats.");ui.add_space(16.0);
                    if ui.button("+ Create project from folder").clicked(){self.choose_folder();}
                    for project in self.library.projects.clone(){ui.add_space(15.0);ui.horizontal(|ui|{ui.label(&project.name);if ui.button("Edit details").clicked(){self.project_editor=Some(project.clone());self.project_is_new=false;}});ui.label(RichText::new(&project.workspace).small().color(MUTED));}
                } else {
                    ui.add_enabled_ui(!self.busy,|ui| {
                        if self.settings_page == "General" {
                            ui.label(RichText::new("Permissions").strong());ui.add_space(12.0);
                            egui::Frame::new().fill(PANEL).corner_radius(18).stroke(Stroke::new(1.0,BORDER)).inner_margin(20).show(ui,|ui| {
                                ui.set_width(width-40.0);
                                ui.checkbox(&mut self.settings.read_only,"Read-only workspace");ui.label(RichText::new("Inspect files without making changes or running commands.").color(MUTED));ui.add_space(14.0);ui.separator();ui.add_space(14.0);
                                ui.label(RichText::new("Review each action").strong());ui.label(RichText::new("Edits, commands, external tools and internet requests always pause for your approval. Longer tasks do not change this.").color(MUTED));
                                ui.add_space(14.0);ui.separator();ui.add_space(14.0);ui.checkbox(&mut self.settings.web_enabled,"Enable reviewed internet tools");ui.label(RichText::new("Links in Sources stay offline until an explicit reviewed fetch. Web search sends the approved query to Ollama's search service when configured.").color(MUTED));
                            });
                            ui.add_space(30.0);ui.label(RichText::new("General").strong());ui.add_space(12.0);
                            egui::Frame::new().fill(PANEL).corner_radius(18).stroke(Stroke::new(1.0,BORDER)).inner_margin(20).show(ui,|ui|{ui.set_width(width-40.0);ui.label("Appearance");ui.label(RichText::new("Dark · follows this workspace's charcoal design").color(MUTED));ui.add_space(16.0);ui.separator();ui.add_space(16.0);ui.label("Agent environment");ui.label(RichText::new("Windows native · selected project folder").color(MUTED));ui.add_space(16.0);ui.separator();ui.add_space(16.0);ui.label("Send shortcut");ui.label(RichText::new("Ctrl+Enter").color(MUTED));});
                        } else {
                            egui::Frame::new().fill(PANEL).corner_radius(18).stroke(Stroke::new(1.0,BORDER)).inner_margin(20).show(ui,|ui| {
                                ui.set_width(width-40.0);ui.label(RichText::new("Local inference").strong());ui.label("Ollama server");ui.add(egui::TextEdit::singleline(&mut self.settings.endpoint).desired_width(f32::INFINITY));ui.label(RichText::new("Only a loopback Ollama endpoint is accepted.").small().color(MUTED));ui.add_space(18.0);
                                ui.horizontal(|ui|{ui.label("Context tokens");ui.add(egui::DragValue::new(&mut self.settings.context_tokens).range(4096..=32768).speed(1024.0));});
                                ui.horizontal(|ui|{ui.label("Output tokens");ui.add(egui::DragValue::new(&mut self.settings.output_tokens).range(512..=8192).speed(256.0));});
                                ui.label(RichText::new("Larger context uses more memory. Longer tasks can keep 8,192 context tokens; they do not require a larger context window.").small().color(MUTED));ui.add_space(20.0);ui.separator();ui.add_space(20.0);
                                ui.label(RichText::new("Execution budget").strong());ui.horizontal(|ui|{ui.label("Steps per turn");ui.add(egui::DragValue::new(&mut self.settings.max_steps).range(1..=256));});ui.horizontal(|ui|{ui.label("Minutes per turn");ui.add(egui::DragValue::new(&mut self.settings.task_minutes).range(1..=120));});
                                ui.label(RichText::new("A step is a tool action or final-answer attempt. The old 12-step limit was an application budget. Tasks pause at a budget or repeated-action boundary and can be continued after fresh inspection.").color(MUTED));ui.add_space(18.0);
                                ui.horizontal(|ui|{ui.label("Tool protocol");egui::ComboBox::from_id_salt("protocol").selected_text(&self.settings.protocol).show_ui(ui,|ui|{for value in ["auto","native","json"]{ui.selectable_value(&mut self.settings.protocol,value.into(),value);}});});
                            });
                        }
                        ui.add_space(22.0);if ui.button("Save settings").clicked(){match ollama::local_endpoint(&self.settings.endpoint){Ok(_)=>{self.persist();self.refresh_models();self.status="Settings saved".into();},Err(e)=>self.error=Some(e.to_string())}}
                    });
                }
                ui.add_space(40.0);
            });});
        });
    }

    fn organization_dialogs(&mut self, ctx: &egui::Context) {
        if let Some(mut project) = self.project_editor.clone() {
            let mut open = true;
            let mut save = false;
            egui::Window::new(if self.project_is_new {"Create project"}else{"Project details"}).open(&mut open).default_width(560.0).collapsible(false).show(ctx,|ui| {
                ui.label("Project name");ui.add(egui::TextEdit::singleline(&mut project.name).desired_width(f32::INFINITY).char_limit(100));
                ui.add_space(12.0);ui.label("Source folder");ui.label(RichText::new(&project.workspace).small().color(MUTED));
                ui.add_space(16.0);ui.label("Project instructions");ui.label(RichText::new("Guidance for tasks in this project. This cannot grant file, command or internet permissions.").small().color(MUTED));
                ui.add(egui::TextEdit::multiline(&mut project.instructions).desired_rows(5).desired_width(f32::INFINITY).char_limit(5000));
                ui.add_space(15.0);if ui.add_enabled(!self.busy && self.project_storage_ready,egui::Button::new(if self.project_is_new {"Create project"}else{"Save project"})).clicked(){save=true;}
            });
            if save {
                let mut next = self.library.clone();
                let result = (|| -> anyhow::Result<Project> {
                    if self.project_is_new {
                        let id = next.create(&project.name, &project.workspace)?;
                        let created = next.projects.iter_mut().find(|p| p.id == id).unwrap();
                        created.instructions = project.instructions.clone();
                        project = created.clone();
                    } else {
                        let current = next
                            .projects
                            .iter_mut()
                            .find(|p| p.id == project.id)
                            .ok_or_else(|| anyhow::anyhow!("Project no longer exists"))?;
                        current.name = project.name.trim().into();
                        current.instructions = project.instructions.clone();
                    }
                    next.save()?;
                    Ok(project.clone())
                })();
                match result {
                    Ok(project) => {
                        self.library = next;
                        self.project_editor = None;
                        if self.project_is_new {
                            self.open_project(&project);
                            self.persist();
                        }
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
            } else if open {
                self.project_editor = Some(project);
            } else {
                self.project_editor = None;
            }
        }
        if self.show_reference {
            let mut open = true;
            let mut save = false;
            let mut pick = false;
            egui::Window::new("Add project reference").open(&mut open).default_width(520.0).collapsible(false).show(ctx,|ui| {
                ui.label("Reference type");egui::ComboBox::from_id_salt("reference-kind").selected_text(&self.reference_kind).show_ui(ui,|ui|{for (kind,label) in [("note","Note"),("file","Project file"),("url","Web link")]{ui.selectable_value(&mut self.reference_kind,kind.into(),label);}});
                ui.add_space(12.0);ui.label("Title");ui.add(egui::TextEdit::singleline(&mut self.reference_title).desired_width(f32::INFINITY).char_limit(140));ui.add_space(12.0);
                if self.reference_kind=="note" {ui.label("Reference notes");ui.add(egui::TextEdit::multiline(&mut self.reference_value).desired_rows(5).desired_width(f32::INFINITY).char_limit(5000));}
                else {ui.label(if self.reference_kind=="file" {"Relative project path"}else{"HTTPS URL"});ui.add(egui::TextEdit::singleline(&mut self.reference_value).desired_width(f32::INFINITY));if self.reference_kind=="file" && ui.button("Choose project file…").clicked(){pick=true;}}
                ui.add_space(10.0);ui.label(RichText::new("References are task context. Adding a link does not fetch it or enable internet access. File references stay inside this project and are read again for each Agent task.").small().color(MUTED));
                if ui.add_enabled(!self.busy && self.project_storage_ready,egui::Button::new("Add reference")).clicked(){save=true;}
            });
            if pick
                && let Some(file) = rfd::FileDialog::new()
                    .set_directory(&self.session.workspace)
                    .pick_file()
            {
                match Workspace::new(&self.session.workspace)
                    .and_then(|w| crate::projects::reference_path(&w, &file))
                {
                    Ok(path) => self.reference_value = path,
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
            if save {
                let mut next = self.library.clone();
                let result = (|| -> anyhow::Result<()> {
                    anyhow::ensure!(
                        !self.reference_title.trim().is_empty()
                            && !self.reference_value.trim().is_empty(),
                        "Enter a title and reference content"
                    );
                    if self.reference_kind == "file" {
                        Workspace::new(&self.session.workspace)?.read(&self.reference_value)?;
                    }
                    if self.reference_kind == "url" {
                        let url = reqwest::Url::parse(&self.reference_value)?;
                        anyhow::ensure!(
                            url.scheme() == "https"
                                && url.host_str().is_some()
                                && url.username().is_empty()
                                && url.password().is_none(),
                            "Use an HTTPS URL without credentials"
                        );
                    }
                    let project = next
                        .projects
                        .iter_mut()
                        .find(|p| Some(&p.id) == self.session.project_id.as_ref())
                        .ok_or_else(|| {
                            anyhow::anyhow!("Select a project before adding references")
                        })?;
                    project.references.push(Reference {
                        id: uuid::Uuid::new_v4().to_string(),
                        kind: self.reference_kind.clone(),
                        title: self.reference_title.trim().into(),
                        value: self.reference_value.clone(),
                    });
                    next.save()
                })();
                match result {
                    Ok(_) => {
                        self.library = next;
                        self.show_reference = false;
                        self.reference_title.clear();
                        self.reference_value.clear();
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
            } else {
                self.show_reference = open;
            }
        }
        if let Some(mut chat) = self.manage_chat.clone() {
            let mut open = true;
            let mut save = false;
            egui::Window::new("Organize chat").open(&mut open).default_width(500.0).collapsible(false).show(ctx,|ui| {
                ui.label("Chat name");ui.add(egui::TextEdit::singleline(&mut chat.title).desired_width(f32::INFINITY).char_limit(120));ui.add_space(14.0);
                ui.label("Project");let selected=self.library.projects.iter().find(|p|Some(&p.id)==chat.project_id.as_ref()).map(|p|p.name.as_str()).unwrap_or("No project");
                egui::ComboBox::from_id_salt("move-chat-project").selected_text(selected).show_ui(ui,|ui|{ui.selectable_value(&mut chat.project_id,None,"No project");for p in &self.library.projects{ui.selectable_value(&mut chat.project_id,Some(p.id.clone()),&p.name);}});
                ui.add_space(12.0);ui.label(RichText::new("Moving a chat keeps its messages. Future tasks use the destination folder and references. A saved task continuation is cleared when its workspace changes. No source files are moved.").color(MUTED));ui.add_space(14.0);
                if ui.add_enabled(!self.busy && !chat.title.trim().is_empty(),egui::Button::new("Save chat")).clicked(){save=true;}
            });
            if save {
                let previous = self
                    .saved
                    .iter()
                    .find(|s| s.id == chat.id)
                    .and_then(|s| s.project_id.clone());
                if previous != chat.project_id {
                    let project = chat.project_id.clone();
                    if let Err(e) = self.library.move_chat(&mut chat, project.as_deref()) {
                        self.error = Some(e.to_string());
                        return;
                    }
                }
                self.save_chat(chat);
                self.manage_chat = None;
            } else if open {
                self.manage_chat = Some(chat);
            } else {
                self.manage_chat = None;
            }
        }
    }

    fn overlays(&mut self, ctx: &egui::Context) {
        let mut edit_selected = None;
        if let Some((path, content)) = &self.preview {
            let mut open = true;
            egui::Window::new(format!("Preview · {path}"))
                .open(&mut open)
                .default_size([820.0, 560.0])
                .show(ctx, |ui| {
                    ui.label(RichText::new("Read-only preview").color(MUTED));
                    if ui
                        .add_enabled(
                            !self.busy && self.files.contains(path),
                            egui::Button::new("Edit this file"),
                        )
                        .clicked()
                    {
                        edit_selected = Some(path.clone());
                    }
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(content).monospace().size(12.0))
                                .selectable(true),
                        );
                    });
                });
            if !open {
                self.preview = None;
            }
        }
        if let Some(path) = edit_selected {
            self.settings.edit_file = Some(path);
            self.settings.mode = "edit".into();
            self.preview = None;
        }
        let mut decision = None;
        if let Some(approval) = &self.approval {
            egui::Modal::new(egui::Id::unique("action-review")).show(ctx,|ui| {
                ui.set_width(720.0_f32.min(ctx.content_rect().width()-80.0));
                ui.label(RichText::new("Review before continuing").size(23.0).strong());ui.add_space(8.0);
                ui.label(RichText::new(&approval.title).color(AMBER).strong());
                match &approval.proposal {
                    Proposal::File {path,..}=>{ui.label(format!("Project file: {path}"));},
                    Proposal::Command {..}=>{ui.label("This command runs with your Windows account in the selected project. It is not an OS sandbox.");},
                    Proposal::Web {..}=>{ui.label("This request connects to the internet. Review the destination and any query text before sending.");},
                    Proposal::MemoryNote {..}=>{ui.label("This note will be saved in local project memory. Do not approve confidential or incorrect information.");},
                    Proposal::Mcp {..}=>{ui.label("This starts an external MCP server executable with your Windows account. It is not sandboxed. Review server, operation, tool and arguments.");},
                    Proposal::KnowledgeIndex {..}=>{ui.label("This sends selected allowed project text to LOCAL Ollama embeddings and saves plaintext snippets and vectors under LocalAppData.");},
                    Proposal::Workflow {..}=>{ui.label("This changes persistent local workflow state only, not project source files.");},
                }
                ui.add_space(12.0);
                egui::Frame::new().fill(BG).inner_margin(12).show(ui,|ui| {
                    egui::ScrollArea::both().max_height((ctx.content_rect().height()*0.5).max(200.0)).show(ui,|ui| {
                        for line in approval.detail.lines() {
                            let color=if line.starts_with('+'){ACCENT}else if line.starts_with('-'){Color32::from_rgb(249,164,164)}else{TEXT};
                            ui.add(egui::Label::new(RichText::new(line).monospace().size(12.0).color(color)).selectable(true));
                        }
                    });
                });
                ui.add_space(15.0);ui.horizontal(|ui| {
                    if ui.add_sized([120.0,36.0],egui::Button::new("Deny")).clicked(){decision=Some(false);}
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                        if ui.add_sized([170.0,36.0],egui::Button::new(RichText::new("Approve once").color(BG).strong()).fill(ACCENT)).clicked(){decision=Some(true);}
                    });
                });
            });
        }
        if let Some(answer) = decision
            && let Some(approval) = self.approval.take()
        {
            let _ = approval.reply.send(answer);
            self.status = if answer {
                "Running approved action"
            } else {
                "Action denied"
            }
            .into();
        }
    }
}

impl eframe::App for Desktop {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll();
        let ctx = ui.ctx().clone();
        egui::Panel::left("navigation-rail")
            .exact_size(56.0)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(32, 32, 32))
                    .inner_margin(9),
            )
            .show(ui, |ui| {
                ui.add_space(12.0);
                if ui
                    .add_sized(
                        [36.0, 36.0],
                        egui::Button::new(RichText::new("V").strong().size(22.0))
                            .fill(SURFACE)
                            .corner_radius(10),
                    )
                    .on_hover_text("Local Agent")
                    .clicked()
                {
                    self.show_settings = false;
                }
                ui.add_space(18.0);
                if ui
                    .add_sized([36.0, 36.0], egui::Button::new("◯").frame(false))
                    .on_hover_text("Chats")
                    .clicked()
                {
                    self.show_settings = false;
                }
                if ui
                    .add_enabled(
                        !self.busy,
                        egui::Button::new("▱")
                            .frame(false)
                            .min_size(egui::vec2(36.0, 36.0)),
                    )
                    .on_hover_text("Create project")
                    .clicked()
                {
                    self.choose_folder();
                }
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
                if ui
                    .add_sized([36.0, 36.0], egui::Button::new("⚙").frame(false))
                    .on_hover_text("Settings")
                    .clicked()
                {
                    self.show_settings = true;
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.add_space(12.0);
                    ui.label(RichText::new("Local").small().color(MUTED));
                });
            });
        egui::Panel::left("sidebar")
            .default_size(if self.show_settings { 270.0 } else { 280.0 })
            .size_range(220.0..=380.0)
            .resizable(true)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(16))
            .show(ui, |ui| self.sidebar(ui));
        if !self.show_settings {
            egui::Panel::top("chat-header")
                .exact_size(56.0)
                .frame(
                    egui::Frame::new()
                        .fill(BG)
                        .inner_margin(egui::Margin::symmetric(20, 15)),
                )
                .show(ui, |ui| self.header(ui));
            if self.show_sources && ctx.content_rect().width() > 1180.0 {
                egui::Panel::right("sources-and-activity")
                    .default_size(300.0)
                    .size_range(260.0..=420.0)
                    .resizable(true)
                    .frame(egui::Frame::new().fill(BG).inner_margin(16))
                    .show(ui, |ui| self.sources_panel(ui));
            }
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(if self.show_settings { 24 } else { 8 }),
            )
            .show(ui, |ui| {
                if let Some(error) = self.error.clone() {
                    egui::Frame::new()
                        .fill(SURFACE)
                        .inner_margin(12)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(error).color(AMBER));
                                if ui.small_button("Dismiss").clicked() {
                                    self.error = None;
                                }
                            });
                        });
                }
                if self.show_settings {
                    self.settings_view(ui);
                } else {
                    self.conversation(ui);
                }
            });
        self.organization_dialogs(&ctx);
        if self.source_drawer {
            let mut open = true;
            egui::Window::new("Sources & activity")
                .open(&mut open)
                .default_size([340.0, 520.0])
                .show(&ctx, |ui| self.sources_panel(ui));
            self.source_drawer = open;
        }
        self.overlays(&ctx);
        if self.busy {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(a) = self.approval.take() {
            let _ = a.reply.send(false);
        }
        self.persist();
    }
}

fn render_message(ui: &mut egui::Ui, text: &str) {
    let mut code = false;
    let mut buffer = String::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            if !buffer.is_empty() {
                render_block(ui, &buffer, code);
                buffer.clear();
            }
            code = !code;
            continue;
        }
        buffer.push_str(line);
        buffer.push('\n');
    }
    if !buffer.is_empty() {
        render_block(ui, &buffer, code);
    }
}
fn render_block(ui: &mut egui::Ui, text: &str, code: bool) {
    if code {
        egui::Frame::new()
            .fill(Color32::from_rgb(32, 32, 32))
            .corner_radius(5)
            .inner_margin(12)
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.small_button("Copy code").clicked() {
                        ui.ctx().copy_text(text.to_string());
                    }
                });
                egui::ScrollArea::horizontal()
                    .id_salt(ui.next_auto_id())
                    .show(ui, |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(text).monospace().size(13.0))
                                .selectable(true),
                        );
                    });
            });
    } else {
        ui.add(
            egui::Label::new(RichText::new(text).size(15.0).color(TEXT))
                .wrap()
                .selectable(true),
        );
    }
}

fn theme(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (name, path, family) in [
        (
            "Segoe UI",
            "C:/Windows/Fonts/segoeui.ttf",
            egui::FontFamily::Proportional,
        ),
        (
            "Consolas",
            "C:/Windows/Fonts/consola.ttf",
            egui::FontFamily::Monospace,
        ),
        (
            "Symbols",
            "C:/Windows/Fonts/seguisym.ttf",
            egui::FontFamily::Proportional,
        ),
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.into(), Arc::new(egui::FontData::from_owned(bytes)));
            let family_fonts = fonts.families.entry(family).or_default();
            if name == "Symbols" {
                family_fonts.push(name.into());
            } else {
                family_fonts.insert(0, name.into());
            }
        }
    }
    ctx.set_fonts(fonts);
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = BG;
    visuals.faint_bg_color = SURFACE;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = Color32::from_rgb(64, 64, 64);
    visuals.hyperlink_color = ACCENT;
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
    ] {
        widget.bg_fill = SURFACE;
        widget.fg_stroke = Stroke::new(1.0, TEXT);
        widget.bg_stroke = Stroke::new(1.0, BORDER);
        widget.corner_radius = egui::CornerRadius::same(5);
    }
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_visuals(visuals);
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        style.spacing.button_padding = egui::vec2(11.0, 7.0);
        style.spacing.interact_size.y = 30.0;
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    });
}
