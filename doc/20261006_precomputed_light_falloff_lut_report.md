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

| Configuration | Run 1 (FPS) | Run 2 (FPS) | Run 3 (FPS) | Moyenne (FPS) | Frame Time (ms) |
|---|---|---|---|---|---|
| **Analytique (Legacy SQRT)** | 486.20 | 364.60 | 400.40 | **417.06** | **2.398 ms** |
| **Precomputed 2D LUT (Phase 1)** | 407.60 | 417.00 | 398.40 | **407.66** | **2.453 ms** |

- **Différence brute :** $-9.40 \text{ FPS}$ ($-0.055 \text{ ms/frame}$)
- **Écart relatif :** $-2.25\%$ (dans la marge de variabilité de l'iGPU)

### 3.3 Analyse Architecturale GPU (Pourquoi pas de gain massif ?)

1. **Débit ALU des EUs Intel Iris Xe :** Les unités d'exécution (EU) de l'architecture Xe-LP disposent d'ALUs superscalaires capables d'exécuter l'instruction `rsqrt` et les opérations arithmétiques vectorielles à un coût marginal par rapport aux transferts mémoire.
2. **Latence du Texture Sampler au stade Vertex :** L'échantillonnage de texture au niveau du Vertex Shader (Vertex Texture Fetch - VTF via `textureLod`) sollicite le cache de texture et les unités de filtrage partagées avec le Fragment Shader (qui échantillonne déjà le sprite de fumée, la flow map et le masque de bruit).
3. **Stabilité :** La variante LUT affiche une variance inter-runs remarquablement faible ($398$ à $417 \text{ FPS}$) par rapport au mode analytique ($364$ à $486 \text{ FPS}$).

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
