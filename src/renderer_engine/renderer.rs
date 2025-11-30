use crate::physic_engine::PhysicEngineIterator;
use crate::RendererEngine;
use anyhow::Result;
use log::info;

use crate::physic_engine::config::PhysicConfig;
use crate::renderer_engine::particle_renderer::ParticleGraphicsRenderer;
use crate::renderer_engine::tools::{setup_opengl_debug, show_opengl_context_info};
use crate::renderer_engine::BloomPass;
use crate::renderer_engine::RendererGraphics;
use crate::renderer_engine::RendererGraphicsInstanced;

// ---------------------------------------------------------
pub struct Renderer {
    max_particles_on_gpu: usize,

    // Window management
    window_size_f32: (f32, f32),

    renderers: Vec<Box<dyn ParticleGraphicsRenderer>>,

    // Bloom post-processing
    bloom_pass: BloomPass,
}

// ---------------------------------------------------------
// Implémentation générique du Renderer pour tout type A
// qui implémente le trait AudioEngine.
//
// Signification exacte :
// - `impl<A: crate::audio_engine::AudioEngine> Renderer<A>`
//   signifie que toutes les méthodes définies ici sont disponibles
//   pour un Renderer dont le type `A` satisfait le trait AudioEngine.
// - `pub fn new(..., audio: A) -> Result<Self>`
//   prend **ownership** d'un objet `audio` de type `A`.
//   Comme le Renderer possède cet objet, il n'y a pas besoin de
//   références mutables externes ou de lifetimes (`&mut`) pour l'audio.
// Conséquences / avantages :
// 1. Typage statique et monomorphisation : pas de dispatch dynamique,
//    ce qui permet des appels plus rapides.
// 2. Flexibilité : on peut injecter un moteur audio réel ou un mock
//    pour les tests, simplement en changeant le type `A`.
// 3. Sécurité mémoire : le Renderer est propriétaire de l'audio et
//    gère sa durée de vie, pas de risque de référence suspendue.
//
// Limitation :
// - Chaque type `A` utilisé génère une version spécifique du Renderer
//   dans le binaire, ce qui peut augmenter légèrement la taille du code.
impl Renderer {
    pub fn new(width: i32, height: i32, physic_config: &PhysicConfig) -> Result<Self> {
        unsafe {
            show_opengl_context_info();

            // activate OpenGL debug output
            setup_opengl_debug();

            // set OpenGL states for the rendering
            // but it's link to the renderer graphics
            gl::Enable(gl::PROGRAM_POINT_SIZE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        let max_particles_on_gpu: usize =
            physic_config.max_rockets * physic_config.particles_per_explosion;

        let renderers: Vec<Box<dyn ParticleGraphicsRenderer>> = vec![
            Box::new(RendererGraphics::new(max_particles_on_gpu)),
            Box::new(RendererGraphicsInstanced::new(
                physic_config.max_rockets,
                crate::physic_engine::ParticleType::Rocket,
                "assets/textures/04ddeae2-7367-45f1-87e0-361d1d242630_scaled.png",
            )),
        ];

        // Initialize bloom pass
        let bloom_pass = BloomPass::new(width, height)
            .map_err(|e| anyhow::anyhow!("Failed to initialize bloom: {}", e))?;

        Ok(Self {
            window_size_f32: (width as f32, height as f32),
            renderers,
            max_particles_on_gpu,
            bloom_pass,
        })
    }

    // Helper internal
    unsafe fn render_particles<P: PhysicEngineIterator>(&mut self, physic: &P) -> usize {
        let mut total_particles = 0;
        for renderer in &mut self.renderers {
            // Remplit le buffer GPU
            let nb = renderer.fill_particle_data_direct(physic);
            // Dessine les particules
            renderer.render_particles_with_persistent_buffer(nb, self.window_size_f32);
            total_particles += nb;
        }
        total_particles
    }

    /// Returns a mutable reference to the bloom pass for configuration
    pub fn bloom_pass_mut(&mut self) -> &mut BloomPass {
        &mut self.bloom_pass
    }
}

// Trait implementation
impl RendererEngine for Renderer {
    fn render_frame<P: PhysicEngineIterator>(&mut self, physic: &P) -> usize {
        unsafe {
            if self.bloom_pass.enabled {
                // Render to HDR framebuffer
                self.bloom_pass.begin_scene();
                gl::ClearColor(0.0, 0.0, 0.0, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                let particle_count = self.render_particles(physic);

                // Apply bloom and render to screen
                self.bloom_pass.end_scene_and_apply_bloom();
                particle_count
            } else {
                // Direct rendering without bloom
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
                gl::ClearColor(0.0, 0.0, 0.0, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                self.render_particles(physic)
            }
        }
    }

    fn set_window_size(&mut self, width: i32, height: i32) {
        unsafe {
            gl::Viewport(0, 0, width, height);
            self.bloom_pass.resize(width, height);
        }
        self.window_size_f32 = (width as f32, height as f32);
    }

    fn recreate_buffers(&mut self, max_particles: usize) {
        if max_particles != self.max_particles_on_gpu {
            info!(
                "🔁 GPU buffer reallocation required ({} → {})",
                self.max_particles_on_gpu, max_particles
            );
            self.max_particles_on_gpu = max_particles;
            unsafe {
                for renderer in &mut self.renderers {
                    renderer.recreate_buffers(max_particles);
                }
            }
        }
    }

    fn reload_shaders(&mut self) -> Result<(), String> {
        info!("🔄 Reloading shaders for all renderers...");
        let mut errors = Vec::new();
        unsafe {
            for renderer in &mut self.renderers {
                if let Err(e) = renderer.reload_shaders() {
                    errors.push(e);
                }
            }

            // Reload bloom shaders
            if let Err(e) = self.bloom_pass.reload_shaders() {
                errors.push(format!("Bloom shaders: {}", e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n\n"))
        }
    }

    fn close(&mut self) {
        info!("🧹 Fermeture du Renderer");
        unsafe {
            for renderer in &mut self.renderers {
                renderer.close();
            }
            self.bloom_pass.close();
        }
    }

    fn bloom_pass_mut(&mut self) -> &mut BloomPass {
        &mut self.bloom_pass
    }
}

// Trait implementation
