//! 闭包环境直驱测试 — RFC-009a 自由变量捕获、RFC-024 spawn 与 RFC-039 D22
//!
//! 准入（证明门）：生产端必须把词法环境交给闭包，消费端必须经同一环境解析
//! 自由变量；这是与寄存器编号、闭包命名等实现选择无关的正确性性质。测试从源码
//! 走完 lex → parse → typecheck → lower，并同时观察生产端 MakeClosure.env 与
//! 消费端 LoadUpvalue / 柯里具名 env 槽，最低成本地覆盖这条生产—消费链。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::{parse, parse_expression};
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::TypeCheckResult;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{Instruction, ModuleIR, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::util::diagnostic::Severity;

fn compile_ir(source: &str) -> ModuleIR {
    let tokens = tokenize(source)
        .unwrap_or_else(|error| panic!("夹具必须可 tokenize：{error:?}\n源码：\n{source}"));
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误：{:?}\n源码：\n{source}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<closure-env-test>");
    let type_result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    let errors: Vec<_> = type_result
        .diagnostics
        .iter()
        .filter(|diagnostic| matches!(diagnostic.severity, Severity::Error))
        .collect();
    assert!(
        errors.is_empty(),
        "夹具必须无类型错误：{errors:?}\n源码：\n{source}"
    );
    let mut generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::scan_one(&parsed.module),
    );
    generator
        .generate_module_ir(&parsed.module)
        .unwrap_or_else(|errors| panic!("夹具必须生成 IR：{errors:?}\n源码：\n{source}"))
}

fn closure_env_lengths(ir: &ModuleIR) -> Vec<usize> {
    ir.functions
        .iter()
        .flat_map(|function| function.all_instructions())
        .chain(ir.init.iter())
        .filter_map(|instruction| match instruction {
            Instruction::MakeClosure { env, .. } => Some(env.len()),
            _ => None,
        })
        .collect()
}

fn upvalue_load_count(ir: &ModuleIR) -> usize {
    ir.functions
        .iter()
        .flat_map(|function| function.all_instructions())
        .chain(ir.init.iter())
        .filter(|instruction| matches!(instruction, Instruction::LoadUpvalue { .. }))
        .count()
}

fn assert_closure_channel(
    ir: &ModuleIR,
    expected_env_len: usize,
) {
    let env_lengths = closure_env_lengths(ir);
    let upvalue_loads = upvalue_load_count(ir);
    assert!(
        env_lengths.contains(&expected_env_len),
        "生产端必须发出 env 长度 {expected_env_len} 的 MakeClosure，实际={env_lengths:?}"
    );
    assert!(
        upvalue_loads > 0,
        "消费端闭包体必须经 LoadUpvalue 读取显式环境，完整 IR={ir:?}"
    );
}

fn assert_curry_channel(
    ir: &ModuleIR,
    outer: &str,
    inner: &str,
    captured: &str,
) {
    let outer_function = ir
        .functions
        .iter()
        .find(|function| function.name == outer)
        .unwrap_or_else(|| panic!("必须生成外层函数 {outer}：{ir:?}"));
    assert!(
        outer_function
            .all_instructions()
            .any(|instruction| matches!(
                instruction,
                Instruction::MakeClosure { func, env, .. } if func == inner && env.len() == 1
            )),
        "{outer} 必须以一项 env 构造 {inner}，完整 IR={ir:?}"
    );
    let inner_function = ir
        .functions
        .iter()
        .find(|function| function.name == inner)
        .unwrap_or_else(|| panic!("必须生成内层函数 {inner}：{ir:?}"));
    let captured_slot = inner_function
        .locals()
        .iter()
        .position(|local| local.name.as_deref() == Some(captured))
        .unwrap_or_else(|| panic!("{inner} 必须保留捕获槽 {captured}：{ir:?}"));
    assert!(
        inner_function
            .all_instructions()
            .any(|instruction| matches!(
                instruction,
                Instruction::Load { src: Operand::Local(index), .. } if *index == captured_slot
            )),
        "{inner} 必须从具名 env 槽 {captured_slot} 读取 {captured}，完整 IR={ir:?}"
    );
}

fn empty_generator() -> AstToIrGenerator {
    AstToIrGenerator::new_with_type_result(
        &TypeCheckResult::default(),
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::default(),
    )
}

fn parse_expr(source: &str) -> crate::ast::Expr {
    let tokens = tokenize(source)
        .unwrap_or_else(|error| panic!("表达式夹具必须可 tokenize：{error:?}；{source}"));
    parse_expression(&tokens)
        .unwrap_or_else(|error| panic!("表达式夹具必须可解析：{error:?}；{source}"))
}

#[test]
fn test_lambda_free_variable_uses_explicit_environment() {
    // Arrange
    let ir = compile_ir(
        "add: (n: Int) -> (x: Int) -> Int = (x) => x + n\nmain = () => { result = add(10)(5) }",
    );

    // Act
    assert_curry_channel(&ir, "add", "__add_l0", "n");

    // Assert
    assert!(
        closure_env_lengths(&ir).contains(&1),
        "普通 lambda 必须在创建点固化外层 n，而非依赖空调用方状态"
    );
}

#[test]
fn test_spawn_task_uses_synthesized_environment() {
    // Arrange
    let ir = compile_ir("main = () => { n = 10; result = spawn { n + 1; n + 2; return 0 } }");

    // Act
    assert_closure_channel(&ir, 1);

    // Assert
    assert!(
        upvalue_load_count(&ir) >= 2,
        "两个 spawn 合成闭包都必须消费捕获的 n，完整 IR={ir:?}"
    );
}

#[test]
fn test_spawn_for_body_uses_synthesized_environment() {
    // Arrange
    let ir =
        compile_ir("main = () => { n = 10; items = [1, 2]; spawn for item in items { item + n } }");

    // Act
    assert_closure_channel(&ir, 2);

    // Assert
    assert!(
        closure_env_lengths(&ir).contains(&2),
        "spawn-for 必须显式携带迭代值及外层捕获环境"
    );
}

#[test]
fn test_list_comprehension_inside_lambda_reads_captured_variables() {
    // Arrange
    let ir = compile_ir(
        "make: (n: Int) -> () -> List(Int) = () => [x + n for x in [1, 2]]\nmain = () => { result = make(3)() }",
    );

    // Act
    assert_curry_channel(&ir, "make", "__make_l0", "n");

    // Assert
    assert!(
        closure_env_lengths(&ir).contains(&1),
        "列表推导闭包必须在创建点固化外层 n"
    );
}

#[test]
fn test_nested_spawn_captures_through_outer_upvalue() {
    // Arrange
    let ir = compile_ir(
        "main = () => { n = 10; result = spawn { spawn { n + 1; n + 2; return 0 }; return 0 } }",
    );

    // Act
    let env_lengths = closure_env_lengths(&ir);

    // Assert
    assert!(
        env_lengths.iter().filter(|length| **length == 1).count() >= 2,
        "内外两层 spawn 闭包都必须各自携带 n，实际 env={env_lengths:?}"
    );
    assert!(
        upvalue_load_count(&ir) >= 3,
        "外层须先读 n 的 upvalue 供内层再捕获，内层任务再读自己的 n，完整 IR={ir:?}"
    );
}

#[test]
fn test_closure_capture_early_error_does_not_leak() {
    // Arrange — 未解析名字直驱 lowering 的既有 E3006 防御分支。
    let mut generator = empty_generator();
    let local = generator.next_local_slot();
    generator.register_local("n", local);
    let invalid = parse_expr("() => { n; missing }");
    let valid = parse_expr("n");
    let mut instructions = Vec::new();
    let mut constants = Vec::new();
    let invalid_result = generator.next_temp_reg();

    // Act
    let error =
        generator.generate_expr_ir(&invalid, invalid_result, &mut instructions, &mut constants);
    instructions.clear();
    let valid_result = generator.next_temp_reg();
    let valid_generation =
        generator.generate_expr_ir(&valid, valid_result, &mut instructions, &mut constants);

    // Assert
    assert!(error.is_err(), "未解析名字必须触发 lowering 的提前返回路径");
    assert!(
        valid_generation.is_ok(),
        "错误后的正常局部变量生成功能不得受残留捕获影响：{valid_generation:?}"
    );
    assert!(
        instructions
            .iter()
            .all(|instruction| !matches!(instruction, Instruction::LoadUpvalue { .. })),
        "提前返回后不得残留 LoadUpvalue 捕获状态：{instructions:?}"
    );
    assert!(
        instructions.iter().any(|instruction| matches!(
            instruction,
            Instruction::Load { src: Operand::Local(index), .. } if *index == local
        )),
        "错误后的 n 必须仍按普通局部槽 {local} 读取：{instructions:?}"
    );
}
