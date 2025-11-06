use crate::physic_engine::types::Vec2;
use crate::renderer_engine::types::ParticleGPU;

// ---------------------------
// Définition des particules smoke
// ---------------------------
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct SmokeParticle {
    pub pos: Vec2,
    // pub vel: Vec2,
    // pub life: f32,
    pub size: f32,
    // pub alpha: f32,
    // pub active: bool,
}

unsafe impl bytemuck::Pod for SmokeParticle {}
unsafe impl bytemuck::Zeroable for SmokeParticle {}

impl Default for SmokeParticle {
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            // vel: Vec2::ZERO,
            // life: 0.0,
            size: 0.0,
            // alpha: 1.0,
            // active: false,
        }
    }
}

// ---------------------------
// Gestionnaire simple
// ---------------------------
pub struct SmokeSystem {
    pub particles: Vec<SmokeParticle>,
}

impl SmokeSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
        }
    }

    /// Spawn quelques particules à une position
    pub fn spawn(&mut self, pos: Vec2) {
        for _ in 0..3 {
            self.particles.push(SmokeParticle {
                pos,
                // vel: Vec2 {
                //     x: rand_range(-0.5, 0.5),
                //     y: rand_range(0.2, 1.0),
                //     _pad: [0.0; 2],
                // },
                // life: 1.0,
                size: rand_range(0.1, 0.3),
                // alpha: 1.0,
                // active: true,
            });
        }
    }

    /// Update physique des particules smoke
    // pub fn update(&mut self, dt: f32) {
    //     self.particles.retain_mut(|p| {
    //         p.pos.x += p.vel.x * dt;
    //         p.pos.y += p.vel.y * dt;
    //         p.alpha -= dt * 0.5;
    //         p.life -= dt;
    //         p.life > 0.0
    //     });
    // }

    /// Convertit les particules CPU en GPU pour ton buffer existant
    pub fn fill_particle_gpu_slice(&self, gpu_slice: &mut [ParticleGPU]) -> usize {
        let n = self.particles.len().min(gpu_slice.len());
        for (i, s) in self.particles.iter().take(n).enumerate() {
            gpu_slice[i] = ParticleGPU {
                pos_x: s.pos.x,
                pos_y: s.pos.y,
                col_r: 0.5,
                col_g: 0.5,
                col_b: 0.5,
                // life: s.life,
                life: 1.0,
                max_life: 1.0,
                size: s.size,
            };
        }
        n
    }

    pub fn smoke_particles_active_slice(&self) -> &[SmokeParticle] {
        // let active_count = self.particles.iter().filter(|p| p.active).count();
        // &self.particles[..active_count]
        &self.particles
    }
}

// ---------------------------
// Fonctions utilitaires
// ---------------------------
fn rand_range(min: f32, max: f32) -> f32 {
    let t: f32 = rand::random();
    min + t * (max - min)
}
