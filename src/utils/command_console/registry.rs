use std::collections::HashMap;

use crate::domain_contracts::EngineCommand;
use crate::AudioEngine;
use crate::PhysicEngine;

pub type AudioCommandFn =
    dyn Fn(&dyn AudioEngine, &str, &mut Vec<EngineCommand>) -> String + 'static;
pub type PhysicCommandFn =
    dyn Fn(&dyn PhysicEngine, &str, &mut Vec<EngineCommand>) -> String + 'static;
pub type RendererCommandFn = dyn Fn(&str, &mut Vec<EngineCommand>) -> String + 'static;
pub type DynamicArgProviderFn =
    dyn Fn(&dyn AudioEngine, &dyn PhysicEngine) -> Vec<String> + 'static;
pub type CurrentValueProviderFn = dyn Fn(&dyn AudioEngine, &dyn PhysicEngine) -> String + 'static;

pub struct CommandRegistry {
    commands_audio: HashMap<String, Box<AudioCommandFn>>,
    commands_physic: HashMap<String, Box<PhysicCommandFn>>,
    commands_renderer: HashMap<String, Box<RendererCommandFn>>,
    arg_suggestions: HashMap<String, Vec<String>>,
    // Dynamic suggestions provider: (Audio, Physic) -> Suggestions
    dynamic_arg_providers: HashMap<String, Box<DynamicArgProviderFn>>,
    hints: HashMap<String, String>,
    current_value_providers: HashMap<String, Box<CurrentValueProviderFn>>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands_audio: HashMap::new(),
            commands_physic: HashMap::new(),
            commands_renderer: HashMap::new(),
            arg_suggestions: HashMap::new(),
            dynamic_arg_providers: HashMap::new(),
            hints: HashMap::new(),
            current_value_providers: HashMap::new(),
        }
    }

    pub fn register_for_audio<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&dyn AudioEngine, &str, &mut Vec<EngineCommand>) -> String + 'static,
    {
        self.commands_audio.insert(name.to_string(), Box::new(func));
    }

    pub fn register_for_physic<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&dyn PhysicEngine, &str, &mut Vec<EngineCommand>) -> String + 'static,
    {
        self.commands_physic
            .insert(name.to_string(), Box::new(func));
    }

    pub fn register_for_renderer<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&str, &mut Vec<EngineCommand>) -> String + 'static,
    {
        self.commands_renderer
            .insert(name.to_string(), Box::new(func));
    }

    pub fn register_args(&mut self, name: &str, args: Vec<&str>) {
        self.arg_suggestions.insert(
            name.to_string(),
            args.iter().map(|s| s.to_string()).collect(),
        );
    }

    pub fn register_dynamic_args<F>(&mut self, name: &str, provider: F)
    where
        F: Fn(&dyn AudioEngine, &dyn PhysicEngine) -> Vec<String> + 'static,
    {
        self.dynamic_arg_providers
            .insert(name.to_string(), Box::new(provider));
    }

    pub fn get_arg_suggestions(&self, name: &str) -> &[String] {
        self.arg_suggestions.get(name).map_or(&[], Vec::as_slice)
    }

    // New method that combines static and dynamic suggestions
    pub fn get_arg_suggestions_combined(
        &self,
        name: &str,
        audio: &dyn AudioEngine,
        physic: &dyn PhysicEngine,
    ) -> Vec<String> {
        let mut suggestions = self.get_arg_suggestions(name).to_vec();

        if let Some(provider) = self.dynamic_arg_providers.get(name) {
            suggestions.extend(provider(audio, physic));
        }

        suggestions
    }

    pub fn register_hint(&mut self, name: &str, hint: &str) {
        self.hints.insert(name.to_string(), hint.to_string());
    }

    pub fn get_hint(&self, name: &str) -> Option<&String> {
        self.hints.get(name)
    }

    pub fn register_current_value<F>(&mut self, name: &str, provider: F)
    where
        F: Fn(&dyn AudioEngine, &dyn PhysicEngine) -> String + 'static,
    {
        self.current_value_providers
            .insert(name.to_string(), Box::new(provider));
    }

    pub fn get_current_value(
        &self,
        name: &str,
        audio: &dyn AudioEngine,
        physic: &dyn PhysicEngine,
    ) -> Option<String> {
        self.current_value_providers
            .get(name)
            .map(|f| f(audio, physic))
    }

    pub fn execute(
        &self,
        audio_engine: &dyn AudioEngine,
        physic_engine: &dyn PhysicEngine,
        cmd_queue: &mut Vec<EngineCommand>,
        input_str: &str,
    ) -> String {
        // Handle potentially multiple commands separated by newlines
        // If input_str contains newlines, we process each line independently.
        let lines: Vec<&str> = input_str
            .split("\\n")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if lines.is_empty() {
            return "".into();
        }

        // If single line, avoid allocation overhead of vector results
        if lines.len() == 1 {
            return self.execute_single_line(audio_engine, physic_engine, cmd_queue, lines[0]);
        }

        let mut results = Vec::with_capacity(lines.len());
        for line in lines {
            let res = self.execute_single_line(audio_engine, physic_engine, cmd_queue, line);
            if !res.is_empty() {
                results.push(res);
            }
        }
        results.join("\n")
    }

    fn execute_single_line(
        &self,
        audio_engine: &dyn AudioEngine,
        physic_engine: &dyn PhysicEngine,
        cmd_queue: &mut Vec<EngineCommand>,
        input: &str,
    ) -> String {
        let cmd_key = input.split_whitespace().next().unwrap_or("");

        if cmd_key.is_empty() {
            return "".into();
        }

        // Try to split at the first dot. Example: "audio.mute" -> ("audio", "mute")
        let (prefix, _) = match cmd_key.split_once('.') {
            Some(pair) => pair,
            None => return format!("Unknown command '{}'. Missing engine prefix.", cmd_key),
        };

        let mut result = match prefix {
            "audio" => {
                if let Some(func) = self.commands_audio.get(cmd_key) {
                    func(audio_engine, input, cmd_queue)
                } else {
                    format!("Unknown command '{}'.", cmd_key)
                }
            }
            "physic" => {
                if let Some(func) = self.commands_physic.get(cmd_key) {
                    func(physic_engine, input, cmd_queue)
                } else {
                    format!("Unknown command '{}'.", cmd_key)
                }
            }
            "renderer" => {
                if let Some(func) = self.commands_renderer.get(cmd_key) {
                    func(input, cmd_queue)
                } else {
                    format!("Unknown command '{}'.", cmd_key)
                }
            }
            _ => format!("Unknown engine prefix '{}'.", prefix),
        };

        // Automatic "Current Value" display mechanism
        // If the user entered ONLY the command (no arguments)
        // AND the command returned a Usage hint (implying parameters were expected but missing)
        // AND we have a registered value provider
        // THEN append the current value to the output.
        if input == cmd_key && (result.starts_with("Usage:") || result.starts_with("x ")) {
            if let Some(val) = self.get_current_value(cmd_key, audio_engine, physic_engine) {
                result.push_str(&format!("\n-> Current value: {}", val));
            }
        }

        result
    }

    // Returns a Vec<String> of all registered command keys.
    // Optimized to avoid unnecessary cloning if we were just iterating,
    // but since we often need to collect them anyway, this is kept simple.
    // For further optimization, we could return an iterator, but that complicates
    // the borrow checker for the caller who might want to mutate the registry or console.
    pub fn get_commands(&self) -> Vec<String> {
        self.commands_audio
            .keys()
            .chain(self.commands_physic.keys())
            .chain(self.commands_renderer.keys())
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain_contracts::{AudioCommand, EngineCommand, PhysicCommand, RendererCommand};

    struct TestAudioEngine;

    impl AudioEngine for TestAudioEngine {
        fn play_rocket(&self, _: glam::Vec2, _: f32) {}
        fn play_rocket_with_id(&self, _: u64, _: glam::Vec2, _: f32) {}
        fn play_explosion(&self, _: glam::Vec2, _: f32) {}
        fn play_explosion_with_id(&self, _: u64, _: glam::Vec2, _: f32) {}
        fn start_audio_thread(&mut self, _: Option<&str>) {}
        fn stop_audio_thread(&mut self) {}
        fn set_listener_position(&mut self, _: glam::Vec2) {}
        fn get_listener_position(&self) -> glam::Vec2 {
            glam::Vec2::ZERO
        }
        fn mute(&mut self) {}
        fn unmute(&mut self) -> f32 {
            0.8
        }
        fn set_effect_enabled(&self, _: crate::audio_engine::effect_flags::AudioEffect, _: bool) {}
        fn set_all_effects_enabled(&self, _: bool) {}
        fn get_effect_enabled(&self, _: crate::audio_engine::effect_flags::AudioEffect) -> bool {
            false
        }
        fn get_effects_status(&self) -> String {
            String::new()
        }
        fn as_audio_engine(&self) -> &dyn AudioEngine {
            self
        }
    }

    #[test]
    fn test_execute_populates_cmd_queue_without_panic() {
        let mut registry = CommandRegistry::new();
        registry.register_for_audio("audio.volume", |_, input, queue| {
            if let Some(val_str) = input.split_whitespace().nth(1) {
                if let Ok(val) = val_str.parse::<f32>() {
                    queue.push(EngineCommand::Audio(AudioCommand::SetMasterVolume(val)));
                    return format!("Master volume set to {}", val);
                }
            }
            "Usage: audio.volume <f32>".into()
        });

        registry.register_for_physic("physic.gravity", |_, input, queue| {
            if let Some(val_str) = input.split_whitespace().nth(1) {
                if let Ok(val) = val_str.parse::<f32>() {
                    queue.push(EngineCommand::Physic(PhysicCommand::SetGravity(val)));
                    return format!("Gravity set to {}", val);
                }
            }
            "Usage: physic.gravity <f32>".into()
        });

        registry.register_for_renderer("renderer.reload_shaders", |_, queue| {
            queue.push(EngineCommand::Renderer(RendererCommand::ReloadShaders));
            "Reloading shaders".into()
        });

        let dummy_audio = TestAudioEngine;
        let config = crate::physic_engine::config::PhysicConfig::default();
        let dummy_physic =
            crate::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks::new(
                &config, 800.0, None,
            );
        let mut cmd_queue = Vec::new();

        let res = registry.execute(
            &dummy_audio,
            &dummy_physic,
            &mut cmd_queue,
            "audio.volume 0.5",
        );
        assert!(res.contains("Master volume set to 0.5"));
        assert_eq!(
            cmd_queue,
            vec![EngineCommand::Audio(AudioCommand::SetMasterVolume(0.5))]
        );

        cmd_queue.clear();
        let res = registry.execute(
            &dummy_audio,
            &dummy_physic,
            &mut cmd_queue,
            "physic.gravity -9.81",
        );
        assert!(res.contains("Gravity set to -9.81"));
        assert_eq!(
            cmd_queue,
            vec![EngineCommand::Physic(PhysicCommand::SetGravity(-9.81))]
        );

        cmd_queue.clear();
        let res = registry.execute(
            &dummy_audio,
            &dummy_physic,
            &mut cmd_queue,
            "renderer.reload_shaders",
        );
        assert!(res.contains("Reloading shaders"));
        assert_eq!(
            cmd_queue,
            vec![EngineCommand::Renderer(RendererCommand::ReloadShaders)]
        );
    }
}
