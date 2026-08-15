# Framework et Guidelines de Profiling A/B avec VTune

Ce document définit la méthode de référence (framework) pour exécuter, analyser et documenter toute comparaison de performance "A/B" dans le projet **Rust Firework**, en utilisant le profiler Intel VTune.

## 1. Principes Fondamentaux (Zero-Trust Performance)
* **Preuve Statistique et Normée** : L'optimisation à l'aveugle est proscrite. Toute modification de code visant l'amélioration des performances CPU/Mémoire DOIT être justifiée par une comparaison A/B entre la branche parente (`develop` ou `baseline`) et la branche optimisée.
* **Reproductibilité Déterministe** : Les runs de mesure doivent toujours emprunter le même flux d'exécution pour être comparables.
* **Isolation du Bruit** : Désactiver tous les threads/systèmes périphériques non liés à la logique mesurée.

## 2. Prérequis d'Exécution : La Stratégie "Fast-Forward"

Afin de profiler la physique et le rendu de manière intensive sur un court laps de temps, nous employons une boucle de simulation débridée et déterministe. Les paramètres suivants sont OBLIGATOIRES lors du profiling :

1. `--deterministic-seed <seed>` : Force le générateur aléatoire à produire la même séquence de fusées et de particules.
2. `--fixed-dt <dt>` (ex: `0.016666`) : Force le delta time à une valeur fixe. La physique effectuera exactement les mêmes intégrations à chaque frame, indépendamment de la vitesse réelle du CPU.
3. `--timeout-secs <secs>` (ex: `5`) : Limite l'exécution à un temps *wall-clock* strict (par ex. 5 secondes). Cela permet de limiter la taille des rapports VTune tout en assurant une charge suffisante.
4. `--disable-audio` : **Crucial**. Le thread audio CPal (et ses buffers limités) ne supporte pas l'exécution à vitesse débridée et produira des *underruns*, ce qui pollue le profilage et bloque le CPU sur des temps d'attente lock-free (`crossbeam`).

## 3. Le Processus de Normalisation (Le Compte des Frames)

Lorsqu'on profile sur un temps fixe (5 secondes), la branche "Baseline" et la branche "Optimisée" n'exécuteront pas le même nombre total de boucles (frames) car l'une est censée être plus rapide que l'autre.

**Le Verrou :** Comparer directement le nombre de `Loads`, `Stores` ou de `LLC Misses` bruts entre deux runs est statistiquement faux si l'un des runs a calculé 3000 frames et l'autre 5000.

**La Solution : La normalisation par frame.**
- À la fin de chaque run `timeout`, l'application imprime sur `stderr` le nombre total de frames exécutées via : `eprintln!("\n=== [METRIC] TOTAL FRAMES GENERATED: {} ===\n", self.frames);`
- Dans l'analyse finale, **toutes les métriques absolues de VTune (Loads, Stores, CPU Time) DOIVENT être divisées par ce nombre de frames**. 
- L'unité de comparaison devient alors le coût unitaire par frame (ex: *1.2ms CPU par frame*, ou *500,000 Loads par frame*).

## 4. Workflows des Profilers
Les scripts d'encapsulation gèrent les spécificités de VTune. **Attention : VTune s'exécute en root (`sudo`)** pour accéder aux compteurs matériels (PMU). Les scripts doivent assurer un output propre vers `/tmp/` pour éviter les conflits de permission (ex: `RES_DIR="/tmp/vtune_results_memory_$(date +%s)"`).

* **Memory Access** (`task profile:vtune`) : Identifie les saturations de la bande passante (DRAM Bound), le nombre d'instructions (Loads/Stores) et les défauts de cache (LLC Miss).
* **Hotspots** (`task profile:vtune-hotspots`) : Évalue le temps de calcul brut par fonction. (Note: induit plus d'overhead d'échantillonnage que Memory Access).
* **Threading** (`task profile:vtune-threading`) : Analyse les verrous (Mutex/RwLock) et les files d'attente inter-threads (Channels).

## 5. Formalisation du Rapport A/B (Livrable)
Toute Pull Request liée à la performance doit comporter dans `doc/` un fichier de rapport (ex: `doc/YYYYMMDD_feature_name_vtune_report.md`) contenant :
1. Les paramètres de la commande exacte exécutée.
2. Un tableau clair de la Baseline (Normalisée par frame).
3. Un tableau de la version Optimisée (Normalisée par frame).
4. Un calcul de l'évolution (Delta en %).
5. Une interprétation architecturale (Pourquoi ça marche ? Cache, Vectorisation, etc.).
