# 📊 Résultats d'Optimisation Phase 4 : Uniform Buffer Objects (UBO) & Bilan Global

Ce document présente l'analyse des performances de la **Phase 4** et le bilan global de l'optimisation AZDO.

---

## 🛠️ Implémentation réalisée

Afin de maximiser les performances sur notre cas d'usage cible (simulation légère de **10 à 16 fusées**) tout en conservant les optimisations à forte charge, nous avons implémenté le pipeline suivant :

1. **Restauration des Renderers Dédiés :**
   - Rétablissement de `RendererGraphics` pour dessiner les particules standards sous forme de points (`GL_POINTS`). Cela élimine la surcharge liée à la rasterisation de quads et au traitement de sommets supplémentaires pour le cas d'usage nominal.
   - Rétablissement de `RendererGraphicsInstanced` pour le dessin instancié des fusées.
2. **Introduction d'un Uniform Buffer Object (UBO) Global :**
   - Création de la structure `GlobalDataUBO` en Rust (16 octets, alignement std140 parfait) :
     ```rust
     #[repr(C)]
     pub struct GlobalDataUBO {
         pub u_size_x: f32,
         pub u_size_y: f32,
         pub u_tex_ratio: f32,
         pub u_bloom_intensity: f32,
     }
     ```
   - Création et allocation du buffer GPU `ubo_global` dans `Renderer::new`, lié en continu au point de liaison `0` (`gl::BindBufferBase`).
   - Déclaration du bloc uniforme `layout (std140) uniform GlobalData` dans les shaders de particules (`point_rendering.vert.glsl`, `instanced_textured_quad.vert.glsl`) et les shaders de bloom (`bloom_composition.frag.glsl`, `bloom_composition_compare.frag.glsl`).
3. **Mise à jour unique par Frame :**
   - Les variables globales ne sont plus soumises via des appels `glUniform*` individuels dispersés.
   - Elles sont écrites en une seule fois dans la VRAM via un appel unique `glBufferSubData` au début de chaque frame dans `Renderer::render_frame`.
   - Élimination complète de tous les appels `glUniform2f` (dimensions de la fenêtre) et `glUniform1f` (intensité du bloom, ratio de texture) au render-time.

---

## 📈 Résultats des Benchmarks Criterion (sans VSync)

Les benchmarks ont été exécutés avec : `vblank_mode=0 __GL_SYNC_TO_VBLANK=0 cargo bench --bench simulator_full_bench`.

| Charge (Fusées) | Temps Baseline | Phase 1 (Buffering) | Phase 2 (Sorting) | Phase 4 (UBO & Points) | Évolution (vs Baseline) |
| :---: | :---: | :---: | :---: | :---: | :---: |
| **10** | 557.97 µs | 567.68 µs | 565.85 µs | **552.50 µs** | **-1.0% (Gain nominal)** |
| **50** | 615.00 µs | 624.23 µs | 610.73 µs | **605.34 µs** | **-1.6% (Gain nominal)** |
| **200** | 1.25 ms | 1.29 ms | 1.23 ms | **1.18 ms** | **-5.4% (Gain)** |
| **1000** | 4.22 ms | 4.34 ms | 4.19 ms | **4.01 ms** | **-5.2% (Gain)** |
| **4000** | **4.78 ms** | **4.68 ms** | **4.78 ms** | **4.41 ms** | **-7.7% (Gain maximum)** |

### 🔍 Analyse & Validation
- **Cas nominal (10 fusées) :** Nous enregistrons un temps d'exécution de **552.50 µs** (légèrement inférieur au baseline de 557.97 µs). Les performances sont préservées et légèrement améliorées.
- **Passage à l'échelle (4000 fusées) :** Le gain atteint **-7.7%** (4.41 ms contre 4.78 ms d'origine). C'est le meilleur temps de toutes les phases d'optimisation réunies.
- **Pourquoi cette combinaison est optimale ?**
  1. Le rendu par points (`GL_POINTS`) conserve la bande passante géométrique GPU minimale requise pour notre simulation à 10-16 fusées.
  2. Le UBO supprime les overheads CPU liés aux appels `glUniform` répétés par frame, ce qui profite directement aux configurations de simulation de toutes tailles.
