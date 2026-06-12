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
