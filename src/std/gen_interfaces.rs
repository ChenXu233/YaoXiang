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
        #[cfg(not(target_arch = "wasm32"))]
        Box::new(crate::std::net::NetModule),
        #[cfg(not(target_arch = "wasm32"))]
        Box::new(crate::std::concurrent::ConcurrentModule),
        Box::new(crate::std::string::StringModule),
        Box::new(crate::std::time::TimeModule),
        #[cfg(not(target_arch = "wasm32"))]
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

/// 将接口文件写入指定目录，返回写入的文件数
///
/// `target_dir` 是接口文件的输出目录（如发行包 `lib/yaoxiang/std/`）
pub fn write_interfaces_to_dir(target_dir: &std::path::Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(target_dir)?;

    let interfaces = generate_all_interfaces();
    let count = interfaces.len();
    for (name, content) in interfaces {
        let file_path = target_dir.join(format!("{}.yx", name));
        std::fs::write(&file_path, content)?;
    }

    Ok(count)
}

/// 获取 YaoXiang 安装根（RFC-037）
///
/// `YAOXIANG_HOME` 环境变量优先（服务 CI 与容器场景），缺省 `~/.yaoxiang`
/// （Windows 为 `%USERPROFILE%\.yaoxiang`）。与 yx 前门共用同一约定。
fn yaoxiang_home() -> Option<std::path::PathBuf> {
    if let Ok(home) = std::env::var("YAOXIANG_HOME") {
        if !home.is_empty() {
            return Some(std::path::PathBuf::from(home));
        }
    }

    #[cfg(target_os = "windows")]
    let home = std::env::var("USERPROFILE").ok();
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var("HOME").ok();

    home.map(|h| std::path::PathBuf::from(h).join(".yaoxiang"))
}

/// 全局标准库接口目录（`<YAOXIANG_HOME>/std/`，缺省 `~/.yaoxiang/std/`），
/// 保留为手工覆盖位
pub fn default_std_interface_dir() -> Option<std::path::PathBuf> {
    yaoxiang_home().map(|home| home.join("std"))
}

/// 查找标准库接口文件
///
/// 按优先级查找（RFC-037 三级链）：
/// 1. 项目目录/.yaoxiang/vendor/std/`<name>`.yx（项目覆盖）
/// 2. exe 相对 ../lib/yaoxiang/std/`<name>`.yx（发行包内置；便携解压、
///    `~/.yaoxiang/versions/<ver>/`、deb 平装三种渠道同构命中）
/// 3. `~/.yaoxiang/std/`<name>`.yx（全局回退）
pub fn find_std_interface_file(
    project_dir: Option<&std::path::Path>,
    module_name: &str,
) -> Option<std::path::PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.to_path_buf()));
    find_std_interface_file_in(
        project_dir,
        exe_dir.as_deref(),
        default_std_interface_dir().as_deref(),
        module_name,
    )
}

/// `find_std_interface_file` 的纯函数形态（exe 目录与全局目录均可注入，供测试）
pub(crate) fn find_std_interface_file_in(
    project_dir: Option<&std::path::Path>,
    exe_dir: Option<&std::path::Path>,
    global_dir: Option<&std::path::Path>,
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

    // 2. 发行包 exe 相对：引擎在 bin/，std 在 ../lib/yaoxiang/std/
    if let Some(bin_dir) = exe_dir {
        if let Some(pkg_root) = bin_dir.parent() {
            let bundled = pkg_root
                .join("lib")
                .join("yaoxiang")
                .join("std")
                .join(&file_name);
            if bundled.exists() {
                return Some(bundled);
            }
        }
    }

    // 3. 全局回退
    if let Some(global) = global_dir {
        let fallback = global.join(&file_name);
        if fallback.exists() {
            return Some(fallback);
        }
    }

    None
}
