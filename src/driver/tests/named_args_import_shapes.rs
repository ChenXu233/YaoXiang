//! WBS 4.9.10-S2b：三种「导入形态 / 推断接收者」的**命名实参**贯通（D-3 载荷）。
//!
//! 规范来源：SPEC syntax §2.3（命名实参 `name = value`；带名参数名字进契约、以声明
//! 为准）+ type-system §3.5（带名参数可被命名实参引用）+ RFC-011a §3（方法级重载
//! 候选的决议与混编 IR 名）+ RFC-029（跨文件方法调用）。
//!
//! ## 三扇门（coding-rules 第七部分）
//!
//! **条款门**：命名实参按声明期形参名入槽是 SPEC 明文语义；这三种形态都是 P0 认可
//! 的可用写法（S1 矩阵正对照的同族），不是本判据自造的能力。
//!
//! **病史门**（红态实测，S1 矩阵 4.9.10-S1 / revision fdd4a3a0）：
//! - 03 别名导入 + 命名实参：`check=0` / `run=1 E6007`——命名实参被**静默丢弃**，
//!   槽位错配到运行期才炸；
//! - 06 命名空间写法 + 命名实参：同上（`run=1 E6007`）；
//! - 18 推断接收者 + 命名实参：`check=1 E1014`（`available:` 为空）——形参名表
//!   拿不到，编译期假拒绝。
//!
//! **证明门**：判据钉的是「名字随决议载荷直达 IR 生成」这条新通路——载荷缺名字时
//! 这三条会退回到 E1014/E6007 的旧红态（`assert` 的自定义消息里写明了这一点），
//! 且值断言（6.0）同时守住「按名入槽后槽位不再错配」。
//!
//! ## 为什么不是手工构造登记数据
//!
//! 三个测试都经 Driver 的真实多文件通路（Discovery → Registry → Typecheck →
//! GlobalSlotAlloc → IrGeneration → Linking）产出：消费端测试手工构造载荷即静默通道
//! （review 打回）。被调名按**合并 IR 的实际调用名**筛，判据与 D-8「决议名 = IR 最终
//! 函数名」同源。

use super::driver::{make_script_project, run_merged_slot_values, run_multi};
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};

/// lib 单元：`Point.scale_by` 的形参名是 `k`（与调用点写出的名字一致）。
const LIB: &str =
    "Point: Type = { x: Float, y: Float }\n\nPoint.scale_by: (self: &Point, k: Float) -> Float = {\n    return self.x * k\n}\n";

/// 合并 IR 里被调名以 `suffix` 结尾的调用 → `(被调名, 实参数)`（函数体 + init 段）。
///
/// 方法调用在 IR 里就是普通 `Call`：接收者占 `args[0]`，值实参从 `args[1]` 起
/// （`lower/expr/call.rs` 的方法臂），故「命名实参没有被丢弃」= 实参数 2。
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

/// 三形态共用断言：编译通过 + 派发到定义单元 + 命名实参进 IR + 值为 6.0。
fn assert_named_arg_binds_one_cross_unit_method(
    files: &[(&str, &str)],
    shape: &str,
) {
    let (_dir, entry) = make_script_project(files);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "{shape}：命名实参不得编译失败（S1 矩阵红态是 E1014 available 为空 / E6007 运行期错值）：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );
    let module = outcome.module.expect("Linking + mono 必须产出合并 IR");
    let calls = calls_ending_with(&module, "Point.scale_by");
    assert_eq!(
        calls.len(),
        1,
        "{shape}：必须恰好一次调用定义单元的方法（派发名带模块键），实际调用：{calls:?}"
    );
    assert_eq!(
        calls[0].1, 2,
        "{shape}：方法调用必须带接收者 + 命名实参两个实参（命名实参不得被静默丢弃），实际：{calls:?}"
    );
    let values = run_merged_slot_values(module);
    assert!(
        values.iter().any(|value| value == "Float(6.0)"),
        "{shape}：self.x * k = 3.0 * 2.0 应求值为 Float(6.0)（按名入槽后槽位不得错配），实际全局槽位：{values:?}"
    );
}

/// ① 别名导入 + 命名实参（S1 矩阵 03 的转绿判据）。
#[test]
fn test_alias_import_method_named_arg_binds_declaration_parameter_name() {
    // Arrange：`use lib.{Point as P}` 只带进本地名 P，接收者靠构造器写出的 P 定身份
    assert_named_arg_binds_one_cross_unit_method(
        &[
            ("lib.yx", LIB),
            (
                "main.yx",
                "use lib.{Point as P}\n\np = P(3.0, 4.0)\nv = p.scale_by(k = 2.0)\n",
            ),
        ],
        "别名导入 + 命名实参",
    );
}

/// ② 命名空间写法 + 命名实参（S1 矩阵 06 的转绿判据）。
#[test]
fn test_namespace_qualified_method_named_arg_binds_declaration_parameter_name() {
    // Arrange：`use lib` 只登记命名空间前缀，`lib.Point(..)` 的限定写法由前缀解析定身份
    assert_named_arg_binds_one_cross_unit_method(
        &[
            ("lib.yx", LIB),
            (
                "main.yx",
                "use lib\n\np = lib.Point(3.0, 4.0)\nv = p.scale_by(k = 2.0)\n",
            ),
        ],
        "命名空间写法 + 命名实参",
    );
}

/// ③ 推断接收者 + 命名实参（S1 矩阵 18 的转绿判据）。
///
/// 接收者 `p` 的类型不是写出来的注解，而是 `make()` 的返回类型推出来的——`VarInfo`
/// 没有类型身份可查（D-6 只在「注解 / 构造器写出路径」登记），方法键只能按静态类型
/// 名（`Point`）经导入面解析。这正是旧「表单靠 IR 调用名反查名字表」拿不到名字的形态。
#[test]
fn test_inferred_receiver_method_named_arg_binds_declaration_parameter_name() {
    // Arrange：跨单元裸名导入 + 本单元函数返回该类型（接收者类型靠推断）
    let files = [
        ("lib.yx", LIB),
        (
            "main.yx",
            "use lib.{Point}\n\nmake: () -> Point = () => {\n    return Point(3.0, 4.0)\n}\n\np = make()\nv = p.scale_by(k = 2.0)\n",
        ),
    ];
    assert_named_arg_binds_one_cross_unit_method(&files, "推断接收者 + 命名实参");
}
