use std::{env, fs, path::PathBuf};

fn main() {
    let mut windows_output_dir = None;
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let vendor_dir = manifest_dir.join("vendor").join("sdl3");
        let library_dir = vendor_dir.join("lib");
        let runtime = vendor_dir.join("bin").join("SDL3.dll");
        let runtime_dir = runtime.parent().unwrap();
        let output_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
        let profile_dir = output_dir.ancestors().nth(3).unwrap();
        let dependency_dir = profile_dir.join("deps");

        println!("cargo:rustc-link-search=native={}", library_dir.display());
        println!("cargo:rustc-link-search=native={}", runtime_dir.display());
        println!("cargo:rerun-if-changed={}", runtime.display());
        println!(
            "cargo:rerun-if-changed={}",
            library_dir.join("SDL3.lib").display()
        );
        println!(
            "cargo:rerun-if-changed={}",
            library_dir.join("libSDL3.dll.a").display()
        );

        fs::copy(&runtime, profile_dir.join("SDL3.dll")).unwrap();
        fs::create_dir_all(&dependency_dir).unwrap();
        fs::copy(&runtime, dependency_dir.join("SDL3.dll")).unwrap();
        windows_output_dir = Some(output_dir);
    }

    tauri_build::build();

    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        if let Some(output_dir) = windows_output_dir {
            let resource_archive = output_dir.join("libresource.a");
            println!("cargo:rustc-link-arg={}", resource_archive.display());
        }
    }
}
