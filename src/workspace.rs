use anyhow::{Context, Result, bail, ensure};
use ignore::WalkBuilder;
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Workspace {
    pub root: PathBuf,
}

pub fn sensitive_component(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    (n.starts_with(".env") && !n.ends_with(".example"))
        || [
            ".git",
            ".ssh",
            ".aws",
            ".azure",
            ".gnupg",
            ".npmrc",
            ".netrc",
            ".git-credentials",
            "credentials.json",
            "id_rsa",
            "id_ed25519",
        ]
        .contains(&n.as_str())
        || [".pem", ".key", ".pfx", ".p12", ".kdbx"]
            .iter()
            .any(|s| n.ends_with(s))
}

fn reparse(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

impl Workspace {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let root = path
            .as_ref()
            .canonicalize()
            .context("Choose an existing project folder")?;
        ensure!(root.is_dir(), "The project must be a folder");
        Ok(Self { root })
    }

    pub fn resolve(&self, relative: &str, allow_missing: bool) -> Result<PathBuf> {
        self.resolve_scoped(relative, allow_missing, false)
    }
    /// Host-only skill inventory/resource reads. The registry independently
    /// verifies registration, package fingerprints and each resource's bytes.
    /// Model project file tools always use resolve(), which blocks profile data.
    pub(crate) fn resolve_registered_resource(&self, relative: &str) -> Result<PathBuf> {
        self.resolve_scoped(relative, false, true)
    }
    fn resolve_scoped(
        &self,
        relative: &str,
        allow_missing: bool,
        registered_resource: bool,
    ) -> Result<PathBuf> {
        ensure!(
            !relative.contains(':') && !relative.contains('\0'),
            "Use a relative project path without drive names or streams"
        );
        let normalized = relative.replace('\\', "/");
        let mut out = self.root.clone();
        for component in Path::new(&normalized).components() {
            match component {
                Component::CurDir => {}
                Component::Normal(value) => {
                    let name = value.to_string_lossy();
                    ensure!(
                        !sensitive_component(&name),
                        "Protected file or directory: {name}"
                    );
                    ensure!(
                        !name.ends_with(['.', ' ']),
                        "Ambiguous Windows path component"
                    );
                    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
                    ensure!(
                        ![
                            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5",
                            "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5",
                            "LPT6", "LPT7", "LPT8", "LPT9"
                        ]
                        .contains(&stem.as_str()),
                        "Windows device paths are not allowed"
                    );
                    out.push(value);
                    match fs::symlink_metadata(&out) {
                        Ok(meta) => ensure!(
                            !reparse(&meta),
                            "Links and junctions are not accessible to file tools"
                        ),
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound && allow_missing => {}
                        Err(e) => return Err(e).context("Project path is unavailable"),
                    }
                }
                _ => bail!(
                    "Paths must stay inside the selected project; parent traversal is not allowed"
                ),
            }
        }
        ensure!(
            out.starts_with(&self.root),
            "Path is outside the selected project"
        );
        // A broad workspace (for example a user's home directory) must never
        // turn the agent's private policy, runtime or artifact state into model
        // editable project files. These are managed only by host interfaces.
        let normalized = |path: &Path| {
            path.to_string_lossy()
                .trim_start_matches("\\\\?\\")
                .replace('\\', "/")
                .to_lowercase()
        };
        let private_root = normalized(&crate::store::data_dir());
        let target = normalized(&out);
        ensure!(
            registered_resource
                || (target != private_root && !target.starts_with(&(private_root + "/"))),
            "Local agent profile state is host-managed and unavailable to project file tools"
        );
        Ok(out)
    }

    pub fn files(&self, relative: &str, limit: usize) -> Result<Vec<String>> {
        let start = self.resolve(relative, false)?;
        let mut files = Vec::new();
        for entry in WalkBuilder::new(start)
            .hidden(false)
            .follow_links(false)
            .max_depth(Some(12))
            .build()
        {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_some_and(|f| f.is_file()) {
                continue;
            }
            let Ok(rel) = entry.path().strip_prefix(&self.root) else {
                continue;
            };
            if rel.components().any(|c| {
                let n = c.as_os_str().to_string_lossy();
                sensitive_component(&n)
                    || ["node_modules", "vendor", "target", ".next", "build", "dist"]
                        .contains(&n.as_ref())
            }) {
                continue;
            }
            let name = rel.to_string_lossy().replace('\\', "/");
            if self.resolve(&name, false).is_ok() {
                files.push(name);
            }
            if files.len() >= limit {
                break;
            }
        }
        files.sort();
        Ok(files)
    }

    pub fn read(&self, relative: &str) -> Result<String> {
        let path = self.resolve(relative, false)?;
        ensure!(path.is_file(), "Select a text file");
        ensure!(
            fs::metadata(&path)?.len() <= 512 * 1024,
            "File exceeds the 512 KB text limit; search it or narrow the task"
        );
        let mut bytes = Vec::new();
        fs::File::open(path)?
            .take(512 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= 512 * 1024,
            "File grew beyond the 512 KB text limit"
        );
        let text = String::from_utf8(bytes).context("File is not readable UTF-8 text")?;
        ensure!(!text.contains('\0'), "Binary files are not supported");
        Ok(text)
    }

    pub fn instructions(&self) -> String {
        let mut result = String::new();
        // Ancestor instructions are included in order; the project is never scanned outside its root.
        let mut ancestors: Vec<_> = self.root.ancestors().collect();
        ancestors.reverse();
        for dir in ancestors {
            let path = dir.join("AGENTS.md");
            if let Ok(meta) = fs::symlink_metadata(&path)
                && meta.len() <= 32_768
                && !reparse(&meta)
                && let Ok(text) = fs::read_to_string(&path)
            {
                result.push_str(&format!(
                    "\nInstructions for {} and its descendants:\n{}\n",
                    dir.display(),
                    clip(&text, 6000)
                ));
            }
        }
        clip(&result, 12_000)
    }

    pub fn nested_instructions(&self, relative: &str) -> Result<String> {
        let file = self.resolve(relative, false)?;
        let mut paths = Vec::new();
        let mut parent = file.parent();
        while let Some(dir) = parent {
            if dir == self.root || !dir.starts_with(&self.root) {
                break;
            }
            paths.push(dir.to_path_buf());
            parent = dir.parent();
        }
        paths.reverse();
        let mut result = String::new();
        for dir in paths {
            let instruction = dir.join("AGENTS.md");
            let rel = instruction
                .strip_prefix(&self.root)?
                .to_string_lossy()
                .to_string();
            if let Ok(text) = self.read(&rel) {
                result.push_str(&format!(
                    "\nScoped instructions from {rel}:\n{}\n",
                    clip(&text, 4000)
                ));
            }
        }
        Ok(clip(&result, 8000))
    }

    pub fn write_reviewed(
        &self,
        relative: &str,
        expected: Option<&str>,
        content: &str,
    ) -> Result<()> {
        ensure!(content.len() <= 512 * 1024, "Proposed file exceeds 512 KB");
        let path = self.resolve(relative, true)?;
        ensure!(path != self.root, "A file path is required");
        match expected {
            Some(old) => ensure!(
                self.read(relative)? == old,
                "The file changed after review. Read it again and propose a new edit."
            ),
            None => ensure!(
                !path.exists(),
                "A file appeared after review; it was preserved"
            ),
        }
        let parent = path.parent().context("Missing parent directory")?;
        fs::create_dir_all(parent)?;
        self.resolve(relative, true)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(content.as_bytes())?;
        temp.as_file().sync_all()?;
        // Recheck as close to persistence as possible. This reduces the window
        // for an external editor to change the target after the initial check.
        // This is not a replacement for OS-level handle-based path isolation.
        self.resolve(relative, true)?;
        match expected {
            Some(old) => {
                ensure!(
                    self.read(relative)? == old,
                    "The file changed during preparation; the reviewed edit was not applied"
                );
                temp.persist(&path)
                    .context("Could not atomically replace the reviewed file")?;
            }
            None => {
                ensure!(
                    fs::symlink_metadata(&path)
                        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
                    "The target appeared during preparation; it was preserved"
                );
                temp.persist_noclobber(&path)
                    .context("A file appeared; no existing file was replaced")?;
            }
        }
        Ok(())
    }
}

pub fn clip(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.into();
    }
    let mut end = limit;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n[Output truncated]", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_tools_block_traversal_secrets_devices_and_streams() {
        let dir = tempfile::tempdir().unwrap();
        let w = Workspace::new(dir.path()).unwrap();
        for p in [
            "../outside",
            "C:/outside",
            ".env",
            "folder/.env.local",
            ".git/config",
            "test.txt:stream",
            "CON",
            "folder./test",
        ] {
            assert!(w.resolve(p, true).is_err(), "{p}");
        }
        assert!(w.resolve(".env.example", true).is_ok());
    }
    #[test]
    fn review_does_not_overwrite_concurrent_changes() {
        let dir = tempfile::tempdir().unwrap();
        let w = Workspace::new(dir.path()).unwrap();
        fs::write(dir.path().join("file.txt"), "original").unwrap();
        fs::write(dir.path().join("file.txt"), "user edit").unwrap();
        assert!(
            w.write_reviewed("file.txt", Some("original"), "agent edit")
                .is_err()
        );
        assert_eq!(w.read("file.txt").unwrap(), "user edit");
    }
    #[test]
    fn reviewed_new_file_is_atomic_and_cannot_clobber() {
        let dir = tempfile::tempdir().unwrap();
        let w = Workspace::new(dir.path()).unwrap();
        w.write_reviewed("src/demo.txt", None, "hello").unwrap();
        assert!(
            w.write_reviewed("src/demo.txt", None, "replacement")
                .is_err()
        );
        assert_eq!(w.read("src/demo.txt").unwrap(), "hello");
    }
    #[cfg(windows)]
    #[test]
    fn windows_junctions_are_rejected_for_workspace_and_app_data() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project");
        let outside = dir.path().join("outside");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("sentinel.txt"), "preserved").unwrap();
        let link = project.join("escape");
        let quoted = |p: &Path| p.to_string_lossy().replace('\'', "''");
        let script = format!(
            "New-Item -ItemType Junction -Path '{}' -Target '{}' -ErrorAction Stop | Out-Null",
            quoted(&link),
            quoted(&outside)
        );
        let status = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .status()
            .unwrap();
        assert!(
            status.success(),
            "Could not create the junction regression fixture"
        );
        let ws = Workspace::new(&project).unwrap();
        assert!(ws.read("escape/sentinel.txt").is_err());
        assert!(
            ws.write_reviewed("escape/new.txt", None, "blocked")
                .is_err()
        );
        assert!(crate::store::check_local_path(&link.join("note.json")).is_err());
        assert_eq!(
            fs::read_to_string(outside.join("sentinel.txt")).unwrap(),
            "preserved"
        );
        assert!(!outside.join("new.txt").exists());
        fs::remove_dir(link).unwrap(); // Remove only the junction, never its target.
    }
    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).unwrap();
        assert!(
            Workspace::new(root.path())
                .unwrap()
                .resolve("escape/file", true)
                .is_err()
        );
    }
}
