# GUI persistence inventory

The control panel has three persistence targets.  The target is part of the
contract: new persistent controls must update the source of truth below, be
restored on startup and through **Reload Session**, and be listed here.

| ID | UI control(s) | Canonical persistence target | Restore path |
| --- | --- | --- | --- |
| `gui.layout` | Panel visibility, selected tab, search, position, size, scroll | `GuiSessionState` / `gui_session.toml` | `GuiSettings::new` and Reload Session |
| `gui.theme` | Theme selector | `GuiSessionState` / `gui_session.toml` | `GuiSettings::new` and pending theme application |
| `audio.output` | Master volume, mute, reverb wet mix, DSP matrix | `GuiSessionState` / `gui_session.toml` | `apply_session_to_audio` |
| `audio.diagnostics` | Diagnostic monitor and visual overlay | `GuiSessionState` / `gui_session.toml` | `apply_session_to_audio` |
| `physics.config` | Capacity, spawn, forces | `PhysicConfig` / `physic.toml` | simulator startup and Reload Session |
| `physics.explosion_shape` | Spherical/image/multi-image selection, active image parameters and weights | `GuiSessionState.explosion_shape` / `gui_session.toml` | `apply_session_to_physic` |
| `physics.preset_weights` | Preset weight controls used before adding a shape | `GuiSessionState.preset_weights` / `gui_session.toml` | `GuiSettings::new` and Reload Session |
| `renderer.config` | Tone mapping and all Bloom controls (enabled, intensity, iterations, downsample, blur method) | `RendererConfig` / `renderer.toml` | simulator startup and Reload Session |
| `renderer.comparison` | Tone-mapping grid comparison | `GuiSessionState` / `gui_session.toml` | `Simulator::new` and Reload Session |

Buttons that perform an immediate action rather than retain a setting (save,
reload, reset, shader reload, start/stop stress scene) have no persisted value.
Their resulting settings are covered by the rows above.

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
