use crate::{
    agent, ollama, store,
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

const BG: Color32 = Color32::from_rgb(22, 31, 39);
const PANEL: Color32 = Color32::from_rgb(28, 40, 50);
const SURFACE: Color32 = Color32::from_rgb(35, 49, 61);
const TEXT: Color32 = Color32::from_rgb(231, 239, 243);
const MUTED: Color32 = Color32::from_rgb(151, 172, 185);
const ACCENT: Color32 = Color32::from_rgb(101, 206, 214);
const BORDER: Color32 = Color32::from_rgb(53, 70, 83);
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
}

impl Desktop {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Option<String>) -> Self {
        theme(&cc.egui_ctx);
        let mut settings = store::load_settings();
        if let Some(path) = initial {
            settings.last_workspace = path;
        }
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            session: Session::fresh(settings.last_workspace.clone(), settings.model.clone()),
            saved: store::sessions(),
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
            error: None,
            show_settings: false,
            preview: None,
            ctx: cc.egui_ctx.clone(),
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
    }
    fn new_chat(&mut self) {
        self.persist();
        self.session = Session::fresh(self.session.workspace.clone(), self.settings.model.clone());
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
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Choose a project folder")
            .pick_folder()
        {
            match Workspace::new(&folder) {
                Ok(workspace) => {
                    self.persist();
                    self.settings.last_workspace = workspace.root.to_string_lossy().to_string();
                    self.session = Session::fresh(
                        self.settings.last_workspace.clone(),
                        self.settings.model.clone(),
                    );
                    self.activity.clear();
                    self.settings.edit_file = None;
                    self.input.clear();
                    self.error = None;
                    self.refresh_files();
                    self.persist();
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
            }
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            egui::Frame::new()
                .fill(ACCENT)
                .corner_radius(5)
                .inner_margin(7)
                .show(ui, |ui| {
                    ui.label(RichText::new("VMT").strong().color(BG));
                });
            ui.add_space(8.0);
            ui.label(RichText::new("Local Agent").size(21.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!self.busy, egui::Button::new("Settings"))
                    .clicked()
                {
                    self.show_settings = true;
                }
                if ui
                    .add_enabled(!self.busy, egui::Button::new("Refresh models"))
                    .clicked()
                {
                    self.refresh_models();
                }
                ui.label(
                    RichText::new(if self.connected {
                        "●  Local Ollama"
                    } else {
                        "○  Offline"
                    })
                    .color(if self.connected { ACCENT } else { MUTED }),
                );
                ui.add_enabled_ui(!self.busy, |ui| {
                    egui::ComboBox::from_id_salt("model-picker")
                        .selected_text(&self.settings.model)
                        .width(240.0)
                        .show_ui(ui, |ui| {
                            for model in &self.models {
                                ui.selectable_value(&mut self.settings.model, model.clone(), model);
                            }
                        });
                });
            });
        });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        ui.add_enabled_ui(!self.busy, |ui| {
            if ui
                .add_sized(
                    [ui.available_width(), 34.0],
                    egui::Button::new("+  New conversation"),
                )
                .clicked()
            {
                self.new_chat();
            }
            ui.add_space(20.0);
            ui.label(RichText::new("Project").strong().size(15.0));
            ui.add_space(7.0);
            let project = std::path::Path::new(&self.session.workspace)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "No project selected".into());
            ui.label(RichText::new(project).color(ACCENT));
            if !self.session.workspace.is_empty() {
                ui.label(
                    RichText::new(&self.session.workspace)
                        .size(11.0)
                        .color(MUTED),
                );
            }
            if ui.button("Open project folder…").clicked() {
                self.choose_folder();
            }
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Files").strong());
                if ui.small_button("Refresh").clicked() {
                    self.refresh_files();
                }
            });
            ui.add(
                egui::TextEdit::singleline(&mut self.file_filter)
                    .hint_text("Filter files…")
                    .desired_width(f32::INFINITY),
            );
            let files = self
                .files
                .iter()
                .filter(|f| {
                    self.file_filter.is_empty()
                        || f.to_lowercase().contains(&self.file_filter.to_lowercase())
                })
                .take(250)
                .cloned()
                .collect::<Vec<_>>();
            egui::ScrollArea::vertical()
                .id_salt("project-files")
                .max_height((ui.available_height() * 0.48).max(100.0))
                .show(ui, |ui| {
                    for file in files {
                        if ui
                            .selectable_label(false, RichText::new(&file).size(12.0))
                            .clicked()
                        {
                            match Workspace::new(&self.session.workspace)
                                .and_then(|w| w.read(&file))
                            {
                                Ok(content) => self.preview = Some((file, content)),
                                Err(error) => self.error = Some(error.to_string()),
                            }
                        }
                    }
                    if self.files.is_empty() {
                        ui.label(
                            RichText::new("Open a project to browse its files.")
                                .color(MUTED)
                                .size(12.0),
                        );
                    }
                });
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(9.0);
            ui.label(RichText::new("Recent conversations").strong());
            let saved = self.saved.iter().take(12).cloned().collect::<Vec<_>>();
            egui::ScrollArea::vertical()
                .id_salt("saved-sessions")
                .show(ui, |ui| {
                    for session in saved {
                        if ui
                            .selectable_label(
                                self.session.id == session.id,
                                RichText::new(&session.title).size(12.0),
                            )
                            .on_hover_text(&session.workspace)
                            .clicked()
                        {
                            self.persist();
                            self.settings.model = session.model.clone();
                            self.settings.last_workspace = session.workspace.clone();
                            self.session = session;
                            self.activity.clear();
                            self.settings.edit_file = None;
                            self.error = None;
                            self.refresh_files();
                        }
                    }
                });
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
        let available = ui.available_height();
        egui::Panel::bottom("composer")
            .exact_size(195.0)
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(22, 12)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!self.busy, |ui| {
                        egui::ComboBox::from_id_salt("work-mode")
                            .selected_text(match self.settings.mode.as_str() {
                                "chat" => "Chat",
                                "edit" => "Edit file",
                                _ => "Agent",
                            })
                            .width(100.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.settings.mode,
                                    "agent".into(),
                                    "Agent",
                                );
                                ui.selectable_value(&mut self.settings.mode, "chat".into(), "Chat");
                                ui.selectable_value(
                                    &mut self.settings.mode,
                                    "edit".into(),
                                    "Edit file",
                                );
                            });
                    });
                    ui.label(
                        RichText::new(if self.settings.read_only {
                            "Read only"
                        } else {
                            "Review changes"
                        })
                        .color(ACCENT)
                        .size(12.0),
                    );
                    ui.label(
                        RichText::new(if self.settings.web_enabled {
                            "Internet tools on"
                        } else {
                            "Internet tools off"
                        })
                        .color(MUTED)
                        .size(12.0),
                    );
                });
                if self.settings.mode == "edit" {
                    ui.label(
                        RichText::new(match &self.settings.edit_file {
                            Some(path) => format!("Editing: {path}"),
                            None => {
                                "Select a file in the sidebar, then choose Edit this file.".into()
                            }
                        })
                        .size(11.0)
                        .color(AMBER),
                    );
                }
                ui.add_space(7.0);
                let response =
                    egui::ScrollArea::vertical()
                        .id_salt("prompt-scroll")
                        .max_height(76.0)
                        .show(ui, |ui| {
                            ui.add_enabled(
                    !self.busy,
                    egui::TextEdit::multiline(&mut self.input)
                        .hint_text("Ask about this project, describe a change, or paste an error…")
                        .desired_width(f32::INFINITY)
                        .desired_rows(3),
                )
                        })
                        .inner;
                let send_key = response.has_focus()
                    && ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Ctrl+Enter to send").size(11.0).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.busy {
                            if ui.button("Stop").clicked() {
                                self.cancel.store(true, Ordering::Relaxed);
                                self.status = "Stopping when the current response yields…".into();
                                if let Some(a) = self.approval.take() {
                                    let _ = a.reply.send(false);
                                }
                            }
                            ui.spinner();
                        } else if ui
                            .add_enabled(
                                !self.input.trim().is_empty(),
                                egui::Button::new(RichText::new("Send message").color(BG).strong())
                                    .fill(ACCENT),
                            )
                            .clicked()
                            || send_key
                        {
                            self.send();
                        }
                    });
                });
            });
        egui::ScrollArea::vertical().id_salt("conversation").stick_to_bottom(true).auto_shrink([false,false]).show(ui,|ui| {
            ui.set_min_width(ui.available_width());
            egui::Frame::new().inner_margin(egui::Margin::symmetric(26,16)).show(ui,|ui| {
                if self.session.entries.is_empty() {
                    ui.add_space((available*0.15).clamp(25.0,100.0));
                    ui.label(RichText::new("Build with your local models.").size(29.0).strong());ui.add_space(12.0);
                    ui.label(RichText::new("Explore a codebase, work through a bug, or make a change. Your project and model stay on this computer.").size(16.0).color(MUTED));
                    ui.add_space(26.0);
                    for (title,prompt) in [
                        ("Explain this project","Inspect the files and explain this project's purpose, architecture, and how to run its checks. Do not change files or run commands."),
                        ("Review the code","Inspect this project and identify concrete bugs or missing error handling. Reference file paths. Do not modify files."),
                        ("Make a change","I want to make a change to this project. First inspect its instructions and main files, then ask me which behavior I want to change."),
                    ] {
                        if ui.add_sized([ui.available_width().min(480.0),44.0],egui::Button::new(title).fill(SURFACE)).clicked(){self.input=prompt.into();}
                        ui.add_space(9.0);
                    }
                    if self.session.workspace.is_empty(){ui.add_space(12.0);if ui.button("Choose a project folder to begin").clicked(){self.choose_folder();}}
                } else {
                    for (index,entry) in self.session.entries.iter().enumerate() {
                        ui.push_id(index,|ui| {
                            let user=entry.role=="user";let error=entry.role=="error";
                            egui::Frame::new().fill(if user{SURFACE}else{BG}).inner_margin(16).corner_radius(7).show(ui,|ui| {
                                ui.horizontal(|ui| {ui.label(RichText::new(if user{"You"}else if error{"Could not complete this turn"}else{"Local Agent"}).color(if error{AMBER}else{ACCENT}).size(13.0).strong());
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{if ui.small_button("Copy").clicked(){ui.ctx().copy_text(entry.content.clone());}});});
                                ui.add_space(8.0);render_message(ui,&entry.content);
                            });ui.add_space(15.0);
                        });
                    }
                    if self.busy {ui.horizontal(|ui| {ui.spinner();ui.label(RichText::new(&self.status).color(MUTED));});ui.add_space(15.0);}
                }
            });
        });
    }

    fn overlays(&mut self, ctx: &egui::Context) {
        if self.show_settings {
            let mut open = true;
            let mut save = false;
            egui::Window::new("Settings").open(&mut open).default_width(520.0).resizable(false).show(ctx,|ui| {
                ui.add_enabled_ui(!self.busy,|ui| {
                    ui.label(RichText::new("Local inference").strong());ui.label("Ollama server");ui.text_edit_singleline(&mut self.settings.endpoint);
                    ui.label(RichText::new("Only a loopback Ollama server is accepted. Cloud-tagged models are excluded.").small().color(MUTED));ui.add_space(12.0);
                    ui.horizontal(|ui| {ui.label("Context tokens");ui.add(egui::DragValue::new(&mut self.settings.context_tokens).range(4096..=32768).speed(1024.0));});
                    ui.horizontal(|ui| {ui.label("Output tokens");ui.add(egui::DragValue::new(&mut self.settings.output_tokens).range(512..=8192).speed(256.0));});
                    ui.horizontal(|ui| {ui.label("Maximum tool steps");ui.add(egui::DragValue::new(&mut self.settings.max_steps).range(1..=30));});
                    ui.label(RichText::new("Larger context uses more GPU memory. Start at 8,192 tokens on the RTX 3060; use 4,096 if memory is tight.").small().color(MUTED));
                    ui.add_space(14.0);ui.separator();ui.add_space(8.0);ui.checkbox(&mut self.settings.read_only,"Read only: disable file changes and commands");
                    ui.checkbox(&mut self.settings.web_enabled,"Enable reviewed internet tools");
                    ui.label(RichText::new("Public page fetching needs no key. Search uses OLLAMA_API_KEY from the app's launch environment and sends the approved query to ollama.com. Keys are never saved in settings.").small().color(MUTED));
                    ui.add_space(12.0);ui.horizontal(|ui| {ui.label("Agent protocol");egui::ComboBox::from_id_salt("protocol").selected_text(&self.settings.protocol).show_ui(ui,|ui| {for value in ["auto","native","json"]{ui.selectable_value(&mut self.settings.protocol,value.to_string(),value);}});});
                    ui.label(RichText::new("Auto uses native tool calling when supported. JSON actions are a fallback. For reasoning models that do not reliably call tools, choose Chat or select a file and use Edit file.").small().color(MUTED));
                    ui.label(RichText::new(format!("Chats and settings: {}",store::data_dir().display())).small().color(MUTED));
                    ui.add_space(12.0);if ui.button("Save settings").clicked(){save=true;}
                });
            });
            if save {
                match ollama::local_endpoint(&self.settings.endpoint) {
                    Ok(_) => {
                        self.persist();
                        self.refresh_models();
                        self.show_settings = false;
                    }
                    Err(error) => self.error = Some(error.to_string()),
                }
            } else if !open {
                self.show_settings = false;
            }
        }
        let mut edit_selected = None;
        if let Some((path, content)) = &self.preview {
            let mut open = true;
            egui::Window::new(format!("Preview · {path}"))
                .open(&mut open)
                .default_size([820.0, 560.0])
                .show(ctx, |ui| {
                    ui.label(RichText::new("Read-only preview").color(MUTED));
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("Edit this file"))
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
        egui::Panel::top("header")
            .exact_size(69.0)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(20, 16)),
            )
            .show(ui, |ui| self.header(ui));
        egui::Panel::bottom("status-bar")
            .exact_size(29.0)
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(18, 5)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.status).small().color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new("Local inference · Reviewed actions")
                                .small()
                                .color(MUTED),
                        );
                    });
                });
            });
        egui::Panel::left("sidebar")
            .default_size(240.0)
            .size_range(200.0..=360.0)
            .resizable(true)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(16))
            .show(ui, |ui| self.sidebar(ui));
        if ctx.content_rect().width() > 1050.0 {
            egui::Panel::right("activity")
                .default_size(285.0)
                .size_range(230.0..=420.0)
                .resizable(true)
                .frame(egui::Frame::new().fill(PANEL).inner_margin(16))
                .show(ui, |ui| self.activity_panel(ui));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(0))
            .show(ui, |ui| {
                if let Some(error) = &self.error {
                    let mut dismiss = false;
                    egui::Frame::new()
                        .fill(Color32::from_rgb(65, 44, 40))
                        .inner_margin(10)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(error).color(AMBER));
                                if ui.small_button("Dismiss").clicked() {
                                    dismiss = true;
                                }
                            });
                        });
                    if dismiss {
                        self.error = None;
                    }
                }
                self.conversation(ui);
            });
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
            .fill(Color32::from_rgb(13, 23, 30))
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
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.into(), Arc::new(egui::FontData::from_owned(bytes)));
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, name.into());
        }
    }
    ctx.set_fonts(fonts);
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = BG;
    visuals.faint_bg_color = SURFACE;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = Color32::from_rgb(45, 98, 112);
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
    });
}
