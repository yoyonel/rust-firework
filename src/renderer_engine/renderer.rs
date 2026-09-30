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

macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const i8
    };
}

#[derive(Debug, Clone, Copy, Default)]
struct PersistentLight {
    active: bool,
    rocket_id: u64,
    pos: [f32; 2],
    color: [f32; 3],
    radius: f32,
    intensity: f32,
    decay_rate: f32,
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
    ubo_lighting: u32,
    sky_haze_program: u32,
    dummy_vao: u32,
    loc_haze_intensity: i32,
    loc_haze_ambient_flash: i32,
    persistent_lights: [PersistentLight; constants::MAX_VOLUMETRIC_LIGHTS],
    smoothed_flash: f32,
    was_lighting_active: bool,
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
        let paths = [
            constants::TEXTURE_PRIMARY_PARTICLE_PATH,
            constants::TEXTURE_SMOKE_PARTICLE_PATH,
            constants::TEXTURE_FLOW_MAP_PATH,
            constants::TEXTURE_NOISE_PATH,
        ];
        let [rocket_tex, smoke_tex, flow_tex, noise_tex] = std::thread::scope(|s| {
            let handles: Vec<_> = paths
                .iter()
                .map(|&p| {
                    s.spawn(move || {
                        crate::renderer_engine::utils::texture::load_image_data_from_disk(p)
                    })
                })
                .collect();
            let mut results = handles.into_iter().map(|h| h.join().unwrap());
            [
                results.next().unwrap(),
                results.next().unwrap(),
                results.next().unwrap(),
                results.next().unwrap(),
            ]
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

        // 🟢 Initialize UBO Global Buffer & Lighting UBO
        let mut ubo_global = 0;
        let mut ubo_lighting = 0;
        let mut dummy_vao = 0;
        let sky_haze_program;
        let loc_haze_intensity;
        let loc_haze_ambient_flash;

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

            // 🟢 Initialize UBO Lighting Buffer (std140, 544 bytes)
            gl::GenBuffers(1, &mut ubo_lighting);
            gl::BindBuffer(gl::UNIFORM_BUFFER, ubo_lighting);
            gl::BufferData(
                gl::UNIFORM_BUFFER,
                std::mem::size_of::<crate::renderer_engine::types::VolumetricLightingBlockGPU>()
                    as isize,
                std::ptr::null(),
                gl::DYNAMIC_DRAW,
            );
            gl::BindBuffer(gl::UNIFORM_BUFFER, 0);

            gl::BindBufferBase(
                gl::UNIFORM_BUFFER,
                constants::LIGHTING_UBO_BINDING_INDEX,
                ubo_lighting,
            );

            // 🟢 Initialize Sky Haze Atmosphere Shader & Dummy VAO
            gl::GenVertexArrays(1, &mut dummy_vao);

            sky_haze_program = crate::renderer_engine::shader::compile_shader_program_from_files(
                constants::SHADER_SKY_HAZE_VERTEX_PATH,
                constants::SHADER_SKY_HAZE_FRAGMENT_PATH,
            );

            let block_idx_global = gl::GetUniformBlockIndex(sky_haze_program, cstr!("GlobalData"));
            if block_idx_global != gl::INVALID_INDEX {
                gl::UniformBlockBinding(
                    sky_haze_program,
                    block_idx_global,
                    constants::GLOBAL_UBO_BINDING_INDEX,
                );
            }

            let block_idx_lighting =
                gl::GetUniformBlockIndex(sky_haze_program, cstr!("LightingBlock"));
            if block_idx_lighting != gl::INVALID_INDEX {
                gl::UniformBlockBinding(
                    sky_haze_program,
                    block_idx_lighting,
                    constants::LIGHTING_UBO_BINDING_INDEX,
                );
            }

            loc_haze_intensity = gl::GetUniformLocation(sky_haze_program, cstr!("u_HazeIntensity"));
            loc_haze_ambient_flash =
                gl::GetUniformLocation(sky_haze_program, cstr!("u_AmbientFlash"));
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
            ubo_lighting,
            sky_haze_program,
            dummy_vao,
            loc_haze_intensity,
            loc_haze_ambient_flash,
            persistent_lights: [PersistentLight::default(); constants::MAX_VOLUMETRIC_LIGHTS],
            smoothed_flash: 0.0,
            was_lighting_active: false,
            bloom_pass,
            gpu_profiler,
            last_gpu_log_time: std::time::Instant::now(),
        })
    }

    // Helper internal
    unsafe fn update_lighting_ubo<P: PhysicEngineIterator>(&mut self, physic: &P) {
        // 1. Décroissance temporelle $C^1$ douce de toutes les lumières persistantes
        for light in &mut self.persistent_lights {
            if light.active {
                light.intensity *= light.decay_rate;
                light.radius *= constants::VOLUMETRIC_LIGHT_RADIUS_EXPANSION;
                if light.intensity < constants::VOLUMETRIC_LIGHT_MIN_INTENSITY {
                    light.active = false;
                    light.intensity = 0.0;
                    light.rocket_id = 0;
                }
            }
        }

        // 2. Échantillonnage des détonations actives de fusées (stabilisation par ID unique)
        physic.for_each_active_rocket(&mut |rocket| {
            if rocket.active && rocket.exploded && rocket.explosion_active_count > 0 {
                let target_intensity =
                    (rocket.explosion_active_count as f32 / 80.0).clamp(0.4, 2.0);

                // Recherche si ce rocket_id possède déjà un slot persistant
                let existing_idx = self
                    .persistent_lights
                    .iter()
                    .position(|l| l.active && l.rocket_id == rocket.id);

                let slot_idx = existing_idx.or_else(|| {
                    // Sinon, chercher un slot inactif ou d'intensité quasi-nulle
                    self.persistent_lights
                        .iter()
                        .position(|l| {
                            !l.active || l.intensity <= constants::VOLUMETRIC_LIGHT_MIN_INTENSITY
                        })
                        .or_else(|| {
                            // En dernier recours, réassigner le slot d'intensité minimale
                            self.persistent_lights
                                .iter()
                                .enumerate()
                                .min_by(|(_, a), (_, b)| {
                                    a.intensity
                                        .partial_cmp(&b.intensity)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                })
                                .map(|(idx, _)| idx)
                        })
                });

                if let Some(idx) = slot_idx {
                    let slot = &mut self.persistent_lights[idx];
                    slot.active = true;
                    slot.rocket_id = rocket.id;
                    slot.pos = [rocket.pos.x, rocket.pos.y];
                    slot.color = [rocket.color.x, rocket.color.y, rocket.color.z];
                    slot.radius = constants::VOLUMETRIC_LIGHT_INITIAL_RADIUS;
                    slot.intensity = slot.intensity.max(target_intensity);
                    slot.decay_rate = constants::VOLUMETRIC_LIGHT_DECAY_RATE;
                }
            }
        });

        // 3. Calcul de l'énergie explosive totale pour le flash ambiant (filtrage EMA continu sans seuil binaire)
        let total_active_energy: f32 = self
            .persistent_lights
            .iter()
            .filter(|l| l.active)
            .map(|l| l.intensity)
            .sum();

        let target_flash = (total_active_energy * constants::VOLUMETRIC_FLASH_ENERGY_SCALE)
            .min(constants::VOLUMETRIC_FLASH_MAX_CAP);

        if target_flash > self.smoothed_flash {
            self.smoothed_flash = self.smoothed_flash * 0.80 + target_flash * 0.20;
        } else {
            self.smoothed_flash = self.smoothed_flash * constants::VOLUMETRIC_FLASH_EMA_DECAY
                + target_flash * constants::VOLUMETRIC_FLASH_EMA_WEIGHT;
        }

        // 4. Copie 1:1 vers le bloc std140 GPU sans permutation d'indices (Zéro clignotement / Zéro saut)
        let mut lights = [crate::renderer_engine::types::PointLightGPU::default();
            constants::MAX_VOLUMETRIC_LIGHTS];
        let mut max_active_idx = 0;

        for (i, light) in self.persistent_lights.iter().enumerate() {
            if light.active && light.intensity > constants::VOLUMETRIC_LIGHT_MIN_INTENSITY {
                lights[i] = crate::renderer_engine::types::PointLightGPU {
                    position_radius: [light.pos[0], light.pos[1], 0.0, light.radius],
                    color_intensity: [
                        light.color[0],
                        light.color[1],
                        light.color[2],
                        light.intensity,
                    ],
                };
                max_active_idx = i + 1;
            } else {
                lights[i] = crate::renderer_engine::types::PointLightGPU::default();
            }
        }

        let flash_clamped = self.smoothed_flash.clamp(0.0, 1.0);
        let lighting_block = crate::renderer_engine::types::VolumetricLightingBlockGPU {
            lights,
            ambient_light: [
                flash_clamped * 0.8,
                flash_clamped * 0.8,
                flash_clamped * 1.0,
                flash_clamped,
            ],
            num_active_lights: max_active_idx as i32,
            scattering_intensity: if self.config.smoke_lighting_enabled {
                self.config.smoke_scattering_intensity
            } else {
                0.0
            },
            _padding: [0; 2],
        };

        gl::BindBuffer(gl::UNIFORM_BUFFER, self.ubo_lighting);
        gl::BufferSubData(
            gl::UNIFORM_BUFFER,
            0,
            std::mem::size_of::<crate::renderer_engine::types::VolumetricLightingBlockGPU>()
                as isize,
            &lighting_block as *const _ as *const std::ffi::c_void,
        );
        gl::BindBufferBase(
            gl::UNIFORM_BUFFER,
            constants::LIGHTING_UBO_BINDING_INDEX,
            self.ubo_lighting,
        );
    }

    /// Réinitialisation à zéro de l'UBO d'éclairage lors de la désactivation dynamique.
    /// Garantit un coût de 0 GPU/CPU pour les frames suivantes et un rendu 100% ISO develop.
    unsafe fn reset_lighting_ubo(&mut self) {
        self.persistent_lights = [PersistentLight::default(); constants::MAX_VOLUMETRIC_LIGHTS];
        self.smoothed_flash = 0.0;
        let empty_block = crate::renderer_engine::types::VolumetricLightingBlockGPU {
            lights: [crate::renderer_engine::types::PointLightGPU::default();
                constants::MAX_VOLUMETRIC_LIGHTS],
            ambient_light: [0.0; 4],
            num_active_lights: 0,
            scattering_intensity: 0.0,
            _padding: [0; 2],
        };
        gl::BindBuffer(gl::UNIFORM_BUFFER, self.ubo_lighting);
        gl::BufferSubData(
            gl::UNIFORM_BUFFER,
            0,
            std::mem::size_of::<crate::renderer_engine::types::VolumetricLightingBlockGPU>()
                as isize,
            &empty_block as *const _ as *const std::ffi::c_void,
        );
        gl::BindBufferBase(
            gl::UNIFORM_BUFFER,
            constants::LIGHTING_UBO_BINDING_INDEX,
            self.ubo_lighting,
        );
    }

    unsafe fn render_sky_haze(&self) {
        if !self.config.volumetric_lighting_enabled || !self.config.sky_haze_enabled {
            return;
        }
        gl::Disable(gl::DEPTH_TEST);
        gl::DepthMask(gl::FALSE);
        gl::Disable(gl::BLEND);

        gl::UseProgram(self.sky_haze_program);
        if self.loc_haze_intensity != -1 {
            gl::Uniform1f(self.loc_haze_intensity, self.config.sky_haze_intensity);
        }
        if self.loc_haze_ambient_flash != -1 {
            gl::Uniform1f(
                self.loc_haze_ambient_flash,
                self.config.sky_haze_ambient_flash,
            );
        }

        gl::BindVertexArray(self.dummy_vao);
        gl::DrawArrays(gl::TRIANGLES, 0, 3);
        gl::BindVertexArray(0);

        // Restore blend state for particle rendering pipeline
        gl::Enable(gl::BLEND);
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

            if let Some(crate::physic_engine::ParticleType::Smoke) = renderer.particle_type() {
                let smoke_lighting =
                    self.config.volumetric_lighting_enabled && self.config.smoke_lighting_enabled;
                renderer.set_smoke_lighting(
                    smoke_lighting,
                    if smoke_lighting {
                        self.config.smoke_scattering_intensity
                    } else {
                        0.0
                    },
                    if smoke_lighting {
                        self.config.smoke_ambient_flash
                    } else {
                        0.0
                    },
                );
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
        // ⏱️ 0. Mettre à jour les UBOs globaux
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

            gl::BindBufferBase(gl::UNIFORM_BUFFER, 0, self.ubo_global);

            let is_lighting_active = self.config.volumetric_lighting_enabled
                && (self.config.smoke_lighting_enabled || self.config.sky_haze_enabled);

            if is_lighting_active {
                self.was_lighting_active = true;
                self.update_lighting_ubo(physic);
            } else if self.was_lighting_active {
                // One-off zero-cost transition: flush UBO and reset internal state
                self.reset_lighting_ubo();
                self.was_lighting_active = false;
            }
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

                    self.render_sky_haze();

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

                self.render_sky_haze();

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
            if self.ubo_lighting != 0 {
                gl::DeleteBuffers(1, &self.ubo_lighting);
                self.ubo_lighting = 0;
            }
            if self.dummy_vao != 0 {
                gl::DeleteVertexArrays(1, &self.dummy_vao);
                self.dummy_vao = 0;
            }
            if self.sky_haze_program != 0 {
                gl::DeleteProgram(self.sky_haze_program);
                self.sky_haze_program = 0;
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
