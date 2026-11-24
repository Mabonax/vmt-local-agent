//! Deterministic tool policy. Model prompts and MCP descriptions cannot grant
//! privileges; decisions occur before both prepare and approved execution.
use anyhow::{Result, bail, ensure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    Read,
    ProjectWrite,
    Shell,
    Internet,
    ExternalProcess,
    LocalDataWrite,
}
impl Capability {
    pub fn label(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::ProjectWrite => "project-write",
            Self::Shell => "shell",
            Self::Internet => "internet",
            Self::ExternalProcess => "external-process",
            Self::LocalDataWrite => "local-data-write",
        }
    }
    pub fn requires_approval(self) -> bool {
        self != Self::Read
    }
}
pub fn classify(tool: &str) -> Result<Capability> {
    Ok(match tool {
        "list_files" | "read_file" | "search_text" | "list_skills" | "load_skill"
        | "skill_resource" | "memory_search" | "knowledge_search" | "list_workflows"
        | "workflow_status" | "mcp_servers" | "mcp_schema" | "research_sources"
        | "task_history" => Capability::Read,
        "write_file" | "edit_file" => Capability::ProjectWrite,
        "run_command" => Capability::Shell,
        "web_search" | "web_fetch" => Capability::Internet,
        "mcp_tools" | "mcp_call" | "design_tool" | "colour_palette" | "colour_contrast"
        | "colour_extract" | "inspect_document" | "format_document" | "render_document"
        | "compose_document" => Capability::ExternalProcess,
        "index_knowledge" | "remember_project" | "workflow_start" | "workflow_advance"
        | "workflow_finish" => Capability::LocalDataWrite,
        _ => bail!("Unregistered tool is forbidden by policy: {tool}"),
    })
}
pub fn authorize(tool: &str, read_only: bool, internet_enabled: bool) -> Result<Capability> {
    let capability = classify(tool)?;
    ensure!(
        !read_only || matches!(capability, Capability::Read),
        "Read-only mode disallows {} operations",
        capability.label()
    );
    ensure!(
        internet_enabled || capability != Capability::Internet,
        "Internet access is disabled by policy"
    );
    Ok(capability)
}
/// This is only a coarse preventative privacy gate, not a data-loss prevention
/// system. The approval dialog MUST still display the entire exact query.
pub fn validate_public_search(query: &str) -> Result<()> {
    ensure!(
        !query.trim().is_empty() && query.len() <= 500,
        "Public research query must contain 1–500 bytes"
    );
    ensure!(
        !query.contains('\n') && !query.contains('\r'),
        "Public research queries must be a single line"
    );
    let q = query.to_ascii_lowercase();
    for marker in [
        "-----begin private key",
        "sk-proj-",
        "ghp_",
        "github_pat_",
        "password=",
        "authorization: bearer",
        "aws_secret_access_key",
        "ollama_api_key=",
    ] {
        ensure!(
            !q.contains(marker),
            "Public query appears to contain a credential; request denied"
        );
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn denies_unregistered_mutations_and_missing_consent() {
        assert!(authorize("unknown", false, true).is_err());
        assert!(authorize("write_file", true, true).is_err());
        assert!(authorize("mcp_call", true, true).is_err());
        assert!(authorize("web_search", false, false).is_err());
        assert_eq!(
            authorize("read_file", true, false).unwrap(),
            Capability::Read
        );
        assert!(classify("run_command").unwrap().requires_approval());
    }
    #[test]
    fn research_query_rejects_common_credentials() {
        assert!(validate_public_search("Rust MCP documentation").is_ok());
        assert!(validate_public_search("search sk-proj-testsecret").is_err());
        assert!(validate_public_search("password=secret").is_err());
        assert!(validate_public_search(&"x".repeat(501)).is_err());
    }
}
