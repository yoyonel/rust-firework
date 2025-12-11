use copypasta::{ClipboardContext, ClipboardProvider};
use imgui::ClipboardBackend;

/// Backend clipboard Linux complet utilisant copypasta.
/// S’il échoue à lire ou écrire, il retourne simplement une chaîne vide
/// pour éviter tout crash dans ImGui ou dans GLFW.
pub struct LinuxClipboard {
    ctx: Option<ClipboardContext>,
}

impl Default for LinuxClipboard {
    fn default() -> Self {
        Self::new()
    }
}
impl LinuxClipboard {
    pub fn new() -> Self {
        // Tentative d'initialisation du clipboard systeme
        let ctx = ClipboardContext::new().ok();
        LinuxClipboard { ctx }
    }
}

impl ClipboardBackend for LinuxClipboard {
    fn get(&mut self) -> Option<String> {
        // Si copypasta ne peut pas lire (wayland sans permission, X11 absent, etc.),
        // on renvoie simplement une chaîne vide.
        if let Some(ctx) = &mut self.ctx {
            if let Ok(val) = ctx.get_contents() {
                return Some(val);
            }
        }
        Some(String::new()) // fallback safe
    }

    fn set(&mut self, text: &str) {
        // On ignore les erreurs pour empêcher les paniques
        if let Some(ctx) = &mut self.ctx {
            let _ = ctx.set_contents(text.to_owned());
        }
    }
}

/// Helper
/// -> Retourne toujours un type concret, comme attendu par imgui.
pub fn make_clipboard_backend() -> LinuxClipboard {
    LinuxClipboard::new()
}
