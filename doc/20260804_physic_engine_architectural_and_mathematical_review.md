# Revue de Code Architecturale et Mathématique : Moteur Physique & Système de Particules

**Date :** 4 août 2026  
**Auteur :** Antigravity AI (Senior Systems & Physics Engine Architect)  
**Cible :** Dépôt [`rust-firework`](https://github.com/yoyonel/rust-firework)

---

## 1. Cinématique et Balistique

### Équations du mouvement et forces appliquées

* **Fusées ([`Rocket`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L24)) :**
  - **Force unique :** Gravité constante $\vec{g} = (0, \text{config.gravity})$ (valeur par défaut : `-200.0` $\text{px/s}^2$, déclarée dans [`constants.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/constants.rs#L36)).
  - **Absence de traînée aérodynamique ($C_d = 0$) :** Trajectoire purement parabolique.
  - **Mise à jour dans [`Rocket::update_movement`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L206-L209) :**
    ```rust
    self.vel += gravity * dt;
    self.pos += self.vel * dt;
    ```
  - **Propulsion initiale :** Définie au réarmement/spawn ([`Rocket::random_vel`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L459-L468)). Vecteur vitesse initiale généré selon l'angle vertical $\frac{\pi}{2} \pm \Delta \theta$ et vitesse scalaire dans `[min_speed, max_speed]`.

* **Particules d'explosion ([`Particle`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/particle.rs#L6)) :**
  - Trajectoire régie par gravité et vitesse initiale transmise lors de l'éclatement ([`Rocket::update_explosions`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L314-L338)).

* **Particules de fumée ([`SmokeParticle`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/smoke_system.rs#L79)) :**
  - Gravité **non appliquée** aux particules de fumée ([`SmokeSystem::update`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/smoke_system.rs#L267-L303)). Déplacement par simple advection uniforme $\vec{x}(t + dt) = \vec{x}(t) + \vec{v}_{\text{smoke}} \cdot dt$.

---

### Éclatement (Spawning) et Dispersion Spatiale

L'explosion se déclenche quand $\vec{v}_{y} \le \text{config.explosion\_threshold}$ (apogée/descente).

1. **Explosion Sphérique Uniforme ([`Rocket::trigger_spherical_explosion`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L367-L387)) :**
   $$\theta \sim \mathcal{U}(0, 2\pi), \quad \|\vec{v}_{\text{exp}}\| \sim \mathcal{U}(v_{\min}, v_{\max})$$
   $$\vec{v}_0 = \begin{pmatrix} \cos\theta \\ \sin\theta \end{pmatrix} \cdot \|\vec{v}_{\text{exp}}\|$$

2. **Explosion Basée sur une Image Noir & Blanc ([`ImageShape`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/explosion_shape.rs#L60), [`Rocket::trigger_image_explosion`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L393-L449)) :**
   - **Échantillonnage :** Pixels d'intensité $\ge \text{threshold}$ normalisés sur $[-0.5, 0.5]$ autour du barycentre ([`ImageShape::from_image`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/explosion_shape.rs#L86-L147)).
   - **Orientation dynamique :** Forme alignée sur l'angle de cap de la fusée $\theta_{\text{rocket}} = \text{atan2}(v_y, v_x) - \frac{\pi}{2}$ ([`get_target_position_rotated`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/explosion_shape.rs#L211-L225)).
   - **Calcul balistique exact ([`ImageShape::compute_initial_velocity`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/explosion_shape.rs#L243-L254)) :**
     $$\vec{P}_{\text{target}} = \vec{P}_0 + \vec{V}_{\text{exp}} \cdot t_{\text{flight}} + \frac{1}{2}\vec{g} \cdot t_{\text{flight}}^2 \implies \vec{V}_{\text{exp}} = \frac{\vec{P}_{\text{target}} - \vec{P}_0 - \frac{1}{2}\vec{g} \cdot t_{\text{flight}}^2}{t_{\text{flight}}}$$
   - **Conservation de la quantité de mouvement :** 
     $$\vec{V}_{\text{finale}} = \vec{V}_{\text{fusée}} + 4.0 \cdot \vec{V}_{\text{exp}}$$

3. **Traînée de Fusée (Trail) ([`Rocket::spawn_trail_particles`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L252-L286)) :**
   - Échantillonnage spatial continu à intervalle $d_{\text{step}} = 2.0\text{px}$ le long du segment d'avancement $[\vec{P}_{\text{last}}, \vec{P}_{\text{current}}]$.

4. **Émission de Fumée ([`SmokeSystem::emit`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/smoke_system.rs#L223-L241)) :**
   - Émission continue à la base de tuyère $\vec{P}_{\text{base}} = \vec{P}_{\text{fusée}} - \hat{v} \cdot d_{\text{offset}}$ ([`Rocket::base_pos`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L125-L132)).
   - Vitesse transmise avec facteur d'héritage $0.05$ + dispersion aléatoire multidirectionnelle.

---

## 2. Intégration du Temps (Time Stepping)

### Méthode d'intégration temporelle

La méthode employée pour l'intégration de la position et de la vitesse est l'**Intégration d'Euler Semi-Implicite (Symplectique)** pour les fusées et les particules de traînée/explosion :

```rust
// Modèle utilisé dans rocket.rs L207-L208 et L332-L333
self.vel += gravity * dt; // 1. Mise à jour de la vitesse (V_n+1)
self.pos += self.vel * dt; // 2. Calcul de position via V_n+1 (et non V_n)
```

$$\begin{aligned}
\vec{v}_{n+1} &= \vec{v}_n + \vec{g} \cdot dt \\
\vec{x}_{n+1} &= \vec{x}_n + \vec{v}_{n+1} \cdot dt
\end{aligned}$$

*Note : Pour la fumée ([`SmokeSystem::update`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/smoke_system.rs#L301)), simple Euler explicite à vitesse sans accélération.*

---

### Gestion du pas de temps (Timestep)

Le pas de temps est **variable, non plafonné et dépendant directement du framerate de rendu**. Il n'y a **pas d'accumulateur temporel découplé** (pas de boucle `while accumulator >= DT`).

---

### Cheminement complet du Delta-Time ($dt$)

```mermaid
sequenceDiagram
    participant Main as main.rs / Simulator::run()
    participant Loop as Simulator::step()
    participant Timing as events::update_frame_timing()
    participant Sim as Simulator::update_simulation()
    participant Engine as PhysicEngineFireworks::update()
    participant Rocket as Rocket::update()

    Main->>Loop: while self.step() {}
    Loop->>Timing: update_frame_timing()
    Timing-->>Loop: delta = now - last_time (f32 seconds)
    Loop->>Sim: update_simulation(delta)
    Sim->>Engine: physic_engine.update(delta)
    Engine->>Rocket: rocket.update(dt, pools, config, shape)
    Rocket->>Rocket: update_movement(dt) / update_trails(dt) / update_explosions(dt)
    Engine->>Engine: smoke_system.update(dt)
```

1. **Boucle principale ([`Simulator::run`](file:///home/latty/Prog/__PERSO__/rust-firework/src/simulator.rs#L285)) :** Invoque [`Simulator::step`](file:///home/latty/Prog/__PERSO__/rust-firework/src/simulator.rs#L296) à chaque frame GLFW.
2. **Mesure du temps ([`update_frame_timing`](file:///home/latty/Prog/__PERSO__/rust-firework/src/simulator/events.rs#L398-L401)) :**
   ```rust
   let now = Instant::now();
   let delta = now.duration_since(self.last_time).as_secs_f32();
   self.last_time = now;
   ```
3. **Transmission ([`Simulator::update_simulation`](file:///home/latty/Prog/__PERSO__/rust-firework/src/simulator.rs#L381)) :** Passe `delta` brut au trait [`PhysicEngine::update`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/trait.rs#L81).
4. **Injection physique ([`PhysicEngineFireworks::update`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/physic_engine_generational_arena.rs#L294)) :**
   - Mise à jour accumulateurs de spawn fusées/fumée (`time_since_last_rocket += dt`).
   - Injection dans [`Rocket::update`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L141-L171) et [`SmokeSystem::update`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/smoke_system.rs#L267).

---

## 3. Stabilité et Robustesse Numérique

### Limites mathématiques et risques d'instabilité

1. **Absence de sub-stepping / dt max clamp :**
   - **Problème :** Si le framerate baisse drastiquement ($dt > 100\text{ms}$, ex. freeze fenêtre ou baisse GPU), la vitesse de la fusée produit des bonds spatiaux massifs ($\Delta \vec{x} = \vec{v} \cdot dt$).
   - **Conséquence 1 (Tunneling / Audio Bypass) :** La fusée saute au-dessus de son apogée en 1 frame sans valider l'intervalle d'anticipation audio (`future_vel_y <= threshold`), provoquant un raté ou un décalage de déclenchement du son d'explosion ([`PhysicEngineFireworks::update`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/physic_engine_generational_arena.rs#L281)).
   - **Conséquence 2 (Débordement du Ring Buffer Traînée) :** Dans [`Rocket::spawn_trail_particles`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L257-L285) :
     ```rust
     let count = (dist / TRAIL_SPACING) as u32;
     ```
     Si $dt$ est grand, `count` dépasse la capacité du bloc `particles_per_trail` (ex: 64). La boucle écrase plusieurs fois les mêmes emplacements dans le tampon circulaire au cours de la même frame.

2. **Divergence de la forme d'explosion basée sur l'image :**
   - Dans [`Rocket::trigger_image_explosion`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L425) :
     ```rust
     let expansion_velocity = image_shape.compute_initial_velocity(...) * 4.0;
     ```
     L'application du facteur magique `4.0` invalide l'équation balistique exacte issue de [`ImageShape::compute_initial_velocity`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/explosion_shape.rs#L243-L254). Les particules dépassent la forme cible projetée de 400% au temps `flight_time`.

---

### Sécurisation du Cycle de Vie des Particules

* **Generational Arena ([`Arena<Rocket>`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/physic_engine_generational_arena.rs#L23)) :** Évite les accès invalides et le problème ABA grâce aux handles indexés par génération ([`generational_arena::Index`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/physic_engine_generational_arena.rs#L1)).
* **Pools de blocs contigus ([`ParticlesPool`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/particles_pools.rs#L67)) :**
  - Mémoire allouée une seule fois à l'initialisation (`max_blocks * per_block`).
  - Allocation/Libération $O(1)$ par indices de tranche `Range<usize>` ([`ParticlesPool::allocate_block`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/particles_pools.rs#L112-L122) et [`free_block`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/particles_pools.rs#L127-L132)).
  - Zero allocation dynamique durant la boucle de mise à jour physique chaude (réutilisation de [`to_deactivate_scratch`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/physic_engine_generational_arena.rs#L28)).

---

## 4. Performance et Profil Mémoire

### Structure Mémoire (Memory Layout) : AoS vs SoA

Structure de la particule ([`Particle`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/particle.rs#L4-L19)) :

```rust
#[repr(C, align(16))]
pub struct Particle {
    pub pos: Vec2,          // 8 octets
    pub color: Color,       // 12 octets
    pub life: f32,          // 4 octets
    pub max_life: f32,      // 4 octets
    pub size: f32,          // 4 octets
    pub angle: f32,         // 4 octets (36 octets partagés GPU)
    pub vel: Vec2,          // 8 octets (CPU only)
    pub active: bool,       // 1 octet  (CPU only)
    pub particle_type: u8,  // 1 octet  (CPU only) + 18 octets padding = 64 octets
}
```

* **Modèle actuel : Array of Structures (AoS)**.
* **Taille d'une structure :** Exactement **64 octets** (alignée sur 16 octets, équivalent à une ligne de cache CPU L1/L2 standard).
* **Analyse de localité de cache CPU :**
  - **Avantage :** Transfert direct vers le buffer GPU via `bytemuck::Pod` sans réagencement des structures.
  - **Inconvénient (Hot-Loop Physics) :** Durant la mise à jour physique ([`Rocket::update_explosions`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L326-L337)), le CPU ne lit/écrit que `pos` (8b), `vel` (8b), `life` (4b) et `active` (1b), soit **21 octets utiles sur 64 octets** perçus par ligne de cache (taux d'efficacité de ligne de cache ~32%).

---

### Goulots d'Étranglement Identifiés

1. **Verrouillage Mutex sur le Pool de Particules ([`ParticlesPool`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/particles_pools.rs#L75)) :**
   ```rust
   free_blocks: Arc<Mutex<VecDeque<usize>>>
   ```
   Chaque allocation/libération de bloc verrouille un `Mutex` et effectue un contournement atomique inutile en contexte mono-thread physique.

2. **Dispatch Virtuel dans l'Itération des Particules ([`PhysicEngineIterator`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/trait.rs#L9-L48)) :**
   Les méthodes d'itération du trait utilisent des fermetures dynamiquement dispatchées (`&mut dyn FnMut(&Particle)`). Cela empêche l'inlining du compilateur et introduit un coût de call via vtable pour chaque particule transmise à la préparation du rendu.

3. **Chaînage d'Itérateurs complexes sur les blocs ([`Rocket::iter_active_particles`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/rocket.rs#L96-L118)) :**
   L'association `flat_map(...)` + `filter(...)` + `chain(...)` introduit un coût fixe d'états d'itérateurs lors du parcours CPU.

---

### Pistes d'Optimisations Concrètes (Low-Overhead)

#### 1. Accumulateur à Pas de Temps Fixe (Fixed Timestep & Sub-stepping)
Découpler l'intégration physique du taux de rafraîchissement avec plafonnement du delta maximum pour garantir la stabilité balistique et audio :

```rust
const FIXED_DT: f32 = 1.0 / 120.0; // 120 Hz physique
let mut frame_time = delta.min(0.25); // Clamp anti-freeze
self.accumulator += frame_time;

while self.accumulator >= FIXED_DT {
    self.step_physics(FIXED_DT);
    self.accumulator -= FIXED_DT;
}
```

#### 2. Représentation SoA (Structure of Arrays) pour le Moteur Physique CPU
Séparer le stockage mémoire physique du stockage de rendu :

```rust
pub struct ParticleSoA {
    pub pos_x: Vec<f32>,
    pub pos_y: Vec<f32>,
    pub vel_x: Vec<f32>,
    pub vel_y: Vec<f32>,
    pub life: Vec<f32>,
    pub active: BitVec,
}
```
* **Bénéfice :** Permet la vectorisation automatique SIMD (AVX2/NEON) du CPU lors des boucles d'intégration de vitesse et de position, avec un remplissage à 100% des lignes de cache L1.

#### 3. Supprimer `Arc<Mutex<...>>` dans [`ParticlesPool`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/particles_pools.rs#L75)
Remplacer `Arc<Mutex<VecDeque<usize>>>` par un simple stack contigu `Vec<usize>` directement détenu par la structure de pool, supprimant toute surcharge d'horloge atomique / mutex.

#### 4. Streaming Direct sans closures `dyn FnMut`
Préférer l'exposition directe de tranches contiguës de mémoire (`&[Particle]`) ou la copie par blocs bruts (`bytemuck::cast_slice`) vers les Persistent Mapped Buffers (VBO/SSBO) OpenGL afin d'éliminer les appels de fonction virtuels par particule.

---

## 5. Architecture Multi-thread & Synchronisation Audio-Physique (CPal)

### Modèle de Précalcul Balistique (Look-ahead)

#### Méthode de précalcul : Résolution analytique directe

Le moteur utilise une **résolution analytique explicite sur une fenêtre temporelle glissante ($\Delta t_{\text{anticipation}}$)**. Aucune simulation accélérée (fast-forward itératif) n'est exécutée.

* **Anticipation du Lancement (Launch Look-ahead) :**
  Dans [`PhysicEngineFireworks::update`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/physic_engine_generational_arena.rs#L228-L245) :
  ```rust
  let launch_anticipation_dt = self.config.audio_launch_anticipation_ms / 1000.0;
  if self.time_since_last_rocket + launch_anticipation_dt >= self.next_rocket_interval
      && !self.audio_launch_triggered
  {
      // Pré-réarmement de la fusée (active = false)
      r.reset(&self.config, self.window_width);
      self.anticipated_launch = Some((r.id, r.pos));
      self.audio_launch_triggered = true;
  }
  ```
  La fusée reste masquée et immobile physiquement pendant les frames d'anticipation, mais son événement audio est émis vers CPal.

* **Anticipation de l'Explosion (Explosion Look-ahead) :**
  Dans [`PhysicEngineFireworks::update`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/physic_engine_generational_arena.rs#L280-L291) :
  ```rust
  let explosion_anticipation_dt = self.config.audio_explosion_anticipation_ms / 1000.0;
  let future_vel_y = rocket.vel.y + gravity.y * explosion_anticipation_dt;
  if future_vel_y <= self.config.explosion_threshold {
      let future_pos = rocket.pos
          + rocket.vel * explosion_anticipation_dt
          + 0.5 * gravity * explosion_anticipation_dt * explosion_anticipation_dt;
      self.anticipated_explosions[anticipated_count] = (rocket.id, future_pos);
      rocket.audio_explosion_triggered = true;
  }
  ```
  Le précalcul projette la vitesse future $v_{y, \text{futur}} = v_y + g_y \cdot \Delta t_{\text{ant}}$ et la position future via l'équation quadratique de la parabole.

---

#### Décalage Temporel : Précalcul Analytique vs Intégration Numérique

* **Divergence continue / discrète :**
  - Le précalcul suppose un mouvement continu $\vec{x}(t) = \vec{x}_0 + \vec{v}_0 t + \frac{1}{2}\vec{g}t^2$.
  - La simulation réelle intègre la position frame par frame par **Euler Semi-Implicite à pas de temps variable** ($\Delta t_{\text{frame}}$ non constant).
* **Conséquence :** Si le framerate fluctue, l'instant réel où $v_{y, \text{réel}} \le \text{threshold}$ dévie de la prédiction théorique faite $\approx 40\text{ms}$ plus tôt.
* **Mécanisme d'Asservissement en Boucle Fermée (Closed-Loop Feedback) :**
  Pour corriger cette dérive, le [`Simulator`](file:///home/latty/Prog/__PERSO__/rust-firework/src/simulator.rs#L485-L529) mesure l'erreur temporelle effective $\Delta t_{\text{erreur}} = t_{\text{audio}} - t_{\text{visuel}}$ via [`Simulator::track_physical_events`](file:///home/latty/Prog/__PERSO__/rust-firework/src/simulator.rs#L485) et ajuste dynamiquement la durée d'anticipation par rétroaction intégrale avec un gain $K_p = 0.05$ ([`adjust_explosion_anticipation_ms`](file:///home/latty/Prog/__PERSO__/rust-firework/src/simulator.rs#L468-L483)) :
  ```rust
  let gain = 0.05;
  let new_val = (old_val + error_ms * gain).clamp(0.0, 150.0);
  ```

---

## 6. Architecture Multi-thread et Passage de Messages

```mermaid
graph TD
    subgraph MainThread ["Main Thread (Render & Physics - 60-144 Hz)"]
        PhysicEngineFireworks --> |Anticipated Events| Simulator
        Simulator --> |enqueue_sound()| AudioEngine
    end

    subgraph LockFreeChannels ["Crossbeam Lock-Free Channels"]
        AudioEngine --> |play_tx: SPSC RingBuffer (256 slots)| PlayQueue
        AudioEngine --> |doppler_tx: 144Hz Doppler Events| DopplerQueue
        AudioEngine --> |AtomicU32 / AtomicVec2| MasterState
    end

    subgraph CpalAudioThread ["CPal Audio Thread (Real-Time SCHED_FIFO)"]
        PlayQueue --> |try_recv()| DspProcessor
        DopplerQueue --> |try_recv()| DspProcessor
        MasterState --> |Lock-free Atomic Read| DspProcessor
        DspProcessor --> |process_block()| SoundCard[Carte Son / Audio Buffer Hardware]
    end
```

### Modèle de communication inter-thread

* **Thread Principal (Rendu / Physique) :** Exécute la simulation GLFW/OpenGL et calcule le look-ahead.
* **Thread Audio CPAL (Temps-Réel Prioritaire) :**
  - Sous Linux, élevé en priorité temps réel via [`libc::pthread_setschedparam`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/fireworks_audio.rs#L358) avec la politique `SCHED_FIFO` (priorité 20) et `nice -20`.
* **Canaux de communication sans verrou (Lock-free Channels) :**
  1. **`play_tx` / `play_rx` ([`crossbeam_channel::bounded`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/fireworks_audio.rs#L115)) :** Ring buffer SPSC borné à 256 requêtes ([`PLAY_REQUEST_CHANNEL_CAPACITY`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/constants.rs)). Transmet la structure [`PlayRequest`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/types.rs#L55).
  2. **`doppler_sender` / `doppler_rx` :** Canal lock-free transmettant les positions/vitesses 144 Hz pour le calcul de l'effet Doppler dans le bus spatial ([`DopplerEvent`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/dsp_processor.rs#L16)).
  3. **`garbage_tx` / `garbage_rx` :** Canal de délestage pour détruire les `Arc<Vec<[f32; 2]>>` audio hors du thread temps réel ([`FireworksAudio3D::enqueue_sound`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/fireworks_audio.rs#L170-L174)).
  4. **Atomiques (`AtomicU32`, `AtomicVec2`, `AudioEffectFlags`) :** Variables atomiques partagées sans aucun Mutex pour le contrôle du volume master, wet de réverbération et position de l'auditeur.

---

### Évitement du Lock Contention et des Buffer Underruns

* **Zéro Allocation Mémoire dans le Thread Audio :**
  Dans [`DspProcessor::process_block`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/dsp_processor.rs), tous les buffers de mixage (`acc`, `bus_w`, `bus_x`, `export_buffer`) sont **pré-alloués** dans la structure. Aucune allocation (`Vec::new`, `Box`) n'est effectuée dans la boucle callback CPal.
* **Pointeurs partagés `Arc` :**
  [`PlayRequest`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/types.rs#L55) transporte un simple pointeur `Arc::clone(&sample_data)`. La donnée PCM audio n'est jamais copiée lors de l'émission d'un son.
* **Non-blocking Push (`try_send`) :**
  Si le canal `play_tx` est plein, l'événement est abandonné silencieusement via `try_send` ([`fireworks_audio.rs#L213`](file:///home/latty/Prog/__PERSO__/rust-firework/src/audio_engine/fireworks_audio.rs#L213)). La boucle physique principale ne bloque jamais sur l'audio.

---

## 7. Compensation de Latence DSP et Robustesse Temporelle

### Réconciliation de la Latence DSP

La latence globale du pipeline audio comprend :
$$\text{Latence Total Audio} = \text{Latence Buffer Hardware CPAL} + \text{Latence Block DSP} + \text{Latence Scheduling OS}$$

* **Taille de bloc DSP (`block_size`) :** 256 à 512 échantillons ($\approx 5.8\text{ms}$ à $11.6\text{ms}$ à 44.1kHz).
* **Délais d'anticipation par défaut ([`constants.rs`](file:///home/latty/Prog/__PERSO__/rust-firework/src/physic_engine/constants.rs#L134-L142)) :**
  - Launch : $35.0\text{ ms}$ (`DEFAULT_AUDIO_LAUNCH_ANTICIPATION_MS`)
  - Explosion : $40.0\text{ ms}$ (`DEFAULT_AUDIO_EXPLOSION_ANTICIPATION_MS`)
* Ces valeurs compensent la latence matérielle de CPal, le temps de mixage DSP et le délai d'affichage des trames OpenGL.

---

### Vulnérabilité aux Spikes de Framerate (Delta-Time Spikes)

* **Scénario d'échec :**
  1. À la frame $N$, le précalcul détecte une explosion dans $40\text{ms}$ et envoie l'événement au thread audio via `play_tx`.
  2. Le thread audio CPal (`SCHED_FIFO`) traite le message et démarre la lecture du sample $40\text{ms}$ plus tard.
  3. Le thread principal subit un spike de framerate GPU/OS ($\Delta t = 150\text{ms}$).
* **Conséquence :** Le son d'explosion retentit **avant** que la frame contenant le rendu visuel de l'explosion ne soit transmise à la carte graphique.

---

### Optimisations Architecturales Proposées pour un Déterminisme 100%

#### 1. Jouer les Sons sur Horloge Echantillon (Sample-Accurate Scheduled Playback)
Au lieu de déclencher immédiatement la voix dans CPal dès la réception de `PlayRequest`, inclure un **timestamp cible en nombre d'échantillons audio** :
$$N_{\text{target}} = N_{\text{current\_sample}} + (\text{anticipation\_ms} \times \text{sample\_rate})$$
Le thread audio accumule les requêtes dans une file ordonnée et n'active le mixage de la voix que lorsque `current_sample >= N_target`.

#### 2. Asservissement sur Horloge Audio Maître (Audio-Driven Master Clock)
Rendre le sous-système audio maître du temps de simulation :
* La boucle physique n'utilise plus `Instant::now()`, mais lit le compteur d'échantillons consommés par la carte son (`AudioEngine::current_audio_time()`).
* Éradique les désynchronisations liées au stuttering du processeur graphique ou du window manager.

#### 3. Plafonnement de la Prédiction par Pas Fixe (Fixed Sub-stepping)
Imposer un pas de temps physique maximal ($\Delta t_{\max} = 16.6\text{ms}$). Si le thread de rendu prend du retard, exécuter plusieurs sous-pas physiques sans augmenter le saut balistique d'une seule frame.

