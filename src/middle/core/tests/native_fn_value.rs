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

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::core::ir::{ConstValue, FunctionBody, Instruction, ModuleIR, Operand};
use crate::middle::core::ir_gen::AstToIrGenerator;
use crate::middle::core::verify::verify_loose;
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
    let type_result = checker.check_module(&parsed.module);
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

/// 某函数体里与「别名作值」相关的两种形态：
/// - 原生函数值常量（`ConstValue::NativeFn`）承载的名字；
/// - 零参 `Call` 的目标名（native 常量的既有形态）。
fn alias_value_shapes(
    ir: &ModuleIR,
    func_name: &str,
) -> (Vec<String>, Vec<String>) {
    let func = ir
        .functions
        .iter()
        .find(|f| f.name == func_name)
        .unwrap_or_else(|| panic!("函数 {func_name} 必须在 IR 函数表里"));
    let FunctionBody::Code { blocks, .. } = &func.body else {
        panic!("函数 {func_name} 必须是代码体");
    };
    let mut native_fns = Vec::new();
    let mut zero_arg_calls = Vec::new();
    for block in blocks {
        for instr in &block.instructions {
            match instr {
                Instruction::Load {
                    src: Operand::Const(ConstValue::NativeFn(name)),
                    ..
                } => native_fns.push(name.clone()),
                Instruction::Call {
                    func: Operand::Const(ConstValue::String(name)),
                    args,
                    ..
                } if args.is_empty() => zero_arg_calls.push(name.clone()),
                _ => {}
            }
        }
    }
    (native_fns, zero_arg_calls)
}

/// 原生函数作值 ⇒ `ConstValue::NativeFn`（不是零参 Call）。
#[test]
fn test_native_fn_alias_value_is_native_fn_const() {
    // Arrange：顶层重绑（值位置），不调用
    let ir = compile_ir(
        "use std.io.{println}
main: () -> Void = {
    f = println
}
main()
",
    );

    // Act
    let (native_fns, zero_arg_calls) = alias_value_shapes(&ir, "main");

    // Assert：物化形态唯一是原生函数值常量，零参 Call 不得出现
    assert!(
        native_fns.iter().any(|name| name == "std.io.println"),
        "原生函数作值必须物化为 ConstValue::NativeFn(std.io.println)，实际={native_fns:?}",
    );
    assert!(
        !zero_arg_calls.iter().any(|name| name == "std.io.println"),
        "原生函数作值不得落零参 Call（静默不动作路径），实际零参调用={zero_arg_calls:?}",
    );
}

/// native **常量**作值 ⇒ 零参 `Call`（既有成功形态，逐字节不变）。
#[test]
fn test_native_constant_alias_value_keeps_zero_arg_call() {
    // Arrange
    let ir = compile_ir(
        "use std.math.{PI}
main: () -> Void = {
    p = PI
}
main()
",
    );

    // Act
    let (native_fns, zero_arg_calls) = alias_value_shapes(&ir, "main");

    // Assert
    assert!(
        zero_arg_calls.iter().any(|name| name == "std.math.PI"),
        "native 常量作值必须保持零参 Call(std.math.PI)，实际零参调用={zero_arg_calls:?}",
    );
    assert!(
        !native_fns.iter().any(|name| name == "std.math.PI"),
        "native 常量不得被物化成原生函数值（它不是函数），实际={native_fns:?}",
    );
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

/// 某函数体里 MakeClosure 的目标名（函数代码物化形态）。
fn make_closure_targets(
    ir: &ModuleIR,
    func_name: &str,
) -> Vec<String> {
    let func = ir
        .functions
        .iter()
        .find(|f| f.name == func_name)
        .unwrap_or_else(|| panic!("函数 {func_name} 必须在 IR 函数表里"));
    let FunctionBody::Code { blocks, .. } = &func.body else {
        panic!("函数 {func_name} 必须是代码体");
    };
    let mut closures = Vec::new();
    for block in blocks {
        for instr in &block.instructions {
            if let Instruction::MakeClosure { func, .. } = instr {
                closures.push(func.clone());
            }
        }
    }
    closures
}

/// 模块**成员**作值（`p = io.println`）与别名作值同权 ⇒ 原生函数值常量。
///
/// 成员访问与 `use lib.{f}` 别名是同一判据的两条入口（emit_module_export_value）——
/// 此前成员路径一律生成零参调用：`p("x")` 变成以零实参调用 println（静默不动作）。
#[test]
fn test_native_fn_member_value_is_native_fn_const() {
    // Arrange
    let ir = compile_ir(
        "use std.io
main: () -> Void = {
    p = io.println
}
main()
",
    );

    // Act
    let (native_fns, zero_arg_calls) = alias_value_shapes(&ir, "main");

    // Assert
    assert!(
        native_fns.iter().any(|name| name == "std.io.println"),
        "模块成员作值必须物化为 ConstValue::NativeFn(std.io.println)，实际={native_fns:?}",
    );
    assert!(
        !zero_arg_calls.iter().any(|name| name == "std.io.println"),
        "模块成员作值不得落零参 Call，实际零参调用={zero_arg_calls:?}",
    );
}

/// 模块成员里的**纯 yx 导出**作值 ⇒ MakeClosure（与别名路径同一判据）。
///
/// `q = json.parse` 的 parse 定义在嵌入源里（函数表有条目）；此前成员路径把它
/// 也当 native 名零参调用——`q("true")` 静默拿到被调体零实参的返回值。
#[test]
fn test_yx_std_member_value_is_make_closure() {
    // Arrange
    let ir = compile_ir(
        "use std.json
main: () -> Void = {
    q = json.parse
}
main()
",
    );

    // Act
    let closures = make_closure_targets(&ir, "main");

    // Assert
    assert!(
        closures.iter().any(|name| name == "std.json.parse"),
        "纯 yx std 成员作值必须物化为 MakeClosure(std.json.parse)，实际闭包={closures:?}",
    );
}
