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

const _: () = {
    assert!(std::mem::size_of::<Particle>() == 48);
    assert!(std::mem::offset_of!(Particle, pos) == 0);
    assert!(std::mem::offset_of!(Particle, color) == 8);
    assert!(std::mem::offset_of!(Particle, life) == 20);
    assert!(std::mem::offset_of!(Particle, angle) == 32);
    assert!(std::mem::offset_of!(Particle, vel) == 36); // ce que lit brightness aujourd'hui
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_memory_layout_and_offsets() {
        assert_eq!(std::mem::size_of::<Particle>(), 48);
        assert_eq!(std::mem::align_of::<Particle>(), 16);
        assert_eq!(std::mem::offset_of!(Particle, pos), 0);
        assert_eq!(std::mem::offset_of!(Particle, color), 8);
        assert_eq!(std::mem::offset_of!(Particle, life), 20);
        assert_eq!(std::mem::offset_of!(Particle, max_life), 24);
        assert_eq!(std::mem::offset_of!(Particle, size), 28);
        assert_eq!(std::mem::offset_of!(Particle, angle), 32);
        assert_eq!(std::mem::offset_of!(Particle, vel), 36);
        assert_eq!(std::mem::offset_of!(Particle, active), 44);
        assert_eq!(std::mem::offset_of!(Particle, particle_type), 45);
    }
}
