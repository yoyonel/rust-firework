# Architecture & Pipeline du Moteur Audio 3D (`src/audio_engine/`)

Ce document décrit l'architecture complète du moteur audio 3D temps réel du projet `rust-firework`.

---

## 🚀 Vue d'Ensemble

Le moteur audio est un système temps réel à **haute performance et zéro-allocation** sur le thread de traitement sonore, conçu pour restituer un environnement sonore spatialisé 3D immersif pour la simulation de feux d'artifice.

### Fonctionnalités Clés
- **Rendu Spatial B-Format 2D & Binaural HRTF** : Traitement par convolutions binaurales ITD (Interaural Time Difference) et ILD (Interaural Level Difference).
- **Modèle d'Atténuation Inverse-Distance Roll-off** : Calcul logarithmique de l'atténuation spatiale avec zone nominale (`ref_distance = 50px`) et fondu de sécurité (`max_distance = 2000px`).
- **Vol de Voix Prioritaire (Prioritized Voice Stealing)** : Gestion dynamique jusqu'à 128 voix simultanées avec comparaison du volume pré-atténué spatialement et priorisation selon le type de son (ex: `Explosion = 2.0`, `Rocket = 1.0`).
- **Positionnement Atomique de l'Auditeur (`AtomicVec2`)** : Synchronisation lock-free et thread-safe de la position du Listener `(width / 2, 0)` entre le thread graphique et le thread audio CPAL.
- **Télémétrie & Diagnostic Audio Temps Réel** : Canal lock-free borné (2048) transmettant les événements audio et métriques de latence (*Transit* et *Render-to-Start*) vers un Moniteur ImGui dédié et des superpositions graphiques.
- **Scène de Stress-Test Audio & Rendu GPU d'Orbites** : Test interactif (128 à 1024 sources virtuelles) avec découplage via `CircleGPURenderer` (quads instanciés) et throttling des événements Doppler à 144 Hz pour supprimer le *zipper noise*.

---

## 📐 Architecture du Thread Audio & Modèle Multi-Thread

```text
       Thread Principal (Simulation & UI)                    Thread Audio (Callback CPAL)
+--------------------------------------------+          +------------------------------------+
|                                            |          |                                    |
| - Simulation Physique / Apogée Fusées      |          | - Callback CPAL (Période ~1.33 ms) |
| - Mises à jour Listener (AtomicVec2)       |--------->| - Lecture atomique Listener        |
| - Mises à jour Doppler (Throttled 144 Hz)  |--Queue-->| - DSP Processing (Zéro allocation) |
| - ImGui Diagnostic Monitor & Overlay       |<--Debug--|   * Vol de Voix Prioritaire        |
|                                            |  (2048)  |   * Inverse-Distance Roll-off      |
+--------------------------------------------+          |   * ITD/ILD & Spatial Reverb       |
                                                        |   * Mixage vers Buffer CPAL        |
                                                        +------------------------------------+
```

---

## 🔊 Modèle d'Atténuation de Distance (Inverse-Distance Roll-off)

L'atténuation sonore s'appuie sur le modèle physique standard de l'industrie (OpenAL / FMOD) :

1. **Zone de Volume Maximal (`d <= ref_distance`)** : Pour une distance inférieure ou égale à **50px**, le gain reste maximal (`1.0`).
2. **Zone Décroissance Logarithmique (`ref_distance < d < max_distance`)** :
   $$\text{Gain}_{\text{raw}} = \frac{\text{ref\_distance}}{d}$$
3. **Fondu de Sécurité Lineaire** :
   $$\text{Fade} = \frac{\text{max\_distance} - d}{\text{max\_distance} - \text{ref\_distance}}$$
   $$\text{Gain}_{\text{final}} = \text{Gain}_{\text{raw}} \times \text{Fade}$$
4. **Coupure Propre (`d >= max_distance`)** : volume ramené doucement à `0.0` à **2000px** pour éviter les sauts audibles et libérer la voix.

---

## 🦹 Algorithme de Vol de Voix Prioritaire (Voice Stealing)

Lorsque la capacité maximale des 128 voix est atteinte :
1. Calcul du volume **pré-atténué spatialement** de la nouvelle requête.
2. Multiplication du gain pré-atténué par le **poids de priorité du type de son** :
   - `Explosion` : Priorité `2.0` (événement critique pour l'utilisateur)
   - `Rocket` : Priorité `1.0` (sifflement / whoosh de vol)
3. Recherche dans les 128 voix actives de la voix ayant la valeur $(\text{Gain Atténué} \times \text{Priorité})$ la plus faible.
4. Si la nouvelle requête dépasse cette voix minimale, la voix faible est **interrompue immédiatement (volée)** et réattribuée au nouveau son, émettant un événement de diagnostic `stolen_req_id`.

---

## 📊 Système de Diagnostic Audio & Interfaces Graphiques

- **Métriques de Latence** :
  - **Transit Latency** : Durée de traversée du canal de commande entre le thread simulation et le thread audio.
  - **Render-to-Start Latency** : Temps réel écoulé entre la détection physique de l'événement et l'écriture du premier échantillon audio dans le buffer CPAL.
- **Audio Diagnostic Monitor (ImGui)** : Fenêtre dédiée affichant les compteurs par type de son, les latences moyennes en temps réel, l'état des drops, et le journal déroulant des 15 derniers événements.
- **Overlay Graphique du Listener** :
  - Icône de casque vert `🎧 Listener` dessinée au sol à `(w/2, h)`.
  - Cercle bleu (rayon `50px`, zone de volume max).
  - Cercle orange (rayon `max_distance`, zone d'audibilité).

---

## 📄 Documentation de Référence Complémentaire

Pour une étude détaillée des implémentations et analyses de performances :
- 📄 [Documentation du Moteur Audio : Diagnostics, Latences, et Modèle d'Atténuation](audio_engine_diagnostic_and_attenuation.md)
- 📄 [Résolution des Dysfonctionnements Audio & Optimisations Graphiques](20260721_audio_diagnostics_underrun_imgui_fixes.md)
- 📄 [Spécification & Architecture Bus Spatial 2D](20260720_spatial_bus_audio_architecture.md)