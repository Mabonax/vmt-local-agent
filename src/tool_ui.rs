//! Desktop host controls. These never grant model permissions or run commands.
use crate::{tool_packs, types::Proposal, workspace::Workspace};
use eframe::egui::{self, RichText};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::mpsc, time::Duration};

enum Reply {
    Inventory(Value, Value),
    Discovery(Value),
    Review(Proposal),
    Done(String),
    Image(String, Vec<u8>, [usize; 2]),
    Error(String),
}
pub struct ToolPanel {
    workspace: String,
    python: String,
    office: String,
    query: String,
    invocation: String,
    invocation_args: String,
    discovery_bytes: usize,
    discovery_limit: usize,
    collection: String,
    ingest_path: String,
    ingest_title: String,
    import_path: String,
    question: String,
    result: String,
    inventory: Option<Value>,
    artifacts: Vec<Value>,
    discovery: Option<Value>,
    tx: mpsc::Sender<(String, Reply)>,
    rx: mpsc::Receiver<(String, Reply)>,
    pending: bool,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    review: Option<Proposal>,
    texture: Option<egui::TextureHandle>,
    image_title: String,
}
impl ToolPanel {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let configuration = tool_packs::config().ok();
        Self {
            workspace: String::new(),
            python: configuration
                .as_ref()
                .map(|c| c.python.clone())
                .unwrap_or_default(),
            office: configuration.and_then(|c| c.soffice).unwrap_or_default(),
            query: "Create a Word business report".into(),
            invocation: "workflow.run".into(),
            invocation_args: "{\"template\":\"pdf_preview\",\"inputs\":{\"title\":\"Local review\",\"body\":\"A reviewed local workflow.\"}}".into(),
            discovery_bytes: tool_packs::config().map(|c|c.discovery_bytes).unwrap_or(6000),
            discovery_limit: tool_packs::config().map(|c|c.discovery_limit).unwrap_or(6),
            collection: "default".into(),
            ingest_path: String::new(),
            ingest_title: String::new(),
            import_path: String::new(),
            question: String::new(),
            result: String::new(),
            inventory: None,
            artifacts: Vec::new(),
            discovery: None,
            tx,
            rx,
            pending: false,
            cancel: Default::default(),
            review: None,
            texture: None,
            image_title: String::new(),
        }
    }
    fn task(
        &mut self,
        ctx: &egui::Context,
        action: impl FnOnce() -> anyhow::Result<Reply> + Send + 'static,
    ) {
        self.pending = true;
        self.cancel
            .store(false, std::sync::atomic::Ordering::Relaxed);
        let tx = self.tx.clone();
        let owner = self.workspace.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let reply = action().unwrap_or_else(|e| Reply::Error(format!("{e:#}")));
            let _ = tx.send((owner, reply));
            ctx.request_repaint();
        });
    }
    fn refresh(&mut self, ctx: &egui::Context) {
        let workspace = self.workspace.clone();
        self.task(ctx, move || {
            let ws = Workspace::new(workspace)?;
            Ok(Reply::Inventory(
                tool_packs::status(&ws)?,
                tool_packs::artifacts(&ws)?,
            ))
        });
    }
    fn propose(
        &mut self,
        ctx: &egui::Context,
        tool: &str,
        arguments: Value,
        read_only: bool,
        web: bool,
    ) {
        let workspace = self.workspace.clone();
        let tool = tool.to_string();
        self.task(ctx, move || {
            let mut tools = vmt_local_agent::tools::Tools {
                workspace: Workspace::new(workspace)?,
                observed: HashMap::new(),
            };
            match tools.prepare(
                "tool_pack_call",
                &json!({"tool":tool,"arguments":arguments}),
                read_only,
                web,
            )? {
                vmt_local_agent::tools::Prepared::Review(proposal) => Ok(Reply::Review(proposal)),
                _ => anyhow::bail!("Expected exact capability review"),
            }
        });
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        workspace: &str,
        read_only: bool,
        web: bool,
        agent_busy: bool,
    ) {
        if self.workspace != workspace {
            self.cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
            self.cancel = Default::default();
            self.workspace = workspace.into();
            self.inventory = None;
            self.artifacts.clear();
            self.discovery = None;
            self.review = None;
            self.texture = None;
            self.pending = false;
            self.result.clear();
            self.collection = Workspace::new(workspace)
                .ok()
                .map(|w| tool_packs::collection(&w))
                .unwrap_or_else(|| "default".into());
        }
        while let Ok((owner, reply)) = self.rx.try_recv() {
            if owner != self.workspace {
                continue;
            }
            self.pending = false;
            match reply {
                Reply::Inventory(status, artifacts) => {
                    self.inventory = Some(status);
                    self.artifacts = artifacts["artifacts"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                }
                Reply::Discovery(value) => self.discovery = Some(value),
                Reply::Review(proposal) => self.review = Some(proposal),
                Reply::Done(message) => self.result = message,
                Reply::Error(error) => self.result = format!("Operation failed: {error}"),
                Reply::Image(title, pixels, size) => {
                    self.texture = Some(ctx.load_texture(
                        "artifact-preview",
                        egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                        egui::TextureOptions::LINEAR,
                    ));
                    self.image_title = title;
                }
            }
        }
        if self.pending {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Running local host operation…");
                if ui.button("Cancel execution").clicked() {
                    self.cancel
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
            });
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        ui.label("Reviewed local tools, private artifacts and workspace knowledge");
        ui.add_space(8.0);
        egui::CollapsingHeader::new("Runtime configuration").default_open(!tool_packs::configured()).show(ui,|ui|{
            ui.label("Python interpreter (absolute path)");ui.text_edit_singleline(&mut self.python);
            ui.label("LibreOffice executable (optional, for Office previews)");ui.text_edit_singleline(&mut self.office);
            ui.label(RichText::new("Configuration pins the selected executables and this build's reviewed tool code. It does not install packages or enable internet access.").small());
            if ui.add_enabled(!self.pending && !agent_busy,egui::Button::new("Review and configure runtime")).clicked(){let python=self.python.clone();let office=self.office.clone();self.task(ctx,move||{tool_packs::configure(&python,if office.trim().is_empty(){None}else{Some(office.trim())})?;Ok(Reply::Done("Reviewed local runtime configured. Refresh capabilities to inspect dependencies.".into()))});}
        });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !self.pending && !agent_busy,
                    egui::Button::new("Refresh capabilities and artifacts"),
                )
                .clicked()
            {
                self.refresh(ctx);
            }
            ui.label(if tool_packs::configured() {
                "Runtime configuration saved"
            } else {
                "Runtime not configured"
            });
        });
        if let Some(status) = &self.inventory {
            let tools = status["tools"].as_array().cloned().unwrap_or_default();
            let available = tools
                .iter()
                .filter(|v| v["missing_modules"].as_array().is_some_and(Vec::is_empty))
                .count();
            ui.label(format!(
                "{} registered capabilities · {} dependencies discoverable · collection {}",
                tools.len(),
                available,
                status["collection"].as_str().unwrap_or("default")
            ));
            egui::CollapsingHeader::new("Capability inventory and dependency diagnostics").show(
                ui,
                |ui| {
                    for entry in tools {
                        ui.label(format!(
                            "{} · {}{}",
                            entry["name"].as_str().unwrap_or("?"),
                            entry["category"].as_str().unwrap_or("?"),
                            if entry["missing_modules"]
                                .as_array()
                                .is_some_and(Vec::is_empty)
                            {
                                "".into()
                            } else {
                                format!(" · missing {}", entry["missing_modules"])
                            }
                        ));
                    }
                },
            );
        }
        ui.separator();
        ui.label(RichText::new("Find tools and workflows").strong());
        ui.text_edit_singleline(&mut self.query);
        if ui
            .add_enabled(
                !self.pending && !agent_busy,
                egui::Button::new("Discover relevant capabilities"),
            )
            .clicked()
        {
            let query = self.query.clone();
            let workspace = self.workspace.clone();
            self.task(ctx, move || {
                Ok(Reply::Discovery(tool_packs::discover(
                    &Workspace::new(workspace)?,
                    &query,
                    web,
                )?))
            });
        }
        if let Some(discovery) = &self.discovery {
            ui.label(serde_json::to_string_pretty(discovery).unwrap_or_default());
        }
        egui::CollapsingHeader::new("Discovery context budget").show(ui, |ui| {
            ui.add(egui::Slider::new(&mut self.discovery_limit, 1..=20).text("Maximum tools"));
            ui.add(egui::Slider::new(&mut self.discovery_bytes, 1000..=32000).text("Schema bytes"));
            if ui
                .add_enabled(
                    !self.pending && !agent_busy,
                    egui::Button::new("Save discovery budget"),
                )
                .clicked()
            {
                let limit = self.discovery_limit;
                let bytes = self.discovery_bytes;
                self.task(ctx, move || {
                    tool_packs::set_discovery_limits(limit, bytes)?;
                    Ok(Reply::Done("Discovery budget saved.".into()))
                });
            }
        });
        egui::CollapsingHeader::new("Review a capability or workflow").show(ui, |ui| {
            ui.label("Choose an exact registered capability and its discovered JSON arguments.");
            ui.text_edit_singleline(&mut self.invocation);
            egui::ComboBox::from_id_salt("capability-picker")
                .selected_text("Registered capabilities")
                .show_ui(ui, |ui| {
                    for name in tool_packs::catalog().keys() {
                        if ui.selectable_label(false, name).clicked() {
                            self.invocation = name.clone();
                            self.invocation_args = "{}".into();
                        }
                    }
                });
            if let Some(spec) = tool_packs::catalog().get(&self.invocation) {
                ui.label(serde_json::to_string_pretty(&spec["input_schema"]).unwrap_or_default());
            }
            ui.add(
                egui::TextEdit::multiline(&mut self.invocation_args)
                    .desired_rows(5)
                    .desired_width(f32::INFINITY),
            );
            if ui
                .add_enabled(
                    !self.pending && !agent_busy,
                    egui::Button::new("Prepare exact action for review"),
                )
                .clicked()
            {
                match serde_json::from_str(&self.invocation_args) {
                    Ok(value) => self.propose(ctx, &self.invocation.clone(), value, read_only, web),
                    Err(error) => self.result = format!("Invalid JSON: {error}"),
                }
            }
        });
        ui.separator();
        ui.label(RichText::new("Artifacts and revisions").strong());
        ui.horizontal(|ui| {
            ui.label("Workspace file to import");
            ui.text_edit_singleline(&mut self.import_path);
            if ui
                .add_enabled(
                    !self.pending && !agent_busy,
                    egui::Button::new("Review import"),
                )
                .clicked()
            {
                self.propose(
                    ctx,
                    "artifact.import",
                    json!({"path":self.import_path}),
                    read_only,
                    web,
                );
            }
        });
        for artifact in self.artifacts.clone() {
            let id = artifact["artifact_id"].as_str().unwrap_or("").to_string();
            let extension = artifact["extension"].as_str().unwrap_or("").to_string();
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} · revision {} · {} bytes",
                        extension.to_uppercase(),
                        artifact["version"],
                        artifact["bytes"]
                    ))
                    .strong(),
                );
                ui.label(format!("ID {id}"));
                ui.label(
                    RichText::new(format!(
                        "SHA-256 {}",
                        artifact["sha256"].as_str().unwrap_or("")
                    ))
                    .small(),
                );
                if !artifact["previous"].is_null() {
                    ui.label(format!(
                        "Previous {}",
                        artifact["previous"].as_str().unwrap_or("")
                    ));
                }
                ui.horizontal_wrapped(|ui| {
                    ui.add_enabled_ui(!self.pending && !agent_busy, |ui| {
                        if ui.button("Open verified copy").clicked() {
                            let workspace = self.workspace.clone();
                            let id = id.clone();
                            self.task(ctx, move || {
                                tool_packs::open_artifact(&Workspace::new(workspace)?, &id)?;
                                Ok(Reply::Done(
                                    "Opened a verified private copy in the associated application."
                                        .into(),
                                ))
                            });
                        }
                        if ui.button("Save as…").clicked()
                            && let Some(destination) = rfd::FileDialog::new()
                                .set_file_name(format!(
                                    "artifact-{}.{}",
                                    &id[..id.len().min(8)],
                                    extension
                                ))
                                .save_file()
                        {
                            let workspace = self.workspace.clone();
                            let id = id.clone();
                            self.task(ctx, move || {
                                tool_packs::save_artifact(
                                    &Workspace::new(workspace)?,
                                    &id,
                                    &destination,
                                )?;
                                Ok(Reply::Done(
                                    "Artifact saved to your selected new file.".into(),
                                ))
                            });
                        }
                        if ["docx", "xlsx", "pptx"].contains(&extension.as_str())
                            && ui.button("Review Office preview").clicked()
                        {
                            self.propose(
                                ctx,
                                "office.preview",
                                json!({"artifact_id":id}),
                                read_only,
                                web,
                            );
                        }
                        if extension == "pdf" && ui.button("Review first-page preview").clicked() {
                            self.propose(
                                ctx,
                                "pdf.render",
                                json!({"artifact_id":id,"page":1,"dpi":72}),
                                read_only,
                                web,
                            );
                        }
                        if ["png", "jpg"].contains(&extension.as_str())
                            && ui.button("Preview image").clicked()
                        {
                            let workspace = self.workspace.clone();
                            let id = id.clone();
                            self.task(ctx, move || {
                                let (_, bytes) =
                                    tool_packs::artifact_bytes(&Workspace::new(workspace)?, &id)?;
                                let mut reader =
                                    image::ImageReader::new(std::io::Cursor::new(bytes))
                                        .with_guessed_format()?;
                                let mut limits = image::Limits::default();
                                limits.max_alloc = Some(32 * 1024 * 1024);
                                limits.max_image_width = Some(2000);
                                limits.max_image_height = Some(2000);
                                reader.limits(limits);
                                let pixels = reader.decode()?.to_rgba8();
                                anyhow::ensure!(
                                    pixels.width() * pixels.height() <= 2_000_000,
                                    "Preview pixel limit exceeded"
                                );
                                let size = [pixels.width() as usize, pixels.height() as usize];
                                Ok(Reply::Image(id, pixels.into_raw(), size))
                            });
                        }
                        if let Some(previous) = artifact["previous"].as_str()
                            && ui.button("Review revision comparison").clicked()
                        {
                            self.propose(
                                ctx,
                                "artifact.compare",
                                json!({"first_id":previous,"second_id":id}),
                                read_only,
                                web,
                            );
                        }
                    });
                });
            });
        }
        ui.separator();
        ui.label(RichText::new("Persistent document knowledge").strong());
        ui.horizontal(|ui| {
            ui.label("Collection");
            ui.text_edit_singleline(&mut self.collection);
            if ui
                .add_enabled(
                    !self.pending && !agent_busy,
                    egui::Button::new("Select collection"),
                )
                .clicked()
            {
                let workspace = self.workspace.clone();
                let name = self.collection.clone();
                self.task(ctx, move || {
                    tool_packs::select_collection(&Workspace::new(workspace)?, &name)?;
                    Ok(Reply::Done(
                        "Workspace knowledge collection selected. Refresh to inspect it.".into(),
                    ))
                });
            }
        });
        ui.label("Ingest reviewed workspace text, DOCX body text or embedded PDF text. Conversation memory and optional Ollama semantic indexes stay separate. Scanned PDF images require a separate OCR workflow.");
        ui.horizontal(|ui| {
            ui.label("Path");
            ui.text_edit_singleline(&mut self.ingest_path);
        });
        ui.horizontal(|ui| {
            ui.label("Title");
            ui.text_edit_singleline(&mut self.ingest_title);
        });
        if ui
            .add_enabled(
                !self.pending && !agent_busy,
                egui::Button::new("Review knowledge ingestion"),
            )
            .clicked()
        {
            self.propose(
                ctx,
                "knowledge.ingest",
                json!({"path":self.ingest_path,"title":self.ingest_title}),
                read_only,
                web,
            );
        }
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.question);
            if ui
                .add_enabled(
                    !self.pending && !agent_busy,
                    egui::Button::new("Review knowledge search"),
                )
                .clicked()
            {
                self.propose(
                    ctx,
                    "knowledge.search",
                    json!({"query":self.question,"limit":5}),
                    read_only,
                    web,
                );
            }
        });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !self.pending && !agent_busy,
                    egui::Button::new("Review collection statistics"),
                )
                .clicked()
            {
                self.propose(ctx, "knowledge.statistics", json!({}), read_only, web);
            }
            if ui
                .add_enabled(
                    !self.pending && !agent_busy,
                    egui::Button::new("Review research sources"),
                )
                .clicked()
            {
                self.propose(ctx, "research.sources", json!({}), read_only, web);
            }
        });
        if !self.result.is_empty() {
            ui.separator();
            ui.label(RichText::new("Last host result").strong());
            ui.add(
                egui::TextEdit::multiline(&mut self.result)
                    .desired_rows(12)
                    .desired_width(f32::INFINITY),
            );
        }
        if let Some(proposal) = self.review.clone() {
            let mut approve = false;
            let mut deny = false;
            egui::Window::new("Review exact local capability")
                .id(egui::Id::unique("tool-panel-review"))
                .default_size([760.0, 550.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(420.0)
                        .show(ui, |ui| {
                            ui.label(vmt_local_agent::tools::diff(&proposal));
                        });
                    ui.horizontal(|ui| {
                        approve = ui
                            .add_enabled(
                                !self.pending && !agent_busy,
                                egui::Button::new("Approve and run once"),
                            )
                            .clicked();
                        deny = ui.button("Deny").clicked();
                    });
                });
            if deny {
                self.review = None;
                self.result = "Action denied; nothing executed.".into();
            }
            if approve {
                self.review = None;
                let workspace = self.workspace.clone();
                let cancel = self.cancel.clone();
                self.task(ctx, move || {
                    let mut tools = vmt_local_agent::tools::Tools {
                        workspace: Workspace::new(workspace)?,
                        observed: HashMap::new(),
                    };
                    let mut journal =
                        vmt_local_agent::task_journal::TaskJournal::start(&tools.workspace)?;
                    let result = vmt_local_agent::execution::execute_reviewed(
                        &mut tools,
                        "tool_pack_call",
                        &proposal,
                        read_only,
                        web,
                        &mut journal,
                        &cancel,
                    );
                    journal.finish(if result.is_ok() {
                        "completed"
                    } else {
                        "failed"
                    })?;
                    Ok(Reply::Done(result?))
                });
            }
        }
        if let Some(texture) = &self.texture {
            let mut open = true;
            egui::Window::new(format!("Artifact preview · {}", self.image_title))
                .open(&mut open)
                .default_size([760.0, 720.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::both().show(ui, |ui| {
                        let size = texture.size_vec2();
                        let scale = (ui.available_width() / size.x).min(1.0);
                        ui.add(
                            egui::Image::new((texture.id(), size)).fit_to_exact_size(size * scale),
                        );
                    });
                });
            if !open {
                self.texture = None;
            }
        }
    }
}
