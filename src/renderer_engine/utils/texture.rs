use image::GenericImageView;
use std::path::Path;

pub struct TextureData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn load_image_data_from_disk(path: &str) -> TextureData {
    let raw_path = format!("{}.raw_tex", path);

    // Fast path: Chargement direct du binaire brut pré-calculé (Zero-Cost PNG Decoding)
    if std::path::Path::new(&raw_path).exists() {
        let bytes = std::fs::read(&raw_path).expect("Failed to read raw texture");
        if bytes.len() >= 8 {
            let width = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
            let height = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
            let data = bytes[8..].to_vec();
            return TextureData {
                data,
                width,
                height,
            };
        }
    }

    // Slow path: Fallback (CPU bound PNG decompression)
    let img = image::open(Path::new(path)).expect("Failed to load texture");
    let flipped_img = img.flipv(); // OpenGL attend l'origine en bas à gauche
    let (width, height) = flipped_img.dimensions();
    let rgba = flipped_img.to_rgba8();
    let data = rgba.into_raw();
    TextureData {
        data,
        width,
        height,
    }
}

pub fn create_gl_texture_from_data(tex_data: &TextureData) -> u32 {
    let mut tex_id = 0;
    unsafe {
        gl::GenTextures(1, &mut tex_id);
        gl::BindTexture(gl::TEXTURE_2D, tex_id);

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR_MIPMAP_LINEAR as i32,
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            tex_data.width as i32,
            tex_data.height as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            tex_data.data.as_ptr() as *const _,
        );
        gl::GenerateMipmap(gl::TEXTURE_2D);

        gl::BindTexture(gl::TEXTURE_2D, 0);
    }
    tex_id
}

pub fn load_texture(path: &str) -> (u32, u32, u32) {
    let tex_data = load_image_data_from_disk(path);
    let tex_id = create_gl_texture_from_data(&tex_data);
    (tex_id, tex_data.width, tex_data.height)
}
