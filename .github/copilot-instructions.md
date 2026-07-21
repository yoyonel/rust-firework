# Development & Commit Discipline Rules

## 🚫 Golden Rule: Commits & Pushes

1. **NEVER run `git commit` or `git push` without explicit approval/validation from the user in chat.**
   - All code, tests, and refactoring MUST be presented and tested/validated locally by the user first.
   - Wait for explicit user confirmation (e.g. "ok tu peux commit", "push", "valide") before executing any `git commit` or `git push` operations.
2. **Format + Lint + Tests REQUIRED** — All must pass before presenting code for validation (`cargo test` and `task lint`).
3. **Separation of Concerns (SoC)** — Keep changes focused, modular, and incremental.

## 🎧 Real-Time Audio & Memory Management Rules (CPAL Thread)

4. **Zero Heap Allocation in Audio Callback / Hot Paths**:
   - `Vec::new()`, `String`, `format!()`, `.resize()`, `Box::new()`, `Arc::new()`, and implicit `Arc` drops MUST NEVER occur inside the CPAL real-time audio thread (`DspProcessor::process_block`, `SpatialReverb::process_block`, etc.).
   - When resetting or stealing an active voice holding an `Arc`, ALWAYS extract `v.data.take()` and forward the dead `Arc` via `garbage_tx` so OS deallocation happens on the main thread.
   - Pre-allocate all audio accumulation buffers (`acc`, `bus_w`, `bus_x`, `export_buffer`) to maximum supported block size at initialization.
5. **Lock-Free Synchronization**:
   - NO blocking `Mutex::lock()`, `RwLock::write()`, or system blocking calls inside the CPAL callback thread. Use `try_lock()` or lock-free atomics (`AtomicU32`, `AtomicVec2`).
6. **Zero Code Duplication across Hot Paths (DRY)**:
   - Extract reusable DSP calculations (distance attenuation, lowpass alpha constants) into inline helper functions on `DspProcessor` instead of duplicating math blocks across legacy and spatial bus rendering paths.
