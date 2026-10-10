# 📊 Rapport de Benchmark & Post-Mortem : Filtre Karis 13-Tap & Downscale Bloom

**Date :** 10 octobre 2026  
**Branche :** `feat/volumetric-smoke-lighting`  
**Auteur :** AI Assistant & Human Architect  
**Plateforme Matérielle :** Mesa Intel(R) Iris(R) Xe Graphics (RPL-U) — OpenGL 4.6 Core Profile  
**Statut de la Piste :** ❌ **REJETÉE (Régression de performance sur iGPU)**  

---

## 1. Contexte & Hypothèse Initiale

### 1.1. Hypothèse
Dans l'audit Tracy du 7 octobre 2026 (`doc/20261007_intel_iris_xe_tracy_profiling_audit_report.md`), la chaîne de Bloom et composition représentait **51,05 % du temps GPU (830,35 µs)**.
L'hypothèse était d'implémenter un filtre de downsample de type **Jorge Jimenez 13-tap** (utilisé dans Call of Duty et Unreal Engine) combiné à une pondération **Brian Karis average** (`1.0 / (1.0 + luma)`) lors du downscaling de la texture d'émission lumineuse HDR :
1. Éliminer le scintillement (spark popping/aliasing) des étincelles sous-pixel dans les passes floutées.
2. Permettre un passage en quart de résolution ($480 \times 270$) sans dégradation visuelle pour économiser ~350 µs sur l'iGPU.

---

## 2. Implémentation Testée

1. **Shader `assets/shaders/bloom/kawase_downsample.frag.glsl` :**
   - Échantillonnage étendu en footprint 13-tap (5 boîtes bilinéaires 2x2 chevauchantes).
   - Calcul de luminance Rec. 709 et facteur de pondération non-linéaire Karis sur la passe 0 (`uUseKaris == 1`).
   - Fast-path 5-tap Dual Kawase standard conservé pour les passes subséquentes (`uUseKaris == 0`).
2. **Câblage Moteur (`src/renderer_engine/bloom/`) :**
   - Cache de l'uniform `uUseKaris` dans `BloomPass`.
   - Activation conditionnelle à la passe initiale d'extraction HDR.

---

## 3. Résultats Comparatifs A/B (iGPU Intel Iris Xe)

Mesures sur 3 runs x 5 secondes consécutifs (Seed déterministe 42, VSync désactivée) :

| Configuration | FPS Moyen | Frame Time | Delta vs Ref (Half-Res 2x) | Gain % | Statut & Diagnostic |
| :--- | :---: | :---: | :---: | :---: | :--- |
| **Baseline 5-Tap (Full-Res 1x)** | **440.26 FPS** | **2.271 ms** | -25.34 FPS | -5.44 % | 1080p natif, saturation bande passante |
| **Baseline 5-Tap (Half-Res 2x)** | **465.60 FPS** | **2.148 ms** | **Ref** | **Ref** | **Configuration canonique existante** |
| **Baseline 5-Tap (Quarter-Res 4x)** | **493.14 FPS** | **2.028 ms** | **+27.54 FPS** | **+5.91 %** 🟢 | Gain de bande passante sans Karis |
| **Target Karis 13-Tap (Half-Res 2x)** | **429.26 FPS** | **2.330 ms** | **-36.34 FPS** | **-7.80 %** 🔴 | **Régression nette : surcoût texture iGPU** |
| **Target Karis 13-Tap (Quarter-Res 4x)** | **458.06 FPS** | **2.183 ms** | **-7.54 FPS** | **-1.62 %** 🔴 | Élimine le gain du quart de résolution |

---

## 4. Analyse Technique du Post-Mortem : Pourquoi l'Optimisation a Échoué

1. **Pression Texture Fetching sur Architecture UMA :**
   - Sur GPU dédié (NVidia/AMD), le filtrage d'échantillonnage 13-tap est absorbé par un cache texture L1/L2 large et une mémoire VRAM GDDR6 dédiée ultra-rapide (300+ Go/s).
   - Sur l'Intel Iris Xe (mémoire système DDR5 partagée ~60-80 Go/s), effectuer 13 lectures bilinéaires par fragment sur une texture HDR 1080p sature le bus mémoire interne et provoque des stalls d'exécution sur les Execution Units (EU).
2. **Surcoût Arithmétique Non Négligeable :**
   - Les 5 calculs de luminance vectorielle (`dot(color, vec3(...))`) et les 5 divisions par fragment ajoutent une latence de calcul supérieure au coût de rastérisation économisé.
3. **Le Quarter-Res Sans Karis est Déjà Optimal :**
   - Le passage pur à `bloom_downsample = 4` avec le kernel 5-tap Dual Kawase existant apporte à lui seul **+27.54 FPS (+5.91 %)**. L'ajout du filtre Karis neutralise ce gain.

---

## 5. Décision & Actions

1. **Rejet de la Piste Karis 13-Tap :** Annulation complète des modifications (`git restore`).
2. **Préservation du Status Quo :** Le moteur conserve son implémentation canonique Dual Kawase 5-tap.
3. **Règle pour le Futur :** Ne pas réintroduire d'échantillonneur large multipoint (> 5-tap) dans les passes plein écran sur iGPU Intel UMA.
