//! WBS 4.10 / #493：别名在**值位置**的物化判据（导出粒度）
//!
//! 规范来源:
//! - docs/src/rfc/accepted/036-module-system.md §4（嵌入纯 YaoXiang std 模块：
//!   std.test / std.json 等与用户模块走同一前端管道，按限定名进函数表）
//! - docs/src/rfc/accepted/029-*.md §2（use 形态表：use lib.{item} 内联导入别名）
//! - docs/src/dev/architecture/07-equivalence-oracle.md（静默错值属 C 类：先红后绿）
//!
//! 判据形态：use <mod>.{f} + f = <别名> 的**值位置**必须按导出实现来源分流——
//! 函数代码物化为 MakeClosure（函数值是一等值），native 常量/函数保持零参
//! Call。修复前判据按 ModuleSource::Std 一刀切，纯 yx std 导出被误判成
//! native 常量：非泛型静默拿到被调体的返回值、泛型直接运行期 E6007。
//!
//! 三个用例分别钉住：非泛型纯 yx 导出 / 泛型纯 yx 导出 / native 导出（#492 面
//! 本批不动，形态必须逐字节保持零参 Call）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::core::ir::{ConstValue, FunctionBody, Instruction, ModuleIR, Operand};
use crate::middle::core::ir_gen::AstToIrGenerator;
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
    let mut checker = TypeChecker::new("<alias-fn-value>");
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

/// 某函数体里由**别名作值**产生的指令：MakeClosure 的目标名 / 零参 Call 的目标名。
///
/// 只返回这两类形态（它们的名字来自 use 别名解析）；不关心其余指令。
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
    let mut closures = Vec::new();
    let mut zero_arg_calls = Vec::new();
    for block in blocks {
        for instr in &block.instructions {
            match instr {
                Instruction::MakeClosure { func, .. } => closures.push(func.clone()),
                Instruction::Call {
                    func: Operand::Const(ConstValue::String(name)),
                    args,
                    ..
                } if args.is_empty() => zero_arg_calls.push(name.clone()),
                _ => {}
            }
        }
    }
    (closures, zero_arg_calls)
}

/// 非泛型纯 yx std 导出作值 ⇒ MakeClosure（不是零参 Call）。
///
/// use std.json.{parse} 的 parse 定义在嵌入源 src/std/json.yx 里，与用户模块
/// 同走前端管道、按 std.json.parse 进函数表——作值必须物化，否则 f("true")
/// 拿到的是 parse 零参调用的返回值（静默错值）。
#[test]
fn test_non_generic_yx_std_export_alias_value_is_make_closure() {
    // Arrange：顶层重绑（值位置），不调用
    let ir = compile_ir(
        "use std.json.{parse}
main: () -> Void = {
    f = parse
}
main()
",
    );

    // Act
    let (closures, zero_arg_calls) = alias_value_shapes(&ir, "main");

    // Assert：物化路径唯一是 MakeClosure，零参 Call 不得出现
    assert!(
        closures.iter().any(|name| name == "std.json.parse"),
        "纯 yx 导出作值必须物化为 MakeClosure(std.json.parse)，实际闭包={closures:?}",
    );
    assert!(
        !zero_arg_calls.iter().any(|name| name == "std.json.parse"),
        "纯 yx 导出作值不得落零参 Call（静默错值路径），实际零参调用={zero_arg_calls:?}",
    );
}

/// 泛型纯 yx std 导出作值 ⇒ MakeClosure（不是零参 Call）。
///
/// use std.list.{map} 的 map 是泛型函数（(T: Type, R: Type) -> …）。零参调用臂
/// 会让它在运行期以 Void 实参进体（E6007）；物化后由 mono 的「非改写面形态
/// 一律保留原件」兜住泛型本体，间接调用按类型擦除执行。
#[test]
fn test_generic_yx_std_export_alias_value_is_make_closure() {
    // Arrange
    let ir = compile_ir(
        "use std.list.{map}
main: () -> Void = {
    f = map
}
main()
",
    );

    // Act
    let (closures, zero_arg_calls) = alias_value_shapes(&ir, "main");

    // Assert
    assert!(
        closures.iter().any(|name| name == "std.list.map"),
        "泛型纯 yx 导出作值必须物化为 MakeClosure(std.list.map)，实际闭包={closures:?}",
    );
    assert!(
        !zero_arg_calls.iter().any(|name| name == "std.list.map"),
        "泛型纯 yx 导出作值不得落零参 Call（运行期 E6007 路径），实际零参调用={zero_arg_calls:?}",
    );
}

/// native 导出作值保持**零参 Call**（#492 面；本批不动，形态逐字节不变）。
///
/// use std.io.{println} 的 println 是 native 导出。它作值的正确表示（原生函数
/// 作一等值）属独立待裁项——本判据必须把它排除在外，否则会把 #492 提前半修。
#[test]
fn test_native_std_export_alias_value_keeps_zero_arg_call() {
    // Arrange
    let ir = compile_ir(
        "use std.io.{println}
main: () -> Void = {
    g = println
}
main()
",
    );

    // Act
    let (closures, zero_arg_calls) = alias_value_shapes(&ir, "main");

    // Assert
    assert!(
        zero_arg_calls.iter().any(|name| name == "std.io.println"),
        "native 导出作值必须保持零参 Call(std.io.println)，实际零参调用={zero_arg_calls:?}",
    );
    assert!(
        !closures.iter().any(|name| name == "std.io.println"),
        "native 导出作值不得被本判据物化（#492 范围），实际闭包={closures:?}",
    );
}
