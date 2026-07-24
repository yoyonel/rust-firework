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

pub(crate) struct CombinedInputHandler<'a> {
    // Fields for HistoryHandler
    pub(crate) history: &'a Vec<String>,
    pub(crate) history_index: &'a mut Option<usize>,

    // For autocomplete
    pub(crate) suggestions: &'a Vec<String>,
    pub(crate) selected_suggestion_index: &'a mut usize,
}

impl<'a> imgui::InputTextCallbackHandler for CombinedInputHandler<'a> {
    // CHAR_FILTER LOGIC
    fn char_filter(&mut self, c: char) -> Option<char> {
        match c {
            '²' | '~' | '`' => None,
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
    if !gl::GenTextures::is_loaded() {
        return 0;
    }
    let mut tex_id = 0;

    unsafe {
        gl::GenTextures(1, &mut tex_id);
        gl::BindTexture(gl::TEXTURE_2D, tex_id);

        crate::label_gl_object!(gl::TEXTURE, tex_id, "Tex_Console_Noise_Overlay");

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
