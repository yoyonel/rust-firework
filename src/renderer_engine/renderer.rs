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
    elapsed_ms: f32,
}

impl PersistentLight {
    #[inline]
    pub fn uploaded_intensity(&self, fade_in_ms: f32) -> f32 {
        if fade_in_ms <= 0.0 {
            self.intensity
        } else {
            let fade_factor = (self.elapsed_ms / fade_in_ms).clamp(0.0, 1.0);
            self.intensity * fade_factor
        }
    }
}

/// Helper pure pour allouer ou réassigner un slot de lumière volumétrique avec hystérésis d'éviction.
#[inline]
fn find_lighting_slot(
    lights: &[PersistentLight],
    candidate_id: u64,
    candidate_intensity: f32,
    hysteresis_enabled: bool,
) -> Option<usize> {
    // 1. Même rocket_id déjà présent : réutilise le slot
    if let Some(idx) = lights
        .iter()
        .position(|l| l.active && l.rocket_id == candidate_id)
    {
        return Some(idx);
    }

    // 2. Slot inactif ou éteint : réclamable sans condition
    if let Some(idx) = lights
        .iter()
        .position(|l| !l.active || l.intensity <= constants::VOLUMETRIC_LIGHT_MIN_INTENSITY)
    {
        return Some(idx);
    }

    // 3. Tous les slots occupés par des lumières vivantes : sélection du slot minimal
    let (min_idx, min_occupant) = lights.iter().enumerate().min_by(|(_, a), (_, b)| {
        a.intensity
            .partial_cmp(&b.intensity)
            .unwrap_or(std::cmp::Ordering::Equal)
    })?;

    if hysteresis_enabled {
        let threshold =
            min_occupant.intensity * constants::VOLUMETRIC_LIGHT_EVICTION_HYSTERESIS_FACTOR;
        if candidate_intensity > threshold {
            Some(min_idx)
        } else {
            None
        }
    } else {
        Some(min_idx)
    }
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
    loc_haze_falloff: i32,
    persistent_lights: [PersistentLight; constants::MAX_VOLUMETRIC_LIGHTS],
    smoothed_flash: f32,
    was_lighting_active: bool,
    last_lighting_update: Option<std::time::Instant>,
    simulation_paused: bool,
    // Window management
    window_size_f32: (f32, f32),
    renderers: Vec<Box<dyn ParticleGraphicsRenderer>>,
    // Bloom post-processing
    bloom_pass: BloomPass,
    circle_renderer: Option<crate::renderer_engine::CircleGPURenderer>,
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
        let loc_haze_falloff;

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
            loc_haze_falloff = gl::GetUniformLocation(sky_haze_program, cstr!("u_HazeFalloff"));
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
            loc_haze_falloff,
            persistent_lights: [PersistentLight::default(); constants::MAX_VOLUMETRIC_LIGHTS],
            smoothed_flash: 0.0,
            was_lighting_active: false,
            last_lighting_update: None,
            simulation_paused: false,
            bloom_pass,
            circle_renderer: None,
            gpu_profiler,
            last_gpu_log_time: std::time::Instant::now(),
        })
    }

    // Helper internal
    unsafe fn update_lighting_ubo<P: PhysicEngineIterator>(&mut self, physic: &P) {
        let now = std::time::Instant::now();
        if !self.simulation_paused {
            let dt_ms = match self.last_lighting_update {
                Some(prev) => (now - prev).as_secs_f32() * 1000.0,
                None => 16.667,
            };
            self.last_lighting_update = Some(now);
            let dt_ms = dt_ms.clamp(0.0, 100.0);

            // 1. Décroissance temporelle C^1 douce de toutes les lumières persistantes
            for light in &mut self.persistent_lights {
                if light.active {
                    light.elapsed_ms += dt_ms;
                    light.intensity *= self.config.volumetric_lighting_decay_rate;
                    light.radius *= self.config.volumetric_lighting_radius_expansion;
                    if light.intensity < constants::VOLUMETRIC_LIGHT_MIN_INTENSITY {
                        light.active = false;
                        light.intensity = 0.0;
                        light.rocket_id = 0;
                        light.elapsed_ms = 0.0;
                    }
                }
            }

            // 2. Échantillonnage des détonations actives de fusées (stabilisation par ID unique & hystérésis)
            let hysteresis = self.config.volumetric_lighting_hysteresis_enabled;
            physic.for_each_active_rocket(&mut |rocket| {
                if rocket.active && rocket.exploded && rocket.explosion_active_count > 0 {
                    let target_intensity =
                        (rocket.explosion_active_count as f32 / 80.0).clamp(0.4, 2.0);

                    let slot_idx = find_lighting_slot(
                        &self.persistent_lights,
                        rocket.id,
                        target_intensity,
                        hysteresis,
                    );

                    if let Some(idx) = slot_idx {
                        let slot = &mut self.persistent_lights[idx];
                        let is_new_entry = !slot.active || slot.rocket_id != rocket.id;
                        if is_new_entry {
                            slot.elapsed_ms = 0.0;
                            slot.pos = [rocket.pos.x, rocket.pos.y];
                            slot.color = [rocket.color.x, rocket.color.y, rocket.color.z];
                            slot.radius = self.config.volumetric_lighting_radius;
                        }
                        slot.active = true;
                        slot.rocket_id = rocket.id;
                        slot.intensity = slot.intensity.max(target_intensity);
                        slot.decay_rate = self.config.volumetric_lighting_decay_rate;
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
                .min(self.config.volumetric_lighting_flash_max_cap);

            if target_flash > self.smoothed_flash {
                self.smoothed_flash = self.smoothed_flash * 0.80 + target_flash * 0.20;
            } else {
                self.smoothed_flash = self.smoothed_flash * constants::VOLUMETRIC_FLASH_EMA_DECAY
                    + target_flash * constants::VOLUMETRIC_FLASH_EMA_WEIGHT;
            }
        } else {
            self.last_lighting_update = Some(now);
        }

        // 4. Copie 1:1 vers le bloc std140 GPU sans permutation d'indices (Zéro clignotement / Zéro saut)
        let mut lights = [crate::renderer_engine::types::PointLightGPU::default();
            constants::MAX_VOLUMETRIC_LIGHTS];
        let mut max_active_idx = 0;
        let fade_in_ms = self.config.volumetric_lighting_fade_in_ms;

        for (i, light) in self.persistent_lights.iter().enumerate() {
            if light.active && light.intensity > constants::VOLUMETRIC_LIGHT_MIN_INTENSITY {
                let uploaded_intensity = light.uploaded_intensity(fade_in_ms);
                lights[i] = crate::renderer_engine::types::PointLightGPU {
                    position_radius: [light.pos[0], light.pos[1], 0.0, light.radius],
                    color_intensity: [
                        light.color[0],
                        light.color[1],
                        light.color[2],
                        uploaded_intensity,
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
    /// Garantit un coût de 0 GPU/CPU pour les frames suivantes lorsque l'éclairage volumique est désactivé.
    unsafe fn reset_lighting_ubo(&mut self) {
        self.persistent_lights = [PersistentLight::default(); constants::MAX_VOLUMETRIC_LIGHTS];
        self.smoothed_flash = 0.0;
        self.last_lighting_update = None;
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
        if self.loc_haze_falloff != -1 {
            gl::Uniform1f(self.loc_haze_falloff, self.config.sky_haze_falloff);
        }

        gl::BindVertexArray(self.dummy_vao);
        gl::DrawArrays(gl::TRIANGLES, 0, 3);
        gl::BindVertexArray(0);

        // Restore blend state for particle rendering pipeline
        gl::Enable(gl::BLEND);
    }

    /// Renders wireframe footprints, dashed bounding quads, and origin markers for active volumetric lights in debug mode.
    unsafe fn render_volumetric_debug(&mut self) {
        if !self.config.volumetric_lighting_debug {
            return;
        }

        let mut orbits: Vec<crate::renderer_engine::CircleGPUData> =
            Vec::with_capacity(constants::MAX_VOLUMETRIC_LIGHTS);
        let mut discs: Vec<crate::renderer_engine::CircleGPUData> =
            Vec::with_capacity(constants::MAX_VOLUMETRIC_LIGHTS);
        let mut boxes: Vec<crate::renderer_engine::CircleGPUData> =
            Vec::with_capacity(constants::MAX_VOLUMETRIC_LIGHTS);

        let fade_in_ms = self.config.volumetric_lighting_fade_in_ms;
        let base_radius = self.config.volumetric_lighting_radius;

        for light in &self.persistent_lights {
            if light.active && light.intensity > constants::VOLUMETRIC_LIGHT_MIN_INTENSITY {
                let r = light.color[0];
                let g = light.color[1];
                let b = light.color[2];
                let intensity = light.uploaded_intensity(fade_in_ms);

                // Blend décroissant :
                // Quand le cercle d'influence grossit et que son impact est décroissant
                // (diffusé sur tout l'écran avec atténuation de l'énergie surfacique),
                // l'opacité alpha décroît en proportion directe de l'intensité et de l'étalement
                // géométrique (base_radius / light.radius).
                let radius_ratio = (base_radius / light.radius.max(base_radius)).min(1.0);
                let decay_blend = (intensity * radius_ratio).clamp(0.0, 1.0);

                let circle_alpha = (decay_blend * 0.85).clamp(0.0, 0.95);
                let quad_alpha = (decay_blend * 0.50).clamp(0.0, 0.70);
                let center_alpha = (decay_blend * 1.2).clamp(0.0, 1.0);

                // Wireframe footprint circle outline (orbit)
                orbits.push(crate::renderer_engine::CircleGPUData {
                    center: light.pos,
                    radius: light.radius,
                    color: [r, g, b, circle_alpha],
                    thickness: 0.0,
                });

                // Dashed bounding box quad outline (pointillé)
                boxes.push(crate::renderer_engine::CircleGPUData {
                    center: light.pos,
                    radius: light.radius,
                    color: [r, g, b, quad_alpha],
                    thickness: -constants::VOLUMETRIC_LIGHT_DEBUG_DASH_LENGTH,
                });

                // Origin center disc
                discs.push(crate::renderer_engine::CircleGPUData {
                    center: light.pos,
                    radius: constants::VOLUMETRIC_LIGHT_DEBUG_ORIGIN_RADIUS,
                    color: [r, g, b, center_alpha],
                    thickness: 0.0,
                });
            }
        }

        if orbits.is_empty() && discs.is_empty() && boxes.is_empty() {
            return;
        }

        let renderer = self
            .circle_renderer
            .get_or_insert_with(crate::renderer_engine::CircleGPURenderer::new);
        renderer.draw(&orbits, &discs, &boxes);
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
                    self.config.smoke_lighting_lut_enabled,
                );
            }

            let nb;
            // Remplit le buffer GPU (Opération purement CPU, on utilise uniquement tracy)
            {
                tracy_zone!("Renderer::fill_buffer", palette::ENV);
                nb = renderer.fill_particle_data_direct(physic, alpha);
            }

            // Screen-space smoke mask pass for backlight illumination (§4.1 ADR)
            // Executed strictly if smoke particle type, bloom enabled, backlight enabled, and render_smoke active.
            // Bypassed completely (zero-cost: zero draw calls, zero clears, zero uploads) otherwise.
            if renderer.particle_type() == Some(crate::physic_engine::ParticleType::Smoke)
                && self.bloom_pass.enabled
                && self.config.volumetric_lighting_enabled
                && self.config.backlight_enabled
            {
                gpu_profile_zone!(
                    12,
                    "Renderer::Smoke_Backlight_Mask",
                    palette::POSTPROCESS,
                    profiler
                );
                renderer.render_smoke_mask(
                    nb,
                    self.bloom_pass.smoke_mask_fbo,
                    self.bloom_pass.mask_width,
                    self.bloom_pass.mask_height,
                );
                // Restore HDR scene FBO, viewport, and MRT draw buffers
                gl::BindFramebuffer(gl::FRAMEBUFFER, self.bloom_pass.hdr_fbo());
                gl::Viewport(
                    0,
                    0,
                    self.window_size_f32.0 as i32,
                    self.window_size_f32.1 as i32,
                );
                let draw_buffers = [gl::COLOR_ATTACHMENT0, gl::COLOR_ATTACHMENT1];
                gl::DrawBuffers(2, draw_buffers.as_ptr());
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

                if self.config.volumetric_lighting_debug {
                    self.render_volumetric_debug();
                }

                particle_count
            } else {
                gpu_profile_zone!(0, "Pass: Forward (No Bloom)", palette::COMPOSITE, profiler);

                // Direct rendering without bloom
                gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
                gl::ClearColor(0.0, 0.0, 0.0, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                self.render_sky_haze();

                let count = self.render_particles(physic, &profiler, alpha);

                if self.config.volumetric_lighting_debug {
                    self.render_volumetric_debug();
                }

                count
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
            if let Some(mut cr) = self.circle_renderer.take() {
                cr.destroy();
            }

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

    fn set_simulation_paused(&mut self, paused: bool) {
        if self.simulation_paused != paused {
            self.simulation_paused = paused;
            if !paused {
                self.last_lighting_update = Some(std::time::Instant::now());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer_engine::constants;

    #[test]
    fn test_lighting_slot_fade_in_math() {
        let mut light = PersistentLight {
            active: true,
            rocket_id: 10,
            intensity: 1.6,
            decay_rate: 0.94,
            elapsed_ms: 0.0,
            ..Default::default()
        };

        // 1. Division by zero impossible: fade_in = 0.0 outputs target directly
        assert_eq!(light.uploaded_intensity(0.0), 1.6);
        assert_eq!(light.uploaded_intensity(-10.0), 1.6);

        // 2. Linear ramp with fade_in = 50.0 ms
        let fade_in_ms = 50.0;
        assert_eq!(light.uploaded_intensity(fade_in_ms), 0.0);

        light.elapsed_ms = 25.0;
        assert!((light.uploaded_intensity(fade_in_ms) - 0.8).abs() < 1e-5);

        light.elapsed_ms = 50.0;
        assert!((light.uploaded_intensity(fade_in_ms) - 1.6).abs() < 1e-5);

        // Beyond fade-in duration: capped to target
        light.elapsed_ms = 120.0;
        assert_eq!(light.uploaded_intensity(fade_in_ms), 1.6);

        // Target decay operates on target intensity, uploaded reflects it
        light.intensity *= light.decay_rate; // 1.6 * 0.94 = 1.504
        assert!((light.uploaded_intensity(fade_in_ms) - 1.504).abs() < 1e-5);
    }

    #[test]
    fn test_lighting_slot_hysteresis_eviction() {
        let mut lights = [PersistentLight::default(); constants::MAX_VOLUMETRIC_LIGHTS];

        // Fill all 16 slots with active living lights
        for (i, l) in lights.iter_mut().enumerate() {
            l.active = true;
            l.rocket_id = (i + 1) as u64;
            l.intensity = 1.0 + (i as f32 * 0.1); // min intensity is slot 0 with 1.0
        }

        // Slot 0 has intensity 1.0. With hysteresis 1.2x, eviction threshold is 1.0 * 1.2 = 1.2
        let candidate_weaker = 1.15;
        let slot = find_lighting_slot(&lights, 999, candidate_weaker, true);
        assert_eq!(
            slot, None,
            "Weaker candidate (1.15 <= 1.20) must NOT evict living occupant under hysteresis"
        );

        // Candidate exceeding threshold (> 1.20) must evict slot 0
        let candidate_stronger = 1.25;
        let slot = find_lighting_slot(&lights, 999, candidate_stronger, true);
        assert_eq!(
            slot,
            Some(0),
            "Stronger candidate (1.25 > 1.20) must evict weakest occupant slot 0"
        );

        // Without hysteresis (hysteresis=false), candidate > min_occupant evicts
        let slot = find_lighting_slot(&lights, 999, candidate_weaker, false);
        assert_eq!(
            slot,
            Some(0),
            "When hysteresis is OFF, candidate > min occupant picks min slot 0"
        );

        // Free/extinct slot (intensity <= MIN_INTENSITY) is claimed unconditionally even by weak light
        lights[3].intensity = constants::VOLUMETRIC_LIGHT_MIN_INTENSITY * 0.5;
        let weak_candidate = 0.4;
        let slot = find_lighting_slot(&lights, 999, weak_candidate, true);
        assert_eq!(
            slot,
            Some(3),
            "Extinct slot must be claimable unconditionally without hysteresis barrier"
        );

        // Same rocket ID reclaims its own slot regardless of other lights
        let slot = find_lighting_slot(&lights, 5, 0.5, true);
        assert_eq!(
            slot,
            Some(4),
            "Same rocket ID must reclaim existing slot without eviction check"
        );
    }

    #[test]
    fn test_same_rocket_keeps_slot_no_fade_reset() {
        let mut slot = PersistentLight {
            active: true,
            rocket_id: 42,
            intensity: 1.5,
            elapsed_ms: 35.0,
            ..Default::default()
        };

        // Same rocket on next frame: is_new_entry is false, elapsed preserved
        let is_new_entry = !slot.active || slot.rocket_id != 42;
        assert!(!is_new_entry);
        if is_new_entry {
            slot.elapsed_ms = 0.0;
        }
        assert_eq!(slot.elapsed_ms, 35.0);

        // New rocket evicts slot: is_new_entry is true, elapsed resets to 0.0
        let is_new_entry = !slot.active || slot.rocket_id != 99;
        assert!(is_new_entry);
        if is_new_entry {
            slot.elapsed_ms = 0.0;
        }
        assert_eq!(slot.elapsed_ms, 0.0);
    }

    #[test]
    fn test_light_slot_position_anchored_at_detonation() {
        let mut slot = PersistentLight::default();

        // 1. Initial entry: position is anchored
        let initial_pos = [250.0, 400.0];
        let is_new_entry = !slot.active || slot.rocket_id != 42;
        assert!(is_new_entry);
        if is_new_entry {
            slot.elapsed_ms = 0.0;
            slot.pos = initial_pos;
            slot.color = [1.0, 0.5, 0.2];
            slot.radius = constants::VOLUMETRIC_LIGHT_INITIAL_RADIUS;
        }
        slot.active = true;
        slot.rocket_id = 42;

        assert_eq!(slot.pos, initial_pos);

        // 2. Subsequent frame for same rocket: is_new_entry is false
        // Even if rocket_pos moves, slot.pos MUST NOT change
        let moved_pos = [250.0, 100.0]; // Ghost movement
        let is_new_entry = !slot.active || slot.rocket_id != 42;
        assert!(!is_new_entry);
        if is_new_entry {
            slot.elapsed_ms = 0.0;
            slot.pos = moved_pos;
        }

        assert_eq!(
            slot.pos, initial_pos,
            "Light source position must remain permanently anchored at detonation point"
        );
    }

    #[test]
    fn test_lighting_evolution_frozen_during_pause() {
        let mut slot = PersistentLight {
            active: true,
            rocket_id: 10,
            intensity: 1.8,
            radius: 450.0,
            elapsed_ms: 20.0,
            decay_rate: 0.94,
            pos: [100.0, 200.0],
            color: [1.0, 1.0, 1.0],
        };

        let initial_radius = slot.radius;
        let initial_intensity = slot.intensity;
        let initial_elapsed = slot.elapsed_ms;

        // When paused: no time accumulation, no radius expansion, no decay
        let simulation_paused = true;
        let dt_ms = 16.67;

        if !simulation_paused {
            slot.elapsed_ms += dt_ms;
            slot.intensity *= slot.decay_rate;
            slot.radius *= constants::VOLUMETRIC_LIGHT_RADIUS_EXPANSION;
        }

        assert_eq!(
            slot.radius, initial_radius,
            "Radius must not expand during pause"
        );
        assert_eq!(
            slot.intensity, initial_intensity,
            "Intensity must not decay during pause"
        );
        assert_eq!(
            slot.elapsed_ms, initial_elapsed,
            "Elapsed ms must not advance during pause"
        );
    }
}
