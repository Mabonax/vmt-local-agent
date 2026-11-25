//! Host-owned competency guidance. Matching is advisory, not authorization.
//! Availability is derived from actual discovery, not merely registration.
use serde_json::Value;

pub struct Competency {
    pub id: &'static str,
    pub skill: &'static str,
    pub triggers: &'static [&'static str],
    pub tools: &'static [&'static str],
    pub verification: &'static str,
    pub limitation: &'static str,
}

pub const REGISTRY: &[Competency] = &[
    Competency {
        id: "word-production",
        skill: "word-documents",
        triggers: &["word", "docx", "cv", "resume"],
        tools: &[
            "office.docx",
            "document.replace_text",
            "document.add_cover",
            "artifact.inspect",
            "office.preview",
        ],
        verification: "Verify new artifact, preserve source on edits, render when configured.",
        limitation: "Exact simple-run edits only; advanced design needs reviewed adapters.",
    },
    Competency {
        id: "pdf-production",
        skill: "pdf-documents",
        triggers: &["pdf", "pdfs"],
        tools: &[
            "pdf.generate",
            "workflow.run",
            "pdf.inspect",
            "pdf.extract_text",
            "pdf.render",
            "pdf.merge",
            "pdf.split",
        ],
        verification: "Inspect actual output, page count and required content; render for review.",
        limitation: "No OCR; preview workflow checks only its rendered page.",
    },
    Competency {
        id: "workbook-production",
        skill: "spreadsheets",
        triggers: &["excel", "xlsx", "spreadsheet", "workbook", "workbooks"],
        tools: &[
            "office.xlsx",
            "finance.check_total",
            "artifact.inspect",
            "office.preview",
        ],
        verification: "Verify data and arithmetic separately, then inspect the workbook.",
        limitation: "Literal cells only; no formula authoring or recalculation.",
    },
    Competency {
        id: "presentation-production",
        skill: "presentations",
        triggers: &["powerpoint", "pptx", "presentation", "slides", "deck"],
        tools: &["office.pptx", "artifact.inspect", "office.preview"],
        verification: "Check actual slide count, content and rendered layout.",
        limitation: "Basic deck producer; advanced design needs reviewed adapters.",
    },
    Competency {
        id: "repository-review",
        skill: "code-review",
        triggers: &["repository", "codebase", "rust", "cargo", "debug"],
        tools: &["project.detect", "code.metrics", "code.dependencies"],
        verification: "Inspect real files; run reviewed tests when requested; cite evidence.",
        limitation: "Metrics do not establish correctness or complete code coverage.",
    },
];

pub fn select(request: &str) -> Vec<&'static Competency> {
    let lower = request.to_lowercase();
    let words: Vec<_> = lower.split(|c: char| !c.is_alphanumeric()).collect();
    REGISTRY
        .iter()
        .filter(|entry| entry.triggers.iter().any(|word| words.contains(word)))
        .take(4)
        .collect()
}

/// Only selected schemas are marked discovered; all other tools require discovery.
/// This does not expand the model's registered functions or bypass approval.
pub fn guidance(request: &str, discovery: Option<&Value>) -> String {
    let selected = select(request);
    if selected.is_empty() {
        return String::new();
    }
    let available: Vec<_> = discovery
        .and_then(|value| value["tools"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|entry| entry["definition"]["name"].as_str())
        .collect();
    let rows: Vec<_> = selected
        .iter()
        .map(|entry| {
            let present: Vec<_> = entry
                .tools
                .iter()
                .copied()
                .filter(|name| available.contains(name))
                .collect();
            let remaining: Vec<_> = entry
                .tools
                .iter()
                .copied()
                .filter(|name| !available.contains(name))
                .collect();
            format!(
                "{}: skill {}; discovered now: {}; candidates requiring discovery: {}; checks: {}; limits: {}",
                entry.id,
                entry.skill,
                present.join(", "),
                remaining.join(", "),
                entry.verification,
                entry.limitation
            )
        })
        .collect();
    format!(
        "Host competency hints (not grants; choose only relevant operations, verify actual runtime availability):\n{}",
        rows.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn registry_references_real_skills_and_tools() {
        let catalog = crate::tool_packs::catalog();
        let mut ids = std::collections::BTreeSet::new();
        for entry in REGISTRY {
            assert!(ids.insert(entry.id));
            assert!(crate::skills::is_builtin(entry.skill));
            for name in entry.tools {
                assert!(catalog.contains_key(*name), "Missing capability {name}");
            }
        }
    }

    #[test]
    fn discovery_does_not_turn_candidates_into_available_tools() {
        let discovery = json!({"tools":[{"definition":{"name":"office.docx"}}]});
        let text = guidance("Create Word report", Some(&discovery));
        assert!(text.contains("discovered now: office.docx;"));
        assert!(text.contains("document.replace_text"));
        assert!(text.contains("not grants"));
        assert!(guidance("What is weather?", None).is_empty());
        assert!(select("swordfish").is_empty());
    }
}
