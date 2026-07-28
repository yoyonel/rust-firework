# Console Commands Reference

This document lists all available console commands in the Fireworks Simulator.
Access the console by pressing `F1` (or `` ` `` depending on configuration).

## System
| Command | Description |
| :--- | :--- |
| `help` | Lists available commands. |
| `clear` | Clears the console output. |

## Audio
| Command | Usage | Description |
| :--- | :--- | :--- |
| `audio.mute` | | Mute all audio. |
| `audio.unmute` | | Unmute audio. |
| `audio.fx` | `<effect> <on\|off>` | Toggle a specific DSP audio effect at runtime (lock-free).<br>Available: `binaural`, `panning`, `distance_atten`, `lowpass`, `doppler`, `fade`, `gain_lerp`, `normalize`, `spatial_bus`, `spatial_reverb`. |
| `audio.fx_all` | `<on\|off>` | Toggle all DSP audio effects at once. |
| `audio.fx_status` | | Display current status of all DSP audio effects (ON/OFF). |
| `audio.reverb_wet` | `<0.0..1.0>` | View or set Spatial Reverb wet mix gain (Default: 0.08 / 8%). |

## Physics & Simulation

### General
| Command | Usage | Description |
| :--- | :--- | :--- |
| `physic.config` | | Displays current applied and pending physics configurations. |
| `physic.apply` | | Applies all pending parameter changes and re-synchronizes engines. |
| `physic.config.save` | | Saves current applied physics configuration to `assets/config/physic.toml`. |
| `physic.config.reload` | | Reloads configuration from `assets/config/physic.toml` and re-synchronizes engines. |

### Configuration Parameters
These commands modify the **pending** configuration. To apply them, run `physic.apply`.

| Command | Usage | Description |
| :--- | :--- | :--- |
| `physic.max_rockets` | `<value>` | Set maximum concurrent rockets. |
| `physic.particles_per_explosion` | `<value>` | Set particles per explosion. |
| `physic.particles_per_trail` | `<value>` | Set particles per trail. |
| `physic.rocket_interval_mean` | `<value>` | Set mean time interval between rocket spawns (seconds). |
| `physic.rocket_interval_variation` | `<value>` | Set variation of interval between rocket spawns. |
| `physic.rocket_max_next_interval` | `<value>` | Set maximum interval constraint between rocket spawns. |
| `physic.spawn_rocket_margin` | `<value>` | Set screen margin for rocket spawns. |
| `physic.spawn_rocket_vertical_angle` | `<value>` | Set vertical spawn angle of rockets (radians). |
| `physic.spawn_rocket_angle_variation` | `<value>` | Set random angle variation of spawned rockets. |
| `physic.spawn_rocket_min_speed` | `<value>` | Set minimum initial speed of spawned rockets. |
| `physic.spawn_rocket_max_speed` | `<value>` | Set maximum initial speed of spawned rockets. |
| `physic.explosion_threshold` | `<value>` | Set speed threshold under which rockets explode. |
| `physic.gravity` | `<value>` | Set gravity value affecting rockets and particles (e.g. -200.0). |
| `physic.initial_rocket_speed` | `<value>` | Set target initial speed (metadata). |
| `physic.explosion_min_vel` | `<value>` | Set minimum velocity of explosion particles. |
| `physic.explosion_max_vel` | `<value>` | Set maximum velocity of explosion particles. |

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
