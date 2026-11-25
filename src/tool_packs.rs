//! Reviewed fixed Python runtime. The anonymous pipe is host-only; no model can
//! issue approvals, select a program, retrieve raw bytes or change private roots.
use crate::{store, types::Settings, workspace::Workspace};
use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const WIRE: usize = 2_000_000;
macro_rules! sources { ($($name:literal),* $(,)?) => { &[$(($name, include_bytes!(concat!("../tool-packs/",$name)) as &[u8])),*] }; }
const SOURCES: &[(&str, &[u8])] = sources![
    "session_worker.py",
    "host_adapter.py",
    "document_cover.py",
    "host-catalog.json",
    "dispatch.py",
    "execution.py",
    "process_limits.py",
    "leaf_limits.py",
    "workspace_adapter.py",
    "artifact_store.py",
    "artifact_host.py",
    "workflow.py",
    "workflow_templates.py",
    "discovery.py",
    "policy.py",
    "integrity.py",
    "system_adapter.py",
    "research_adapter.py",
    "evidence.py",
    "knowledge.py",
    "foundation/manifest.json",
    "foundation/runner.py",
    "analysis/manifest.json",
    "analysis/runner.py",
    "documents/manifest.json",
    "documents/runner.py",
    "repository/manifest.json",
    "repository/runner.py",
    "office/manifest.json",
    "office/runner.py",
    "pdf/manifest.json",
    "pdf/runner.py",
    "quality/manifest.json",
    "quality/runner.py",
    "visual/manifest.json",
    "visual/runner.py"
];
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn covered_document_publication_rejects_overwrite_traversal_and_non_docx() {
        let directory = tempfile::tempdir().unwrap();
        let ws = Workspace::new(directory.path()).unwrap();
        fs::write(directory.path().join("CV.docx"), b"fixture").unwrap();
        assert!(
            cover_destination(
                &ws,
                &json!({"path":"CV.docx","output":"CV_With_Cover.docx"})
            )
            .is_ok()
        );
        for output in [
            "CV.docx",
            "../escape.docx",
            "missing/out.docx",
            "payload.ps1",
        ] {
            assert!(
                cover_destination(&ws, &json!({"path":"CV.docx","output":output})).is_err(),
                "{output}"
            );
        }
    }
    #[test]
    fn registered_aliases_are_unique_and_host_transport_is_not_a_capability() {
        let entries = catalog();
        assert_eq!(entries.len(), 58);
        let aliases: std::collections::BTreeSet<_> =
            entries.keys().map(|name| alias(name)).collect();
        assert_eq!(aliases.len(), entries.len());
        for name in entries.keys() {
            assert_eq!(canonical(&alias(name)).as_ref(), Some(name));
        }
        for name in [
            "approve",
            "export",
            "init",
            "receive_artifact",
            "pack__approve",
        ] {
            assert!(canonical(name).is_none());
            assert!(!entries.contains_key(name));
        }
    }
    #[test]
    fn strict_schemas_reject_unknown_fields_binary_arguments_and_wrong_types() {
        let entries = catalog();
        let schema = &entries["pdf.render"]["input_schema"];
        let good = json!({"artifact_id":"a".repeat(32),"page":1,"dpi":72});
        assert!(validate(&good, schema, 0).is_ok());
        for bad in [
            json!({"artifact_id":"../escape","page":1,"dpi":72}),
            json!({"artifact_id":"a".repeat(32),"page":0,"dpi":72}),
            json!({"artifact_id":"a".repeat(32),"page":true,"dpi":72}),
            json!({"artifact_id":"a".repeat(32),"page":1,"dpi":72,"content":"base64"}),
        ] {
            assert!(validate(&bad, schema, 0).is_err());
        }
        assert!(
            validate(
                &json!("x"),
                &json!({"type":"string","pattern":"arbitrary"}),
                0
            )
            .is_err()
        );
        assert!(validate(&json!({}), &json!({"type":"unreviewed"}), 0).is_err());
    }
    #[test]
    fn tool_packs_require_review_and_internet_remains_independent() {
        for name in [
            "pack__math_calculate",
            "pack__office_docx",
            "tool_pack_call",
        ] {
            assert!(crate::policy::authorize(name, true, false).is_err());
            assert!(crate::policy::authorize(name, false, false).is_ok());
        }
        assert!(crate::policy::authorize("pack__web_fetch", false, false).is_err());
        assert!(is_network("tool_pack_call", &json!({"tool":"web.fetch"})));
        assert!(!valid_collection("../private"));
    }
}
fn code_digest() -> String {
    let mut hash = Sha256::new();
    for (name, bytes) in SOURCES {
        hash.update(name.as_bytes());
        hash.update([0]);
        hash.update(bytes);
    }
    hash.update(include_bytes!("../scripts/design_runner.py"));
    format!("{:x}", hash.finalize())
}
fn config_path() -> PathBuf {
    store::data_dir().join("tool-pack-runtime.json")
}
fn cache_root() -> PathBuf {
    store::data_dir().join("tool-runtime").join(code_digest())
}
pub fn catalog() -> BTreeMap<String, Value> {
    serde_json::from_str(include_str!("../tool-packs/host-catalog.json"))
        .expect("compiled reviewed catalog")
}
pub fn alias(name: &str) -> String {
    format!("pack__{}", name.replace('.', "_"))
}
pub fn canonical(alias_name: &str) -> Option<String> {
    catalog()
        .keys()
        .find(|name| alias(name) == alias_name)
        .cloned()
}
pub fn is_tool(name: &str) -> bool {
    name == "tool_pack_call" || canonical(name).is_some()
}
pub fn is_network(name: &str, args: &Value) -> bool {
    canonical(name).as_deref() == Some("web.fetch")
        || (name == "tool_pack_call" && args["tool"] == "web.fetch")
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub python: String,
    pub python_sha256: String,
    pub code_sha256: String,
    pub soffice: Option<String>,
    pub soffice_sha256: Option<String>,
    pub discovery_bytes: usize,
    pub discovery_limit: usize,
}
fn file_digest(path: &Path, limit: u64) -> Result<String> {
    store::check_local_path(path)?;
    let meta = fs::metadata(path)?;
    ensure!(
        meta.is_file() && meta.len() <= limit,
        "Runtime file is unavailable or oversized"
    );
    let mut data = Vec::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut data)?;
    ensure!(data.len() as u64 <= limit, "Runtime file grew");
    Ok(digest(&data))
}
pub fn configured() -> bool {
    config_path().is_file()
}
pub fn set_discovery_limits(limit: usize, budget: usize) -> Result<()> {
    ensure!(
        (1..=20).contains(&limit) && (2..=32000).contains(&budget),
        "Discovery limits outside reviewed bounds"
    );
    let mut configuration = config()?;
    configuration.discovery_limit = limit;
    configuration.discovery_bytes = budget;
    store::atomic_json(config_path(), &configuration)?;
    reset_workers();
    Ok(())
}
pub fn register_generated(
    workspace: &Workspace,
    path: &str,
    expected_sha256: &str,
) -> Result<Value> {
    let source = workspace.resolve(path, false)?;
    let extension = source
        .extension()
        .and_then(|e| e.to_str())
        .context("No artifact extension")?
        .to_lowercase();
    ensure!(
        ["docx", "xlsx", "pptx", "pdf", "png", "jpg"].contains(&extension.as_str()),
        "Format not supported by private artifact storage"
    );
    let mut bytes = Vec::new();
    fs::File::open(source)?
        .take(700001)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 700000 && digest(&bytes) == expected_sha256,
        "Generated artifact exceeded storage limit or changed before registration"
    );
    receive_artifact(workspace, &extension, &STANDARD.encode(&bytes))
}
pub fn config() -> Result<Config> {
    store::check_local_path(&config_path())?;
    ensure!(
        fs::metadata(config_path())?.len() < 16000,
        "Invalid runtime configuration"
    );
    let config: Config = serde_json::from_slice(&fs::read(config_path())?)?;
    ensure!(
        config.code_sha256 == code_digest(),
        "Tool runtime version changed; review and configure the current build"
    );
    ensure!(
        (2..=32000).contains(&config.discovery_bytes) && (1..=20).contains(&config.discovery_limit),
        "Invalid discovery limits"
    );
    ensure!(
        Path::new(&config.python).is_absolute()
            && file_digest(Path::new(&config.python), 128 * 1024 * 1024)? == config.python_sha256,
        "Python executable changed; reconfigure after review"
    );
    if let Some(path) = &config.soffice {
        ensure!(
            Path::new(path).is_absolute()
                && Some(file_digest(Path::new(path), 128 * 1024 * 1024)?) == config.soffice_sha256,
            "LibreOffice executable changed after configuration"
        );
    }
    Ok(config)
}
pub fn configure(python: &str, soffice: Option<&str>) -> Result<Config> {
    ensure!(
        Path::new(python).is_absolute(),
        "Use an absolute interpreter path"
    );
    let python_sha256 = file_digest(Path::new(python), 128 * 1024 * 1024)?;
    let root = cache_root();
    store::check_local_path(&root)?;
    fs::create_dir_all(&root)?;
    for (name, bytes) in SOURCES.iter().copied().chain(std::iter::once((
        "design_runner.py",
        include_bytes!("../scripts/design_runner.py") as &[u8],
    ))) {
        let path = root.join(name);
        store::check_local_path(&path)?;
        fs::create_dir_all(path.parent().context("No parent")?)?;
        if path.exists() && file_digest(&path, 2_000_000)? != digest(bytes) {
            fs::rename(
                &path,
                path.with_extension(format!("quarantine-{}", uuid::Uuid::new_v4())),
            )?;
        }
        if !path.exists() {
            let mut file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)?;
            file.write_all(bytes)?;
            file.sync_all()?;
        }
    }
    let config = Config {
        python: python.into(),
        python_sha256,
        code_sha256: code_digest(),
        soffice: soffice.map(str::to_string),
        soffice_sha256: soffice
            .map(|p| file_digest(Path::new(p), 128 * 1024 * 1024))
            .transpose()?,
        discovery_bytes: 6000,
        discovery_limit: 6,
    };
    store::atomic_json(config_path(), &config)?;
    reset_workers();
    Ok(config)
}
fn verify_code() -> Result<()> {
    let root = cache_root();
    for (name, bytes) in SOURCES.iter().copied().chain(std::iter::once((
        "design_runner.py",
        include_bytes!("../scripts/design_runner.py") as &[u8],
    ))) {
        ensure!(
            file_digest(&root.join(name), 2_000_000)? == digest(bytes),
            "Reviewed tool runtime fingerprint changed"
        );
    }
    Ok(())
}
fn state_root(workspace: &Workspace) -> PathBuf {
    store::data_dir()
        .join("tool-sessions")
        .join(digest(workspace.root.to_string_lossy().as_bytes()))
}
pub fn collection(workspace: &Workspace) -> String {
    fs::read_to_string(state_root(workspace).join("active-collection.txt"))
        .ok()
        .filter(|name| valid_collection(name))
        .unwrap_or_else(|| "default".into())
}
fn valid_collection(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 40
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
pub fn select_collection(workspace: &Workspace, name: &str) -> Result<()> {
    ensure!(
        valid_collection(name),
        "Collection names use 1-40 letters, digits, underscore or hyphen"
    );
    let path = state_root(workspace);
    store::check_local_path(&path)?;
    fs::create_dir_all(&path)?;
    store::check_local_path(&path.join("active-collection.txt"))?;
    fs::write(path.join("active-collection.txt"), name)?;
    reset_workers();
    Ok(())
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub tool: String,
    pub arguments: Value,
    pub previous: Option<String>,
    pub config: Config,
    pub workspace: PathBuf,
    pub input_path: Option<String>,
    pub input_sha256: Option<String>,
    pub artifact_hashes: BTreeMap<String, String>,
    pub collection: String,
    pub design_fingerprints: BTreeMap<String, String>,
}
pub fn validate(value: &Value, schema: &Value, depth: usize) -> Result<()> {
    ensure!(depth <= 20, "Input nesting exceeds limit");
    match schema["type"].as_str().context("Unsupported schema")? {
        "object" => {
            let object = value.as_object().context("Expected object")?;
            let props = schema["properties"].as_object().context("Invalid schema")?;
            ensure!(
                object.keys().all(|k| props.contains_key(k)),
                "Unknown input field; use the discovered schema"
            );
            if let Some(required) = schema["required"].as_array() {
                ensure!(
                    required
                        .iter()
                        .all(|k| k.as_str().is_some_and(|k| object.contains_key(k))),
                    "Required field missing"
                );
            }
            for (k, v) in object {
                validate(v, &props[k], depth + 1)?;
            }
        }
        "array" => {
            let items = value.as_array().context("Expected array")?;
            ensure!(
                items.len() >= schema["minItems"].as_u64().unwrap_or(0) as usize
                    && items.len() <= schema["maxItems"].as_u64().unwrap_or(10000) as usize,
                "Array length exceeds schema"
            );
            for item in items {
                validate(item, &schema["items"], depth + 1)?;
            }
        }
        "string" => {
            let text = value.as_str().context("Expected string")?;
            ensure!(
                text.chars().count() >= schema["minLength"].as_u64().unwrap_or(0) as usize
                    && text.chars().count()
                        <= schema["maxLength"].as_u64().unwrap_or(100000) as usize,
                "String length exceeds schema"
            );
            if let Some(pattern) = schema["pattern"].as_str() {
                let valid = match pattern {
                    "^[0-9a-f]{32}$" => {
                        text.len() == 32
                            && text
                                .bytes()
                                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    }
                    "^#[0-9a-fA-F]{6}$" => {
                        text.len() == 7
                            && text.starts_with('#')
                            && text.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
                    }
                    _ => false,
                };
                ensure!(valid, "Input does not match the reviewed pattern");
            }
        }
        "integer" => {
            let number = value.as_i64().context("Expected integer")?;
            ensure!(
                number >= schema["minimum"].as_i64().unwrap_or(-1000000)
                    && number <= schema["maximum"].as_i64().unwrap_or(1000000),
                "Integer exceeds schema bounds"
            );
        }
        "number" => {
            ensure!(
                value.as_f64().is_some_and(f64::is_finite),
                "Expected finite number"
            );
        }
        "scalar" => {
            ensure!(
                value.is_null()
                    || value.is_boolean()
                    || value.is_number()
                    || value.as_str().is_some_and(|s| s.chars().count() <= 500),
                "Expected bounded scalar"
            );
        }
        _ => bail!("Unsupported reviewed schema type"),
    }
    if let Some(options) = schema["enum"].as_array() {
        ensure!(options.contains(value), "Invalid enum choice");
    }
    Ok(())
}
pub fn prepare(
    workspace: &Workspace,
    tool_name: &str,
    args: &Value,
    internet_enabled: bool,
) -> Result<Plan> {
    let (tool, mut arguments, mut previous) = if tool_name == "tool_pack_call" {
        let object = args.as_object().context("Expected invocation object")?;
        ensure!(
            object
                .keys()
                .all(|k| ["tool", "arguments", "previous"].contains(&k.as_str())),
            "Unknown invocation field"
        );
        (
            args["tool"]
                .as_str()
                .context("Choose a discovered registered tool")?
                .to_string(),
            args["arguments"].clone(),
            args["previous"].as_str().map(str::to_string),
        )
    } else {
        (
            canonical(tool_name).context("Unregistered tool-pack alias")?,
            args.clone(),
            None,
        )
    };
    if let Some(object) = arguments.as_object_mut()
        && let Some(value) = object.remove("previous_artifact_id")
    {
        ensure!(previous.is_none(), "Duplicate revision argument");
        previous = Some(value.as_str().context("Expected revision ID")?.into());
    }
    let entries = catalog();
    let spec = entries
        .get(&tool)
        .context("Unregistered tool pack capability")?;
    validate(&arguments, &spec["input_schema"], 0)?;
    ensure!(
        serde_json::to_vec(&arguments)?.len() <= 1_000_000,
        "Tool input exceeds limit"
    );
    ensure!(
        tool != "web.fetch" || internet_enabled,
        "Internet access is disabled"
    );
    if tool == "web.fetch" {
        let url = arguments["url"].as_str().context("Expected URL")?;
        crate::tools::validate_url(url)?;
    }
    let config = config()?;
    verify_code()?;
    let mut design_fingerprints = BTreeMap::new();
    if tool == "document.add_cover" {
        let registry = crate::portable_skills::Registry::load()?;
        for id in ["professional-document-design-v2", "colour-intelligence-v2"] {
            let (_, fingerprint) = registry.adapter_script(id, "SKILL.md")?;
            design_fingerprints.insert(id.into(), fingerprint);
        }
        cover_destination(workspace, &arguments)?;
        ensure!(
            config.soffice.is_some(),
            "Configure LibreOffice before adding a verified cover page"
        );
    }
    let input_path: Option<String> = if matches!(
        tool.as_str(),
        "artifact.import" | "knowledge.ingest" | "document.add_cover"
    ) {
        Some(
            arguments["path"]
                .as_str()
                .context("Expected workspace path")?
                .into(),
        )
    } else {
        None
    };
    let input_sha256 = if let Some(path) = &input_path {
        let path = workspace.resolve(path, false)?;
        let limit = if tool == "knowledge.ingest" {
            400000
        } else {
            700000
        };
        Some(file_digest(&path, limit)?)
    } else {
        None
    };
    if tool.starts_with("workspace.") {
        workspace.resolve(
            arguments["path"]
                .as_str()
                .context("Expected workspace path")?,
            false,
        )?;
    }
    let mut ids = Vec::new();
    for key in ["artifact_id", "first_id", "second_id"] {
        if let Some(id) = arguments[key].as_str() {
            ids.push(id.to_string());
        }
    }
    if let Some(values) = arguments["artifact_ids"].as_array() {
        ids.extend(values.iter().filter_map(Value::as_str).map(str::to_string));
    }
    if let Some(id) = &previous {
        ids.push(id.clone());
    }
    let mut artifact_hashes = BTreeMap::new();
    for id in ids {
        let record = artifact_metadata(workspace, &id)?;
        artifact_hashes.insert(
            id,
            record["sha256"]
                .as_str()
                .context("Invalid artifact digest")?
                .into(),
        );
    }
    Ok(Plan {
        tool,
        arguments,
        previous,
        config,
        workspace: workspace.root.clone(),
        input_path,
        input_sha256,
        artifact_hashes,
        collection: collection(workspace),
        design_fingerprints,
    })
}
pub fn mutates(plan: &Plan) -> bool {
    matches!(
        plan.tool.as_str(),
        "office.docx"
            | "office.xlsx"
            | "office.pptx"
            | "pdf.generate"
            | "pdf.merge"
            | "pdf.split"
            | "pdf.render"
            | "office.preview"
            | "artifact.import"
            | "knowledge.ingest"
            | "workflow.run"
            | "document.add_cover"
    )
}
pub fn review(plan: &Plan) -> String {
    format!(
        "Reviewed local capability: {}\nArguments:\n{}\nPrevious revision: {:?}\nInput SHA-256: {:?}\nReferenced artifact digests: {:?}\nPython: {}\nCode fingerprint: {}\nKnowledge collection: {}\nDesign skills (reviewed fingerprints): {:?}\nA one-use 60-second capability token is issued only after this review. Binary artifacts remain private. This runtime is not an OS filesystem/network sandbox.",
        plan.tool,
        serde_json::to_string_pretty(&plan.arguments).unwrap_or_default(),
        plan.previous,
        plan.input_sha256,
        plan.artifact_hashes,
        plan.config.python,
        plan.config.code_sha256,
        plan.collection,
        plan.design_fingerprints
    )
}

pub fn execute(workspace: &Workspace, plan: &Plan, cancel: &Arc<AtomicBool>) -> Result<String> {
    ensure!(
        workspace.root == plan.workspace
            && config()? == plan.config
            && collection(workspace) == plan.collection,
        "Reviewed runtime/workspace/collection changed"
    );
    verify_code()?;
    let design = if plan.tool == "document.add_cover" {
        let registry = crate::portable_skills::Registry::load()?;
        for (id, expected) in &plan.design_fingerprints {
            ensure!(
                registry.adapter_script(id, "SKILL.md")?.1 == *expected,
                "Design skill changed after cover review"
            );
        }
        let (colour_script, colour_fp) =
            registry.adapter_script("colour-intelligence-v2", "scripts/colour.py")?;
        let (standard, document_fp) = registry.adapter_script(
            "professional-document-design-v2",
            "references/design-standard.md",
        )?;
        ensure!(
            plan.design_fingerprints.get("colour-intelligence-v2") == Some(&colour_fp)
                && plan
                    .design_fingerprints
                    .get("professional-document-design-v2")
                    == Some(&document_fp),
            "Cover design resources changed after review"
        );
        Some(
            json!({"colour_script":colour_script,"professional_standard":standard,"fingerprints":plan.design_fingerprints}),
        )
    } else {
        None
    };
    if plan.tool == "document.add_cover" {
        cover_destination(workspace, &plan.arguments)?;
    }
    for (id, expected) in &plan.artifact_hashes {
        ensure!(
            artifact_metadata(workspace, id)?["sha256"].as_str() == Some(expected),
            "Reviewed artifact metadata changed"
        );
    }
    let attachment = if let Some(path) = &plan.input_path {
        let path = workspace.resolve(path, false)?;
        let mut data = Vec::new();
        fs::File::open(path)?
            .take(1_048_577)
            .read_to_end(&mut data)?;
        ensure!(
            data.len()
                <= if plan.tool == "knowledge.ingest" {
                    400000
                } else {
                    700000
                },
            "Reviewed input grew beyond its bound"
        );
        ensure!(
            Some(digest(&data)) == plan.input_sha256,
            "Input changed after review"
        );
        Some(STANDARD.encode(data))
    } else {
        None
    };
    let approval = rpc(
        workspace,
        json!({"action":"approve","tool":plan.tool,"arguments":plan.arguments,"previous":plan.previous,"attachment":attachment,"design":design}),
        cancel,
    )?;
    let mut result = rpc(
        workspace,
        json!({"action":"call","tool":plan.tool,"arguments":plan.arguments,"previous":plan.previous,"approval":approval["approval"]}),
        cancel,
    )?;
    ensure!(
        result["ok"] == true,
        "Tool rejected input or failed; no success may be claimed"
    );
    if plan.tool == "document.add_cover" {
        ensure!(
            result["result"]["preservation"]["source_sha256"].as_str()
                == plan.input_sha256.as_deref()
                && result["result"]["preservation"]["original_body_preserved"] == true
                && result["result"]["preservation"]["unchanged_package_parts"] == true,
            "CV preservation verification failed"
        );
        let artifact = &result["result"]["artifact"];
        let id = artifact["artifact_id"]
            .as_str()
            .context("Missing covered CV artifact")?;
        let value = rpc(
            workspace,
            json!({"action":"export","artifact_id":id}),
            cancel,
        )?;
        let bytes = STANDARD.decode(
            value["content"]
                .as_str()
                .context("Missing covered CV bytes")?,
        )?;
        ensure!(
            bytes.len() <= 700000
                && value["metadata"]["extension"] == "docx"
                && value["metadata"]["sha256"] == digest(&bytes)
                && artifact["sha256"] == digest(&bytes),
            "Covered CV integrity failure"
        );
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "Cancelled before saving covered CV"
        );
        let destination = cover_destination(workspace, &plan.arguments)?;
        let mut file = tempfile::NamedTempFile::new_in(
            destination.parent().context("Missing output parent")?,
        )?;
        file.write_all(&bytes)?;
        file.as_file().sync_all()?;
        cover_destination(workspace, &plan.arguments)?;
        file.persist_noclobber(&destination)?;
        ensure!(
            file_digest(&destination, 700000)? == digest(&bytes),
            "Covered CV read-back verification failed"
        );
        result["result"]["created"] = plan.arguments["output"].clone();
        result["result"]["sha256"] = json!(digest(&bytes));
        result["result"]["saved_to_workspace"] = json!(true);
    }
    Ok(serde_json::to_string_pretty(&result)?)
}

fn cover_destination(workspace: &Workspace, args: &Value) -> Result<PathBuf> {
    let input = args["path"]
        .as_str()
        .context("Existing DOCX path required")?;
    let output = args["output"]
        .as_str()
        .context("New DOCX output required")?;
    ensure!(
        Path::new(input)
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("docx"))
            && Path::new(output)
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("docx")),
        "Cover input and output must be DOCX files"
    );
    workspace.resolve(input, false)?;
    let destination = workspace.resolve(output, true)?;
    ensure!(
        !destination.exists(),
        "Cover output already exists; choose a new filename"
    );
    ensure!(
        destination.parent().is_some_and(Path::is_dir),
        "Cover output parent must already exist"
    );
    Ok(destination)
}

#[cfg(windows)]
struct Tree(usize);
#[cfg(windows)]
impl Tree {
    fn attach(child: &Child) -> Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            ensure!(!job.is_null(), "Cannot create worker Job Object");
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_JOB_MEMORY;
            limits.JobMemoryLimit = 2 * 1024 * 1024 * 1024;
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            ) == 0
                || AssignProcessToJobObject(job, child.as_raw_handle()) == 0
            {
                CloseHandle(job);
                bail!("Cannot contain worker process tree");
            }
            Ok(Self(job as usize))
        }
    }
    fn stop(&mut self) {
        if self.0 != 0 {
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(self.0 as _);
            }
            self.0 = 0;
        }
    }
}
#[cfg(not(windows))]
struct Tree(u32);
#[cfg(not(windows))]
impl Tree {
    fn attach(child: &Child) -> Result<Self> {
        Ok(Self(child.id()))
    }
    fn stop(&mut self) {
        unsafe {
            libc::kill(-(self.0 as i32), libc::SIGTERM);
        }
        std::thread::sleep(Duration::from_millis(300));
        unsafe {
            libc::kill(-(self.0 as i32), libc::SIGKILL);
        }
    }
}

struct Worker {
    child: Child,
    input: Arc<Mutex<ChildStdin>>,
    output: mpsc::Receiver<Result<Value>>,
    tree: Tree,
    configuration: Config,
    collection: String,
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.tree.stop();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Worker {
    fn start(workspace: &Workspace) -> Result<Self> {
        let configuration = config()?;
        verify_code()?;
        let root = state_root(workspace);
        store::check_local_path(&root)?;
        fs::create_dir_all(&root)?;
        let mut command = Command::new(&configuration.python);
        command
            .args(["-I", "-B"])
            .arg(cache_root().join("session_worker.py"))
            .current_dir(cache_root())
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for name in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command
            .spawn()
            .context("Cannot start reviewed Python worker")?;
        let tree = match Tree::attach(&child) {
            Ok(tree) => tree,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        let input = child.stdin.take().context("Worker input unavailable")?;
        let stdout = child.stdout.take().context("Worker output unavailable")?;
        let (tx, output) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = Vec::new();
                let result = (&mut reader)
                    .take(WIRE as u64 + 1)
                    .read_until(b'\n', &mut line);
                let parsed = (|| {
                    ensure!(
                        result? > 0 && line.len() <= WIRE && line.ends_with(b"\n"),
                        "Worker output closed or exceeded limit"
                    );
                    serde_json::from_slice::<Value>(&line).context("Malformed worker protocol")
                })();
                let stop = parsed.is_err();
                if tx.send(parsed).is_err() || stop {
                    break;
                }
            }
        });
        let mut worker = Self {
            child,
            input: Arc::new(Mutex::new(input)),
            output,
            tree,
            configuration,
            collection: collection(workspace),
        };
        worker.request(json!({"action":"init","workspace":workspace.root,"state_directory":root,"collection":worker.collection,"allowed":catalog().keys().collect::<Vec<_>>(),"soffice":worker.configuration.soffice}),&Arc::new(AtomicBool::new(false)))?;
        Ok(worker)
    }
    fn request(&mut self, mut request: Value, cancel: &Arc<AtomicBool>) -> Result<Value> {
        ensure!(!cancel.load(Ordering::Relaxed), "Tool call cancelled");
        let id = uuid::Uuid::new_v4().to_string();
        request["id"] = json!(id);
        let mut data = serde_json::to_vec(&request)?;
        ensure!(data.len() < WIRE, "Host request exceeds transport limit");
        data.push(b'\n');
        // Pipe writes can block too. Keep both writing and reading behind the
        // same cancellation/deadline loop; dropping the owned tree unblocks IO.
        let input = Arc::clone(&self.input);
        let (write_tx, write_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = (|| -> Result<()> {
                let mut input = input
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Worker writer unavailable"))?;
                input.write_all(&data)?;
                input.flush()?;
                Ok(())
            })();
            let _ = write_tx.send(result);
        });
        let started = Instant::now();
        let mut written = false;
        loop {
            ensure!(!cancel.load(Ordering::Relaxed), "Tool operation cancelled");
            ensure!(
                started.elapsed() < Duration::from_secs(120),
                "Tool runtime deadline exceeded"
            );
            if !written {
                match write_rx.try_recv() {
                    Ok(result) => {
                        result?;
                        written = true;
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                    Err(_) => bail!("Worker writer disconnected"),
                }
            }
            match self.output.recv_timeout(Duration::from_millis(30)) {
                Ok(response) => {
                    let response = response?;
                    ensure!(response["id"] == id, "Worker response identity mismatch");
                    ensure!(
                        response["ok"] == true,
                        "Tool runtime rejected request ({}). Check the exact schema, dependencies, input limits and review state.",
                        response["error"]["code"].as_str().unwrap_or("transport")
                    );
                    return Ok(response["result"].clone());
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => bail!("Tool worker exited"),
            }
        }
    }
}
static WORKERS: OnceLock<Mutex<BTreeMap<PathBuf, Worker>>> = OnceLock::new();
fn workers() -> &'static Mutex<BTreeMap<PathBuf, Worker>> {
    WORKERS.get_or_init(|| Mutex::new(BTreeMap::new()))
}
pub fn reset_workers() {
    if let Ok(mut workers) = workers().lock() {
        workers.clear();
    }
}
fn rpc(workspace: &Workspace, request: Value, cancel: &Arc<AtomicBool>) -> Result<Value> {
    let configuration = config()?;
    verify_code()?;
    let waiting = Instant::now();
    let mut workers = loop {
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "Cancelled while waiting for tool runtime"
        );
        ensure!(
            waiting.elapsed() < Duration::from_secs(120),
            "Tool runtime queue deadline exceeded"
        );
        match workers().try_lock() {
            Ok(guard) => break guard,
            Err(std::sync::TryLockError::WouldBlock) => {
                std::thread::sleep(Duration::from_millis(30))
            }
            Err(_) => bail!("Worker manager unavailable"),
        }
    };
    let stale = workers
        .get(&workspace.root)
        .is_some_and(|w| w.configuration != configuration || w.collection != collection(workspace));
    if stale {
        workers.remove(&workspace.root);
    }
    if !workers.contains_key(&workspace.root) {
        if workers.len() >= 2
            && let Some(key) = workers.keys().next().cloned()
        {
            workers.remove(&key);
        }
        workers.insert(workspace.root.clone(), Worker::start(workspace)?);
    }
    let result = workers
        .get_mut(&workspace.root)
        .context("Worker unavailable")?
        .request(request, cancel);
    if result.is_err() {
        workers.remove(&workspace.root);
    }
    result
}
pub fn discover(workspace: &Workspace, query: &str, internet_enabled: bool) -> Result<Value> {
    let configuration = config()?;
    ensure!(query.len() <= 4000, "Discovery query too long");
    rpc(
        workspace,
        json!({"action":"discover","query":query,"internet_enabled":internet_enabled,"budget_bytes":configuration.discovery_bytes,"limit":configuration.discovery_limit}),
        &Arc::new(AtomicBool::new(false)),
    )
}
pub fn status(workspace: &Workspace) -> Result<Value> {
    rpc(
        workspace,
        json!({"action":"status"}),
        &Arc::new(AtomicBool::new(false)),
    )
}
pub fn artifacts(workspace: &Workspace) -> Result<Value> {
    rpc(
        workspace,
        json!({"action":"artifacts"}),
        &Arc::new(AtomicBool::new(false)),
    )
}
fn artifact_metadata(workspace: &Workspace, id: &str) -> Result<Value> {
    validate(
        &json!(id),
        &json!({"type":"string","pattern":"^[0-9a-f]{32}$"}),
        0,
    )?;
    let records = artifacts(workspace)?;
    records["artifacts"]
        .as_array()
        .context("Invalid artifact inventory")?
        .iter()
        .find(|v| v["artifact_id"] == id)
        .cloned()
        .context("Unknown artifact in selected workspace")
}
pub fn artifact_bytes(workspace: &Workspace, id: &str) -> Result<(Value, Vec<u8>)> {
    artifact_metadata(workspace, id)?;
    let value = rpc(
        workspace,
        json!({"action":"export","artifact_id":id}),
        &Arc::new(AtomicBool::new(false)),
    )?;
    let data = STANDARD.decode(
        value["content"]
            .as_str()
            .context("No host artifact bytes")?,
    )?;
    ensure!(
        data.len() <= 700000 && value["metadata"]["sha256"] == digest(&data),
        "Artifact integrity failure"
    );
    Ok((value["metadata"].clone(), data))
}
pub fn receive_artifact(workspace: &Workspace, extension: &str, encoded: &str) -> Result<Value> {
    ensure!(
        ["docx", "xlsx", "pptx", "pdf", "png", "jpg"].contains(&extension),
        "Unsupported received artifact"
    );
    ensure!(encoded.len() <= 940000, "Received artifact exceeds limit");
    let bytes = STANDARD.decode(encoded)?;
    ensure!(bytes.len() <= 700000, "Received artifact exceeds limit");
    rpc(
        workspace,
        json!({"action":"receive_artifact","artifact":{"extension":extension,"encoding":"base64","bytes":bytes.len(),"content":encoded}}),
        &Arc::new(AtomicBool::new(false)),
    )
}
pub fn save_artifact(workspace: &Workspace, id: &str, destination: &Path) -> Result<()> {
    let (_, data) = artifact_bytes(workspace, id)?;
    store::check_local_path(destination)?;
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)?;
    file.write_all(&data)?;
    file.sync_all()?;
    Ok(())
}
pub fn open_artifact(workspace: &Workspace, id: &str) -> Result<()> {
    let (meta, data) = artifact_bytes(workspace, id)?;
    let extension = meta["extension"]
        .as_str()
        .context("No artifact extension")?;
    ensure!(
        ["docx", "xlsx", "pptx", "pdf", "png", "jpg"].contains(&extension),
        "Unsupported open format"
    );
    let folder = state_root(workspace).join("opened");
    store::check_local_path(&folder)?;
    fs::create_dir_all(&folder)?;
    let path = folder.join(format!("{}.{extension}", uuid::Uuid::new_v4()));
    fs::write(&path, data)?;
    #[cfg(windows)]
    {
        Command::new("explorer.exe").arg(&path).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(&path).spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(&path).spawn()?;
    }
    Ok(())
}

pub fn model_definitions(discovery: &Value) -> Vec<Value> {
    fn standard(schema: &mut Value) {
        if schema["type"] == "scalar" {
            *schema = json!({"anyOf":[{"type":"string","maxLength":500},{"type":"number"},{"type":"boolean"},{"type":"null"}]});
            return;
        }
        if let Some(object) = schema.as_object_mut() {
            for value in object.values_mut() {
                standard(value);
            }
        } else if let Some(array) = schema.as_array_mut() {
            for value in array {
                standard(value);
            }
        }
    }
    discovery["tools"].as_array().into_iter().flatten().filter_map(|entry|{let definition=&entry["definition"];let name=definition["name"].as_str()?;let mut schema=definition["input_schema"].clone();standard(&mut schema);if matches!(name,"office.docx"|"office.xlsx"|"office.pptx"|"pdf.generate"|"pdf.split"|"pdf.merge") {schema["properties"]["previous_artifact_id"]=json!({"type":"string","pattern":"^[0-9a-f]{32}$","description":"Optional parent artifact ID for a same-format new revision"});}Some(json!({"type":"function","function":{"name":alias(name),"description":definition["description"],"parameters":schema}}))}).collect()
}

pub fn conversation_definitions(
    settings: &Settings,
    request: &str,
    discovery: Option<&Value>,
) -> Value {
    let mut existing = crate::ollama::tool_definitions(settings)
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(discovery) = discovery.filter(|_| !settings.read_only) {
        let names: Vec<_> = discovery["tools"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v["definition"]["name"].as_str())
            .collect();
        let document_task = names.iter().any(|name| {
            name.starts_with("office.") || name.starts_with("pdf.") || *name == "workflow.run"
        });
        let request = request.to_lowercase();
        let code_task = [
            "code",
            "rust",
            "cargo",
            "repository",
            "source code",
            "implement",
            "debug",
        ]
        .iter()
        .any(|word| request.contains(word));
        if document_task && !code_task {
            existing.retain(|tool| {
                matches!(
                    tool["function"]["name"].as_str(),
                    Some(
                        "discover_tools"
                            | "tool_schema"
                            | "tool_pack_call"
                            | "list_files"
                            | "read_file"
                            | "search_text"
                            | "list_skills"
                            | "load_skill"
                            | "skill_resource"
                            | "inspect_document"
                            | "format_document"
                            | "compose_document"
                            | "render_document"
                            | "colour_palette"
                            | "colour_contrast"
                            | "design_tool"
                            | "mcp_servers"
                            | "mcp_schema"
                            | "mcp_tools"
                            | "mcp_call"
                    )
                )
            });
        }
        existing.extend(model_definitions(discovery));
    }
    Value::Array(existing)
}
