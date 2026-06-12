//! YaoXiang 编译脚本
//!
//! 配置 Z3 链接并复制 DLL（Windows）。

fn main() {
    let header = match std::env::var("Z3_SYS_Z3_HEADER") {
        Ok(h) => h,
        Err(_) => {
            println!("cargo:warning=Z3_SYS_Z3_HEADER not set.");
            println!("cargo:warning=Run: cd tools/setup-z3 && cargo run");
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

        // DLL 始终在 bin/ 目录
        let dll = prefix.join("bin").join("libz3.dll");
        if dll.exists() {
            let out = std::env::var("OUT_DIR").unwrap();
            let profile = std::path::Path::new(&out)
                .parent()
                .unwrap() // <pkg>
                .parent()
                .unwrap() // build
                .parent()
                .unwrap(); // target/debug 或 target/release
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
