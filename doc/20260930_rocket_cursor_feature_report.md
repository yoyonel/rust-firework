# Rapport Technique : Intégration du Curseur Fusée (Rocket Cursor)

**Date :** 30 Septembre 2026  
**Auteur :** Antigravity Agent & Lionel ATTY  
**Branche :** `feat/rocket-cursor`  
**Statut :** Validé (0 allocations en frame-loop, tests unitaires et headless Mesa validés)

---

## 1. Contexte & Rationnel

Le projet disposait historiquement d'une tentative de curseur personnalisé sous forme de fusée (`feature/rocket_cursor` fin 2025). Cette tentative présentait deux limitations architecturales majeures :
1. **Divergence architecturale** : Écrite avant le découplage `WindowEngine` / `Simulator`, elle brisait les signatures de test headless.
2. **Violation Zero-Allocation** : Le curseur GLFW était réalloué et reconstruit via `glfw::Cursor::create_from_pixels` à chaque frame pour contrer l'écrasement standard par `imgui-glfw-rs`.

L'objectif de cette implémentation moderne est de fournir un curseur fusée transparent 32x32 pixel-art réactif, avec :
- Une initialisation unique au lancement de l'application (Zero-Allocation).
- Un hotspot calibré à la pointe de l'ogive de la fusée ($x = 16, y = 1$).
- Le contrôle via ImGui (`NO_MOUSE_CURSOR_CHANGE`) évitant les surcharges de frame.
- Un toggle UI interactif dans l'onglet *Renderer & Post-FX* et une commande console `renderer.rocket_cursor [true|false]`.
- La persistance complète via `GuiSessionState` (`gui_session.toml`) et conformité Règle 5 (`GUI_PERSIST: gui.rocket_cursor`).

---

## 2. Architecture & Choix Techniques

### 2.1 Modules Single Source of Truth (SSOT)
- **Constantes canoniques** : Centralisées dans `src/window_engine/constants.rs` :
  - `ROCKET_CURSOR_TEXTURE_PATH = "assets/textures/rocket_cursor_32.png"`
  - `ROCKET_CURSOR_HOTSPOT_X = 16`, `ROCKET_CURSOR_HOTSPOT_Y = 1`
  - Dimensions : 32x32 pixels.
- **Conversion Little-Endian GLFW** : Implémentée dans `src/window_engine/cursor.rs` via `as_chunks::<4>()` et `u32::from_ne_bytes`, garantissant un layout mémoire conforme aux spécifications GLFW `[R, G, B, A]`.

### 2.2 Gestion RAII & Swap de Curseur
- `GlfwWindowEngine` charge et met en cache les pixels `cursor_pixels: Option<Vec<u32>>` (4 Ko) une seule fois à l'initialisation.
- Lors de l'activation (`set_rocket_cursor(true)`), un curseur GLFW est instancié à partir des pixels mis en cache et assigné à la fenêtre.
- Lors de la désactivation (`set_rocket_cursor(false)`), `window.set_cursor(None)` restaure immédiatement le curseur flèche standard du système d'exploitation.
- Zéro réallocation en boucle de rendu : les bascules ne se font que sur action utilisateur explicite (UI / console).

### 2.3 Découplage ImGui & CQRS
- Pour empêcher `imgui-glfw-rs` d'écraser le pointeur par `StandardCursor::Arrow` lors de son appel de rendu, le flag `imgui::ConfigFlags::NO_MOUSE_CURSOR_CHANGE` est dynamiquement synchronisé sur le contexte ImGui juste avant `sys.glfw.draw(&mut sys.context, win)`.
- En cas de désactivation du curseur fusée, ce flag est retiré, permettant à ImGui de piloter normalement les curseurs contextuels (ex: redimensionnement de fenêtres).
- L'émission d'ordre depuis l'UI utilise le pattern CQRS : `EngineCommand::Gui(GuiCommand::SetRocketCursor(bool))`.

---

## 3. Validation Empirique & Tests

### 3.1 Tests Unitaires & Intégration
- `tests/rocket_cursor_test.rs` valide :
  1. L'intégrité de la texture PNG (dimensions 32x32, pointe opaque à $x=16, y=1$, transparence aux coins).
  2. La conversion binaire vers `Vec<u32>`.
  3. La persistance round-trip dans `GuiSessionState` et la rétrocompatibilité (valeur par défaut `true`).
  4. L'implémentation mock pour `DummyWindowEngine`.

### 3.2 Protocole de Validation Locale (Fail-Fast Pyramid)
```bash
# 1. Vérification de l'inventaire de persistance GUI (Règle 5)
task test:gui-persistence-check

# 2. Linter exhaustif (Clippy strict, rustfmt, Vale doc)
task lint:all

# 3. Validation globale des tests unitaires et intégration
task test:all

# 4. Validation des spécifications OpenGL Mesa headless
task test:opengl-mesa
```
Résultats : **100% PASS** (92 tests unitaires, 12 entrées d'inventaire de persistance, 0 violation OpenGL Mesa).

---

## 4. Runbook de Reproductibilité

Pour tester et manipuler le curseur fusée :
1. Lancer le simulateur :
   ```bash
   cargo run --release
   ```
2. Observer le pointeur de la souris : une fusée transparente vole et pointe avec son ogive.
3. Basculer via la console in-game (touche `~` ou `F1`) :
   ```text
   renderer.rocket_cursor false
   renderer.rocket_cursor true
   ```
4. Basculer via le panneau ImGui (touche `F4`) :
   - Onglet *Renderer & Post-FX* $\rightarrow$ Section *MOUSE CURSOR* $\rightarrow$ Case à cocher *Custom Rocket Cursor*.
