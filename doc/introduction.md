# 🎆 Fireworks Simulator Documentation

Bienvenue dans la documentation technique du **Simulateur de Feux d'Artifices** (`fireworks_sim`). Ce projet est une application haute performance écrite en **Rust** combinant un moteur physique 2D, un rendu graphique OpenGL moderne, et un moteur audio 3D binaural en temps réel.

---

## 🎯 Vue d'ensemble du Projet

Le simulateur est conçu avec une architecture modulaire et performante :
1. **Moteur Physique (`src/physic_engine/`) :** Utilise une arène générationnelle (`generational-arena`) et des pools de particules pré-alloués pour simuler de manière performante le déplacement des fusées et l'expansion des explosions de particules.
2. **Moteur Graphique (`src/renderer_engine/`) :** Rendu 2D instancié et direct avec OpenGL 3.3. Utilise des buffers GPU persistants (AZDO) pour optimiser les téléversements CPU-GPU et intègre un traitement post-process de Bloom.
3. **Moteur Audio (`src/audio_engine/`) :** Synthèse et traitement audio 3D temps réel basé sur `cpal`. Intègre la spatialisation binaurale (HRTF) et le calcul dynamique de l'effet Doppler.
4. **Console Interactive (`src/utils/command_console/`) :** Permet de modifier les paramètres physiques et audio en temps réel à l'aide de commandes console.

---

## 📖 Structure de la Documentation

La documentation est organisée en plusieurs catégories clés :

* [Guide des Tâches (Taskfile)](taskfile_guide.md) : Commandes d'automatisation pour le développement, la compilation, les tests et le profilage.
* **Moteur Physique :**
  * [Gestion de la mémoire physique](physic_memory_management.md) : Modèle de données et structures pré-allouées.
  * [Formes d'explosions](physic_explosion_shapes.md) : Paramétrage géométrique et par image des explosions.
* **Moteur Audio :**
  * [Rapport de Refactoring Audio](20260713_audio_engine_refactoring_report.md) : Détails techniques du passage en temps réel asynchrone.
  * [Spécification technique Doppler](20260711_doppler_audio_technical_spec.md) : Physique et calcul des effets Doppler.
  * [Benchmarks Audio](benchmarks_audio.md) : Mesures de performance des filtres DSP et de la binauralisation.
* **Rendu Graphique :**
  * [Persistent Mapped Buffers (AZDO)](opengl_azdo_persistent_mapped_buffers.md) : Technologie d'optimisation GPU du Renderer.
  * [Manuel du Renderer](renderer_manual.md) : Guide d'utilisation et shaders.
* **Profilage & Performance (Heaptrack & Tracy) :**
  * [Guide complet Heaptrack](20260714_heaptrack_memory_profiling_guide.md) : Tutoriel de profilage et suppression des allocations.
  * [Plan d'optimisation GPU et stalls](20260715_gpu_stalls_analysis_and_optimization_plan.md) : Analyse des temps d'attente OpenGL et plan d'action.
  * [Audit mémoire Tracy](20260715_tracy_memory_audit.md) : Vérification fine du thread audio en Tracy.
