// use gl::types::*;
use gl::types::*;
use log::{debug, info, warn};
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_void;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref LOGGED_IDS: Mutex<HashSet<u32>> = Mutex::new(HashSet::new());
    static ref MESSAGE_COUNT: Mutex<std::collections::HashMap<u32, u32>> = Mutex::new(std::collections::HashMap::new());
}

#[macro_export]
macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const i8
    };
}

/// Affiche les informations OpenGL / GPU du contexte actuel
/// # Safety
///
/// L'appelant doit s'assurer que le contexte OpenGL est valide et actif.
pub unsafe fn show_opengl_context_info() {
    use std::ffi::CStr;

    // Vendor / Renderer / Version / GLSL
    let vendor = CStr::from_ptr(gl::GetString(gl::VENDOR) as *const i8)
        .to_str()
        .unwrap_or("Unknown");
    let renderer = CStr::from_ptr(gl::GetString(gl::RENDERER) as *const i8)
        .to_str()
        .unwrap_or("Unknown");
    let version = CStr::from_ptr(gl::GetString(gl::VERSION) as *const i8)
        .to_str()
        .unwrap_or("Unknown");
    let glsl_version = CStr::from_ptr(gl::GetString(gl::SHADING_LANGUAGE_VERSION) as *const i8)
        .to_str()
        .unwrap_or("Unknown");

    info!("🖥 OpenGL context info:");
    info!("  Vendor   : {}", vendor);
    info!("  Renderer : {}", renderer);
    info!("  OpenGL   : {}", version);
    info!("  GLSL     : {}", glsl_version);

    // Nombre d'extensions
    let mut num_ext = 0;
    gl::GetIntegerv(gl::NUM_EXTENSIONS, &mut num_ext);
    info!("  Extensions: {} extensions detected", num_ext);

    // Récupère toutes les extensions OpenGL et les affiche en une seule ligne
    let mut extensions = Vec::new();
    for i in 0..num_ext {
        let ext = CStr::from_ptr(gl::GetStringi(gl::EXTENSIONS, i as u32) as *const i8)
            .to_str()
            .unwrap_or("Unknown");
        extensions.push(ext);
    }

    debug!("GL_EXTENSIONS = {}", extensions.join(" "));

    // Consommer le glerror si nécessaire
    let err = gl::GetError();
    if err != gl::NO_ERROR {
        warn!("glerror consumed after getting context info: 0x{:X}", err);
    }
}

/// Callback OpenGL debug, safe pour Rust
extern "system" fn gl_debug_callback(
    source: GLenum,
    type_: GLenum,
    id: GLuint,
    severity: GLenum,
    _length: GLsizei,
    message: *const i8,
    _user_param: *mut c_void,
) {
    // Unsafe uniquement pour lire le C string
    let msg = unsafe { CStr::from_ptr(message).to_string_lossy() };

    if severity == gl::DEBUG_SEVERITY_NOTIFICATION {
        return; // ignore notifications
    }

    // Ne logue qu’une fois par ID
    let mut logged = LOGGED_IDS.lock().unwrap();
    if logged.contains(&id) {
        return;
    }
    logged.insert(id);

    let src_str = match source {
        gl::DEBUG_SOURCE_API => "API",
        gl::DEBUG_SOURCE_WINDOW_SYSTEM => "Window System",
        gl::DEBUG_SOURCE_SHADER_COMPILER => "Shader Compiler",
        gl::DEBUG_SOURCE_THIRD_PARTY => "Third Party",
        gl::DEBUG_SOURCE_APPLICATION => "Application",
        gl::DEBUG_SOURCE_OTHER => "Other",
        _ => "Unknown",
    };

    let type_str = match type_ {
        gl::DEBUG_TYPE_ERROR => "Error",
        gl::DEBUG_TYPE_DEPRECATED_BEHAVIOR => "Deprecated Behavior",
        gl::DEBUG_TYPE_UNDEFINED_BEHAVIOR => "Undefined Behavior",
        gl::DEBUG_TYPE_PORTABILITY => "Portability",
        gl::DEBUG_TYPE_PERFORMANCE => "Performance",
        gl::DEBUG_TYPE_MARKER => "Marker",
        gl::DEBUG_TYPE_PUSH_GROUP => "Push Group",
        gl::DEBUG_TYPE_POP_GROUP => "Pop Group",
        gl::DEBUG_TYPE_OTHER => "Other",
        _ => "Unknown",
    };

    let sev_str = match severity {
        gl::DEBUG_SEVERITY_HIGH => "High",
        gl::DEBUG_SEVERITY_MEDIUM => "Medium",
        gl::DEBUG_SEVERITY_LOW => "Low",
        gl::DEBUG_SEVERITY_NOTIFICATION => "Notification",
        _ => "Unknown",
    };

    let mut counts = MESSAGE_COUNT.lock().unwrap();
    let count = counts.entry(id).or_insert(0);
    *count += 1;
    if *count == 1 || (*count).is_multiple_of(60) {
        warn!(
            "[OpenGL Debug] id: {:X}, source: {}, type: {}, severity: {}, message: {}",
            id, src_str, type_str, sev_str, msg
        );
    }
}

/// Active le debug OpenGL
/// Configure le debug OpenGL via `glDebugMessageCallback`.
///
/// # Safety
///
/// Cette fonction est unsafe car elle enregistre un callback C vers Rust.
/// L'appelant doit s'assurer que :
/// - Le contexte OpenGL est actif.
/// - Le callback reste valide pendant toute la durée du contexte.
/// - Aucun autre thread ne détruit le callback pendant son usage.
pub unsafe fn setup_opengl_debug() {
    gl::Enable(gl::DEBUG_OUTPUT);
    gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS); // important pour que le callback soit synchrone

    // Passe le callback safe
    gl::DebugMessageCallback(Some(gl_debug_callback), std::ptr::null_mut());

    // Optionnel : filtrer certains messages
    gl::DebugMessageControl(
        gl::DONT_CARE,
        gl::DONT_CARE,
        gl::DONT_CARE,
        0,
        std::ptr::null(),
        gl::TRUE,
    );
}

/// Formats a byte size into a human-readable string with appropriate units (bytes, KB, MB, GB).
pub fn format_bytes(size: isize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let size_f64 = size as f64;

    if size_f64 >= GB {
        format!("{:.3} GB", size_f64 / GB)
    } else if size_f64 >= MB {
        format!("{:.3} MB", size_f64 / MB)
    } else if size_f64 >= KB {
        format!("{:.3} KB", size_f64 / KB)
    } else {
        format!("{} bytes", size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 bytes");
        assert_eq!(format_bytes(1024), "1.000 KB");
        assert_eq!(format_bytes(1536), "1.500 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.000 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.000 GB");
    }

    #[test]
    fn test_format_bytes_edge_cases() {
        // Zero bytes
        assert_eq!(format_bytes(0), "0 bytes");

        // Negative values (edge case)
        assert_eq!(format_bytes(-100), "-100 bytes");

        // Boundary values
        assert_eq!(format_bytes(1023), "1023 bytes");
        assert_eq!(format_bytes(1024 * 1024 - 1), "1023.999 KB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 - 1), "1024.000 MB");

        // Very large values
        let large_value = 5 * 1024 * 1024 * 1024; // 5 GB
        assert_eq!(format_bytes(large_value), "5.000 GB");

        // Fractional KB
        assert_eq!(format_bytes(2048), "2.000 KB");
        assert_eq!(format_bytes(2560), "2.500 KB");
    }

    #[test]
    fn test_cstr_macro() {
        let ptr = cstr!("hello");
        unsafe {
            let c_str = CStr::from_ptr(ptr);
            assert_eq!(c_str.to_str().unwrap(), "hello");
        }
    }

    #[test]
    fn test_cstr_macro_edge_cases() {
        // Empty string
        let ptr = cstr!("");
        unsafe {
            let c_str = CStr::from_ptr(ptr);
            assert_eq!(c_str.to_str().unwrap(), "");
        }

        // String with spaces
        let ptr = cstr!("hello world");
        unsafe {
            let c_str = CStr::from_ptr(ptr);
            assert_eq!(c_str.to_str().unwrap(), "hello world");
        }

        // String with special characters
        let ptr = cstr!("test_123");
        unsafe {
            let c_str = CStr::from_ptr(ptr);
            assert_eq!(c_str.to_str().unwrap(), "test_123");
        }
    }

    #[test]
    fn test_gl_debug_callback_deduplication() {
        use std::ffi::CString;

        // Use a unique ID for this test to avoid collision
        let id = 0x12345678;
        let msg = CString::new("Test debug message").unwrap();

        // First call
        gl_debug_callback(
            gl::DEBUG_SOURCE_APPLICATION,
            gl::DEBUG_TYPE_ERROR,
            id,
            gl::DEBUG_SEVERITY_HIGH,
            0,
            msg.as_ptr(),
            std::ptr::null_mut(),
        );

        // Check if it's in LOGGED_IDS
        {
            let logged = LOGGED_IDS.lock().unwrap();
            assert!(logged.contains(&id));
        }

        // Check MESSAGE_COUNT
        {
            let counts = MESSAGE_COUNT.lock().unwrap();
            assert_eq!(counts.get(&id), Some(&1));
        }

        // Second call - should return early due to deduplication
        gl_debug_callback(
            gl::DEBUG_SOURCE_APPLICATION,
            gl::DEBUG_TYPE_ERROR,
            id,
            gl::DEBUG_SEVERITY_HIGH,
            0,
            msg.as_ptr(),
            std::ptr::null_mut(),
        );

        // Check MESSAGE_COUNT again - should still be 1
        {
            let counts = MESSAGE_COUNT.lock().unwrap();
            assert_eq!(counts.get(&id), Some(&1));
        }
    }
}
