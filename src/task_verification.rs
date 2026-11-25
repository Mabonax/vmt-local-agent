//! Runtime evidence, independent of the model's completion narrative.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct Failures(BTreeMap<(String, String), (Value, bool)>);
impl Failures {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn failed(&mut self, tool: &str, args: &Value) {
        self.0
            .insert((tool.into(), args.to_string()), (args.clone(), false));
    }
    pub fn failed_preflight(&mut self, tool: &str, args: &Value) {
        self.0
            .entry((tool.into(), args.to_string()))
            .or_insert((args.clone(), true));
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
            self.0.retain(|(name, _), (old, _)| {
                name != "read_file" || old["path"].as_str() != Some(input)
            });
        }
        // Only validation failures have no possible side effects. A successful
        // equivalent design operation may use a different unused output name.
        // Execution failures and commands still require their own reconciliation.
        let operation = |tool: &str, args: &Value| {
            crate::design_tools::operation(tool)
                .map(str::to_string)
                .or_else(|| args["operation"].as_str().map(str::to_string))
        };
        let op = operation(tool, args);
        if crate::design_tools::is_tool(tool)
            && matches!(op.as_deref(), Some("format_document" | "render"))
        {
            let comparable = |args: &Value| {
                let mut value = args.clone();
                if let Some(object) = value.as_object_mut() {
                    object.remove("output");
                    object.remove("operation");
                    if op.as_deref() == Some("format_document") {
                        object.entry("preset").or_insert(Value::String("cv".into()));
                    }
                }
                value
            };
            self.0.retain(|(name, _), (old, preflight)| {
                !(*preflight
                    && crate::design_tools::is_tool(name)
                    && operation(name, old) == op
                    && comparable(old) == comparable(args))
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
    let words: Vec<_> = request
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
        .collect();
    if !words.iter().any(|word| {
        matches!(
            *word,
            "docx" | "document" | "documents" | "word" | "cv" | "resume" | "résumé"
        )
    }) {
        return false;
    }
    let first = request.split(['.', '\n']).next().unwrap_or("");
    let primary = words.iter().find(|word| {
        matches!(
            **word,
            "create" | "generate" | "draft" | "compose" | "author" | "edit" | "add" | "replace"
        ) || word.starts_with("format")
            || word.starts_with("reformat")
    });
    if primary.is_some_and(|word| {
        matches!(
            *word,
            "create" | "generate" | "draft" | "compose" | "author"
        )
    }) && (first.contains("from scratch") || !first.contains("existing"))
    {
        return false;
    }
    if primary.is_some_and(|word| matches!(*word, "edit" | "add" | "replace"))
        && words.contains(&"content")
    {
        return false;
    }
    words.iter().enumerate().any(|(index, word)| {
        let intent = word.starts_with("format")
            || word.starts_with("reformat")
            || word.starts_with("restyl")
            || *word == "layout"
            || word.starts_with("typograph");
        let tool_name = word.contains('_');
        let negative = words[index.saturating_sub(5)..index]
            .iter()
            .any(|word| matches!(*word, "not" | "never" | "without"));
        intent && !tool_name && !negative
    })
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
/// Narrow delivery evidence for explicit artifact-production requests. This is
/// not a content-quality classifier and never certifies the model's prose.
pub fn requested_artifacts(request: &str) -> BTreeSet<String> {
    let r = request.to_lowercase();
    let words: Vec<_> = r.split(|c: char| !c.is_alphanumeric()).collect();
    if !words.iter().any(|word| {
        matches!(
            *word,
            "create" | "generate" | "produce" | "draft" | "make" | "write"
        )
    }) {
        return BTreeSet::new();
    }
    if (r.contains("function") || r.contains("script") || r.contains("source code"))
        && !r.contains("run it")
    {
        return BTreeSet::new();
    }
    let mut required = BTreeSet::new();
    if r.contains("docx")
        || (words.contains(&"word") && !r.contains("without word") && !r.contains("markdown"))
    {
        required.insert("docx".into());
    }
    if words
        .iter()
        .any(|w| matches!(*w, "xlsx" | "excel" | "workbook" | "workbooks"))
    {
        required.insert("xlsx".into());
    }
    if words
        .iter()
        .any(|w| matches!(*w, "pptx" | "powerpoint" | "presentation" | "presentations"))
    {
        required.insert("pptx".into());
    }
    if words.contains(&"pdf") || words.contains(&"pdfs") {
        required.insert("pdf".into());
    }
    required
}
/// Conservative explicit deliverable counts. This is not a general intent parser.
pub fn artifact_requirements(request: &str) -> BTreeMap<String, usize> {
    let mut required: BTreeMap<String, usize> = requested_artifacts(request)
        .into_iter()
        .map(|kind| (kind, 1))
        .collect();
    let lower = request.to_lowercase();
    let words: Vec<_> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    for pair in words.windows(2) {
        let count = match pair[0] {
            "two" => Some(2),
            "three" => Some(3),
            "four" => Some(4),
            "five" => Some(5),
            "six" => Some(6),
            "seven" => Some(7),
            "eight" => Some(8),
            "nine" => Some(9),
            "ten" => Some(10),
            other => other
                .parse::<usize>()
                .ok()
                .filter(|n| (1..=100).contains(n)),
        };
        let kind = match pair[1] {
            "word" | "docx" => "docx",
            "excel" | "xlsx" | "workbooks" => "xlsx",
            "powerpoint" | "pptx" | "presentations" => "pptx",
            "pdf" | "pdfs" => "pdf",
            _ => continue,
        };
        if let (Some(count), Some(current)) = (count, required.get_mut(kind)) {
            *current = (*current).max(count);
        }
    }
    required
}

pub fn missing_artifacts(
    required: &BTreeMap<String, usize>,
    created: &BTreeMap<String, String>,
) -> Vec<String> {
    required
        .iter()
        .filter_map(|(kind, count)| {
            let actual = created.values().filter(|value| *value == kind).count();
            (actual < *count).then(|| format!("{kind}: {actual}/{count}"))
        })
        .collect()
}

pub fn created_artifacts(tool: &str, args: &Value, result: &Value) -> BTreeMap<String, String> {
    let canonical = crate::tool_packs::canonical(tool).or_else(|| {
        (tool == "tool_pack_call").then(|| args["tool"].as_str().unwrap_or("").to_string())
    });
    let producer = canonical.as_deref().is_some_and(|name| {
        matches!(
            name,
            "office.docx"
                | "document.add_cover"
                | "office.xlsx"
                | "office.pptx"
                | "pdf.generate"
                | "pdf.merge"
                | "pdf.split"
                | "workflow.run"
        )
    });
    let mut found = BTreeMap::new();
    if producer && result["ok"] == true {
        fn visit(value: &Value, found: &mut BTreeMap<String, String>) {
            if let (Some(id), Some(kind), Some(sha)) = (
                value["artifact_id"].as_str(),
                value["extension"].as_str(),
                value["sha256"].as_str(),
            ) && id.len() == 32
                && sha.len() == 64
            {
                found.insert(id.into(), kind.into());
            }
            if let Some(object) = value.as_object() {
                for child in object.values() {
                    visit(child, found);
                }
            } else if let Some(array) = value.as_array() {
                for child in array {
                    visit(child, found);
                }
            }
        }
        // Count requested deliverables, not nested preview or input metadata.
        let output = match canonical.as_deref() {
            Some("document.add_cover") => &result["result"]["artifact"],
            Some("workflow.run") => &result["result"]["outputs"]["document"],
            _ => &result["result"],
        };
        visit(output, &mut found);
    }
    if crate::design_tools::is_tool(tool)
        && let (Some(path), Some(sha)) = (result["created"].as_str(), result["sha256"].as_str())
        && sha.len() == 64
        && let Some(extension) = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
    {
        found.insert(path.into(), extension.to_lowercase());
    }
    found
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn creation_claims_need_new_producer_evidence_not_an_old_artifact_list() {
        let args = json!({"tool":"artifact.list","arguments":{}});
        let artifact =
            json!({"artifact_id":"a".repeat(32),"extension":"docx","sha256":"b".repeat(64)});
        let listed = json!({"ok":true,"result":{"artifacts":[artifact.clone()]}});
        assert!(created_artifacts("tool_pack_call", &args, &listed).is_empty());
        assert_eq!(
            created_artifacts(
                "pack__office_docx",
                &json!({}),
                &json!({"ok":true,"result":artifact})
            )
            .len(),
            1
        );
        assert!(requested_artifacts("Create Word and Excel reports").contains("docx"));
        assert!(!requested_artifacts("Write a Python function to parse PDF").contains("pdf"));
    }
    #[test]
    fn counts_distinct_outputs_and_excludes_preview_derivatives() {
        let required = artifact_requirements("Create three Word documents and two PDF reports");
        assert_eq!(required["docx"], 3);
        assert_eq!(required["pdf"], 2);
        let doc = json!({"artifact_id":"a".repeat(32),"extension":"docx","sha256":"b".repeat(64)});
        let pdf = json!({"artifact_id":"c".repeat(32),"extension":"pdf","sha256":"d".repeat(64)});
        let result = json!({"ok":true,"result":{"artifact":doc,"preview":{"pdf":pdf}}});
        let created = created_artifacts("pack__document_add_cover", &json!({}), &result);
        assert_eq!(created.len(), 1);
        assert_eq!(
            missing_artifacts(&required, &created),
            vec!["docx: 1/3", "pdf: 0/2"]
        );
        assert!(created_artifacts("pack__office_preview", &json!({}), &result).is_empty());
    }

    #[test]
    fn creation_and_content_editing_do_not_require_the_word_formatter() {
        assert!(!document_format_request(
            "Create a Word proposal from scratch. Use good typography. Do not use format_document."
        ));
        assert!(!document_format_request(
            "Create a document about information architecture."
        ));
        assert!(!document_format_request(
            "Edit the existing Word content without changing its formatting."
        ));
        assert!(document_format_request(
            "Create a newly formatted copy of the existing CV."
        ));
        assert!(document_format_request(
            "Format the CV I previously created from scratch."
        ));
        assert!(document_format_request(
            "Improve the formating of this CV document"
        ));
    }
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
    #[test]
    fn new_output_recovers_only_equivalent_preflight_design_failures() {
        let old = json!({"input":"CV.docx","output":"exists.docx","accent":"#087F83"});
        let new = json!({"operation":"format_document","input":"CV.docx","output":"new.docx","accent":"#087F83","preset":"cv"});
        let mut failures = Failures::default();
        failures.failed_preflight("format_document", &old);
        failures.succeeded("design_tool", &json!({"operation":"format_document","input":"other.docx","output":"new.docx","accent":"#087F83"}));
        assert!(!failures.is_empty());
        failures.succeeded("design_tool", &json!({"operation":"format_document","input":"CV.docx","output":"new.docx","accent":"#000000"}));
        assert!(!failures.is_empty());
        failures.succeeded("design_tool", &new);
        assert!(failures.is_empty());
        // A possibly partial execution failure cannot be erased by a rename,
        // including after a subsequent preflight rejection of the same request.
        failures.failed("format_document", &old);
        failures.failed_preflight("format_document", &old);
        failures.succeeded("design_tool", &new);
        assert!(!failures.is_empty());
        failures.succeeded("format_document", &old);
        assert!(failures.is_empty());
        failures.failed("run_command", &json!({"command":"mkdir existing"}));
        failures.succeeded("design_tool", &new);
        assert!(!failures.is_empty());
    }
}
