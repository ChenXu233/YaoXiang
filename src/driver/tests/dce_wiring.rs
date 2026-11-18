//! DCE 接线判据（#542-D3）：两个 mono 生产入口（单文件 / 多文件）都在
//! mono 成功之后、codegen 之前跑 post-mono DCE。
//!
//! 规范：RFC-011 §7.1（实例化图可达性删除）；RFC-039 D2（职责归属：根集/删除判据
//! 单点在 `middle/passes/dce`，driver 只负责序列位置与错误映射）。
//!
//! 病史：#542 —— 共享层失去唯一 MakeClosure 引用后成为孤立死 IR，post-mono
//! `verify_loose` 因 env 槽数反推失效而报 Dominance 红；接线后 DCE 在 codegen 前
//! 删除不可达函数，红转绿。
//!
//! 本文件的判据都走**生产入口**（Compiler / Driver MultiFile），不用直驱 IR：
//! 只证「pass 本身对」不足以证「接线可达」。

use crate::frontend::compiler::Compiler;
use crate::middle::ir::ir::ModuleIR;
use crate::middle::ir::verify::verify_loose;

use super::driver::{make_script_project, run_multi, run_merged_slot_values};

/// 函数表里是否有该名字（精确匹配）。
fn has(
    module: &ModuleIR,
    name: &str,
) -> bool {
    module.functions.iter().any(|f| f.name == name)
}

/// 函数表里是否有以 `suffix` 结尾的名字（多文件形态的单元限定名用）。
fn has_suffix(
    module: &ModuleIR,
    suffix: &str,
) -> bool {
    module.functions.iter().any(|f| f.name.ends_with(suffix))
}

/// 单文件生产入口：`Compiler` → `Pipeline` → Driver 单文件臂 → mono → DCE。
///
/// 夹具含一个**未被引用的普通函数**和一个被实例化改写的泛型：前者必须被 DCE 删除，
/// 后者证明「改写到实例名后原件不可达」也由 DCE 收口（mono 只负责生成）。
#[test]
fn test_single_file_entry_runs_post_mono_dce() {
    // Arrange：id 泛型触发 mono（否则 NoObligations 跳过整臂，判据空转）
    let source = r#"
id: (T: Type) -> (x: T) -> T = (x) => { x }
dead_helper: () -> Int = () => { 42 }
main = () => { id(1) }
"#;

    // Act
    let ir = Compiler::new()
        .compile("dce_wiring_single.yx", source)
        .expect("单文件生产入口应编译成功");

    // Assert：不可达函数被删除；实例与入口存活；结构仍合法
    assert!(
        !has(&ir, "dead_helper"),
        "未被引用的普通函数必须由 DCE 删除；实际函数表: {:?}",
        ir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    assert!(has(&ir, "main"), "入口 main 必须存活");
    assert!(
        has(&ir, "id(int64)"),
        "实例 id(int64) 必须存活（main 的调用点已指向它）；实际: {:?}",
        ir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    assert!(
        !has(&ir, "id"),
        "全部调用点改写后泛型原件不可达，应由 DCE 删除；实际: {:?}",
        ir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    assert!(
        verify_loose(&ir).is_ok(),
        "DCE 后的单文件 IR 必须通过 verify_loose；实际 {:?}",
        verify_loose(&ir).err()
    );
}

/// 多文件生产入口：Driver `ProgramKind::MultiFile`（Linking merged IR）→ mono → DCE。
///
/// 夹具里 `lib.dead` / `lib.helper` 都没有调用点：多文件函数表同样必须被 DCE 收口，
/// 且运行时行为不变（`v = id(1)` 仍得 1）。
#[test]
fn test_multi_file_entry_runs_post_mono_dce() {
    // Arrange：lib 提供两个不可达函数；main 顶层调用泛型 id 触发 mono
    let files = [
        (
            "lib.yx",
            "dead: () -> Int = () => { 42 }\nhelper: () -> Int = () => { 7 }\n",
        ),
        (
            "main.yx",
            "use lib\nid: (T: Type) -> (x: T) -> T = (x) => { x }\nv = id(1)\n",
        ),
    ];
    let (_dir, entry) = make_script_project(&files);

    // Act
    let outcome = run_multi(&entry).expect("多文件 driver 运行不应失败");
    assert!(
        outcome.failure.is_none(),
        "多文件编译不得失败: {:?}",
        outcome.failure
    );
    let merged = outcome.module.expect("多文件形态必须产出 merged IR");

    // Assert：两单元里的不可达函数都被删；实例存活；IR 合法；行为不变
    assert!(
        !has_suffix(&merged, ".dead") && !has_suffix(&merged, "dead"),
        "lib.dead 不可达，必须由 DCE 删除；实际: {:?}",
        merged.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    assert!(
        !has_suffix(&merged, ".helper"),
        "lib.helper 不可达，必须由 DCE 删除；实际: {:?}",
        merged.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    assert!(
        merged
            .functions
            .iter()
            .any(|f| f.name.contains("id(int64)")),
        "id 的实例必须存活（init 的调用点已指向它）；实际: {:?}",
        merged.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    assert!(
        verify_loose(&merged).is_ok(),
        "DCE 后的 merged IR 必须通过 verify_loose；实际 {:?}",
        verify_loose(&merged).err()
    );
    let values = run_merged_slot_values(merged);
    assert!(
        values.iter().any(|v| v == "Int(1)"),
        "行为必须不变：v = id(1) 应得 Int(1)，实际 {values:?}"
    );
}

/// 红→绿的绿侧（生产入口）：语料 `fn_const_generic` 的孤立共享层接线后消失、
/// post-mono IR 通过 `verify_loose`。
///
/// 红侧（接线前的同一份 IR 报 Dominance）由 t1 在冻结 revision 上实测；接线后
/// DCE 在 codegen 前删掉不可达的 `__f_l0`，verify 的 env 槽数反推恢复自洽。
#[test]
fn test_post_mono_orphan_shared_layer_is_removed_and_verify_green() {
    // Arrange：语料本体（不复制源码，避免夹具与语料漂移）
    let source =
        include_str!("../../../tests/yaoxiang/02-type-system/generics/fn_const_generic.yx");

    // Act
    let ir = Compiler::new()
        .compile("fn_const_generic.yx", source)
        .expect("语料必须通过生产入口编译");

    // Assert：孤儿共享层消失、实例/克隆存活、post-mono IR 合法
    assert!(
        !has(&ir, "__f_l0"),
        "孤立共享层 __f_l0 必须由 DCE 删除（#542 的修复对象）；实际: {:?}",
        ir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    assert!(has(&ir, "f(int64::42)"), "实例必须存活");
    assert!(
        has(&ir, "__f_l0(int64::42)"),
        "逐实例克隆必须存活（实例的 MakeClosure 目标）"
    );
    assert!(
        verify_loose(&ir).is_ok(),
        "接线后 post-mono IR 必须通过 verify_loose（红→绿的绿侧）；实际 {:?}",
        verify_loose(&ir).err()
    );
}
