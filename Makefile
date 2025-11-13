# =========================================
# 🚀 Makefile pour projet Rust + tests + coverage
# =========================================

APP_NAME = fireworks_sim
CARGO = cargo
COVERAGE_DIR = target/llvm-cov
# X Virtual FrameBuffer
XVFB = xvfb-run -a

# Run the project in release mode
run-release:
	cargo run --release

# Gallium HUD (intel compatible)
run-release-with-hud:
	env \
	vblank_mode=0 \
	__GL_SYNC_TO_VBLANK=0 \
	GALLIUM_HUD_PERIOD=0.15 \
	GALLIUM_HUD="cpu;fps;N vertices submitted" \
	cargo run --release 2>&1

# MangoHub (NVidia compatible) shortcut: RIGHT-SHIFT+F10 for chaging the HUD mode
run-prime-with-hud:
	echo -e "ℹ️🖥️ Using MangoHud, press RSHIFT+F10 for chaging the HUD mode."
	env \
	__NV_PRIME_RENDER_OFFLOAD=1 \
	__GLX_VENDOR_LIBRARY_NAME=nvidia \
	__VK_LAYER_NV_optimus=NVIDIA_onl \
	vblank_mode=0 \
	__GL_SYNC_TO_VBLANK=0 \
	RUST_LOG=INFO \
	mangohud ./target/release/fireworks_sim 2>&1

# -----------------------------------------
# 🧪 Tests unitaires + d'intégration
# -----------------------------------------
test:
	@echo "▶️  Lancement des tests..."
	@$(XVFB) $(CARGO) test --all --quiet

# -----------------------------------------
# 🧹 Nettoyage
# -----------------------------------------
clean:
	@echo "🧹 Nettoyage des artefacts..."
	@$(CARGO) clean

# -----------------------------------------
# 📈 Couverture de tests (llvm-cov)
# -----------------------------------------
coverage:
	@echo "📊 Génération du rapport de couverture avec cargo-llvm-cov..."
	@$(XVFB) $(CARGO) llvm-cov --workspace --html --output-dir $(COVERAGE_DIR) --ignore-filename-regex 'tests/'
	@echo "✅ Rapport généré : $(COVERAGE_DIR)/index.html"
	@xdg-open $(COVERAGE_DIR)/index.html 2>/dev/null || open $(COVERAGE_DIR)/index.html 2>/dev/null || true

coverage-without-tests:
	@$(XVFB) $(CARGO) llvm-cov --ignore-filename-regex "tests/"

# -----------------------------------------
# 📦 Build optimisé
# -----------------------------------------
release:
	@echo "⚙️  Compilation en mode release..."
	@$(CARGO) build --release

# -----------------------------------------
# 🧰 Vérification de formatage & lint
# -----------------------------------------
fmt:
	@echo "🎨 Vérification du formatage..."
	@$(CARGO) fmt -- --check

clippy:
	@echo "🕵️  Vérification statique avec Clippy..."
	@$(CARGO) clippy -- -D warnings

# Lint the code
lint: fmt clippy

# Run cargo-shear for removing unused dependencies
remove-unused-dependencies:
	cargo shear --fix

# -----------------------------------------
# 🧪 Benchmarks
# -----------------------------------------
# Profiling with Valgrind
valgrind-callgrind: ./target/release/fireworks_sim
	valgrind --tool=callgrind ./target/release/fireworks_sim
	callgrind_annotate $(ls -tr | grep callgrind.out | tail -1) | grep -e "fireworks_sim::"

./target/profiling/fireworks_sim:
	cargo build --profile profiling

# Profiling with Heaptrack
heaptrack: ./target/profiling/fireworks_sim
	heaptrack ./target/profiling/fireworks_sim

# -----------------------------------------
# 💡 Cible par défaut
# -----------------------------------------
.DEFAULT_GOAL := test
