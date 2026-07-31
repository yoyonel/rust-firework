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
