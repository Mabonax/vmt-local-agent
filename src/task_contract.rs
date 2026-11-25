//! Host-derived task requirements. A contract is not an authorization grant.
//! V1 tracks explicit artifact counts and existing formatter evidence only.
use crate::task_verification;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub struct TaskContract {
    required: BTreeMap<String, usize>,
    requires_formatter: bool,
    requires_tools: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Assessment {
    pub missing_artifacts: Vec<String>,
    pub missing_formatter: bool,
    pub missing_tool_evidence: bool,
}

impl Assessment {
    pub fn complete(&self) -> bool {
        self.missing_artifacts.is_empty() && !self.missing_formatter && !self.missing_tool_evidence
    }
}

impl TaskContract {
    pub fn from_request(request: &str, requires_tools: bool) -> Self {
        Self {
            required: task_verification::artifact_requirements(request),
            requires_formatter: task_verification::document_format_request(request),
            requires_tools,
        }
    }

    pub fn assess(
        &self,
        created: &BTreeMap<String, String>,
        formatted: &BTreeSet<String>,
        completed_tools: usize,
    ) -> Assessment {
        Assessment {
            missing_artifacts: task_verification::missing_artifacts(&self.required, created),
            missing_formatter: self.requires_formatter && formatted.is_empty(),
            missing_tool_evidence: self.requires_tools && completed_tools == 0,
        }
    }

    /// Compact host-authored constraints; original user request remains authoritative.
    pub fn guidance(&self) -> String {
        let outputs = self
            .required
            .iter()
            .map(|(kind, count)| format!("{kind}: {count}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "Runtime task contract (conservative inferred requirements, not permissions): required new outputs: {}; existing DOCX formatting evidence required: {}; tool evidence required: {}. Preserve the original request and source documents. Distinguish supplied facts from proposals and unknowns. Never invent completed work. Preview derivatives do not count as deliverables. Content accuracy and visual quality need separate verification.",
            if outputs.is_empty() {
                "none inferred"
            } else {
                &outputs
            },
            self.requires_formatter,
            self.requires_tools,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_counts_outputs_and_survives_model_claims() {
        let contract = TaskContract::from_request("Create two Word reports and three PDFs", true);
        let mut created = BTreeMap::from([("one".into(), "docx".into())]);
        let assessment = contract.assess(&created, &BTreeSet::new(), 1);
        assert_eq!(assessment.missing_artifacts, vec!["docx: 1/2", "pdf: 0/3"]);
        assert!(!assessment.complete());
        created.insert("two".into(), "docx".into());
        for n in 0..3 {
            created.insert(format!("pdf-{n}"), "pdf".into());
        }
        assert!(contract.assess(&created, &BTreeSet::new(), 5).complete());
    }

    #[test]
    fn formatter_and_tool_evidence_are_independent() {
        let contract = TaskContract::from_request("Format the existing CV", true);
        let assessment = contract.assess(&BTreeMap::new(), &BTreeSet::new(), 0);
        assert!(assessment.missing_formatter);
        assert!(assessment.missing_tool_evidence);
        assert!(!assessment.complete());
        assert!(
            contract
                .assess(&BTreeMap::new(), &BTreeSet::from(["new.docx".into()]), 1)
                .complete()
        );
    }
}
