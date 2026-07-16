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
        window_size: (f32, f32),
        active_shader: &mut u32,
        active_texture: &mut u32,
    );

    /// Retourne l'ID du programme shader associé à ce renderer.
    fn get_shader_program(&self) -> u32;

    /// Retourne l'ID de la texture associée à ce renderer (0 si aucune).
    fn get_texture_id(&self) -> u32;

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
