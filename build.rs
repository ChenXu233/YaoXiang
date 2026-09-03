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
    println!("cargo:rerun-if-changed=src/util/diagnostic/codes");
    println!("cargo:rerun-if-changed=src/std/result.rs");

    // #325：构建期翻译完备性门槛——注册码在 zh.json 缺失即拒绝编译。
    // 只拦 zh（人工源）；其余语言由 i18n bot 异步补齐，渲染走 zh 回退链。
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    if let Err(msg) = check_zh_locale_completeness(Path::new(&manifest_dir)) {
        panic!("{}", msg);
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
    copy_dll(&z3_dir);
}

fn link_z3(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let lib_dir = ["lib", "bin"]
        .iter()
        .map(|s| z3_dir.join(s))
        .find(|d| d.exists())
        .unwrap_or_else(|| z3_dir.join("bin"));

    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    if target_os == "windows" {
        println!("cargo:rustc-link-lib=libz3");
    } else {
        println!("cargo:rustc-link-lib=static=z3");
        let cxx = if target_os == "macos" {
            "c++".to_string()
        } else {
            env::var("CXXSTDLIB").unwrap_or_else(|_| "stdc++".into())
        };
        println!("cargo:rustc-link-lib={}", cxx);
    }
}

fn copy_dll(z3_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target_os != "windows" {
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

/// 扫描诊断码注册源，校验 zh.json 条目完备性（#325 构建期门槛）。
///
/// 注册源两处：
/// - `src/util/diagnostic/codes/{e,w}?xxx.rs` 的 `code: "E6001"` 行
/// - `src/std/result.rs` RUNTIME_ERROR_CODES 的 `("E6009", ...)` 元组行
///
/// zh.json 条目要求：键存在、title 非空、template 字段存在。
fn check_zh_locale_completeness(root: &Path) -> Result<(), String> {
    let mut codes: Vec<String> = Vec::new();

    let codes_dir = root.join("src/util/diagnostic/codes");
    let mut entries: Vec<_> = fs::read_dir(&codes_dir)
        .map_err(|e| format!("读取 {} 失败: {}", codes_dir.display(), e))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "rs")
                && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                    (n.starts_with('e') || n.starts_with('w')) && n.ends_with("xxx.rs")
                })
        })
        .collect();
    entries.sort();

    for path in entries {
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("读取 {} 失败: {}", path.display(), e))?;
        for line in content.lines() {
            if let Some(rest) = line.trim_start().strip_prefix("code:") {
                if let Some(code) = extract_quoted(rest) {
                    codes.push(code);
                }
            }
        }
    }

    let result_rs = root.join("src/std/result.rs");
    if result_rs.exists() {
        let content = fs::read_to_string(&result_rs)
            .map_err(|e| format!("读取 {} 失败: {}", result_rs.display(), e))?;
        for line in content.lines() {
            let t = line.trim_start();
            if let Some(rest) = t.strip_prefix("(\"") {
                // rest 形如 `E6009", "语义"`——首个引号即码的闭合引号
                if let Some(code) = rest.split('"').next() {
                    if !code.is_empty() {
                        codes.push(code.to_string());
                    }
                }
            }
        }
    }

    let zh_path = root.join("locales/zh.json");
    let zh_text = fs::read_to_string(&zh_path)
        .map_err(|e| format!("读取 {} 失败: {}", zh_path.display(), e))?;
    let zh: serde_json::Value = serde_json::from_str(&zh_text)
        .map_err(|e| format!("解析 {} 失败: {}", zh_path.display(), e))?;

    let mut missing = Vec::new();
    for code in &codes {
        match zh.get(code.as_str()) {
            None => missing.push(format!("{}（缺条目）", code)),
            Some(entry) => {
                let title_ok = entry
                    .get("title")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| !s.is_empty());
                let has_template = entry.get("template").is_some();
                if !title_ok || !has_template {
                    missing.push(format!(
                        "{}（{}）",
                        code,
                        if !title_ok {
                            "title 缺失或为空"
                        } else {
                            "缺 template 字段"
                        }
                    ));
                }
            }
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "错误码翻译完备性校验失败：以下 {} 个码在 locales/zh.json 缺失或不完整。\n\
             zh 是唯一人工翻译源（新增码请同步补 zh 条目），其余语言由 i18n bot 异步补齐：\n  {}",
            missing.len(),
            missing.join("\n  ")
        ))
    }
}

/// 从形如 `"E6001",` 的文本段提取第一个引号包裹的 token
fn extract_quoted(s: &str) -> Option<String> {
    let start = s.find('"')? + 1;
    let end = s[start..].find('"')? + start;
    Some(s[start..end].to_string())
}
