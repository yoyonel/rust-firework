#!/usr/bin/env python3
"""
analyze_optical_flow.py

Analyse et annotation d'une série de screenshots (PNG) utilisant Optical Flow (Farneback).
- CLI via Typer
- Validation d'arguments via Pydantic
- Parallélisation CPU avec ProcessPoolExecutor
- Logs détaillés des étapes intermédiaires

Usage (exemples) :
  python analyze_optical_flow.py /path/to/input_dir /path/to/output_dir --fps 60 --step 16
"""

from __future__ import annotations
import sys
import time
import math
import logging
from pathlib import Path
from typing import List, Tuple, Optional
from dataclasses import dataclass

import numpy as np
import cv2

import typer
from pydantic import BaseModel, Field, field_validator
from concurrent.futures import ProcessPoolExecutor, as_completed
import multiprocessing

app = typer.Typer(add_completion=False)

# ----------------------------
# Logging configuration
# ----------------------------
logger = logging.getLogger("optflow")
logger.setLevel(logging.INFO)
ch = logging.StreamHandler(sys.stdout)
ch.setFormatter(logging.Formatter("%(asctime)s [%(levelname)s] %(message)s"))
logger.addHandler(ch)


# ----------------------------
# Pydantic config model
# ----------------------------
class Config(BaseModel):
    input_dir: Path = Field(
        ..., description="Répertoire contenant screenshots nommés screenshot_*.png"
    )
    output_dir: Path = Field(
        ..., description="Répertoire de sortie pour images annotées, MP4 et graphiques"
    )
    fps: float = Field(
        60.0,
        gt=0.0,
        description="Framerate utilisé pour les calculs et la vidéo (fps).",
    )
    step: int = Field(
        16, gt=0, description="Espacement entre les vecteurs tracés (en pixels)."
    )
    max_workers: Optional[int] = Field(
        None, description="Nombre de workers (par défaut = cpu_count())."
    )
    annotate_scale: float = Field(
        10.0,
        gt=0.0,
        description="Échelle pour couleur / intensité (scaling magnitude).",
    )

    @field_validator("input_dir")
    @classmethod
    def input_must_exist(cls, v: Path):
        if not v.exists() or not v.is_dir():
            raise ValueError(f"input_dir {v} n'existe pas ou n'est pas un dossier.")
        return v

    @field_validator("output_dir")
    @classmethod
    def create_output_dir(cls, v: Path):
        v.mkdir(parents=True, exist_ok=True)
        return v


# ----------------------------
# Data-classes / types
# ----------------------------
@dataclass
class FramePairResult:
    index: int  # index de la paire (correspond à frame i -> i+1)
    speed: float  # vitesse moyenne (px/frame)
    angle_deg: float  # direction moyenne (degrés)
    accel: Optional[float] = None  # rempli plus tard, si on calcule l'accélération
    annotated_path: Optional[str] = None  # chemin de l'image annotée produite


# ----------------------------
# Helper functions
# ----------------------------
def list_input_frames(input_dir: Path) -> List[Path]:
    """Retourne la liste triée des fichiers matching screenshot_*.png"""
    paths = sorted(input_dir.glob("screenshot_*.png"))
    if not paths:
        raise FileNotFoundError(
            f"Aucune image trouvée dans {input_dir} (pattern screenshot_*.png)."
        )
    logger.info("Found %d input frames in %s", len(paths), input_dir)
    return paths


def compute_flow_and_annotate(
    args: Tuple[int, str, str, int, float, str],
) -> Tuple[int, float, float, str]:
    """
    Fonction worker exécutée dans un process séparé.
    Calcule l'optical flow pour une paire (prev_path, curr_path),
    calcule la vitesse moyenne et angle moyen, et sauvegarde l'image annotée.

    Retourne : (index, speed, angle_deg, annotated_path)
    """
    index, prev_path, curr_path, step, annotate_scale, out_dir = args
    # On configure logging minimal ici (les processus enfants héritent souvent du handler principal,
    # mais il est plus simple d'éviter beaucoup de logs enfants).
    # Charger images
    prev = cv2.imread(prev_path)
    curr = cv2.imread(curr_path)
    if prev is None or curr is None:
        raise RuntimeError(f"Failed to read frames: {prev_path}, {curr_path}")

    prev_gray = cv2.cvtColor(prev, cv2.COLOR_BGR2GRAY)
    curr_gray = cv2.cvtColor(curr, cv2.COLOR_BGR2GRAY)

    # Calcule optical flow Farneback
    flow = cv2.calcOpticalFlowFarneback(
        prev_gray,
        curr_gray,
        None,
        pyr_scale=0.5,
        levels=3,
        winsize=15,
        iterations=3,
        poly_n=5,
        poly_sigma=1.2,
        flags=0,
    )

    # Moyennes (globales)
    mean_dx = float(np.mean(flow[..., 0]))
    mean_dy = float(np.mean(flow[..., 1]))
    speed = math.hypot(mean_dx, mean_dy)  # px/frame
    angle_deg = math.degrees(math.atan2(mean_dy, mean_dx))

    # Annotate (draw arrows on a copy of curr)
    annotated = curr.copy()
    h, w = flow.shape[:2]
    for y in range(0, h, step):
        for x in range(0, w, step):
            dx, dy = flow[y, x]
            mag = math.hypot(dx, dy)

            # color map: convert mag to 0..255 using annotate_scale
            cval = min(int(mag * annotate_scale), 255)
            color = (0, cval, 255 - cval)  # BGR

            pt1 = (x, y)
            pt2 = (int(round(x + dx)), int(round(y + dy)))
            # draw arrow if significant
            if cval > 0:
                cv2.arrowedLine(
                    annotated, pt1, pt2, color=color, thickness=1, tipLength=0.3
                )

    out_dir_p = Path(out_dir)
    out_dir_p.mkdir(parents=True, exist_ok=True)
    annotated_path = str(out_dir_p / f"annotated_{index:03d}.png")
    cv2.imwrite(annotated_path, annotated)

    # Retour minimal
    return (index, speed, angle_deg, annotated_path)


# ----------------------------
# Main processing pipeline
# ----------------------------
def process_all_frames(cfg: Config) -> List[FramePairResult]:
    """
    Orchestration :
    - liste des images
    - création des tâches pour chaque paire (i -> i+1)
    - exécution en parallèle (ProcessPoolExecutor)
    - collecte des résultats ordonnés
    """
    start_time = time.time()
    paths = list_input_frames(cfg.input_dir)
    n = len(paths)
    if n < 2:
        raise RuntimeError("Besoin d'au moins 2 frames pour calculer l'optical flow.")

    # prepare args for workers: pairs (0,1), (1,2), ...
    tasks_args = []
    annotated_dir = str(cfg.output_dir / "annotated")
    for i in range(n - 1):
        tasks_args.append(
            (
                i,
                str(paths[i]),
                str(paths[i + 1]),
                cfg.step,
                cfg.annotate_scale,
                annotated_dir,
            )
        )

    max_workers = cfg.max_workers or multiprocessing.cpu_count()
    logger.info("Starting ProcessPoolExecutor with %d workers", max_workers)

    results = []
    with ProcessPoolExecutor(max_workers=max_workers) as exe:
        future_to_idx = {
            exe.submit(compute_flow_and_annotate, arg): arg[0] for arg in tasks_args
        }
        completed = 0
        for fut in as_completed(future_to_idx):
            idx = future_to_idx[fut]
            try:
                index, speed, angle_deg, annotated_path = fut.result()
                results.append(
                    FramePairResult(
                        index=index,
                        speed=speed,
                        angle_deg=angle_deg,
                        annotated_path=annotated_path,
                    )
                )
                completed += 1
                if completed % 10 == 0 or completed == len(tasks_args):
                    logger.info(
                        "Completed %d/%d frame pairs", completed, len(tasks_args)
                    )
            except Exception as exc:
                logger.exception("Worker failed for index %s: %s", idx, exc)
                raise

    # order results by index
    results.sort(key=lambda r: r.index)

    elapsed = time.time() - start_time
    logger.info(
        "Finished computing flows and annotations for %d pairs in %.2fs",
        len(results),
        elapsed,
    )
    return results


def compute_accelerations(results: List[FramePairResult], fps: float) -> None:
    """Remplit le champ accel dans les FramePairResult (px/frame² or px/s² depending on fps).
    We compute accel in px/frame^2 then convert to px/s^2 by multiplying by fps."""
    speeds = np.array([r.speed for r in results])
    if speeds.size < 2:
        for r in results:
            r.accel = 0.0
        return
    # discrete derivative of speed vs frame (units px/frame^2)
    accel_frames = np.gradient(speeds)
    # convert to px/s^2 by multiplying by fps
    accel_s = accel_frames * fps
    for r, a in zip(results, accel_s):
        r.accel = float(a)


# ----------------------------
# Output helpers
# ----------------------------
def create_video_from_images(
    image_paths: List[str], output_path: Path, fps: float
) -> None:
    """Crée une vidéo MP4 simple (cv2 VideoWriter)."""
    if not image_paths:
        logger.warning("No images to create video.")
        return
    first = cv2.imread(image_paths[0])
    h, w = first.shape[:2]
    logger.info(
        "Creating video %s size=%dx%d fps=%.1f frames=%d",
        output_path,
        w,
        h,
        fps,
        len(image_paths),
    )
    fourcc = cv2.VideoWriter_fourcc(*"mp4v")
    writer = cv2.VideoWriter(str(output_path), fourcc, fps, (w, h))
    for p in image_paths:
        frame = cv2.imread(p)
        writer.write(frame)
    writer.release()
    logger.info("Video written to %s", output_path)


def plot_and_save_stats(
    results: List[FramePairResult], output_path: Path, fps: float
) -> None:
    """Crée un PNG avec 3 sous-graphes : vitesse, accélération, histogramme des directions."""
    speeds = np.array([r.speed for r in results])
    angles = np.array([r.angle_deg for r in results])
    accels = np.array([r.accel if r.accel is not None else 0.0 for r in results])
    t = np.arange(len(speeds)) / fps

    import matplotlib.pyplot as plt

    plt.figure(figsize=(12, 8))

    plt.subplot(3, 1, 1)
    plt.plot(t, speeds, color="blue")
    plt.ylabel("Vitesse (px/frame)")
    plt.title("Vitesse moyenne par frame")

    plt.subplot(3, 1, 2)
    plt.plot(t, accels, color="red")
    plt.ylabel("Accélération (px/s²)")
    plt.title("Accélération par frame")

    plt.subplot(3, 1, 3)
    plt.hist(angles, bins=36, range=(-180, 180), color="green", alpha=0.7)
    plt.xlabel("Angle (°)")
    plt.ylabel("Occurrences")
    plt.title("Histogramme des directions (°)")

    plt.tight_layout()
    plt.savefig(str(output_path), dpi=150)
    logger.info("Saved stats plot to %s", output_path)


# ----------------------------
# Typer CLI
# ----------------------------
@app.command()
def cli(
    input_dir: Path = typer.Argument(
        ..., help="Répertoire contenant screenshots screenshot_*.png"
    ),
    output_dir: Path = typer.Argument(..., help="Répertoire de sortie"),
    fps: float = typer.Option(60.0, "--fps", "-f", help="Frame rate (fps)"),
    step: int = typer.Option(16, "--step", "-s", help="Espacement vecteurs (px)"),
    max_workers: Optional[int] = typer.Option(
        None, "--workers", "-w", help="Nombre de workers (default = cpu_count())"
    ),
    annotate_scale: float = typer.Option(
        10.0, "--scale", help="Échelle couleur pour magnitude"
    ),
    verbose: bool = typer.Option(False, "--verbose", "-v", help="Activer logs DEBUG"),
):
    """Script d'analyse Optical Flow (parallélisé) — Typer + Pydantic + ProcessPoolExecutor"""
    if verbose:
        logger.setLevel(logging.DEBUG)
        logger.debug("Verbose logging enabled")

    logger.info("Starting optical-flow analysis")
    cfg = Config(
        input_dir=input_dir,
        output_dir=output_dir,
        fps=fps,
        step=step,
        max_workers=max_workers,
        annotate_scale=annotate_scale,
    )

    # pipeline
    start = time.time()
    results = process_all_frames(cfg)
    compute_accelerations(results, cfg.fps)

    # collect annotated image paths (in same order)
    annotated_images = [r.annotated_path for r in results]

    # create video
    output_video = cfg.output_dir / "annotated.mp4"
    create_video_from_images(annotated_images, output_video, cfg.fps)

    # create stats plot
    output_plot = cfg.output_dir / "motion_analysis.png"
    plot_and_save_stats(results, output_plot, cfg.fps)

    # final summary & exit code
    mean_speed = float(np.mean([r.speed for r in results]) if results else 0.0)
    logger.info("Mean speed (px/frame): %.4f", mean_speed)
    elapsed = time.time() - start
    logger.info("All done in %.2fs", elapsed)

    # Exit with non-zero if too small movement
    if mean_speed < 0.1:
        logger.error("Mean speed too small (%.4f px/frame) -> failing", mean_speed)
        raise typer.Exit(code=1)

    logger.info("Success")
    raise typer.Exit(code=0)


if __name__ == "__main__":
    app()
