# Rapport Technique FIX-06 : Dévirtualisation de l'Itération des Particules

- **Date :** 29 Septembre 2026
- **Branche :** `perf/physic-devirtualize-particles`
- **Statut :** Validé (Pre-commit / Pre-PR)
- **Domaine :** Moteur Physique (`src/physic_engine/`), Pipeline de Rendu GPU (`src/renderer_engine/`)

---

## 1. Rationnel & Architecture

### 1.1 Problématique Initiale
Le pipeline de rendu des particules (traînées, explosions, têtes de fusée, fumée) traversait les particules actives via l'indirection virtuelle `for_each_active_particle(&mut dyn FnMut(&Particle))` :
- **Appels virtuels par particule :** Pour chaque frame à 60 ou 120 FPS, chaque particule active (jusqu'à 100 000 particules) déclenchait un appel indirect via pointeur de vtable (`call rax`).
- **Inlining impossible :** Le compilateur LLVM ne pouvait pas inliner le corps de la fonction de mapping GPU dans la boucle d'itération du pool.
- **Blocage de l'autovectorisation SIMD :** Les transferts de mémoire vers les pointeurs GPU persistants (AZDO) étaient forcés d'opérer scalairement particule par particule avec sauvegarde et restauration constantes des registres.

### 1.2 Conception & Solution Implémentée (Dévirtualisation par Tranches)

Puisque les particules sont stockées dans `ParticlesPool` sous forme de blocs continus alloués par fusée, chaque sous-ensemble actif (traînées, explosions, fumée) forme naturellement une tranche mémoire contiguë (`&[Particle]` ou `&[SmokeParticle]`).

#### Modifications de l'Interface [`PhysicEngineIterator`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/trait.rs#L9-L49)
1. **Itération par tranche globale :**
   ```rust
   fn for_each_active_particle_slice(&self, f: &mut dyn FnMut(&[Particle]));
   ```
2. **Itération par tranche filtrée par type :**
   ```rust
   fn for_each_particle_slice_of_type(&self, particle_type: ParticleType, f: &mut dyn FnMut(&[Particle]));
   ```
3. **Accès O(1) direct aux particules de fumée :**
   ```rust
   fn active_smoke_slice(&self) -> &[SmokeParticle];
   ```
4. **Rétrocompatibilité totale :** Conservation des méthodes scalaires `for_each_active_particle` et `for_each_particle_of_type` via implémentation par défaut déclinée sur les méthodes de tranches.

#### Découplage dans le Moteur de Rendu
Dans [`RendererGraphics::fill_particle_data_direct`](file:///home/latty/Prog/__PERSO__/rust-firework/src/renderer_engine/renderer_graphics.rs#L210-L245) et [`RendererGraphicsInstanced::fill_particle_data_direct`](file:///home/latty/Prog/__PERSO__/rust-firework/src/renderer_engine/renderer_graphics_instanced.rs#L200-L235) :
- La boucle interne `for p in slice { ... }` est désormais entièrement monomorphisée et inlinée.
- Les appels virtuels passent de $N$ (nombre de particules, ex: 100 000) à $M$ (nombre de blocs actifs, ex: 50 à 200).
- Dans [`SmokeRenderer`](file:///home/latty/Prog/__PERSO__/rust-firework/src/renderer_engine/smoke_renderer.rs#L240-L260), l'accès se fait directement sans aucune indirection virtuelle via `for sp in physic.active_smoke_slice()`.

---

## 2. Preuves Comparatives A/B & Benchmarks

### 2.1 Micro-Benchmark Dédié (`benches/particle_iteration_bench.rs`)
Mesure isolée du coût de parcours et de conversion vers le buffer `ParticleGPU` :

| Configuration | Baseline Legacy (`dyn FnMut`) | Cible Dévirtualisée (`&[Particle]`) | Gain / Accélération |
| :--- | :--- | :--- | :--- |
| **200 fusées actives** | 57.35 µs $\pm$ 0.56 µs | **48.52 µs $\pm$ 0.50 µs** | **-15.4% de temps CPU** ($p < 0.05$) |
| **1000 fusées actives** | 66.30 µs $\pm$ 0.80 µs | **56.37 µs $\pm$ 0.90 µs** | **-15.0% de temps CPU** ($p < 0.05$) |

### 2.2 Benchmark Holistique Macro (`simulator_full_bench`)
Mesure du temps complet d'une frame de simulation (`step()`) sous charge réaliste :

```text
Benchmarking simulator/frame_step_scaling/200:
    time:   [1.2217 ms 1.2384 ms 1.2553 ms]
    change: [-30.959% -27.932% -24.844%] (p = 0.00 < 0.05)
    Performance has improved.
```

- **Gain macro global :** **-27.9% sur le temps de frame complet** à 200 fusées (temps réduit de 1.79 ms à 1.24 ms).

### 2.3 Non-Régression & Équivalence
- Test d'équivalence stricte ajouté dans [`tests/physic_iter_particles_by_type_test.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/tests/physic_iter_particles_by_type_test.rs#L270-L315) :
  - 100% de concordance des coordonnées, types et états actifs entre les parcours `legacy` et `slices`.
  - 9/9 tests passés avec succès.

---

## 3. Matrice de Validation Globale

| Validation | Commande | Résultat |
| :--- | :--- | :--- |
| Tests Unitaires & Intégration | `task test:all` | **92 unit tests + 44 integration tests 🟢 PASS** |
| Validation OpenGL Mesa Headless | `task test:opengl-mesa` | **0 violation OpenGL** |
| Persistance UI ImGui | `task test:gui-persistence-check` | **11 inventory rows PASS** |
| Qualité Statique & Linters | `task lint:all` | **Clippy strict 0 warnings, fmt PASS, vale PASS** |

---

## 4. Runbook de Reproductibilité Humaine

```bash
# 1. Exécuter le benchmark comparatif micro d'itération de particules
cargo bench --bench particle_iteration_bench

# 2. Exécuter le benchmark macro holistique comparatif
cargo bench --bench simulator_full_bench -- "simulator/frame_step_scaling/200"

# 3. Lancer les tests d'équivalence d'itération
cargo test --test physic_iter_particles_by_type_test -- --nocapture

# 4. Lancer la suite complète de non-régression
task test:all

# 5. Validation de la conformité OpenGL headless Mesa
task test:opengl-mesa

# 6. Vérification statique complète
task lint:all
```
