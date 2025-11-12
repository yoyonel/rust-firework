// use gl::types::*;
use gl::types::*;
use log::{debug, info, warn};
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_void;
use std::sync::Mutex;
use std::{ffi::CString, ptr};

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

/// # Safety
/// Interagit directement avec des pointeurs OpenGL.
pub unsafe fn compile_shader_program(vertex_src: &str, fragment_src: &str) -> u32 {
    fn compile_shader(src: &str, ty: GLenum) -> u32 {
        let shader = unsafe { gl::CreateShader(ty) };
        let c_str = CString::new(src).unwrap();
        unsafe {
            gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
            gl::CompileShader(shader);

            let mut success = gl::FALSE as GLint;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as GLint {
                let mut len = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
                let mut buf = Vec::with_capacity(len as usize);
                gl::GetShaderInfoLog(shader, len, ptr::null_mut(), buf.as_mut_ptr() as *mut _);
                buf.set_len(len as usize);
                let log = String::from_utf8_lossy(&buf);

                eprintln!("\n❌ Shader compilation failed:\n{}", log);

                // --- Essayons de donner du contexte ---
                if let Some((line_number, _col)) = parse_glsl_error_line(&log) {
                    show_glsl_error_context(src, line_number);
                }

                panic!("Shader compilation failed (see above).");
            }
        }
        shader
    }

    let vs = compile_shader(vertex_src, gl::VERTEX_SHADER);
    let fs = compile_shader(fragment_src, gl::FRAGMENT_SHADER);

    let program = unsafe { gl::CreateProgram() };
    unsafe {
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);

        let mut success = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
        if success != gl::TRUE as GLint {
            let mut len = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            gl::GetProgramInfoLog(program, len, ptr::null_mut(), buf.as_mut_ptr() as *mut _);
            buf.set_len(len as usize);
            let log = String::from_utf8_lossy(&buf);
            panic!("Shader link failed:\n{}", log);
        }

        gl::DeleteShader(vs);
        gl::DeleteShader(fs);
    }
    program
}

/// Essaie d’extraire le numéro de ligne de l’erreur GLSL (ex: "0:12(105): ...")
fn parse_glsl_error_line(log: &str) -> Option<(usize, usize)> {
    let re = regex::Regex::new(r"(\d+):(\d+)\((\d+)\)").ok()?;
    re.captures(log).and_then(|cap| {
        let line = cap.get(2)?.as_str().parse::<usize>().ok()?;
        let col = cap.get(3)?.as_str().parse::<usize>().ok()?;
        Some((line, col))
    })
}

/// Affiche un extrait du code GLSL autour de la ligne fautive
fn show_glsl_error_context(src: &str, line_number: usize) {
    let lines: Vec<&str> = src.lines().collect();
    let context_range = 2; // nb de lignes avant/après à afficher

    eprintln!("🔍 Error context (line {}):", line_number);

    let start = line_number.saturating_sub(1 + context_range);
    let end = (line_number + context_range).min(lines.len());

    for (i, line) in lines[start..end].iter().enumerate() {
        let current = start + i + 1;
        if current == line_number {
            eprintln!("> {:>3} | {}", current, line);
            eprintln!("        {}", "^".repeat(line.len().min(80)));
        } else {
            eprintln!("  {:>3} | {}", current, line);
        }
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
