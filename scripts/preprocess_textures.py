#!/usr/bin/env python3
import os
import struct
import sys

try:
    from PIL import Image
except ImportError:
    print("Error: Pillow is required. Install with: pip install Pillow")
    sys.exit(1)


def process_image(filepath):
    if not filepath.endswith(".png"):
        return

    outpath = filepath + ".raw_tex"
    print(f"Processing {filepath} -> {outpath}")

    try:
        with Image.open(filepath) as img:
            # OpenGL expects the origin at the bottom-left, so we flip vertically
            img = img.transpose(Image.FLIP_TOP_BOTTOM)
            # Ensure the image is in RGBA format
            img = img.convert("RGBA")
            width, height = img.size
            pixels = img.tobytes()

            with open(outpath, "wb") as f:
                # Write 8-byte header (width, height as little-endian u32)
                f.write(struct.pack("<I", width))
                f.write(struct.pack("<I", height))
                # Write raw RGBA pixels
                f.write(pixels)
    except OSError as e:
        print(f"Failed to process {filepath}: {e}")


if __name__ == "__main__":
    assets_dir = "assets/textures"
    if not os.path.exists(assets_dir):
        print(f"Error: Directory {assets_dir} not found.")
        sys.exit(1)

    for root, dirs, files in os.walk(assets_dir):
        for filename in files:
            process_image(os.path.join(root, filename))
