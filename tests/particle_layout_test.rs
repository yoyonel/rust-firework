use fireworks_sim::physic_engine::particle::Particle;
use fireworks_sim::physic_engine::ParticleType;
use fireworks_sim::renderer_engine::types::ParticleGPU;
use glam::{Vec2, Vec3};

#[test]
fn test_particle_to_gpu_cast_memory_overlap() {
    let p = Particle {
        pos: Vec2::new(10.0, 20.0),
        color: Vec3::new(0.2, 0.4, 0.6),
        life: 1.5,
        max_life: 3.0,
        size: 8.0,
        angle: 45.0,
        vel: Vec2::new(99.0, 77.0),
        active: true,
        particle_type: ParticleType::Explosion,
    };

    // SAFETY: On reproduit fidèlement le cast de renderer_graphics.rs:231-232 pour sceller le comportement
    let src_ptr = &p as *const Particle as *const ParticleGPU;
    let gpu_p = unsafe { *src_ptr };

    // 1. Validation de la concordance des 36 premiers octets
    assert_eq!(gpu_p.pos_x, 10.0);
    assert_eq!(gpu_p.pos_y, 20.0);
    assert_eq!(gpu_p.col_r, 0.2);
    assert_eq!(gpu_p.col_g, 0.4);
    assert_eq!(gpu_p.col_b, 0.6);
    assert_eq!(gpu_p.life, 1.5);
    assert_eq!(gpu_p.max_life, 3.0);
    assert_eq!(gpu_p.size, 8.0);
    assert_eq!(gpu_p.angle, 45.0);

    // 2. Preuve concrète F-05 : aux octets 36..40, brightness lit p.vel.x (99.0)
    assert_eq!(gpu_p.brightness, 99.0);
}
