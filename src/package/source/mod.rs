//! 依赖来源抽象
//!
//! 定义 `Source` trait 和各种来源实现，包括本地路径、Git 和注册表。

pub mod conflict;
pub mod git;
pub mod module_resolver;
pub mod resolver;

use std::path::{Path, PathBuf};

use crate::package::dependency::DependencySpec;
use crate::package::error::PackageResult;

/// 依赖来源类型
#[derive(Debug, Clone, PartialEq)]
pub enum SourceKind {
    /// 本地路径来源
    Local,
    /// Git 仓库来源
    Git,
    /// 注册表来源（预留）
    Registry,
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
#[derive(Debug)]
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

/// 注册表来源（预留 Phase 3）
///
/// 目前仅保存版本信息到锁文件，不进行实际下载。
#[derive(Debug)]
pub struct RegistrySource;

impl RegistrySource {
    /// 创建注册表来源
    pub fn new() -> Self {
        RegistrySource
    }
}

impl Default for RegistrySource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for RegistrySource {
    fn name(&self) -> &str {
        "registry"
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Registry
    }

    fn resolve(
        &self,
        spec: &DependencySpec,
    ) -> PackageResult<String> {
        // Phase 3 将实现注册表版本查询
        // 目前直接返回声明的版本
        Ok(spec.version.clone())
    }

    fn download(
        &self,
        spec: &DependencySpec,
        _dest: &Path,
    ) -> PackageResult<ResolvedPackage> {
        // Phase 3 将实现注册表下载
        // 目前创建一个占位入口
        Ok(ResolvedPackage {
            name: spec.name.clone(),
            version: spec.version.clone(),
            source_kind: SourceKind::Registry,
            source_url: "registry".to_string(),
            local_path: PathBuf::new(),
            checksum: None,
        })
    }
}

/// 根据依赖规格选择合适的来源
pub fn select_source(spec: &DependencySpec) -> Box<dyn Source> {
    if spec.path.is_some() {
        Box::new(LocalSource::new())
    } else if spec.git.is_some() {
        Box::new(git::GitSource::new())
    } else {
        // 注册表来源（Phase 3 完善）
        Box::new(RegistrySource::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_kind_display() {
        assert_eq!(SourceKind::Local.to_string(), "path");
        assert_eq!(SourceKind::Git.to_string(), "git");
        assert_eq!(SourceKind::Registry.to_string(), "registry");
    }

    #[test]
    fn test_local_source_name() {
        let source = LocalSource::new();
        assert_eq!(source.name(), "local");
        assert_eq!(source.kind(), SourceKind::Local);
    }

    #[test]
    fn test_local_source_resolve() {
        let source = LocalSource::new();
        let spec = DependencySpec {
            name: "foo".to_string(),
            version: "1.0.0".to_string(),
            git: None,
            path: Some("./local-dep".to_string()),
        };
        let version = source.resolve(&spec).unwrap();
        assert_eq!(version, "1.0.0");
    }

    #[test]
    fn test_local_source_download_missing_path() {
        let source = LocalSource::new();
        let spec = DependencySpec {
            name: "foo".to_string(),
            version: "1.0.0".to_string(),
            git: None,
            path: None,
        };
        let result = source.download(&spec, Path::new("/tmp"));
        assert!(result.is_err());
    }

    #[test]
    fn test_select_source_local() {
        let spec = DependencySpec {
            name: "foo".to_string(),
            version: "1.0.0".to_string(),
            git: None,
            path: Some("./local".to_string()),
        };
        let source = select_source(&spec);
        assert_eq!(source.kind(), SourceKind::Local);
    }

    #[test]
    fn test_select_source_git() {
        let spec = DependencySpec {
            name: "foo".to_string(),
            version: "1.0.0".to_string(),
            git: Some("https://github.com/example/foo".to_string()),
            path: None,
        };
        let source = select_source(&spec);
        assert_eq!(source.kind(), SourceKind::Git);
    }
}
