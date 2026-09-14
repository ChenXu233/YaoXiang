//! YaoXiang 编译脚本
//!
//! 自动下载并配置 Z3。首次运行下载到 .z3/，后续复用缓存。

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

const Z3_VERSION: &str = "4.16.0";

fn main() {
    // 重建触发面显式化：build.rs 只在这些路径（及自身/Cargo.toml）变化时重跑
    println!("cargo:rerun-if-changed=locales/zh.json");
    println!("cargo:rerun-if-changed=docs/src/design/rfc/accepted/013-error-code-specification.md");
    println!("cargo:rerun-if-changed=src/util/diagnostic/codes");
    println!("cargo:rerun-if-changed=src/std/result.rs");

    // #325/#326：构建期权威校验（解析/校验/比对逻辑单一实现于 tools/code-tables lib）——
    // 注册码唯一性、段位、zh 完备性、locales 孤立、RFC-013 码表区间一致性；
    // 任一 error 即拒绝编译。bot 语言缺失降级为 cargo:warning。
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&manifest_dir);
    let entries = code_tables::parse_registry(root).unwrap_or_else(|msg| panic!("{msg}"));
    let report = code_tables::validate(root, &entries).unwrap_or_else(|msg| panic!("{msg}"));
    for warning in &report.warnings {
        println!("cargo:warning={warning}");
    }
    for error in &report.errors {
        println!("cargo:warning={error}");
    }
    if !report.is_ok() {
        panic!(
            "错误码注册表校验失败（{} 项 error，见上方清单）。zh 是唯一人工翻译源：新增码请同步补 zh 条目。",
            report.errors.len()
        );
    }

    // RFC-013 码表区间一致性（漂移即拒绝编译；治愈：code-tables --fix）
    let rfc_path = root.join(code_tables::RFC013_REL);
    let doc = std::fs::read_to_string(&rfc_path)
        .unwrap_or_else(|e| panic!("读取 {} 失败: {}", rfc_path.display(), e));
    let zh = code_tables::load_locale(root, "zh").unwrap_or_else(|msg| panic!("{msg}"));
    let mismatches = code_tables::check_rfc_tables(&doc, &entries, &zh);
    if !mismatches.is_empty() {
        panic!(
            "RFC-013 码表区间与注册表不一致：
  {}
治愈：cargo run --manifest-path tools/code-tables/Cargo.toml -- --fix",
            mismatches.join(
                "
  "
            )
        );
    }

    // Skip Z3 linking for wasm targets
    let _target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_arch == "wasm32" {
        // wasm 不自动下载（没有预编译 wasm 二进制），但查找本地预编译的 libz3.a
        let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
        let z3_root = Path::new(&manifest).join(".z3");
        let local = find_local_z3_wasm(&z3_root);
        if let Some(ref dir) = local {
            println!("cargo:warning=Linking Z3 wasm from {:?}", dir);
            link_z3_wasm(dir);
            return;
        }
        println!("cargo:warning=No precompiled Z3 wasm found in .z3/, Z3 features disabled");
        return;
    }

    // 1. 尝试系统安装的 Z3（Z3_SYS_Z3_HEADER 环境变量）
    if let Ok(header) = env::var("Z3_SYS_Z3_HEADER") {
        if Path::new(&header).exists() {
            let dir = Path::new(&header).parent().unwrap().parent().unwrap();
            link_z3(dir);
            copy_shared_lib(dir);
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
        copy_shared_lib(dir);
        return;
    }

    // 3. 自动下载
    let target = match detect_target() {
        Some(t) => t,
        None => {
            let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
            let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
            println!(
                "cargo:warning=Z3 prebuilt binaries not available for {}/{}. \
                 Set Z3_SYS_Z3_HEADER or place Z3 in .z3/ to provide Z3.",
                os, arch
            );
            return;
        }
    };
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
        // 验证下载的文件是否合法（至少 1MB）
        let meta = fs::metadata(&archive).expect("Failed to read Z3 archive metadata");
        if meta.len() < 1024 * 1024 {
            let _ = fs::remove_file(&archive);
            panic!(
                "Z3 archive too small ({} bytes), download likely failed. \
                 Set Z3_SYS_Z3_HEADER or place Z3 in .z3/ to skip download.",
                meta.len()
            );
        }
        println!("cargo:warning=Extracting Z3...");
        extract(&archive, &z3_root);
        let _ = fs::remove_file(&archive);
    }

    link_z3(&z3_dir);
    copy_shared_lib(&z3_dir);
}

fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let lib_dir = ["lib", "bin"]
        .iter()
        .map(|s| z3_dir.join(s))
        .find(|d| d.exists())
        .unwrap_or_else(|| z3_dir.join("bin"));

    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // RFC-037：全平台动态链接。共享库随发行包 bin/ 分发，用户可整体替换升级 Z3。
    if target_os == "windows" {
        // MSVC import lib 命名为 libz3.lib
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=z3");
        // 动态链接器默认不搜二进制所在目录，必须注入 rpath，“解压即用”才成立
        // （发行包内 exe 与 libz3 同在 bin/；Windows 默认搜 exe 目录，无需处理）
        match target_os.as_str() {
            "linux" => println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN"),
            "macos" => println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path"),
            _ => {}
        }
        let cxx = if target_os == "macos" {
            "c++".to_string()
        } else {
            env::var("CXXSTDLIB").unwrap_or_else(|_| "stdc++".into())
        };
        println!("cargo:rustc-link-lib={}", cxx);
    }
}

/// 把 Z3 共享库复制进 target profile 目录，本地 cargo run/测试才能加载；
/// Z3 许可证（MIT 分发义务）随库一并落盘。发版产物由 package-dist.sh
/// 从同一目录取用，不重复维护平台映射。
fn copy_shared_lib(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let name = match target_os.as_str() {
        "windows" => "libz3.dll",
        "macos" => "libz3.dylib",
        "linux" => "libz3.so",
        _ => return,
    };
    // 布局不统一：官方发行包在 lib/ 或 bin/；系统 Z3（Debian multiarch）在
    // lib/<arch>-linux-gnu/
    let mut search_dirs = vec![z3_dir.join("lib"), z3_dir.join("bin")];
    if target_os == "linux" {
        let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        search_dirs.push(z3_dir.join("lib").join(format!("{arch}-linux-gnu")));
    }
    let src = search_dirs
        .iter()
        .map(|d| d.join(name))
        .find(|p| p.exists());
    let src = match src {
        Some(p) => p,
        None => return,
    };
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
    let _ = fs::copy(&src, profile.join(name));
    let _ = fs::copy(&src, deps.join(name));
    // MIT 要求分发二进制时附带许可文本；Z3 发行包根有 LICENSE.txt
    for license in ["LICENSE.txt", "LICENSE"] {
        let license_src = z3_dir.join(license);
        if license_src.exists() {
            let _ = fs::copy(&license_src, profile.join("LICENSE-Z3.txt"));
            break;
        }
    }
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

fn detect_target() -> Option<&'static str> {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    match (os.as_str(), arch.as_str()) {
        ("windows", "x86_64") => Some("x64-win"),
        ("windows", "aarch64") => Some("arm64-win"),
        ("linux", "x86_64") => Some("x64-glibc-2.39"),
        ("linux", "aarch64") => Some("arm64-glibc-2.38"),
        ("macos", "x86_64") => Some("x64-osx-15.7.3"),
        ("macos", "aarch64") => Some("arm64-osx-15.7.3"),
        _ => None,
    }
}

fn download(
    url: &str,
    dest: &Path,
) {
    // 使用宿主平台的工具，不是目标平台
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
    // 使用宿主平台的工具，不是目标平台
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

/// 查找 .z3/ 目录下的 wasm 预编译 Z3
fn find_local_z3_wasm(z3_root: &Path) -> Option<std::path::PathBuf> {
    if !z3_root.exists() {
        return None;
    }
    for entry in fs::read_dir(z3_root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        // 检查 lib/libz3.a 是否存在
        if path.join("lib").join("libz3.a").exists() {
            return Some(path);
        }
    }
    None
}

/// 链接 wasm 预编译的 Z3（Emscripten 产出的 .a 文件）
fn link_z3_wasm(z3_dir: &Path) {
    let lib_dir = z3_dir.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=z3");
}
