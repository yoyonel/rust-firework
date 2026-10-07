# 📊 Rapport d'Audit & Profiling Matériel Intel Iris Xe (Tracy & GPU Profiler)

**Date :** 7 Octobre 2026  
**Auteur :** Antigravity Engine Core Team  
**Cible Matérielle :** Intel(R) Iris(R) Xe Graphics (RPL-U / AlderLake-U/RaptorLake-U GT2)  
**Pilote & Stack :** Mesa 25.1.0-devel (LLVM 20.1.0), OpenGL 4.6 Core Profile, X11 (`DISPLAY=:0.0`)  
**Protocole de Mesure :** Profilage synchrone multi-moteur (CPU + GPU timer queries hardware + Audio thread CPal) via Tracy Profiler Protocol 76 (3 410 frames complètes analysées en régime stabilisé).

---

## 1. Executive Summary & Diagnostic Global

L'analyse holistique effectuée sur le processeur graphique intégré **Intel Iris Xe** démontre de manière formelle que le moteur est **GPU-Bound** :

* **Temps GPU Total (`Renderer::render_frame`) :** **1 626,36 µs (1,63 ms)** en moyenne par frame.
  * **Plafond matériel GPU théorique :** **~615 FPS**.
* **Temps CPU Actif Total (Simulation + Rendu) :** **~764 µs (0,76 ms)** par frame.
  * **Plafond CPU théorique :** **~1 300 FPS** (la CPU est deux fois plus rapide que l'iGPU).
* **Attente CPU dans `glfwSwapBuffers` :** **669,30 µs (46,7 % du temps de boucle CPU)**. La CPU passe près de la moitié de son temps bloquée à attendre que l'iGPU Intel termine de traiter les commandes de la frame précédente.
* **Synchronisation Mémoire RAM / VRAM :** **0 ns d'attente bloquante**. Le pipeline AZDO (Persistent Mapped Buffers en triple-buffering) fonctionne avec une efficacité absolue : aucun calage de synchronisation (`glClientWaitSync` non bloquant), écriture mémoire unifiée directe en 11 µs pour la fumée et 65 µs pour les fusées.

### Répartition Macroscopique du Temps GPU

```
Total GPU Frame : 1 626 µs (100 %)
├── [51,05 %] Pass: Bloom & Composite (830,35 µs)
│   └── Chaîne ping-pong downsample/upsample Kawase & tone mapping
└── [48,88 %] Pass: HDR Scene (794,91 µs)
    ├── [23,00 %] Draw All Particles (374,15 µs)
    │   ├── Persistent Buffers (sparks, rockets, trails) : 263,73 µs (3 x 87,91 µs)
    │   └── Instanced Smoke Billows (wrap shading & lighting) : 110,42 µs
    ├── [19,19 %] Sky Background, Atmosphere Haze & Stars (312,08 µs)
    └── [ 6,68 %] Renderer::Smoke_Backlight_Mask (108,68 µs)
```

---

## 2. Tableaux Exhaustifs des Métriques Profilées

### 2.1. Métriques GPU (Requêtes Matérielles `gl::TIMESTAMP`)

Échantillonnage sur **3 410 frames** consécutives :

| Zone GPU / Passe de Rendu | Invocations / frame | Moyenne (µs) | Min (µs) | Max (µs) | Part du Temps GPU |
| :--- | :---: | :---: | :---: | :---: |
| **`Pass: Bloom & Composite`** | 1 | **830,35** | 710,36 | 4 370,00 | **51,05 %** |
| **`Pass: HDR Scene`** | 1 | **794,91** | 357,19 | 3 545,78 | **48,88 %** |
| ↳ *`Draw All Particles`* | 1 | **374,15** | 57,92 | 2 291,98 | **23,00 %** |
| ↳↳ *`Particles_with_Persistent_Buffer` (sparks/rockets/trails)* | 3 | **263,73** (3×87,91) | 6,57 | 1 169,67 | **16,21 %** |
| ↳↳ *`SmokeRenderer::render_smoke_instanced`* | 1 | **110,42** | 32,10 | 1 122,31 | **6,79 %** |
| ↳ *`Renderer::Smoke_Backlight_Mask`* | 1 | **108,68** | 35,57 | 2 227,60 | **6,68 %** |
| ↳ *`Sky, Stars & Atmospheric Haze` (calcul déduit)* | 1 | **312,08** | 263,70 | 850,20 | **19,19 %** |
| **Total Global `Renderer::render_frame` (GPU)** | 1 | **1 626,36** | 1 070,57 | 6 234,48 | **100,00 %** |

### 2.2. Métriques CPU (Boucle Principale de Simulation)

| Zone CPU | Invocations / frame | Moyenne (µs) | Min (µs) | Max (µs) | Rôle & Analyse |
| :--- | :---: | :---: | :---: | :---: | :--- |
| **`simulator::finalize_frame(swap_buffer)`** | 1 | **669,30** | 98,78 | 5 470,74 | Attente active GPU / VSync (46,7 % du cycle CPU) |
| **`Renderer::render_frame`** | 1 | **490,45** | 80,25 | 17 742,81 | Émission des commandes GL & transferts mémoire |
| ↳ *`Draw All Particles`* | 1 | **359,61** | 19,87 | 10 551,95 | Itération et soumission des draw calls |
| ↳ *`Pass: Bloom & Composite`* | 1 | **84,30** | 15,53 | 17 518,08 | Bindings FBO et draw calls post-process |
| ↳ *`Renderer::fill_buffer`* | 3 | **65,72** (3×21,9) | 0,24 | 480,06 | Écriture CPU vers mémoire mappée AZDO |
| ↳ *`SmokeRenderer::render_smoke_instanced`* | 1 | **56,11** | 12,18 | 512,48 | Soumission de la passe instanciée de fumée |
| ↳ *`Renderer::Smoke_Backlight_Mask`* | 1 | **44,58** | 6,70 | 10 216,53 | Soumission de la passe masque backlight |
| ↳ *`SmokeRenderer::fill_particle_data_direct`* | 1 | **11,47** | 0,12 | 131,22 | Écriture mémoire mappée des instances fumée |
| **`simulator::physics`** | 1 | **225,34** | 3,33 | 4 436,26 | Physique gravitationnelle N-corps & collision |
| ↳ *`SmokeSystem::update`* | 2 | **11,82** | 0,07 | 251,97 | Simulation advection, dissipation & turbulence |
| ↳ *`SmokeSystem::emit`* | ~18 | **0,32** | 0,05 | 95,71 | Injection de nouvelles bouffées de fumée |
| ↳ *`physics::update`* | 1 | **0,33** | 0,06 | 39,57 | Intégration SoA particules de fusées |
| **`simulator::render_ui`** | 1 | **0,88** | 0,10 | 75,61 | Préparation buffers ImGui |
| **`simulator::log_metrics`** | 1 | **1,00** | 0,14 | 44,77 | Télémétrie & compteurs internes |

### 2.3. Métriques Thread Audio (DSP & Spatialisation Lock-Free)

Le moteur audio s'exécute sur un thread indépendant asynchrone (CPal). Les mesures confirment l'absence complète de contention :

| Zone Audio DSP | Fréquence | Durée Moyenne | Analyse |
| :--- | :---: | :---: | :--- |
| **`audio::process_dsp_spatial_bus`** | ~70 Hz (callback CPal) | **14,43 ms** | Traitement du bloc audio complet (binaural, convolution réverb) |
| **`audio::process_doppler`** | À chaque buffer | **647,88 µs** | Calcul vectoriel de pitch-shift Doppler |
| **`audio::consume_requests`** | À chaque buffer | **81,75 µs** | Dépilement de la queue lock-free CQRS |
| **`audio::soft_clipping`** | À chaque buffer | **3,88 µs** | Limiteur dynamique anti-saturation SIMD |
| **`audio::free_garbage_buffer`** | Fréquent | **0,12 µs** | Recyclage lock-free des voix expirées |

---

## 3. Analyse Approfondie Mémoire, VRAM & Synchronisation

### 3.1. Efficacité AZDO & Triple-Buffering

L'architecture OpenGL AZDO implémentée sur le projet produit d'excellents résultats sur l'architecture mémoire unifiée (UMA) de l'iGPU Intel :

* **Pointeurs Mappés Persistants (`GL_MAP_PERSISTENT_BIT | GL_MAP_COHERENT_BIT`) :**
  * La CPU écrit directement dans les régions mémoire mappées sans faire d'appel système `memcpy` intermédiaire ni de `glBufferSubData` lourd.
  * Durée d'écriture CPU pour 100 000+ particules de fumée : **11,47 µs**.
  * Durée d'écriture CPU pour les fusées et étincelles : **65,72 µs**.
* **Zero GPU Stall sur les Sync Fences :**
  * Le triple-buffering (`BUFFER_COUNT = 3`) accorde 2 frames entières d'avance au GPU.
  * Aucun blocage `glClientWaitSync` n'a été observé au cours des 3 410 frames (attente < 100 ns).

### 3.2. Uniform Buffer Objects (UBO)

* Le transfert du bloc de lumières (`update_lighting_ubo`) transmet 544 octets via `glBufferSubData`.
* Impact CPU insignifiant (< 1 µs), aucun calage de bus PCI-e puisqu'il s'agit d'une mémoire unifiée sur le die CPU/GPU Intel.

---

## 4. Analyse des Goulots d'Étranglement (Where Time is Spent)

### 🔴 Goulot #1 : Chaîne de Bloom & Post-Process (51,05 % du temps GPU — 830,35 µs)
* **Cause :** La passe de flou (Bloom) exécute une série d'itérations Kawase / gaussiennes descendantes puis ascendantes. Chaque itération requiert le changement de Framebuffer Object (`glBindFramebuffer`), la réaffectation de textures (`glBindTexture`) et le rendu d'un quad plein écran.
* **Problème architectural spécifique iGPU :** Sur l'Intel Iris Xe (architecture tile-based / unified cache), les changements successifs de FBO forcent des invalidations répétées du cache L3 et des écritures/lectures en mémoire système partagée, saturant la bande passante mémoire de la DDR5/LPDDR5.

### 🟡 Goulot #2 : Shaders Plein Écran Atmosphère & Ciel (19,19 % du temps GPU — 312,08 µs)
* **Cause :** Le calcul du ciel nocturne, des étoiles procédurales et de la brume atmosphérique s'exécute sur l'intégralité des pixels du viewport HDR.
* **Impact :** Les fonctions trigonométriques et le calcul procédural de bruit/haze sollicitent lourdement les Execution Units (EU) de l'Iris Xe.

### 🟡 Goulot #3 : Passe Séparée Masque Backlight de Fumée (6,68 % du temps GPU — 108,68 µs)
* **Cause :** Le moteur effectue actuellement deux passes distinctes pour le rendu de la fumée :
  1. `SmokeRenderer::render_smoke_mask` (108 µs GPU) : Rendu d'un masque alpha séparé dans un FBO dédié pour le calcul du rétroéclairage screen-space.
  2. `SmokeRenderer::render_smoke_instanced` (110 µs GPU) : Rendu volumétrique complet avec wrap shading et lighting.
* **Inefficience :** Les quads de particules de fumée sont rastérisés deux fois de suite par le GPU, doublant le coût de vertex assembly et de rasterization sur les particules transparentes.

---

## 5. Pistes d'Optimisation Hautement Pertinentes (Roadmap Actionnable)

Au vu des résultats formels de l'audit, voici les 4 axes d'optimisation prioritaires classés par ratio gain/effort :

### 🚀 Piste 1 (Quickwin Majeur GPU) : Pré-Downscale Demi-Résolution pour la Chaîne de Bloom
* **Principe :** Dès la fin de la passe HDR, downscaler la texture d'émission lumineuse en **demi-résolution ($W/2 \times H/2$)** ou **quart de résolution** AVANT d'entamer les passes de flou Kawase.
* **Gain estimé :** Réduction de **40 à 50 %** du coût de la passe Bloom (**~350 à 400 µs économisées par frame sur l'iGPU**).
* **Impact FPS projeté :** Le temps GPU frame passerait de 1 626 µs à ~1 250 µs $\rightarrow$ **Hausse du plafond GPU de 615 FPS à ~800 FPS (+30 %)**.
* **Impact visuel :** Imperceptible pour du flou d'éblouissement (Bloom).

### 🚀 Piste 2 (Fusion Structurelle GPU) : Intégration du Backlight Mask en MRT Single-Pass
* **Principe :** Éliminer la passe dédiée `Smoke_Backlight_Mask` (108 µs). Utiliser le *Multiple Render Targets* (MRT) existant de la passe HDR pour écrire le masque d'opacité de rétroéclairage directement dans le canal Alpha du G-buffer / HDR target pendant `render_smoke_instanced`.
* **Gain estimé :** **108 µs économisées nettes sur l'iGPU** (+7 % FPS) et suppression de 44 µs de soumission CPU.
* **Effort :** Moyen (fusion d'un fragment shader et élimination d'un FBO intermédiaire).

### 💡 Piste 3 (Optimisation Arithmétique Shaders) : Sky Background & Haze
* **Principe :** Remplacer les calculs analytiques coûteux du shader de ciel (fonctions transcendantes répétées par pixel) par une LUT 1D de gradient atmosphérique ou une approximation polynomiale simplifiée.
* **Gain estimé :** **~100 à 150 µs** sur l'iGPU.

### 💡 Piste 4 (Économie d'Énergie & Frame Pacing CPU) : Limiteur Dynamique / VSync Throttling
* **Principe :** Constatant que la CPU passe **46,7 % de son temps en attente bloquante dans `glfwSwapBuffers`** (669 µs par frame), implémenter un frame pacer configurable pour soulager les coeurs CPU sur PC portable lors d'un affichage sur écran 60 Hz / 120 Hz / 144 Hz.

---

## 6. Runbook de Reproductibilité Humaine

Pour reproduire l'intégralité de cet audit matériel avec Tracy sur l'hôte :

```bash
# 1. Compiler le binaire avec instrumentation Tracy et GPU profiler activé
cargo build --release --features tracy

# 2. Démarrer la capture Tracy CLI synchrone (port standard 8086)
/home/latty/Prog/__PERSO__/suckless-ogl/deps/tracy/capture/build/tracy-capture -o /tmp/audit_run.tracy -s 5 &
CAPTURE_PID=$!
sleep 1

# 3. Lancer la simulation sur l'iGPU Intel (X11 natif)
DISPLAY=:0.0 ./target/release/rust-firework

# 4. Attendre la fin de la capture et exporter les zones CPU et GPU
wait $CAPTURE_PID
/home/latty/Prog/__PERSO__/suckless-ogl/deps/tracy/csvexport/build/tracy-csvexport -u /tmp/audit_run.tracy > /tmp/audit_cpu_zones.csv
/home/latty/Prog/__PERSO__/suckless-ogl/deps/tracy/csvexport/build/tracy-csvexport -g /tmp/audit_run.tracy > /tmp/audit_gpu_zones.csv
```
