# 📊 Rapport d'Autopsie : Point Rendering — Discard vs Branchless (Point 1)

**Date :** 10 octobre 2026  
**Branche :** `feat/volumetric-smoke-lighting`  
**Auteur :** AI Assistant & Human Architect  
**Plateforme Matérielle :** Mesa Intel(R) Iris(R) Xe Graphics (RPL-U) — OpenGL 4.6 Core Profile  
**Statut de la Piste :** ❌ **REJETÉE (Régression de performance de -3,20 %)**  

---

## 1. Contexte & Hypothèse Initiale

Dans le shader de rendu des étincelles et traînées ([`assets/shaders/point_rendering.frag.glsl`](file:///home/latty/Prog/__PERSO__/rust-firework/assets/shaders/point_rendering.frag.glsl)) :
```glsl
vec2 uv = gl_PointCoord - vec2(0.5);
float dist = dot(uv, uv);
if(dist > 0.25) discard;    
float falloff = smoothstep(0.25, 0.0, dist);
```

### Hypothèse testée :
Le `discard` est généralement déconseillé sur les architectures GPU car il casse l'Early-Z et force la sérialisation des warps SIMD. En le supprimant, et étant donné que le mode de mélange est additif pur (`gl::BlendFunc(gl::SRC_ALPHA, gl::ONE)`), un fragment en dehors du disque circulaire (`dist > 0.25`) produit `alpha * falloff = 0.0`, ce qui équivaut mathématiquement à ajouter 0 au framebuffer. Le passage en shader branchless devait permettre une vectorisation parfaite et un gain d'environ 30 à 50 µs.

---

## 2. Résultats Comparatifs A/B Matériels (Intel Iris Xe)

Mesures sur 3 runs x 5 secondes consécutifs (Seed déterministe 42, VSync désactivée) :

| Configuration | FPS Moyen | Frame Time | Delta FPS | Gain % | Diagnostic |
| :--- | :---: | :---: | :---: | :---: | :--- |
| **Baseline (Avec `discard` branch)** | **458.06 FPS** | **2.183 ms** | Ref | Ref | Référence commit `2748fe7` |
| **Target (Branchless sans `discard`)** | **443.40 FPS** | **2.255 ms** | **-14.66 FPS** | **-3.20 %** 🔴 | **Régression nette (+72 µs/frame)** |

---

## 3. Analyse Technique du Post-Mortem : Pourquoi `discard` est Gagnant

1. **Géométrie des Point Sprites :**
   - Chaque particule `gl::POINTS` est rastérisée comme un carré plein de côté `gl_PointSize`.
   - L'aire d'un disque de rayon $0.5$ inscrit dans un carré de côté $1.0$ représente :
     $$\frac{\pi \cdot r^2}{1.0} = \frac{\pi \cdot 0.25}{1.0} \approx 78,54\ \% \text{ de la surface.}$$
   - Les 4 coins extérieurs au cercle représentent **21,46 % de l'ensemble des fragments générés**.
2. **Épargne de Bande Passante ROP & Blending :**
   - Avec `discard` : Pour 21,46 % de tous les fragments des 100 000 particules, le pipeline GPU s'interrompt immédiatement avant le Render Output Unit (ROP). Aucun cycle de lecture-modification-écriture (read-modify-write) ni mélange additif n'est envoyé aux buffers FBO `FragColor` et `BrightColor`.
   - Sans `discard` : Bien que la valeur ajoutée soit nulle (`0.0`), le matériel ROP doit exécuter un cycle d'écriture mémoire complet pour chacun de ces fragments sur les deux attachements HDR.
   - Sur l'Intel Iris Xe (mémoire système DDR5 partagée), l'augmentation de 21,5 % des transactions d'écriture mémoire submerge le gain théorique de l'élimination de branche.

---

## 4. Décision & Directives

1. **Rollback Immédiat :** Conservation intégrale de `if(dist > 0.25) discard;` dans `point_rendering.frag.glsl`.
2. **Règle pour le Rendu Point Sprite :** Le `discard` sur les coins extérieurs d'un point sprite est une optimisation de bande passante critique sur iGPU mémoire unifiée (UMA) et ne doit pas être supprimé.
