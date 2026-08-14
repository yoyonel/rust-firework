# Analyse Détaillée Profiling Mémoire (VTune & Heaptrack)

**Date** : 2026-08-13
**Benchmark Source** : Historisé dans [20260813_memory_profiling_history.md](./20260813_memory_profiling_history.md)

## 1. HEAPTRACK : Explication de la "Fuite" de 3.00M (Pourquoi c'est sain ?)

*   **Différence entre "Fuite Active" et "Fuite d'Arrêt"** :
    *   **Fuite Active (Danger)** : Mémoire allouée en boucle dans le *hot-path* (ex: 60 fois par seconde) et jamais libérée. Le *Peak heap memory* exploserait vers les Gigaoctets et crasherait la RAM.
    *   **Fuite d'Arrêt (Sain)** : Allocation unique au démarrage (singletons, buffers drivers). Le processus s'arrête, l'outil signale que ces blocs n'ont pas appelé `free()`.
*   **Origine (ALSA `libasound.so.2`)** :
    *   Les hotspots montrent clairement `snd_config_update_r`. ALSA charge en RAM un arbre de configuration matériel complexe au démarrage (le fameux 3.00M).
*   **Paradigme de fermeture "Tear-down"** :
    *   Au lieu de perdre des cycles CPU précieux à détruire proprement des arbres ALSA complexes lors d'un arrêt de l'application, on délègue cette tâche à l'OS.
    *   Lorsque le processus meurt, le noyau Linux libère instantanément et massivement 100% de la mémoire virtuelle associée au PID (page tables). C'est plus rapide, plus sûr (évite les *use-after-free*), et c'est le standard dans les moteurs temps-réel (Data-Oriented Design).
*   **Preuve empirique** : *Peak heap memory* = 6.25M. S'il y avait une vraie fuite, ce chiffre ne serait pas aussi infime après plusieurs secondes d'exécution. Zéro allocation dans la boucle de rendu Rust.

## 2. VTUNE : Analyse Micro-Architecturale (Memory Bound)

*   **P-Core Memory Bound (15.3%)** :
    *   Sur 100 cycles CPU, seuls 15.3 cycles sont perdus à attendre la mémoire. C'est un score d'excellence. 84.7% du temps, le CPU calcule utilement. Preuve que la disposition mémoire contiguë (AoS/SoA) en Rust est optimale.
*   **L1 Bound (7.1%)** :
    *   Faible taux de défauts de cache L1. Les données sont bien pré-chargées et tiennent dans les 32-64 Ko du L1.
*   **Paradoxe de la Bande Passante (26.3% vs 5.7%)** :
    *   **DRAM Bandwidth Bound (26.3% of Elapsed time)** : Le bus mémoire physique de la carte mère tourne à plein régime (pic observé : 20.5 GB/sec sur 21 GB/sec max théorique). C'est normal : le CPU stream massivement des buffers de particules ou d'audio vers le GPU/DSP.
    *   **DRAM Bound (5.7% of Pipeline slots)** : Malgré le bus saturé, le pipeline du CPU n'est bloqué que 5.7% du temps.
    *   **Conclusion** : Le CPU masque parfaitement la latence RAM grâce à l'exécution *Out-Of-Order* (OoO) et aux *Hardware Prefetchers*. Il n'attend pas bêtement.
*   **LLC Miss Count (4.75 Millions)** :
    *   Défauts du Cache L3. Rapporté aux 3.8 Milliards de *Loads* (lectures), le taux d'échec est de ~0.12%. Extrêmement bas.

## 3. ROI (Return On Investment) de l'optimisation L1 Cache

**Faut-il chasser les 7.1% de L1 Bound restants ? NON.**

*   **Gain théorique CPU** : Éliminer 50% des défauts L1 ferait gagner ~3.5% de temps CPU.
*   **Gain Frametime** : Sur 16.6 ms (60 FPS), le gain serait de **0.5 ms**.
*   **Gain FPS Réel** : Nul (0 FPS). L'application étant contrainte par le bus mémoire physique (DRAM Bandwidth à 26.3%) et le pipeline graphique (GPU Bound), optimiser le cache L1 fera simplement terminer le CPU plus vite, qui se mettra en veille en attendant le GPU.
*   **Conclusion** : Le ratio (Complexité de code / Gain de performance) est désastreux. L'architecture actuelle (SoA + Zéro alloc) est déjà optimale. Il faut viser l'optimisation Shaders GPU ou Culling.

**Pistes d'exploration théoriques (Micro-optimisations pures)** :
1.  **Alignement 64-bytes** : `#[repr(align(64))]` sur les structures critiques pour éviter le *False Sharing* inter-threads.
2.  **Software Prefetching** : Intrinsèques `_mm_prefetch::<_MM_HINT_T0>` avant l'accès mémoire dans les grosses boucles de particules.
3.  **SoA Extrême** : Séparer rigoureusement les champs "Hot" (Position) des champs "Cold" (Debug ID) dans des tableaux séparés pour saturer chaque ligne de cache de 64 bytes avec uniquement de la donnée utile.
