//! Active window detection module.
//! Provides cross-platform functionality to get the currently focused window,
//! similar to the `mezon-active-windows` npm package used in Electron.

use serde::{Deserialize, Serialize};

/// Information about the currently active window
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveWindowInfo {
    /// The window class or application name (e.g., "Code", "Spotify", "chrome")
    pub window_class: String,
    /// The window title
    pub window_name: String,
    /// The executable path (if available)
    pub path: Option<String>,
    /// Process ID
    pub pid: Option<u32>,
}

/// Get the currently active/focused window
pub fn get_active_window() -> Option<ActiveWindowInfo> {
    #[cfg(target_os = "windows")]
    {
        windows_impl::get_active_window()
    }

    #[cfg(target_os = "macos")]
    {
        macos_impl::get_active_window()
    }

    #[cfg(target_os = "linux")]
    {
        linux_impl::get_active_window()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

// ============================================================================
// Windows Implementation
// ============================================================================

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId,
    };

    pub fn get_active_window() -> Option<ActiveWindowInfo> {
        unsafe {
            // Get the foreground window handle
            let hwnd = GetForegroundWindow();
            if hwnd.0 == std::ptr::null_mut() {
                return None;
            }

            // Get window title
            let title_len = GetWindowTextLengthW(hwnd);
            let window_name = if title_len > 0 {
                let mut title_buf: Vec<u16> = vec![0; (title_len + 1) as usize];
                GetWindowTextW(hwnd, &mut title_buf);
                String::from_utf16_lossy(&title_buf[..title_len as usize])
            } else {
                String::new()
            };

            // Get process ID
            let mut process_id: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut process_id));

            // Get process name and path
            let (window_class, path) = get_process_info(process_id);

            Some(ActiveWindowInfo {
                window_class,
                window_name,
                path,
                pid: Some(process_id),
            })
        }
    }

    unsafe fn get_process_info(process_id: u32) -> (String, Option<String>) {
        let handle = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            process_id,
        );

        if let Ok(handle) = handle {
            let mut name_buf: [u16; 260] = [0; 260];
            let len = GetModuleBaseNameW(handle, None, &mut name_buf);

            if len > 0 {
                let name = OsString::from_wide(&name_buf[..len as usize])
                    .to_string_lossy()
                    .to_string();

                // Clean up the name (remove .exe extension)
                let clean_name = name
                    .strip_suffix(".exe")
                    .or_else(|| name.strip_suffix(".EXE"))
                    .unwrap_or(&name)
                    .to_string();

                let _ = windows::Win32::Foundation::CloseHandle(handle);

                return (clean_name, Some(name));
            }

            let _ = windows::Win32::Foundation::CloseHandle(handle);
        }

        ("Unknown".to_string(), None)
    }
}

// ============================================================================
// macOS Implementation
// ============================================================================

#[cfg(target_os = "macos")]
mod macos_impl {
    use super::*;
    use cocoa::appkit::NSRunningApplication;
    use cocoa::base::{id, nil};
    use cocoa::foundation::NSString;
    use objc::{class, msg_send, sel, sel_impl};

    pub fn get_active_window() -> Option<ActiveWindowInfo> {
        unsafe {
            // Get the shared workspace
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            if workspace == nil {
                return None;
            }

            // Get the frontmost application
            let frontmost_app: id = msg_send![workspace, frontmostApplication];
            if frontmost_app == nil {
                return None;
            }

            // Get the application name
            let app_name: id = msg_send![frontmost_app, localizedName];
            let window_class = if app_name != nil {
                nsstring_to_string(app_name)
            } else {
                String::new()
            };

            // Get the bundle identifier as path
            let bundle_id: id = msg_send![frontmost_app, bundleIdentifier];
            let path = if bundle_id != nil {
                Some(nsstring_to_string(bundle_id))
            } else {
                None
            };

            // Get the process ID
            let pid: i32 = msg_send![frontmost_app, processIdentifier];

            // Try to get the window title using Accessibility API or CGWindow
            let window_name = get_frontmost_window_title().unwrap_or_else(|| window_class.clone());

            Some(ActiveWindowInfo {
                window_class,
                window_name,
                path,
                pid: Some(pid as u32),
            })
        }
    }

    unsafe fn nsstring_to_string(nsstring: id) -> String {
        let bytes: *const std::os::raw::c_char = msg_send![nsstring, UTF8String];
        if bytes.is_null() {
            return String::new();
        }
        std::ffi::CStr::from_ptr(bytes)
            .to_string_lossy()
            .into_owned()
    }

    fn get_frontmost_window_title() -> Option<String> {
        // Use CGWindowListCopyWindowInfo to get the frontmost window title
        unsafe {
            use core_graphics::window::{
                kCGNullWindowID, kCGWindowListOptionOnScreenOnly,
                kCGWindowListExcludeDesktopElements, CGWindowListCopyWindowInfo,
            };
            use core_foundation::array::CFArray;
            use core_foundation::base::{CFRelease, TCFType, FromVoid};
            use core_foundation::dictionary::CFDictionary;
            use core_foundation::number::CFNumber;
            use core_foundation::string::CFString;
            use std::ffi::c_void;

            let options = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
            let window_list = CGWindowListCopyWindowInfo(options, kCGNullWindowID);

            if window_list.is_null() {
                return None;
            }

            let array = CFArray::<*const c_void>::wrap_under_get_rule(window_list as _);

            // Get the first window (frontmost)
            for i in 0..array.len() {
                let dict_ptr = array.get(i as isize);
                if let Some(ptr) = dict_ptr {
                    let dict = CFDictionary::<CFString, *const c_void>::wrap_under_get_rule(
                        *ptr as core_foundation::dictionary::CFDictionaryRef
                    );

                    // Check window layer (only layer 0 are normal windows)
                    let layer_key = CFString::new("kCGWindowLayer");
                    if let Some(layer_ptr) = dict.find(&layer_key) {
                        let layer = CFNumber::from_void(*layer_ptr);
                        if layer.to_i32() != Some(0) {
                            continue;
                        }
                    }

                    // Get window name
                    let name_key = CFString::new("kCGWindowName");
                    if let Some(name_ptr) = dict.find(&name_key) {
                        let name = CFString::from_void(*name_ptr);
                        let title = name.to_string();
                        if !title.is_empty() {
                            CFRelease(window_list as *const c_void);
                            return Some(title);
                        }
                    }

                    // Fallback to owner name
                    let owner_key = CFString::new("kCGWindowOwnerName");
                    if let Some(owner_ptr) = dict.find(&owner_key) {
                        let owner = CFString::from_void(*owner_ptr);
                        let title = owner.to_string();
                        if !title.is_empty() {
                            CFRelease(window_list as *const c_void);
                            return Some(title);
                        }
                    }
                }
            }

            CFRelease(window_list as *const c_void);
            None
        }
    }
}

// ============================================================================
// Linux Implementation
// ============================================================================

#[cfg(target_os = "linux")]
mod linux_impl {
    use super::*;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, Window};
    use x11rb::rust_connection::RustConnection;

    pub fn get_active_window() -> Option<ActiveWindowInfo> {
        let (conn, screen_num) = RustConnection::connect(None).ok()?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;

        // Get _NET_ACTIVE_WINDOW atom
        let atom_active = conn
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")
            .ok()?
            .reply()
            .ok()?
            .atom;

        // Get the active window
        let reply = conn
            .get_property(false, root, atom_active, AtomEnum::WINDOW, 0, 1)
            .ok()?
            .reply()
            .ok()?;

        if reply.format != 32 || reply.value.is_empty() {
            return None;
        }

        let active_window: Window = reply.value32()?.next()?;

        if active_window == 0 {
            return None;
        }

        // Get window name
        let window_name = get_window_name(&conn, active_window)?;

        // Get window class (WM_CLASS)
        let window_class = get_window_class(&conn, active_window).unwrap_or_else(|| "Unknown".to_string());

        // Get process ID
        let pid = get_window_pid(&conn, active_window);

        Some(ActiveWindowInfo {
            window_class,
            window_name,
            path: None,
            pid,
        })
    }

    fn get_window_name(conn: &RustConnection, window: Window) -> Option<String> {
        // Try _NET_WM_NAME first (UTF-8)
        let atom_net_wm_name = conn
            .intern_atom(false, b"_NET_WM_NAME")
            .ok()?
            .reply()
            .ok()?
            .atom;

        let atom_utf8 = conn
            .intern_atom(false, b"UTF8_STRING")
            .ok()?
            .reply()
            .ok()?
            .atom;

        if let Ok(reply) = conn
            .get_property(false, window, atom_net_wm_name, atom_utf8, 0, 1024)
        {
            if let Ok(reply) = reply.reply() {
                if !reply.value.is_empty() {
                    return Some(String::from_utf8_lossy(&reply.value).trim_end_matches('\0').to_string());
                }
            }
        }

        // Fallback to WM_NAME
        if let Ok(reply) = conn.get_property(false, window, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 1024) {
            if let Ok(reply) = reply.reply() {
                if !reply.value.is_empty() {
                    return Some(String::from_utf8_lossy(&reply.value).trim_end_matches('\0').to_string());
                }
            }
        }

        None
    }

    fn get_window_class(conn: &RustConnection, window: Window) -> Option<String> {
        // Get WM_CLASS
        if let Ok(reply) = conn.get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 1024) {
            if let Ok(reply) = reply.reply() {
                if !reply.value.is_empty() {
                    // WM_CLASS contains two null-terminated strings:
                    // instance name and class name
                    let parts: Vec<&str> = std::str::from_utf8(&reply.value)
                        .ok()?
                        .split('\0')
                        .filter(|s| !s.is_empty())
                        .collect();

                    // Return the class name (second part) or instance name
                    if parts.len() >= 2 {
                        return Some(parts[1].to_string());
                    } else if !parts.is_empty() {
                        return Some(parts[0].to_string());
                    }
                }
            }
        }

        None
    }

    fn get_window_pid(conn: &RustConnection, window: Window) -> Option<u32> {
        let atom_pid = conn
            .intern_atom(false, b"_NET_WM_PID")
            .ok()?
            .reply()
            .ok()?
            .atom;

        let reply = conn
            .get_property(false, window, atom_pid, AtomEnum::CARDINAL, 0, 1)
            .ok()?
            .reply()
            .ok()?;

        if reply.format == 32 {
            reply.value32()?.next()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_active_window() {
        // This test will only pass when running with a display
        if let Some(info) = get_active_window() {
            println!("Active window: {:?}", info);
            assert!(!info.window_class.is_empty() || !info.window_name.is_empty());
        }
    }
}
