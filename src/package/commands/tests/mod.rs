//! Package commands 测试模块

mod add;
mod gen_std;
mod init;
mod install;
mod list;
mod rm;
mod update;

use crate::package::manifest::PackageManifest;
use std::path::Path;

/// 写入/覆盖一个本地路径依赖（`git`/`path` 是当前唯一可安装的来源）。
///
/// 注册表来源（仅版本号）在 RFC-014a 之前一律失败，故用路径依赖构造可安装场景。
pub(crate) fn add_path_dep(
    project_dir: &Path,
    name: &str,
    version: &str,
    dev: bool,
) {
    let dep_dir = project_dir.join(format!("{name}-src"));
    std::fs::create_dir_all(&dep_dir).unwrap();
    std::fs::write(dep_dir.join("lib.yx"), "value: Int = 42\n").unwrap();

    let mut manifest = PackageManifest::load(project_dir).unwrap();
    let mut table = toml::map::Map::new();
    table.insert(
        "version".to_string(),
        toml::Value::String(version.to_string()),
    );
    table.insert(
        "path".to_string(),
        toml::Value::String(format!("./{name}-src")),
    );
    let target = if dev {
        &mut manifest.dev_dependencies
    } else {
        &mut manifest.dependencies
    };
    target.insert(name.to_string(), toml::Value::Table(table));
    manifest.save(project_dir).unwrap();
}
