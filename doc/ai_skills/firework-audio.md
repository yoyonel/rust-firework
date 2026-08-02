---
name: firework-audio
description: Compétence spécialisée dans le moteur audio temps réel (CPal, DSP, traitement binaural, Doppler, atténuation inverse-distance, vol de voix, diagnostics ImGui, SafeWavWriter) du projet Rust Firework. À déclencher impérativement pour toute question, modification ou débuggage touchant aux sons, au DSP, au routage audio ou aux threads temps réel.
---

# Rust Firework - Moteur Audio & DSP (`src/audio_engine/`)

Référence architecturale et technique sur le pipeline audio temps réel.

## ⚠️ Invariants et Règles d'Or Audio (Zéro Compromis)
1. **Zéro Allocation Temps Réel :** Interdiction absolue d'allouer de la mémoire sur le tas (`heap`, `Vec`, `Box`, `String`, `Rc`, `Arc`) dans le chemin critique DSP (`dsp_processor.rs`) ou à l'intérieur des callbacks CPal.
2. **Lock-Free Obligatoire :** Aucun verrou bloquant (`Mutex`, `RwLock` de `std::sync`) dans la boucle de rendu audio. La synchronisation et la communication inter-threads doivent utiliser exclusivement des ring buffers lock-free (canal borné 2048) ou des primitives atomiques (`AtomicVec2`).
3. **Atténuation & Vol de Voix Prioritaire :** Calcul de la distance via modèle Inverse-Distance Roll-off (volume max à 50px, fondu à 2000px) et vol de voix basé sur le gain pré-atténué spatialement et la priorité du type de son.
4. **Vectorisation & SIMD :** Structurer les boucles de traitement audio et les calculs d'atténuation/binauraux pour favoriser l'autovectorisation du compilateur LLVM.

## Comment naviguer dans ce skill
- **Vue d'ensemble et architecture DSP :** Lire `references/summary.md` et les synthèses techniques associées (`doc/*audio*.md`, `doc/audio_engine_diagnostic_and_attenuation.md`).
- **Code source exhaustif :** Chercher le fichier cible dans `references/files.md` via `## File: src/audio_engine/<fichier>.rs`.
- **Arborescence :** Vérifier `references/project-structure.md` pour le poids et la complexité des modules.
