//! `yaoxiang init` command - Initialize a new YaoXiang project

use std::fs;
use std::path::Path;

use crate::package::error::{PackageError, PackageResult};
use crate::package::lock::LockFile;
use crate::package::manifest::PackageManifest;
use crate::package::template::{generate_gitignore, generate_main_yx, generate_lib_yx};
use crate::util::i18n::{t, current_lang, MSG};

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
/// │   └── std/           ← 标准库接口文件（LSP 跳转用）
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
    let std_dir = project_dir.join(".yaoxiang").join("std");
    if let Err(e) = crate::std::gen_interfaces::write_interfaces_to_dir(&std_dir) {
        eprintln!("Warning: failed to generate std interface files: {}", e);
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
    println!("  {}/.yaoxiang/std/", name);

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

/// Initialize a new YaoXiang project in the current directory.
///
/// The project name is taken from the current directory name.
/// If yaoxiang.toml already exists, returns PackageError::ProjectExists.
/// Existing template files (src/main.yx, .gitignore, etc.) are skipped
/// rather than overwritten.
pub fn exec_here(options: &InitOptions) -> PackageResult<()> {
    let cwd = std::env::current_dir()?;
    let project_name = cwd.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Cannot determine current directory name",
        )
    })?;

    // Check if project already exists
    if cwd.join("yaoxiang.toml").exists() {
        return Err(PackageError::ProjectExists(cwd.clone()));
    }

    // Create subdirectories (project root already exists)
    fs::create_dir_all(cwd.join("src"))?;
    fs::create_dir_all(cwd.join("tests"))?;

    // Generate yaoxiang.toml (skip if exists — already checked above)
    let manifest = PackageManifest::new(project_name);
    manifest.save(&cwd)?;

    // Generate yaoxiang.lock (skip if exists)
    let lock_path = cwd.join("yaoxiang.lock");
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
        lock.save(&cwd)?;
    }

    // Generate source file (skip if exists)
    if options.lib {
        let lib_path = cwd.join("src").join("lib.yx");
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
        let main_path = cwd.join("src").join("main.yx");
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
    let gitignore_path = cwd.join(".gitignore");
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
    let std_dir = cwd.join(".yaoxiang").join("std");
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
    println!("  .yaoxiang/std/");

    Ok(())
}
