use fireworks_sim::utils::command_console::{CommandRegistry, HistoryCursor, SelectionCycler};
use std::cell::RefCell;
use std::rc::Rc;

mod helpers;
use helpers::{TestAudio, TestPhysic};

// ============================================================================
// HistoryCursor Tests
// ============================================================================

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
fn test_history_cursor_next_recent() {
    let history = vec!["cmd1".to_string(), "cmd2".to_string(), "cmd3".to_string()];
    let mut cursor = HistoryCursor::new(&history);

    // Navigate backward first
    assert_eq!(cursor.prev(), Some("cmd3"));
    assert_eq!(cursor.prev(), Some("cmd2"));

    // Now navigate forward (toward more recent)
    assert_eq!(cursor.next_recent(), Some("cmd3"));

    // Going past the end returns None (back to empty input)
    assert_eq!(cursor.next_recent(), None);

    // Calling next_recent again when already at None should still return None
    assert_eq!(cursor.next_recent(), None);
}

#[test]
fn test_history_cursor_reset() {
    let history = vec!["cmd1".to_string(), "cmd2".to_string()];
    let mut cursor = HistoryCursor::new(&history);

    // Navigate into history
    assert_eq!(cursor.prev(), Some("cmd2"));
    assert_eq!(cursor.prev(), Some("cmd1"));

    // Reset should bring us back to "no selection" state
    cursor.reset();

    // After reset, prev() should start from the last command again
    assert_eq!(cursor.prev(), Some("cmd2"));
}

#[test]
fn test_history_cursor_empty_history() {
    let history: Vec<String> = vec![];
    let mut cursor = HistoryCursor::new(&history);

    // With empty history, prev() and next_recent() should return None
    assert_eq!(cursor.prev(), None);
    assert_eq!(cursor.next_recent(), None);
}

#[test]
fn test_history_cursor_single_element() {
    let history = vec!["only_cmd".to_string()];
    let mut cursor = HistoryCursor::new(&history);

    // First prev should return the only command
    assert_eq!(cursor.prev(), Some("only_cmd"));

    // Second prev should return None (at boundary)
    assert_eq!(cursor.prev(), None);

    // After boundary, next prev should loop back
    assert_eq!(cursor.prev(), Some("only_cmd"));

    // Reset and navigate forward
    cursor.reset();
    assert_eq!(cursor.next_recent(), None); // No index set, returns None
}

// ============================================================================
// SelectionCycler Tests
// ============================================================================

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
fn test_selection_cycler_empty() {
    let suggestions: Vec<String> = vec![];
    let mut cycler = SelectionCycler::new(&suggestions);

    // With empty suggestions, all methods should handle gracefully
    assert_eq!(cycler.get_current(), None);
    assert_eq!(cycler.next_cyclic(), None);
    assert_eq!(cycler.get_index(), 0); // Index stays at 0 even if empty
}

#[test]
fn test_selection_cycler_single_element() {
    let suggestions = vec!["only_suggestion".to_string()];
    let mut cycler = SelectionCycler::new(&suggestions);

    assert_eq!(cycler.get_current(), Some("only_suggestion"));
    assert_eq!(cycler.get_index(), 0);

    // Cycling with single element should stay on same element
    assert_eq!(cycler.next_cyclic(), Some("only_suggestion"));
    assert_eq!(cycler.get_index(), 0);
}

#[test]
fn test_selection_cycler_many_cycles() {
    let suggestions = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let mut cycler = SelectionCycler::new(&suggestions);

    // Cycle through multiple times
    for _ in 0..10 {
        cycler.next_cyclic();
    }

    // After 10 cycles starting from 0: 10 % 3 = 1
    assert_eq!(cycler.get_index(), 1);
    assert_eq!(cycler.get_current(), Some("b"));
}

// ============================================================================
// CommandRegistry Tests
// ============================================================================

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

#[test]
fn test_command_registry_renderer_commands() {
    let log = Rc::new(RefCell::new(vec![]));
    let mut audio = TestAudio::new(log.clone());
    let mut physic = TestPhysic::new(log.clone());

    let mut registry = CommandRegistry::new();

    // Register renderer command (stateless, doesn't use engine reference)
    registry.register_for_renderer("renderer.test", |args| {
        format!("Renderer received: {}", args)
    });

    let result = registry.execute(&mut audio, &mut physic, "renderer.test some_arg");
    assert_eq!(result, "Renderer received: renderer.test some_arg");
}

#[test]
fn test_command_registry_get_commands() {
    let mut registry = CommandRegistry::new();

    registry.register_for_audio("audio.mute", |_, _| "".to_string());
    registry.register_for_physic("physic.pause", |_, _| "".to_string());
    registry.register_for_renderer("renderer.bloom", |_| "".to_string());

    let commands = registry.get_commands();

    assert!(commands.contains(&"audio.mute".to_string()));
    assert!(commands.contains(&"physic.pause".to_string()));
    assert!(commands.contains(&"renderer.bloom".to_string()));
    assert_eq!(commands.len(), 3);
}

#[test]
fn test_command_registry_arg_suggestions() {
    let mut registry = CommandRegistry::new();

    // Register command with argument suggestions
    registry.register_for_renderer("renderer.bloom.method", |_| "".to_string());
    registry.register_args("renderer.bloom.method", vec!["gaussian", "kawase"]);

    let suggestions = registry.get_arg_suggestions("renderer.bloom.method");
    assert_eq!(suggestions.len(), 2);
    assert!(suggestions.contains(&"gaussian".to_string()));
    assert!(suggestions.contains(&"kawase".to_string()));

    // Non-existent command should return empty slice
    let empty = registry.get_arg_suggestions("nonexistent.cmd");
    assert!(empty.is_empty());
}

#[test]
fn test_command_registry_hints() {
    let mut registry = CommandRegistry::new();

    registry.register_hint("renderer.bloom.intensity", "Usage: <0.0-10.0>");
    registry.register_hint("audio.volume", "Usage: <0-100>");

    assert_eq!(
        registry.get_hint("renderer.bloom.intensity"),
        Some(&"Usage: <0.0-10.0>".to_string())
    );
    assert_eq!(
        registry.get_hint("audio.volume"),
        Some(&"Usage: <0-100>".to_string())
    );
    assert_eq!(registry.get_hint("nonexistent"), None);
}

#[test]
fn test_command_registry_empty_input() {
    let log = Rc::new(RefCell::new(vec![]));
    let mut audio = TestAudio::new(log.clone());
    let mut physic = TestPhysic::new(log);

    let registry = CommandRegistry::new();

    // Empty input should return empty string
    let result = registry.execute(&mut audio, &mut physic, "");
    assert_eq!(result, "");

    // Whitespace-only input should also return empty string
    let result2 = registry.execute(&mut audio, &mut physic, "   ");
    assert_eq!(result2, "");
}

#[test]
fn test_command_registry_no_dot_in_command() {
    let log = Rc::new(RefCell::new(vec![]));
    let mut audio = TestAudio::new(log.clone());
    let mut physic = TestPhysic::new(log);

    let registry = CommandRegistry::new();

    // Command without dot should report missing prefix
    let result = registry.execute(&mut audio, &mut physic, "nodotcommand");
    assert!(result.contains("Missing engine prefix"));
}

#[test]
fn test_command_registry_with_arguments() {
    let log = Rc::new(RefCell::new(vec![]));
    let mut audio = TestAudio::new(log.clone());
    let mut physic = TestPhysic::new(log);

    let mut registry = CommandRegistry::new();

    // Register command that parses arguments
    registry.register_for_audio("audio.volume", |_engine, args| {
        // args contains the full input string "audio.volume 50"
        let parts: Vec<&str> = args.split_whitespace().collect();
        if parts.len() >= 2 {
            format!("Volume set to {}", parts[1])
        } else {
            "No volume specified".to_string()
        }
    });

    let result = registry.execute(&mut audio, &mut physic, "audio.volume 75");
    assert_eq!(result, "Volume set to 75");

    let result_no_arg = registry.execute(&mut audio, &mut physic, "audio.volume");
    assert_eq!(result_no_arg, "No volume specified");
}

#[test]
fn test_command_registry_default_trait() {
    // Test that Default trait implementation works
    let registry = CommandRegistry::default();
    assert!(registry.get_commands().is_empty());
}

// ============================================================================
// Console Integration Tests (require OpenGL context via interactive_tests)
// ============================================================================

#[cfg(feature = "interactive_tests")]
mod console_integration_tests {
    use super::*;
    use fireworks_sim::utils::command_console::{generate_noise_texture, Console};
    use fireworks_sim::window_engine::{GlfwWindowEngine, WindowEngine};

    /// Helper to create a window engine for tests
    fn create_test_context() -> GlfwWindowEngine {
        GlfwWindowEngine::init(100, 100, "Console Test").expect("Failed to create test context")
    }

    // ---------- Console Creation Tests ----------

    #[test]
    fn test_console_new_with_opengl_context() {
        let _window = create_test_context();
        let console = Console::new();

        assert!(!console.open);
        assert!(!console.focus_previous_widget);
    }

    #[test]
    fn test_console_default_trait() {
        let _window = create_test_context();
        let console = Console::default();

        assert!(!console.open);
    }

    #[test]
    fn test_generate_noise_texture() {
        let _window = create_test_context();
        let tex_id = generate_noise_texture();

        // Texture ID should be non-zero (valid OpenGL texture)
        assert!(tex_id > 0);

        // Cleanup
        unsafe {
            gl::DeleteTextures(1, &tex_id);
        }
    }

    // ---------- Console Log Tests ----------

    #[test]
    fn test_console_log() {
        let _window = create_test_context();
        let mut console = Console::new();

        console.log("First message");
        console.log("Second message");
        console.log(String::from("Third message"));

        let output = console.get_output();
        assert_eq!(output.len(), 3);
        assert_eq!(output[0], "First message");
        assert_eq!(output[1], "Second message");
        assert_eq!(output[2], "Third message");
    }

    // ---------- Autocomplete Tests ----------

    #[test]
    fn test_console_update_autocomplete_empty_input() {
        let _window = create_test_context();
        let mut console = Console::new();
        let registry = CommandRegistry::new();

        console.set_input("");
        console.update_autocomplete(&registry);

        assert!(console.get_suggestions().is_empty());
        assert_eq!(console.get_selected_suggestion(), 0);
    }

    #[test]
    fn test_console_update_autocomplete_matches_commands() {
        let _window = create_test_context();
        let mut console = Console::new();
        let mut registry = CommandRegistry::new();

        registry.register_for_audio("audio.mute", |_, _| "".to_string());
        registry.register_for_audio("audio.unmute", |_, _| "".to_string());
        registry.register_for_renderer("renderer.bloom", |_| "".to_string());

        // Search for "audio"
        console.set_input("audio");
        console.update_autocomplete(&registry);

        let suggestions = console.get_suggestions();
        assert!(suggestions.len() >= 2);
        assert!(suggestions.iter().any(|s| s.contains("audio.mute")));
        assert!(suggestions.iter().any(|s| s.contains("audio.unmute")));
    }

    #[test]
    fn test_console_update_autocomplete_fuzzy_match() {
        let _window = create_test_context();
        let mut console = Console::new();
        let mut registry = CommandRegistry::new();

        registry.register_for_renderer("renderer.bloom.intensity", |_| "".to_string());
        registry.register_for_renderer("renderer.bloom.threshold", |_| "".to_string());

        // Fuzzy match "blint" should match "bloom.intensity"
        console.set_input("blint");
        console.update_autocomplete(&registry);

        let suggestions = console.get_suggestions();
        assert!(!suggestions.is_empty());
        // The fuzzy matcher should find bloom.intensity
        assert!(suggestions.iter().any(|s| s.contains("intensity")));
    }

    #[test]
    fn test_console_update_autocomplete_includes_internal_commands() {
        let _window = create_test_context();
        let mut console = Console::new();
        let registry = CommandRegistry::new();

        // Search for "cl" should match "clear"
        console.set_input("cl");
        console.update_autocomplete(&registry);

        let suggestions = console.get_suggestions();
        assert!(suggestions.iter().any(|s| s == "clear"));
    }

    #[test]
    fn test_console_update_autocomplete_with_arg_suggestions() {
        let _window = create_test_context();
        let mut console = Console::new();
        let mut registry = CommandRegistry::new();

        registry.register_for_renderer("renderer.bloom.method", |_| "".to_string());
        registry.register_args("renderer.bloom.method", vec!["gaussian", "kawase"]);

        // Type command with space to trigger arg completion
        console.set_input("renderer.bloom.method ");
        console.update_autocomplete(&registry);

        let suggestions = console.get_suggestions();
        assert_eq!(suggestions.len(), 2);
        assert!(suggestions.iter().any(|s| s.contains("gaussian")));
        assert!(suggestions.iter().any(|s| s.contains("kawase")));
    }

    #[test]
    fn test_console_update_autocomplete_arg_fuzzy_match() {
        let _window = create_test_context();
        let mut console = Console::new();
        let mut registry = CommandRegistry::new();

        registry.register_for_renderer("renderer.tonemapping", |_| "".to_string());
        registry.register_args(
            "renderer.tonemapping",
            vec!["reinhard", "aces", "filmic", "uncharted2"],
        );

        // Partial arg match
        console.set_input("renderer.tonemapping ac");
        console.update_autocomplete(&registry);

        let suggestions = console.get_suggestions();
        assert!(!suggestions.is_empty());
        // "aces" should be in suggestions
        assert!(suggestions.iter().any(|s| s.contains("aces")));
    }

    #[test]
    fn test_console_update_autocomplete_adds_trailing_space() {
        let _window = create_test_context();
        let mut console = Console::new();
        let mut registry = CommandRegistry::new();

        registry.register_for_renderer("renderer.bloom.method", |_| "".to_string());
        registry.register_args("renderer.bloom.method", vec!["gaussian"]);

        // Commands with args should have trailing space
        console.set_input("renderer.bloom");
        console.update_autocomplete(&registry);

        let suggestions = console.get_suggestions();
        // The command with arg_suggestions should end with a space
        assert!(suggestions.iter().any(|s| s == "renderer.bloom.method "));
    }

    // ---------- Execute Command Tests ----------

    #[test]
    fn test_console_execute_clear_command() {
        let _window = create_test_context();
        let log = Rc::new(RefCell::new(vec![]));
        let mut audio = TestAudio::new(log.clone());
        let mut physic = TestPhysic::new(log);

        let mut console = Console::new();
        let registry = CommandRegistry::new();

        // Add some output
        console.log("line1");
        console.log("line2");
        assert_eq!(console.get_output().len(), 2);

        // Execute clear
        let result = console.execute_command("clear", &mut audio, &mut physic, &registry);

        assert_eq!(result, "");
        assert!(console.get_output().is_empty());
    }

    #[test]
    fn test_console_execute_help_command() {
        let _window = create_test_context();
        let log = Rc::new(RefCell::new(vec![]));
        let mut audio = TestAudio::new(log.clone());
        let mut physic = TestPhysic::new(log);

        let mut console = Console::new();
        let mut registry = CommandRegistry::new();
        registry.register_for_audio("audio.test", |_, _| "".to_string());

        let result = console.execute_command("help", &mut audio, &mut physic, &registry);

        assert_eq!(result, "");
        let output = console.get_output();
        assert_eq!(output.len(), 1);
        assert!(output[0].contains("Available commands"));
        assert!(output[0].contains("audio.test"));
        assert!(output[0].contains("clear"));
        assert!(output[0].contains("help"));
    }

    #[test]
    fn test_console_execute_registered_command() {
        let _window = create_test_context();
        let log = Rc::new(RefCell::new(vec![]));
        let mut audio = TestAudio::new(log.clone());
        let mut physic = TestPhysic::new(log);

        let mut console = Console::new();
        let mut registry = CommandRegistry::new();

        registry.register_for_audio("audio.mute", |engine, _| {
            engine.mute();
            "Audio muted".to_string()
        });

        let result = console.execute_command("audio.mute", &mut audio, &mut physic, &registry);

        assert_eq!(result, "Audio muted");
    }

    #[test]
    fn test_console_execute_unknown_command() {
        let _window = create_test_context();
        let log = Rc::new(RefCell::new(vec![]));
        let mut audio = TestAudio::new(log.clone());
        let mut physic = TestPhysic::new(log);

        let mut console = Console::new();
        let registry = CommandRegistry::new();

        let result = console.execute_command("nonexistent.cmd", &mut audio, &mut physic, &registry);

        assert!(result.contains("Unknown"));
    }

    #[test]
    fn test_console_execute_with_whitespace() {
        let _window = create_test_context();
        let log = Rc::new(RefCell::new(vec![]));
        let mut audio = TestAudio::new(log.clone());
        let mut physic = TestPhysic::new(log);

        let mut console = Console::new();
        let registry = CommandRegistry::new();

        // Command with leading/trailing whitespace
        let result = console.execute_command("  clear  ", &mut audio, &mut physic, &registry);

        assert_eq!(result, "");
        // clear should have worked
    }

    // ---------- Console State Tests ----------

    #[test]
    fn test_console_open_state() {
        let _window = create_test_context();
        let mut console = Console::new();

        assert!(!console.open);
        console.open = true;
        assert!(console.open);
    }

    #[test]
    fn test_console_focus_state() {
        let _window = create_test_context();
        let mut console = Console::new();

        assert!(!console.focus_previous_widget);
        console.focus_previous_widget = true;
        assert!(console.focus_previous_widget);
    }

    // ---------- Edge Cases ----------

    #[test]
    fn test_console_autocomplete_no_match() {
        let _window = create_test_context();
        let mut console = Console::new();
        let registry = CommandRegistry::new();

        console.set_input("xyznonexistent");
        console.update_autocomplete(&registry);

        // No matches should result in empty suggestions
        assert!(console.get_suggestions().is_empty());
    }

    #[test]
    fn test_console_autocomplete_selection_resets() {
        let _window = create_test_context();
        let mut console = Console::new();
        let mut registry = CommandRegistry::new();

        registry.register_for_audio("audio.a", |_, _| "".to_string());
        registry.register_for_audio("audio.b", |_, _| "".to_string());

        console.set_input("audio");
        console.update_autocomplete(&registry);

        // Selection should be reset to 0
        assert_eq!(console.get_selected_suggestion(), 0);
    }

    #[test]
    fn test_console_multiple_autocomplete_updates() {
        let _window = create_test_context();
        let mut console = Console::new();
        let mut registry = CommandRegistry::new();

        registry.register_for_audio("audio.mute", |_, _| "".to_string());
        registry.register_for_renderer("renderer.bloom", |_| "".to_string());

        // First search
        console.set_input("audio");
        console.update_autocomplete(&registry);
        let first_suggestions = console.get_suggestions().len();

        // Second search with different input
        console.set_input("renderer");
        console.update_autocomplete(&registry);
        let second_suggestions = console.get_suggestions().len();

        // Suggestions should be different
        assert!(first_suggestions > 0);
        assert!(second_suggestions > 0);

        // Third search clear
        console.set_input("");
        console.update_autocomplete(&registry);
        assert!(console.get_suggestions().is_empty());
    }
}
