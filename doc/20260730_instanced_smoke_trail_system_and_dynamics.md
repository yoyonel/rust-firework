# Spécification Technique & Architecture : Moteur de Trainée de Fumée Instancié & Érosion Alpha (Dissolve)

Date : 30-31 juillet 2026
Branche : `feature/smoke-trail-system`

---

## 1. Contexte et Objectifs

L'objectif de cette fonctionnalité est d'intégrer un système de trainées de fumée volumétriques avec un effet d'**Érosion Alpha (Dissolve GPU)** stylisé dans le moteur `rust-firework`.

### Exigences Visuelles et de Performance
1. **Émission Continue en Ascension** : Émission de particules de fumée uniquement lorsque la fusée est active et non explosée (`!rocket.exploded`). L'émission s'arrête immédiatement lors de l'explosion.
2. **Origine d'Émission Précise** : La fumée s'échappe de la tuyère de combustion à la base du corps cylindrique de la fusée (`rocket.base_pos()`).
3. **Dissipation par Érosion Alpha (Dissolve)** : Remplacement du simple fondu alpha global par une érosion de masque de bruit Perlin (`noise.png`). Plus la particule vieillit (âge normalisé $vNormalizedAge \in [0.0, 1.0]$), plus la carte de bruit "mange" la texture de fumée en créant des trous et des bordures déchiquetées bio-organiques.
4. **Couture Incandescente d'Érosion (Burn Seam)** : Détection de la frontière de déchirure (`noiseTex.r < erosionThreshold + u_ErosionEdgeWidth`) et application d'une couleur incandescente personnalisable (`u_ErosionEdgeColor`).
5. **Désactivation & Contrôle d'Agressivité** : Possibilité de basculer l'effet d'érosion en marche/arrêt (`smoke_erosion_enabled`) et d'ajuster l'agressivité de la dissolution (`smoke_erosion_scale`).
6. **Interface ImGui Dédiée & Preview ISO GPU FBO** : Onglet ImGui sur-mesure `"Smoke & Erosion"` doté d'un canevas de prévisualisation temps réel **100% BIT-FOR-BIT ISO** rendu dans un Framebuffer OpenGL (FBO) exécutant les vrais shaders GLSL du jeu.
7. **Contrôles de Viewport 3D (Unreal/Unity)** : Navigation interactive dans le canevas de preview :
   - **Glisser Clic-Milieu (`Middle Drag`)** : Translation Pan X / Y.
   - **Glisser Clic-Droit (`Right Drag`)** : Rotation Z euclidienne rigide isotropique (0° à 360°).
   - **Molette (`Mouse Wheel`)** : Zoom / Dézoom (`0.4x` à `3.5x`) avec isolation stricte (`WindowFlags::NO_SCROLL_WITH_MOUSE` & `igGetIO().MouseWheel = 0.0`).

---

## 2. Architecture & Pipeline GPU

```mermaid
flowchart TD
    A["Rocket (Ascension)"] -->|base_pos()| B["SmokeSystem (Physics Engine)"]
    B -->|for_each_particle_of_type(Smoke)| C["SmokeRenderer (GPU Persistent VBO)"]
    C -->|DrawArraysInstanced (Pass 10)| D["FrameBuffer / Blend SRC_ALPHA"]
    E["PhysicConfig / physic.toml"] -->|SSOT Parameters| B
    F["ImGui Dedicated Smoke & Erosion Tab"] -->|Live Sliders & Presets| E
    F -->|Render Scene to FBO| G["Offscreen FBO Preview (smoke_instanced.frag.glsl)"]
    G -->|imgui::Image TextureId| F
```

### 2.1 Moteur Physique (`SmokeSystem` & `SmokeParticle`)
- **Structure de la Particule** ([`src/physic_engine/smoke_system.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/smoke_system.rs)) :
  ```rust
  pub struct SmokeParticle {
      pub pos: Vec2,
      pub vel: Vec2,
      pub color: Color,
      pub initial_size: f32,
      pub current_size: f32,
      pub growth_rate: f32,
      pub initial_alpha: f32,
      pub alpha: f32,
      pub rotation: f32,
      pub age: f32,
      pub max_life: f32,
      pub active: bool,
  }
  ```
- **Position de la Base de Fusée** ([`src/physic_engine/rocket.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs)) :
  La tuyère se situe à la jonction du corps cylindrique et de la baguette. Elle est calculée en reculant de $6.0$ unités le long du vecteur inverse de déplacement :
  ```rust
  pub fn base_pos(&self) -> Vec2 {
      let dir = if self.vel.length_squared() > 0.001 { self.vel.normalize() } else { Vec2::Y };
      self.pos - dir * 6.0
  }
  ```

---

### 2.2 Technique d'Érosion Alpha dans le Shader GLSL

- **Vertex Shader** ([`assets/shaders/smoke_instanced.vert.glsl`](file:///home/latty/Prog/__PERSO__/rust-firework/assets/shaders/smoke_instanced.vert.glsl)) :
  Transmet l'âge normalisé de la particule $vNormalizedAge \in [0.0, 1.0]$ au fragment shader.

- **Fragment Shader** ([`assets/shaders/smoke_instanced.frag.glsl`](file:///home/latty/Prog/__PERSO__/rust-firework/assets/shaders/smoke_instanced.frag.glsl)) :
  ```glsl
  #version 330 core
  in vec2 vUV;
  in float vAlpha;
  in float vIntensity;
  in vec3 vColor;
  in float vNormalizedAge;

  layout(location = 0) out vec4 FragColor;
  layout(location = 1) out vec4 BrightColor;

  uniform sampler2D u_SmokeTexture;
  uniform sampler2D u_NoiseTexture;
  uniform bool u_ErosionEnabled;
  uniform float u_ErosionScale;
  uniform float u_ErosionEdgeWidth;
  uniform vec3 u_ErosionEdgeColor;

  void main() {
      vec4 smokeTex = texture(u_SmokeTexture, vUV);

      vec3 finalColor = smokeTex.rgb * vColor;
      float finalAlpha = smokeTex.a * vAlpha;

      if (u_ErosionEnabled) {
          vec4 noiseTex = texture(u_NoiseTexture, vUV);
          float erosionThreshold = clamp(vNormalizedAge * u_ErosionScale, 0.0, 1.0);

          // Discard des fragments rongés par le bruit
          if (noiseTex.r < erosionThreshold) {
              discard;
          }

          // Couture incandescente d'érosion
          if (noiseTex.r < erosionThreshold + u_ErosionEdgeWidth) {
              float edgeFactor = (noiseTex.r - erosionThreshold) / max(0.0001, u_ErosionEdgeWidth);
              finalColor = mix(u_ErosionEdgeColor, finalColor, edgeFactor);
              finalAlpha = min(1.0, finalAlpha * 1.5);
          }
      }

      FragColor = vec4(finalColor * vIntensity, finalAlpha * vIntensity);
      BrightColor = vec4(0.0, 0.0, 0.0, 0.0);
  }
  ```

---

### 2.3 Single Source of Truth (SSOT) & Paramètres d'Érosion

Toutes les propriétés de la fumée et de l'érosion sont enregistrées dans la structure SSOT `PhysicConfig` ([`src/physic_engine/config.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/config.rs)) et persister dans l'inventaire GUI ([`doc/gui_persistence_inventory.md`](file:///home/latty/Prog/__PERSO__/rust-firework/doc/gui_persistence_inventory.md)) :

| Paramètre | Fichier SSOT | Description | Valeur par Défaut |
| --- | --- | --- | --- |
| `smoke_erosion_enabled` | `constants.rs` / `config.rs` | Activation/Désactivation de l'érosion de bruit | `true` |
| `smoke_erosion_scale` | `constants.rs` / `config.rs` | Agressivité/Vitesse de la dissolution ($0.0 \dots 2.0$) | `1.0` |
| `smoke_erosion_edge_width` | `constants.rs` / `config.rs` | Largeur de la bordure incandescente ($0.00 \dots 0.80$) | `0.08` |
| `smoke_erosion_edge_color` | `constants.rs` / `config.rs` | Couleur de la couture incandescente RGB | `[1.0, 0.45, 0.15]` |
| `smoke_spawn_rate` | `constants.rs` / `config.rs` | Taux d'émission (particules/s) | `30.0` |
| `smoke_initial_size` | `constants.rs` / `config.rs` | Taille initiale du sprite | `10.0` |
| `smoke_growth_rate_multiplier` | `constants.rs` / `config.rs` | Facteur d'expansion | `1.2` |
| `smoke_fade_duration` | `constants.rs` / `config.rs` | Durée du fondu (s) | `0.75` |
| `max_smoke_particles` | `constants.rs` / `config.rs` | Capacité maximale du pool GPU | `2048` |
| `smoke_intensity` | `constants.rs` / `config.rs` | Multiplicateur de luminosité/fondu | `0.5` |
| `smoke_color_mode` | `constants.rs` / `config.rs` | Mode de couleur (`RocketColor` / `Custom`) | `RocketColor` |
| `smoke_custom_color` | `constants.rs` / `config.rs` | Couleur personnalisée RGB | `[0.85, 0.85, 0.85]` |
| `smoke_inherited_color_intensity` | `constants.rs` / `config.rs` | Intensité de la couleur d'origine héritée ($0.0 \dots 2.0$) | `1.0` |

---

## 3. Architecture du Preview ISO GPU FBO & Contrôles Viewport

L'onget ImGui `"Smoke & Erosion"` ([`src/simulator/gui_settings/smoke.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/simulator/gui_settings/smoke.rs)) intègre un composant hors-écran `SmokePreviewGpu` :

1. **Rendu Hors-Écran (FBO 480x200)** : Rendu OpenGL direct utilisant la texture de fusée réelle, la texture de fumée, le masque de bruit Perlin et le fragment shader `smoke_instanced.frag.glsl`.
2. **Matrice Isotropique Rigide (Zero-Distortion)** :
   Calculation explicite du ratio d'aspect du canevas (`canvas_aspect = avail_width / 145.0`) pour adapter les limites de simulation (`sim_w = sim_h * canvas_aspect`). Garantit que $1\text{px X} = 1\text{px Y}$ sur l'écran et qu'aucune déformation d'échelle ou d'aplatissement de la fusée ne se produit lors des rotations.
3. **Isolation Complète du Focus Souris** :
   - `imgui::WindowFlags::NO_SCROLL_WITH_MOUSE | imgui::WindowFlags::NO_SCROLLBAR` sur la fenêtre enfant.
   - Manipulation réservée à l'élément image via `ui.is_item_hovered()`.
   - Réinitialisation de `igGetIO().MouseWheel = 0.0` sur survol pour empêcher tout défilement résiduel de la fenêtre ImGui parente.

---

## 4. Couverture de Tests & Validation

- **Tests d'Intégration** ([`tests/smoke_system_integration_test.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/tests/smoke_system_integration_test.rs)) :
  - `test_physic_engine_smoke_erosion_params_and_toggle` : Validation des paramètres d'érosion et du basculement ON/OFF.
- **Vérification de Persistance GUI** : Script automatisé `task gui-persistence-check` validé.
- **Vérification Qualité** : `task lint` et `cargo test -- --test-threads=1` validés (221/221 passés).
