//! Progress saved between shifts at `~/.dispatch112/progress.json`
//! (override the directory with the `DISPATCH112_HOME` environment variable).

use crate::i18n::Lang;
use crate::sim::Week;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const SAVE_VERSION: u32 = 1;
pub const FILE_NAME: &str = "progress.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub lang: Lang,
    pub week: Week,
}

impl SaveData {
    pub fn new(lang: Lang, week: Week) -> SaveData {
        SaveData { version: SAVE_VERSION, lang, week }
    }
}

/// The save directory: `$DISPATCH112_HOME`, else `~/.dispatch112`.
pub fn default_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("DISPATCH112_HOME").filter(|d| !d.is_empty()) {
        return PathBuf::from(d);
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".dispatch112")
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

/// Write atomically (temp file + rename).
pub fn save(dir: &Path, data: &SaveData) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let json = serde_json::to_string_pretty(data).map_err(io::Error::other)?;
    let tmp = dir.join(format!("{FILE_NAME}.tmp"));
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path(dir))
}

/// Load a save; `None` if missing, unreadable, from another version, or finished.
pub fn load(dir: &Path) -> Option<SaveData> {
    let text = fs::read_to_string(path(dir)).ok()?;
    let data: SaveData = serde_json::from_str(&text).ok()?;
    (data.version == SAVE_VERSION && !data.week.is_over()).then_some(data)
}

pub fn clear(dir: &Path) -> io::Result<()> {
    match fs::remove_file(path(dir)) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}
