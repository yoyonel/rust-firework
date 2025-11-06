use crate::physic_engine::particle::Particle;
use crate::physic_engine::rocket::Rocket;

pub const NB_PARTICLES_PER_EXPLOSION: usize = 256;
pub const NB_PARTICLES_PER_TRAIL: usize = 64;

// ------------------------
// UpdateResult
// ------------------------
pub struct UpdateResult<'a> {
    pub new_rocket: Option<Rocket>,
    pub explosions: &'a [Particle],
}
