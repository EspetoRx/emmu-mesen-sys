fn main() {
    let lib_dir = "/tmp/MesenCE/InteropDLL/obj.osx-arm64";
    println!("cargo:rustc-link-search=native={lib_dir}");
    println!("cargo:rustc-link-lib=dylib=MesenCore");
    println!("cargo:rerun-if-changed={lib_dir}/libMesenCore.dylib");
}
