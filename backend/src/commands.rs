use tauri::AppHandle;

use crate::{
    error::{Error, ErrorKind},
    settings::{get_settings_path, Settings},
};

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart()
}

#[tauri::command]
pub fn update_settings(app: AppHandle, new_settings: Settings) -> Result<(), Error> {
    let settings_path = get_settings_path(&app)?;

    let settings_json = serde_json::to_string(&new_settings).map_err(|e| {
        Error::new(
            ErrorKind::Serialization,
            e.to_string(),
            "invalid settings format",
        )
    })?;
    std::fs::write(settings_path, settings_json)
        .map_err(|e| Error::new(ErrorKind::FS, e.to_string(), "unable to save settings"))
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<Settings, Error> {
    let settings_path = get_settings_path(&app)?;

    let settings = std::fs::read(settings_path)
        .map_err(|e| Error::new(ErrorKind::FS, e.to_string(), "unable to access settings"))?;

    let settings: Settings = serde_json::from_slice(&settings).map_err(|e| {
        Error::new(
            ErrorKind::Serialization,
            e.to_string(),
            "incompatable settings format",
        )
    })?;

    Ok(settings)
}
