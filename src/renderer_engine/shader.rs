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

    // Handle empty source or line number beyond source length
    if lines.is_empty() || line_number == 0 {
        return;
    }

    let context_range = 2; // nb de lignes avant/après à afficher

    eprintln!("🔍 Error context (line {}):", line_number);

    let start = line_number.saturating_sub(1 + context_range);
    let end = (line_number + context_range).min(lines.len());

    // Ensure we don't try to slice beyond the array bounds
    let safe_start = start.min(lines.len());
    let safe_end = end.min(lines.len());

    for (i, line) in lines[safe_start..safe_end].iter().enumerate() {
        let current = safe_start + i + 1;
        if current == line_number {
            eprintln!("> {:>3} | {}", current, line);
            eprintln!("        {}", "^".repeat(line.len().min(80)));
        } else {
            eprintln!("  {:>3} | {}", current, line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_glsl_error_line() {
        // Test standard format: "0:12(105): error: ..."
        // The regex is r"(\d+):(\d+)\((\d+)\)"
        // It captures group 2 as line, group 3 as col?
        // Let's check the regex in the code:
        // let re = regex::Regex::new(r"(\d+):(\d+)\((\d+)\)").ok()?;
        // cap.get(2) -> line
        // cap.get(3) -> col
        // So "0:12(105)" -> line 12, col 105.

        let log = "0:12(105): error: undefined variable";
        assert_eq!(parse_glsl_error_line(log), Some((12, 105)));

        let log_no_match = "Error: some error without line info";
        assert_eq!(parse_glsl_error_line(log_no_match), None);
    }

    #[test]
    fn test_parse_glsl_error_line_edge_cases() {
        // Different line and column numbers
        assert_eq!(parse_glsl_error_line("0:1(1): error"), Some((1, 1)));
        assert_eq!(
            parse_glsl_error_line("0:999(9999): error"),
            Some((999, 9999))
        );

        // Multiple matches - should get the first one
        assert_eq!(
            parse_glsl_error_line("0:5(10): error and 0:6(20): another"),
            Some((5, 10))
        );

        // Malformed patterns
        assert_eq!(parse_glsl_error_line("0:12: error"), None); // Missing column
        assert_eq!(parse_glsl_error_line("12(105): error"), None); // Missing first number
        assert_eq!(parse_glsl_error_line("abc:12(105): error"), None); // Non-numeric

        // Empty string
        assert_eq!(parse_glsl_error_line(""), None);

        // Error message with context
        let complex_log = "ERROR: 0:42(256): 'undefined_var' : undeclared identifier";
        assert_eq!(parse_glsl_error_line(complex_log), Some((42, 256)));
    }

    #[test]
    fn test_show_glsl_error_context() {
        let src = r#"void main() {
            gl_Position = vec4(0.0);
            // error here
        }"#;
        // Just ensure it doesn't panic
        show_glsl_error_context(src, 2);
    }

    #[test]
    fn test_show_glsl_error_context_edge_cases() {
        // Empty source
        show_glsl_error_context("", 1);

        // Single line source
        show_glsl_error_context("void main() {}", 1);

        // Error at first line
        let src = "line1\nline2\nline3\nline4\nline5";
        show_glsl_error_context(src, 1);

        // Error at last line
        show_glsl_error_context(src, 5);

        // Error beyond source length (should not panic)
        show_glsl_error_context(src, 100);

        // Very long line (should truncate in display)
        let long_line = "a".repeat(200);
        show_glsl_error_context(&long_line, 1);

        // Multi-line with error in middle
        let multi = "line1\nline2\nline3\nline4\nline5\nline6\nline7";
        show_glsl_error_context(multi, 4);
    }
}
