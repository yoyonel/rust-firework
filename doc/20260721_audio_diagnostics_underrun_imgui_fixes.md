# Résolution des Dysfonctionnements Audio & Optimisations Graphiques du Rendu Diagnostic (21 Juillet 2026)

Ce document récapitule les dysfonctionnements identifiés lors des phases d'évaluation du stress-test audio (128 à 1024 sources) et détaille les solutions techniques implémentées pour résoudre les problèmes d'underruns ALSA, de crash d'indexation ImGui, et de violation d'accès mémoire (Segfault) à la fermeture de l'application.

---

## 🎧 1. Correction du Référentiel et Panning Stéréo
* **Problème** : Lors de la simulation, toutes les sources sonores mobiles semblaient confinées à l'oreille gauche de l'auditeur.
* **Cause** : Le moteur physique transmettait les positions absolues des sources à l'écran, tandis que le simulateur calculait la distance par rapport à l'auditeur en soumettant un vecteur déjà relatif, ce qui décalait artificiellement l'origine des coordonnées de spatialisation vers le coin inférieur gauche `(0, 0)`.
* **Résolution** : Les positions absolues à l'écran `source.pos` sont désormais transmises directement à l'audio-engine. L'alignement géométrique spatialisé et le panning (ITD/ILD) sont parfaitement équilibrés et réactifs gauche/droite.

---

## 🔊 2. Éradication des Underruns ALSA (`snd_pcm_recover`)
* **Problème** : Lors de l'explication en mode release, le terminal était inondé d'erreurs `ALSA lib pcm.c:8772:(snd_pcm_recover) underrun occurred`, altérant la qualité sonore.
* **Causes identifiées** :
  1. **Surcharge de messages Doppler** : À 500+ FPS (sans VSync), le moteur physique émettait des mises à jour `DopplerEvent` à chaque tick graphique, saturant le canal de communication inter-thread et le cache CPU du thread audio.
  2. **Single-Buffering Matériel** : La négociation CPAL imposait une taille fixe de buffer matériel à `Fixed(256)` échantillons. En l'absence de marge de sécurité, tout décalage d'ordonnancement de 2 à 3 ms (provoqué par les threads de pilotes graphiques ou le compositeur Wayland/X11 à 500 FPS) entraînait une famine de la carte son.
* **Résolutions implémentées** :
  * **Throttling Doppler à 144 Hz** : Ajout d'un limiteur temporel dans `src/physic_engine/physic_engine_generational_arena.rs` pour plafonner l'envoi des événements Doppler à un intervalle minimal de `6.94 ms` (144 Hz), divisant par 5 le trafic inter-thread.
  * **Multi-Buffering Matériel via BufferSize::Default** : Modification de `get_cpal_config` dans `src/audio_engine/fireworks_audio.rs`. En utilisant le buffer par défaut du système, le serveur audio (PipeWire/PulseAudio/ALSA) alloue un tampon matériel multi-période (ex: 1024 ou 2048 échantillons) offrant une marge de sécurité temporelle robuste face aux pics d'ordonnancement du GPU, sans augmenter la latence ressentie.

---

## 🎨 3. Rendu GPU Instancié des Cercles de Diagnostic (Bypass d'ImGui)
* **Problème** : Lors de l'exécution avec plus de 384 sources actives (ex: `cargo run --release -- --audio-stress-scene 384`), l'application plantait immédiatement avec l'erreur :
  `Assertion 'draw_list->_VtxCurrentIdx < (1 << 16) && "Too many vertices in ImDrawList using 16-bit indices..."' failed.`
* **Cause** : Le tracé des orbites et des sources était géré par le CPU d'ImGui (`draw_list.add_circle`), générant plus de 36 000 sommets (vertices) complexes. ImGui utilisant des index 16 bits non signés, la limite de 65 536 indices par draw list était instantanément dépassée.
* **Résolution (CircleGPURenderer)** :
  * Création d'un module de rendu GPU dédié : [**`src/renderer_engine/circle_renderer.rs`**](../src/renderer_engine/circle_renderer.rs).
  * Confection de shaders GLSL dédiés : [**`circle.vert.glsl`**](../assets/shaders/circle.vert.glsl) et [**`circle.frag.glsl`**](../assets/shaders/circle.frag.glsl). Les cercles sont dessinés à partir de quads instanciés (4 sommets seulement), le fragment shader calculant l'équation mathématique du disque (`length(UV)`) en temps réel sur le GPU.
  * **Isolation de la machine d'état** : Les états `GL_DEPTH_TEST` et `GL_CULL_FACE` sont explicitement sauvegardés, désactivés (pour éviter le masquage par la passe de composition HDR/Bloom précédente) puis restaurés après l'appel unique de dessin instancié (`glDrawArraysInstanced`).

---

## ⚡ 4. Élimination du Segfault à l'Arrêt
* **Problème** : L'application plantait avec un `Segmentation fault` lors de sa fermeture.
* **Cause** : À l'arrêt du programme, les champs de la structure `Simulator` sont détruits dans leur ordre de déclaration. La fenêtre GLFW (`window_engine`) étant détruite avant le `circle_renderer`, le destructeur (`Drop`) de ce dernier tentait d'appeler `glDeleteVertexArrays` et `glDeleteBuffers` sur un contexte OpenGL déjà libéré, provoquant un plantage du driver graphique.
* **Résolution** :
  1. Rendue la méthode `destroy()` du `CircleGPURenderer` idempotente (vérifie si les IDs sont non nuls et les remet à zéro).
  2. Intégré un nettoyage explicite dans la méthode [`close()` du `Simulator`](../src/simulator.rs#L1377) en utilisant `self.circle_renderer.take()`. Les ressources GPU sont ainsi proprement libérées **pendant** que le contexte OpenGL de GLFW est encore valide et actif.

---

## ⏱️ 5. Contrôle du Frame-Rate (VSync) & Déterminisme des Trajectoires en Mode Stress-Test (22 Juillet 2026)

* **Problèmes identifiés** :
  1. **Frame-Rate plafonné ou instable** : Le framerate restait bloqué à la fréquence de rafraîchissement de l'écran (VSync active par défaut), faussant les mesures de performance pure du moteur de rendu.
  2. **Trajectoires aléatoires sur recyclage** : Les sources virtuelles du stress-test audio changeaient d'orbite aléatoirement à chaque fois que leur cycle sonore (lancement -> explosion) se terminait, empêchant de visualiser et d'étudier de manière stable des trajectoires de référence prédéterminées.
  3. **Dérive de trajectoire continue** : Même au cours de leur mouvement, les sources virtuelles modifiaient continuellement leur rayon cible de manière aléatoire dès qu'elles s'approchaient de leur destination temporaire, empêchant toute stabilité géométrique.

* **Résolutions implémentées** :
  * **Désactivation matérielle de la VSync** : Ajout de la configuration explicite `glfw.set_swap_interval(glfw::SwapInterval::None);` dans [`glfw_window_engine.rs`](../src/window_engine/glfw_window_engine.rs) pour forcer le driver et le compositeur graphique à ignorer le rafraîchissement vertical, ce qui a débloqué le framerate au-delà de 400 Hz.
  * **Mise en cache de l'état initial des sources** : Extension de la structure `VirtualSource` avec des champs `initial_*` mémorisant les angles, rayons et vitesses de spawn à l'initialisation.
  * **Oscillation et Relance Déterministes par Défaut** :
    * Lors du cycle de vie sonore `Explosion -> Rocket`, si l'option de randomisation n'est pas demandée, la fusée ne subit aucun repositionnement ou saut spatial, ce qui maintient sa trajectoire d'origine de façon fluide.
    * Au cours de la mise à jour de la physique dans `update_audio_stress_simulation`, si la source s'approche de son rayon cible, elle n'effectue plus de tirage aléatoire mais oscille de manière strictement déterministe entre ses rayons initial et cible d'origine.
  * **Option de Randomisation CLI** : Ajout du drapeau `--randomize-stress-positions` sur la ligne de commande pour réactiver explicitement le comportement historique de randomisation continue.
  * **Tests Unitaires Dédiés** : Intégration de tests unitaires (`test_virtual_source_determinism`) à la fin de `simulator.rs` pour valider mathématiquement et hors-contexte graphique (headless) le déterminisme de l'oscillation et de la randomisation.

---

## 🧠 6. Rationalisation du Throttling Doppler : Fréquence de Mise à Jour, Crénelage Temporel & Bruit de Fermeture (Zipper Noise)

Lors du développement du système de mise à jour Doppler temps réel, la question s'est posée d'harmoniser la fréquence d'envoi des événements cinématiques (`DopplerEvent`) depuis le thread de simulation principal vers le thread de traitement audio CPAL. Par défaut, la physique sous-jacente de l'application est simulée à **60 Hz** tandis que les fenêtres graphiques débloquées tournent à **400+ Hz**. Le seuil de throttling temporel a été fixé à **144 Hz** (intervalle minimal de **6.94 ms**).

### A. Fréquence d'Échantillonnage Audio (Audio Rate) vs Fréquence de Contrôle (Control Rate)
En synthèse audio et traitement du signal temps réel, il est courant de distinguer l'**Audio Rate** (le taux d'échantillonnage matériel, ici 48 kHz) et le **Control Rate** (la fréquence de mise à jour des paramètres macroscopiques comme le volume, le pitch, la position). 
Le thread CPAL traite les échantillons par blocs de N = 64 frames. La durée temporelle d'un bloc de traitement DSP est de :

`Δt_bloc = N / fs = 64 / 48000 ≈ 1.33 ms`

Toutes les 1.33 ms, le processeur DSP (`DspProcessor::process_block`) calcule l'atténuation de distance, le retard interaural (ITD), et le facteur Doppler (α) pour chaque échantillon.

### B. Le Phénomène du Bruit de Fermeture (Zipper Noise) & Crénelage Spatial
Si les coordonnées spatiales de la source ne sont rafraîchies qu'à une fréquence trop basse (ex : le moteur physique standard à 60 Hz, soit toutes les 16.67 ms) :
1. **Discontinuités des coefficients de gain et de retard** : La position absolue de la fusée à l'écran change par grands sauts discrets toutes les 12 boucles du processeur DSP (16.67 ms / 1.33 ms).
2. **zipper noise** : L'application brutale de ces nouveaux paramètres à la frontière d'un bloc audio crée des discontinuités de phase et d'amplitude dans la forme d'onde. Ces sauts discrets agissent comme de petites fonctions échelons qui injectent des harmoniques haute fréquence indésirables, perçues par l'auditeur comme un grésillement ou un bruit de fermeture à glissière (*zipper noise*).
3. **Crénelage Spatial (Spatial Aliasing)** : Pour des objets balistiques très rapides se déplaçant à proximité immédiate de l'auditeur (où le gradient de distance par rapport au temps est maximal), un échantillonnage spatial à 60 Hz est insuffisant pour représenter le mouvement de façon continue (aliasing de la trajectoire).

### C. La Rationale du Choix de 144 Hz (6.94 ms)
Le throttling à 144 Hz a été choisi comme le compromis architectural optimal (*sweet spot*) :
* **Lissage Acoustique Sub-bloc** : En rafraîchissant les coordonnées toutes les 6.94 ms (environ tous les 5 blocs DSP au lieu de 12), le signal de contrôle est suffisamment dense pour que l'interpolation linéaire interne du DSP (`target_gains` LERP) produise des rampes lisses et continues, masquant totalement le zipper noise.
* **Bande Passante Inter-thread** : Sur le canal `crossbeam_channel`, cela plafonne le trafic à un maximum de 144 messages par seconde et par source active. À 256 sources, le débit d'événements passe d'une saturation de 102 400 messages/sec (en rendu débloqué à 400 FPS) à seulement 36 864 messages/sec au maximum théorique (et uniquement pour les sources avec voix active), réduisant de près de 3x la charge de synchronisation CPU.

### D. Références et Bibliographie
* **J. O. Smith III**, *Physical Audio Signal Processing*, Center for Computer Research in Music and Acoustics (CCRMA), Stanford University. (Section sur le *Control Rate and Audio Rate* et l'atténuation du zipper noise par interpolation linéaire).
* **D. A. Jaffe & J. O. Smith**, *Extensions of the Karplus-Strong Plucked-String Algorithm* (1983) — Analyse de l'impact des variations temporelles discontinues de pitch et de phase.
* **Woodworth, R. S., & Schlosser, H. C.**, *Experimental psychology* (1954) — Modélisation de la diffraction et du délai interaural (ITD) dynamique à partir de coordonnées sphériques continues de la tête.

---

## 🏗️ 7. Refactoring Architectural de la Scène de Stress, Auto-détection du Dossier de Travail & Intégration Taskfile (22 Juillet 2026)

Afin d'éviter que le fichier `src/simulator.rs` ne devienne trop volumineux et difficile à maintenir (dépassant 110 Ko), un refactoring structurel majeur a été entrepris pour décommissionner et isoler les composants.

### A. Découplage de la Scène de Stress (SoC)
* **Création d'un module dédié** : Tout le code métier, cinématique, d'intégration ImGui, et de rendu de la scène de stress audio a été déplacé dans le nouveau fichier [**`src/simulator/audio_stress_scene.rs`**](../src/simulator/audio_stress_scene.rs).
* **Encapsulation complète** : Les structures `VirtualSource` et `AudioStressScene` y sont définies de manière autonome. La structure `Simulator` n'a plus qu'un seul champ `audio_stress_scene` et lui délègue les traitements requis.
* **Déplacement des tests unitaires** : Le test unitaire `test_virtual_source_determinism` a été déplacé de manière cohérente à la fin de ce nouveau module.

### B. Auto-détection de CWD & Robustesse du Parsing CLI
* **Résolution des crashs de chemins relatifs** : Si le binaire est exécuté à partir d'un sous-dossier (comme `src/` lors de l'usage de `cargo run`), le programme détecte automatiquement l'absence du dossier `assets` et redirige le répertoire de travail vers le dossier parent (racine du projet). Cela élimine les erreurs de fichiers introuvables (`os error 2`).
* **Filtrage des arguments d'export** : Ajout d'un contrôle pour empêcher le programme de confondre des flags de ligne de commande (comme `--audio-stress-scene`) avec des noms de fichiers d'export WAV à créer.

### C. Intégration de la tâche Taskfile
* **Nouveau raccourci `task run-audio-stress`** : Ajout d'une tâche dédiée dans [**`Taskfile.yml`**](../Taskfile.yml) qui pré-configure toutes les variables d'environnement optimales (VSync désactivée, HUD de performance graphique activé) et lance la simulation interactive. Le nombre de sources peut être passé en paramètre (ex: `task run-audio-stress -- 256`, 128 par défaut).



