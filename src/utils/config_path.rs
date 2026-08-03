use std::path::PathBuf;

pub const DEFAULT_CONFIG_DIR: &str = "assets/config";
pub const GUI_SESSION_FILE: &str = "gui_session.toml";
pub const PHYSIC_CONFIG_FILE: &str = "physic.toml";
pub const RENDERER_CONFIG_FILE: &str = "renderer.toml";
pub const AUDIO_CONFIG_FILE: &str = "audio.toml";
pub const IMGUI_INI_FILE: &str = "imgui.ini";
pub const DEFAULT_FONT_PATH: &str = "assets/fonts/PerfectDOSVGA437.ttf";

/// Detects whether the current process is running as a test (unit test or integration test binary in target/deps/).
pub fn is_test_environment() -> bool {
    if cfg!(test) {
        return true;
    }
    if let Ok(exe) = std::env::current_exe() {
        let exe_str = exe.to_string_lossy();
        if exe_str.contains("/target/")
            && (exe_str.contains("/deps/") || exe_str.contains("\\deps\\"))
        {
            return true;
        }
    }
    false
}

/// Returns the configuration directory path.
/// Respects `FIREWORKS_CONFIG_DIR` environment variable if set.
pub fn get_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("FIREWORKS_CONFIG_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    PathBuf::from(DEFAULT_CONFIG_DIR)
}

/// Returns the path to a specific config file in the active config directory.
pub fn get_config_path(filename: &str) -> PathBuf {
    get_config_dir().join(filename)
}

pub fn get_gui_session_path() -> PathBuf {
    get_config_path(GUI_SESSION_FILE)
}

pub fn get_physic_config_path() -> PathBuf {
    get_config_path(PHYSIC_CONFIG_FILE)
}

pub fn get_renderer_config_path() -> PathBuf {
    get_config_path(RENDERER_CONFIG_FILE)
}

pub fn get_audio_config_path() -> PathBuf {
    get_config_path(AUDIO_CONFIG_FILE)
}

/// Returns the imgui.ini path. Returns None during test execution unless FIREWORKS_CONFIG_DIR is set.
pub fn get_imgui_ini_path() -> Option<PathBuf> {
    if is_test_environment() && std::env::var_os("FIREWORKS_CONFIG_DIR").is_none() {
        return None;
    }
    Some(get_config_path(IMGUI_INI_FILE))
}

/// Returns whether configuration saving to disk is enabled.
/// Disabled during unit/integration test execution or if `FIREWORKS_NO_CONFIG_SAVE` is set,
/// unless `FIREWORKS_CONFIG_DIR` is explicitly configured.
pub fn is_config_save_enabled() -> bool {
    if std::env::var_os("FIREWORKS_NO_CONFIG_SAVE").is_some() {
        return false;
    }
    if is_test_environment() && std::env::var_os("FIREWORKS_CONFIG_DIR").is_none() {
        return false;
    }
    true
}
