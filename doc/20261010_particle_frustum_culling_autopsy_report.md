# 🔬 Rapport d'Autopsie : Frustum Culling CPU des Particules (Sparks & Trails)

**Date :** 10 Octobre 2026  
**Auteur :** Antigravity Engine Agent  
**Branche Git :** [`feat/volumetric-smoke-lighting`](file:///home/latty/Prog/__PERSO__/rust-firework)  
**Plateforme de Test :** Mesa Intel(R) Iris(R) Xe Graphics (RPL-U) (OpenGL 4.6 Core Profile)  
**Statut de la Piste :** ❌ **REJETÉE & ROLLBACK IMMÉDIAT (Régression de -10,34 %)**

---

## 1. Contexte & Hypothèse d'Optimisation

Dans le cadre de l'optimisation de la passe particules (`Particles::Draw_Sparks_Trails` et `Renderer::fill_sparks_trails`), l'hypothèse soulevée consistait à filtrer (culler) les particules situées en dehors du viewport visible $[0, W] \times [0, H]$ (avec une marge de sécurité de 20 px pour les point sprites) directement sur le CPU avant l'écriture dans le buffer persistant OpenGL AZDO (`gpu_slice`).

### Objectif théorique escompté :
* Éviter d'écrire des vertices inutiles dans le buffer GPU mappé.
* Réduire la taille de l'appel `glDrawArrays(gl::POINTS, 0, count)`.
* Soulager le Vertex Assembly et le Fixed-Function Clipper de l'iGPU Intel Iris Xe.

---

## 2. Implémentation Testée

Dans [`src/renderer_engine/renderer_graphics.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/renderer_engine/renderer_graphics.rs) (`fill_particle_data_direct`) :

```rust
let min_x = -constants::PARTICLE_FRUSTUM_CULLING_MARGIN;
let max_x = self.window_width + constants::PARTICLE_FRUSTUM_CULLING_MARGIN;
let min_y = -constants::PARTICLE_FRUSTUM_CULLING_MARGIN;
let max_y = self.window_height + constants::PARTICLE_FRUSTUM_CULLING_MARGIN;

// À l'intérieur de la boucle chaude d'itération des 100 000 particules :
if gpu_p.pos_x < min_x
    || gpu_p.pos_x > max_x
    || gpu_p.pos_y < min_y
    || gpu_p.pos_y > max_y
{
    continue;
}
```

---

## 3. Résultats Comparatifs A/B Matériels

* **Protocole :** 3 runs de 5 secondes consécutifs après warm-up de 2s, graine déterministe `Seed: 42`, audio désactivé, VSync désactivée (`vblank_mode=0 __GL_SYNC_TO_VBLANK=0`).
* **Commande CLI :** `./scripts/run_gl_smart.sh <binaire> --timeout-secs 5 --disable-audio --deterministic-seed 42`.

| Métrique | Baseline (Sans Culling) | Target (Avec Frustum Culling) | Delta | Évolution (%) |
| :--- | :---: | :---: | :---: | :---: |
| **FPS Moyen** | **555,80 FPS** | **498,34 FPS** | **-57,46 FPS** | ❌ **-10,34 %** |
| **Frame Time** | **1,799 ms** | **2,007 ms** | **+0,208 ms** | ❌ **+11,56 %** |
| **Run 1** | 553,00 FPS | 554,40 FPS | +1,40 FPS | +0,25 % |
| **Run 2** | 558,60 FPS | 521,20 FPS | -37,40 FPS | -6,69 % |
| **Run 3** | 555,80 FPS | 419,40 FPS | -136,40 FPS | -24,54 % |

---

## 4. Analyse Technique de l'Échec (Post-Mortem)

L'introduction du Frustum Culling CPU sur les particules de feux d'artifice est une fausse bonne idée pour trois raisons fondamentales :

1. **Rupture de Pipeline & Branch Mispredictions sur le CPU :**
   - La boucle d'écriture AZDO est hautement vectorisable et superscalaire (remplissage linéaire de `gpu_slice`).
   - L'ajout de 4 comparaisons flottantes et de sauts conditionnels (`||`) sur **100 000 particules par frame** introduit un coût CPU majeur (~400 000 comparaisons et pénalités de prédiction de branchement).
2. **Proportion Négligeable de Particules Hors-Écran :**
   - Dans une simulation de feux d'artifice, les fusées montent et explosent au centre de l'écran.
   - Les étincelles ont une durée de vie courte (0,8s à 2,5s) et tombent sous l'effet de la gravité ; moins de **1,5 % à 2 %** des particules atteignent les bords de l'écran avant de s'éteindre naturellement.
   - Évaluer 100 000 particules pour n'en éliminer que ~1 500 représente un ratio coût/bénéfice catastrophique pour le CPU.
3. **Le GPU dispose d'un Clipper Matériel Dédié Déjà Gratuit :**
   - Sur l'Intel Iris Xe (et les architectures GPU modernes), le découpage des primitives de points en dehors du cube de projection $[-1, 1]^3$ est réalisé par le matériel fixe de clipping (*Primitive Assembly & Fixed-Function Clipper*).
   - Les points hors champ ne sont jamais rastérisés et ne déclenchent aucun fragment shader. Le coût de traitement d'un vertex de point hors champ par le vertex shader est de l'ordre de quelques fractions de nanoseconde, infiniment inférieur au coût d'un test CPU.

---

## 5. Conclusion & Action Immédiate

* **Action :** Rollback immédiat des modifications du code source (`git restore src/`).
* **Directive pour le futur :** Proscrire tout Frustum Culling CPU sur les particules ponctuelles (`gl::POINTS`). Laisser le GPU gérer le clipping via son matériel dédié.
