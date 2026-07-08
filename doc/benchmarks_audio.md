# Benchmarks Audio Engine — Guide complet

## Table des matières

1. [Contexte et motivation](#1-contexte-et-motivation)
2. [Outillage : pourquoi Criterion.rs](#2-outillage--pourquoi-criterionrs)
3. [Architecture des benchmarks](#3-architecture-des-benchmarks)
4. [Ce que mesurent réellement les benchmarks](#4-ce-que-mesurent-réellement-les-benchmarks)
5. [Résultats et enseignements](#5-résultats-et-enseignements)
6. [Workflow d'optimisation SIMD](#6-workflow-doptimisation-simd)
7. [Référence des tâches Task](#7-référence-des-tâches-task)

---

## 1. Contexte et motivation

### Pourquoi benchmarker l'audio processing ?

Le moteur audio de fireworks opère sous une contrainte de temps réel stricte : le **Budget Tampon**. À 48 kHz avec un buffer de 256 échantillons, le thread DSP dispose de **5,33 ms** pour calculer et livrer ses échantillons. Tout dépassement provoque un XRUN (décrochage audible).

Deux fonctions sont au cœur du traitement sur le chemin critique :

- **`resample_linear_mono`** ([dsp.rs](../src/audio_engine/dsp.rs)) — rééchantillonnage par interpolation linéaire, appelé au chargement et potentiellement à la volée.
- **`binauralize_mono`** ([binaural_processing.rs](../src/audio_engine/binaural_processing.rs)) — spatialisation 3D par ITD (Interaural Time Difference) + ILD (Interaural Level Difference), appelée **par voix et par bloc** dans la boucle audio.

Avec 16 voix simultanées et un bloc de 512 échantillons à 48 kHz (~10,7 ms de signal), `binauralize_mono` est appelée **16 fois par période audio**. Connaître son coût réel en nanosecondes n'est pas une curiosité : c'est une information de conception.

### Problème avec `cargo bench` built-in

Le benchmark intégré de Rust (`#[bench]`) nécessite la toolchain **nightly** et `#![feature(test)]`. Sur la toolchain **stable** utilisée par ce projet, `cargo bench` compile les tests en mode `release` et les exécute — mais les `#[test]` classiques sont tous marqués `ignored` par le runner de benchmark, produisant `0 measured`.

La sortie trompeuse que l'on obtenait :

```
running 29 tests
test audio_engine::... ... ignored
[...]
test result: ok. 0 passed; 0 failed; 29 ignored; 0 measured
```

Ce n'est pas un bug — c'est le comportement attendu sur stable en l'absence de benchmarks Criterion.

---

## 2. Outillage : pourquoi Criterion.rs

[Criterion.rs](https://bheisler.github.io/criterion.rs/book/) est le standard de-facto pour les benchmarks Rust stable. Il apporte :

| Fonctionnalité | Détail |
|---|---|
| **Statistiques robustes** | Moyenne, médiane, écart-type, intervalles de confiance à 95% sur 100 échantillons |
| **Détection de régressions** | Comparaison automatique "avant/après" avec test de Welch |
| **Rapports HTML** | Graphiques SVG interactifs dans `target/criterion/` |
| **Compatible stable** | Pas besoin de nightly, pas de `#![feature(test)]` |
| **`black_box`** | Barrière de compilation qui empêche le compilateur d'éliminer le code mesuré |

### Intégration dans le projet

**`Cargo.toml`** — Criterion est déclaré en `dev-dependency` avec le feature `html_reports` :

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "audio_dsp_bench"
harness = false          # ← Criterion gère son propre runner

[[bench]]
name = "audio_binaural_bench"
harness = false
```

Le `harness = false` est la clé : il désactive le test runner intégré de Cargo, laissant Criterion piloter l'exécution.

> **Note** : Gnuplot n'est pas requis. Sans lui, Criterion bascule automatiquement sur le backend `plotters` (SVG pur Rust). Pour des graphiques plus riches (courbes de densité de probabilité), installer `gnuplot` : `sudo apt install gnuplot`.

---

## 3. Architecture des benchmarks

### Structure des fichiers

```
benches/
├── audio_dsp_bench.rs       # resample_linear_mono
└── audio_binaural_bench.rs  # binauralize_mono
```

### `audio_dsp_bench.rs` — Rééchantillonnage DSP

**Fichier** : [benches/audio_dsp_bench.rs](../benches/audio_dsp_bench.rs)  
**Fonction testée** : [`resample_linear_mono`](../src/audio_engine/dsp.rs)

| Groupe Criterion | Paramètres | Objectif |
|---|---|---|
| `resample/upsample_44100_to_48000` | 6 tailles : 64→44100 | Scalabilité — ratio ×1.09 (le plus courant) |
| `resample/downsample_48000_to_44100` | 6 tailles : 64→48000 | Asymétrie up/down |
| `resample/identity_48000` | 3 tailles : 512→48000 | Chemin trivial (`src_rate == dst_rate`) |
| `resample/extreme_upsample_8000_to_48000` | 3 tailles : 512→8000 | Ratio ×6 (téléphonie → hi-fi) |
| `resample/one_second_throughput` | 44100 et 48000 samples | Throughput sur 1 seconde audio complète |

**Signal synthétique** : sinus 440 Hz généré en pur Rust. Aucune dépendance fichier WAV, reproductible à l'identique sur toute machine.

```rust
fn make_sine(n_samples: usize, freq_hz: f32, sample_rate: u32) -> Vec<f32> {
    (0..n_samples)
        .map(|i| (2.0 * PI * freq_hz * i as f32 / sample_rate as f32).sin())
        .collect()
}
```

**Pattern Criterion** utilisé — `bench_with_input` pour le paramétrage par taille :

```rust
group.bench_with_input(
    BenchmarkId::from_parameter(n_samples),
    &input,
    |b, inp| {
        b.iter(|| {
            let out = resample_linear_mono(inp, 44100, 48000);
            criterion::black_box(out);  // ← empêche l'élision du code par LLVM
        });
    },
);
```

Le `black_box` est crucial : sans lui, LLVM peut détecter que le résultat n'est jamais utilisé et supprimer l'appel entier, mesurant 0 ns.

---

### `audio_binaural_bench.rs` — Spatialisation 3D

**Fichier** : [benches/audio_binaural_bench.rs](../benches/audio_binaural_bench.rs)  
**Fonction testée** : [`binauralize_mono`](../src/audio_engine/binaural_processing.rs)

| Groupe Criterion | Paramètres | Objectif |
|---|---|---|
| `binaural/positions` | 5 positions spatiales | Impact de l'azimut sur le chemin de calcul |
| `binaural/distances` | 6 distances (1→999m) | Impact de l'atténuation distance |
| `binaural/block_sizes` | 7 tailles (64→4096) | Profil latence audio vs throughput |
| `binaural/signal_types` | Constant / sinus 440Hz / sinus 4kHz | Comportement cache selon le signal |
| `binaural/sample_rates` | 22050, 44100, 48000, 96000 Hz | Sensibilité au sample rate (ITD en samples) |
| `binaural/multi_voice` | 7 paliers : 1, 4, 8, 16, **32, 64, 128** voix | Coût total de mixage — scenarios actuels (32) et futurs (64+, Doppler) |

**Positions spatiales couvertes** :

```
        z- (devant)
           |
 x-  ------+------  x+ (droite)
 (gauche)  |
           z+ (derrière)
```

- `center` : source à `(0, 0, -10)` — azimut 0, ITD nul, traitement symétrique
- `right_90deg` : source à `(10, 0, 0)` — azimut π/2, chemin ITD+ILD droite
- `left_90deg` : source à `(-10, 0, 0)` — azimut -π/2, chemin ITD+ILD gauche
- `above_right_45deg` : source à `(10, 10, 0)` — test du calcul d'élévation 3D
- `behind` : source à `(0, 0, 10)` — azimut 0 mais signal en arrière

**Simulation multi-voix** : le groupe `binaural/multi_voice` simule le cas réel de mixage audio avec `N` voix simultanées. Les positions sont générées par une **spirale de Fibonacci** projetée sur un hémisphère supérieur (feux d'artifice au-dessus du spectateur), ce qui garantit une distribution spatiale uniforme et déterministe sur toute la sphère — sans biais de clustering.

```rust
// Spirale de Fibonacci : distribution isotrope de N points sur la sphère
let golden = PI * (3.0 - 5.0_f32.sqrt()); // ≈ 2.399 rad (angle d'or)
let angle  = i as f32 * golden;
let y_norm = 0.1 + 0.9 * (i as f32 + 0.5) / N as f32; // hémisphère supérieur
let radius = (1.0 - y_norm * y_norm).sqrt();
let scale  = 50.0 + (i as f32 / (N-1) as f32) * 450.0; // 50m – 500m
```

Les paliers couverts : `1, 4, 8, 16, 32, 64, 128` voix — correspondant au défaut actuel (32), aux cibles court terme (64) et aux scénarios futurs avec effets Doppler et autres (128+).

---

## 4. Ce que mesurent réellement les benchmarks

### Ce qui est mesuré

Criterion mesure le **temps CPU wall-clock** de la fonction cible en isolation, en mode `release` (optimisations LLVM complètes activées), sur le cœur courant. Il collecte 100 échantillons et calcule :

- **Médiane** : valeur centrale, robuste aux outliers
- **Intervalle de confiance 95%** : `[borne_basse  médiane  borne_haute]`
- **Outliers** : échantillons ≥ 1.5×IQR de la médiane (bruit OS, migration de cache)

### Ce qui n'est pas mesuré

- **Contention de threads** : les benchmarks s'exécutent sur le thread principal, sans le thread audio CPAL en parallèle. Le coût réel en production inclut la pression cache des autres threads.
- **Effets thermiques** : Criterion ne fait pas de throttling management. Sur un laptop, les résultats peuvent varier entre une exécution froide et une exécution après 10 minutes de charge.
- **Allocations** : Criterion ne mesure pas le nombre d'allocations heap (utiliser `cargo bench --features dhat-heap` ou `heaptrack` pour ça). Les `Vec<[f32; 2]>` retournés par `binauralize_mono` sont alloués à chaque appel — c'est un candidat d'optimisation (pré-allocation de buffer).

### Interprétation de l'intervalle de confiance

```
resample/upsample_44100_to_48000/512
                        time:   [8.4526 µs  8.8591 µs  9.2911 µs]
                                 ↑ borne basse  ↑ médiane  ↑ borne haute
```

Un intervalle large (ex: `[45 µs … 52 µs]`) indique une variabilité élevée — souvent du bruit OS (interruptions, migration de cœur). Relancer avec `taskset -c 0` pour épingler sur un cœur réduit ce bruit.

---

## 5. Résultats et enseignements

### `resample_linear_mono` — Résultats complets

#### Upsample 44100 → 48000 Hz (ratio ×1.09)

| Buffer (samples) | Durée audio @ 48kHz | Temps mesuré | Facteur temps réel |
|---|---|---|---|
| 64 | 1.3 ms | **1.04 µs** | ×1300 |
| 128 | 2.7 ms | **2.37 µs** | ×1140 |
| 512 | 10.7 ms | **8.86 µs** | ×1200 |
| 1024 | 21.3 ms | **17.14 µs** | ×1245 |
| 4096 | 85.3 ms | **64.05 µs** | ×1332 |
| 44100 | 1000 ms | **684.6 µs** | ×1460 |

#### Downsample 48000 → 44100 Hz vs Upsample (même taille sortie)

| Buffer | Upsample | Downsample | Ratio |
|---|---|---|---|
| 64 | 1.04 µs | 1.17 µs | ≈ identique |
| 512 | 8.86 µs | 6.16 µs | **down -30%** |
| 4096 | 64.05 µs | 60.02 µs | **down -6%** |

> **Enseignement** : Le downsample est plus rapide aux grandes tailles car il produit _moins_ de samples de sortie (ratio ÷1.09). La boucle d'interpolation itère sur `out_len` qui est plus petit. Cela confirme que la scalabilité est bien dominée par la taille de sortie, pas la taille d'entrée.

#### Chemin identity (src_rate == dst_rate)

| Buffer | Temps mesuré | Ratio vs upsample équivalent |
|---|---|---|
| 512 | **137 ns** | ×65 plus rapide |
| 4096 | **435 ns** | ×147 plus rapide |
| 48000 | **13.77 µs** | ×50 plus rapide |

> **Enseignement** : Le court-circuit `if src_rate == dst_rate { return input.to_vec(); }` est massivement efficace. `to_vec()` utilise `memcpy` optimisé par LLVM (voire SIMD implicite). C'est le comportement attendu — ne jamais supprimer ce guard.

#### Extreme upsample 8000 → 48000 Hz (ratio ×6)

| Buffer | Temps mesuré |
|---|---|
| 512 | **48.5 µs** |
| 4096 | **309.6 µs** |
| 8000 | **605.9 µs** |

> **Enseignement** : Le ratio ×6 produit ~5.5× plus de samples que le ratio ×1.09 pour la même entrée. Le temps suit fidèlement — la fonction est `O(n_output)` sans overhead caché, ce qui est une bonne nouvelle pour la prévisibilité.

#### Throughput 1 seconde d'audio

| Conversion | Temps mesuré | Facteur temps réel |
|---|---|---|
| 44100 → 48000 | **705 µs** | **×1418** |
| 48000 → 44100 | **820 µs** | **×1220** |

> **Enseignement** : 1 seconde d'audio entier est traitée en moins de 1 ms. La fonction `resample_linear_mono` n'est **pas** un goulot sur le chemin critique audio — elle est appelée au chargement, pas dans la boucle temps réel. C'est rassurant, mais pas une raison pour ne pas l'optimiser si on voulait du streaming live.

---

### `binauralize_mono` — Attendus et points d'attention

Le groupe `binaural/block_sizes` est le plus important : il révèle le **coût par voix par période audio**.

Avec un bloc de 512 samples (taille typique à 48 kHz) et 16 voix simultanées :

```
Coût total par période ≈ 16 voix × coût_binauralize(512)
Budget tampon à 48 kHz / 256 samples ≈ 5,33 ms
```

Si `binauralize_mono(512)` coûte ~100 µs (hypothèse avant mesure), 16 voix = 1.6 ms, soit ~30% du budget tampon. C'est acceptable mais surveiller.

Le groupe `binaural/multi_voice` simule exactement ce scénario — il fournit le coût agrégé réel incluant la pression cache entre appels successifs.

---

## 6. Workflow d'optimisation SIMD

### Contexte : la feature `simd` du projet

Le projet dispose d'une feature `simd` déclarée dans `Cargo.toml` :

```toml
[features]
default = ["simd", "test_helpers"]
simd = []    # Active le code SIMD
no_simd = [] # Force le mode scalaire
```

À l'heure actuelle, la feature `simd` est déclarée mais le code audio n'utilise pas encore d'intrinsics SIMD explicites. Le compilateur peut générer de l'auto-vectorisation LLVM, mais sans garantie. L'objectif est d'instrumenter, mesurer, puis optimiser de façon outillée.

---

### Principe général du workflow

```
Mesurer (baseline) → Modifier → Mesurer (comparaison) → Décider
```

Criterion supporte nativement cette boucle via les **baselines nommées** : une baseline est un snapshot des résultats sauvegardés sous un nom, contre lequel la prochaine exécution se compare automatiquement.

---

### Étape 1 — Établir la baseline scalaire (sans SIMD)

Sauvegarder les performances actuelles _sans_ la feature `simd` comme référence :

```sh
# Via Task (recommandé)
task bench-save-baseline -- scalar

# Ou directement via cargo
cargo bench --no-default-features -- --save-baseline scalar
```

Les résultats sont persistés dans `target/criterion/` sous le nom `scalar`. Ils survivent aux recompilations.

> **Attention** : effectuer cette mesure dans des conditions stables (machine non chargée, pas de mises à jour en arrière-plan, préférer un cœur dédié avec `taskset -c 0 task bench-save-baseline -- scalar`).

---

### Étape 2 — Identifier les cibles SIMD dans le code audio

#### Candidat principal : `interpolate_sample_fast` dans `binauralize_mono`

```rust
// binaural_processing.rs — boucle interne appelée N fois par sample
let stereo: Vec<[f32; 2]> = (0..n)
    .map(|i| {
        let idx_l = (i as f32) - itd_left_samples;
        let idx_r = (i as f32) - itd_right_samples;
        let s_left  = interpolate_sample_fast(mono, idx_l) * gain_left;
        let s_right = interpolate_sample_fast(mono, idx_r) * gain_right;
        [s_left, s_right]
    })
    .collect();
```

Cette boucle est `O(n_samples)` avec des accès mémoire strided et des opérations FP indépendantes — profil idéal pour SIMD. On peut traiter 4 ou 8 samples `f32` en parallèle avec SSE2/AVX2.

#### Candidat secondaire : `resample_linear_mono`

La boucle principale est une interpolation linéaire scalaire :

```rust
let mut idx = 0.0_f32;
for _ in 0..out_len {
    // ...
    let s0 = src[i0];
    let s1 = src[i0 + 1];
    out.push(s0 + (s1 - s0) * frac);
    idx += step;
}
```

Vectorisable en AVX2 (8 × f32 en parallèle), mais l'indexing `i0 = idx.floor() as usize` avec gather est plus complexe. Cible secondaire après `binauralize_mono`.

---

### Étape 3 — Implémenter le code SIMD derrière la feature gate

La feature `simd` permet de switcher entre l'implémentation scalaire et SIMD sans changer l'API :

```rust
// binaural_processing.rs
#[cfg(feature = "simd")]
pub fn binauralize_mono(mono: &[f32], ...) -> Vec<[f32; 2]> {
    binauralize_mono_simd(mono, ...)
}

#[cfg(not(feature = "simd"))]
pub fn binauralize_mono(mono: &[f32], ...) -> Vec<[f32; 2]> {
    binauralize_mono_scalar(mono, ...)  // implémentation actuelle
}
```

Les intrinsics SIMD en Rust nécessitent `unsafe` et le feature gate CPU cible :

```rust
#[cfg(feature = "simd")]
#[target_feature(enable = "avx2")]
unsafe fn binauralize_mono_simd(mono: &[f32], ...) -> Vec<[f32; 2]> {
    use std::arch::x86_64::*;
    // Traitement de 8 samples f32 en parallèle via _mm256_*
    // ...
}
```

> Pour la détection runtime du support AVX2 (portabilité), utiliser [`is_x86_feature_detected!`](https://doc.rust-lang.org/std/macro.is_x86_feature_detected.html) ou la crate [`multiversion`](https://crates.io/crates/multiversion).

---

### Étape 4 — Comparer SIMD vs scalaire

```sh
# Via Task (recommandé)
task bench-compare -- scalar

# Ou directement via cargo
cargo bench --no-default-features --features simd -- --baseline scalar
```

Criterion affiche alors une comparaison détaillée pour chaque benchmark :

```
binaural/block_sizes/512
                        time:   [45.234 µs 46.891 µs 48.203 µs]
                        change: [-42.1% -40.8% -39.2%] (p = 0.00 < 0.05)
                        Performance has improved.
```

La ligne `change` donne :
- Le pourcentage d'amélioration ou de régression
- La valeur `p` du test statistique de Welch (< 0.05 = résultat significatif)
- Un verdict textuel : `Performance has improved` / `Performance has regressed` / `No change in performance detected`

---

### Étape 5 — Itérer et sauvegarder les milestones

```sh
# Sauvegarder l'implémentation SIMD v1 comme nouvelle baseline
task bench-save-baseline -- simd_v1

# Après optimisation v2, comparer contre v1
task bench-compare -- simd_v1
```

Il est recommandé de nommer les baselines de façon sémantique : `scalar`, `avx2_v1`, `avx2_v2_prefetch`, etc.

---

### Étape 6 — Vérifier l'absence de régression fonctionnelle

Un gain de performance n'a de valeur que si le résultat reste correct. Après chaque modification SIMD :

```sh
# Tests unitaires (vérifie la correction fonctionnelle)
task test

# Benchmarks (vérifie la performance)
task bench-compare -- <baseline>
```

Le groupe `binaural/positions` vérifie implicitement la correction : si le canal gauche est soudainement plus fort pour une source à droite, les tests de `audio_dsp_test.rs` le détecteront.

---

### Récapitulatif du workflow complet

```
1. task bench-save-baseline -- scalar          ← snapshot de référence

2. Implémenter SIMD derrière #[cfg(feature="simd")]

3. task test                                   ← vérifier la correction

4. task bench-compare -- scalar                ← mesurer le gain

5. Si gain satisfaisant :
   task bench-save-baseline -- simd_v1         ← nouvelle baseline

6. Itérer sur l'implémentation
   task bench-compare -- simd_v1              ← comparer v2 vs v1

7. task bench-open-report                      ← rapport HTML complet
```

---

## 7. Référence des tâches Task

Toutes les commandes passent par [Taskfile.yml](../Taskfile.yml) — ne pas utiliser `cargo bench` directement pour garantir la reproductibilité des flags (`--no-default-features --features simd`).

| Tâche | Équivalent cargo | Description |
|---|---|---|
| `task bench` | `cargo bench --no-default-features --features simd` | Tous les benchmarks |
| `task bench-dsp` | `... --bench audio_dsp_bench` | DSP uniquement |
| `task bench-binaural` | `... --bench audio_binaural_bench` | Binaural uniquement |
| `task bench-save-baseline -- <nom>` | `... -- --save-baseline <nom>` | Snapshot nommé |
| `task bench-compare -- <nom>` | `... -- --baseline <nom>` | Comparaison vs snapshot |
| `task bench-open-report` | *(ouvre le navigateur)* | Rapport HTML Criterion |

### `bench-open-report` — serveur HTTP local

Les rapports Criterion utilisent des **liens relatifs** entre pages HTML. Ouvrir le fichier directement via `file://` échoue avec les navigateurs sandboxés (Flatpak/Snap) car ils copient le fichier dans `/run/user/1000/doc/<hash>/` — les chemins relatifs ne se résolvent plus.

La tâche démarre donc un **serveur HTTP Python local** (`python3 -m http.server`) servant depuis `target/criterion/`. Le navigateur navigue sur `http://localhost:17380/` où tous les liens relatifs fonctionnent.

```sh
task bench-open-report
# 📊 Démarrage du serveur HTTP sur http://localhost:17380/report/index.html
#    (Ctrl+C pour arrêter)
```

> Le port `17380` est choisi pour éviter les conflits avec les ports courants (3000, 8080, 8000).

Pour ne benchmarker qu'une partie, passer le filtre après `--` :

```sh
# Via cargo (le filtrage n'est pas encore exposé dans les tâches Task)
cargo bench --no-default-features --features simd -- "binaural/block_sizes"
cargo bench --no-default-features --features simd -- "binaural/multi_voice"
cargo bench --no-default-features --features simd -- "resample/one_second"
```

### Rapport HTML

Après exécution, les rapports sont disponibles dans :

```
target/criterion/
├── report/index.html                      ← vue d'ensemble globale
├── resample/
│   ├── upsample_44100_to_48000/
│   │   ├── report/index.html              ← courbes par groupe
│   │   └── 512/report/index.html          ← courbes par paramètre
│   └── ...
└── binaural/
    ├── block_sizes/
    │   └── 512/report/index.html
    └── ...
```

```sh
task bench-open-report
```

---

## Voir aussi

- [profiling_guide.md](./profiling_guide.md) — Flamegraph, Callgrind, Heaptrack pour le profiling CPU/RAM
- [audio.md](./audio.md) — Architecture du thread audio (CPAL, Condvar, voices pool)
- [2026-07-08 - SYNTHÈSE OPTIMISATIONS AUDIO](./2026-07-08%20-%20SYNTH%C3%88SE%20D%27ARCHITECTURE%20:%20REFACTORING%20&%20OPTIMISATIONS%20AUDIO%20TEMPS%20R%C3%89EL%20%28RUST%20-%20LINUX%29.md) — Historique des optimisations lock-free, FTZ/DAZ, buffer ALSA/PipeWire
- [benches/audio_dsp_bench.rs](../benches/audio_dsp_bench.rs) — Code source des benchmarks DSP
- [benches/audio_binaural_bench.rs](../benches/audio_binaural_bench.rs) — Code source des benchmarks binaural
