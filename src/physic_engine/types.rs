use crate::physic_engine::{particle::Particle, rocket::Rocket};

// ------------------------
// UpdateResult
// ------------------------
pub struct UpdateResult<'a> {
    pub new_rocket: Option<Rocket>,
    pub triggered_explosions: &'a [Particle],
    pub triggered_explosion_ids: &'a [u64],
    pub anticipated_rocket_launch: Option<(u64, glam::Vec2)>,
    pub anticipated_explosions: &'a [(u64, glam::Vec2)],
}
