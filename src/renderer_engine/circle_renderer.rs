use std::ptr;
use crate::renderer_engine::shader::compile_shader_program_from_files;

pub struct CircleGPURenderer {
    shader_program: u32,
    vao: u32,
    vbo_quad: u32,
    vbo_instances: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CircleGPUData {
    pub center: [f32; 2],
    pub radius: f32,
    pub color: [f32; 4],
    pub thickness: f32,
}

impl CircleGPURenderer {
    pub fn new() -> Self {
        unsafe {
            // Compile shaders
            let shader_program = compile_shader_program_from_files(
                "assets/shaders/circle.vert.glsl",
                "assets/shaders/circle.frag.glsl"
            );

            // Bind global data uniform block to binding point 0 (matches particles)
            let block_idx = gl::GetUniformBlockIndex(shader_program, crate::cstr!("GlobalData"));
            if block_idx != gl::INVALID_INDEX {
                gl::UniformBlockBinding(shader_program, block_idx, 0);
            }

            // Quad vertices (-0.5 to 0.5 to have the coordinate system centered on UV coords)
            const QUAD_VERTICES: [f32; 8] = [
                -0.5, -0.5,
                 0.5, -0.5,
                -0.5,  0.5,
                 0.5,  0.5,
            ];

            let mut vao = 0;
            let mut vbo_quad = 0;
            let mut vbo_instances = 0;

            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo_quad);
            gl::GenBuffers(1, &mut vbo_instances);

            gl::BindVertexArray(vao);

            // 1. Quad vertices
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_quad);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (QUAD_VERTICES.len() * std::mem::size_of::<f32>()) as isize,
                QUAD_VERTICES.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());

            // 2. Instance data VBO (Center, Radius, Color, Thickness)
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_instances);
            let stride = std::mem::size_of::<CircleGPUData>() as i32;

            // Attribute 1: Center (vec2)
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, 0 as *const _);
            gl::VertexAttribDivisor(1, 1); // 1 per instance

            // Attribute 2: Radius (float)
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(2, 1, gl::FLOAT, gl::FALSE, stride, 8 as *const _);
            gl::VertexAttribDivisor(2, 1);

            // Attribute 3: Color (vec4)
            gl::EnableVertexAttribArray(3);
            gl::VertexAttribPointer(3, 4, gl::FLOAT, gl::FALSE, stride, 12 as *const _);
            gl::VertexAttribDivisor(3, 1);

            // Attribute 4: Thickness (float)
            gl::EnableVertexAttribArray(4);
            gl::VertexAttribPointer(4, 1, gl::FLOAT, gl::FALSE, stride, 28 as *const _);
            gl::VertexAttribDivisor(4, 1);

            gl::BindVertexArray(0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);

            Self {
                shader_program,
                vao,
                vbo_quad,
                vbo_instances,
            }
        }
    }

    pub unsafe fn draw(&mut self, circles: &[CircleGPUData]) {
        if circles.is_empty() {
            return;
        }

        // Upload instance data to GPU
        gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_instances);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (circles.len() * std::mem::size_of::<CircleGPUData>()) as isize,
            circles.as_ptr() as *const _,
            gl::STREAM_DRAW,
        );

        // Save current OpenGL states
        let mut depth_test_enabled = 0;
        gl::GetIntegerv(gl::DEPTH_TEST, &mut depth_test_enabled);
        let mut cull_face_enabled = 0;
        gl::GetIntegerv(gl::CULL_FACE, &mut cull_face_enabled);
        let mut blend_enabled = 0;
        gl::GetIntegerv(gl::BLEND, &mut blend_enabled);

        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        gl::UseProgram(self.shader_program);
        gl::BindVertexArray(self.vao);

        // Draw instanced TRIANGLE_STRIP quads
        gl::DrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, circles.len() as i32);

        gl::BindVertexArray(0);
        gl::UseProgram(0);

        // Restore OpenGL states
        if depth_test_enabled == gl::TRUE as i32 {
            gl::Enable(gl::DEPTH_TEST);
        }
        if cull_face_enabled == gl::TRUE as i32 {
            gl::Enable(gl::CULL_FACE);
        }
        if blend_enabled != gl::TRUE as i32 {
            gl::Disable(gl::BLEND);
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            if self.vao != 0 {
                gl::DeleteVertexArrays(1, &self.vao);
                self.vao = 0;
            }
            if self.vbo_quad != 0 {
                gl::DeleteBuffers(1, &self.vbo_quad);
                self.vbo_quad = 0;
            }
            if self.vbo_instances != 0 {
                gl::DeleteBuffers(1, &self.vbo_instances);
                self.vbo_instances = 0;
            }
            if self.shader_program != 0 {
                gl::DeleteProgram(self.shader_program);
                self.shader_program = 0;
            }
        }
    }
}

impl Drop for CircleGPURenderer {
    fn drop(&mut self) {
        self.destroy();
    }
}
