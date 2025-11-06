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

RUN cargo build --release && cargo test --no-run

FROM builder AS tester
WORKDIR /app
ENV DISPLAY=:99

CMD ["bash", "-c", "\
    Xvfb :99 -screen 0 1280x720x24 & \
    sleep 2 && \
    echo '🧪 Running functional tests with virtual display...' && \
    cargo test --no-run && cargo test -- --nocapture --test-threads=1 && \
    echo '✅ Tests completed successfully.' \
    "]