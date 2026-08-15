# Guide : Validation OpenGL avec Mesa Debug Layer

## Contexte

Le projet utilise des **Persistently Mapped Buffers** (extension `ARB_buffer_storage`, OpenGL 4.4+) 
pour les buffers de particules GPU. Ce pattern impose des contraintes de lifecycle strictes :

> **Spec ARB_buffer_storage §4** : Un buffer mappé persistamment ne doit pas être supprimé
> (`DeleteBuffers`) avant d'avoir été unmappé (`UnmapBuffer`). Toute violation est un
> **comportement indéfini** — crash, corruption silencieuse, ou fuite selon le driver.

Ce guide documente comment **détecter automatiquement** ces violations sans GPU physique,
via le renderer logiciel Mesa (llvmpipe) et le mécanisme `GL_DEBUG_OUTPUT`.

---

## Bug historique corrigé (2026-07-19)

### Symptôme
`RendererGraphics::recreate_buffers()` et `RendererGraphicsInstanced::recreate_buffers()`
appelaient `gl::DeleteBuffers()` **sans** avoir préalablement appelé `gl::UnmapBuffer()`.
La méthode `close()` des mêmes structs le faisait correctement — oubli de copie du pattern.

### Impact
- Sur drivers stricts (NVIDIA + Validation, macOS) : crash ou memory corruption.
- Sur drivers permissifs (Intel/AMD Linux Mesa) : fuite du mapping GPU silencieuse.

### Fix
Extraction d'une fonction privée `release_particle_buffers()` partagée entre
`recreate_buffers()` et `close()` pour garantir une séquence identique :

```
UnmapBuffer → DeleteVertexArrays → DeleteBuffers
```

Fichiers concernés :
- `src/renderer_engine/renderer_graphics.rs`
- `src/renderer_engine/renderer_graphics_instanced.rs`

---

## Variables d'environnement Mesa

| Variable | Valeur | Effet |
| :--- | :--- | :--- |
| `LIBGL_ALWAYS_SOFTWARE=1` | `1` | Force Mesa llvmpipe (pas de GPU requis) |
| `MESA_GL_DEBUG=1` | `1` | Active les messages `GL_DEBUG_OUTPUT` sur stderr |
| `GALLIUM_SOFTPIPE=1` | `1` | Force le backend softpipe (plus strict que llvmpipe) |

### Patterns de messages d'erreur Mesa

Quand une violation est détectée, Mesa émet sur **stderr** :

```
GL_INVALID_OPERATION in glDeleteBuffers(buffer 3 is mapped)
```

ou avec `MESA_GL_DEBUG=1` et le callback `GL_DEBUG_OUTPUT` activé :

```
API_ID_INVALID_OPERATION error has been generated. ...
```

La commande de grep utilisée dans les cibles `make`/`task` pour détecter ces violations :

```bash
grep -qE "GL_INVALID|error generated|API_ID_" /tmp/mesa_gl_debug.log
```

---

## Exécution

### Via Make

```bash
# Tests headless avec Mesa + validation OpenGL
make test-opengl-mesa

# Log complet disponible dans :
cat /tmp/mesa_gl_debug.log
```

### Via Task

```bash
task test:opengl-mesa

# Description détaillée
task --summary test-opengl-mesa
```

### Manuellement

```bash
env \
  LIBGL_ALWAYS_SOFTWARE=1 \
  MESA_GL_DEBUG=1 \
  GALLIUM_SOFTPIPE=1 \
xvfb-run -a cargo test --features interactive_tests -- --test-threads=1 2>&1 | \
tee /tmp/mesa_gl_debug.log
```

---

## Tests verrouillant le fix

Le fichier `tests/renderer_recreate_buffers_test.rs` contient 5 tests
`#[cfg(feature = "interactive_tests")]` qui s'exécutent avec cette cible :

| Test | Invariant vérifié |
| :--- | :--- |
| `test_renderer_graphics_recreate_buffers_ptr_invariant` | `mapped_ptr != null` après recreate, `null` après close |
| `test_renderer_graphics_multiple_recreate_cycles` | 5 cycles sans crash ni double-delete |
| `test_renderer_graphics_instanced_recreate_buffers_ptr_invariant` | Même invariant pour la variante instanced |
| `test_renderer_graphics_instanced_multiple_recreate_cycles` | Cycles multiples pour la variante instanced |
| `test_renderer_high_level_recreate_after_config_change` | Simule `physic.apply` complet via `Renderer` haut-niveau |

Ces tests sont **complémentaires** à Mesa debug :
- **Tests Rust** : vérifient les invariants observables depuis Rust (valeur de `mapped_ptr`).
- **Mesa debug** : vérifie les violations de spec OpenGL côté driver (couche de validation).

---

## Comparaison des approches de validation

| Approche | Sans GPU ? | Automatique | Effort | Détecte ce bug |
| :--- | :--- | :--- | :--- | :--- |
| `cargo clippy` | ✅ | ✅ | Nul | ❌ (code unsafe sémantique) |
| Tests Rust `mapped_ptr` | ✅ (avec Mesa) | ✅ | Faible | ✅ (invariant observable) |
| **Mesa `MESA_GL_DEBUG=1`** | ✅ | ✅ | Faible | ✅ (violation spec driver) |
| RenderDoc In-Application API | ❌ | ✅ (complexe) | Élevé | ✅ |
| RenderDoc CLI + script Python | ❌ | ✅ (avec GPU/CI) | Moyen | ✅ |

### Recommandation

Combiner les deux approches légères :
1. `make test` (rapide, 166 tests, toujours vert)
2. `make test-opengl-mesa` (plus lent, nécessite `interactive_tests`, détecte les UB OpenGL)

---

## Prérequis système

```bash
# Debian/Ubuntu
apt install libgl1-mesa-dri mesa-utils xvfb

# Vérifier llvmpipe disponible
LIBGL_ALWAYS_SOFTWARE=1 glxinfo | grep renderer
# → OpenGL renderer string: llvmpipe (LLVM x.y, 256 bits)

# Vérifier la version OpenGL supportée (doit être >= 4.4 pour ARB_buffer_storage)
LIBGL_ALWAYS_SOFTWARE=1 glxinfo | grep "OpenGL version"
# → OpenGL version string: 4.5 (Compatibility Profile) Mesa x.y.z
```

Sur ce projet : Mesa 25.0.7, llvmpipe LLVM 19.1.7, OpenGL 4.5 ✅

---

## Liens de référence

- [ARB_buffer_storage spec](https://registry.khronos.org/OpenGL/extensions/ARB/ARB_buffer_storage.txt)
- [GL_DEBUG_OUTPUT (OpenGL Wiki)](https://www.khronos.org/opengl/wiki/Debug_Output)
- [Mesa Environment Variables](https://docs.mesa3d.org/envvars.html)
- [Persistent Mapped Buffers (AZDO)](opengl_azdo_persistent_mapped_buffers.md)
