//! YaoXiang 编译脚本
//!
//! 自动下载并配置 Z3。首次运行下载到 .z3/，后续复用缓存。

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

const Z3_VERSION: &str = "4.16.0";

fn main() {
    // 1. 尝试系统安装的 Z3（Z3_SYS_Z3_HEADER 环境变量）
    if let Ok(header) = env::var("Z3_SYS_Z3_HEADER") {
        if Path::new(&header).exists() {
            link_z3(Path::new(&header).parent().unwrap().parent().unwrap());
            return;
        }
    }

    // 2. 查找项目本地 .z3/
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let z3_root = Path::new(&manifest).join(".z3");

    // 遍历 .z3/ 子目录，找第一个包含 include/z3.h 的
    let local = find_local_z3(&z3_root);
    if let Some(ref dir) = local {
        link_z3(dir);
        copy_dll(dir);
        return;
    }

    // 3. 自动下载
    let target = detect_target();
    let archive_name = format!("z3-{}-{}.zip", Z3_VERSION, target);
    let url = format!(
        "https://github.com/Z3Prover/z3/releases/download/z3-{}/{}",
        Z3_VERSION, archive_name
    );

    fs::create_dir_all(&z3_root).ok();
    let archive = z3_root.join(&archive_name);
    let z3_dir = z3_root.join(format!("z3-{}-{}", Z3_VERSION, target));

    if !z3_dir.join("include").join("z3.h").exists() {
        if !archive.exists() {
            println!(
                "cargo:warning=Downloading Z3 {} for {}...",
                Z3_VERSION, target
            );
            download(&url, &archive);
        }
        println!("cargo:warning=Extracting Z3...");
        extract(&archive, &z3_root);
        let _ = fs::remove_file(&archive);
    }

    link_z3(&z3_dir);
    copy_dll(&z3_dir);
}

fn link_z3(z3_dir: &Path) {
    let lib_dir = ["lib", "bin"]
        .iter()
        .map(|s| z3_dir.join(s))
        .find(|d| d.exists())
        .unwrap_or_else(|| z3_dir.join("bin"));

    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=static=z3");
        let cxx = env::var("CXXSTDLIB").unwrap_or_else(|_| "stdc++".into());
        println!("cargo:rustc-link-lib={}", cxx);
    }
}

fn copy_dll(z3_dir: &Path) {
    if !cfg!(target_os = "windows") {
        return;
    }
    let dll = z3_dir.join("bin").join("libz3.dll");
    if !dll.exists() {
        return;
    }
    let out = env::var("OUT_DIR").unwrap();
    let profile = Path::new(&out)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let deps = profile.join("deps");
    let _ = fs::create_dir_all(&deps);
    let _ = fs::copy(&dll, profile.join("libz3.dll"));
    let _ = fs::copy(&dll, deps.join("libz3.dll"));
}

fn find_local_z3(z3_root: &Path) -> Option<std::path::PathBuf> {
    if !z3_root.exists() {
        return None;
    }
    for entry in fs::read_dir(z3_root).ok()? {
        let entry = entry.ok()?;
        if entry.path().join("include").join("z3.h").exists() {
            return Some(entry.path());
        }
    }
    None
}

fn detect_target() -> &'static str {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    match (os.as_str(), arch.as_str()) {
        ("windows", "x86_64") => "x64-win",
        ("linux", "x86_64") => "x64-glibc-2.35",
        ("macos", "x86_64") => "x64-osx-13.7.4",
        ("macos", "aarch64") => "arm64-osx-13.7.4",
        _ => panic!("Unsupported platform: {}/{}", os, arch),
    }
}

fn download(
    url: &str,
    dest: &Path,
) {
    let status = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Invoke-WebRequest -Uri '{}' -OutFile '{}'",
                    url,
                    dest.display()
                ),
            ])
            .status()
    } else {
        Command::new("curl")
            .args(["-sL", url, "-o"])
            .arg(dest)
            .status()
    };
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!("Download failed, exit: {:?}", s.code()),
        Err(e) => panic!("Download failed: {}", e),
    }
}

fn extract(
    archive: &Path,
    dest: &Path,
) {
    let status = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    archive.display(),
                    dest.display()
                ),
            ])
            .status()
    } else {
        Command::new("unzip")
            .args(["-q", "-o"])
            .arg(archive)
            .arg("-d")
            .arg(dest)
            .status()
    };
    match status {
        Ok(s) if s.success() => {}
        _ => panic!("Failed to extract Z3 archive"),
    }
}
