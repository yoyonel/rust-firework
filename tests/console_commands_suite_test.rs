#![cfg(feature = "interactive_tests")]

use fireworks_sim::utils::command_console::{CommandRegistry, Console};

mod helpers;
use helpers::{TestAudio, TestPhysic};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_console_commands_full_suite() {
    let log = Rc::new(RefCell::new(vec![]));
    let mut physic = TestPhysic::new(log.clone());
    let mut audio = TestAudio::new(log);
    let mut console = Console::new();
    let mut registry = CommandRegistry::new();

    // Register basic test commands
    registry.register_for_audio("audio.mute", |engine, _| {
        engine.mute();
        "Audio muted".to_string()
    });
    registry.register_for_audio("audio.unmute", |engine, _| {
        engine.unmute();
        "Audio unmuted".to_string()
    });
    registry.register_for_audio("audio.volume", |engine, input| {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() > 1 {
            let val = parts[1].parse::<f32>().unwrap_or(1.0);
            engine.set_master_volume(val);
            format!("Master Audio Volume -> {:.2}", val)
        } else {
            format!("Master Audio Volume = {:.2}", engine.get_master_volume())
        }
    });

    registry.register_for_physic("physic.config", |engine, _| {
        format!("Applied Configuration: {:?}", engine.get_config())
    });
    registry.register_for_physic("physic.apply", |_, _| "applied".to_string());
    registry.register_for_physic("physic.reset", |_, _| "reset".to_string());
    registry.register_for_physic("physic.max_rockets", |engine, args| {
        let val_str = args.split_whitespace().nth(1).unwrap_or("");
        if val_str.is_empty() {
            format!(
                "Usage: physic.max_rockets <value> (applied: {})",
                engine.get_config().max_rockets
            )
        } else if let Ok(val) = val_str.parse::<usize>() {
            engine.get_config_mut().max_rockets = val;
            format!("Set physic.max_rockets = {}", val)
        } else {
            "Invalid number".to_string()
        }
    });

    registry.register_for_renderer("renderer.bloom", |input| {
        format!("Bloom command: {}", input)
    });
    registry.register_for_renderer("renderer.tonemapping.mode", |input| {
        format!("Tonemapping mode: {}", input)
    });

    // 1. Audio Commands
    let res_mute = console.execute_command("audio.mute", &mut audio, &mut physic, &registry);
    assert_eq!(res_mute, "Audio muted");

    let res_unmute = console.execute_command("audio.unmute", &mut audio, &mut physic, &registry);
    assert_eq!(res_unmute, "Audio unmuted");

    let res_vol_get = console.execute_command("audio.volume", &mut audio, &mut physic, &registry);
    assert!(res_vol_get.contains("Master Audio Volume"));

    let res_vol_set =
        console.execute_command("audio.volume 0.5", &mut audio, &mut physic, &registry);
    assert!(res_vol_set.contains("0.50"));

    // 2. Physics Commands
    let res_get = console.execute_command("physic.max_rockets", &mut audio, &mut physic, &registry);
    assert!(res_get.contains("Usage:"));

    let res_set = console.execute_command(
        "physic.max_rockets 4096",
        &mut audio,
        &mut physic,
        &registry,
    );
    assert!(res_set.contains("Set"));

    let res_cfg = console.execute_command("physic.config", &mut audio, &mut physic, &registry);
    assert!(res_cfg.contains("Applied Configuration"));

    let res_apply = console.execute_command("physic.apply", &mut audio, &mut physic, &registry);
    assert_eq!(res_apply, "applied");

    let res_reset = console.execute_command("physic.reset", &mut audio, &mut physic, &registry);
    assert_eq!(res_reset, "reset");

    // 3. Renderer Commands
    let res_bloom =
        console.execute_command("renderer.bloom enable", &mut audio, &mut physic, &registry);
    assert!(res_bloom.contains("Bloom command"));

    let res_tone = console.execute_command(
        "renderer.tonemapping.mode aces",
        &mut audio,
        &mut physic,
        &registry,
    );
    assert!(res_tone.contains("Tonemapping mode"));
}
