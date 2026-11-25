//! Narrow protocol normalization, never authorization or argument invention.
use crate::types::Action;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

/// Decode an object or one JSON-encoded object. No recursive coercion.
pub fn object_arguments(value: Value) -> Result<Value> {
    let value = match value {
        Value::String(text) => serde_json::from_str(&text)
            .context("Tool arguments must be a valid JSON object")?,
        other => other,
    };
    ensure!(value.is_object(), "Tool arguments must be an object");
    Ok(value)
}

/// Normalize exact registered pack names only. Do not fuzzy-match names,
/// remove fields, supply missing values or alter the underlying arguments.
pub fn normalize(action: &mut Action) -> Result<()> {
    if action.kind != "tool" {
        return Ok(());
    }
    action.args = object_arguments(std::mem::take(&mut action.args))?;
    if crate::tool_packs::catalog().contains_key(&action.tool) {
        action.args = json!({"tool":action.tool,"arguments":action.args});
        action.tool = "tool_pack_call".into();
    }
    Ok(())
}

/// Select recovery guidance from the actual task contract. A document task
/// must not be redirected to arbitrary source-code inspection.
pub fn no_execution_guidance(document_task: bool) -> &'static str {
    if document_task {
        "No tools have executed. Discover the document capability or use its provided schema. Produce the requested artifact through the normal review path; do not inspect unrelated source code or claim completion."
    } else {
        "No tools have executed. Choose a provided tool relevant to the original request, inspect its actual result, and continue. Do not claim completion."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_preserves_arguments_and_exact_names() {
        let mut action = Action {
            kind: "tool".into(),
            message: String::new(),
            tool: "finance.check_total".into(),
            args: json!("{\"currency\":\"ZAR\",\"amounts\":[\"0.10\"],\"claimed_total\":\"0.10\"}"),
        };
        normalize(&mut action).unwrap();
        assert_eq!(action.tool, "tool_pack_call");
        assert_eq!(action.args["tool"], "finance.check_total");
        assert_eq!(action.args["arguments"]["amounts"][0], "0.10");
        normalize(&mut action).unwrap();
        assert_eq!(action.args["arguments"]["currency"], "ZAR");
    }

    #[test]
    fn malformed_arguments_and_unregistered_names_are_not_repaired() {
        for value in [json!("{} trailing"), json!("[]"), json!(null), json!(3)] {
            assert!(object_arguments(value).is_err());
        }
        let mut action = Action {
            kind: "tool".into(),
            message: String::new(),
            tool: "office.docxx".into(),
            args: json!({"unexpected":true}),
        };
        normalize(&mut action).unwrap();
        assert_eq!(action.tool, "office.docxx");
        assert_eq!(action.args["unexpected"], true);
        assert!(no_execution_guidance(true).contains("document capability"));
    }
}
