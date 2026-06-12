//! 测试 `.gitignore` 模板生成
//!
//! 覆盖:
//! - 包含 `.yaoxiang/` 目录忽略
//! - 包含字节码文件忽略 (`*.42`)
//! - 包含 IDE 文件忽略

use crate::package::template::gitignore::generate_gitignore;

#[test]
fn test_gitignore_contains_yaoxiang_dir() {
    let content = generate_gitignore();
    assert!(content.contains(".yaoxiang/"));
}

#[test]
fn test_gitignore_contains_bytecode() {
    let content = generate_gitignore();
    assert!(content.contains("*.42"));
}

#[test]
fn test_gitignore_contains_ide_files() {
    let content = generate_gitignore();
    assert!(content.contains(".vscode/"));
    assert!(content.contains(".idea/"));
}
