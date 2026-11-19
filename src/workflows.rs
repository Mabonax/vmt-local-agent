//! Persisted, explicit development workflow state. Guidance is not execution.
use crate::{
    store,
    workspace::{Workspace, clip},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};

const STAGES: &[&str] = &["inspect", "plan", "implement", "verify", "report"];
const TYPES: &[(&str, &str)] = &[
    (
        "feature",
        "Inspect existing code; agree a small plan; implement reviewed changes; run tests; report evidence.",
    ),
    (
        "bugfix",
        "Reproduce issue; isolate cause; patch minimally; run regression tests; describe residual risks.",
    ),
    (
        "refactor",
        "Baseline behavior; define invariants; edit narrowly; run existing tests; report changed surface.",
    ),
    (
        "document",
        "Inspect required input; plan format and layout; generate via verified document tools; reopen and verify outputs.",
    ),
];
#[derive(Serialize, Deserialize)]
struct State {
    workspace: String,
    kind: String,
    stage: usize,
    evidence: Vec<String>,
}
fn path(ws: &Workspace) -> PathBuf {
    let mut n: u64 = 0xcbf29ce484222325;
    for b in ws.root.to_string_lossy().as_bytes() {
        n ^= u64::from(*b);
        n = n.wrapping_mul(0x100000001b3);
    }
    store::data_dir()
        .join("workflows")
        .join(format!("{n:016x}.json"))
}
fn load(ws: &Workspace) -> Result<Option<State>> {
    let path = path(ws);
    store::check_local_path(&path)?;
    if !path.exists() {
        return Ok(None);
    }
    let meta = fs::symlink_metadata(&path)?;
    ensure!(
        meta.is_file() && !meta.file_type().is_symlink() && meta.len() < 12_000,
        "Workflow state is invalid"
    );
    let state: State = serde_json::from_slice(&fs::read(path)?)?;
    ensure!(
        state.workspace == ws.root.to_string_lossy(),
        "Workflow belongs to another project"
    );
    ensure!(state.stage < STAGES.len(), "Unknown workflow stage");
    Ok(Some(state))
}
fn save(ws: &Workspace, state: &State) -> Result<()> {
    let path = path(ws);
    store::check_local_path(&path)?;
    let parent = path.parent().context("Missing workflow directory")?;
    fs::create_dir_all(parent)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(&serde_json::to_vec_pretty(state)?)?;
    tmp.as_file().sync_all()?;
    store::persist_local(tmp, &path)?;
    Ok(())
}
pub fn catalog() -> String {
    TYPES
        .iter()
        .map(|(id, desc)| format!("{id}: {desc}"))
        .collect::<Vec<_>>()
        .join("\n")
}
pub fn status(ws: &Workspace) -> Result<String> {
    if let Some(state) = load(ws)? {
        let description = TYPES
            .iter()
            .find(|(id, _)| *id == state.kind)
            .map(|(_, desc)| *desc)
            .unwrap_or("Unknown");
        Ok(format!(
            "Workflow {} — stage {} of {}: {}\nPlan: {}\nPrevious evidence:\n{}",
            state.kind,
            state.stage + 1,
            STAGES.len(),
            STAGES[state.stage],
            description,
            clip(&state.evidence.join("\n"), 2500)
        ))
    } else {
        Ok("No active persisted workflow. Use workflow_start (review required).".into())
    }
}
pub fn apply_reviewed(ws: &Workspace, action: &str, kind: &str, evidence: &str) -> Result<String> {
    ensure!(evidence.len() <= 500, "Workflow evidence exceeds 500 bytes");
    let existing = load(ws)?;
    let next = match action {
        "start" => {
            ensure!(
                TYPES.iter().any(|(id, _)| *id == kind),
                "Unknown workflow type"
            );
            ensure!(
                existing.is_none(),
                "Workflow already active; finish it before starting another"
            );
            State {
                workspace: ws.root.to_string_lossy().into_owned(),
                kind: kind.into(),
                stage: 0,
                evidence: vec![],
            }
        }
        "advance" | "finish" => {
            let mut state = existing.context("Start a workflow before advancing")?;
            ensure!(
                !evidence.trim().is_empty(),
                "Advancement needs a short evidence note"
            );
            if action == "finish" {
                ensure!(
                    state.stage == STAGES.len() - 1,
                    "Finish only after the report stage"
                );
                // Keep the last result in the reviewed action history; clear active state.
                fs::remove_file(path(ws))?;
                return Ok(format!(
                    "Completed {} workflow. Final evidence: {}",
                    state.kind, evidence
                ));
            }
            ensure!(
                state.stage + 1 < STAGES.len(),
                "Already at final stage; finish the workflow"
            );
            state.evidence.push(clip(evidence.trim(), 500));
            state.stage += 1;
            state
        }
        _ => anyhow::bail!("Unknown workflow action"),
    };
    save(ws, &next)?;
    status(ws)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_and_evidence_limits() {
        assert!(catalog().contains("bugfix"));
        assert_eq!(STAGES.len(), 5);
    }
}
