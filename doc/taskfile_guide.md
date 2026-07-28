# 🛠️ Guide des Tâches de Compilation et Exécution (Taskfile)

Ce projet utilise **Go-Task** (`task`) via le fichier [**`Taskfile.yml`**](file:///home/latty/Prog/__PERSO__/rust-firework/Taskfile.yml) pour automatiser et standardiser les commandes de développement, de test, de benchmark et de profilage.

---

## 🚀 1. Compilation & Exécution Standard

Ces tâches permettent de compiler et lancer l'application avec différents profils Cargo.

| Commande | Description |
|:---|:---|
| `task build-debug` | Compile le projet en mode debug (développement). |
| `task run-debug` | Exécute le simulateur en mode debug. |
| `task build-release` | Compile le projet en mode release standard (optimisé). |
| `task run-release` | Exécute le simulateur en mode release standard. |

---

## 🏎️ 2. Profils Haute Performance & HUDs

Pour mesurer précisément les limites du moteur ou jouer dans les meilleures conditions de fluidité :

| Commande | Description |
|:---|:---|
| `task build-ultra-speed` | Compile avec les optimisations maximales (LTO, single-codegen, panic=abort). |
| `task run-ultra-speed` | Exécute avec le profil `ultra-speed` (FPS maximal). |
| `task run-release-with-hud` | Lance en mode release avec le HUD Gallium (FPS/CPU Intel/AMD) activé en surcouche. |
| `task run-prime-with-hud` | Lance sur carte graphique dédiée NVidia (PRIME) avec MangoHud activé. |
| `task run-audio-stress` | Lance la scène interactive de stress-test audio (128 sources par défaut). VSync désactivée et Gallium HUD actif. *Exemple de surcharge :* `task run-audio-stress -- 256` |

---

## 🧪 3. Tests, Qualité & Couverture

Toutes les tâches graphiques et audio s'exécutent par défaut dans un tampon d'affichage virtuel (`xvfb-run`) afin de pouvoir tourner sans écran physique (ex: sur une CI).

| Commande | Description |
|:---|:---|
| `task test` | Lancement de tous les tests unitaires et de spécifications du projet. |
| `task test-one -- <NomTest>` | Lance un test unitaire spécifique de la bibliothèque. |
| `task test-integration` | Lance les scénarios de tests d'intégration complets (capture d'image et flux audio). |
| `task coverage` | Calcule la couverture de code locale via `cargo-llvm-cov` et ouvre le rapport HTML. |
| `task test-opengl-mesa` | Valide les appels OpenGL avec Mesa `llvmpipe` et capture toute violation des spécifications OpenGL via le mécanisme `GL_DEBUG_OUTPUT`. |
| `task lint` | Exécute la vérification complète de formatage (`rustfmt`), d'analyse statique (`clippy`) et de la syntaxe de la documentation (`vale`). |

---

## 🕵️ 4. Profilage & Diagnostic

Ces outils aident à traquer les goulots d'étranglement (CPU/GPU) et les allocations de mémoire.

| Commande | Description |
|:---|:---|
| `task valgrind-callgrind` | Lance le simulateur sous Valgrind Callgrind pour profiler l'usage des caches et du CPU. |
| `task heaptrack` | Analyse en détail les allocations de tas dynamiques (Heap tracking) pour éliminer tout débordement ou allocation dans les boucles de rendu. |

---

## ⏱️ 5. Benchmarks de Performance (Criterion.rs)

Des benchmarks fins mesurent l'impact d'optimisations comme l'usage des instructions SIMD ou le Bus Spatial 2D (Ambisonics).

| Commande | Description |
|:---|:---|
| `task bench` | Lance la totalité des benchmarks Criterion (moteur DSP et Binauralisation). |
| `task bench-dsp` | Benchmark ciblé sur les algorithmes de rééchantillonnage de signaux. |
| `task bench-binaural` | Benchmark ciblé sur le calcul des retards interauraux (ITD) et du gain dynamique. |
| `task bench-spatial-bus` | Compare l'algorithme d'Ambisonics 2D (Spatial Bus) contre le rendu direct classique pour 1 à 512 voix. |
| `task bench-save-baseline -- <nom>` | Exécute les benchs et sauvegarde les résultats sous un nom de référence. |
| `task bench-compare -- <nom>` | Compare les performances actuelles par rapport à la référence sauvegardée. |
| `task bench-open-report` | Lance un serveur HTTP local pour consulter les graphiques Criterion générés. |

---

## 📖 6. Documentation (mdBook & Vale)

Le projet utilise `mdBook` pour compiler les rapports d'architecture et guides en HTML, et `Vale` pour valider la syntaxe mathématique LaTeX de la documentation.

| Commande | Description |
|:---|:---|
| `task doc-build` | Compile le livre de documentation en HTML dans le répertoire `book/`. |
| `task doc-serve` | Lance un serveur web local de documentation avec rechargement automatique sur `http://localhost:3000`. |
| `task doc-clean` | Nettoie les fichiers de documentation compilés (supprime le répertoire `book/`). |
| `task doc-setup-vale` | Installe automatiquement la CLI `vale` localement dans `./bin/vale` si elle n'est pas présente dans l'OS. |
| `task doc-lint` | Valide la syntaxe LaTeX/MathJax des fichiers Markdown de documentation avec Vale. |

---

## 🧠 7. Compétences IA & Indexation Codebase

| Commande | Description |
|:---|:---|
| `task update-ai-skills` | Régénère et configure les compétences IA locales (Repomix + overlays doc/ai_skills/) pour Antigravity / agy. |
