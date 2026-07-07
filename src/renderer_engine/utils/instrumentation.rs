// src/renderer_engine/utils/instrumentation.rs

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
            if let Ok(c_str) = std::ffi::CString::new($name) {
                gl::ObjectLabel($obj_type, $obj_id, -1, c_str.as_ptr());
            }
        }
    };
}
