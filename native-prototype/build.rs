fn main() {
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        tauri_winres::WindowsResource::new()
            .set_icon_with_id("../src-tauri/icons/icon.ico", "1")
            .set("FILEVERSION", "0,1,0,0")
            .set("PRODUCTVERSION", "0,1,0,0")
            .set("FileDescription", "StackDock Native Dock")
            .set("ProductName", "StackDock")
            .compile()
            .expect("Failed to compile Windows resources");
        println!("cargo:rerun-if-changed=../src-tauri/icons/icon.ico");
    }
}