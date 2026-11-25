//! Provider-reported capability profile, not a measured model benchmark.
use anyhow::{Result, ensure};
use serde_json::Value;

#[derive(Debug, PartialEq, Eq)]
pub struct ModelProfile {
    pub tools: bool,
    pub vision: bool,
    pub thinking: bool,
}

impl ModelProfile {
    pub fn from_local_show(model: &str, info: &Value) -> Result<Self> {
        ensure!(
            !model.ends_with(":cloud")
                && info["remote_host"].is_null()
                && info["remote_model"].is_null()
                && info["details"]["format"]
                    .as_str()
                    .is_some_and(|f| !f.is_empty()),
            "This model is not a local weights model. Cloud runtimes are disabled."
        );
        let caps = info["capabilities"].as_array();
        let has =
            |name: &str| caps.is_some_and(|items| items.iter().any(|c| c.as_str() == Some(name)));
        ensure!(
            has("completion"),
            "Select a chat/completion model rather than an embedding model"
        );
        Ok(Self {
            tools: has("tools"),
            vision: has("vision"),
            thinking: has("thinking"),
        })
    }

    pub fn native_mode(&self, preference: &str) -> bool {
        match preference {
            "json" => false,
            "native" => true,
            _ => self.tools,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reported_capabilities_select_protocol_without_claiming_reliability() {
        let info = json!({"details":{"format":"gguf"},"capabilities":["completion","tools"]});
        let profile = ModelProfile::from_local_show("local", &info).unwrap();
        assert!(profile.native_mode("auto"));
        assert!(!profile.native_mode("json"));
        assert!(!profile.vision);
        assert!(ModelProfile::from_local_show("local:cloud", &info).is_err());
        assert!(ModelProfile::from_local_show("local", &json!({})).is_err());
    }
}
