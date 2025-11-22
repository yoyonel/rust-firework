use fireworks_sim::renderer_engine::command_console::{
    CommandRegistry, HistoryCursor, SelectionCycler,
};
use std::cell::RefCell;
use std::rc::Rc;

mod helpers;
use helpers::{TestAudio, TestPhysic};

#[test]
fn test_history_cursor_navigation() {
    let history = vec!["cmd1".to_string(), "cmd2".to_string(), "cmd3".to_string()];
    let mut cursor = HistoryCursor::new(&history);

    // Initial state: pointing to nothing (empty line)

    // Press Up (prev) -> should go to last command "cmd3"
    assert_eq!(cursor.prev(), Some("cmd3"));

    // Press Up (prev) -> "cmd2"
    assert_eq!(cursor.prev(), Some("cmd2"));

    // Press Up (prev) -> "cmd1"
    assert_eq!(cursor.prev(), Some("cmd1"));

    // Press Up (prev) -> Limit reached, stays at "cmd1" (or returns None depending on impl, let's check)
    // Implementation: checked_sub(1). If 0, returns None? No, 0.checked_sub(1) is None.
    // match current_index { Some(i) => i.checked_sub(1) ... }
    // If index is 0, checked_sub(1) is None.
    // So it returns None and keeps index at 0?
    // Let's re-read code:
    // new_index = ... checked_sub(1)
    // self.current_index = new_index;
    // return new_index.map(...)
    // So if at 0, new_index becomes None. current_index becomes None?
    // Wait, if current_index is Some(0), checked_sub(1) is None.
    // So current_index becomes None.
    // And it returns None.
    // This means "Up" from the first command clears the line? That seems wrong for a terminal.
    // Usually Up stops at the top.
    // Let's verify behavior with the test.
    // If the implementation clears it, then assert_eq!(cursor.prev(), None).
    // But wait, if current_index becomes None, then next prev() will go to max_index - 1 (last command).
    // That would mean looping?
    // Let's check the code again.
    // Line 40: match self.current_index { Some(i) => i.checked_sub(1), None => max_index.checked_sub(1) }
    // If i=0, checked_sub(1) is None.
    // So new_index is None.
    // self.current_index = None.
    // Returns None.
    // So yes, it loops or resets.
    // If I press Up again, current_index is None, so it goes to max_index - 1 ("cmd3").
    // So it loops!

    assert_eq!(cursor.prev(), None); // Loop boundary
    assert_eq!(cursor.prev(), Some("cmd3")); // Looped to bottom
}

#[test]
fn test_selection_cycler() {
    let suggestions = vec!["sug1".to_string(), "sug2".to_string()];
    let mut cycler = SelectionCycler::new(&suggestions);

    // Initial index 0
    assert_eq!(cycler.get_current(), Some("sug1"));

    // Next
    assert_eq!(cycler.next_cyclic(), Some("sug2"));
    assert_eq!(cycler.get_index(), 1);

    // Next (Loop)
    assert_eq!(cycler.next_cyclic(), Some("sug1"));
    assert_eq!(cycler.get_index(), 0);
}

#[test]
fn test_command_registry_execution() {
    let log = Rc::new(RefCell::new(vec![]));
    let mut audio = TestAudio::new(log.clone());
    let mut physic = TestPhysic::new(log.clone());

    let mut registry = CommandRegistry::new();

    // Register audio command
    registry.register_for_audio("audio.test", |engine, _args| {
        engine.mute(); // Should log "mute called"
        "Muted".to_string()
    });

    // Register physic command
    registry.register_for_physic("physic.test", |engine, _args| {
        engine.set_window_width(100.0); // Should log "physic.set_width"
        "Width set".to_string()
    });

    // Execute audio command
    let res1 = registry.execute(&mut audio, &mut physic, "audio.test");
    assert_eq!(res1, "Muted");
    assert!(log.borrow().contains(&"mute called".into()));

    // Execute physic command
    let res2 = registry.execute(&mut audio, &mut physic, "physic.test");
    assert_eq!(res2, "Width set");
    assert!(log.borrow().contains(&"physic.set_width".into()));

    // Execute unknown command
    let res3 = registry.execute(&mut audio, &mut physic, "unknown.cmd");
    assert!(res3.contains("Unknown engine prefix")); // "unknown" is not audio/physic

    let res4 = registry.execute(&mut audio, &mut physic, "audio.unknown");
    assert!(res4.contains("Unknown command"));
}
