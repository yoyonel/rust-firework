pub mod r#trait;
pub use r#trait::PhysicEngine;
pub use r#trait::PhysicEngineFull;
pub use r#trait::PhysicEngineIterator;

pub mod types;
pub use self::types::UpdateResult;

pub mod rocket;
pub use self::rocket::Rocket;

pub mod particles_pools;
pub use self::particles_pools::ParticlesPool;

pub mod particle;
pub use self::particle::Particle;

pub mod config;
pub use self::config::PhysicConfig;

// pub mod physic_engine_static_aos;
pub mod physic_engine_generational_arena;
