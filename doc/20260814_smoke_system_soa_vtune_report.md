# Rapport de Profiling VTune : Architecture SoA (Struct of Arrays) sur SmokeSystem

**Date** : 2026-08-14
**Sujet** : Refactoring massif de `SmokeSystem` de AoS (Array of Structs) vers SoA (Struct of Arrays) pour maximiser la vectorisation SIMD.

## 1. Protocole et Commandes
**Commande d'exécution stricte (Fast-Forward) :**
```bash
task benchmark-vtune -- --deterministic-seed 42 --fixed-dt 0.016666 --timeout-secs 10 --disable-audio
```

## 2. Tableaux des Métriques Normalisées (par Frame)

### Baseline (SmokeSystem AoS Classique)
* **Frames Totales** : 3883
* **LLC Miss Count** : 650 273
* **DRAM Bandwidth Bound** : 21.7%

| Métrique par Frame | Valeur |
|--------------------|--------|
| **CPU Time / frame** | 1.083 ms |
| **Loads / frame**    | 448 867 |
| **Stores / frame**   | 198 254 |

### Optimisée (SmokeSystem SoA avec 8 Vecteurs)
* **Frames Totales** : 3468
* **LLC Miss Count** : 2 950 849
* **DRAM Bandwidth Bound** : 49.2%

| Métrique par Frame | Valeur |
|--------------------|--------|
| **CPU Time / frame** | 1.281 ms |
| **Loads / frame**    | 448 687 |
| **Stores / frame**   | 203 668 |

## 3. Analyse et Évolution (Delta)

| Métrique | Baseline | Optimisée (SoA) | Delta | Statut |
|----------|----------|-----------------|-------|--------|
| CPU Time / frame | 1.083 ms | 1.281 ms | **+18.2%** | 🔴 Régression Majeure |
| LLC Miss Count (Total) | 650k | 2.95M | **+353%** | 🔴 Crash du Cache L3 |
| DRAM Bandwidth | 21.7% | 49.2% | **+126%** | 🔴 Saturation Mémoire |

## 4. Interprétation Architecturale & Décision (Fail-Fast)
C'est un cas d'école absolument fascinant de **Data-Oriented Design poussé trop loin** (Over-Fragmentation). 

Bien que la théorie du SoA promette une vectorisation parfaite, en "éclatant" la particule de fumée en 8 tableaux distincts (`positions`, `velocities`, `colors`, `opacities`, `lifecycles`, etc.), nous avons provoqué une catastrophe matérielle :
1. **Saturation des Hardware Prefetchers :** Le CPU doit lire simultanément depuis 8 régions mémoire totalement disjointes lors de la Phase 1 et lors de la passe de rendu (`for_each_active`). Les limiteurs matériels du processeur (Prefetch Streams) ont été dépassés.
2. **Thrashing du Cache L3 (LLC) :** Le nombre de défauts de cache L3 (LLC Misses) a été **multiplié par 4.5**. Le CPU a passé son temps à attendre la RAM.
3. **Goulot d'étranglement Bande Passante :** La saturation du bus mémoire DRAM est passée de 21% à 49% du temps d'exécution total.

**Conclusion :** L'approche AoS (Array of Structs) initiale offre en réalité une excellente localité spatiale de cache (Spatial Locality) puisque tous les champs d'une particule tiennent dans une ou deux lignes de cache (Cache-Lines) contiguës. L'optimisation SoA est **REJETÉE**. Le fichier `smoke_system.rs` doit être restauré à l'original (Rollback).
