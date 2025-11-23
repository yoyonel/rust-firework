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

    // Navigate backward through history: starts at end, moves toward beginning
    assert_eq!(cursor.prev(), Some("cmd3")); // First Up -> last command
    assert_eq!(cursor.prev(), Some("cmd2")); // Second Up -> middle command
    assert_eq!(cursor.prev(), Some("cmd1")); // Third Up -> first command

    // Boundary behavior: when at index 0, prev() returns None and resets position
    // This causes the cursor to loop back to the end on the next prev() call
    assert_eq!(cursor.prev(), None); // At boundary, returns None
    assert_eq!(cursor.prev(), Some("cmd3")); // Loops back to last command
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
