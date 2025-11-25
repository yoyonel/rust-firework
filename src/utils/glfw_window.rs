use glfw::{Monitor, Window, WindowMode};
use std::mem::discriminant;

pub trait CenterWindow {
    fn center_on_primary_monitor(&mut self);
}

impl CenterWindow for Window {
    fn center_on_primary_monitor(&mut self) {
        let mut glfw = self.glfw.clone();

        glfw.with_primary_monitor(|_, primary_monitor| {
            if let Some(mon) = primary_monitor {
                center_window(self, mon);
            }
        });
    }
}

fn center_window(window: &mut Window, monitor: &Monitor) {
    if let Some(mode) = monitor.get_video_mode() {
        let (monitor_x, monitor_y) = monitor.get_pos();
        let (window_w, window_h) = window.get_size();

        window.set_pos(
            monitor_x + ((mode.width as i32) - window_w) / 2,
            monitor_y + ((mode.height as i32) - window_h) / 2,
        );
    }
}

pub trait Fullscreen {
    fn is_fullscreen(&self) -> bool;
    fn set_fullscreen(&mut self, monitor: &Monitor);
}

impl Fullscreen for Window {
    fn is_fullscreen(&self) -> bool {
        self.with_window_mode(|mode| discriminant(&mode) != discriminant(&WindowMode::Windowed))
    }

    fn set_fullscreen(&mut self, monitor: &Monitor) {
        if let Some(mode) = monitor.get_video_mode() {
            self.set_monitor(
                WindowMode::FullScreen(monitor),
                0,
                0,
                mode.width,
                mode.height,
                Some(mode.refresh_rate),
            );
        }
    }
}
