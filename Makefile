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
	@$(CARGO) fmt --all -- --check

clippy:
	@echo "🕵️  Vérification statique avec Clippy..."
	@$(CARGO) clippy --all-targets --all-features -- -D warnings

# Lint the code
lint:
	cargo fmt -- --check
	cargo clippy -- -D warnings

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

# Profiling with Heaptrack
heaptrack: ./target/release/fireworks_sim
	heaptrack ./target/release/fireworks_sim

# -----------------------------------------
# 💡 Cible par défaut
# -----------------------------------------
.DEFAULT_GOAL := test
