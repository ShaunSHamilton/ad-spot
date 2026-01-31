use tauri::{
    menu::{Menu, MenuEvent, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, Window, WindowEvent,
};

use crate::{
    commands::{get_settings, update_settings},
    settings::{get_settings_path, Settings},
};

mod commands;
mod error;
mod settings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::restart_app,
            commands::get_settings,
            commands::update_settings
        ])
        .setup(setup)
        .on_window_event(on_window_event)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    // Create settings config dir/file if it does not exist
    let config_path = app.path().app_config_dir()?;
    let exists = std::fs::exists(&config_path)?;
    if !exists {
        std::fs::create_dir_all(config_path)?;
    }
    let settings_path = get_settings_path(app.handle())?;
    let exists = std::fs::exists(settings_path)?;
    if !exists {
        update_settings(app.handle().clone(), Settings::default())?;
    }

    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit_i])?;
    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .icon(
            app.default_window_icon()
                .expect("window default icon to exist")
                .clone(),
        )
        .title("Ad Spot")
        .tooltip("Ad Spot")
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_icon_event)
        .build(app)?;

    // Start the optional Windows audio monitor. The monitor and COM are
    // initialized on the dedicated thread to avoid COM initialization issues.
    #[cfg(target_os = "windows")]
    {
        std::thread::Builder::new()
            .name("app-mute-monitor".into())
            .spawn(|| {
                // Create and own the monitor inside the thread
                match app_mute::Monitor::new() {
                    Ok(monitor) => loop {
                        if let Err(e) = monitor.check_and_apply() {
                            eprintln!("app-mute monitor error: {:?}", e);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                    },
                    Err(e) => eprintln!("Failed to init app-mute monitor: {:?}", e),
                }
            })
            .map_err(|e| eprintln!("Failed to spawn app-mute monitor thread: {:?}", e))
            .ok();
    }

    Ok(())
}

pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window.hide();
    };
}

pub fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    if event.id.as_ref() == "quit" {
        app.exit(0);
    }
}

pub fn on_tray_icon_event(tray: &TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        let app = tray.app_handle();
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}
