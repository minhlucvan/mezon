use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tauri::{Emitter, Manager};

// ============================================================================
// App Info Commands
// ============================================================================

/// Get the current application version
#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Get the current platform (matching Electron's process.platform)
#[tauri::command]
pub fn get_platform() -> String {
    #[cfg(target_os = "windows")]
    return "win32".to_string();
    #[cfg(target_os = "macos")]
    return "darwin".to_string();
    #[cfg(target_os = "linux")]
    return "linux".to_string();
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return "unknown".to_string();
}

/// Get a unique device ID
#[tauri::command]
pub fn get_device_id() -> String {
    let mut hasher = DefaultHasher::new();
    if let Ok(hostname) = hostname::get() {
        hostname.to_string_lossy().hash(&mut hasher);
    }
    // Add username for more uniqueness
    if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
        user.hash(&mut hasher);
    }
    format!("{:x}", hasher.finish())
}

/// Get sender ID for push notifications
#[tauri::command]
pub fn get_sender_id() -> String {
    std::env::var("MEZON_SENDER_ID").unwrap_or_default()
}

// ============================================================================
// Window Management Commands
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TitleBarAction {
    Minimize,
    Maximize,
    Close,
    Hide,
}

/// Handle title bar actions (minimize, maximize, close)
#[tauri::command]
pub fn title_bar_action(app: tauri::AppHandle, action: TitleBarAction) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    match action {
        TitleBarAction::Minimize => window.minimize().map_err(|e| e.to_string())?,
        TitleBarAction::Maximize => {
            if window.is_maximized().unwrap_or(false) {
                window.unmaximize().map_err(|e| e.to_string())?;
            } else {
                window.maximize().map_err(|e| e.to_string())?;
            }
        }
        TitleBarAction::Close => {
            window.hide().map_err(|e| e.to_string())?;
        }
        TitleBarAction::Hide => {
            window.hide().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct WindowState {
    pub is_maximized: bool,
    pub is_minimized: bool,
    pub is_focused: bool,
    pub is_visible: bool,
}

/// Get the current window state
#[tauri::command]
pub fn get_window_state(app: tauri::AppHandle) -> Result<WindowState, String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;

    Ok(WindowState {
        is_maximized: window.is_maximized().unwrap_or(false),
        is_minimized: window.is_minimized().unwrap_or(false),
        is_focused: window.is_focused().unwrap_or(false),
        is_visible: window.is_visible().unwrap_or(false),
    })
}

/// Set the window title
#[tauri::command]
pub fn set_window_title(app: tauri::AppHandle, title: String) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window.set_title(&title).map_err(|e| e.to_string())
}

/// Set window zoom ratio
#[tauri::command]
pub fn set_ratio_window(app: tauri::AppHandle, ratio: f64) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    window.set_zoom(ratio).map_err(|e| e.to_string())
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

/// Reload the application
#[tauri::command]
pub fn reload_app(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.emit("reload-app", ()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ============================================================================
// Badge Commands
// ============================================================================

/// Set badge count on the dock/taskbar icon
#[tauri::command]
pub fn set_badge_count(app: tauri::AppHandle, count: i32) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        // Use NSApplication dock badge
        use std::process::Command;
        let badge_text = if count > 0 {
            if count > 9 {
                "9+".to_string()
            } else {
                count.to_string()
            }
        } else {
            String::new()
        };

        let script = format!(
            r#"
            tell application "System Events"
                tell dock preferences
                    set badge of (application file id "app.mezon.ai") to "{}"
                end tell
            end tell
            "#,
            badge_text
        );
        let _ = Command::new("osascript").arg("-e").arg(&script).output();
    }

    // For Windows/Linux, emit event to renderer for custom badge handling
    #[cfg(not(target_os = "macos"))]
    {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("update-badge", count);
        }
    }

    Ok(())
}

/// Clear badge count
#[tauri::command]
pub fn clear_badge(app: tauri::AppHandle) -> Result<(), String> {
    set_badge_count(app, 0)
}

// ============================================================================
// Notification Commands
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct NotificationOptions {
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
    pub silent: Option<bool>,
    pub data: Option<serde_json::Value>,
}

/// Show a system notification
#[tauri::command]
pub async fn show_notification(
    app: tauri::AppHandle,
    options: NotificationOptions,
) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;

    let mut notification = app.notification().builder();
    notification = notification.title(&options.title).body(&options.body);

    if let Some(silent) = options.silent {
        notification = notification.silent(silent);
    }

    notification.show().map_err(|e| e.to_string())?;

    // Store notification data for click handling
    if let Some(data) = options.data {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.emit("notification-data", data);
        }
    }

    Ok(())
}

// ============================================================================
// File Commands
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct DownloadFileOptions {
    pub url: String,
    pub filename: Option<String>,
}

/// Download a file with save dialog
#[tauri::command]
pub async fn download_file(
    app: tauri::AppHandle,
    options: DownloadFileOptions,
) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;
    use tauri_plugin_http::reqwest;

    let filename = options.filename.unwrap_or_else(|| {
        options
            .url
            .split('/')
            .last()
            .unwrap_or("download")
            .split('?')
            .next()
            .unwrap_or("download")
            .to_string()
    });

    // Show save dialog
    let file_path = app
        .dialog()
        .file()
        .set_file_name(&filename)
        .save_file()
        .ok_or("Save dialog cancelled")?;

    // Download file
    let response = reqwest::get(&options.url)
        .await
        .map_err(|e| e.to_string())?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;

    // Write to file
    std::fs::write(&file_path, &bytes).map_err(|e| e.to_string())?;

    Ok(file_path.to_string_lossy().to_string())
}

/// Show save file dialog
#[tauri::command]
pub async fn save_file_dialog(
    app: tauri::AppHandle,
    default_name: Option<String>,
    filters: Option<Vec<(String, Vec<String>)>>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let mut dialog = app.dialog().file();

    if let Some(name) = default_name {
        dialog = dialog.set_file_name(&name);
    }

    if let Some(filters) = filters {
        for (name, extensions) in filters {
            let ext_refs: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
            dialog = dialog.add_filter(&name, &ext_refs);
        }
    }

    let path = dialog.save_file();
    Ok(path.map(|p| p.to_string_lossy().to_string()))
}

// ============================================================================
// Clipboard Commands
// ============================================================================

/// Copy text to clipboard
#[tauri::command]
pub async fn copy_to_clipboard(app: tauri::AppHandle, text: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard()
        .write_text(text)
        .map_err(|e| e.to_string())
}

/// Copy image to clipboard from URL
#[tauri::command]
pub async fn copy_image_to_clipboard(
    app: tauri::AppHandle,
    url: String,
) -> Result<(), String> {
    use tauri_plugin_http::reqwest;

    // Download image
    let response = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;

    // Check size limit (50MB)
    if bytes.len() > 50 * 1024 * 1024 {
        return Err("Image too large (max 50MB)".to_string());
    }

    // Emit event for frontend to handle platform-specific clipboard
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("copy-image-data", base64::encode(&bytes));
    }

    Ok(())
}

/// Read text from clipboard
#[tauri::command]
pub async fn read_clipboard(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().read_text().map_err(|e| e.to_string())
}

// ============================================================================
// Image Window Commands
// ============================================================================

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ImageWindowOptions {
    pub url: String,
    pub filename: Option<String>,
    pub sender_name: Option<String>,
    pub timestamp: Option<String>,
    pub attachments: Option<Vec<AttachmentInfo>>,
    pub current_index: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AttachmentInfo {
    pub url: String,
    pub filename: Option<String>,
    pub filetype: Option<String>,
    pub size: Option<u64>,
}

/// Open image viewer window
#[tauri::command]
pub async fn open_image_window(
    app: tauri::AppHandle,
    options: ImageWindowOptions,
) -> Result<(), String> {
    crate::window::create_image_window(&app, options).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageAction {
    CopyLink,
    CopyImage,
    OpenLink,
    SaveImage,
}

/// Handle image actions (copy, save, open)
#[tauri::command]
pub async fn handle_action_show_image(
    app: tauri::AppHandle,
    action: ImageAction,
    url: String,
) -> Result<(), String> {
    match action {
        ImageAction::CopyLink => {
            copy_to_clipboard(app, url).await?;
        }
        ImageAction::CopyImage => {
            copy_image_to_clipboard(app, url).await?;
        }
        ImageAction::OpenLink => {
            use tauri_plugin_shell::ShellExt;
            app.shell().open(&url, None).map_err(|e| e.to_string())?;
        }
        ImageAction::SaveImage => {
            download_file(
                app,
                DownloadFileOptions {
                    url,
                    filename: None,
                },
            )
            .await?;
        }
    }
    Ok(())
}

// ============================================================================
// Update Commands
// ============================================================================

#[derive(Debug, Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
}

/// Check for updates
#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> Result<UpdateInfo, String> {
    use tauri_plugin_updater::UpdaterExt;

    match app.updater().map_err(|e| e.to_string())?.check().await {
        Ok(Some(update)) => Ok(UpdateInfo {
            available: true,
            version: Some(update.version.clone()),
            notes: update.body.clone(),
        }),
        Ok(None) => Ok(UpdateInfo {
            available: false,
            version: None,
            notes: None,
        }),
        Err(e) => Err(e.to_string()),
    }
}

/// Install pending update
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;

    if let Ok(Some(update)) = app.updater().map_err(|e| e.to_string())?.check().await {
        let app_clone = app.clone();
        update
            .download_and_install(
                |chunk_length, content_length| {
                    log::info!("Downloaded {} of {:?}", chunk_length, content_length);
                    if let Some(window) = app_clone.get_webview_window("main") {
                        let _ = window.emit(
                            "update-progress",
                            serde_json::json!({
                                "downloaded": chunk_length,
                                "total": content_length
                            }),
                        );
                    }
                },
                || {
                    log::info!("Download complete, restarting...");
                },
            )
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ============================================================================
// Permission Commands (macOS)
// ============================================================================

/// Request microphone permission (macOS)
#[tauri::command]
pub async fn request_permission_microphone() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        // On macOS, permissions are handled by the system when the app first tries to use the mic
        // Return true to indicate permission should be requested
        Ok(true)
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(true) // Non-macOS platforms don't need explicit permission
    }
}

/// Request camera permission (macOS)
#[tauri::command]
pub async fn request_permission_camera() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        Ok(true)
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(true)
    }
}

/// Check microphone permission status (macOS)
#[tauri::command]
pub async fn check_permission_microphone() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        // Would need to use AVCaptureDevice.authorizationStatus
        Ok("granted".to_string())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok("granted".to_string())
    }
}

/// Check camera permission status (macOS)
#[tauri::command]
pub async fn check_permission_camera() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        Ok("granted".to_string())
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok("granted".to_string())
    }
}

// ============================================================================
// Screen Capture Commands
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ScreenSource {
    pub id: String,
    pub name: String,
    pub thumbnail: Option<String>, // Base64 encoded
    pub source_type: String,       // "screen" or "window"
}

/// Get available screen sources for screen sharing
#[tauri::command]
pub async fn get_screen_sources(
    _source_type: Option<String>,
) -> Result<Vec<ScreenSource>, String> {
    // Screen capture in Tauri requires platform-specific implementation
    // or using WebRTC's getDisplayMedia which is handled by the browser
    // This is a placeholder - actual implementation would need platform-specific code
    Ok(vec![])
}

// ============================================================================
// Activity Tracking Commands
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ActiveWindowInfo {
    pub title: String,
    pub owner: String,
    pub path: Option<String>,
}

/// Enable/disable activity tracking
#[tauri::command]
pub fn update_activity_tracking(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("activity-tracking-updated", enabled);
    }
    Ok(())
}

/// Get the currently active window
#[tauri::command]
pub async fn get_active_window() -> Result<Option<ActiveWindowInfo>, String> {
    // Platform-specific implementation would go here
    // Similar to Electron's mezon-active-windows package
    Ok(None)
}

// Helper for base64 encoding
mod base64 {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(data: &[u8]) -> String {
        let mut result = String::new();
        let chunks = data.chunks(3);

        for chunk in chunks {
            let b0 = chunk[0] as usize;
            let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
            let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

            result.push(CHARS[b0 >> 2] as char);
            result.push(CHARS[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

            if chunk.len() > 1 {
                result.push(CHARS[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
            } else {
                result.push('=');
            }

            if chunk.len() > 2 {
                result.push(CHARS[b2 & 0x3f] as char);
            } else {
                result.push('=');
            }
        }

        result
    }
}
