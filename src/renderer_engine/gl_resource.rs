//! # OpenGL RAII Resource Wrappers
//!
//! **INVARIANT THREAD-GL STRICT :**
//! Ces types encapsulent des handles OpenGL natifs (`u32`).
//! Ils ne sont **PAS thread-safe** (non `Send`, non `Sync`) et **DOIVENT** être créés,
//! manipulés et détruits (`Drop`) **exclusivement sur le thread où le contexte OpenGL est courant**.
//!
//! **Politique d'accès :**
//! L'accès au handle brut se fait via la méthode explicite `.raw()` (aucun trait `Deref`),
//! afin de garantir l'étanchéité des types (impossibilité d'échanger par inadvertance un `GlVao`
//! et un `GlBuffer`) et d'assurer une traçabilité rigoureuse aux frontières FFI OpenGL.

macro_rules! define_gl_single_wrapper {
    ($name:ident, $gl_delete_fn:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Default, PartialEq, Eq)]
        pub struct $name(u32);

        impl $name {
            /// Crée un wrapper à partir d'un handle OpenGL existant.
            #[inline]
            pub const fn from_raw(handle: u32) -> Self {
                Self(handle)
            }

            /// Crée un wrapper vide avec un handle nul (0).
            #[inline]
            pub const fn empty() -> Self {
                Self(0)
            }

            /// Retourne le handle brut sous-jacent.
            #[inline]
            pub const fn raw(&self) -> u32 {
                self.0
            }

            /// Vérifie si le handle est non nul.
            #[inline]
            pub const fn is_valid(&self) -> bool {
                self.0 != 0
            }

            /// Extrait le handle brut en transférant la propriété (ne déclenche pas Drop).
            #[inline]
            pub fn into_raw(mut self) -> u32 {
                let handle = self.0;
                self.0 = 0;
                handle
            }

            /// Remplace le handle courant par un nouveau et supprime l'ancien s'il était valide.
            pub fn reset(&mut self, new_handle: u32) {
                if self.0 != 0 && self.0 != new_handle {
                    unsafe {
                        if gl::$gl_delete_fn::is_loaded() {
                            gl::$gl_delete_fn(self.0);
                        }
                    }
                }
                self.0 = new_handle;
            }

            /// Prend la valeur du handle et réinitialise le wrapper à 0 (sans appeler glDelete).
            #[inline]
            pub fn take(&mut self) -> u32 {
                let handle = self.0;
                self.0 = 0;
                handle
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                if self.0 != 0 {
                    unsafe {
                        if gl::$gl_delete_fn::is_loaded() {
                            gl::$gl_delete_fn(self.0);
                        }
                    }
                    self.0 = 0;
                }
            }
        }
    };
}

macro_rules! define_gl_array_wrapper {
    ($name:ident, $gl_delete_fn:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Default, PartialEq, Eq)]
        pub struct $name(u32);

        impl $name {
            /// Crée un wrapper à partir d'un handle OpenGL existant.
            #[inline]
            pub const fn from_raw(handle: u32) -> Self {
                Self(handle)
            }

            /// Crée un wrapper vide avec un handle nul (0).
            #[inline]
            pub const fn empty() -> Self {
                Self(0)
            }

            /// Retourne le handle brut sous-jacent.
            #[inline]
            pub const fn raw(&self) -> u32 {
                self.0
            }

            /// Vérifie si le handle est non nul.
            #[inline]
            pub const fn is_valid(&self) -> bool {
                self.0 != 0
            }

            /// Extrait le handle brut en transférant la propriété (ne déclenche pas Drop).
            #[inline]
            pub fn into_raw(mut self) -> u32 {
                let handle = self.0;
                self.0 = 0;
                handle
            }

            /// Remplace le handle courant par un nouveau et supprime l'ancien s'il était valide.
            pub fn reset(&mut self, new_handle: u32) {
                if self.0 != 0 && self.0 != new_handle {
                    unsafe {
                        if gl::$gl_delete_fn::is_loaded() {
                            gl::$gl_delete_fn(1, &self.0);
                        }
                    }
                }
                self.0 = new_handle;
            }

            /// Prend la valeur du handle et réinitialise le wrapper à 0 (sans appeler glDelete).
            #[inline]
            pub fn take(&mut self) -> u32 {
                let handle = self.0;
                self.0 = 0;
                handle
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                if self.0 != 0 {
                    unsafe {
                        if gl::$gl_delete_fn::is_loaded() {
                            gl::$gl_delete_fn(1, &self.0);
                        }
                    }
                    self.0 = 0;
                }
            }
        }
    };
}

define_gl_single_wrapper!(
    GlProgram,
    DeleteProgram,
    "Wrapper RAII pour un Shader Program OpenGL (`glDeleteProgram`)."
);

define_gl_single_wrapper!(
    GlShader,
    DeleteShader,
    "Wrapper RAII pour un Shader OpenGL individuel (`glDeleteShader`)."
);

define_gl_array_wrapper!(
    GlBuffer,
    DeleteBuffers,
    "Wrapper RAII pour un Buffer OpenGL (VBO, UBO, EBO, etc.) (`glDeleteBuffers`)."
);

define_gl_array_wrapper!(
    GlVao,
    DeleteVertexArrays,
    "Wrapper RAII pour un Vertex Array Object (VAO) OpenGL (`glDeleteVertexArrays`)."
);

define_gl_array_wrapper!(
    GlTexture,
    DeleteTextures,
    "Wrapper RAII pour une Texture OpenGL (`glDeleteTextures`)."
);

define_gl_array_wrapper!(
    GlFbo,
    DeleteFramebuffers,
    "Wrapper RAII pour un Framebuffer Object (FBO) OpenGL (`glDeleteFramebuffers`)."
);

define_gl_array_wrapper!(
    GlRenderbuffer,
    DeleteRenderbuffers,
    "Wrapper RAII pour un Renderbuffer Object (RBO) OpenGL (`glDeleteRenderbuffers`)."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gl_wrapper_lifecycle_and_take() {
        let mut buf = GlBuffer::from_raw(42);
        assert_eq!(buf.raw(), 42);
        assert!(buf.is_valid());

        let taken = buf.take();
        assert_eq!(taken, 42);
        assert_eq!(buf.raw(), 0);
        assert!(!buf.is_valid());
    }

    #[test]
    fn test_gl_wrapper_into_raw() {
        let vao = GlVao::from_raw(123);
        let raw = vao.into_raw();
        assert_eq!(raw, 123);
    }

    #[test]
    fn test_gl_wrapper_empty_and_reset() {
        let mut tex = GlTexture::empty();
        assert_eq!(tex.raw(), 0);
        assert!(!tex.is_valid());

        tex.reset(77);
        assert_eq!(tex.raw(), 77);
        assert!(tex.is_valid());
    }

    #[test]
    fn test_gl_program_wrapper() {
        let mut prog = GlProgram::from_raw(99);
        assert_eq!(prog.raw(), 99);
        assert!(prog.is_valid());
        let raw = prog.take();
        assert_eq!(raw, 99);
        assert_eq!(prog.raw(), 0);
    }
}
