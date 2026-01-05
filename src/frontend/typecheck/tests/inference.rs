//! 类型推断和验证测试
//!
//! 测试类型检查器的推断功能和验证规则

use crate::frontend::lexer::tokenize;
use crate::frontend::parser::ast::Type;
use crate::frontend::parser::parse;
use crate::frontend::typecheck::check::TypeChecker;
use crate::frontend::typecheck::types::TypeConstraintSolver;
use crate::frontend::typecheck::{check_module, MonoType, PolyType, TypeEnvironment};

/// 检查类型推断是否成功
fn check_type_inference(input: &str) -> Result<(), String> {
    let tokens = tokenize(input).map_err(|e| format!("Lexer error: {:?}", e))?;
    let ast = parse(&tokens).map_err(|e| format!("Parse error: {:?}", e))?;
    let mut env = TypeEnvironment::new();

    // Provide minimal built-ins used by tests (e.g., print: String -> Void)
    env.add_var(
        "print".to_string(),
        PolyType::mono(MonoType::Fn {
            params: vec![MonoType::String],
            return_type: Box::new(MonoType::Void),
            is_async: false,
        }),
    );

    check_module(&ast, Some(&mut env))
        .map(|_| ())
        .map_err(|e| format!("Type error: {:?}", e))
}

/// 检查类型推断是否失败（预期错误）
fn check_type_inference_fails(input: &str) -> bool {
    check_type_inference(input).is_err()
}

/// 检查类型是否通过类型检查
fn check_type(input: &str) -> bool {
    let tokens = match tokenize(input) {
        Ok(t) => t,
        Err(_) => return false,
    };

    let module = match parse(&tokens) {
        Ok(m) => m,
        Err(_) => return false,
    };

    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver);

    match checker.check_module(&module) {
        Ok(_) => {
            if checker.has_errors() {
                for err in checker.errors() {
                    println!("Error: {:?}", err);
                }
                false
            } else {
                true
            }
        },
        Err(errors) => {
            for err in errors {
                println!("Error: {:?}", err);
            }
            false
        },
    }
}

/// 检查类型是否应该被拒绝
fn reject_type(input: &str) -> bool {
    !check_type(input)
}

// ============================================================================
// 标准语法测试（完整类型标注）
// ============================================================================

#[test]
fn test_standard_full_annotation() {
    // 完整类型标注应该通过
    assert!(check_type("add: (Int, Int) -> Int = (a, b) => a + b"));
    assert!(check_type("inc: Int -> Int = x => x + 1"));
    assert!(check_type("log: (String) -> Void = (msg) => {}"));
    assert!(check_type("get_val: () -> Int = () => 42"));
    assert!(check_type("empty: () -> Void = () => {}"));
}

#[test]
fn test_single_param_annotation() {
    // 单参数类型标注应该通过
    assert!(check_type("inc: Int -> Int = x => x + 1"));
}

#[test]
fn test_void_return() {
    // Void 返回类型测试
    assert!(check_type("log: (String) -> Void = (msg) => {}"));
}

// ============================================================================
// 新语法推断测试（无类型标注）
// ============================================================================

#[test]
fn test_inference_empty_block() {
    // 空块 {} 推断为 Void
    assert!(check_type_inference("main = () => {}").is_ok());
    assert!(check_type("main = () => {}"));
}

#[test]
fn test_inference_expression_return() {
    // 从表达式推断返回类型
    assert!(check_type_inference("get_num = () => 42").is_ok());
    assert!(check_type_inference("get_str = () => \"hello\"").is_ok());
    assert!(check_type_inference("get_bool = () => true").is_ok());
    assert!(check_type("get_num = () => 42"));
}

#[test]
fn test_inference_with_typed_param() {
    // 有类型标注的参数应该通过
    assert!(check_type_inference("square: (Int) -> Int = (x) => x * x").is_ok());
    assert!(check_type_inference("add_typed: (Int, Int) -> Int = (a, b) => a + b").is_ok());
    assert!(check_type("square: (Int) -> Int = (x) => x * x"));
    assert!(check_type("add_typed: (Int, Int) -> Int = (a, b) => a + b"));
}

#[test]
fn test_inference_untyped_param_fails() {
    // 无类型标注且无法推断的参数应该失败
    assert!(check_type_inference_fails("add = (a, b) => a + b"));
    assert!(check_type_inference_fails("square() = x => x * x"));
    assert!(check_type_inference_fails("foo = x => x"));
    assert!(check_type_inference_fails("id = x => x"));
    assert!(reject_type("add = (a, b) => a + b"));
}

#[test]
fn test_reject_no_param_type_square() {
    // 无类型参数应该被拒绝
    assert!(reject_type("square = (x) => x * x"));
}

#[test]
fn test_reject_no_param_type_foo() {
    // 无类型参数应该被拒绝
    assert!(reject_type("foo = x => x"));
}

#[test]
fn test_reject_no_param_type_print() {
    // 无类型参数应该被拒绝
    assert!(reject_type("print_msg = (msg) => {}"));
}

#[test]
fn test_inference_typed_lambda_param() {
    // Lambda 参数带类型标注应该通过
    assert!(check_type_inference("identity = (x: Int) => x").is_ok());
    assert!(check_type_inference("double = (x: Int) => x * 2").is_ok());
}

// ============================================================================
// 旧语法推断测试
// ============================================================================

#[test]
fn test_legacy_syntax_empty() {
    // 旧语法空函数应该通过
    assert!(check_type_inference("empty3() = () => {}").is_ok());
    assert!(check_type_inference("main = () => {}").is_ok());
    assert!(check_type("empty3() = () => {}"));
}

#[test]
fn test_legacy_empty() {
    // 旧语法空函数测试
    assert!(check_type("empty3() = () => {}"));
}

#[test]
fn test_legacy_syntax_with_return() {
    // 旧语法有返回值应该通过
    assert!(check_type_inference("get_random() = () => 42").is_ok());
    assert!(check_type("get_random() = () => 42"));
}

#[test]
fn test_legacy_return_val() {
    // 旧语法返回值测试
    assert!(check_type("get_random() = () => 42"));
}

#[test]
fn test_legacy_syntax_with_param_type() {
    // 旧语法有参数类型应该通过
    assert!(check_type_inference("square2(Int) = (x) => x * x").is_ok());
    assert!(check_type_inference("mul(Int, Int) = (a, b) => a * b").is_ok());
    assert!(check_type("square2(Int) = (x) => x * x"));
}

#[test]
fn test_legacy_param_type() {
    // 旧语法参数类型测试
    assert!(check_type("square2(Int) = (x) => x * x"));
}

#[test]
fn test_legacy_full_params() {
    // 旧语法完整参数测试
    assert!(check_type("mul(Int, Int) = (a, b) => a * b"));
}

#[test]
fn test_legacy_syntax_untyped_param_fails() {
    // 旧语法无参数类型应该失败
    assert!(check_type_inference_fails("square3() = (x) => x * x"));
    assert!(check_type_inference_fails("mul3() = (a, b) => a * b"));
    assert!(check_type_inference_fails("id2() = x => x"));
}

// ============================================================================
// 部分推断测试（参数有标注，返回类型推断）
// ============================================================================

#[test]
fn test_legacy_partial_infer_two_params() {
    // 旧语法，参数有标注，返回推断
    assert!(check_type("add(Int, Int) = (a, b) => a + b"));
}

#[test]
fn test_legacy_partial_infer_single_param() {
    // 旧语法，参数有标注，返回推断
    assert!(check_type("square(Int) = (x) => x * x"));
}

#[test]
fn test_legacy_partial_infer_no_params() {
    // 旧语法，无参数，返回推断
    assert!(check_type("get_random() = () => 42"));
}

#[test]
fn test_legacy_partial_infer_void() {
    // 旧语法，返回推断为 Void
    assert!(check_type("log(String) = (msg) => {}"));
}

// ============================================================================
// return 语句测试
// ============================================================================

#[test]
fn test_return_with_full_annotation() {
    // 有标注 + return 应该通过
    assert!(check_type_inference("add: (Int, Int) -> Int = (a, b) => { return a + b; }").is_ok());
    assert!(check_type_inference("square: Int -> Int = (x) => { return x * x; }").is_ok());
    assert!(check_type_inference("get_value: () -> Int = () => { return 42; }").is_ok());
    assert!(
        check_type_inference("log: (String) -> Void = (msg) => { print(msg); return; }").is_ok()
    );
}

#[test]
fn test_return_stmt_annotated() {
    // 有标注 + return 应该通过
    assert!(check_type(
        "add: (Int, Int) -> Int = (a, b) => { return a + b; }"
    ));
}

#[test]
fn test_return_without_annotation() {
    // 无标注 + return 应该通过（从 return 推断类型）
    assert!(check_type_inference("get = () => { return 42; }").is_ok());
    assert!(check_type_inference("get_str = () => { return \"hello\"; }").is_ok());
}

#[test]
fn test_return_stmt_inferred() {
    // return 推断测试
    assert!(check_type("get = () => { return 42; }"));
}

#[test]
fn test_return_untyped_param_fails() {
    // 无标注参数 + return 应该失败
    assert!(check_type_inference_fails(
        "add = (a, b) => { return a + b; }"
    ));
    assert!(check_type_inference_fails(
        "square = (x) => { return x * x; }"
    ));
    assert!(reject_type("add = (a, b) => { return a + b; }"));
}

#[test]
fn test_reject_return_stmt_no_annotation() {
    // 无参数标注 + return 应该被拒绝
    assert!(reject_type("add = (a, b) => { return a + b; }"));
}

#[test]
fn test_early_return() {
    // 早期 return 应该通过
    assert!(
        check_type_inference("early: Int -> Int = (x) => { if x < 0 { return 0; } x }").is_ok()
    );
    assert!(check_type(
        "early: Int -> Int = (x) => { if x < 0 { return 0; } x }"
    ));
}

#[test]
fn test_legacy_return_syntax() {
    // 旧语法 + return 应该通过
    assert!(check_type_inference("mul(Int, Int) -> Int = (a, b) => { return a * b; }").is_ok());
    assert!(check_type_inference("square2(Int) -> Int = (x) => { return x * x; }").is_ok());
    assert!(check_type_inference("get_random2() -> Int = () => { return 42; }").is_ok());
    assert!(
        check_type_inference("say_hello2() -> Void = () => { print(\"hi\"); return; }").is_ok()
    );
}

// ============================================================================
// 递归函数测试
// ============================================================================

#[test]
fn test_recursive_factorial() {
    // 递归函数，参数有类型标注
    assert!(check_type(
        "fact: Int -> Int = (n) => if n <= 1 { 1 } else { n * fact(n - 1) }"
    ));
}

#[test]
fn test_recursive_fibonacci() {
    // 递归函数
    assert!(check_type(
        "fib: Int -> Int = (n) => if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }"
    ));
}

#[test]
fn test_complex_function_with_inference() {
    // 复杂函数（有类型标注）应该通过
    let code = r#"
fact: Int -> Int = (n) => {
    if n <= 1 {
        return 1;
    }
    return n * fact(n - 1);
}
"#;
    assert!(check_type_inference(code).is_ok());
}

// ============================================================================
// 柯里化函数测试
// ============================================================================

#[test]
fn test_curried_function() {
    // 柯里化函数应该通过
    let input = "add_curried: Int -> Int -> Int = a => b => a + b";
    eprintln!("INPUT: {}", input);
    let tokens = tokenize(input).unwrap();
    eprintln!("TOKENS: {:?}", tokens);
    let ast = parse(&tokens).unwrap();
    eprintln!("AST: {:?}", ast);
    let result = check_type_inference(input);
    if let Err(e) = &result {
        eprintln!("CURRIED FUNCTION ERROR: {:?}", e);
    }
    assert!(result.is_ok());
    assert!(check_type_inference(
        "multiply_curried: Int -> Int -> Int -> Int = a => b => c => a * b * c"
    )
    .is_ok());
    assert!(check_type(
        "add_curried: Int -> Int -> Int = a => b => a + b"
    ));
}

#[test]
fn test_curried_add() {
    // 柯里化函数测试
    assert!(check_type(
        "add_curried: Int -> Int -> Int = a => b => a + b"
    ));
}

#[test]
fn test_curried_partial() {
    // 部分应用柯里化函数
    assert!(check_type(
        "add_curried: Int -> Int -> Int = a => b => a + b"
    ));
}

#[test]
fn test_make_adder() {
    // 返回函数的函数
    assert!(check_type(
        "make_adder: Int -> (Int -> Int) = x => y => x + y"
    ));
}

// ============================================================================
// 高阶函数测试
// ============================================================================

#[test]
fn test_higher_order_function() {
    // 高阶函数应该通过
    assert!(check_type_inference("apply: ((Int) -> Int, Int) -> Int = (f, x) => f(x)").is_ok());
    assert!(check_type_inference(
        "compose: ((Int) -> Int, (Int) -> Int) -> (Int) -> Int = (f, g) => x => f(g(x))"
    )
    .is_ok());
    assert!(check_type(
        "apply: ((Int) -> Int, Int) -> Int = (f, x) => f(x)"
    ));
}

#[test]
fn test_higher_order_apply() {
    // 高阶函数测试
    assert!(check_type(
        "apply: ((Int) -> Int, Int) -> Int = (f, x) => f(x)"
    ));
}

#[test]
fn test_higher_order_compose() {
    // 函数组合测试
    assert!(check_type(
        "compose: ((Int) -> Int, (Int) -> Int) -> (Int) -> Int = (f, g) => x => f(g(x))"
    ));
}

#[test]
fn test_higher_order_map() {
    // 高阶函数处理列表测试
    assert!(check_type(
        "map: ((Int) -> Int, List[Int]) -> List[Int] = (f, xs) => xs"
    ));
}

// ============================================================================
// 类型解析测试
// ============================================================================

#[test]
fn test_fn_type_with_fn_return() {
    // 直接测试类型解析
    use crate::frontend::lexer::tokenize;
    use crate::frontend::parser::ParserState;

    // 测试 int -> (int -> int) 的类型解析
    let tokens = tokenize("int -> (int -> int)").unwrap();
    let mut state = ParserState::new(&tokens);
    let result = state.parse_type_anno();
    assert!(result.is_some());

    // 应该解析为 Fn { params: [Int], return_type: Fn { params: [Int], return_type: Int } }
    match &result {
        Some(Type::Fn {
            params,
            return_type,
        }) => {
            assert_eq!(params.len(), 1);
            // 返回类型应该是 Fn 类型
            assert!(
                matches!(**return_type, Type::Fn { .. }),
                "Expected Fn type, got {:?}",
                return_type
            );
        },
        _ => panic!("Expected Fn type, got {:?}", result),
    }
}

// ============================================================================
// 类型不匹配检测测试
// ============================================================================

#[test]
fn test_reject_type_mismatch_binary_op() {
    // 类型不匹配：Int + String 应该报错
    assert!(reject_type(
        "bad_add: (Int, String) -> Int = (a, b) => a + b"
    ));
}

#[test]
fn test_reject_type_mismatch_return() {
    // 返回类型不匹配：应该返回 String 但返回了 Int
    assert!(reject_type(
        "bad_return: Int -> String = (x) => { return 42; }"
    ));
}

// ============================================================================
// 复杂控制流测试
// ============================================================================

#[test]
fn test_while_loop() {
    // while 循环测试
    assert!(check_type("sum_to: Int -> Int = (n) => { i = 0; total = 0; while i < n { total = total + i; i = i + 1; }; total }"));
}

// ============================================================================
// 列表和元组测试
// ============================================================================

#[test]
fn test_tuple_return() {
    // 返回元组测试
    assert!(check_type(
        "divmod: (Int, Int) -> (Int, Int) = (a, b) => (a / b, a % b)"
    ));
}

#[test]
fn test_nested_tuple_return() {
    // 返回嵌套元组测试
    assert!(check_type(
        "get_point: () -> (Int, (Float, Float)) = () => (0, (1.0, 2.0))"
    ));
}

// ============================================================================
// 条件表达式测试
// ============================================================================

#[test]
fn test_conditional_expression() {
    // 条件表达式测试
    assert!(check_type(
        "max: (Int, Int) -> Int = (a, b) => if a > b { a } else { b }"
    ));
}

#[test]
fn test_elif_expression() {
    // 多分支条件测试
    assert!(check_type(
        "sign: Int -> Int = (n) => if n < 0 { -1 } elif n == 0 { 0 } else { 1 }"
    ));
}

// ============================================================================
// Lambda 类型标注测试
// ============================================================================

#[test]
fn test_lambda_with_param_annotation() {
    // Lambda 参数带类型标注测试
    assert!(check_type(
        "add: (Int, Int) -> Int = (a: Int, b: Int) => a + b"
    ));
}

// ============================================================================
// 变量声明类型测试
// ============================================================================

#[test]
fn test_variable_with_annotation() {
    // 变量声明带类型标注测试
    assert!(check_type("x: Int = 42"));
}

#[test]
fn test_variable_inferred() {
    // 变量声明类型推断测试
    assert!(check_type("y = 42"));
}

// ============================================================================
// 无效语法测试（解析层面拒绝）
// ============================================================================

#[test]
fn test_invalid_missing_equals() {
    // 缺少 '=' 符号应该被解析器拒绝
    assert!(check_type_inference_fails("neg: Int -> Int (a) => -a"));
}

#[test]
fn test_invalid_missing_arrow() {
    // 缺少 '=>' 符号 - 这个实际上是有效的变量声明
    // 解析会通过，类型检查会报错
    assert!(check_type_inference_fails("inc: Int -> Int = a + 1"));
}

#[test]
fn test_invalid_missing_body() {
    // 缺少函数体应该被拒绝
    assert!(check_type_inference_fails("dec: Int -> Int = (a) => "));
    assert!(check_type_inference_fails(
        "missing_body: Int -> Int = => 42"
    ));
}

#[test]
fn test_invalid_bad_parens() {
    // 错误的括号形式应该被拒绝
    assert!(check_type_inference_fails(
        "bad_parens: Int, Int -> Int = (a, b) => a + b"
    ));
}

// ============================================================================
// 泛型函数测试
// ============================================================================

#[test]
fn test_generic_function() {
    // 泛型函数应该通过
    assert!(check_type_inference("identity: <T> (T) -> T = x => x").is_ok());
    assert!(check_type_inference("first: <A, B> ((A, B)) -> A = (a, b) => a").is_ok());
}

// ============================================================================
// 边界情况测试
// ============================================================================

#[test]
fn test_unit_type_return() {
    // 显式返回 Void 测试
    assert!(check_type("do_nothing: () -> Void = () => {}"));
}

#[test]
fn test_single_param_parens() {
    // 单参数带括号测试
    assert!(check_type("inc: (Int) -> Int = (x) => x + 1"));
}

#[test]
fn test_three_params() {
    // 三个参数测试
    assert!(check_type(
        "sum3: (Int, Int, Int) -> Int = (a, b, c) => a + b + c"
    ));
}

#[test]
fn test_no_param_annotation() {
    // 无参数类型标注测试
    assert!(check_type("get_val: () -> Int = () => 42"));
}

#[test]
fn test_empty_body() {
    // 空函数体测试
    assert!(check_type("empty: () -> Void = () => {}"));
}
