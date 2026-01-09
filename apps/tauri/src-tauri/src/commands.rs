use tauri::Manager;

/// Get the current application version
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get the current platform
#[tauri::command]
pub fn get_platform() -> String {
    #[cfg(target_os = "windows")]
    return "windows".to_string();
    #[cfg(target_os = "macos")]
    return "macos".to_string();
    #[cfg(target_os = "linux")]
    return "linux".to_string();
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return "unknown".to_string()
}

/// Set badge count on the dock/taskbar icon (macOS only for now)
#[tauri::command]
pub fn set_badge_count(count: i32) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if count > 0 {
            let script = format!(
                r#"tell application "System Events" to set badge of application "Mezon" to {}"#,
                count
            );
            Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .output()
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Clear badge count
#[tauri::command]
pub fn clear_badge() -> Result<(), String> {
    set_badge_count(0)
}

/// Show a system notification
#[tauri::command]
pub async fn show_notification(
    app: tauri::AppHandle,
    title: String,
    body: String,
) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;

    app.notification()
        .builder()
        .title(&title)
        .body(&body)
        .show()
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Minimize the main window to system tray
#[tauri::command]
pub fn minimize_to_tray(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Quit the application
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}
