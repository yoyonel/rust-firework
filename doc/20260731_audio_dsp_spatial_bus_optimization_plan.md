# Audio DSP Spatial Bus Optimization Specification & Benchmark Plan

## 1. Executive Summary & Profiling Baseline

During performance profiling using Valgrind Callgrind (`task valgrind-callgrind`), the audio engine's 2D Spatial Bus processing (`process_dsp_spatial_bus`) was identified as the primary CPU hotspot of `rust-firework`.

### Callgrind Instruction Cost Breakdown
- Total application instruction footprint: `4,196,094,627` instructions (`Ir`).
- **`process_dsp_spatial_bus` cumulative footprint**: **71.99%** of total CPU cycles (`3,020,722,723` instructions).
- **Instanced Smoke System overhead (`SmokeSystem` + `SmokeRenderer`)**: **~2.01%** of total CPU cycles (`11,192,288` instructions).

```text
2,593,917,463 (61.82%)  src/audio_engine/dsp_processor.rs:<fireworks_sim::audio_engine::dsp_processor::DspProcessor>::process_dsp_spatial_bus
  145,236,564 ( 3.46%)  /rustlib/src/rust/library/core/src/slice/index.rs (Bounds Checks)
   72,829,470 ( 1.74%)  /rustlib/src/rust/library/core/src/num/f32.rs (Scalar float ops)
   36,343,890 ( 0.87%)  /rustlib/src/rust/library/core/src/iter/range.rs (Loop iterators)
   36,309,141 ( 0.87%)  /rustlib/src/rust/library/core/src/cmp.rs
```

---

## 2. Inefficiency & Root Cause Analysis

While the 2D Ambisonics $(W, X)$ spatial bus algorithm is mathematically optimal ($O(1)$ channel decoding relative to output channels), the Rust micro-implementation exhibits 4 low-level bottlenecks:

1. **Repeated Slice Bounds Checks (3.46% Callgrind)**: Direct indexing `self.bus_w[i]`, `self.bus_x[i]`, and `slice_ref[index]` forces the Rust compiler to emit runtime bound assertions at every sample iteration.
2. **Invariant Branching inside Hot Loop**:
   - `apply_fade_in_out` evaluates `if index < fade_in_samples` and `if remaining < fade_out_samples` on every sample tick.
   - `apply_iir_lowpass` performs a `if filtered.abs() < 1e-15` denormal check on every sample tick.
3. **On-the-Fly Stereo-to-Mono Downmixing**: `interpolate_mono_sample` reads `[f32; 2]` and calculates `(s[0] + s[1]) * 0.5` at every iteration.
4. **Scalar Loop Execution**: Accumulation (`bus_w` / `bus_x`) and stereo decoding (`w ± FRAC_1_SQRT_2 * x`) run in scalar 1-wide `f32` loops without AVX2/SSE vectorization.

---

## 3. Incremental Optimization Plan

### Phase 1: Bounds Check Elimination & Loop Unswitching (Target: -30% CPU)
- Replace direct slice indexing with contiguous iterator pairs (`zip`) or raw mutable slice references `&mut [f32]`.
- Unswitch the fade-in/fade-out loop: process the initial fade-in block and final fade-out block separately, keeping the main center loop 100% branchless.
- Replace per-sample denormal checks with CPU Flush-To-Zero (FTZ) or subnormal dither (+1e-25f32).

### Phase 2: Native Mono Audio Buffering (Target: -15% CPU)
- Retain mono audio sources as contiguous `Vec<f32>` buffers inside `Voice.data`.
- Remove `(sample[0] + sample[1]) * 0.5` downmixing from `interpolate_mono_sample`.

### Phase 3: SIMD 8-Wide Vectorization (Target: -40% CPU)
- Vectorize accumulation loops `bus_w[i] += sample * w_weight` using 8-wide float vectors (`f32x8` / AVX2).
- Vectorize stereo decoding `w ± FRAC_1_SQRT_2 * x` and soft-clipping/normalization using SIMD.

---

## 4. Benchmark & Quality Verification Plan

### Performance Benchmark Suite
1. **Criterion Micro-benchmarks**:
   ```bash
   task bench-save-baseline -- audio_v1_baseline
   task bench-compare -- audio_v1_baseline
   ```
   Measures throughput in microseconds per 256-sample audio block across 1 to 512 active voices.
2. **Instruction Count Reduction**:
   ```bash
   task valgrind-callgrind
   ```
   Verifies that total instruction count (`Ir`) and `slice::index.rs` bounds check overhead decrease.

### Audio Quality & Bit-Exact Verification
1. **Phase Continuity & Derivative Derivative Check**:
   - Run `test_phase_continuity_across_block_boundaries` in `src/audio_engine/dsp_processor/tests.rs`.
   - Asserts maximum sample error $|S_{\text{ref}} - S_{\text{opt}}| < 10^{-5}$ and derivative slope continuity across block boundaries.
2. **Signal-to-Noise Ratio (SNR > 100 dB)**:
   - Compute SNR against unoptimized reference render:
     $$\text{SNR} = 10 \cdot \log_{10} \left( \frac{\sum S_{\text{ref}}^2}{\sum (S_{\text{ref}} - S_{\text{opt}})^2} \right) > 100 \text{ dB}$$
   - Guarantees zero pops, clicks, or audible phase distortion.
