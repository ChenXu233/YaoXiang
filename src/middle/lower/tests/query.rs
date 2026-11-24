//! `lower/query.rs` 测试 — #403 A 案 L3 第四门（类型域落点归一）
//!
//! 判据源：[`MonoType::source_container_face`]（mono.rs）的文档注释——源名进
//! 类型域的门共四处（AST 注解 / typecheck 侧 `type_arg_of_expr` / native 签名
//! 串解析 / **L3 IR 视图提取**），四处都调本函数；规范锚：SPEC type-system.md
//! §4.1.1「源名 `List` 进类型域统一落 `Vec` 面」。
//!
//! 落位说明（test-specification 规则 1.1）：`query.rs` 是单文件模块，测试按规范
//! 落父级 `tests/`。判据本体 `type_arg_of_expr_ir` 私有于 `lower::query`（E0624
//! 实测：从父级 tests/ 直调报 `associated function ... is private`），本测试经其
//! **唯一消费点**——`get_expr_mono_type` 的类型值调用臂（`pub(in crate::middle)`，
//! `Vec(Int)(...)` 形态的 IR 静态类型提取臂）——驱动同一数据流：每个类型实参
//! 逐个走 `type_arg_of_expr_ir`，内层 `List(Int)` 的归一发生在同一条
//! `source_container_face` 调用上。

use crate::ast::{Expr, PlanId};
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::{MonoType, TypeCheckResult};
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{ConstValue, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::util::span::Span;

/// t2r 实测 M3（query.rs 类型值调用臂摘除 `source_container_face`）在
/// check/dump/run 三面无可观测差异——第四门的表示面变化（`Generic{"List"}` vs
/// `Generic{"Vec"}`）只有单元层能钉住。本测试就是那颗 pin。
#[test]
fn test_ir_view_type_args_normalize_list_to_vec() {
    // Arrange — 类型值 `Vec(List(Int))` 被调用的 AST 形态（`Vec(List(Int))()`；
    // 类型值调用臂的输入形状）。解析器之外的合成调用记 `PlanId::UNPLANNED`
    // （`Expr::call` 文档约定）。外层运行时实参与本判据无关，留空。谓词命名
    // 空间显式传空集：本判据路径只读 `declared_type_names` 与内建名表，不消费
    // 谓词分类（构造器文档：空集是调用方的显式选择）。
    let list_int = Expr::call(
        Expr::Var("List".to_string(), Span::dummy()),
        vec![Expr::Var("Int".to_string(), Span::dummy())],
        vec![],
        Span::dummy(),
        PlanId::UNPLANNED,
    );
    let vec_list_int = Expr::call(
        Expr::Var("Vec".to_string(), Span::dummy()),
        vec![list_int],
        vec![],
        Span::dummy(),
        PlanId::UNPLANNED,
    );
    let call_of_type_value = Expr::call(
        vec_list_int,
        vec![],
        vec![],
        Span::dummy(),
        PlanId::UNPLANNED,
    );
    let type_result = TypeCheckResult::default();
    let generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::new(),
        None,
        TypeLevelNamespace::default(),
    );

    // Act
    let mono = generator.get_expr_mono_type(&call_of_type_value);

    // Assert — 外层容器名 `Vec` 原样落 Generic 面，**内层** `List(Int)` 也必须
    // 归一到 `Vec` 面（`type_arg_of_expr_ir` 的 Call 臂经 `source_container_face`）。
    // M3 精确逆（摘类型值调用臂的 `source_container_face` 调用，name 退回源名
    // 原样）实测红（cargo test --lib 本测试，M3 挂载时）：
    //   left:  Some(Generic { name: "Vec", args: [Generic { name: "List", args: [Int(64)] }] })
    //   right: Some(Generic { name: "Vec", args: [Generic { name: "Vec",  args: [Int(64)] }] })
    //   `test result: FAILED. 0 passed; 1 failed`
    // 同时域的 L2 pin（frontend::core::types::tests::mono）在 M3 下保持绿——两 pin
    // 各自杀死本面变异（M1），互不搭便车。
    let expected = Some(MonoType::Generic {
        name: "Vec".to_string(),
        args: vec![MonoType::Generic {
            name: "Vec".to_string(),
            args: vec![MonoType::Int(64)],
        }],
    });
    assert_eq!(
        mono, expected,
        "IR 视图的内层 List(Int) 必须归一到 Vec 面（M3 精确逆：摘类型值调用臂的 source_container_face 调用即红——内层退回 Generic{{\"List\"}}，与 L2 表示劈叉）"
    );
}

/// 调用名解析链的 NativeName 臂序 pin（issue #494 收敛后的消费臂之一）。
///
/// `resolve_function_name` 的名字解析优先级：本模块声明名 → use 别名 →
/// **NativeName**（registry 单一查询接口）→ std 短名限定表 → 原样。钉两格：
/// - 点分限定名（该臂的设计输入类）：NativeName 答「是」→ 名字原样保留——
///   若该臂被改回本地硬编或误接别的判据，此格必红；
/// - 裸名（源码可达形态——parser 把点分写法拆成 FieldAccess，Var 恒为裸名）：
///   NativeName 答「否」（无点拆不出模块）→ 链落到 std 短名限定表
///   `println → std.io.println`——若臂序被改（NativeName 提前吞掉短名或
///   限定表被绕过），此格必红。
///
/// 判据本体的形态矩阵钉在 `src/frontend/module/tests/registry.rs`（#494 面板）。
#[test]
fn test_resolve_function_name_native_arm_keeps_chain_order() {
    // Arrange — 空 typecheck 面（本链只读声明名/别名/注册表，不依赖绑定）；
    // 注册表带 std（短名限定表与 native 判据的数据源）。
    let type_result = TypeCheckResult::default();
    let generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::default(),
    );
    let dotted = Expr::Var("std.io.println".to_string(), Span::dummy());
    let bare = Expr::Var("println".to_string(), Span::dummy());

    // Act
    let dotted_resolved = generator.resolve_function_name(&dotted);
    let bare_resolved = generator.resolve_function_name(&bare);

    // Assert — 点分限定名：NativeName 命中，原样保留
    let dotted_name = match dotted_resolved {
        Ok(Operand::Const(ConstValue::String(name))) => name,
        other => panic!("点分限定名必须解析为字符串操作数，实际: {other:?}"),
    };
    assert_eq!(
        dotted_name, "std.io.println",
        "NativeName 对已注册 std 导出（点分形态）必须答「是」并原样保留名字（#494 臂序）"
    );
    // Assert — 裸名：NativeName 不命中（无点），链落到短名限定表
    let bare_name = match bare_resolved {
        Ok(Operand::Const(ConstValue::String(name))) => name,
        other => panic!("裸名必须解析为字符串操作数，实际: {other:?}"),
    };
    assert_eq!(
        bare_name, "std.io.println",
        "裸名 println 必须经 std 短名限定表解析为 std.io.println（NativeName 对无点裸名答「否」，臂序不得改动）"
    );
}
