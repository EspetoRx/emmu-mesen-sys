use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let target = env::var("TARGET").unwrap_or_default();
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let staged = manifest_dir.join("runtime");
    let link_alias = manifest_dir.join("runtime-link");
    let core_name = if target.contains("apple") {
        "MesenCore.dylib"
    } else {
        "MesenCore.so"
    };

    // `scripts/build-sdl2.sh` copia o MesenCore e o SDL2 para `runtime/`.
    // Sem esse passo, mantemos o comportamento de desenvolvimento: linkar
    // diretamente do diretório onde o MesenCE foi compilado (MESEN_LIB_DIR).
    let default_dir = if target.contains("apple") {
        "/tmp/MesenCE/InteropDLL/obj.osx-arm64"
    } else if target.starts_with("aarch64") {
        "/tmp/MesenCE/InteropDLL/obj.linux-arm64"
    } else {
        "/tmp/MesenCE/InteropDLL/obj.linux-x64"
    };

    let link_dir = if Path::new(&staged).join(core_name).exists() {
        // `runtime-link/` contém só o symlink `libMesenCore.*` exigido por
        // `-lMesenCore`; `runtime/` vai para o instalador sem esse alias, para
        // o dyld não carregar o núcleo duas vezes.
        link_alias.clone()
    } else {
        println!(
            "cargo:warning=SDL2/MesenCore empacotados ausentes. Rode crates/emmu-mesen-sys/scripts/build-sdl2.sh para distribuir o app sem SDL2 do sistema."
        );
        PathBuf::from(env::var("MESEN_LIB_DIR").unwrap_or_else(|_| default_dir.to_owned()))
    };

    println!("cargo:rustc-link-search=native={}", link_dir.display());
    println!("cargo:rustc-link-lib=dylib=MesenCore");
    println!("cargo:rerun-if-env-changed=MESEN_LIB_DIR");
    println!(
        "cargo:rerun-if-changed={}",
        link_dir.join(core_name).display()
    );
}
