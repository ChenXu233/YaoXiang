//! `yaoxiang gen-std` command - Generate standard library interface files (RFC-037)

use std::path::{Path, PathBuf};

use crate::package::error::PackageResult;
use crate::package::vendor::{VENDOR_DIR, VENDOR_SUBDIR};
use crate::util::i18n::{t, current_lang, MSG};

/// Generate standard library interface files at the given project directory
///
/// 输出目录缺省 `<project>/.yaoxiang/vendor/std`（RFC-037 查找链第一级）；
/// 发行包打包传 `--out-dir <pkg>/lib/yaoxiang/std`。native 模块输出签名
/// 接口视图（实现在二进制内），供 LSP 跳转/补全与用户阅读。
pub fn exec_in(
    project_dir: &Path,
    out_dir: Option<PathBuf>,
) -> PackageResult<()> {
    let dir =
        out_dir.unwrap_or_else(|| project_dir.join(VENDOR_DIR).join(VENDOR_SUBDIR).join("std"));
    let count = crate::std::gen_interfaces::write_interfaces_to_dir(&dir)?;

    println!(
        "{}",
        t(
            MSG::PackageGenStdDone,
            current_lang(),
            Some(&[&count.to_string(), &dir.to_string_lossy().to_string()]),
        )
    );
    Ok(())
}

/// Generate standard library interface files in the current project
pub fn exec(out_dir: Option<PathBuf>) -> PackageResult<()> {
    exec_in(&std::env::current_dir()?, out_dir)
}
