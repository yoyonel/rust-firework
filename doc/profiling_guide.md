# Guide de Profiling - Fireworks Simulation

## Objectif
Identifier les goulots d'étranglement réels (CPU, RAM, GPU, I/O) avant toute optimisation.

---

## 1. Profiling CPU

### 1.1 Flamegraph avec `perf` (Linux)

**Installation** :
```bash
# Installer perf
sudo apt install linux-tools-common linux-tools-generic

# Installer cargo-flamegraph
cargo install flamegraph
```

**Utilisation** :
```bash
# Profiler avec flamegraph (génère flamegraph.svg)
cargo flamegraph --release

# Ou avec perf directement
perf record --call-graph dwarf ./target/release/fireworks_sim
perf report
```

**Interprétation** :
- Chercher les fonctions qui prennent >10% du temps CPU
- Identifier si c'est `update()`, `render()`, ou autre
- Vérifier la profondeur des call stacks (trop de petites fonctions = overhead)

---

### 1.2 Profiling avec `cargo-instruments` (macOS)

```bash
cargo install cargo-instruments
cargo instruments --release --template "Time Profiler"
```

---

### 1.3 Profiling avec `valgrind` (Callgrind)

```bash
# Installer valgrind
sudo apt install valgrind kcachegrind

# Profiler
valgrind --tool=callgrind --callgrind-out-file=callgrind.out ./target/release/fireworks_sim

# Visualiser
kcachegrind callgrind.out
```

**Métriques clés** :
- `Ir` (Instructions Read) : nombre d'instructions exécutées
- `Self` : temps passé dans la fonction elle-même
- `Incl` : temps total incluant les appels

---

### 1.4 Profiling intégré dans le code

Ajouter des timers manuels :

```rust
use std::time::Instant;

// Dans simulator.rs ou main loop
let start = Instant::now();
physic_engine.update(dt);
let physics_time = start.elapsed();

let start = Instant::now();
renderer.render_frame(&physic_engine);
let render_time = start.elapsed();

// Log toutes les 100 frames
if frame_count % 100 == 0 {
    info!("Physics: {:?}, Render: {:?}", physics_time, render_time);
}
```

---

## 2. Profiling RAM

### 2.1 Heaptrack (Linux)

**Installation** :
```bash
sudo apt install heaptrack heaptrack-gui
```

**Utilisation** :
```bash
# Profiler l'allocation mémoire
heaptrack ./target/release/fireworks_sim

# Analyser les résultats
heaptrack_gui heaptrack.fireworks_sim.*.gz
```

**Métriques clés** :
- Peak memory usage
- Allocations temporaires (churn)
- Fuites mémoire

---

### 2.2 Valgrind Massif

```bash
valgrind --tool=massif ./target/release/fireworks_sim
ms_print massif.out.*
```

---

### 2.3 Profiling intégré

Utiliser `jemalloc` avec profiling :

```toml
# Cargo.toml
[dependencies]
jemallocator = "0.5"

[profile.release]
debug = true  # Pour avoir les symboles
```

```rust
// main.rs
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
```

Puis :
```bash
MALLOC_CONF=prof:true ./target/release/fireworks_sim
jeprof --pdf target/release/fireworks_sim jeprof.*.heap > profile.pdf
```

---

## 3. Profiling GPU

### 3.1 `nvidia-smi` (NVIDIA)

```bash
# Monitoring en temps réel
watch -n 0.5 nvidia-smi

# Ou avec logging
nvidia-smi dmon -s pucvmet -i 0
```

**Métriques clés** :
- GPU Utilization (%) : doit être proche de 100% si GPU-bound
- Memory Usage : vérifier si on sature la VRAM
- Power Draw : indicateur de charge

---

### 3.2 `radeontop` (AMD)

```bash
sudo apt install radeontop
radeontop
```

---

### 3.3 `intel_gpu_top` (Intel)

```bash
sudo intel_gpu_top
```

---

### 3.4 Profiling OpenGL avec `apitrace`

```bash
# Installer apitrace
sudo apt install apitrace apitrace-gui

# Capturer une trace
apitrace trace ./target/release/fireworks_sim

# Analyser
qapitrace fireworks_sim.trace
```

**Métriques clés** :
- Nombre de draw calls par frame (idéalement <1000)
- Taille des buffers uploadés
- Temps GPU par draw call

---

### 3.5 Profiling avec RenderDoc (Recommandé pour OpenGL)

**Installation** :
```bash
sudo apt install renderdoc
```

#### Méthode 1 : Capture avec Overlay (F12)

```bash
# Lancer avec RenderDoc en overlay
renderdoccmd capture --wait-for-exit ./target/release/fireworks_sim
```

**Pendant l'exécution** :
- Appuyez sur **F12** pour capturer une frame
- L'overlay affiche "X Captures saved"
- Fermez l'application

**Ouvrir la capture** :
```bash
# Lancer le GUI
qapitrace

# File > Open Capture > Sélectionner le .rdc généré
```

---

#### Méthode 2 : Capture Automatique

```bash
# Capturer automatiquement après 5 secondes
renderdoccmd capture --wait-for-exit --capture-delay 5 ./target/release/fireworks_sim
```

---

#### Méthode 3 : Via le GUI RenderDoc

1. Lancer RenderDoc (GUI)
2. **File > Launch Application**
3. **Executable Path** : `./target/release/fireworks_sim`
4. **Working Directory** : `$PATH_TO_PROJECT/rust-fireworks_sim`
5. Cocher **"Capture Immediately"** (capture la première frame)
6. Cliquer **Launch**

---

#### Analyse pour Fireworks Sim

Une fois la capture ouverte dans RenderDoc :

##### 1. **Event Browser** (panneau gauche)

Voir tous les appels OpenGL de la frame :
```
glClear
glUseProgram (shader particles)
glBindVertexArray
glBindBuffer
glDrawArrays (particules trails)    <- Combien de draw calls ?
glDrawArrays (particules explosions)
glDrawArrays (particules rockets)
glSwapBuffers
```

**Métriques clés** :
- **Nombre de draw calls** : Idéalement <10 pour votre cas (instancing)
- Si >100 draw calls → Problème de batching

---

##### 2. **Pipeline State** (panneau central)

Cliquer sur un `glDrawArrays` pour voir :
- **Vertex Shader** : Code du shader utilisé
- **Fragment Shader** : Code du shader de pixels
- **Vertex Input** : Layout des attributs (position, couleur, life, etc.)
- **Textures** : Textures bindées (si vous utilisez des textures pour les rockets)

**Pour Fireworks Sim, vérifier** :
- Shader compilé correctement
- Attributs bien mappés (aPos, aColor, aLifeMaxLife)
- Pas de texture inutile bindée

---

##### 3. **Mesh Viewer** (onglet en bas)

Visualiser les particules envoyées au GPU :
- Cliquer sur un `glDrawArrays`
- Onglet **"Mesh Viewer"**
- Voir les positions X/Y/Z des particules
- Vérifier que les couleurs sont correctes

**Utile pour** :
- Détecter des particules hors écran (gaspillage GPU)
- Vérifier que les particules inactives ne sont pas rendues

---

##### 4. **Texture Viewer**

Si vous utilisez des textures (ex: pour les têtes de fusées) :
- Onglet **"Texture Viewer"**
- Voir les textures chargées en VRAM
- Vérifier la résolution (pas trop haute)

**Pour Fireworks Sim** :
- Texture de rocket head : devrait être ~256×256 max
- Si >1024×1024 → Gaspillage de VRAM

---

##### 5. **Performance Counters** (si supporté par le GPU)

Onglet **"Performance Counters"** :
- GPU Time par draw call
- Nombre de pixels dessinés (overdraw)
- Utilisation VRAM

**Métriques attendues pour Fireworks Sim** :
- Chaque `glDrawArrays` : <1ms
- Overdraw : <2× (peu de particules se chevauchent)

---

#### Diagnostics Spécifiques Fireworks Sim

##### Problème 1 : Trop de Draw Calls

**Symptôme** : >50 `glDrawArrays` par frame

**Cause** : Pas d'instancing, ou un draw call par fusée

**Solution** :
- Utiliser `glDrawArraysInstanced` pour toutes les particules
- Batching : regrouper trails + explosions en un seul buffer

---

##### Problème 2 : Particules Inactives Rendues

**Symptôme** : Dans Mesh Viewer, beaucoup de particules à (0,0) ou hors écran

**Cause** : Le buffer GPU contient des particules `active=false`

**Solution** :
- Filtrer côté CPU avant d'envoyer au GPU
- Ou utiliser un compute shader pour compacter le buffer

---

##### Problème 3 : Shader Lent

**Symptôme** : GPU time >5ms par draw call

**Cause** : Shader trop complexe (calculs inutiles)

**Solution dans votre shader** :
```glsl
// ❌ Éviter les divisions dans le fragment shader
float alpha = aLifeMaxLife.x / max(aLifeMaxLife.y, 0.0001);

// ✅ Pré-calculer côté CPU et passer en uniform
uniform float invMaxLife;
float alpha = aLifeMaxLife.x * invMaxLife;
```

---

##### Problème 4 : Overdraw Élevé

**Symptôme** : Beaucoup de particules se chevauchent

**Cause** : Explosions denses au centre

**Solution** :
- Trier les particules back-to-front (depth sorting)
- Ou utiliser `glBlendFunc(GL_ONE, GL_ONE)` (additive blending)

---

#### Checklist RenderDoc pour Fireworks Sim

- [ ] Nombre de draw calls <10 (idéalement 3-4)
- [ ] Chaque draw call <1ms GPU time
- [ ] Pas de particules inactives dans le buffer
- [ ] Shaders compilés sans erreurs
- [ ] Textures en résolution raisonnable (<512×512)
- [ ] Overdraw <2× (vérifier avec Performance Counters)

---

#### Commandes Rapides RenderDoc

```bash
# Capturer une frame après 3 secondes
renderdoccmd capture --wait-for-exit --capture-delay 3 ./target/release/fireworks_sim

# Ouvrir la dernière capture
qapitrace $(ls -t *.rdc | head -1)

# Capturer 5 frames consécutives
renderdoccmd capture --wait-for-exit --num-frames 5 ./target/release/fireworks_sim
```

---

## 4. Profiling I/O

### 4.1 `strace` (syscalls)

```bash
strace -c ./target/release/fireworks_sim
```

**Métriques** :
- Nombre de `read()`, `write()`, `open()`
- Temps passé dans les syscalls

---

### 4.2 `iotop` (I/O disk)

```bash
sudo iotop -o
```

---

## 5. Plan d'Action Recommandé

### Étape 1 : Baseline Metrics

```bash
# 1. Mesurer FPS actuel
echo "max_rockets = 2048" > /tmp/test.toml
./target/release/fireworks_sim  # Noter FPS

# 2. Flamegraph CPU
cargo flamegraph --release

# 3. GPU monitoring
nvidia-smi dmon &  # En arrière-plan
./target/release/fireworks_sim
```

### Étape 2 : Identifier le Goulot

**Si CPU > 80%** → Goulot CPU
- Regarder flamegraph pour identifier la fonction
- Vérifier si c'est physics ou autre

**Si GPU > 80%** → Goulot GPU
- Vérifier nombre de draw calls
- Analyser avec RenderDoc

**Si RAM augmente** → Fuite mémoire
- Utiliser Heaptrack

**Si aucun > 60%** → Goulot synchronisation
- Vérifier VSync
- Profiler avec `perf` les mutex/locks

### Étape 3 : Tester avec Charge Élevée

```bash
# Modifier physic.toml
max_rockets = 10000
particles_per_explosion = 512
particles_per_trail = 128

# Re-profiler
cargo flamegraph --release
```

### Étape 4 : Analyser les Résultats

Créer un tableau :

| Config | FPS | CPU % | GPU % | RAM (MB) | Goulot Identifié |
|--------|-----|-------|-------|----------|------------------|
| 2048 rockets | 3000 | ? | ? | ? | ? |
| 5000 rockets | ? | ? | ? | ? | ? |
| 10000 rockets | ? | ? | ? | ? | ? |

---

## 6. Outils Rapides pour Diagnostic

### Script de monitoring automatique

```bash
#!/bin/bash
# monitor.sh

echo "Timestamp,FPS,CPU%,GPU%,RAM_MB" > metrics.csv

while true; do
    timestamp=$(date +%s)
    
    # CPU (approximatif)
    cpu=$(top -bn1 | grep "fireworks_sim" | awk '{print $9}')
    
    # GPU (NVIDIA)
    gpu=$(nvidia-smi --query-gpu=utilization.gpu --format=csv,noheader,nounits)
    
    # RAM
    ram=$(ps aux | grep fireworks_sim | awk '{print $6}')
    
    # FPS (à extraire des logs)
    fps=$(tail -1 /tmp/fireworks.log | grep -oP 'FPS: \K[0-9]+')
    
    echo "$timestamp,$fps,$cpu,$gpu,$ram" >> metrics.csv
    sleep 1
done
```

---

## 7. Checklist Avant Optimisation

- [ ] Flamegraph généré et analysé
- [ ] GPU utilization mesurée
- [ ] RAM usage stable (pas de fuite)
- [ ] Goulot principal identifié (CPU physics / GPU rendering / autre)
- [ ] Baseline FPS documenté pour comparaison
- [ ] Configuration de test définie (nombre de rockets, particules)

---

## 8. Résultats Attendus

Après profiling, vous devriez savoir :

1. **Où est le goulot** : CPU physics, GPU rendering, ou synchronisation
2. **Quelle fonction coûte cher** : `update()`, `render()`, `iter_particles()`, etc.
3. **Scalabilité** : Comment les perfs évoluent avec le nombre de particules
4. **Marge d'amélioration** : Combien de temps "gaspillé" peut être optimisé

**Seulement après**, vous pouvez décider :
- Parallélisation CPU (si `update()` > 50% du temps)
- GPU compute shaders (si rendering > 50%)
- Optimisations mémoire (si allocations excessives)
- SIMD (si boucles simples identifiées)
