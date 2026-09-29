//! 依赖来源抽象
//!
//! 定义 `Source` trait 和各种来源实现，包括本地路径、Git 和注册表。

pub mod conflict;
pub mod git;
pub mod resolver;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use crate::package::dependency::DependencySpec;
use crate::package::error::{PackageError, PackageResult};

/// 依赖来源类型
#[derive(Debug, Clone, PartialEq)]
pub enum SourceKind {
    /// 本地路径来源
    Local,
    /// Git 仓库来源
    Git,
    /// 注册表来源（Phase 4 后置，RFC-014a 决议 2）
    Registry,
    /// GitHub Release 适配层（Phase 4 落地）
    GitHub,
}

impl std::fmt::Display for SourceKind {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            SourceKind::Local => write!(f, "path"),
            SourceKind::Git => write!(f, "git"),
            SourceKind::Registry => write!(f, "registry"),
            SourceKind::GitHub => write!(f, "github"),
        }
    }
}

/// 已解析的依赖包信息
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    /// 包名
    pub name: String,
    /// 解析后的版本
    pub version: String,
    /// 来源类型
    pub source_kind: SourceKind,
    /// 来源描述（URL、路径等）
    pub source_url: String,
    /// 下载后的本地路径
    pub local_path: PathBuf,
    /// SHA-256 校验和
    pub checksum: Option<String>,
}

/// 依赖来源 trait
///
/// 所有依赖来源（Git、本地路径、注册表）都需要实现此 trait。
pub trait Source {
    /// 来源名称
    fn name(&self) -> &str;

    /// 来源类型
    fn kind(&self) -> SourceKind;

    /// 解析依赖版本
    ///
    /// 根据依赖规格查找可用版本，返回最佳匹配的版本字符串。
    fn resolve(
        &self,
        spec: &DependencySpec,
    ) -> PackageResult<String>;

    /// 下载依赖到指定目录
    ///
    /// 将依赖下载到 `dest` 目录，返回已解析的包信息。
    fn download(
        &self,
        spec: &DependencySpec,
        dest: &Path,
    ) -> PackageResult<ResolvedPackage>;
}

/// 本地路径来源
///
/// 从本地文件系统路径加载依赖。
#[derive(Debug, Clone)]
pub struct LocalSource;

impl LocalSource {
    /// 创建新的本地来源
    pub fn new() -> Self {
        LocalSource
    }
}

impl Default for LocalSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for LocalSource {
    fn name(&self) -> &str {
        "local"
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Local
    }

    fn resolve(
        &self,
        spec: &DependencySpec,
    ) -> PackageResult<String> {
        // 本地依赖使用 manifest 中声明的版本
        Ok(spec.version.clone())
    }

    fn download(
        &self,
        spec: &DependencySpec,
        _dest: &Path,
    ) -> PackageResult<ResolvedPackage> {
        let path = spec.path.as_ref().ok_or_else(|| {
            crate::package::error::PackageError::InvalidManifest(format!(
                "本地依赖 '{}' 缺少 path 字段",
                spec.name
            ))
        })?;

        let local_path = PathBuf::from(path);
        if !local_path.exists() {
            return Err(crate::package::error::PackageError::DependencyNotFound(
                format!("本地路径不存在: {}", path),
            ));
        }

        Ok(ResolvedPackage {
            name: spec.name.clone(),
            version: spec.version.clone(),
            source_kind: SourceKind::Local,
            source_url: path.clone(),
            local_path,
            checksum: None,
        })
    }
}

/// 内置依赖来源（RFC-014a 决议 4：封闭集合 enum 分发）
///
/// 避免 dyn-async 的 Send 约束与 `async-trait` 依赖；[`Source`] trait 保留在
/// 语义层，未来若开放第三方 Source 再经 trait 对象接入。Registry/GitHub 在
/// Phase 4 落地前返回明确的「后置」错误（RFC-014a 决议 2：官方 Registry
/// 无限期后置）。
#[derive(Debug, Clone)]
pub enum AnySource {
    /// 本地路径来源
    Local(LocalSource),
    /// Git 仓库来源
    Git(git::GitSource),
    /// 官方 Registry（Phase 4 后置；`--git`/`--path` 显式来源上线前的占位）
    Registry,
    /// GitHub Release 适配层（Phase 4 落地）
    GitHub,
}

impl AnySource {
    /// 来源名称
    pub fn name(&self) -> &'static str {
        match self {
            AnySource::Local(_) => "local",
            AnySource::Git(_) => "git",
            AnySource::Registry => "registry",
            AnySource::GitHub => "github",
        }
    }

    /// 来源类型
    pub fn kind(&self) -> SourceKind {
        match self {
            AnySource::Local(_) => SourceKind::Local,
            AnySource::Git(_) => SourceKind::Git,
            AnySource::Registry => SourceKind::Registry,
            AnySource::GitHub => SourceKind::GitHub,
        }
    }

    /// 解析依赖版本（各具体来源实现见 [`Source`]）
    pub fn resolve(
        &self,
        spec: &DependencySpec,
    ) -> PackageResult<String> {
        match self {
            AnySource::Local(s) => s.resolve(spec),
            AnySource::Git(s) => s.resolve(spec),
            AnySource::Registry | AnySource::GitHub => Err(unsupported_source(self.name())),
        }
    }

    /// 下载依赖到指定目录
    pub fn download(
        &self,
        spec: &DependencySpec,
        dest: &Path,
    ) -> PackageResult<ResolvedPackage> {
        match self {
            AnySource::Local(s) => s.download(spec, dest),
            AnySource::Git(s) => s.download(spec, dest),
            AnySource::Registry | AnySource::GitHub => Err(unsupported_source(self.name())),
        }
    }
}

/// 未落地来源的统一错误（Phase 4 后置，RFC-014a 决议 2/4）
fn unsupported_source(name: &str) -> PackageError {
    PackageError::DependencyNotFound(format!(
        "source '{name}' not implemented yet (deferred to Phase 4, RFC-014a); \
         use --git or --path dependencies"
    ))
}

/// 根据依赖规格选择合适的来源
///
/// 014a 决议 4：内置来源是封闭集合，裸版本依赖（无 git/path）归 Registry
/// 占位——resolve/download 返回明确的「后置」错误，由调用方（fetcher 预检）
/// 决定呈现方式。
pub fn select_source(spec: &DependencySpec) -> AnySource {
    if spec.path.is_some() {
        AnySource::Local(LocalSource::new())
    } else if spec.git.is_some() {
        AnySource::Git(git::GitSource::new())
    } else {
        AnySource::Registry
    }
}
