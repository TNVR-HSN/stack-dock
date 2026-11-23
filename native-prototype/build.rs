fn main() {
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        use tauri_winres::{VersionInfo, WindowsResource};
        let icon = "assets/stackdock-native.ico";
        WindowsResource::new()
            .set_icon_with_id(icon, "1")
            .set_version_info(VersionInfo::FILEVERSION, 2 << 32)
            .set_version_info(VersionInfo::PRODUCTVERSION, 2 << 32)
            .set("FileVersion", "0.2.0")
            .set("ProductVersion", "0.2.0 beta")
            .set("FileDescription", "StackDock Native")
            .set("ProductName", "StackDock Native")
            .set("CompanyName", "Stackdock by Tanveer")
            .compile().expect("Failed to compile native dock resources");
        println!("cargo:rerun-if-changed={icon}");
    }
}
