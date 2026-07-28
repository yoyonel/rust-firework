---
name: firework-audio
description: Compétence spécialisée dans le moteur audio temps réel (CPal, DSP, traitement binaural, Doppler, effets audio, SafeWavWriter) du projet Rust Firework. À déclencher impérativement pour toute question, modification ou débuggage touchant aux sons, au DSP, au routage audio ou aux threads temps réel.
---

# Rust Firework - Moteur Audio & DSP (`src/audio_engine/`)

52 097 tokens de référence architecturale et technique sur le pipeline audio.

## ⚠️ Invariants et Règles d'Or Audio (Zéro Compromis)
1. **Zéro Allocation Temps Réel :** Interdiction absolue d'allouer de la mémoire sur le tas (`heap`, `Vec`, `Box`, `String`, `Rc`, `Arc`) dans le chemin critique DSP (`dsp_processor.rs`) ou à l'intérieur des callbacks CPal.
2. **Lock-Free Obligatoire :** Aucun verrou bloquant (`Mutex`, `RwLock` de `std::sync`) dans la boucle de rendu audio. La synchronisation et la communication inter-threads doivent utiliser exclusivement des ring buffers lock-free (triple buffering) ou des primitives atomiques.
3. **Vectorisation & SIMD :** Structurer les boucles de traitement audio et les calculs d'atténuation/binauraux pour favoriser l'autovectorisation du compilateur LLVM.

## Comment naviguer dans ce skill
- **Vue d'ensemble et architecture DSP :** Lire `references/summary.md` et les synthèses techniques associées (`doc/*audio*.md`, `doc/*doppler*.md`).
- **Code source exhaustif :** Chercher le fichier cible dans `references/files.md` via `## File: src/audio_engine/<fichier>.rs`.
- **Arborescence :** Vérifier `references/project-structure.md` pour le poids et la complexité des modules.
