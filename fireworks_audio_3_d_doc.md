# FireworksAudio3D: Deep Dive into Architecture and Spatial Audio DSP

## Overview

This document explores the **FireworksAudio3D engine**, a real-time 3D audio system written in Rust.  
We focus on **two complementary aspects**:

1. **Concurrency & real-time architecture** — how the engine handles audio threads, queues, and synchronization.
2. **Spatial DSP & psychoacoustic modeling** — how sounds are spatialized, attenuated, and filtered.

The goal is to give a conceptual and practical understanding for developers familiar with Rust and audio programming.

---

## 1. Engine Architecture

### 1.1 Threading Model

The engine separates **audio rendering** from the **main game/application thread**:

```
Main Thread          Audio Thread
-----------          ------------
enqueue_sound() ---> [PlayRequest Queue] ---> mix voices -> output stream
```

- `Main Thread` schedules sounds using `enqueue_sound()`.
- `Audio Thread` continuously reads from a **queue** and renders audio blocks to the output device.
- Communication is mediated via `Arc<Mutex<VecDeque<PlayRequest>>>`.

### 1.2 Queue & Synchronization

**Key points:**

- `Arc` (Atomic Reference Counting) allows multiple threads to share ownership of the queue safely.
- `Mutex` ensures **exclusive access** to the queue when adding or removing `PlayRequest`s.
- **Trade-offs**:
  - Pros: Safe, simple to implement, easy to reason about.
  - Cons: Blocking locks can be costly if the main thread produces many events per frame.

**Visualizing queue access:**

```
Main Thread                   Audio Thread
------------                   ------------
lock(queue)                     lock(queue)
push(PlayRequest)               pop_front() while not empty
unlock(queue)                   unlock(queue)
```

- Locks are held **only briefly**, minimizing the chance of audio dropouts.

---

## 2. Real-Time Audio Flow

### 2.1 CPAL Output Stream

- CPAL manages the hardware output.
- Audio is rendered in **blocks** of `block_size` frames (e.g., 1024 samples).
- Each block is processed in a callback:

```
+---------------------------+
| Audio Callback (CPAL)     |
+---------------------------+
        |
        v
+---------------------------+
| Reset accumulator buffer  |
+---------------------------+
        |
        v
+---------------------------+
| For each voice:           |
|   Apply fade-in/out       |
|   Apply low-pass filter   |
|   Mix into accumulator    |
+---------------------------+
        |
        v
+---------------------------+
| Apply global gain & tanh  |
| Write to output buffer    |
+---------------------------+
```

### 2.2 Zero-Allocation Strategy

- Uses **pre-allocated buffers** (`acc` & `chunk`) to avoid heap allocations per callback.
- Avoiding allocations is crucial for **real-time audio** to prevent dropouts or glitches.

---

## 3. Digital Signal Processing (DSP) Model

### 3.1 Mono-to-Stereo Binauralization

- Converts mono sound into stereo with **spatial cues**.
- Uses **ITD (Interaural Time Difference)** and **ILD (Interaural Level Difference)**.

```
Listener
   O
  / \
 L   R   <- sound arrives with ITD + ILD
```

**ITD (time delay between ears)**

- Calculated using head radius `r` and azimuth `θ`.
- Simplified formula:  
  `itd = (head_radius / c) * (θ + sin(θ))`
- c = speed of sound (≈ 343 m/s).

**ILD (level difference between ears)**

- `ild_db = max_ild_db * sin(θ)`
- Gain applied: `10^(-ild_db / 20)`

### 3.2 Distance Attenuation

- Attenuates sound by distance linearly for simplicity:

```
att = max(1 - distance / max_distance, 0)
```

- More complex models (inverse-square law) could be used for realistic falloff.

### 3.3 Low-Pass Filtering

- First-order filter applied per channel to simulate **distance-dependent muffling**.
- Filter coefficient:  
  `a = dt / (RC + dt)`, where `RC = 1 / (2πfc)` and `fc` depends on distance.
- Advantage: cheap computation, smooth attenuation.
- Limitation: only approximate frequency response, no resonance control.

### 3.4 Fade-In / Fade-Out

- Smooths the start/end of sounds to avoid clicks.
- Linear ramp is applied over `fade_in_samples` and `fade_out_samples`.
- Advantage: simple, robust.
- Limitation: not envelope-based, may feel mechanical for long sounds.

### 3.5 Advantages & Limitations

| Feature                  | Advantages                                   | Limitations / Approximations                  |
|--------------------------|---------------------------------------------|-----------------------------------------------|
| Binaural ITD/ILD         | Simple, gives good spatial cues             | No HRTF, no pinna filtering                   |
| Distance attenuation     | Easy to compute                             | Linear, ignores inverse-square physics       |
| Low-pass filter          | Efficient, smooth frequency roll-off        | Single-pole, no complex frequency shaping   |
| Fade-in/out              | Eliminates clicks                           | Linear only, no ADSR modeling               |
| Pre-allocated buffers    | Real-time safe                              | Fixed block size, memory overhead           |
| Mutex + Arc              | Thread-safe                                 | Blocking possible if many threads contend   |

---

## 4. Summary Diagrams

### Audio Flow End-to-End

```
[ WAV file ]
    ↓
[ load_audio() ]
    ↓
[ resample_linear() ]
    ↓
[ prepare_voice() ]
   ↳ binauralize_mono() / panning
    ↓
[ enqueue_sound() ]
    ↓
[ PlayRequest Queue ] ← Arc<Mutex>
    ↓
[ Audio Thread ]
   ↳ mix voices
   ↳ lowpass + fade + limiter
    ↓
[ Output Stream (CPAL) ]
```

### Thread Interaction

```
Main Thread                           Audio Thread
-----------                           ------------
enqueue_sound()                        lock(queue)
push PlayRequest                        pop PlayRequest
unlock(queue)                           unlock(queue)
                                        mix voices
                                        apply fade/filters
                                        write to CPAL buffer
```

### DSP Processing for One Voice

```
Voice.data [mono/stereo]
    ↓ (fade-in/out)
[Linear fade applied]
    ↓ (low-pass filter)
[Filtered samples]
    ↓ (mix)
Accumulate into block buffer
```