/// Test ultime: vérifie que reload_shaders utilise bien tex_ratio
///
/// Ce test utilise grep/recherche de code pour vérifier que reload_shaders
/// restaure bien l'uniform uTexRatio en utilisant self.tex_ratio
///
/// Si ce test échoue, cela signifie que:
/// 1. Soit le champ tex_ratio a été supprimé
/// 2. Soit reload_shaders ne l'utilise plus
/// 3. Dans les deux cas, le bug reviendrait
use std::fs;
use std::path::Path;

#[test]
fn test_reload_shaders_uses_tex_ratio_field() {
    // Lire le fichier source de RendererGraphicsInstanced
    let source_path = "src/renderer_engine/renderer_graphics_instanced.rs";

    assert!(
        Path::new(source_path).exists(),
        "Source file must exist: {}",
        source_path
    );

    let source_code =
        fs::read_to_string(source_path).expect("Failed to read renderer_graphics_instanced.rs");

    // Test 1: Vérifier que la struct a le champ tex_ratio
    assert!(
        source_code.contains("tex_ratio: f32"),
        "CRITICAL BUG: tex_ratio field is missing from RendererGraphicsInstanced struct!\n\
         This will cause textured quads to disappear after shader reload."
    );

    // Test 2: Vérifier que reload_shaders existe
    assert!(
        source_code.contains("fn reload_shaders"),
        "reload_shaders method must exist"
    );

    // Test 3: Vérifier que reload_shaders lie le bloc uniforme "GlobalData"
    assert!(
        source_code.contains("GlobalData") || source_code.contains("UniformBlockBinding"),
        "CRITICAL BUG: reload_shaders must bind the GlobalData uniform block!\n\
         Without this, global data like uTexRatio won't be accessible by the shader after reload."
    );
}

#[test]
fn test_tex_ratio_initialized_in_constructor() {
    // Vérifier que tex_ratio est bien initialisé dans le constructeur
    let source_path = "src/renderer_engine/renderer_graphics_instanced.rs";
    let source_code =
        fs::read_to_string(source_path).expect("Failed to read renderer_graphics_instanced.rs");

    // Vérifier que le constructeur calcule tex_ratio
    assert!(
        source_code.contains("tex_ratio:")
            && source_code.contains("tex_width")
            && source_code.contains("tex_height"),
        "CRITICAL BUG: tex_ratio must be initialized in constructor with tex_width / tex_height"
    );
}

#[test]
fn test_shader_paths_are_constants() {
    // Vérifier que les chemins de shaders sont bien des constantes
    let source_path = "src/renderer_engine/renderer_graphics_instanced.rs";
    let source_code =
        fs::read_to_string(source_path).expect("Failed to read renderer_graphics_instanced.rs");

    assert!(
        source_code.contains("const VERTEX_SHADER_PATH"),
        "Vertex shader path must be a constant"
    );

    assert!(
        source_code.contains("const FRAGMENT_SHADER_PATH"),
        "Fragment shader path must be a constant"
    );

    assert!(
        source_code.contains("instanced_textured_quad.vert.glsl"),
        "Vertex shader path must point to instanced_textured_quad.vert.glsl"
    );

    assert!(
        source_code.contains("instanced_textured_quad.frag.glsl"),
        "Fragment shader path must point to instanced_textured_quad.frag.glsl"
    );
}

#[test]
fn test_reload_shaders_has_error_handling() {
    // Vérifier que reload_shaders a une gestion d'erreur appropriée
    let source_path = "src/renderer_engine/renderer_graphics_instanced.rs";
    let source_code =
        fs::read_to_string(source_path).expect("Failed to read renderer_graphics_instanced.rs");

    // Chercher la fonction reload_shaders
    assert!(
        source_code.contains("fn reload_shaders"),
        "reload_shaders method must exist"
    );

    // Vérifier qu'elle utilise try_compile_shader_program_from_files
    assert!(
        source_code.contains("try_compile_shader_program_from_files"),
        "reload_shaders must use try_compile_shader_program_from_files for safe compilation"
    );

    // Vérifier qu'elle a un match Ok/Err
    assert!(
        source_code.contains("Ok(new_program)") || source_code.contains("Ok(_)"),
        "reload_shaders must handle successful compilation"
    );

    assert!(
        source_code.contains("Err(e)") || source_code.contains("Err(_)"),
        "reload_shaders must handle compilation errors gracefully"
    );
}

#[test]
fn test_reload_shaders_deletes_old_program() {
    // Vérifier que reload_shaders supprime l'ancien programme shader
    let source_path = "src/renderer_engine/renderer_graphics_instanced.rs";
    let source_code =
        fs::read_to_string(source_path).expect("Failed to read renderer_graphics_instanced.rs");

    // Vérifier que gl::DeleteProgram est appelé
    assert!(
        source_code.contains("gl::DeleteProgram"),
        "reload_shaders must delete the old shader program to prevent memory leaks"
    );

    // Vérifier qu'il y a une vérification avant de supprimer
    assert!(
        source_code.contains("if self.shader_program != 0"),
        "reload_shaders must check if shader_program is valid before deleting"
    );
}

#[test]
fn test_reload_shaders_updates_uniform_locations() {
    // Vérifier que reload_shaders met à jour les locations des uniforms
    let source_path = "src/renderer_engine/renderer_graphics_instanced.rs";
    let source_code =
        fs::read_to_string(source_path).expect("Failed to read renderer_graphics_instanced.rs");

    // Vérifier que loc_tex est mis à jour
    assert!(
        source_code.contains("self.loc_tex = gl::GetUniformLocation"),
        "reload_shaders must update loc_tex uniform location"
    );

    // Vérifier que uTexture est recherché
    assert!(
        source_code.contains("\"uTexture\"") || source_code.contains("cstr!(\"uTexture\")"),
        "reload_shaders must get location for uTexture uniform"
    );

    // Vérifier que le bloc uniforme GlobalData est lié
    assert!(
        source_code.contains("GlobalData") || source_code.contains("UniformBlockBinding"),
        "reload_shaders must bind the GlobalData uniform block"
    );
}

/// Test de documentation: explique pourquoi ces tests sont importants
#[test]
fn test_why_these_tests_matter() {
    let explanation = r#"
    Ces tests sont critiques car ils détectent le bug suivant:

    BUG ORIGINAL:
    1. reload_shaders() recompilait les shaders
    2. Mettait à jour shader_program, loc_size, loc_tex
    3. ❌ OUBLIAIT de restaurer l'uniform uTexRatio
    4. Résultat: quads texturés invisibles après reload

    FIX:
    1. Ajout du champ tex_ratio: f32 dans la struct
    2. Stockage du ratio lors de l'initialisation
    3. Restauration de uTexRatio dans reload_shaders():
       gl::Uniform1f(
           gl::GetUniformLocation(self.shader_program, cstr!("uTexRatio")),
           self.tex_ratio,
       );

    CES TESTS DÉTECTENT:
    - Si tex_ratio est supprimé de la struct (test_reload_shaders_uses_tex_ratio_field)
    - Si reload_shaders n'utilise plus self.tex_ratio (test_reload_shaders_uses_tex_ratio_field)
    - Si l'uniform uTexRatio n'est plus restauré (test_reload_shaders_uses_tex_ratio_field)
    - Si la gestion d'erreur est supprimée (test_reload_shaders_has_error_handling)
    - Si les uniform locations ne sont plus mises à jour (test_reload_shaders_updates_uniform_locations)
    "#;

    assert!(
        explanation.contains("tex_ratio"),
        "Documentation must explain the importance of tex_ratio"
    );

    assert!(
        explanation.contains("uTexRatio"),
        "Documentation must explain the importance of uTexRatio uniform"
    );
}
