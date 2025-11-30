use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashMap;

use crate::AudioEngine;
use crate::PhysicEngine;

const INTERNAL_COMMANDS: &[&str] = &["clear", "help"];
const INPUT_BUFFER_GROWTH: usize = 256;
const SUGGESTION_BOX_HEIGHT: f32 = 80.0;
const NOISE_TEXTURE_SIZE: usize = 16;

pub struct HistoryCursor<'a> {
    history: &'a [String],
    // Optional: points to the currently displayed index. None = empty command line.
    current_index: Option<usize>,
}

impl<'a> HistoryCursor<'a> {
    // Creates the initial cursor
    pub fn new(history: &'a Vec<String>) -> Self {
        HistoryCursor {
            history: history.as_slice(),
            current_index: None,
        }
    }

    // Resets the cursor to the empty command line
    pub fn reset(&mut self) {
        self.current_index = None;
    }

    // Navigates to the older command (up arrow)
    pub fn prev(&mut self) -> Option<&'a str> {
        let max_index = self.history.len();
        if max_index == 0 {
            return None;
        }

        let new_index = match self.current_index {
            Some(i) => i.checked_sub(1),      // Go to previous element
            None => max_index.checked_sub(1), // Start at the last command
        };

        self.current_index = new_index;

        // Return safe reference
        new_index.map(|i| self.history[i].as_str())
    }

    // Navigates to the newer command (down arrow)
    pub fn next_recent(&mut self) -> Option<&'a str> {
        if self.history.is_empty() {
            return None;
        }

        let new_index = self.current_index.and_then(|i| i.checked_add(1));

        if let Some(i) = new_index {
            if i < self.history.len() {
                self.current_index = Some(i);
                return Some(self.history[i].as_str());
            }
        }

        // If we reach the end of history, return to empty command line
        self.current_index = None;
        None
    }
}

pub struct SelectionCycler<'a> {
    suggestions: &'a [String],
    current_index: usize, // Index is always a simple usize (for rotation)
}

impl<'a> SelectionCycler<'a> {
    // Creates the cycler. Must be re-instantiated every time the list changes.
    pub fn new(suggestions: &'a Vec<String>) -> Self {
        SelectionCycler {
            suggestions: suggestions.as_slice(),
            current_index: 0,
        }
    }

    // Returns the current index (used only for highlighting in ImGui render)
    pub fn get_index(&self) -> usize {
        self.current_index
    }

    // Returns the currently selected suggestion
    pub fn get_current(&self) -> Option<&'a str> {
        if self.suggestions.is_empty() {
            return None;
        }
        // Ensure safe read
        self.suggestions.get(self.current_index).map(|s| s.as_str())
    }

    // Used for completion
    pub fn next_cyclic(&mut self) -> Option<&'a str> {
        if self.suggestions.is_empty() {
            return None;
        }
        self.current_index = (self.current_index + 1) % self.suggestions.len();
        Some(self.suggestions[self.current_index].as_str())
    }
}

struct CombinedInputHandler<'a> {
    // Fields for HistoryHandler
    history: &'a Vec<String>,
    history_index: &'a mut Option<usize>,

    // For autocomplete
    suggestions: &'a Vec<String>,
    selected_suggestion_index: &'a mut usize,
}

impl<'a> imgui::InputTextCallbackHandler for CombinedInputHandler<'a> {
    // CHAR_FILTER LOGIC
    fn char_filter(&mut self, c: char) -> Option<char> {
        match c {
            '²' | '~' => None,
            other => Some(other),
        }
    }

    // COMPLETION_HANDLER LOGIC
    fn on_completion(&mut self, mut data: imgui::TextCallbackData) {
        if self.suggestions.is_empty() {
            return;
        }

        // Apply the currently selected suggestion to the input buffer
        let selected_suggestion = &self.suggestions[*self.selected_suggestion_index];

        // Clear current input
        let current_len = data.str().len();
        data.remove_chars(0, current_len);

        // Insert selected suggestion
        data.insert_chars(0, selected_suggestion);

        // Move to next suggestion for next TAB press (cycling behavior)
        let mut cycler = SelectionCycler::new(self.suggestions);
        cycler.current_index = *self.selected_suggestion_index;

        if cycler.next_cyclic().is_some() {
            *self.selected_suggestion_index = cycler.get_index();
        }
    }

    // HISTORY_HANDLER LOGIC
    fn on_history(
        &mut self,
        direction: imgui::HistoryDirection,
        mut data: imgui::TextCallbackData,
    ) {
        // 1. Instantiate Cursor and load current state
        let mut cursor = HistoryCursor::new(self.history);
        cursor.current_index = *self.history_index;

        let command_option = match direction {
            imgui::HistoryDirection::Up => cursor.prev(),
            imgui::HistoryDirection::Down => cursor.next_recent(),
        };

        // 2. Save updated state
        *self.history_index = cursor.current_index;

        // 3. Update ImGui buffer
        let current_len = data.str().len();
        data.remove_chars(0, current_len);

        if let Some(command) = command_option {
            data.insert_chars(0, command);
        }
    }
}

pub fn generate_noise_texture() -> u32 {
    let mut tex_id = 0;

    unsafe {
        gl::GenTextures(1, &mut tex_id);
        gl::BindTexture(gl::TEXTURE_2D, tex_id);

        let mut data = [0u8; NOISE_TEXTURE_SIZE * NOISE_TEXTURE_SIZE];

        for item in &mut data {
            *item = (rand::random::<f32>() * 255.0) as u8;
        }

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RED as i32,
            NOISE_TEXTURE_SIZE as i32,
            NOISE_TEXTURE_SIZE as i32,
            0,
            gl::RED,
            gl::UNSIGNED_BYTE,
            data.as_ptr() as *const _,
        );

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
    }

    tex_id
}

pub struct Console {
    pub open: bool,
    pub focus_previous_widget: bool,

    input: String,
    output: Vec<String>, // Display history

    // Background
    noise_tex: u32,

    // Scroll
    auto_scroll: bool,
    new_text_entered: bool,

    // Autocomplete
    autocomplete_suggestions: Vec<String>,
    selected_suggestion: usize,
    matcher: SkimMatcherV2,

    // History
    history: Vec<String>,         // Command history
    history_index: Option<usize>, // Current position in history

    window: Option<()>,
}

impl Default for Console {
    fn default() -> Self {
        Self::new()
    }
}

impl Console {
    pub fn new() -> Self {
        let noise_tex = generate_noise_texture();

        Self {
            open: false,
            input: String::new(),
            output: Vec::new(),
            focus_previous_widget: false,
            noise_tex,
            auto_scroll: true,
            new_text_entered: false,
            autocomplete_suggestions: Vec::new(),
            selected_suggestion: 0,
            matcher: SkimMatcherV2::default(),
            history: Vec::new(),
            history_index: None,
            window: None,
        }
    }

    pub fn log(&mut self, text: impl Into<String>) {
        self.output.push(text.into());
    }
}

impl Console {
    pub fn draw<P: PhysicEngine, A: AudioEngine>(
        &mut self,
        ui: &mut imgui::Ui,
        audio: &mut A,
        physic: &mut P,
        registry: &CommandRegistry,
    ) {
        if self.input.capacity() < INPUT_BUFFER_GROWTH {
            self.input
                .reserve(INPUT_BUFFER_GROWTH - self.input.capacity());
        }

        // Apply colors
        let _window_bg = ui.push_style_color(imgui::StyleColor::WindowBg, [0.08, 0.08, 0.08, 0.65]);
        let _child_bg = ui.push_style_color(imgui::StyleColor::ChildBg, [0.0, 0.0, 0.0, 0.0]);
        let _border = ui.push_style_color(imgui::StyleColor::Border, [0.0, 0.0, 0.0, 0.0]);
        let _text = ui.push_style_color(imgui::StyleColor::Text, [0.8, 0.8, 0.8, 1.0]);

        // Apply style variable
        let _rounding = ui.push_style_var(imgui::StyleVar::WindowRounding(0.0));

        let window_width = ui.io().display_size[0];
        let window_height = ui.io().display_size[1];
        let console_height = window_height * 0.50;

        self.window = ui
            .window("Console")
            .size([window_width, console_height], imgui::Condition::Always)
            .position([0.0, 0.0], imgui::Condition::Always)
            .movable(false)
            .resizable(true)
            .collapsible(false)
            .flags(imgui::WindowFlags::NO_TITLE_BAR | imgui::WindowFlags::NO_SCROLLBAR)
            .build(|| {
                let pos = ui.window_pos();
                let size = ui.window_size();

                // 1. Background Overlay
                self.draw_background_overlay(ui, pos, size);

                // 2. Scrolling Region
                self.draw_scrolling_region(ui);

                // 3. Suggestions Region
                self.draw_suggestions_region(ui);

                ui.separator();

                // 4. Input Bar & Interaction
                self.draw_input_bar(ui, audio, physic, registry);
            });
    }

    fn draw_background_overlay(&self, ui: &imgui::Ui, pos: [f32; 2], size: [f32; 2]) {
        let draw = ui.get_window_draw_list();
        draw.add_image(
            imgui::TextureId::new(self.noise_tex as usize),
            pos,
            [pos[0] + size[0], pos[1] + size[1]],
        )
        .uv_min([0.0, 0.0])
        .uv_max([size[0] / 12.0, size[1] / 12.0]) // repetition and upscale
        .col([1.0, 1.0, 1.0, 0.12]) // alpha = 12%
        .build();
    }

    fn draw_scrolling_region(&mut self, ui: &imgui::Ui) {
        let input_height = ui.frame_height_with_spacing();

        ui.child_window("scrolling")
            .size([0.0, -(input_height + SUGGESTION_BOX_HEIGHT)])
            .scroll_bar(true)
            .scrollable(true)
            .horizontal_scrollbar(false)
            .build(|| {
                // Display history
                for line in &self.output {
                    ui.text_wrapped(line);
                }

                // Handle user scroll
                let scroll_y = ui.scroll_y();
                let scroll_max_y = ui.scroll_max_y();

                // If user scrolls up -> disable autoscroll
                if self.auto_scroll && scroll_y < scroll_max_y {
                    self.auto_scroll = false;
                }

                // If user returns to bottom -> re-enable autoscroll
                if !self.auto_scroll && (scroll_max_y - scroll_y) < 1.0 {
                    self.auto_scroll = true;
                }

                // If new text entered -> enable autoscroll
                if !self.auto_scroll && self.new_text_entered {
                    self.auto_scroll = true;
                    self.new_text_entered = false;
                }

                // Apply autoscroll
                if self.auto_scroll {
                    ui.set_scroll_here_y();
                }
            });
    }

    fn draw_suggestions_region(&self, ui: &imgui::Ui) {
        ui.child_window("suggestions")
            .size([0.0, SUGGESTION_BOX_HEIGHT])
            .build(|| {
                if !self.autocomplete_suggestions.is_empty() {
                    ui.text("Suggestions:");
                    for (i, suggestion) in self.autocomplete_suggestions.iter().enumerate() {
                        if i == self.selected_suggestion {
                            ui.text_colored([1.0, 1.0, 0.0, 1.0], suggestion);
                        } else {
                            ui.text(suggestion);
                        }
                    }
                }
            });
    }

    fn draw_input_bar<P: PhysicEngine, A: AudioEngine>(
        &mut self,
        ui: &imgui::Ui,
        audio: &mut A,
        physic: &mut P,
        registry: &CommandRegistry,
    ) {
        // Instantiate combined handler
        let handler = CombinedInputHandler {
            history: &self.history,
            history_index: &mut self.history_index,
            suggestions: &self.autocomplete_suggestions,
            selected_suggestion_index: &mut self.selected_suggestion,
        };

        let input_modified = ui
            .input_text("##console_input", &mut self.input)
            .enter_returns_true(true)
            .flags(
                imgui::InputTextFlags::CALLBACK_HISTORY
                    | imgui::InputTextFlags::CALLBACK_COMPLETION
                    | imgui::InputTextFlags::CALLBACK_CHAR_FILTER,
            )
            .callback(imgui::InputTextCallback::COMPLETION, handler)
            .build();

        if self.focus_previous_widget {
            ui.set_keyboard_focus_here_with_offset(imgui::FocusedWidget::Previous);
        }

        // --- MANUAL KEY HANDLING (Anti-Spam and Autocomplete) ---
        let input_focused = ui.is_item_focused();

        // Update Autocomplete if text changed
        if input_modified {
            self.update_autocomplete(registry);
        }

        // Command Submission
        if ui.is_key_pressed(imgui::Key::Enter) && input_focused {
            self.handle_command_submission(audio, physic, registry);
        }
    }

    fn handle_command_submission<P: PhysicEngine, A: AudioEngine>(
        &mut self,
        audio: &mut A,
        physic: &mut P,
        registry: &CommandRegistry,
    ) {
        self.new_text_entered = true;

        let command = if !self.autocomplete_suggestions.is_empty() {
            // Use selected suggestion
            self.autocomplete_suggestions[self.selected_suggestion]
                .trim()
                .to_string()
        } else {
            // Use input
            self.input.trim().to_string()
        };

        // Abort if empty
        if command.is_empty() {
            return;
        }

        let result = self.execute_command(&command, audio, physic, registry);

        // Display and cleanup
        self.output.push(format!("> {}", command));
        if !result.is_empty() {
            self.output.push(result);
            self.history.push(command.to_string());
            self.history_index = None;
        }
        self.focus_previous_widget = true;
        self.input.clear();
        self.autocomplete_suggestions.clear();
    }

    fn execute_command<P: PhysicEngine, A: AudioEngine>(
        &mut self,
        input: &str,
        audio: &mut A,
        physic: &mut P,
        registry: &CommandRegistry,
    ) -> String {
        let trimmed_input = input.trim();

        // 1. Handle Internal Commands
        match trimmed_input {
            "clear" => {
                self.output.clear();
                return "".into();
            }
            "help" => {
                let available = registry.get_commands();

                let all_cmds = available
                    .iter()
                    .map(|s| s.as_str())
                    .chain(INTERNAL_COMMANDS.iter().cloned())
                    .collect::<Vec<&str>>()
                    .join(", ");

                self.output
                    .push(format!("Available commands: {}", all_cmds));
                return "".into();
            }
            _ => {}
        }

        // 2. Delegate to Registry
        registry.execute(audio, physic, trimmed_input)
    }
}

impl Console {
    fn update_autocomplete(&mut self, registry: &CommandRegistry) {
        if self.input.is_empty() {
            self.autocomplete_suggestions.clear();
            self.selected_suggestion = 0;
            return;
        }

        let input = self.input.trim();

        // Check if we're completing an argument (space after command name)
        if let Some(space_pos) = input.find(' ') {
            let cmd_name = &input[..space_pos];
            let arg_part = &input[space_pos + 1..];

            // Special handling for commands with known argument values
            let arg_suggestions: Vec<&str> = match cmd_name {
                "renderer.bloom.method" => vec!["gaussian", "kawase"],
                _ => vec![],
            };

            if !arg_suggestions.is_empty() {
                // Filter and score argument suggestions
                let mut scored_args: Vec<(i64, String)> = arg_suggestions
                    .iter()
                    .filter_map(|arg| {
                        self.matcher
                            .fuzzy_match(arg, arg_part)
                            .map(|score| (score, format!("{} {}", cmd_name, arg)))
                    })
                    .collect();

                // Sort by score descending
                scored_args.sort_by(|a, b| b.0.cmp(&a.0));

                self.autocomplete_suggestions = scored_args
                    .into_iter()
                    .map(|(_, suggestion)| suggestion)
                    .collect();

                self.selected_suggestion = 0;
                return;
            }
        }

        // Default: command name completion
        // 1. Collect ALL possible commands (Registry + Internal)
        let command_list_iter = registry
            .get_commands()
            .into_iter()
            .chain(INTERNAL_COMMANDS.iter().copied().map(String::from));

        // 2. Score and Filter (Fuzzy Match) using cached matcher
        let mut scored_suggestions: Vec<(i64, String)> = command_list_iter
            .filter_map(|cmd| {
                self.matcher
                    .fuzzy_match(&cmd, input)
                    .map(|score| (score, cmd))
            })
            .collect();

        // 3. Sort by Score (descending)
        scored_suggestions.sort_by(|a, b| b.0.cmp(&a.0));

        // 4. Update console suggestions
        self.autocomplete_suggestions =
            scored_suggestions.into_iter().map(|(_, cmd)| cmd).collect();

        // Reset selection index
        self.selected_suggestion = 0;
    }
}

type AudioCommandFn = dyn Fn(&mut dyn AudioEngine, &str) -> String + 'static;
type PhysicCommandFn = dyn Fn(&mut dyn PhysicEngine, &str) -> String + 'static;
type RendererCommandFn = dyn Fn(&str) -> String + 'static;

pub struct CommandRegistry {
    commands_audio: HashMap<String, Box<AudioCommandFn>>,
    commands_physic: HashMap<String, Box<PhysicCommandFn>>,
    commands_renderer: HashMap<String, Box<RendererCommandFn>>,
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
        }
    }

    pub fn register_for_audio<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&mut dyn AudioEngine, &str) -> String + 'static,
    {
        self.commands_audio.insert(name.to_string(), Box::new(func));
    }

    pub fn register_for_physic<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&mut dyn PhysicEngine, &str) -> String + 'static,
    {
        self.commands_physic
            .insert(name.to_string(), Box::new(func));
    }

    pub fn register_for_renderer<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&str) -> String + 'static,
    {
        self.commands_renderer
            .insert(name.to_string(), Box::new(func));
    }

    pub fn execute(
        &self,
        audio_engine: &mut dyn AudioEngine,
        physic_engine: &mut dyn PhysicEngine,
        input_str: &str,
    ) -> String {
        let input = input_str.trim();
        let cmd_name_with_args = input.split_whitespace().next().unwrap_or("");

        if cmd_name_with_args.is_empty() {
            return "".into();
        }

        // Try to split at the first dot. Example: "audio.mute" -> ("audio", "mute")
        let (prefix, _) = match cmd_name_with_args.split_once('.') {
            Some(pair) => pair,
            None => {
                return format!(
                    "Unknown command '{}'. Missing engine prefix.",
                    cmd_name_with_args
                )
            }
        };

        let cmd_key = cmd_name_with_args;

        match prefix {
            "audio" => {
                if let Some(func) = self.commands_audio.get(cmd_key) {
                    return func(audio_engine, input);
                }
            }
            "physic" => {
                if let Some(func) = self.commands_physic.get(cmd_key) {
                    return func(physic_engine, input);
                }
            }
            "renderer" => {
                if let Some(func) = self.commands_renderer.get(cmd_key) {
                    return func(input);
                }
            }
            _ => return format!("Unknown engine prefix '{}'.", prefix),
        }

        format!("Unknown command '{}'.", cmd_key)
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
