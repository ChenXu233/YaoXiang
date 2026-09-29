//! `yaoxiang workspace` command - Workspace management (RFC-014c Phase 6a)

use crate::package::error::PackageResult;
use crate::package::workspace::{find_workspace_root, load_workspace};
use crate::util::i18n::{t, current_lang, MSG};

/// `yaoxiang workspace list`：列出工作空间成员
///
/// 从当前目录向上探测工作空间根；成员按 `[workspace.members]` key 序输出
/// （key、[package].version、toml 路径）。
pub fn list() -> PackageResult<()> {
    let cwd = std::env::current_dir()?;
    let root =
        find_workspace_root(&cwd).ok_or(crate::package::error::PackageError::NotWorkspace)?;
    let ws = load_workspace(&root)?;

    let lang = current_lang();
    println!(
        "{}",
        t(
            MSG::WorkspaceListHeader,
            lang,
            Some(&[
                &ws.root.display().to_string(),
                &ws.members.len().to_string()
            ])
        )
    );
    for member in &ws.members {
        let toml_path = ws
            .manifest
            .workspace
            .members
            .get(&member.name)
            .cloned()
            .unwrap_or_default();
        println!(
            "{}",
            t(
                MSG::WorkspaceListMemberRow,
                lang,
                Some(&[&member.name, &member.manifest.package.version, &toml_path])
            )
        );
    }

    Ok(())
}
