use crate::renderer_engine::gl_resource::{GlBuffer, GlProgram, GlVao};
use crate::renderer_engine::shader::compile_shader_program_from_files;
use std::ptr;

pub struct CircleGPURenderer {
    shader_program: GlProgram,
    vao: GlVao,
    vao_orbits: GlVao,
    vao_boxes: GlVao,
    vbo_quad: GlBuffer,
    vbo_unit_circle: GlBuffer,
    vbo_box: GlBuffer,
    vbo_instances: GlBuffer,
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
            use crate::renderer_engine::constants;

            // Compile shaders
            let shader_program = compile_shader_program_from_files(
                constants::SHADER_CIRCLE_VERTEX_PATH,
                constants::SHADER_CIRCLE_FRAGMENT_PATH,
            );
            let shader_program = GlProgram::from_raw(shader_program);

            // Bind global data uniform block to binding point (matches particles)
            let block_idx =
                gl::GetUniformBlockIndex(shader_program.raw(), crate::cstr!("GlobalData"));
            if block_idx != gl::INVALID_INDEX {
                gl::UniformBlockBinding(
                    shader_program.raw(),
                    block_idx,
                    constants::GLOBAL_UBO_BINDING_INDEX,
                );
            }

            // 1. Quad vertices for filled disks (-0.5 to 0.5 to center on UV)
            let quad_vertices = constants::QUAD_VERTICES;

            // 2. Circle vertices for outlines (LINE_LOOP - 64 segments, radius 0.5 to match quad UV scale)
            let segments = constants::CIRCLE_OUTLINE_SEGMENTS;
            let radius_mult = constants::CIRCLE_RADIUS_MULTIPLIER;
            let mut unit_circle_vertices = Vec::with_capacity(segments * 2);
            for i in 0..segments {
                let angle = 2.0 * std::f32::consts::PI * (i as f32) / (segments as f32);
                unit_circle_vertices.push(radius_mult * angle.cos());
                unit_circle_vertices.push(radius_mult * angle.sin());
            }

            let mut raw_vao = 0;
            let mut raw_vao_orbits = 0;
            let mut raw_vao_boxes = 0;
            let mut raw_vbo_quad = 0;
            let mut raw_vbo_unit_circle = 0;
            let mut raw_vbo_box = 0;
            let mut raw_vbo_instances = 0;

            gl::GenVertexArrays(1, &mut raw_vao);
            gl::GenVertexArrays(1, &mut raw_vao_orbits);
            gl::GenVertexArrays(1, &mut raw_vao_boxes);
            gl::GenBuffers(1, &mut raw_vbo_quad);
            gl::GenBuffers(1, &mut raw_vbo_unit_circle);
            gl::GenBuffers(1, &mut raw_vbo_box);
            gl::GenBuffers(1, &mut raw_vbo_instances);

            let vao = GlVao::from_raw(raw_vao);
            let vao_orbits = GlVao::from_raw(raw_vao_orbits);
            let vao_boxes = GlVao::from_raw(raw_vao_boxes);
            let vbo_quad = GlBuffer::from_raw(raw_vbo_quad);
            let vbo_unit_circle = GlBuffer::from_raw(raw_vbo_unit_circle);
            let vbo_box = GlBuffer::from_raw(raw_vbo_box);
            let vbo_instances = GlBuffer::from_raw(raw_vbo_instances);

            let stride = std::mem::size_of::<CircleGPUData>() as i32;

            // ==================== VAO FOR FILLED DISKS (QUADS) ====================
            gl::BindVertexArray(vao.raw());

            // Bind static Quad VBO
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_quad.raw());
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (quad_vertices.len() * std::mem::size_of::<f32>()) as isize,
                quad_vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());

            // Bind dynamic Instances VBO
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_instances.raw());

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
            gl::BindVertexArray(vao_orbits.raw());

            // Bind static Unit Circle VBO
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_unit_circle.raw());
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (unit_circle_vertices.len() * std::mem::size_of::<f32>()) as isize,
                unit_circle_vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());

            // Bind dynamic Instances VBO (shares the same buffer!)
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_instances.raw());

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

            // ==================== VAO FOR OUTLINE BOXES (LINE LOOP) ====================
            gl::BindVertexArray(vao_boxes.raw());

            // Bind static Box VBO
            let box_vertices = constants::BOX_OUTLINE_VERTICES;
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_box.raw());
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (box_vertices.len() * std::mem::size_of::<f32>()) as isize,
                box_vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());

            // Bind dynamic Instances VBO (shares the same buffer!)
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_instances.raw());

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
                vao_boxes,
                vbo_quad,
                vbo_unit_circle,
                vbo_box,
                vbo_instances,
            }
        }
    }

    /// Draws the circles.
    ///
    /// # Safety
    ///
    /// This function performs raw OpenGL calls and binds vertex array buffers, which requires a valid active OpenGL context.
    pub unsafe fn draw(
        &mut self,
        orbits: &[CircleGPUData],
        discs: &[CircleGPUData],
        dashed_boxes: &[CircleGPUData],
    ) {
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

        gl::UseProgram(self.shader_program.raw());

        // 1. Draw outline orbits using GL_LINE_LOOP (extremely cheap, no pixel overdraw)
        if !orbits.is_empty() {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_instances.raw());
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(orbits) as isize,
                orbits.as_ptr() as *const _,
                gl::STREAM_DRAW,
            );

            gl::BindVertexArray(self.vao_orbits.raw());
            gl::DrawArraysInstanced(gl::LINE_LOOP, 0, 64, orbits.len() as i32);
        }

        // 2. Draw filled discs/quads using GL_TRIANGLE_STRIP
        if !discs.is_empty() {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_instances.raw());
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(discs) as isize,
                discs.as_ptr() as *const _,
                gl::STREAM_DRAW,
            );

            gl::BindVertexArray(self.vao.raw());
            gl::DrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, discs.len() as i32);
        }

        // 3. Draw dashed outline boxes using GL_LINE_LOOP
        if !dashed_boxes.is_empty() {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_instances.raw());
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(dashed_boxes) as isize,
                dashed_boxes.as_ptr() as *const _,
                gl::STREAM_DRAW,
            );

            gl::BindVertexArray(self.vao_boxes.raw());
            gl::DrawArraysInstanced(gl::LINE_LOOP, 0, 4, dashed_boxes.len() as i32);
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
        self.vao.reset(0);
        self.vao_orbits.reset(0);
        self.vao_boxes.reset(0);
        self.vbo_quad.reset(0);
        self.vbo_unit_circle.reset(0);
        self.vbo_box.reset(0);
        self.vbo_instances.reset(0);
        self.shader_program.reset(0);
    }
}

impl Drop for CircleGPURenderer {
    fn drop(&mut self) {
        self.destroy();
    }
}
