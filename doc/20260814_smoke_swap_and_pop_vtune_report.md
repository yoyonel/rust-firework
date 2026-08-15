# Rapport de Profiling VTune : Tentative de Mutualisation SmokeSystem Swap-and-Pop

**Date** : 2026-08-14
**Sujet** : Tentative d'extraction de la boucle Swap-and-Pop dans `smoke_system.rs` pour répliquer les gains obtenus sur `rocket.rs`.

## 1. Protocole et Commandes
**Commande d'exécution stricte (Fast-Forward) :**
```bash
task profile:vtune -- --deterministic-seed 42 --fixed-dt 0.016666 --timeout-secs 10 --disable-audio
```
*(Note: La Baseline inclut l'optimisation préalable de `rocket.rs` pour isoler uniquement le delta de `smoke_system.rs`)*

## 2. Tableaux des Métriques Normalisées (par Frame)

### Baseline (SmokeSystem avec Swap-and-Pop manuel inline)
* **Frames Totales** : 3883
* **CPU Time (Total)** : 4.209s
* **Loads (Total)** : 1,742,952,287
* **Stores (Total)** : 769,823,094

| Métrique par Frame | Valeur |
|--------------------|--------|
| **CPU Time / frame** | 1.083 ms |
| **Loads / frame**    | 448 867 |
| **Stores / frame**   | 198 254 |

### Optimisée (Fonction mutualisée `apply_smoke_swap_and_pop`)
* **Frames Totales** : 3980
* **CPU Time (Total)** : 4.348s
* **Loads (Total)** : 2,052,561,575
* **Stores (Total)** : 919,927,597

| Métrique par Frame | Valeur |
|--------------------|--------|
| **CPU Time / frame** | 1.092 ms |
| **Loads / frame**    | 515 719 |
| **Stores / frame**   | 231 137 |

## 3. Analyse et Évolution (Delta)

| Métrique | Baseline | Optimisée | Delta | Statut |
|----------|----------|-----------|-------|--------|
| CPU Time / frame | 1.083 ms | 1.092 ms | **+0.8%** | 🔴 Bruit/Légère Régression |
| Loads / frame | 448 867 | 515 719 | **+14.8%** | 🔴 Régression |
| Stores / frame | 198 254 | 231 137 | **+16.5%** | 🔴 Régression |

## 4. Interprétation Architecturale & Décision (Fail-Fast)
Contrairement aux particules basiques, `SmokeParticle` est une structure beaucoup plus lourde en mémoire. L'extraction de la logique de parcours dans une fonction générique (prenant `&mut [SmokeParticle]` et `&mut usize`) n'a pas produit l'isolation attendue par l'Alias Analysis de LLVM. Au contraire :
1. Les vérifications de bornes (Bounds Checks) ont potentiellement été réactivées ou moins bien élidées par LLVM, forçant la génération de `Loads/Stores` de contrôle supplémentaires (+15%).
2. Le compilateur a probablement refusé d'inliner complètement ou a paniqué sur l'analyse de dépendance des pointeurs (emprunt mutable de `active_count` combiné à la slice).

**Conclusion :** Conformément à l'Invariant Holistique de Performance et au protocole Zero-Trust, cette optimisation est **REJETÉE**. La lisibilité ne doit pas primer sur une régression matérielle de 15% du flux mémoire. Le fichier `smoke_system.rs` doit être restauré à l'original (Rollback).
