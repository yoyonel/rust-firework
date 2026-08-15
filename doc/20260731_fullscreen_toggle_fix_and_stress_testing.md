# Correctif de Basculement Plein Écran & Harness de Stress Test

Ce document détaille le diagnostic, la cause racine, la résolution technique et le harness de stress test mis en place pour la bascule de mode fenêtre ↔ plein écran (`Fullscreen`) dans `rust-firework`.

---

## 1. Contexte & Symptômes

Lors du basculement rapide entre le mode fenêtré et le mode plein écran (via la touche F11 ou l'interface ImGui), l'application provoquait dans certains cas un crash brutal du serveur d'affichage X11 (`X connection to :0.0 broken`), entraînant une fermeture forcée de la session utilisateur MATE (reset du serveur X / Window Manager Marco).

---

## 2. Analyse Technique et Détection du Bug

L'analyse de l'architecture d'affichage et de l'intégration GLFW dans `rust-firework` a permis de révéler deux facteurs majeurs :

### A. Saturation des Reconfigurations Physiques RandR X11
* **Localisation** : `src/utils/glfw_window.rs`
* **Mécanisme** : L'implémentation de `set_fullscreen` passait explicitement la fréquence de rafraîchissement du moniteur principal (`Some(mode.refresh_rate)`).
* **Impact** : Sous X11 / GLFW, fournir un paramètre `refresh_rate` non nul lors de l'appel à `glfwSetWindowMonitor` demande au serveur d'affichage d'exécuter un changement de mode vidéo d'affichage physique via la sous-couche RandR. Des appels rapprochés ou répétés saturent les requêtes d'affichage du serveur Xorg et du gestionnaire de fenêtres (Marco), déclenchant le crash d'X11 et la déconnexion immédiate de la session utilisateur.

### B. Absence de Synchronization de la Pipeline GPU
* **Localisation** : `src/simulator/events.rs`
* **Impact** : Le basculement de moniteur s'exécutait alors que des commandes de rendu GL et des framebuffers (Bloom MRT, pipelines de particules) étaient encore en cours de traitement sur le GPU.

---

## 3. Correctifs Appliqués

1. **Suppression du Forçage RandR en Plein Écran (`src/utils/glfw_window.rs`)** :
   Le paramètre `refresh_rate` est désormais positionné à `None` lors de l'appel à `set_monitor`. GLFW utilise ainsi le mode plein écran bureau natif sans demander de changement de fréquence RandR, éliminant totalement les resets du serveur X11.

   ```rust
   fn set_fullscreen(&mut self, monitor: &Monitor) {
       if let Some(mode) = monitor.get_video_mode() {
           self.set_monitor(
               WindowMode::FullScreen(monitor),
               0,
               0,
               mode.width,
               mode.height,
               None, // Préserve le mode vidéo d'affichage X11 sans forcer RandR
           );
       }
   }
   ```

2. **Flushing Synchrone GPU (`src/simulator/events.rs`)** :
   Ajout de `unsafe { gl::Finish(); }` au début de la méthode `toggle_fullscreen` afin d'assurer que la pipeline OpenGL soit totalement vidée avant que GLFW ne reconfigure la surface de la fenêtre.

---

## 4. Intégration du Harness de Stress Test

Un script dédié a été intégré pour valider la stabilité sous des cycles agressifs de basculement :

* **Script Shell** : `scripts/test_stress_fullscreen_rust.sh`
* **Sauvegarde Atomique** : Le script journalise en temps réel la progression dans `stress_progress.log` après chaque itération afin d'assurer la persistance des données sur disque même en cas d'interruption brutale.
* **Tâche Taskfile** : Ajout de la cible `task test:stress-fullscreen` exécutant 50 cycles de basculement avec un intervalle de 100 ms.

### Résultats de Validation
- **Cycles exécutés** : 50 / 50
- **Stabilité X11 / Session** : 0 crash, 0 deconnexion MATE
- **Temps total** : 14s

```
╔══════════════════════════════════════════════════════════════╗
║             RUST-FIREWORK STRESS TEST RESULTS               ║
╠══════════════════════════════════════════════════════════════╣
║ Toggles completed: 50 / 50
║ Hangs detected:    0
║ Crash detected:    NO
║ Total time:        14s
╚══════════════════════════════════════════════════════════════╝
```
