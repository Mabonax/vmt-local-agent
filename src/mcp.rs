//! Explicitly reviewed local stdio MCP client (legacy 2025-11-25 lifecycle).
//! No server is started just by opening the app or listing its configuration.
//! Each tools/list or tools/call runs only after a fresh approval. No HTTP MCP yet.
use crate::{store, workspace::clip};
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Server {
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
    !id.is_empty() && id.len() <= 64
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn registry() -> Result<Registry> {
    let path = store::data_dir().join("mcp-servers.json");
    if !path.exists() {
        return Ok(Registry { servers: vec![] });
    }
    let meta = fs::symlink_metadata(&path)?;
    ensure!(meta.is_file() && !meta.file_type().is_symlink() && meta.len() <= 64_000,
        "MCP registry must be a regular JSON file below 64 KiB");
    let registry: Registry = serde_json::from_slice(&fs::read(path)?)?;
    ensure!(registry.servers.len() <= 32, "At most 32 MCP servers are supported");
    for (i, server) in registry.servers.iter().enumerate() {
        ensure!(valid_id(&server.id), "Invalid MCP server ID");
        ensure!(!registry.servers[..i].iter().any(|other| other.id == server.id),
            "Duplicate MCP server ID");
        let path = Path::new(&server.command);
        ensure!(path.is_absolute() && path.is_file(),
            "MCP server executable must be an existing absolute file path");
        ensure!(server.args.len() <= 20 && server.args.iter().all(|s| s.len() <= 1024),
            "MCP server arguments exceed limits");
    }
    Ok(registry)
}

pub fn configured_servers() -> Result<String> {
    let servers = registry()?.servers;
    if servers.is_empty() {
        return Ok("No MCP servers configured. See MCP_AND_CRAFT.md; no processes started.".into());
    }
    Ok(servers.iter().map(|s| format!("{}: {} ({} args)", s.id, s.command, s.args.len()))
        .collect::<Vec<_>>().join("\n"))
}

fn send(child: &mut Child, method: &str, id: u64, params: Value) -> Result<()> {
    let payload = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params});
    let mut line = serde_json::to_vec(&payload)?;
    ensure!(line.len() <= 32_000, "MCP request too large");
    line.push(b'\n');
    child.stdin.as_mut().context("MCP stdin unavailable")?.write_all(&line)?;
    child.stdin.as_mut().context("MCP stdin unavailable")?.flush()?;
    Ok(())
}

fn notify(child: &mut Child) -> Result<()> {
    child.stdin.as_mut().context("MCP stdin unavailable")?
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")?;
    child.stdin.as_mut().context("MCP stdin unavailable")?.flush()?;
    Ok(())
}

fn response(rx: &mpsc::Receiver<std::result::Result<Value, String>>, id: u64, timeout: Duration) -> Result<Value> {
    for _ in 0..128 {
        let message = rx.recv_timeout(timeout).context("MCP response timeout or disconnected server")?
            .map_err(anyhow::Error::msg)?;
        if message.get("id").and_then(Value::as_u64) != Some(id) {
            continue; // legal server notifications
        }
        if let Some(error) = message.get("error") {
            bail!("MCP error: {}", clip(&error.to_string(), 3000));
        }
        return message.get("result").cloned().context("MCP response missing result");
    }
    bail!("MCP server sent too many unrelated messages")
}

/// Only called AFTER the generic review broker approves an exact request.
/// Every invocation is a fresh process; stateful multi-call sessions are not supported.
pub fn run(server_id: &str, operation: &str, tool: &str, args: &Value) -> Result<String> {
    ensure!(valid_id(server_id), "Invalid server ID");
    ensure!(matches!(operation, "tools/list" | "tools/call"), "Unsupported MCP operation");
    if operation == "tools/call" {
        ensure!(!tool.is_empty() && tool.len() <= 128, "Invalid MCP tool name");
        ensure!(args.is_object(), "MCP tool arguments must be a JSON object");
    }
    let registry = registry()?;
    let server = registry.servers.iter().find(|s| s.id == server_id)
        .context("MCP server not registered")?;
    let mut command = Command::new(&server.command);
    command.args(&server.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // No secrets, API keys, or inherited environment variables reach MCP subprocesses.
    // Some platform runtime variables are required to start native Windows apps.
    command.env_clear();
    for key in ["PATH","SystemRoot","WINDIR","TEMP","TMP","LOCALAPPDATA","APPDATA",
        "USERPROFILE","HOMEDRIVE","HOMEPATH","PROGRAMFILES"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    let mut child = command.spawn().context("Could not launch MCP server")?;
    let outcome = (|| -> Result<String> {
        let stdout = child.stdout.take().context("MCP stdout unavailable")?;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = Vec::new();
                match (&mut reader).take(128 * 1024 + 1).read_until(b'\n', &mut line) {
                    Ok(0) => break,
                    Ok(n) if n > 128 * 1024 => {
                        let _ = tx.send(Err("MCP record exceeds 128 KiB".into()));
                        break;
                    }
                    Ok(_) => match serde_json::from_slice::<Value>(&line) {
                        Ok(v) => { if tx.send(Ok(v)).is_err() { break; } }
                        Err(_) => { let _ = tx.send(Err("Invalid MCP JSON response".into())); break; }
                    },
                    Err(e) => { let _ = tx.send(Err(e.to_string())); break; }
                }
            }
        });
        send(&mut child, "initialize", 1, json!({
            "protocolVersion":"2025-11-25",
            "capabilities":{},
            "clientInfo":{"name":"vmt-local-agent","version":"0.3.0"}
        }))?;
        let init = response(&rx, 1, Duration::from_secs(15))?;
        ensure!(init["protocolVersion"].as_str() == Some("2025-11-25"),
            "Server did not negotiate supported MCP protocol 2025-11-25");
        notify(&mut child)?;
        if operation == "tools/list" {
            send(&mut child, "tools/list", 2, json!({}))?;
        } else {
            send(&mut child, "tools/call", 2, json!({"name":tool,"arguments":args}))?;
        }
        let result = response(&rx, 2, Duration::from_secs(60))?;
        if operation == "tools/list" {
            let names = result["tools"].as_array().context("MCP tools/list returned no tools")?;
            let entries: Vec<_> = names.iter().take(80).map(|v| {
                format!("{}: {}", v["name"].as_str().unwrap_or("?"),
                    clip(v["description"].as_str().unwrap_or(""), 180))
            }).collect();
            Ok(format!("UNTRUSTED MCP TOOL CATALOG for {server_id}:\n{}",
                clip(&entries.join("\n"), 11_000)))
        } else {
            ensure!(result["isError"].as_bool() != Some(true),
                "MCP tool returned an error: {}", clip(&result.to_string(), 4000));
            Ok(format!("UNTRUSTED MCP TOOL RESULT from {server_id}/{tool}:\n{}",
                clip(&result.to_string(), 12_000)))
        }
    })();
    let _ = child.kill();
    let _ = child.wait();
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mcp_ids_are_strict() {
        for invalid in ["../x","", "SHELL", "a/b", "tool:name"] {
            assert!(!valid_id(invalid));
        }
        assert!(valid_id("wordcraft"));
    }
    #[test]
    fn empty_registry_does_not_launch_processes() {
        // Registry path is user-specific; validate parsing shape without reading user files.
        assert!(serde_json::from_str::<Registry>(r#"{"servers":[]}"#).is_ok());
        assert!(serde_json::from_str::<Registry>(r#"{"servers":[{"id":"x","command":"a","extra":1}]}"#).is_err());
    }
}
