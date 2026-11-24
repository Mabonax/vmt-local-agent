//! Explicitly reviewed adapters for two known design packages, never arbitrary scripts.
use crate::{portable_skills::Registry, store, workspace::Workspace};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Runtime {
    pub python: String,
    pub python_sha256: String,
    pub soffice: String,
    pub soffice_sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub operation: String,
    pub seed: Option<String>,
    pub mode: Option<String>,
    pub theme: Option<String>,
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub input: Option<String>,
    pub output: Option<String>,
    pub count: Option<u8>,
    pub font: Option<String>,
    pub sample: Option<String>,
    pub size: Option<u8>,
    pub width: Option<u16>,
}
#[derive(Clone, Debug)]
pub struct Plan {
    pub request: Request,
    pub runtime: Runtime,
    pub fingerprint: String,
    pub input_sha256: Option<String>,
    pub workspace: PathBuf,
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    store::check_local_path(path)?;
    let metadata = fs::metadata(path)?;
    ensure!(
        metadata.is_file() && metadata.len() <= limit as u64,
        "Input exceeds limit or is not a regular file"
    );
    let mut data = Vec::new();
    fs::File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut data)?;
    ensure!(data.len() <= limit, "Input grew beyond limit");
    Ok(data)
}
fn executable(path: &str) -> Result<String> {
    ensure!(
        Path::new(path).is_absolute(),
        "Runtime executable must be an absolute path"
    );
    Ok(sha(&bounded(Path::new(path), 64 * 1024 * 1024)?))
}
pub fn configure(python: &str, soffice: &str) -> Result<()> {
    let runtime = Runtime {
        python: python.into(),
        python_sha256: executable(python)?,
        soffice: soffice.into(),
        soffice_sha256: executable(soffice)?,
    };
    store::atomic_json(store::data_dir().join("design-runtime.json"), &runtime)
}
fn runtime() -> Result<Runtime> {
    let runtime: Runtime = serde_json::from_slice(
        &bounded(&store::data_dir().join("design-runtime.json"), 8192)
            .context("Design runtime not configured; see docs/skills/design-skills.md")?,
    )?;
    ensure!(
        executable(&runtime.python)? == runtime.python_sha256
            && executable(&runtime.soffice)? == runtime.soffice_sha256,
        "Design runtime changed; operator reconfiguration required"
    );
    Ok(runtime)
}
pub fn status(id: &str) -> &'static str {
    if [
        "colour-intelligence",
        "colour-intelligence-v2",
        "professional-document-design",
        "professional-document-design-v2",
        "typography-intelligence",
        "layout-intelligence",
        "vector-graphics",
    ]
    .contains(&id)
    {
        if store::data_dir().join("design-runtime.json").is_file() {
            "Reviewed design adapter configured · each execution requires approval"
        } else {
            "Design adapter available · local runtime setup required"
        }
    } else {
        "Guidance only · tools not certified by registration"
    }
}
pub fn integration_note(id: &str) -> &'static str {
    if id == "vector-graphics" {
        "\nUse design_tool svg_compose with a reviewed JSON scene and a new SVG output. Native shapes and text remain editable. No raw SVG markup or external resources."
    } else if id == "layout-intelligence" {
        "\nUse design_tool layout_grid or layout_check with a project-relative JSON input. Results use points and top-left coordinates. Geometry is not visual certification."
    } else if id == "typography-intelligence" {
        "\nUse design_tool type_scale or measure_text. Measurements use the approved local font and are estimates of Office layout, not visual certification."
    } else if id == "colour-intelligence-v2" {
        "\nUse design_tool operations repair, gradient, categorical, simulate, system and distance. Do not execute skill scripts via shell. Each operation requires review. V1 palette/contrast/extract remain separate. Repair targets 4.5:1; simulation and distance do not certify accessibility."
    } else if id == "professional-document-design-v2" {
        "\nUse design_tool compose_v2 with reviewed JSON and a new DOCX/PPTX path. Use render separately for structural checks and PNG review. Do not run scripts via shell. Visual review remains pending."
    } else if ["colour-intelligence", "professional-document-design"].contains(&id) {
        "\nDesktop integration: use the registered design_tool with operation palette/contrast/extract for colour-intelligence or compose/render for professional-document-design. Do not use arbitrary shell commands to run these scripts. Outputs must be new project-relative paths. Rendering produces visual_review PENDING; inspect every PNG before claiming visual acceptance. Tool execution requires independent action review."
    } else {
        ""
    }
}
const ADVANCED_COLOUR: &[&str] = &[
    "repair",
    "gradient",
    "categorical",
    "simulate",
    "system",
    "distance",
];

impl Request {
    fn skill_script(&self) -> (&'static str, &'static str) {
        if self.operation == "svg_compose" {
            ("vector-graphics", "SKILL.md")
        } else if ["layout_grid", "layout_check", "layout_arrange"].contains(&self.operation.as_str()) {
            ("layout-intelligence", "SKILL.md")
        } else if ["type_scale", "measure_text"].contains(&self.operation.as_str()) {
            ("typography-intelligence", "SKILL.md")
        } else if ADVANCED_COLOUR.contains(&self.operation.as_str()) {
            ("colour-intelligence-v2", "scripts/advanced.py")
        } else if self.operation == "compose_v2" {
            ("professional-document-design-v2", "scripts/v2.py")
        } else if ["palette", "contrast", "extract"].contains(&self.operation.as_str()) {
            ("colour-intelligence", "scripts/colour.py")
        } else {
            ("professional-document-design", "scripts/compose.py")
        }
    }
    fn validate(&self, workspace: &Workspace) -> Result<()> {
        let operation = self.operation.as_str();
        ensure!(
            ["palette", "contrast", "extract", "compose", "render"].contains(&operation)
                || ADVANCED_COLOUR.contains(&operation)
                || operation == "compose_v2"
                || ["type_scale", "measure_text"].contains(&operation)
                || ["layout_grid", "layout_check", "layout_arrange"].contains(&operation)
                || operation == "svg_compose",
            "Unregistered design operation"
        );
        let value = serde_json::to_value(self)?;
        let allowed: &[&str] = match operation {
            "palette" => &["seed", "mode", "theme"],
            "contrast" => &["foreground", "background"],
            "extract" => &["input", "count"],
            "repair" | "distance" => &["foreground", "background"],
            "gradient" => &["foreground", "background", "count"],
            "categorical" => &["seed", "count"],
            "simulate" => &["seed", "mode"],
            "system" => &["seed", "theme"],
            "layout_grid" | "layout_check" | "layout_arrange" => &["input"],
            "type_scale" => &["mode", "size"],
            "measure_text" => &["font", "sample", "size", "width"],
            _ => &["input", "output"],
        };
        for (key, value) in value.as_object().unwrap() {
            ensure!(
                key == "operation" || value.is_null() || allowed.contains(&key.as_str()),
                "Unexpected argument for operation: {key}"
            );
        }
        let colour = |value: Option<&str>| -> Result<()> {
            let value = value.context("Missing hex colour")?;
            ensure!(
                value.len() == 7
                    && value.starts_with('#')
                    && value[1..].bytes().all(|b| b.is_ascii_hexdigit()),
                "Expected #RRGGBB"
            );
            Ok(())
        };
        if operation == "palette" {
            colour(self.seed.as_deref())?;
            ensure!(
                [
                    "monochromatic",
                    "analogous",
                    "complementary",
                    "split-complementary",
                    "triadic",
                    "tetradic",
                    "square"
                ]
                .contains(&self.mode.as_deref().unwrap_or("analogous")),
                "Unsupported harmony"
            );
            ensure!(
                ["light", "dark"].contains(&self.theme.as_deref().unwrap_or("light")),
                "Unsupported theme"
            );
        }
        if ["contrast", "repair", "distance", "gradient"].contains(&operation) {
            colour(self.foreground.as_deref())?;
            colour(self.background.as_deref())?;
        }
        if ["categorical", "simulate", "system"].contains(&operation) {
            colour(self.seed.as_deref())?;
        }
        if operation == "categorical" {
            ensure!(
                (2..=12).contains(&self.count.unwrap_or(6)),
                "Category count must be 2-12"
            );
        }
        if operation == "gradient" {
            ensure!(
                (2..=64).contains(&self.count.unwrap_or(7)),
                "Gradient count must be 2-64"
            );
        }
        if operation == "simulate" {
            ensure!(
                matches!(self.mode.as_deref(), Some("protan" | "deutan" | "tritan")),
                "Unsupported simulation"
            );
        }
        if operation == "system" {
            ensure!(
                ["light", "dark"].contains(&self.theme.as_deref().unwrap_or("light")),
                "Unsupported theme"
            );
        }
        if operation == "type_scale" {
            ensure!(
                matches!(self.mode.as_deref(), Some("document" | "presentation")),
                "Specify document or presentation"
            );
            ensure!(
                (10..=28).contains(&self.size.unwrap_or(12)),
                "Base size must be 10-28 points"
            );
        }
        if operation == "measure_text" {
            ensure!(
                matches!(self.font.as_deref(), Some("Arial" | "Liberation Sans")),
                "Unsupported font"
            );
            let sample = self.sample.as_deref().context("Text sample is required")?;
            ensure!(
                !sample.is_empty() && sample.len() <= 4096,
                "Text sample must be 1-4096 bytes"
            );
            ensure!(
                (8..=72).contains(&self.size.unwrap_or(12))
                    && (40..=1200).contains(&self.width.unwrap_or(400)),
                "Unsupported point size or width"
            );
        }
        if operation == "extract" {
            ensure!(
                (2..=16).contains(&self.count.unwrap_or(6)),
                "Extraction count must be 2-16"
            );
        }
        if [
            "extract",
            "compose",
            "compose_v2",
            "render",
            "layout_grid",
            "layout_check",
            "layout_arrange",
            "svg_compose",
        ]
        .contains(&operation)
        {
            let input = self.input.as_deref().context("Input path is required")?;
            ensure!(input.len() <= 240, "Input path too long");
            workspace.resolve(input, false)?;
            let suffix = Path::new(input)
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let extensions: &[&str] = match operation {
                "compose" | "compose_v2" | "layout_grid" | "layout_check" | "layout_arrange"
                | "svg_compose" => &["json"],
                "extract" => &["png", "jpg", "jpeg", "webp"],
                _ => &["docx", "pptx"],
            };
            ensure!(
                extensions.contains(&suffix.as_str()),
                "Unsupported input format"
            );
        }
        if ["compose", "compose_v2", "render", "svg_compose"].contains(&operation) {
            let output = self.output.as_deref().context("Output path is required")?;
            ensure!(
                !output.is_empty()
                    && output.len() <= 240
                    && !output
                        .replace('\\', "/")
                        .split('/')
                        .any(|v| v.starts_with('.')),
                "Invalid output path"
            );
            let path = workspace.resolve(output, true)?;
            ensure!(
                !path.exists() && path.parent().is_some_and(Path::is_dir),
                "Output must be new and its parent must exist"
            );
            if operation == "svg_compose" {
                ensure!(
                    path.extension().and_then(|v| v.to_str()).is_some_and(|v| v.eq_ignore_ascii_case("svg")),
                    "Output must be SVG"
                );
            }
            if matches!(operation, "compose" | "compose_v2") {
                ensure!(
                    matches!(
                        path.extension()
                            .and_then(|v| v.to_str())
                            .map(str::to_ascii_lowercase)
                            .as_deref(),
                        Some("docx" | "pptx")
                    ),
                    "Output must be DOCX or PPTX"
                );
            }
        }
        Ok(())
    }
    fn input_bytes(&self, workspace: &Workspace) -> Result<Option<Vec<u8>>> {
        self.input
            .as_ref()
            .map(|input| {
                bounded(
                    &workspace.resolve(input, false)?,
                    if matches!(
                        self.operation.as_str(),
                        "compose" | "compose_v2" | "layout_grid" | "layout_check" | "layout_arrange" | "svg_compose"
                    ) {
                        64 * 1024
                    } else {
                        16 * 1024 * 1024
                    },
                )
            })
            .transpose()
    }
}
pub fn prepare(workspace: &Workspace, arguments: &Value) -> Result<Plan> {
    let request: Request = serde_json::from_value(arguments.clone())?;
    request.validate(workspace)?;
    let (_, fingerprint) =
        Registry::load()?.adapter_script(request.skill_script().0, request.skill_script().1)?;
    let input_sha256 = request.input_bytes(workspace)?.map(|data| sha(&data));
    Ok(Plan {
        request,
        runtime: runtime()?,
        fingerprint,
        input_sha256,
        workspace: workspace.root.clone(),
    })
}
impl Plan {
    pub fn describe(&self) -> String {
        format!(
            "Run reviewed local design adapter as your Windows user (NOT OS-sandboxed). No shell, installs or network operation is provided. Outputs never overwrite files.\nSkill: {}\nPackage SHA-256: {}\nPython: {}\nRenderer: {}\nInput SHA-256: {:?}\nExact arguments: {}\nRendered images require visual review; render success alone is not visual acceptance.",
            self.request.skill_script().0,
            self.fingerprint,
            self.runtime.python,
            self.runtime.soffice,
            self.input_sha256,
            serde_json::to_string_pretty(&self.request).unwrap_or_default()
        )
    }
}
struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let _ = Command::new("taskkill.exe")
                    .args(["/PID", &self.0.id().to_string(), "/T", "/F"])
                    .creation_flags(0x08000000)
                    .output();
            }
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
pub fn execute(workspace: &Workspace, plan: &Plan, cancel: &Arc<AtomicBool>) -> Result<String> {
    ensure!(!cancel.load(Ordering::Relaxed), "Design operation stopped");
    ensure!(
        plan.workspace == workspace.root && runtime()? == plan.runtime,
        "Design runtime or workspace changed after review"
    );
    plan.request.validate(workspace)?;
    let (script, fingerprint) = Registry::load()?
        .adapter_script(plan.request.skill_script().0, plan.request.skill_script().1)?;
    ensure!(
        fingerprint == plan.fingerprint,
        "Skill changed after action review"
    );
    let colour_base = if ADVANCED_COLOUR.contains(&plan.request.operation.as_str()) {
        let (base, base_fingerprint) =
            Registry::load()?.adapter_script("colour-intelligence-v2", "scripts/colour.py")?;
        ensure!(
            base_fingerprint == plan.fingerprint,
            "Colour package changed after review"
        );
        Some(base)
    } else {
        None
    };
    let input = plan.request.input_bytes(workspace)?;
    ensure!(
        input.as_ref().map(|data| sha(data)) == plan.input_sha256,
        "Input changed after review; request a fresh review"
    );
    let scratch = tempfile::tempdir()?;
    if let Some(input) = input {
        fs::write(scratch.path().join("input"), input)?;
    }
    let mut args = serde_json::to_value(&plan.request)?;
    args.as_object_mut()
        .unwrap()
        .retain(|_, value| !value.is_null());
    let payload = serde_json::to_vec(
        &json!({"script":script,"colour_base":colour_base,"args":args,"soffice":plan.runtime.soffice}),
    )?;
    ensure!(payload.len() <= 100_000, "Adapter payload exceeds limit");
    let mut command = Command::new(&plan.runtime.python);
    command
        .args([
            "-I",
            "-B",
            "-c",
            include_str!("../scripts/design_runner.py"),
        ])
        .current_dir(scratch.path())
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = OwnedChild(
        command
            .spawn()
            .context("Cannot launch design Python runtime")?,
    );
    let mut stdin = child.0.stdin.take().context("No worker stdin")?;
    let stdout = child.0.stdout.take().context("No worker stdout")?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let sent = stdin.write_all(&payload);
        drop(stdin);
        let mut data = Vec::new();
        let read = stdout.take(32_001).read_to_end(&mut data);
        let _ = tx.send((sent, read, data));
    });
    let started = Instant::now();
    let data = loop {
        ensure!(
            !cancel.load(Ordering::Relaxed),
            "Design operation cancelled"
        );
        ensure!(
            started.elapsed() < Duration::from_secs(120),
            "Design operation timed out"
        );
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok((sent, read, data)) => {
                sent?;
                read?;
                break data;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                anyhow::bail!("Design worker disconnected")
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    };
    ensure!(data.len() <= 32_000, "Design result exceeds limit");
    while child.0.try_wait()?.is_none() {
        ensure!(
            !cancel.load(Ordering::Relaxed) && started.elapsed() < Duration::from_secs(120),
            "Design worker cancelled or timed out"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let result: Value = serde_json::from_slice(&data).context("Invalid design worker response")?;
    ensure!(
        child.0.try_wait()?.is_some_and(|status| status.success()) && result.get("error").is_none(),
        "Design operation failed: {}",
        result
    );
    ensure!(
        !cancel.load(Ordering::Relaxed),
        "Stopped before publishing artifacts"
    );
    if let Some(output) = &plan.request.output {
        plan.request.validate(workspace)?;
        let destination = workspace.resolve(output, true)?;
        if matches!(
            plan.request.operation.as_str(),
            "compose" | "compose_v2" | "svg_compose"
        ) {
            let suffix = Path::new(output)
                .extension()
                .context("Missing output extension")?
                .to_string_lossy()
                .to_ascii_lowercase();
            let bytes = bounded(
                &scratch.path().join(format!("artifact.{suffix}")),
                16 * 1024 * 1024,
            )?;
            ensure!(
                result["sha256"].as_str() == Some(&sha(&bytes)),
                "Artifact hash mismatch"
            );
            let mut file = tempfile::NamedTempFile::new_in(
                destination.parent().context("Missing output parent")?,
            )?;
            file.write_all(&bytes)?;
            file.as_file().sync_all()?;
            workspace.resolve(output, true)?;
            file.persist_noclobber(destination)?;
        } else {
            let staged =
                tempfile::tempdir_in(destination.parent().context("Missing review parent")?)?;
            for entry in fs::read_dir(scratch.path().join("review"))? {
                let entry = entry?;
                let name = entry.file_name();
                let name = name.to_str().context("Invalid review filename")?;
                ensure!(
                    name == "review.json"
                        || (name.starts_with("page-")
                            && name.ends_with(".png")
                            && name.len() == 12),
                    "Unexpected review artifact"
                );
                let data = bounded(&entry.path(), 16 * 1024 * 1024)?;
                fs::write(staged.path().join(name), data)?;
            }
            workspace.resolve(output, true)?;
            // Create the destination exclusively before publishing; never replace
            // a concurrent directory, even if another process creates one empty.
            fs::create_dir(&destination)?;
            for entry in fs::read_dir(staged.path())? {
                let entry = entry?;
                let target = workspace.resolve(
                    &format!("{output}/{}", entry.file_name().to_string_lossy()),
                    true,
                )?;
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(target)?;
                file.write_all(&fs::read(entry.path())?)?;
            }
        }
    }
    Ok(serde_json::to_string_pretty(&result)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unregistered_inputs_and_overwrites() {
        let directory = tempfile::tempdir().unwrap();
        let ws = Workspace::new(directory.path()).unwrap();
        for value in [
            json!({"operation":"palette","seed":"#FFFFFF; whoami"}),
            json!({"operation":"palette","seed":"#FFFFFF","command":"echo bypass"}),
            json!({"operation":"shell"}),
            json!({"operation":"palette","seed":"#FFFFFF","input":".env"}),
            json!({"operation":"extract","input":"../private.png"}),
            json!({"operation":"extract","input":".env"}),
        ] {
            assert!(
                serde_json::from_value::<Request>(value)
                    .and_then(|request| request.validate(&ws).map_err(serde::de::Error::custom))
                    .is_err()
            );
        }
        fs::write(directory.path().join("input.json"), "{}").unwrap();
        fs::write(directory.path().join("out.docx"), "keep").unwrap();
        let request: Request = serde_json::from_value(
            json!({"operation":"compose","input":"input.json","output":"out.docx"}),
        )
        .unwrap();
        assert!(request.validate(&ws).is_err());
        let valid: Request = serde_json::from_value(
            json!({"operation":"contrast","foreground":"#000000","background":"#FFFFFF"}),
        )
        .unwrap();
        assert!(valid.validate(&ws).is_ok());
    }
}
