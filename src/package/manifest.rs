//! yaoxiang.toml manifest parsing and writing

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::package::error::{PackageError, PackageResult};
use crate::util::config::I18nConfig;

/// The main manifest file name
pub const MANIFEST_FILE: &str = "yaoxiang.toml";

/// Represents the `[package]` section of yaoxiang.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package name
    pub name: String,
    /// Package version (semver string)
    pub version: String,
    /// Package description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Package authors
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    /// Package license
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Package repository URL（publish --github 的目标仓库解析来源之一）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

/// `[lib]` section (RFC-015): the package's library entry file.
/// 消费语义见 RFC-029f：lib.path 是导入面的单文件回退（exports 缺省时）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LibTarget {
    /// Library entry path (relative to the manifest directory)
    pub path: String,
}

/// `[[bin]]` section (RFC-015): a binary target entry file.
/// 消费语义见 RFC-029f：bin 角色文件内未使用的 pub 可报死代码（W1001 族）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BinTarget {
    /// Target name (used by build/dispatch tooling)
    pub name: String,
    /// Entry path (relative to the manifest directory)
    pub path: String,
}

/// `[run]` section (RFC-015): default program entry and arguments.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RunConfig {
    /// Default entry file (`run` without explicit path)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,
    /// Arguments passed to the program
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

/// `[build]` section (RFC-014b)：构建声明。
/// strategy 缺省视为 "none"（纯 .yx 包）；类型校验在 `build` 模块的
/// `BuildStrategy::parse`（未知名在那里报错，manifest 保持宽容解析）。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct BuildConfig {
    /// 构建策略名：`none` / `cargo` / `cmake` / `custom`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    /// 需要 yx-bindgen 处理的 C 头文件（RFC-026b 集成点）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<String>,
    /// cargo 策略配置（`strategy = "cargo"` 时读取）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cargo: Option<CargoBuildConfig>,
    /// 构建工具预检：工具名 → 版本要求（如 `">= 1.70"`）
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub requirements: BTreeMap<String, String>,
    /// 平台特定覆盖：target triple → 覆盖项（如 `cargo-features`）
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub platforms: BTreeMap<String, PlatformOverrides>,
}

/// `[build.cargo]` section (RFC-014b)
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct CargoBuildConfig {
    /// `cargo build --features <逗号合并>`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    /// 构建档位：`"release"`（默认）或 `"debug"`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

/// `[build.platforms.<triple>]` 覆盖项 (RFC-014b)
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PlatformOverrides {
    /// 追加到基础 features 之后的平台特征
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        rename = "cargo-features"
    )]
    pub cargo_features: Vec<String>,
}

/// `[binaries.<triple>]` 条目 (RFC-014b)
///
/// sha256 缺省 = 该平台预编译路径不可用（完整性不可验证即不启用，
/// 回退源码构建——RFC「跳过构建的条件」要求校验通过）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BinaryArtifact {
    /// 下载地址（绝对 URL，或相对包仓库根的路径）
    pub url: String,
    /// 整包（tar.gz）的 SHA-256
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// Represents the complete yaoxiang.toml manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    /// Package metadata
    pub package: PackageInfo,
    /// Runtime dependencies
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependencies: BTreeMap<String, toml::Value>,
    /// Development dependencies
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        rename = "dev-dependencies"
    )]
    pub dev_dependencies: BTreeMap<String, toml::Value>,
    /// I18n configuration (project-level overrides user-level)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub i18n: Option<I18nConfig>,
    /// `[lib]` library entry (RFC-015)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lib: Option<LibTarget>,
    /// `[[bin]]` binary targets (RFC-015)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bin: Vec<BinTarget>,
    /// `[exports]` path mapping (RFC-015): use-path prefix → file path.
    /// 消费语义见 RFC-029f：导出面 = 映射值文件集合，跨包 use 的合法起点。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub exports: BTreeMap<String, String>,
    /// `[run]` default entry (RFC-015)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<RunConfig>,
    /// `[build]` 构建声明（RFC-014b）。成员自包含（014c 决议 2：无 workspace 级 [build]）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildConfig>,
    /// `[binaries]` 预编译产物声明（RFC-014b：唯一二进制分发机制；
    /// 平台三元组 → 产物）。存在即触发预编译优先，无需显式 strategy。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub binaries: BTreeMap<String, BinaryArtifact>,
}

impl PackageManifest {
    /// Create a new manifest with the given project name
    pub fn new(name: &str) -> Self {
        PackageManifest {
            package: PackageInfo {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                description: None,
                authors: Vec::new(),
                license: None,
                repository: None,
            },
            dependencies: BTreeMap::new(),
            dev_dependencies: BTreeMap::new(),
            i18n: None,
            lib: None,
            bin: Vec::new(),
            exports: BTreeMap::new(),
            run: None,
            build: None,
            binaries: BTreeMap::new(),
        }
    }

    /// Load manifest from a directory containing yaoxiang.toml
    pub fn load(dir: &Path) -> PackageResult<Self> {
        let path = dir.join(MANIFEST_FILE);
        if !path.exists() {
            return Err(PackageError::NotProject);
        }
        let content = std::fs::read_to_string(&path)?;
        let manifest: PackageManifest = toml::from_str(&content)?;
        Ok(manifest)
    }

    /// Save manifest to a directory
    pub fn save(
        &self,
        dir: &Path,
    ) -> PackageResult<()> {
        let path = dir.join(MANIFEST_FILE);
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Add a dependency
    pub fn add_dependency(
        &mut self,
        name: &str,
        version: &str,
    ) {
        self.dependencies
            .insert(name.to_string(), toml::Value::String(version.to_string()));
    }

    /// Add a dev dependency
    pub fn add_dev_dependency(
        &mut self,
        name: &str,
        version: &str,
    ) {
        self.dev_dependencies
            .insert(name.to_string(), toml::Value::String(version.to_string()));
    }

    /// Remove a dependency. Returns true if it was present.
    pub fn remove_dependency(
        &mut self,
        name: &str,
    ) -> bool {
        self.dependencies.remove(name).is_some()
    }

    /// Remove a dev dependency. Returns true if it was present.
    pub fn remove_dev_dependency(
        &mut self,
        name: &str,
    ) -> bool {
        self.dev_dependencies.remove(name).is_some()
    }

    /// Check if a dependency exists (in either dependencies or dev-dependencies)
    pub fn has_dependency(
        &self,
        name: &str,
    ) -> bool {
        self.dependencies.contains_key(name) || self.dev_dependencies.contains_key(name)
    }
}
