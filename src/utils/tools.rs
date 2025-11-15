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
