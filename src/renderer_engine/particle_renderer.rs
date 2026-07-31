use crate::physic_engine::PhysicEngineIterator;

/// Trait générique pour un rendu de particules.
/// Permet d'abstraire le type de rendu (points, quads texturés, etc.)
/// et de gérer une collection de renderers de manière uniforme.
pub trait ParticleGraphicsRenderer {
    /// Recrée les buffers GPU avec une nouvelle taille maximale.
    ///
    /// # Safety
    /// Cette fonction est unsafe car elle manipule directement des ressources OpenGL.
    unsafe fn recreate_buffers(&mut self, new_max: usize);

    /// Remplit le buffer GPU avec les données des particules.
    /// Retourne le nombre de particules à dessiner.
    ///
    /// # Safety
    /// Cette fonction est unsafe car elle manipule directement des ressources OpenGL.
    unsafe fn fill_particle_data_direct(&mut self, physic: &dyn PhysicEngineIterator) -> usize;

    /// Dessine les particules à l'écran.
    ///
    /// # Safety
    /// Cette fonction est unsafe car elle manipule directement des ressources OpenGL.
    unsafe fn render_particles_with_persistent_buffer(
        &mut self,
        count: usize,
        active_shader: &mut u32,
        active_texture: &mut u32,
    );

    /// Retourne l'ID du programme shader associé à ce renderer.
    fn get_shader_program(&self) -> u32;

    /// Retourne l'ID de la texture associée à ce renderer (0 si aucune).
    fn get_texture_id(&self) -> u32;

    /// Retourne le ratio d'aspect de la texture.
    fn get_tex_ratio(&self) -> f32;

    /// Retourne le type de particule spécifique géré par ce renderer, ou None si générique.
    fn particle_type(&self) -> Option<crate::physic_engine::ParticleType> {
        None
    }

    /// Définit les bascules de visibilité pour les sous-types de particules (ex: trails, explosions).
    fn set_visibility(&mut self, _render_trails: bool, _render_explosions: bool) {}

    /// Retourne l'ordre de priorité de rendu (pass order) pour trier les passes.
    fn render_order(&self) -> u32 {
        0
    }

    /// Recharge les shaders depuis les fichiers.
    ///
    /// # Safety
    /// Cette fonction est unsafe car elle manipule directement des ressources OpenGL.
    unsafe fn reload_shaders(&mut self) -> Result<(), String>;

    /// Libère les ressources GPU.
    ///
    /// # Safety
    /// Cette fonction est unsafe car elle manipule directement des ressources OpenGL.
    unsafe fn close(&mut self);
}
