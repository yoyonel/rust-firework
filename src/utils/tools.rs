use log::info;

/// Affiche les informations Rust et les dépendances principales de la compilation.
pub fn show_rust_core_dependencies() {
    // Info système (Rust version, OS)
    info!(
        "Rust compiler version: {}",
        rustc_version_runtime::version()
    );
    info!("  Platform    : {}", std::env::consts::OS);
    info!("  Arch        : {}", std::env::consts::ARCH);

    let gl_version = std::env::var("GL").unwrap_or_else(|_| "Unknown".into());
    let glfw_version = std::env::var("GLFW").unwrap_or_else(|_| "Unknown".into());
    let cpal_version = std::env::var("CPAL").unwrap_or_else(|_| "Unknown".into());

    info!("Rust core dependancies");
    info!("  GL   version: {}", gl_version);
    info!("  GLFW version: {}", glfw_version);
    info!("  CPAL version: {}", cpal_version);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_show_rust_core_dependencies_no_panic() {
        // This test verifies that the function doesn't panic
        // even when environment variables are not set
        show_rust_core_dependencies();
    }

    #[test]
    fn test_show_rust_core_dependencies_with_env_vars() {
        // Set environment variables for testing
        std::env::set_var("GL", "4.6");
        std::env::set_var("GLFW", "3.3");
        std::env::set_var("CPAL", "0.15");

        // Should not panic with env vars set
        show_rust_core_dependencies();

        // Clean up
        std::env::remove_var("GL");
        std::env::remove_var("GLFW");
        std::env::remove_var("CPAL");
    }

    #[test]
    fn test_show_rust_core_dependencies_empty_env_vars() {
        // Set empty environment variables
        std::env::set_var("GL", "");
        std::env::set_var("GLFW", "");
        std::env::set_var("CPAL", "");

        // Should not panic with empty env vars
        show_rust_core_dependencies();

        // Clean up
        std::env::remove_var("GL");
        std::env::remove_var("GLFW");
        std::env::remove_var("CPAL");
    }

    #[test]
    fn test_env_var_fallback() {
        // Remove env vars to test fallback behavior
        std::env::remove_var("GL");
        std::env::remove_var("GLFW");
        std::env::remove_var("CPAL");

        // The function should use "Unknown" as fallback
        // We can't easily test the log output, but we can verify no panic
        show_rust_core_dependencies();
    }
}
