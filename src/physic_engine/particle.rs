use crate::physic_engine::ParticleType;
use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3 as Color};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ParticleVertexCore {
    // Aligné exactement sur les 36 premiers octets envoyés au GPU
    pub pos: Vec2,
    pub color: Color,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub angle: f32,
}

unsafe impl Pod for ParticleVertexCore {}
unsafe impl Zeroable for ParticleVertexCore {}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default)]
pub struct Particle {
    pub core: ParticleVertexCore,

    // CPU-only physical fields (at the end)
    pub vel: Vec2,
    pub active: bool,
    pub particle_type: ParticleType,
}

unsafe impl Pod for Particle {}
unsafe impl Zeroable for Particle {}

const _: () = {
    assert!(std::mem::size_of::<ParticleVertexCore>() == 36);
    assert!(std::mem::offset_of!(ParticleVertexCore, pos) == 0);
    assert!(std::mem::offset_of!(ParticleVertexCore, color) == 8);
    assert!(std::mem::offset_of!(ParticleVertexCore, life) == 20);
    assert!(std::mem::offset_of!(ParticleVertexCore, max_life) == 24);
    assert!(std::mem::offset_of!(ParticleVertexCore, size) == 28);
    assert!(std::mem::offset_of!(ParticleVertexCore, angle) == 32);

    assert!(std::mem::size_of::<Particle>() == 48);
    assert!(std::mem::offset_of!(Particle, core) == 0);
    assert!(std::mem::offset_of!(Particle, core.pos) == 0);
    assert!(std::mem::offset_of!(Particle, core.color) == 8);
    assert!(std::mem::offset_of!(Particle, core.life) == 20);
    assert!(std::mem::offset_of!(Particle, core.max_life) == 24);
    assert!(std::mem::offset_of!(Particle, core.size) == 28);
    assert!(std::mem::offset_of!(Particle, core.angle) == 32);
    assert!(std::mem::offset_of!(Particle, vel) == 36);
    assert!(std::mem::offset_of!(Particle, active) == 44);
    assert!(std::mem::offset_of!(Particle, particle_type) == 45);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_memory_layout_and_offsets() {
        assert_eq!(std::mem::size_of::<ParticleVertexCore>(), 36);
        assert_eq!(std::mem::align_of::<ParticleVertexCore>(), 4);
        assert_eq!(std::mem::offset_of!(ParticleVertexCore, pos), 0);
        assert_eq!(std::mem::offset_of!(ParticleVertexCore, color), 8);
        assert_eq!(std::mem::offset_of!(ParticleVertexCore, life), 20);
        assert_eq!(std::mem::offset_of!(ParticleVertexCore, max_life), 24);
        assert_eq!(std::mem::offset_of!(ParticleVertexCore, size), 28);
        assert_eq!(std::mem::offset_of!(ParticleVertexCore, angle), 32);

        assert_eq!(std::mem::size_of::<Particle>(), 48);
        assert_eq!(std::mem::align_of::<Particle>(), 16);
        assert_eq!(std::mem::offset_of!(Particle, core), 0);
        assert_eq!(std::mem::offset_of!(Particle, core.pos), 0);
        assert_eq!(std::mem::offset_of!(Particle, core.color), 8);
        assert_eq!(std::mem::offset_of!(Particle, core.life), 20);
        assert_eq!(std::mem::offset_of!(Particle, core.max_life), 24);
        assert_eq!(std::mem::offset_of!(Particle, core.size), 28);
        assert_eq!(std::mem::offset_of!(Particle, core.angle), 32);
        assert_eq!(std::mem::offset_of!(Particle, vel), 36);
        assert_eq!(std::mem::offset_of!(Particle, active), 44);
        assert_eq!(std::mem::offset_of!(Particle, particle_type), 45);
    }
}
