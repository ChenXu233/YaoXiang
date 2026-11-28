//! 跨单元同名类型的方法：定义名与调用目标同源（RFC-011a §3 / RFC-039 D21 / WBS 7c.3）
//!
//! 规范来源：
//! - RFC-011a §3：同名方法重载按定义序混编（裸名 / `#N`），调用点按决议名派发
//! - RFC-039 D21：`method_def_ordinals` 归 04（P7 批 c）
//! - `docs/src/dev/architecture/04-ssa.md` §span 键控的额外机制「机制三」：
//!   定义名由 typecheck 按声明身份一次性产出、IR 生成只读消费
//!
//! 判据：两个编译单元各定义同名类型 `Temp` 与同名方法 `Temp.tag`（各自单定义，
//! 无重载），入口经内联别名引入 b 的类型。合并 IR 中两个定义名必须各保持**裸名**
//!（不得因跨单元同名而出现 `#N` 位移），调用目标与运行值分别落各自单元——
//! 定义名与调用名同源，不靠定义遍历顺序。
//!
//! 重载（多个候选）与命名实参的判据在同模块直驱夹具
//! `middle::ir::tests::method_def_names`：方法候选表的键是裸 `Type.method`，
//! 跨单元同名类型下的**同名重载候选**会合并在同一键下（既有边界，非本次改动引入），
//! 故跨单元夹具只覆盖单定义形态。

use super::driver::{make_script_project, run_merged_slot_values, run_multi};
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};

/// a 单元：`Temp.tag` = v + 1.0。
const UNIT_A: &str = "Temp: Type = { v: Float }\n\nTemp.tag: (self: &Temp) -> Float = (self) => {\n    self.v + 1.0\n}\n";

/// b 单元：同名类型、同名方法，实现 = v * 10.0（可区分）。
const UNIT_B: &str = "Temp: Type = { v: Float }\n\nTemp.tag: (self: &Temp) -> Float = (self) => {\n    self.v * 10.0\n}\n";

/// 入口：内联别名引入 b 的类型，两侧各调用一次。
const ENTRY: &str = "use a.{Temp}\nuse b.{Temp as TempB}\n\nt = Temp(1.0)\nu = TempB(2.0)\nx = t.tag()\ny = u.tag()\n";

/// 合并 IR 里全部 Call 的被调名（函数体 + init 段）。
fn call_names(merged: &crate::middle::ModuleIR) -> Vec<String> {
    let of = |instr: &Instruction| match instr {
        Instruction::Call {
            func: Operand::Const(ConstValue::String(name)),
            ..
        } => Some(name.clone()),
        _ => None,
    };
    let from_fns = merged.functions.iter().flat_map(|f| f.all_instructions());
    let from_init = merged.init.iter();
    from_fns.chain(from_init).filter_map(of).collect()
}

/// 跨单元同名类型 + 同名方法（单定义）：定义名保持裸名，调用与值各归定义单元。
#[test]
fn test_cross_unit_same_named_method_definitions_keep_bare_names() {
    // Arrange：a / b 各定义 Temp.tag（无重载），入口经别名导入 b 的类型
    let (_dir, entry) =
        make_script_project(&[("a.yx", UNIT_A), ("b.yx", UNIT_B), ("main.yx", ENTRY)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert（定义名）：两个方法定义各保持裸名（单定义不产生 #N），且不带单元前缀
    // 的定义只有结论层看到的这两个——名字来源是 typecheck 的定义身份映射
    assert!(
        outcome.failure.is_none(),
        "跨单元同名方法不得编译失败：{:?}",
        outcome.failure
    );
    let merged = outcome.module.expect("Linking+mono must produce IR");
    let defined: Vec<String> = merged.functions.iter().map(|f| f.name.clone()).collect();
    for bare in ["a.Temp.tag", "b.Temp.tag"] {
        assert_eq!(
            defined.iter().filter(|n| n.as_str() == bare).count(),
            1,
            "定义名必须恰有一份 {bare}（单定义保持裸名），实际定义：{defined:?}"
        );
    }
    assert!(
        !defined.iter().any(|n| n.contains('#')),
        "单定义方法不得出现 #N 混编后缀（定义身份映射按声明身份取裸名），实际定义：{defined:?}"
    );

    // Assert（调用与值）：t 走 a 的 v+1.0=2.0，u 走 b 的 v*10.0=20.0
    let callees = call_names(&merged);
    for expected in ["a.Temp.tag", "b.Temp.tag"] {
        assert_eq!(
            callees.iter().filter(|c| c.as_str() == expected).count(),
            1,
            "调用 {expected} 必须恰好一次，实际调用：{callees:?}"
        );
    }
    let values = run_merged_slot_values(merged);
    for expected in ["Float(2.0)", "Float(20.0)"] {
        assert!(
            values.iter().any(|v| v == expected),
            "运行值必须包含 {expected}（各走定义单元实现），实际全局值：{values:?}"
        );
    }
}
