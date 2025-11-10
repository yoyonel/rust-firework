#!/usr/bin/env python3
"""
Analyse d'une série de captures d'écran (simulation de feux d'artifice)
======================================================================
Ce script traite une séquence d'images (PNG) issues d'une simulation :
- détecte la densité, les couleurs moyennes, les explosions (cercles) et fusées (lignes)
- trace des visualisations (heatmaps, courbes d'évolution, corrélation)
- génère un rapport HTML complet.

L’objectif : suivre l’évolution dynamique de l’animation (activité, intensité, synchronisation).

Auteur : (toi 😉)
"""

import sys
import os
from pathlib import Path
import numpy as np
from PIL import Image
import matplotlib.pyplot as plt
from matplotlib.colors import Normalize
import cv2
from concurrent.futures import ProcessPoolExecutor, as_completed

# --- PARAMÈTRES GLOBAUX ---
PARTICLE_DENSITY_MIN = 5000
PARTICLE_DENSITY_MAX = 100000
DELTA_MAX = 10.0
MAX_WORKERS = os.cpu_count() or 4  # parallélisation sur tous les cœurs CPU


# ============================================================
#                 TRAITEMENT D'UNE IMAGE
# ============================================================


def process_image(idx: int, screenshot_path: Path, output_dir: Path) -> dict:
    """
    Traite une image unique :
    - calcule densité, barycentre, couleurs moyennes
    - détecte explosions (cercles) et fusées (lignes)
    - génère les images dérivées : heatmap + image annotée.

    Utilise :
      - Pillow + NumPy → manipulation de pixels
      - OpenCV → détection géométrique (cercles, lignes)
      - Matplotlib → visualisation intermédiaire

    Retourne un dictionnaire de résultats exploitables.
    """
    # --- Chargement ---
    img = Image.open(screenshot_path).convert("RGB")
    img_data = np.array(img).astype(np.float64)

    # --- Densité / masque des pixels actifs ---
    mask = img_data.max(axis=2) > 0
    density = np.count_nonzero(mask)

    # Barycentre = moyenne des positions des pixels actifs
    centroid_y, centroid_x = (
        np.mean(np.argwhere(mask), axis=0) if density > 0 else (0, 0)
    )

    # Couleur moyenne sur les pixels non-nuls
    mean_colors = img_data[mask].mean(axis=0) if density > 0 else np.array([0, 0, 0])

    # --- Génération de la heatmap ---
    heatmap_dir = output_dir / "heatmaps"
    heatmap_dir.mkdir(exist_ok=True)
    heatmap_path = heatmap_dir / f"heatmap_{idx:02d}.png"

    plt.figure(figsize=(6, 6))
    plt.imshow(mask.astype(float), cmap="hot", norm=Normalize(vmin=0, vmax=1))
    plt.axis("off")
    plt.title(f"Heatmap {screenshot_path.name}")
    plt.tight_layout()
    plt.savefig(heatmap_path)
    plt.close()

    # --- Détection d'explosions (cercles) et fusées (lignes) ---
    # Conversion pour OpenCV
    cv_img = cv2.cvtColor(np.array(img), cv2.COLOR_RGB2BGR)
    gray = cv2.cvtColor(cv_img, cv2.COLOR_BGR2GRAY)
    gray_blur = cv2.medianBlur(gray, 5)

    # 🔹 Détection de cercles (méthode de Hough)
    # -> utile pour repérer les formes radiales lumineuses (explosions)
    circles = cv2.HoughCircles(
        gray_blur,
        cv2.HOUGH_GRADIENT,
        dp=1.2,
        minDist=15,
        param1=50,
        param2=30,
        minRadius=5,
        maxRadius=50,
    )
    num_circles = 0
    if circles is not None:
        circles = np.uint16(np.around(circles))
        num_circles = len(circles[0, :])
        for c in circles[0, :]:
            cv2.circle(cv_img, (c[0], c[1]), c[2], (0, 0, 255), 3)  # rouge

    # 🔹 Détection de lignes (méthode de Hough probabiliste)
    # -> utile pour repérer les trajectoires rectilignes (fusées)
    edges = cv2.Canny(gray, 50, 150)
    lines = cv2.HoughLinesP(
        edges, 1, np.pi / 180, threshold=50, minLineLength=10, maxLineGap=5
    )
    num_lines = 0
    if lines is not None:
        num_lines = len(lines)
        for line in lines:
            x1, y1, x2, y2 = line[0]
            cv2.line(cv_img, (x1, y1), (x2, y2), (255, 0, 0), 2)  # bleu

    # Sauvegarde du visuel combiné
    ef_dir = output_dir / "explosions_fusées"
    ef_dir.mkdir(exist_ok=True)
    ef_path = ef_dir / f"ef_{idx:02d}.png"
    cv2.imwrite(str(ef_path), cv_img)

    return {
        "idx": idx,
        "screenshot": screenshot_path.name,
        "density": density,
        "centroid": (centroid_x, centroid_y),
        "mean_colors": mean_colors,
        "img_data": img_data,
        "heatmap": heatmap_path.name,
        "explosions_fusées": ef_path.name,
        "num_circles": num_circles,
        "num_lines": num_lines,
    }


# ============================================================
#                 CALCULS DYNAMIQUES GLOBAUX
# ============================================================


def compute_deltas(results):
    """
    Calcule les différences moyennes entre frames successives.
    Sert à estimer l’intensité de variation visuelle (activité visuelle).
    """
    deltas = [0.0]
    for i in range(1, len(results)):
        prev = results[i - 1]["img_data"]
        curr = results[i]["img_data"]
        if (
            abs(prev.shape[0] - curr.shape[0]) > 50
            or abs(prev.shape[1] - curr.shape[1]) > 50
        ):
            print(f"⚠️ Skipping delta for frames {i - 1} and {i} (dimension mismatch)")
            deltas.append(0.0)
            continue
        deltas.append(np.mean(np.abs(curr - prev)))
    return deltas


def extract_series(results):
    """
    Extrait toutes les séries temporelles nécessaires aux graphiques
    en une seule passe pour éviter la redondance.
    """
    centroids_x, centroids_y = [], []
    colors_r, colors_g, colors_b = [], [], []
    explosions, rockets = [], []

    for r in results:
        cx, cy = r["centroid"]
        centroids_x.append(cx)
        centroids_y.append(cy)
        cr, cg, cb = r["mean_colors"]
        colors_r.append(cr)
        colors_g.append(cg)
        colors_b.append(cb)
        explosions.append(r["num_circles"])
        rockets.append(r["num_lines"])

    return centroids_x, centroids_y, colors_r, colors_g, colors_b, explosions, rockets


# ============================================================
#                 VISUALISATIONS
# ============================================================


def plot_graphs(
    output_dir,
    centroids_x,
    centroids_y,
    colors_r,
    colors_g,
    colors_b,
    explosions,
    rockets,
):
    """
    Génère toutes les figures (PNG) utilisées dans le rapport HTML :
    - trajectoire des barycentres
    - évolution des couleurs moyennes
    - activité (explosions, fusées)
    - corrélation croisée (explosions ↔ fusées)
    """
    # --- Trajectoire des barycentres ---
    trajectory_path = output_dir / "centroid_trajectory.png"
    plt.figure()
    plt.plot(centroids_x, centroids_y, marker="o", linestyle="-", color="blue")
    plt.xlabel("X centroid")
    plt.ylabel("Y centroid")
    plt.title("Centroid trajectory")
    plt.gca().invert_yaxis()
    plt.savefig(trajectory_path)
    plt.close()

    # --- Couleurs moyennes ---
    color_plot_path = output_dir / "color_evolution.png"
    plt.figure()
    plt.plot(range(1, len(colors_r) + 1), colors_r, "r-o", label="R")
    plt.plot(range(1, len(colors_g) + 1), colors_g, "g-o", label="G")
    plt.plot(range(1, len(colors_b) + 1), colors_b, "b-o", label="B")
    plt.xlabel("Frame")
    plt.ylabel("Mean color intensity")
    plt.title("Mean color evolution")
    plt.legend()
    plt.savefig(color_plot_path)
    plt.close()

    # --- Explosions ---
    explosions_plot = output_dir / "explosions_per_frame.png"
    plt.figure()
    plt.plot(range(1, len(explosions) + 1), explosions, "r-o")
    plt.xlabel("Frame")
    plt.ylabel("Explosions (circles detected)")
    plt.title("Explosions per frame")
    plt.savefig(explosions_plot)
    plt.close()

    # --- Fusées ---
    rockets_plot = output_dir / "rockets_per_frame.png"
    plt.figure()
    plt.plot(range(1, len(rockets) + 1), rockets, "b-o")
    plt.xlabel("Frame")
    plt.ylabel("Rockets (lines detected)")
    plt.title("Rockets per frame")
    plt.savefig(rockets_plot)
    plt.close()

    # --- Corrélation croisée ---
    correlation_plot = output_dir / "rockets_vs_explosions_corr.png"
    corr = np.correlate(
        rockets - np.mean(rockets), explosions - np.mean(explosions), mode="full"
    )
    lags = np.arange(-len(rockets) + 1, len(rockets))
    best_lag = lags[np.argmax(corr)]
    plt.figure()
    plt.plot(lags, corr, label="Cross-correlation")
    plt.axvline(
        best_lag, color="red", linestyle="--", label=f"Max corr lag = {best_lag} frames"
    )
    plt.xlabel("Lag (frames)")
    plt.ylabel("Correlation")
    plt.legend()
    plt.title("Rockets vs Explosions – Cross-correlation")
    plt.savefig(correlation_plot)
    plt.close()

    return (
        trajectory_path,
        color_plot_path,
        explosions_plot,
        rockets_plot,
        correlation_plot,
    )


# ============================================================
#                 RAPPORT HTML FINAL
# ============================================================


def generate_html_report(output_dir, results, paths):
    """
    Construit un rapport HTML complet avec toutes les visualisations
    et les images intermédiaires (heatmaps, détections).
    """
    (
        trajectory_path,
        color_plot_path,
        explosions_plot,
        rockets_plot,
        correlation_plot,
    ) = paths
    html_path = output_dir / "report.html"

    with open(html_path, "w") as f:
        f.write("<html><head><title>Fireworks Simulation Report</title></head><body>\n")
        f.write("<h1>Fireworks Simulation Analysis</h1>\n")
        for title, path in [
            ("Centroid trajectory", trajectory_path),
            ("Mean color evolution", color_plot_path),
            ("Explosions per frame", explosions_plot),
            ("Rockets per frame", rockets_plot),
            ("Rockets vs Explosions – Cross-correlation", correlation_plot),
        ]:
            f.write(f"<h2>{title}</h2>\n<img src='{path.name}' width='600'><br>\n")

        # --- Détails par frame ---
        f.write(
            "<h2>Frame details</h2>\n<table border='1' cellpadding='5' cellspacing='0'>\n"
        )
        f.write(
            "<tr><th>Screenshot</th><th>Density</th><th>Centroid</th><th>Mean Colors</th>"
            "<th>Delta</th><th>Alert</th><th>Heatmap</th><th>Explosions & Fusées</th></tr>\n"
        )

        for r in results:
            f.write("<tr>")
            f.write(f"<td><img src='{r['screenshot']}' width='200'></td>")
            f.write(f"<td>{r['density']}</td>")
            f.write(f"<td>({r['centroid'][0]:.2f},{r['centroid'][1]:.2f})</td>")
            f.write(f"<td>{r['mean_colors'].round(2).tolist()}</td>")
            f.write(f"<td>{r['delta']:.2f}</td>")
            f.write(f"<td>{r['alert']}</td>")
            f.write(f"<td><img src='heatmaps/{r['heatmap']}' width='200'></td>")
            f.write(
                f"<td><img src='explosions_fusées/{r['explosions_fusées']}' width='200'></td>"
            )
            f.write("</tr>\n")

        f.write("</table></body></html>")

    print(f"✅ HTML report generated at {html_path}")


# ============================================================
#                 MAIN – PIPELINE GLOBAL
# ============================================================


def main():
    """
    Point d’entrée principal :
    1. charge les images
    2. lance le traitement en parallèle (ProcessPoolExecutor)
    3. agrège, calcule, trace et exporte le rapport
    """
    if len(sys.argv) < 2:
        print("Usage: uv run main.py <screenshots_folder>")
        sys.exit(1)

    screenshots_folder = Path(sys.argv[1])
    screenshots = sorted(screenshots_folder.glob("*.png"))
    if not screenshots:
        print("No .png files found in folder.")
        sys.exit(1)

    print(
        f"⚙️ Processing {len(screenshots)} screenshots using {MAX_WORKERS} CPU cores..."
    )

    # --- Parallélisation ---
    # On distribue chaque image à un processus indépendant
    # ProcessPoolExecutor → isole la charge CPU lourde (OpenCV / numpy)
    results = []
    with ProcessPoolExecutor(max_workers=MAX_WORKERS) as executor:
        futures = {
            executor.submit(process_image, idx, path, screenshots_folder): (idx, path)
            for idx, path in enumerate(screenshots, start=1)
        }

        for future in as_completed(futures):
            idx, path = futures[future]
            try:
                result = future.result()
                results.append(result)
                print(f"✅ {path.name} processed.")
            except Exception as e:
                print(f"❌ Error processing {path.name}: {e}")

    # --- Tri, calculs complémentaires ---
    results.sort(key=lambda r: r["idx"])
    deltas = compute_deltas(results)

    for i, d in enumerate(deltas):
        results[i]["delta"] = d
        alert = ""
        if (
            results[i]["density"] < PARTICLE_DENSITY_MIN
            or results[i]["density"] > PARTICLE_DENSITY_MAX
        ):
            alert += "⚠️ Density out of bounds!"
        if d > DELTA_MAX:
            alert += " ⚠️ Delta too high!"
        results[i]["alert"] = alert

    # --- Séries + Graphiques ---
    centroids_x, centroids_y, colors_r, colors_g, colors_b, explosions, rockets = (
        extract_series(results)
    )
    paths = plot_graphs(
        screenshots_folder,
        centroids_x,
        centroids_y,
        colors_r,
        colors_g,
        colors_b,
        explosions,
        rockets,
    )

    # --- Rapport HTML ---
    generate_html_report(screenshots_folder, results, paths)


# ============================================================
if __name__ == "__main__":
    main()
