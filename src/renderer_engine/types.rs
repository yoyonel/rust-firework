use gl::types::*;
use memoffset::offset_of;
use std::mem;

/// Structure envoyée au GPU
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ParticleGPU {
    pub pos_x: f32,
    pub pos_y: f32,
    pub col_r: f32,
    pub col_g: f32,
    pub col_b: f32,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
}

impl ParticleGPU {
    pub fn setup_vertex_attribs() {
        let stride = mem::size_of::<Self>() as GLsizei;

        unsafe {
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(Self, pos_x) as *const _,
            );
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(Self, col_r) as *const _,
            );
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(
                2,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(Self, life) as *const _,
            );
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(
                3,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(Self, max_life) as *const _,
            );
            gl::EnableVertexAttribArray(3);
            gl::VertexAttribPointer(
                4,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(Self, size) as *const _,
            );
            gl::EnableVertexAttribArray(4);
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SmokeGPU {
    pub pos_x: f32,
    pub pos_y: f32,
    pub size: f32,  // taille du sprite
    pub alpha: f32, // opacité pour fade
}
