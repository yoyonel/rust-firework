#!/usr/bin/env python3
import os
import sys
import numpy as np
from PIL import Image
from skimage.metrics import structural_similarity as ssim
import concurrent.futures

CANDIDATES_DIR = "tests/visual_baselines/candidates"
BASELINES_DIR = "tests/visual_baselines"
ARTIFACTS_DIR = "tests/visual_baselines/artifacts"

PIXEL_DIFF_THRESHOLD_COUNT = 50
PIXEL_COLOR_DIFF_PERCENT = 0.05  # 5% of 255 = 12.75
SSIM_THRESHOLD = 0.999
CROSS_VAL_SSIM_MAX = 0.98


def run_self_calibration():
    path_kawase = os.path.join(BASELINES_DIR, "bloom_kawase_4x.png")
    path_gaussian = os.path.join(BASELINES_DIR, "bloom_gaussian_2x.png")

    if not os.path.exists(path_kawase) or not os.path.exists(path_gaussian):
        print("⚠️ Self-calibration skipped: baseline test images missing.")
        return

    img_k = np.array(Image.open(path_kawase).convert("RGB"))
    img_g = np.array(Image.open(path_gaussian).convert("RGB"))

    score = ssim(img_k, img_g, channel_axis=2)
    print(f"🧪 Self-Calibration Test: Kawase 4x vs Gaussian 2x SSIM = {score:.6f}")
    if score > CROSS_VAL_SSIM_MAX:
        print(
            f"❌ Calibration Failure: Cannot distinguish Kawase from Gaussian (SSIM {score:.6f} > {CROSS_VAL_SSIM_MAX})!"
        )
        sys.exit(1)
    print("✅ Self-Calibration PASSED (Distinct visual signatures confirmed)")


def create_heatmap_diff(base_np, diff_mask, diff_path):
    base_dark = (base_np.astype(float) * 0.2).astype(np.uint8)
    heatmap_np = base_dark.copy()
    heatmap_np[diff_mask] = [255, 0, 0]
    Image.fromarray(heatmap_np).save(diff_path)


def evaluate_single_image(filename):
    cand_path = os.path.join(CANDIDATES_DIR, filename)
    base_path = os.path.join(BASELINES_DIR, filename)

    if not os.path.exists(base_path):
        return (
            filename,
            False,
            f"[FAIL: NO GOLDEN BASELINE] {filename} missing baseline in '{BASELINES_DIR}'",
        )

    try:
        cand_img = Image.open(cand_path).convert("RGB")
        base_img = Image.open(base_path).convert("RGB")
    except Exception as e:
        return (filename, False, f"[FAIL: IMAGE LOAD ERROR] {filename}: {e}")

    if cand_img.size != base_img.size:
        return (
            filename,
            False,
            f"[FAIL: DIMENSION MISMATCH] {filename} (candidate: {cand_img.size}, baseline: {base_img.size})",
        )

    cand_np = np.array(cand_img)
    base_np = np.array(base_img)

    # Pass A: Strict Pixel Threshold
    channel_diffs = np.abs(cand_np.astype(float) - base_np.astype(float))
    max_pixel_diff = np.max(channel_diffs, axis=2)
    diff_mask = max_pixel_diff > (255.0 * PIXEL_COLOR_DIFF_PERCENT)
    differing_pixel_count = np.sum(diff_mask)

    # Pass B: SSIM
    score_ssim = ssim(cand_np, base_np, channel_axis=2)

    if differing_pixel_count > PIXEL_DIFF_THRESHOLD_COUNT:
        diff_filename = f"{os.path.splitext(filename)[0]}_diff.png"
        diff_path = os.path.join(ARTIFACTS_DIR, diff_filename)
        create_heatmap_diff(base_np, diff_mask, diff_path)
        return (
            filename,
            False,
            f"[FAIL: STRICT PIXEL THRESHOLD] {filename} ({differing_pixel_count} pixels differ by > 5% color distance > limit {PIXEL_DIFF_THRESHOLD_COUNT}) -> Heatmap saved to {diff_path}",
        )
    elif score_ssim < SSIM_THRESHOLD:
        diff_filename = f"{os.path.splitext(filename)[0]}_diff.png"
        diff_path = os.path.join(ARTIFACTS_DIR, diff_filename)
        create_heatmap_diff(base_np, diff_mask, diff_path)
        return (
            filename,
            False,
            f"[FAIL: SSIM THRESHOLD] {filename} (SSIM: {score_ssim:.6f} < threshold {SSIM_THRESHOLD}) -> Heatmap saved to {diff_path}",
        )
    else:
        return (
            filename,
            True,
            f"[PASS: DUAL-PASS VALIDATED] {filename} (SSIM: {score_ssim:.6f}, diff pixels: {differing_pixel_count})",
        )


def main():
    os.makedirs(ARTIFACTS_DIR, exist_ok=True)

    # 1. Level 3 FX Finesse: Self-Calibration Test (Sequential)
    run_self_calibration()

    if not os.path.exists(CANDIDATES_DIR):
        print(
            f"[FAIL: CRITICAL] Candidates directory '{CANDIDATES_DIR}' does not exist."
        )
        sys.exit(1)

    candidate_files = sorted(
        [f for f in os.listdir(CANDIDATES_DIR) if f.endswith(".png")]
    )
    if not candidate_files:
        print(f"[FAIL: CRITICAL] No candidate PNG files found in '{CANDIDATES_DIR}'.")
        sys.exit(1)

    print(
        f"\n🔍 Parallel Dual-Pass Evaluation: {len(candidate_files)} candidate(s) (Strict Pixel Threshold > {PIXEL_DIFF_THRESHOLD_COUNT} px @ 5%, SSIM >= {SSIM_THRESHOLD})..."
    )

    failed = False
    results = {}

    with concurrent.futures.ProcessPoolExecutor() as executor:
        future_to_file = {
            executor.submit(evaluate_single_image, f): f for f in candidate_files
        }
        for future in concurrent.futures.as_completed(future_to_file):
            filename = future_to_file[future]
            try:
                fname, passed, msg = future.result()
                results[fname] = (passed, msg)
            except Exception as e:
                results[filename] = (False, f"[FAIL: WORKER EXCEPTION] {filename}: {e}")

    for filename in candidate_files:
        passed, msg = results[filename]
        print(msg)
        if not passed:
            failed = True

    if failed:
        print("\n❌ Universal Hybrid Visual Regression Pipeline: FAILED")
        sys.exit(1)
    else:
        print("\n✅ Universal Hybrid Visual Regression Pipeline: PASSED")
        sys.exit(0)


if __name__ == "__main__":
    main()
