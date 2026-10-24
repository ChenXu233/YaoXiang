//! #491（WBS 4.10.5）钉测试：模块导入环在编译期发 W1007，**不得静默**。
//!
//! 判据来源：用户 2026-10-23 裁决 A（环不得静默）+ RFC-013「W1xxx 警告码」的
//! W1007 条目（生成物见 docs/src/reference/warning-code/warning-codes.md）。
//!
//! ① 底线是**存在环**：编译集 `use` 依赖图有成员数 ≥2 的强连通分量即告警，
//!    不看本次运行是否真的读到未初始化值（那是运行期 E6007 的判据，两件事）；
//! ② 环路径进参数：消息里是 `use` 方向的 `a → b → a`；
//! ③ 定位落在**真实语句**上：锚在环内一条 `use` 语句（有值边时优先被违反的
//!    那条），故本码不需要 span 豁免；
//! ④ 精确信息：环内读侧 `Load{src: Global}` 早于该槽位首条 `Store{dst: Global}`
//!    且跨单元时，「被违反的边 + 被读槽的限定名」并入**同一**诊断的 help；
//! ⑤ 误报面：无环项目零 W1007；
//! ⑥ 非阻断：告警不影响编译结果（本模块不测退出码——那是 CLI 的判据，
//!    tests/yaoxiang-multifile/import-cycle-reads-before-init 端到端钉着）。
//!
//! 检出单源：环来自 4.10.4 已算的 Tarjan SCC（==helpers::init_topological_order==
//! 出参），本模块不做任何环检测，只读诊断面。
//!
//! 红态（修复前，实测）：同一形状的夹具 `check` 退出 0 且**零环诊断**，`run`
//! 只把结构原因藏在运行期 `E6007 type mismatch in binary operation Add`
//! （`__yx_module_init` 栈帧）里——用户看不到「环 / 初始化顺序」这层事实。

use std::path::Path;

use super::driver::{make_script_project, run_multi};
use crate::driver::DriverOutcome;
use crate::util::diagnostic::Diagnostic;

/// 环成员 zlib：init 读 zother.o_val（值依赖要求 zother 先初始化）。
const CYCLE_ZLIB: &str = "use zother.{o_val}\n\nz_val: Int = o_val + 1\n";

/// 环成员 zother：导入 zlib 且 init 读 zlib.z_val（use 依赖把 zlib 排在前面）。
/// 注解行有意占掉第 1 行——定位判据要能区分「use 在第 1 行」与「在真实行」。
const CYCLE_ZOTHER: &str =
    "// zother —— 环成员：import zlib\nuse zlib.{z_val}\n\no_val: Int = z_val + 1\n";

/// 入口：同时导入环上两个单元（都被真实消费，避免 W1003/W1004 噪声干扰判据）。
const CYCLE_MAIN: &str = "use std.assert\nuse zlib.{z_val}\nuse zother.{o_val}\n\n\
                          main = () => {\n    assert.assert(z_val + o_val > 0, \"cycle probe\")\n}\n";

/// 收集多文件形态警告面里码号为 `code` 的诊断（逐文件收集通道，WBS 3.4.6）。
fn diagnostics_with_code<'a>(
    outcome: &'a DriverOutcome,
    code: &str,
) -> Vec<(&'a Path, &'a Diagnostic)> {
    outcome
        .check_diagnostics
        .iter()
        .flat_map(|(path, ds)| {
            ds.iter()
                .filter(|d| d.code == code)
                .map(move |d| (path.as_path(), d))
        })
        .collect()
}

/// 环 ⟹ 恰一条 W1007，且消息里带 `use` 方向的环路径；编译本身不被阻断。
#[test]
fn test_multifile_import_cycle_emits_single_w1007_with_cycle_path() {
    // Arrange：zlib ↔ zother 互导（SCC 成员 2），入口同时导入两者
    let (_dir, entry) = make_script_project(&[
        ("zlib.yx", CYCLE_ZLIB),
        ("zother.yx", CYCLE_ZOTHER),
        ("main.yx", CYCLE_MAIN),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("MultiFile 形态必须返回产物");

    // Assert：恰一条 W1007，消息含环路径；编译成功（警告非阻断）
    let cycles = diagnostics_with_code(&outcome, "W1007");
    assert_eq!(
        cycles.len(),
        1,
        "一个导入环只报一条 W1007（环是编译集共同的事实，逐成员重报是噪声）：{:?}",
        cycles
            .iter()
            .map(|(p, d)| (p.display().to_string(), d.message.clone()))
            .collect::<Vec<_>>()
    );
    assert!(
        cycles[0].1.message.contains("zlib → zother → zlib"),
        "W1007 消息必须带环路径（use 方向，首尾相接），实际：{}",
        cycles[0].1.message
    );
    assert!(
        outcome.failure.is_none(),
        "导入环只是警告（W 默认非阻断），不得把编译结果变成失败：{:?}",
        outcome.failure.as_ref().map(|f| f.to_string())
    );
}

/// 定位：环诊断锚在环内**真实 `use` 语句**上（不是类型、不是空 span）。
#[test]
fn test_multifile_import_cycle_warning_anchors_on_use_statement() {
    // Arrange：同上环；zother.yx 的 `use zlib.{z_val}` 在第 2 行
    let (_dir, entry) = make_script_project(&[
        ("zlib.yx", CYCLE_ZLIB),
        ("zother.yx", CYCLE_ZOTHER),
        ("main.yx", CYCLE_MAIN),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("MultiFile 形态必须返回产物");

    // Assert：挂在 zother.yx（zother 导入 zlib —— 被违反的那条边）第 2 行
    let cycles = diagnostics_with_code(&outcome, "W1007");
    assert_eq!(cycles.len(), 1, "本夹具应恰有一条环诊断：{cycles:?}");
    let (path, diagnostic) = cycles[0];
    assert!(
        path.ends_with("zother.yx"),
        "环诊断应归到**被违反的边**所在文件（zother 导入 zlib），实际 {}",
        path.display()
    );
    let span = diagnostic.span.expect("W1007 不豁免 span，必须带源码位置");
    assert_eq!(
        span.start.line, 2,
        "环诊断应锚在 use 语句那一行（zother.yx:2），实际第 {} 行",
        span.start.line
    );
}

/// 精确信息：环内「读早于写」的跨单元值边并入同一诊断的 help（同码同发射点）。
#[test]
fn test_multifile_import_cycle_reports_violated_value_edge() {
    // Arrange：zlib 读 zother.o_val（owner=zother，reader=zlib）
    let (_dir, entry) = make_script_project(&[
        ("zlib.yx", CYCLE_ZLIB),
        ("zother.yx", CYCLE_ZOTHER),
        ("main.yx", CYCLE_MAIN),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("MultiFile 形态必须返回产物");

    // Assert：help 里同时有被读槽的限定名与被违反的边两个单元
    let cycles = diagnostics_with_code(&outcome, "W1007");
    assert_eq!(cycles.len(), 1, "本夹具应恰有一条环诊断：{cycles:?}");
    let help = &cycles[0].1.help;
    assert!(
        help.contains("zother.o_val"),
        "help 必须点名被读槽的限定名（zother.o_val），实际：{help}"
    );
    assert!(
        help.contains("zlib") && help.contains("zother"),
        "help 必须写出被违反的边两端的单元，实际：{help}"
    );
}

/// 良性环（只互导函数、无值边）⟹ 基础消息**不预判运行期后果**（复审 F3）。
///
/// 判据：基础消息只陈述确定事实（存在环 / 环路径 / 环内初始化顺序不可判定），
/// 不得让用户以为自己的程序会崩——该形状实测 `run` 退出 0、零 E6007。具体后果只在
/// **确有值边且读早于写**时由 help 给出（见
/// test_multifile_import_cycle_reports_violated_value_edge）。
#[test]
fn test_multifile_benign_import_cycle_message_predicts_no_runtime_failure() {
    // Arrange：两模块互导**函数**（无顶层值 ⇒ 无值边；环是良性的）
    let a = "use b.{bee}\n\naye: () -> Int = () => bee()\n";
    // b 只 `use` a 的函数、自身不再回调 a：环来自导入图（不是运行期递归），
    // 故该形状在 CLI 上 run 退出 0、零 E6007——正是 F3 说的「良性环」。
    let b = "use a.{aye}\n\nbee: () -> Int = () => 1\n";
    let main = "use a.{aye}\n\nmain = () => {\n    x = aye()\n}\n";
    let (_dir, entry) = make_script_project(&[("a.yx", a), ("b.yx", b), ("main.yx", main)]);

    // Act
    let outcome = run_multi(&entry).expect("MultiFile 形态必须返回产物");

    // Assert：恰一条 W1007，确定事实齐全，且无任何运行期后果预判
    let cycles = diagnostics_with_code(&outcome, "W1007");
    assert_eq!(
        cycles.len(),
        1,
        "良性环也必须报且只报一条 W1007（存在环即报是底线）：{:?}",
        cycles
            .iter()
            .map(|(p, d)| (p.display().to_string(), d.message.clone(), d.help.clone()))
            .collect::<Vec<_>>()
    );
    let diagnostic = cycles[0].1;
    assert!(
        diagnostic.message.contains("a → b → a"),
        "基础消息必须保留确定事实（use 方向环路径），实际：{}",
        diagnostic.message
    );
    // 后果预判禁词：E6007 码号 + 「尚未初始化 / not-yet-initialized」式后果从句。
    // 两种语言都查：诊断语言由 error_lang 决定（CI 默认 en），判据不得依赖语言。
    let forbidden = ["E6007", "尚未初始化", "not-yet-initialized", "reads void"];
    for marker in forbidden {
        assert!(
            !diagnostic.message.contains(marker),
            "良性环的基础消息不得预判运行期后果（命中 {marker}），实际：{}",
            diagnostic.message
        );
        assert!(
            !diagnostic.help.contains(marker),
            "良性环的 help 也不得预判运行期后果（命中 {marker}），实际：{}",
            diagnostic.help
        );
    }
}

/// 误报面：无环项目（main → lib 单向）不得出现 W1007。
#[test]
fn test_multifile_acyclic_imports_emit_no_w1007() {
    // Arrange：单向依赖 main → lib
    let lib = "helper: () -> Int = () => 7\n";
    let main = "use lib.{helper}\n\nmain = () => {\n    x = helper()\n}\n";
    let (_dir, entry) = make_script_project(&[("lib.yx", lib), ("main.yx", main)]);

    // Act
    let outcome = run_multi(&entry).expect("MultiFile 形态必须返回产物");

    // Assert：零 W1007（环检测不得把无环的 use 链当环）
    let cycles = diagnostics_with_code(&outcome, "W1007");
    assert!(
        cycles.is_empty(),
        "无环项目不得报 W1007（误报面必须为零）：{:?}",
        cycles
            .iter()
            .map(|(p, d)| (p.display().to_string(), d.message.clone()))
            .collect::<Vec<_>>()
    );
}
