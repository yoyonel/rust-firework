# 📈 Registre de Suivi des Baselines de Performance & Profiling (SSOT)

**Dernière mise à jour :** 7 Octobre 2026  
**Statut :** Document Vivant (Single Source of Truth des Baselines)  
**Maintenance :** Équipe Moteur & DevSecOps  
**Référence Architecturale :** [AGENTS.md (Pilier 2 & 3)](file:///home/latty/Prog/__PERSO__/rust-firework/AGENTS.md)

---

## 1. Objectif du Document

Ce registre centralise et pérennise l'historique complet des mesures de performances, profils matériels (CPU / iGPU / dGPU / Émulateur CI), ratios de profilage Tracy et temps de frame du simulateur de feux d'artifice. Il sert de **référentiel absolu de non-régression** pour tout arbitrage architectural ou optimisation future.

---

## 2. Profils Matériels Officiels Enregistrés

| Profil ID | Processeur Graphique (GPU) | Architecture / Pilote | Type d'Exécution | Source de Mesure |
| :--- | :--- | :--- | :--- | :--- |
| **`iris-xe-rpl-u`** | **Intel(R) Iris(R) Xe Graphics (RPL-U)** | Mesa 25.1.0 (LLVM 20.1.0), OpenGL 4.6 Core Profile | Hôte local X11 (`:0.0`) | Requêtes GPU matérielles (`gl::TIMESTAMP`) + Tracy CPU |
| **`llvmpipe-mesa`** | **llvmpipe (LLVM 18.1.8, 256 bits)** | Mesa Gallium software rasterizer, OpenGL 4.5 | CI Headless (`Xvfb`) | Tracy CPU zones (simulation & rasterisation logicielle) |
| **`nvidia-rtx`** *(Legacy)*| **NVIDIA GeForce RTX (Turing/Ampere)** | Pilote propriétaire NVIDIA, OpenGL 4.6 | Hôte Distrobox | Tracy GPU + CPU |

---

## 3. Baseline Courante en Régime Stabilisé : Profil `iris-xe-rpl-u`

* **Date de Capture :** 07/10/2026  
* **Branche Git :** [`feat/volumetric-smoke-lighting`](file:///home/latty/Prog/__PERSO__/rust-firework)  
* **Volume d'Échantillonnage :** 3 410 frames consécutives (régime chaud stabilisé, 100 000+ particules actives)  
* **Rapport Détaillé Associé :** [doc/20261007_intel_iris_xe_tracy_profiling_audit_report.md](file:///home/latty/Prog/__PERSO__/rust-firework/doc/20261007_intel_iris_xe_tracy_profiling_audit_report.md)

### 3.1. Enveloppe Temporelle Globale (Framerate & Bottleneck)

* **Temps GPU Total (`Renderer::render_frame`) :** **1 626,36 µs (1,63 ms)** $\rightarrow$ **Plafond GPU : ~615 FPS**.
* **Temps CPU Actif Total (Simulation + Émission GL) :** **~764 µs (0,76 ms)** $\rightarrow$ **Plafond CPU : ~1 300 FPS**.
* **Diagnostic Déterminant :** **Strictement GPU-Bound**. La CPU attend passivement le GPU pendant **669,30 µs par frame (46,7 % du temps CPU)** dans `glfwSwapBuffers`.

### 3.2. Répartition Décomposée du Budget GPU

| Étape de Rendu GPU | Durée Moyenne (µs) | Part Relative (%) | Objectif / Rôle Technique |
| :--- | :---: | :---: | :--- |
| **Pass: Bloom & Composite** | **830,35** | **51,05 %** | Chaîne Kawase/Gaussienne FBO + Tone Mapping |
| **Pass: HDR Scene (Total)** | **794,91** | **48,88 %** | Rendu géométrie et particules dans buffer HDR |
| ↳ *Draw All Particles* | **374,15** | 23,00 % | Particules instanciées fumée + buffers persistants AZDO |
| ↳↳ *Persistent Buffers (fusées/étincelles/traînées)* | 263,73 (3×87,9) | 16,21 % | Quads dynamiques streamés en mémoire unifiée |
| ↳↳ *Instanced Smoke (volumétrie & wrap shading)* | 110,42 | 6,79 % | Particules de fumée instanciées avec LUT de falloff |
| ↳ *Sky, Stars & Atmospheric Haze* | **312,08** | 19,19 % | Shader plein écran de dégradé céleste et brume |
| ↳ *Renderer::Smoke_Backlight_Mask* | **108,68** | 6,68 % | Passe FBO séparée pour masque de rétroéclairage |
| **Total Frame GPU (`render_frame`)** | **1 626,36** | **100,00 %** | Budget GPU complet par image |

### 3.3. Répartition Décomposée du Budget CPU

| Zone CPU | Durée Moyenne (µs) | Part Relative (%) | Analyse de Charge |
| :--- | :---: | :---: | :--- |
| **`simulator::finalize_frame(swap_buffer)`** | **669,30** | **46,7 %** | Attente de synchronisation VSync / GPU |
| **`Renderer::render_frame`** | **490,45** | **34,2 %** | Soumission OpenGL et transferts AZDO |
| ↳ *`Draw All Particles` (GL commands)* | 359,61 | 25,1 % | Commandes `glDrawArrays` / `glDrawArraysInstanced` |
| ↳ *`Pass: Bloom & Composite` (FBO switch)* | 84,30 | 5,9 % | Changements de framebuffers et draw quad |
| ↳ *`Renderer::fill_buffer` (AZDO write)* | 65,72 (3×21,9) | 4,6 % | Écriture CPU en mémoire mappée persistante |
| ↳ *`SmokeRenderer::render_smoke_instanced`* | 56,11 | 3,9 % | Préparation state machine OpenGL fumée |
| ↳ *`Renderer::Smoke_Backlight_Mask`* | 44,58 | 3,1 % | Soumission passe masque rétroéclairage |
| ↳ *`SmokeRenderer::fill_particle_data_direct`* | 11,47 | 0,8 % | Remplissage direct du buffer d'instances fumée |
| **`simulator::physics`** | **225,34** | **15,7 %** | Simulation N-corps, vent, gravité, intégration |
| ↳ *`SmokeSystem::update`* | 11,82 | 0,8 % | Advection et dissipation des bouffées |
| ↳ *`SmokeSystem::emit`* | 0,32 | 0,02 % | Spawn des particules |
| ↳ *`physics::update`* | 0,33 | 0,02 % | Intégration vectorielle SIMD |
| **`simulator::render_ui`** | **0,88** | **0,06 %** | Génération de l'interface ImGui |
| **`simulator::log_metrics`** | **1,00** | **0,07 %** | Comptage et télémétrie interne |

### 3.4. Thread Audio Dédié (CPal & Bus Spatial Lock-Free)

* **Fréquence du Callback :** ~70 Hz (tampon de 14,4 ms à 48 kHz).
* **Durée DSP par Tampon (`process_dsp_spatial_bus`) :** **14,43 ms** (respect strict de la deadline temps réel sans buffer underrun).
* **Calcul Effet Doppler (`process_doppler`) :** **647,88 µs**.
* **Consommation Queue Commandes CQRS (`consume_requests`) :** **81,75 µs**.
* **Contention Mutex / Locks :** **0 ns** (tampon circulaire lock-free `crossbeam`).

---

## 4. Historique Chronologique des Baselines Moteur

| Date | Jalon / Événement | Cible Matérielle | Tps Frame GPU | Tps Frame CPU | FPS Global | Notes Clés |
| :--- | :--- | :--- | :---: | :---: | :---: | :--- |
| **10/10/2026** | **Audit Point 2 : Frustum Culling CPU Particules** | Intel Iris Xe | **2.007 ms** | **0.78 ms** | **498.34 FPS** | ❌ **REJETÉ** (-10.34 % vs baseline 555.80 FPS). Les 4 comparaisons flottantes sur 100k particules provoquent des branch mispredictions CPU ; l'iGPU possède déjà un clipper matériel gratuit. Voir [`20261010_particle_frustum_culling_autopsy_report.md`](20261010_particle_frustum_culling_autopsy_report.md). |
| **10/10/2026** | **Audit Point 1 : Discard vs Branchless Alpha dans point_rendering** | Intel Iris Xe | **2.257 ms** | **0.72 ms** | **443.40 FPS** | ❌ **REJETÉ** (-3.20 % vs baseline 458.06 FPS). Le `discard` évite les lectures/écritures ROP inutiles sur les 21.46% de fragments hors disque ; sa suppression sature la bande passante ROP. Voir [`20261010_point_rendering_discard_vs_branchless_autopsy_report.md`](20261010_point_rendering_discard_vs_branchless_autopsy_report.md). |
| **10/10/2026** | **Audit Piste 3 : Sky Haze Arithmétique & DrawBuffers** | Intel Iris Xe | **2.065 ms** | **0.72 ms** | **484.20 FPS** | ❌ **REJETÉ** (-0.38 % vs baseline 486.06 FPS). Mesa NIR abaisse déjà `exp` en `fexp2` ; reconfiguration `glDrawBuffers` induit un pipeline stall. Voir [`20261010_sky_haze_arithmetic_and_drawbuffers_optimization_report.md`](20261010_sky_haze_arithmetic_and_drawbuffers_optimization_report.md). |
| **10/10/2026** | **Audit Piste 1 : Downsample Bloom 13-Tap Karis** | Intel Iris Xe | **2.330 ms** | **0.74 ms** | **429.26 FPS** | ❌ **REJETÉ** (-7.80 % vs baseline 5-tap 465.60 FPS). 13 lectures textures saturent la bande passante DDR5 UMA. Voir [`20261010_bloom_karis_downsample_benchmark_and_postmortem_report.md`](20261010_bloom_karis_downsample_benchmark_and_postmortem_report.md). |
| **08/10/2026** | **Fusion Backlight MRT Single-Pass** | Intel Iris Xe | **1.711 ms** | **0.72 ms** | **584.46 FPS** | +31.60 FPS (+5.72 % vs baseline 552.86 FPS), suppression passe Smoke_Backlight_Mask. |
| **07/10/2026** | **Audit Matériel iGPU & Real GPU Queries** | Intel Iris Xe | **1,63 ms** | **0,76 ms** | **~615** | GPU profiler activé (`feature = tracy`), bloom identifié à 51% GPU. |
| **06/10/2026** | Volumetric Shading & Wrap Lighting | Intel Iris Xe | ~1,65 ms | ~0,80 ms | ~600 | Intégration LUT 2D Zero SQRT et wrap lighting fumée. |
| **14/08/2026** | Refactoring Swap-and-Pop & Memory SoA | Intel Iris Xe | N/A | ~0,85 ms | ~580 | Élimination des réallocations vectorielles dans la physique. |
| **16/07/2026** | Rendu AZDO Persistent Mapped Buffers | NVIDIA / Intel | ~1,90 ms | ~1,20 ms | ~520 | Éradication de `glBufferSubData` synchrone via triple-buffering. |
| **08/07/2026** | Pipeline Initiale Legacy | Multi-GPU | > 3,50 ms | > 2,50 ms | < 250 | Rendu dynamique synchrone, allocations audio par sample. |

---

## 5. Ratios de Profilage Normalisés (Tracy KPI Invariants)

Pour rappel des ratios suivis dans [`benches/baselines/tracy_ratios_mesa_intel_r_iris_r_xe_graphics_rpl_u.csv`](file:///home/latty/Prog/__PERSO__/rust-firework/benches/baselines/tracy_ratios_mesa_intel_r_iris_r_xe_graphics_rpl_u.csv) :

* **Part Moteur Rendu (`perc_renderer`) :** `94,71 %`
* **Part Moteur Physique (`perc_physics`) :** `5,15 %`
* **Part Interface ImGui (`perc_ui`) :** `0,14 %`
* **Sous-Part HDR Scene (`perc_hdr`) :** `80,57 %` de la soumission render
* **Sous-Part Bloom & Composite (`perc_bloom`) :** `18,43 %` de la soumission render (CPU) / **51,05 % sur le GPU réel**
* **Sous-Part Particules (`perc_particles`) :** `78,51 %`
* **Ratio Physique / Rendu (`r_phys_rend`) :** `0,0544`
* **Ratio Bloom / HDR (`r_bloom_hdr`) :** `0,2288`

---

## 6. Procédure de Mise à Jour d'une Baseline (Protocole Zero-Trust)

Toute mise à jour de baseline dans ce document exige de respecter les règles suivantes :

1. **Isolation Matérielle :** Interdiction de mélanger les métriques d'une machine hôte avec les métriques Mesa LLVMpipe de la CI.
2. **Exécution Synchrone Bloquante :** L'agent IA ne doit lancer aucune tâche d'édition de code, d'indexation ou de recherche pendant la capture de profilage.
3. **Période de Warm-Up :** Ignorer les 300 premières frames pour laisser le JIT du pilote GL, les caches de shaders et la montée en fréquence CPU/GPU se stabiliser.
4. **Validation Statistique :** Au minimum 2 000 frames mesurées par capture.
5. **Approbation Humaine (Règle 1) :** La modification des baselines de référence doit être soumise et validée par le développeur humain.
