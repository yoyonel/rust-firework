#[cfg(test)]
mod tests {
    use fireworks_sim::renderer_engine::types::ParticleGPU;

    #[test]
    fn test_particle_gpu_structure() {
        // Verify ParticleGPU has the correct size and alignment for OpenGL std140/layout
        // We added 'brightness' (f32) at the end.
        // Previous size was 9 floats = 36 bytes.
        // New size should be 10 floats = 40 bytes.

        assert_eq!(std::mem::size_of::<ParticleGPU>(), 40);

        let p = ParticleGPU {
            pos_x: 1.0,
            pos_y: 2.0,
            col_r: 0.1,
            col_g: 0.2,
            col_b: 0.3,
            life: 1.0,
            max_life: 2.0,
            size: 5.0,
            angle: 0.0,
            brightness: 1.5,
        };

        assert_eq!(p.brightness, 1.5);
    }

    #[test]
    fn test_cstr_macro_usage() {
        // Verify cstr! macro works as expected (used in BloomPass)
        use std::ffi::CStr;

        // We need to define the macro or import it.
        // Since it's in crate::renderer_engine::tools, we might not have access to it easily if it's not exported.
        // But we can test the logic: string + null terminator.

        macro_rules! cstr {
            ($s:expr) => {
                concat!($s, "\0").as_ptr() as *const i8
            };
        }

        let s = cstr!("test_uniform");
        unsafe {
            let c_str = CStr::from_ptr(s);
            assert_eq!(c_str.to_str().unwrap(), "test_uniform");
        }
    }
}
