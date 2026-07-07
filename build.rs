// build.rs
use std::collections::HashSet;
use std::env;
use std::fs;

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

    // Ensemble des crates qui nous intéressent
    let tracked = HashSet::from(["glfw", "cpal", "gl"]);

    // SOLUTION DE REMPLACEMENT :
    // On parse manuellement le fichier Cargo.lock comme un simple texte.
    // Cela permet d'éviter l'appel toxique à `cargo_metadata` qui détruisait
    // silencieusement le cache du répertoire `target/` en arrière-plan.
    if let Ok(lock_contents) = fs::read_to_string("Cargo.lock") {
        let mut current_package = String::new();

        for line in lock_contents.lines() {
            let line = line.trim();
            if line.starts_with("name =") {
                // Extrait le nom du package en nettoyant les guillemets
                current_package = line
                    .replace("name =", "")
                    .replace('"', "")
                    .trim()
                    .to_string();
            } else if line.starts_with("version =") && tracked.contains(current_package.as_str()) {
                // Extrait la version si le package fait partie de notre liste
                let version = line
                    .replace("version =", "")
                    .replace('"', "")
                    .trim()
                    .to_string();

                println!(
                    "cargo:rustc-env={}={}",
                    current_package.to_uppercase(),
                    version
                );

                // On réinitialise pour ne pas matcher plusieurs fois la même crate
                current_package.clear();
            }
        }
    }

    // Indique à Cargo de ne relancer ce script QUE si ces fichiers changent
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.lock");
}
