use crate::physic_engine::ParticleType;
use glam::{Vec2, Vec3 as Color};

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default)]
pub struct Particle {
    // Public - Aligné exactement sur les 36 premiers octets de ParticleGPU
    pub pos: Vec2,
    pub color: Color,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub angle: f32,

    // CPU-only physical fields (at the end)
    pub vel: Vec2,
    pub active: bool,
    pub particle_type: ParticleType,
}

use bytemuck::{Pod, Zeroable};

unsafe impl Pod for Particle {}
unsafe impl Zeroable for Particle {}
