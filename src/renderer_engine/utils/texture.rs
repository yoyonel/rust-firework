use image::GenericImageView;
use std::path::Path;

pub fn load_texture(path: &str) -> (u32, u32, u32) {
    // Charge l'image
    let img = image::open(Path::new(path)).expect("Failed to load texture");
    let flipped_img = img.flipv(); // OpenGL attend l'origine en bas à gauche
    let (width, height) = flipped_img.dimensions();
    let rgba = flipped_img.to_rgba8();
    let data = rgba.as_raw();

    // Crée une texture OpenGL
    let mut tex_id = 0;
    unsafe {
        gl::GenTextures(1, &mut tex_id);
        gl::BindTexture(gl::TEXTURE_2D, tex_id);

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            width as i32,
            height as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            data.as_ptr() as *const _,
        );

        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    (tex_id, width, height)
}

pub fn load_texture_array(rocket_path: &str) -> (u32, u32, u32) {
    // 1. Charge la texture de fusée
    let img = image::open(Path::new(rocket_path)).expect("Failed to load texture");
    let flipped_img = img.flipv();
    let (width, height) = flipped_img.dimensions();
    let rgba = flipped_img.to_rgba8();
    let rocket_data = rgba.as_raw();

    // 2. Génère la texture de particule (étincelle/cercle avec falloff radial) à la même taille
    let mut spark_data = Vec::with_capacity((width * height * 4) as usize);
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let radius = width as f32 / 2.0;
    for y in 0..height {
        for x in 0..width {
            let dx = (x as f32 - cx) / radius;
            let dy = (y as f32 - cy) / radius;
            let dist_sq = dx * dx + dy * dy;
            let alpha = if dist_sq >= 1.0 {
                0
            } else {
                let dist = dist_sq.sqrt();
                // Falloff radial quadratique inversé pour un effet doux/flou
                ((1.0 - dist).powi(2) * 255.0) as u8
            };
            spark_data.push(255); // R
            spark_data.push(255); // G
            spark_data.push(255); // B
            spark_data.push(alpha); // A
        }
    }

    // 3. Crée le Texture Array OpenGL
    let mut tex_id = 0;
    unsafe {
        gl::GenTextures(1, &mut tex_id);
        gl::BindTexture(gl::TEXTURE_2D_ARRAY, tex_id);

        gl::TexParameteri(
            gl::TEXTURE_2D_ARRAY,
            gl::TEXTURE_WRAP_S,
            gl::CLAMP_TO_EDGE as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_2D_ARRAY,
            gl::TEXTURE_WRAP_T,
            gl::CLAMP_TO_EDGE as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_2D_ARRAY,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_2D_ARRAY,
            gl::TEXTURE_MAG_FILTER,
            gl::LINEAR as i32,
        );

        // Alloue de la mémoire pour 2 couches de taille width x height
        gl::TexImage3D(
            gl::TEXTURE_2D_ARRAY,
            0,
            gl::RGBA as i32,
            width as i32,
            height as i32,
            2, // 2 couches
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            std::ptr::null(),
        );

        // Upload Layer 0: Fusée
        gl::TexSubImage3D(
            gl::TEXTURE_2D_ARRAY,
            0,
            0,
            0,
            0,
            width as i32,
            height as i32,
            1, // 1 couche
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            rocket_data.as_ptr() as *const _,
        );

        // Upload Layer 1: Étincelle
        gl::TexSubImage3D(
            gl::TEXTURE_2D_ARRAY,
            0,
            0,
            0,
            1, // Z-offset = 1 (couche 1)
            width as i32,
            height as i32,
            1, // 1 couche
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            spark_data.as_ptr() as *const _,
        );

        gl::BindTexture(gl::TEXTURE_2D_ARRAY, 0);
    }

    (tex_id, width, height)
}
