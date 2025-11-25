use super::r#trait::{ImguiSystem, WindowEngine, WindowEvents};
use anyhow::{anyhow, Result};
use glfw::{Context, CursorMode, WindowMode};
use imgui::Context as ImContext;
use imgui_glfw_rs::glfw;
use imgui_glfw_rs::ImguiGLFW;
use log::info;

use crate::renderer_engine::tools::{setup_opengl_debug, show_opengl_context_info};
use crate::utils::Fullscreen;

pub struct GlfwWindowEngine {
    glfw: glfw::Glfw,
    window: glfw::PWindow,
    events: WindowEvents,
    imgui_system: ImguiSystem,
}

impl WindowEngine for GlfwWindowEngine {
    fn init(width: i32, height: i32, title: &str) -> Result<Self> {
        let _ = env_logger::builder().is_test(true).try_init();

        let mut glfw = glfw::init(glfw::fail_on_errors)
            .map_err(|_| anyhow!("Impossible d'initialiser GLFW"))?;

        glfw.window_hint(glfw::WindowHint::ContextVersionMajor(3));
        glfw.window_hint(glfw::WindowHint::ContextVersionMinor(3));
        glfw.window_hint(glfw::WindowHint::OpenGlProfile(
            glfw::OpenGlProfileHint::Core,
        ));

        let (mut window, events) = glfw
            .create_window(
                width as u32,
                height as u32,
                title,
                glfw::WindowMode::Windowed,
            )
            .expect("Erreur création fenêtre GLFW");

        window.make_current();
        window.set_key_polling(true);
        window.set_char_polling(true);
        window.set_framebuffer_size_polling(true);
        window.set_cursor_pos_polling(true);
        window.set_mouse_button_polling(true);
        window.set_scroll_polling(true);

        info!("✅ OpenGL context ready for '{}'", title);

        // load OpenGL function pointers
        gl::load_with(|s| window.get_proc_address(s) as *const _);

        unsafe {
            show_opengl_context_info();
            setup_opengl_debug();
            gl::Enable(gl::PROGRAM_POINT_SIZE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        let mut imgui = ImContext::create();
        let font_data =
            std::fs::read("assets/fonts/PerfectDOSVGA437.ttf").expect("Failed to read font file");
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
        imgui.style_mut().use_dark_colors();

        let imgui_glfw = ImguiGLFW::new(&mut imgui, &mut window);

        Ok(Self {
            glfw,
            window,
            events,
            imgui_system: ImguiSystem {
                context: imgui,
                glfw: imgui_glfw,
            },
        })
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

    fn get_imgui_system_mut(&mut self) -> &mut ImguiSystem {
        &mut self.imgui_system
    }

    fn get_window_and_imgui_mut(&mut self) -> (&mut glfw::PWindow, &mut ImguiSystem) {
        (&mut self.window, &mut self.imgui_system)
    }
}
