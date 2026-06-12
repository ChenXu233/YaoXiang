//! 依赖下载管理（Vendor）
//!
//! 管理 `.yaoxiang/vendor/` 目录中的已下载依赖。

pub mod cache;
pub mod fetcher;

use std::path::{Path, PathBuf};

use crate::package::dependency::DependencySpec;
use crate::package::error::PackageResult;
use crate::package::source::{self, ResolvedPackage};

/// Vendor 目录名称
pub const VENDOR_DIR: &str = ".yaoxiang";
/// Vendor 子目录
pub const VENDOR_SUBDIR: &str = "vendor";

/// Vendor 目录管理器
///
/// 管理依赖的下载和本地存储。
#[derive(Debug)]
pub struct VendorManager {
    /// 项目根目录
    project_dir: PathBuf,
    /// vendor 目录完整路径
    vendor_dir: PathBuf,
}

impl VendorManager {
    /// 创建新的 Vendor 管理器
    pub fn new(project_dir: &Path) -> Self {
        let vendor_dir = project_dir.join(VENDOR_DIR).join(VENDOR_SUBDIR);
        VendorManager {
            project_dir: project_dir.to_path_buf(),
            vendor_dir,
        }
    }

    /// 获取 vendor 目录路径
    pub fn vendor_dir(&self) -> &Path {
        &self.vendor_dir
    }

    /// 获取项目根目录
    pub fn project_dir(&self) -> &Path {
        &self.project_dir
    }

    /// 确保 vendor 目录存在
    pub fn ensure_vendor_dir(&self) -> PackageResult<()> {
        if !self.vendor_dir.exists() {
            std::fs::create_dir_all(&self.vendor_dir)?;
        }
        Ok(())
    }

    /// 获取依赖的安装路径
    ///
    /// 格式: `.yaoxiang/vendor/<name>-<version>/`
    pub fn dep_path(
        &self,
        name: &str,
        version: &str,
    ) -> PathBuf {
        self.vendor_dir.join(format!("{}-{}", name, version))
    }

    /// 检查依赖是否已安装
    pub fn is_installed(
        &self,
        name: &str,
        version: &str,
    ) -> bool {
        self.dep_path(name, version).exists()
    }

    /// 安装单个依赖
    ///
    /// 根据依赖规格选择来源，下载并安装到 vendor 目录。
    pub fn install_dependency(
        &self,
        spec: &DependencySpec,
    ) -> PackageResult<ResolvedPackage> {
        self.ensure_vendor_dir()?;

        let source = source::select_source(spec);

        // 解析版本
        let resolved_version = source.resolve(spec)?;

        // 检查是否已安装
        if self.is_installed(&spec.name, &resolved_version) {
            // 已安装，直接返回信息
            let local_path = self.dep_path(&spec.name, &resolved_version);
            let checksum = cache::compute_directory_checksum(&local_path)?;
            return Ok(ResolvedPackage {
                name: spec.name.clone(),
                version: resolved_version,
                source_kind: source.kind(),
                source_url: spec
                    .git
                    .clone()
                    .or_else(|| spec.path.clone())
                    .unwrap_or_else(|| "registry".to_string()),
                local_path,
                checksum: Some(checksum),
            });
        }

        // 下载依赖
        let mut resolved = source.download(spec, &self.vendor_dir)?;

        // 计算校验和
        let checksum = cache::compute_directory_checksum(&resolved.local_path)?;
        resolved.checksum = Some(checksum);

        Ok(resolved)
    }

    /// 卸载指定依赖
    pub fn uninstall_dependency(
        &self,
        name: &str,
        version: &str,
    ) -> PackageResult<bool> {
        let path = self.dep_path(name, version);
        if path.exists() {
            std::fs::remove_dir_all(&path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 列出已安装的依赖
    pub fn list_installed(&self) -> PackageResult<Vec<(String, String)>> {
        if !self.vendor_dir.exists() {
            return Ok(Vec::new());
        }

        let mut installed = Vec::new();
        for entry in std::fs::read_dir(&self.vendor_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let dir_name = entry.file_name().to_string_lossy().to_string();
                if let Some((name, version)) = parse_vendor_dir_name(&dir_name) {
                    installed.push((name, version));
                }
            }
        }
        installed.sort();
        Ok(installed)
    }

    /// 清理不再需要的依赖
    ///
    /// 删除所有不在 `keep` 列表中的已安装依赖。
    pub fn clean(
        &self,
        keep: &[(String, String)],
    ) -> PackageResult<Vec<String>> {
        let installed = self.list_installed()?;
        let mut removed = Vec::new();

        for (name, version) in &installed {
            let should_keep = keep
                .iter()
                .any(|(k_name, k_ver)| k_name == name && k_ver == version);

            if !should_keep {
                self.uninstall_dependency(name, version)?;
                removed.push(format!("{}-{}", name, version));
            }
        }

        Ok(removed)
    }

    /// 验证已安装依赖的完整性
    pub fn verify_integrity(
        &self,
        name: &str,
        version: &str,
        expected_checksum: &str,
    ) -> PackageResult<bool> {
        let path = self.dep_path(name, version);
        if !path.exists() {
            return Ok(false);
        }

        let actual_checksum = cache::compute_directory_checksum(&path)?;
        Ok(actual_checksum == expected_checksum)
    }
}

/// 解析 vendor 目录名称
///
/// 格式: `<name>-<version>`
/// 返回 `(name, version)`,如果无法解析则返回 None
fn parse_vendor_dir_name(dir_name: &str) -> Option<(String, String)> {
    // 从右侧查找最后一个 '-',因为包名可能包含 '-'
    let idx = dir_name.rfind('-')?;
    if idx == 0 || idx == dir_name.len() - 1 {
        return None;
    }

    let name = dir_name[..idx].to_string();
    let version = dir_name[idx + 1..].to_string();

    // 验证版本号格式（至少含一个数字）
    if version.chars().any(|c| c.is_ascii_digit()) {
        Some((name, version))
    } else {
        None
    }
}
