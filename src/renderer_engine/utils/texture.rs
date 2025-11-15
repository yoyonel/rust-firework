use image::GenericImageView;
use std::path::Path;

pub fn load_texture(path: &str) -> (u32, u32, u32) {
    // Charge l'image
    let img = image::open(Path::new(path)).expect("Failed to load texture");
    let img = img.flipv(); // OpenGL attend l'origine en bas à gauche
    let (width, height) = img.dimensions();
    let rgba = img.to_rgba8();
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
