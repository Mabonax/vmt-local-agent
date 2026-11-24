//! Local stdio MCP client for the negotiated 2025-11-25 protocol.
//! Servers start only after explicit approval; sessions survive tool calls during
//! the active agent turn and are closed on task completion. No remote transport.
use crate::{store, workspace::clip};
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Server {
    id: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    servers: Vec<Server>,
}
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn registry() -> Result<Registry> {
    let path = store::data_dir().join("mcp-servers.json");
    if !path.exists() {
        return Ok(Registry { servers: vec![] });
    }
    store::check_local_path(&path)?;
    let meta = fs::symlink_metadata(&path)?;
    ensure!(
        meta.is_file() && !meta.file_type().is_symlink() && meta.len() <= 64_000,
        "MCP registry must be a regular JSON file below 64 KiB"
    );
    let registry: Registry = serde_json::from_slice(&fs::read(path)?)?;
    ensure!(
        registry.servers.len() <= 32,
        "At most 32 MCP servers supported"
    );
    for (i, server) in registry.servers.iter().enumerate() {
        ensure!(valid_id(&server.id), "Invalid MCP server ID");
        ensure!(
            !registry.servers[..i]
                .iter()
                .any(|other| other.id == server.id),
            "Duplicate MCP server ID"
        );
        let executable = Path::new(&server.command);
        ensure!(
            executable.is_absolute() && executable.is_file(),
            "MCP executable must be an existing absolute file path"
        );
        ensure!(
            server.args.len() <= 20 && server.args.iter().all(|s| s.len() <= 1024),
            "MCP server arguments exceed limits"
        );
    }
    Ok(registry)
}
pub fn describe_server(id: &str) -> Result<String> {
    snapshot(id)?.describe()
}
pub fn snapshot(id: &str) -> Result<Server> {
    ensure!(valid_id(id), "Invalid MCP server ID");
    let server = registry()?
        .servers
        .into_iter()
        .find(|server| server.id == id)
        .context("MCP server not registered")?;
    Ok(server)
}
impl Server {
    pub fn describe(&self) -> Result<String> {
        Ok(format!(
            "Executable: {}\nArguments: {}",
            self.command,
            serde_json::to_string(&self.args)?
        ))
    }
}

pub fn configured_servers() -> Result<String> {
    let servers = registry()?.servers;
    if servers.is_empty() {
        return Ok("No local MCP servers configured; see MCP_AND_CRAFT.md.".into());
    }
    Ok(servers
        .iter()
        .map(|s| format!("{}: {} ({} args)", s.id, s.command, s.args.len()))
        .collect::<Vec<_>>()
        .join("\n"))
}
type Wire = std::result::Result<Value, String>;
struct Session {
    command: String,
    args: Vec<String>,
    child: Child,
    rx: mpsc::Receiver<Wire>,
    next_id: u64,
    discovered: HashSet<String>,
    schemas: HashMap<String, Value>,
}
impl Drop for Session {
    fn drop(&mut self) {
        // Drop stdin first, then terminate the owned process tree. Approval is
        // not isolation, but children must not survive a cancelled tool turn.
        drop(self.child.stdin.take());
        #[cfg(windows)]
        if self.child.try_wait().ok().flatten().is_none() {
            use std::os::windows::process::CommandExt;
            let _ = Command::new("taskkill.exe")
                .args(["/PID", &self.child.id().to_string(), "/T", "/F"])
                .creation_flags(0x08000000)
                .output();
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
static SESSIONS: OnceLock<Mutex<HashMap<String, Session>>> = OnceLock::new();
fn sessions() -> &'static Mutex<HashMap<String, Session>> {
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}
pub fn disconnect_all() {
    if let Ok(mut active) = sessions().lock() {
        active.clear();
    }
}
fn write_message(
    session: &mut Session,
    payload: &Value,
    cancel: &AtomicBool,
    timeout: Duration,
) -> Result<()> {
    let mut bytes = serde_json::to_vec(payload)?;
    ensure!(bytes.len() <= 32_000, "MCP request too large");
    bytes.push(b'\n');
    let mut stdin = session
        .child
        .stdin
        .take()
        .context("MCP stdin unavailable")?;
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let result = stdin.write_all(&bytes).and_then(|_| stdin.flush());
        let _ = tx.send((stdin, result));
    });
    let started = Instant::now();
    while started.elapsed() < timeout {
        ensure!(!cancel.load(Ordering::Relaxed), "MCP send cancelled");
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok((stdin, result)) => {
                session.child.stdin = Some(stdin);
                result?;
                return Ok(());
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => bail!("MCP writer disconnected"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    bail!("MCP send timed out")
}
fn request(
    session: &mut Session,
    method: &str,
    params: Value,
    cancel: &AtomicBool,
    timeout: Duration,
) -> Result<Value> {
    let id = session.next_id;
    session.next_id += 1;
    let started = Instant::now();
    write_message(
        session,
        &json!({
            "jsonrpc":"2.0", "id":id, "method":method, "params":params
        }),
        cancel,
        timeout,
    )?;
    while started.elapsed() < timeout {
        ensure!(!cancel.load(Ordering::Relaxed), "MCP request cancelled");
        let remaining = timeout.saturating_sub(started.elapsed());
        match session
            .rx
            .recv_timeout(remaining.min(Duration::from_millis(250)))
        {
            Ok(Ok(message)) => {
                ensure!(message["jsonrpc"] == "2.0", "Invalid MCP JSON-RPC version");
                if message.get("id").and_then(Value::as_u64) != Some(id) {
                    continue;
                }
                if let Some(error) = message.get("error") {
                    bail!("MCP error: {}", clip(&error.to_string(), 3000));
                }
                return message
                    .get("result")
                    .cloned()
                    .context("MCP response missing result");
            }
            Ok(Err(error)) => bail!("MCP stream failed: {error}"),
            Err(mpsc::RecvTimeoutError::Disconnected) => bail!("MCP server disconnected"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    bail!("MCP {method} timed out")
}
fn spawn(server: &Server, cancel: &AtomicBool) -> Result<Session> {
    ensure!(!cancel.load(Ordering::Relaxed), "MCP startup cancelled");
    let mut cmd = Command::new(&server.command);
    cmd.args(&server.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env_clear();
    // Allow only minimal platform variables; API keys and other app secrets
    // are not inherited. Root flags on Craft servers remain defense-in-depth.
    for key in [
        "PATH",
        "SystemRoot",
        "WINDIR",
        "TEMP",
        "TMP",
        "LOCALAPPDATA",
        "APPDATA",
        "USERPROFILE",
        "HOMEDRIVE",
        "HOMEPATH",
        "PROGRAMFILES",
    ] {
        if let Some(value) = std::env::var_os(key) {
            cmd.env(key, value);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = cmd
        .spawn()
        .context("Cannot start configured MCP executable")?;
    let stdout = child.stdout.take().context("MCP stdout unavailable")?;
    let (tx, rx) = mpsc::sync_channel(16);
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut bytes = Vec::new();
            match (&mut reader)
                .take(128 * 1024 + 1)
                .read_until(b'\n', &mut bytes)
            {
                Ok(0) => break,
                Ok(n) if n > 128 * 1024 => {
                    let _ = tx.send(Err("MCP response record exceeded 128 KiB".into()));
                    break;
                }
                Ok(_) => match serde_json::from_slice::<Value>(&bytes) {
                    Ok(message) => {
                        if tx.send(Ok(message)).is_err() {
                            break;
                        }
                    }
                    Err(_) => {
                        let _ = tx.send(Err("Invalid JSON from MCP server".into()));
                        break;
                    }
                },
                Err(err) => {
                    let _ = tx.send(Err(err.to_string()));
                    break;
                }
            }
        }
    });
    let mut session = Session {
        command: server.command.clone(),
        args: server.args.clone(),
        child,
        rx,
        next_id: 1,
        discovered: HashSet::new(),
        schemas: HashMap::new(),
    };
    let init = request(
        &mut session,
        "initialize",
        json!({
            "protocolVersion":"2025-11-25",
            "capabilities":{},
            "clientInfo":{"name":"vmt-local-agent","version":env!("CARGO_PKG_VERSION")}
        }),
        cancel,
        Duration::from_secs(15),
    )?;
    ensure!(
        matches!(
            init["protocolVersion"].as_str(),
            Some("2025-11-25" | "2025-06-18")
        ),
        "Unsupported MCP protocol; supported stdio lifecycles: 2025-11-25 and 2025-06-18"
    );
    ensure!(
        init["capabilities"]["tools"].is_object(),
        "MCP server did not advertise tools capability"
    );
    write_message(
        &mut session,
        &json!({
            "jsonrpc":"2.0","method":"notifications/initialized"
        }),
        cancel,
        Duration::from_secs(5),
    )?;
    Ok(session)
}
/// Retrieve a precise cached schema after reviewed tools/list discovery.
pub fn tool_schema(server: &str, tool: &str) -> Result<String> {
    ensure!(
        valid_id(server) && !tool.is_empty() && tool.len() <= 128,
        "Invalid MCP server/tool ID"
    );
    let active = sessions()
        .lock()
        .map_err(|_| anyhow::anyhow!("MCP lock poisoned"))?;
    let session = active
        .get(server)
        .context("Run reviewed mcp_tools before requesting a schema")?;
    let schema = session
        .schemas
        .get(tool)
        .context("MCP tool was not discovered")?;
    let definition = serde_json::to_string_pretty(schema)?;
    ensure!(
        definition.len() <= 12_000,
        "MCP schema exceeds the 12 KB context limit; no partial schema returned"
    );
    Ok(format!(
        "UNTRUSTED MCP TOOL DEFINITION {} / {}:\n{}",
        server, tool, definition
    ))
}

/// Invoked only following review of the displayed MCP operation. Sessions stay
/// active within a task, then the agent runtime closes them with disconnect_all.
pub fn run_reviewed(
    server_id: &str,
    approved: &Server,
    operation: &str,
    tool: &str,
    args: &Value,
    cancel: &AtomicBool,
) -> Result<String> {
    run_reviewed_in(server_id, approved, operation, tool, args, cancel, None)
}
pub fn run_reviewed_with_workspace(
    server_id: &str,
    approved: &Server,
    operation: &str,
    tool: &str,
    args: &Value,
    cancel: &AtomicBool,
    workspace: &crate::workspace::Workspace,
) -> Result<String> {
    run_reviewed_in(
        server_id,
        approved,
        operation,
        tool,
        args,
        cancel,
        Some(workspace),
    )
}
fn run_reviewed_in(
    server_id: &str,
    approved: &Server,
    operation: &str,
    tool: &str,
    args: &Value,
    cancel: &AtomicBool,
    workspace: Option<&crate::workspace::Workspace>,
) -> Result<String> {
    ensure!(valid_id(server_id), "Invalid MCP server ID");
    ensure!(
        matches!(operation, "tools/list" | "tools/call"),
        "Unsupported MCP operation"
    );
    if operation == "tools/call" {
        ensure!(!tool.is_empty() && tool.len() <= 128, "Invalid tool name");
        ensure!(args.is_object(), "Tool arguments must be a JSON object");
    }
    let config = registry()?;
    let server = config
        .servers
        .iter()
        .find(|s| s.id == server_id)
        .context("MCP server is not registered")?;
    ensure!(
        server == approved,
        "MCP configuration changed after review; request fresh approval"
    );
    let mut sessions = sessions()
        .lock()
        .map_err(|_| anyhow::anyhow!("MCP session lock poisoned"))?;
    let reuse = sessions
        .get(server_id)
        .is_some_and(|existing| existing.command == server.command && existing.args == server.args);
    if !reuse {
        sessions.remove(server_id);
        sessions.insert(server_id.into(), spawn(server, cancel)?);
    }
    let result = (|| -> Result<String> {
        let session = sessions.get_mut(server_id).context("MCP session missing")?;
        let mut result = if operation == "tools/list" {
            let mut tools = Vec::new();
            let mut cursors = HashSet::new();
            let mut params = json!({});
            let began = Instant::now();
            loop {
                let remaining = Duration::from_secs(60).saturating_sub(began.elapsed());
                ensure!(!remaining.is_zero(), "MCP discovery timed out");
                let page = request(session, "tools/list", params, cancel, remaining)?;
                tools.extend(
                    page["tools"]
                        .as_array()
                        .context("MCP server has no tools list")?
                        .iter()
                        .cloned(),
                );
                ensure!(tools.len() <= 500, "MCP tool count exceeds 500");
                let Some(cursor) = page.get("nextCursor").filter(|v| !v.is_null()) else {
                    break;
                };
                let cursor = cursor.as_str().context("Invalid MCP pagination cursor")?;
                ensure!(
                    cursor.len() <= 1024
                        && cursors.insert(cursor.to_string())
                        && cursors.len() <= 10,
                    "Invalid or excessive MCP pagination"
                );
                params = json!({"cursor":cursor});
            }
            json!({"tools":tools})
        } else {
            ensure!(
                session.discovered.contains(tool),
                "Call mcp_tools and select a discovered tool before invoking it"
            );
            request(
                session,
                "tools/call",
                json!({"name":tool,"arguments":args}),
                cancel,
                Duration::from_secs(60),
            )?
        };
        if operation == "tools/list" {
            let list = result["tools"]
                .as_array()
                .context("MCP server has no tools list")?;
            let mut names = HashSet::new();
            for item in list {
                let name = item["name"].as_str().context("Missing MCP tool name")?;
                ensure!(
                    !name.is_empty()
                        && name.len() <= 128
                        && names.insert(name)
                        && item["inputSchema"].is_object(),
                    "Invalid or duplicate MCP tool schema"
                );
            }
            session.schemas = list
                .iter()
                .filter_map(|item| {
                    item["name"]
                        .as_str()
                        .map(|name| (name.to_owned(), item.clone()))
                })
                .collect();
            session.discovered = session.schemas.keys().cloned().collect();
            let entries: Vec<String> = list
                .iter()
                .take(50)
                .map(|item| {
                    format!(
                        "{}: {}\nSchema: {}",
                        item["name"].as_str().unwrap_or("?"),
                        clip(item["description"].as_str().unwrap_or(""), 130),
                        clip(&item["inputSchema"].to_string(), 350)
                    )
                })
                .collect();
            Ok(format!(
                "UNTRUSTED MCP TOOL DEFINITIONS from {server_id}:\n{}",
                clip(&entries.join("\n"), 12_000)
            ))
        } else {
            strip_binary(&mut result, workspace);
            ensure!(
                result["isError"].as_bool() != Some(true),
                "MCP tool failed: {}",
                clip(&result.to_string(), 4000)
            );
            Ok(format!(
                "UNTRUSTED MCP TOOL RESULT from {server_id}/{tool}:\n{}",
                clip(&result.to_string(), 12_000)
            ))
        }
    })();
    if result.is_err() {
        sessions.remove(server_id); // stop crashed, timed-out or cancelled process
    }
    result
}
fn strip_binary(value: &mut Value, workspace: Option<&crate::workspace::Workspace>) {
    if let Some(object) = value.as_object_mut() {
        let kind = object.get("type").and_then(Value::as_str).unwrap_or("");
        let mime = object
            .get("mimeType")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let binary = matches!(kind, "image" | "audio")
            || object.contains_key("blob")
            || (!mime.is_empty() && object.contains_key("data"));
        if binary {
            let data = object.remove("data").or_else(|| object.remove("blob"));
            let extension = match mime.as_str() {
                "image/png" => Some("png"),
                "image/jpeg" => Some("jpg"),
                "application/pdf" => Some("pdf"),
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                    Some("docx")
                }
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => Some("xlsx"),
                "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
                    Some("pptx")
                }
                _ => None,
            };
            let artifact = workspace
                .zip(extension)
                .zip(data.as_ref().and_then(Value::as_str))
                .and_then(|((workspace, extension), data)| {
                    crate::tool_packs::receive_artifact(workspace, extension, data).ok()
                });
            object.insert("binary_omitted".into(), json!(true));
            object.insert("artifact".into(),artifact.unwrap_or_else(||json!({"status":"not_registered","reason":"Unsupported binary or artifact runtime unavailable; raw bytes are excluded from model context"})));
        }
        for child in object.values_mut() {
            strip_binary(child, workspace);
        }
    } else if let Some(values) = value.as_array_mut() {
        for child in values {
            strip_binary(child, workspace);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binary_mcp_payloads_never_enter_model_context() {
        let mut result = json!({"content":[{"type":"image","mimeType":"image/png","data":"private-base64"},{"type":"resource","resource":{"mimeType":"application/pdf","blob":"private-document"}},{"type":"text","text":"kept evidence"}]});
        strip_binary(&mut result, None);
        assert!(!result.to_string().contains("private-base64"));
        assert!(!result.to_string().contains("private-document"));
        assert!(result.to_string().contains("kept evidence"));
    }
    #[test]
    fn server_id_validation() {
        assert!(valid_id("wordcraft"));
        for bad in ["", "../escape", "C:", "UPPER", "a/b"] {
            assert!(!valid_id(bad));
        }
    }
    #[test]
    fn rejects_registry_unknown_keys() {
        assert!(serde_json::from_str::<Registry>(r#"{"servers":[]}"#).is_ok());
        assert!(
            serde_json::from_str::<Registry>(
                r#"{"servers":[{"id":"x","command":"a","extra":"bad"}]}"#
            )
            .is_err()
        );
    }
}
