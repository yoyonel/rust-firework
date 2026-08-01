#![cfg(feature = "interactive_tests")]

use fireworks_sim::utils::command_console::{CommandRegistry, Console};

mod helpers;
use helpers::{TestAudio, TestPhysic};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_console_commands_full_suite() {
    let log = Rc::new(RefCell::new(vec![]));
    let physic = TestPhysic::new(log.clone());
    let audio = TestAudio::new(log);
    let mut console = Console::new();
    let mut registry = CommandRegistry::new();

    // Register basic test commands
    registry.register_for_audio("audio.mute", |_engine, _, _cmd_queue| {
        "Audio muted".to_string()
    });
    registry.register_for_audio("audio.unmute", |_engine, _, _cmd_queue| {
        "Audio unmuted".to_string()
    });
    registry.register_for_audio("audio.volume", |engine, input, _cmd_queue| {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() > 1 {
            let val = parts[1].parse::<f32>().unwrap_or(1.0);
            format!("Master Audio Volume -> {:.2}", val)
        } else {
            format!("Master Audio Volume = {:.2}", engine.get_master_volume())
        }
    });

    registry.register_for_physic("physic.config", |engine, _, _cmd_queue| {
        format!("Applied Configuration: {:?}", engine.get_config())
    });
    registry.register_for_physic("physic.apply", |_, _, _cmd_queue| "applied".to_string());
    registry.register_for_physic("physic.reset", |_, _, _cmd_queue| "reset".to_string());
    registry.register_for_physic("physic.max_rockets", |engine, args, _cmd_queue| {
        let val_str = args.split_whitespace().nth(1).unwrap_or("");
        if val_str.is_empty() {
            format!(
                "Usage: physic.max_rockets <value> (applied: {})",
                engine.get_config().max_rockets
            )
        } else if let Ok(val) = val_str.parse::<usize>() {
            format!("Set physic.max_rockets = {}", val)
        } else {
            "Invalid number".to_string()
        }
    });

    registry.register_for_renderer("renderer.bloom", |input, _cmd_queue| {
        format!("Bloom command: {}", input)
    });
    registry.register_for_renderer("renderer.tonemapping.mode", |input, _cmd_queue| {
        format!("Tonemapping mode: {}", input)
    });

    let mut cmd_queue = Vec::new();

    // 1. Audio Commands
    let res_mute =
        console.execute_command("audio.mute", &mut cmd_queue, &audio, &physic, &registry);
    assert_eq!(res_mute, "Audio muted");

    let res_unmute =
        console.execute_command("audio.unmute", &mut cmd_queue, &audio, &physic, &registry);
    assert_eq!(res_unmute, "Audio unmuted");

    let res_vol_get =
        console.execute_command("audio.volume", &mut cmd_queue, &audio, &physic, &registry);
    assert!(res_vol_get.contains("Master Audio Volume"));

    let res_vol_set = console.execute_command(
        "audio.volume 0.5",
        &mut cmd_queue,
        &audio,
        &physic,
        &registry,
    );
    assert!(res_vol_set.contains("0.50"));

    // 2. Physics Commands
    let res_get = console.execute_command(
        "physic.max_rockets",
        &mut cmd_queue,
        &audio,
        &physic,
        &registry,
    );
    assert!(res_get.contains("Usage:"));

    let res_set = console.execute_command(
        "physic.max_rockets 4096",
        &mut cmd_queue,
        &audio,
        &physic,
        &registry,
    );
    assert!(res_set.contains("Set"));

    let res_cfg =
        console.execute_command("physic.config", &mut cmd_queue, &audio, &physic, &registry);
    assert!(res_cfg.contains("Applied Configuration"));

    let res_apply =
        console.execute_command("physic.apply", &mut cmd_queue, &audio, &physic, &registry);
    assert_eq!(res_apply, "applied");

    let res_reset =
        console.execute_command("physic.reset", &mut cmd_queue, &audio, &physic, &registry);
    assert_eq!(res_reset, "reset");

    // 3. Renderer Commands
    let res_bloom = console.execute_command(
        "renderer.bloom enable",
        &mut cmd_queue,
        &audio,
        &physic,
        &registry,
    );
    assert!(res_bloom.contains("Bloom command"));

    let res_tone = console.execute_command(
        "renderer.tonemapping.mode aces",
        &mut cmd_queue,
        &audio,
        &physic,
        &registry,
    );
    assert!(res_tone.contains("Tonemapping mode"));
}
