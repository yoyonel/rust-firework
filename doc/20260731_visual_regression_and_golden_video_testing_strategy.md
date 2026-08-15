# Visual Non-Regression & Golden Image / Video Testing Strategy

## 1. Overview & Objectives

To guarantee visual fidelity, prevent shader/pipeline regressions (such as MRT buffer corruption, alpha blending cutout artifacts, or tone-mapping/blur regressions), `rust-firework` establishes a **Golden Reference Dataset & E2E Visual Regression Pipeline**.

This strategy provides:
1. **Deterministic Offline Frame/Clip Capture**: Renders predictable simulation state matrix snapshots without manual user interaction.
2. **100% GUI-Free Capture Rule**: All golden reference snapshots and videos are recorded with ImGui hidden (`gui_open = false` or headless FBO rendering) to prevent control panels from obscuring visual rendering.
3. **Human Baseline Validation Protocol**: A structured workflow for human inspection and sign-off on golden reference images and videos.
4. **Automated E2E Pixel & Structural Non-Regression Testing**: Automated image diff comparison (using MSE / SSIM pixel delta thresholds) integrated into CI/CD pipelines.

---

## 2. Test Parameter Matrix

The test matrix covers all combinations of graphical settings across Renderer, Post-FX, Tone Mapping, and Element Visibility:

| Test Group | Parameter | Combinations / Variants Tested |
| :--- | :--- | :--- |
| **Tone Mapping** | Mode (`renderer.tonemapping`) | `Reinhard`, `ReinhardExtended`, `ACES`, `Uncharted2`, `AgX`, `KhronosPBR` |
| **Bloom Pipeline** | Blur Method (`renderer.bloom.method`) | `Gaussian`, `Kawase` |
| **Bloom Pipeline** | Downsample Factor | `1x` (Full), `2x` (Half), `4x` (Quarter), `8x` (Eighth) |
| **Bloom Pipeline** | Intensity & Iterations | Default (`2.676`, `4`), High (`5.0`, `8`), Disabled (`0.0`, `0`) |
| **Visibility Toggles** | Graphical Elements | All Active (`Rockets+Smoke+Trails+Explosions`), Smoke Only, Rockets Only, Explosions Only |

---

## 3. Human Validation Protocol (Golden Dataset Baseline)

Golden reference images are stored in `tests/visual_baselines/`:

```
tests/visual_baselines/
├── tonemapping_reinhard.png
├── tonemapping_aces.png
├── tonemapping_agx.png
├── tonemapping_khronos_pbr.png
├── bloom_gaussian_2x.png
├── bloom_kawase_4x.png
├── visibility_smoke_only.png
├── visibility_rockets_only.png
└── manifests/golden_index.json
```

### Baseline Creation Command
Human maintainers regenerate and sign off on reference baselines using Taskfile:
```bash
# Capture full matrix of golden snapshots for human validation
task test:visual-baselines-generate
```

Each generated snapshot is logged in `tests/visual_baselines/manifests/golden_index.json` along with its SHA-256 checksum and timestamp.

---

## 4. E2E Non-Regression Automated Verification

The integration test suite (`tests/visual_regression_test.rs`) runs during `cargo test --features interactive_tests` or `task test:all`:

```rust
#[test]
fn test_visual_non_regression_kawase_vs_golden() {
    let current_frame = capture_offline_frame_with_config(&KawaseConfig::default());
    let golden_frame = load_golden_reference("bloom_kawase_4x.png");
    
    let diff = compute_pixel_delta_mse(&current_frame, &golden_frame);
    assert!(diff < MAX_ALLOWED_PIXEL_DELTA, "Visual regression detected in Kawase Bloom pass: MSE={}", diff);
}
```

---

## 5. Summary of Kawase Blur & MRT Shader Fixes

- **MRT Write Mask Protection**: Added `gl::ColorMaski(1, GL_FALSE, GL_FALSE, GL_FALSE, GL_FALSE)` during `SmokeRenderer` passes to prevent non-emissive smoke quads from erasing `COLOR_ATTACHMENT1` (Bloom mask).
- **Post-Processing State Isolation**: Explicitly disabled `gl::BLEND` during Gaussian and Kawase blur passes to eliminate alpha accumulation artifacts.
- **Kawase Ping-Pong Target Sync**: Corrected upsample iteration indexing so that final Kawase blurred texture output lands deterministically in `ping_pong_textures[0]`.
