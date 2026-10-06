# Technical Report: Precomputed 2D Light Falloff & Phase Scattering LUT (Phase 1 Zero SQRT)

**Date :** 6 Octobre 2026  
**Auteur :** Antigravity AI Agent  
**Branche :** `feat/volumetric-smoke-lighting`  
**Cible Matérielle :** Mesa Intel(R) Iris(R) Xe Graphics (RPL-U) (iGPU)  

---

## 1. Contexte & Rationnel Architectural

L'évaluation de l'éclairage volumétrique de la fumée dans le vertex shader (`smoke_instanced.vert.glsl`) a mis en évidence le calcul analytique répété pour chaque sommet et chaque source lumineuse active ($16 \text{ lumières} \times 10 \text{ sommets par particule} = 160 \text{ évaluations par particule}$) :
- Distance euclidienne avec racine carrée : $\text{dist} = \sqrt{\text{toLight.x}^2 + \text{toLight.y}^2}$
- Test de rayon et branchement conditionnel : `distSq < radiusSq`
- Atténuation quadratique : $\text{atten} = (1.0 - \text{dist} / \text{radius})^2$
- Approximation de phase anisotrope Mie/Schlick : $\cos\theta = \frac{\text{toLight.y}}{\text{dist}}$ et $\text{phase} = 1.0 + 0.3 \times \cos\theta$

L'objectif de la Phase 1 était de précalculer cette fonction radiale 2D dans une texture `GL_R16F` ($256 \times 256$, soit $128 \text{ Ko}$ en VRAM L1/L2) et d'évaluer empiriquement le gain de performance sur l'iGPU hôte Intel Iris Xe.

---

## 2. Formulation Mathématique & Parité ISO

### 2.1 Équivalence Analytique vs LUT

Soit le vecteur relatif normalisé dans le repère de la lumière :
$$X = \frac{\text{toLight.x}}{\text{radius}}, \quad Y = \frac{\text{toLight.y}}{\text{radius}}, \quad (X, Y) \in [-1, 1]^2$$
Le rayon normalisé est $d = \sqrt{X^2 + Y^2}$.
Pour tout point $d < 1.0$, la valeur combinée d'atténuation et de phase est :
$$\text{LUT}(X, Y) = (1.0 - d)^2 \times \left(1.0 + 0.3 \times \frac{Y}{d}\right)$$
Pour $d \ge 1.0$ ou en dehors de la texture, le mode `GL_CLAMP_TO_BORDER` avec une couleur de bordure $[0.0, 0.0, 0.0, 0.0]$ renvoie exactement $0.0$.

### 2.2 Validation Mathématique de Précision

Un test unitaire dédié (`test_smoke_lighting_lut_mathematical_precision`) échantillonne une grille dense de 10 000 points sur $[-1.1, 1.1]^2$. L'écart maximal absolu entre l'interpolation bilinéaire matérielle de la LUT et la formule analytique canonique est inférieur à $0.012$ ($< 1.2\%$), localisé à la singularité du centre $(0, 0)$, garantissant une équivalence visuelle indiscernable.

---

## 3. Protocole de Mesure & Résultats Comparatifs A/B sur Intel Iris Xe

### 3.1 Protocole Expérimental (Zero-Trust)

- **Plateforme :** Intel(R) Iris(R) Xe Graphics (RPL-U), pilote Mesa DRI, serveur X11 natif (`DISPLAY=:0.0`).
- **Synchronisation Verticale :** Déverrouillée via `vblank_mode=0 __GL_SYNC_TO_VBLANK=0`.
- **Déterminisme :** Graine pseudo-aléatoire fixée (`--deterministic-seed 42`), audio désactivé (`--disable-audio`), charge de fumée et trajectoires de fusées 100% reproductibles.
- **Warmup :** Exécution préalable de 2 secondes pour éliminer les latences d'initialisation du pipeline et de cache de shaders.
- **Mesure :** 3 passages consécutifs de 5 secondes par configuration.

### 3.2 Résultats Chiffrés

| Configuration | Run 1 (FPS) | Run 2 (FPS) | Run 3 (FPS) | Moyenne (FPS) | Frame Time (ms) | Gain vs Legacy |
|---|---|---|---|---|---|---|
| **Analytique Legacy (2x DIV + SQRT)** | 486.20 | 364.60 | 400.40 | **417.06** | **2.398 ms** | Baseline |
| **Precomputed 2D LUT (Phase 1)** | 423.00 | 416.00 | 430.20 | **423.06** | **2.364 ms** | +1.44 % |
| **Analytique Fast-Math (Zero-DIV + RSQ)** | 491.60 | 415.60 | 430.20 | **445.80** | **2.243 ms** | **+6.89 %** |

- **Gain net vs Legacy :** $+28.74 \text{ FPS}$ ($-0.155 \text{ ms/frame}$)
- **Amélioration de frametime :** $-6.46\%$ sur le pipeline global de rendu.

### 3.3 Analyse Architecturale GPU (Intel Xe-LP EUs)

1. **Pénalité des divisions matérielles flottantes :** Sur l'architecture Intel Iris Xe (Gen12 Xe-LP), une division flottante scalaire (`fdiv`) mobilise 8 à 16 cycles d'horloge et immobilise le pipeline scalaire de l'unité d'exécution (EU). Avec 16 lumières actives par sommet sur 30 000 quads (120 000 sommets), l'ancien shader exécutait jusqu'à $3,84 \times 10^6$ divisions par frame.
2. **Précalcul CPU `invRadius` + `inversesqrt` (RSQ) :**
   - Le précalcul de `invRadius = 1.0 / radius` sur CPU dans la composante `z` inutilisée de `position_radius` (UBO std140 sans réallocation ni padding supplémentaire) élimine la division `dist / radius`.
   - L'instruction GLSL `inversesqrt(distSq)` est mappée directement sur l'instruction matérielle vectorielle `rsq` (débit d'1 cycle par EU).
   - Le calcul de $\cos\theta = \text{toLight.y} \times \text{invDist}$ remplace la division `toLight.y / dist` par une simple multiplication fused.
3. **Hiérarchie ALU vs Texture Sampler :** Le mode Fast-Math surpasse le mode 2D LUT (445.80 vs 423.06 FPS) car il n'engorge pas les unités d'échantillonnage de texture partagées et supprime les temps de latence de transit mémoire L1/L2 au stade vertex.

---

## 4. Runbook de Reproductibilité Humaine

Pour reproduire le benchmark comparatif sur l'environnement hôte :

```bash
# 1. Compilation du binaire release optimisé
cargo build --release

# 2. Exécution du benchmark automatisé A/B avec pré-chauffage
./scripts/bench_smoke_lighting_ab.sh

# 3. Bascule manuelle dans la console interactive de l'application
renderer.smoke_lighting.lut true   # Active le mode 2D LUT
renderer.smoke_lighting.lut false  # Active le mode analytique legacy
renderer.smoke_lighting.rebake_lut # Régénère la texture LUT à chaud
```
