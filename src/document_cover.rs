//! Fixed existing-DOCX cover workflow. One exact operator review covers the
//! bounded edit, render, private registration and new workspace file publication.
use crate::{
    execution,
    task_journal::TaskJournal,
    tools::{Prepared, Tools, diff},
    types::{Activity, Approval, ApprovalDecision, Event, Settings},
    workspace::Workspace,
};
use anyhow::{Context, Result, ensure};
use serde_json::json;
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Sender},
    },
    time::{Duration, Instant},
};

pub fn requested(request: &str) -> bool {
    let r = request.to_lowercase().replace('-', " ");
    let action = [
        "add ",
        "insert ",
        "please add ",
        "please insert ",
        "can you add ",
        "could you add ",
        "can you insert ",
    ]
    .iter()
    .any(|prefix| r.trim_start().starts_with(prefix));
    action
        && r.contains("cover page")
        && ["cv", "docx", "document", "resume", "résumé"]
            .iter()
            .any(|word| r.contains(word))
}
fn normalized(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn paths(workspace: &Workspace, request: &str) -> Result<(String, String)> {
    let files = workspace.files(".", 500)?;
    let candidates: Vec<_> = files
        .into_iter()
        .filter(|name| name.to_lowercase().ends_with(".docx") && !name.starts_with("~$"))
        .collect();
    ensure!(
        !candidates.is_empty(),
        "No existing DOCX found in this workspace. Select the CV's folder first."
    );
    let user = normalized(request);
    let literal = request.to_lowercase().replace('\\', "/");
    let mut explicit: Vec<_> = candidates
        .iter()
        .filter(|name| literal.contains(&name.to_lowercase()))
        .collect();
    explicit.sort_by_key(|name| std::cmp::Reverse(name.len()));
    let mut matched: Vec<_> = candidates
        .iter()
        .filter(|name| {
            Path::new(name)
                .file_stem()
                .and_then(|s| s.to_str())
                .is_some_and(|stem| normalized(stem).len() >= 4 && user.contains(&normalized(stem)))
        })
        .collect();
    if matched.is_empty() && candidates.len() == 1 {
        matched.push(&candidates[0]);
    }
    matched.sort_by_key(|name| (name.matches('/').count(), name.len()));
    if let Some(first) = explicit.first() {
        matched = vec![*first];
    }
    let first = matched
        .first()
        .context("Name the existing DOCX to cover; several documents are available.")?;
    let depth = first.matches('/').count();
    ensure!(
        matched
            .iter()
            .filter(|name| name.matches('/').count() == depth)
            .count()
            == 1,
        "Several DOCX files match. Specify its workspace-relative path before adding a cover."
    );
    let input = (*first).clone();
    let source = Path::new(&input);
    let stem = source
        .file_stem()
        .context("Invalid CV name")?
        .to_string_lossy();
    for version in 1..=100 {
        let extra = if version == 1 {
            String::new()
        } else {
            format!("_{version}")
        };
        let output = source
            .with_file_name(format!("{stem}_With_Cover{extra}.docx"))
            .to_string_lossy()
            .replace('\\', "/");
        if !workspace.resolve(&output, true)?.exists() {
            return Ok((input, output));
        }
    }
    anyhow::bail!("Choose a new cover output filename; existing versions are preserved")
}

pub fn run(
    settings: &Settings,
    workspace: &Workspace,
    request: &str,
    tx: &Sender<Event>,
    cancel: &Arc<AtomicBool>,
    ctx: &eframe::egui::Context,
) -> Result<()> {
    ensure!(
        !settings.read_only,
        "Read-only workspace: disable read-only before requesting a new covered CV"
    );
    let (input, output) = paths(workspace, request)?;
    let name = "pack__document_add_cover";
    let mut tools = Tools {
        workspace: workspace.clone(),
        observed: Default::default(),
    };
    let mut journal = TaskJournal::start(workspace)?;
    journal.track_cancellation(cancel);
    let Prepared::Review(proposal) = tools.prepare(
        name,
        &json!({"path":input,"output":output}),
        settings.read_only,
        false,
    )?
    else {
        anyhow::bail!("Expected exact cover review")
    };
    let _=tx.send(Event::Note("Using the fixed existing-document cover workflow: preserve the CV, add the cover, render all pages and save one new file. One exact review covers these steps.".into()));
    let (reply, approval) = mpsc::channel();
    journal.record("approval", name, "requested")?;
    let _ = tx.send(Event::Status("Review the cover-page workflow".into()));
    tx.send(Event::Approval(Approval {
        title: format!("Add cover page: {input} → {output}"),
        detail: diff(&proposal),
        proposal: proposal.clone(),
        reply,
        allow_task_command: false,
    }))?;
    ctx.request_repaint();
    let started = Instant::now();
    let decision = loop {
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "Stopped before cover approval"
        );
        ensure!(
            started.elapsed() < Duration::from_secs(settings.task_minutes.clamp(1, 60) as u64 * 60),
            "Cover review expired; no file was created"
        );
        match approval.recv_timeout(Duration::from_millis(100)) {
            Ok(decision) => break decision,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => anyhow::bail!("Cover review was closed; no file was created"),
        }
    };
    // An exact-command grant is not an approval for a document workflow.
    if decision != ApprovalDecision::Once {
        journal.record("approval", name, "denied")?;
        journal.finish("completed")?;
        let _ = tx.send(Event::Final(
            "Cover-page action declined. No new CV was saved.".into(),
        ));
        return Ok(());
    }
    journal.record("approval", name, "approved")?;
    let _ = tx.send(Event::Status(
        "Adding cover, checking preservation and rendering pages…".into(),
    ));
    let _ = tx.send(Event::Activity(Activity {
        title: name.into(),
        state: "Running".into(),
        detail: format!("Existing DOCX {input}; new output {output}"),
    }));
    ctx.request_repaint();
    let executed = execution::execute_reviewed(
        &mut tools,
        name,
        &proposal,
        settings.read_only,
        false,
        &mut journal,
        cancel,
    );
    match executed {
        Ok(text) => {
            let result: serde_json::Value = serde_json::from_str(&text)?;
            ensure!(
                result["result"]["saved_to_workspace"] == true,
                "Cover did not produce a verified workspace file"
            );
            journal.finish("completed")?;
            let pages = result["result"]["preview"]["pages"]
                .as_array()
                .map_or(0, Vec::len);
            let _ = tx.send(Event::Activity(Activity {
                title: name.into(),
                state: "Complete".into(),
                detail: text,
            }));
            let _=tx.send(Event::Final(format!("Saved `{output}` beside the original CV using Professional Document Design V2 and Colour Intelligence V2. The CV content and other package parts were preserved; {pages} pages were rendered and checked. Visual review remains pending.\n\nExecution record: 1 reviewed document workflow; 1 saved DOCX; 0 commands; 0 web requests.")));
            Ok(())
        }
        Err(error) => {
            journal.finish("failed")?;
            let _ = tx.send(Event::Activity(Activity {
                title: name.into(),
                state: "Failed".into(),
                detail: error.to_string(),
            }));
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cover_intent_excludes_explanations_and_negations() {
        assert!(requested("add cover page to this cv_template"));
        assert!(requested(
            "Please add a cover-page to the existing Word document"
        ));
        for r in [
            "Explain how to add a cover page to a CV",
            "Do not add a cover page to the CV",
            "Add a cover page to my website",
            "Edit the CV body",
        ] {
            assert!(!requested(r), "{r}");
        }
    }
    #[test]
    fn source_resolution_preserves_existing_versions_and_ambiguity() {
        let directory = tempfile::tempdir().unwrap();
        let workspace = Workspace::new(directory.path()).unwrap();
        std::fs::write(directory.path().join("CV_Template.docx"), b"fixture").unwrap();
        std::fs::write(directory.path().join("CV_Template_Refined.docx"), b"other").unwrap();
        let (input, output) = paths(&workspace, "add cover page to this cv_template").unwrap();
        assert_eq!(input, "CV_Template.docx");
        assert_eq!(output, "CV_Template_With_Cover.docx");
        std::fs::write(directory.path().join(&output), b"preserved").unwrap();
        assert_eq!(
            paths(&workspace, "add cover page to this cv_template")
                .unwrap()
                .1,
            "CV_Template_With_Cover_2.docx"
        );
        assert!(paths(&workspace, "add cover page to my document").is_err());
    }
}
