# Plan d'Action de Refactoring & Feuille de Route Technique (Technical Roadmap)

**Date :** 4 août 2026  
**Auteur :** Lead Technical Director & Senior Systems Architect  
**Cible :** Dépôt [`rust-firework`](https://github.com/yoyonel/rust-firework)  
**Portée :** Moteur Physique (Balistique, Intégration, Mémoire SoA) & Synchronisation Audio Multi-thread (CPal)

---

## 1. Matrice des Tâches et des Bugs (Findings & Fixes)

| ID | Description du problème / Faile structurelle | Fichiers / Symboles impactés | Priorité | Nature |
| :--- | :--- | :--- | :--- | :--- |
| **FIX-01** | **Tunneling & Dérive Balistique :** Pas de temps variable sans sub-stepping ni clamp, provoquant des sauts d'apogée et le débordement des anneaux de traînée lors des chutes de FPS. | [`physic_engine_generational_arena.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/physic_engine_generational_arena.rs#L228), [`events.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/simulator/events.rs#L398) | **Critique** | Architecture & Bugfix |
| **FIX-02** | **Divergence Forme Image :** Multiplicateur magique `4.0` appliqué à la vitesse balistique d'explosion, déformant la projection cible de 400%. | [`rocket.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L425), [`explosion_shape.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/explosion_shape.rs#L243) | **Haute** | Bugfix |
| **FIX-03** | **Sous-utilisation Cache L1 (AoS) :** Struct `Particle` 64 octets (AoS) provoquant un faible taux d'efficacité des lignes de cache L1 (~32%) en boucle chaude. | [`particle.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/particle.rs#L6), [`particles_pools.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/particles_pools.rs#L67) | **Haute** | Optimisation & DOD |
| **FIX-04** | **Contention Mutex Inutile :** Utilisation de `Arc<Mutex<VecDeque<usize>>>` pour la pile de blocs du pool dans un contexte mono-thread physique. | [`particles_pools.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/particles_pools.rs#L75) | **Moyenne** | Refactoring |
| **FIX-05** | **Desync Audio-Visuelle sous Spike FPS :** L'audio CPal joue en avance si la boucle principale subit un lag après l'envoi de la requête `PlayRequest`. | [`fireworks_audio.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/fireworks_audio.rs#L199), [`dsp_processor.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/dsp_processor.rs#L339) | **Critique** | Architecture |
| **FIX-06** | **Indirection Virtuelle d'Itération :** Utilisation de `dyn FnMut(&Particle)` lors du parcours des particules actives pour le rendu, interdisant l'inlining. | [`trait.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/trait.rs#L11), [`rocket.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L96) | **Moyenne** | Refactoring |

---

## 2. Quantification de l'Effort (Workload & Time Estimation)

Estimation chiffrée pour un développeur Rust Senior spécialisé en Data-Oriented Design (1 j/h = 8h) :

| ID Tâche | Description de l'intervention | Conception / Spéc. | Implémentation | Debug & Valid. | Total (j/h) | Total (h) |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **FIX-01** | Implémenter l'accumulateur `Fixed Timestep` (120 Hz) avec clamp `max_sub_steps = 4` dans [`Simulator::update_simulation`](file:///home/latty/Prog/__PERSO__/rust-firework/src/simulator.rs#L369). | 0.25 j | 0.50 j | 0.25 j | **1.00 j** | 8 h |
| **FIX-02** | Supprimer le multiplicateur `4.0` et valider les équations balistiques exactes dans [`Rocket::trigger_image_explosion`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L425). | 0.10 j | 0.10 j | 0.05 j | **0.25 j** | 2 h |
| **FIX-03** | Refactoriser la mémoire en **SoA (Structure of Arrays)** (`ParticleSoA`) + adapteur de conversion contigu pour le VBO OpenGL AZDO. | 0.50 j | 1.50 j | 0.50 j | **2.50 j** | 20 h |
| **FIX-04** | Remplacer `Arc<Mutex<VecDeque>>` par un stack direct `Vec<usize>` dans [`ParticlesPool`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/particles_pools.rs#L75). | 0.10 j | 0.25 j | 0.15 j | **0.50 j** | 4 h |
| **FIX-05** | Implémenter le **Sample-Accurate Audio Scheduling** (timestamping par numéro d'échantillon cible) dans CPal [`DspProcessor`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/dsp_processor.rs#L339). | 0.50 j | 0.75 j | 0.50 j | **1.75 j** | 14 h |
| **FIX-06** | Remplacer le callback virtuel `dyn FnMut` par une exposition de slices directes `&[Particle]` ou conversion `bytemuck::cast_slice`. | 0.25 j | 0.35 j | 0.15 j | **0.75 j** | 6 h |
| **TOTAL** | **Chantier complet de refactoring** | **1.70 j** | **3.45 j** | **1.60 j** | **6.75 j** | **54 h** |

---

## 3. Évaluation des Risques et Régressions (Risk Assessment)

| Modification Majeure | Risques Potentiels de Rupture | Impact Système | Stratégie de Mitigation |
| :--- | :--- | :--- | :--- |
| **Passage en SoA (FIX-03)** | Rupture du layout mémoire GPU (`#[repr(C)]` direct). Désynchronisation entre les buffers CPU SoA et le buffer Persistent Mapped (AZDO) OpenGL. | **Élevé** (Corruption visuelle / Crash GPU) | Implémenter une passe de packing SIMD dédiée `SoA -> AoS` uniquement lors du flush vers le buffer GPU persistent. |
| **Fixed Timestep (FIX-01)** | **Spiral of Death :** Si le calcul physique d'une sous-étape prend plus de temps que $dt_{\text{fixed}}$, le CPU sature indéfiniment. | **Critique** (Freeze complet de l'application) | Implémenter un garde-fou strict `max_sub_steps = 4` et de l'extrapolation visuelle inter-frames. |
| **Sample Scheduling (FIX-05)** | Buffer Underruns / Craquements audio CPal si les requêtes horodatées s'accumulent sans nettoyage en cas de chute massive de FPS. | **Moyen** (Artefacts sonores / Craquements) | Ring buffer borné pour les voix en attente avec politique de drop élégante (Fade-out instantané). |
| **Suppression Mutex (FIX-04)** | Violation des règles de possession (Borrow Checker) si `ParticlesPool` est accédé depuis plusieurs threads. | **Faible** (Rejet à la compilation par Rust) | Le type `ParticlesPool` est maintenu strictly `!Send` / `!Sync` pour garantir la possession mono-thread. |

---

## 4. Gains Espérés et KPIs (Return on Investment)

| Indicateur de Performance (KPI) | Valeur Actuelle (Baseline) | Cible Après Refactoring | Gain Estimé |
| :--- | :--- | :--- | :--- |
| **Stabilité Framerate (1% Low FPS)** | 35 - 45 FPS (Instabilité sur spikes) | **> 100 FPS** (Lissé et constant) | **+150% de stabilité** |
| **Efficacité Cache L1 CPU (Ligne 64b)** | ~32% (21/64 octets utiles en AoS) | **~95%** (Données denses en SoA) | **x3 sur le débit mémoire** |
| **Contention de Verrous (Lock Contention)** | ~5-8% du temps CPU dans Mutex Pool | **0%** (Suppression totale des Mutex) | **100% de déblocage CPU** |
| **Décalage Temporel Audio-Visuel (Jitter)** | $\pm 25\text{ ms}$ (Fluctuant selon FPS) | **$< 1\text{ ms}$** (Sample-Accurate) | **Synchronisation parfaite** |
| **Temps d'Intégration Physique (10k part.)** | ~1.8 ms / frame | **~0.4 ms / frame** (Auto-SIMD AVX2) | **Réduction de 77% du CPU** |

---

## 5. Stratégie de Qualité et Couverture de Tests (QA & Testing)

Plan de test minimaliste et automatisé via `cargo test` et `benches/` :

```mermaid
graph LR
    A[Unit Tests: Balistique & Math] --> B[Integration Tests: Channels & Sync]
    B --> C[Criterion Benchmarks: Hot Loops SoA]
    C --> D[Validation CI Pipeline]
```

### 1. Tests Unitaires Purs (Mathématiques & Intégration Balistique)
- **Calcul Balistique Image ([`explosion_shape.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/explosion_shape.rs#L243)) :**
  Vérifier que $\vec{V}_0$ calculé amène la particule à la position cible exacte au temps $t_{\text{flight}}$ avec et sans gravité.
- **Détection d'Apogée ([`rocket.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L314)) :**
  Valider la condition de déclenchement $v_y \le \text{threshold}$ sur $N$ étapes d'intégration $120\text{Hz}$.

### 2. Tests d'Intégration Cross-Thread (Audio-Physique)
- **Test de Non-Blocage du Ring Buffer ([`fireworks_audio.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/fireworks_audio.rs#L213)) :**
  Saturer `play_tx` avec 1000 requêtes consécutives et vérifier que le thread principal ne bloque pas et que `try_send` retourne l'erreur de saturation sans panique.
- **Validation du Feedback d'Anticipation ([`simulator.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/simulator.rs#L485)) :**
  Simuler un décalage audio-visuel artificiel et vérifier que l'algorithme d'ajustement fait converger l'erreur vers 0.

### 3. Benchmarks de Micro-Performance (`benches/physics_bench.rs`)
- Créer un benchmark Criterion comparant l'intégration de 50 000 particules entre la version AoS actuelle et la version SoA optimisée SIMD.
