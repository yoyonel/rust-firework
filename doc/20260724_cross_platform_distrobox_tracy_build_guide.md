# 🌐 Architecture Build Hybride Multi-Plateforme (Distrobox, Tracy & NVIDIA)

Ce document décrit le fonctionnement et la configuration du système de build et d'exécution hybride multi-environnement de `rust-firework`.

---

## 🎯 1. Objectifs & Enjeux

Le projet doit pouvoir être développé et exécuté sur deux environnements distincts sans modification manuelle du code ou de `Cargo.toml` :
1. **Environnement A (Host Debian 13 natif)** : Outils de dev installés directement sur l'hôte, avec un fork local de `tracy-client` supportant la couleur dynamique (`span_alloc_color`).
2. **Environnement B (Host Bazzite / Fedora + Distrobox `clang-dev`)** : Environnement conteneurisé pour la compilation (avec dépendances C telles que `alsa-lib-devel`, `cmake`), tout en conservant le rendu graphique natif NVIDIA (GeForce GTX 950M) et les overlays (MangoHud) sur l'hôte.

---

## 🏗️ 2. Architecture de Build Hybride (`Taskfile.yml`)

### Détection Dynamique du Conteneur
Le `Taskfile.yml` contient une variable d'exécution shell dynamique `.DISTROBOX` :

```yaml
vars:
  DISTROBOX:
    sh: 'if [ -z "$CONTAINER_ID" ] && command -v distrobox >/dev/null 2>&1 && distrobox list --no-color 2>/dev/null | grep -w "clang-dev" >/dev/null; then echo "distrobox enter clang-dev --"; else echo ""; fi'
  CARGO: '{{.DISTROBOX}} cargo'
```

### Séparation Responsabilité Build vs Run
- **Cibles de compilation** (`build:release`, `build:tracy`, `test`, `clippy`, etc.) :  
  Utilisent `{{.CARGO}}` (`distrobox enter clang-dev -- cargo ...`) pour exécuter la chaîne de compilation Rust et les compilations de dépendances système C dans le conteneur `clang-dev`.
- **Cibles d'exécution** (`run:release`, `run:prime-hud`, `run:tracy`, etc.) :  
  Dépendent de la cible de build (`deps: [build-release]`), puis exécutent le binaire produit (`./target/release/fireworks_sim`) **directement sur l'hôte**.

### Pourquoi cette séparation ?
L'exécutable binaire tournant sur l'hôte accède directement :
- Au pilote propriétaire NVIDIA via PRIME Offload.
- À la bibliothèque partagée MangoHud natif.
- Aux paramètres d'environnement configurés localement via `direnv`.

---

## ⚡ 3. Compatibilité Conditionnelle Tracy Profiler

Le projet supporte à la fois la version standard de `tracy-client` (sur crates.io) et la version patchée avec prise en charge de la couleur dynamique.

### Feature Flag `tracy_custom_color`
Dans `Cargo.toml` :
```toml
[features]
tracy = ["tracy-client"]
tracy_custom_color = ["tracy"]
```

Dans `src/renderer_engine/utils/gpu_profiler.rs` :
```rust
#[cfg(feature = "tracy")]
let tracy_span = self.tracy_ctx.as_ref().and_then(|ctx| {
    #[cfg(feature = "tracy_custom_color")]
    {
        ctx.span_alloc_color(name, "GPU", _file, _line, color).ok()
    }
    #[cfg(not(feature = "tracy_custom_color"))]
    {
        ctx.span_alloc(name, "GPU", _file, _line).ok()
    }
});
```

### Patch Automatique Local via `.envrc`
Afin d'éviter d'inclure des chemins absolus ou relatifs cassés dans `Cargo.toml`, le fichier `.envrc` (géré par `direnv`) génère ou supprime à la volée la configuration locale `.cargo/config.toml` (ignorée par Git) :

```sh
TRACY_FORK_PATH="../rust_tracy_client/tracy-client"
CARGO_CONFIG_FILE=".cargo/config.toml"

if [ -d "$TRACY_FORK_PATH" ]; then
    mkdir -p .cargo
    cat <<EOF > "$CARGO_CONFIG_FILE"
# Fichier généré automatiquement par .envrc (Machine avec fork local tracy-client)
[patch.crates-io]
tracy-client = { path = "../rust_tracy_client/tracy-client" }
EOF
else
    if [ -f "$CARGO_CONFIG_FILE" ] && grep -q "rust_tracy_client" "$CARGO_CONFIG_FILE" 2>/dev/null; then
        rm -f "$CARGO_CONFIG_FILE"
    fi
fi
```

---

## 🛠️ 4. Fichier d'Environnement Local (`.envrc`)

Le fichier `.envrc` configure l'environnement matériel et les outils d'overlay :

```sh
# PRIME Offload pour forcer le GPU NVIDIA dédié
export __NV_PRIME_RENDER_OFFLOAD=1
export __GLX_VENDOR_LIBRARY_NAME=nvidia
export __VK_LAYER_NV_optimus=NVIDIA_onl

# Désactivation de la VSync pour la mesure brute des FPS
export vblank_mode=0
export __GL_SYNC_TO_VBLANK=0

# Overlay MangoHud et préchargement OpenGL
export MANGOHUD=1
export LD_PRELOAD=/usr/lib64/mangohud/libMangoHud_shim.so
```

---

## 🧪 5. Commandes Usuelles

```sh
# Compilation et exécution standard avec GPU NVIDIA & MangoHud
task run:release

# Compilation et exécution avec profilage Tracy activé
task run:tracy

# Exécution des tests unitaires (automatiquement sous Virtual Framebuffer et Distrobox)
task test:all
```
