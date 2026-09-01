use crate::xdg::Xdg;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Identificador público de la aplicación Bakeneko registrada en Discord.
/// No es un secreto y se distribuye con todas las instalaciones, como hace Pear.
pub const DISCORD_APPLICATION_ID: &str = "1004971679476363274";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub accent: String,
    pub default_source: Option<String>,
    pub download_concurrency: u32,
    pub library_view: String,
    pub discord_client_id: String,
    pub discord_presence_enabled: bool,
    pub discord_show_adult: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(), accent: "#7c5cbf".into(), default_source: None,
            download_concurrency: 2, library_view: "grid".into(),
            discord_client_id: DISCORD_APPLICATION_ID.into(), discord_presence_enabled: true,
            discord_show_adult: false,
        }
    }
}

fn settings_path() -> PathBuf { Xdg::config_root().join("settings.json") }

pub fn load() -> Settings {
    let path = settings_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(s: &Settings) -> std::io::Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(s)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut f = fs::File::create(&tmp)?;
    f.write_all(json.as_bytes())?;
    f.sync_all()?;
    fs::rename(&tmp, &path)?;
    Ok(())
}
