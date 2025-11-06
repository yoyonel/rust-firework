// build.rs
use cargo_metadata::MetadataCommand;
use std::collections::HashSet;
use std::env;

fn main() {
    // Détection des features
    let simd = env::var("CARGO_FEATURE_SIMD").is_ok();
    let no_simd = env::var("CARGO_FEATURE_NO_SIMD").is_ok();
    let fft = env::var("CARGO_FEATURE_FFT").is_ok();

    if simd && no_simd {
        panic!("Features `simd` et `no_simd` sont mutuellement exclusives !");
    }

    if simd {
        println!("cargo:warning=🟢 Compilation avec SIMD activé (feature = \"simd\")");
    } else if no_simd {
        println!("cargo:warning=⚪ Compilation en mode scalaire (feature = \"no_simd\")");
    } else {
        println!(
            "cargo:warning=⚪ Compilation par défaut : scalaire (aucune feature SIMD activée)"
        );
    }

    if fft {
        println!("cargo:warning=🟢 Compilation avec FFT activé (feature = \"fft\")");
    }

    // Récupère la metadata du projet
    let metadata = MetadataCommand::new()
        .exec()
        .expect("cargo metadata failed");

    // Ensemble des crates qui nous intéressent
    let tracked = HashSet::from(["glfw", "cpal", "gl"]);

    for package in &metadata.packages {
        if tracked.contains(package.name.as_str()) {
            println!(
                "cargo:rustc-env={}={}",
                package.name.to_uppercase(),
                package.version
            );
        }
    }
}
