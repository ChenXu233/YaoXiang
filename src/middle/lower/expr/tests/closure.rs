//! 闭包环境直驱测试 — RFC-009a 自由变量捕获、RFC-024 spawn 与 RFC-039 D22
//!
//! 准入（条款门 + 证明门）：RFC-009a 的柯里 env、RFC-024 的 spawn Move 捕获与
//! RFC-039 D22 的显式环境边界。完整管线用例同时观察 MakeClosure.env 与消费端；
//! 直接 lowering 探针只测收集器，不改变 SPEC 对普通 lambda 隐式捕获的禁止。
//! 闭包由父函数 MakeClosure 关系、形参数和捕获槽定位，不依赖内部命名。
//! B 类先红后绿只声明真实红过的嵌套、错误恢复与 spawn-for 槽位用例；
//! 普通 lambda、列表收集和既有 spawn 形态是回归覆盖，不冒充修复证据。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::{parse, parse_expression};
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::TypeCheckResult;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{FunctionIR, Instruction, ModuleIR, Operand};
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
    captured: &str,
    expected_params: usize,
) {
    let outer_function = ir
        .functions
        .iter()
        .find(|function| function.name == outer)
        .unwrap_or_else(|| panic!("必须生成外层函数 {outer}：{ir:?}"));
    let inner_name = outer_function
        .all_instructions()
        .find_map(|instruction| match instruction {
            Instruction::MakeClosure { func, env, .. } if env.len() == 1 => Some(func.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{outer} 必须生成一项 env 的内层闭包：{ir:?}"));
    let inner_function = ir
        .functions
        .iter()
        .find(|function| function.name == inner_name)
        .unwrap_or_else(|| panic!("{outer} 的内层闭包 {inner_name} 必须在模块函数表：{ir:?}"));
    assert_eq!(
        inner_function.params.len(),
        expected_params,
        "{outer} 的内层闭包形参数必须符合声明结构"
    );
    let captured_slot = inner_function
        .locals()
        .iter()
        .position(|local| local.name.as_deref() == Some(captured))
        .unwrap_or_else(|| panic!("内层闭包必须保留捕获槽 {captured}：{ir:?}"));
    assert!(
        inner_function
            .all_instructions()
            .any(|instruction| matches!(
                instruction,
                Instruction::Load { src: Operand::Local(index), .. } if *index == captured_slot
            )),
        "内层闭包必须从具名 env 槽 {captured_slot} 读取 {captured}，完整 IR={ir:?}"
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
    assert_curry_channel(&ir, "add", "n", 1);

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
    let source = "main = () => { n = 10; items = [1, 2]; spawn for item in items { item + n } }";
    let ir = compile_ir(source);

    // Act
    assert_closure_channel(&ir, 2);

    // Assert
    assert!(
        closure_env_lengths(&ir).contains(&2),
        "spawn-for 必须显式携带迭代值及外层捕获环境"
    );
    let closure = ir
        .functions
        .iter()
        .find(|function| {
            function.params.len() == 1
                && function
                    .all_instructions()
                    .any(|instruction| matches!(instruction, Instruction::LoadUpvalue { .. }))
        })
        .unwrap_or_else(|| panic!("必须按单形参与 upvalue 读取定位 spawn-for 闭包：{ir:?}"));
    let upvalue_indices: Vec<usize> = closure
        .all_instructions()
        .filter_map(|instruction| match instruction {
            Instruction::LoadUpvalue { upvalue_idx, .. } => Some(*upvalue_idx),
            _ => None,
        })
        .collect();
    assert_eq!(
        upvalue_indices,
        vec![1],
        "匿名迭代 env 必须占 0 号槽，外层捕获 n 必须位于其后的 1 号槽"
    );
    let item_slot = closure
        .locals()
        .iter()
        .position(|local| local.name.as_deref() == Some("item"))
        .unwrap_or_else(|| panic!("spawn-for 闭包必须保留 item 形参槽：{ir:?}"));
    assert_eq!(
        item_slot, 2,
        "item 形参必须位于两个 env 槽之后，而非与命名捕获重叠"
    );
    let execution = crate::run(source);
    assert!(
        execution.is_ok(),
        "spawn-for + 外层捕获必须执行成功，不得再把 n 读成 Void：{execution:?}"
    );
}

#[test]
fn test_list_comprehension_inside_lambda_reads_captured_variables() {
    // Arrange
    let ir = compile_ir(
        "make: (n: Int) -> () -> List(Int) = () => [x + n for x in [1, 2]]\nmain = () => { result = make(3)() }",
    );

    // Act
    assert_curry_channel(&ir, "make", "n", 0);

    // Assert
    assert!(
        closure_env_lengths(&ir).contains(&1),
        "列表推导闭包必须在创建点固化外层 n"
    );
}

fn lower_lambda_with_local(
    source: &str,
    captured: &str,
) -> (Vec<Instruction>, FunctionIR) {
    let mut generator = empty_generator();
    let local = generator.next_local_slot();
    generator.register_local(captured, local);
    let expression = parse_expr(source);
    let mut instructions = Vec::new();
    let mut constants = Vec::new();
    let result = generator.next_temp_reg();
    generator
        .generate_expr_ir(&expression, result, &mut instructions, &mut constants)
        .unwrap_or_else(|error| panic!("表达式必须经生产 lowering：{error:?}；{source}"));
    let closure = generator
        .nested_functions
        .last()
        .unwrap_or_else(|| panic!("lambda 必须产生闭包函数：{source}"))
        .clone();
    (instructions, closure)
}

#[test]
fn test_lambda_free_variable_collector_uses_local_binding() {
    // Arrange — 内部 lowering 探针，不改变语言规范的隐式捕获禁令。
    let (instructions, closure) = lower_lambda_with_local("(x) => x + n", "n");

    // Act
    let envs: Vec<usize> = instructions
        .iter()
        .filter_map(|instruction| match instruction {
            Instruction::MakeClosure { env, .. } => Some(env.len()),
            _ => None,
        })
        .collect();
    let uses_upvalue = closure
        .all_instructions()
        .any(|instruction| matches!(instruction, Instruction::LoadUpvalue { .. }));

    // Assert — 条款门：RFC-009a 的闭包 env 与消费者必须同链。
    assert_eq!(envs, vec![1], "普通 lambda 必须收集局部 n 作为 env");
    assert!(uses_upvalue, "普通 lambda 必须从 env 读取 n：{closure:?}");
}

#[test]
fn test_list_comprehension_free_variable_collector_captures_outer_value() {
    // Arrange — 直接驱动 generate_lambda_expr_ir 的自由变量分支。
    let (instructions, closure) = lower_lambda_with_local("() => [x + n for x in [1, 2]]", "n");

    // Act
    let envs: Vec<usize> = instructions
        .iter()
        .filter_map(|instruction| match instruction {
            Instruction::MakeClosure { env, .. } => Some(env.len()),
            _ => None,
        })
        .collect();
    let uses_upvalue = closure
        .all_instructions()
        .any(|instruction| matches!(instruction, Instruction::LoadUpvalue { .. }));

    // Assert — 条款门：ListComp 的自由变量必须进入同一 env 链。
    assert_eq!(envs, vec![1], "列表推导必须把 n 纳入 lambda env");
    assert!(uses_upvalue, "列表推导元素必须从 env 读取 n：{closure:?}");
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
