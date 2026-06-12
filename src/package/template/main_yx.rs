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
