//! custom 构建策略——build.yx 执行 + 信任门（RFC-014b Phase 5e）
//!
//! 信任门（2026-09-15 决议 1，不可省略的下限）：`custom` 执行任意代码是包
//! 管理器最大的供应链攻击面。
//! - 记录在案（用户配置 `[trust] build-scripts`）或本次 `--trust` → 放行
//! - 交互环境：首次执行前确认，确认即持久化（下次不再问）
//! - 非交互环境（CI）：默认拒绝，除非显式 `--trust`
//!
//! 执行模型（最小规范）：build.yx 是普通 .yx 脚本（顶层语句即构建逻辑，
//! 同 Script/eval 语义），经 [`crate::run_file`] 进程内执行；工作目录临时
//! 切到包根（恢复）；编译/运行错误即构建失败。退出码语义：干净执行 = 成功
//! （当前 std 无 exit/进程 API，脚本能力随 std 演进）。

use std::path::Path;

use crate::package::build::TrustDecision;
use crate::package::error::{PackageError, PackageResult};
use crate::package::manifest::PackageManifest;
use crate::util::config;

use super::BuildOutcome;

/// build.yx 固定文件名（RFC-014b）
pub(crate) const BUILD_SCRIPT: &str = "build.yx";

/// 执行 custom 构建（信任门 → 进程内执行）
///
/// `confirm` 注入交互确认（CLI 接 stdin；测试注入闭包），参数为确认文案。
pub(crate) fn build(
    pkg_dir: &Path,
    trust: &TrustDecision,
    confirm: &dyn Fn(&str) -> bool,
) -> PackageResult<BuildOutcome> {
    let manifest = PackageManifest::load(pkg_dir)?;
    let key = format!("{}@{}", manifest.package.name, manifest.package.version);
    let script = pkg_dir.join(BUILD_SCRIPT);
    if !script.is_file() {
        return Err(PackageError::InvalidManifest(format!(
            "custom 构建策略要求包根存在 {BUILD_SCRIPT}（{key}）"
        )));
    }

    ensure_trusted(&key, trust, confirm)?;
    execute(&script, pkg_dir)?;
    Ok(BuildOutcome {
        native_dir: None,
        via: "custom",
    })
}

/// 信任门：已记录 / 显式 --trust（放行并持久化，RFC 决议 1）/ 交互确认
/// （确认即持久化）/ 其余拒绝
fn ensure_trusted(
    key: &str,
    trust: &TrustDecision,
    confirm: &dyn Fn(&str) -> bool,
) -> PackageResult<()> {
    if trust.flag {
        // RFC：「add/install --trust 把信任记录持久化」——本次放行的包记入
        // 档案，后续不带 --trust 的 install 不再被打断
        record_trust(key, trust)?;
        return Ok(());
    }
    if trusted_in_store(key, trust) {
        return Ok(());
    }

    if !trust.interactive {
        return Err(PackageError::AuthFailed(
            "非交互环境拒绝执行 build.yx（CI 默认拒绝，RFC-014b 决议 1）；\
             确认信任请显式传入 --trust"
                .to_string(),
        ));
    }

    let question = format!("首次执行 {key} 的 build.yx（任意代码）。信任并继续？[y/N]");
    if !confirm(&question) {
        return Err(PackageError::AuthFailed(format!(
            "已拒绝执行 {key} 的 build.yx"
        )));
    }
    // 确认即持久化——重新安装不再重复打扰
    record_trust(key, trust)
}

fn trusted_in_store(
    key: &str,
    trust: &TrustDecision,
) -> bool {
    match &trust.store {
        Some(path) => config::load_user_config_from(path)
            .map(|c| c.trust.build_scripts.iter().any(|s| s == key))
            .unwrap_or(false),
        None => config::is_trusted_build_script(key),
    }
}

fn record_trust(
    key: &str,
    trust: &TrustDecision,
) -> PackageResult<()> {
    match &trust.store {
        Some(path) => {
            let mut cfg = config::load_user_config_from(path).map_err(config_err)?;
            if !cfg.trust.build_scripts.iter().any(|s| s == key) {
                cfg.trust.build_scripts.push(key.to_string());
            }
            config::save_user_config_to(path, &cfg).map_err(config_err)?;
        }
        None => config::add_trusted_build_script(key)
            .map_err(|e| PackageError::InvalidManifest(format!("信任记录写入失败: {e}")))?,
    }
    Ok(())
}

/// ConfigError → 包层错误
fn config_err(e: config::ConfigError) -> PackageError {
    PackageError::InvalidManifest(format!("信任记录读写失败: {e}"))
}

/// 进程内执行 build.yx；工作目录临时切到包根（无论成败都恢复）
///
/// cwd 翻转是进程全局状态：全局锁串行化翻转窗口，保证"保存的原目录"始终
/// 是稳定cwd（并发构建脚本本来就该串行——install 流程自身也是顺序执行）。
fn execute(
    script: &Path,
    pkg_dir: &Path,
) -> PackageResult<()> {
    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let original = std::env::current_dir()?;
    std::env::set_current_dir(pkg_dir).map_err(|e| {
        PackageError::InvalidManifest(format!("无法切换工作目录到 {}: {e}", pkg_dir.display()))
    })?;
    let result = crate::run_file(script);
    // 恢复 cwd：错误路径也不能把进程留在包目录里
    if let Err(e) = std::env::set_current_dir(&original) {
        if result.is_ok() {
            return Err(PackageError::InvalidManifest(format!(
                "无法恢复工作目录: {e}"
            )));
        }
    }
    result.map_err(|e| PackageError::InvalidManifest(format!("{BUILD_SCRIPT} 执行失败: {e:#}")))
}

/// CLI 默认确认器：stdin 读一行，`y`/`yes`（忽略大小写）为确认
pub fn confirm_via_stdin(question: &str) -> bool {
    use std::io::Write as _;
    print!("{question} ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).unwrap_or(0) == 0 {
        return false;
    }
    matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}
