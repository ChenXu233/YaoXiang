//! 运行期形参槽位口径 — 基于 RFC-039 6.5（表示重塑零行为差分）
//! 与 WBS 8.9.1（参数位形态归一，D53）
//!
//! 判据：参数位的**裸标识符**（`(Int64)` → `Param.name` 是标识符、`ty = None`）
//! **不占运行期形参位**——这是 3e4be8c27 的基线口径；6.5 把 `Type::Fn.params`
//! 从 `Vec<Type>` 重塑为 `Vec<Param>` 时不得改变它。
//!
//! **此口径由 8.9.1 重新裁定；改它必须改这条测试。**

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::core::ir::ModuleIR;
use crate::middle::core::ir_gen::AstToIrGenerator;

/// 源码 → 规范化前的 module IR：与语料快照判据（`tests/snapshot.rs`）同一条
/// lex → parse → typecheck → ir_gen 生成序列。
fn generate_ir(source: &str) -> ModuleIR {
    let tokens = tokenize(source).expect("测试源码词法不应失败");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "测试源码语法必须正确（否则无法区分 bug 与测试写错）: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("bare_param_slots");
    let type_result = checker.check_module(&parsed.module);
    assert!(
        type_result.diagnostics.is_empty(),
        "测试源码类型检查应通过: {:?}",
        type_result.diagnostics
    );
    let mut generator =
        AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
    generator
        .generate_module_ir(&parsed.module)
        .expect("IR 生成应成功")
}

#[test]
fn test_bare_identifier_param_takes_no_runtime_slot() {
    // Arrange & Act：外层参数位是裸标识符 `(Int64)`，内层是带类型形参 `(a: Int64)`
    let module = generate_ir("f: (Int64) -> (a: Int64) -> Int64 = 1");

    // Assert：外层函数不因裸标识符获得运行期形参位（基线 3e4be8c27 口径）
    let outer = module
        .functions
        .iter()
        .find(|f| f.name == "f")
        .expect("应生成外层函数 f");
    assert!(
        outer.params.is_empty(),
        "参数位裸标识符 (Int64) 不占运行期形参位（基线口径；8.9.1 重新裁定前不得改此测试），got: {:?}",
        outer.params
    );

    // Assert：内层带类型形参仍占位（过滤只作用于「不带类型的形参」，不是整条链）
    let inner = module
        .functions
        .iter()
        .find(|f| f.name.starts_with("__f_l"))
        .expect("应生成内层 curry 函数 __f_l0");
    assert_eq!(
        inner.params.len(),
        1,
        "内层带类型形参应保留 1 个运行期形参位，got: {:?}",
        inner.params
    );
}
