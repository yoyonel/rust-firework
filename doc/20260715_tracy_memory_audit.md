# 2026-07-15 - AUDIT DE L'USAGE DE LA MÉMOIRE (RAM)

Cet audit analyse en détail la consommation mémoire de l'application `fireworks_sim` à partir des relevés Tracy, afin d'expliquer l'origine des variations et des changements de paliers constatés.

---

## 1. L'Effet "Microscope" de Tracy (Analyse de l'Échelle)

> [!IMPORTANT]
> Le point le plus crucial à noter est **l'échelle verticale (axe Y)** de la courbe de mémoire dans Tracy :
> * **Consommation totale :** ~5,7 Mo (`5703 KB` à `5705 KB`).
> * **Plage de variation (y-range) :** **2 634 octets** (soit seulement **2,57 Ko**).
>
> Cette fluctuation représente moins de **0,045 %** de la mémoire totale de l'application. La consommation est donc **extrêmement stable**. Tracy met à l'échelle automatiquement l'axe Y pour remplir l'écran, ce qui donne l'illusion visuelle de grandes vagues ou de changements majeurs, alors qu'il s'agit de bruit de fond microscopique.

---

## 2. Origine des Variations (Micro-variations & Spikes)

Les petites variations de l'ordre de quelques octets à 1-2 Ko observées frame par frame proviennent principalement de trois sources :

### A. Le formatage des Logs périodiques
Toutes les 4 à 5 secondes, l'application formate des chaînes de caractères pour afficher les statistiques dans la console :
* **Audio Thread (`dsp_processor.rs`) :** Formate et affiche `audio_frame: avg = ...`, `write_cpal_buffer: avg = ...`, etc.
* **Main Thread (`simulator.rs`) :** Formate et affiche le graphe ASCII (`ascii_sample_timeline`), le FPS moyen (EMA, iter), etc.
* **Coût mémoire :** La macro `format!` ou le passage de variables au logger alloue temporairement des chaînes de caractères (`String`) sur le tas. Ces chaînes sont libérées immédiatement après l'écriture sur la console, créant des pics éphémères. De plus, `self.summary()` dans `src/profiler.rs` alloue une nouvelle `HashMap` à chaque appel pour regrouper les métriques.

### B. Dear ImGui (Interface utilisateur)
Comme Dear ImGui est un moteur d'UI en mode immédiat (Immediate Mode), l'interface est entièrement reconstruite à chaque frame :
* Lorsque la fenêtre d'options (visible dans la capture) ou la console ImGui est ouverte, ImGui génère des sommets (vertices), des commandes de dessin (draw commands) et gère des structures d'événements.
* Si l'utilisateur déplace la souris sur l'interface, clique sur un menu déroulant, ou si du texte change, ImGui peut réallouer dynamiquement de petits buffers internes pour s'adapter à la géométrie de l'UI.

### C. La file d'événements GLFW
À chaque frame, l'application appelle :
```rust
let events: Vec<_> = glfw::flush_messages(self.window_engine.get_events()).collect();
```
* Si des événements système (mouvements de la souris, clics, touches pressées, focus de la fenêtre) se produisent, la méthode `collect()` alloue dynamiquement un vecteur (`Vec`) sur le tas pour stocker temporairement ces événements.
* S'il n'y a aucun événement, `collect()` renvoie un vecteur vide sans allouer de mémoire, mais le moindre mouvement de souris génère de petites allocations/désallocations.

---

## 3. Origine des Paliers (Plateaus / Step-ups)

Les changements de paliers semi-permanents (les sauts horizontaux où la mémoire monte d'un coup et y reste) proviennent de mécanismes de cache :

### A. Initialisation paresseuse (Lazy Caching) des buffers système
* **Stdout/Console Buffering :** Lors du premier affichage d'un log ou d'une sortie console, les bibliothèques standards de Rust/C (comme le `BufWriter` interne de `stdout` ou les buffers de formatage de `env_logger`) allouent dynamiquement des buffers d'écriture.
* Une fois ces buffers alloués, ils ne sont **jamais libérés** pendant la durée de vie du thread pour éviter de répéter des allocations coûteuses (comportement standard de la `glibc` et de `std::io`). Cela crée un palier persistant dès le premier log.

### B. Redimensionnement des buffers ImGui
* Si le nombre d'éléments affichés à l'écran augmente (par exemple, l'ouverture d'un menu déroulant avec plus d'options), ImGui agrandit ses buffers de sommets et de commandes.
* Une fois qu'un buffer de dessin ImGui a grandi, il conserve sa capacité maximale pour éviter des réallocations futures, ce qui stabilise la mémoire sur un palier plus haut.

### C. Caches internes de l'allocateur de mémoire (Glibc/Rust)
* L'allocateur mémoire (le gestionnaire de tas système de Linux) ne restitue pas immédiatement chaque bloc de mémoire libéré au noyau OS pour des raisons de performance. Il conserve des listes de blocs libres prêts à être réattribués, ce qui peut se traduire par une consommation rapportée légèrement supérieure et stable (en paliers).

---

## 4. Bilan de l'Audit

| Métrique | Valeur / Comportement | Diagnostic |
| :--- | :--- | :--- |
| **Mémoire totale** | **~5,7 Mo** | **Excellent.** L'usage de la mémoire est extrêmement bas pour une application graphique avec moteur physique et audio 3D. |
| **Fuite mémoire** | **Aucune** | La mémoire n'augmente pas de manière linéaire ou infinie au cours du temps. Elle se stabilise rapidement après l'initialisation. |
| **Stabilité** | **Haute** | Les variations observées (2,5 Ko) sont négligeables (0,04% de la RAM totale). |

---

## 5. Recommandations (Optionnelles)

Si vous souhaitez obtenir une ligne de mémoire **parfaitement plate** (zéro variation) dans Tracy, vous pouvez appliquer les optimisations suivantes :

1. **Désactiver les logs console périodiques** en production ou augmenter leur intervalle de déclenchement.
2. **Éviter de générer des chaînes dynamiques** dans la boucle de rendu principale (par exemple, en pré-allouant les chaînes de caractères pour l'affichage ou en désactivant le tracé du graphe FPS ASCII).
3. **Fermer l'interface ImGui** (les menus d'options) lors des mesures de performance pures.
