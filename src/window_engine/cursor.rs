//! Custom rocket cursor loading and GLFW pixel conversion.

use crate::window_engine::constants::{
    ROCKET_CURSOR_HEIGHT, ROCKET_CURSOR_HOTSPOT_X, ROCKET_CURSOR_HOTSPOT_Y,
    ROCKET_CURSOR_TEXTURE_PATH, ROCKET_CURSOR_WIDTH,
};
use anyhow::{anyhow, Result};
use imgui_glfw_rs::glfw;

/// Loads and converts RGBA8 pixel data into little-endian u32 pixels for GLFW.
///
/// Returns (width, height, pixels_u32).
pub fn load_cursor_pixel_data(path: &str) -> Result<(u32, u32, Vec<u32>)> {
    let cursor_img = image::open(path)
        .map_err(|e| anyhow!("Failed to load cursor image from '{}': {}", path, e))?;
    let cursor_rgba = cursor_img.to_rgba8();
    let (width, height) = cursor_rgba.dimensions();

    let pixels: Vec<u32> = cursor_rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|rgba| u32::from_ne_bytes(*rgba))
        .collect();

    Ok((width, height, pixels))
}

/// Creates a GLFW cursor instance from the rocket cursor texture.
pub fn load_rocket_cursor() -> Result<glfw::Cursor> {
    let (width, height, pixels) = load_cursor_pixel_data(ROCKET_CURSOR_TEXTURE_PATH)?;
    if width != ROCKET_CURSOR_WIDTH || height != ROCKET_CURSOR_HEIGHT {
        return Err(anyhow!(
            "Unexpected rocket cursor dimensions: {}x{} (expected {}x{})",
            width,
            height,
            ROCKET_CURSOR_WIDTH,
            ROCKET_CURSOR_HEIGHT
        ));
    }

    let pixel_image = glfw::PixelImage {
        width,
        height,
        pixels,
    };

    Ok(glfw::Cursor::create_from_pixels(
        pixel_image,
        ROCKET_CURSOR_HOTSPOT_X,
        ROCKET_CURSOR_HOTSPOT_Y,
    ))
}
