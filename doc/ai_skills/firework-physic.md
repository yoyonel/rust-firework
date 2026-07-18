---
name: firework-physic
description: Compétence spécialisée dans le moteur physique et la simulation de particules de Rust Firework (Generational Arena, Static AoS, gestion des explosions, roquettes, fumée). À déclencher pour tout travail sur la physique, la cinématique, les pools de particules et l'architecture mémoire du simulateur.
---

# Rust Firework - Moteur Physique & Simulation (`src/physic_engine/`, `src/simulator.rs`)

41 306 tokens de référence sur les algorithmes de simulation et la gestion mémoire des entités.

## ⚠️ Invariants et Règles d'Or Physique (Zéro Compromis)
1. **Data-Oriented Design (DoD) & Localité de Cache :** Stockage des particules et entités physiques structuré en Static AoS (Array of Structures) ou SoA selon les contextes critiques pour maximiser les hits dans les caches L1/L2 du CPU.
2. **Zéro Fragmentation (Generational Arena) :** Les cycles de vie des particules (spawn d'explosions, traînées de fumée) sont gérés sans allocations dynamiques continues grâce aux pools pré-alloués et aux index générationnels (Generational Arena).
3. **Déterminisme et Découplage :** Le pas de temps de la physique (`step`) doit être rigoureusement isolé du framerate de rendu.

## Comment naviguer dans ce skill
- **Gestion mémoire et algorithmes :** Lire `references/summary.md` et les documents de conception dans `doc/*physic*.md` et `doc/physic_memory_management.md`.
- **Code source de simulation :** Chercher directement dans `references/files.md` via `## File: src/physic_engine/...` ou `## File: src/simulator.rs`.
