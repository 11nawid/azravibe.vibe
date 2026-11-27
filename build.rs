//! Build the optional C++ networking core (`native/azra_net.cpp`).
//!
//! If no C++ toolchain is available, the Rust fallback in `src/native.rs` is
//! used instead, so `cargo build` always succeeds (including wasm builds).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(azra_native_cpp)");
    println!("cargo:rerun-if-changed=native/azra_net.cpp");
    println!("cargo:rerun-if-changed=native/azra_net.h");
    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        return;
    }

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    match build_native(&out) {
        Ok(lib_dir) => {
            println!("cargo:rustc-cfg=azra_native_cpp");
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
            println!("cargo:rustc-link-lib=static=azra_net");
            if cfg!(windows) {
                println!("cargo:rustc-link-lib=ws2_32");
            } else {
                println!("cargo:rustc-link-lib=stdc++");
                println!("cargo:rustc-link-lib=pthread");
            }
        }
        Err(reason) => {
            println!(
                "cargo:warning=C++ networking core unavailable ({reason}); using Rust fallback"
            );
        }
    }
}

fn build_native(out: &Path) -> Result<PathBuf, String> {
    if cfg!(windows) {
        build_msvc(out)
    } else {
        build_gcc(out)
    }
}

fn list_dirs(path: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.path())
        .collect();
    dirs.sort();
    dirs.reverse();
    dirs
}

fn find_msvc() -> Option<PathBuf> {
    let roots = [
        PathBuf::from(r"C:\Program Files\Microsoft Visual Studio"),
        PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio"),
    ];
    for root in roots {
        for year in list_dirs(&root) {
            for edition in list_dirs(&year) {
                let tools = edition.join("VC").join("Tools").join("MSVC");
                if let Some(version) = list_dirs(&tools).first() {
                    return Some(version.clone());
                }
            }
        }
    }
    None
}

fn find_kit_version(root: &Path) -> Option<PathBuf> {
    list_dirs(root).first().cloned()
}

fn build_msvc(out: &Path) -> Result<PathBuf, String> {
    let msvc = find_msvc().ok_or("MSVC not found")?;
    let bin = msvc.join("bin").join("Hostx64").join("x64");
    let cl = bin.join("cl.exe");
    let lib_exe = bin.join("lib.exe");
    if !cl.exists() || !lib_exe.exists() {
        return Err("cl.exe/lib.exe not found".to_string());
    }
    let kit_include =
        find_kit_version(Path::new(r"C:\Program Files (x86)\Windows Kits\10\Include"))
            .ok_or("Windows SDK includes not found")?;
    let kit_lib = find_kit_version(Path::new(r"C:\Program Files (x86)\Windows Kits\10\Lib"))
        .ok_or("Windows SDK libraries not found")?;

    let includes = [
        msvc.join("include"),
        kit_include.join("ucrt"),
        kit_include.join("shared"),
        kit_include.join("um"),
        kit_include.join("winrt"),
    ];
    let libs = [
        msvc.join("lib").join("x64"),
        kit_lib.join("ucrt").join("x64"),
        kit_lib.join("um").join("x64"),
    ];
    let include_env = includes
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(";");
    let lib_env = libs
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(";");

    let obj = out.join("azra_net.obj");
    let compile = Command::new(&cl)
        .args(["/nologo", "/EHsc", "/O2", "/MD", "/c"])
        .arg("native/azra_net.cpp")
        .arg(format!("/Fo{}", obj.display()))
        .env("INCLUDE", &include_env)
        .status()
        .map_err(|e| e.to_string())?;
    if !compile.success() {
        return Err("cl.exe failed".to_string());
    }

    let lib_path = out.join("azra_net.lib");
    let archive = Command::new(&lib_exe)
        .arg("/nologo")
        .arg(format!("/OUT:{}", lib_path.display()))
        .arg(&obj)
        .env("LIB", &lib_env)
        .status()
        .map_err(|e| e.to_string())?;
    if !archive.success() {
        return Err("lib.exe failed".to_string());
    }
    Ok(out.to_path_buf())
}

fn build_gcc(out: &Path) -> Result<PathBuf, String> {
    let obj = out.join("azra_net.o");
    let lib_path = out.join("libazra_net.a");
    let compile = Command::new("g++")
        .args(["-std=c++17", "-O2", "-c", "native/azra_net.cpp"])
        .arg(format!("-o{}", obj.display()))
        .status()
        .map_err(|e| e.to_string())?;
    if !compile.success() {
        return Err("g++ failed".to_string());
    }
    let archive = Command::new("ar")
        .arg("rcs")
        .arg(&lib_path)
        .arg(&obj)
        .status()
        .map_err(|e| e.to_string())?;
    if !archive.success() {
        return Err("ar failed".to_string());
    }
    Ok(out.to_path_buf())
}
