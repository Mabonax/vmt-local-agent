//! Operator-selected, in-memory grants. No skill, model text or project file can create a grant.
use crate::{projects, types::Proposal, workspace::Workspace};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

pub const MAX_EDITS: usize = 12;
pub const MAX_CHANGED_BYTES: usize = 64 * 1024;
const MAX_ONE_EDIT: usize = 16 * 1024;
const MAX_WORKFLOW: usize = 20;
const MAX_COMMAND_USES: usize = 8;
const TTL: Duration = Duration::from_secs(15 * 60);

pub fn repeatable_command(proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Command {command} if matches!(command.as_str(),
        "cargo test --offline" | "cargo fmt --all -- --check" |
        "cargo clippy --offline --all-targets -- -D warnings" |
        "git status --short" | "git diff --check"))
}
pub fn signature(tool: &str, proposal: &Proposal) -> String {
    // Debug is used only as input to a cryptographic fingerprint; never persisted or logged.
    let mut hash = Sha256::new();
    hash.update(tool.as_bytes());
    hash.update(format!("{proposal:?}").as_bytes());
    format!("{:x}", hash.finalize())
}
fn routine_edit(path: &str, before: Option<&str>, after: &str) -> Option<usize> {
    if path.contains(':') || path.contains('\\') || path.starts_with('/') {
        return None;
    }
    let lower = path.to_ascii_lowercase();
    for part in lower.split('/') {
        if part.is_empty()
            || part == ".."
            || part.starts_with('.')
            || matches!(
                part,
                "agents.md"
                    | "skill.md"
                    | "cargo.toml"
                    | "cargo.lock"
                    | "package.json"
                    | "package-lock.json"
                    | "composer.json"
                    | "composer.lock"
                    | "dockerfile"
                    | "docker-compose.yml"
                    | "compose.yml"
                    | "settings.json"
                    | "policy.rs"
                    | "task_permissions.rs"
                    | "execution.rs"
                    | "tools.rs"
                    | "store.rs"
                    | "skill-libraries.json"
                    | "design-runtime.json"
                    | "design_tools.rs"
                    | "projects.json"
                    | "security"
                    | "auth"
                    | "policies"
                    | "permissions"
                    | "migrations"
                    | "scripts"
                    | "config"
            )
            || [
                ".ps1", ".bat", ".cmd", ".sh", ".exe", ".dll", ".key", ".pem",
            ]
            .iter()
            .any(|ext| part.ends_with(ext))
        {
            return None;
        }
    }
    if after.trim().is_empty() {
        return None;
    }
    if before.is_some_and(|s| s.len() > 128 * 1024) || after.len() > 128 * 1024 {
        return None;
    }
    let diff = similar::TextDiff::from_lines(before.unwrap_or(""), after);
    let changed = diff
        .iter_all_changes()
        .filter(|c| c.tag() != similar::ChangeTag::Equal)
        .collect::<Vec<_>>();
    let bytes = changed.iter().map(|c| c.value().len()).sum::<usize>();
    let deleted = changed
        .iter()
        .filter(|c| c.tag() == similar::ChangeTag::Delete)
        .count();
    (bytes > 0 && bytes <= MAX_ONE_EDIT && deleted <= 100).then_some(bytes)
}

pub struct TaskPermissions {
    root: String,
    routine: bool,
    started: Instant,
    edits: usize,
    bytes: usize,
    workflow: usize,
    command_uses: usize,
    commands: HashMap<String, usize>,
    denied: HashSet<String>,
}
impl TaskPermissions {
    pub fn new(workspace: &Workspace, routine: bool) -> Self {
        Self {
            root: workspace.root.to_string_lossy().into_owned(),
            routine,
            started: Instant::now(),
            edits: 0,
            bytes: 0,
            workflow: 0,
            command_uses: 0,
            commands: HashMap::new(),
            denied: HashSet::new(),
        }
    }
    fn active(&self, workspace: &Workspace, read_only: bool) -> bool {
        !read_only
            && self.started.elapsed() < TTL
            && projects::same_root(&self.root, &workspace.root.to_string_lossy())
    }
    pub fn deny(&mut self, tool: &str, proposal: &Proposal) {
        self.denied.insert(signature(tool, proposal));
    }
    pub fn was_denied(&self, tool: &str, proposal: &Proposal) -> bool {
        self.denied.contains(&signature(tool, proposal))
    }
    pub fn grant_exact_command(
        &mut self,
        workspace: &Workspace,
        proposal: &Proposal,
        read_only: bool,
    ) -> bool {
        if !self.active(workspace, read_only)
            || !repeatable_command(proposal)
            || self.command_uses >= MAX_COMMAND_USES
        {
            return false;
        }
        // First execution consumes one of four explicitly approved uses.
        self.commands.insert(signature("run_command", proposal), 3);
        self.command_uses += 1;
        true
    }
    /// Called immediately before dispatch. A spent quota cannot silently expand itself.
    pub fn consume(
        &mut self,
        workspace: &Workspace,
        tool: &str,
        proposal: &Proposal,
        read_only: bool,
    ) -> Option<&'static str> {
        if !self.active(workspace, read_only) || self.was_denied(tool, proposal) {
            return None;
        }
        match proposal {
            Proposal::File {
                path,
                before,
                after,
            } if self.routine && matches!(tool, "write_file" | "edit_file") => {
                workspace.resolve(path, true).ok()?;
                let changed = routine_edit(path, before.as_deref(), after)?;
                if self.edits >= MAX_EDITS || self.bytes + changed > MAX_CHANGED_BYTES {
                    return None;
                }
                self.edits += 1;
                self.bytes += changed;
                Some("task-edit-grant")
            }
            Proposal::Workflow { .. }
                if self.routine
                    && self.workflow < MAX_WORKFLOW
                    && matches!(
                        tool,
                        "workflow_start" | "workflow_advance" | "workflow_finish"
                    ) =>
            {
                self.workflow += 1;
                Some("task-workflow-grant")
            }
            Proposal::Command { .. }
                if tool == "run_command"
                    && repeatable_command(proposal)
                    && self.command_uses < MAX_COMMAND_USES =>
            {
                let remaining = self.commands.get_mut(&signature(tool, proposal))?;
                if *remaining == 0 {
                    return None;
                }
                *remaining -= 1;
                self.command_uses += 1;
                Some("exact-command-task-grant")
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn edit(path: &str) -> Proposal {
        Proposal::File {
            path: path.into(),
            before: Some("old\n".into()),
            after: "new\n".into(),
        }
    }
    #[test]
    fn task_grants_are_small_scoped_expiring_and_never_come_from_the_proposal() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let ws = Workspace::new(root.path()).unwrap();
        let other = Workspace::new(other.path()).unwrap();
        let mut grant = TaskPermissions::new(&ws, false);
        assert!(
            grant
                .consume(&ws, "edit_file", &edit("src/main.rs"), false)
                .is_none()
        );
        let mut grant = TaskPermissions::new(&ws, true);
        assert!(
            grant
                .consume(&other, "edit_file", &edit("src/main.rs"), false)
                .is_none()
        );
        assert!(
            grant
                .consume(&ws, "edit_file", &edit("src/main.rs"), true)
                .is_none()
        );
        grant.started = Instant::now() - TTL;
        assert!(
            grant
                .consume(&ws, "edit_file", &edit("src/main.rs"), false)
                .is_none()
        );
    }
    #[test]
    fn routine_edit_quota_and_sensitive_paths_require_a_new_review() {
        let root = tempfile::tempdir().unwrap();
        let ws = Workspace::new(root.path()).unwrap();
        let mut grant = TaskPermissions::new(&ws, true);
        for path in [
            "AGENTS.md",
            "SKILL.md",
            ".env",
            "../outside.txt",
            "Cargo.toml",
            "src/policy.rs",
            "scripts/run.py",
            "auth/login.php",
            "build.ps1",
            "config/settings.php",
        ] {
            assert!(
                grant
                    .consume(&ws, "edit_file", &edit(path), false)
                    .is_none(),
                "{path}"
            );
        }
        for _ in 0..MAX_EDITS {
            assert!(
                grant
                    .consume(&ws, "edit_file", &edit("hello.txt"), false)
                    .is_some()
            );
        }
        assert!(
            grant
                .consume(&ws, "edit_file", &edit("hello.txt"), false)
                .is_none()
        );
        let mut grant = TaskPermissions::new(&ws, true);
        grant.deny("edit_file", &edit("hello.txt"));
        assert!(
            grant
                .consume(&ws, "edit_file", &edit("hello.txt"), false)
                .is_none()
        );
    }
    #[test]
    fn exact_commands_require_explicit_grant_and_never_match_variants() {
        let root = tempfile::tempdir().unwrap();
        let ws = Workspace::new(root.path()).unwrap();
        let mut grant = TaskPermissions::new(&ws, false);
        let command = Proposal::Command {
            command: "cargo test --offline".into(),
        };
        assert!(grant.consume(&ws, "run_command", &command, false).is_none());
        assert!(grant.grant_exact_command(&ws, &command, false));
        for _ in 0..3 {
            assert!(grant.consume(&ws, "run_command", &command, false).is_some());
        }
        assert!(grant.consume(&ws, "run_command", &command, false).is_none());
        for text in [
            "cargo test --offline; Remove-Item x",
            "cargo test",
            "python evil.py",
            "cargo test --offline\nwhoami",
        ] {
            let modified = Proposal::Command {
                command: text.into(),
            };
            assert!(!grant.grant_exact_command(&ws, &modified, false));
            assert!(
                grant
                    .consume(&ws, "run_command", &modified, false)
                    .is_none()
            );
        }
        assert!(
            grant
                .consume(
                    &ws,
                    "remember_project",
                    &Proposal::MemoryNote {
                        content: "trusted".into()
                    },
                    false
                )
                .is_none()
        );
    }

    #[test]
    fn byte_workflow_and_destructive_change_thresholds_fall_back_to_review() {
        let root = tempfile::tempdir().unwrap();
        let ws = Workspace::new(root.path()).unwrap();
        let mut grant = TaskPermissions::new(&ws, true);
        let large = Proposal::File {
            path: "report.txt".into(),
            before: None,
            after: format!("{}\n", "x".repeat(8192)),
        };
        for _ in 0..7 {
            assert!(grant.consume(&ws, "write_file", &large, false).is_some());
        }
        assert!(grant.consume(&ws, "write_file", &large, false).is_none());
        let mut grant = TaskPermissions::new(&ws, true);
        let destructive = Proposal::File {
            path: "notes.txt".into(),
            before: Some("line\n".repeat(101)),
            after: "replacement\n".into(),
        };
        assert!(
            grant
                .consume(&ws, "edit_file", &destructive, false)
                .is_none()
        );
        let workflow = Proposal::Workflow {
            action: "advance".into(),
            kind: String::new(),
            evidence: "verified fixture".into(),
        };
        for _ in 0..MAX_WORKFLOW {
            assert!(
                grant
                    .consume(&ws, "workflow_advance", &workflow, false)
                    .is_some()
            );
        }
        assert!(
            grant
                .consume(&ws, "workflow_advance", &workflow, false)
                .is_none()
        );
        let network = Proposal::Web {
            tool: "web_fetch".into(),
            args: serde_json::json!({"url":"https://example.com"}),
        };
        assert!(grant.consume(&ws, "web_fetch", &network, false).is_none());
    }
}
