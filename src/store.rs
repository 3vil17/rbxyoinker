// Copyright (C) 2026 3vil17. AGPL-3.0 with additional terms, see LICENSE and NOTICE.md.
use crate::error::Result;
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const SERVICE: &str = "dev.wares.rbxdl";

fn entry(key: &str) -> Result<Entry> {
    Ok(Entry::new(SERVICE, key)?)
}

pub fn save_secret(key: &str, value: &str) -> Result<()> {
    entry(key)?.set_password(value)?;
    Ok(())
}

pub fn get_secret(key: &str) -> Option<String> {
    entry(key).ok()?.get_password().ok()
}

pub fn clear_secret(key: &str) -> Result<()> {
    let e = entry(key)?;
    match e.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

const MAX_COOKIES: usize = 50;

fn cookie_key(i: usize) -> String {
    if i == 0 { "cookie".into() } else { format!("cookie_{}", i + 1) }
}

pub fn load_cookies() -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..MAX_COOKIES {
        match get_secret(&cookie_key(i)) {
            Some(c) => out.push(c),
            None => break,
        }
    }
    out
}

pub fn save_cookies(list: &[String]) -> Result<()> {
    for i in 0..MAX_COOKIES {
        clear_secret(&cookie_key(i))?;
    }
    for (i, c) in list.iter().take(MAX_COOKIES).enumerate() {
        save_secret(&cookie_key(i), c)?;
    }
    Ok(())
}

static HISTORY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn config_dir() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("rbxdl");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn default_save_dir() -> PathBuf {
    let base = dirs::download_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("rbxyoinker")
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub save_dir: String,
    #[serde(default)]
    pub sort_by_type: bool,
    #[serde(default = "yes")]
    pub mesh_obj: bool,
    #[serde(default)]
    pub mesh_glb: bool,
    #[serde(default)]
    pub accepted_terms: bool,
    #[serde(default)]
    pub seen_version: String,
    #[serde(default = "default_template")]
    pub name_template: String,
    #[serde(default = "yes")]
    pub images_png: bool,
    #[serde(default)]
    pub plugins_dir: String,
    #[serde(default = "default_scan")]
    pub scan_services: Vec<String>,
    #[serde(default = "yes")]
    pub scan_skip_coregui: bool,
    #[serde(default = "default_script_ids")]
    pub script_ids: String,
    #[serde(default)]
    pub migrate_prefix: String,
    #[serde(default = "default_speed")]
    pub migrate_speed: String,
    #[serde(default = "yes")]
    pub migrate_reuse: bool,
    #[serde(default)]
    pub migrate_keep: bool,
    #[serde(default)]
    pub migrate_stop_after: u32,
    #[serde(default = "yes")]
    pub skip_defaults: bool,
    #[serde(default)]
    pub required_version: String,
}

pub const DEFAULT_TEMPLATE: &str = "{name}_{id}";

fn default_template() -> String {
    DEFAULT_TEMPLATE.into()
}

pub const SCAN_SERVICES: &[&str] = &[
    "Workspace",
    "Lighting",
    "MaterialService",
    "ReplicatedFirst",
    "ReplicatedStorage",
    "ServerScriptService",
    "ServerStorage",
    "StarterGui",
    "StarterPack",
    "StarterPlayer",
    "SoundService",
    "Teams",
    "TextChatService",
    "Chat",
    "Other",
];

fn default_scan() -> Vec<String> {
    SCAN_SERVICES.iter().map(|s| s.to_string()).collect()
}

pub const SCRIPT_ID_MODES: &[&str] = &["off", "ask", "auto"];

fn default_script_ids() -> String {
    "off".into()
}

pub const MIGRATE_SPEEDS: &[&str] = &["safe", "normal", "fast"];

fn default_speed() -> String {
    "normal".into()
}

fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            save_dir: default_save_dir().to_string_lossy().to_string(),
            sort_by_type: false,
            mesh_obj: true,
            mesh_glb: false,
            accepted_terms: false,
            seen_version: String::new(),
            name_template: default_template(),
            images_png: true,
            plugins_dir: String::new(),
            scan_services: default_scan(),
            scan_skip_coregui: true,
            script_ids: default_script_ids(),
            migrate_prefix: String::new(),
            migrate_speed: default_speed(),
            migrate_reuse: true,
            migrate_keep: false,
            migrate_stop_after: 0,
            skip_defaults: true,
            required_version: String::new(),
        }
    }
}

pub fn load_settings() -> Settings {
    let path = config_dir().join("settings.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(s: &Settings) -> Result<()> {
    let path = config_dir().join("settings.json");
    std::fs::write(&path, serde_json::to_vec_pretty(s)?)?;
    Ok(())
}

pub type Moved = std::collections::HashMap<String, std::collections::HashMap<String, i64>>;

pub fn load_moved() -> Moved {
    std::fs::read_to_string(config_dir().join("moved.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_moved(moved: &Moved) -> Result<()> {
    std::fs::write(config_dir().join("moved.json"), serde_json::to_vec(moved)?)?;
    Ok(())
}

pub fn clear_moved() {
    let _ = std::fs::remove_file(config_dir().join("moved.json"));
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub id: i64,
    pub name: String,
    pub type_label: String,
    pub path: String,
    pub method: String,
    pub when: String,
}

pub fn load_history() -> Vec<HistoryEntry> {
    let path = config_dir().join("history.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn push_history(entry: HistoryEntry) -> Result<()> {
    let _guard = HISTORY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut all = load_history();
    all.insert(0, entry);
    all.truncate(200);
    let path = config_dir().join("history.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&all)?)?;
    Ok(())
}

pub fn clear_history() -> Result<()> {
    let path = config_dir().join("history.json");
    let _ = std::fs::remove_file(path);
    Ok(())
}

pub fn existing_download(id: i64) -> Option<HistoryEntry> {
    load_history()
        .into_iter()
        .find(|h| h.id == id && std::path::Path::new(&h.path).exists())
}

#[cfg(test)]
mod tests {
    #[test]
    fn config_stays_in_appdata_rbxdl() {
        let want = std::path::PathBuf::from(std::env::var("APPDATA").unwrap()).join("rbxdl");
        assert_eq!(super::config_dir(), want);
    }
}
