//! 构建系统（RFC-014b）
//!
//! 安装决策树（预编译优先，源码兜底）：
//! 1. `[binaries]` 有当前平台条目且 SHA-256 校验通过 → 下载解包，跳过构建（5d）
//! 2. `[build].headers` 有值 → yx-bindgen 生成绑定（RFC-026b 落地前明确报错，5f）
//! 3. 按 `[build].strategy` 执行：none 直接过 / cargo（5c）/ cmake / custom（信任门，5e 殿后）
//!
//! 执行顺序（2026-09-15 决议 2）：`5a → 5b → 5c → 5d → 5f → 5e`——声明式先行，
//! 任意代码执行殿后。

pub mod cargo;
pub mod requirements;

use std::path::{Path, PathBuf};

use crate::package::error::{PackageError, PackageResult};
use crate::package::manifest::BuildConfig;

/// 构建策略（RFC-014b）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildStrategy {
    /// 纯 .yx 包，无需构建（strategy 缺省同此）
    None,
    /// 调用 cargo build，读 `[build.cargo]` 配置
    Cargo,
    /// 调用 cmake
    Cmake,
    /// 执行 build.yx 脚本（信任门强制，5e 殿后实施）
    Custom,
}

impl BuildStrategy {
    /// 从 manifest 的 strategy 字符串解析；未知值硬错误（列合法值）
    pub fn parse(s: Option<&str>) -> PackageResult<Self> {
        match s.unwrap_or("none") {
            "none" => Ok(BuildStrategy::None),
            "cargo" => Ok(BuildStrategy::Cargo),
            "cmake" => Ok(BuildStrategy::Cmake),
            "custom" => Ok(BuildStrategy::Custom),
            other => Err(PackageError::InvalidManifest(format!(
                "未知构建策略 '{other}'（合法值：none / cargo / cmake / custom）"
            ))),
        }
    }

    /// 策略名（报错与日志用）
    pub fn name(&self) -> &'static str {
        match self {
            BuildStrategy::None => "none",
            BuildStrategy::Cargo => "cargo",
            BuildStrategy::Cmake => "cmake",
            BuildStrategy::Custom => "custom",
        }
    }
}

impl BuildConfig {
    /// 解析后的策略（缺省 none）
    pub fn parsed_strategy(&self) -> PackageResult<BuildStrategy> {
        BuildStrategy::parse(self.strategy.as_deref())
    }
}

/// 当前运行平台的 target triple（RFC-014b：Rust 三元组格式，与 Cargo 生态对齐）
///
/// `arch-vendor-os[-env]`：Windows 分 msvc/gnu，Linux 分 gnu/musl，
/// macOS 用 apple vendor 的三段形态（`aarch64-apple-darwin`）。
pub fn current_triple() -> String {
    let arch = std::env::consts::ARCH;
    match std::env::consts::OS {
        "windows" => {
            let env = if cfg!(target_env = "msvc") {
                "msvc"
            } else {
                "gnu"
            };
            format!("{arch}-pc-windows-{env}")
        }
        "linux" => {
            let env = if cfg!(target_env = "musl") {
                "musl"
            } else {
                "gnu"
            };
            format!("{arch}-unknown-linux-{env}")
        }
        "macos" => format!("{arch}-apple-darwin"),
        other => format!("{arch}-unknown-{other}"),
    }
}

/// 单个依赖包的构建结果
#[derive(Debug, Default)]
pub struct BuildOutcome {
    /// 预编译产物目录（`build/native/<triple>/`），位于 vendor 内包目录下
    pub native_dir: Option<PathBuf>,
    /// 实际走的路径（报错/日志用）
    pub via: &'static str,
}

/// build.yx 信任决策输入（5e 信任门落地时启用）
#[derive(Debug, Clone, Default)]
pub struct TrustDecision {
    /// 命令行 `--trust` 显式信任
    pub flag: bool,
    /// 非交互环境（CI 默认拒绝 custom，除非 flag）
    pub interactive: bool,
}

/// 执行安装决策树（在依赖包源码落位 `pkg_dir` 后调用）
///
/// `scratch_root` 是项目构建 scratch（`.yaoxiang/build/`），cargo target/
/// 隔离于此（见 [`cargo`] 模块文档）。无 `[build]` 且无 `[binaries]` 的包
/// 零构建直过——绝大多数纯 .yx 包的成本是一次字段查找。
pub fn run_install_build(
    pkg_dir: &Path,
    manifest: &crate::package::manifest::PackageManifest,
    scratch_root: &Path,
    _trust: &TrustDecision,
) -> PackageResult<BuildOutcome> {
    let build = match &manifest.build {
        Some(b) => b,
        None if manifest.binaries.is_empty() => {
            return Ok(BuildOutcome {
                native_dir: None,
                via: "no-build",
            })
        }
        None => &BuildConfig::default(),
    };

    // 1. 预编译优先（5d 落地）
    if !manifest.binaries.is_empty() {
        return Err(PackageError::InvalidManifest(
            "[binaries] 预编译分发尚未实现（RFC-014b Phase 5d）".to_string(),
        ));
    }

    // 2. headers → yx-bindgen（5f；RFC-026b 落地前明确报错）
    if !build.headers.is_empty() {
        return Err(PackageError::InvalidManifest(
            "[build].headers 需要 yx-bindgen（RFC-026b，尚未实现）；\
             移除 headers 或改用 [binaries] 预编译分发"
                .to_string(),
        ));
    }

    // 3. 策略执行（cargo/cmake 前先过工具预检，RFC-014b 5b）
    let strategy = build.parsed_strategy()?;
    match strategy {
        BuildStrategy::None => Ok(BuildOutcome {
            native_dir: None,
            via: "strategy-none",
        }),
        BuildStrategy::Cargo => {
            requirements::check(&build.requirements)?;
            cargo::build(pkg_dir, build, scratch_root)
        }
        BuildStrategy::Cmake => Err(PackageError::InvalidManifest(
            "cmake 构建策略尚未实现（RFC-014b Phase 5c 之后评估）".to_string(),
        )),
        BuildStrategy::Custom => Err(PackageError::InvalidManifest(
            "custom 构建策略尚未实现（RFC-014b Phase 5e，带信任门）".to_string(),
        )),
    }
}
