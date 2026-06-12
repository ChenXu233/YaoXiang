//! YaoXiang 编译脚本
//!
//! 配置 Z3 链接。
//! 自动检测 .z3/ 目录（由 tools/setup-z3 下载），也支持 Z3_SYS_Z3_HEADER 环境变量。

fn main() {
    let header = find_z3_header().unwrap_or_else(|| {
        println!("cargo:warning=Z3 not found. Run: cd tools/setup-z3 && cargo run");
        println!("cargo:warning=Or set Z3_SYS_Z3_HEADER env var to point to z3.h");
        return;
    });

    let include_dir = std::path::Path::new(&header).parent().unwrap();
    let prefix = include_dir.parent().unwrap();

    let lib_dir = ["lib", "bin"]
        .iter()
        .map(|s| prefix.join(s))
        .find(|d| d.exists())
        .unwrap_or_else(|| prefix.join("bin"));

    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=libz3");

        let dll = prefix.join("bin").join("libz3.dll");
        if dll.exists() {
            let out = std::env::var("OUT_DIR").unwrap();
            let profile = std::path::Path::new(&out)
                .parent().unwrap()
                .parent().unwrap()
                .parent().unwrap();
            let deps = profile.join("deps");
            let _ = std::fs::create_dir_all(&deps);
            let _ = std::fs::copy(&dll, profile.join("libz3.dll"));
            let _ = std::fs::copy(&dll, deps.join("libz3.dll"));
        }
    } else {
        println!("cargo:rustc-link-lib=static=z3");
        let cxx = std::env::var("CXXSTDLIB").unwrap_or_else(|_| "stdc++".into());
        println!("cargo:rustc-link-lib={}", cxx);
    }
}

/// 查找 Z3 header：先查环境变量，再自动搜索 .z3/
fn find_z3_header() -> Option<String> {
    // 1. 环境变量
    if let Ok(h) = std::env::var("Z3_SYS_Z3_HEADER") {
        if std::path::Path::new(&h).exists() {
            return Some(h);
        }
    }

    // 2. 自动检测项目根 .z3/
    let manifest = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let z3_root = std::path::Path::new(&manifest).join(".z3");
    if !z3_root.exists() {
        return None;
    }

    // 遍历 .z3/ 下的子目录，找第一个包含 include/z3.h 的
    for entry in std::fs::read_dir(&z3_root).ok()? {
        let entry = entry.ok()?;
        let header = entry.path().join("include").join("z3.h");
        if header.exists() {
            return Some(header.to_str()?.replace('\\', "/"));
        }
    }

    None
}
