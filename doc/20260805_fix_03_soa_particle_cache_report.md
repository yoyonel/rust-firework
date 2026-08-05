# Rapport Technique & ADR FIX-03 : Analyse & Rejet de la Migration SoA (Structure of Arrays)

**Date :** 5 Août 2026  
**Branche Origine :** `feat/fix-03-soa`  
**Statut Final :** ❌ **REJETÉ / ABANDONNÉ (AoS Maintenu)**  
**Auteur :** Antigravity AI & Équipe Ingénierie  

---

## 1. Contexte & Hypothèse Initiale

L'objectif de FIX-03 était de refactoriser la structure `Particle` (64 octets alignés) en **Structure of Arrays (SoA)** dans `ParticlesPool` afin d'augmenter la densité du cache L1 lors des boucles d'intégration physique (`pos += vel * dt`).

### Hypothèse Théorique
En séparant les champs chauds (`pos_x`, `pos_y`, `vel_x`, `vel_y`, `life`, `active`) des champs froids (`color`, `max_life`, `size`, `angle`, `particle_type`), le balayage mémoire lors du calcul physique devait utiliser 100% de chaque ligne de cache L1 (64 octets).

---

## 2. Résultats de l'Expérimentation & Double Mesure

### A. Micro-Benchmark Isolée (`physic_pool_bench` — Physique Pure)
Les résultats Criterion sur le micro-benchmark d'intégration physique seule indiquaient un gain net :

| Métrique | Baseline AoS (`develop`) | Target SoA (`feat/fix-03-soa`) | Écart |
| :--- | :---: | :---: | :---: |
| **`particle-update-25k`** | `1.8381 ms` | `1.1234 ms` | 🟢 **-38.88% (Gain +63.6%)** |

### B. Macro-Benchmark Holistique (`simulator_full_bench` — Moteur Complet)
Lors de l'évaluation sur le cycle complet de l'application (Physique + Upload VBO GPU + Rendu + Audio), les résultats Criterion ont révélé un **piège d'optimum local** :

| Charge (Fusées) | Baseline AoS (`develop`) | Target SoA (`feat/fix-03-soa`) | Statut Criterion |
| :--- | :---: | :---: | :---: |
| **N = 10** | `495.2 µs` | `552.1 µs` | ❌ **Régression (+11.48%)** ($p = 0.01$) |
| **N = 50** | `637.6 µs` | `586.5 µs` | 🟢 Amélioration (-8.01%) ($p = 0.00$) |
| **N = 200** | `905.3 µs` | `950.0 µs` | ⚪ **Aucun changement** ($p = 0.43$) |
| **N = 1000** | `2.87 ms` | `2.69 ms` | ⚠️ **Dans le bruit de mesure** (-6.34%, $p = 0.02$) |
| **N = 4000** | `4.67 ms` | `4.49 ms` | ⚠️ **Dans le bruit de mesure** (-3.86%, $p = 0.05$) |

---

## 3. Analyse de la Défaillance (Pourquoi la SoA est une Fausse Bonne Idée ici)

1. **Perte du Fast Cast-Copy GPU en AoS :**  
   Dans le modèle **AoS legacy**, `Particle` est structuré pour s'aligner exactement sur `ParticleGPU`. Le renderer effectue un cast direct `*(p as *const ParticleGPU)` (une simple copie mémoire contiguë ultra-rapide).  
   En **SoA**, la passe de rendu doit lire dans 13 sous-tableaux différents pour reconstruire les attributs de sommets, ce qui détruit le Fast Cast-Copy et réintroduit des cache-misses pendant l'upload VBO.

2. **Ratio Temps Physique vs Rendu GPU :**  
   À 793 FPS (1.26 ms/frame), la physique CPU ne représente que ~50 µs (< 4% du temps de frame). Économiser 19 µs sur la physique est insignifiant par rapport au coût du rendu GPU et de l'assemblage des VBOs.

---

## 4. Décision & Invariants Produits

- **Décision :** Rejet définitif du refactoring SoA. Conservation de la structure contiguë **AoS** dans `ParticlesPool`.
- **Nouveau Principe Manifeste (`AGENTS.md`) :** Ajout de l'**Invariant de Performance Holistique (Pilier 3)** interdisant de valider un refactoring sur un micro-benchmark s'il ne montre pas d'amélioration sur le benchmark macro complet (`simulator_full_bench`).
- **Nouveau Principe Profiling GPU (Pilier 2) :** Directives explicites pour le nommage des ressources OpenGL (`glObjectLabel`), les debug groups RenderDoc (`gpu_profile_zone!`) et la capture CLI `renderdoccmd`.
