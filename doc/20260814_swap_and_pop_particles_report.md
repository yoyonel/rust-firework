# Optimisation Swap-and-Pop des Systèmes de Particules

## 1. Rationnel Architectural et Mathématique

Suite à la suppression des branchements conditionnels (`if !p.active { continue; }`) dans la hotloop de mise à jour physique, le moteur bénéficiait d'une vectorisation SIMD maximale au détriment d'un calcul "à vide" sur la totalité des particules mortes ou non-encore instanciées de chaque slice (jusqu'à 90% de déchets calculatoires en fin de vie d'une explosion).

L'implémentation du pattern **Swap-and-Pop** (partitionnement des données actives) permet de réconcilier la densité des données exigée par le SIMD et l'élimination des calculs superflus. 

**Stratégie Hybride en 2 passes (Data-Oriented Design) :**
1. **Pass Mathématique (SIMD) :** Itération inconditionnelle et purement scalaire/vectorisée, restreinte **uniquement** à la sous-slice des particules vivantes (`&mut slice[..active_count]`).
2. **Pass de Nettoyage (Swap-and-Pop) :** Identification des particules mortes (`life <= 0.0`), échange O(1) avec la dernière particule de la slice active (`swap(i, active_count - 1)`), et décrémentation de `active_count`.

Cette architecture a été appliquée transversalement sur l'ensemble des systèmes émetteurs :
- Les trainées (`Trail`) des fusées.
- Les éclats d'explosions (`Explosion`) des fusées.
- Le système volumétrique de fumée (`SmokeSystem`), abandonnant au passage l'ancien modèle fragmenté de "Free List" (`free_indices`) au profit d'un tableau 100% dense garantissant l'itération linéaire.

## 2. Preuves Comparatives A/B (VTune Profiler)

Les captures Intel VTune CPU Hotspots valident sans équivoque le gain exponentiel apporté par l'élimination du traitement des particules mortes.

**Baseline (Vectorisation pure, avec calcul des particules mortes) :**
- `<fireworks_sim::physic_engine::rocket::Rocket>::update` : **0.458s** (8.1% du temps CPU global)

**Optimisation Swap-and-Pop (Vectorisation restreinte aux particules vivantes) :**
- `<fireworks_sim::physic_engine::rocket::Rocket>::update` : **0.172s**

**Bilan :** 
- **Réduction de 62.4%** du temps CPU brut alloué à la mise à jour physique des fusées et de leurs particules intrinsèques (Trails & Explosions).
- `SmokeSystem::update` est totalement sorti du Top 15 des hotspots CPU.

## 3. Runbook de Reproductibilité

Le runbook suivant permet d'auditer et de vérifier localement ces gains via instrumentation matérielle :

```bash
# Compilation du binaire avec symboles de débogage et optimisations agressives
task build:profiling

# Lancement de l'échantillonneur Intel VTune (Hotspots) - Requiert privilèges sudo
# La session collecte les évènements PMU pendant la durée de vie du script d'automatisation
task profile:vtune-hotspots

# Analyse du rapport
# Le temps CPU alloué aux fonctions de mise à jour physique doit être comparé à la trace historique
```
