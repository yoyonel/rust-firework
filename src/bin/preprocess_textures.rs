use image::GenericImageView;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

fn process_image(filepath: &Path) {
    if filepath.extension().and_then(|e| e.to_str()) != Some("png") {
        return;
    }

    let outpath = PathBuf::from(format!("{}.raw_tex", filepath.display()));
    println!("Processing {} -> {}", filepath.display(), outpath.display());

    match image::open(filepath) {
        Ok(img) => {
            // OpenGL expects the origin at the bottom-left, so we flip vertically
            let flipped = img.flipv();
            let (width, height) = flipped.dimensions();
            let rgba = flipped.to_rgba8();
            let pixels = rgba.into_raw();

            match File::create(&outpath) {
                Ok(mut f) => {
                    // Write 8-byte header (width, height as little-endian u32)
                    if let Err(e) = f.write_all(&width.to_le_bytes()) {
                        eprintln!("Failed to write header to {}: {}", outpath.display(), e);
                        return;
                    }
                    if let Err(e) = f.write_all(&height.to_le_bytes()) {
                        eprintln!("Failed to write header to {}: {}", outpath.display(), e);
                        return;
                    }
                    // Write raw RGBA pixels
                    if let Err(e) = f.write_all(&pixels) {
                        eprintln!("Failed to write pixel data to {}: {}", outpath.display(), e);
                    }
                }
                Err(e) => eprintln!("Failed to create {}: {}", outpath.display(), e),
            }
        }
        Err(e) => eprintln!("Failed to process {}: {}", filepath.display(), e),
    }
}

fn walk_dir(dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_dir(&path);
            } else if path.is_file() {
                process_image(&path);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let target = Path::new(&args[1]);
        if target.is_file() {
            process_image(target);
        } else if target.is_dir() {
            walk_dir(target);
        } else {
            eprintln!("Error: Target {} not found.", args[1]);
            std::process::exit(1);
        }
    } else {
        let assets_dir = Path::new("assets/textures");
        if !assets_dir.exists() {
            eprintln!("Error: Directory assets/textures not found.");
            std::process::exit(1);
        }
        walk_dir(assets_dir);
    }
}
