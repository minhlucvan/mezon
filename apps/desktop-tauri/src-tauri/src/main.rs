// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

mod commands;
mod window;

fn main() {
    let mut builder = tauri::Builder::default();

    // Add plugins
    builder = builder
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build());

    // Add desktop-only plugins
    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                Some(vec!["--minimized"]),
            ))
            .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
                // Focus main window when another instance is launched
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                // Handle deep link from argv if present
                if argv.len() > 1 {
                    let url = &argv[1];
                    if url.starts_with("mezonapp://") {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("deep-link", url);
                        }
                    }
                }
            }));
    }

    builder
        .invoke_handler(tauri::generate_handler![
            // App info commands
            commands::get_app_version,
            commands::get_platform,
            commands::get_device_id,
            commands::get_sender_id,
            // Window management commands
            commands::title_bar_action,
            commands::get_window_state,
            commands::set_window_title,
            commands::set_ratio_window,
            commands::minimize_to_tray,
            commands::quit_app,
            commands::reload_app,
            // Badge commands
            commands::set_badge_count,
            commands::clear_badge,
            // Notification commands
            commands::show_notification,
            // File commands
            commands::download_file,
            commands::save_file_dialog,
            // Clipboard commands
            commands::copy_to_clipboard,
            commands::copy_image_to_clipboard,
            commands::read_clipboard,
            // Image commands
            commands::open_image_window,
            commands::handle_action_show_image,
            // Update commands
            commands::check_update,
            commands::install_update,
            // Permission commands (macOS)
            commands::request_permission_microphone,
            commands::request_permission_camera,
            commands::check_permission_microphone,
            commands::check_permission_camera,
            // Screen capture commands
            commands::get_screen_sources,
            // Activity tracking commands
            commands::update_activity_tracking,
            commands::get_active_window,
        ])
        .setup(|app| {
            // Create system tray
            let quit = MenuItem::with_id(app, "quit", "Quit Mezon", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "Show Mezon", true, None::<&str>)?;
            let hide = MenuItem::with_id(app, "hide", "Hide Mezon", true, None::<&str>)?;
            let check_updates =
                MenuItem::with_id(app, "check_updates", "Check for Updates", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[&show, &hide, &check_updates, &quit])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .menu_on_left_click(false)
                .tooltip("Mezon")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    "check_updates" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("check-updates", ());
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Handle window close to minimize to tray instead
            let main_window = app.get_webview_window("main").unwrap();
            let main_window_clone = main_window.clone();
            let app_handle = app.handle().clone();

            main_window.on_window_event(move |event| match event {
                WindowEvent::CloseRequested { api, .. } => {
                    // Prevent closing, hide to tray instead
                    api.prevent_close();
                    let _ = main_window_clone.hide();
                }
                WindowEvent::Focused(focused) => {
                    let event_name = if *focused {
                        "window-focused"
                    } else {
                        "window-blurred"
                    };
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.emit(event_name, ());
                    }
                }
                _ => {}
            });

            // Handle deep link on macOS
            #[cfg(target_os = "macos")]
            {
                let handle = app.handle().clone();
                app.listen("deep-link://new-url", move |event| {
                    if let Some(window) = handle.get_webview_window("main") {
                        let _ = window.emit("deep-link", event.payload());
                    }
                });
            }

            // Setup global shortcuts
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

                let app_handle = app.handle().clone();
                let shortcut_str = if cfg!(target_os = "macos") {
                    "cmd+,"
                } else {
                    "ctrl+,"
                };

                if let Ok(shortcut) = shortcut_str.parse::<Shortcut>() {
                    let _ = app.global_shortcut().on_shortcut(
                        shortcut,
                        move |_app, _shortcut, event| {
                            if event.state == ShortcutState::Pressed {
                                if let Some(window) = app_handle.get_webview_window("main") {
                                    let _ = window.emit("trigger-shortcut", "settings");
                                }
                            }
                        },
                    );
                }
            }

            log::info!("Mezon Tauri app initialized");

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Mezon");
}
