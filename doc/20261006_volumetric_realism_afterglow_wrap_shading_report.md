# Rapport Technique : Réalisme Volumétrique — Afterglow Spectral Atmosphérique & Shading Sphérique 3D (Half-Lambert Wrap)

**Date :** 6 Octobre 2026  
**Branche :** `feat/volumetric-smoke-lighting`  
**Plateforme :** Mesa Intel(R) Iris(R) Xe Graphics (RPL-U) / Linux X11 (`DISPLAY=:0.0`)  
**Statut :** Validé (0 Régression Visuelle, 0 Violation OpenGL Mesa, Linters PASS, Tests PASS)

---

## 1. Contexte & Problématique Visuelle

À la suite de la Phase 1 (compaction UBO, Fast-Math RSQ et Frustum Culling), le moteur d'éclairage volumétrique affichait un excellent débit (~565 FPS). Toutefois, deux limites physiques et esthétiques identifiées dans la note de vision ADR subsistaient :

1. **Monochromie et Rupture Temporelle du Flash Ambiant :**
   Le flash atmosphérique appliqué au ciel nocturne et aux volutes de fumée était hardcodé sur une teinte bleutée statique (`[0.8, 0.8, 1.0]`), ignorant la composition pyrotechnique réelle des charges (or, émeraude, rubis, pourpre). De plus, l'extinction abrupte dès la fin de l'explosion rompait l'illusion de braise résiduelle emprisonnée dans la brume.
2. **Aspect Plat des Quads Billboard de Fumée :**
   L'accumulation de lumière était évaluée uniformément sur le quad de fumée sans tenir compte de la courbure radiale de la bouffée. Les particules apparaissaient comme des disques 2D plats plutôt que des volumes de fumée tridimensionnels.

---

## 2. Solutions Techniques Implémentées

### 2.1 Canal Afterglow Spectral Atmosphérique (CPU-Side, Zéro Coût GPU)
Dans `src/renderer_engine/renderer.rs` :
- **Suivi d'Énergie et de Teinte Pondérée :** Chaque frame active calcule le barycentre chromatique des lumières actives :
  $$\vec{C}_{\text{dominant}} = \frac{\sum I_k \cdot \vec{C}_k}{\sum I_k}$$
- **Double Dynamique Temporelle Asymétrique :**
  - *Attaque instantanée (Flash) :* Dès qu'une explosion survient, la couleur ambiante absorbe rapidement ($25\%$/frame) la teinte dominante.
  - *Décroissance Spectrale Résiduelle (Afterglow) :* À l'extinction, un filtre exponentiel lent ($\times 0.985$/frame) maintient une lueur spectrale chaude dans l'atmosphère pendant 1 à 2 secondes avant de converger doucement vers le crépuscule neutre `DEFAULT_VOLUMETRIC_AMBIENT_TINT`.
- **Injection UBO Directe :** Transmis sans allocation supplémentaire via `u_AmbientLight` dans le bloc GPU std140.

### 2.2 Shading Sphérique 3D (Half-Lambert Wrap) & Diffusion Mie (Vertex Stage)
Dans `assets/shaders/smoke_instanced.vert.glsl` :
- **Normale Radiale Unitaire du Quad :**
  Calculée analytiquement sans racine carrée ni division grâce à la géométrie canonique du quad $[-1, 1]$ :
  $$\vec{N}_{\text{puff}} = \mathbf{Rot} \cdot (\vec{a}_{\text{Quad}} \cdot \frac{1}{\sqrt{2}})$$
- **Shading Directionnel Enveloppant (Half-Lambert Wrap) :**
  $$\text{wrap} = \text{clamp}((\vec{N}_{\text{puff}} \cdot \vec{L}_{\text{dir}}) \times 0.35 + 0.65, 0.3, 1.0)$$
  - La face orientée vers la détonation capte $100\%$ de la lumière directe.
  - La face opposée reçoit un éclairage rasant doux ($30\%$), simulant la transmission interne et l'auto-occlusion de la particule.
- **Phase Anisotrope Mie / Multi-Scattering :**
  $$\text{phase} = \text{wrap} \cdot (1.0 + 0.25 \cdot \text{atten})$$
  Sublime les volutes avec un effet de bord argenté (rim lighting) réaliste tout en conservant le budget 1-cycle `inversesqrt()`.

---

## 3. Intégration UI/UX, Console CLI & Persistance Canonique

Pour respecter la Règle 5 (Standards de Persistance UI) et la Règle 7 (Zéro Constante Magique & UI Scaling Dynamique), toutes les grandeurs de réalisme volumétrique sont entièrement pilotables et configurables :

### 3.1 Single Source of Truth (SSOT) — `src/renderer_engine/constants.rs`
- `DEFAULT_SMOKE_WRAP_RELIEF = 0.35` (bornes `[0.0, 1.0]`)
- `DEFAULT_SPECTRAL_AFTERGLOW_ENABLED = true`
- `DEFAULT_SPECTRAL_AFTERGLOW_DECAY = 0.985` (bornes `[0.80, 0.999]`)
- `AFTERGLOW_ATTACK_WEIGHT = 0.25`
- `AFTERGLOW_SUSTAIN_WEIGHT = 0.60`
- `VOLUMETRIC_WRAP_BASE_AMBIENT = 0.30`

### 3.2 ImGui UI/UX (`src/simulator/gui_settings/renderer.rs`)
- Section **"Atmospheric Spectral Afterglow"** :
  - Checkbox toggle activable en direct.
  - Slider de décroissance temporelle (`ui.current_font_size() * 14.0`).
- Section **"3D Smoke Billow Relief (Wrap Shading)"** :
  - Slider enveloppant Half-Lambert (`0.0 = plat`, `0.35 = naturel`, `1.0 = relief prononcé`).
- Bouton **"Reset Volumetric Realism"** réinitialisant aux valeurs par défaut.
- Marqueur d'inventaire : `// GUI_PERSIST: renderer.config`.

### 3.3 Commandes Console Développeur (`src/simulator/console_commands/renderer.rs`)
- `renderer.smoke_wrap_relief <0.0-1.0>`
- `renderer.afterglow.enable` / `renderer.afterglow.disable` / `renderer.afterglow.toggle`
- `renderer.afterglow.decay <0.80-0.999>`
- `renderer.realism.reset`
- Providers de lecture directe pour autocomplétion et introspection.

---

## 4. Preuves de Validation & Tests

- `task lint:all` : **PASS** (Clippy strict `-D warnings`, Rustfmt, Vale doc-lint 0 err).
- `task test:gui-persistence-check` : **PASS** (12/12 inventaires conformes).
- `task test:fix01-visual` : **PASS** (MSE = 0.021833, conformité golden).
- `task test:opengl-mesa` : **PASS** (0 violation de spécification OpenGL).
- `cargo test --lib` : **PASS** (100/100 tests unitaires réussis en 0.14s).

---

## 5. Runbook de Reproductibilité Humaine

```bash
# 1. Vérification des linters et du formatage
task lint:all

# 2. Vérification des spécifications OpenGL Mesa
task test:opengl-mesa

# 3. Test de non-régression visuelle FIX-01
task test:fix01-visual

# 4. Vérification de la persistance UI
task test:gui-persistence-check

# 5. Benchmark comparatif A/B
./scripts/bench_smoke_lighting_ab.sh
```
