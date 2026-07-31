# ImGui Control Panel & Session Persistence Technical Documentation

This document describes the design, architecture, and feature set of the ImGui Engine Control Panel (`F4`) and the persistent session system introduced in the Fireworks Simulator.

---

## 📌 Overview & Architecture

The ImGui Control Panel provides an interactive graphical user interface (GUI) integrating all engine settings across **Audio**, **Physics**, and **Renderer & Post-FX** into a unified 4-tab window.

- **Shortcut Key**: `F4` (toggles panel visibility & cursor mode).
- **Core Code**:
  - `src/simulator/gui_settings.rs` ([GuiSettings](../src/simulator/gui_settings.rs#L60-L88))
  - `src/simulator/ui.rs` ([ui.rs](../src/simulator/ui.rs))
  - `src/simulator/events.rs` ([events.rs](../src/simulator/events.rs))
- **Session File**: `assets/config/gui_session.toml` ([GuiSessionState](../src/simulator/gui_settings.rs#L13-L45))

---

## 🎛️ GUI Control Panel Structure

```
+-----------------------------------------------------------------------------------------------+
| Engine Control Panel -- Settings (F4)                                                         |
+-----------------------------------------------------------------------------------------------+
| [SAVE SESSION]  [RELOAD SESSION]  Theme: [Cyberpunk Cyan v]  Filter: [Filter settings...    ] |
+-----------------------------------------------------------------------------------------------+
| [ Audio ]  [ Physics ]  [ Renderer & Post-FX ]  [ Console Commands ]                         |
+-----------------------------------------------------------------------------------------------+
|                                                                                               |
|  (Tab Content rendered based on selected tab and live search filter)                          |
|                                                                                               |
+-----------------------------------------------------------------------------------------------+

---

### 🎨 ImGui Themes & Visual Styling

The ImGui Control Panel includes a live **Theme Selector** in the header toolbar, allowing instant switching between 5 distinct visual styles:

| Theme Name | Visual Style & Color Palette | Highlights & Accents |
| :--- | :--- | :--- |
| **Cyberpunk Cyan** *(Default)* | Obsidian dark (`#0f121a`), Neon Cyan borders | Cyan (`#00d9f2`) buttons/sliders & Magenta (`#f233a6`) active highlights |
| **Deep Sapphire** | Sleek Slate (`#1a1f29`), Soft rounded frame corners | Sapphire Blue (`#3380f2`) & Ice Blue (`#59b3ff`) accents |
| **Emerald Firework** | Deep Charcoal (`#141714`), Nature-inspired palette | Emerald Green (`#33e68c`) & Gold Spark (`#f2b333`) highlights |
| **Dracula Synthwave** | Purple Navy (`#1f1c2b`), Synthwave aesthetic | Pink Synth (`#fa73b3`) & Purple Glow (`#bf66e6`) accents |
| **Classic Dark** | Standard ImGui dark theme | Classic grey frames and sharp edges |

- **Live Theme Switching**: Theme changes take effect instantly without restarting the application.
- **Persistence**: Selected theme is saved to `assets/config/gui_session.toml` (`theme = "CyberpunkCyan"`) and automatically restored across runs.
```

---

### 1. Tab 1: Audio Engine Controls
- **Global Audio Controls**:
  - Master Output Volume Slider (`audio.volume`): Continuous volume control (`0%` to `200%`, default `80%`) with `[Reset Vol]`.
  - Quick Volume Presets: `Mute (0%)`, `Low (25%)`, `Medium (50%)`, `Default (80%)`, `Full (100%)`, `Boost (150%)`.
  - Master Mute toggle reading live status (`audio_engine.is_muted()`) + `Mute` / `Unmute` buttons.
  - `[RESET AUDIO DEFAULTS]` button (resets volume to 80%, unmutes, sets reverb wet to 8%, enables all DSP effects).
- **Spatial Reverb Wet Mix (`audio.reverb_wet`)**:
  - Continuous slider (`0.00` to `1.00`).
  - Presets: `Dry (0%)`, `Default (8%) [Reset]`, `Medium (20%)`, `Cathedral (50%)`, `Full Wet (100%)`.
- **DSP Effects Matrix (`audio.fx`, `audio.fx_all`)**:
  - Live checkboxes for all 11 DSP effects (`binaural`, `panning`, `distance_atten`, `lowpass`, `doppler`, `fade`, `gain_lerp`, `normalize`, `spatial_bus`, `spatial_reverb`, `hrtf_bus`).
  - `[ON] Enable All DSP Effects` and `[OFF] Disable All DSP Effects` buttons + `[RESET DSP EFFECTS]`.
- **Audio Diagnostics & Stress Scene**:
  - Checkboxes for Audio Diagnostic Monitor (`F3`) and Visual Event Overlay (waves / beams / badges).
  - Start / Stop Audio Stress Test Scene (32 active 3D sound sources).

---

### 2. Tab 2: Physics Engine Controls
- **Configuration Sync Banner & Actions**:
  - Live status badge (`[OK] Physics Configuration Synced` vs `WARNING: Pending Physics Changes Detected!`).
  - Buttons: `[APPLY] PENDING CHANGES` (`physic.apply`), `[SAVE] Save Config`, `[RELOAD] Reload Disk Config`, `[RESET PHYSICS DEFAULTS]`.
- **Simulation Capacity & Buffers**:
  - Sliders: `Max Concurrent Rockets` (1..2048), `Particles / Explosion` (10..2000), `Particles / Trail` (0..500) + `Reset Capacity Defaults`.
- **Rocket Launch & Spawn Dynamics**:
  - Sliders: `Spawn Interval Mean (s)`, `Interval Variation`, `Max Next Interval`, `Spawn Margin`, `Vertical Angle (rad)`, `Angle Variation`, `Spawn Min Speed`, `Spawn Max Speed`, `Initial Rocket Speed` + `Reset Spawn Defaults`.
- **Forces & Particle Physics**:
  - Sliders: `Gravity` (-2000..2000), `Explosion Speed Threshold`, `Explosion Min Velocity`, `Explosion Max Velocity` + `Reset Forces Defaults`.
- **Explosion Shape & Presets (`physic.explosion.*`)**:
  - Mode indicator (`Spherical`, `Single Image`, `MultiImage`).
  - `Spherical Mode` button + `Reset to Spherical`.
  - **Presets & Weight Adjustment**:
    - Vertical row layout per preset (`Heart`, `Star`, `Smiley`, `Note`, `Ring`).
    - Dedicated weight slider per preset (`0.1` to `10.0`) + `Reset W` button.
    - Buttons: `Set Single` (single image mode) and `+ Add (<w>w)` (cumulative weighted MultiImage mode).
  - **Active Shape Controls & Parameter Reset**:
    - Displays active probability breakdown (e.g. `heart (33.3% d'apparition)`).
    - Dedicated live sliders for `Weight`, `Image Scale (px)` (20..500), and `Flight Time (s)` (0.2..5.0) per active shape.
    - Dedicated reset buttons per parameter (`Reset W`, `Reset Scale`, `Reset Flight`, `[Reset Defaults]`).
    - Dedicated **`[X Delete]`** button to remove any shape from the MultiImage group (reverts to Spherical if empty).

---

### 3. Tab 3: Renderer & Post-FX Controls
- **Shader & Config Actions**:
  - Buttons: `[RELOAD] Reload Shaders` (`renderer.reload_shaders`), `[SAVE] Save Config`, `[RELOAD] Reload Disk Config`, `[RESET RENDERER DEFAULTS]`.
- **Tonemapping (`renderer.tonemapping`)**:
  - Combo dropdown selector: `Reinhard`, `Reinhard Extended`, `ACES`, `Uncharted 2`, `AgX`, `Khronos PBR`.
  - Checkbox: `Grid Comparison Mode` (`renderer.tonemapping.compare`).
  - Button: `Reset Tonemapping`.
- **Bloom Pipeline (`renderer.bloom.*`)**:
  - Checkbox `Enable Bloom`.
  - Sliders: `Intensity` (0.0..10.0), `Iterations` (1..10).
  - Combo: `Downsample` (`1x (Native)`, `2x (Half)`, `4x (Quarter)`).
  - Combo: `Blur Method` (`Gaussian (Classic 5-tap dual)`, `Kawase (Optimized pyramid)`).
  - Button: `Reset Bloom Defaults`.

---

### 4. Tab 4: Console Commands Overview
- Complete catalog of all registered console commands sorted alphabetically.
- Real-time value inspection (`= <current_value>`) and usage hints.

---

## 💾 Session Persistence (`assets/config/gui_session.toml`)

The GUI session state is serialized using `serde` and `toml` into `assets/config/gui_session.toml`.

### Persisted Fields
```toml
gui_open = true
active_tab = 1
search_filter = ""
show_audio_diagnostic = false
show_audio_visual_overlay = true
audio_muted = false
audio_reverb_wet = 0.08
audio_dsp_mask = 2047
window_pos = [25.6, 34.8]
window_size = [660.0, 580.0]
scroll_y = 120.5
```

### Automatic Lifecycle
1. **Startup (`Simulator::new`)**: `GuiSessionState::load_from_file` restores window visibility (`open`), active tab, search filter, window position (`window_pos`), window size (`window_size`), vertical scroll offset (`scroll_y`), diagnostic overlay state, mute status, spatial reverb gain, and DSP mask onto the engine.
2. **Shutdown (`Simulator::step`)**: Automatically saves the exact GUI state, layout, and scroll position when closing or exiting.
3. **Manual**: Buttons `[SAVE SESSION]` and `[RELOAD SESSION]` in the top GUI toolbar.

---

## 🛡️ Systemic Rule: "Reset to Default"

> **Systemic UI Directive**:
> *Every single parameter, slider, control, or input added to any GUI tab MUST have a clear, dedicated "Reset to Default" button next to it.*

### Reset Hierarchy
1. **Parameter Level**: `Reset W`, `Reset Scale`, `Reset Flight` buttons next to individual sliders.
2. **Section Level**: `Reset Capacity Defaults`, `Reset Spawn Defaults`, `Reset Forces Defaults`, `Reset Bloom Defaults`, `Reset Tonemapping`.
3. **Global Level**: `[RESET AUDIO DEFAULTS]`, `[RESET PHYSICS DEFAULTS]`, `[RESET RENDERER DEFAULTS]`.

---

## 📊 Exhaustive Comparison: Console Commands vs ImGui GUI

| Feature / Setting | Console Command | ImGui GUI (`F4`) | GUI Status & Capabilities |
|---|---|---|---|
| Mute Audio | `audio.mute` / `audio.unmute` | Checkbox + Mute/Unmute buttons | ✅ Fully supported with live status query |
| Spatial Reverb Wet Gain | `audio.reverb_wet <val>` | Slider (0.0..1.0) + 5 Presets | ✅ Fully supported with presets & reset |
| DSP Effects (11 effects) | `audio.fx <name> <on/off>` | 11 Checkboxes | ✅ Fully supported with live toggles |
| Enable/Disable All DSP | `audio.fx_all <on/off>` | `[ON] Enable All` / `[OFF] Disable All` | ✅ Fully supported |
| Physics Config Sync | `physic.apply` | `[APPLY] PENDING CHANGES` button | ✅ Fully supported |
| Physics Disk Config | `physic.config.save` / `reload` | `[SAVE]` / `[RELOAD]` buttons | ✅ Fully supported |
| Simulation Capacity | `physic.max_rockets`, etc. | 3 Sliders + Reset Defaults | ✅ Fully supported |
| Rocket Spawn Dynamics | `physic.spawn_*`, `interval_*` | 9 Sliders + Reset Defaults | ✅ Fully supported |
| Forces & Gravity | `physic.gravity`, `explosion_*` | 4 Sliders + Reset Defaults | ✅ Fully supported |
| Spherical Shape Mode | `physic.explosion.shape spherical` | `Spherical Mode` button | ✅ Fully supported |
| Preset Shapes (Heart, Star, etc.) | `physic.explosion.preset <name>` | Vertical Rows per Preset | ✅ Fully supported with Set & + Add |
| Preset Add Weight | `physic.explosion.add <path> <w>` | Weight Slider per Preset | ✅ Fully supported with individual sliders |
| Shape Weight Adjustment | `physic.explosion.weight <name> <w>` | Live Weight Slider per Active Shape | ✅ Fully supported with `Reset W` |
| Shape Scale Adjustment | `physic.explosion.scale <val>` | Live Image Scale Slider per Shape | ✅ Fully supported with `Reset Scale` |
| Shape Flight Time | `physic.explosion.flight_time <val>`| Live Flight Time Slider per Shape | ✅ Fully supported with `Reset Flight` |
| Shape Probability Stats | `physic.explosion.stats` | Live Probability Breakdown (%) | ✅ Fully supported |
| Shape Deletion | *(N/A via single cmd)* | **`[X Delete]`** button per Shape | ✅ **GUI Exclusive Feature** |
| Reload Shaders | `renderer.reload_shaders` | `[RELOAD] Reload Shaders` button | ✅ Fully supported |
| Tonemapping Curve | `renderer.tonemapping <mode>` | Combo Dropdown (6 modes) | ✅ Fully supported |
| Grid Tonemapping Compare | `renderer.tonemapping.compare` | Checkbox `Grid Comparison Mode` | ✅ Fully supported |
| Graphical Elements Visibility | `renderer.{rockets,smoke,trails,explosions}.{enable,disable}` | 4 Checkboxes (Rockets, Smoke, Trails, Explosions) | ✅ Fully supported with `[Reset Visibility Defaults]` |
| Bloom Controls | `renderer.bloom.*` | 5 Sliders/Combos + Reset Defaults | ✅ Fully supported |
| Bloom Enable/Disable | `renderer.bloom.enable` / `disable` | Checkbox `Enable Bloom` | ✅ Fully supported |
| Bloom Parameters | `intensity`, `iterations`, `downsample`, `method` | Sliders & Combo Dropdowns | ✅ Fully supported |
