//! Generate default .gitignore template

/// Generate the default `.gitignore` content for a new project.
pub fn generate_gitignore() -> &'static str {
    r#"# YaoXiang 构建产物
.yaoxiang/
*.42

# IDE 文件
.vscode/
.idea/
*.swp
*.swo
*~

# OS 文件
.DS_Store
Thumbs.db
"#
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
