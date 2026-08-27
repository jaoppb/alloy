use alloy::{VERSION_FINGERPRINT, XdgScriptManager, render_frame};
use std::path::{Path, PathBuf};

fn setup_test_env(test_name: &str) -> (PathBuf, PathBuf) {
    let base =
        std::env::temp_dir().join(format!("alloy_test_{}_{}", test_name, std::process::id()));
    let xdg_data = base.join("data");
    let xdg_config = base.join("config");

    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&xdg_data).unwrap();
    std::fs::create_dir_all(&xdg_config).unwrap();

    // SAFETY: Tests run sequentially or use unique directories per test
    unsafe {
        std::env::set_var("XDG_DATA_HOME", &xdg_data);
        std::env::set_var("XDG_CONFIG_HOME", &xdg_config);
    }

    (xdg_data, xdg_config)
}

#[test]
fn test_xdg_multi_version_isolation_and_seeding() {
    let (data_dir, _) = setup_test_env("seeding");

    let xdg = XdgScriptManager::new(None).expect("Initialize XdgScriptManager");
    xdg.seed_scripts().expect("Seed default scripts");

    let versioned_dir = data_dir
        .join("alloy")
        .join("versions")
        .join(VERSION_FINGERPRINT);
    assert!(versioned_dir.exists(), "Versioned dir should exist");

    // Verify all 3 module-paired scripts are seeded into the version directory
    assert!(versioned_dir.join("pipeline.rhai").exists());
    assert!(versioned_dir.join("cascade.rhai").exists());
    assert!(versioned_dir.join("layout.rhai").exists());

    // Verify symlink 'current' points to the active version
    #[cfg(unix)]
    {
        let symlink = data_dir.join("alloy").join("current");
        assert!(symlink.is_symlink(), "Current must be a symlink");
        let target = std::fs::read_link(&symlink).unwrap();
        assert_eq!(target, Path::new("versions").join(VERSION_FINGERPRINT));
    }
}

#[test]
fn test_xdg_script_resolution_shadowing_order() {
    let (data_dir, config_dir) = setup_test_env("shadowing");

    let custom_dir = std::env::temp_dir().join(format!("alloy_custom_cli_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&custom_dir);

    let xdg = XdgScriptManager::new(Some(custom_dir.clone())).expect("Initialize XdgScriptManager");
    let script_name = "test_module.rhai";

    // 1. Level 5: In-memory fallback
    let res = xdg.resolve_script(script_name, "// embedded fallback");
    assert_eq!(res, "// embedded fallback");

    // 2. Level 4: Versioned data directory
    let data_script = data_dir
        .join("alloy")
        .join("versions")
        .join(VERSION_FINGERPRINT)
        .join(script_name);
    std::fs::create_dir_all(data_script.parent().unwrap()).unwrap();
    std::fs::write(&data_script, "// data versioned").unwrap();
    let res = xdg.resolve_script(script_name, "// embedded fallback");
    assert_eq!(res, "// data versioned");

    // 3. Level 3: Unversioned user config directory
    let config_script = config_dir.join("alloy").join(script_name);
    std::fs::create_dir_all(config_dir.join("alloy")).unwrap();
    std::fs::write(&config_script, "// user config").unwrap();
    let res = xdg.resolve_script(script_name, "// embedded fallback");
    assert_eq!(res, "// user config");

    // 4. Level 2: Versioned user config directory
    let config_ver_script = config_dir
        .join("alloy")
        .join("versions")
        .join(VERSION_FINGERPRINT)
        .join(script_name);
    std::fs::create_dir_all(config_ver_script.parent().unwrap()).unwrap();
    std::fs::write(&config_ver_script, "// versioned config").unwrap();
    let res = xdg.resolve_script(script_name, "// embedded fallback");
    assert_eq!(res, "// versioned config");

    // 5. Level 1: Explicit CLI `--scripts-dir` overrides all
    let cli_script = custom_dir.join(script_name);
    std::fs::write(&cli_script, "// cli explicit override").unwrap();
    let res = xdg.resolve_script(script_name, "// embedded fallback");
    assert_eq!(res, "// cli explicit override");

    let _ = std::fs::remove_dir_all(&custom_dir);
}

#[test]
fn test_auto_sync_origin_to_data() {
    let (data_dir, _) = setup_test_env("sync");

    let xdg = XdgScriptManager::new(None).expect("Initialize XdgScriptManager");
    xdg.seed_scripts().expect("Seed default scripts");

    let updated_source = "// Modified in core/css/src/application/cascade.rhai\nfn test() { 42 }";
    xdg.sync_origin_to_data("cascade.rhai", updated_source)
        .expect("Auto sync origin");

    let target_file = data_dir
        .join("alloy")
        .join("versions")
        .join(VERSION_FINGERPRINT)
        .join("cascade.rhai");

    let content = std::fs::read_to_string(&target_file).expect("Read synced file");
    assert_eq!(content, updated_source);
}

#[test]
fn test_full_headless_render_pipeline_with_rhai_and_xdg() {
    let (data_dir, _) = setup_test_env("pipeline");

    let xdg = XdgScriptManager::new(None).expect("Initialize XdgScriptManager");
    xdg.seed_scripts().expect("Seed default scripts");

    let temp_html = data_dir.join("test_page.html");
    let temp_png = data_dir.join("test_render.png");

    let sample_html = r#"
        <!DOCTYPE html>
        <html>
            <head>
                <style>
                    body { background-color: #f0f0f0; margin: 0; }
                    h1 { color: #ff0000; font-size: 24px; margin: 10px; }
                </style>
            </head>
            <body>
                <h1>Hello Headless Alloy</h1>
            </body>
        </html>
    "#;

    std::fs::write(&temp_html, sample_html).unwrap();

    let render_result = render_frame(
        temp_html.to_str().unwrap(),
        temp_png.to_str().unwrap(),
        800,
        600,
        None,
        &xdg,
    );

    assert!(render_result.is_ok(), "Rendering failed: {render_result:?}");
    assert!(temp_png.exists(), "Target PNG was not generated");
    assert!(
        std::fs::metadata(&temp_png).unwrap().len() > 0,
        "Generated PNG must not be empty"
    );
}
