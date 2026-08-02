#!/usr/bin/env python3
import os
import sys
try:
    from PIL import Image, ImageChops, ImageEnhance
except ImportError:
    import subprocess
    subprocess.check_call([sys.executable, "-m", "pip", "install", "--break-system-packages", "Pillow"])
    from PIL import Image, ImageChops, ImageEnhance

CANDIDATES_DIR = "tests/visual_baselines/candidates"
BASELINES_DIR = "tests/visual_baselines"
ARTIFACTS_DIR = "tests/visual_baselines/artifacts"
DEFAULT_TOLERANCE = 0.001

def compute_image_mse(img_a, img_b):
    """
    Replicates exact compute_image_mse math from tests/visual_regression_test.rs:
    diff = (px_a[c] - px_b[c]) / 255.0
    sum_squared_diff += diff * diff
    mse = sum_squared_diff / (total_pixels * 3.0)
    """
    if img_a.size != img_b.size:
        raise ValueError(f"Dimension mismatch: {img_a.size} vs {img_b.size}")
        
    width, height = img_a.size
    total_pixels = width * height
    
    img_a_rgb = img_a.convert("RGB")
    img_b_rgb = img_b.convert("RGB")
    
    data_a = list(img_a_rgb.getdata())
    data_b = list(img_b_rgb.getdata())
    
    sum_squared_diff = 0.0
    for px_a, px_b in zip(data_a, data_b):
        for c in range(3):
            diff = (px_a[c] - px_b[c]) / 255.0
            sum_squared_diff += diff * diff
            
    mse = sum_squared_diff / (total_pixels * 3.0)
    return mse

def create_heatmap_diff(img_cand, img_base, diff_img, diff_path):
    cand_rgba = img_cand.convert("RGBA")
    base_rgba = img_base.convert("RGBA")
    
    base_dark = ImageEnhance.Brightness(base_rgba).enhance(0.2)
    diff_gray = diff_img.convert("L")
    threshold_mask = diff_gray.point(lambda p: 255 if p > 5 else 0)
    
    red_layer = Image.new("RGBA", img_cand.size, (255, 0, 0, 255))
    heatmap = Image.composite(red_layer, base_dark, threshold_mask)
    heatmap.save(diff_path)

def main():
    os.makedirs(ARTIFACTS_DIR, exist_ok=True)
    
    if not os.path.exists(CANDIDATES_DIR):
        print(f"[WARN] Candidates directory '{CANDIDATES_DIR}' does not exist.")
        sys.exit(0)
        
    candidate_files = [f for f in os.listdir(CANDIDATES_DIR) if f.endswith(".png")]
    if not candidate_files:
        print(f"[WARN] No candidate PNG files found in '{CANDIDATES_DIR}'.")
        sys.exit(0)
        
    failed = False
    print(f"🔍 Evaluating {len(candidate_files)} candidate frame(s) against golden baselines (tolerance MSE <= {DEFAULT_TOLERANCE})...")
    
    for filename in sorted(candidate_files):
        cand_path = os.path.join(CANDIDATES_DIR, filename)
        base_path = os.path.join(BASELINES_DIR, filename)
        
        if not os.path.exists(base_path):
            print(f"[FAIL: NO GOLDEN BASELINE] {filename} missing baseline in '{BASELINES_DIR}'")
            failed = True
            continue
            
        try:
            img_cand = Image.open(cand_path)
            img_base = Image.open(base_path)
        except Exception as e:
            print(f"[FAIL: IMAGE LOAD ERROR] {filename}: {e}")
            failed = True
            continue
            
        if img_cand.size != img_base.size:
            print(f"[FAIL: DIMENSION MISMATCH] {filename} (candidate: {img_cand.size}, baseline: {img_base.size})")
            failed = True
            continue
            
        mse = compute_image_mse(img_cand, img_base)
        
        if mse > DEFAULT_TOLERANCE:
            diff = ImageChops.difference(img_cand.convert("RGB"), img_base.convert("RGB"))
            diff_filename = f"{os.path.splitext(filename)[0]}_diff.png"
            diff_path = os.path.join(ARTIFACTS_DIR, diff_filename)
            create_heatmap_diff(img_cand, img_base, diff, diff_path)
            print(f"[FAIL: MSE EXCEEDS THRESHOLD] {filename} (MSE: {mse:.6f} > {DEFAULT_TOLERANCE}) -> Heatmap saved to {diff_path}")
            failed = True
        else:
            print(f"[PASS: MSE WITHIN TOLERANCE] {filename} (MSE: {mse:.6f} <= {DEFAULT_TOLERANCE})")
            
    if failed:
        print("❌ Universal MSE Baseline Validation: FAILED")
        sys.exit(1)
    else:
        print("✅ Universal MSE Baseline Validation: PASSED")
        sys.exit(0)

if __name__ == "__main__":
    main()
