//! 标准库接口文件生成器
//!
//! 从 `StdModule` trait 的 `exports()` 自动生成 `.yx` 接口文件。
//! 用于 LSP 跳转定义和补全功能。

use crate::std::StdModule;

/// 为单个模块生成 `.yx` 接口文件内容
fn generate_interface_content(module: &dyn StdModule) -> String {
    let mut output = String::new();
    let module_path = module.module_path();

    // 文件头注释
    output.push_str(&format!(
        "// {}.yx - 标准库 {} 模块接口\n",
        module_path.split('.').next_back().unwrap_or(module_path),
        module_path
    ));
    output.push_str("// 仅供 LSP 跳转和类型查看，不参与实际执行\n");
    output.push('\n');

    for export in module.exports() {
        let sig = export.signature;
        let name = export.name;

        // 常量（签名不以 '(' 开头）
        if !sig.starts_with('(') {
            output.push_str(&format!("{}: {} = {{\n    ...\n}}\n\n", name, sig));
        } else {
            // 函数：name: signature = { ... }
            output.push_str(&format!("{}: {} = {{\n    ...\n}}\n\n", name, sig));
        }
    }

    output
}

/// 为所有标准库模块生成接口文件内容
///
/// 返回 `(module_name, content)` 列表
pub fn generate_all_interfaces() -> Vec<(String, String)> {
    let modules: Vec<Box<dyn StdModule>> = vec![
        Box::new(crate::std::convert::ConvertModule),
        Box::new(crate::std::dict::DictModule),
        Box::new(crate::std::io::IoModule),
        Box::new(crate::std::list::ListModule),
        Box::new(crate::std::math::MathModule),
        Box::new(crate::std::net::NetModule),
        Box::new(crate::std::concurrent::ConcurrentModule),
        Box::new(crate::std::string::StringModule),
        Box::new(crate::std::time::TimeModule),
        Box::new(crate::std::os::OsModule),
    ];

    modules
        .iter()
        .map(|m| {
            let name = m
                .module_path()
                .strip_prefix("std.")
                .unwrap_or(m.module_path())
                .to_string();
            let content = generate_interface_content(m.as_ref());
            (name, content)
        })
        .collect()
}

/// 将接口文件写入指定目录
///
/// `target_dir` 是接口文件的输出目录（如 `~/.yaoxiang/std/`）
pub fn write_interfaces_to_dir(target_dir: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target_dir)?;

    for (name, content) in generate_all_interfaces() {
        let file_path = target_dir.join(format!("{}.yx", name));
        std::fs::write(&file_path, content)?;
    }

    Ok(())
}

/// 获取标准库接口文件的默认安装目录
///
/// 查找顺序：
/// 1. 项目目录/.yaoxiang/vendor/std/（项目本地）
/// 2. ~/.yaoxiang/std/（全局）
pub fn default_std_interface_dir() -> Option<std::path::PathBuf> {
    // 跨平台获取 home 目录
    #[cfg(target_os = "windows")]
    let home = std::env::var("USERPROFILE").ok();
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var("HOME").ok();

    home.map(|h| std::path::PathBuf::from(h).join(".yaoxiang").join("std"))
}

/// 查找标准库接口文件
///
/// 按优先级查找：
/// 1. 项目目录/.yaoxiang/vendor/std/`<name>`.yx
/// 2. ~/.yaoxiang/std/`<name>`.yx
pub fn find_std_interface_file(
    project_dir: Option<&std::path::Path>,
    module_name: &str,
) -> Option<std::path::PathBuf> {
    let file_name = format!("{}.yx", module_name);

    // 1. 项目本地覆盖
    if let Some(proj) = project_dir {
        let local = proj
            .join(".yaoxiang")
            .join("vendor")
            .join("std")
            .join(&file_name);
        if local.exists() {
            return Some(local);
        }
    }

    // 2. 全局回退
    if let Some(global_dir) = default_std_interface_dir() {
        let global = global_dir.join(&file_name);
        if global.exists() {
            return Some(global);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_all_interfaces() {
        let interfaces = generate_all_interfaces();
        assert!(!interfaces.is_empty(), "应至少生成一个接口文件");

        // 检查包含关键模块
        let names: Vec<&str> = interfaces.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"io"), "应包含 io 模块");
        assert!(names.contains(&"list"), "应包含 list 模块");
        assert!(names.contains(&"math"), "应包含 math 模块");
        assert!(names.contains(&"dict"), "应包含 dict 模块");
        assert!(names.contains(&"string"), "应包含 string 模块");
    }

    #[test]
    fn test_io_interface_content() {
        let interfaces = generate_all_interfaces();
        let io = interfaces.iter().find(|(n, _)| n == "io").unwrap();
        let content = &io.1;

        assert!(content.contains("print:"), "io 接口应包含 print");
        assert!(content.contains("println:"), "io 接口应包含 println");
        assert!(content.contains("read_line:"), "io 接口应包含 read_line");
        assert!(content.contains("read_file:"), "io 接口应包含 read_file");
        assert!(content.contains("..."), "接口函数体应包含 ...");
    }

    #[test]
    fn test_math_interface_has_constants() {
        let interfaces = generate_all_interfaces();
        let math = interfaces.iter().find(|(n, _)| n == "math").unwrap();
        let content = &math.1;

        assert!(content.contains("PI:"), "math 接口应包含 PI 常量");
        assert!(content.contains("E:"), "math 接口应包含 E 常量");
    }

    #[test]
    fn test_list_interface_content() {
        let interfaces = generate_all_interfaces();
        let list = interfaces.iter().find(|(n, _)| n == "list").unwrap();
        let content = &list.1;

        assert!(content.contains("push:"), "list 接口应包含 push");
        assert!(content.contains("pop:"), "list 接口应包含 pop");
        assert!(content.contains("map:"), "list 接口应包含 map");
        assert!(content.contains("filter:"), "list 接口应包含 filter");
    }

    #[test]
    fn test_write_interfaces_to_temp_dir() {
        let temp_dir = std::env::temp_dir().join("yaoxiang_test_interfaces");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let result = write_interfaces_to_dir(&temp_dir);
        assert!(result.is_ok(), "写入接口文件应成功");

        // 验证文件存在
        assert!(temp_dir.join("io.yx").exists());
        assert!(temp_dir.join("list.yx").exists());
        assert!(temp_dir.join("math.yx").exists());

        // 清理
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_find_std_interface_file() {
        // 不指定项目目录，且全局目录可能不存在 → 返回 None
        let result = find_std_interface_file(None, "nonexistent_module");
        assert!(result.is_none());
    }
}
