# Rapport d'Optimisation : Élimination du Bottleneck PNG Decoding (Zero-Cost)

## 1. Rationnel et Architecture
**Problème initial :** 
Lors de l'analyse des hotspots CPU via `vtune`, la fonction `png::decoder::Reader::next_frame` (via la crate `image`) accaparait une part disproportionnée du temps d'exécution (~10.5% du CPU time global, soit ~0.560s), constituant un goulot d'étranglement majeur lors du chargement ou spawn dynamique des entités (roquette, explosion).

**Solution retenue (Trade-off Espace vs Temps CPU) :**
- **Architecture Zero-Cost :** Remplacement de la décompression PNG CPU-bound par un chargement direct (I/O direct) de buffers binaires bruts (`.raw_tex`).
- **Pre-Processing :** Un script Python (`scripts/preprocess_textures.py`) a été mis à jour pour parcourir récursivement `assets/textures/` et transcrire chaque image PNG en fichier `.raw_tex` RGBA (incluant l'inversion verticale `FLIP_TOP_BOTTOM` requise par OpenGL). L'explosion de la taille du dossier `assets` (219 Mo) est un compromis assumé contre la libération du CPU.
- **Fast-Path & Fallback :** Le moteur graphique (`renderer_engine/utils/texture.rs`) et le moteur physique (`physic_engine/explosion_shape.rs`) interceptent le chargement pour prioriser la lecture brute `std::fs::read` en 0-cost, tout en préservant le fallback via `image::open` pour rétro-compatibilité.

## 2. Preuves comparatives A/B (VTune CPU Hotspots)

**Baseline (Avant) :**
```text
_RNvMs4...png7decoder...next_frame...fireworks_sim    0.560s
```

**Target (Après, Fast-path activé) :**
Disparition totale de `png::decoder` du Top 15 VTune.
- `func@0x332a0 libGLX_mesa.so.0`: 1.066s (18.9%)
- `<fireworks_sim::physic_engine::rocket::Rocket>::update`: 0.458s (8.1%) (Refactoring scalaire branchless intégré)

## 3. Runbook de Reproductibilité
- **Générer les raw_tex :** `python3 scripts/preprocess_textures.py` (ou `task assets:preprocess`).
- **Benchmarking VTune :** `task profile:vtune-hotspots`. Le rapport CLI synthétisera les 15 appels les plus lourds. L'absence de la stack `image` valide le Zero-Cost.
