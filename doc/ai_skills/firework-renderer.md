---
name: firework-renderer
description: Compétence spécialisée dans le moteur de rendu graphique OpenGL AZDO (Approaching Zero Driver Overhead), les Shaders GLSL, l'instancing, le post-processing Bloom, ImGui et le profiling GPU (Tracy/RenderDoc) pour Rust Firework. À déclencher pour toute tâche graphique, shader, caméra, UI ImGui ou optimisation GPU.
---

# Rust Firework - Moteur de Rendu Graphic & AZDO (`src/renderer_engine/`)

52 075 tokens de référence sur le pipeline de rendu hautes performances et les shaders.

## ⚠️ Invariants et Règles d'Or Graphiques (Zéro Compromis)
1. **Paradigme AZDO & Mappings Persistants :** Utilisation stricte de buffers mappés de manière persistante (`glMapBufferRange` avec Write-Combining). Zéro appel OpenGL synchronisant (pas de `glGetError` ou de lecture back-buffer) dans la boucle de rendu principale.
2. **Alignement Mémoire Rust -> GLSL :** Toutes les structures de données Rust transmises au GPU (UBO, SSBO, Instance Data) doivent implémenter un alignement strict `#[repr(C)]` compatible avec les règles `std140` ou `std430` de l'OpenGL.
3. **Découplage UI (ImGui) :** Les widgets et panneaux d'interface ImGui doivent refléter l'état de la simulation au frame courant de manière immédiate, sans jamais posséder ou stocker de références directes vers les ressources de bas niveau (handles GL bruts) en dehors du scope autorisé.

## Comment naviguer dans ce skill
- **Audit de performance et AZDO :** Consulter `references/summary.md` ainsi que les rapports techniques `doc/*azdo*.md` et `doc/*tracy*.md`.
- **Shaders et Renderer :** Les codes sources GLSL et Rust sont indexés dans `references/files.md` sous `## File: src/renderer_engine/...` et `## File: assets/shaders/...`.
