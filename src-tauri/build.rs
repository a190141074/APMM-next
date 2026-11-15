fn main() {
    println!("cargo:rustc-check-cfg=cfg(mobile)");
    println!("cargo:rerun-if-changed=../src-release");
    println!("cargo:rerun-if-changed=../config");
    tauri_build::build();
}
