use gl::types::*;
use memoffset::offset_of;
use std::mem;

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
/// | `0`       | `vec2` | `pos_x`, `pos_y`          |
/// | `1`       | `vec3` | `col_r`, `col_g`, `col_b` |
/// | `2`       | `float`| `life`                    |
/// | `3`       | `float`| `max_life`                |
/// | `4`       | `float`| `size`                    |
#[repr(C)] // garantit un layout C-compatible pour l’envoi GPU
#[derive(Debug, Clone, Copy, Default)]
pub struct ParticleGPU {
    /// Position horizontale de la particule.
    pub pos_x: f32,

    /// Position verticale de la particule.
    pub pos_y: f32,

    /// Composante rouge de la couleur.
    pub col_r: f32,

    /// Composante verte de la couleur.
    pub col_g: f32,

    /// Composante bleue de la couleur.
    pub col_b: f32,

    /// Durée de vie actuelle de la particule.
    pub life: f32,

    /// Durée de vie maximale (utilisée pour normaliser l’animation).
    pub max_life: f32,

    /// Taille de la particule à l’écran.
    pub size: f32,
}

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
                offset_of!(Self, pos_x) as *const _,
            );
            gl::EnableVertexAttribArray(0);

            // Attribut 1 : couleur (r, g, b)
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(Self, col_r) as *const _,
            );
            gl::EnableVertexAttribArray(1);

            // Attribut 2 : vie actuelle
            gl::VertexAttribPointer(
                2,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(Self, life) as *const _,
            );
            gl::EnableVertexAttribArray(2);

            // Attribut 3 : vie maximale
            gl::VertexAttribPointer(
                3,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(Self, max_life) as *const _,
            );
            gl::EnableVertexAttribArray(3);

            // Attribut 4 : taille
            gl::VertexAttribPointer(
                4,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(Self, size) as *const _,
            );
            gl::EnableVertexAttribArray(4);
        }
    }
}

/// Structure envoyée au GPU représentant une particule de fumée.
///
/// Utilisée pour les effets de type "smoke" ou "dust".
///
/// # Layout mémoire GPU
///
/// ```text
/// ┌────────────────────────────┐
/// │         SmokeGPU           │
/// ├────────────┬───────────────┤
/// │ pos_x (f32)│ pos_y (f32)   │
/// │ size  (f32)│ alpha (f32)   │
/// └────────────┴───────────────┘
///
/// Stride total : 4 × f32 = 16 octets
///
/// Attributs possibles (exemple) :
/// — 0 → vec2 position (pos_x, pos_y)
/// — 1 → float size
/// — 2 → float alpha
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SmokeGPU {
    /// Position horizontale du centre de la particule.
    pub pos_x: f32,

    /// Position verticale du centre de la particule.
    pub pos_y: f32,

    /// Taille du sprite de fumée.
    pub size: f32,

    /// Opacité (alpha) pour les effets de fade.
    pub alpha: f32,
}
