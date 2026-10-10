use fireworks_sim::physic_engine::particle::{Particle, ParticleVertexCore};
use fireworks_sim::physic_engine::ParticleType;
use fireworks_sim::renderer_engine::types::ParticleGPU;
use glam::{Vec2, Vec3};

#[test]
fn test_particle_vertex_core_layout() {
    assert_eq!(std::mem::size_of::<ParticleVertexCore>(), 36);
    assert_eq!(std::mem::align_of::<ParticleVertexCore>(), 4);
    assert_eq!(std::mem::offset_of!(ParticleVertexCore, pos), 0);
    assert_eq!(std::mem::offset_of!(ParticleVertexCore, color), 8);
    assert_eq!(std::mem::offset_of!(ParticleVertexCore, life), 20);
    assert_eq!(std::mem::offset_of!(ParticleVertexCore, max_life), 24);
    assert_eq!(std::mem::offset_of!(ParticleVertexCore, size), 28);
    assert_eq!(std::mem::offset_of!(ParticleVertexCore, angle), 32);
}

#[test]
fn test_particle_layout() {
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

#[test]
fn test_particle_gpu_layout() {
    assert_eq!(std::mem::size_of::<ParticleGPU>(), 40);
    assert_eq!(std::mem::align_of::<ParticleGPU>(), 4);
    assert_eq!(std::mem::offset_of!(ParticleGPU, core), 0);
    assert_eq!(std::mem::offset_of!(ParticleGPU, core.pos), 0);
    assert_eq!(std::mem::offset_of!(ParticleGPU, core.color), 8);
    assert_eq!(std::mem::offset_of!(ParticleGPU, core.life), 20);
    assert_eq!(std::mem::offset_of!(ParticleGPU, core.max_life), 24);
    assert_eq!(std::mem::offset_of!(ParticleGPU, core.size), 28);
    assert_eq!(std::mem::offset_of!(ParticleGPU, core.angle), 32);
    assert_eq!(std::mem::offset_of!(ParticleGPU, brightness), 36);
}

#[test]
fn test_particle_to_core_fast_cast_punning() {
    let p = Particle {
        core: ParticleVertexCore {
            pos: Vec2::new(10.0, 20.0),
            color: Vec3::new(0.2, 0.4, 0.6),
            life: 1.5,
            max_life: 3.0,
            size: 8.0,
            angle: 45.0,
        },
        vel: Vec2::new(99.0, 77.0),
        active: true,
        particle_type: ParticleType::Explosion,
    };

    // Fast-cast core pointer
    let core_ptr = &p as *const Particle as *const ParticleVertexCore;
    let core = unsafe { *core_ptr };

    assert_eq!(core.pos, Vec2::new(10.0, 20.0));
    assert_eq!(core.color, Vec3::new(0.2, 0.4, 0.6));
    assert_eq!(core.life, 1.5);
    assert_eq!(core.max_life, 3.0);
    assert_eq!(core.size, 8.0);
    assert_eq!(core.angle, 45.0);

    // Fast-cast ParticleGPU pointer
    let gpu_ptr = &p as *const Particle as *const ParticleGPU;
    let gpu_p = unsafe { *gpu_ptr };

    assert_eq!(gpu_p.core.pos, Vec2::new(10.0, 20.0));
    assert_eq!(gpu_p.core.color, Vec3::new(0.2, 0.4, 0.6));
    assert_eq!(gpu_p.core.life, 1.5);
    assert_eq!(gpu_p.core.max_life, 3.0);
    assert_eq!(gpu_p.core.size, 8.0);
    assert_eq!(gpu_p.core.angle, 45.0);
    // Preuve: brightness lit les octets 36..40 (p.vel.x = 99.0)
    assert_eq!(gpu_p.brightness, 99.0);
}
