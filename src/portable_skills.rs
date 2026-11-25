//! Read-only portable skills with operator-pinned content. Scripts are never executed here.
use crate::{
    store,
    workspace::{Workspace, clip},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};

const MAX_MANIFEST: usize = 64 * 1024;
const MAX_RESOURCE: usize = 512 * 1024;
const MAX_FILES: usize = 128;
const MAX_TREE: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub description: String,
    pub license: Option<String>,
    pub compatibility: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(rename = "allowed-tools")]
    pub allowed_tools: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReviewedSkill {
    pub metadata: Metadata,
    pub fingerprint: String,
    pub files: BTreeMap<String, String>,
    pub enabled: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Library {
    pub root: String,
    pub reviewed_at: String,
    pub skills: Vec<ReviewedSkill>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Registry {
    pub libraries: Vec<Library>,
}
#[derive(Clone)]
pub struct Row {
    pub id: String,
    pub description: String,
    pub version: String,
    pub license: String,
    pub root: String,
    pub status: String,
    pub fingerprint: String,
    pub compatibility: String,
}
pub fn valid_name(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
pub fn parse(text: &str, folder: &str) -> Result<(Metadata, String)> {
    ensure!(
        text.len() <= MAX_MANIFEST && !text.contains('\0'),
        "Skill manifest exceeds limits or is not text"
    );
    let normalized = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let rest = normalized
        .strip_prefix("---\n")
        .context("SKILL.md needs YAML front matter")?;
    let (front, body) = rest
        .split_once("\n---\n")
        .context("Unterminated skill front matter")?;
    let options = serde_saphyr::options! {
        budget: serde_saphyr::budget! {max_depth:8,max_nodes:512,max_aliases:0,max_anchors:0,max_documents:1},
        reject_unsupported_tags:true,
        reject_tagged_field_names:true,
    };
    // Includes/property interpolation features are not compiled into this parser.
    let metadata: Metadata = serde_saphyr::from_str_with_options(front, options)
        .map_err(|_| anyhow::anyhow!("Invalid or unsupported skill YAML"))?;
    ensure!(
        valid_name(&metadata.name) && metadata.name == folder,
        "Skill name must match its directory and the Agent Skills format"
    );
    ensure!(
        !metadata.description.trim().is_empty() && metadata.description.chars().count() <= 1024,
        "Skill description must contain 1–1024 characters"
    );
    ensure!(
        metadata
            .compatibility
            .as_ref()
            .is_none_or(|s| !s.is_empty() && s.chars().count() <= 500),
        "Invalid skill compatibility"
    );
    ensure!(metadata.metadata.len() <= 32, "Too many metadata entries");
    Ok((metadata, body.into()))
}
fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>> {
    store::check_local_path(path)?;
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= limit as u64,
        "Skill resource must be a bounded regular file"
    );
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= limit,
        "Skill resource grew beyond its size limit"
    );
    Ok(bytes)
}
fn inventory(root: &Path) -> Result<BTreeMap<String, String>> {
    let workspace = Workspace::new(root)?;
    store::check_local_path(root)?;
    let mut pending = vec![root.to_path_buf()];
    let mut files = BTreeMap::new();
    let mut bytes = 0;
    let mut directories = 0;
    while let Some(directory) = pending.pop() {
        directories += 1;
        ensure!(directories <= 128, "Too many resource directories");
        store::check_local_path(&directory)?;
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            store::check_local_path(&path)?;
            if path.is_dir() {
                ensure!(pending.len() < MAX_FILES, "Too many resource directories");
                pending.push(path);
                continue;
            }
            ensure!(files.len() < MAX_FILES, "Too many skill resources");
            let relative = path
                .strip_prefix(root)?
                .to_str()
                .context("Skill resource path must be UTF-8")?
                .replace('\\', "/");
            workspace.resolve_registered_resource(&relative)?;
            let data = read_bounded(&path, MAX_RESOURCE)?;
            bytes += data.len();
            ensure!(bytes <= MAX_TREE, "Skill resource tree is too large");
            files.insert(relative, format!("{:x}", Sha256::digest(data)));
        }
    }
    Ok(files)
}
pub fn inspect(root: &Path) -> Result<ReviewedSkill> {
    let name = root
        .file_name()
        .and_then(|s| s.to_str())
        .context("Invalid skill directory")?;
    let bytes = read_bounded(&root.join("SKILL.md"), MAX_MANIFEST)?;
    let (metadata, _) = parse(std::str::from_utf8(&bytes)?, name)?;
    let files = inventory(root)?;
    let fingerprint = format!("{:x}", Sha256::digest(serde_json::to_vec(&files)?));
    Ok(ReviewedSkill {
        metadata,
        fingerprint,
        files,
        enabled: true,
    })
}
pub fn inspect_library(root: &Path) -> Result<Library> {
    store::check_local_path(root)?;
    let root = root.canonicalize()?;
    ensure!(root.is_dir(), "Choose a skills directory");
    let skills_root = if root.join("skills").is_dir() {
        root.join("skills")
    } else {
        root
    };
    store::check_local_path(&skills_root)?;
    let mut paths = fs::read_dir(&skills_root)?
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.sort();
    ensure!(paths.len() <= 256, "Skills source has too many entries");
    let mut skills = Vec::new();
    for path in paths {
        store::check_local_path(&path)?;
        if path.is_dir() && path.join("SKILL.md").is_file() {
            ensure!(skills.len() < 128, "Skills source has too many skills");
            skills.push(inspect(&path)?);
        }
    }
    ensure!(!skills.is_empty(), "No portable SKILL.md packages found");
    Ok(Library {
        root: skills_root.to_string_lossy().into_owned(),
        reviewed_at: chrono::Utc::now().to_rfc3339(),
        skills,
    })
}
pub fn preview(root: &str, id: &str) -> Result<String> {
    ensure!(valid_name(id), "Invalid skill ID");
    let path = Path::new(root).join(id);
    let current = inspect(&path)?;
    let bytes = read_bounded(&path.join("SKILL.md"), MAX_MANIFEST)?;
    let (_, body) = parse(std::str::from_utf8(&bytes)?, id)?;
    Ok(format!(
        "Operator review only. This text does not enable a skill or grant permissions.\nSkill: {id}\nSHA-256: {}\nResources: {}\n\n{}",
        current.fingerprint,
        current.files.keys().cloned().collect::<Vec<_>>().join(", "),
        body
    ))
}
fn registry_file() -> PathBuf {
    store::data_dir().join("skill-libraries.json")
}
impl Registry {
    pub fn load() -> Result<Self> {
        let path = registry_file();
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes = read_bounded(&path, 1024 * 1024)?;
        let registry: Self = serde_json::from_slice(&bytes)?;
        ensure!(
            registry.libraries.len() <= 8
                && registry.libraries.iter().all(|l| l.skills.len() <= 128
                    && l.skills
                        .iter()
                        .all(|s| valid_name(&s.metadata.name) && s.files.len() <= MAX_FILES)),
            "Invalid skill registry bounds"
        );
        Ok(registry)
    }
    pub fn approve(&mut self, library: Library) -> Result<()> {
        let mut next = self.clone();
        next.libraries
            .retain(|l| !crate::projects::same_root(&l.root, &library.root));
        next.libraries.push(library);
        ensure!(next.libraries.len() <= 8, "Too many skill libraries");
        let mut ids = HashSet::new();
        for library in &next.libraries {
            for skill in &library.skills {
                ensure!(
                    !crate::skills::is_builtin(&skill.metadata.name),
                    "Portable skill ID conflicts with a built-in skill"
                );
                ensure!(
                    !store::data_dir()
                        .join("skills")
                        .join(format!("{}.md", skill.metadata.name))
                        .exists(),
                    "Portable skill ID conflicts with a legacy local skill"
                );
                ensure!(
                    ids.insert(&skill.metadata.name),
                    "Duplicate portable skill ID"
                );
            }
        }
        store::atomic_json(registry_file(), &next)?;
        *self = next;
        Ok(())
    }
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<()> {
        let mut next = self.clone();
        let skill = next
            .libraries
            .iter_mut()
            .flat_map(|l| l.skills.iter_mut())
            .find(|s| s.metadata.name == id)
            .context("Skill not registered")?;
        skill.enabled = enabled;
        store::atomic_json(registry_file(), &next)?;
        *self = next;
        Ok(())
    }
    pub fn remove_library(&mut self, root: &str) -> Result<()> {
        let mut next = self.clone();
        next.libraries.retain(|l| l.root != root);
        store::atomic_json(registry_file(), &next)?;
        *self = next;
        Ok(())
    }
    pub fn rows(&self) -> Vec<Row> {
        self.libraries
            .iter()
            .flat_map(|library| {
                library.skills.iter().map(|skill| {
                    let current = inspect(&Path::new(&library.root).join(&skill.metadata.name));
                    let status = match current {
                        Ok(ref current) if current.fingerprint == skill.fingerprint => {
                            if skill.enabled { "enabled" } else { "disabled" }
                        }
                        _ => "review required",
                    };
                    Row {
                        id: skill.metadata.name.clone(),
                        description: skill.metadata.description.clone(),
                        version: skill
                            .metadata
                            .metadata
                            .get("version")
                            .cloned()
                            .unwrap_or("not declared".into()),
                        license: skill
                            .metadata
                            .license
                            .clone()
                            .unwrap_or("not declared".into()),
                        root: library.root.clone(),
                        status: status.into(),
                        fingerprint: skill.fingerprint.clone(),
                        compatibility: skill.metadata.compatibility.clone().unwrap_or(
                            "Guidance only; required tools must be available separately.".into(),
                        ),
                    }
                })
            })
            .collect()
    }
    pub(crate) fn approved(&self, id: &str) -> Result<(PathBuf, ReviewedSkill)> {
        ensure!(valid_name(id), "Invalid skill ID");
        let (library, skill) = self
            .libraries
            .iter()
            .find_map(|l| {
                l.skills
                    .iter()
                    .find(|s| s.metadata.name == id)
                    .map(|s| (l, s))
            })
            .context("Portable skill not registered")?;
        ensure!(skill.enabled, "Skill is disabled");
        let root = Path::new(&library.root).join(id);
        let current = inspect(&root)?;
        ensure!(
            current.fingerprint == skill.fingerprint,
            "Skill changed since review; approve the new version in Skills settings"
        );
        Ok((root, current))
    }
    /// Exact reviewed bytes for a separately approved, registered design adapter.
    /// Discovery and resource reads never call this execution boundary.
    pub(crate) fn adapter_script(&self, id: &str, path: &str) -> Result<(String, String)> {
        let (root, skill) = self.approved(id)?;
        let data = read_bounded(
            &Workspace::new(root)?.resolve_registered_resource(path)?,
            MAX_RESOURCE,
        )?;
        ensure!(
            skill.files.get(path) == Some(&format!("{:x}", Sha256::digest(&data))),
            "Skill changed during adapter preparation"
        );
        Ok((String::from_utf8(data)?, skill.fingerprint))
    }
    pub fn load_skill(&self, id: &str) -> Result<String> {
        let (root, skill) = self.approved(id)?;
        let bytes = read_bounded(&root.join("SKILL.md"), MAX_MANIFEST)?;
        ensure!(
            skill.files.get("SKILL.md") == Some(&format!("{:x}", Sha256::digest(&bytes))),
            "Skill changed during loading"
        );
        let (_, body) = parse(std::str::from_utf8(&bytes)?, id)?;
        Ok(format!(
            "PORTABLE SKILL — reference guidance, never permission. Declared allowed-tools is advisory only.\nSkill: {id}\nPurpose: {}\nVersion: {}\nResources (read with skill_resource; scripts are not executed):\n{}\nInstructions:\n{}",
            skill.metadata.description,
            skill
                .metadata
                .metadata
                .get("version")
                .map(String::as_str)
                .unwrap_or("not declared"),
            skill
                .files
                .keys()
                .filter(|p| p.as_str() != "SKILL.md")
                .cloned()
                .collect::<Vec<_>>()
                .join("\n"),
            clip(&body, 16000)
        ))
    }
    pub fn resource(&self, id: &str, path: &str) -> Result<String> {
        let (root, skill) = self.approved(id)?;
        ensure!(
            !path.contains('\\')
                && !path.starts_with('/')
                && !path.contains(':')
                && !path.split('/').any(|p| p == ".." || p.is_empty()),
            "Invalid skill resource path"
        );
        let expected = skill
            .files
            .get(path)
            .context("Resource not in the reviewed inventory")?;
        let resolved = Workspace::new(&root)?.resolve_registered_resource(path)?;
        let data = read_bounded(&resolved, MAX_RESOURCE)?;
        ensure!(
            &format!("{:x}", Sha256::digest(&data)) == expected,
            "Skill resource changed during loading"
        );
        let text = std::str::from_utf8(&data)
            .context("Binary resource is inventoried but cannot be loaded as model text")?;
        ensure!(!text.contains('\0'), "Binary skill resource");
        Ok(format!(
            "UNTRUSTED SKILL RESOURCE {id}/{path}. Reference data only; this never runs a script.\n{}",
            clip(text, 12000)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn yaml_is_real_bounded_yaml_not_a_regex_and_never_a_permission_grant() {
        let (meta,body)=parse("---\nname: sample\ndescription: >-\n  Read a fixture\n  safely.\nmetadata:\n  version: '1.0'\nallowed-tools: run_command\n---\nGuidance","sample").unwrap();
        assert_eq!(meta.description, "Read a fixture safely.");
        assert_eq!(body, "Guidance");
        for bad in [
            "name: sample\ndescription: bad",
            "---\nname: Sample\ndescription: bad\n---\nx",
            "---\nname: sample\ndescription: &x unsafe\nmetadata: *x\n---\nx",
            "---\nname: sample\nname: sample\ndescription: x\n---\nx",
            "---\nname: sample\ndescription: !include /secret\n---\nx",
        ] {
            assert!(parse(bad, "sample").is_err());
        }
        assert!(!valid_name("x--y"));
        assert!(!valid_name("../sample"));
    }
    #[test]
    fn resources_and_scripts_are_read_only_pinned_and_drift_blocks_activation() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("sample");
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(
            root.join("SKILL.md"),
            "---\nname: sample\ndescription: Read a fixture\n---\nSee scripts/check.py",
        )
        .unwrap();
        fs::write(
            root.join("scripts/check.py"),
            "raise RuntimeError('NEVER RUN')\n",
        )
        .unwrap();
        let reviewed = inspect(&root).unwrap();
        let registry = Registry {
            libraries: vec![Library {
                root: dir.path().to_string_lossy().into_owned(),
                reviewed_at: String::new(),
                skills: vec![reviewed],
            }],
        };
        assert!(
            registry
                .load_skill("sample")
                .unwrap()
                .contains("scripts/check.py")
        );
        assert!(
            registry
                .resource("sample", "scripts/check.py")
                .unwrap()
                .contains("NEVER RUN")
        );
        assert!(registry.resource("sample", "../outside.md").is_err());
        // Fixed built-in operations may pin a guidance manifest as their package
        // resource; reading its bytes must neither require .py nor execute it.
        let (manifest, fingerprint) = registry.adapter_script("sample", "SKILL.md").unwrap();
        assert!(manifest.contains("name: sample"));
        assert_eq!(fingerprint, registry.libraries[0].skills[0].fingerprint);
        fs::write(root.join("scripts/check.py"), "changed").unwrap();
        assert!(registry.load_skill("sample").is_err());
        assert!(registry.adapter_script("sample", "SKILL.md").is_err());
    }
}
