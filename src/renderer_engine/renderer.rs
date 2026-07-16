use crate::physic_engine::PhysicEngineIterator;
use crate::renderer_engine::utils::instrumentation::palette;
use crate::RendererEngine;
use anyhow::Result;
use log::info;

use crate::gpu_profile_zone;
use crate::physic_engine::config::PhysicConfig;
use crate::renderer_engine::particle_renderer::ParticleGraphicsRenderer;
use crate::renderer_engine::renderer_graphics::RendererGraphics;
use crate::renderer_engine::renderer_graphics_instanced::RendererGraphicsInstanced;
use crate::renderer_engine::BloomPass;

/// Macro pour créer une zone Tracy **sans conditionner l'exécution du code**.
/// Utilisation: `tracy_zone!("nom_zone", 0xRRGGBB);`
#[cfg(feature = "tracy")]
macro_rules! tracy_zone {
    ($name:expr, $color:expr) => {
        let _span = tracy_client::span!($name);
        _span.emit_color($color);
    };
}

/// Macro vide si Tracy n'est pas activé
#[cfg(not(feature = "tracy"))]
macro_rules! tracy_zone {
    ($name:expr, $color:expr) => {};
}

// ---------------------------------------------------------
pub struct Renderer {
    max_particles_on_gpu: usize,
    // Window management
    window_size_f32: (f32, f32),
    renderers: Vec<Box<dyn ParticleGraphicsRenderer>>,
    // Bloom post-processing
    bloom_pass: BloomPass,
    // moteur de profilage GPU autonome
    pub gpu_profiler:
        std::sync::Arc<std::sync::Mutex<crate::renderer_engine::utils::gpu_profiler::GpuProfiler>>,
    last_gpu_log_time: std::time::Instant,
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
        // Note: OpenGL context initialization (show_opengl_context_info, setup_opengl_debug, etc.)
        // is already done by GlfwWindowEngine::init(), so we don't duplicate it here.

        let max_particles_on_gpu: usize =
            physic_config.max_rockets * physic_config.particles_per_explosion;

        let mut renderers: Vec<Box<dyn ParticleGraphicsRenderer>> = vec![
            Box::new(RendererGraphics::new(max_particles_on_gpu)),
            Box::new(RendererGraphicsInstanced::new(
                physic_config.max_rockets,
                crate::physic_engine::ParticleType::Rocket,
                "assets/textures/04ddeae2-7367-45f1-87e0-361d1d242630_scaled.png",
            )),
        ];

        // 🏷️ Phase 2 : Tri d'états (State Sorting)
        renderers.sort_by_key(|r| (r.get_shader_program(), r.get_texture_id()));

        // Initialize bloom pass
        let bloom_pass = BloomPass::new(width, height)
            .map_err(|e| anyhow::anyhow!("Failed to initialize bloom: {}", e))?;

        // 🟢 Initialize OpenGL Ring-Buffer
        let gpu_profiler = std::sync::Arc::new(std::sync::Mutex::new(unsafe {
            crate::renderer_engine::utils::gpu_profiler::GpuProfiler::new()
        }));

        Ok(Self {
            window_size_f32: (width as f32, height as f32),
            renderers,
            max_particles_on_gpu,
            bloom_pass,
            gpu_profiler,
            last_gpu_log_time: std::time::Instant::now(),
        })
    }

    // Helper internal
    unsafe fn render_particles<P: PhysicEngineIterator>(
        &mut self,
        physic: &P,
        profiler: &std::sync::Arc<
            std::sync::Mutex<crate::renderer_engine::utils::gpu_profiler::GpuProfiler>,
        >,
    ) -> usize {
        // 🟢 RAII unifié (GPU + CPU + RenderDoc). Englobe toute la fonction.
        gpu_profile_zone!(10, "Draw All Particles", palette::NBODY, profiler);

        let mut active_shader = 0u32;
        let mut active_texture = 0u32;
        let mut total_particles = 0;
        for renderer in &mut self.renderers {
            let nb;
            // Remplit le buffer GPU (Opération purement CPU, on utilise uniquement tracy)
            {
                tracy_zone!("Renderer::fill_buffer", palette::ENV);
                nb = renderer.fill_particle_data_direct(physic);
            }

            // Dessine les particules (Opération hautement GPU, on utilise le profiler complet)
            {
                gpu_profile_zone!(
                    11,
                    "Renderer::Particles_with_Persistent_Buffer",
                    palette::SHOCKWAVE,
                    profiler
                );
                renderer.render_particles_with_persistent_buffer(
                    nb,
                    self.window_size_f32,
                    &mut active_shader,
                    &mut active_texture,
                );
            }

            total_particles += nb;
        }

        total_particles
    } // ⬅️ Ici, Drop automatique de "Draw All Particles"

    /// Returns a mutable reference to the bloom pass for configuration
    pub fn bloom_pass_mut(&mut self) -> &mut BloomPass {
        &mut self.bloom_pass
    }
}

// Trait implementation
impl RendererEngine for Renderer {
    fn render_frame<P: PhysicEngineIterator>(&mut self, physic: &P) -> usize {
        // ⏱️ 1. Récolte asynchrone des chronométrages réels de la frame N-1 et affichage
        if let Ok(mut profiler) = self.gpu_profiler.lock() {
            profiler.begin_frame();

            // Echantillonnage à 2 secondes (Throttle anti-flood)
            if self.last_gpu_log_time.elapsed().as_secs() >= 2 {
                for result in &profiler.latest_results {
                    log::info!("⏱️ GPU [{}]: {:.3} ms", result.name, result.duration_ms);
                }
                // Réinitialise le chronomètre après affichage
                self.last_gpu_log_time = std::time::Instant::now();
            }
        }

        // ⏱️ 2. On clone l'Arc (coût nul) pour alimenter nos macros SANS emprunter self !
        let profiler = self.gpu_profiler.clone();

        // Zone globale de la frame
        gpu_profile_zone!(0, "Renderer::render_frame", palette::FRAME, profiler);

        unsafe {
            if self.bloom_pass.enabled {
                let particle_count;

                // Render to HDR framebuffer
                {
                    gpu_profile_zone!(1, "Pass: HDR Scene", palette::SCENE, profiler);
                    self.bloom_pass.begin_scene();
                    gl::ClearColor(0.0, 0.0, 0.0, 1.0);
                    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                    particle_count = self.render_particles(physic, &profiler);
                } // ⬅️ Drop RAII (PopDebugGroup + fin de query GPU)
                {
                    gpu_profile_zone!(2, "Pass: Bloom & Composite", palette::BLOOM, profiler);
                    // Apply bloom and render to screen
                    self.bloom_pass.end_scene_and_apply_bloom();
                } // ⬅️ Drop RAII
                particle_count
            } else {
                gpu_profile_zone!(0, "Pass: Forward (No Bloom)", palette::COMPOSITE, profiler);

                // Direct rendering without bloom
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
                gl::ClearColor(0.0, 0.0, 0.0, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                self.render_particles(physic, &profiler)
            } // ⬅️ Drop RAII
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
            // Disable OpenGL debug callback BEFORE closing resources
            // to prevent the callback from being invoked during/after context destruction
            gl::DebugMessageCallback(None, std::ptr::null_mut());
            gl::Disable(gl::DEBUG_OUTPUT);

            for renderer in &mut self.renderers {
                renderer.close();
            }
            self.bloom_pass.close();
        }
    }

    fn bloom_pass_mut(&mut self) -> &mut BloomPass {
        &mut self.bloom_pass
    }

    fn sync_bloom_config(&mut self, config: &crate::renderer_engine::RendererConfig) {
        self.bloom_pass.sync_with_renderer_config(config);
    }
}

// Trait implementation
