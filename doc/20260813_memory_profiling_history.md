# Historique Profiling & Benchmarks Mémoire

Ce document centralise l'évolution de la consommation mémoire, des allocations dynamiques et des défauts de cache (L1/L2/L3) profilés via `heaptrack` et `vtune`.
Les rapports sont horodatés et standardisés pour faciliter la détection de régressions ou la validation d'optimisations.

## Template Standard

```markdown
### 📝 [YYYY-MM-DD] Bilan Profiling Mémoire

**Objectif** : Validation X, Optimisation Y.
**Commandes Exécutées** : `task benchmark-heaptrack`, `task benchmark-vtune`

#### 1. Heaptrack (Allocations Tas)
- **Peak heap memory consumption** : X MB
- **Calls to allocation functions** : Y
- **Total memory leaked** : Z MB
- **Top 3 Hotspots d'Allocation** :
  1. `fonction_1()` - X allocs
  2. `fonction_2()` - Y allocs
  3. `fonction_3()` - Z allocs
- **Allocations dans le Hot-Path (Render Thread)** : Oui/Non (Détail)

#### 2. VTune (Memory Bound & Cache)
- **Memory Bound** : X %
- **L1 Bound** : Y %
- **DRAM Bound** : Z %
- **LLC Misses** : W Millions
- **Top 3 Hotspots Cache Misses** :
  1. `structure_1`
  2. `structure_2`
  3. `structure_3`

#### Conclusion
- ...
```

---

### 📝 [2026-08-13] Initialisation du Suivi Mémoire

**Objectif** : Mise en place de l'outillage (`vtune`, `heaptrack`) sur la base du projet C++ (suckless-vulkan).
**Commandes Exécutées** : Scripts rapatriés et adaptés pour Rust.

#### 1. Heaptrack (Allocations Tas)
- **Peak heap memory consumption** : 6.25 MB
- **Calls to allocation functions** : 4583
- **Total memory leaked** : 3.00 MB
- **Top 3 Hotspots d'Allocation** :
  1. `libasound.so.2` (ALSA Audio) - 2657 calls
  2. `snd_config_update_r` (ALSA) - 293 calls
- **Allocations dans le Hot-Path (Render Thread)** : Non (majoritairement initialisation Audio/ALSA)

#### 2. VTune (Memory Bound & Cache)
- **Memory Bound (P-core)** : 15.3 %
- **L1 Bound** : 7.1 %
- **DRAM Bound** : 5.7 % (Note: DRAM Bandwidth Bound 26.3%)
- **LLC Miss Count** : 4,751,410
- **Loads/Stores** : 3.82B Loads / 1.33B Stores

#### Conclusion
- L'infrastructure est fonctionnelle. Les allocations dynamiques proviennent quasi exclusivement du sous-système audio ALSA. Le Memory Bound de 15.3% est proche de la cible théorique (15%), validant l'architecture SOA et l'absence d'allocations GUI (ImGui) dans le hot-path.
- **Analyse détaillée** : Voir [20260813_memory_profiling_analysis.md](./20260813_memory_profiling_analysis.md)
