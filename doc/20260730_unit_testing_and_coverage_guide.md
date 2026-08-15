# 🧪 Guide des Tests Unitaires, Mocks et Couverture de Code

Ce document présente l'architecture des tests unitaires et d'intégration, les règles de conception des mocks, la sérialisation des threads de test pour les contextes graphiques GLFW/X11 et le flux de travail de couverture de code.

---

## 🏛️ 1. Architecture des Mocks & Rendu Headless

### 1.1 `DummyWindowEngine` (Mock Headless Complètement Isolé)

Le composant [`DummyWindowEngine`](../tests/helpers.rs) permet d'exécuter des tests d'intégration du simulateur en mode 100% headless (sans affichage X11/Wayland ni fenêtre GLFW physique).

#### Principes de Conception :
- **Absence de Fenêtre Physique** : Le mock n'ouvre aucune fenêtre GLFW réelle. Il instancie un récepteur d'événements `WindowEvents` non-bloquant.
- **Gestion des Événements** : `get_events(&self)` renvoie une référence valide vers `WindowEvents` (`crossbeam_channel::Receiver`), évitant toute erreur de type `Option::unwrap()` ou `expect()` en cours d'exécution.
- **Tolérance aux Pannes** : L'initialisation s'appuie sur une tentative avec gestion d'erreurs (`glfw::log_errors`), garantissant la stabilité sur des serveurs CI sans GPU.

```rust
// Extrait de tests/helpers.rs
pub struct DummyWindowEngine {
    pub events: WindowEvents,
}

impl Default for DummyWindowEngine {
    fn default() -> Self {
        let glfw = glfw::init(glfw::fail_on_errors)
            .ok()
            .or_else(|| glfw::init(glfw::log_errors).ok());

        let events = if let Some(mut g) = glfw {
            g.window_hint(glfw::WindowHint::Visible(false));
            g.create_window(1, 1, "dummy", glfw::WindowMode::Windowed)
                .map(|(_, rx)| rx)
        } else {
            None
        };

        let events = events.expect("DummyWindowEngine requires GLFW context for WindowEvents");
        Self { events }
    }
}
```

---

## 🎯 2. Qualité des Assertions & Isolation des Tests

### 2.1 Granularité des Suites de Tests
Chaque aspect du simulateur (`physic`, `renderer`, `audio`) fait l'objet de tests unitaires dédiés et isolés :
- **`test_simulator_with_dummy_engines`** : Vérifie l'instanciation de base du simulateur avec des composants factices (`DummyRenderer`, `DummyAudio`, `DummyPhysic`).
- **`test_renderer_called_by_simulator`** : Valide les appels du renderer (`render_frame`, `close`).
- **`test_audio_called_by_simulator`** : Valide le cycle d'arrêt du moteur audio (`audio.stop`).
- **`test_physic_called_by_simulator`** : Valide les étapes de mise à jour et de fermeture du moteur physique (`physic.update`, `physic.close`).
- **`test_call_order_in_simulator_run_and_close`** : Vérifie l'ordonnancement exact du cycle de vie.

### 2.2 Filtrage Positif (Whitelisting) vs Blacklisting Modèle
Les assertions sur les journaux d'appels des mocks n'utilisent **pas** de filtres d'exclusion négatifs fragiles (ex: `!s.starts_with(...)`), mais des filtres d'inclusions explicites (`matches!(*s, ...)`), évitant de masquer accidentellement des bugs d'effets de bord :

```rust
// Filtrage explicite (Whitelisting) recommandé dans tests/simulator_error_test.rs
let lifecycle_calls: Vec<&str> = calls
    .iter()
    .map(|s| s.as_str())
    .filter(|s| {
        matches!(
            *s,
            "audio.start"
                | "set_listener_position called"
                | "physic.update"
                | "renderer.render_frame"
                | "renderer.close"
                | "physic.close"
                | "audio.stop"
        )
    })
    .collect();
```

---

## 🧵 3. Sérialisation des Threads de Test (`--test-threads=1`)

### 3.1 Problématique Multithreading X11 / GLFW
La bibliothèque C GLFW sous-jacente gère des gestionnaires d'erreurs globaux non thread-safe sous X11 (`_glfwGrabErrorHandlerX11`). Lors du lancement parallèle des tests unitaires Rust par `cargo test`, l'initialisation simultanée de contextes GLFW sur plusieurs threads peut provoquer une assertion échec C (`Assertion _glfw.x11.errorHandler == NULL failed`) ou un segfault.

### 3.2 Solution Intégrée au `Taskfile.yml`
Toutes les tâches de test (`task test:all`, `task test:coverage`, `task ci:coverage`) sérialisent l'exécution via l'option Cargo `-- --test-threads=1` :

```yaml
  test:
    desc: "Lancement des tests unitaires"
    cmds:
      - echo "▶️  Lancement des tests unitaires..."
      - "{{.XVFB}} cargo test --all --quiet -- --test-threads=1"
```

---

## 📊 4. Rapport de Couverture de Code (`cargo-llvm-cov`)

Le projet utilise `cargo-llvm-cov` pour mesurer la couverture de code par région et ligne de code.

### 4.1 Génération du Rapport
Pour calculer la couverture de code locale et générer le rapport HTML :

```bash
task test:coverage
```

Le rapport est généré dans le répertoire `coverage/html/index.html`.

### 4.2 Tâche CI
En intégration continue, la commande suivante est exécutée :

```bash
task ci:coverage
```
