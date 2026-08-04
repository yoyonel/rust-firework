use imgui::StyleColor;
use serde::{Deserialize, Serialize};

// UI Text Palette Tokens
pub const COLOR_HEADER: [f32; 4] = [0.4, 0.8, 1.0, 1.0];
pub const COLOR_TITLE: [f32; 4] = [0.3, 0.8, 1.0, 1.0];
pub const COLOR_SUCCESS: [f32; 4] = [0.2, 0.9, 0.4, 1.0];
pub const COLOR_WARNING: [f32; 4] = [0.9, 0.9, 0.4, 1.0];
pub const COLOR_ALERT: [f32; 4] = [1.0, 0.8, 0.0, 1.0];
pub const COLOR_COMMAND_NAME: [f32; 4] = [0.2, 1.0, 0.6, 1.0];
pub const COLOR_TEXT_MUTED: [f32; 4] = [0.8, 0.8, 0.8, 1.0];
pub const COLOR_TEXT_HINT: [f32; 4] = [0.5, 0.5, 0.5, 1.0];

// UI Layout & Zoom Scaling Tokens
pub const DEFAULT_GUI_SCALE: f32 = 0.85;
pub const GUI_SCALE_MIN: f32 = 0.60;
pub const GUI_SCALE_MAX: f32 = 1.50;
pub const GUI_SCALE_STEP: f32 = 0.05;

pub const ZOOM_PRESETS: [(f32, &str); 6] = [
    (0.65, "65% (Tiny)"),
    (0.75, "75% (Compact)"),
    (0.85, "85% (Optimal)"),
    (1.00, "100% (Standard)"),
    (1.15, "115% (Large)"),
    (1.30, "130% (Huge)"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GuiTheme {
    #[default]
    CyberpunkCyan,
    DeepSapphire,
    EmeraldFirework,
    DraculaSynthwave,
    ClassicDark,
}

impl GuiTheme {
    pub fn all_themes() -> &'static [(GuiTheme, &'static str)] {
        &[
            (GuiTheme::CyberpunkCyan, "Cyberpunk Cyan (Neon Accent)"),
            (GuiTheme::DeepSapphire, "Deep Sapphire (Modern Slate)"),
            (GuiTheme::EmeraldFirework, "Emerald Firework (Gold & Green)"),
            (
                GuiTheme::DraculaSynthwave,
                "Dracula Synthwave (Purple & Pink)",
            ),
            (GuiTheme::ClassicDark, "Classic Dark (ImGui Standard)"),
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            GuiTheme::CyberpunkCyan => "Cyberpunk Cyan (Neon Accent)",
            GuiTheme::DeepSapphire => "Deep Sapphire (Modern Slate)",
            GuiTheme::EmeraldFirework => "Emerald Firework (Gold & Green)",
            GuiTheme::DraculaSynthwave => "Dracula Synthwave (Purple & Pink)",
            GuiTheme::ClassicDark => "Classic Dark (ImGui Standard)",
        }
    }
}

pub fn apply_theme_to_context(ctx: &mut imgui::Context, theme: GuiTheme) {
    let style = ctx.style_mut();

    if theme == GuiTheme::ClassicDark {
        style.use_dark_colors();
        style.window_rounding = 0.0;
        style.frame_rounding = 0.0;
        style.popup_rounding = 0.0;
        style.scrollbar_rounding = 0.0;
        style.grab_rounding = 0.0;
        style.tab_rounding = 0.0;
        style.window_border_size = 1.0;
        style.frame_border_size = 0.0;
        return;
    }

    style.use_dark_colors();
    style.window_rounding = 8.0;
    style.frame_rounding = 4.0;
    style.popup_rounding = 6.0;
    style.scrollbar_rounding = 6.0;
    style.grab_rounding = 4.0;
    style.tab_rounding = 5.0;
    style.window_border_size = 1.0;
    style.frame_border_size = 1.0;
    style.window_padding = [12.0, 10.0];
    style.frame_padding = [8.0, 4.0];
    style.item_spacing = [8.0, 6.0];

    match theme {
        GuiTheme::CyberpunkCyan => {
            style[StyleColor::WindowBg] = [0.06, 0.07, 0.10, 0.95];
            style[StyleColor::ChildBg] = [0.08, 0.09, 0.13, 0.80];
            style[StyleColor::PopupBg] = [0.08, 0.09, 0.13, 0.95];
            style[StyleColor::Border] = [0.00, 0.85, 0.95, 0.40];
            style[StyleColor::FrameBg] = [0.10, 0.12, 0.18, 0.90];
            style[StyleColor::FrameBgHovered] = [0.14, 0.18, 0.26, 1.00];
            style[StyleColor::FrameBgActive] = [0.18, 0.24, 0.35, 1.00];
            style[StyleColor::TitleBg] = [0.04, 0.05, 0.08, 1.00];
            style[StyleColor::TitleBgActive] = [0.08, 0.10, 0.15, 1.00];
            style[StyleColor::CheckMark] = [0.00, 0.95, 0.90, 1.00];
            style[StyleColor::SliderGrab] = [0.00, 0.85, 0.95, 0.90];
            style[StyleColor::SliderGrabActive] = [0.95, 0.20, 0.65, 1.00];
            style[StyleColor::Button] = [0.12, 0.25, 0.35, 0.80];
            style[StyleColor::ButtonHovered] = [0.00, 0.75, 0.90, 0.90];
            style[StyleColor::ButtonActive] = [0.90, 0.15, 0.60, 1.00];
            style[StyleColor::Header] = [0.15, 0.22, 0.32, 0.80];
            style[StyleColor::HeaderHovered] = [0.00, 0.75, 0.90, 0.80];
            style[StyleColor::HeaderActive] = [0.90, 0.15, 0.60, 0.90];
            style[StyleColor::Tab] = [0.10, 0.14, 0.20, 0.90];
            style[StyleColor::TabHovered] = [0.00, 0.85, 0.95, 0.90];
            style[StyleColor::TabActive] = [0.00, 0.75, 0.95, 1.00];
            style[StyleColor::Separator] = [0.00, 0.85, 0.95, 0.30];
        }
        GuiTheme::DeepSapphire => {
            style[StyleColor::WindowBg] = [0.10, 0.12, 0.16, 0.95];
            style[StyleColor::ChildBg] = [0.13, 0.15, 0.20, 0.80];
            style[StyleColor::PopupBg] = [0.13, 0.15, 0.20, 0.95];
            style[StyleColor::Border] = [0.22, 0.30, 0.42, 0.50];
            style[StyleColor::FrameBg] = [0.16, 0.20, 0.28, 0.90];
            style[StyleColor::FrameBgHovered] = [0.22, 0.28, 0.38, 1.00];
            style[StyleColor::FrameBgActive] = [0.28, 0.36, 0.48, 1.00];
            style[StyleColor::TitleBg] = [0.08, 0.10, 0.14, 1.00];
            style[StyleColor::TitleBgActive] = [0.14, 0.18, 0.25, 1.00];
            style[StyleColor::CheckMark] = [0.35, 0.65, 1.00, 1.00];
            style[StyleColor::SliderGrab] = [0.30, 0.60, 0.95, 0.90];
            style[StyleColor::SliderGrabActive] = [0.45, 0.75, 1.00, 1.00];
            style[StyleColor::Button] = [0.18, 0.28, 0.42, 0.85];
            style[StyleColor::ButtonHovered] = [0.25, 0.42, 0.65, 0.90];
            style[StyleColor::ButtonActive] = [0.30, 0.52, 0.80, 1.00];
            style[StyleColor::Header] = [0.20, 0.32, 0.48, 0.80];
            style[StyleColor::HeaderHovered] = [0.28, 0.44, 0.65, 0.90];
            style[StyleColor::HeaderActive] = [0.35, 0.55, 0.82, 1.00];
            style[StyleColor::Tab] = [0.14, 0.18, 0.25, 0.90];
            style[StyleColor::TabHovered] = [0.25, 0.45, 0.70, 0.90];
            style[StyleColor::TabActive] = [0.22, 0.40, 0.65, 1.00];
            style[StyleColor::Separator] = [0.22, 0.30, 0.42, 0.50];
        }
        GuiTheme::EmeraldFirework => {
            style[StyleColor::WindowBg] = [0.08, 0.09, 0.08, 0.95];
            style[StyleColor::ChildBg] = [0.11, 0.13, 0.11, 0.80];
            style[StyleColor::PopupBg] = [0.11, 0.13, 0.11, 0.95];
            style[StyleColor::Border] = [0.18, 0.50, 0.35, 0.50];
            style[StyleColor::FrameBg] = [0.12, 0.18, 0.14, 0.90];
            style[StyleColor::FrameBgHovered] = [0.18, 0.26, 0.20, 1.00];
            style[StyleColor::FrameBgActive] = [0.22, 0.34, 0.25, 1.00];
            style[StyleColor::TitleBg] = [0.06, 0.08, 0.06, 1.00];
            style[StyleColor::TitleBgActive] = [0.10, 0.16, 0.12, 1.00];
            style[StyleColor::CheckMark] = [0.20, 0.90, 0.55, 1.00];
            style[StyleColor::SliderGrab] = [0.15, 0.80, 0.48, 0.90];
            style[StyleColor::SliderGrabActive] = [0.95, 0.70, 0.20, 1.00];
            style[StyleColor::Button] = [0.14, 0.32, 0.22, 0.85];
            style[StyleColor::ButtonHovered] = [0.20, 0.55, 0.36, 0.90];
            style[StyleColor::ButtonActive] = [0.90, 0.65, 0.15, 1.00];
            style[StyleColor::Header] = [0.16, 0.36, 0.25, 0.80];
            style[StyleColor::HeaderHovered] = [0.22, 0.52, 0.35, 0.90];
            style[StyleColor::HeaderActive] = [0.90, 0.65, 0.15, 0.90];
            style[StyleColor::Tab] = [0.12, 0.18, 0.14, 0.90];
            style[StyleColor::TabHovered] = [0.20, 0.55, 0.36, 0.90];
            style[StyleColor::TabActive] = [0.16, 0.48, 0.30, 1.00];
            style[StyleColor::Separator] = [0.18, 0.50, 0.35, 0.40];
        }
        GuiTheme::DraculaSynthwave => {
            style[StyleColor::WindowBg] = [0.12, 0.11, 0.17, 0.95];
            style[StyleColor::ChildBg] = [0.16, 0.14, 0.22, 0.80];
            style[StyleColor::PopupBg] = [0.16, 0.14, 0.22, 0.95];
            style[StyleColor::Border] = [0.65, 0.35, 0.85, 0.50];
            style[StyleColor::FrameBg] = [0.20, 0.17, 0.28, 0.90];
            style[StyleColor::FrameBgHovered] = [0.28, 0.22, 0.38, 1.00];
            style[StyleColor::FrameBgActive] = [0.35, 0.26, 0.48, 1.00];
            style[StyleColor::TitleBg] = [0.09, 0.08, 0.14, 1.00];
            style[StyleColor::TitleBgActive] = [0.16, 0.13, 0.24, 1.00];
            style[StyleColor::CheckMark] = [0.98, 0.45, 0.70, 1.00];
            style[StyleColor::SliderGrab] = [0.75, 0.40, 0.90, 0.90];
            style[StyleColor::SliderGrabActive] = [0.98, 0.45, 0.70, 1.00];
            style[StyleColor::Button] = [0.30, 0.18, 0.42, 0.85];
            style[StyleColor::ButtonHovered] = [0.52, 0.28, 0.72, 0.90];
            style[StyleColor::ButtonActive] = [0.95, 0.40, 0.65, 1.00];
            style[StyleColor::Header] = [0.32, 0.20, 0.45, 0.80];
            style[StyleColor::HeaderHovered] = [0.55, 0.30, 0.75, 0.90];
            style[StyleColor::HeaderActive] = [0.95, 0.40, 0.65, 0.90];
            style[StyleColor::Tab] = [0.18, 0.14, 0.26, 0.90];
            style[StyleColor::TabHovered] = [0.52, 0.28, 0.72, 0.90];
            style[StyleColor::TabActive] = [0.42, 0.22, 0.60, 1.00];
            style[StyleColor::Separator] = [0.65, 0.35, 0.85, 0.40];
        }
        GuiTheme::ClassicDark => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_gui_theme_all_themes_and_display_name() {
        let themes = GuiTheme::all_themes();
        assert_eq!(themes.len(), 5);

        for (theme, name) in themes {
            assert_eq!(theme.display_name(), *name);
        }
    }

    #[test]
    #[serial]
    fn test_gui_theme_apply_to_context() {
        let _guard = crate::simulator::gui_settings::IMGUI_TEST_MUTEX
            .lock()
            .unwrap();
        let mut ctx = imgui::Context::create();
        for (theme, _) in GuiTheme::all_themes() {
            apply_theme_to_context(&mut ctx, *theme);
        }
    }
}
