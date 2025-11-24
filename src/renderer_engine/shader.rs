use gl::types::*;
use std::{ffi::CString, fs, path::Path, ptr};

/// Charge le code source d'un shader depuis un fichier.
///
/// # Arguments
/// * `path` - Chemin vers le fichier shader (relatif ou absolu)
///
/// # Returns
/// Le contenu du fichier shader sous forme de String
///
/// # Panics
/// Panique si le fichier ne peut pas être lu
pub fn load_shader_from_file<P: AsRef<Path>>(path: P) -> String {
    let path = path.as_ref();
    fs::read_to_string(path).unwrap_or_else(|err| {
        panic!(
            "❌ Failed to load shader file '{}': {}",
            path.display(),
            err
        )
    })
}

/// Compile un programme shader à partir de fichiers GLSL.
///
/// # Arguments
/// * `vertex_path` - Chemin vers le fichier vertex shader
/// * `fragment_path` - Chemin vers le fichier fragment shader
///
/// # Returns
/// L'ID du programme shader compilé
///
/// # Safety
/// Cette fonction est unsafe car elle interagit directement avec des pointeurs OpenGL.
pub unsafe fn compile_shader_program_from_files<P: AsRef<Path>>(
    vertex_path: P,
    fragment_path: P,
) -> u32 {
    let vertex_src = load_shader_from_file(vertex_path);
    let fragment_src = load_shader_from_file(fragment_path);
    compile_shader_program(&vertex_src, &fragment_src)
}

/// Tente de compiler un programme shader à partir de fichiers GLSL.
/// Version sécurisée qui retourne un Result au lieu de paniquer.
///
/// # Arguments
/// * `vertex_path` - Chemin vers le fichier vertex shader
/// * `fragment_path` - Chemin vers le fichier fragment shader
///
/// # Returns
/// `Ok(program_id)` si la compilation réussit, `Err(error_message)` sinon
///
/// # Safety
/// Cette fonction est unsafe car elle interagit directement avec des pointeurs OpenGL.
pub unsafe fn try_compile_shader_program_from_files<P: AsRef<Path>>(
    vertex_path: P,
    fragment_path: P,
) -> Result<u32, String> {
    let vertex_path = vertex_path.as_ref();
    let fragment_path = fragment_path.as_ref();

    // Charger les fichiers shader
    let vertex_src = match std::fs::read_to_string(vertex_path) {
        Ok(src) => src,
        Err(e) => {
            return Err(format!(
                "Failed to load vertex shader '{}': {}",
                vertex_path.display(),
                e
            ))
        }
    };

    let fragment_src = match std::fs::read_to_string(fragment_path) {
        Ok(src) => src,
        Err(e) => {
            return Err(format!(
                "Failed to load fragment shader '{}': {}",
                fragment_path.display(),
                e
            ))
        }
    };

    // Tenter de compiler
    try_compile_shader_program(&vertex_src, &fragment_src)
}

/// Tente de compiler un programme shader à partir de sources.
/// Version sécurisée qui retourne un Result au lieu de paniquer.
///
/// # Safety
/// Cette fonction est unsafe car elle interagit directement avec des pointeurs OpenGL.
unsafe fn try_compile_shader_program(vertex_src: &str, fragment_src: &str) -> Result<u32, String> {
    fn try_compile_shader(src: &str, ty: GLenum) -> Result<u32, String> {
        let shader = unsafe { gl::CreateShader(ty) };
        let c_str = CString::new(src).map_err(|e| format!("CString error: {}", e))?;

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
                let log_cow = String::from_utf8_lossy(&buf);
                let log = log_cow.trim_matches(char::from(0));

                gl::DeleteShader(shader);

                let mut error_msg = format!("Shader compilation failed:\n{}", log);
                if let Some((line, _col)) = parse_glsl_error_line(log) {
                    error_msg.push_str(&format_glsl_error_context(src, line));
                } else {
                    error_msg.push_str(&format!(
                        "\n(Debug: Failed to parse line number. Raw log: {:?})",
                        log
                    ));
                }

                return Err(error_msg);
            }
        }
        Ok(shader)
    }

    let vs = try_compile_shader(vertex_src, gl::VERTEX_SHADER)?;
    let fs = try_compile_shader(fragment_src, gl::FRAGMENT_SHADER)?;

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

            gl::DeleteShader(vs);
            gl::DeleteShader(fs);
            gl::DeleteProgram(program);
            return Err(format!("Shader link failed:\n{}", log));
        }

        gl::DeleteShader(vs);
        gl::DeleteShader(fs);
    }
    Ok(program)
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
                    eprintln!("{}", format_glsl_error_context(src, line_number));
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
/// Essaie d’extraire le numéro de ligne de l’erreur GLSL
/// Supporte plusieurs formats :
/// - "0:12(105): ..." (Standard/Intel)
/// - "0(12) : error ..." (NVIDIA)
/// - "ERROR: 0:12: ..." (AMD/ATI)
fn parse_glsl_error_line(log: &str) -> Option<(usize, usize)> {
    // 1. Format Standard/Intel: 0:12(105)
    // Group 2 = Line
    let re_standard = regex::Regex::new(r"(\d+):(\d+)\((\d+)\)").ok()?;
    if let Some(cap) = re_standard.captures(log) {
        if let Some(line_match) = cap.get(2) {
            if let Ok(line) = line_match.as_str().parse::<usize>() {
                return Some((line, 0)); // Col is optional/variable
            }
        }
    }

    // 2. Format NVIDIA: 0(12) : error ...
    // Group 2 = Line
    let re_nvidia = regex::Regex::new(r"(\d+)\((\d+)\)\s*:").ok()?;
    if let Some(cap) = re_nvidia.captures(log) {
        if let Some(line_match) = cap.get(2) {
            if let Ok(line) = line_match.as_str().parse::<usize>() {
                return Some((line, 0));
            }
        }
    }

    // 3. Format AMD/ATI: ERROR: 0:12: ...
    // Group 2 = Line
    let re_amd = regex::Regex::new(r":\s*(\d+):(\d+):").ok()?;
    if let Some(cap) = re_amd.captures(log) {
        if let Some(line_match) = cap.get(2) {
            if let Ok(line) = line_match.as_str().parse::<usize>() {
                return Some((line, 0));
            }
        }
    }

    None
}

/// Formate un extrait du code GLSL autour de la ligne fautive
fn format_glsl_error_context(src: &str, line_number: usize) -> String {
    let lines: Vec<&str> = src.lines().collect();
    let mut output = String::new();

    // Handle empty source or line number beyond source length
    if lines.is_empty() || line_number == 0 {
        return output;
    }

    let context_range = 2; // nb de lignes avant/après à afficher

    output.push_str(&format!("\n🔍 Error context (line {}):\n", line_number));

    let start = line_number.saturating_sub(1 + context_range);
    let end = (line_number + context_range).min(lines.len());

    // Ensure we don't try to slice beyond the array bounds
    let safe_start = start.min(lines.len());
    let safe_end = end.min(lines.len());

    for (i, line) in lines[safe_start..safe_end].iter().enumerate() {
        let current = safe_start + i + 1;
        if current == line_number {
            output.push_str(&format!("> {:>3} | {}\n", current, line));
            output.push_str(&format!("        {}\n", "^".repeat(line.len().min(80))));
        } else {
            output.push_str(&format!("  {:>3} | {}\n", current, line));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_glsl_error_line() {
        // Standard/Intel
        let log_intel = "0:12(105): error: undefined variable";
        assert_eq!(parse_glsl_error_line(log_intel).map(|(l, _)| l), Some(12));

        // NVIDIA
        let log_nvidia = "0(12) : error C1000: undefined variable";
        assert_eq!(parse_glsl_error_line(log_nvidia).map(|(l, _)| l), Some(12));

        // AMD
        let log_amd = "ERROR: 0:12: 'undefined_var' : undeclared identifier";
        assert_eq!(parse_glsl_error_line(log_amd).map(|(l, _)| l), Some(12));

        let log_no_match = "Error: some error without line info";
        assert_eq!(parse_glsl_error_line(log_no_match), None);
    }

    #[test]
    fn test_parse_glsl_error_line_edge_cases() {
        // Different line and column numbers
        assert_eq!(
            parse_glsl_error_line("0:1(1): error").map(|(l, _)| l),
            Some(1)
        );
        assert_eq!(
            parse_glsl_error_line("0:999(9999): error").map(|(l, _)| l),
            Some(999)
        );

        // Multiple matches - should get the first one
        assert_eq!(
            parse_glsl_error_line("0:5(10): error and 0:6(20): another").map(|(l, _)| l),
            Some(5)
        );

        // Malformed patterns
        assert_eq!(parse_glsl_error_line("0:12: error"), None); // Missing column
        assert_eq!(
            parse_glsl_error_line("12(105): error").map(|(l, _)| l),
            Some(105)
        ); // Valid NVIDIA-like format
        assert_eq!(
            parse_glsl_error_line("abc:12(105): error").map(|(l, _)| l),
            Some(105)
        ); // Contains valid pattern

        // Empty string
        assert_eq!(parse_glsl_error_line(""), None);

        // Trailing null character
        let log_with_null = "0:10(2): error: 'toto' undeclared\0";
        assert_eq!(
            parse_glsl_error_line(log_with_null).map(|(l, _)| l),
            Some(10)
        );

        // Error message with context
        let complex_log = "ERROR: 0:42(256): 'undefined_var' : undeclared identifier";
        assert_eq!(parse_glsl_error_line(complex_log).map(|(l, _)| l), Some(42));
    }

    #[test]
    fn test_format_glsl_error_context() {
        let src = r#"void main() {
            gl_Position = vec4(0.0);
            // error here
        }"#;
        // Just ensure it doesn't panic and returns something
        let output = format_glsl_error_context(src, 2);
        assert!(output.contains("Error context"));
        assert!(output.contains(">   2 |             gl_Position = vec4(0.0);"));
    }

    #[test]
    fn test_format_glsl_error_context_edge_cases() {
        // Empty source
        assert_eq!(format_glsl_error_context("", 1), "");

        // Single line source
        let out = format_glsl_error_context("void main() {}", 1);
        assert!(out.contains(">   1 | void main() {}"));

        // Error at first line
        let src = "line1\nline2\nline3\nline4\nline5";
        let out = format_glsl_error_context(src, 1);
        assert!(out.contains(">   1 | line1"));

        // Error at last line
        let out = format_glsl_error_context(src, 5);
        assert!(out.contains(">   5 | line5"));

        // Error beyond source length (should not panic)
        format_glsl_error_context(src, 100);

        // Very long line (should truncate in display logic if we had one, but here we just check it doesn't crash)
        let long_line = "a".repeat(200);
        format_glsl_error_context(&long_line, 1);

        // Multi-line with error in middle
        let multi = "line1\nline2\nline3\nline4\nline5\nline6\nline7";
        let out = format_glsl_error_context(multi, 4);
        assert!(out.contains(">   4 | line4"));
    }
}
