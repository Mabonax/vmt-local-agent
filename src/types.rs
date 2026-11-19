use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::mpsc::Sender;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChatEntry {
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub workspace: String,
    pub model: String,
    pub updated: String,
    pub entries: Vec<ChatEntry>,
}

impl Session {
    pub fn fresh(workspace: String, model: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: "New conversation".into(),
            workspace,
            model,
            updated: chrono::Local::now().to_rfc3339(),
            entries: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub endpoint: String,
    pub model: String,
    pub context_tokens: u32,
    pub output_tokens: u32,
    pub max_steps: u32,
    pub last_workspace: String,
    pub read_only: bool,
    pub web_enabled: bool,
    pub protocol: String,
    pub mode: String,
    #[serde(skip)]
    pub edit_file: Option<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:11434".into(),
            model: "qwen3.5:4b".into(),
            context_tokens: 8192,
            output_tokens: 2048,
            max_steps: 12,
            last_workspace: String::new(),
            read_only: false,
            web_enabled: false,
            protocol: "auto".into(),
            mode: "agent".into(),
            edit_file: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Action {
    pub kind: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub tool: String,
    #[serde(default)]
    pub args: Value,
}

#[derive(Clone, Debug)]
pub enum Proposal {
    File {
        path: String,
        before: Option<String>,
        after: String,
    },
    Command {
        command: String,
    },
    Web {
        tool: String,
        args: Value,
    },
    MemoryNote {
        content: String,
    },
    Mcp {
        server: String,
        launch: crate::mcp::Server,
        operation: String,
        tool: String,
        arguments: Value,
    },
    KnowledgeIndex {
        model: String,
        endpoint: String,
    },
    Workflow {
        action: String,
        kind: String,
        evidence: String,
    },
}

#[derive(Debug)]
pub struct Approval {
    pub title: String,
    pub detail: String,
    pub proposal: Proposal,
    pub reply: Sender<bool>,
}

#[derive(Clone, Debug)]
pub struct Activity {
    pub title: String,
    pub detail: String,
    pub state: String,
}

#[derive(Debug)]
pub enum Event {
    Status(String),
    Note(String),
    Final(String),
    Activity(Activity),
    Approval(Approval),
    Finished,
    Error(String),
    Models(Result<Vec<String>, String>),
}
