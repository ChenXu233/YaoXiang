//! Vendor 模块集成
//!
//! 将 `PackageManifest`（yaoxiang.toml）和 `VendorManager`（.yaoxiang/vendor/）
//! 与通用模块系统桥接，支持从 vendor 目录加载依赖模块。
//!
//! # 工作流程
//!
//! 1. 读取 `yaoxiang.toml` 获取声明的依赖
//! 2. 扫描 `.yaoxiang/vendor/` 目录查找已安装的依赖
//! 3. 为每个依赖创建 `ModuleInfo` 并注册到 `ModuleRegistry`

use std::path::{Path, PathBuf};

use crate::package::manifest::PackageManifest;
use crate::package::vendor::VendorManager;

use super::loader::ModuleLoader;
use super::registry::ModuleRegistry;
use super::ModuleError;

/// Vendor 模块桥接器
///
/// 负责发现 vendor 目录中的依赖模块并注册到模块系统。
#[derive(Debug)]
pub struct VendorBridge {
    /// 项目根目录
    project_root: PathBuf,
    /// Vendor 管理器
    vendor_manager: VendorManager,
}

impl VendorBridge {
    /// 创建新的 Vendor 桥接器
    pub fn new(project_root: &Path) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            vendor_manager: VendorManager::new(project_root),
        }
    }

    /// 从 yaoxiang.toml 和 vendor 目录发现并注册所有依赖模块
    ///
    /// # 工作流程
    ///
    /// 1. 尝试加载 yaoxiang.toml
    /// 2. 扫描 vendor 目录中已安装的依赖
    /// 3. 对每个已安装的依赖，解析其入口文件并提取导出项
    /// 4. 注册到 ModuleRegistry
    pub fn discover_and_register(
        &self,
        registry: &mut ModuleRegistry,
    ) -> Result<Vec<String>, ModuleError> {
        let mut registered = Vec::new();

        // 尝试加载 manifest
        let manifest = PackageManifest::load(&self.project_root).ok();

        // 获取已安装的依赖
        let installed = self.vendor_manager.list_installed().unwrap_or_default();

        for (name, version) in &installed {
            // 如果有 manifest，检查该依赖是否在声明的依赖中
            if let Some(ref m) = manifest {
                if !m.has_dependency(name) {
                    continue;
                }
            }

            // 查找依赖的入口文件
            let dep_path = self.vendor_manager.dep_path(name, version);
            if let Some(entry_file) = self.find_entry_file(&dep_path, name) {
                // 使用 ModuleLoader 解析依赖的导出项
                let mut loader = ModuleLoader::new(dep_path.clone(), entry_file.clone());

                match loader.load_vendor_module(name, &entry_file) {
                    Ok(module) => {
                        registry.register(module);
                        registered.push(name.clone());
                    }
                    Err(_) => {
                        // 依赖解析失败不阻止其他依赖加载
                        continue;
                    }
                }
            }
        }

        Ok(registered)
    }

    /// 查找依赖的入口文件
    ///
    /// 搜索规则：
    /// 1. `<dep>/src/<name>.yx`
    /// 2. `<dep>/src/mod.yx`
    /// 3. `<dep>/src/index.yx`
    /// 4. `<dep>/<name>.yx`
    fn find_entry_file(
        &self,
        dep_path: &Path,
        name: &str,
    ) -> Option<PathBuf> {
        let candidates = [
            dep_path.join("src").join(format!("{}.yx", name)),
            dep_path.join("src").join("mod.yx"),
            dep_path.join("src").join("index.yx"),
            dep_path.join(format!("{}.yx", name)),
        ];

        candidates.into_iter().find(|p| p.exists())
    }

    /// 检查项目是否有 yaoxiang.toml
    pub fn has_manifest(&self) -> bool {
        self.project_root.join("yaoxiang.toml").exists()
    }

    /// 获取 manifest 中声明的依赖名列表
    pub fn declared_dependencies(&self) -> Vec<String> {
        PackageManifest::load(&self.project_root)
            .map(|m| m.dependencies.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// 检查声明的依赖是否都已安装
    pub fn check_missing_dependencies(&self) -> Vec<String> {
        let declared = self.declared_dependencies();
        let installed = self.vendor_manager.list_installed().unwrap_or_default();

        let installed_names: std::collections::HashSet<String> =
            installed.into_iter().map(|(name, _)| name).collect();

        declared
            .into_iter()
            .filter(|name| !installed_names.contains(name))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::module::ModuleSource;
    use tempfile::TempDir;

    #[test]
    fn test_vendor_bridge_no_manifest() {
        let dir = TempDir::new().unwrap();
        let bridge = VendorBridge::new(dir.path());
        assert!(!bridge.has_manifest());
        assert!(bridge.declared_dependencies().is_empty());
    }

    #[test]
    fn test_vendor_bridge_with_manifest() {
        let dir = TempDir::new().unwrap();

        // 创建 yaoxiang.toml
        let mut manifest = PackageManifest::new("test-project");
        manifest.add_dependency("foo", "1.0.0");
        manifest.save(dir.path()).unwrap();

        let bridge = VendorBridge::new(dir.path());
        assert!(bridge.has_manifest());
        assert_eq!(bridge.declared_dependencies(), vec!["foo".to_string()]);
    }

    #[test]
    fn test_check_missing_dependencies() {
        let dir = TempDir::new().unwrap();

        let mut manifest = PackageManifest::new("test-project");
        manifest.add_dependency("foo", "1.0.0");
        manifest.add_dependency("bar", "2.0.0");
        manifest.save(dir.path()).unwrap();

        let bridge = VendorBridge::new(dir.path());
        let missing = bridge.check_missing_dependencies();
        // 两个依赖都未安装
        assert!(missing.contains(&"foo".to_string()));
        assert!(missing.contains(&"bar".to_string()));
    }

    #[test]
    fn test_discover_empty_vendor() {
        let dir = TempDir::new().unwrap();
        let bridge = VendorBridge::new(dir.path());
        let mut registry = ModuleRegistry::with_std();
        let result = bridge.discover_and_register(&mut registry);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_discover_with_installed_dep() {
        let dir = TempDir::new().unwrap();

        // 创建 yaoxiang.toml
        let mut manifest = PackageManifest::new("test-project");
        manifest.add_dependency("mylib", "1.0.0");
        manifest.save(dir.path()).unwrap();

        // 创建 vendor 目录和依赖
        let vendor_dir = dir
            .path()
            .join(".yaoxiang")
            .join("vendor")
            .join("mylib-1.0.0")
            .join("src");
        std::fs::create_dir_all(&vendor_dir).unwrap();

        // 创建依赖的入口文件
        std::fs::write(
            vendor_dir.join("mylib.yx"),
            r#"
pub greet: (name: String) -> String = (name) => {
    "Hello, " + name
}

VERSION = "1.0.0"
"#,
        )
        .unwrap();

        let bridge = VendorBridge::new(dir.path());
        let mut registry = ModuleRegistry::with_std();
        let result = bridge.discover_and_register(&mut registry);
        assert!(result.is_ok());
        let registered = result.unwrap();
        assert_eq!(registered, vec!["mylib".to_string()]);

        // 验证模块已注册
        let module = registry.get("mylib").unwrap();
        assert!(module.has_export("greet"));
        assert!(module.has_export("VERSION"));
        assert_eq!(module.source, ModuleSource::Vendor);
    }
}
