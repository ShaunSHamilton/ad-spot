use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

pub const SETTINGS_PATH: &str = "settings.json";

#[derive(Default, Serialize, Deserialize)]
pub struct Settings {
    pub enabled: bool,
}

pub fn get_settings_path(app: &AppHandle) -> Result<PathBuf, tauri::Error> {
    let config_path = app.path().app_config_dir()?;
    let settings_path = config_path.join(SETTINGS_PATH);
    Ok(settings_path)
}
