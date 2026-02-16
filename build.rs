fn main() {
    slint_build::compile("ui/app.slint").unwrap();

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
