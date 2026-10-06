# Rapport d'Autopsie Technique : Buffer d'Éclairage Volumétrique Basse Résolution (Phase 2)

**Date :** 6 Octobre 2026  
**Auteur :** Antigravity AI Agent  
**Branche :** `feat/volumetric-smoke-lighting`  
**Cible Matérielle :** Mesa Intel(R) Iris(R) Xe Graphics (RPL-U) (iGPU)  
**Statut :** ❌ **REJETÉ & ANNULÉ (ROLLBACK INTÉGRAL EN VERTU DU PILIER 3 ZERO-TRUST & PERTE QUALITÉ VISUELLE INACCEPTABLE)**

---

## 1. Contexte & Hypothèse Initiale

Dans la feuille de route d'optimisation de l'éclairage volumétrique de la fumée (`SmokeRenderer`) et de la brume atmosphérique (`SkyHaze`), la **Phase 2** prévoyait de découpler le calcul d'illumination de la géométrie des particules en utilisant un **Buffer d'Éclairage Volumétrique Basse Résolution (Low-Res Volumetric Light Buffer)** :

### Hypothèse théorique formulée :
Plutôt que d'évaluer la boucle d'accumulation de lumière ponctuelle dans le vertex shader de chaque particule de fumée (`smoke_instanced.vert.glsl`) et dans le fragment shader plein écran de la brume (`sky_haze.frag.glsl`) :
1. Rendre l'illumination volumétrique diffuse dans un FBO basse résolution hors-écran (Multiple Render Targets : `RT0` pour le sky haze, `RT1` pour la lumière diffuse de fumée) :
   - Facteur $1/2$ (Half-Res : $960 \times 540$)
   - Facteur $1/4$ (Quarter-Res : $480 \times 270$)
   - Facteur $1/8$ (Eighth-Res : $240 \times 135$)
2. Échantillonner bilinéairement cette texture 2D (`u_SmokeLightTexture` et `u_SkyHazeTexture`) :
   - Au stade vertex dans le shader de fumée pour teinter les quads.
   - Au stade fragment dans le shader de brume pour le fond céleste.

L'objectif escompté était de transformer une complexité algorithmique dépendante du nombre de particules actives $O(N_{\text{lights}} \times N_{\text{particles}})$ en un coût fixe indépendant de la densité de particules $O(N_{\text{lights}} \times W_{\text{buffer}} \times H_{\text{buffer}})$.

---

## 2. Protocole de Mesure & Résultats Comparatifs sur Intel Iris Xe

### 2.1 Conditions Expérimentales Strictes (Strict ISO)
- **Plateforme matérielle :** Mesa Intel(R) Iris(R) Xe Graphics (RPL-U), pilote Mesa DRI natif X11 (`DISPLAY=:0.0`).
- **Synchronisation Verticale :** Désactivée (`vblank_mode=0 __GL_SYNC_TO_VBLANK=0`).
- **Déterminisme :** Graine pseudo-aléatoire fixée (`--deterministic-seed 42`), audio coupé (`--disable-audio`).
- **Warmup :** Pré-chauffe active pour éliminer les latences de cache et de compilation de pipeline.
- **Protocole :** 3 passages consécutifs de 5 secondes par configuration.

### 2.2 Données Chiffrées Brutes

| Configuration / Variante | FPS Moyen | Frame Time | Delta FPS vs Baseline | Gain Relatif | Qualité Visuelle Perçue |
|---|:---:|:---:|:---:|:---:|:---:|
| **BASELINE (Commit `4167c3a`)**<br>• Fast-Math RSQ (`inversesqrt`)<br>• Frustum Culling Vertex<br>• Compaction UBO Lumières Actives | **560.20** | **1.785 ms** | Ref | Ref | **Excellente (Nette, fluide, haute fidélité)** |
| **TARGET Phase 2 (Half-Res 1/2)**<br>• FBO $960 \times 540$<br>• Échantillonnage bilinéaire | 438.86 | 2.279 ms | -121.34 FPS | **-21.66 %** (Régression vs UBO pur)<br>*+0.60 % vs baseline non compactée* | **Moyenne** (Flou diffus, perte de relief) |
| **TARGET Phase 2 (Quarter-Res 1/4)**<br>• FBO $480 \times 270$<br>• Échantillonnage bilinéaire | 464.40 | 2.153 ms | -95.80 FPS | **-17.10 %** (Régression vs UBO pur)<br>*+6.45 % vs baseline non compactée* | **Inacceptable** (Scintillement, popping, aliasing) |
| **TARGET Phase 2 (Eighth-Res 1/8)**<br>• FBO $240 \times 135$<br>• Échantillonnage bilinéaire | ~470.00 | ~2.127 ms | -90.20 FPS | -16.10 % | **Délétère** (Banding massif, discontinuités) |

---

## 3. Autopsie Technique : Pourquoi cette Optimisation est un Échec

L'évaluation conjointe de la qualité de rendu et du profil d'exécution GPU met en lumière deux défauts rédhibitoires :

### 3.1 Dégradation Visuelle Inacceptable & Aliasing Temporel (Popping)
1. **Perte des Hautes Fréquences Spatiales :**
   Les feux d'artifice sont caractérisés par des sources lumineuses ponctuelles intenses et mobiles (étincelles et fusées). En divisant la résolution de l'illumination par 4 ($480 \times 270$) ou 8 ($240 \times 135$), l'interpolation bilinéaire lisse excessivement le gradient d'intensité.
2. **Scintillement Temporel et Décrochage (Popping) :**
   Lorsqu'une fusée traverse rapidement l'écran, son centre lumineux saute d'un texel basse résolution à l'autre. Ce sous-échantillonnage spatial brutal engendre des oscillations violentes d'intensité sur les particules de fumée avoisinantes, provoquant un phénomène de popping et de scintillement visuel agressif.
3. **Inadéquation du Mode Half-Res (1/2) :**
   Le mode 1/2 réduit les artefacts les plus grossiers mais conserve un rendu pâteux et flou qui dégrade la lisibilité des volutes de fumée, tout en affichant un débit inférieur de plus de 120 FPS par rapport à la baseline compactée.

### 3.2 Le Piège de l'Overhead FBO Hors-Écran sur GPU UMA
1. **Pénalité Fixe de Commutation FBO :**
   L'introduction d'un FBO supplémentaire impose des barrières de pipeline OpenGL : `glBindFramebuffer`, `glViewport`, dispatch d'un quad plein écran, flush mémoire, puis rebinding vers le FBO HDR principal. Sur une architecture UMA (Intel Iris Xe) où le bus mémoire est partagé avec le processeur, ces allers-retours FBO consomment de la bande passante sans gain arithmétique.
2. **Redondance face à la Compaction UBO (`4167c3a`) :**
   La Phase 1 combinée à la compaction UBO active a déjà éradiqué le goulet d'étranglement initial : seules les sources réelles sont parcourues, les calculs sont vectorisés en 1 cycle via `inversesqrt()`, et le culling rejette les particules hors-champ. Dès lors, le coût de la boucle de lumière au stade vertex est devenu tellement faible que le coût fixe de la passe FBO basse résolution s'avère supérieur au coût de calcul direct !

---

## 4. Décision d'Ingénierie & Application du Pilier 3 (Zero-Trust)

Le **Pilier 3 du manifeste `AGENTS.md`** et la Règle d'Or n°6 stipulent expressément :
- *Rejet Immédiat & Rollback en cas de régression ou de gain insuffisant face au bruit de mesure.*
- *Préservation impérative de l'intégrité visuelle et respect des standards graphiques du projet.*

En conséquence :
1. **Rejet Formel de la Phase 2 :** L'approche par buffer d'éclairage volumétrique basse résolution est définitivement rejetée.
2. **Rollback Intégral :**
   - Suppression du composant `VolumetricLightBuffer` et de ses shaders associés (`volumetric_light_buffer.frag.glsl`).
   - Nettoyage des commandes console (`renderer.volumetric_buffer*`), des messages CQRS (`RendererCommand::SetVolumetricBuffer*`) et des contrôles ImGui associés.
   - Restauration des fichiers de production (`git restore .`).
3. **Consolidation de la Baseline Canonique :**
   Le pipeline graphique conserve l'architecture optimale validée au commit `4167c3a` :
   - **Éclairage direct par particule au stade vertex** (zéro aliasing spatial, zéro popping temporel, continuité parfaite).
   - **Compaction UBO CPU/GPU** (zéro itération morte).
   - **Fast-Math RSQ** (zéro division / racine carrée lente).
   - **Frustum Culling GPU** et **Bypass du masque de rétro-éclairage**.
   - **Débit maximal mesuré :** **560.20 FPS (1.785 ms)**.

---

## 5. Runbook de Reproductibilité Humaine

```bash
# 1. Vérification de l'intégrité du code source (arborescence propre)
git status

# 2. Vérification des linters et du formatage
task lint:all

# 3. Validation de la conformité des inventaires GUI
task test:gui-persistence-check

# 4. Exécution de la suite complète de non-régression
task test:all
```
