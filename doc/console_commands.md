# Console Commands Reference

This document lists all available console commands in the Fireworks Simulator.
Access the console by pressing `F1` (or `` ` `` depending on configuration).

## System
| Command | Description |
| :--- | :--- |
| `help` | Lists available commands. |
| `clear` | Clears the console output. |

## Audio
| Command | Description |
| :--- | :--- |
| `audio.mute` | Mute all audio. |
| `audio.unmute` | Unmute audio. |

## Physics & Simulation

### General
| Command | Description |
| :--- | :--- |
| `physic.config` | Displays current physics configuration. |

### Explosion Shapes (Standard)
| Command | Usage | Description |
| :--- | :--- | :--- |
| `physic.explosion.shape` | `[spherical]` | Show current shape info, or reset to spherical. |
| `physic.explosion.image` | `<path> [scale] [time]` | Load a single image shape.<br>Default scale: 150.0, time: 1.5s. |
| `physic.explosion.preset`| `<name> [weight]` | Load a built-in preset (`heart`, `star`, `smiley`, `note`, `ring`).<br>If `weight` is provided, adds it to the weighted list (see below). |
| `physic.explosion.scale` | `<value>` | Set scale for current explosion shape(s). |
| `physic.explosion.flight_time` | `<value>` | Set flight time/deployment speed (seconds). |

### Explosion Shapes (Weighted Multi-Image)
These commands allow you to mix multiple shapes with different probabilities.

| Command | Usage | Description |
| :--- | :--- | :--- |
| `physic.explosion.add` | `<path> <weight> [scale] [time]` | Add a new image to the current set with a specific probability weight. |
| `physic.explosion.weight`| `<name> <new_weight>` | Update the probability weight of an existing loaded image.<br>Use TAB completion to see loaded image names. |
| `physic.explosion.stats` | | Show current probability distribution of loaded shapes. |

**Example Workflow (Multi-Image):**
```bash
# Start fresh (spherical)
physic.explosion.shape spherical

# Add a heart with weight 1.0 (base probability)
physic.explosion.add assets/textures/explosion_shapes/heart.png 1.0

# Add a star with weight 3.0 (3x more likely than heart)
physic.explosion.add assets/textures/explosion_shapes/star.png 3.0

# Add a preset smiley with weight 0.5 (rare)
physic.explosion.preset smiley 0.5

# Check stats
physic.explosion.stats
```

## Renderer

### Configuration
| Command | Description |
| :--- | :--- |
| `renderer.config` | View current renderer configuration. |
| `renderer.config.save` | Save current settings to `assets/config/renderer.toml`. |
| `renderer.config.reload` | Reload settings from disk. |
| `renderer.reload_shaders` | Hot-reload all shaders. |

### Bloom (Post-Processing)
| Command | Usage | Description |
| :--- | :--- | :--- |
| `renderer.bloom.enable` | | Enable bloom effect. |
| `renderer.bloom.disable` | | Disable bloom effect. |
| `renderer.bloom.intensity` | `<0.0-10.0>` | Set bloom intensity / brightness multiplier. |
| `renderer.bloom.iterations` | `<1-10>` | Number of blur passes (Gaussian only). |
| `renderer.bloom.downsample` | `<1\|2\|4>` | Resolution divisor (2 is recommended). |
| `renderer.bloom.method` | `<gaussian\|kawase>` | Switch blur algorithm. |
| `renderer.bloom.threshold` | `<0.0-1.0>` | Brightness threshold for bloom extraction. |

### Tone Mapping
| Command | Usage | Description |
| :--- | :--- | :--- |
| `renderer.tonemapping` | `<method>` | Set tone mapping operator.<br>Methods: `reinhard`, `aces`, `filmic`, `uncharted2`. |
