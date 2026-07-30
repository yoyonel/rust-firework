/// Audio Event Overlay Renderer
///
/// Renders shader-based visual indicators (ripple rings + beam lines) for
/// audio debug mode. All animation math runs on the GPU; the CPU only uploads
/// a compact per-event instance buffer once per frame.
///
/// # GPU architecture
/// - 1 VAO / 1 shared VBO for ripple rings (instanced TRIANGLE_STRIP quads)
/// - 1 VAO / 1 shared VBO for beam lines (instanced GL_LINES, 2 verts/instance)
/// - Both share the same shader program and the same instance data layout.
///   The beam pass simply ignores the quad aQuad attribute and uses the
///   line VBO instead.
/// - 2 draw calls per frame regardless of the number of active events.
use crate::renderer_engine::shader::compile_shader_program_from_files;
use std::ptr;

// ── GPU instance layout ─────────────────────────────────────────────────────

/// Per-event data uploaded to the GPU every frame via STREAM_DRAW.
/// Repr(C) + manually computed byte offsets → matches shader `layout(location=N)`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AudioEventGPUData {
    /// World position of the audio event (rocket/explosion position).
    pub pos: [f32; 2],
    /// Age of this event in seconds [0 .. ttl].
    pub age: f32,
    /// Total lifetime in seconds (controls ring expansion speed).
    pub ttl: f32,
    /// 0.0 = Launch (green), 1.0 = Explosion (orange-red).
    pub kind: f32,
    /// World position of the audio listener (for beam line destination).
    pub listener: [f32; 2],
    /// Padding to align the struct to a 4-float (16-byte) boundary.
    pub _pad: f32,
}

impl AudioEventGPUData {
    /// Total byte size.
    const STRIDE: i32 = std::mem::size_of::<Self>() as i32;

    // Byte offsets of each field (manually matching repr(C) layout)
    const OFF_POS: usize = 0;
    const OFF_AGE: usize = 8;
    const OFF_TTL: usize = 12;
    const OFF_KIND: usize = 16;
    const OFF_LISTENER: usize = 20;
}

// ── CPU event bookkeeping ────────────────────────────────────────────────────

/// Kind of audio event to visualise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioEventKind {
    Launch,
    Explosion,
}

impl AudioEventKind {
    fn as_gpu_kind(self) -> f32 {
        match self {
            AudioEventKind::Launch => 0.0,
            AudioEventKind::Explosion => 1.0,
        }
    }

    /// Total lifetime in seconds of the visual effect.
    pub fn ttl_secs(self) -> f32 {
        match self {
            AudioEventKind::Launch => {
                crate::renderer_engine::constants::AUDIO_EVENT_LAUNCH_TTL_SECS
            }
            AudioEventKind::Explosion => {
                crate::renderer_engine::constants::AUDIO_EVENT_EXPLOSION_TTL_SECS
            }
        }
    }
}

/// A single live audio event tracked by the CPU.
#[derive(Clone, Debug)]
pub struct AudioEvent {
    pub pos: glam::Vec2,
    pub kind: AudioEventKind,
    /// Age in seconds (incremented each frame by `dt`).
    pub age: f32,
}

impl AudioEvent {
    pub fn new(pos: glam::Vec2, kind: AudioEventKind) -> Self {
        Self {
            pos,
            kind,
            age: 0.0,
        }
    }

    /// Returns true when this event should be removed from the pool.
    pub fn is_expired(&self) -> bool {
        self.age >= self.kind.ttl_secs()
    }

    /// Pack into the GPU-ready struct for upload.
    pub fn to_gpu(&self, listener: glam::Vec2) -> AudioEventGPUData {
        AudioEventGPUData {
            pos: self.pos.to_array(),
            age: self.age,
            ttl: self.kind.ttl_secs(),
            kind: self.kind.as_gpu_kind(),
            listener: listener.to_array(),
            _pad: 0.0,
        }
    }
}

// ── Renderer ─────────────────────────────────────────────────────────────────

pub struct AudioEventRenderer {
    shader_program: u32,
    /// Cached location of the `uMode` uniform (0=ring, 1=beam).
    umode_loc: i32,

    // Ripple rings: instanced quad (TRIANGLE_STRIP, 4 verts)
    vao_rings: u32,
    vbo_quad: u32,

    // Beam lines: instanced line (GL_LINES, 2 verts per instance)
    vao_beams: u32,
    vbo_line: u32,

    // Shared per-instance buffer (STREAM_DRAW, updated every frame)
    vbo_instances: u32,
}

impl Default for AudioEventRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEventRenderer {
    pub fn new() -> Self {
        unsafe {
            let shader_program = compile_shader_program_from_files(
                "assets/shaders/audio_event.vert.glsl",
                "assets/shaders/audio_event.frag.glsl",
            );

            // Bind shared GlobalData UBO to binding point 0 (same as particles)
            let block_idx = gl::GetUniformBlockIndex(shader_program, crate::cstr!("GlobalData"));
            if block_idx != gl::INVALID_INDEX {
                gl::UniformBlockBinding(shader_program, block_idx, 0);
            }

            // Cache the uMode uniform location (set per-draw-pass to 0=ring or 1=beam)
            let umode_loc = gl::GetUniformLocation(shader_program, crate::cstr!("uMode"));

            // ── Static geometry buffers ───────────────────────────────────

            // Quad: 4 vertices for a centered unit quad [-0.5, 0.5]
            const QUAD: [f32; 8] = [-0.5, -0.5, 0.5, -0.5, -0.5, 0.5, 0.5, 0.5];

            // Line: 2 vertices — will be transformed in the vertex shader.
            // We encode the two endpoints as (0,0) and (1,0); the shader
            // uses aQuad.x to lerp between aPos and aListener.
            const LINE: [f32; 4] = [0.0, 0.0, 1.0, 0.0];

            let mut vao_rings = 0u32;
            let mut vao_beams = 0u32;
            let mut vbo_quad = 0u32;
            let mut vbo_line = 0u32;
            let mut vbo_instances = 0u32;

            gl::GenVertexArrays(1, &mut vao_rings);
            gl::GenVertexArrays(1, &mut vao_beams);
            gl::GenBuffers(1, &mut vbo_quad);
            gl::GenBuffers(1, &mut vbo_line);
            gl::GenBuffers(1, &mut vbo_instances);

            // Upload static geometry
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_quad);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (QUAD.len() * 4) as isize,
                QUAD.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_line);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (LINE.len() * 4) as isize,
                LINE.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            let stride = AudioEventGPUData::STRIDE;

            // ── VAO: ripple rings ─────────────────────────────────────────
            gl::BindVertexArray(vao_rings);

            // attr 0: quad vertex position (vec2) — from static vbo_quad
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_quad);
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());

            // Instance attrs from vbo_instances
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_instances);
            Self::setup_instance_attribs(stride);

            // ── VAO: beam lines ───────────────────────────────────────────
            gl::BindVertexArray(vao_beams);

            // attr 0: line vertex position (vec2) — x encodes [0,1] lerp
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_line);
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, ptr::null());

            // Same instance attrs
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_instances);
            Self::setup_instance_attribs(stride);

            gl::BindVertexArray(0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);

            Self {
                shader_program,
                umode_loc,
                vao_rings,
                vbo_quad,
                vao_beams,
                vbo_line,
                vbo_instances,
            }
        }
    }

    /// Set up per-instance vertex attribute pointers (locations 1..=5).
    /// Assumes `vbo_instances` is already bound to ARRAY_BUFFER.
    unsafe fn setup_instance_attribs(stride: i32) {
        // loc 1: pos (vec2)
        gl::EnableVertexAttribArray(1);
        gl::VertexAttribPointer(
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            stride,
            AudioEventGPUData::OFF_POS as *const _,
        );
        gl::VertexAttribDivisor(1, 1);

        // loc 2: age (float)
        gl::EnableVertexAttribArray(2);
        gl::VertexAttribPointer(
            2,
            1,
            gl::FLOAT,
            gl::FALSE,
            stride,
            AudioEventGPUData::OFF_AGE as *const _,
        );
        gl::VertexAttribDivisor(2, 1);

        // loc 3: ttl (float)
        gl::EnableVertexAttribArray(3);
        gl::VertexAttribPointer(
            3,
            1,
            gl::FLOAT,
            gl::FALSE,
            stride,
            AudioEventGPUData::OFF_TTL as *const _,
        );
        gl::VertexAttribDivisor(3, 1);

        // loc 4: kind (float)
        gl::EnableVertexAttribArray(4);
        gl::VertexAttribPointer(
            4,
            1,
            gl::FLOAT,
            gl::FALSE,
            stride,
            AudioEventGPUData::OFF_KIND as *const _,
        );
        gl::VertexAttribDivisor(4, 1);

        // loc 5: listener (vec2)
        gl::EnableVertexAttribArray(5);
        gl::VertexAttribPointer(
            5,
            2,
            gl::FLOAT,
            gl::FALSE,
            stride,
            AudioEventGPUData::OFF_LISTENER as *const _,
        );
        gl::VertexAttribDivisor(5, 1);
    }

    /// Draw all active audio event indicators.
    ///
    /// # Arguments
    /// * `instances` — GPU-ready slice, built from `AudioEvent::to_gpu()` each frame.
    ///
    /// # Safety
    /// Requires a valid current OpenGL context.
    pub unsafe fn draw(&mut self, instances: &[AudioEventGPUData]) {
        if instances.is_empty() {
            return;
        }

        // ── Save & override OpenGL state ──────────────────────────────────
        let mut depth_test = 0i32;
        let mut cull_face = 0i32;
        let mut blend = 0i32;
        gl::GetIntegerv(gl::DEPTH_TEST, &mut depth_test);
        gl::GetIntegerv(gl::CULL_FACE, &mut cull_face);
        gl::GetIntegerv(gl::BLEND, &mut blend);

        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::CULL_FACE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        gl::UseProgram(self.shader_program);

        // ── Upload instance data (one STREAM_DRAW per frame) ─────────────
        gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_instances);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            std::mem::size_of_val(instances) as isize,
            instances.as_ptr() as *const _,
            gl::STREAM_DRAW,
        );

        // ── Pass 1: ripple rings (TRIANGLE_STRIP quads) ───────────────────
        gl::Uniform1i(self.umode_loc, 0); // ring mode
        gl::BindVertexArray(self.vao_rings);
        gl::DrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, instances.len() as i32);

        // ── Pass 2: beam lines (GL_LINES, 2 verts per instance) ──────────
        gl::Uniform1i(self.umode_loc, 1); // beam mode
        gl::BindVertexArray(self.vao_beams);
        gl::DrawArraysInstanced(gl::LINES, 0, 2, instances.len() as i32);

        // ── Restore state ─────────────────────────────────────────────────
        gl::BindVertexArray(0);
        gl::UseProgram(0);
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);

        if depth_test == gl::TRUE as i32 {
            gl::Enable(gl::DEPTH_TEST);
        }
        if cull_face == gl::TRUE as i32 {
            gl::Enable(gl::CULL_FACE);
        }
        if blend != gl::TRUE as i32 {
            gl::Disable(gl::BLEND);
        }
    }

    pub fn destroy(&mut self) {
        unsafe {
            for vao in [self.vao_rings, self.vao_beams] {
                if vao != 0 {
                    gl::DeleteVertexArrays(1, &vao);
                }
            }
            for vbo in [self.vbo_quad, self.vbo_line, self.vbo_instances] {
                if vbo != 0 {
                    gl::DeleteBuffers(1, &vbo);
                }
            }
            if self.shader_program != 0 {
                gl::DeleteProgram(self.shader_program);
                self.shader_program = 0;
            }
        }
        self.vao_rings = 0;
        self.vao_beams = 0;
        self.vbo_quad = 0;
        self.vbo_line = 0;
        self.vbo_instances = 0;
    }
}

impl Drop for AudioEventRenderer {
    fn drop(&mut self) {
        self.destroy();
    }
}
