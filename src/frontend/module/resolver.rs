//! 模块路径解析器
//!
//! 将模块路径（如 "std.io"、"my_module.utils"）解析为实际的文件系统路径或内置模块引用。
//!
//! # 搜索顺序
//!
//! 1. std 模块（内置，不需要文件系统）
//! 2. vendor 目录（.yaoxiang/vendor/）
//! 3. 当前项目 src 目录
//! 4. 全局路径（$YXPATH，预留）

use std::path::{Path, PathBuf};

use super::ModuleError;

/// 解析后的模块位置
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleLocation {
    /// 标准库模块（内置）
    Std(String),
    /// 文件系统模块
    File(PathBuf),
    /// vendor 目录下的依赖模块
    Vendor(PathBuf),
}

/// 模块路径解析器
#[derive(Debug)]
pub struct ModuleResolver {
    /// 项目根目录
    project_root: PathBuf,
    /// 当前文件路径
    current_file: PathBuf,
}

impl ModuleResolver {
    /// 创建新的解析器
    pub fn new(
        project_root: PathBuf,
        current_file: PathBuf,
    ) -> Self {
        Self {
            project_root,
            current_file,
        }
    }

    /// 解析模块路径
    ///
    /// 将模块路径转换为模块位置。
    ///
    /// # 示例
    ///
    /// - "std.io" -> Std("std.io")
    /// - "my_module" -> File("./my_module.yx") 或 File("./my_module/mod.yx")
    pub fn resolve(
        &self,
        module_path: &str,
    ) -> Result<ModuleLocation, ModuleError> {
        // 1. 检查是否是 std 模块
        if module_path == "std" || module_path.starts_with("std.") {
            return Ok(ModuleLocation::Std(module_path.to_string()));
        }

        // 2. 尝试从 vendor 目录查找
        if let Some(vendor_path) = self.find_in_vendor(module_path) {
            return Ok(ModuleLocation::Vendor(vendor_path));
        }

        // 3. 尝试从当前文件的相对路径查找
        if let Some(file_path) = self.find_relative(module_path) {
            return Ok(ModuleLocation::File(file_path));
        }

        // 4. 尝试从项目 src 目录查找
        if let Some(file_path) = self.find_in_src(module_path) {
            return Ok(ModuleLocation::File(file_path));
        }

        Err(ModuleError::NotFound {
            path: module_path.to_string(),
            searched_paths: self.searched_paths(module_path),
        })
    }

    /// 在 vendor 目录中查找模块
    fn find_in_vendor(
        &self,
        module_path: &str,
    ) -> Option<PathBuf> {
        let vendor_dir = self.project_root.join(".yaoxiang").join("vendor");
        if !vendor_dir.exists() {
            return None;
        }

        let parts: Vec<&str> = module_path.split('.').collect();
        if parts.is_empty() {
            return None;
        }

        // 在 vendor 中搜索匹配的包
        // 模式: .yaoxiang/vendor/package-version/src/path.yx
        if let Ok(entries) = std::fs::read_dir(&vendor_dir) {
            for entry in entries.flatten() {
                let dir_name = entry.file_name().to_string_lossy().to_string();
                // 检查目录名是否以模块名开头
                if dir_name.starts_with(parts[0]) {
                    let remaining = &parts[1..];
                    let base = entry.path().join("src");
                    if let Some(path) = self.find_module_file(&base, remaining) {
                        return Some(path);
                    }
                }
            }
        }

        None
    }

    /// 相对于当前文件查找模块
    fn find_relative(
        &self,
        module_path: &str,
    ) -> Option<PathBuf> {
        let current_dir = self.current_file.parent()?;
        let parts: Vec<&str> = module_path.split('.').collect();
        self.find_module_file(current_dir, &parts)
    }

    /// 在项目 src 目录中查找模块
    fn find_in_src(
        &self,
        module_path: &str,
    ) -> Option<PathBuf> {
        let src_dir = self.project_root.join("src");
        if !src_dir.exists() {
            return None;
        }

        let parts: Vec<&str> = module_path.split('.').collect();
        self.find_module_file(&src_dir, &parts)
    }

    /// 从基础目录查找模块文件
    ///
    /// 搜索规则：
    /// 1. base/module_name.yx
    /// 2. base/module_name/mod.yx
    /// 3. base/module_name/index.yx
    fn find_module_file(
        &self,
        base: &Path,
        parts: &[&str],
    ) -> Option<PathBuf> {
        if parts.is_empty() {
            return None;
        }

        // 构建路径
        let mut path = base.to_path_buf();
        for &part in &parts[..parts.len() - 1] {
            path = path.join(part);
        }

        let last = parts[parts.len() - 1];

        // 尝试 module_name.yx
        let file_path = path.join(format!("{}.yx", last));
        if file_path.exists() {
            return Some(file_path);
        }

        // 尝试 module_name/mod.yx
        let mod_path = path.join(last).join("mod.yx");
        if mod_path.exists() {
            return Some(mod_path);
        }

        // 尝试 module_name/index.yx
        let index_path = path.join(last).join("index.yx");
        if index_path.exists() {
            return Some(index_path);
        }

        None
    }

    /// 返回搜索过的路径列表（用于错误消息）
    fn searched_paths(
        &self,
        module_path: &str,
    ) -> Vec<String> {
        let parts: Vec<&str> = module_path.split('.').collect();
        let last = parts.last().map(|s| *s).unwrap_or(module_path);

        let current_dir = self.current_file.parent().unwrap_or(Path::new("."));
        let src_dir = self.project_root.join("src");

        let mut paths = Vec::new();

        // 相对路径的搜索位置
        let mut rel_path = current_dir.to_path_buf();
        for &part in &parts[..parts.len().saturating_sub(1)] {
            rel_path = rel_path.join(part);
        }
        paths.push(format!("{}/{}.yx", rel_path.display(), last));
        paths.push(format!("{}/{}/mod.yx", rel_path.display(), last));

        // src 目录的搜索位置
        let mut src_path = src_dir;
        for &part in &parts[..parts.len().saturating_sub(1)] {
            src_path = src_path.join(part);
        }
        paths.push(format!("{}/{}.yx", src_path.display(), last));
        paths.push(format!("{}/{}/mod.yx", src_path.display(), last));

        // vendor 目录
        let vendor_dir = self.project_root.join(".yaoxiang").join("vendor");
        paths.push(format!(
            "{}/{}*/src/{}.yx",
            vendor_dir.display(),
            parts[0],
            last
        ));

        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_std_module() {
        let resolver = ModuleResolver::new(
            PathBuf::from("/project"),
            PathBuf::from("/project/src/main.yx"),
        );

        assert_eq!(
            resolver.resolve("std.io").unwrap(),
            ModuleLocation::Std("std.io".to_string())
        );

        assert_eq!(
            resolver.resolve("std").unwrap(),
            ModuleLocation::Std("std".to_string())
        );

        assert_eq!(
            resolver.resolve("std.math").unwrap(),
            ModuleLocation::Std("std.math".to_string())
        );
    }

    #[test]
    fn test_resolve_nonexistent_module() {
        let resolver = ModuleResolver::new(
            PathBuf::from("/project"),
            PathBuf::from("/project/src/main.yx"),
        );

        let result = resolver.resolve("nonexistent_module");
        assert!(result.is_err());
        if let Err(ModuleError::NotFound { path, .. }) = result {
            assert_eq!(path, "nonexistent_module");
        }
    }
}
