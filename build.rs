//! Build the optional C++ networking core (`native/azra_net.cpp`).
//!
//! If no C++ toolchain is available, the Rust fallback in `src/native.rs` is
//! used instead, so `cargo build` always succeeds (including wasm builds).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    generate_stdlib_index();
    println!("cargo::rustc-check-cfg=cfg(azra_native_cpp)");
    println!("cargo:rerun-if-changed=native/azra_net.cpp");
    println!("cargo:rerun-if-changed=native/azra_net.h");
    println!("cargo:rerun-if-changed=native/azra_gui.cpp");
    println!("cargo:rerun-if-changed=native/azra_gui.h");
    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        return;
    }

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    match build_native(&out) {
        Ok(lib_dir) => {
            if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
                let _ = embed_icon(&lib_dir);
            }
            println!("cargo:rustc-cfg=azra_native_cpp");
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
            println!("cargo:rustc-link-lib=static=azra_net");
            if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
                println!("cargo:rustc-link-lib=ws2_32");
                println!("cargo:rustc-link-lib=user32");
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

/// Generate `include_str!` lookup table so the stdlib ships inside the exe.
fn generate_stdlib_index() {
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let std_dir = Path::new("std");
    let mut source = String::from(
        "/// Standard-library modules embedded at compile time.\n\
         pub fn get(name: &str) -> Option<&'static str> {\n    match name {\n",
    );
    let mut modules: Vec<(String, PathBuf)> = Vec::new();
    if let Ok(entries) = fs::read_dir(std_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("vibe"))
            {
                if let Some(stem) = path.file_stem() {
                    modules.push((stem.to_string_lossy().to_string(), path));
                }
            }
        }
    }
    modules.sort();
    for (name, path) in modules {
        let absolute = fs::canonicalize(&path).unwrap_or(path.clone());
        let literal = absolute.to_string_lossy().replace('\\', "/");
        source.push_str(&format!(
            "        {:?} => Some(include_str!({:?})),\n",
            name, literal
        ));
    }
    source.push_str("        _ => None,\n    }\n}\n");
    let _ = fs::write(out.join("stdlib_embedded.rs"), source);
    println!("cargo:rerun-if-changed=std");
}

fn find_rc() -> Option<PathBuf> {
    let sdk = r"C:\Program Files (x86)\Windows Kits\10\bin";
    if let Ok(versions) = fs::read_dir(sdk) {
        for version in versions.flatten() {
            let rc = version.path().join("x64").join("rc.exe");
            if rc.exists() {
                return Some(rc);
            }
        }
    }
    let roots = [
        PathBuf::from(r"C:\Program Files\Microsoft Visual Studio"),
        PathBuf::from(r"C:\Program Files (x86)\Microsoft Visual Studio"),
    ];
    for root in roots {
        for year in list_dirs(&root) {
            for edition in list_dirs(&year) {
                let tools = edition.join("VC").join("Tools").join("MSVC");
                for version in list_dirs(&tools) {
                    let rc = version
                        .join("bin")
                        .join("Hostx64")
                        .join("x64")
                        .join("rc.exe");
                    if rc.exists() {
                        return Some(rc);
                    }
                }
            }
        }
    }
    None
}

fn embed_icon(out: &Path) -> Result<(), String> {
    let rc = find_rc().ok_or("rc.exe not found")?;
    let icon = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("assets")
        .join("icon.ico");
    if !icon.exists() {
        return Err("icon.ico missing".to_string());
    }
    let rc_source = out.join("azra.rc");
    let script = format!(
        "1 ICON \"{}\"\n",
        icon.to_string_lossy().replace('\\', "\\\\")
    );
    fs::write(&rc_source, script).map_err(|e| e.to_string())?;
    let resource = out.join("azra.res");
    let status = Command::new(rc)
        .arg("/nologo")
        .arg(format!("/fo{}", resource.display()))
        .arg(&rc_source)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("rc.exe failed".to_string());
    }
    println!("cargo:rustc-link-arg-bins={}", resource.display());
    Ok(())
}

fn build_native(out: &Path) -> Result<PathBuf, String> {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
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
    let gui_obj = out.join("azra_gui.obj");
    for (source, object) in [
        ("native/azra_net.cpp", &obj),
        ("native/azra_gui.cpp", &gui_obj),
    ] {
        let compile = Command::new(&cl)
            .args(["/nologo", "/EHsc", "/O2", "/MD", "/c"])
            .arg(source)
            .arg(format!("/Fo{}", object.display()))
            .env("INCLUDE", &include_env)
            .status()
            .map_err(|e| e.to_string())?;
        if !compile.success() {
            return Err(format!("cl.exe failed on {source}"));
        }
    }

    let lib_path = out.join("azra_net.lib");
    let archive = Command::new(&lib_exe)
        .arg("/nologo")
        .arg(format!("/OUT:{}", lib_path.display()))
        .arg(&obj)
        .arg(&gui_obj)
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
    let gui_obj = out.join("azra_gui.o");
    let lib_path = out.join("libazra_net.a");
    for (source, object) in [
        ("native/azra_net.cpp", &obj),
        ("native/azra_gui.cpp", &gui_obj),
    ] {
        let compile = Command::new("g++")
            .args(["-std=c++17", "-O2", "-fPIC", "-c", source])
            .arg(format!("-o{}", object.display()))
            .status()
            .map_err(|e| e.to_string())?;
        if !compile.success() {
            return Err(format!("g++ failed on {source}"));
        }
    }
    let archive = Command::new("ar")
        .arg("rcs")
        .arg(&lib_path)
        .arg(&obj)
        .arg(&gui_obj)
        .status()
        .map_err(|e| e.to_string())?;
    if !archive.success() {
        return Err("ar failed".to_string());
    }
    Ok(out.to_path_buf())
}
