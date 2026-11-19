use crate::types::{Session, Settings};
use anyhow::{Context, Result};
use std::{fs, io::Write, path::PathBuf};

pub fn data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|x| PathBuf::from(x).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir)
        .join("VMT")
        .join("LocalAgent")
}
fn atomic_json<T: serde::Serialize>(path: PathBuf, value: &T) -> Result<()> {
    let parent = path.parent().context("Missing settings directory")?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(&serde_json::to_vec_pretty(value)?)?;
    temp.as_file().sync_all()?;
    temp.persist(path)?;
    Ok(())
}
pub fn load_settings() -> Settings {
    fs::read(data_dir().join("settings.json"))
        .ok()
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
    if let Ok(entries) = fs::read_dir(data_dir().join("sessions")) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|x| x == "json")
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
