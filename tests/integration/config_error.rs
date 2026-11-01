//! #520（甲案，2026-10-31 用户裁决）：三处「配置存在但读不了/解析不了 →
//! 静默用默认」收窄为显式报错——CLI 端到端判据。
//!
//! 规范依据：RFC-015 §向后兼容性（015-configuration-system.md:282）明文
//! 「配置解析失败时显示友好错误，提示具体行号和错误原因」；[tool.test] 经
//! RFC-036:146 挂在 RFC-015 的 [tool.*] 扩展约定下。
//!
//! 红态（修前实测 2026-10-31，CLI 探针；红绿实录见 issue #520 与
//! 09a 收口实录 3.23.2 节）：
//! - run：yaoxiang.toml 写 `[mono`（非法 TOML）⇒ E3005 按**默认闸 100** 报
//!   「(100，当前 101)」——合法对照 `[mono] max_depth = 150` 报 150，即静默
//!   回退实锤；
//! - format：`[fmt`（非法 TOML，意图 line_width=40）⇒ 长表达式不换行（默认
//!   120）且退出 0——合法对照按 40 换行；
//! - format（用户级）：XDG_CONFIG_HOME 下坏 config.toml ⇒ 同上静默默认。
//!
//! 守卫（本文件与单元测试共同钉住）：无配置文件 = 合法默认；未知 section/
//! 未知字段仍容忍（serde ignore）；合法配置行为不变。

#![cfg(feature = "cli")]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// 编译产物路径（cargo 在测试构建时注入）
fn yx_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_yaoxiang-rs"))
}

/// 跑 CLI 子命令（cwd = 工作目录，可注入环境变量），返回 (退出码, stdout, stderr)
fn run_cli(
    dir: &Path,
    args: &[&str],
    env: &[(&str, &str)],
) -> (i32, String, String) {
    let mut cmd = Command::new(yx_bin());
    cmd.args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn yaoxiang: {e}"));
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// 写一组文件到临时目录（含可选 yaoxiang.toml）
fn make_project(
    toml_content: Option<&str>,
    files: &[(&str, &str)],
) -> TempDir {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("create tempdir: {e}"));
    if let Some(src) = toml_content {
        std::fs::write(tmp.path().join("yaoxiang.toml"), src)
            .unwrap_or_else(|e| panic!("write yaoxiang.toml: {e}"));
    }
    for (name, content) in files {
        let path = tmp.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| panic!("mkdir {name}: {e}"));
        }
        std::fs::write(&path, content).unwrap_or_else(|e| panic!("write {name}: {e}"));
    }
    tmp
}

/// run：yaoxiang.toml 存在但非法 TOML ⇒ 显式报错 + 非零退出（消息含文件
/// 路径与 toml 行号原因），不再静默按默认 runtime 跑（红态：exit 0 且闸门
/// 值回落默认——见文件头红态留档）。
#[test]
fn test_e2e_run_broken_project_config_fails_with_config_error() {
    // Arrange：非法 TOML（表头未闭合）+ 平凡可运行程序
    let tmp = make_project(
        Some("[mono\nmax_depth = 150\n"),
        &[("main.yx", "main = () => { print(42) }\n")],
    );

    // Act
    let (code, _stdout, stderr) = run_cli(tmp.path(), &["run", "main.yx"], &[]);

    // Assert：非零退出 + 消息含文件路径与解析原因（含行号）
    assert_ne!(code, 0, "坏项目配置必须非零退出，实际 exit = {code}");
    assert!(
        stderr.contains("yaoxiang.toml"),
        "错误消息必须含配置文件路径，实际 stderr：{stderr:?}"
    );
    assert!(
        stderr.contains("TOML parse error") && stderr.contains("line 1"),
        "错误消息必须含 toml 行号与原因（RFC-015:282），实际 stderr：{stderr:?}"
    );
}

/// 守卫（未修分支）：无 yaoxiang.toml = 合法（script 模式默认语义不动），
/// 不得把「不存在」也报错。
#[test]
fn test_e2e_run_without_project_config_keeps_script_semantics() {
    // Arrange：无配置文件
    let tmp = make_project(None, &[("main.yx", "print(42)\n")]);

    // Act
    let (code, stdout, stderr) = run_cli(tmp.path(), &["run", "main.yx"], &[]);

    // Assert：退出 0 且正常输出；无配置错误字样
    assert_eq!(
        code, 0,
        "无配置文件是合法默认，不得报错，stderr：{stderr:?}"
    );
    assert!(
        stdout.contains("42"),
        "script 模式默认语义必须照常执行，实际 stdout：{stdout:?}"
    );
    assert!(
        !stderr.contains("yaoxiang.toml"),
        "无配置文件不得出现配置错误，实际 stderr：{stderr:?}"
    );
}

/// 守卫（未修分支）：未知 section / 未知字段仍容忍（serde ignore 语义不动）。
#[test]
fn test_e2e_run_unknown_config_fields_tolerated() {
    // Arrange：未知节 + 已知节内未知字段 + 合法值
    let tmp = make_project(
        Some(
            "[unknown_section]\nfoo = 1\n\n[runtime]\nmode = \"embedded\"\nunknown_field = \"x\"\n",
        ),
        &[("main.yx", "main = () => { print(42) }\n")],
    );

    // Act
    let (code, stdout, stderr) = run_cli(tmp.path(), &["run", "main.yx"], &[]);

    // Assert：照常运行
    assert_eq!(code, 0, "未知字段必须容忍，stderr：{stderr:?}");
    assert!(stdout.contains("42"), "程序必须照常执行：{stdout:?}");
}

/// format：yaoxiang.toml 存在但非法 TOML ⇒ 显式报错 + 非零退出（红态：
/// 静默按默认格式化且退出 0——见文件头红态留档）。
#[test]
fn test_e2e_format_broken_project_config_fails_with_config_error() {
    // Arrange：非法 TOML + 待格式化文件
    let tmp = make_project(
        Some("[fmt\nline_width = 40\n"),
        &[("fmt.yx", "value = 1 + 2\n")],
    );

    // Act
    let (code, _stdout, stderr) = run_cli(tmp.path(), &["format", "fmt.yx", "--stdout"], &[]);

    // Assert
    assert_ne!(code, 0, "坏项目配置必须非零退出，实际 exit = {code}");
    assert!(
        stderr.contains("yaoxiang.toml") && stderr.contains("TOML parse error"),
        "错误消息必须含路径与解析原因，实际 stderr：{stderr:?}"
    );
}

/// format：用户级配置存在但非法 TOML ⇒ 显式报错 + 非零退出（红态：静默
/// 按默认格式化——见文件头红态留档）。
#[test]
fn test_e2e_format_broken_user_config_fails_with_config_error() {
    // Arrange：XDG_CONFIG_HOME 指向含坏 config.toml 的临时目录（config
    // 模块的路径解析 XDG 优先，Windows 下注入同样生效）
    let user_dir = TempDir::new().unwrap_or_else(|e| panic!("create tempdir: {e}"));
    let cfg_dir = user_dir.path().join("yaoxiang");
    std::fs::create_dir_all(&cfg_dir).unwrap_or_else(|e| panic!("mkdir yaoxiang: {e}"));
    std::fs::write(cfg_dir.join("config.toml"), "[fmt\nline_width = 40\n")
        .unwrap_or_else(|e| panic!("write config.toml: {e}"));
    let tmp = make_project(None, &[("fmt.yx", "value = 1 + 2\n")]);
    let xdg = user_dir.path().display().to_string();

    // Act
    let (code, _stdout, stderr) = run_cli(
        tmp.path(),
        &["format", "fmt.yx", "--stdout"],
        &[("XDG_CONFIG_HOME", xdg.as_str())],
    );

    // Assert：非零退出 + 消息含用户配置路径与解析原因
    assert_ne!(code, 0, "坏用户配置必须非零退出，实际 exit = {code}");
    assert!(
        stderr.contains("config.toml") && stderr.contains("TOML parse error"),
        "错误消息必须含用户配置路径与解析原因，实际 stderr：{stderr:?}"
    );
}

/// 守卫（未修分支）：format 无任何配置 = 默认输出不变；合法 [fmt] 配置仍
/// 生效（guard① 的 format 对照）。
#[test]
fn test_e2e_format_without_config_keeps_default_and_valid_config_applies() {
    // Arrange：同一源文件两份——无配置 / 合法 [fmt] line_width = 40
    let src = "value = 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10 + 11 + 12 + 13 + 14 + 15 + 16 + 17 + 18 + 19 + 20\n";
    let no_cfg = make_project(None, &[("fmt.yx", src)]);
    let valid_cfg = make_project(Some("[fmt]\nline_width = 40\n"), &[("fmt.yx", src)]);

    // Act
    let (c1, out1, err1) = run_cli(no_cfg.path(), &["format", "fmt.yx", "--stdout"], &[]);
    let (c2, out2, err2) = run_cli(valid_cfg.path(), &["format", "fmt.yx", "--stdout"], &[]);

    // Assert：无配置 = 默认行宽（120，单行不换）；合法配置 = 40 换行
    assert_eq!(c1, 0, "无配置 format 必须成功，stderr：{err1:?}");
    assert_eq!(c2, 0, "合法配置 format 必须成功，stderr：{err2:?}");
    assert!(
        out1.lines().count() == 1,
        "默认行宽下长表达式不换行（红态同款输出），实际：{out1:?}"
    );
    assert!(
        out2.lines().count() > 1 && out2.contains("+ 11"),
        "合法 [fmt] line_width=40 必须仍生效（换行），实际：{out2:?}"
    );
}
