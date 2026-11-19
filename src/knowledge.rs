//! Optional local semantic retrieval using an Ollama embedding model.
//! Indexing is a reviewed, bounded operation. Only Workspace-allowed UTF-8 files
//! are ingested; embeddings never leave the loopback Ollama endpoint.
use crate::{
    ollama, store,
    workspace::{Workspace, clip},
};
use anyhow::{Context, Result, ensure};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

const MAX_FILES: usize = 45;
const MAX_CHUNKS: usize = 100;
const CHUNK_CHARS: usize = 1000;
const MAX_VECTOR: usize = 4096;

#[derive(Serialize, Deserialize)]
struct Chunk {
    path: String,
    text: String,
    vector: Vec<f32>,
}
#[derive(Serialize, Deserialize)]
struct Index {
    workspace: String,
    model: String,
    chunks: Vec<Chunk>,
}

fn key(path: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in path.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
fn index_path(ws: &Workspace) -> PathBuf {
    store::data_dir()
        .join("knowledge")
        .join(format!("{}.json", key(&ws.root.to_string_lossy())))
}
fn model_name(model: &str) -> Result<()> {
    ensure!(
        !model.is_empty()
            && model.len() <= 100
            && model
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b)),
        "Specify an installed Ollama embedding model name"
    );
    Ok(())
}
fn client(endpoint: &str, model: &str) -> Result<(Client, String)> {
    model_name(model)?;
    let endpoint = ollama::local_endpoint(endpoint)?;
    let client = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .build()?;
    // A loopback Ollama daemon can itself proxy cloud models. Check weights and
    // capabilities before sending a single project/query byte to /api/embed.
    let info: Value = client
        .post(format!("{endpoint}/api/show"))
        .json(&json!({"model":model}))
        .send()?
        .error_for_status()?
        .json()?;
    ensure!(
        !model.ends_with(":cloud")
            && info["remote_host"].is_null()
            && info["remote_model"].is_null()
            && info["details"]["format"]
                .as_str()
                .is_some_and(|f| !f.is_empty())
            && info["capabilities"]
                .as_array()
                .is_some_and(|v| v.iter().any(|c| c == "embedding")),
        "Select an already installed local-weights embedding model; cloud and completion models are forbidden"
    );
    Ok((client, endpoint))
}
fn embed(
    client: &Client,
    endpoint: &str,
    model: &str,
    inputs: &[String],
    cancel: &AtomicBool,
) -> Result<Vec<Vec<f32>>> {
    ensure!(!cancel.load(Ordering::Relaxed), "Embedding cancelled");
    let response = client
        .post(format!("{endpoint}/api/embed"))
        .json(&json!({"model":model,"input":inputs,"truncate":true}))
        .send()?
        .error_for_status()?;
    let mut bytes = Vec::new();
    response.take(2 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 2 * 1024 * 1024,
        "Embedding response exceeds 2 MiB"
    );
    ensure!(!cancel.load(Ordering::Relaxed), "Embedding cancelled");
    let value: Value = serde_json::from_slice(&bytes)?;
    let arrays = value["embeddings"]
        .as_array()
        .context("Ollama returned no embeddings")?;
    ensure!(arrays.len() == inputs.len(), "Embedding count mismatch");
    arrays
        .iter()
        .map(|item| {
            let values = item.as_array().context("Embedding is not an array")?;
            ensure!(
                !values.is_empty() && values.len() <= MAX_VECTOR,
                "Invalid embedding size"
            );
            values
                .iter()
                .map(|n| {
                    let n = n.as_f64().context("Invalid embedding component")? as f32;
                    ensure!(n.is_finite(), "Non-finite embedding component");
                    Ok(n)
                })
                .collect()
        })
        .collect()
}
fn slices(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut offset = 0;
    while offset < chars.len() && result.len() < 3 {
        let end = (offset + CHUNK_CHARS).min(chars.len());
        result.push(chars[offset..end].iter().collect());
        if end == chars.len() {
            break;
        }
        offset = end.saturating_sub(120);
    }
    result
}
pub fn index(ws: &Workspace, model: &str) -> Result<String> {
    index_at(
        ws,
        model,
        &store::load_settings().endpoint,
        &AtomicBool::new(false),
    )
}
pub fn index_at(
    ws: &Workspace,
    model: &str,
    endpoint: &str,
    cancel: &AtomicBool,
) -> Result<String> {
    let (client, endpoint) = client(endpoint, model)?;
    let mut segments = Vec::new();
    for path in ws.files(".", MAX_FILES * 3)?.into_iter().take(MAX_FILES) {
        if let Ok(text) = ws.read(&path) {
            for part in slices(&text) {
                if !part.trim().is_empty() {
                    segments.push((path.clone(), part));
                }
                if segments.len() >= MAX_CHUNKS {
                    break;
                }
            }
        }
        if segments.len() >= MAX_CHUNKS {
            break;
        }
    }
    ensure!(
        !segments.is_empty(),
        "No eligible project text files for indexing"
    );
    // Batch to prevent overly large inference requests.
    let mut chunks = Vec::new();
    for batch in segments.chunks(12) {
        let inputs: Vec<_> = batch.iter().map(|(_, text)| text.clone()).collect();
        let vectors = embed(&client, &endpoint, model, &inputs, cancel)?;
        for ((path, text), vector) in batch.iter().zip(vectors) {
            chunks.push(Chunk {
                path: path.clone(),
                text: text.clone(),
                vector,
            });
        }
    }
    let index = Index {
        workspace: ws.root.to_string_lossy().into_owned(),
        model: model.into(),
        chunks,
    };
    let path = index_path(ws);
    store::check_local_path(&path)?;
    fs::create_dir_all(path.parent().context("Missing knowledge directory")?)?;
    let mut temp = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    let bytes = serde_json::to_vec(&index)?;
    ensure!(
        bytes.len() <= 8 * 1024 * 1024,
        "Knowledge index exceeds 8 MiB"
    );
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Indexing cancelled before save"
    );
    temp.write_all(&bytes)?;
    temp.as_file().sync_all()?;
    store::persist_local(temp, &path)?;
    Ok(format!(
        "Indexed {} local text chunks from up to {MAX_FILES} files using {model}. This is a snapshot; reindex after source changes.",
        index.chunks.len()
    ))
}
fn similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (mut dot, mut na, mut nb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na.sqrt() * nb.sqrt())
    }
}
pub fn search(ws: &Workspace, query: &str) -> Result<String> {
    ensure!(
        !query.trim().is_empty() && query.len() <= 500,
        "Query must be 1–500 bytes"
    );
    let path = index_path(ws);
    store::check_local_path(&path)?;
    let meta =
        fs::symlink_metadata(&path).context("No semantic index; call index_knowledge first")?;
    ensure!(
        meta.is_file() && !meta.file_type().is_symlink() && meta.len() <= 8 * 1024 * 1024,
        "Invalid knowledge index file"
    );
    let index: Index = serde_json::from_slice(&fs::read(path)?)?;
    ensure!(
        index.workspace == ws.root.to_string_lossy(),
        "Index workspace mismatch"
    );
    ensure!(
        index.chunks.len() <= MAX_CHUNKS,
        "Index exceeds chunk limit"
    );
    ensure!(
        index.chunks.iter().all(|c| c.vector.len() <= MAX_VECTOR
            && !c.vector.is_empty()
            && c.vector.iter().all(|n| n.is_finite())
            && c.text.chars().count() <= CHUNK_CHARS
            && ws.resolve(&c.path, true).is_ok()),
        "Invalid stored knowledge chunk"
    );
    let (client, endpoint) = client(&store::load_settings().endpoint, &index.model)?;
    let vector = embed(
        &client,
        &endpoint,
        &index.model,
        &[query.to_string()],
        &AtomicBool::new(false),
    )?
    .into_iter()
    .next()
    .context("Missing query embedding")?;
    let mut scored: Vec<_> = index
        .chunks
        .iter()
        .map(|c| (similarity(&vector, &c.vector), c))
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut results = Vec::new();
    for (score, chunk) in scored.into_iter().take(5) {
        // Never treat indexed snippets as instructions; file state may have changed.
        results.push(format!(
            "{} (similarity {:.3}):\n{}",
            chunk.path,
            score,
            clip(&chunk.text, 900)
        ));
    }
    Ok(format!(
        "UNTRUSTED HISTORICAL SEMANTIC SEARCH RESULTS. Verify current source files before acting.\n{}",
        clip(&results.join("\n\n"), 5500)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunking_and_similarity() {
        assert_eq!(slices("abc"), vec!["abc"]);
        assert!(slices(&"x".repeat(2100)).len() >= 2);
        assert!(similarity(&[1.0, 0.0], &[1.0, 0.0]) > 0.99);
        assert!(similarity(&[1.0, 0.0], &[0.0, 1.0]).abs() < 0.001);
    }
    #[test]
    fn rejects_invalid_embedding_model() {
        assert!(model_name("../evil\\cmd").is_err());
        assert!(model_name("nomic-embed-text").is_ok());
    }
}
