# Note de Vision & ADR : Éclairage Volumétrique Global pour Rendu de Feux d'Artifice

**Date :** 2026-10-01
**Statut :** Session suspendue — idées et décisions d'architecture préservées pour reprise ultérieure
**Contexte :** Prolongement de `20260930_volumetric_smoke_lighting_spec.md` (Phase 1 : 16 lumières forward + sky haze, implémentée et mesurée)
**Branche de travail :** `feat/volumetric-smoke-lighting`

---

## 1. Constat & Problème à Résoudre

Le système actuel (Phase 1) éclaire la fumée via une liste discrète de 16 sources lumineuses extraites CPU-side et uploadées en UBO. Trois limites structurelles :

1. **Caractère discret** : l'éclairage apparaît/disparaît par éviction de slots → clignotement perçu (thrashing du top-16). La décroissance 0.94 existante ne couvre que l'extinction, ni l'entrée ni le yoyo de sélection.
2. **Plafond arbitraire** : M = 16 sources max, imposé par le coût du fragment shader (boucle par lumière) et le budget UBO.
3. **Aucune propagation** : chaque particule de fumée n'est éclairée que par les sources, jamais par la lumière renvoyée par les autres particules (pas d'indirect, pas d'occlusion).

**Objectif de réalisme** : la lumière des détonations doit habiter l'espace de manière continue — un champ — plutôt que d'être portée par des entités ponctuelles gérées manuellement.

---

## 2. Idée Majeure A — Champ Radiatif Persistant 2D (pivot architectural recommandé)

Le monde est 2D orthographique → une simple texture RGBA16F en espace-monde peut jouer le rôle de volume lumineux (déclinaison 2D d'un Light Propagation Volume), sans harmoniques sphériques.

**Architecture en 3 passes légères à ¼ de résolution (~480×270) :**

| Passe | Contenu | Sortie |
|---|---|---|
| 1. Densité | Re-render instancié de la fumée en blend additif, ne sortir que l'alpha | `smokeDensity` (R16F) |
| 2. Splat émissif | Chaque émetteur (explosions, têtes de fusée, étincelles majeures) dépose un billboard gaussien additif — plus de limite à 16, 64–128 émetteurs possibles | `lightField` (RGBA16F) |
| 3. Decay + diffusion | Ping-pong : moyenne des 4 voisins pondérée + décroissance exponentielle | `lightField` persistant |

**Équations clés (math en texte brut, compatible mdBook) :**

- Décroissance indépendante du framerate : I(t+Δt) = I(t) · e^(−k·Δt), avec k = −ln(0,94) × 60 ≈ 3,7 s⁻¹ pour reproduire le comportement actuel à 60 fps.
- Diffusion pseudo-indirecte (bleeding) : champ(t+1) = (champ + a · (moyenne_voisins − champ)) · e^(−k·Δt)

**Consommation** : la boucle de 16 lumières des fragments (fumée ET sky haze) est remplacée par une seule fetch de texture → le shader de fumée devient moins cher qu'aujourd'hui.

**Bénéfices spécifiques au besoin "sporadique" :**

- L'apparition/disparition des sources devient une propriété naturelle du champ (décroissance du texel) → tout le §3.5 du spec existant (slots stables par rocket.id, EMA, continuité C¹) devient gratuit et supprimable.
- Splat de la tête de fusée chaque frame → traînée de montée éclairée en continu (effet comète).
- Extension : splats étirés selon la vélocité → traînées lumineuses d'étincelles (motion blur dans le champ).

**Pièges identifiés** : rayon de splat ≥ 1,5 texel (sinon aliasing) ; clamp d'intensité (~64) pour ne pas saturer le RGBA16F ; aligner la conversion monde→UV sur la convention Y-up du sky haze ; laisser le bloom absorber l'excès.

**Coût estimé** : +0,15–0,30 ms à 1080p sur iGPU type Iris Xe, partiellement compensé par l'économie en fragment fumée. Compatible zero-cost bypass : devient un tier de qualité, l'UBO 16 lumières restant le fallback "low".

---

## 3. Idée Majeure B — Radiance Cascades 2D (option haut de gamme)

Référence : Sannikov 2023, implémentations 2D connues (Godot, Shadertoy). Le seul choix donnant la vraie GI 2D :

- Occlusion par la fumée incluse : la texture `smokeDensity` sert d'occluder → la lumière filtre à travers les trouées du nuage.
- Indirect gratuit (lumière renvoyée par les particules elles-mêmes).
- Émetteurs = les particules (texture émissive persistante avec decay) → les flashs sporadiques ne peuvent structurellement pas "popper".
- Hiérarchie de 4–6 cascades, ½ résolution + accumulation temporelle.
- Coût réaliste iGPU : 0,5–1,5 ms → tier "ultra" uniquement. Complexité d'implémentation élevée.

---

## 4. Améliorations Écran (quick wins réels, indépendants de l'architecture lumière)

1. **Backlight screen-space** — meilleur ratio coût/impact pour des feux d'artifice. Principe : réutiliser la bright-pass du bloom (déjà calculée et floutée) multipliée par un masque de densité de fumée en screen-space → la fumée capte le flash situé derrière elle et s'illumine en silhouette (nuage devant le soleil). Variante anisotrope : radial blur de la bright-pass centré sur les 2–3 flashs majeurs = god-rays à travers la fumée. Prérequis déjà en place : attachement MRT libre (COLOR_ATTACHMENT2) → smoke mask quasi gratuit. Coût : ~0,05 ms.
2. **Auto-exposure / adaptation oculaire** — luminance moyenne via mip chain + EMA, appliquée au tonemap existant (Reinhard/ACES/AgX déjà en place). Un flash sature 100–200 ms puis l'image "se réadapte" : c'est ce qui vend le caractère violent et sporadique des détonations. Design GPU-only (reduce vers texture 1×1 ping-pong, zéro readback CPU).
3. **Fake multi-scattering** (hack Wrenninge/Hillaire) — somme d'octaves de la fonction de phase : p_eff(θ) ≈ p(g) + 0,25·p(g/2) + 0,06·p(g/4), plus ambient légèrement rehaussé. Effet : la fumée garde une luminescence résiduelle après le flash au lieu de retomber à zéro sec.
4. **Canal afterglow** — énergie ambiante à double dynamique : attaque instantanée (afterglow = max(afterglow, cible)), décroissance lente (×0,985/frame), clamp de sécurité. → braise résiduelle 1–2 s après chaque détonation, dans la teinte de la charge.
5. **Ombres portées dans la fumée** — au moment du splat, 4–6 taps de densité le long du segment source→particule → transmittance T = e^(−Σ ρ·Δ). Version pro : moments / variance shadow map (2 fetches, transmittance douce sans raymarching).
6. **Dither anti-banding IGN statique** — interleaved gradient noise ajouté en toute fin de pipeline LDR (post-gamma, pré-quantification 8-bit), amplitude ~0,6/255, phase gelée. Dissout les marches d'escalier des halos sur fond noir en grain fixe imperceptible.

---

## 5. Stabilisation Temporelle des Sources (correctif direct du flicker actuel)

Complément à la décroissance 0.94 existante, dans update_lighting_ubo (bypassé par le master switch) :

- **Hystérésis d'éviction** : un slot occupé ne peut être perdu que face à une candidate dont l'intensité dépasse 1,2× la sienne (ou si aucun slot libre) → supprime le yoyo du top-16.
- **Fade-in** : toute nouvelle lumière entrant dans un slot monte de 0 à sa cible en ~50 ms (linéaire) → plus de pop à l'apparition.

---

## 6. REX de la Session du 2026-10-01 (à ne pas reperdre)

1. **Dither animé rejeté en runtime** : un bruit re-tiré à chaque frame SANS accumulation temporelle (pas de TAA) = scintillement plein écran. La recette IGN animée (Jimenez 2014) est conçue pour du TAA qui moyenne le grain. En caméra fixe sans accumulation → dither statique uniquement. Le dither animé a par ailleurs rendu plus saillant le flicker préexistant des lumières (problème distinct = §5).
2. **Méthode de validation validée** : chaque feature doit exposer son toggle + slider ImGui (pattern des sections existantes du panneau F4, recherche par mot-clé, GUI_PERSIST + console) → comparaison ON/OFF instantanée dans la même frame, sans rebuild. C'est le protocole A/B du projet.
3. **Workflow agent** : spec fermée + périmètre de fichiers + commit local sans push, validation runtime humaine avant push. Rapport agent = info métier (pièges, décisions, risques), pas d'inventaire git.

---

## 7. Roadmap Proposée (à la reprise)

| Phase | Contenu | Dépendance | Coût estimé (1080p, Iris Xe) |
|---|---|---|---|
| A | Stabilisation sources (§5) + dither statique (§4.6) | rien | ~0 |
| B | Backlight screen-space (§4.1) — smoke mask via COLOR_ATTACHMENT2 | rien | +0,05 ms |
| C | Auto-exposure (§4.2) + fake multi-scatter + afterglow (§4.3–4.4) | rien | +0,1 ms |
| D | Champ radiatif persistant 2D (§2) — remplace les boucles 16 lumières | rien | +0,15–0,30 ms net |
| E | Ombres/transmittance au splat (§4.5) | D | +0,1 ms |
| F | Radiance Cascades 2D, tier "ultra" (§3) | D (densité) | +0,5–1,5 ms |

Chaque phase = 1 toggle ImGui indépendant, benchable avec le protocole existant (Criterion + glFinish, 1000 frames, iso-develop en bypass).

---

## 8. Ouvertures Futures (hors périmètre 2D actuel)

- Passage 3D éventuel → froxel grid 3D ou LPV 3D avec harmoniques sphériques ordre 2 : +1–3 ms, pertinent seulement si la caméra devient libre.
- Éclairage des fusées/étincelles elles-mêmes par le champ (feedback particules←lumière).
- Bloom énergétique : seuil de luminance piloté par l'afterglow pour lier bloom et média participatif.
