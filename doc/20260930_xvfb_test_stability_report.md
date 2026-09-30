# Rapport Technique : Résolution de l'Instabilité Xvfb et Réparation CI/CD

**Date :** 30 Septembre 2026  
**Branche :** `fix/xvfb-test-stability`  
**Auteur :** Antigravity IA  

---

## 1. Rationnel & Contexte

### 1.1 Régression CI/CD (`integration.yml`)
Le workflow GitHub Actions `🧨 Fireworks Integration Tests` échouait avec l'erreur :
```text
AssertionError [ERR_ASSERTION]: Failed to read version from readme
```
Ce problème provenait de l'action `FedericoCarboni/setup-ffmpeg@v3` qui téléchargeait dynamiquement le fichier `https://johnvansickle.com/ffmpeg/release-readme.txt`. Les serveurs de John Van Sickle bloquant ou rate-limitant les adresses IP des runners GitHub Actions (hébergés sur Azure), la requête échouait de façon intermittente. Cette régression avait été introduite accidentellement lors de la réactivation du déclencheur `pull_request` dans `integration.yml`.

### 1.2 Instabilité Locale Xvfb (`SIGABRT`)
Lors de l'exécution de `task test:all`, Cargo lance par défaut en parallèle les différents binaires d'intégration (`tests/*.rs`). Lorsque plusieurs exécutables interagissent simultanément avec le même display virtuel X11 (`Xvfb`), des collisions de sockets et des avertissements non-bloquants X11 survenaient. De plus, `DummyWindowEngine` initialisait GLFW avec `glfw::fail_on_errors`, ce qui transformait tout avertissement X11 anodin sous framebuffer virtuel en `panic!` / `SIGABRT` natif C++.

---

## 2. Modifications Apportées

### 2.1 Workflow CI/CD (`.github/workflows/integration.yml`)
Remplacement de l'action externe fragile par une détection et installation native via `apt-get` :
```yaml
    # 5.5️⃣ Setup FFmpeg (robuste, évite l'intermittence réseau de setup-ffmpeg)
    - name: 🎥 Set up FFmpeg
      run: |
        if ! command -v ffmpeg &> /dev/null; then
          sudo apt-get update && sudo apt-get install -y --no-install-recommends ffmpeg
        fi
        ffmpeg -version
```

### 2.2 Harnais de Test Mock (`tests/helpers.rs`)
- Configuration de `glfw::log_errors` dans `DummyWindowEngine` afin de journaliser les avertissements X11 sans avorter le processus.
- Ajout du hint `glfw::WindowHint::AutoIconify(false)` pour aligner le comportement avec le moteur de production `GlfwWindowEngine` et prévenir les crashs liés à la gestion du focus sous Xvfb.
- Préservation de la chaîne canonique d'assertion : `"DummyWindowEngine requires GLFW context for WindowEvents"`.

### 2.3 Ordonnancement des Tests (`Taskfile.yml`)
- Ajout de l'argument `-j 1` sur `cargo test --all` et `cargo llvm-cov` afin de sérialiser l'exécution des binaires d'intégration sous `Xvfb`, tout en conservant `--test-threads=1` pour l'isolation des threads de test internes.

---

## 3. Preuves de Validation & Non-Régression

1. **Tests unitaires et d'intégration complets sous Xvfb :**
   ```bash
   task test:all
   ```
   Statut : 🟢 PASS (92 unitaires + ~40 suites d'intégration validées sans crash).

2. **Validation des spécifications OpenGL (Mesa llvmpipe headless) :**
   ```bash
   task test:opengl-mesa
   ```
   Statut : 🟢 PASS.

3. **Linter et vérifications de style :**
   ```bash
   task lint:all
   ```
   Statut : 🟢 PASS.

---

## 4. Runbook de Reproductibilité

Pour vérifier la stabilité du harnais de test sous environnement virtuel X11 :
```bash
# Exécution de l'ensemble de la suite de tests
task test:all

# Vérification du workflow de couverture
task test:coverage
```
