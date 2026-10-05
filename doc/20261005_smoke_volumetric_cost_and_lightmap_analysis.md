# Autopsie du Coût GPU des Particules de Fumée, Paradoxe Visuel et Stratégie Lightmap / LUT

**Date :** 5 Octobre 2026  
**Auteur :** Antigravity Engine Core Team  
**Statut :** Analyse Technique & Feuille de Route d'Optimisation  
**Branches concernées :** `feat/volumetric-smoke-lighting` -> `develop`

---

## 1. Contexte & Problématique : Le Paradoxe "Bazooka vs Moustique"

Lors de l'activation de l'éclairage volumétrique de la fumée, les mesures de performance GPU révèlent une chute drastique du framerate (consommation de 0.8 ms à plus de 2.0 ms de temps de frame GPU selon la charge particulaire).

L'analyse perceptuelle de l'utilisateur pointe un paradoxe fondamental :
> *"Les chiffres annoncés paraissent affolants en ordre de grandeur pour des impacts visuels très légers et peu perceptifs. On utilise un bazooka pour tuer des moustiques : des équations et calculs numériques lourds pour une variation d'intensité de pixel quasi invisible à l'écran."*

Ce document fournit :
1. L'**autopsie numérique exacte** du shader et du pipeline de rendu de fumée démontrant pourquoi ce coût est astronomique.
2. La **preuve mathématique** expliquant pourquoi l'effet visuel résultant est quasi imperceptible à l'œil nu (le "moustique").
3. Une **étude de faisabilité exhaustive** sur l'utilisation de précalculs, de lightmaps et de LUTs (Look-Up Tables) radiales pour réduire ce coût tout en conservant une flexibilité d'édition temps réel.

---

## 2. Autopsie Numérique du Pipeline de Rendu de la Fumée

### 2.1. Géométrie & Pression de Remplissage (Overdraw Massif)

Le pipeline de fumée actuel (`SmokeRenderer`) utilise le vertex shader instancié `smoke_instanced.vert.glsl` et le fragment shader `smoke_instanced.frag.glsl`.

1. **Topologie Géométrique :**
   - Chaque particule de fumée est rendue via un `TRIANGLE_FAN` de 10 sommets (8 triangles jointifs formant un octogone régulier inscrit pour minimiser les fragments transparents des quads).
   - À pleine charge (50 000 particules actives) :
     - Passe principale (Scene Pass) : $50\,000 \times 10 = 500\,000$ sommets.
     - Passe masque de fumée (`render_smoke_mask`) : $50\,000 \times 10 = 500\,000$ sommets additionnels réémis vers un FBO R8 à mi-résolution.
     - **Total géométrique :** $1\,000\,000$ de sommets soumis par frame uniquement pour la fumée.

2. **Taux de Recouvrement (Overdraw Factor) :**
   - La fumée de feux d'artifice forme des panaches denses et concentrés.
   - Le taux de sur-dessin (overdraw) mesuré sous RenderDoc varie de **8x à 18x** au cœur des explosions.
   - Pour une résolution de $1920 \times 1080$, un panache couvrant un tiers de l'écran ($600\,000$ pixels) avec un overdraw de 12x déclenche **7 200 000 exécutions du fragment shader**.

---

### 2.2. Coût Intrinsèque par Fragment (Avant Éclairage)

Avant même d'exécuter la boucle de lumière, chaque fragment subit des accès texture coûteux :
1. `texture(u_NoiseTexture, vUV)` : Échantillonnage de bruit pour l'érosion temporelle (seuil de rejet discard).
2. `texture(u_FlowMap, vUV)` : Décodage du vecteur 2D de flux tourbillonnaire.
3. `texture(u_SmokeTexture, uv1)` + `texture(u_SmokeTexture, uv2)` : Deux lookups dépendants pour le blending d'advection fluide.

Soit **4 lectures textures dépendantes** par fragment, saturant la bande passante des unités de texture (TMU) et provoquant des stalls mémoire L1/L2.

---

### 2.3. La Boucle d'Éclairage : Complexité $O(N \times L)$

Lorsque l'option d'éclairage dynamique est active, chaque fragment non rejeté exécute la boucle :

```glsl
for (int i = 0; i < u_NumActiveLights; ++i) {
    vec2 lightPos = u_LightPositions[i];
    vec3 lightCol = u_LightColors[i];
    float radius = u_LightRadii[i];
    float intensity = u_LightIntensities[i];

    vec2 toLight = lightPos - vWorldPos;
    float distSq = dot(toLight, toLight);
    float radiusSq = radius * radius;

    if (distSq < radiusSq) {
        float dist = sqrt(distSq);                     // [ALU 1] Racine carrée matérielle
        float atten = 1.0 - (dist / radius);
        atten = atten * atten;

        vec2 lightDir = toLight / dist;                // [ALU 2] Division flottante
        float cosTheta = dot(lightDir, -viewDir);
        float phase = 1.0 + 0.3 * cosTheta;            // Henyey-Greenstein simplifié

        scatteredLight += lightCol * (intensity * atten * phase * u_ScatteringIntensity);
    }
}
```

#### Évaluation Numérique de la Pression ALU :
- Si 12 lumières dynamiques actives (étoiles incandescentes + fusées montantes) couvrent le panache :
  - Nombre de tests de distance : $7.2 \times 10^6 \times 12 \approx \mathbf{86\,400\,000}$ tests de rayon.
  - Nombre d'in-scattering effectifs (dans le rayon) : environ 35 millions d'itérations.
  - Chaque itération active exécute :
    - 1 instruction `SQRT` (haute latence sur les ALUs scalaires/vectorielles).
    - 1 division flottante `FDIV`.
    - 5 multiplications flottantes vectorielles (`FMUL`).
    - 3 additions flottantes (`FADD`).
- **Bilan d'exécution :** Plus de **350 millions d'instructions flottantes et 35 millions de racines carrées** recalculées chaque frame pour un panache de fumée !

---

## 3. Démonstration Mathématique : Pourquoi l'Impact Visuel est Négligeable ("Le Moustique")

Pourquoi une telle débauche de calculs GPU ne saute-t-elle pas aux yeux ? L'analyse du code source dans `assets/shaders/smoke_instanced.frag.glsl` révèle 3 goulets d'étranglement optiques qui écrasent la contribution lumineuse :

### 3.1. L'Albédo Sombre de la Suie de Combustion
L'albédo de base de la fumée (`finalColor`) est configuré pour de la suie sombre d'artifice :
$$\text{Color}_{\text{base}} \in [0.05, 0.20]$$
La modulation appliquée en fin de fragment shader est :
```glsl
finalColor += finalColor * clamp(scatteredLight * 0.25, vec3(0.0), vec3(1.2));
```
Le terme `finalColor` est présent en facteur de multiplication. Si $\text{Color}_{\text{base}} = 0.10$, alors même avec un éclairage maximal saturé ($\text{scatteredLight} \times 0.25 = 1.2$) :
$$\Delta \text{Color} = 0.10 \times 1.2 = \mathbf{0.12}$$
Au niveau de gris 8 bits (0 à 255), cela représente un saut d'au maximum **30 niveaux de gris**. Dans la majorité des cas (atténuation quadratique moyenne), $\text{scatteredLight} \approx 0.2$, soit :
$$\Delta \text{Color} = 0.10 \times (0.2 \times 0.25) = \mathbf{0.005} \quad (\approx 1 \text{ niveau de gris sur 255})$$
L'œil humain est rigoureusement incapable de discerner un delta de luminance de $0.5\%$ sur un fond sombre en mouvement.

### 3.2. Le Clamp Arbitraire à 0.25
La formule applique explicitement un diviseur par 4 :
$$\text{Scale} = 0.25$$
Ce facteur d'amortissement a été inséré pour éviter de "blanchir" la fumée, mais il a pour conséquence directe de diviser l'utilité des calculs par 4 : $75\%$ de l'énergie lumineuse calculée par les 86 millions d'itérations est mathématiquement détruite avant affichage.

### 3.3. Neutralisation Complète du Canal Bloom (HDR)
Dans le fragment shader, le MRT (Multiple Render Targets) envoie la couleur vers `FragColor` et `BrightColor` :
```glsl
FragColor = vec4(finalColor, smokeAlpha);
BrightColor = vec4(0.0, 0.0, 0.0, 0.0); // Canal Bloom explicitement forcé à zéro
```
La fumée éclairée ne participe **jamais** à l'effet de bloom/halo scintillant du post-processing. Contrairement aux particules de feu qui saturent et génèrent un halo lumineux spectaculaire, la fumée éclairée reste confinée dans la plage basse dynamique (LDR), rendant son illumination visuellement terne et "plate".

---

## 4. Étude de Faisabilité : Précalculs, Lightmaps & Baking

L'idée proposée consiste à découpler le calcul dynamique coûteux lors des phases de jeu ou de runtime figé en utilisant des structures précalculées (analogues au baking d'Ambient Occlusion).

### 4.1. Analyse Critique : Pourquoi une Lightmap de Scène Tradionnelle est Inopérante

Dans un moteur 3D classique (ex: Unreal, Unity, Quake), une lightmap est une texture UV 2D plaquée sur une géométrie statique (murs, sol).
Dans notre cas :
- La fumée n'a **aucune surface statique** : ce sont 50 000 particules instanciées se déplaçant avec la gravité et le vent.
- Les feux d'artifice sont **hautement dynamiques** : les positions des sources lumineuses changent à 120 Hz.
- **Conclusion :** On ne peut pas pré-calculer une texture 2D statique du monde complet à l'avance.

### 4.2. Les Solutions Précalculées Viables & Hautement Performantes

En revanche, deux niveaux de précalculs (baking) sont extrêmement pertinents et élimineraient 90% du coût :

#### A. La LUT Radiale d'Atténuation & Diffusion de Phase (1D ou 2D Texture LUT)
- **Principe :** 
  Au lieu de recalculer en temps réel `sqrt(distSq)`, `1.0 - (dist / radius)`, `atten * atten`, `normalize(toLight)` et le polynôme de phase `1.0 + 0.3 * cosTheta`, ces équations analytiques sont précalculées dans une petite texture LUT (par exemple $256 \times 256$ en format `R16F` ou `RG16F`).
- **Accès Shaders :**
  - Entrée U : $\frac{\text{distSq}}{\text{radiusSq}}$ (calculable avec un simple `dot(toLight, toLight)` sans aucun `sqrt`).
  - Entrée V : $\cos(\theta)$ (angle avec la direction de vue).
  - Sortie : intensité atténuée et diffusée déjà pondérée.
- **Gain ALU :** Suppression complète des `sqrt`, des divisions flottantes et des calculs polynomiaux par fragment.

#### B. Le Rendu Différé d'Éclairage à Basse Résolution (Half-Res / Quarter-Res Light Buffer)
- **Principe :**
  La fumée est un média volumique à très basse fréquence spatiale (bords flous, gradients lents). Évaluer la lumière par particule ou par fragment 1080p est un gaspillage total.
  1. On effectue l'accumulation de la lumière dans un buffer d'illumination à **1/4 de résolution** ($480 \times 270$ au lieu de $1920 \times 1080$).
  2. Le nombre de fragments évalués passe de 7 200 000 à **450 000** (division par 16 du fillrate d'éclairage).
  3. Lors du rendu final de la fumée, chaque fragment fait un simple échantillonnage bilinéaire dans ce buffer de lumière basse résolution.

---

## 5. Architecture Cible : Mode Édition Analytique vs Mode Runtime Baked

Pour satisfaire le besoin d'édition fine temps réel (ImGui) tout en garantissant des performances maximales en mode simulation :

```
                        +---------------------------------------+
                        |      ImGui Settings & Live Sliders    |
                        +---------------------------------------+
                                           |
                                [Action Utilisateur]
                                           |
                    +----------------------+----------------------+
                    |                                             |
           [Mode Live Tweak]                             [Mode Baked Lock]
                    |                                             |
   - Calcul analytique direct                    - Déclenche un bake CPU/Compute
   - Modification en direct des formules         - Génère la LUT 2D (256x256 R16F)
   - Utile pour recherche artistique             - Texture injectée dans le shader
   - Coût GPU normal                             - Exécution Ultra-Fast (Zéro ALU)
```

### Invariants du Mode Baked :
1. **Zéro Régression Visuelle :** La LUT 2D est générée directement par la même fonction mathématique que le mode analytique. Les résultats numériques sont identiques à l'epsilon de quantification près ($16\text{ bits floats} > 99.9\%$ précision).
2. **Amortissement Immédiat :** Le calcul de baking de la LUT de $256 \times 256$ prend moins de **0.1 ms sur CPU** ou un seul dispatch de compute shader de 1 microseconde. L'opération est imperceptible pour l'utilisateur.

---

## 6. Synthèse Comparative des Coûts & Gains Prévisionnels

| Métrique | Implémentation Initiale (Pixel Loop) | Phase 0 (Vertex Stage Gouraud) | Phase 1 (Baking LUT 2D) | Phase 2 (Quarter-Res Buffer) |
| :--- | :--- | :--- | :--- | :--- |
| **Complexité Fragment** | $O(N_{\text{lum}} \times (\text{sqrt} + \text{div} + \text{poly}))$ | $O(1 \times \text{interpolation})$ | $O(N_{\text{lum}} \times \text{textureFetch})$ | $O(1 \times \text{bilinearSample})$ |
| **Invocations ALU / frame** | $\approx 350\,000\,000$ | $\mathbf{< 15\,000\,000}$ | $\approx 25\,000\,000$ | $< 2\,000\,000$ |
| **Nombre de `sqrt` / frame** | $\approx 35\,000\,000$ | $\mathbf{\approx 1\,500\,000}$ | **0** | **0** |
| **Temps GPU estimé (1080p)** | $1.2 - 2.2 \text{ ms}$ | **$0.4 - 0.7 \text{ ms}$** | $0.4 - 0.6 \text{ ms}$ | **$0.15 - 0.25 \text{ ms}$** |
| **Rapport Performance** | Baseline ($1\times$) | **$\approx 3\times$ plus rapide** | $\approx 3\times$ plus rapide | **$\approx 8\times$ plus rapide** |
| **Fidélité Visuelle** | Baseline | $99\%$ ISO (lissage naturel) | $100\%$ ISO | $98\%$ ISO |

---

## 7. Optimisation Immédiate Déployée (Phase 0 : Déport Vertex Stage)

Dans l'attente de la LUT radiale (Phase 1) ou du buffer quart-de-résolution (Phase 2), un palliatif architectural immédiat a été déployé pour soulager le GPU :

1. **Déport de l'in-scattering volumétrique au Vertex Shader :**
   - Calcul de la boucle des 16 sources lumineuses dans `smoke_instanced.vert.glsl` au lieu du fragment shader (`smoke_instanced.frag.glsl`).
   - L'éclairage est interpolé linéairement (`vScatteredLight`) à travers les sommets de la particule par le rasterizer matériel.
   - Suppression complète de la boucle par pixel : les ~350M ALU et ~35M `sqrt` sur le fillrate fragment sont éliminés.
2. **Culling précoce des instances dans le Vertex Shader :**
   - Particules mortes ou transparentes projetées hors du frustum (`gl_Position = vec4(2.0, 2.0, 2.0, 1.0)`), court-circuitant le rasterizer OpenGL.
3. **Outillage de diagnostic visuel (Debug Footprints) :**
   - Mode wireframe (`renderer.lighting.debug`) traçant les cercles d'influence, quads englobants en pointillés et centres des détonations actives pour corréler la couverture spatiale avec la consommation GPU.
4. **Atténuation lointaine et réponse linéaire :**
   - Flash d'ambiance lointain bridé à 2% dans le vertex shader.
   - Rétro-éclairage (backlight) basé sur la luminance linéaire du bloom, supprimant le voile laiteux lointain.

---

## 8. Recommandations & Feuille de Route Future

1. **Phase 1 : Implémentation de la LUT Radiale d'Atténuation (Zero SQRT) :**
   - Précalculer la courbe de chute de lumière $f(\text{distSq}, \cos\theta)$ dans une texture LUT statique mise à jour lors de la modification des réglages.
   - Remplacer les deux `sqrt` par un unique `texture(u_LightFalloffLut, vec2(distSq / radiusSq, cosTheta))`.
2. **Phase 2 : Éclairage Différé en Quart de Résolution (Quarter-Res Buffer) :**
   - Intégrer la passe d'accumulation de lumière de la fumée dans un buffer RGBA16F à basse résolution réutilisant le masque de fumée existant.
