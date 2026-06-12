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
