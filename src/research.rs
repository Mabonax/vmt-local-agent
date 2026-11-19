//! Minimal source provenance for approved web fetches, kept separate from
//! model prose. Never store URL query strings, response bodies, or cookies.
use crate::{store, workspace::Workspace};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};

#[derive(Serialize, Deserialize)]
struct Source {
    at: String,
    url: String,
    #[serde(default)]
    kind: String,
}
#[derive(Default, Serialize, Deserialize)]
struct Ledger {
    workspace: String,
    sources: Vec<Source>,
}
fn path(ws: &Workspace) -> PathBuf {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in ws.root.to_string_lossy().as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    store::data_dir()
        .join("research")
        .join(format!("{hash:016x}.json"))
}
fn load(ws: &Workspace) -> Result<Ledger> {
    let file = path(ws);
    store::check_local_path(&file)?;
    if !file.exists() {
        return Ok(Ledger {
            workspace: ws.root.to_string_lossy().into_owned(),
            sources: vec![],
        });
    }
    let meta = fs::symlink_metadata(&file)?;
    ensure!(
        meta.is_file() && !meta.file_type().is_symlink() && meta.len() <= 64 * 1024,
        "Research ledger is invalid"
    );
    let ledger: Ledger = serde_json::from_slice(&fs::read(file)?)?;
    ensure!(
        ledger.workspace == ws.root.to_string_lossy(),
        "Research ledger scope mismatch"
    );
    Ok(ledger)
}
pub fn record_fetch(ws: &Workspace, url: &str) -> Result<()> {
    let mut parsed = reqwest::Url::parse(url)?;
    ensure!(
        parsed.scheme() == "https",
        "Only HTTPS source URLs can be recorded"
    );
    ensure!(
        parsed.username().is_empty() && parsed.password().is_none(),
        "Source URL credentials are forbidden"
    );
    // The approved URL may have credential-like query parameters. Do not persist them.
    parsed.set_query(None);
    parsed.set_fragment(None);
    let mut ledger = load(ws)?;
    ledger.sources.push(Source {
        at: chrono::Utc::now().to_rfc3339(),
        url: parsed.to_string(),
        kind: "approved-fetch".into(),
    });
    if ledger.sources.len() > 100 {
        ledger.sources.drain(..ledger.sources.len() - 100);
    }
    let file = path(ws);
    store::check_local_path(&file)?;
    let parent = file.parent().context("Missing research folder")?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(&serde_json::to_vec_pretty(&ledger)?)?;
    temp.as_file().sync_all()?;
    store::persist_local(temp, &file)?;
    Ok(())
}
pub fn list_sources(ws: &Workspace) -> Result<String> {
    let ledger = load(ws)?;
    if ledger.sources.is_empty() {
        return Ok("No approved HTTPS sources recorded for this project.".into());
    }
    let sources = ledger
        .sources
        .iter()
        .rev()
        .take(20)
        .map(|s| format!("{} {} [{}]", s.at, s.url, s.kind))
        .collect::<Vec<_>>();
    Ok(format!(
        "RECORDED SOURCE METADATA (not a verification of accuracy):\n{}",
        sources.join("\n")
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strips_query_and_fragment_from_persisted_source() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = Workspace::new(dir.path()).unwrap();
        record_fetch(&workspace, "https://example.com/read?token=private#section").unwrap();
        let text = list_sources(&workspace).unwrap();
        assert!(text.contains("https://example.com/read"));
        assert!(!text.contains("token"));
        assert!(!text.contains("private"));
        assert!(!text.contains("#section"));
    }
}
