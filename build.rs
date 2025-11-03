const EMBED_NAME: &str = "embedded";
const RESOURCES_DIR: &str = "resources";
const PANDOC_UPX: &str = "pandoc_upx.exe";

fn main() {
    println!("cargo:rerun-if-env-changed={EMBED_NAME}");
    println!("cargo:rerun-if-env-changed={RESOURCES_DIR}");
    println!("cargo:rerun-if-env-changed={PANDOC_UPX}");
    println!("cargo:rerun-if-changed={RESOURCES_DIR}/{PANDOC_UPX}");

    #[cfg(target_os = "windows")]
    include_bytes!("resources/pandoc_upx.exe");
}
