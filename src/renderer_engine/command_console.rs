use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashMap;

use crate::AudioEngine;
use crate::PhysicEngine;

const INTERNAL_COMMANDS: &[&str] = &["clear", "help"];

pub struct HistoryCursor<'a> {
    history: &'a [String],
    // Optionnel: pointe vers l'index actuellement affiché. None = ligne de commande vide.
    current_index: Option<usize>,
}

impl<'a> HistoryCursor<'a> {
    // Crée le curseur initial
    pub fn new(history: &'a Vec<String>) -> Self {
        HistoryCursor {
            history: history.as_slice(),
            current_index: None,
        }
    }

    // Réinitialise le curseur à la ligne de commande vide
    pub fn reset(&mut self) {
        self.current_index = None;
    }

    // Navigue vers la commande plus ancienne (flèche haut)
    pub fn prev(&mut self) -> Option<&'a str> {
        let max_index = self.history.len();
        if max_index == 0 {
            return None;
        }

        let new_index = match self.current_index {
            Some(i) => i.checked_sub(1),      // Va à l'élément précédent
            None => max_index.checked_sub(1), // Commence à la dernière commande
        };

        self.current_index = new_index;

        // Retourne la référence sécurisée
        new_index.map(|i| self.history[i].as_str())
    }

    // Navigue vers la commande plus récente (flèche bas)
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

        // Si on atteint la fin de l'historique, on revient à la ligne de commande vide
        self.current_index = None;
        None
    }
}

pub struct SelectionCycler<'a> {
    suggestions: &'a [String],
    current_index: usize, // L'index est toujours un usize simple (pour la rotation)
}

impl<'a> SelectionCycler<'a> {
    // Crée le cycler. Il doit être ré-instancié chaque fois que la liste change.
    pub fn new(suggestions: &'a Vec<String>) -> Self {
        SelectionCycler {
            suggestions: suggestions.as_slice(),
            current_index: 0,
        }
    }

    // Retourne l'index actuel (utilisé uniquement pour le surlignage dans le rendu ImGui)
    pub fn get_index(&self) -> usize {
        self.current_index
    }

    // Retourne la suggestion actuellement sélectionnée
    pub fn get_current(&self) -> Option<&'a str> {
        if self.suggestions.is_empty() {
            return None;
        }
        // Assure une lecture sécurisée
        self.suggestions.get(self.current_index).map(|s| s.as_str())
    }

    // utilisé pour la completion
    pub fn next_cyclic(&mut self) -> Option<&'a str> {
        if self.suggestions.is_empty() {
            return None;
        }
        self.current_index = (self.current_index + 1) % self.suggestions.len();
        Some(self.suggestions[self.current_index].as_str())
    }
}

struct CombinedInputHandler<'a> {
    // Les champs pour l'HistoryHandler
    history: &'a Vec<String>,
    history_index: &'a mut Option<usize>,

    // pour l'autocomplétion
    suggestions: &'a Vec<String>,
    selected_suggestion_index: &'a mut usize,
}

impl<'a> imgui::InputTextCallbackHandler for CombinedInputHandler<'a> {
    // LOGIQUE DU CHAR_FILTER
    fn char_filter(&mut self, c: char) -> Option<char> {
        // Logique de CharFilter
        match c {
            '²' | '~' => None,
            other => Some(other),
        }
    }

    // LOGIQUE DU COMPLETION_HANDLER
    fn on_completion(&mut self, mut _data: imgui::TextCallbackData) {
        if self.suggestions.is_empty() {
            return;
        }

        // 1. Instancier le Cycler et charger l'état actuel
        let mut cycler = SelectionCycler::new(self.suggestions);
        cycler.current_index = *self.selected_suggestion_index;

        // 2. Exécuter l'action idiomatique (rotation)
        if cycler.next_cyclic().is_some() {
            // 3. Sauvegarder l'état mis à jour
            *self.selected_suggestion_index = cycler.get_index();
        }

        // Rappel: Ici, nous faisons UNIQUEMENT la rotation de l'index.
        // L'application du texte est gérée par la touche ENTER ou TAB dans Console::draw.
    }

    // LOGIQUE DU HISTORY_HANDLER
    fn on_history(
        &mut self,
        direction: imgui::HistoryDirection,
        mut data: imgui::TextCallbackData,
    ) {
        // 1. Instancier le Cursor et charger l'état actuel
        let mut cursor = HistoryCursor::new(self.history);
        cursor.current_index = *self.history_index;

        let command_option = match direction {
            imgui::HistoryDirection::Up => cursor.prev(),
            imgui::HistoryDirection::Down => cursor.next_recent(),
        };

        // 2. Sauvegarder l'état mis à jour
        *self.history_index = cursor.current_index;

        // 3. Mettre à jour le buffer ImGui
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

        const SIZE: usize = 16;
        let mut data = [0u8; SIZE * SIZE];

        for item in &mut data {
            *item = (rand::random::<f32>() * 255.0) as u8;
        }

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RED as i32,
            SIZE as i32,
            SIZE as i32,
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
    output: Vec<String>, // historique d’affichage

    // Background
    noise_tex: u32,

    // Scroll
    auto_scroll: bool,
    new_text_entered: bool,

    // Autocomplétion
    autocomplete_suggestions: Vec<String>,
    selected_suggestion: usize,

    // Historique
    history: Vec<String>,         // historique des commandes
    history_index: Option<usize>, // position actuelle dans l'historique

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
        if self.input.capacity() < 256 {
            self.input.reserve(256 - self.input.capacity());
        }

        // Appliquer les couleurs
        let _window_bg = ui.push_style_color(imgui::StyleColor::WindowBg, [0.08, 0.08, 0.08, 0.65]);
        let _child_bg = ui.push_style_color(imgui::StyleColor::ChildBg, [0.0, 0.0, 0.0, 0.0]);
        let _border = ui.push_style_color(imgui::StyleColor::Border, [0.0, 0.0, 0.0, 0.0]);
        let _text = ui.push_style_color(imgui::StyleColor::Text, [0.8, 0.8, 0.8, 1.0]);

        // Appliquer une variable de style
        let _rounding = ui.push_style_var(imgui::StyleVar::WindowRounding(0.0));

        let window_width = ui.io().display_size[0];
        let window_height = ui.io().display_size[1];
        let console_height = window_height * 0.50;

        let mut command = String::new();

        self.window = ui
            .window("Console")
            .size([window_width, console_height], imgui::Condition::Always)
            .position([0.0, 0.0], imgui::Condition::Always)
            .movable(false)
            .resizable(true)
            .collapsible(false)
            .flags(imgui::WindowFlags::NO_TITLE_BAR | imgui::WindowFlags::NO_SCROLLBAR)
            .build(|| {
                let draw = ui.get_window_draw_list();

                let pos = ui.window_pos();
                let size = ui.window_size();

                let input_height = ui.frame_height_with_spacing();
                let suggestion_height = 80.0;

                // 1. Instanciation du Cycler pour la logique manuelle
                let mut cycler = SelectionCycler::new(&self.autocomplete_suggestions);
                cycler.current_index = self.selected_suggestion;

                // Instanciation du handler combiné (avec les références)
                let handler = CombinedInputHandler {
                    history: &self.history,
                    history_index: &mut self.history_index,
                    suggestions: &self.autocomplete_suggestions,
                    selected_suggestion_index: &mut self.selected_suggestion,
                };

                // --- 1. Overlay noise (effet de faux blur) ---
                draw.add_image(
                    imgui::TextureId::new(self.noise_tex as usize),
                    pos,
                    [pos[0] + size[0], pos[1] + size[1]],
                )
                .uv_min([0.0, 0.0])
                .uv_max([size[0] / 12.0, size[1] / 12.0]) // répétition et upscale
                .col([1.0, 1.0, 1.0, 0.12]) // alpha = 12%
                .build();

                ui.child_window("scrolling")
                    // .size([0.0, -30.0])
                    .size([0.0, -(input_height + suggestion_height)])
                    .scroll_bar(true)
                    .scrollable(true)
                    .horizontal_scrollbar(false)
                    .build(|| {
                        // Affichage historique
                        for line in &self.output {
                            ui.text_wrapped(line);
                        }

                        // Gestion du scroll utilisateur
                        let scroll_y = ui.scroll_y();
                        let scroll_max_y = ui.scroll_max_y();

                        // Si l’utilisateur scrolle vers le haut → désactiver autoscroll
                        if self.auto_scroll && scroll_y < scroll_max_y {
                            self.auto_scroll = false;
                        }

                        // Si l’utilisateur revient en bas → réactiver autoscroll
                        if !self.auto_scroll && (scroll_max_y - scroll_y) < 1.0 {
                            self.auto_scroll = true;
                        }

                        // Si un nouveau texte est entré -> activer l'autoscroll
                        if !self.auto_scroll && self.new_text_entered {
                            self.auto_scroll = true;
                            self.new_text_entered = false;
                        }

                        // 3. Autosroll si activé
                        if self.auto_scroll {
                            ui.set_scroll_here_y();
                        }
                    });

                // Suggestions en dessous
                ui.child_window("suggestions")
                    .size([0.0, suggestion_height])
                    .build(|| {
                        if !self.autocomplete_suggestions.is_empty() {
                            ui.text("Suggestions:");
                            for (i, suggestion) in self.autocomplete_suggestions.iter().enumerate()
                            {
                                if i == cycler.current_index {
                                    ui.text_colored([1.0, 1.0, 0.0, 1.0], suggestion);
                                } else {
                                    ui.text(suggestion);
                                }
                            }
                        }
                    });

                // Barre d'entrée
                ui.separator();

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

                // --- GESTION MANUELLE DES TOUCHES (Anti-Spam et Autocomplétion) ---
                let input_focused = ui.is_item_focused();

                // 2. Mise à jour de l'Autocomplétion si le texte a changé
                // C'est ici que nous évitons la réinitialisation après TAB.
                if input_modified {
                    self.update_autocomplete(registry);
                }

                // 3. Soumission de commande (Anti-Spam)
                if ui.is_key_pressed(imgui::Key::Enter) && input_focused {
                    self.new_text_entered = true;

                    command = if !self.autocomplete_suggestions.is_empty() {
                        // Clone de la suggestion sélectionnée
                        self.autocomplete_suggestions[self.selected_suggestion]
                            .trim()
                            .to_string()
                    } else {
                        // Clone de l'input
                        self.input.trim().to_string()
                    };

                    // si au final aucune commande par input ou suggestion => on abort
                    if command.is_empty() {
                        return;
                    }

                    let result = self.execute_command(&command, audio, physic, registry); // Note: on passe &command (un &str)

                    // Affichage et nettoyage
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
            });
    }

    fn execute_command<P: PhysicEngine, A: AudioEngine>(
        &mut self,
        input: &str,
        audio: &mut A,
        physic: &mut P,
        registry: &CommandRegistry,
    ) -> String {
        let trimmed_input = input.trim();

        // 1. Gestion des commandes internes (état de la console)
        match trimmed_input {
            "clear" => {
                self.output.clear();
                return "".into();
            }
            "help" => {
                // 1. Obtenir les commandes externes (Vec<String>)
                let available = registry.get_commands();

                let all_cmds = available
                    .iter()
                    // MAP: Convertit l'élément de &String à &str
                    .map(|s| s.as_str())
                    // CHAIN: Maintenant, Item = &str pour les deux itérateurs !
                    .chain(INTERNAL_COMMANDS.iter().cloned()) // .cloned() est nécessaire car INTERNAL_COMMANDS.iter() donne &&str.
                    // L'itérateur résultant produit des &str
                    .collect::<Vec<&str>>() // On collecte en Vec<&str> temporaire
                    .join(", "); // Puis on joint en String

                self.output
                    .push(format!("Available commands: {}", all_cmds));
                return "".into();
            }
            _ => {} // Ne rien faire
        }

        // 2. Délégation...
        // registry.execute se charge maintenant de la coercition (A -> dyn AudioEngine)
        // et du routage.
        registry.execute(audio, physic, trimmed_input)
    }
}

impl Console {
    fn update_autocomplete(&mut self, registry: &CommandRegistry) {
        if self.input.is_empty() {
            self.autocomplete_suggestions.clear();
            self.selected_suggestion = 0; // Réinitialiser
            return;
        }

        // --- NOUVEAU : Initialisation du Matcher ---
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();
        let input = self.input.trim();

        // 1. Collecter TOUTES les commandes possibles (Registry + Internes)
        let command_list_iter = registry
            .get_commands()
            .into_iter() // on consomme le Vec<String> du registre
            .chain(INTERNAL_COMMANDS.iter().copied().map(String::from)); // Chainer avec les commandes internes (converties en String)

        // 2. Scorer et Filtrer (Fuzzy Match)
        let mut scored_suggestions: Vec<(i64, String)> = command_list_iter
            .filter_map(|cmd| {
                // Calcule un score de ressemblance floue (i64)
                matcher.fuzzy_match(&cmd, input).map(|score| (score, cmd))
            })
            .collect();

        // 3. Trier par Score (du plus haut au plus bas)
        // Les scores les plus élevés (meilleures correspondances) doivent être en tête de liste.
        scored_suggestions.sort_by(|a, b| b.0.cmp(&a.0));

        // 4. Mettre à jour les suggestions de la console
        self.autocomplete_suggestions =
            scored_suggestions.into_iter().map(|(_, cmd)| cmd).collect();

        // Réinitialiser l'index de sélection
        self.selected_suggestion = 0;
    }
}

type AudioCommandFn = dyn Fn(&mut dyn AudioEngine, &str) -> String + 'static;
type PhysicCommandFn = dyn Fn(&mut dyn PhysicEngine, &str) -> String + 'static;

pub struct CommandRegistry {
    commands_audio: HashMap<String, Box<AudioCommandFn>>,
    commands_physic: HashMap<String, Box<PhysicCommandFn>>,
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

    pub fn execute(
        &self,
        audio_engine: &mut dyn AudioEngine,
        physic_engine: &mut dyn PhysicEngine,
        input: &str,
    ) -> String {
        let input = input.trim();
        let cmd_name_with_args = input.split_whitespace().next().unwrap_or("");

        if cmd_name_with_args.is_empty() {
            return "".into();
        }

        // Tente de diviser au premier point. Exemple: "audio.mute" -> ("audio", "mute")
        let (prefix, _) = match cmd_name_with_args.split_once('.') {
            Some(pair) => pair, // Utilise la commande complète comme clé (ex: "audio.mute")
            None => {
                return format!(
                    "Unknown command '{}'. Missing engine prefix.",
                    cmd_name_with_args
                )
            }
        };

        // La clé de la HashMap est le nom complet de la commande sans arguments (ex: "audio.mute")
        let cmd_key = cmd_name_with_args; // Pas besoin de le reconstruire

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
            _ => return format!("Unknown engine prefix '{}'.", prefix),
        }

        format!("Unknown command '{}'.", cmd_key)
    }

    pub fn get_commands(&self) -> Vec<String> {
        self.commands_audio
            .keys()
            .cloned()
            .chain(self.commands_physic.keys().cloned())
            .collect()
    }
}
