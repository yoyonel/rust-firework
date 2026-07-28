# Documentation Technique - Moteur de Rendu Fireworks Sim

Ce document détaille les aspects techniques du moteur de rendu, les méthodes utilisées, les optimisations disponibles et les commandes de configuration.

## 🎨 Méthodes de Rendu

### 1. Rendu Instancié (Instanced Rendering)
Le moteur utilise le rendu instancié pour afficher efficacement des milliers de particules.
- **Technique** : `gl::DrawArraysInstanced`
- **Shaders** : `instanced_textured_quad.vert.glsl` / `.frag.glsl`
- **Données** : Les données des particules (position, couleur, taille, rotation, luminosité) sont envoyées dans un buffer unique (`ParticleGPU`) mappé en mémoire (`gl::MapBufferRange`).
- **Texture** : Utilisation d'atlas ou de textures individuelles pour les particules (ex: `sparkle.png`, `rocket_head.png`).

### 2. Bloom (Post-Processing)
L'effet de Bloom est appliqué pour créer l'éclat lumineux des feux d'artifice.
- **Pipeline** :
    1. **MRT (Multiple Render Targets)** : Le rendu de la scène génère simultanément l'image couleur (`GL_COLOR_ATTACHMENT0`) et l'image des pixels brillants (`GL_COLOR_ATTACHMENT1`) basée sur un seuil de luminosité.
    2. **Downsampling** : L'image brillante est réduite (downsampled) pour améliorer les performances et la qualité du flou.
    3. **Blur** : Application d'un flou (Gaussian ou Kawase) sur l'image brillante réduite.
    4. **Composition** : L'image floutée est additionnée à l'image originale.

### 3. Algorithmes de Flou (Blur)

Le moteur supporte deux algorithmes de flou, commutables à la volée :

#### A. Gaussian Blur (Défaut)
- **Description** : Flou gaussien séparable (passes horizontales puis verticales).
- **Passes** : 2 passes par itération (Ping-Pong). Pour 5 itérations = 10 passes.
- **Qualité** : Très douce, mathématiquement correcte.
- **Coût** : Élevé si beaucoup d'itérations.

#### B. Dual Filtering (alias Dual Kawase)
- **Description** : Algorithme multipasse basé sur des downsamples et upsamples successifs. Techniquement, il s'agit de **Dual Filtering** (inspiré par Kawase mais utilisant des kernels fixes 5-tap/9-tap), souvent appelé "Dual Kawase" dans l'industrie.
- **Passes** : Nombre fixe de passes (généralement 3 down + 3 up = 6 passes).
- **Qualité** : Très bonne pour les effets de glow, légèrement moins "parfaite" que le gaussien mais visuellement très proche.
- **Coût** : Constant et généralement plus faible (~40% plus rapide que 5 itérations de Gaussien).

## ⚙️ Paramètres et Configuration

Tous les paramètres sont ajustables via la console (`F1`) ou le fichier de config.

### Bloom
| Paramètre | Commande Console | Description |
|-----------|------------------|-------------|
| **Méthode** | `renderer.bloom.method <gaussian|kawase>` | Choix de l'algorithme de flou. |
| **Downsample** | `renderer.bloom.downsample <1|2|4>` | Facteur de réduction de résolution pour le bloom. 2 est recommandé. |
| **Intensité** | `renderer.bloom.intensity <float>` | Puissance de l'effet lumineux. |
| **Itérations** | `renderer.bloom.iterations <int>` | Nombre de passes de flou (Gaussian uniquement). |

### Particules
| Paramètre | Description |
|-----------|-------------|
| **Brightness** | Calculée dynamiquement : `(life / max_life)^3`. Décroissance exponentielle : les particules brillent fort à la naissance et s'éteignent rapidement. |

## 🚀 Optimisations

### 1. Downsampling Configurable
Le bloom peut être calculé à une résolution inférieure (1/2 ou 1/4 de l'écran).
- **Gain** : Réduit drastiquement le nombre de pixels à traiter (fill-rate).
- **Qualité** : Un léger downsample (1/2) améliore souvent le look du bloom en le rendant plus diffus.

### 2. Dual Kawase Blur
Alternative performante au flou gaussien pour les grands rayons de flou.
- **Gain** : Moins de passes de rendu et moins de texture fetches par pixel.

### 3. SIMD (Audio)
Le traitement audio (FFT, filtrage) utilise les instructions SIMD (AVX/SSE) via la feature `simd` de Rust pour paralléliser les calculs sur le CPU.

### 5. RAII & Gestion du Lifecycle GPU (Persistent Mapping)
- **Support RAII (`impl Drop`)** : `RendererGraphics` et `RendererGraphicsInstanced` implémentent le trait `Drop`. La destruction de ces structures libère automatiquement et de manière sécurisée l'ensemble des ressources OpenGL (`gl::UnmapBuffer` sur le VBO persistant, `gl::DeleteVertexArrays`, `gl::DeleteBuffers`, `gl::DeleteProgram`, `gl::DeleteSync`).
- **Calcul de Capacité GPU Dynamique (`recreate_buffers`)** : Lors d'une ré-initialisation ou synchronisation des moteurs (`physic.apply`), la taille des buffers de particules pour le rendu par points est recalculée selon la formule `max_rockets * (particles_per_explosion + particles_per_trail)` pour accueillir l'ensemble des particules d'explosions et de traînées actives sans débordement de buffer.

## ⌨️ Liste des Commandes (Console F1)

### Renderer
- `renderer.bloom.enable <true|false>` : Active/Désactive le bloom.
- `renderer.bloom.method <gaussian|kawase>` : Change l'algo de flou.
- `renderer.bloom.downsample <1|2|4>` : Change la résolution du bloom.
- `renderer.bloom.intensity <val>` : Règle l'intensité (ex: 2.0).
- `renderer.bloom.iterations <val>` : Règle les itérations (Gaussian).
- `renderer.reload_shaders` : Recharge les shaders à chaud.

### Audio
- `audio.volume <0.0-1.0>` : Règle le volume global.
- `audio.mute <true|false>` : Coupe le son.

### Physique
- `physic.gravity <x> <y>` : Change la gravité (ex: 0.0 -9.81).
- `physic.wind <x> <y>` : Change le vent.
- `physic.reset` : Réinitialise la simulation.

### Système
- `clear` : Efface la console.
- `help` : Affiche l'aide.
