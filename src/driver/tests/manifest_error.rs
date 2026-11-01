//! 清单存在但不可读的直驱钉测试（WBS 3.23.1 / issue #503）
//!
//! 判据来源：RFC-029f §判定三层（声明面 = manifest 声明的包目标——引用池
//! 的根）+ issue #503 的登记判据（WBS 3.23.1）。裁决（2026-10-30，用户全权
//! 委托记录在案，方案甲）：`yaoxiang.toml` **存在但不可读**（非法 TOML /
//! 缺 `[package]` version 必填字段）时与非法 .yx 源码同待遇——报 E3024
//! （error 级）并中止。理由：坏清单下声明面/角色分类/引用池全部建立在错误
//! 前提上，继续跑产出的一切结果不可信；`RoleClassification` 标记失败后
//! `DeadCodeAnalysis` 拓扑跳过，退化单文件语义产的 W1002 误报随之消失
//! （中止发生在退化之前），check 退出码非 0（error 级 ⇒ error_count > 0）。
//!
//! 两形态红态（修前实测，2026-10-30，CLI 探针 `yaoxiang check main.yx`）：
//! 0 条清单诊断 + 2 条 W1002 误报 + 退出 0。守卫：无清单文件（合法单文件
//! 模式）与合法清单两形态行为不变（红态输出留档于批次实录）。

use crate::driver::DriverOutcome;
use crate::util::diagnostic::Severity;

use super::ref_pool::{assert_warnings, run_check, write_project, MANIFEST};

/// issue #503 最小复现的三个文件：a/b 各定义同名 `Temp`，main 消费两者——
/// 坏清单退化单文件语义时恰好在 a/b 上误报 2×W1002
const A_YX: &str =
    "Temp: Type = { v: Float }\nTemp.tag: (self: &Temp) -> Float = (self) => { self.v + 1.0 }\n";
const B_YX: &str =
    "Temp: Type = { v: Float }\nTemp.tag: (self: &Temp) -> Float = (self) => { self.v * 10.0 }\n";
const MAIN_YX: &str = "use std.assert\nuse a.{Temp}\nuse b.{Temp as TempB}\n\nmain = () => {\n    t = Temp(1.0)\n    u = TempB(2.0)\n    assert.assert(t.tag() == 2.0, \"a\")\n    assert.assert(u.tag() == 20.0, \"b\")\n}\n";

/// 断言入口条目恰好报出 E3024（error 级、消息含清单路径）并返回该诊断。
/// E3024 挂在入口文件名下（逐文件收集通道的载体；错误位置在 yaoxiang.toml，
/// span 豁免故 line 记 0）。
fn assert_entry_carries_e3024(outcome: &DriverOutcome) -> &crate::util::diagnostic::Diagnostic {
    let files = outcome
        .check_diagnostics
        .iter()
        .map(|(p, _)| p.display().to_string())
        .collect::<Vec<_>>();
    let entry_diags = outcome
        .check_diagnostics
        .iter()
        .find(|(p, _)| p.ends_with("main.yx"))
        .map(|(_, ds)| ds)
        .unwrap_or_else(|| panic!("check 收集通道必须含入口条目: {files:?}"));
    let e3024 = entry_diags
        .iter()
        .find(|d| d.code == "E3024")
        .unwrap_or_else(|| {
            panic!(
                "坏清单必须报 E3024，入口条目实际: {:?}",
                entry_diags
                    .iter()
                    .map(|d| d.code.clone())
                    .collect::<Vec<_>>()
            )
        });
    assert!(
        matches!(e3024.severity, Severity::Error),
        "E3024 必须是 error 级（check 的 error_count>0 ⇒ 退出码非 0 的依据）"
    );
    assert!(
        e3024.message.contains("yaoxiang.toml"),
        "E3024 消息必须含清单路径: {}",
        e3024.message
    );
    e3024
}

/// 断言退化路径的 W1002 误报不再出现（a/b 条目存在但无 W1002——发现集成员
/// 身份不变，只是 DeadCodeAnalysis 已被拓扑跳过）。
fn assert_degraded_w1002_gone(outcome: &DriverOutcome) {
    for (path, diags) in &outcome.check_diagnostics {
        if path.ends_with("a.yx") || path.ends_with("b.yx") {
            assert!(
                !diags.iter().any(|d| d.code == "W1002"),
                "坏清单下 W1002 是错误前提上的误报，必须消失: {}",
                path.display()
            );
        }
    }
}

/// 形态一（缺 `[package]` version 必填字段）：E3024 报出、退化 W1002 消失。
#[test]
fn test_check_with_versionless_manifest_reports_e3024_and_drops_w1002() {
    // Arrange：合法 TOML 但 [package] 缺 version（issue #503 表格第一行）
    let (_dir, entry) = write_project(
        &[("a.yx", A_YX), ("b.yx", B_YX), ("main.yx", MAIN_YX)],
        Some("[package]\nname = \"t\"\n\n[run]\nmain = \"main.yx\"\n"),
    );

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：E3024（error 级、消息含清单路径）+ 退化 W1002 消失
    assert_entry_carries_e3024(&outcome);
    assert_degraded_w1002_gone(&outcome);
}

/// 形态二（非法 TOML）：E3024 报出、退化 W1002 消失。
#[test]
fn test_check_with_invalid_toml_manifest_reports_e3024_and_drops_w1002() {
    // Arrange：清单是非法 TOML（issue #503 表格第二行）
    let (_dir, entry) = write_project(
        &[("a.yx", A_YX), ("b.yx", B_YX), ("main.yx", MAIN_YX)],
        Some("this is not toml {{{\n"),
    );

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：与形态一同一绿态
    assert_entry_carries_e3024(&outcome);
    assert_degraded_w1002_gone(&outcome);
}

/// 守卫 A（未修分支）：无清单文件 = 合法单文件模式，行为零漂移——无 E3024，
/// 跨文件消费豁免照旧（used 被 main 消费不得报 W1001/W1002）。
#[test]
fn test_check_without_manifest_keeps_single_file_semantics() {
    // Arrange：同样的项目形状，但不放 yaoxiang.toml
    let lib = "used: () -> Int = () => { return 1 }\n";
    let main = "use std.assert\nuse lib.{used}\n\nmain: () -> Void = {\n    assert.assert(used() == 1, \"used() should be 1\")\n}\n";
    let (_dir, entry) = write_project(&[("lib.yx", lib), ("main.yx", main)], None);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：无 E3024（无清单不是错误）；消费豁免照旧
    assert!(
        !outcome
            .check_diagnostics
            .iter()
            .any(|(_, ds)| ds.iter().any(|d| d.code == "E3024")),
        "无清单是合法单文件模式，不得报 E3024: {:?}",
        outcome.check_diagnostics
    );
    assert_warnings(&outcome, "lib.yx", &[], &[("W1001", 1), ("W1002", 1)]);
}

/// 守卫 B（未修分支）：合法清单零漂移——无 E3024，跨文件消费豁免照旧
/// （ref_pool.rs 对照组同款夹具，防本修复把池整体关掉）。
#[test]
fn test_check_with_valid_manifest_free_of_manifest_diagnostics() {
    // Arrange：合法 manifest（[package] 齐全 + [run].main）
    let lib = "used: () -> Int = () => { return 1 }\n\nPoint: Type = { x: Int }\n";
    let main = "use std.assert\nuse lib.{used, Point}\n\nmain: () -> Void = {\n    p = Point(1)\n    assert.assert(used() == 1, \"used() should be 1\")\n    assert.assert(p.x == 1, \"Point field should be 1\")\n}\n";
    let (_dir, entry) = write_project(&[("lib.yx", lib), ("main.yx", main)], Some(MANIFEST));

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：无 E3024；Bin/Lib 跨文件消费豁免照旧
    assert!(
        !outcome
            .check_diagnostics
            .iter()
            .any(|(_, ds)| ds.iter().any(|d| d.code == "E3024")),
        "合法清单不得报 E3024: {:?}",
        outcome.check_diagnostics
    );
    assert_warnings(&outcome, "lib.yx", &[], &[("W1001", 2), ("W1002", 4)]);
}

/// 坏清单下 RoleClassification 标记失败 ⇒ 依赖它的 DeadCodeAnalysis 被拓扑
/// 跳过（SkipReason::UpstreamFailed）——「中止发生在退化之前」的机制直证。
#[test]
fn test_dead_code_analysis_skipped_when_manifest_unreadable() {
    // Arrange：形态一夹具（缺 version）
    let (_dir, entry) = write_project(
        &[("a.yx", A_YX), ("b.yx", B_YX), ("main.yx", MAIN_YX)],
        Some("[package]\nname = \"t\"\n\n[run]\nmain = \"main.yx\"\n"),
    );

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：跳过记录里 DeadCodeAnalysis 的原因是上游 RoleClassification 失败
    assert!(
        outcome.skipped.iter().any(
            |s| matches!(s.stage, crate::driver::Stage::DeadCodeAnalysis)
                && matches!(
                    s.reason,
                    crate::driver::SkipReason::UpstreamFailed(
                        crate::driver::Stage::RoleClassification
                    )
                )
        ),
        "DeadCodeAnalysis 必须因 RoleClassification 失败被拓扑跳过: {:?}",
        outcome.skipped
    );
}

/// #520 甲案（2026-10-31 用户裁决）：`[tool.test]` 面的类型错误（patterns 写
/// 成字符串而非列表）可穿透 PackageManifest（无 tool 字段、serde 忽略未知
/// 字段），旧代码 `.ok()` 静默吞掉 ⇒ 显式 patterns 静默回退内建约定。红态
/// （修前实测 2026-10-31，CLI 探针）：0 条清单诊断 + 退出 0 + `custom_tests`
/// 下文件误判 Internal 报 W1001。修后与声明面同待遇：同一文件存在但不可读
/// ⇒ E3024（error 级，消息含路径与 patterns 解析原因）并中止（RFC-015:282
/// 「配置解析失败时显示友好错误」/ RFC-036:146 [tool.*] 约定）。
#[test]
fn test_check_with_tool_test_type_error_reports_e3024() {
    // Arrange：声明面合法（[package] 齐全），[tool.test] patterns 是字符串
    let (_dir, entry) = write_project(
        &[("main.yx", "main = () => { }\n")],
        Some(concat!(
            "[package]\nname = \"t\"\nversion = \"0.1.0\"\n\n",
            "[run]\nmain = \"main.yx\"\n\n",
            "[tool.test]\npatterns = \"custom_tests/**/*.yx\"\n"
        )),
    );

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：E3024（error 级、消息含路径）+ detail 携带 patterns 的类型错
    let e3024 = assert_entry_carries_e3024(&outcome);
    assert!(
        e3024.message.contains("patterns"),
        "E3024 消息必须携带 [tool.test].patterns 的解析原因: {}",
        e3024.message
    );
}

/// 守卫（未修分支，#520）：合法 `[tool.test]` patterns（列表形态）零漂移——
/// 无 E3024，且规则真实生效：custom_tests 下的文件按显式 pattern 判 Test、
/// 跳过死代码判定（其未使用函数不报 W1001）。防修复把 test_rules 整体关掉。
/// 入口 = t.yx 本身（与 CLI `check custom_tests/t.yx` 的逐入口 check_project
/// 同构；若 test_rules 被静默丢弃，t.yx 退化为 Internal ⇒ W1001 现形）。
#[test]
fn test_check_with_valid_tool_test_rules_free_of_manifest_diagnostics() {
    // Arrange：合法清单 + patterns 列表；t.yx 在 custom_tests 下、带未使用函数
    let t_yx = "t1: () -> Int = () => { return 1 }\n";
    let manifest = concat!(
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\n\n",
        "[run]\nmain = \"main.yx\"\n\n",
        "[tool.test]\npatterns = [\"custom_tests/**/*.yx\"]\n"
    );
    let (_dir, entry) = write_project(
        &[
            ("main.yx", "main = () => { }\n"),
            ("custom_tests/t.yx", t_yx),
        ],
        Some(manifest),
    );
    let t_entry = entry.parent().unwrap().join("custom_tests").join("t.yx");

    // Act：入口 = t.yx 的单文件 Check（与 ref_pool::run_check 同构，键取 t）
    let program = crate::driver::Program::new(
        crate::driver::ProgramKind::Check,
        vec![crate::driver::Unit::new("t", t_entry, "")],
        crate::frontend::config::CompileConfig::default(),
    );
    let outcome = crate::driver::Driver::new()
        .run(program)
        .expect("check 必须返回产物");

    // Assert：无 E3024；t.yx 判 Test ⇒ 其未使用函数不报 W1001
    assert!(
        !outcome
            .check_diagnostics
            .iter()
            .any(|(_, ds)| ds.iter().any(|d| d.code == "E3024")),
        "合法 [tool.test] 不得报 E3024: {:?}",
        outcome.check_diagnostics
    );
    let t_diags = outcome
        .check_diagnostics
        .iter()
        .find(|(p, _)| p.ends_with("t.yx"))
        .map(|(_, ds)| ds)
        .unwrap_or_else(|| {
            panic!(
                "check 收集通道必须含 t.yx 条目: {:?}",
                outcome.check_diagnostics
            )
        });
    assert!(
        !t_diags.iter().any(|d| d.code == "W1001"),
        "显式 pattern 命中的测试文件判 Test、不参与死代码判定: {:?}",
        t_diags
    );
}
