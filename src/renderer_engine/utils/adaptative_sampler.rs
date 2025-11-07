use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::time::{Duration, Instant};

/// ----------------------------------------------------------------------------
/// # AdaptiveSampler
///
/// ## Objectif
/// Maintenir un échantillonnage aléatoire mais régulier de mesures (ici du FPS)
/// sur une période de temps fixe (`window`), sans connaître à l’avance le nombre
/// total de frames qui seront générées pendant cette période.
///
/// ## Principe mathématique
/// Cette méthode repose sur une **loi de Bernoulli adaptative** :
///
/// - À chaque frame `i`, on évalue une probabilité `p_i` de prendre un échantillon.
/// - Cette probabilité est calculée de manière à ce que **l’espérance** du nombre
///   total de samples capturés à la fin de la fenêtre soit ≈ `target_samples`.
///
/// En notant :
///   - `N(t)` : nombre de frames observées jusqu’au temps `t`
///   - `dt̄` : estimation glissante du temps moyen entre frames (→ FPS estimé)
///   - `T_left` : durée restante avant la fin de la fenêtre
///   - `S_left` : nombre de samples restant à prendre = `target - taken`
///
/// Alors on définit :
///
/// ```text
///     p = S_left / E[frames_remaining]
///       ≈ (target - taken) / (T_left / dt̄)
/// ```
///
/// Cette équation donne une probabilité d’échantillonnage **adaptative** :
///
/// - si le FPS baisse (moins de frames restantes prévues), `dt̄` ↑ ⇒ `p` ↑
///   → on prend plus souvent des samples pour ne pas en manquer.
/// - si le FPS augmente, `dt̄` ↓ ⇒ `p` ↓
///   → on espace naturellement les prises.
/// - ainsi, l’espérance mathématique du nombre total de samples tend vers `target_samples`.
///
/// Statistiquement, cette méthode se rapproche d’un **processus de Poisson homogène**
/// mais avec un taux dynamique estimé en ligne. Elle permet une répartition quasi-uniforme
/// des samples dans le temps, sans connaître le futur.
///
/// ## Qualité du générateur aléatoire
/// - `SmallRng` (PCG32) est rapide et a d’excellentes propriétés statistiques :
///   pas de corrélation perceptible sur quelques milliers de frames,
///   distribution uniforme sur [0,1), période très longue (~2¹²⁸).
/// - Pour reproductibilité, on peut utiliser `seed_from_u64(seed)`
///   sinon, on initialise avec `thread_rng()` pour un bruit système sûr.
/// ----------------------------------------------------------------------------
pub struct AdaptiveSampler {
    /// Générateur pseudo-aléatoire local
    rng: SmallRng,

    /// Durée totale de la fenêtre d’échantillonnage (ex: 5s)
    window: Duration,

    /// Nombre de samples souhaités sur la fenêtre
    pub target_samples: usize,

    /// Nombre de samples déjà pris dans la fenêtre courante
    samples_taken: usize,

    /// Instant de début de la fenêtre actuelle
    window_start: Instant,

    /// Estimation glissante du temps moyen entre frames (dt moyen)
    /// Sert à estimer combien de frames on verra dans le temps restant
    avg_dt: f32,

    /// Facteur de lissage exponentiel (EMA) pour avg_dt (plus petit = plus stable)
    alpha: f32,

    /// Historique des samples capturés :
    /// (temps écoulé depuis le début de la fenêtre, fps mesuré)
    pub samples: Vec<(f32, f32)>,
}

impl AdaptiveSampler {
    /// Crée un nouveau sampler adaptatif
    ///
    /// - `window`: durée totale de la fenêtre d’échantillonnage (ex: 5s)
    /// - `target_samples`: nombre de samples visés (ex: 200)
    /// - `initial_fps_guess`: estimation initiale du FPS pour initier avg_dt
    pub fn new(window: Duration, target_samples: usize, initial_fps_guess: f32) -> Self {
        Self {
            // 🔹 Initialisation du SmallRng :
            // - rapide, statistiquement fiable
            // - seedée via rand::rng() pour avoir un bruit système
            rng: SmallRng::from_rng(&mut rand::rng()),
            window,
            target_samples,
            samples_taken: 0,
            window_start: Instant::now(),
            avg_dt: 1.0 / initial_fps_guess,
            alpha: 0.15, // pondération pour la moyenne glissante (EMA)
            samples: Vec::with_capacity(target_samples),
        }
    }

    /// Détermine s’il faut échantillonner à cette frame
    ///
    /// Retourne `true` si un échantillon doit être pris.
    ///
    /// Principe :
    /// - met à jour la moyenne glissante du dt (EMA)
    /// - estime le nombre de frames restantes avant la fin de la fenêtre
    /// - calcule la probabilité `p` de prendre un sample
    /// - tire un nombre aléatoire uniformément dans [0,1) → Bernoulli(p)
    pub fn should_sample(&mut self, dt: f32) -> bool {
        // 🔹 Mise à jour de l’estimation du temps moyen entre frames
        // Formule d’EMA : avg_dt ← α·dt + (1−α)·avg_dt
        self.avg_dt = self.alpha * dt + (1.0 - self.alpha) * self.avg_dt;

        // 🔹 Temps écoulé depuis le début de la fenêtre
        let elapsed = self.window_start.elapsed();

        // Si la fenêtre est terminée, on arrête de prendre des samples
        if elapsed >= self.window {
            return false;
        }

        // 🔹 Temps restant avant la fin de la fenêtre
        let t_left = (self.window - elapsed).as_secs_f32().max(0.001);

        // 🔹 Nombre de frames restantes estimées :
        //    E[frames_remaining] ≈ T_left / avg_dt
        let expected_frames_left = (t_left / self.avg_dt).max(1.0);

        // 🔹 Samples restants à capturer
        let remaining = self.target_samples.saturating_sub(self.samples_taken);

        // 🔹 Probabilité d’échantillonnage adaptative
        //    p = remaining / expected_frames_left
        let mut p = remaining as f32 / expected_frames_left;
        if p > 1.0 {
            p = 1.0; // borne pour éviter p > 1
        }

        // 🔹 Tirage Bernoulli(p) via SmallRng
        let take = self.rng.random::<f32>() < p;

        // 🔹 Si on décide de prendre un sample, on l’enregistre
        if take {
            self.samples_taken += 1;
            self.samples
                .push((elapsed.as_secs_f32(), 1.0 / dt.max(0.00001))); // (temps, FPS instantané)
        }

        take
    }

    /// Réinitialise le sampler pour une nouvelle fenêtre
    ///
    /// - Réinitialise le compteur de samples
    /// - Vide le vecteur d’échantillons
    /// - Redémarre le chrono de la fenêtre
    pub fn reset(&mut self) {
        self.samples_taken = 0;
        self.samples.clear();
        self.window_start = Instant::now();
    }
}

pub fn ascii_sample_timeline(
    samples: &[(f32, f32)], // (timestamp, fps)
    window_secs: f32,
    width: usize,
    avg_fps: f32,
) -> String {
    let mut line = vec!['.'; width];

    for &(t, fps) in samples {
        let pos = ((t / window_secs) * (width as f32 - 1.0)).round() as usize;
        if pos < width {
            // Choisir caractère selon position relative à la moyenne
            let ch = if fps > avg_fps * 1.05 {
                '+' // au-dessus de la moyenne
            } else if fps < avg_fps * 0.95 {
                '-' // en dessous
            } else {
                '#' // dans ±5%
            };
            line[pos] = ch;
        }
    }

    format!(
        "[0s{}5s]\n|{}|",
        ".".repeat(width.saturating_sub(4)),
        line.into_iter().collect::<String>()
    )
}
