fn main() {
    let manifest_dir = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set"),
    );
    let assets_dir = manifest_dir.join("assets");
    let icon_path = assets_dir.join("icon.ico");
    let assets_dir_str = assets_dir.to_string_lossy().into_owned();
    let icon_path_str = icon_path.to_string_lossy().into_owned();

    println!("cargo:rerun-if-changed={assets_dir_str}");

    if std::env::var_os("CARGO_CFG_TARGET_OS").as_deref() == Some(std::ffi::OsStr::new("windows")) {
        let mut res = winresource::WindowsResource::new();
        res.set_icon(&icon_path_str);
        res.set("ProductName", "Circle");
        res.set("FileDescription", "Circle");
        res.set("Comments", "Circle");
        res.set("CompanyName", "Jason Xue <hi@jasonxue.dev>");
        if let Err(err) = res.compile() {
            panic!("failed to compile Windows resources: {err}");
        }
    }
}
