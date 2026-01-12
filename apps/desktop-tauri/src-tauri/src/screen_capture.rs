//! Screen capture module for getting screen and window sources with thumbnails.
//! Provides cross-platform functionality similar to Electron's desktopCapturer.

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;

/// Represents a capturable screen or window source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSource {
    /// Unique identifier (e.g., "screen:0" or "window:12345")
    pub id: String,
    /// Display name of the source
    pub name: String,
    /// Base64 encoded PNG thumbnail (data URL format)
    pub thumbnail: String,
    /// Base64 encoded icon (optional, for windows)
    pub icon: String,
    /// Source type: "screen" or "window"
    pub source_type: String,
}

/// Response for get_screen_sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSourcesResponse {
    pub sources: Vec<ScreenSource>,
    pub total: usize,
    pub has_more: bool,
}

/// Response for load_more_screen_sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadMoreResponse {
    pub sources: Vec<ScreenSource>,
    pub has_more: bool,
}

/// Thumbnail dimensions
#[derive(Debug, Clone, Copy)]
pub struct ThumbnailSize {
    pub width: u32,
    pub height: u32,
}

impl ThumbnailSize {
    pub fn for_screen() -> Self {
        Self {
            width: 272,
            height: 136,
        }
    }

    pub fn for_window() -> Self {
        Self {
            width: 150,
            height: 90,
        }
    }
}

/// Cache for screen sources to avoid re-capturing on pagination
static SOURCES_CACHE: Lazy<RwLock<HashMap<String, Vec<ScreenSource>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

const INITIAL_BATCH_SIZE: usize = 12;
const LOAD_MORE_BATCH_SIZE: usize = 8;

/// Get screen sources with pagination
pub fn get_screen_sources(source_type: &str) -> Result<ScreenSourcesResponse, String> {
    let cache_key = source_type.to_string();

    // Check cache first
    {
        let cache = SOURCES_CACHE.read();
        if let Some(cached) = cache.get(&cache_key) {
            return Ok(ScreenSourcesResponse {
                sources: cached.iter().take(INITIAL_BATCH_SIZE).cloned().collect(),
                total: cached.len(),
                has_more: cached.len() > INITIAL_BATCH_SIZE,
            });
        }
    }

    // Capture new sources
    let sources = match source_type {
        "screen" => capture_screens()?,
        "window" => capture_windows()?,
        _ => return Err(format!("Unknown source type: {}", source_type)),
    };

    // Store in cache
    {
        let mut cache = SOURCES_CACHE.write();
        cache.insert(cache_key, sources.clone());
    }

    Ok(ScreenSourcesResponse {
        sources: sources.iter().take(INITIAL_BATCH_SIZE).cloned().collect(),
        total: sources.len(),
        has_more: sources.len() > INITIAL_BATCH_SIZE,
    })
}

/// Load more sources from cache with pagination
pub fn load_more_screen_sources(source_type: &str, offset: usize) -> Result<LoadMoreResponse, String> {
    let cache_key = source_type.to_string();

    let cache = SOURCES_CACHE.read();
    if let Some(cached) = cache.get(&cache_key) {
        let next_batch: Vec<_> = cached
            .iter()
            .skip(offset)
            .take(LOAD_MORE_BATCH_SIZE)
            .cloned()
            .collect();
        let has_more = offset + next_batch.len() < cached.len();

        Ok(LoadMoreResponse {
            sources: next_batch,
            has_more,
        })
    } else {
        Ok(LoadMoreResponse {
            sources: vec![],
            has_more: false,
        })
    }
}

/// Clear the sources cache
pub fn clear_screen_sources_cache(source_type: Option<&str>) {
    let mut cache = SOURCES_CACHE.write();
    if let Some(st) = source_type {
        cache.remove(st);
    } else {
        cache.clear();
    }
}

/// Convert raw image bytes to base64 data URL
fn image_to_data_url(data: &[u8], width: u32, height: u32, thumbnail_size: ThumbnailSize) -> String {
    use image::{ImageBuffer, Rgba, imageops::FilterType};

    // Create image from raw RGBA data
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = match ImageBuffer::from_raw(width, height, data.to_vec()) {
        Some(img) => img,
        None => return String::new(),
    };

    // Resize to thumbnail
    let resized = image::imageops::resize(
        &img,
        thumbnail_size.width,
        thumbnail_size.height,
        FilterType::Triangle,
    );

    // Encode to PNG
    let mut png_data = Cursor::new(Vec::new());
    if resized
        .write_to(&mut png_data, image::ImageFormat::Png)
        .is_err()
    {
        return String::new();
    }

    // Convert to base64 data URL
    let base64_data = base64_encode(png_data.get_ref());
    format!("data:image/png;base64,{}", base64_data)
}

/// Simple base64 encoder
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
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

// ============================================================================
// Platform-specific implementations
// ============================================================================

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, TRUE};
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
        GetDC, GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
        BI_RGB, DIB_RGB_COLORS, SRCCOPY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible, GetDesktopWindow, GetSystemMetrics,
        SM_CXSCREEN, SM_CYSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
        SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    };

    pub fn capture_screens() -> Result<Vec<ScreenSource>, String> {
        let mut sources = Vec::new();
        let thumbnail_size = ThumbnailSize::for_screen();

        unsafe {
            // Get virtual screen dimensions (all monitors combined)
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let width = GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32;
            let height = GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32;

            if width == 0 || height == 0 {
                // Fallback to primary screen
                let width = GetSystemMetrics(SM_CXSCREEN) as u32;
                let height = GetSystemMetrics(SM_CYSCREEN) as u32;

                if let Some(thumbnail) = capture_screen_area(0, 0, width, height, thumbnail_size) {
                    sources.push(ScreenSource {
                        id: "screen:0:0".to_string(),
                        name: "Entire Screen".to_string(),
                        thumbnail,
                        icon: String::new(),
                        source_type: "screen".to_string(),
                    });
                }
            } else {
                // Capture entire virtual screen
                if let Some(thumbnail) = capture_screen_area(x, y, width, height, thumbnail_size) {
                    sources.push(ScreenSource {
                        id: format!("screen:{}:{}", x, y),
                        name: "Entire Screen".to_string(),
                        thumbnail,
                        icon: String::new(),
                        source_type: "screen".to_string(),
                    });
                }
            }
        }

        Ok(sources)
    }

    unsafe fn capture_screen_area(
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        thumbnail_size: ThumbnailSize,
    ) -> Option<String> {
        let desktop_hwnd = GetDesktopWindow();
        let hdc_screen = GetDC(desktop_hwnd);
        if hdc_screen.is_invalid() {
            return None;
        }

        let hdc_mem = CreateCompatibleDC(hdc_screen);
        if hdc_mem.is_invalid() {
            ReleaseDC(desktop_hwnd, hdc_screen);
            return None;
        }

        let hbm = CreateCompatibleBitmap(hdc_screen, width as i32, height as i32);
        if hbm.is_invalid() {
            DeleteDC(hdc_mem);
            ReleaseDC(desktop_hwnd, hdc_screen);
            return None;
        }

        let old_obj = SelectObject(hdc_mem, hbm);

        // Copy screen content
        let _ = BitBlt(hdc_mem, 0, 0, width as i32, height as i32, hdc_screen, x, y, SRCCOPY);

        // Get bitmap data
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32), // Top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default()],
        };

        let mut pixels: Vec<u8> = vec![0; (width * height * 4) as usize];

        let result = GetDIBits(
            hdc_mem,
            hbm,
            0,
            height,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Cleanup
        SelectObject(hdc_mem, old_obj);
        DeleteObject(hbm);
        DeleteDC(hdc_mem);
        ReleaseDC(desktop_hwnd, hdc_screen);

        if result == 0 {
            return None;
        }

        // Convert BGRA to RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        Some(image_to_data_url(&pixels, width, height, thumbnail_size))
    }

    pub fn capture_windows() -> Result<Vec<ScreenSource>, String> {
        let mut sources: Vec<ScreenSource> = Vec::new();
        let thumbnail_size = ThumbnailSize::for_window();

        unsafe {
            let sources_ptr = &mut sources as *mut Vec<ScreenSource>;
            let _ = EnumWindows(
                Some(enum_windows_callback),
                LPARAM(sources_ptr as isize),
            );
        }

        Ok(sources)
    }

    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let sources = &mut *(lparam.0 as *mut Vec<ScreenSource>);
        let thumbnail_size = ThumbnailSize::for_window();

        // Skip invisible windows
        if IsWindowVisible(hwnd).as_bool() == false {
            return TRUE;
        }

        // Get window title
        let title_len = GetWindowTextLengthW(hwnd);
        if title_len == 0 {
            return TRUE;
        }

        let mut title_buf: Vec<u16> = vec![0; (title_len + 1) as usize];
        GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

        // Skip empty titles
        if title.trim().is_empty() {
            return TRUE;
        }

        // Get window rect
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return TRUE;
        }

        let width = (rect.right - rect.left) as u32;
        let height = (rect.bottom - rect.top) as u32;

        // Skip too small windows
        if width < 100 || height < 50 {
            return TRUE;
        }

        // Get class name to filter out certain windows
        let mut class_buf: [u16; 256] = [0; 256];
        let class_len = GetClassNameW(hwnd, &mut class_buf);
        let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);

        // Skip system windows
        let skip_classes = [
            "Progman",
            "Windows.UI.Core.CoreWindow",
            "Shell_TrayWnd",
            "Shell_SecondaryTrayWnd",
            "DV2ControlHost",
            "MsgrIMEWindowClass",
            "SysShadow",
            "Button",
        ];

        if skip_classes.iter().any(|&c| class_name == c) {
            return TRUE;
        }

        // Get process ID for unique identification
        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        // Capture window thumbnail
        if let Some(thumbnail) = capture_window_thumbnail(hwnd, width, height, thumbnail_size) {
            sources.push(ScreenSource {
                id: format!("window:{}:{}", hwnd.0 as usize, process_id),
                name: title,
                thumbnail,
                icon: String::new(),
                source_type: "window".to_string(),
            });
        }

        TRUE
    }

    unsafe fn capture_window_thumbnail(
        hwnd: HWND,
        width: u32,
        height: u32,
        thumbnail_size: ThumbnailSize,
    ) -> Option<String> {
        let hdc_window = GetDC(hwnd);
        if hdc_window.is_invalid() {
            return None;
        }

        let hdc_mem = CreateCompatibleDC(hdc_window);
        if hdc_mem.is_invalid() {
            ReleaseDC(hwnd, hdc_window);
            return None;
        }

        let hbm = CreateCompatibleBitmap(hdc_window, width as i32, height as i32);
        if hbm.is_invalid() {
            DeleteDC(hdc_mem);
            ReleaseDC(hwnd, hdc_window);
            return None;
        }

        let old_obj = SelectObject(hdc_mem, hbm);

        // Try to capture window content
        let _ = BitBlt(
            hdc_mem,
            0,
            0,
            width as i32,
            height as i32,
            hdc_window,
            0,
            0,
            SRCCOPY,
        );

        // Get bitmap data
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default()],
        };

        let mut pixels: Vec<u8> = vec![0; (width * height * 4) as usize];

        let result = GetDIBits(
            hdc_mem,
            hbm,
            0,
            height,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        // Cleanup
        SelectObject(hdc_mem, old_obj);
        DeleteObject(hbm);
        DeleteDC(hdc_mem);
        ReleaseDC(hwnd, hdc_window);

        if result == 0 {
            return None;
        }

        // Convert BGRA to RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        Some(image_to_data_url(&pixels, width, height, thumbnail_size))
    }
}

#[cfg(target_os = "macos")]
mod macos_impl {
    use super::*;
    use core_foundation::base::{CFRelease, TCFType};
    use core_foundation::dictionary::CFDictionaryRef;
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_graphics::display::{
        CGDisplay, CGMainDisplayID, CGGetActiveDisplayList,
    };
    use core_graphics::geometry::{CGRect, CGSize};
    use core_graphics::image::CGImage;
    use core_graphics::window::{
        CGWindowListCopyWindowInfo, CGWindowListCreateImage,
        kCGNullWindowID, kCGWindowListOptionOnScreenOnly,
        kCGWindowListExcludeDesktopElements, kCGWindowImageDefault,
    };
    use std::ffi::c_void;

    // CFDictionary key constants
    const K_CG_WINDOW_NUMBER: &str = "kCGWindowNumber";
    const K_CG_WINDOW_NAME: &str = "kCGWindowName";
    const K_CG_WINDOW_OWNER_NAME: &str = "kCGWindowOwnerName";
    const K_CG_WINDOW_BOUNDS: &str = "kCGWindowBounds";
    const K_CG_WINDOW_LAYER: &str = "kCGWindowLayer";

    pub fn capture_screens() -> Result<Vec<ScreenSource>, String> {
        let mut sources = Vec::new();
        let thumbnail_size = ThumbnailSize::for_screen();

        unsafe {
            // Get number of displays
            let mut display_count: u32 = 0;
            CGGetActiveDisplayList(0, std::ptr::null_mut(), &mut display_count);

            if display_count == 0 {
                return Err("No displays found".to_string());
            }

            let mut displays: Vec<u32> = vec![0; display_count as usize];
            CGGetActiveDisplayList(display_count, displays.as_mut_ptr(), &mut display_count);

            for (index, &display_id) in displays.iter().enumerate() {
                let display = CGDisplay::new(display_id);
                let bounds = display.bounds();

                // Capture display
                let image = CGDisplay::screenshot(
                    bounds,
                    kCGWindowListOptionOnScreenOnly,
                    kCGNullWindowID,
                    kCGWindowImageDefault,
                );

                if let Some(img) = image {
                    if let Some(thumbnail) = cgimage_to_data_url(&img, thumbnail_size) {
                        let name = if index == 0 {
                            "Entire Screen".to_string()
                        } else {
                            format!("Screen {}", index + 1)
                        };

                        sources.push(ScreenSource {
                            id: format!("screen:{}", display_id),
                            name,
                            thumbnail,
                            icon: String::new(),
                            source_type: "screen".to_string(),
                        });
                    }
                }
            }
        }

        Ok(sources)
    }

    pub fn capture_windows() -> Result<Vec<ScreenSource>, String> {
        let mut sources = Vec::new();
        let thumbnail_size = ThumbnailSize::for_window();

        unsafe {
            let options = kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements;
            let window_list = CGWindowListCopyWindowInfo(options, kCGNullWindowID);

            if window_list.is_null() {
                return Ok(sources);
            }

            let count = core_foundation::array::CFArray::<*const c_void>::wrap_under_get_rule(
                window_list as _
            ).len();

            for i in 0..count {
                let window_info = core_foundation::array::CFArray::<CFDictionaryRef>::wrap_under_get_rule(
                    window_list as _
                ).get(i as isize);

                if let Some(info) = window_info {
                    if let Some(source) = process_window_info(*info, thumbnail_size) {
                        sources.push(source);
                    }
                }
            }

            CFRelease(window_list as *const c_void);
        }

        Ok(sources)
    }

    unsafe fn process_window_info(
        info: CFDictionaryRef,
        thumbnail_size: ThumbnailSize,
    ) -> Option<ScreenSource> {
        use core_foundation::dictionary::CFDictionary;
        use core_foundation::base::FromVoid;

        let dict = CFDictionary::<CFString, *const c_void>::wrap_under_get_rule(info);

        // Get window layer - skip windows not on layer 0 (normal windows)
        let layer_key = CFString::new(K_CG_WINDOW_LAYER);
        if let Some(layer_ptr) = dict.find(&layer_key) {
            let layer = CFNumber::from_void(*layer_ptr);
            if layer.to_i32() != Some(0) {
                return None;
            }
        }

        // Get window ID
        let window_id_key = CFString::new(K_CG_WINDOW_NUMBER);
        let window_id: u32 = dict
            .find(&window_id_key)
            .and_then(|ptr| CFNumber::from_void(*ptr).to_i32())
            .unwrap_or(0) as u32;

        if window_id == 0 {
            return None;
        }

        // Get window name
        let name_key = CFString::new(K_CG_WINDOW_NAME);
        let window_name = dict
            .find(&name_key)
            .map(|ptr| CFString::from_void(*ptr).to_string())
            .unwrap_or_default();

        // Get owner name as fallback
        let owner_key = CFString::new(K_CG_WINDOW_OWNER_NAME);
        let owner_name = dict
            .find(&owner_key)
            .map(|ptr| CFString::from_void(*ptr).to_string())
            .unwrap_or_default();

        let display_name = if window_name.is_empty() {
            owner_name.clone()
        } else {
            format!("{} - {}", owner_name, window_name)
        };

        // Skip if no name
        if display_name.trim().is_empty() {
            return None;
        }

        // Capture window thumbnail
        let image = CGWindowListCreateImage(
            CGRect::new(
                &core_graphics::geometry::CGPoint::new(0.0, 0.0),
                &CGSize::new(0.0, 0.0),
            ),
            kCGWindowListOptionOnScreenOnly,
            window_id,
            kCGWindowImageDefault,
        );

        let thumbnail = if !image.is_null() {
            let img = CGImage::from_ptr(image);
            cgimage_to_data_url(&img, thumbnail_size).unwrap_or_default()
        } else {
            String::new()
        };

        Some(ScreenSource {
            id: format!("window:{}", window_id),
            name: display_name,
            thumbnail,
            icon: String::new(),
            source_type: "window".to_string(),
        })
    }

    fn cgimage_to_data_url(image: &CGImage, thumbnail_size: ThumbnailSize) -> Option<String> {
        let width = image.width() as u32;
        let height = image.height() as u32;
        let bytes_per_row = image.bytes_per_row();
        let data = image.data();

        if width == 0 || height == 0 {
            return None;
        }

        // Get raw bytes
        let bytes = data.bytes();

        // Convert to RGBA if needed (CGImage might be BGRA or other formats)
        let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height {
            for x in 0..width {
                let offset = (y as usize * bytes_per_row) + (x as usize * 4);
                if offset + 3 < bytes.len() {
                    // Assuming BGRA format
                    rgba_data.push(bytes[offset + 2]); // R
                    rgba_data.push(bytes[offset + 1]); // G
                    rgba_data.push(bytes[offset]);     // B
                    rgba_data.push(bytes[offset + 3]); // A
                }
            }
        }

        Some(super::image_to_data_url(&rgba_data, width, height, thumbnail_size))
    }
}

#[cfg(target_os = "linux")]
mod linux_impl {
    use super::*;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{
        ConnectionExt, Window, AtomEnum,
    };
    use x11rb::rust_connection::RustConnection;

    pub fn capture_screens() -> Result<Vec<ScreenSource>, String> {
        let mut sources = Vec::new();
        let thumbnail_size = ThumbnailSize::for_screen();

        let (conn, screen_num) = RustConnection::connect(None)
            .map_err(|e| format!("Failed to connect to X11: {}", e))?;

        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;
        let width = screen.width_in_pixels as u32;
        let height = screen.height_in_pixels as u32;

        // Capture root window (entire screen)
        if let Some(thumbnail) = capture_x11_window(&conn, root, width, height, thumbnail_size) {
            sources.push(ScreenSource {
                id: format!("screen:{}", screen_num),
                name: "Entire Screen".to_string(),
                thumbnail,
                icon: String::new(),
                source_type: "screen".to_string(),
            });
        }

        Ok(sources)
    }

    pub fn capture_windows() -> Result<Vec<ScreenSource>, String> {
        let mut sources = Vec::new();
        let thumbnail_size = ThumbnailSize::for_window();

        let (conn, screen_num) = RustConnection::connect(None)
            .map_err(|e| format!("Failed to connect to X11: {}", e))?;

        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;

        // Get _NET_CLIENT_LIST to get managed windows
        let atom_client_list = conn
            .intern_atom(false, b"_NET_CLIENT_LIST")
            .map_err(|e| format!("Failed to intern atom: {}", e))?
            .reply()
            .map_err(|e| format!("Failed to get atom reply: {}", e))?
            .atom;

        let reply = conn
            .get_property(false, root, atom_client_list, AtomEnum::WINDOW, 0, u32::MAX)
            .map_err(|e| format!("Failed to get property: {}", e))?
            .reply()
            .map_err(|e| format!("Failed to get property reply: {}", e))?;

        if reply.format == 32 {
            let windows: Vec<Window> = reply
                .value32()
                .map(|iter| iter.collect())
                .unwrap_or_default();

            for window in windows {
                if let Some(source) = get_window_source(&conn, window, thumbnail_size) {
                    sources.push(source);
                }
            }
        }

        Ok(sources)
    }

    fn get_window_source(
        conn: &RustConnection,
        window: Window,
        thumbnail_size: ThumbnailSize,
    ) -> Option<ScreenSource> {
        // Get window name
        let name = get_window_name(conn, window)?;

        if name.trim().is_empty() {
            return None;
        }

        // Get window geometry
        let geom = conn.get_geometry(window).ok()?.reply().ok()?;
        let width = geom.width as u32;
        let height = geom.height as u32;

        if width < 100 || height < 50 {
            return None;
        }

        // Capture thumbnail
        let thumbnail = capture_x11_window(conn, window, width, height, thumbnail_size)
            .unwrap_or_default();

        Some(ScreenSource {
            id: format!("window:{}", window),
            name,
            thumbnail,
            icon: String::new(),
            source_type: "window".to_string(),
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

        if let Ok(reply) = conn.get_property(false, window, atom_net_wm_name, atom_utf8, 0, 1024) {
            if let Ok(reply) = reply.reply() {
                if !reply.value.is_empty() {
                    return Some(String::from_utf8_lossy(&reply.value).to_string());
                }
            }
        }

        // Fallback to WM_NAME
        if let Ok(reply) = conn.get_property(false, window, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 1024) {
            if let Ok(reply) = reply.reply() {
                if !reply.value.is_empty() {
                    return Some(String::from_utf8_lossy(&reply.value).to_string());
                }
            }
        }

        None
    }

    fn capture_x11_window(
        conn: &RustConnection,
        window: Window,
        width: u32,
        height: u32,
        thumbnail_size: ThumbnailSize,
    ) -> Option<String> {
        use x11rb::protocol::xproto::ImageFormat;

        // Get window image
        let image = conn
            .get_image(ImageFormat::Z_PIXMAP, window, 0, 0, width as u16, height as u16, !0)
            .ok()?
            .reply()
            .ok()?;

        let depth = image.depth;
        let data = image.data;

        if data.is_empty() {
            return None;
        }

        // Convert to RGBA based on depth
        let rgba_data = if depth == 24 || depth == 32 {
            // Assuming BGRA format
            let mut rgba = Vec::with_capacity((width * height * 4) as usize);
            for chunk in data.chunks(4) {
                if chunk.len() >= 3 {
                    rgba.push(chunk[2]); // R
                    rgba.push(chunk[1]); // G
                    rgba.push(chunk[0]); // B
                    rgba.push(if chunk.len() >= 4 { chunk[3] } else { 255 }); // A
                }
            }
            rgba
        } else {
            return None;
        };

        Some(super::image_to_data_url(&rgba_data, width, height, thumbnail_size))
    }
}

// ============================================================================
// Public API - Platform dispatch
// ============================================================================

#[cfg(target_os = "windows")]
fn capture_screens() -> Result<Vec<ScreenSource>, String> {
    windows_impl::capture_screens()
}

#[cfg(target_os = "macos")]
fn capture_screens() -> Result<Vec<ScreenSource>, String> {
    macos_impl::capture_screens()
}

#[cfg(target_os = "linux")]
fn capture_screens() -> Result<Vec<ScreenSource>, String> {
    linux_impl::capture_screens()
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn capture_screens() -> Result<Vec<ScreenSource>, String> {
    Err("Screen capture not supported on this platform".to_string())
}

#[cfg(target_os = "windows")]
fn capture_windows() -> Result<Vec<ScreenSource>, String> {
    windows_impl::capture_windows()
}

#[cfg(target_os = "macos")]
fn capture_windows() -> Result<Vec<ScreenSource>, String> {
    macos_impl::capture_windows()
}

#[cfg(target_os = "linux")]
fn capture_windows() -> Result<Vec<ScreenSource>, String> {
    linux_impl::capture_windows()
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn capture_windows() -> Result<Vec<ScreenSource>, String> {
    Err("Window capture not supported on this platform".to_string())
}
