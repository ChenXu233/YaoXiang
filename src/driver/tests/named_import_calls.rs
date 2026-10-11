//! 钉测试：**具名导入的别名通道**与跨单元调用矩阵。
//!
//! 背景：ir_gen 的 `qualify_names` 用**别名表**把具名导入的调用名改写成限定名
//!（`use lib.{pick}` 的 `pick(7)` → `lib.pick(7)`），而 typecheck 的实例化请求
//! 名是裸名 `pick`。t5 的聚合侧只按单元键限定化（`pick` → `main.pick`，不存在）
//! 且删掉了后缀启发式，于是这类请求无解析路径——mono 误报 E3018（t6 回归）。
//!
//! 本组钉两件事：
//! ① 请求名解析与 IR 调用名**同源**：聚合侧直调 ir_gen 的
//!    `AstToIrGenerator::import_aliases_from(ast, registry)`（`qualify_names`
//!    改写调用名用的同一个实现）取别名表，再走**同一个解析函数**
//!    `AstToIrGenerator::resolve_call_target`——三形态（具名导入 / 整模块导入 /
//!    限定调用）都不得再触发 E3018；
//! ② 跨单元矩阵的多文件执行结果与**单文件同表达式**等价。

use super::driver::{make_script_project, run_multi};

/// 跑合并 IR 的 codegen + 解释器，返回 (全部全局槽位值, 执行错误)。
fn run_codegen(module: crate::middle::ModuleIR) -> (Vec<String>, Option<String>) {
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let file = match ctx.generate() {
        Ok(f) => f,
        Err(d) => return (Vec::new(), Some(format!("codegen error: {d}"))),
    };
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(file);
    let mut interp = crate::backends::interpreter::Interpreter::new();
    use crate::backends::Executor as _;
    let exec = interp
        .execute_module(&bytecode_module)
        .err()
        .map(|e| format!("{e:?}"));
    let values = (0..16)
        .map_while(|i| interp.global_slot(i).map(|v| format!("{v:?}")))
        .collect();
    (values, exec)
}

/// 单文件同表达式的执行结果（等价判据的对照侧）。
fn single_file_values(source: &str) -> Vec<String> {
    let tokens = crate::frontend::core::lexer::tokenize(source).expect("单文件源码可词法分析");
    let parsed = crate::frontend::core::parser::parse(&tokens);
    assert!(
        !parsed.has_errors,
        "单文件源码必须可解析：{:?}",
        parsed.errors
    );
    let mut checker = crate::frontend::core::typecheck::TypeChecker::new("main");
    let tr = checker.check_module_collect_all(&parsed.module);
    assert!(
        tr.diagnostics.is_empty(),
        "单文件对照程序必须类型检查通过：{:?}",
        tr.diagnostics
    );
    let mut generator = crate::middle::core::ir_gen::AstToIrGenerator::new_with_type_result(
        &tr,
        crate::frontend::module::registry::ModuleRegistry::with_std(),
        None,
    );
    let ir = generator
        .generate_module_ir(&parsed.module)
        .expect("单文件 IR 生成必须成功");
    let (values, exec) = run_codegen(ir);
    assert!(exec.is_none(), "单文件对照程序执行不得报错：{exec:?}");
    values
}

/// 编译多文件夹具并断言执行结果与单文件同表达式等价。
fn assert_multi_matches_single(
    files: &[(&str, &str)],
    single_source: &str,
    expected: &str,
    label: &str,
) {
    // Arrange
    let (_dir, entry) = make_script_project(files);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert①：不得编译失败（E3018 回归的直接判据）
    assert!(
        outcome.failure.is_none(),
        "{label}: 多文件编译不得失败（E3018 即别名通道缺失）：{:?}",
        outcome.failure.as_ref().map(|e| format!("{e}"))
    );

    // Assert②：执行结果含期望值
    let (values, exec) = run_codegen(outcome.module.expect("Linking+mono must produce IR"));
    assert!(exec.is_none(), "{label}: 执行不得报错：{exec:?}");
    assert!(
        values.iter().any(|v| v == expected),
        "{label}: 多文件执行应得 {expected}，实际全局值：{values:?}"
    );

    // Assert③：与单文件同表达式等价
    let single = single_file_values(single_source);
    assert!(
        single.iter().any(|v| v == expected),
        "{label}: 单文件对照应得 {expected}，实际：{single:?}"
    );
}

/// 矩阵 1：具名导入（`use lib.{pick}`）+ **函数体内**裸名调用。
#[test]
fn test_named_import_bare_call_in_function_body() {
    assert_multi_matches_single(
        &[
            ("lib.yx", "pick: (T: Type) -> (x: T) -> T = (x) => x\n"),
            (
                "main.yx",
                "use lib.{pick}\n\nmain = () => {\n    return pick(7)\n}\n\nv = pick(7)\n",
            ),
        ],
        "pick: (T: Type) -> (x: T) -> T = (x) => x\n\nmain = () => {\n    return pick(7)\n}\n\nv = pick(7)\n",
        "Int(7)",
        "具名导入·函数体",
    );
}

/// 矩阵 2：具名导入 + **模块顶层**（init 段）裸名调用。
#[test]
fn test_named_import_bare_call_in_module_init() {
    assert_multi_matches_single(
        &[
            ("lib.yx", "pick: (T: Type) -> (x: T) -> T = (x) => x\n"),
            ("main.yx", "use lib.{pick}\nv = pick(7)\n"),
        ],
        "pick: (T: Type) -> (x: T) -> T = (x) => x\nv = pick(7)\n",
        "Int(7)",
        "具名导入·init",
    );
}

/// 矩阵 3：整模块导入（`use lib`）+ **裸名**调用（别名通道的前缀形态）。
#[test]
fn test_whole_module_import_qualified_call() {
    assert_multi_matches_single(
        &[
            (
                "lib.yx",
                "pick: (T: Type) -> (x: T) -> T = (x) => x\nadd1: (x: Int) -> Int = (x) => { x + 1 }\n",
            ),
            ("main.yx", "use lib\nv = lib.add1(41)\n"),
        ],
        "add1: (x: Int) -> Int = (x) => { x + 1 }\nv = add1(41)\n",
        "Int(42)",
        "整模块导入·限定调用",
    );
}

/// 矩阵 4：跨单元泛型的**借用内层**形态（`x: &T`）经具名导入调用。
#[test]
fn test_named_import_borrowed_param_generic() {
    assert_multi_matches_single(
        &[
            ("lib.yx", "peek: (T: Type) -> (x: &T) -> T = (x) => x\n"),
            ("main.yx", "use lib.{peek}\nv = peek(9)\n"),
        ],
        "peek: (T: Type) -> (x: &T) -> T = (x) => x\nv = peek(9)\n",
        "Int(9)",
        "具名导入·借用内层",
    );
}

/// 矩阵 5：跨单元泛型的**双参**形态经具名导入调用。
#[test]
fn test_named_import_two_param_generic() {
    assert_multi_matches_single(
        &[
            (
                "lib.yx",
                "pair: (T: Type, Acc: Type) -> (x: T, acc: Acc) -> T = (x, acc) => x\n",
            ),
            ("main.yx", "use lib.{pair}\nv = pair(5, \"s\")\n"),
        ],
        "pair: (T: Type, Acc: Type) -> (x: T, acc: Acc) -> T = (x, acc) => x\nv = pair(5, \"s\")\n",
        "Int(5)",
        "具名导入·双参",
    );
}

/// 矩阵 6：跨单元**构造器**（类型的具名导入）+ 字段读取。
#[test]
fn test_named_import_constructor() {
    assert_multi_matches_single(
        &[
            ("lib.yx", "Point: Type = { x: Int }\n"),
            ("main.yx", "use lib.{Point}\np = Point(5)\nv = p.x\n"),
        ],
        "Point: Type = { x: Int }\np = Point(5)\nv = p.x\n",
        "Int(5)",
        "具名导入·构造器",
    );
}
