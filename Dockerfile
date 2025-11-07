FROM rust:1.90.0-slim-bookworm AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential cmake g++ \
    libgl1-mesa-dev libx11-dev libxcursor-dev libxi-dev \
    libxrandr-dev libxinerama-dev libglu1-mesa-dev \
    libasound2-dev alsa-utils alsa-oss \
    pulseaudio pulseaudio-utils \
    xvfb xauth pkg-config make git ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# --- Étape de cache Cargo ---
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo fetch

# --- Copie réelle du projet ---
COPY . .

# --- tests sans exécution ---
RUN cargo test --no-run

# --- Installer cargo-llvm-cov et LLVM tools ---
RUN cargo install cargo-llvm-cov
RUN rustup component add llvm-tools-preview --toolchain 1.90.0-x86_64-unknown-linux-gnu

# --- Étape 2 : Image finale pour exécution des tests ---
FROM builder AS tester
WORKDIR /app

# Configurer Xvfb
ENV DISPLAY=:99

# Persister le target pour éviter recompilation
VOLUME /app/target

# Entrypoint pour exécuter les tests avec coverage
CMD ["bash", "-c", "\
    # Nettoyer les éventuels locks X11 \
    rm -f /tmp/.X99-lock && \
    # Lancer Xvfb en background \
    Xvfb :99 -screen 0 1024x768x24 & \
    sleep 2 && \
    echo '🧪 Running functional tests with virtual display...' && \
    # Préparer les binaires instrumentés (sans exécuter tests) \
    cargo llvm-cov --workspace --no-run && \
    # Lancer les tests avec coverage et capture du rapport \
    cargo llvm-cov --workspace --tests -- --nocapture --test-threads=1 && \
    echo '✅ Tests completed successfully.' \
    "]
