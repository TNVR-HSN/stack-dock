fn main() {
    let mut attributes = tauri_build::Attributes::new();
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        cc::Build::new().cpp(true).file("src/settings_taskbar.cpp").compile("settings_taskbar");
        for lib in ["shell32", "propsys", "ole32", "uuid"] {
            println!("cargo:rustc-link-lib={lib}");
        }
        println!("cargo:rerun-if-changed=src/settings_taskbar.cpp");
        println!("cargo:rerun-if-changed=icons/settings-taskbar.ico");
        let file = std::env::current_dir().unwrap().join("icons/settings-taskbar.ico");
        let path = file.to_str().unwrap().replace('\\', "/");
        let rc = format!("41000 ICON \"{path}\"\nSTRINGTABLE\nBEGIN\n401 \"StackDock Native Settings\"\nEND\n");
        attributes = attributes.windows_attributes(tauri_build::WindowsAttributes::new()
            .window_icon_path("icons/settings.ico").append_rc_content(rc));
    }
    tauri_build::try_build(attributes).expect("Failed to build Settings resources");
}
