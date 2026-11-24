//! Operator-triggered clipboard writes. Never reads existing clipboard contents.
use anyhow::{Context, Result};
use std::sync::{Mutex, OnceLock};

pub fn copy(text: &str) -> Result<()> {
    // Retain ownership for Linux as well as Windows; confine access to this lock.
    static CLIPBOARD: OnceLock<Mutex<Option<arboard::Clipboard>>> = OnceLock::new();
    let mut clipboard = CLIPBOARD
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| anyhow::anyhow!("Clipboard unavailable"))?;
    let mut last_error = None;
    for attempt in 0..3 {
        if clipboard.is_none() {
            match arboard::Clipboard::new() {
                Ok(value) => *clipboard = Some(value),
                Err(error) => last_error = Some(error),
            }
        }
        if let Some(clipboard) = clipboard.as_mut() {
            match clipboard.set_text(text) {
                Ok(()) => return Ok(()),
                Err(error) => last_error = Some(error),
            }
        }
        if attempt < 2 {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    Err(anyhow::anyhow!(
        "{}",
        last_error.context("Clipboard did not open")?
    ))
    .context("Could not copy. The clipboard may be busy; try again")
}
