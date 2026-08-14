# Rapport d'Optimisation : Physique Branchless & Pipeline Superscalaire (Rocket::update)

## 1. Rationnel et Diagnostic (VTune Hotspot)
Le profilage VTune a mis en évidence que `<Rocket>::update` consommait environ 7.4% à 8.0% du temps CPU total.
L'analyse de l'implémentation de la physique (`integrate_trail_particles` et `update_explosions`) a montré une limitation de l'architecture **AoS (Array of Structs)**. Le layout mémoire de 48 octets (`Particle`), requis pour le binding direct vers OpenGL (AZDO), empêche le compilateur LLVM d'utiliser efficacement des instructions SIMD (AVX2).

Plus grave, l'implémentation contenait une branche conditionnelle très fine dans la boucle chaude :
```rust
if !p.active {
    continue;
}
```
Ce branchement conditionnel empêchait le pipeline superscalaire du processeur de s'exécuter à pleine capacité (casses du pipeline sur les prédictions ratées, impossibilité de dérouler la boucle efficacement et de cacher la latence mémoire).

## 2. Optimisation : Inconditional Physics (Branchless)
Nous avons supprimé cette branche. L'intégration mathématique (gravité, mise à jour des positions, gestion de la durée de vie) s'exécute désormais **inconditionnellement** sur toutes les particules du bloc alloué, qu'elles soient mortes ou vivantes.

Les particules "mortes" (`!p.active`) subissent donc les mathématiques de physique et leur coordonnée Y s'enfonce vers `-Inf`, mais cela n'a aucun impact métier : 
1. Le code de rendu (Graphics Renderer) omet les particules dont `p.active == false`.
2. Le système de cycle de vie (spawning) écrase proprement toutes ces propriétés avec de nouvelles valeurs valides lorsqu'une particule est recyclée/ressuscitée.

## 3. Le Pari sur la Densité (Sparsité vs Scaling)
L'optimisation repose sur un compromis fondamental concernant la bande passante mémoire :

*   **L'inconvénient assumé** : Les particules mortes subissent des calculs inutiles, mais surtout des **écritures mémoire (Write-back)** qui marquent leur ligne de cache (Cache-Line) comme *"Dirty"*. 
*   **Le pari (Densité)** : Le `ParticlesPool` alloue les particules par petits blocs adaptés à chaque fusée. En moyenne, durant le vol d'une fusée, environ **50% des particules du bloc sont actives**. Le surcoût d'écriture inutile pour les 50% de mortes est **largement compensé** par l'efficacité du pipeline CPU (instructions ininterrompues) pour les 50% vivantes.
*   **Alerte de Dégradation** : Si l'architecture d'allocation mémoire changeait (ex: passage à un énorme Pool Global pré-alloué) entrainant une sparsité massive (ex: 95% ou 99% de particules mortes en permanence), cette optimisation "Branchless" deviendrait contre-productive. Les écritures mémoires fantômes effondreraient les performances (Memory Bound).

## 4. Preuves comparatives A/B (VTune CPU Profiler)
Les mesures ont été réalisées avec le profilage évènementiel VTune (Sampling CPU) sur `develop` vs `branche d'optimisation`.

| Cas d'usage / Profil | CPU Time (`Rocket::update`) | % Global CPU | Temps Total (Elapsed) |
|----------------------|---------------------------|--------------|-----------------------|
| Baseline (Branched)  | 0.428s - 0.450s           | ~7.5% - 8.0% | ~7.778s               |
| Optimisée (Branchless) | 0.342s                    | 6.1%         | 7.462s                |

**Conclusion** : Gain net de **~20% à 25%** sur le temps d'exécution pur de l'intégration physique.

## 5. Runbook de Reproductibilité
Pour auditer ce comportement et vérifier la santé de l'optimisation (ou détecter une future régression si la densité chute) :

```bash
# Lancer une capture VTune Hotspots (requiert les droits perf_event)
task benchmark-vtune-hotspots

# Analyser les résultats graphiquement pour cibler les fonctions de fireworks_sim
vtune-gui ./vtune_hotspots_results
```
