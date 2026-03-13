fn main() {
    let mut library = std::collections::HashMap::new();
    library.insert(
        "lucide".to_string(),
        std::path::PathBuf::from(lucide_slint::lib()),
    );
    let config = slint_build::CompilerConfiguration::new().with_library_paths(library);

    slint_build::compile_with_config("ui/app.slint", config).unwrap();

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        let mut res = winres::WindowsResource::new();
        if std::path::Path::new("icon.ico").exists() {
            res.set_icon("icon.ico");
        }
        res.compile().unwrap_or_else(|e| {
            eprintln!("Failed to compile Windows resource: {}", e);
        });
    }
}
