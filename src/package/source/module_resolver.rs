//! 模块解析器 - Vendor 集成
//!
//! 将 `.yaoxiang/vendor/` 中的依赖集成到模块解析路径中。
//! 查找顺序: vendor → src → YXPATH → std

use std::path::{Path, PathBuf};

use crate::package::vendor::{VENDOR_DIR, VENDOR_SUBDIR};

/// 模块解析器
///
/// 按优先级在多个路径中查找模块源文件。
#[derive(Debug, Clone)]
pub struct ModuleResolver {
    /// 项目根目录
    project_dir: PathBuf,
    /// 额外搜索路径 (YXPATH)
    search_paths: Vec<PathBuf>,
}

impl ModuleResolver {
    /// 创建新的模块解析器
    pub fn new(project_dir: &Path) -> Self {
        let mut search_paths = Vec::new();

        // 从环境变量 YXPATH 读取额外搜索路径
        if let Ok(yxpath) = std::env::var("YXPATH") {
            for path in yxpath.split(';').chain(yxpath.split(':')) {
                let p = PathBuf::from(path.trim());
                if p.exists() && !search_paths.contains(&p) {
                    search_paths.push(p);
                }
            }
        }

        ModuleResolver {
            project_dir: project_dir.to_path_buf(),
            search_paths,
        }
    }

    /// 解析模块路径
    ///
    /// 根据 `use` 语句中的模块路径查找源文件。
    ///
    /// 查找顺序:
    /// 1. `.yaoxiang/vendor/<module_name>-*/` — 已安装的依赖
    /// 2. `src/` — 项目源代码
    /// 3. YXPATH 环境变量中的目录
    /// 4. 标准库（内置，不在此处解析）
    ///
    /// # 参数
    /// - `module_path`: 点分隔的模块路径，如 `"foo.bar"`
    ///
    /// # 返回
    /// 找到的模块源文件路径，或 None
    pub fn resolve(
        &self,
        module_path: &str,
    ) -> Option<PathBuf> {
        let parts: Vec<&str> = module_path.split('.').collect();
        if parts.is_empty() || parts[0].is_empty() {
            return None;
        }

        let module_name = parts[0];
        let sub_path = if parts.len() > 1 {
            Some(parts[1..].join(std::path::MAIN_SEPARATOR_STR))
        } else {
            None
        };

        // 1. 在 vendor 目录中查找
        if let Some(path) = self.resolve_in_vendor(module_name, sub_path.as_deref()) {
            return Some(path);
        }

        // 2. 在 src 目录中查找
        if let Some(path) = self.resolve_in_dir(
            &self.project_dir.join("src"),
            module_name,
            sub_path.as_deref(),
        ) {
            return Some(path);
        }

        // 3. 在 YXPATH 中查找
        for search_path in &self.search_paths {
            if let Some(path) = self.resolve_in_dir(search_path, module_name, sub_path.as_deref()) {
                return Some(path);
            }
        }

        None
    }

    /// 在 vendor 目录中查找模块
    fn resolve_in_vendor(
        &self,
        module_name: &str,
        sub_path: Option<&str>,
    ) -> Option<PathBuf> {
        let vendor_dir = self.project_dir.join(VENDOR_DIR).join(VENDOR_SUBDIR);
        if !vendor_dir.exists() {
            return None;
        }

        // 查找匹配的已安装依赖（格式: <name>-<version>/）
        let entries = std::fs::read_dir(&vendor_dir).ok()?;
        let mut best_match: Option<PathBuf> = None;

        for entry in entries {
            let entry = entry.ok()?;
            if !entry.file_type().ok()?.is_dir() {
                continue;
            }

            let dir_name = entry.file_name().to_string_lossy().to_string();
            // 检查目录是否以模块名开头
            if dir_name.starts_with(module_name)
                && dir_name.len() > module_name.len()
                && dir_name.as_bytes()[module_name.len()] == b'-'
            {
                let dep_dir = entry.path();

                // 查找入口文件
                let entry_file = self.find_entry_file(&dep_dir, sub_path);
                if entry_file.is_some() {
                    best_match = entry_file;
                    // 继续查找，取最后一个（通常是最高版本，因为字母排序）
                }
            }
        }

        best_match
    }

    /// 在指定目录中查找模块
    fn resolve_in_dir(
        &self,
        base_dir: &Path,
        module_name: &str,
        sub_path: Option<&str>,
    ) -> Option<PathBuf> {
        if !base_dir.exists() {
            return None;
        }

        let module_dir = base_dir.join(module_name);
        self.find_entry_file(&module_dir, sub_path).or_else(|| {
            // 尝试直接作为文件
            let file_path = base_dir.join(format!("{}.yx", module_name));
            if file_path.exists() {
                Some(file_path)
            } else {
                None
            }
        })
    }

    /// 在依赖目录中查找入口文件
    fn find_entry_file(
        &self,
        dep_dir: &Path,
        sub_path: Option<&str>,
    ) -> Option<PathBuf> {
        if !dep_dir.exists() {
            return None;
        }

        if let Some(sub) = sub_path {
            // 查找子模块: <dep_dir>/src/<sub>.yx 或 <dep_dir>/<sub>.yx
            let candidates = [
                dep_dir.join("src").join(format!("{}.yx", sub)),
                dep_dir.join(format!("{}.yx", sub)),
                dep_dir.join("src").join(sub).join("mod.yx"),
                dep_dir.join(sub).join("mod.yx"),
            ];

            for candidate in &candidates {
                if candidate.exists() {
                    return Some(candidate.clone());
                }
            }
        } else {
            // 查找入口文件
            let candidates = [
                dep_dir.join("src").join("lib.yx"),
                dep_dir.join("src").join("main.yx"),
                dep_dir.join("lib.yx"),
                dep_dir.join("main.yx"),
                dep_dir.join("mod.yx"),
            ];

            for candidate in &candidates {
                if candidate.exists() {
                    return Some(candidate.clone());
                }
            }
        }

        None
    }

    /// 列出所有可用的依赖模块
    pub fn list_available_modules(&self) -> Vec<String> {
        let mut modules = Vec::new();

        let vendor_dir = self.project_dir.join(VENDOR_DIR).join(VENDOR_SUBDIR);
        if vendor_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&vendor_dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        let dir_name = entry.file_name().to_string_lossy().to_string();
                        if let Some(idx) = dir_name.rfind('-') {
                            let name = &dir_name[..idx];
                            if !modules.contains(&name.to_string()) {
                                modules.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }

        modules.sort();
        modules
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_project_with_vendor() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().to_path_buf();

        // 创建 vendor 目录和模拟依赖
        let vendor = project_dir.join(VENDOR_DIR).join(VENDOR_SUBDIR);
        let dep_dir = vendor.join("foo-1.0.0");
        std::fs::create_dir_all(dep_dir.join("src")).unwrap();
        std::fs::write(dep_dir.join("src").join("lib.yx"), "export x = 42").unwrap();
        std::fs::write(dep_dir.join("src").join("utils.yx"), "export y = 100").unwrap();

        // 创建 src 目录
        let src_dir = project_dir.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("main.yx"), "main = { }").unwrap();
        std::fs::write(src_dir.join("local_mod.yx"), "export z = 0").unwrap();

        (tmp, project_dir)
    }

    #[test]
    fn test_resolve_vendor_module() {
        let (_tmp, project_dir) = setup_project_with_vendor();
        let resolver = ModuleResolver::new(&project_dir);

        let result = resolver.resolve("foo");
        assert!(result.is_some());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("foo-1.0.0"));
        assert!(path.to_string_lossy().contains("lib.yx"));
    }

    #[test]
    fn test_resolve_vendor_submodule() {
        let (_tmp, project_dir) = setup_project_with_vendor();
        let resolver = ModuleResolver::new(&project_dir);

        let result = resolver.resolve("foo.utils");
        assert!(result.is_some());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("utils.yx"));
    }

    #[test]
    fn test_resolve_src_module() {
        let (_tmp, project_dir) = setup_project_with_vendor();
        let resolver = ModuleResolver::new(&project_dir);

        let result = resolver.resolve("local_mod");
        assert!(result.is_some());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("local_mod.yx"));
    }

    #[test]
    fn test_vendor_takes_priority() {
        let (_tmp, project_dir) = setup_project_with_vendor();

        // 也在 src 中创建同名模块
        std::fs::write(project_dir.join("src").join("foo.yx"), "export x = -1").unwrap();

        let resolver = ModuleResolver::new(&project_dir);

        let result = resolver.resolve("foo");
        assert!(result.is_some());
        let path = result.unwrap();
        // vendor 优先
        assert!(path.to_string_lossy().contains("vendor"));
    }

    #[test]
    fn test_resolve_nonexistent() {
        let (_tmp, project_dir) = setup_project_with_vendor();
        let resolver = ModuleResolver::new(&project_dir);

        let result = resolver.resolve("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_list_available_modules() {
        let (_tmp, project_dir) = setup_project_with_vendor();

        // 添加更多依赖
        let vendor = project_dir.join(VENDOR_DIR).join(VENDOR_SUBDIR);
        std::fs::create_dir_all(vendor.join("bar-2.0.0")).unwrap();

        let resolver = ModuleResolver::new(&project_dir);
        let modules = resolver.list_available_modules();

        assert_eq!(modules.len(), 2);
        assert!(modules.contains(&"bar".to_string()));
        assert!(modules.contains(&"foo".to_string()));
    }

    #[test]
    fn test_resolve_empty_path() {
        let (_tmp, project_dir) = setup_project_with_vendor();
        let resolver = ModuleResolver::new(&project_dir);

        let result = resolver.resolve("");
        assert!(result.is_none());
    }

    #[test]
    fn test_multi_version_resolves_latest() {
        let tmp = TempDir::new().unwrap();
        let project_dir = tmp.path().to_path_buf();

        let vendor = project_dir.join(VENDOR_DIR).join(VENDOR_SUBDIR);
        // 两个版本
        for ver in &["1.0.0", "2.0.0"] {
            let dep = vendor.join(format!("mylib-{}", ver));
            std::fs::create_dir_all(dep.join("src")).unwrap();
            std::fs::write(dep.join("src").join("lib.yx"), "export v = 1").unwrap();
        }

        let resolver = ModuleResolver::new(&project_dir);
        let result = resolver.resolve("mylib");
        assert!(result.is_some());
    }
}
