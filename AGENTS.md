# Human Explicit Confirmation Before Git Commit Policy

**NEVER perform `git commit` or `git push` automatically without explicit human confirmation first!**

- Always present the implementation, test results, and diff summary to the user.
- Explicitly ask for human approval before executing any `git commit` or `git push`.
- Only proceed with `git commit` / `git push` once the human user explicitly responds with agreement/validation.

# GUI persistence rule

For every persistent ImGui control, update its canonical configuration, save on
shutdown, startup restore, and **Reload Session** in the same change.  Add its
`GUI_PERSIST:` marker and inventory row in `doc/gui_persistence_inventory.md`,
then run `task gui-persistence-check`.

Do not store a second copy of engine state in `GuiSettings`: derive it from the
audio, physics, or renderer source of truth when serializing the GUI session.

# Zero Magic Constants & Dynamic UI Scaling Policy

**STRICT ZERO MAGIC CONSTANTS & RESPONSIVE UI SCALING**:

- **No Magic Literals**: Never hardcode numeric literals (int, float), color RGBA tuples (`[f32; 4]`), asset path string templates, or preset specifications inline in domain logic, command handlers, session state, or GUI rendering functions.
- **SSOT Centralization**: Every constant MUST be declared as a named public constant or struct in its canonical domain single source of truth module:
  - Physics & Smoke constants: `src/physic_engine/constants.rs`
  - Audio constants: `src/audio_engine/constants.rs`
  - Renderer constants: `src/renderer_engine/constants.rs`
  - UI Theme & Color Palette Tokens: `src/simulator/gui_settings/theme.rs`
- **Dynamic UI Item Width Scaling**: Never hardcode static pixel widths (e.g. `set_next_item_width(200.0)` or static `clamp(200.0, 360.0)`). ALWAYS scale item widths dynamically relative to `ui.current_font_size()` (e.g. `set_next_item_width(ui.current_font_size() * 14.0)` and `clamp(font_sz * 14.0, font_sz * 26.0)`) to respect font size changes and global UI scaling (`gui_scale`).

