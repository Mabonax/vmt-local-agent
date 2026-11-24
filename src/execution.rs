//! Single execution boundary for reviewed agent tools.
//! Writes a durable "dispatching" marker before invoking tools, preventing
//! recovery code from assuming a crashed task never made a side effect.
use crate::{policy, task_journal::TaskJournal, tools::Tools, types::Proposal};
use anyhow::{Result, ensure};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

fn matches_tool(tool: &str, proposal: &Proposal) -> bool {
    match proposal {
        Proposal::Design { plan } => {
            tool == "design_tool"
                || crate::design_tools::operation(tool)
                    .is_some_and(|operation| operation == plan.request.operation)
        }
        Proposal::File { .. } => matches!(tool, "edit_file" | "write_file"),
        Proposal::Command { .. } => tool == "run_command",
        Proposal::Web { tool: web_tool, .. } => {
            tool == web_tool && matches!(tool, "web_fetch" | "web_search")
        }
        Proposal::MemoryNote { .. } => tool == "remember_project",
        Proposal::Mcp { operation, .. } => {
            (tool == "mcp_tools" && operation == "tools/list")
                || (tool == "mcp_call" && operation == "tools/call")
        }
        Proposal::KnowledgeIndex { .. } => tool == "index_knowledge",
        Proposal::Workflow { action, .. } => matches!(
            (tool, action.as_str()),
            ("workflow_start", "start")
                | ("workflow_advance", "advance")
                | ("workflow_finish", "finish")
        ),
    }
}

pub fn execute_reviewed(
    tools: &mut Tools,
    name: &str,
    proposal: &Proposal,
    read_only: bool,
    internet_enabled: bool,
    journal: &mut TaskJournal,
    cancel: &Arc<AtomicBool>,
) -> Result<String> {
    ensure!(!cancel.load(Ordering::Relaxed), "Stopped before dispatch");
    policy::authorize(name, read_only, internet_enabled)?;
    ensure!(
        matches_tool(name, proposal),
        "Approved proposal/tool mismatch"
    );
    journal.record("dispatch", name, "uncertain-until-result")?;
    let result = tools.execute(proposal, cancel);
    // A failed journal write must not silently become a successful action.
    journal.record(
        "result",
        name,
        if result.is_ok() {
            "completed"
        } else {
            "failed-or-uncertain"
        },
    )?;
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_mismatched_proposals() {
        let proposal = Proposal::Command {
            command: "echo hi".into(),
        };
        assert!(!matches_tool("web_fetch", &proposal));
        assert!(matches_tool("run_command", &proposal));
        let edit = Proposal::File {
            path: "a.txt".into(),
            before: None,
            after: "hello".into(),
        };
        assert!(!matches_tool("run_command", &edit));
        assert!(matches_tool("edit_file", &edit));
    }
}
