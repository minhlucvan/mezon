//! Native clipboard module with image support.
//! Provides cross-platform functionality to copy images to the system clipboard,
//! similar to Electron's clipboard.writeImage().

use arboard::{Clipboard, ImageData};
use image::{DynamicImage, GenericImageView, ImageFormat};
use std::borrow::Cow;
use std::io::Cursor;

/// Maximum dimension for clipboard images (to prevent memory issues)
const MAX_DIMENSION: u32 = 4096;

/// Maximum file size for clipboard images (50MB)
const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;

/// Result type for clipboard operations
pub type ClipboardResult<T> = Result<T, String>;

/// Copy image data (raw bytes) to the system clipboard
/// Supports PNG, JPEG, GIF, WebP, and other common formats
pub fn copy_image_to_clipboard(image_data: &[u8]) -> ClipboardResult<()> {
    // Check size limit
    if image_data.len() > MAX_FILE_SIZE {
        return Err("Image too large (max 50MB)".to_string());
    }

    // Load and decode the image
    let img = image::load_from_memory(image_data)
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    // Resize if necessary
    let img = resize_if_needed(img);

    // Convert to RGBA
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    // Create arboard ImageData
    let image_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(rgba.into_raw()),
    };

    // Copy to clipboard
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;

    clipboard
        .set_image(image_data)
        .map_err(|e| format!("Failed to copy image to clipboard: {}", e))?;

    log::info!("Image copied to clipboard: {}x{}", width, height);
    Ok(())
}

/// Copy image from URL to the system clipboard
pub async fn copy_image_from_url_to_clipboard(url: &str) -> ClipboardResult<()> {
    use tauri_plugin_http::reqwest;

    // Download the image
    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Failed to download image: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download image: HTTP {}",
            response.status()
        ));
    }

    // Check content length
    if let Some(content_length) = response.content_length() {
        if content_length as usize > MAX_FILE_SIZE {
            return Err("Image too large (max 50MB)".to_string());
        }
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read image data: {}", e))?;

    copy_image_to_clipboard(&bytes)
}

/// Copy image from base64 string to the system clipboard
pub fn copy_base64_image_to_clipboard(base64_data: &str) -> ClipboardResult<()> {
    // Remove data URL prefix if present
    let base64_clean = if let Some(pos) = base64_data.find(",") {
        &base64_data[pos + 1..]
    } else {
        base64_data
    };

    // Decode base64
    let image_data = base64_decode(base64_clean)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    copy_image_to_clipboard(&image_data)
}

/// Read image from clipboard and return as PNG bytes
pub fn read_image_from_clipboard() -> ClipboardResult<Vec<u8>> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| format!("Failed to access clipboard: {}", e))?;

    let image_data = clipboard
        .get_image()
        .map_err(|e| format!("Failed to read image from clipboard: {}", e))?;

    // Convert to PNG
    let img = image::RgbaImage::from_raw(
        image_data.width as u32,
        image_data.height as u32,
        image_data.bytes.into_owned(),
    )
    .ok_or("Failed to create image from clipboard data")?;

    let mut png_data = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(img)
        .write_to(&mut png_data, ImageFormat::Png)
        .map_err(|e| format!("Failed to encode image as PNG: {}", e))?;

    Ok(png_data.into_inner())
}

/// Check if clipboard contains an image
pub fn clipboard_has_image() -> bool {
    if let Ok(mut clipboard) = Clipboard::new() {
        clipboard.get_image().is_ok()
    } else {
        false
    }
}

/// Resize image if it exceeds maximum dimensions
fn resize_if_needed(img: DynamicImage) -> DynamicImage {
    let (width, height) = img.dimensions();

    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        let scale = (MAX_DIMENSION as f32 / width as f32).min(MAX_DIMENSION as f32 / height as f32);
        let new_width = (width as f32 * scale) as u32;
        let new_height = (height as f32 * scale) as u32;

        log::info!(
            "Resizing image from {}x{} to {}x{}",
            width,
            height,
            new_width,
            new_height
        );

        img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
    } else {
        img
    }
}

/// Simple base64 decoder
fn base64_decode(input: &str) -> Result<Vec<u8>, &'static str> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    fn char_to_value(c: u8) -> Option<u8> {
        CHARS.iter().position(|&x| x == c).map(|p| p as u8)
    }

    let input = input.trim().replace(['\n', '\r', ' '], "");
    let input = input.as_bytes();

    if input.is_empty() {
        return Ok(Vec::new());
    }

    let mut output = Vec::with_capacity(input.len() * 3 / 4);

    for chunk in input.chunks(4) {
        let mut buffer = [0u8; 4];
        let mut valid_chars = 0;

        for (i, &byte) in chunk.iter().enumerate() {
            if byte == b'=' {
                break;
            }
            buffer[i] = char_to_value(byte).ok_or("Invalid base64 character")?;
            valid_chars += 1;
        }

        if valid_chars >= 2 {
            output.push((buffer[0] << 2) | (buffer[1] >> 4));
        }
        if valid_chars >= 3 {
            output.push((buffer[1] << 4) | (buffer[2] >> 2));
        }
        if valid_chars >= 4 {
            output.push((buffer[2] << 6) | buffer[3]);
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_decode() {
        let encoded = "SGVsbG8gV29ybGQ="; // "Hello World"
        let decoded = base64_decode(encoded).unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn test_base64_decode_with_data_url() {
        let data_url = "data:image/png;base64,iVBORw0KGgo=";
        let base64_part = &data_url[data_url.find(",").unwrap() + 1..];
        let result = base64_decode(base64_part);
        assert!(result.is_ok());
    }
}
