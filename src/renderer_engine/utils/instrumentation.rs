/// Palette de couleurs "Nord Theme" pour le profiling Tracy
pub mod palette {
    pub const FRAME: u32 = 0xECEFF4;
    pub const ENV: u32 = 0x88C0D0;
    pub const SCENE: u32 = 0xD08770;
    pub const AUTO_EXPOSURE: u32 = 0xEBCB8B;
    pub const BLOOM: u32 = 0x5E81AC;
    pub const DOF: u32 = 0xA3BE8C;
    pub const MOTION_BLUR: u32 = 0xBF616A;
    pub const COMPOSITE: u32 = 0x81A1C1;
    pub const POSTPROCESS: u32 = 0xB48EAD;
    pub const UI: u32 = 0x4C566A;
    pub const GI_SYNC: u32 = 0x8FBCBB;
    pub const GI_DEBUG: u32 = 0xB48EAD;
    pub const NBODY: u32 = 0xD8DEE9;
    pub const SHOCKWAVE: u32 = 0xE5A3C9;
}

#[macro_export]
macro_rules! push_debug_group {
    ($id:expr, $name:expr) => {
        if gl::PushDebugGroup::is_loaded() {
            if let Ok(c_str) = std::ffi::CString::new($name) {
                gl::PushDebugGroup(gl::DEBUG_SOURCE_APPLICATION, $id, -1, c_str.as_ptr());
            }
        }
    };
}

#[macro_export]
macro_rules! pop_debug_group {
    () => {
        if gl::PopDebugGroup::is_loaded() {
            gl::PopDebugGroup();
        }
    };
}

#[macro_export]
macro_rules! label_gl_object {
    ($obj_type:expr, $obj_id:expr, $name:expr) => {
        if gl::ObjectLabel::is_loaded() && $obj_id != 0 {
            // .as_bytes() fonctionne par auto-deref nativement sur :
            // les &str (littéraux), les String, ET les &String !
            if let Ok(c_str) = std::ffi::CString::new(($name).as_bytes()) {
                gl::ObjectLabel($obj_type, $obj_id, -1, c_str.as_ptr());
            }
        }
    };
}

/// Macro unifiée pour le chronométrage GPU (RenderDoc + Ring-Buffer OpenGL) et CPU (Tracy).
#[macro_export]
macro_rules! gpu_profile_zone {
    ($id:expr, $name:expr, $color:expr, $profiler:expr) => {
        // 1. Span CPU Tracy (si la feature est active)
        #[cfg(feature = "tracy")]
        let _tracy_span = {
            let span = tracy_client::span!($name);
            span.emit_color($color);
            span
        };

        // 2. Garde RAII unifié (RenderDoc Push/Pop + Ring-buffer GL_TIMESTAMP)
        let _gpu_guard = $crate::renderer_engine::utils::gpu_profiler::GpuProfileGuard::new(
            $profiler.clone(),
            $id,
            $name,
            $color,
            file!(),
            line!(),
        );
    };
}
