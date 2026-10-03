//! 测试 Git 来源的 URL 解析和来源信息
//!
//! 覆盖:
//! - 基本 Git URL 解析
//! - 带 tag 参数的 URL 解析
//! - 带 branch 参数的 URL 解析
//! - 带 rev 参数的 URL 解析
//! - GitSource 的 name 和 kind
//! - ls-remote 失败的显式报错语义（RFC-014 Phase 3：不得静默降级）
//! - git 子进程剥离仓库定位环境变量（结构性 + 行为级）
//! - 全仓只有 `git_command` 可以构造 git 子进程（防回归）
//! - 夹具建仓守卫：已有仓库拒绝 reinit（`force` 才放行）、建仓必须落在目标目录
//!
//! 进程纪律部分的依据：`git help hooks`（钩子会继承 `GIT_DIR` / `GIT_INDEX_FILE`）
//! 与测试规范原则 4「测试行为，不测试实现」——因此除环境变量清单外，另有
//! 一条真正继承 `GIT_DIR` 的行为级用例。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::package::source::git::{
    GitRef, GitSource, git_command, guard_repo_absent, guard_repo_created,
};
use crate::package::source::{Source, SourceKind};

#[test]
fn test_parse_git_url_basic() {
    let (url, git_ref) = GitSource::parse_git_url("https://github.com/user/repo");
    assert_eq!(url, "https://github.com/user/repo");
    assert_eq!(git_ref, GitRef::DefaultBranch);
}

#[test]
fn test_parse_git_url_tag() {
    let (url, git_ref) = GitSource::parse_git_url("https://github.com/user/repo?tag=v1.0.0");
    assert_eq!(url, "https://github.com/user/repo");
    assert_eq!(git_ref, GitRef::Tag("v1.0.0".to_string()));
}

#[test]
fn test_parse_git_url_branch() {
    let (url, git_ref) = GitSource::parse_git_url("https://github.com/user/repo?branch=dev");
    assert_eq!(url, "https://github.com/user/repo");
    assert_eq!(git_ref, GitRef::Branch("dev".to_string()));
}

#[test]
fn test_parse_git_url_rev() {
    let (url, git_ref) = GitSource::parse_git_url("https://github.com/user/repo?rev=abc123");
    assert_eq!(url, "https://github.com/user/repo");
    assert_eq!(git_ref, GitRef::Rev("abc123".to_string()));
}

#[test]
fn test_git_source_name() {
    let source = GitSource::new();
    assert_eq!(source.name(), "git");
    assert_eq!(source.kind(), SourceKind::Git);
}

#[test]
fn test_list_tags_unreachable_url_errors_loudly() {
    // Arrange：不存在的本地仓库路径——git ls-remote 必然失败
    let source = GitSource::new();

    // Act
    let result = source.list_tags("/nonexistent/yx-probe/repo");

    // Assert：失败必须显式报错。静默返回空列表会让 semver 择优落空、
    // 回退到默认分支，把钉住的版本请求静默解析成 HEAD 的内容
    //（CI 上曾因此把 "1.0.0" 解析成 v1.1.0 的 manifest）
    assert!(
        result.is_err(),
        "ls-remote 失败应显式报错，实际: {result:?}"
    );
}

#[test]
fn test_git_command_scrubs_repo_location_env() {
    // Arrange：模拟 `git commit` 钩子环境注入的仓库定位变量——
    // 子 git 继承后会把 fixture 的 init/clone 重定向到宿主仓库
    let repo_location_vars = [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_COMMON_DIR",
        "GIT_NAMESPACE",
        "GIT_CONFIG_PARAMETERS",
    ];

    // Act
    let cmd = git_command();

    // Assert：定位类变量全部被显式移除（get_envs 以 (key, None) 呈现）
    for key in repo_location_vars {
        let target = std::ffi::OsStr::new(key);
        let scrubbed = cmd.get_envs().any(|(k, v)| k == target && v.is_none());
        assert!(scrubbed, "{key} 应被 git_command 剥离");
    }
}

// === 防回归守卫：子进程纪律与夹具建仓 ===

/// 探针环境变量：重新拉起的子进程用它自证「本进程真的继承了 GIT_DIR」。
const GIT_DIR_PROBE_ENV: &str = "YX_GIT_DIR_PROBE";
/// 探针环境变量：诱饵仓库路径。
const DECOY_DIR_ENV: &str = "YX_GIT_DIR_DECOY";
/// 探针环境变量：诱饵仓库的建仓指纹清单文件。
const DECOY_MANIFEST_ENV: &str = "YX_GIT_DIR_DECOY_MANIFEST";

#[test]
fn test_git_command_immunizes_child_against_inherited_git_dir() {
    // Arrange：本用例要求**自身进程**真的继承 GIT_DIR。进程内 set_var 会污染
    // 同进程并行运行的其它测试，因此重新拉起本测试可执行文件，只在它的环境里
    // 注入诱饵 GIT_DIR / GIT_INDEX_FILE。
    if std::env::var_os(GIT_DIR_PROBE_ENV).is_none() {
        let tmp = tempfile::TempDir::new().expect("临时目录");
        let decoy = tmp.path().join("decoy");
        make_probe_repo(&decoy);
        let manifest = tmp.path().join("manifest.txt");
        std::fs::write(&manifest, repo_manifest(&decoy)).expect("写入诱饵指纹");

        // Act：重新拉起自身，把诱饵仓库放进子进程的仓库定位变量
        let exe = std::env::current_exe().expect("测试可执行文件路径");
        let output = Command::new(exe)
            .arg("--exact")
            .arg("package::source::tests::git::test_git_command_immunizes_child_against_inherited_git_dir")
            .env(GIT_DIR_PROBE_ENV, "1")
            .env(DECOY_DIR_ENV, &decoy)
            .env(DECOY_MANIFEST_ENV, &manifest)
            .env("GIT_DIR", decoy.join(".git"))
            .env("GIT_INDEX_FILE", decoy.join(".git").join("index"))
            .output()
            .expect("重新拉起测试进程");

        // Assert：探针必须真的跑到了本用例，否则这条测试形同虚设
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("1 passed"),
            "探针进程应通过其 1 个用例；stdout={stdout} stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    // Act：在已经继承 GIT_DIR 的本进程里，用 git_command() 在另一个目录建仓
    let fresh = tempfile::TempDir::new().expect("临时目录");
    let fresh_dir = fresh.path().join("fresh");
    std::fs::create_dir_all(&fresh_dir).expect("创建目标目录");
    let status = git_command()
        .arg("-C")
        .arg(&fresh_dir)
        .args(["init", "-b", "main"])
        .status()
        .expect("git 可用");

    // Assert：仓库必须建在目标目录；GIT_DIR 指向的诱饵仓库必须原封不动
    assert!(status.success(), "git init 应成功");
    assert!(
        fresh_dir.join(".git").is_dir(),
        "仓库应建在目标目录 {}，而不是 GIT_DIR 指向处",
        fresh_dir.display()
    );
    let decoy = PathBuf::from(std::env::var_os(DECOY_DIR_ENV).expect("探针缺少诱饵目录"));
    let manifest = std::fs::read_to_string(
        std::env::var_os(DECOY_MANIFEST_ENV).expect("探针缺少诱饵指纹文件"),
    )
    .expect("读取诱饵指纹");
    assert_eq!(
        repo_manifest(&decoy),
        manifest,
        "继承的 GIT_DIR 指向的仓库不得被改写（core.bare 与全部文件应保持不变）"
    );
}

#[test]
fn test_git_spawn_only_through_git_command() {
    // Arrange：扫描 crate 源码树；模式串在运行时拼装，扫描器才不会命中本文件
    let src_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    let mut factory_hits = 0usize;

    // Act
    collect_git_spawns(&src_root, &src_root, &mut offenders, &mut factory_hits);

    // Assert：工厂自身必须被扫到（否则本用例形同虚设），且不得再有第二处
    assert!(
        factory_hits >= 1,
        "扫描应至少命中工厂自身，实际 {factory_hits} 处"
    );
    assert!(
        offenders.is_empty(),
        "git 子进程必须经 git_command() 构造，以下文件存在裸 spawn: {offenders:?}"
    );
}

#[test]
fn test_guard_repo_absent_rejects_existing_repo_unless_forced() {
    // Arrange：一个已经建好的仓库目录（force=false 是默认，force=true 是出口）
    let tmp = tempfile::TempDir::new().expect("临时目录");
    let dir = tmp.path().join("existing");
    make_probe_repo(&dir);

    // Act
    let refused = guard_repo_absent(&dir, false);
    let forced = guard_repo_absent(&dir, true);

    // Assert
    assert!(
        refused.is_err(),
        "已有仓库必须被拒绝（除非 force=true），实际: {refused:?}"
    );
    assert!(forced.is_ok(), "force=true 应放行，实际: {forced:?}");
}

#[test]
fn test_guard_repo_created_rejects_dir_without_repo() {
    // Arrange：一个还没有仓库的目录
    let tmp = tempfile::TempDir::new().expect("临时目录");
    let dir = tmp.path().join("empty");
    std::fs::create_dir_all(&dir).expect("创建目录");

    // Act
    let before = guard_repo_created(&dir);
    make_probe_repo(&dir);
    let after = guard_repo_created(&dir);

    // Assert
    assert!(before.is_err(), "没有仓库的目录应被拒绝，实际: {before:?}");
    assert!(after.is_ok(), "建仓之后应通过，实际: {after:?}");
}

/// 用 git_command() 在 `dir` 建一个最小仓库（探针与守卫用例共用）。
fn make_probe_repo(dir: &Path) {
    std::fs::create_dir_all(dir).expect("创建仓库目录");
    let run = |args: &[&str]| {
        let status = git_command()
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .expect("git 可用");
        assert!(status.success(), "git {args:?} 应成功");
    };
    run(&["init", "-b", "main"]);
    run(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t.local",
        "commit",
        "--allow-empty",
        "-m",
        "seed",
    ]);
}

/// 递归扫描 `dir` 下的 `.rs`，统计构造 git 子进程的行：
/// `package/source/git.rs`（工厂）计入 `factory_hits`，其余计入 `offenders`。
fn collect_git_spawns(
    root: &Path,
    dir: &Path,
    offenders: &mut Vec<String>,
    factory_hits: &mut usize,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    // 运行时拼装模式串：源码里不出现完整模式，扫描器才不会命中本文件自身
    let ctor = ["Command", "::new("].concat();
    let quoted = ["\"", "git", "\""].concat();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_git_spawns(root, &path, offenders, factory_hits);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let hits = text
            .lines()
            .filter(|line| line.contains(&ctor) && line.contains(&quoted))
            .count();
        if hits == 0 {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        if rel == "package/source/git.rs" {
            *factory_hits += hits;
        } else {
            offenders.push(format!("{rel} ({hits} 处)"));
        }
    }
}

/// 目录清单：所有文件（含仓库元数据）的「相对路径 + 长度 + 内容散列」，按路径排序。
///
/// 用自实现的 FNV-1a 而非 `DefaultHasher`：父子两个进程必须得到完全一致的
/// 结果，跨进程可复现是这里的硬要求。
fn repo_manifest(root: &Path) -> String {
    let mut rows = Vec::new();
    collect_manifest_rows(root, root, &mut rows);
    rows.sort();
    rows.join("\n")
}

fn collect_manifest_rows(
    root: &Path,
    dir: &Path,
    rows: &mut Vec<String>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_manifest_rows(root, &path, rows);
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let rel = path
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        rows.push(format!("{rel}\t{}\t{:016x}", bytes.len(), fnv1a(&bytes)));
    }
}

/// FNV-1a 64 位散列：跨进程、跨运行可复现。
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
