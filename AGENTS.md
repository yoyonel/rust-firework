# 📂 MANIFESTE TECHNIQUE & CADRE COMPORTEMENTAL DU SYSTEM PROMPT V2
# MOTEUR DE SIMULATION RUST (FIREWORKS SIMULATOR)

Ce manifeste constitue le cadre comportemental absolu, universel et immuable ("DNA Agent") régissant toutes les interventions de l'agent IA. L'agent agit sous une méthodologie **Zero-Trust** et applique rigoureusement les directives architecturales, les 7 Règles d'Or et les 6 Piliers d'Ingénierie de Précision.

---

## 1. STACK TECHNIQUE & ARCHITECTURE DE RÉFÉRENCE
* **Langage :** Rust (Edition 2021).
* **Moteur Physique & Rendu :** Moteur 3D temps réel custom, physique gravitationnelle N-corps, rendu audio spatialisé via CPal (thread audio dédié, architecture lock-free).
* **Projection Visuelle :** Projection équirectangulaire (equirectangular mapping) explicite. Aucune supposition de layout cubemap ne doit être faite.
* **Interface Utilisateur :** ImGui (Rust).
* **CQRS / Event-Driven :** Découplage strict. L'UI ne mute jamais directement l'état des moteurs (physique/audio). Émission de commandes (ex: `PhysicCommand`) dans des files de messages thread-safe (`cmd_queue`).

---

## 2. LES 7 RÈGLES D'OR IMMUABLES (ZERO-TRUST)

### Règle 1 : Stop-Point & Validation Humaine Explicite (Zero-Trust)
Interdiction formelle d'exécuter un `git commit` ou `git push` sans afficher le diff résumé, les preuves de tests/profiling et d'obtenir la validation humaine explicite.

### Règle 2 : Target Branch Isolée & PR Gatekeeper
Tout développement s'effectue sur une branche isolée (`fix/...`, `feat/...`) issue de `develop`. Commits directs sur `master` ou `develop` stricts interdits. Tout merge s'effectue via une Pull Request ouverte vers `develop` via l'outil `gh`.

### Règle 3 : Idempotence & Isolation du Scope (No Scope Creep)
Le code généré est strictement limité au besoin spécifié. Toute modification parasite (fichiers touchés hors scope, reformatages non sollicités) doit être rejetée immédiatement via `git restore`.

### Règle 4 : Mimétisme Architectural
Ne jamais inventer de nouveau pattern de conception si un pattern legacy fonctionnel existe dans le codebase. L'agent doit inspecter l'existant et reproduire la structure canonique.

### Règle 5 : Standards de Persistance UI
Tout contrôle ImGui réclame la mise à jour de sa configuration canonique, sa sauvegarde à l'arrêt et sa restauration au démarrage/reload. Obligation de taguer le code avec `// GUI_PERSIST: <module>`, d'enregistrer la ligne dans `doc/gui_persistence_inventory.md`, et de valider via `task gui-persistence-check`.

### Règle 6 : Respect des Intentions Mathématiques & Stratégies de Mutation UI
* **Intentions mathématiques :** Toujours analyser les commentaires et invariants pré-existants (ex: usage explicite d'un `Vec2::ZERO` ou constante physique) avant tout refactoring.
* **Mutation UI :** Appliquer une stratégie différée ("Apply Changes") pour la réallocation de buffers lourds, et dynamique ("Live Tweak") uniquement pour les scalaires légers.

### Règle 7 : Zéro Constante Magique & UI Scaling Dynamique
* **Zéro Littéral Inline :** Interdiction d'inliner des littéraux numériques, tuples de couleur RGBA ou spécifications de presets dans la logique métier ou le rendu UI. Centralisation obligatoire dans les modules Single Source of Truth (SSOT) : `src/physic_engine/constants.rs`, `src/audio_engine/constants.rs`, `src/renderer_engine/constants.rs` ou `src/simulator/gui_settings/theme.rs`.
* **Dynamic UI Item Width :** Interdiction de hardcoder des largeurs statiques en pixels (ex: `200.0`). Adapter dynamiquement la largeur en fonction de `ui.current_font_size()` (ex: `ui.current_font_size() * 14.0`).

---

## 3. LES 6 PILIERS D'INGÉNIERIE DE PRÉCISION

### PILIER 1 : PROTOCOLES DE VALIDATION & TESTS (FAIL-FAST PYRAMID & ZERO-TRUST)
1. **Pyramide d'Exécution Strict (Fail-Fast) :**
  - **Boucle Dev :** Exécuter le test unitaire ciblé (`task test-one -- <nom_test>`).
  - **Validation Globale :** Exécuter `task test` (`--test-threads=1`).
  - **Pre-Commit / Render Pipeline Mod :** Exécuter `task test-opengl-mesa` (validation des spécifications OpenGL headless Mesa) et `task test-visual-full` (non-régression visuelle 120-frames).
2. **Anti-Triche & Zero-Trust :**
  - Interdiction formelle de modifier, assouplir, supprimer une assertion ou d'ajouter `#[ignore]` pour forcer le succès d'un test. Le code de production s'adapte au test.
  - En cas d'échec : Débogage obligatoire avec `--nocapture` et `--test-threads=1`.
  - **Exception Unique (Refactoring Architectural majeur) :** Si un test devient techniquement obsolète (ex: migration AoS $\rightarrow$ SoA), l'agent doit effectuer un **Stop-Point immédiat** et
soumettre une demande d'autorisation explicite et argumentée à l'humain.

### PILIER 2 : PROFILING CPU & GPU (OBSERVABILITÉ & PREUVE CHIFFRÉE)
1. **Politique d'Instrumentation (Code Source) :**
  - **Macro-instrumentation :** Traçage des passes de rendu majeures et boucles structurelles globales via `gpu_profile_zone!` (spans Tracy + marqueurs GL `push_debug_group!`). Conservation définitive
  dans le code sous la feature Cargo `tracy`.
  - **Micro-instrumentation :** Marqueurs temporaires dans les boucles chaudes par particule/sample. Nettoyage strict et obligatoire avant le Stop-Point pre-commit pour éviter la pollution des mesures.
2. **Protocole de Preuve CLI A/B (Zero-Trust Performance) :**
  - Aucune optimisation/refactoring de performance n'est accepté sans mesure comparative.
  - **Isolation CPU (Anti-Bruit) :** L'exécution des outils de profilage DOIT être strictement synchrone/bloquante. Interdiction absolue de lancer ces outils en arrière-plan (background task) pendant que l'agent réfléchit ou manipule d'autres fichiers.
  - **Workflow A/B :**
    1. Capture Baseline CLI sur `develop` (`task valgrind-callgrind`, `task asm-count-simd` ou `task heaptrack`).
    2. Capture Target CLI sur la branche du fix.
    3. Restitution obligatoire d'un diff textuel synthétique (% gain d'instructions, ratio d'instructions SIMD AVX2 vectorielles vs scalaires, réduction d'allocations mémoire) avant la demande de validation humaine.

### PILIER 3 : BENCHMARKING & PREUVE STATISTIQUE (PERF-TDD)
1. **Workflow Bench-First (Perf-TDD) :**
  - Interdiction formelle de modifier le code de production à des fins de performance sans métrique pré-existante.
  - En cas d'absence de benchmark : Création préalable obligatoire d'un benchmark dans `benches/` sur `develop`.
  - Exigences de code : Utilisation stricte de `criterion::black_box` (anti-inlining) et `iter_batched` (isolation des allocations de setup).
  - **Isolation CPU (Anti-Bruit) :** Tout comme le profilage, l'exécution de `cargo bench` DOIT monopoliser l'agent de manière synchrone. L'exécution en arrière-plan est strictement interdite pour ne pas polluer le cache L1/L2 et fausser la baseline de Criterion.
  - Protocole A/B : Capture `--save-baseline legacy` sur `develop` $\rightarrow$ Implémentation sur branche isolée $\rightarrow$ Exécution comparative `--baseline legacy`.
2. **Validation Statistique Strict (Zero-Trust) :**
  - Seul le rapport natif Criterion fait foi.
  - **Validation Stop-Point :** Requiert une preuve statistique formelle ($p < 0.05$) et un gain nettement au-delà du bruit de mesure.
  - **Rejet Immédiat & Rollback :** Si Criterion indique *"Change within noise threshold"* ou *"No change"*, l'optimisation est rejetée. Exécution obligatoire de `git restore` et révision de la conception.

### PILIER 4 : SANITIZERS & SÛRETÉ CONCURRENTE (FAIL-SAFE)
1. **Périmètre Déclencheur (Targeted Sanitizing) :**
  - **Concurrence & Multithread :** Toute modification touchant le thread audio CPal, les canaux lock-free (`crossbeam`), les atomiques ou la queue CQRS exige un succès préalable à `task test-tsan` (`-
  Zsanitizer=thread`).
  - **Mémoire Bas-Niveau & Unsafe :** Tout refactoring de mémoire brute (AoS/SoA, pointeurs bruts, buffers persistant OpenGL AZDO) requiert un passage propre sous Valgrind Memcheck (ou ASan).
  - **Exemption :** Code logique pur, UI ImGui et mathématiques scalaires sans concurrence/unsafe.
2. **Fallback & Gestion d'Environnement (Fail-Safe) :**
  - **Absence de Nightly :** L'agent tente l'installation automatique (`rustup toolchain install nightly --component rust-src`).
  - **Conflits Drivers Graphics (Mesa/TSan) :** Obligation d'isoler la logique métier dans un test unitaire headless dédié, puis d'exécuter TSan sur ce binaire unitaire.
  - **Échec Outillage :** Fallback sur `valgrind --tool=helgrind` (ou `memcheck`). Si impossibilité mécanique totale de vérifier la sûreté $\rightarrow$ **Stop-Point absolu**, alerte humaine avec rapport d'incident, commit interdit.

### PILIER 5 : DOCUMENTATION & REPRODUCTIBILITÉ HUMAINE ("ZÉRO MAGIE")
1. **Rapport Technique Horodaté (ADR / Traçabilité) :**
  - Toute modification structurelle ou optimisation de performance exige la création d'un rapport `doc/YYYYMMDD_<nom_fix>_report.md` (référencé obligatoirement dans `doc/SUMMARY.md`).
  - **Structure obligatoire :**
    1. Rationnel (Architecture, choix mathématiques/physiques).
    2. Preuves comparatives A/B (Tableaux Criterion, sorties Callgrind/TSan).
    3. Runbook de Reproductibilité : Commandes CLI exactes (avec explication synthétique des flags) permettant au développeur humain de relancer l'audit à l'identique.
2. **Code & Outillage (Standard Zéro Magie) :**
  - **Constantes :** Proscription absolue des valeurs/tuples magiques inline. Centralisation SSOT obligatoire (`constants.rs` ou `theme.rs`).
  - **Blocs Unsafe :** Présence obligatoire de la clause `// SAFETY: <justification technique>` avant chaque bloc `unsafe` ou manipulation de pointeur brut.
  - **CLI & Transparence :** Interdiction d'exécuter des commandes Bash opaques. Explication synthétique des options et flags CLI clés exigée.

### PILIER 6 : CI/CD, PRE-COMMITS & RUNNERS DISTANTS (SHIFT-LEFT)
1. **Vérification Locale Prébail (Pre-CI Gate & Shift-Left) :**
  - L'agent ne doit JAMAIS initier un Stop-Point de fin de tâche, ni ouvrir une PR (`gh pr create`), sans avoir d'abord simulé le pipeline CI en local.
  - **Séquence obligatoire pre-PR :**
    1. Activation des hooks : `task hooks:install`.
    2. Linter exhaustif : `task lint` (fmt, clippy strict `-D warnings`, doc-lint).
    3. Validation architecturale UI : `task gui-persistence-check`.
    4. Simulation Headless : `task test-opengl-mesa` (vérification du code sous GPU logiciel Mesa/Xvfb).
2. **Gestion des Échecs CI Distants (Asymétrie Local/Runner) :**
  - Interdiction du commit-spamming. En cas d'échec CI distant, diagnostic strict :
    1. Extraction des logs distants : `gh run view --log-failed`.
    2. Reproduction locale de l'environnement dégradé (émulation de la suppression des `[patch.crates-io]` locaux, isolement sous `MESA_GL_DEBUG=1` / `xvfb-run`).
    3. Validation locale du correctif dans ces conditions dégradées avant mise à jour de la PR.

---

## 4. PROTOCOLE D'AUDIT ET DE CONSOLIDATION (PRE-COMMIT / PRE-PR)

Avant d'initier tout Stop-Point et de solliciter la validation humaine pour commit/PR, l'agent exécute la séquence de consolidation suivante :

1. **Audit Diff Global :**
  `git diff develop...HEAD`
2. Shift-Left Local Check :
  `task lint && task gui-persistence-check && task test-opengl-mesa`
3. Consolidation Documentaire :
  • Rapport `doc/YYYYMMDD_<nom_fix>_report.md` rédigé et référencé dans `doc/SUMMARY.md`.
  • Inventaire `doc/gui_persistence_inventory.md` à jour le cas échéant.
4. Squash Atomique (Commit Unique) :
  `git reset --soft develop`
  Présentation du diff consolidé et demande d'autorisation humaine explicite pour exécuter le commit atomique et ouvrir la Pull Request vers `develop` via `gh pr create`.
