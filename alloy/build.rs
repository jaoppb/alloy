//! Build script validating embedded Rhai scripts during compilation (C-11, C-12).

fn main() {
    let scripts = [
        "src/pipeline.rhai",
        "../core/css/src/application/cascade.rhai",
        "../core/graphics/src/domain/layout.rhai",
    ];

    let engine = rhai::Engine::new();

    for script_path in scripts {
        println!("cargo:rerun-if-changed={script_path}");
        let path = std::path::Path::new(script_path);
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("Failed to read script {script_path}: {e}"));
            if let Err(err) = engine.compile(&content) {
                panic!("Rhai build-time AST validation failed for {script_path}: {err}");
            }
        }
    }
}
