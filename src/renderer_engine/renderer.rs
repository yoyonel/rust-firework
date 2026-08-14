use crate::physic_engine::PhysicEngineIterator;
use crate::renderer_engine::utils::instrumentation::palette;
use crate::RendererEngine;
use anyhow::Result;
use log::info;

use crate::gpu_profile_zone;
use crate::physic_engine::config::PhysicConfig;
use crate::renderer_engine::constants;
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

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalDataUBO {
    pub u_size_x: f32,
    pub u_size_y: f32,
    pub u_tex_ratio: f32,
    pub u_bloom_intensity: f32,
}

// ---------------------------------------------------------
pub struct Renderer {
    config: crate::renderer_engine::RendererConfig,
    max_particles_on_gpu: usize,
    ubo_global: u32,
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

        let max_particles_on_gpu: usize = physic_config.max_rockets
            * (physic_config.particles_per_explosion + physic_config.particles_per_trail);

        // Load textures in parallel
        let (rocket_tex, smoke_tex, flow_tex, noise_tex) = std::thread::scope(|s| {
            let r = s.spawn(|| {
                crate::renderer_engine::utils::texture::load_image_data_from_disk(
                    constants::TEXTURE_PRIMARY_PARTICLE_PATH,
                )
            });
            let sm = s.spawn(|| {
                crate::renderer_engine::utils::texture::load_image_data_from_disk(
                    constants::TEXTURE_SMOKE_PARTICLE_PATH,
                )
            });
            let f = s.spawn(|| {
                crate::renderer_engine::utils::texture::load_image_data_from_disk(
                    constants::TEXTURE_FLOW_MAP_PATH,
                )
            });
            let n = s.spawn(|| {
                crate::renderer_engine::utils::texture::load_image_data_from_disk(
                    constants::TEXTURE_NOISE_PATH,
                )
            });
            (
                r.join().unwrap(),
                sm.join().unwrap(),
                f.join().unwrap(),
                n.join().unwrap(),
            )
        });

        let mut renderers: Vec<Box<dyn ParticleGraphicsRenderer>> = vec![
            Box::new(RendererGraphics::new(max_particles_on_gpu)),
            Box::new(RendererGraphicsInstanced::new(
                physic_config.max_rockets,
                crate::physic_engine::ParticleType::Rocket,
                &rocket_tex,
            )),
            Box::new(crate::renderer_engine::smoke_renderer::SmokeRenderer::new(
                physic_config.max_smoke_particles,
                &smoke_tex,
                &flow_tex,
                &noise_tex,
            )),
        ];

        // 🏷️ Phase 2 : Tri d'états (State Sorting) avec ordre de passe explicite
        renderers.sort_by_key(|r| (r.render_order(), r.get_shader_program(), r.get_texture_id()));

        // Initialize bloom pass
        let bloom_pass = BloomPass::new(width, height)
            .map_err(|e| anyhow::anyhow!("Failed to initialize bloom: {}", e))?;

        // 🟢 Initialize OpenGL Ring-Buffer
        let gpu_profiler = std::sync::Arc::new(std::sync::Mutex::new(unsafe {
            crate::renderer_engine::utils::gpu_profiler::GpuProfiler::new()
        }));

        // 🟢 Initialize UBO Global Buffer
        let mut ubo_global = 0;
        unsafe {
            gl::GenBuffers(1, &mut ubo_global);
            gl::BindBuffer(gl::UNIFORM_BUFFER, ubo_global);
            gl::BufferData(
                gl::UNIFORM_BUFFER,
                std::mem::size_of::<GlobalDataUBO>() as isize,
                std::ptr::null(),
                gl::DYNAMIC_DRAW,
            );
            gl::BindBuffer(gl::UNIFORM_BUFFER, 0);

            // Bind global UBO to binding point
            gl::BindBufferBase(
                gl::UNIFORM_BUFFER,
                constants::GLOBAL_UBO_BINDING_INDEX,
                ubo_global,
            );
        }

        Ok(Self {
            config: crate::renderer_engine::RendererConfig::from_file(
                crate::utils::config_path::get_renderer_config_path(),
            )
            .unwrap_or_default(),
            window_size_f32: (width as f32, height as f32),
            renderers,
            max_particles_on_gpu,
            ubo_global,
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
        alpha: f32,
    ) -> usize {
        // 🟢 RAII unifié (GPU + CPU + RenderDoc). Englobe toute la fonction.
        gpu_profile_zone!(10, "Draw All Particles", palette::NBODY, profiler);

        let mut active_shader = 0u32;
        let mut active_texture = 0u32;
        let mut total_particles = 0;
        for renderer in &mut self.renderers {
            let is_enabled = match renderer.particle_type() {
                Some(crate::physic_engine::ParticleType::Rocket) => self.config.render_rockets,
                Some(crate::physic_engine::ParticleType::Smoke) => self.config.render_smoke,
                _ => {
                    renderer
                        .set_visibility(self.config.render_trails, self.config.render_explosions);
                    self.config.render_trails || self.config.render_explosions
                }
            };

            if !is_enabled {
                continue;
            }

            let nb;
            // Remplit le buffer GPU (Opération purement CPU, on utilise uniquement tracy)
            {
                tracy_zone!("Renderer::fill_buffer", palette::ENV);
                nb = renderer.fill_particle_data_direct(physic, alpha);
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
                    &mut active_shader,
                    &mut active_texture,
                );
            }

            total_particles += nb;
        }

        total_particles
    } // ⬅️ Ici, Drop automatique de "Draw All Particles"

    /// Returns an immutable reference to the bloom pass (reserved for visual integration tests)
    #[cfg(any(test, feature = "interactive_tests"))]
    pub fn bloom_pass(&self) -> &BloomPass {
        &self.bloom_pass
    }

    /// Returns a mutable reference to the bloom pass for configuration
    pub fn bloom_pass_mut(&mut self) -> &mut BloomPass {
        &mut self.bloom_pass
    }
}

// Trait implementation
impl RendererEngine for Renderer {
    fn render_frame<P: PhysicEngineIterator>(&mut self, physic: &P, alpha: f32) -> usize {
        // ⏱️ 0. Mettre à jour le UBO global
        unsafe {
            let ubo_data = GlobalDataUBO {
                u_size_x: self.window_size_f32.0,
                u_size_y: self.window_size_f32.1,
                u_tex_ratio: self
                    .renderers
                    .iter()
                    .find(|r| r.render_order() == 20)
                    .map_or(1.0, |r| r.get_tex_ratio()),
                u_bloom_intensity: self.bloom_pass.intensity,
            };
            gl::BindBuffer(gl::UNIFORM_BUFFER, self.ubo_global);
            gl::BufferSubData(
                gl::UNIFORM_BUFFER,
                0,
                std::mem::size_of::<GlobalDataUBO>() as isize,
                &ubo_data as *const _ as *const _,
            );

            // Bind global UBO to binding point 0 for this frame (prevent override by other context users like ImGui)
            gl::BindBufferBase(gl::UNIFORM_BUFFER, 0, self.ubo_global);
        }

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

                    particle_count = self.render_particles(physic, &profiler, alpha);
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
                self.render_particles(physic, &profiler, alpha)
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

            if self.ubo_global != 0 {
                gl::DeleteBuffers(1, &self.ubo_global);
                self.ubo_global = 0;
            }
        }
    }

    fn bloom_pass_mut(&mut self) -> &mut BloomPass {
        &mut self.bloom_pass
    }

    fn sync_bloom_config(&mut self, config: &crate::renderer_engine::RendererConfig) {
        self.config = *config;
        self.bloom_pass.sync_with_renderer_config(config);
    }
}

// Trait implementation
