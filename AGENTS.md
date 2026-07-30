# GUI persistence rule

For every persistent ImGui control, update its canonical configuration, save on
shutdown, startup restore, and **Reload Session** in the same change.  Add its
`GUI_PERSIST:` marker and inventory row in `doc/gui_persistence_inventory.md`,
then run `task gui-persistence-check`.

Do not store a second copy of engine state in `GuiSettings`: derive it from the
audio, physics, or renderer source of truth when serializing the GUI session.
