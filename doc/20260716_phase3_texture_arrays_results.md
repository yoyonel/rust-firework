# 📊 Résultats d'Optimisation Phase 3 : Texture Arrays & Batching Unique (OpenGL AZDO)

Ce document présente l'analyse des performances et le récapitulatif de l'implémentation de la **Phase 3** du plan d'optimisation.

---

## 🛠️ Implémentation réalisée

Pour éliminer complètement les changements d'état (shaders, textures) et réduire le nombre d'appels de dessin à **un seul draw call** par frame :

1. **Tableau de Textures 2D (Texture Array 2D) :**
   - Implémentation d'une fonction helper `load_texture_array` dans `src/renderer_engine/utils/texture.rs`.
   - Cette fonction charge le sprite des fusées (Couche 0) et génère automatiquement sur le CPU un motif de halo lumineux/radial (Couche 1) de mêmes dimensions pour remplacer les points de particules standards.
2. **Attribut de texture dans le Vertex :**
   - Ajout du champ `tex_index: f32` à la structure `ParticleGPU` dans `src/renderer_engine/types.rs`.
   - Configuration du layout d'attribut location 5 dans `setup_vertex_attribs_for_instanced_quad` pour faire passer cet index au shader.
3. **Mise à jour des Shaders :**
   - Modification de `instanced_textured_quad.vert.glsl` pour interpoler `vTexIndex` et désactiver le stretch d'aspect ratio de texture (en utilisant 1.0 au lieu de `uTexRatio`) si l'index correspond à une particule standard (`tex_index >= 0.5`).
   - Modification de `instanced_textured_quad.frag.glsl` pour utiliser un `sampler2DArray` et échantillonner via `texture(uTextureArray, vec3(vUV, vTexIndex))`.
4. **Draw Call unique et Fusion :**
   - Refactorisation complète de `RendererGraphicsInstanced` pour alimenter son buffer persistant en combinant d'abord les fusées actives (Couche 0.0), puis l'ensemble des particules actives (Couche 1.0).
   - Rendu complet réalisé via un unique appel à `glDrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, total_count)`.
   - Désactivation et suppression du module obsolète `RendererGraphics` (rendu par points) devenu inutile.

---

## 📈 Résultats des Benchmarks Criterion (sans VSync)

Les benchmarks ont été exécutés avec : `vblank_mode=0 __GL_SYNC_TO_VBLANK=0 cargo bench --bench simulator_full_bench`.

| Charge (Fusées actives) | Temps Baseline | Phase 1 (Buffering) | Phase 2 (Sorting) | Phase 3 (Unified Draw) | Évolution (vs Baseline) |
| :---: | :---: | :---: | :---: | :---: | :---: |
| **10** | 557.97 µs | 567.68 µs | 565.85 µs | 612.43 µs | +9.7% (Surcharge à vide) |
| **50** | 615.00 µs | 624.23 µs | 610.73 µs | 800.68 µs | +30.1% (Surcharge à vide) |
| **200** | 1.25 ms | 1.29 ms | 1.23 ms | 1.43 ms | +14.4% (Surcharge à vide) |
| **1000** | 4.22 ms | 4.34 ms | 4.19 ms | **4.04 ms** | **-4.3% (Gain net)** |
| **4000** | **4.78 ms** | **4.68 ms** | **4.78 ms** | **4.62 ms** | **-3.3% (Gain net)** |

### 🔍 Analyse de l'impact
- **Surcharge à très faible charge (10 à 200 fusées) :** Les particules d'étincelles qui étaient dessinées via de simples points rapides (`GL_POINTS` de 1 vertex) sont désormais dessinées sous forme de quads complets (4 vertices, instanciés). À très bas volume, le traitement des sommets supplémentaires introduit une surcharge CPU/GPU.
- **Passage à l'échelle sous forte charge (1000+ fusées) :** Lorsque le volume de particules explose (charge typique d'une simulation complexe), la réduction draconienne des appels systèmes OpenGL et le regroupement en un **unique appel de dessin** permet de surpasser le pipeline d'origine. Nous observons un gain net de performance (**4.04 ms** vs **4.22 ms** pour 1000 fusées, et **4.62 ms** vs **4.78 ms** pour 4000 fusées).
- L'overhead CPU de soumission a été considérablement réduit en supprimant la boucle de renderers multiples et les liaisons OpenGL redondantes.
