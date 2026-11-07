//! WBS 4.10 / #492：原生函数作一等值（常量形态承载名字）
//!
//! 规范来源:
//! - docs/src/rfc/accepted/036-module-system.md §4（std native 导出的注册面：
//!   `std.io.println` 一类函数由 FFI 注册表按限定名分发，IR 函数表里没有它们）
//! - docs/src/dev/architecture/07-equivalence-oracle.md（**静默不动作**属 C 类：
//!   旧行为非成功，可随重构顺路修，条件是先红后绿 + 不改变原先成功程序的表现）
//!
//! 判据形态：`use <mod>.{native_fn}` + `f = <别名>` 的**值位置**必须物化为
//! `Load { src: Const(NativeFn(name)) }`（运行时 `RuntimeValue::NativeFunction`），
//! 而不是落零参 `Call`——后者会把 `f("hello")` 变成「以零实参调用 println」：
//! stdout 空、exit 0、零诊断。
//!
//! 零漂移反向判据：native **常量**（`std.math.PI`）作值必须仍走零参 `Call`，
//! 那是既有成功程序的表现，本修复不得改变它。

use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{ModuleIR};
use crate::middle::ir::ir_gen::AstToIrGenerator;
use crate::middle::ir::verify::verify_loose;
use crate::util::diagnostic::Severity;

/// 源码 → 真实生成序列（lex → parse → typecheck → ir_gen）的直驱探针。
fn compile_ir(source: &str) -> ModuleIR {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<native-fn-value>");
    let type_result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    let errors: Vec<_> = type_result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "夹具必须无类型错误: {errors:?}");
    let mut generator =
        AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
    generator
        .generate_module_ir(&parsed.module)
        .expect("夹具必须生成 IR")
}

/// 不变量（verify.rs:151 同批）：原生函数值常量**不引入 env 槽**——
/// `verify` 的闭包 env 槽数只从 MakeClosure 恢复，本形态没有闭包体；
/// 含本形态的模块必须在宽松口径下零违规。
#[test]
fn test_native_fn_value_module_verifies_loose() {
    // Arrange
    let ir = compile_ir(
        "use std.io.{println}
main: () -> Void = {
    f = println
    f(\"hello\")
}
main()
",
    );

    // Act
    let verified = verify_loose(&ir);

    // Assert
    assert!(
        verified.is_ok(),
        "含原生函数值的模块必须在宽松口径下零违规，实际: {:?}",
        verified.err()
    );
}
