/// Types de particules supportés par le moteur physique et le renderer
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ParticleType {
    /// Particule de fusée (tête de la fusée avant explosion)
    #[default]
    Rocket = 0,
    /// Particule d'explosion (après que la fusée explose)
    Explosion = 1,
    /// Fumée (effets de fumée autour de la fusée)
    Smoke = 2,
    /// Traînée (particules laissées derrière la fusée)
    Trail = 3,
}

impl ParticleType {
    /// Retourne le chemin de la texture par défaut pour ce type de particule
    pub fn default_texture_path(&self) -> &'static str {
        use super::constants;
        match self {
            ParticleType::Rocket => constants::TEXTURE_ROCKET_PATH,
            ParticleType::Explosion => constants::TEXTURE_EXPLOSION_CIRCLE_PATH,
            ParticleType::Smoke => constants::TEXTURE_SMOKE_PATH,
            ParticleType::Trail => constants::TEXTURE_TRAIL_TRACE_PATH,
        }
    }

    /// Retourne une description lisible du type de particule
    pub fn description(&self) -> &'static str {
        match self {
            ParticleType::Rocket => "Rocket head particle",
            ParticleType::Explosion => "Explosion particle",
            ParticleType::Smoke => "Smoke particle",
            ParticleType::Trail => "Trail particle",
        }
    }
}

// Implémentation de Pod et Zeroable pour permettre l'utilisation dans les buffers GPU
use bytemuck::{Pod, Zeroable};

unsafe impl Pod for ParticleType {}
unsafe impl Zeroable for ParticleType {}
