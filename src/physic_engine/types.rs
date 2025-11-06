use crate::physic_engine::particle::Particle;
use crate::physic_engine::rocket::Rocket;

pub const NB_PARTICLES_PER_EXPLOSION: usize = 256;
pub const NB_PARTICLES_PER_TRAIL: usize = 64;

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
    pub _pad: [f32; 2], // padding explicite
}

impl Default for Vec2 {
    #[inline]
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            _pad: [0.0; 2],
        }
    }
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            _pad: [0.0; 2],
        }
    }

    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        _pad: [0.0; 2],
    };
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub _pad: f32, // padding explicite
}

impl Default for Color {
    #[inline]
    fn default() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            _pad: 0.0,
        }
    }
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, _pad: 0.0 }
    }

    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        _pad: 0.0,
    };
}

pub struct UpdateResult<'a> {
    pub new_rocket: Option<Rocket>,
    pub explosions: &'a [Particle],
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::{Pod, Zeroable};
    use memoffset::offset_of;
    use std::mem::{align_of, size_of};

    #[test]
    fn layout_color_is_correct() {
        // Vérifie la taille et l’alignement
        assert_eq!(size_of::<Color>(), 16, "Color doit faire 16 octets");
        assert_eq!(
            align_of::<Color>(),
            16,
            "Color doit être aligné sur 16 octets"
        );

        // Vérifie les offsets mémoire
        assert_eq!(offset_of!(Color, r), 0, "r doit être au début");
        assert_eq!(offset_of!(Color, g), 4, "g doit suivre r");
        assert_eq!(offset_of!(Color, b), 8, "b doit suivre g");

        // Vérifie que Color implémente Pod et Zeroable
        fn assert_pod<T: Pod>() {}
        fn assert_zeroable<T: Zeroable>() {}

        assert_pod::<Color>();
        assert_zeroable::<Color>();
    }

    #[test]
    fn cast_color_slice_to_bytes_is_valid() {
        let c = vec![
            Color {
                r: 1.0,
                g: 0.5,
                b: 0.25,
                ..Default::default()
            };
            4
        ];
        let bytes: &[u8] = bytemuck::cast_slice(&c);
        assert_eq!(bytes.len(), c.len() * size_of::<Color>());
    }

    #[test]
    fn layout_vec2_is_correct() {
        // Vérifie la taille et l’alignement
        assert_eq!(size_of::<Vec2>(), 16, "Vec2 doit faire 16 octets");
        assert_eq!(
            align_of::<Vec2>(),
            16,
            "Vec2 doit être aligné sur 16 octets"
        );

        // Vérifie les offsets mémoire
        assert_eq!(offset_of!(Vec2, x), 0, "x doit être au début");
        assert_eq!(offset_of!(Vec2, y), 4, "y doit suivre immédiatement x");

        // Vérifie que Vec2 implémente Pod et Zeroable
        fn assert_pod<T: Pod>() {}
        fn assert_zeroable<T: Zeroable>() {}

        assert_pod::<Vec2>();
        assert_zeroable::<Vec2>();
    }

    #[test]
    fn cast_vec2_slice_to_bytes_is_valid() {
        let v = vec![
            Vec2 {
                x: 1.0,
                y: 2.0,
                ..Default::default()
            };
            4
        ];
        let bytes: &[u8] = bytemuck::cast_slice(&v);
        assert_eq!(bytes.len(), v.len() * size_of::<Vec2>());
    }
}
