//! Explicit operator-owned synthetic model acceptance. Not a model capability.
use crate::{
    agent, store, tool_packs,
    types::{ChatEntry, Event, Proposal, Settings},
    workspace::Workspace,
};
use anyhow::{Result, ensure};
use serde_json::json;
use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

pub fn run(
    model: &str,
    project: &str,
    request_file: &str,
    json_protocol: bool,
    deny_all: bool,
) -> Result<()> {
    let workspace = Workspace::new(project)?;
    let local = std::env::current_dir()?.join(".local").canonicalize()?;
    ensure!(
        workspace.root.starts_with(&local),
        "Synthetic trials require a .local workspace"
    );
    ensure!(
        store::data_dir().starts_with(&local),
        "Synthetic trials require a .local profile"
    );
    let request = fs::read_to_string(request_file)?;
    let planner = if crate::document_cover::requested(&request) {
        "fixed-document-cover"
    } else {
        "local-model"
    };
    ensure!(request.len() <= 12000, "Trial request too long");
    let before = tool_packs::artifacts(&workspace)?;
    let settings = Settings {
        protocol: if json_protocol { "json" } else { "auto" }.into(),
        model: model.into(),
        context_tokens: 16384,
        output_tokens: 2048,
        max_steps: 18,
        task_minutes: 3,
        web_enabled: false,
        task_permission_mode: "review".into(),
        ..Default::default()
    };
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    let started = Instant::now();
    let path = workspace.root.to_string_lossy().to_string();
    let worker = std::thread::spawn(move || {
        agent::run(
            settings,
            path,
            vec![ChatEntry {
                role: "user".into(),
                content: request,
                ..Default::default()
            }],
            tx,
            stop,
            eframe::egui::Context::default(),
        )
    });
    let mut activity = vec![];
    let mut answer = String::new();
    let mut errors = vec![];
    let mut denials = 0;
    let mut approvals = 0;
    loop {
        if started.elapsed() > Duration::from_secs(195) {
            cancel.store(true, Ordering::Relaxed);
        }
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(Event::Approval(review)) => {
                let allow = !deny_all
                    && match &review.proposal {
                        Proposal::ToolPack { plan } => matches!(
                            plan.tool.as_str(),
                            "office.docx"
                                | "document.add_cover"
                                | "office.xlsx"
                                | "office.pptx"
                                | "office.preview"
                                | "pdf.generate"
                                | "pdf.render"
                                | "artifact.inspect"
                                | "artifact.hash"
                                | "workflow.run"
                                | "knowledge.search"
                                | "knowledge.citation"
                                | "knowledge.documents"
                                | "knowledge.statistics"
                                | "math.calculate"
                        ),
                        Proposal::Design { plan } => {
                            matches!(plan.request.operation.as_str(), "palette" | "contrast")
                        }
                        _ => false,
                    };
                if !allow {
                    denials += 1;
                } else {
                    approvals += 1;
                }
                println!(
                    "Review {}: {}",
                    review.title,
                    if allow {
                        "approved synthetic scope"
                    } else {
                        "denied"
                    }
                );
                let _ = review.reply.send(allow.into());
            }
            Ok(Event::Activity(a)) => {
                println!("{}: {}", a.title, a.state);
                activity.push(json!({"tool":a.title,"state":a.state,"detail":a.detail}));
            }
            Ok(Event::Final(text)) => answer = text,
            Ok(Event::Error(error)) => errors.push(error),
            Ok(Event::Finished) => break,
            Ok(_) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if started.elapsed() > Duration::from_secs(210) {
                    anyhow::bail!("Trial exceeded cancellation grace period");
                }
            }
            Err(_) => break,
        }
    }
    cancel.store(true, Ordering::Relaxed);
    worker
        .join()
        .map_err(|_| anyhow::anyhow!("Trial worker panic"))?;
    let after = tool_packs::artifacts(&workspace)?;
    let report = json!({"model":model,"planner":planner,"context_tokens":16384,"output_tokens":2048,"seconds":started.elapsed().as_secs_f64(),"activity":activity,"answer":answer,"errors":errors,"approvals":approvals,"denials":denials,"artifacts_before":before,"artifacts_after":after});
    fs::write(
        workspace.root.join("model-trial.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!(
        "Trial report saved. Model acceptance and artifact semantics must be assessed separately."
    );
    Ok(())
}
