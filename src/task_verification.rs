//! Runtime evidence, independent of the model's completion narrative.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct Failures(BTreeMap<(String, String), Value>);
impl Failures {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn failed(&mut self, tool: &str, args: &Value) {
        self.0.insert((tool.into(), args.to_string()), args.clone());
    }
    pub fn succeeded(&mut self, tool: &str, args: &Value) {
        self.0.remove(&(tool.into(), args.to_string()));
        // An actual DOCX inspection/format is the supported replacement for a
        // failed UTF-8 read of that same document, never an unrelated file.
        if crate::design_tools::is_tool(tool)
            && matches!(
                crate::design_tools::operation(tool).or_else(|| args["operation"].as_str()),
                Some("inspect_document" | "format_document")
            )
            && let Some(input) = args["input"].as_str()
        {
            self.0.retain(|(name, _), old| {
                name != "read_file" || old["path"].as_str() != Some(input)
            });
        }
    }
}
impl std::fmt::Debug for Failures {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: BTreeSet<_> = self.0.keys().map(|(name, _)| name).collect();
        formatter.debug_set().entries(names).finish()
    }
}
pub fn document_format_request(request: &str) -> bool {
    let request = request.to_lowercase();
    ["format", "restyl", "layout", "typograph"]
        .iter()
        .any(|word| request.contains(word))
        && ["docx", "document", "word", "cv", "resume", "résumé"]
            .iter()
            .any(|word| request.contains(word))
}
pub fn incomplete(
    failures: &Failures,
    document_missing: bool,
    outputs: &BTreeSet<String>,
) -> String {
    let mut message="Task incomplete. The runtime could not verify completion, so the model's proposed success summary has been withheld.".to_string();
    if !failures.is_empty() {
        message.push_str(&format!(
            "\nUnresolved failed operations: {failures:?}. Review Activity for the exact errors."
        ));
    }
    if document_missing {
        message.push_str("\nNo verified formatted Word output was produced. Rendering or copying a file does not prove its formatting changed.");
    }
    if !outputs.is_empty() {
        message.push_str(&format!(
            "\nVerified document outputs created: {}. Visual review is still required.",
            outputs.iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    message
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn unrelated_success_cannot_erase_a_failed_file_operation() {
        let mut failures = Failures::default();
        failures.failed("read_file", &json!({"path":"CV.docx"}));
        failures.succeeded("read_file", &json!({"path":"other.txt"}));
        assert!(!failures.is_empty());
        failures.succeeded(
            "design_tool",
            &json!({"operation":"inspect_document","input":"other.docx"}),
        );
        assert!(!failures.is_empty());
        failures.succeeded(
            "design_tool",
            &json!({"operation":"inspect_document","input":"CV.docx"}),
        );
        assert!(failures.is_empty());
    }
    #[test]
    fn document_completion_requires_evidence_and_safe_failure_text() {
        assert!(document_format_request(
            "improve the formating of this CV document"
        ));
        assert!(!document_format_request("Explain the JSON format"));
        let mut failures = Failures::default();
        failures.failed("run_command", &json!({"command":"private argument"}));
        let result = incomplete(&failures, true, &BTreeSet::new());
        assert!(result.contains("No verified formatted Word output"));
        assert!(!result.contains("private argument"));
    }
}
