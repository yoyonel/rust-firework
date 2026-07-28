---
name: firework-imgui
description: Compétence spécialisée dans la création d'interfaces GUI en mode immédiat via ImGui, la console de commande interactive (command_console), la gestion des fenêtres (GLFW) et les widgets de diagnostic/profiling pour le projet Rust Firework. À déclencher pour ajouter un panneau d'interface, modifier un widget de debug, étendre la console ou relier l'UI aux données de simulation.
---

# Rust Firework - Interface ImGui & Diagnostics (`src/window_engine/`, `src/utils/command_console/`)

Référence technique sur l'architecture d'interface en mode immédiat (ImGui) et l'outillage de diagnostic visuel.

## ⚠️ Invariants et Règles d'Or UI / ImGui (Zéro Compromis)
1. **Paradigme Immédiat & Découplage de l'État :** Ne jamais dupliquer l'état du domaine dans les structures de l'UI. L'interface est une pure projection de l'état de la simulation ou du rendu au frame \\( t \\). Si un widget doit modifier une valeur, il émet une commande ou modifie directement la configuration source autorisée.
2. **Isolation des Handles GL :** Les widgets ImGui ne doivent jamais manipuler de primitives OpenGL brutes ou de buffers bas niveau de manière ad-hoc. L'affichage de textures dans ImGui (ex: preview d'un FBO de Bloom ou de particules) doit impérativement passer par les abstractions existantes du renderer.
3. **Ergonomie & Zéro-Allocation dans l'UI Loop :** Éviter les allocations répétées de `String` à chaque frame pour le formattage des textes ImGui (utiliser des buffers réutilisables ou des formattages in-place autant que possible dans les boucles de profiling à haute fréquence).

## Comment naviguer dans ce skill
- **Console et Profiler :** Consulter la documentation `doc/console_commands.md` et `doc/*profiling*` pour voir comment intégrer un nouvel outil de monitoring.
- **Code de l'interface :** Les implémentations se trouvent dans `references/files.md` sous `## File: src/utils/command_console/`, `src/window_engine/...` et `src/profiler.rs`.
