//! WBS 4.9.9 — **跨单元方法**的命名实参形参名表贯通：驱动级直驱钉测试。
//!
//! 规范来源：SPEC syntax §2.3（命名实参 `name = value`；带名参数**名字进契约、
//! 以声明为准**）+ type-system §3.5（带名参数可被命名实参引用）+ RFC-011a §3
//!（方法级重载候选的决议与混编 IR 名）+ RFC-029（跨文件方法调用）。
//!
//! 机制形态（coding-rules 第六部分「登记-消费」型）：声明期形参名由**定义单元**
//! 登记进 `TypeEnvironment::method_overload_param_names`，经
//! `method_overload_snapshot` → `ModuleInfo::method_overloads` 跨单元携带；
//! 消费端是 IR 生成的 `AstToIrGenerator::lookup_param_info`（按定义单元限定名
//! `{module}.{Type}.{method}[#N]` 取候选组的形参名表）。本文件**不手工构造**任何
//! 注册数据：全部经 Driver 的真实多文件通路（Discovery → Registry → Typecheck →
//! GlobalSlotAlloc → IrGeneration → Linking）产出——生产端与消费端同批直驱；
//! 消费端测试手工构造登记数据即静默通道（review 打回）。
//!
//! 修复前红态（实测）：`error [E1014] Function 'scale_by' has no parameter named 'k'
//! (available: )`——IR 生成侧只按「本单元 `fn_param_names`」与「模块导出面」查名字，
//! 而方法键的首段是**类型名**不是模块键，导出面对它恒不命中。
//!
//! **准入（病史门）**：本判据钉的是一次真实缺陷——跨单元单定义方法的命名实参拿不到
//! 声明期形参名表（`E1014 available` 为空），旧行为为静默丢参后运行期失败；上面的
//! 红态与下面的值断言即病史证据。
//!
//! 范围：只覆盖**裸名导入**（`local == target`）的跨单元方法。别名导入
//! （`use M.{Type as X}`）的候选注入面只认 `local == target` ⇒ 该形态的命名实参
//! 回到先前的静默丢弃，登记为 4.9.9 已知缺口（未修），不在本判据内。

use super::driver::{make_script_project, run_merged_slot_values, run_multi};
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};

/// 合并 IR 里**被调名以 `suffix` 结尾**的调用 → `(被调名, 实参数)`（函数体 + init 段）。
///
/// 方法调用在 IR 里就是普通 `Call`：接收者占 `args[0]`，值实参从 `args[1]` 起
///（`lower/expr/call.rs` 的方法臂）。按被调名而不是源码位置筛查，判据与
/// 「派发名 = 决议出的混编名」这条既有契约同源。
fn calls_ending_with(
    module: &crate::middle::ModuleIR,
    suffix: &str,
) -> Vec<(String, usize)> {
    module
        .functions
        .iter()
        .flat_map(|function| function.all_instructions())
        .chain(module.init.iter())
        .filter_map(|instruction| match instruction {
            Instruction::Call {
                func: Operand::Const(ConstValue::String(name)),
                args,
                ..
            } => Some((name.clone(), args.len())),
            _ => None,
        })
        .filter(|(name, _)| name.ends_with(suffix))
        .collect()
}

/// 单候选跨单元方法：命名实参按**声明期形参名**入槽，接收者与命名实参都进 IR 调用。
///
/// 判别力：名字表拿不到时 IR 生成侧在 `bind_args_to_params` 报 E1014（编译中止、
/// 无合并 IR ⇒ 第一条断言红）；名字表拿对但实参序重建错时调用只带接收者（第二条红）；
/// 实参进 IR 但槽位错位时求值不是 6.0（第三条红）。
#[test]
fn test_cross_unit_method_named_arg_binds_declaration_parameter_name() {
    // Arrange
    let (_dir, entry) = make_script_project(&[
        (
            "lib.yx",
            "Point: Type = { x: Float, y: Float }\n\nPoint.scale_by: (self: &Point, k: Float) -> Float = {\n    return self.x * k\n}\n",
        ),
        (
            "main.yx",
            "use lib.{Point}\n\np = Point(3.0, 4.0)\nv = p.scale_by(k = 2.0)\n",
        ),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "跨单元方法的命名实参不得编译失败（修复前是 E1014 available 为空）：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let module = outcome.module.expect("Linking + mono 必须产出合并 IR");
    let calls = calls_ending_with(&module, "Point.scale_by");
    assert!(
        calls.iter().any(|(_, argc)| *argc == 2),
        "方法调用必须带接收者 + 命名实参两个实参（命名实参不得被丢弃），实际调用：{calls:?}"
    );
    let values = run_merged_slot_values(module);
    assert!(
        values.iter().any(|value| value == "Float(6.0)"),
        "self.x * k = 3.0 * 2.0 应求值为 Float(6.0)，实际全局槽位：{values:?}"
    );
}

/// 两候选重载：命名实参按**形参名**筛候选（k → Float 候选、n → Int 候选），
/// 位置实参按实参类型决议——四条调用各两次落到两个候选上。
///
/// 判别力：名字表缺失 ⇒ 编译中止（第一条红）；按名筛候选若退化成「总取首候选」，
/// 命名 `n` 与位置 `3` 的调用都会落到 Float 候选（裸名 3 次 / 声明序 1 一次，
/// 第二条按次数分布红）；实参序错位则 `Int(30)` 求不出来（第四条红）。
#[test]
fn test_cross_unit_method_named_arg_selects_overload_by_parameter_name() {
    // Arrange：两个候选形参名互不相同（k: Float / n: Int），四处调用
    let (_dir, entry) = make_script_project(&[
        (
            "lib.yx",
            "Point: Type = { x: Float, y: Float }\n\nPoint.scale_by: (self: &Point, k: Float) -> Float = {\n    return self.x * k\n}\n\nPoint.scale_by: (self: &Point, n: Int) -> Int = {\n    return n * 10\n}\n",
        ),
        (
            "main.yx",
            "use lib.{Point}\n\np = Point(3.0, 4.0)\na = p.scale_by(k = 2.0)\nb = p.scale_by(n = 3)\nc = p.scale_by(2.0)\nd = p.scale_by(3)\n",
        ),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "两候选重载的命名实参调用不得编译失败：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let module = outcome.module.expect("Linking + mono 必须产出合并 IR");
    // 混编名是候选序的载体：裸名 = 声明序 0（Float 候选），后缀 #1 = 声明序 1（Int 候选）。
    let first_candidate = calls_ending_with(&module, "Point.scale_by");
    let second_candidate = calls_ending_with(&module, "Point.scale_by#1");
    assert!(
        first_candidate.iter().all(|(_, argc)| *argc == 2)
            && second_candidate.iter().all(|(_, argc)| *argc == 2),
        "四处调用都必须带接收者 + 一个值实参，实际：裸名 {first_candidate:?} / 声明序 1 {second_candidate:?}"
    );
    assert!(
        first_candidate.len() == 2 && second_candidate.len() == 2,
        "命名与位置两侧都要各命中两个候选（裸名 2 次、声明序 1 两次），实际：裸名 {first_candidate:?} / 声明序 1 {second_candidate:?}"
    );
    let values = run_merged_slot_values(module);
    assert!(
        values.iter().any(|value| value == "Float(6.0)"),
        "Float 候选（self.x * k）应求值为 Float(6.0)，实际全局槽位：{values:?}"
    );
    assert!(
        values.iter().any(|value| value == "Int(30)"),
        "Int 候选（n * 10）应求值为 Int(30)，实际全局槽位：{values:?}"
    );
}
