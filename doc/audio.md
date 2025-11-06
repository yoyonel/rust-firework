audio_engine.rs
-------------------------
3D Fireworks Audio Engine
-------------------------
This file implements a lightweight 3D audio engine in Rust.
It supports:
- Loading WAV audio files
- Linear resampling
- Binaural spatialization (ITD + ILD)
- Distance attenuation with low-pass filtering
- Smooth fade-in and fade-out per voice
- Concurrent playback of multiple voices
- Streaming audio output via CPAL
The code is designed to be educational, with comments explaining
both Rust techniques and audio DSP concepts.

Main Thread
-----------
```
[Initialize FireworksAudio3D]
    |
    |-- Load WAVs (rocket / explosion)
    |-- Resample if needed
    |-- Prepare voices pool
    |-- Initialize settings
    |
[Start Audio Thread] ------------------------------+
    |                                              |
    |-- Spawn audio thread                          |
    |                                              |
    |                                              |
    +----------------------------------------------+
                                                   |
                                                   v
```

Audio Thread
------------
```
[CPAL Output Stream Setup]
    |
    |-- Pre-warm: fill first 500ms of audio with silence
    |       (accumulator + chunk buffers)
    |
    |-- Play stream
    |
[Loop: wait on Condvar] <--------------------------+
    |
    |-- Mutex lock on running + queue + voices
    |
    |-- While running = true:
    |       wait on Condvar (sleep efficiently)
    |       when woken up:
    |           - check play queue
    |           - assign sounds to inactive voices
    |           - process voices:
    |               * fade-in / fade-out
    |               * low-pass filter
    |               * binaural/panning
    |           - mix voices into accumulator
    |           - write final samples to CPAL buffer
    |
    |-- If running = false:
    |       exit loop
    |
    v
Thread terminates cleanly
```

Main Thread (stop)
-----------------
```
[Stop Audio Thread]
    |
    |-- Lock running mutex
    |-- Set running = false
    |-- Notify Condvar
    |-- Join / cleanup
```