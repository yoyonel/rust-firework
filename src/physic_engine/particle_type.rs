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
        match self {
            ParticleType::Rocket => {
                "assets/textures/04ddeae2-7367-45f1-87e0-361d1d242630_scaled.png"
            }
            ParticleType::Explosion => {
                "assets/textures/kenney_particle-pack/PNG (Black background)/circle_05.png"
            }
            ParticleType::Smoke => {
                "assets/textures/kenney_particle-pack/PNG (Black background)/smoke_01.png"
            }
            ParticleType::Trail => {
                "assets/textures/kenney_particle-pack/PNG (Black background)/trace_03.png"
            }
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
