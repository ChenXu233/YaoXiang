//! Generate default main.yx template

/// Generate the default `main.yx` file content for a new project.
pub fn generate_main_yx(project_name: &str) -> String {
    format!(
        r#"// {project_name} - YaoXiang 项目
// 由 yaoxiang init 自动生成

main = {{
    print("你好，{project_name}！")
}}
"#,
        project_name = project_name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_main_yx_contains_project_name() {
        let content = generate_main_yx("my-project");
        assert!(content.contains("my-project"));
    }

    #[test]
    fn test_generate_main_yx_contains_hello() {
        let content = generate_main_yx("test");
        assert!(content.contains("你好"));
    }

    #[test]
    fn test_generate_main_yx_contains_main_fn() {
        let content = generate_main_yx("test");
        // YaoXiang 使用 `main = {...}` 语法而非 `fn main() {}`
        assert!(content.contains("main ="));
    }

    #[test]
    fn test_generate_main_yx_contains_print() {
        let content = generate_main_yx("test");
        assert!(content.contains("print("));
    }
}
