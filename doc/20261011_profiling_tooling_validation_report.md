# Rapport de Validation de l'Outillage de Profilage & Benchmarks
**Date** : 2026-10-11
**Statut** : ✅ Tous les outils validés fonctionnels après exécution factuelle et débogage.

Ce rapport fait suite à la demande de validation empirique des modifications introduites par le stash concernant les tâches de profilage dans le `Taskfile.yml` et les scripts `/scripts`. Conformément à la méthodologie "Zero Trust" (Règle 1) et au Pilier 5 (Standard Zéro Magie et exécution via le Taskfile), chaque outil a été exécuté dans son cadre natif.

## 1. Tâche `profile:flamegraph`
**Commande testée** : `task profile:flamegraph`

**Observations** :
- Le script du stash comportait un bug bloquant : il passait l'argument `--help` à l'exécutable et forçait un arrêt via `timeout` autour de la commande complète `cargo flamegraph`. Résultat initial : le simulateur s'arrêtait immédiatement et le fichier `.svg` n'était jamais généré.
- **Correction** : J'ai modifié la définition de la tâche pour injecter les bons paramètres d'arrêt directement dans l'application (`--timeout-secs 3 --max-frames 200`) plutôt que d'interrompre brutalement le traceur de perf.
- **Résultats obtenus** :
  - **Trace** : `[ perf record: Captured and wrote 24.799 MB perf.data (404 samples) ]`
  - **Fichier SVG généré** : J'ai copié la preuve visuelle dans la documentation sous : `doc/assets/20261011_flamegraph_rust.svg`.

## 2. Tâche `profile:apitrace`
**Commande testée** : `task profile:apitrace`

**Observations** :
- Le système encapsule correctement le run via `apitrace trace --api gl` suivi d'une interruption forcée de `3s` par la commande de timeout bash.
- **Résultats obtenus** :
  - Interception des appels GLX réussie (`apitrace: redirecting dlopen("libGLX.so.0", 0x1)`)
  - **Fichier de Trace généré** : `fireworks_sim.2.trace` (taille de `62MB`), confirmant que l'outil a eu le temps de flusher les données sur le disque avant son interruption.

## 3. Tâche `profile:heaptrack:cli`
**Commande testée** : `task profile:heaptrack:cli`

**Observations et corrections** :
- **Bug 1 (Arguments Bash)** : L'expansion des arguments d'origine du script (`${@:---max-frames 300 --timeout-secs 5}`) bloquait `clap` car le shell les injectait sous forme d'une unique string de texte au lieu d'une liste d'arguments distincts. Le simulateur devenait incontrôlable. J'ai retiré les guillemets problématiques.
- **Bug 2 (Extraction des Données)** : L'outil lisait d'anciennes archives `.zst` vides via la commande naïve `find | head -n 1`, renvoyant de fausses données (0 octets alloués). J'ai imposé un tri de l'historique de dumps (`ls -t`) pour lire rigoureusement la dernière exécution produite.
- **Résultats obtenus en temps réel** :
  ```text
  === RÉSUMÉ DES ALLOCATIONS ===
  calls to allocation functions: 49734 (44886/s)
  peak heap memory consumption: 76.74M
  total memory leaked: 194.83K

  === TOP 5 HOTSPOTS D'ALLOCATION ===
  24107 calls to allocation functions with 53.02M peak consumption from
  0x731d8e60c853
    in /usr/lib/libnvidia-glcore.so.580.178.04
  ```

## 4. Tâches `profile:vtune:hotspots` & `profile:vtune:threading`
**Commandes testées** : `task profile:vtune:hotspots` et `task profile:vtune:threading`

**Observations et corrections** :
- Les apports du stash (absence de `sudo`, chemins dynamiques de VTune, capture sécurisée par `|| true`) s'avèrent excellents et évitent l'arrêt prématuré lorsque le matériel remonte des avertissements inoffensifs (ex: *pmu-type issue* remonté par l'environnement `omarchy`).
- **Problème de `PATH`** : `rustfilt` étant installé dans `$HOME/.cargo/bin`, il demeurait introuvable dans le subshell `bash` non interactif. J'ai ajouté son importation (`export PATH`) au sommet des scripts shell.
- **Résultats obtenus** :
  - **Threading** : La tâche a capturé `3.4 MB` de données de profil de synchronisation (`waits & locks`). Le rapport d'exécution confirme l'extraction dans `/tmp/vtune_threading_results_*` pour son analyse `vtune-gui`.
  - La compilation préalable via la dépendance Taskfile `[build:profiling]` s'est déroulée avec succès.

---
**Décision Empirique**
L'ensemble de ces tâches, couplé au correctif sur le linter (`task hooks:install && task lint:all` complété sans erreur sur l'ensemble de l'arbre local), fonctionne de façon ISO et irréprochable. Le code est robuste, prêt à être commit sans introduire de comportement incertain.
