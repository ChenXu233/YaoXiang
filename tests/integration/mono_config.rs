//! #471（WBS 3.21.3）：`[mono]` 节从 yaoxiang.toml 贯通到单态化深度闸（端到端）
//!
//! 通过 `CARGO_BIN_EXE_yaoxiang-rs` 子进程验证用户真实路径：
//! yaoxiang.toml 的 `[mono] max_depth` 抬高编译期特化链深度闸——E3005 的
//! 拦截点随配置平移；未配置 `[mono]` 节时保持默认上限 100。
//!
//! 判据来源：
//! - issue #471 / WBS 3.21.3 验收：toml 配置 mono.max_depth 后 run 触达高闸门形态
//! - 3.10.1：病态泛型递归（类型逐层增长永不收敛）的唯一编译期防线是 E3005，
//!   拦截位置随 `mono.max_depth` 平移（同形判据见
//!   `src/driver/tests/mono_cross_unit.rs` 的 150 档用例）
//! - docs/src/dev/test-specification.md 规则 9.1：E2E 覆盖 error/boundary 路径
//!
//! 深度档说明：本文件取 150（与 mono_cross_unit.rs 的配置平移用例同档，
//! 逐层计数肉眼可验，峰值内存 ≈ 3.19 MiB/层 × 150 ≈ 0.5 GB）。5000 档实测
//! 72.6 s / 峰值 15.6 GB（issue #449），不进默认测试套件。

#![cfg(feature = "cli")]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// 编译产物路径（cargo 在测试构建时注入）
fn yx_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_yaoxiang-rs"))
}

/// 病态跨单元泛型互递归夹具（与 `src/driver/tests/mono_cross_unit.rs` 的
/// `PATHOLOGICAL_FILES` 同形态）：`a.f` → `b.g` → `a.f` 以自嵌套类型互调，
/// 类型逐层增长（T → Vec(T) → Vec(Vec(T)) → …）且永不重复——只能由深度闸拦截。
/// 与 driver 直驱版的差别仅在把触发点 `a.f(1)` 放进 `main`（项目内有
/// yaoxiang.toml → Bin 入口语义，需 main 绑定）。
const PATHOLOGICAL_A: &str =
    "use b\nf: (T: Type) -> (x: T) -> T = (x) => {\n    return b.g([x])\n}\n";
const PATHOLOGICAL_B: &str =
    "use a\ng: (T: Type) -> (x: T) -> T = (x) => {\n    return a.f([x])\n}\n";
const PATHOLOGICAL_MAIN: &str = "use a\nmain = () => { a.f(1) }\n";

/// 建一个临时项目：写入给定的 yaoxiang.toml 内容 + 病态递归三文件
fn make_mono_probe_project(toml_content: &str) -> TempDir {
    let tmp = TempDir::new().unwrap_or_else(|e| panic!("create tempdir: {e}"));
    std::fs::write(tmp.path().join("yaoxiang.toml"), toml_content)
        .unwrap_or_else(|e| panic!("write yaoxiang.toml: {e}"));
    std::fs::write(tmp.path().join("a.yx"), PATHOLOGICAL_A)
        .unwrap_or_else(|e| panic!("write a.yx: {e}"));
    std::fs::write(tmp.path().join("b.yx"), PATHOLOGICAL_B)
        .unwrap_or_else(|e| panic!("write b.yx: {e}"));
    std::fs::write(tmp.path().join("main.yx"), PATHOLOGICAL_MAIN)
        .unwrap_or_else(|e| panic!("write main.yx: {e}"));
    tmp
}

/// 在项目目录内跑 `yaoxiang run main.yx`（cwd = 项目根——Run 臂按相对路径
/// 读 cwd 下的 yaoxiang.toml），返回 (退出码, stdout, stderr)
fn run_main_in_project(project: &Path) -> (i32, String, String) {
    let output = Command::new(yx_bin())
        .args(["run", "main.yx"])
        .current_dir(project)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn yaoxiang: {e}"));
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// yaoxiang.toml 的 `[mono] max_depth` 必须抬高编译期深度闸：150 上限下
/// E3005 报出「(150，当前 151)」——拦截点随配置平移，而不是钉死在默认 100。
///
/// 红态（修复前）：Run 路径不读 mono 节，toml 写 150 仍报 (100，当前 101)。
#[test]
fn test_e2e_run_toml_mono_max_depth_moves_compile_gate() {
    // Arrange：临时项目，yaoxiang.toml 显式抬高特化链深度上限到 150
    let tmp = make_mono_probe_project("[mono]\nmax_depth = 150\n");

    // Act
    let (code, _stdout, stderr) = run_main_in_project(tmp.path());

    // Assert①：病态递归仍在**编译期**失败（不是静默通过、不是崩栈）
    assert_ne!(code, 0, "病态泛型递归必须编译失败，exit code = {code}");
    // Assert②：E3005 报出配置上限 150 与当前深度 151（证明闸门真的吃到 toml）
    assert!(
        stderr.contains("E3005") && stderr.contains("(150，当前 151)"),
        "E3005 必须按 toml 的 max_depth=150 报「(150，当前 151)」，实际 stderr：{stderr:?}"
    );
}

/// 未配置 `[mono]` 节时保持默认上限 100——行为与既有默认档一致。
#[test]
fn test_e2e_run_without_mono_section_keeps_default_gate_100() {
    // Arrange：临时项目，yaoxiang.toml 存在但没有 [mono] 节
    let tmp = make_mono_probe_project("");

    // Act
    let (code, _stdout, stderr) = run_main_in_project(tmp.path());

    // Assert：E3005 报默认上限 100 与当前深度 101
    assert_ne!(code, 0, "病态泛型递归必须编译失败，exit code = {code}");
    assert!(
        stderr.contains("E3005") && stderr.contains("(100，当前 101)"),
        "无 [mono] 节必须按默认上限报「(100，当前 101)」，实际 stderr：{stderr:?}"
    );
}

/// 验收「未配置 mono 节时与当前默认档逐字节一致」的可执行形态：
/// 显式写 `[mono] max_depth = 100` 与完全省略 `[mono]` 节，同一探针的
/// (退出码, stdout, stderr) 必须完全相等。
#[test]
fn test_e2e_run_explicit_mono_100_matches_omitted_mono_section_bytewise() {
    // Arrange：同一探针夹具两份，只差 [mono] 节的显式/省略
    let explicit = make_mono_probe_project("[mono]\nmax_depth = 100\n");
    let omitted = make_mono_probe_project("");

    // Act
    let explicit_run = run_main_in_project(explicit.path());
    let omitted_run = run_main_in_project(omitted.path());

    // Assert：逐字节一致（serde default 保证 + 实测对照）
    assert_eq!(
        explicit_run.0, omitted_run.0,
        "退出码必须一致：显式 100 = {:?}，省略节 = {:?}",
        explicit_run.0, omitted_run.0
    );
    assert_eq!(
        explicit_run.1, omitted_run.1,
        "stdout 必须逐字节一致：\n显式 100：{:?}\n省略节：{:?}",
        explicit_run.1, omitted_run.1
    );
    assert_eq!(
        explicit_run.2, omitted_run.2,
        "stderr 必须逐字节一致：\n显式 100：{:?}\n省略节：{:?}",
        explicit_run.2, omitted_run.2
    );
}
