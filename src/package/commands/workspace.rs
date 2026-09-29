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

/// `yaoxiang workspace add <path>`：把已有包目录登记为成员
///
/// key 取成员 `[package].name`，`--as` 可覆盖（RFC-014c：key 由工作空间
/// 控制、稳定唯一）。
pub fn add(
    path: &str,
    key_override: Option<&str>,
) -> PackageResult<()> {
    let cwd = std::env::current_dir()?;
    let ws_root =
        find_workspace_root(&cwd).ok_or(crate::package::error::PackageError::NotWorkspace)?;
    let member_dir = cwd.join(path);

    let key = crate::package::workspace::register_member(&ws_root, &member_dir, key_override)?;

    let lang = current_lang();
    println!(
        "{}",
        t(
            MSG::WorkspaceMemberAdded,
            lang,
            Some(&[&key, &member_dir.display().to_string()])
        )
    );
    Ok(())
}

/// `yaoxiang workspace remove <key>`：移除成员登记
///
/// 仅摘除根 toml 里的登记项，不删目录；有其他成员仍以
/// `{ workspace = "<key>" }` 引用它时在 stderr 提示（引用会在下次
/// 校验时报错，由用户决定先删引用还是保留）。
pub fn remove(key: &str) -> PackageResult<()> {
    let cwd = std::env::current_dir()?;
    let ws_root =
        find_workspace_root(&cwd).ok_or(crate::package::error::PackageError::NotWorkspace)?;

    let ws = load_workspace(&ws_root)?;
    if !ws.manifest.workspace.members.contains_key(key) {
        return Err(crate::package::error::PackageError::InvalidManifest(
            format!("工作空间没有成员 '{key}'"),
        ));
    }

    // 移除前扫描其余成员的 workspace 引用
    let mut referencers: Vec<String> = Vec::new();
    for member in &ws.members {
        if member.name == key {
            continue;
        }
        let refs = member
            .manifest
            .dependencies
            .values()
            .chain(member.manifest.dev_dependencies.values())
            .filter_map(|v| v.get("workspace").and_then(|w| w.as_str()))
            .any(|w| w == key);
        if refs {
            referencers.push(member.name.clone());
        }
    }

    let mut manifest = crate::package::workspace::load_workspace_manifest(&ws_root)?;
    manifest.workspace.members.remove(key);
    crate::package::workspace::save_workspace_manifest(&ws_root, &manifest)?;

    let lang = current_lang();
    println!(
        "{}",
        t(MSG::WorkspaceMemberRemoved, lang, Some(&[&key.to_string()]))
    );
    if !referencers.is_empty() {
        eprintln!(
            "{}",
            t(
                MSG::WorkspaceMemberReferenced,
                lang,
                Some(&[&key, &referencers.join(", ")])
            )
        );
    }
    Ok(())
}
