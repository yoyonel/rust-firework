//! Module pour gérer les formes d'explosion des feux d'artifice.
//!
//! Ce module permet de définir comment les particules d'explosion sont distribuées :
//! - `Spherical` : distribution aléatoire uniforme en cercle (comportement legacy)
//! - `Image` : distribution basée sur un sampling de pixels blancs d'une image N&B

use glam::Vec2;
use image::Luma;
use rand::Rng;
use std::path::Path;

/// Configuration d'une forme d'explosion
#[derive(Debug, Clone, Default)]
pub enum ExplosionShape {
    /// Explosion sphérique classique (directions aléatoires uniformes)
    #[default]
    Spherical,
    /// Explosion basée sur une image N&B unique
    Image(ImageShape),
    /// Plusieurs images avec des poids (probabilités) respectifs
    MultiImage {
        shapes: Vec<(ImageShape, f32)>,
        total_weight: f32,
    },
}

impl ExplosionShape {
    /// Retourne une forme d'image échantillonnée aléatoirement selon les poids,
    /// ou None si la forme est sphérique.
    pub fn sample(&self, rng: &mut impl Rng) -> Option<&ImageShape> {
        match self {
            ExplosionShape::Spherical => None,
            ExplosionShape::Image(shape) => Some(shape),
            ExplosionShape::MultiImage {
                shapes,
                total_weight,
            } => {
                if shapes.is_empty() {
                    return None;
                }
                let target = rng.random_range(0.0..*total_weight);
                let mut current = 0.0;
                for (shape, weight) in shapes {
                    current += weight;
                    if target <= current {
                        return Some(shape);
                    }
                }
                // Fallback (ne devrait pas arriver avec une logique flottante correcte, sauf rounding)
                Some(&shapes.last().unwrap().0)
            }
        }
    }
}

/// Forme d'explosion basée sur une image noir & blanc.
///
/// Les pixels blancs (ou non-noirs) de l'image sont échantillonnés pour
/// définir les positions cibles des particules d'explosion.
#[derive(Debug, Clone)]
pub struct ImageShape {
    pub file_stem: String,
    /// Points échantillonnés, normalisés dans l'espace [-0.5, 0.5] x [-0.5, 0.5]
    /// Le centre de l'image correspond à (0, 0)
    pub sampled_points: Vec<Vec2>,
    /// Échelle pour mapper les coordonnées normalisées vers l'espace monde (en pixels)
    pub scale: f32,
    /// Temps de vol pour que les particules atteignent leur destination (en secondes)
    pub flight_time: f32,
}

impl ImageShape {
    /// Crée une nouvelle forme d'image à partir d'un fichier image.
    ///
    /// # Arguments
    /// * `path` - Chemin vers l'image (PNG, JPEG, etc.)
    /// * `n_samples` - Nombre de points à échantillonner
    /// * `scale` - Taille de l'image projetée dans l'espace monde
    /// * `flight_time` - Temps de vol des particules (toutes arrivent en même temps)
    ///
    /// # Algorithme
    /// 1. Charge l'image et la convertit en niveaux de gris
    /// 2. Collecte tous les pixels dont l'intensité dépasse un seuil
    /// 3. Échantillonne aléatoirement `n_samples` points parmi ces pixels
    /// 4. Normalise les coordonnées dans [-0.5, 0.5]
    pub fn from_image(
        path: &str,
        n_samples: usize,
        scale: f32,
        flight_time: f32,
    ) -> anyhow::Result<Self> {
        let path_to_img = Path::new(path);
        let img = image::open(path_to_img)
            .map_err(|e| anyhow::anyhow!("Échec du chargement de l'image '{}': {}", path, e))?;

        let gray = img.to_luma8();
        let (width, height) = gray.dimensions();

        // Seuil pour considérer un pixel comme "blanc" (non-noir)
        const THRESHOLD: u8 = 128;

        // Collecte tous les pixels blancs
        let mut white_pixels: Vec<(u32, u32)> = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let Luma([intensity]) = gray.get_pixel(x, y);
                if *intensity >= THRESHOLD {
                    white_pixels.push((x, y));
                }
            }
        }

        if white_pixels.is_empty() {
            return Err(anyhow::anyhow!(
                "L'image '{}' ne contient aucun pixel blanc (seuil: {})",
                path,
                THRESHOLD
            ));
        }

        // Calcul du barycentre des pixels blancs
        let (sum_x, sum_y): (f64, f64) =
            white_pixels.iter().fold((0.0, 0.0), |(sx, sy), &(x, y)| {
                (sx + x as f64, sy + y as f64)
            });
        let count = white_pixels.len() as f64;
        let barycenter_x = sum_x / count;
        let barycenter_y = sum_y / count;

        // Dimension maximale pour normaliser (on veut que l'étendue soit [-0.5, 0.5])
        let max_dim = width.max(height) as f32;

        // Échantillonnage aléatoire des pixels blancs, centrés sur le barycentre
        let mut rng = rand::rng();
        let sampled_points: Vec<Vec2> = (0..n_samples)
            .map(|_| {
                let idx = rng.random_range(0..white_pixels.len());
                let (px, py) = white_pixels[idx];

                // Coordonnées relatives au barycentre, normalisées par la dimension max
                let nx = (px as f32 - barycenter_x as f32) / max_dim;
                // Inversion de Y car les coordonnées image ont Y vers le bas
                let ny = (barycenter_y as f32 - py as f32) / max_dim;

                Vec2::new(nx, ny)
            })
            .collect();

        log::info!(
            "ImageShape: chargé '{}' ({}x{}), {} pixels blancs, barycentre ({:.1}, {:.1}), {} points",
            path,
            width,
            height,
            white_pixels.len(),
            barycenter_x,
            barycenter_y,
            sampled_points.len()
        );

        Ok(Self {
            file_stem: path_to_img
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
            sampled_points,
            scale,
            flight_time,
        })
    }

    /// Retourne le nombre de points échantillonnés
    pub fn sample_count(&self) -> usize {
        self.sampled_points.len()
    }

    /// Retourne le temps de vol configuré
    pub fn flight_time(&self) -> f32 {
        self.flight_time
    }

    /// Retourne l'échelle configurée
    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Calcule la position cible dans l'espace monde pour une particule donnée.
    ///
    /// # Arguments
    /// * `index` - Index de la particule (modulo le nombre de points échantillonnés)
    /// * `explosion_center` - Position du centre de l'explosion dans l'espace monde
    ///
    /// # Returns
    /// Position cible dans l'espace monde
    pub fn get_target_position(&self, index: usize, explosion_center: Vec2) -> Vec2 {
        let point = self.sampled_points[index % self.sampled_points.len()];
        explosion_center + point * self.scale
    }

    /// Calcule la position cible avec rotation.
    ///
    /// # Arguments
    /// * `index` - Index de la particule
    /// * `explosion_center` - Centre de l'explosion
    /// * `cos_angle` - Cosinus de l'angle de rotation
    /// * `sin_angle` - Sinus de l'angle de rotation
    ///
    /// # Returns
    /// Position cible dans l'espace monde, avec rotation appliquée
    #[inline(always)]
    pub fn get_target_position_rotated(
        &self,
        index: usize,
        explosion_center: Vec2,
        cos_angle: f32,
        sin_angle: f32,
    ) -> Vec2 {
        let point = self.sampled_points[index % self.sampled_points.len()];

        // Rotation 2D : x' = x*cos - y*sin, y' = x*sin + y*cos
        let rotated_x = point.x * cos_angle - point.y * sin_angle;
        let rotated_y = point.x * sin_angle + point.y * cos_angle;

        explosion_center + Vec2::new(rotated_x, rotated_y) * self.scale
    }

    /// Calcule la vitesse initiale nécessaire pour qu'une particule atteigne
    /// sa position cible après `flight_time` secondes, en tenant compte de la gravité.
    ///
    /// # Équation balistique
    /// ```text
    /// P_target = P_0 + V_0 * t + 0.5 * g * t²
    /// => V_0 = (P_target - P_0 - 0.5 * g * t²) / t
    /// ```
    ///
    /// # Arguments
    /// * `start_pos` - Position initiale de la particule
    /// * `target_pos` - Position cible à atteindre
    /// * `gravity` - Vecteur gravité (typiquement (0, -200))
    ///
    /// # Returns
    /// Vitesse initiale nécessaire pour atteindre la cible
    pub fn compute_initial_velocity(
        &self,
        start_pos: Vec2,
        target_pos: Vec2,
        gravity: Vec2,
    ) -> Vec2 {
        let t = self.flight_time;
        let displacement = target_pos - start_pos;
        let gravity_term = 0.5 * gravity * t * t;

        (displacement - gravity_term) / t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_initial_velocity_no_gravity() {
        let shape = ImageShape {
            file_stem: "test".to_string(),
            sampled_points: vec![Vec2::new(0.5, 0.5)],
            scale: 100.0,
            flight_time: 1.0,
        };

        let start = Vec2::new(0.0, 0.0);
        let target = Vec2::new(100.0, 100.0);
        let gravity = Vec2::ZERO;

        let v0 = shape.compute_initial_velocity(start, target, gravity);

        // Sans gravité, V0 = (target - start) / t = (100, 100) / 1 = (100, 100)
        assert!((v0.x - 100.0).abs() < 0.001);
        assert!((v0.y - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_compute_initial_velocity_with_gravity() {
        let shape = ImageShape {
            file_stem: "test".to_string(),
            sampled_points: vec![Vec2::new(0.0, 0.0)],
            scale: 100.0,
            flight_time: 1.0,
        };

        let start = Vec2::new(0.0, 0.0);
        let target = Vec2::new(0.0, 0.0); // Reste sur place
        let gravity = Vec2::new(0.0, -200.0);

        let v0 = shape.compute_initial_velocity(start, target, gravity);

        // Pour rester sur place avec g=-200, on doit compenser :
        // V0 = (0 - 0 - 0.5 * (-200) * 1²) / 1 = 100
        assert!((v0.x - 0.0).abs() < 0.001);
        assert!((v0.y - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_get_target_position() {
        let shape = ImageShape {
            file_stem: "test".to_string(),
            sampled_points: vec![Vec2::new(0.5, 0.0), Vec2::new(-0.5, 0.0)],
            scale: 200.0,
            flight_time: 1.0,
        };

        let center = Vec2::new(400.0, 300.0);

        let target0 = shape.get_target_position(0, center);
        assert!((target0.x - 500.0).abs() < 0.001); // 400 + 0.5 * 200
        assert!((target0.y - 300.0).abs() < 0.001);

        let target1 = shape.get_target_position(1, center);
        assert!((target1.x - 300.0).abs() < 0.001); // 400 - 0.5 * 200
        assert!((target1.y - 300.0).abs() < 0.001);
    }

    #[test]
    fn test_multi_image_sampling() {
        use rand::SeedableRng;
        // Use a deterministic RNG for testing
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let shape1 = ImageShape {
            file_stem: "shape1".to_string(),
            sampled_points: vec![],
            scale: 1.0,
            flight_time: 1.0,
        };
        let shape2 = ImageShape {
            file_stem: "shape2".to_string(),
            sampled_points: vec![],
            scale: 1.0,
            flight_time: 1.0,
        };

        let multi = ExplosionShape::MultiImage {
            shapes: vec![
                (shape1.clone(), 1.0), // Weight 1.0
                (shape2.clone(), 3.0), // Weight 3.0 (3x more likely)
            ],
            total_weight: 4.0,
        };

        let mut counts = std::collections::HashMap::new();
        let samples = 10_000;

        for _ in 0..samples {
            if let Some(s) = multi.sample(&mut rng) {
                *counts.entry(s.file_stem.clone()).or_insert(0) += 1;
            }
        }

        let count1 = *counts.get("shape1").unwrap_or(&0);
        let count2 = *counts.get("shape2").unwrap_or(&0);

        // Expect rough ratio of 1:3
        // shape1 ~ 2500, shape2 ~ 7500
        let ratio = count2 as f32 / count1 as f32;

        println!(
            "Counts: shape1={}, shape2={}, ratio={}",
            count1, count2, ratio
        );

        // Allow some variance, but 10k samples should be close
        assert!(ratio > 2.5 && ratio < 3.5, "Ratio should be around 3.0");
    }
}
