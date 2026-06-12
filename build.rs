//! YaoXiang 编译脚本
//!
//! 配置 Z3 链接。

fn main() {
    let header = match std::env::var("Z3_SYS_Z3_HEADER") {
        Ok(h) => h,
        Err(_) => {
            println!("cargo:warning=Z3_SYS_Z3_HEADER not set — SMT solving will not be available");
            println!("cargo:warning=Run: python tools/setup-z3.py");
            return;
        }
    };

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
    } else {
        println!("cargo:rustc-link-lib=static=z3");
        let cxx = std::env::var("CXXSTDLIB").unwrap_or_else(|_| "stdc++".into());
        println!("cargo:rustc-link-lib={}", cxx);
    }
}
