use crate::physic_engine::ParticleType;
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
    pub angle: f32,
    pub particle_type: ParticleType,
}

use bytemuck::{Pod, Zeroable};

unsafe impl Pod for Particle {}
unsafe impl Zeroable for Particle {}
