//! Explicit local-model document trial. Never used by desktop dispatch or startup.
use crate::{
    agent, design_tools, store,
    types::{ChatEntry, Event, Proposal, Settings},
    workspace::Workspace,
};
use anyhow::{Result, ensure};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

fn hash(path: &Path) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

pub fn run(
    model: &str,
    project: &str,
    request_file: &str,
    stage: u8,
    output_tokens: u32,
) -> Result<()> {
    ensure!((1..=3).contains(&stage), "Use stage 1-3");
    ensure!(
        (2048..=8192).contains(&output_tokens),
        "Use 2048-8192 output tokens"
    );
    let workspace = Workspace::new(project)?;
    let isolated_root = std::env::current_dir()?.join(".local").canonicalize()?;
    ensure!(
        workspace.root.starts_with(&isolated_root)
            && workspace
                .root
                .file_name()
                .is_some_and(|name| name == "project"),
        "Document trials only accept isolated .local/<trial>/project workspaces"
    );
    let root = workspace
        .root
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Missing trial parent"))?;
    // Keep synthetic acceptance journals separate from the normal desktop profile.
    let registry = fs::read(store::data_dir().join("skill-libraries.json"))?;
    let runtime = fs::read(store::data_dir().join("design-runtime.json"))?;
    store::configure_data_dir(root.join(format!("profile-stage-{stage}")))?;
    fs::write(store::data_dir().join("skill-libraries.json"), registry)?;
    fs::write(store::data_dir().join("design-runtime.json"), runtime)?;
    let request = fs::read_to_string(request_file)?;
    ensure!(request.len() <= 16000, "Trial request exceeds bound");
    let mut immutable = BTreeMap::new();
    for file in workspace.files(".", 100)? {
        if file != "proposal.json" {
            immutable.insert(file.clone(), hash(&workspace.resolve(&file, false)?)?);
        }
    }
    let settings = Settings {
        model: model.into(),
        context_tokens: 16384,
        output_tokens,
        max_steps: 48,
        task_minutes: 8,
        web_enabled: false,
        task_permission_mode: "review".into(),
        ..Default::default()
    };
    let path = workspace.root.to_string_lossy().into_owned();
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    let started = Instant::now();
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
    let mut activity = Vec::new();
    let mut errors = Vec::new();
    let mut denials = 0;
    let mut approvals = 0;
    let mut answer = String::new();
    let output = format!("proposal-v{stage}.docx");
    let previous = format!("proposal-v{}.docx", stage.saturating_sub(1));
    let review = format!("review-v{stage}");
    while let Ok(event) = rx.recv_timeout(Duration::from_secs(270)) {
        match event {
            Event::Approval(action) => {
                // Explicit operator test fixture, not a grant supplied by a skill
                // or model. No commands, network, MCP, or arbitrary paths qualify.
                let allow = match &action.proposal {
                    Proposal::File { path, after, .. } => {
                        path == "proposal.json" && after.len() <= 64000
                    }
                    Proposal::Design { plan } => {
                        let r = &plan.request;
                        match r.operation.as_str() {
                            "palette" | "contrast" => true,
                            "inspect_document" => r
                                .input
                                .as_deref()
                                .is_some_and(|v| v == previous || v == output),
                            "compose" | "compose_v2" => {
                                r.input.as_deref() == Some("proposal.json")
                                    && r.output.as_deref() == Some(output.as_str())
                            }
                            "render" => {
                                r.input.as_deref() == Some(output.as_str())
                                    && r.output.as_deref() == Some(review.as_str())
                            }
                            _ => false,
                        }
                    }
                    _ => false,
                };
                if allow {
                    approvals += 1;
                } else {
                    denials += 1;
                }
                println!(
                    "review {}: {}",
                    action.title,
                    if allow {
                        "approved trial scope"
                    } else {
                        "denied outside trial scope"
                    }
                );
                action.reply.send(allow.into())?;
            }
            Event::Activity(a) => {
                println!("{}: {}", a.title, a.state);
                activity.push(json!({"tool":a.title,"state":a.state,"detail":a.detail}));
            }
            Event::Error(error) => errors.push(error),
            Event::Final(text) => answer = text,
            Event::Finished => break,
            _ => {}
        }
    }
    cancel.store(true, Ordering::Relaxed);
    worker
        .join()
        .map_err(|_| anyhow::anyhow!("Document trial worker panicked"))?;
    let preserved = immutable.iter().all(|(file, expected)| {
        workspace
            .resolve(file, false)
            .and_then(|path| hash(&path))
            .is_ok_and(|actual| &actual == expected)
    });
    let artifact = workspace.resolve(&output, true)?;
    let source = workspace.resolve("proposal.json", true)?;
    let runtime_passed = artifact.is_file()
        && source.is_file()
        && workspace.root.join(&review).join("review.json").is_file()
        && preserved
        && denials == 0
        && errors.is_empty()
        && !answer.is_empty()
        && !answer.starts_with("Task incomplete")
        && !answer.starts_with("Paused:");
    if source.is_file() {
        fs::copy(&source, root.join(format!("proposal-source-v{stage}.json")))?;
    }
    let report = json!({"runtime_passed":runtime_passed,"semantic_validation":"SEPARATE","model":model,"stage":stage,"context_tokens":16384,"output_tokens":output_tokens,"seconds":started.elapsed().as_secs_f64(),"approvals":approvals,"denials":denials,"errors":errors,"immutable_inputs_preserved":preserved,"initial_hashes":immutable,"artifact_sha256":if artifact.is_file() {Some(hash(&artifact)?)} else {None},"activity":activity,"answer":answer});
    fs::write(
        root.join(format!("stage-{stage}-report.json")),
        serde_json::to_vec_pretty(&report)?,
    )?;
    fs::write(root.join(format!("stage-{stage}-answer.txt")), &answer)?;
    let design_count = report["activity"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| {
            a["state"] == "Complete" && a["tool"].as_str().is_some_and(design_tools::is_tool)
        })
        .count();
    println!(
        "Document trial stage {stage}: runtime_passed={runtime_passed}, design operations={design_count}, approvals={approvals}, denials={denials}, seconds={:.2}. Semantic and visual checks remain separate.",
        started.elapsed().as_secs_f64()
    );
    ensure!(
        runtime_passed,
        "Document trial incomplete; inspect retained report"
    );
    Ok(())
}
