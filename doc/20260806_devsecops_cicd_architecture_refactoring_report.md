# 📐 RAPPORT ARCHITECTURAL : REFONTE DEVSECOPS & CI/CD DAG (`rust-firework`)

* **Date** : 2026-08-06
* **Branche** : `feat/devsecops-cicd-refactoring`
* **Auteur** : Agent (Architecte DevSecOps Expert)
* **Périmètre** : Refonte CI/CD, Cold Build Golden Image, Supply Chain Security, DAG Parallèle, Local ISO Debugging.

---

## 1. RATIONNEL & ARCHITECTURE

L'ancienne CI (`ci.yml`) s'exécutait en monobloc linéaire sous runner `ubuntu-latest` brut, dépensant **147s sur 285s (51.5% du temps total)** uniquement pour l'installation répétitive des paquets APT, Rust toolchain, `sccache`, `cargo-llvm-cov`, `renderdoccmd` et des dépendances Tracy.

La nouvelle architecture adopte la stratégie "Golden Image + Containerized DAG" :

1. **Golden Image** (`ghcr.io/yoyonel/rust-firework-ci:latest`) :
   * Conteneur multi-stage basé sur Ubuntu 24.04.
   * Contient nativement l'intégralité des dépendances système, Rust toolchain 1.90.0, CLI pre-compilés (`sccache`, `task`, `cargo-llvm-cov`, `cargo-deny`, `renderdoccmd`, `tracy-capture`, `tracy-csvexport`).

2. **Graphe DAG Parallèle** (`.github/workflows/ci.yml`) :
   * **Job Fast-Fail (`lint-and-security`)** : Exécute `task lint:all` et `cargo deny check` en premier.
   * **Jobs Parallèles (`needs: lint-and-security`)** :
     - `unit-tests-coverage` (`task ci:coverage`)
     - `mesa-visual-regression` (`task test:opengl-mesa` & `task test:visual-full`)
     - `renderdoc-validation` (`task renderdoc:capture`)
     - `tracy-ratio-benchmark` (`xvfb-run -a task bench:tracy-ratios`)
   * **Job Aggregator (`ci-summary-report`)** : Génère le tableau de bord Markdown nativement dans `$GITHUB_STEP_SUMMARY`.

3. **Reproductibilité ISO Locale (`Taskfile.yml`)** :
   * Tâches `task devops:run` et `task devops:shell` permettant d'exécuter localement n'importe quelle cible dans la Golden Image avec auto-détection `podman`/`docker`, injection `--userns=keep-id` (Podman Bazzite) ou `--user $(id -u):$(id -g)` (Docker Debian), et forwarding du socket X11 `/tmp/.X11-unix`.

---

## 2. DEVSECOPS & SUPPLY CHAIN

* **`deny.toml`** : Configuration `cargo-deny` 0.20+ avec blocage strict des vulnérabilités CVE et non-conformités de licences.
* **CVE Patching** : Mise à jour immédiate des crates vulnérables (`rand`, `anyhow`, `bytes`, `crossbeam-epoch`).
* **`renovate.json`** : Batching hebdomadaire des PRs d'auto-update.

---

## 3. RUNBOOK DE REPRODUCTIBILITÉ HUMAINE

### 1. Validation de la Sécurité Crate & Licences en local
```bash
cargo deny check advisories licenses bans
```

### 2. Exécution d'une tâche sous la Golden Image locale (Podman/Docker)
```bash
# Build de la Golden Image en local
task devops:build-image

# Exécution ISO d'un test OpenGL Mesa
task devops:run -- task test:opengl-mesa

# Shell interactif pour débogage local
task devops:shell
```
