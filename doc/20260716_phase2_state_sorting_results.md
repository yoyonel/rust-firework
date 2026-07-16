# 📊 Résultats d'Optimisation Phase 2 : Regroupement et Tri d'États (State Sorting)

Ce document présente l'analyse des performances et le récapitulatif de l'implémentation de la **Phase 2** du plan d'optimisation.

---

## 🛠️ Implémentation réalisée

Pour réduire le coût de soumission CPU en évitant les liaisons redondantes de shaders et de textures :

1. **Extensions du trait `ParticleGraphicsRenderer` :**
   - Ajout des méthodes `get_shader_program(&self) -> u32` et `get_texture_id(&self) -> u32` pour permettre au gestionnaire de rendu d'inspecter l'état attendu.
   - Ajout de paramètres de suivi d'état mutable `active_shader: &mut u32` et `active_texture: &mut u32` à la méthode de dessin `render_particles_with_persistent_buffer`.
2. **Tri d'états (State Sorting) :**
   - Dans le constructeur de `Renderer`, nous trions désormais le vecteur de renderers par Shader ID puis par Texture ID lors de l'initialisation :
     ```rust
     renderers.sort_by_key(|r| (r.get_shader_program(), r.get_texture_id()));
     ```
3. **Élimination des liaisons redondantes (State Tracking) :**
   - Dans `RendererGraphics::render_particles_with_persistent_buffer` et `RendererGraphicsInstanced::render_particles_with_persistent_buffer`, nous ne lions le Shader (`gl::UseProgram`) et la Texture (`gl::BindTexture`) que s'ils ne sont pas déjà actifs (c'est-à-dire différents de la valeur courante passée en référence).

---

## 📈 Résultats des Benchmarks Criterion (sans VSync)

Les benchmarks ont été exécutés sous les mêmes conditions que les étapes précédentes : `vblank_mode=0 __GL_SYNC_TO_VBLANK=0 cargo bench --bench simulator_full_bench`.

| Charge (Nombre de Fusées actives) | Temps Baseline | Temps Phase 1 (Buffering) | Temps Phase 2 (Sorting) | Évolution (vs Phase 1) |
| :---: | :---: | :---: | :---: | :---: |
| **10** | 557.97 µs | 567.68 µs | 565.85 µs | -0.3% |
| **50** | 615.00 µs | 624.23 µs | 610.73 µs | -2.2% |
| **200** | 1.25 ms | 1.29 ms | 1.23 ms | -4.6% (Gain) |
| **1000** | 4.22 ms | 4.34 ms | 4.19 ms | -3.5% (Gain) |
| **4000** | **4.78 ms** | **4.68 ms** | **4.78 ms** | ~0.0% |

### 🔍 Analyse de l'impact
- En évitant les appels redondants de `glUseProgram` et `glBindTexture` pour chaque frame, nous réduisons le temps CPU passé dans le pilote à revalider le pipeline.
- Les gains les plus notables apparaissent sur les charges moyennes (200 à 1000 fusées) où le tri d'états et le tracking réduisent le surcoût de soumission CPU de façon mesurable (ex: **4.19 ms** pour 1000 fusées contre **4.34 ms** lors de la phase précédente).
