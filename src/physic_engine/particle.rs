use crate::renderer_engine::ParticleGPU;
use glam::{Vec2, Vec4 as Color};

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default)]
pub struct Particle {
    // Public
    pub pos: Vec2,
    pub color: Color,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,

    // TODO: Make private
    pub vel: Vec2,
    pub active: bool,
}

use bytemuck::{Pod, Zeroable};

unsafe impl Pod for Particle {}
unsafe impl Zeroable for Particle {}

impl Particle {
    #[inline(always)]
    pub fn to_particle_gpu(&self) -> ParticleGPU {
        ParticleGPU {
            pos_x: self.pos.x,
            pos_y: self.pos.y,
            col_r: self.color.x,
            col_g: self.color.y,
            col_b: self.color.z,
            life: self.life,
            max_life: self.max_life,
            size: self.size,
        }
    }
}
