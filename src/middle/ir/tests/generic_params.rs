//! IR 泛型参数名的**扫描面**判据（`ast::extract_generic_param_names` 的调用面）
//!
//! 规范：SPEC type-system §7.1「类型位置指声明中的全部 `Type` 子树」——
//! 形参标注与返回位同属「类型位置」，判定时遍历全部子树、不按位置分情况。
//! 消费方：IR 的 `FunctionIR.generic_params`（单态化请求的泛型签名面）。
//!
//! 覆盖：#528 §4-4——名字**只**出现在返回位的编译期值参数
//! （`f: (N: Int) -> Array(Int, N)`）此前不被收录，返回类型里因此留下
//! 无绑定的自由 `TypeRef("N")`。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_with_probe;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::SemanticProbe;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{FunctionIR, ModuleIR};
use crate::middle::ir::ir_gen::AstToIrGenerator;

/// 源码 → IR：与 IR 快照判据同一条真实生成序列（词法 → 语法 → 三遍类型检查 → ir_gen）。
fn module_ir(source: &str) -> ModuleIR {
    let tokens = tokenize(source).expect("词法分析应成功");
    let parsed = parse_with_probe(&tokens, &SemanticProbe);
    assert!(
        !parsed.has_errors,
        "解析应无错误，实际: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("test");
    let result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    assert!(
        result.diagnostics.is_empty(),
        "类型检查应无诊断，实际: {:#?}",
        result.diagnostics
    );
    let mut generator = AstToIrGenerator::new_with_type_result(
        &result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::scan_one(&parsed.module),
    );
    generator
        .generate_module_ir(&parsed.module)
        .expect("IR 生成应成功")
}

/// 按名取函数 IR（取不到即失败并列出全部函数名，避免静默通过）。
fn function<'a>(
    module: &'a ModuleIR,
    name: &str,
) -> &'a FunctionIR {
    module
        .functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| {
            panic!(
                "IR 里应有函数 {name}；实际函数表: {:?}",
                module.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
            )
        })
}

/// 规范：SPEC §7.1 —— 名字只被**返回位**的类型位置引用时，仍是编译期值参数。
///
/// `f: (N: Int) -> Array(Int, N)`：N 不在任何形参标注里，只在返回位的类型构造
/// 实参位。实现体用 Never 爆炸原理（`assert.assert`: Never <: T）满足该返回类型。
/// 期望：IR 的 `generic_params` 收录 `N`——否则返回类型里的 `TypeRef("N")`
/// 是没有绑定的自由引用，单态化请求（`f(3)` → `f(int64::3)`）也无从匹配。
#[test]
fn test_generic_params_includes_const_param_referenced_only_in_return_type() {
    // Arrange — N 只出现在返回位
    let source = concat!(
        "use std.assert\n",
        "f: (N: Int) -> Array(Int, N) = {\n",
        "    return assert.assert(false, \"unreachable\")\n",
        "}\n",
        "main: () -> Void = {\n",
        "}\n",
    );

    // Act
    let module = module_ir(source);
    let f = function(&module, "f");

    // Assert
    assert_eq!(
        f.generic_params.clone(),
        Some(vec!["N".to_string()]),
        "只被返回位引用的 N 必须进 IR 的 generic_params（否则 TypeRef(\"N\") 无绑定）"
    );
}

/// 规范：SPEC §7.1 —— 负控：名字只在**值位置**出现时不是编译期值参数。
///
/// `g: (N: Int) -> Int = 42` 的 N 未在类型位置被引用 ⇒ 运行期值参数，
/// 不得进 `generic_params`（防止扫描面放宽成「收全部形参名」）。
#[test]
fn test_generic_params_excludes_const_param_never_referenced_in_types() {
    // Arrange — N 不在任何类型位置
    let source = concat!("g: (N: Int) -> Int = 42\n", "main: () -> Void = {\n", "}\n",);

    // Act
    let module = module_ir(source);
    let g = function(&module, "g");

    // Assert
    assert_eq!(
        g.generic_params, None,
        "只在值位置出现的 N 不是编译期值参数 ⇒ generic_params 必须为空"
    );
}

/// 规范 / 病史：谓词应用的实参位是**值**，不得进 `generic_params`。
///
/// 判据锚：RFC-027 §2 / §3.1（谓词应用 = 显式函数调用，形参位与返回位同形）与
/// RFC-027a（谓词应用实参位是编译期表达式，不是类型引用）。
/// 病史：6.9.1 收口实录 §七 第 6 条——`refined_relation_arg` 的 `f`/`g` 被登记成
/// `generic_params: ["N"]` / `["n"]`：**用户声明**的谓词（`Eq` / `SumUpTo`）不在
/// 内置名表里，静态 `SemanticProbe` 答「不是谓词」⇒ 真值参数被当成类型引用。
#[test]
fn test_generic_params_excludes_predicate_application_arguments() {
    // Arrange — 与 `02-type-system/refinement/refined_relation_arg.yx` 同形的两份关系谓词
    let source = concat!(
        "Eq: (a: Int, b: Int) -> Type = { a == b }\n",
        "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }\n",
        "f: (N: Int, n: Eq(n, N)) -> Int = { n }\n",
        "g: (n: Int, s: SumUpTo(n, s)) -> Int = { s }\n",
        "main: () -> Void = {\n",
        "}\n",
    );

    // Act
    let module = module_ir(source);

    // Assert — 两份函数的形参都是运行期值位，不是泛型参数
    for name in ["f", "g"] {
        assert_eq!(
            function(&module, name).generic_params,
            None,
            "{name} 的谓词应用实参是值 ⇒ generic_params 必须为空（谓词名来自声明面扫描，不得由静态探针独立判定）"
        );
    }
}
