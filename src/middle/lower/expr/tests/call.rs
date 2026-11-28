//! `lower/expr/call.rs` 的「补 0」降级分支 —— 哪些是合法零初始化、哪些是内部不一致
//!
//! 判据来源：
//! - 语言规范 `docs/src/reference/language-spec/` 类型系统 §4.3（泛型构造调用）
//!   与语料 `tests/yaoxiang/02-type-system/generics/generic_constructor.yx` 的
//!   「空构造」用例（断言 `z.first == 0`）：**空构造** `Pair(Int)()` 的字段取零值、
//!   数据事后赋值。这条零值是被规范与语料依赖的合法初始化，不是兜底，故 lowering
//!   必须继续产出它。
//! - 类型检查的实参元数判据：缺位（既无实参也无默认值）由 E1010
//!   「expected N arguments, found M」拒绝；lowering 只在「类型检查已报错、
//!   IR 仍在生成」的内部不一致状态下才会看到缺位。本文件的直驱夹具**故意**
//!   驱动该状态（类型检查诊断不拦截），用来判定降级分支该不该存在。
//! - RFC-039 D59 C 类（静默给错答案可随重构顺路修）：把静默补 0 改成响亮失败，
//!   条件是**不改变原先成功程序的对外表现**——合法空构造的零值由
//!   `test_struct_empty_constructor_keeps_zero_value_fill` 与语料同提交钉住。

use super::super::call::CallArgs;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{ConstValue, FunctionBody, Instruction, ModuleIR, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::util::diagnostic::Diagnostic;
use crate::util::plan_id::PlanId;
use crate::util::span::Span;

/// 直驱：lex → parse → typecheck（诊断**不拦截**）→ ir_gen。
///
/// 正常编译在类型检查即终止；「类型检查已报错、IR 仍在生成」这个状态只能由
/// 直驱构造——它正是「补 0」降级分支声称服务的状态。返回类型检查诊断与
/// lowering 结果，调用方先钉住前置条件（类型检查确实报了错），再断言 lowering。
fn drive_lowering(source: &str) -> (Vec<Diagnostic>, Result<ModuleIR, Vec<Diagnostic>>) {
    let tokens =
        tokenize(source).unwrap_or_else(|e| panic!("夹具必须可 tokenize：{e}\n源码：\n{source}"));
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误：{:?}\n源码：\n{source}",
        parsed.errors
    );
    let level = TypeLevelNamespace::scan_one(&parsed.module);
    let mut checker = TypeChecker::new("<fill-fallback>");
    let type_result = checker.check_module(&parsed.module, &level);
    let type_diagnostics = type_result.diagnostics.clone();
    let mut generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::scan_one(&parsed.module),
    );
    (
        type_diagnostics,
        generator.generate_module_ir(&parsed.module),
    )
}

/// `func_name` 函数体里 `Load { src: Const(Int(0)) }` 的条数——零值初始化的落点。
fn zero_load_count(
    ir: &ModuleIR,
    func_name: &str,
) -> usize {
    let func = ir
        .functions
        .iter()
        .find(|f| f.name == func_name)
        .unwrap_or_else(|| panic!("函数 {func_name} 必须在 IR 函数表里"));
    let FunctionBody::Code { blocks, .. } = &func.body else {
        panic!("函数 {func_name} 必须是代码体");
    };
    blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter(|instruction| {
            matches!(
                instruction,
                Instruction::Load {
                    src: Operand::Const(ConstValue::Int(0)),
                    ..
                }
            )
        })
        .count()
}

/// 空构造 `Pair(Int)()` 的字段取零值（合法零初始化，必须保留）。
///
/// 语料锚：`tests/yaoxiang/02-type-system/generics/generic_constructor.yx:30-31`
/// 断言 `z.first == 0`——零值可被用户程序观察到，不是内部细节。
#[test]
fn test_struct_empty_constructor_keeps_zero_value_fill() {
    // Arrange：泛型 struct 的两个字段都没有默认值，空构造不给实参
    let source = r#"
Pair: (T: Type) -> Type = {
    first: T,
    second: T,
}

main: () -> Void = {
    z = Pair(Int)()
}
main()
"#;

    // Act
    let (type_diagnostics, lowering) = drive_lowering(source);

    // Assert
    assert!(
        !type_diagnostics.iter().any(|d| d.code == "E1010"),
        "空构造是合法形态（SPEC §4.3），类型检查不得报 E1010：{type_diagnostics:?}"
    );
    let ir = lowering.unwrap_or_else(|errs| panic!("空构造必须生成 IR，实际诊断：{errs:?}"));
    assert_eq!(
        zero_load_count(&ir, "main"),
        2,
        "空构造的两个字段都必须落零值（语料 generic_constructor.yx:31 依赖该行为）"
    );
}

/// 部分构造（`Pair(Int)(1)`）缺位既无实参也无默认值 → 内部错误，不静默补 0。
///
/// 类型检查对同一形态报 E1010（本测试先钉住该前置），故 lowering 看到缺位
/// 只可能是内部不一致——旧行为把它当 `Int(0)` 传下去，错误无声无息。
#[test]
fn test_struct_partial_constructor_without_default_reports_internal_error() {
    // Arrange
    let source = r#"
Pair: (T: Type) -> Type = {
    first: T,
    second: T,
}

main: () -> Void = {
    z = Pair(Int)(1)
}
main()
"#;

    // Act
    let (type_diagnostics, lowering) = drive_lowering(source);

    // Assert
    assert!(
        type_diagnostics.iter().any(|d| d.code == "E1010"),
        "前置：部分构造必须被类型检查按 E1010 拒绝，实际诊断：{type_diagnostics:?}"
    );
    let errors = lowering.expect_err("缺位既无实参也无默认值时必须报内部错误，不得静默补 0");
    let diagnostic = errors
        .iter()
        .find(|d| d.code == "E3005")
        .unwrap_or_else(|| panic!("必须报 E3005（IR 内部错误），实际：{errors:?}"));
    assert!(
        diagnostic.message.contains("Pair")
            && diagnostic.message.contains("second")
            && diagnostic.message.contains("既无实参也无默认值"),
        "诊断消息必须点名结构体、缺失字段与病因，实际：{}",
        diagnostic.message
    );
}

/// 命名式构造（`P(x = 1)`）缺位既无实参也无默认值 → 内部错误，不静默补 0。
#[test]
fn test_named_constructor_missing_field_without_default_reports_internal_error() {
    // Arrange
    let source = r#"
P: Type = {
    x: Int,
    y: Int,
}

main: () -> Void = {
    p = P(x = 1)
}
main()
"#;

    // Act
    let (type_diagnostics, lowering) = drive_lowering(source);

    // Assert
    assert!(
        type_diagnostics.iter().any(|d| d.code == "E1010"),
        "前置：命名式部分构造必须被类型检查按 E1010 拒绝，实际诊断：{type_diagnostics:?}"
    );
    let errors = lowering.expect_err("缺位既无实参也无默认值时必须报内部错误，不得静默补 0");
    let diagnostic = errors
        .iter()
        .find(|d| d.code == "E3005")
        .unwrap_or_else(|| panic!("必须报 E3005（IR 内部错误），实际：{errors:?}"));
    assert!(
        diagnostic.message.contains("结构体 P 的字段 y")
            && diagnostic.message.contains("既无实参也无默认值"),
        "诊断消息必须点名结构体、缺失字段与病因，实际：{}",
        diagnostic.message
    );
}

/// 命名实参未覆盖的形参槽位 → 内部错误（消息含调用名、槽位序号与形参名）。
///
/// 该分支在合法程序里不可达：普通函数调用路径的形参名表**不含可选位**
/// （`fn_param_names` 与导出 `param_names` 的必需下限都是表长），缺位在
/// `bind_args_to_params` 就被规则⑥按 E1010 拒绝；可选位只随**带点导出名**的
/// 签名出现，而这条路径的查找键恒为裸名。故只能直驱 `CallArgs` 覆盖它——
/// 语料 412 条与探针实测均 0 命中，全量语料跑在 `yx_runner` 里。
#[test]
fn test_named_argument_uncovered_slot_reports_internal_error() {
    // Arrange：形参表 (a, b) 的槽位绑定只覆盖了 a
    let mut call = CallArgs::new(&[], &[], Span::default(), PlanId::UNPLANNED);
    let slots = vec![Some(Operand::Local(0)), None];
    let param_names = ["a".to_string(), "b".to_string()];

    // Act
    let error = call
        .require_complete_slots("f", &param_names, slots)
        .expect_err("命名实参未覆盖的槽位必须报内部错误，不得静默补 0");

    // Assert
    assert_eq!(
        error.code, "E3005",
        "必须沿用 IR 内部错误码 E3005，不新造码"
    );
    assert!(
        error.message.contains("调用 f 的实参槽位未覆盖")
            && error.message.contains("槽位 1")
            && error.message.contains("形参 b"),
        "诊断消息必须点名调用名、槽位序号与形参名，实际：{}",
        error.message
    );
}
