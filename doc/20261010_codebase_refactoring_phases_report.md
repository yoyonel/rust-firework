# 🏗️ Rapport Technique : Refactoring Architectural Global & Sûreté Temps Réel (Phases 0 à 3)

**Date :** 10 Octobre 2026  
**Branche :** `feat/volumetric-smoke-lighting`  
**Statut :** Validé (306/306 tests passés, 0 régression, CI 100% verte)  
**Auteur :** Antigravity & Équipe Moteur  

---

## 1. Rationnel & Vision Architecturale

Ce chantier de refactoring profond a pour objectif de consolider la robustesse, la modularité et les performances du simulateur de feux d'artifice à travers 4 phases d'ingénierie strictes (Zero-Trust, Zero-Allocation temps réel, Séparation des Responsabilités SoC et Inversion de Contrôle IoC).

### 1.1. Phase 0 : Scellement des Layouts & Vérifications Résiduelles
- **Layout des Particules :** Pinning explicite de la structure mémoire `Particle` et de son sous-ensemble GPU via des assertions statiques au compile-time (`static_assertions`).
- **Alignement & Offsets :** Garantie formelle d'alignement mémoire pour les transferts vers les buffers persistants OpenGL (AZDO) sans padding inattendu.

### 1.2. Phase 1 : Sûreté Temps Réel (CPAL & Audio Thread Lock-Free)
- **F-01 (Purger le Profiler du chemin audio) :** Remplacement de l'objet `Profiler` (allouant dynamiquement et verrouillant) par `AudioRealtimeStats` (atomiques lock-free `AtomicU64` et instant vDSO).
- **F-02 (Zéro allocation dans `export_wav`) :** Réutilisation de buffer persistant via `std::mem::take` et recyclage vectoriel, éliminant toute réallocation dynamique sur le tas pendant l'export.
- **F-03 (Purger les logs du callback audio) :** Élimination de toutes les macros de formatage (`println!`, `eprintln!`, `log::*`) dans la boucle DSP temps réel.
- **F-12 (Throttling des logs OpenGL Debug) :** Réparation du limiteur de logs de debug GL pour éviter la saturation I/O sous Mesa debug.
- **F-14 (Purge du moteur AOS statique orphelin) :** Élimination du legacy code `static_aos` orphelin.

### 1.3. Phase 2 : Mémoire & RAII
- **F-05 (Isolation `ParticleVertexCore`) :** Séparation du noyau de vertex pur (`ParticleVertexCore` : position, couleur, vie, taille, angle — 36 octets) des métadonnées de simulation (`Particle` complète).
- **F-04 (Élimination de `static mut PREVIEW_GPU`) :** Suppression intégrale du pointeur statique mutable global non-thread-safe au profit d'un cycle de vie encapsulé dans `SmokePreviewRenderer`.
- **F-09 (Wrappers RAII OpenGL) :** Introduction des types RAII `GlBuffer`, `GlTexture`, `GlFbo`, `GlVao` gérant automatiquement `glDelete*` à la destruction sans fuite GPU ni double-free.

### 1.4. Phase 3 : Architecture, Dégraissage & Inversion de Contrôle (IoC)
- **F-10 (Extraction CLI & Stress Runner) :** Extraction du parsing des arguments CLI dans `src/cli.rs` (orchestration `main.rs` réduite à moins de 30 lignes) et déport du runner de stress headless vers `src/simulator/audio_stress_scene.rs`.
- **F-06 (Découplage `WindowEngine`) :** Abstraction de la gestion de fenêtre via `WindowEngine` trait, découplant complètement `glfw` du renderer et du simulateur.
- **F-08 (Dégraissage de `Simulator` de 63 à 32 champs) :** Extraction comportementale de la télémétrie (`SyncTelemetryTracker`), des diagnostics audio (`AudioDiagnosticOverlay`), des métriques FPS (`FpsMetrics`) et du debug visuel (`AudioVisualDebugOverlay`).
- **domain_contracts & IoC :** `src/domain_contracts.rs` devient un module racine pur sans dépendance amont vers les moteurs (`crate::audio_engine`, `crate::physic_engine`, `crate::renderer_engine`). Migration des enums purs (`AudioEffect`, `SmokeColorMode`, `BlurMethod`, `ToneMappingMode`, `DopplerEvent`, `ExplosionShape`, `AudioSoundType`, `AudioDebugEvent`) et des snapshots sans cache (`PhysicConfigSnapshot`, `RendererConfigSnapshot`, `SmokeConfigSnapshot`).

---

## 2. Preuves Comparatives & Benchmarking (A/B)

### 2.1. Benchmark F-05 (Itération des Particules)
Mesures comparatives A/B exécutées avec Criterion (`cargo bench --bench particle_iteration_bench`) :

| Cas d'Épreuve | Baseline Legacy (`d9e0822`) | Target F-05 (`2af7fff`) | Delta Moyen (%) | Intervalle de Confiance 95% | Statut Statistique |
| :--- | :---: | :---: | :---: | :---: | :--- |
| `devirtualized_slice/1000` | 2.15 µs | 2.30 µs | +7.25 % | `[-2.25 %, +16.93 %]` | Non significatif ($p = 0.15 > 0.05$) |
| `legacy_dyn_fnmut/1000` | 2.22 µs | 2.24 µs | +1.04 % | `[-3.35 %, +3.76 %]` | Non significatif ($p = 0.71 > 0.05$) |
| `devirtualized_slice/200` | 442 ns | 470 ns | +6.37 % | `[+5.69 %, +7.02 %]` | Écart d'agrégat safe vs cast unsafe |
| `legacy_dyn_fnmut/200` | 451 ns | 477 ns | +5.77 % | `[+4.88 %, +6.96 %]` | Écart d'agrégat safe vs cast unsafe |

*Rapprochement technique :* L'écart sur petits volumes provient du passage d'un cast brut non-sûr (`unsafe { *(p as *const ParticleGPU) }`) à une copie sûre de structure `core` avec instanciation d'agrégat. Sur grands volumes (1000+ particules), la différence est noyée dans le bruit statistique.

### 2.2. Évolution de la Couverture de Tests

| Étape / Commit | Description | Tests Passés | Tests Ignorés |
| :--- | :--- | :---: | :---: |
| **Baseline initiale** | État avant Phase 0 (`d9e0822`) | **287** | 1 (`effect_flags.rs`) |
| `2af7fff` | Phase 2 (F-05 layout ParticleVertexCore) | **290** (+3) | 1 |
| `4bdb706` | Phase 2 (RAII GlBuffer, GlTexture, GlFbo, GlVao) | **294** (+4) | 1 |
| `e33a39f` | Phase 3 (F-10 CLI extraction & stress scene) | **301** (+7) | 1 |
| `950d4dd` | Phase 3 (F-08 Extract Telemetry & Dégraissage) | **305** (+4) | 1 |
| `5ae14ae` | Phase 3 (domain_contracts IoC & Snapshots) | **306** (+1) | 1 |

---

## 3. Runbook de Reproductibilité Humaine

Commandes exactes permettant de relancer l'audit et la validation complète de l'architecture :

```bash
# 1. Vérification du formatage et des linters stricts (-D warnings)
task lint:all

# 2. Vérification de l'absence d'imports amont dans domain_contracts (IoC strict)
git grep "crate::audio_engine\|crate::physic_engine\|crate::renderer_engine" src/domain_contracts.rs
# Résultat attendu : 0 résultat (Exit code 1)

# 3. Exécution de l'intégralité des tests unitaires et d'intégration
task test:all

# 4. Validation des spécifications OpenGL headless Mesa (0 violation acceptée)
task test:opengl-mesa

# 5. Contrôle de la persistance de l'inventaire UI ImGui
task test:gui-persistence-check

# 6. Compilation du binaire release optimisé
cargo build --release
```
