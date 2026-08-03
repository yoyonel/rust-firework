# GUI persistence inventory

The control panel has three persistence targets.  The target is part of the
contract: new persistent controls must update the source of truth below, be
restored on startup and through **Reload Session**, and be listed here.

| ID | UI control(s) | Canonical persistence target | Restore path |
| --- | --- | --- | --- |
| `gui.layout` | Panel visibility, selected tab, search, position, size, scroll, show geometry trimming, smoke preview max zoom, rocket color, simulated speed, simulated angle offset | `GuiSessionState` / `gui_session.toml` | `GuiSettings::new` and Reload Session |
| `gui.fullscreen` | Fullscreen toggle (F11) | `GuiSessionState.fullscreen` / `gui_session.toml` | `Simulator::new` startup restoration |
| `gui.scale` | Global UI Zoom / Font Scale Slider | `GuiSessionState.gui_scale` / `gui_session.toml` | `Simulator::render_ui` frame scaling |
| `gui.theme` | Theme selector | `GuiSessionState` / `gui_session.toml` | `GuiSettings::new` and pending theme application |
| `audio.output` | Master volume, mute, reverb wet mix, DSP matrix | `GuiSessionState` / `gui_session.toml` | `apply_session_to_audio` |
| `audio.diagnostics` | Diagnostic monitor and visual overlay | `GuiSessionState` / `gui_session.toml` | `apply_session_to_audio` |
| `physics.config` | Capacity, spawn, forces, smoke emission (rate, size, growth, fade, max particles, intensity, color mode, custom color, inherited color intensity), alpha erosion (enabled, scale, edge width, edge color), flow map UV distortion (strength, speed) | `PhysicConfig` / `physic.toml` | simulator startup and Reload Session |
| `physics.explosion_shape` | Spherical/image/multi-image selection, active image parameters and weights | `GuiSessionState.explosion_shape` / `gui_session.toml` | `apply_session_to_physic` |
| `physics.preset_weights` | Preset weight controls used before adding a shape | `GuiSessionState.preset_weights` / `gui_session.toml` | `GuiSettings::new` and Reload Session |
| `renderer.config` | Tone mapping, all Bloom controls, and graphical elements visibility toggles (rockets, smoke, trails, explosions) | `RendererConfig` / `renderer.toml` | simulator startup and Reload Session |
| `renderer.comparison` | Tone-mapping grid comparison | `GuiSessionState` / `gui_session.toml` | `Simulator::new` and Reload Session |

Buttons that perform an immediate action rather than retain a setting (save,
reload, reset, shader reload, start/stop stress scene) have no persisted value.
Their resulting settings are covered by the rows above.

## Session Management

The Zero-Allocation UI architecture decouples state persistence into domain-specific engine commands:

* **GUI Session**: `GuiCommand::SaveSession` saves `GuiSessionState` to `assets/config/gui_session.toml` (restored via `GuiCommand::ReloadSession`).
* **Physics Configuration**: `physic.config.save` (`PhysicCommand::SaveConfig`) saves `PhysicConfig` to `assets/config/physic.toml` (restored via `physic.config.reload` / `PhysicCommand::ReloadConfig`).
* **Renderer Configuration**: `renderer.config.save` (`RendererCommand::SaveConfig`) saves `RendererConfig` to `assets/config/renderer.toml` (restored via `renderer.config.reload` / `RendererCommand::ReloadConfig`).

Pressing **Save Session** dispatches `GuiCommand::SaveSession`, `physic.config.save`, and `renderer.config.save`. Pressing **Reload Session** dispatches `GuiCommand::ReloadSession`, `physic.config.reload`, and `renderer.config.reload`.

## Exit contract

`Simulator::save_gui_session`, called for both normal shutdown paths, writes
the GUI session plus `physic.toml` and `renderer.toml`.  Therefore Bloom is
saved on application exit even when the user never presses **Save Session**.

## Maintenance rule for AI/LLM contributors

Before adding or changing an ImGui setting:

1. Add a `GUI_PERSIST:` marker in `src/simulator/gui_settings.rs` using one
   inventory ID from this document, or add the ID and its full lifecycle here.
2. Update the canonical value, shutdown save, startup restoration, and Reload
   Session restoration together.
3. Add a round-trip test when the value is stored in `GuiSessionState`.
4. Run `task gui-persistence-check` before committing.

The marker/inventory consistency is enforced by the task and pre-commit hook.
