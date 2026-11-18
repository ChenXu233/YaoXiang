//! 系统 C 工具链与原子产物替换。这里不负责语言代码生成。

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{anyhow, bail, Context, Result};
use tempfile::{NamedTempFile, TempDir};

const RUNTIME_C: &str = include_str!("runtime.c");

pub(super) fn link_executable(
    object: &Path,
    output: &Path,
    linker: &OsStr,
) -> Result<()> {
    let work = TempDir::new().context("failed to create native build scratch directory")?;
    let runtime_c = work.path().join("yx_runtime.c");
    let runtime_o = work.path().join("yx_runtime.o");
    fs::write(&runtime_c, RUNTIME_C).context("failed to write native runtime source")?;

    run_tool(
        Command::new(linker)
            .arg("-std=c11")
            .arg("-O2")
            .arg("-fPIC")
            .arg("-c")
            .arg(&runtime_c)
            .arg("-o")
            .arg(&runtime_o),
        linker,
    )?;

    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temporary = NamedTempFile::new_in(parent).with_context(|| {
        format!(
            "failed to create temporary executable next to {}",
            output.display()
        )
    })?;
    let temporary_path = temporary.into_temp_path();
    fs::remove_file(&temporary_path).with_context(|| {
        format!(
            "failed to prepare temporary output {}",
            temporary_path.display()
        )
    })?;

    run_tool(
        Command::new(linker)
            .arg(object)
            .arg(&runtime_o)
            .arg("-o")
            .arg(&temporary_path),
        linker,
    )?;
    temporary_path.persist(output).map_err(|e| {
        anyhow!(
            "failed to replace native output {} with completed link result: {}",
            output.display(),
            e.error
        )
    })?;
    Ok(())
}

fn run_tool(
    command: &mut Command,
    program: &OsStr,
) -> Result<Output> {
    let printable = PathBuf::from(program).display().to_string();
    let output = command.output().map_err(|e| {
        anyhow!(
            "failed to start native toolchain driver {printable:?}: {e}; install a C compiler or set YAOXIANG_CC"
        )
    })?;
    if !output.status.success() {
        bail!(
            "native toolchain driver {printable:?} failed with {}:\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output)
}
