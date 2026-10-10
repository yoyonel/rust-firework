use gl::types::*;
use std::mem;

pub use crate::physic_engine::particle::ParticleVertexCore;

/// Structure envoyée au GPU représentant une particule.
///
/// Chaque instance de `ParticleGPU` correspond à un *vertex* (ou une particule)
/// stockée dans un *Vertex Buffer Object (VBO)* et transmise au *Vertex Shader*.
///
/// Les champs sont organisés de manière à correspondre aux attributs de sommets
/// utilisés dans le shader : position, couleur, vie, etc.
///
/// # Layout mémoire GPU
///
/// Voici comment les données de `ParticleGPU` sont interprétées par OpenGL :
///
///
/// | Champ   | Type  | Description           | Attribut GPU |
/// |----------|-------|----------------------|---------------|
/// | `pos_x`  | `f32` | Position horizontale | `location = 0` |
/// | `pos_y`  | `f32` | Position verticale   | `location = 0` |
/// | `size`   | `f32` | Taille du sprite     | `location = 1` |
/// | `alpha`  | `f32` | Opacité              | `location = 2` |
///
/// **Stride total** : `4 × f32 = 16 octets`
/// # Attributs GPU
///
/// | Location | Type   | Champs                     |
/// |:---------:|:-------|:---------------------------|
/// |:---------:|:-------|----------------------------|
/// | `0`       | `vec2` | `core.pos`                 |
/// | `1`       | `vec3` | `core.color`               |
/// | `2`       | `vec4` | `life, max_life, size, angle` |
/// | `3`       | `float`| `brightness`               |
#[repr(C)] // garantit un layout C-compatible pour l’envoi GPU
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ParticleGPU {
    /// Données communes au CPU et au GPU (36 octets).
    pub core: ParticleVertexCore,

    /// Multiplicateur de luminosité pour HDR (1.0 = normal, >1.0 = bloom).
    /// Calculé côté CPU basé sur la vitesse/accélération de la particule.
    pub brightness: f32,
}

const _: () = {
    assert!(std::mem::size_of::<ParticleGPU>() == 40);
    assert!(std::mem::offset_of!(ParticleGPU, core) == 0);
    assert!(std::mem::offset_of!(ParticleGPU, core.pos) == 0);
    assert!(std::mem::offset_of!(ParticleGPU, core.color) == 8);
    assert!(std::mem::offset_of!(ParticleGPU, core.life) == 20);
    assert!(std::mem::offset_of!(ParticleGPU, core.max_life) == 24);
    assert!(std::mem::offset_of!(ParticleGPU, core.size) == 28);
    assert!(std::mem::offset_of!(ParticleGPU, core.angle) == 32);
    assert!(std::mem::offset_of!(ParticleGPU, brightness) == 36);
};

impl ParticleGPU {
    /// Configure les attributs de sommets (vertex attributes) pour OpenGL.
    ///
    /// Chaque appel à `gl::VertexAttribPointer` indique à OpenGL comment lire
    /// les différents champs de `ParticleGPU` dans le buffer mémoire.
    ///
    /// ⚠️ Pré-requis : un *Vertex Array Object (VAO)* doit déjà être lié avant l’appel.
    pub fn setup_vertex_attribs() {
        let stride = mem::size_of::<Self>() as GLsizei;

        unsafe {
            // Attribut 0 : position (x, y)
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                std::mem::offset_of!(Self, core.pos) as *const _,
            );
            gl::EnableVertexAttribArray(0);

            // Attribut 1 : couleur (r, g, b)
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                std::mem::offset_of!(Self, core.color) as *const _,
            );
            gl::EnableVertexAttribArray(1);

            // Attribut 2 : vie actuelle, vie maximale, taille, angle
            gl::VertexAttribPointer(
                2,
                4,
                gl::FLOAT,
                gl::FALSE,
                stride,
                std::mem::offset_of!(Self, core.life) as *const _,
            );
            gl::EnableVertexAttribArray(2);

            // Attribut 3 : brightness (multiplicateur HDR)
            gl::VertexAttribPointer(
                3,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                std::mem::offset_of!(Self, brightness) as *const _,
            );
            gl::EnableVertexAttribArray(3);
        }
    }

    pub fn setup_vertex_attribs_for_instanced_quad() {
        let stride = std::mem::size_of::<Self>() as GLsizei;

        unsafe {
            // layout(location = 1) : position (vec2)
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                std::mem::offset_of!(Self, core.pos) as *const _,
            );
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribDivisor(1, 1); // 🔑 une fois par particule

            // layout(location = 2) : couleur (vec3)
            gl::VertexAttribPointer(
                2,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                std::mem::offset_of!(Self, core.color) as *const _,
            );
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribDivisor(2, 1);

            // layout(location = 3) : vie (float), vie max (float), taille (float), angle (float)
            gl::VertexAttribPointer(
                3,
                4,
                gl::FLOAT,
                gl::FALSE,
                stride,
                std::mem::offset_of!(Self, core.life) as *const _,
            );
            gl::EnableVertexAttribArray(3);
            gl::VertexAttribDivisor(3, 1);

            // layout(location = 4) : brightness (float)
            gl::VertexAttribPointer(
                4,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                std::mem::offset_of!(Self, brightness) as *const _,
            );
            gl::EnableVertexAttribArray(4);
            gl::VertexAttribDivisor(4, 1);
        }
    }
}

/// Point light representation sent to GPU for volumetric smoke in-scattering.
/// Memory layout: position_radius (16 bytes) + color_intensity (16 bytes) = 32 bytes (std140 compatible).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PointLightGPU {
    pub position_radius: [f32; 4], // xyz: world position, w: effective radius
    pub color_intensity: [f32; 4], // rgb: light color, w: light intensity
}

unsafe impl bytemuck::Pod for PointLightGPU {}
unsafe impl bytemuck::Zeroable for PointLightGPU {}

/// std140 uniform block for volumetric lighting.
/// Total size: 16 * 32 (512) + 16 (ambient) + 4 (num_active) + 4 (intensity) + 8 (padding) = 544 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VolumetricLightingBlockGPU {
    pub lights: [PointLightGPU; crate::renderer_engine::constants::MAX_VOLUMETRIC_LIGHTS],
    pub ambient_light: [f32; 4],
    pub num_active_lights: i32,
    pub scattering_intensity: f32,
    pub _padding: [i32; 2],
}

unsafe impl bytemuck::Pod for VolumetricLightingBlockGPU {}
unsafe impl bytemuck::Zeroable for VolumetricLightingBlockGPU {}

impl Default for VolumetricLightingBlockGPU {
    fn default() -> Self {
        Self {
            lights: [PointLightGPU::default();
                crate::renderer_engine::constants::MAX_VOLUMETRIC_LIGHTS],
            ambient_light: [0.08, 0.08, 0.10, 0.0],
            num_active_lights: 0,
            scattering_intensity:
                crate::renderer_engine::constants::DEFAULT_SMOKE_SCATTERING_INTENSITY,
            _padding: [0; 2],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_gpu_memory_layout_and_offsets() {
        assert_eq!(mem::size_of::<ParticleGPU>(), 40);
        assert_eq!(mem::align_of::<ParticleGPU>(), 4);
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
    fn test_volumetric_lighting_block_gpu_layout() {
        assert_eq!(mem::size_of::<PointLightGPU>(), 32);
        assert_eq!(mem::size_of::<VolumetricLightingBlockGPU>(), 544);
        assert_eq!(mem::size_of::<VolumetricLightingBlockGPU>() % 16, 0);
    }
}
