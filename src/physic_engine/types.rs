use crate::physic_engine::{particle::Particle, rocket::Rocket};

// ------------------------
// UpdateResult
// ------------------------
pub struct UpdateResult<'a> {
    pub new_rocket: Option<Rocket>,
    pub triggered_explosions: &'a [Particle],
}
