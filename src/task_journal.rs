//! Durable per-turn execution journal with atomic snapshots and bounded metadata.
//! This is the 0.4 foundation; it does not yet implement SQLite, encrypted
//! records, cross-process locking, or automatic replay after a crash.
use crate::{store, workspace::Workspace};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};

#[derive(Clone, Serialize, Deserialize)]
pub struct JournalEvent {
    pub at: String,
    pub step: String,
    pub tool: String,
    pub outcome: String,
}
#[derive(Serialize, Deserialize)]
struct Journal {
    id: String,
    workspace: String,
    started: String,
    state: String,
    events: Vec<JournalEvent>,
}
pub struct TaskJournal {
    file: PathBuf,
    journal: Journal,
}
impl TaskJournal {
    pub fn start(workspace: &Workspace) -> Result<Self> {
        let id = uuid::Uuid::new_v4().to_string();
        let file = store::data_dir().join("task-journal").join(format!("{id}.json"));
        let journal = Journal {
            id, workspace: workspace.root.to_string_lossy().into_owned(),
            started: chrono::Utc::now().to_rfc3339(),
            state: "running".into(),
            events: Vec::new(),
        };
        let task = Self { file, journal };
        task.persist()?;
        Ok(task)
    }
    fn persist(&self) -> Result<()> {
        let parent = self.file.parent().context("Missing journal parent")?;
        fs::create_dir_all(parent)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(&serde_json::to_vec_pretty(&self.journal)?)?;
        temp.as_file().sync_all()?;
        temp.persist(&self.file)?;
        Ok(())
    }
    pub fn record(&mut self, step: &str, tool: &str, outcome: &str) -> Result<()> {
        ensure!(step.len() <= 40 && tool.len() <= 80 && outcome.len() <= 60,
            "Task journal metadata exceeds limits");
        self.journal.events.push(JournalEvent {
            at: chrono::Utc::now().to_rfc3339(),
            step: step.into(),
            tool: tool.into(),
            outcome: outcome.into(),
        });
        if self.journal.events.len() > 256 {
            self.journal.events.drain(..self.journal.events.len() - 256);
        }
        self.persist()
    }
    pub fn finish(&mut self, state: &str) -> Result<()> {
        ensure!(matches!(state, "completed" | "failed" | "stopped"),
            "Invalid terminal task state");
        self.journal.state = state.into();
        self.persist()
    }
    pub fn id(&self) -> &str { &self.journal.id }
}
/// Journal summaries are intentionally read-only; old tasks must never
/// automatically re-execute commands or writes after a restart.
pub fn recent_for(workspace: &Workspace, limit: usize) -> Result<String> {
    recent_scoped(limit, Some(&workspace.root.to_string_lossy()))
}
pub fn recent(limit: usize) -> Result<String> {
    recent_scoped(limit, None)
}
fn recent_scoped(limit: usize, scope: Option<&str>) -> Result<String> {
    let directory = store::data_dir().join("task-journal");
    if !directory.exists() { return Ok("No previous task journal entries.".into()); }
    let mut records = Vec::new();
    for entry in fs::read_dir(directory)?.flatten().take(500) {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "json") { continue; }
        let Ok(meta) = fs::symlink_metadata(&path) else { continue; };
        if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 96 * 1024 {
            continue;
        }
        if let Ok(record) = serde_json::from_slice::<Journal>(&fs::read(path)?)
            && scope.is_none_or(|workspace| record.workspace == workspace)
        {
            records.push(record);
        }
    }
    records.sort_by(|a,b| b.started.cmp(&a.started));
    let lines: Vec<_> = records.into_iter().take(limit.min(25)).map(|r| {
        let status = if r.state == "running" {
            "interrupted/uncertain (manual reconciliation required)"
        } else { r.state.as_str() };
        format!("{} {} {} ({} recorded events)",
            r.id, r.started, status, r.events.len())
    }).collect();
    if lines.is_empty() { Ok("No previous task journal entries.".into()) }
    else { Ok(lines.join("\n")) }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creates_and_updates_atomic_journal() {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::new(dir.path()).unwrap();
        let mut journal = TaskJournal::start(&ws).unwrap();
        journal.record("approval", "edit_file", "denied").unwrap();
        journal.finish("stopped").unwrap();
        let bytes = fs::read(&journal.file).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["state"], "stopped");
        assert_eq!(value["events"][0]["outcome"], "denied");
        assert!(value["events"][0].get("arguments").is_none());
    }
}
