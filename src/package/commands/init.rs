//! `yaoxiang init` command - Initialize a new YaoXiang project

use std::fs;
use std::path::Path;

use crate::package::error::{PackageError, PackageResult};
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;
use crate::package::template::{generate_gitignore, generate_lib_yx, generate_main_yx};
use crate::util::i18n::{current_lang, t, MSG};

/// Options for project initialization
pub struct InitOptions {
    /// Create a library project instead of a binary project
    pub lib: bool,
}

/// Initialize a new YaoXiang project at the given base directory
///
/// Creates the following structure:
/// ```text
/// <base>/<name>/
/// ├── yaoxiang.toml
/// ├── yaoxiang.lock
/// ├── .gitignore
/// ├── tests/
/// ├── .yaoxiang/
/// │   └── vendor/std/    ← 标准库接口文件（LSP 查找链第一级，RFC-037）
/// └── src/
///     └── main.yx  (or lib.yx if --lib)
/// ```
pub fn exec_in(
    base: &Path,
    options: &InitOptions,
    name: &str,
) -> PackageResult<()> {
    let project_dir = base.join(name);

    // Check if directory already exists
    if project_dir.exists() {
        return Err(PackageError::ProjectExists(project_dir.clone()));
    }

    // Create project directory structure
    fs::create_dir_all(project_dir.join("src"))?;

    // Generate yaoxiang.toml
    let manifest = PackageManifest::new(name);
    manifest.save(&project_dir)?;

    // Generate yaoxiang.lock
    let lock = LockFile::new();
    lock.save(&project_dir)?;

    // Generate source file (main.yx or lib.yx)
    if options.lib {
        let lib_content = generate_lib_yx(name);
        fs::write(project_dir.join("src").join("lib.yx"), lib_content)?;
    } else {
        let main_content = generate_main_yx(name);
        fs::write(project_dir.join("src").join("main.yx"), main_content)?;
    }

    // Create tests directory
    fs::create_dir_all(project_dir.join("tests"))?;

    // Generate .gitignore
    let gitignore_content = generate_gitignore();
    fs::write(project_dir.join(".gitignore"), gitignore_content)?;

    // Generate standard library interface files for LSP
    // RFC-037：统一写入 .yaoxiang/vendor/std/（find_std_interface_file 查找链第一级）
    let std_dir = project_dir.join(".yaoxiang").join("vendor").join("std");
    if let Err(e) = crate::std::gen_interfaces::write_interfaces_to_dir(&std_dir) {
        eprintln!("Warning: failed to generate std interface files: {}", e);
    }

    // RFC-014c：项目落在工作空间内时自动登记为成员（Cargo 式开箱体验；
    // key = [package].name，登记失败即报错——重名时用户改名重建）
    if let Some(ws_root) = crate::package::workspace::find_workspace_root(&project_dir) {
        let key = crate::package::workspace::register_member(&ws_root, &project_dir, None)?;
        println!(
            "{}",
            t(
                MSG::WorkspaceAutoRegistered,
                current_lang(),
                Some(&[&ws_root.display().to_string(), &key])
            )
        );
    }

    let lang = current_lang();
    if options.lib {
        println!(
            "{}",
            t(
                MSG::PackageProjectCreatedLib,
                lang,
                Some(&[&name.to_string()])
            )
        );
        println!("  {}/src/lib.yx", name);
    } else {
        println!(
            "{}",
            t(MSG::PackageProjectCreated, lang, Some(&[&name.to_string()]))
        );
        println!("  {}/src/main.yx", name);
    }
    println!("  {}/yaoxiang.toml", name);
    println!("  {}/yaoxiang.lock", name);
    println!("  {}/.gitignore", name);
    println!("  {}/tests/", name);
    println!("  {}/.yaoxiang/vendor/std/", name);

    Ok(())
}

/// Initialize a new YaoXiang project in the current directory,
/// creating a subdirectory with the given name.
pub fn exec(
    options: &InitOptions,
    name: &str,
) -> PackageResult<()> {
    exec_in(&std::env::current_dir()?, options, name)
}

/// Initialize a new YaoXiang project in the **current directory**（CLI 入口）。
///
/// 项目名取当前目录名；`yaoxiang.toml` 已存在 ⇒ `PackageError::ProjectExists`；
/// 已存在的模板文件（`src/main.yx`、`.gitignore` 等）跳过而不覆盖。
///
/// 行为等价于 [`exec_here_in`]`(&std::env::current_dir()?, options)`——把「在哪个目录」
/// 变成参数之后，测试不再需要改进程级 cwd（`std::env::set_current_dir` 是全进程共享的，
/// 并行测试下会把同进程其它线程的 `current_dir()` 指到临时目录）。
pub fn exec_here(options: &InitOptions) -> PackageResult<()> {
    exec_here_in(&std::env::current_dir()?, options)
}

/// Initialize a new YaoXiang project **in `dir`**（项目根目录已存在）。
///
/// `dir` 即项目根：项目名取它的目录名；产物写进它（`yaoxiang.toml` / `yaoxiang.lock` /
/// `src/{main,lib}.yx` / `tests/` / `.gitignore` / `.yaoxiang/vendor/std/`）。
/// 不进工作空间登记以外的地方——工作空间发现同样以 `dir` 为根。
///
/// 可见性 `pub(crate)`：CLI 只经 [`exec_here`] 调用它，测试直接给临时目录，
/// 不新增对外 API。
pub(crate) fn exec_here_in(
    dir: &Path,
    options: &InitOptions,
) -> PackageResult<()> {
    // 报错文面与 CLI 逐字一致（零行为：`exec_here` 只是把 cwd 递进来）
    let project_name = dir.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Cannot determine current directory name",
        )
    })?;

    // Check if project already exists
    if dir.join("yaoxiang.toml").exists() {
        return Err(PackageError::ProjectExists(dir.to_path_buf()));
    }

    // Create subdirectories (project root already exists)
    fs::create_dir_all(dir.join("src"))?;
    fs::create_dir_all(dir.join("tests"))?;

    // Generate yaoxiang.toml (skip if exists — already checked above)
    let manifest = PackageManifest::new(project_name);
    manifest.save(dir)?;

    // Generate yaoxiang.lock (skip if exists)
    let lock_path = dir.join("yaoxiang.lock");
    if lock_path.exists() {
        let lang = current_lang();
        println!(
            "{}",
            t(
                MSG::PackageFileSkipped,
                lang,
                Some(&[&"yaoxiang.lock".to_string()])
            )
        );
    } else {
        let lock = LockFile::new();
        lock.save(dir)?;
    }

    // Generate source file (skip if exists)
    if options.lib {
        let lib_path = dir.join("src").join("lib.yx");
        if lib_path.exists() {
            let lang = current_lang();
            println!(
                "{}",
                t(
                    MSG::PackageFileSkipped,
                    lang,
                    Some(&[&"src/lib.yx".to_string()])
                )
            );
        } else {
            let lib_content = generate_lib_yx(project_name);
            fs::write(&lib_path, lib_content)?;
        }
    } else {
        let main_path = dir.join("src").join("main.yx");
        if main_path.exists() {
            let lang = current_lang();
            println!(
                "{}",
                t(
                    MSG::PackageFileSkipped,
                    lang,
                    Some(&[&"src/main.yx".to_string()])
                )
            );
        } else {
            let main_content = generate_main_yx(project_name);
            fs::write(&main_path, main_content)?;
        }
    }

    // Generate .gitignore (skip if exists)
    let gitignore_path = dir.join(".gitignore");
    if gitignore_path.exists() {
        let lang = current_lang();
        println!(
            "{}",
            t(
                MSG::PackageFileSkipped,
                lang,
                Some(&[&".gitignore".to_string()])
            )
        );
    } else {
        let gitignore_content = generate_gitignore();
        fs::write(&gitignore_path, gitignore_content)?;
    }

    // Generate standard library interface files for LSP
    // RFC-037：统一写入 .yaoxiang/vendor/std/（find_std_interface_file 查找链第一级）
    let std_dir = dir.join(".yaoxiang").join("vendor").join("std");
    if let Err(e) = crate::std::gen_interfaces::write_interfaces_to_dir(&std_dir) {
        eprintln!("Warning: failed to generate std interface files: {}", e);
    }

    let lang = current_lang();
    println!(
        "{}",
        t(
            MSG::PackageInitHere,
            lang,
            Some(&[&project_name.to_string()])
        )
    );
    if options.lib {
        println!("  src/lib.yx");
    } else {
        println!("  src/main.yx");
    }
    println!("  yaoxiang.toml");
    println!("  yaoxiang.lock");
    println!("  .gitignore");
    println!("  tests/");
    println!("  .yaoxiang/vendor/std/");

    // RFC-014c：工作空间内 init（exec_here_in）同样自动登记（key = [package].name）
    if let Some(ws_root) = crate::package::workspace::find_workspace_root(dir) {
        if ws_root.as_path() != dir {
            let key = crate::package::workspace::register_member(&ws_root, dir, None)?;
            println!(
                "{}",
                t(
                    MSG::WorkspaceAutoRegistered,
                    current_lang(),
                    Some(&[&ws_root.display().to_string(), &key])
                )
            );
        }
    }

    Ok(())
}
