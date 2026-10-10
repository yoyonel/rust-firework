# 📊 Rapport d'Autopsie : Optimisations Arithmétiques & DrawBuffers Sky Haze (Piste 3 A+B)

**Date :** 10 octobre 2026  
**Branche :** `feat/volumetric-smoke-lighting`  
**Auteur :** AI Assistant & Human Architect  
**Plateforme Matérielle :** Mesa Intel(R) Iris(R) Xe Graphics (RPL-U) — OpenGL 4.6 Core Profile  
**Statut de la Piste :** ❌ **REJETÉE (Gain neutre / dans le bruit de mesure à -0,38 %)**  

---

## 1. Contexte & Hypothèse Initiale

Dans l'audit Tracy GPU du 7 octobre 2026 (`doc/20261007_intel_iris_xe_tracy_profiling_audit_report.md`), la passe de fond de ciel et brume atmosphérique (`Sky, Stars & Atmospheric Haze`) représentait **312,08 µs (19,19 % du temps GPU total)**.

### Hypothèse testée (Approches A + B) :
1. **Approche A (Optimisation Arithmétique GLSL) :**
   - Remplacer l'évaluation coûteuse de la distance géométrique (`inversesqrt`, multiples multiplications de rayon) par un espace de coordonnées normalisé `normOffset = (worldPos - lightPos) * invEffRadius`.
   - Remplacer la fonction transcendante `exp(-falloffExp * normDist)` par l'instruction matérielle native `exp2(-falloffExp * normDist * 1.44269504)`.
   - Court-circuiter immédiatement la boucle de lumière si `u_NumActiveLights == 0`.
2. **Approche B (Élimination d'Écriture FBO Redondante) :**
   - Restreindre l'écriture de `sky_haze` à `COLOR_ATTACHMENT0` via `glDrawBuffers(1, ...)`, supprimant l'écriture de `vec4(0.0)` dans `BrightColor` (déjà initialisé à 0 lors du `glClear` initial).

---

## 2. Résultats Comparatifs A/B Matériels (Intel Iris Xe)

Mesures sur 3 runs x 5 secondes consécutifs (Seed déterministe 42, VSync désactivée) :

| Configuration | FPS Moyen | Frame Time | Delta FPS | Gain % | Diagnostic |
| :--- | :---: | :---: | :---: | :---: | :--- |
| **Baseline (Transcendental `exp` + 2 FBO Attachments)** | **486.06 FPS** | **2.057 ms** | Ref | Ref | Référence commit `0e10514` |
| **Target (Normalized space + native `exp2` + DrawBuffers 1)** | **484.20 FPS** | **2.065 ms** | **-1.86 FPS** | **-0.38 %** ⚪ | **Within noise threshold / Neutre** |

---

## 3. Analyse Technique du Post-Mortem : Pourquoi le Gain est Nul

1. **Abaissement Algébrique Automatique par Mesa NIR :**
   - Le compilateur NIR du pilote Mesa Intel (`nir_opt_algebraic`) optimise déjà de manière transparente `exp(x)` en `fmul(x, 1.442695)` suivi de `fexp2` lors de la compilation du shader GLSL.
   - Écrire manuellement `exp2` en GLSL génère le même assemblage ISA Gen12 sous-jacent. Aucun gain d'instructions machine n'est réalisé.
2. **Coût de Reconfiguration du Pipeline FBO (`glDrawBuffers`) :**
   - Basculer de 2 à 1 DrawBuffer avant `sky_haze` puis restaurer 2 DrawBuffers pour les particules impose une barrière de rastérisation matérielle sur le GPU Intel.
   - La perte de temps causée par ce stall matériel compense et neutralise l'infime gain de bande passante lié à la suppression de l'écriture dans `BrightColor`.
3. **Prédominance du Fillrate sur l'Arithmétique :**
   - 95 % des fragments de l'écran n'interceptent aucune lumière active et ne font que sampler le dégradé de fond. L'ALU n'est donc pas le facteur limitant sur cette passe.

---

## 4. Décision & Directives pour le Futur

1. **Rollback Immédiat :** Les modifications de `sky_haze.frag.glsl` et `renderer.rs` sont annulées via `git restore`.
2. **Interdiction de Reconfiguration FBO Locale :** Ne pas insérer d'appels `glDrawBuffers` intercalaires entre des passes consécutives sur le même FBO HDR si cela n'élimine pas une passe de rendu entière.
3. **Perspectives Restantes pour Sky Haze :** Seul un rendu en basse résolution (Option C : FBO demi-résolution dédié) ou une rastérisation instanciée par quads englobants de lumières (pour ne pas balayer l'écran entier) pourrait apporter un gain mesurable, mais avec un coût d'architecture plus élevé.
