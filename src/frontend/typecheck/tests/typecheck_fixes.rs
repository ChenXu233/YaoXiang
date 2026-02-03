//! 类型检查器修复测试
//!
//! 验证类型检查器修复后的正确行为

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::typecheck::TypeChecker;
use crate::frontend::core::type_system::TypeConstraintSolver;
use crate::util::span::{Position, Span};

fn create_dummy_span() -> Span {
    Span::new(Position::dummy(), Position::dummy())
}

/// 测试函数参数类型检查
#[test]
fn test_fn_param_type_checking() {
    // RFC-007: 参数名在签名中声明
    let code = r#"
        add: (a: Int, b: Int) -> Int = (a, b) => a + b
        main = () => {
            result: Int = add(5, 10)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试函数返回类型一致性
#[test]
fn test_fn_return_type_consistency() {
    // RFC-007: 空参数函数
    let code = r#"
        get_number: () -> Int = () => 42
        get_string: () -> String = () => "hello"
        main = () => {
            num: Int = get_number()
            str: String = get_string()
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试语句块默认返回 void
#[test]
fn test_statement_block_void_return() {
    let code = r#"
        main = () => {
            x: Int = 10
            y: Int = 20
            // 这个块应该隐式返回 Void
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试表达式函数返回
#[test]
fn test_expression_fn_return() {
    // RFC-007: 参数名在签名中
    let code = r#"
        double: (x: Int) -> Int = (x) => x * 2
        main = () => {
            result: Int = double(21)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试函数调用参数类型匹配
#[test]
fn test_fn_call_param_type_matching() {
    // RFC-007: 参数名在签名中
    let code = r#"
        concat: (a: String, b: String) -> String = (a, b) => a + b
        main = () => {
            greeting: String = "Hello, "
            name: String = "World"
            message: String = concat(greeting, name)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试嵌套函数调用
#[test]
fn test_nested_fn_calls() {
    // RFC-007: 参数名在签名中
    let code = r#"
        square: (x: Int) -> Int = (x) => x * x
        cube: (x: Int) -> Int = (x) => x * square(x)
        main = () => {
            result: Int = cube(3)  // 应该是 27
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试复杂块表达式
#[test]
fn test_complex_block_expression() {
    // RFC-007: 参数名在签名中
    let code = r#"
        max: (a: Int, b: Int) -> Int = (a, b) => a
        main = () => {
            larger: Int = max(15, 25)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试函数参数类型注解
#[test]
fn test_fn_param_annotations() {
    // RFC-007: 参数名在签名中
    let code = r#"
        process: (num: Int, text: String, flag: Bool) -> Int = (num, text, flag) => num
        main = () => {
            x: Int = 42
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试函数返回类型注解验证
#[test]
fn test_fn_return_annotation_validation() {
    // RFC-007: 参数名在签名中
    let code = r#"
        add: (a: Int, b: Int) -> Int = (a, b) => {
            result: Int = a + b
            return result
        }
        main = () => {
            result: Int = add(10, 20)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试无参数函数
#[test]
fn test_no_param_function() {
    // RFC-007: 空参数函数
    let code = r#"
        get_number: () -> Int = () => 42
        main = () => {
            y: Int = get_number()
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试函数类型推断
#[test]
fn test_function_type_inference() {
    // RFC-007: 空参数函数
    let code = r#"
        calculate: () -> Int = () => {
            x: Int = 10
            y: Int = 20
            product: Int = x * y
            return product
        }
        main = () => {
            result: Int = calculate()
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试混合表达式和语句块
#[test]
fn test_mixed_expr_stmt_block() {
    // RFC-007: 参数名在签名中
    let code = r#"
        compute: (input: Int) -> Int = (input) => {
            temp: Int = input * 2
            result: Int = temp + 10
            return result
        }
        main = () => {
            value: Int = compute(5)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试嵌套函数作用域
#[test]
fn test_fn_variable_scope() {
    // RFC-007: 参数名在签名中
    let code = r#"
        simple_nested: (x: Int, y: Int) -> Int = (x, y) => {
            temp: Int = x + y
            return temp
        }
        main = () => {
            value: Int = simple_nested(5, 3)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试复杂嵌套块
#[test]
fn test_complex_nested_blocks() {
    // RFC-007: 空参数函数
    let code = r#"
        outer_func: () -> Int = () => {
            inner_block: () -> Int = () => {
                value: Int = 42
                return value
            }
            return inner_block()
        }
        main = () => {
            result: Int = outer_func()
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试块变量类型注解
#[test]
fn test_block_var_type_annotations() {
    // RFC-007: 空参数函数
    let code = r#"
        test_func: () -> Int = () => {
            a: Int = 10
            b: Int = 20
            c: Int = a + b
            c
        }
        main = () => {
            result: Int = test_func()
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new("test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}
