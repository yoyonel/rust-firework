# 🎆 Fireworks Optical Flow Analyzer

---

## 📘 Description du projet

Ce projet permet d’**analyser automatiquement les mouvements visuels d’une simulation de feux d’artifice** (ou toute autre application graphique OpenGL/Rust) à partir d’une **capture vidéo** ou d’une **série de screenshots PNG**.

L’analyse utilise la technique d’**optical flow (flux optique)** pour détecter et quantifier les déplacements entre deux images successives.  
L’objectif est de mesurer la **vitesse**, la **direction** et l’**accélération** des particules ou objets animés afin d’évaluer la qualité de la simulation.

Ce projet est conçu pour être :

- ⚙️ **Automatisable** (tests d’intégration CI, GitHub Actions)  
- 🧠 **Analytique** (vitesse, direction, accélération)  
- 🚀 **Optimisé CPU** (parallélisation via `ProcessPoolExecutor`)  
- 📦 **Facile à déployer** (gestion via [`uv`](https://docs.astral.sh/uv/))

---

## 🧩 Fonctionnalités principales

- 📸 Analyse automatique de séries d’images `screenshot_*.png`  
- 🎞️ Génération d’une vidéo annotée avec les vecteurs de mouvement  
- 📊 Calcul et affichage :
  - de la **vitesse moyenne par frame**
  - de l’**accélération moyenne**
  - d’un **histogramme des directions**
- 🧰 Interface CLI basée sur **Typer** (documentation automatique, validation Pydantic)  
- ⚡ Traitement parallèle via `concurrent.futures.ProcessPoolExecutor`  
- 🧱 Intégration complète avec **Makefile** et **uv**

---

## 📦 Structure du projet

```bash
fireworks-opticalflow/
├── pyproject.toml        # Métadonnées du projet + dépendances gérées par uv
├── Makefile              # Automatisation des tâches
├── README.md             # Ce fichier
├── analyze_optical_flow.py  # Script principal Typer + Pydantic + multiprocessing
├── output/               # Screenshots d’entrée
└── output_results/       # Résultats (vidéos, graphiques, logs)
```

---

## ⚙️ Installation

### 1️⃣ Installer `uv`

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh
# ou :
pip install uv
```

### 2️⃣ Créer l’environnement virtuel et installer les dépendances

```bash
make venv
make sync
```

---

## 🚀 Utilisation

### Exemple simple

```bash
make run INPUT=./output OUTPUT=./output_results
```

Cela va :

1. Charger les images `screenshot_*.png`  
2. Calculer les vecteurs de mouvement  
3. Annoter chaque frame  
4. Générer :
   - `annotated.mp4`
   - `motion_analysis.png`
   - `annotated/*.png`

---

### Exemple complet avec paramètres

```bash
make run INPUT=./frames OUTPUT=./results FPS=30 STEP=12 WORKERS=8 SCALE=12.0
```

### Arguments disponibles (CLI Typer)

| Argument      | Type  | Description                           | Défaut        |
|---------------|-------|---------------------------------------|---------------|
| `input_dir`   | str   | Dossier contenant les screenshots     | *obligatoire* |
| `output_dir`  | str   | Dossier de sortie                     | *obligatoire* |
| `--fps`       | float | Nombre d’images par seconde utilisées | `60.0`        |
| `--step`      | int   | Espacement des vecteurs de mouvement  | `16`          |
| `--workers`   | int   | Nombre de processus utilisés          | auto (nb CPU) |
| `--scale`     | float | Échelle des couleurs pour l’intensité | `10.0`        |

---

## 📊 Résultats générés

Après exécution, le dossier de sortie contient :

```bash
output_results/
├── annotated.mp4           # Vidéo annotée avec vecteurs colorés
├── motion_analysis.png     # Graphique vitesse / accélération / directions
└── annotated/              # Images annotées individuelles
```

### Visualisations

- **Vitesse moyenne (bleu)** → montre l’intensité des mouvements  
- **Accélération (rouge)** → variation de la vitesse  
- **Histogramme des directions (vert)** → orientation dominante des déplacements  

---

## 🧰 Commandes Makefile

| Commande           | Description                                   |
|--------------------|-----------------------------------------------|
| `make venv`        | Crée un virtualenv via `uv venv`              |
| `make sync`        | Installe les dépendances avec `uv sync`       |
| `make lock`        | Génère un lockfile `uv lock`                  |
| `make run`         | Lance l’analyse sur un jeu d’images           |
| `make clean`       | Supprime `.venv` et les fichiers générés      |
| `make dev-install` | Installe les outils de développement (lint, etc.) |

---

## 🧠 Exemple de capture et analyse locale (Linux + Xvfb)

```bash
# Capture 100 frames à 60 FPS depuis ton application OpenGL
for i in $(seq -w 0 99); do
  xwd -root -silent | convert xwd:- png:"output/screenshot_$i.png"
  sleep 0.016
done

# Lancer l’analyse
make run INPUT=output OUTPUT=output_results
```

---

## 🧩 Pipeline de traitement

```bash
┌───────────────┐
│  Screenshots  │
│   (PNG files) │
└──────┬────────┘
       │
       ▼
┌─────────────────────────┐
│  Analyse Optical Flow   │
│  (OpenCV Farneback)     │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│  Calcul vitesse/accél.  │
│  + histogramme direction│
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│  Génération sorties :   │
│  - annotated.mp4        │
│  - motion_analysis.png  │
│  - annotated/*.png      │
└─────────────────────────┘
```

---

## 🧠 Points techniques clés

- **OpenCV Farneback Optical Flow**
  - Estimation dense des déplacements entre frames
  - Robustesse aux variations de luminosité

- **Multiprocessing**
  - Distribution du calcul sur plusieurs CPU via `ProcessPoolExecutor`

- **Pydantic**
  - Validation stricte des arguments CLI (types, bornes, cohérence)

- **Typer**
  - Interface CLI ergonomique et auto-documentée (`--help`)

- **uv**
  - Gestionnaire de projet rapide (Rust) : `venv`, `sync`, `lock`
  - Idéal pour CI/CD reproductibles

- **Matplotlib**
  - Visualisation des métriques et histogrammes

---

## 📈 Exemple de sortie CLI

```bash
[INFO] Chargement des 100 frames depuis ./output
[INFO] Lancement du calcul des vecteurs de mouvement (8 workers)
[INFO] Frame 1/99 — vitesse moyenne: 0.312 px/frame
[INFO] Frame 50/99 — accélération moyenne: 0.021 px/frame²
[INFO] Génération des images annotées...
[INFO] Export vidéo : output_results/annotated.mp4
[INFO] Export graphique : output_results/motion_analysis.png
✅ Animation détectée — test réussi
```

---

## 🧑‍💻 Développement

### Ajouter une dépendance

```bash
uv add <nom_du_package>
```

### Supprimer une dépendance

```bash
uv remove <nom_du_package>
```

### Lancer le script directement

```bash
uv run analyze -- ./frames ./results --fps 60
```

---

## 📜 Licence

**MIT License**  
© 2025 — Conçu pour le projet *Rust Fireworks Simulator*  
Auteur : **ATTY Lionel**
