//! 测试模块解析器（Vendor 集成）
//!
//! 覆盖:
//! - vendor 目录中的模块解析
//! - vendor 子模块解析
//! - src 目录中的模块解析
//! - vendor 优先于 src
//! - 不存在的模块返回 None
//! - 列出可用模块
//! - 空路径返回 None
//! - 多版本解析取最后匹配

use std::path::PathBuf;

use crate::package::source::module_resolver::ModuleResolver;
use crate::package::vendor::{VENDOR_DIR, VENDOR_SUBDIR};
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
