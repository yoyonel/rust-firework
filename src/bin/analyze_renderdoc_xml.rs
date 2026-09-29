use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("⚠️ Usage: cargo run --bin analyze_renderdoc_xml <path_to_xml_file>");
        std::process::exit(1);
    }

    let xml_path = Path::new(&args[1]);
    let file = match File::open(xml_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("❌ Erreur lors de l'ouverture du fichier XML : {}", e);
            std::process::exit(1);
        }
    };

    let reader = BufReader::new(file);
    let chunk_re = Regex::new(r#"<chunk\b[^>]*\bname="([^"]+)""#).unwrap();
    let string_re = Regex::new(r#"<string>([^<]+)</string>"#).unwrap();
    let chunk_end_re = Regex::new(r#"</chunk>"#).unwrap();

    let mut gl_calls: HashMap<String, usize> = HashMap::new();
    let mut debug_groups = Vec::new();
    let mut object_labels = Vec::new();
    let mut errors_and_warnings = Vec::new();
    let mut draw_calls = 0;

    let mut current_chunk: Option<String> = None;
    let mut current_chunk_strings = Vec::new();

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => continue,
        };

        if let Some(caps) = chunk_re.captures(&line) {
            let name = caps[1].to_string();
            current_chunk = Some(name);
            current_chunk_strings.clear();
        }

        if let Some(caps) = string_re.captures(&line) {
            current_chunk_strings.push(caps[1].to_string());
        }

        if chunk_end_re.is_match(&line) || (line.contains("/>") && current_chunk.is_some()) {
            if let Some(name) = current_chunk.take() {
                *gl_calls.entry(name.clone()).or_insert(0) += 1;

                if name.contains("Draw") || name.contains("Dispatch") {
                    draw_calls += 1;
                } else if name == "PushDebugGroup" || name == "glPushDebugGroup" {
                    let group_name = current_chunk_strings
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "Debug Group".to_string());
                    debug_groups.push(group_name);
                } else if name == "ObjectLabel" || name == "glObjectLabel" {
                    if let Some(label) = current_chunk_strings.first() {
                        if !label.is_empty() {
                            object_labels.push(label.clone());
                        }
                    }
                } else if name.contains("DebugMessage") || name.contains("Error") {
                    let msg = current_chunk_strings
                        .first()
                        .cloned()
                        .unwrap_or_else(|| name.clone());
                    errors_and_warnings.push(format!("{}: {}", name, msg));
                }
            }
        }
    }

    let total_calls: usize = gl_calls.values().sum();
    println!("{}", "=".repeat(60));
    println!("📊 RAPPORT D'ANALYSE RENDERDOC GPU CAPTURE");
    println!("{}", "=".repeat(60));
    println!("🔹 Total appels d'API OpenGL capturés : {}", total_calls);
    println!("🎯 Total Draw Calls (Passes de rendu) : {}", draw_calls);
    println!(
        "🏷️  Objets OpenGL nommés (glObjectLabel) : {}",
        object_labels.len()
    );

    if !debug_groups.is_empty() {
        println!("\n📌 Debug Groups (Passes Rendu Détectées) :");
        for g in &debug_groups {
            println!("  - 🏷️  {}", g);
        }
    }

    println!("\n⚡ Top 10 des commandes OpenGL les plus fréquentes :");
    let mut sorted_calls: Vec<_> = gl_calls.into_iter().collect();
    sorted_calls.sort_by_key(|a| std::cmp::Reverse(a.1));
    for (cmd, count) in sorted_calls.into_iter().take(10) {
        println!("  - {:<35} : {} fois", cmd, count);
    }
    println!("{}", "=".repeat(60));

    if !errors_and_warnings.is_empty() {
        println!(
            "❌ Erreurs / Warnings API OpenGL détectés ({}) :",
            errors_and_warnings.len()
        );
        for err in &errors_and_warnings {
            println!("  - ⚠️  {}", err);
        }
        println!("{}", "=".repeat(60));
        std::process::exit(1);
    }

    println!("✅ Validation de la spec OpenGL RenderDoc : AUCUNE erreur d'API fatale.");
}
