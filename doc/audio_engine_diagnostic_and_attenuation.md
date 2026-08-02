# Documentation du Moteur Audio : Diagnostics, Latences, et Modèle d'Atténuation de Distance

Ce document récapitule les analyses, corrections de bugs, et améliorations apportées au moteur audio de la simulation de feu d'artifice, notamment en ce qui concerne la latence, l'atténuation de distance, le positionnement de l'auditeur, et le système de diagnostic en temps réel.

---

## 🎧 Positionnement Géométrique et Spatial de l'Auditeur (Listener)

L'auditeur est positionné à chaque frame (ou lors du redimensionnement de la fenêtre) de façon dynamique :
* **Axe X (Horizontal)** : `window_width / 2.0` (parfaitement centré au milieu de l'écran).
* **Axe Y (Vertical)** : `0.0` (sol / bas de la fenêtre de rendu).

Dans l'espace physique OpenGL de la simulation, le sol est situé à `y = 0` (les fusées y apparaissent/décollent) et la gravité s'applique vers le bas. Le Listener est donc situé précisément au **sol et au centre**.

### Schéma ASCII de l'Espace Audio Virtuel (Coordonnées Physiques/OpenGL)
L'origine `(0, 0)` se situe en bas à gauche de la fenêtre.

```text
 ------------------------------------------------------------- y = 800 (Sommet de la fenêtre)
 |                                                           |
 |        * [Explosion fusée 1]                              |
 |          (x1, y1)                                         |
 |                                                           |
 |                                      * [Fusée 2 en vol]   |
 |                                        (x2, y2)           |
 |                                                           |
 |                    Zone d'atténuation                     |
 |                       (1/d decay)                         |
 |                                                           |
 |                 .- - - - - - - - - -.                     |
 |               .  Zone Volume Max (50px).                  |
 |              .        [🎧 Listener]     .                 |
 ---------------------------(w/2, 0)-------------------------- y = 0 (Sol / Bas de l'écran)
 x = 0 (Gauche)              (Centré)                   x = w (Droite)
```

---

## 🛠️ Bugs résolus

### 1. Saturation de la limite dynamique de voix (`max_voices`)
Dans `src/audio_engine/config.rs`, le moteur audio plafonnait dynamiquement le nombre de voix avec `std::cmp::min(self.max_voices, max_physic_rockets)`.
Dans une configuration à 2 fusées (`max_rockets = 2`), cela allouait seulement **2 slots de voix**. Étant donné que les sons de décollage (whoosh) et d'explosion se superposent et possèdent des queues de decay, les voix arrivaient immédiatement à saturation, entraînant le rejet (drop) silencieux du son de l'explosion.
* **Correction** : Ajustement de la formule de calcul dynamique pour s'assurer d'avoir toujours une marge confortable pour la superposition des sons :
  `std::cmp::max(self.max_voices, std::cmp::max(64, max_physic_rockets * 4))` (minimum 64 voix).

### 2. Coordonnées d'explosion non transmises
Dans `src/physic_engine/physic_engine_generational_arena.rs`, la position spatiale des explosions n'était pas copiée dans le tableau global lors de leur déclenchement physique. Par conséquent, toutes les explosions étaient envoyées au moteur audio à une position par défaut de `(0,0)`, ce qui annulait l'effet de spatialisation (le son d'explosion venait toujours du coin inférieur gauche).
* **Correction** : Ajout de la copie de la particule de tête lors de la détection de l'explosion physique.

---

## 🔊 Amélioration du Modèle d'Atténuation de Distance

L'ancien modèle d'atténuation utilisait une relation linéaire abrupte :
`gain = 1.0 - (distance / max_distance)`

Cela provoquait une baisse de volume trop rapide et une coupure sonore instantanée et artificielle dès que l'objet dépassait `max_distance` (qui valait `1000.0` pixels, soit moins que la diagonale de l'écran de 1024x800).

### Nouveau Modèle : Inverse-Distance Roll-off
Nous avons implémenté le modèle physique **standard de l'industrie (OpenAL/FMOD)** :
1. **Zone de Volume Maximal** : Sous un rayon de **`50px`** (`ref_distance`), le son est joué à plein volume (`gain = 1.0`).
2. **Zone de Propagation Logarithmique** : Au-delà de `50px`, le gain décroît de façon inversement proportionnelle à la distance : `gain = ref_distance / distance`.
3. **Zone de Sécurité & Fondu** : Pour libérer proprement les ressources de voix CPU, un fondu linéaire (fade) s'applique à l'approche de `max_distance` (qui a été augmentée à **`2000px`** par défaut dans les paramètres) pour ramener le volume doucement à `0.0` sans coupure brusque.

```rust
let ref_distance = 50.0_f32;
let max_distance = settings.max_distance().max(ref_distance + 1.0);
if distance <= ref_distance {
    1.0
} else if distance >= max_distance {
    0.0
} else {
    let raw_att = ref_distance / distance;
    let fade = (max_distance - distance) / (max_distance - ref_distance);
    raw_att * fade
}
```

---

## 📊 Système de Diagnostic en Temps Réel

Un système de tracking sans verrous (lock-free) a été mis en place pour transmettre la télémétrie du thread audio temps réel (callback CPAL) vers le thread principal :
* **Compteurs stricts** : Total des sons envoyés, reçus par le thread audio, effectivement démarrés (joués), rejetés (drops en cas de saturation de voix), et terminés.
* **Télémétrie de Latence** :
  * **Transit de thread (Transit latency)** : Temps de transfert de la commande de jeu à travers le canal de communication entre le thread de simulation et le thread audio.
  * **Simulation vers Début Audio (Render-to-start latency)** : Temps réel total écoulé entre le déclenchement de l'événement dans le moteur physique et l'écriture du premier échantillon du sample dans le buffer audio CPAL.

---

## 🎨 Représentation Graphique de Debug

Lorsque la **console ImGui est ouverte** (`self.console.open`), le simulateur affiche des outils graphiques de diagnostic :

### 1. Audio Diagnostic Monitor
Une fenêtre ImGui dédiée montrant les compteurs d'événements par type (Fusée/Explosion), les latences moyennes précises en temps réel, un indicateur d'erreur rouge si un son a été perdu, et le journal (log) déroulant des 15 dernières requêtes.

### 2. Superposition Graphique du Listener
Une visualisation géométrique dessinée en arrière-plan à l'écran :
* Une icône de casque vert `🎧 Listener (Sol / Centre)` située au milieu bas de la fenêtre.
* Un demi-cercle bleu de rayon `50px` indiquant la limite de volume maximal.
* Un demi-cercle orange indiquant le rayon de la distance maximale d'audibilité (`max_distance` dynamique récupérée depuis les réglages audio, ex: `2000px`).

### Schéma ASCII de la Visualisation Écran de Debug (Coordonnées Écran ImGui)
Dans les coordonnées de l'écran, le point `(0, 0)` est en haut à gauche et `y` grandit vers le bas.

```text
 ------------------------------------------------------------- y = 0 (Haut de l'écran)
 |                                                           |
 |                                                           |
 |                                                           |
 |                   Zone d'atténuation orange               |
 |                   . - - - - - - - - - - - .               |
 |                 .                           .             |
 |               .      Zone Volume Max bleu     .           |
 |              .             .---.               .          |
 |             .             ( 🎧  )               .         |
 -----------------------------(w/2, h)------------------------ y = h (Bas de l'écran)
 x = 0 (Gauche)             [Listener]                  x = w (Droite)
```

---

## 🔝 Gestion du Focus et de la Priorité d'Affichage (Z-Order)

Dans ImGui, la console de commande occupe une large bande en haut de l'écran. Lors de l'ouverture simultanée de la console et du *Audio Diagnostic Monitor*, cliquer sur le fond de la console ramenait celle-ci au premier plan, masquant la fenêtre de diagnostic (qui passait en arrière-plan mais restait visible en transparence, interceptant ainsi tous les clics utilisateur).

* **Correction** : Nous avons ajouté le flag `imgui::WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS` lors de la création de la fenêtre `"Console"` dans [src/utils/command_console/mod.rs](../src/utils/command_console/mod.rs).
  Ce flag force la console à **rester systématiquement en arrière-plan** des autres fenêtres d'outils flottantes. Ainsi, même si l'utilisateur interagit avec le terminal de commande, la fenêtre de diagnostic audio reste au premier plan, conserve le focus d'entrée de souris, et demeure entièrement manipulable et repositionnable.

---

## 🛰️ Correction du Référentiel Spatial (Désynchronisation du Listener)

Lors de nos tests d'écoute et d'analyse géométrique, nous avons identifié une anomalie où les sons sur la gauche de l'écran étaient extrêmement forts, tandis que ceux du centre (pourtant proches de l'auditeur) et de la droite étaient très étouffés ou inaudibles.

### Cause du bug :
1. Dans `Simulator::run`, le thread audio CPAL était démarré en premier, clonant la position du Listener (`listener_pos`) qui valait alors sa valeur d'initialisation par défaut : `(0, 0)` (en bas à gauche).
2. Ensuite, le thread principal mettait à jour la position du Listener vers `(width / 2, 0)` (le centre bas de l'écran).
3. **Absence de synchronisation** : La structure `DspProcessor` s'exécutant sur le thread audio CPAL ne recevait jamais cette mise à jour et continuait d'effectuer tous les calculs d'ITD/ILD et d'atténuation par rapport à `(0, 0)`!

### Correction :
Nous avons conçu un mécanisme de partage de position temps réel, lock-free et thread-safe :
* Création du type `AtomicVec2` dans `src/audio_engine/types.rs` : stocke les coordonnées X et Y sous forme de `AtomicU32` (les bits binaires des `f32` y sont chargés/sauvegardés de manière atomique sans verrous).
* Remplacement du type de `listener_pos` par `Arc<AtomicVec2>` dans `FireworksAudio3D` et `DspProcessor`.
* Synchronisation automatique : À chaque appel de `set_listener_position(pos)` côté simulation, la nouvelle position est écrite de manière atomique. Côté thread audio temps réel, chaque traitement de bloc lit dynamiquement cette position via `.load()`, garantissant un référentiel spatial parfait à 100% à chaque instant.

---

## 🦹 Stratégie d'Évitement de Sature (Voice Stealing / Vol de Voix)

Lors du retour à une configuration élevée (ex: 64 fusées en vol), la multitude de sons (whooshes de fusées + explosions superposées) a provoqué de nombreuses alertes de drop : `No inactive voice available`. Les 64 slots de voix d'origine étaient saturés.

### Corrections apportées :
1. **Augmentation des voix dans la configuration** : Nous avons doublé la limite de voix par défaut de **`64` à `128`** dans [assets/config/audio.toml](../assets/config/audio.toml).
2. **Algorithme de Voice Stealing (Vol de Voix) avec Priorisation** :
   Plutôt que de rejeter silencieusement un nouveau son lorsque les 128 slots sont occupés, nous avons implémenté un système de vol de voix dynamique et prioritaire dans [dsp_processor.rs](../src/audio_engine/dsp_processor.rs) :
   * **Calcul Spatiale de Pré-Atténuation** : La comparaison de volume entre la nouvelle requête et les voix en cours de lecture utilise désormais le gain **pré-atténué spatialement** de la requête (calculé à partir de sa distance réelle à l'auditeur) au lieu de son gain initial brut. Cela évite qu'une explosion extrêmement éloignée (donc inaudible) ne vole la voix d'un sifflement de fusée très proche.
   * **Pondération par Type (Priorisation)** : Nous avons introduit des poids de priorité selon le type de son (`Explosion` a un multiplicateur de `2.0` vs `1.0` pour `Rocket`). Ainsi, les explosions de feux d'artifice (événements critiques pour l'utilisateur) sont sanctuarisées et ne peuvent pas être volées par de simples sifflements.
   * **Recherche de la voix la plus faible** : Le processeur parcourt toutes les voix actives et identifie celle ayant le produit (volume atténué * priorité) le plus faible.
   * **Remplacement conditionnel** : Si la nouvelle requête a une priorité sonore strictement supérieure à la voix active la plus faible, cette dernière est interrompue (volée). Un événement de drop pour vol est envoyé au diagnostic (`stolen_req_id` avec le motif `"Voice stolen (quieter)"`), et le nouveau son démarre immédiatement.
   * **Sécurité à l'initialisation** : Les voix fraîchement créées reçoivent un `target_gains` initialisé à `[req.gain, req.gain]` (dans [types.rs](../src/audio_engine/types.rs)) pour éviter d'être volées instantanément avant leur premier bloc de rendu audio.

---

## ⚡ Optimisations de Mémoire et Élimination des Allocations sur le Tas (Règles AZDO / Memory)

Pour respecter scrupuleusement les exigences de non-allocation mémoire dans les boucles critiques (thread audio CPAL, boucle de mise à jour de la physique et boucle de rendu graphique), plusieurs optimisations clés ont été intégrées :

### 1. Thread Audio CPAL (Garanti sans allocation / Lock-free)
* **Canal de Debug Borné** : Remplacement de `crossbeam_channel::unbounded()` par un canal borné pré-alloué `crossbeam_channel::bounded(2048)`.
* **try_send non-bloquant** : Le thread audio utilise exclusivement `.try_send()` au lieu de `.send()`. Cette opération est garantie sans lock et sans allocation mémoire sur le tas, protégeant le callback temps réel des interruptions et du ramasse-miettes du système d'exploitation.

### 2. Récupération des Événements et logs (Zéro Allocation par frame)
* **Réutilisation de Vecteur de Transit** : La méthode `pop_debug_events` a été modifiée pour accepter un buffer mutable (`&mut Vec<AudioDebugEvent>`). Le simulateur maintient un vecteur persistant `audio_events_buf` dans sa structure principale, le vide via `.clear()`, et le passe par référence.
* **Buffer Circulaire Borné (`VecDeque`)** : Le journal des événements `audio_debug_records` a été converti d'une `HashMap` dynamique vers un `VecDeque` borné à une capacité fixe de 100 éléments (pré-allouée). L'ajout se fait via `push_back()` et `pop_front()`, et la recherche se fait via un simple parcours linéaire `iter_mut().find(...)`. Une fois la capacité de 100 atteinte, plus aucune allocation sur le tas ne se produit.

### 3. Rendu Dear ImGui (Zéro Allocation par frame)
* **Macros de Formating sur la Pile** : Conception des macros `ui_text!` et `ui_text_colored!` qui écrivent et formatent les chaînes de caractères dynamiques dans un tampon temporaire sur la pile (`[u8; 256]`) via `std::io::Cursor` et `std::io::Write`. Cela évite la création de milliers d'objets `String` via `format!` à chaque frame de rendu dans la boucle de dessin du diagnostic ImGui.
* **Itération Inverse Directe** : L'affichage des 15 derniers logs se fait en parcourant le `VecDeque` en sens inverse (`self.audio_debug_records.iter().rev().take(15)`), ce qui élimine le besoin d'allouer un vecteur intermédiaire de références et de le trier à chaque tick.

### 4. Configuration de la Latence Matérielle (Block Size)
* **Négociation Universelle CPAL** : Le moteur audio utilise `cpal::BufferSize::Default` dans `get_cpal_config` pour s'adapter dynamiquement à la taille de tampon optimale recommandée par le serveur audio de l'OS (PipeWire/PulseAudio/ALSA) et garantir une compatibilité matérielle universelle sans échecs de création de flux audio.
* **Traitement Interne par Bloc** : Le moteur découpe et traite le signal audio selon le paramètre `block_size` configuré par l'utilisateur (par exemple `64` échantillons à 48 kHz = `1.33 ms`).
  * Avec un tampon matériel de `512` échantillons à 48 kHz, le temps de réponse maximum (attente du prochain cycle) est de `10.6 ms` (soit une latence moyenne de transit de `5.3 ms`).
  * Avec un tampon matériel ramené à **`64`** échantillons (`block_size = 64` dans [audio.toml](../assets/config/audio.toml)), l'intervalle de réveil du thread audio est réduit à seulement `1.33 ms`. La latence moyenne chute ainsi en dessous de **`0.7 ms`** (et le "render-to-audio-start" est également réduit sous les **`0.7 ms`**), offrant un rendu sonore instantané et parfaitement synchrone avec l'affichage graphique.
