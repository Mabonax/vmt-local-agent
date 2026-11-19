use crate::types::{Session, Settings};
use anyhow::{Context, Result, ensure};
use std::{fs, io::Write, path::PathBuf};

pub fn data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|x| PathBuf::from(x).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("VMT")
        .join("LocalAgent")
}
/// Reject linked files and linked ancestor directories before local persistence.
/// This reduces reparse attacks; it does not eliminate concurrent path races.
pub fn check_local_path(path: &std::path::Path) -> Result<()> {
    for parent in path.ancestors() {
        match fs::symlink_metadata(parent) {
            Ok(meta) => {
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::MetadataExt;
                    meta.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let linked = meta.file_type().is_symlink();
                ensure!(
                    !linked,
                    "Local app data must not traverse links or junctions"
                );
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
fn atomic_json<T: serde::Serialize>(path: PathBuf, value: &T) -> Result<()> {
    check_local_path(&path)?;
    let parent = path.parent().context("Missing settings directory")?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(&serde_json::to_vec_pretty(value)?)?;
    temp.as_file().sync_all()?;
    persist_local(temp, &path)?;
    Ok(())
}
/// Windows scanners can briefly hold a just-written local state file. Keep the
/// exact temporary snapshot and retry only transient permission failures.
pub fn persist_local(mut temp: tempfile::NamedTempFile, path: &std::path::Path) -> Result<()> {
    for attempt in 0..4 {
        check_local_path(path)?;
        match temp.persist(path) {
            Ok(_) => return Ok(()),
            Err(error)
                if error.error.kind() == std::io::ErrorKind::PermissionDenied && attempt < 3 =>
            {
                temp = error.file;
                std::thread::sleep(std::time::Duration::from_millis(25 * (attempt + 1)));
            }
            Err(error) => return Err(error.into()),
        }
    }
    unreachable!()
}
pub fn load_settings() -> Settings {
    let path = data_dir().join("settings.json");
    check_local_path(&path)
        .ok()
        .and_then(|_| fs::metadata(&path).ok())
        .filter(|m| m.is_file() && m.len() <= 16 * 1024)
        .and_then(|_| fs::read(path).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
pub fn save_settings(settings: &Settings) -> Result<()> {
    atomic_json(data_dir().join("settings.json"), settings)
}
pub fn save_session(session: &Session) -> Result<()> {
    // Only our generated UUID is used for a filename, never a model-produced path.
    uuid::Uuid::parse_str(&session.id)?;
    atomic_json(
        data_dir()
            .join("sessions")
            .join(format!("{}.json", session.id)),
        session,
    )
}
pub fn sessions() -> Vec<Session> {
    let mut result = Vec::new();
    if check_local_path(&data_dir().join("sessions")).is_err() {
        return result;
    }
    if let Ok(entries) = fs::read_dir(data_dir().join("sessions")) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|x| x == "json")
                && check_local_path(&entry.path()).is_ok()
                && entry
                    .metadata()
                    .is_ok_and(|m| m.is_file() && m.len() <= 2 * 1024 * 1024)
                && let Ok(data) = fs::read(entry.path())
                && let Ok(session) = serde_json::from_slice::<Session>(&data)
            {
                result.push(session);
            }
        }
    }
    result.sort_by(|a, b| b.updated.cmp(&a.updated));
    result.truncate(100);
    result
}
