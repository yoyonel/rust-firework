#!/usr/bin/env python3
import os
import sys
import hashlib
try:
    from PIL import Image, ImageChops, ImageEnhance
except ImportError:
    import subprocess
    subprocess.check_call([sys.executable, "-m", "pip", "install", "--break-system-packages", "Pillow"])
    from PIL import Image, ImageChops, ImageEnhance

OUTPUT_DIR = "tests/visual/output"
REF_DIR = "tests/references"
ARTIFACTS_DIR = "tests/visual/artifacts"
MANIFEST_PATH = os.path.join(REF_DIR, "manifest.sha256")
TOLERANCE_THRESHOLD = 0.001  # 0.1%

def compute_sha256(filepath):
    hasher = hashlib.sha256()
    with open(filepath, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            hasher.update(chunk)
    return hasher.hexdigest()

def load_manifest():
    manifest = {}
    if not os.path.exists(MANIFEST_PATH):
        return manifest
    with open(MANIFEST_PATH, "r") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split(maxsplit=1)
            if len(parts) == 2:
                sha, filename = parts[0], os.path.basename(parts[1])
                manifest[filename] = sha
    return manifest

def create_heatmap_diff(img_out, img_ref, diff_img, diff_path):
    # Convert images to RGBA
    out_rgba = img_out.convert("RGBA")
    ref_rgba = img_ref.convert("RGBA")
    diff_rgba = diff_img.convert("RGBA")
    
    # Create dark base background
    base = ImageEnhance.Brightness(ref_rgba).enhance(0.2)
    
    # Create red mask where pixels differ
    diff_gray = diff_img.convert("L")
    threshold_mask = diff_gray.point(lambda p: 255 if p > 5 else 0)
    
    red_layer = Image.new("RGBA", img_out.size, (255, 0, 0, 255))
    heatmap = Image.composite(red_layer, base, threshold_mask)
    heatmap.save(diff_path)

def main():
    os.makedirs(ARTIFACTS_DIR, exist_ok=True)
    manifest = load_manifest()
    
    if not os.path.exists(OUTPUT_DIR):
        print(f"[FAIL] Output directory '{OUTPUT_DIR}' does not exist.")
        sys.exit(1)
        
    png_files = [f for f in os.listdir(OUTPUT_DIR) if f.endswith(".png")]
    if not png_files:
        print(f"[WARN] No PNG files found in '{OUTPUT_DIR}'.")
        sys.exit(0)
        
    failed = False
    
    for filename in sorted(png_files):
        out_path = os.path.join(OUTPUT_DIR, filename)
        ref_path = os.path.join(REF_DIR, filename)
        actual_hash = compute_sha256(out_path)
        expected_hash = manifest.get(filename)
        
        # 1. Fast Path: SHA256 Match
        if expected_hash and actual_hash == expected_hash:
            print(f"[PASS: HASH MATCH] {filename}")
            continue
            
        print(f"[INFO: HASH MISMATCH] {filename} (actual: {actual_hash[:8]}, expected: {str(expected_hash)[:8]})")
        
        # 2. Slow Path: Pixel Comparison
        if not os.path.exists(ref_path):
            print(f"[FAIL: NO GOLDEN REF] Golden reference missing for {filename} at '{ref_path}'")
            failed = True
            continue
            
        try:
            img_out = Image.open(out_path)
            img_ref = Image.open(ref_path)
        except Exception as e:
            print(f"[FAIL: IMAGE LOAD ERROR] Failed to load images for {filename}: {e}")
            failed = True
            continue
            
        if img_out.size != img_ref.size:
            print(f"[FAIL: SIZE MISMATCH] {filename} (output: {img_out.size}, ref: {img_ref.size})")
            failed = True
            continue
            
        # Pixel diffing
        diff = ImageChops.difference(img_out.convert("RGB"), img_ref.convert("RGB"))
        bbox = diff.getbbox()
        if not bbox:
            print(f"[PASS: PIXEL IDENTICAL] {filename}")
            continue
            
        # Count differing pixels
        diff_data = diff.getdata()
        differing_pixels = sum(1 for p in diff_data if sum(p) > 10)
        total_pixels = img_out.size[0] * img_out.size[1]
        diff_ratio = differing_pixels / total_pixels
        
        if diff_ratio > TOLERANCE_THRESHOLD:
            diff_filename = f"{os.path.splitext(filename)[0]}_diff.png"
            diff_path = os.path.join(ARTIFACTS_DIR, diff_filename)
            create_heatmap_diff(img_out, img_ref, diff, diff_path)
            print(f"[FAIL: VISUAL DIFF EXCEEDS THRESHOLD] {filename} (diff ratio: {diff_ratio * 100:.3f}% > {TOLERANCE_THRESHOLD * 100:.1f}%) -> Artifact saved to {diff_path}")
            failed = True
        else:
            print(f"[PASS: WITHIN TOLERANCE] {filename} (diff ratio: {diff_ratio * 100:.3f}% <= {TOLERANCE_THRESHOLD * 100:.1f}%)")
            
    if failed:
        print("❌ Universal Visual Regression Suite: FAILED")
        sys.exit(1)
    else:
        print("✅ Universal Visual Regression Suite: PASSED")
        sys.exit(0)

if __name__ == "__main__":
    main()
