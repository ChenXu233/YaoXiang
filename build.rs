//! YaoXiang 编译脚本
//!
//! 配置 Z3 链接。

fn main() {
    // Z3 链接。从 Z3_SYS_Z3_HEADER 推断 lib 目录，
    // 或从 Z3_LIB_DIR 直接指定。
    if let Ok(lib_dir) = std::env::var("Z3_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", lib_dir);
    } else if let Ok(header) = std::env::var("Z3_SYS_Z3_HEADER") {
        let include_dir = std::path::Path::new(&header)
            .parent()
            .expect("Z3_SYS_Z3_HEADER should be a file path (e.g. .../include/z3.h)");
        let prefix = include_dir
            .parent()
            .expect("Z3 header should be in .../include/z3.h");

        for sub in &["lib", "bin"] {
            let candidate = prefix.join(sub);
            if candidate.join("libz3.lib").exists() || candidate.join("libz3.a").exists() {
                println!("cargo:rustc-link-search=native={}", candidate.display());
                break;
            }
        }
    }

    if cfg!(target_os = "windows") {
        // Windows: Z3 预编译包 libz3.lib 是 DLL 导入库
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=static=z3");
        let cxx = std::env::var("CXXSTDLIB").unwrap_or_else(|_| "stdc++".into());
        println!("cargo:rustc-link-lib={}", cxx);
    }
}
