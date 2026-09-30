//! Cargo 构建策略（RFC-014b Phase 5c）
//!
//! 读 `[build.cargo]` 拼命令：`cargo build [--release] [--features a,b]`，
//! 当前平台覆盖（`[build.platforms].cargo-features`）追加进 features。
//!
//! 产物布局（两层分离）：
//! - cargo scratch（target/）经 `CARGO_TARGET_DIR` 指到项目
//!   `.yaoxiang/build/cargo/<pkg>/`——不落 vendor 包目录，否则目录完整性
//!   校验和被增量构建产物撑爆、每次安装全量重算
//! - FFI 可消费的库文件（.so/.dll/.dylib/.a）复制到 vendor 包目录
//!   `build/native/<triple>/`——随目录校验和走，安装即稳定

use std::path::Path;
use std::process::Command;

use crate::package::error::{PackageError, PackageResult};
use crate::package::manifest::BuildConfig;

use super::{current_triple, BuildOutcome};

/// FFI 可消费的 cargo 产物扩展名（cdylib / staticlib；.rlib 是 Rust 内部格式不复制）
const LIB_EXTENSIONS: &[&str] = &[".so", ".dll", ".dylib", ".a"];

/// 执行 cargo 构建；`scratch_root` 为项目 `.yaoxiang/build/`
pub(crate) fn build(
    pkg_dir: &Path,
    config: &BuildConfig,
    scratch_root: &Path,
) -> PackageResult<BuildOutcome> {
    let cargo_config = config.cargo.clone().unwrap_or_default();
    let profile = cargo_config.target.as_deref().unwrap_or("release");
    if !matches!(profile, "release" | "debug") {
        return Err(PackageError::InvalidManifest(format!(
            "[build.cargo] target 只支持 \"release\" / \"debug\"，得到 '{profile}'"
        )));
    }

    let features = compose_features(config);
    let target_dir = scratch_root.join("cargo").join(
        pkg_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string()),
    );
    std::fs::create_dir_all(&target_dir)?;

    let mut cmd = Command::new("cargo");
    cmd.current_dir(pkg_dir)
        .arg("build")
        .env("CARGO_TARGET_DIR", &target_dir);
    if profile == "release" {
        cmd.arg("--release");
    }
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }

    let output = cmd
        .output()
        .map_err(|e| PackageError::InvalidManifest(format!("无法执行 cargo: {e}")))?;
    if !output.status.success() {
        return Err(PackageError::InvalidManifest(format!(
            "cargo build 失败:\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let native_dir = pkg_dir.join("build").join("native").join(current_triple());
    std::fs::create_dir_all(&native_dir)?;
    let built = target_dir.join(profile);
    if built.is_dir() {
        for entry in std::fs::read_dir(&built)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if LIB_EXTENSIONS.iter().any(|ext| name.ends_with(ext)) {
                std::fs::copy(&path, native_dir.join(&name))?;
            }
        }
    }

    Ok(BuildOutcome {
        native_dir: Some(native_dir),
        via: "cargo",
    })
}

/// 基础 features + 当前平台覆盖（去重；RFC：`--features ffi,linux-ffi`）
pub(crate) fn compose_features(config: &BuildConfig) -> Vec<String> {
    let mut features = config
        .cargo
        .as_ref()
        .map(|c| c.features.clone())
        .unwrap_or_default();
    if let Some(overrides) = config.platforms.get(&current_triple()) {
        for f in &overrides.cargo_features {
            if !features.contains(f) {
                features.push(f.clone());
            }
        }
    }
    features
}
