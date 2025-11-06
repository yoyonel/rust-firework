use gl::types::*;
use std::{ffi::CString, ptr};

/// # Safety
/// This function is unsafe because it interacts with raw OpenGL pointers and requires
/// the caller to ensure that the provided shader source strings are valid for the duration
/// of the shader compilation process.
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
                panic!(
                    "Shader compilation failed: {}",
                    String::from_utf8_lossy(&buf)
                );
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
            panic!("Shader link failed: {}", String::from_utf8_lossy(&buf));
        }

        gl::DeleteShader(vs);
        gl::DeleteShader(fs);
    }
    program
}
