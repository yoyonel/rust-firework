pub mod r#trait;
pub use r#trait::PhysicEngine;

pub mod types;
pub use self::types::{Color, UpdateResult, Vec2};

pub mod rocket;
pub use self::rocket::Rocket;

pub mod particle;
pub use self::particle::Particle;

pub mod config;
pub use self::config::PhysicConfig;

// pub mod physic_engine_static_aos;
pub mod physic_engine_generational_arena;
