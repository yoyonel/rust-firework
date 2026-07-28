use crate::renderer_engine::shader::compile_shader_program_from_files;
use std::ptr;

pub struct CircleGPURenderer {
    shader_program: u32,
    vao: u32,
    vao_orbits: u32,
    vbo_quad: u32,
    vbo_unit_circle: u32,
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

impl Default for CircleGPURenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl CircleGPURenderer {
    pub fn new() -> Self {
        unsafe {
            // Compile shaders
            let shader_program = compile_shader_program_from_files(
                "assets/shaders/circle.vert.glsl",
                "assets/shaders/circle.frag.glsl",
            );

            // Bind global data uniform block to binding point 0 (matches particles)
            let block_idx = gl::GetUniformBlockIndex(shader_program, crate::cstr!("GlobalData"));
            if block_idx != gl::INVALID_INDEX {
                gl::UniformBlockBinding(shader_program, block_idx, 0);
            }

            // 1. Quad vertices for filled disks (-0.5 to 0.5 to center on UV)
            const QUAD_VERTICES: [f32; 8] = [-0.5, -0.5, 0.5, -0.5, -0.5, 0.5, 0.5, 0.5];

            // 2. Circle vertices for outlines (LINE_LOOP - 64 segments, radius 0.5 to match quad UV scale)
            let mut unit_circle_vertices = Vec::with_capacity(64 * 2);
            for i in 0..64 {
                let angle = 2.0 * std::f32::consts::PI * (i as f32) / 64.0;
                unit_circle_vertices.push(0.5 * angle.cos());
                unit_circle_vertices.push(0.5 * angle.sin());
            }

            let mut vao = 0;
            let mut vao_orbits = 0;
            let mut vbo_quad = 0;
            let mut vbo_unit_circle = 0;
            let mut vbo_instances = 0;

            gl::GenVertexArrays(1, &mut vao);
            gl::GenVertexArrays(1, &mut vao_orbits);
            gl::GenBuffers(1, &mut vbo_quad);
            gl::GenBuffers(1, &mut vbo_unit_circle);
            gl::GenBuffers(1, &mut vbo_instances);

            let stride = std::mem::size_of::<CircleGPUData>() as i32;

            // ==================== VAO FOR FILLED DISKS (QUADS) ====================
            gl::BindVertexArray(vao);

            // Bind static Quad VBO
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_quad);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (QUAD_VERTICES.len() * std::mem::size_of::<f32>()) as isize,
                QUAD_VERTICES.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());

            // Bind dynamic Instances VBO
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_instances);

            // Attribute 1: Center (vec2)
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, ptr::null());
            gl::VertexAttribDivisor(1, 1);

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

            // ==================== VAO FOR OUTLINE ORBITS (LINE LOOP) ====================
            gl::BindVertexArray(vao_orbits);

            // Bind static Unit Circle VBO
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_unit_circle);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (unit_circle_vertices.len() * std::mem::size_of::<f32>()) as isize,
                unit_circle_vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());

            // Bind dynamic Instances VBO (shares the same buffer!)
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_instances);

            // Attribute 1: Center (vec2)
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, ptr::null());
            gl::VertexAttribDivisor(1, 1);

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
                vao_orbits,
                vbo_quad,
                vbo_unit_circle,
                vbo_instances,
            }
        }
    }

    /// Draws the circles.
    ///
    /// # Safety
    ///
    /// This function performs raw OpenGL calls and binds vertex array buffers, which requires a valid active OpenGL context.
    pub unsafe fn draw(&mut self, orbits: &[CircleGPUData], discs: &[CircleGPUData]) {
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

        // 1. Draw outline orbits using GL_LINE_LOOP (extremely cheap, no pixel overdraw)
        if !orbits.is_empty() {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_instances);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(orbits) as isize,
                orbits.as_ptr() as *const _,
                gl::STREAM_DRAW,
            );

            gl::BindVertexArray(self.vao_orbits);
            gl::DrawArraysInstanced(gl::LINE_LOOP, 0, 64, orbits.len() as i32);
        }

        // 2. Draw filled discs/quads using GL_TRIANGLE_STRIP
        if !discs.is_empty() {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_instances);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(discs) as isize,
                discs.as_ptr() as *const _,
                gl::STREAM_DRAW,
            );

            gl::BindVertexArray(self.vao);
            gl::DrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, discs.len() as i32);
        }

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
            if self.vao_orbits != 0 {
                gl::DeleteVertexArrays(1, &self.vao_orbits);
                self.vao_orbits = 0;
            }
            if self.vbo_quad != 0 {
                gl::DeleteBuffers(1, &self.vbo_quad);
                self.vbo_quad = 0;
            }
            if self.vbo_unit_circle != 0 {
                gl::DeleteBuffers(1, &self.vbo_unit_circle);
                self.vbo_unit_circle = 0;
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
