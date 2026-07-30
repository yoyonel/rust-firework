# Refactoring Architectural SSOT & Éradication des Valeurs Magiques

## 1. Objectif & Contexte

Ce document décrit le refactoring architectural global visant à éradiquer l'ensemble des constantes codées en dur ("magic numbers"), valeurs implicites et structures de configuration dispersées dans l'ensemble de la codebase de `rust-firework` (Moteurs Audio, Physique et Rendu).

L'ensemble des constantes a été extrait et centralisé au sein d'une architecture **Single Source of Truth (SSOT)** avec une documentation Rustdoc exhaustive pour chaque valeur.

---

## 2. Architecture SSOT par Domaine

Chaque moteur métier dispose désormais d'un module dédié `constants.rs` exposant des constantes typées avec `const` / `static` sans aucun surcoût à l'exécution :

### 2.1 Moteur Audio (`src/audio_engine/constants.rs`)

Le module [`audio_engine::constants`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/constants.rs) centralise :
- **Configuration matérielle & buffers :** `HARDWARE_BUFFER_SIZE` (16384 samples), `PLAY_REQUEST_CHANNEL_CAPACITY` (512), `DEBUG_EVENT_CHANNEL_CAPACITY` (2048), `DEFAULT_SAMPLE_RATE` (48000 Hz), `DEFAULT_BLOCK_SIZE` (512 samples).
- **Gestion des voix :** `DEFAULT_MAX_VOICES` (256), `MIN_VOICES_FLOOR` (64), `VOICE_SAFETY_MULTIPLIER` (4).
- **Physique acoustique & Spatialisateur binaural :** `SPEED_OF_SOUND_M_S` (343.0 m/s), `REFERENCE_DISTANCE_METERS` (50.0 m), `MIN_DISTANCE_EPSILON` (1e-6 m), `MAX_ITD_SECONDS` (0.001 s), `ILD_ELEVATION_ATTENUATION_FACTOR` (0.25), `DEFAULT_HEAD_RADIUS` (0.0875 m), `DEFAULT_MAX_ILD_DB` (18.0 dB), `DEFAULT_MAX_DISTANCE` (2000.0 m), `DEFAULT_GLOBAL_GAIN` (0.8), `DEFAULT_FADE_IN_MS` (20.0 ms), `DEFAULT_FADE_OUT_MS` (50.0 ms), `DEFAULT_F_MIN_HZ` (1000.0 Hz), `DEFAULT_F_MAX_HZ` (15000.0 Hz), `DEFAULT_DISTANCE_ALPHA` (0.0025).
- **Réverbération Spatiale (Schroeder/FDN) :** `REVERB_BASE_SAMPLE_RATE` (44100.0 Hz), `REVERB_COMB_DELAYS_BASE_SAMPLES` ([1553, 2129, 2801, 3547]), `REVERB_STEREO_UNCORRELATION_OFFSET_SAMPLES` (47), `REVERB_ALLPASS_DELAYS_BASE_SAMPLES` ([641, 317]), `REVERB_DEFAULT_FEEDBACK` (0.68), `REVERB_DEFAULT_DAMPING` (0.50), `REVERB_DEFAULT_WET_GAIN` (0.08).
- **Limites FFT & Convoluteur HRTF :** `FFT_MIN_CHUNK_BOUND` (128), `FFT_MAX_CHUNK_BOUND` (512).

---

### 2.2 Moteur Physique (`src/physic_engine/constants.rs`)

Le module [`physic_engine::constants`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/constants.rs) centralise :
- **Capacités Arena & Pools :** `DEFAULT_MAX_ROCKETS` (1024), `DEFAULT_PARTICLES_PER_EXPLOSION` (256), `DEFAULT_PARTICLES_PER_TRAIL` (64).
- **Cinématique & Gravité :** `DEFAULT_GRAVITY` (-200.0 m/s²), `DEFAULT_INITIAL_ROCKET_SPEED` (100.0 m/s), `DEFAULT_SPAWN_ROCKET_MIN_SPEED` (350.0 m/s), `DEFAULT_SPAWN_ROCKET_MAX_SPEED` (500.0 m/s), `DEFAULT_SPAWN_ROCKET_MARGIN` (50.0 m).
- **Angles & Intervalles :** `DEFAULT_SPAWN_ROCKET_VERTICAL_ANGLE` (π/2 rad), `DEFAULT_SPAWN_ROCKET_ANGLE_VARIATION` (0.3 rad ≈ 17°), `DEFAULT_ROCKET_INTERVAL_MEAN` (0.025 s), `DEFAULT_ROCKET_INTERVAL_VARIATION` (0.01875 s), `DEFAULT_ROCKET_MAX_NEXT_INTERVAL` (0.025 s).
- **Seuils & Anticipation Audio :** `DEFAULT_EXPLOSION_THRESHOLD_SPEED` (50.0 m/s), `DEFAULT_EXPLOSION_MIN_VELOCITY` (60.0 m/s), `DEFAULT_EXPLOSION_MAX_VELOCITY` (200.0 m/s), `DEFAULT_AUDIO_LAUNCH_ANTICIPATION_MS` (25.0 ms), `DEFAULT_AUDIO_EXPLOSION_ANTICIPATION_MS` (25.0 ms), `IMAGE_SHAPE_THRESHOLD` (128).

---

### 2.3 Moteur de Rendu (`src/renderer_engine/constants.rs`)

Le module [`renderer_engine::constants`](file:///home/latty/Prog/__PERSO__/rust-firework/src/renderer_engine/constants.rs) centralise :
- **Configuration Bloom & Post-processing :** `DEFAULT_BLOOM_ENABLED` (true), `DEFAULT_BLOOM_INTENSITY` (1.5), `DEFAULT_BLOOM_ITERATIONS` (3), `DEFAULT_BLOOM_DOWNSAMPLE` (2), `DEFAULT_BLOOM_BLUR_METHOD` (Gaussian), `DEFAULT_TONE_MAPPING_MODE` (KhronosPBR).
- **Caméra & Projection :** `CAMERA_DEFAULT_FOV_DEGREES` (45.0°), `CAMERA_DEFAULT_NEAR_PLANE` (0.1 m), `CAMERA_DEFAULT_FAR_PLANE` (1000.0 m), `DEFAULT_CLEAR_COLOR` ([0.05, 0.05, 0.05, 1.0]).
- **Géométrie :** `QUAD_VERTICES` (coordonnées quad NDC), `CIRCLE_OUTLINE_SEGMENTS` (64 vertices), `CIRCLE_RADIUS_MULTIPLIER` (0.5).
- **Overlay Visualisation Audio :** `AUDIO_EVENT_LAUNCH_TTL_SECS` (0.55 s), `AUDIO_EVENT_EXPLOSION_TTL_SECS` (0.75 s).
- **Chemins Shaders :** `SHADER_POINT_VERTEX_PATH`, `SHADER_POINT_FRAGMENT_PATH`.

---

## 3. Norme de Documentation Exigée

Chaque constante documentée comporte obligatoirement la structure Rustdoc suivante :
- **Unit of measurement:** (ex: `ms`, `m/s`, `radians`, `samples`, `Hz`, `count`, `dB`).
- **Technical/Physical meaning:** Signification exacte dans le modèle d'exécution.
- **Orders of magnitude & bounds:** Gamme de valeurs acceptables (`min..max`).
- **System influence:** Impact précis sur la mémoire, le CPU, la latence ou le rendu.
