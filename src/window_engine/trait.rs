use anyhow::Result;
use glfw::{CursorMode, WindowMode};

pub type WindowEvents = glfw::GlfwReceiver<(f64, glfw::WindowEvent)>;

pub struct ImguiSystem {
    pub context: imgui::Context,
    pub glfw: imgui_glfw_rs::ImguiGLFW,
}

pub trait WindowEngine {
    fn init(width: i32, height: i32, title: &str) -> Result<Self>
    where
        Self: Sized;

    fn poll_events(&mut self);
    fn swap_buffers(&mut self);
    fn should_close(&self) -> bool;
    fn set_should_close(&mut self, value: bool);
    fn get_size(&self) -> (i32, i32);
    fn get_pos(&self) -> (i32, i32);
    fn is_fullscreen(&self) -> bool;
    fn set_monitor(
        &mut self,
        mode: WindowMode,
        xpos: i32,
        ypos: i32,
        width: u32,
        height: u32,
        refresh_rate: Option<u32>,
    );
    fn set_cursor_mode(&mut self, mode: CursorMode);
    fn make_current(&mut self);
    fn get_glfw(&self) -> &glfw::Glfw;
    fn get_window_mut(&mut self) -> &mut glfw::PWindow;
    fn get_events(&self) -> &WindowEvents;
    fn get_imgui_system_mut(&mut self) -> &mut ImguiSystem;

    // Helper method to get both window and imgui system for rendering
    fn get_window_and_imgui_mut(&mut self) -> (&mut glfw::PWindow, &mut ImguiSystem);
}
