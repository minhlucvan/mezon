use crate::commands::ImageWindowOptions;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// Create an image viewer window
pub async fn create_image_window(
    app: &tauri::AppHandle,
    options: ImageWindowOptions,
) -> Result<(), String> {
    let window_label = format!("image-viewer-{}", uuid::Uuid::new_v4());

    // Create the image viewer window
    let _window = WebviewWindowBuilder::new(
        app,
        &window_label,
        WebviewUrl::App("image-viewer.html".into()),
    )
    .title(options.filename.as_deref().unwrap_or("Image Viewer"))
    .inner_size(1200.0, 800.0)
    .min_inner_size(600.0, 400.0)
    .center()
    .decorations(false)
    .transparent(true)
    .visible(true)
    .build()
    .map_err(|e| e.to_string())?;

    // Send image data to the window
    // The frontend will handle rendering
    if let Some(window) = app.get_webview_window(&window_label) {
        let _ = window.emit("image-data", &options);
    }

    Ok(())
}

/// Create a popup window for external content
pub fn create_popup_window(
    app: &tauri::AppHandle,
    url: &str,
    title: &str,
) -> Result<(), String> {
    let window_label = format!("popup-{}", uuid::Uuid::new_v4());

    // Get main window dimensions
    let main_window = app
        .get_webview_window("main")
        .ok_or("Main window not found")?;
    let monitor = main_window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No monitor found")?;
    let screen_size = monitor.size();

    let width = screen_size.width as f64 * 0.5;
    let height = screen_size.height as f64;

    let _window = WebviewWindowBuilder::new(app, &window_label, WebviewUrl::External(url.parse().unwrap()))
        .title(title)
        .inner_size(width, height)
        .position(screen_size.width as f64 - width, 0.0)
        .decorations(false)
        .transparent(true)
        .visible(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}
