use super::r#trait::{ImguiSystem, WindowEngine, WindowEvents};
use anyhow::{anyhow, Result};
use glfw::{Context, CursorMode, WindowMode};
use imgui::Context as ImContext;
use imgui_glfw_rs::glfw;
use imgui_glfw_rs::ImguiGLFW;
use log::{debug, info};

use crate::utils::clipboard_backend::make_clipboard_backend;
use crate::utils::Fullscreen;

pub struct GlfwWindowEngine {
    glfw: glfw::Glfw,
    window: glfw::PWindow,
    events: WindowEvents,
    imgui_system: Option<ImguiSystem>,
    cursor_pixels: Option<Vec<u32>>,
    rocket_cursor_enabled: bool,
}

impl WindowEngine for GlfwWindowEngine {
    fn init(width: i32, height: i32, title: &str) -> Result<Self> {
        let _ = env_logger::builder().is_test(true).try_init();

        let mut glfw = glfw::init(glfw::fail_on_errors)
            .map_err(|_| anyhow!("Impossible d'initialiser GLFW"))?;

        if std::env::var("FIREWORKS_HEADLESS").is_ok() || std::env::var("FIREWORKS_BENCH").is_ok() {
            glfw.window_hint(glfw::WindowHint::Visible(false));
        }

        glfw.window_hint(glfw::WindowHint::ContextVersionMajor(3));
        glfw.window_hint(glfw::WindowHint::ContextVersionMinor(3));
        glfw.window_hint(glfw::WindowHint::OpenGlProfile(
            glfw::OpenGlProfileHint::Core,
        ));

        // Empêche GLFW de crasher sous Xvfb en cas de perte de focus
        glfw.window_hint(glfw::WindowHint::AutoIconify(false));

        let (mut window, events) = glfw
            .create_window(
                width as u32,
                height as u32,
                title,
                glfw::WindowMode::Windowed,
            )
            .expect("Erreur création fenêtre GLFW");

        window.make_current();
        glfw.set_swap_interval(glfw::SwapInterval::None);
        window.set_key_polling(true);
        window.set_char_polling(true);
        window.set_framebuffer_size_polling(true);
        window.set_cursor_pos_polling(true);
        window.set_mouse_button_polling(true);
        window.set_scroll_polling(true);

        info!("✅ OpenGL context ready for '{}'", title);

        // load OpenGL function pointers
        gl::load_with(|s| window.get_proc_address(s) as *const _);

        let mut imgui = ImContext::create();
        imgui.set_ini_filename(crate::utils::config_path::get_imgui_ini_path());

        let font_data = std::fs::read(crate::utils::config_path::DEFAULT_FONT_PATH)
            .expect("Failed to read font file");
        imgui.fonts().add_font(&[imgui::FontSource::TtfData {
            data: &font_data,
            size_pixels: 18.0,
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                oversample_v: 1,
                rasterizer_multiply: 1.0,
                ..Default::default()
            }),
        }]);

        imgui.fonts().build_rgba32_texture();

        let imgui_glfw = ImguiGLFW::new(&mut imgui, &mut window);

        // Remplace le backend GLFW (cause d'un panic sur clipboard vide `xsel -bc`)
        imgui.set_clipboard_backend(make_clipboard_backend());

        let cursor_pixels = match crate::window_engine::cursor::load_cursor_pixel_data(
            crate::window_engine::constants::ROCKET_CURSOR_TEXTURE_PATH,
        ) {
            Ok((w, h, pixels)) => {
                if w == crate::window_engine::constants::ROCKET_CURSOR_WIDTH
                    && h == crate::window_engine::constants::ROCKET_CURSOR_HEIGHT
                {
                    Some(pixels)
                } else {
                    log::warn!("⚠️ Invalid rocket cursor dimensions {}x{}", w, h);
                    None
                }
            }
            Err(e) => {
                log::warn!("⚠️ Could not load rocket cursor pixels: {}", e);
                None
            }
        };

        let mut engine = Self {
            glfw,
            window,
            events,
            imgui_system: Some(ImguiSystem {
                context: imgui,
                glfw: imgui_glfw,
            }),
            cursor_pixels,
            rocket_cursor_enabled: false,
        };

        if engine.cursor_pixels.is_some() {
            engine.set_rocket_cursor(true);
        }

        Ok(engine)
    }

    fn poll_events(&mut self) {
        self.glfw.poll_events();
    }

    fn swap_buffers(&mut self) {
        self.window.swap_buffers();
    }

    fn should_close(&self) -> bool {
        self.window.should_close()
    }

    fn set_should_close(&mut self, value: bool) {
        self.window.set_should_close(value);
    }

    fn get_size(&self) -> (i32, i32) {
        self.window.get_size()
    }

    fn get_pos(&self) -> (i32, i32) {
        self.window.get_pos()
    }

    fn is_fullscreen(&self) -> bool {
        self.window.is_fullscreen()
    }

    fn set_monitor(
        &mut self,
        mode: WindowMode,
        xpos: i32,
        ypos: i32,
        width: u32,
        height: u32,
        refresh_rate: Option<u32>,
    ) {
        self.window
            .set_monitor(mode, xpos, ypos, width, height, refresh_rate);
    }

    fn set_cursor_mode(&mut self, mode: CursorMode) {
        self.window.set_cursor_mode(mode);
    }

    fn make_current(&mut self) {
        self.window.make_current();
    }

    fn get_glfw(&self) -> &glfw::Glfw {
        &self.glfw
    }

    fn get_window_mut(&mut self) -> &mut glfw::PWindow {
        &mut self.window
    }

    fn get_events(&self) -> &WindowEvents {
        &self.events
    }

    fn has_imgui(&self) -> bool {
        self.imgui_system.is_some()
    }

    fn get_imgui_system_mut(&mut self) -> &mut ImguiSystem {
        self.imgui_system
            .as_mut()
            .expect("ImguiSystem has been closed or not initialized")
    }

    fn get_window_and_imgui_mut(&mut self) -> (&mut glfw::PWindow, &mut ImguiSystem) {
        (
            &mut self.window,
            self.imgui_system
                .as_mut()
                .expect("ImguiSystem has been closed or not initialized"),
        )
    }

    fn set_rocket_cursor(&mut self, enabled: bool) {
        self.rocket_cursor_enabled = enabled;
        if enabled {
            if let Some(pixels) = &self.cursor_pixels {
                let pixel_image = glfw::PixelImage {
                    width: crate::window_engine::constants::ROCKET_CURSOR_WIDTH,
                    height: crate::window_engine::constants::ROCKET_CURSOR_HEIGHT,
                    pixels: pixels.clone(),
                };
                let cursor = glfw::Cursor::create_from_pixels(
                    pixel_image,
                    crate::window_engine::constants::ROCKET_CURSOR_HOTSPOT_X,
                    crate::window_engine::constants::ROCKET_CURSOR_HOTSPOT_Y,
                );
                self.window.set_cursor(Some(cursor));
            }
        } else {
            self.window.set_cursor(None);
        }
    }

    fn is_rocket_cursor_enabled(&self) -> bool {
        self.rocket_cursor_enabled
    }
}

impl GlfwWindowEngine {
    /// Explicitly close and drop the ImGui system.
    /// This is useful to ensure ImGui resources (OpenGL) are cleaned up
    /// BEFORE the OpenGL context is destroyed or before other renderers are closed.
    pub fn close_imgui(&mut self) {
        if let Some(imgui) = self.imgui_system.take() {
            debug!("🧹 Explicitly closing ImGui system");
            drop(imgui);
            debug!("✅ ImGui system closed");
        }
    }
}

// Implement Drop to ensure proper cleanup order:
// ImGui resources must be destroyed BEFORE the GLFW window/context
impl Drop for GlfwWindowEngine {
    fn drop(&mut self) {
        debug!("🧹 Cleaning up GlfwWindowEngine");

        // CRITICAL: Disable OpenGL debug callback BEFORE ImGui cleanup
        // The debug callback can be invoked during ImGui's OpenGL resource cleanup,
        // and if the callback tries to log after some resources are freed, it can cause SIGSEGV
        unsafe {
            gl::DebugMessageCallback(None, std::ptr::null_mut());
            gl::Disable(gl::DEBUG_OUTPUT);
        }

        debug!("✅ OpenGL debug callback disabled");

        // If close_imgui wasn't called manually, drop it here.
        if let Some(imgui) = self.imgui_system.take() {
            debug!("🧹 Dropping ImGui system in Drop");
            drop(imgui);
            debug!("✅ ImGui system dropped");
        }

        debug!("🧹 GlfwWindowEngine cleanup complete");
    }
}
