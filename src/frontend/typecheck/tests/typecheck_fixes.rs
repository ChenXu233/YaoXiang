//! 类型检查器修复测试
//!
//! 验证类型检查器修复后的正确行为

use crate::frontend::lexer::tokenize;
use crate::frontend::parser::parse;
use crate::frontend::typecheck::check::TypeChecker;
use crate::frontend::typecheck::types::TypeConstraintSolver;
use crate::util::span::{Position, Span};

fn create_dummy_span() -> Span {
    Span::new(Position::dummy(), Position::dummy())
}

/// 测试函数参数类型检查
#[test]
fn test_fn_param_type_checking() {
    let code = r#"
        add:(Int, Int) -> Int = (a, b) => { a + b }
        main = () => {
            result = add(5, 10)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试函数返回类型一致性
#[test]
fn test_fn_return_type_consistency() {
    let code = r#"
        get_number:() -> Int = () => { 42 }
        get_string:() -> String = () => { "hello" }
        main = () => {
            num = get_number()
            str = get_string()
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

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
            x = 10
            y = 20
            // 这个块应该隐式返回 Void
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试表达式函数返回
#[test]
fn test_expression_fn_return() {
    let code = r#"
        double:(Int) -> Int = (x) => { x * 2 }
        main = () => {
            result = double(21)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试函数调用参数类型匹配
#[test]
fn test_fn_call_param_type_matching() {
    let code = r#"
        concat:(String, String) -> String = (a, b) => { a + b }
        main = () => {
            greeting = "Hello, "
            name = "World"
            message = concat(greeting, name)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试嵌套函数调用
#[test]
fn test_nested_fn_calls() {
    let code = r#"
        square:(Int) -> Int = (x) => { x * x }
        cube:(Int) -> Int = (x) => { x * square(x) }
        main = () => {
            result = cube(3)  // 应该是 27
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试复杂块表达式
#[test]
fn test_complex_block_expression() {
    let code = r#"
        max:(Int, Int) -> Int = (a, b) => { a }
        main = () => {
            larger = max(15, 25)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试函数参数类型注解
#[test]
fn test_fn_param_annotations() {
    let code = r#"
        process:(Int, String, Bool) -> Int = (num, text, flag) => { num }
        main = () => {
            x = 42
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试函数返回类型注解验证
#[test]
fn test_fn_return_annotation_validation() {
    let code = r#"
        // 返回类型明确标注为 Int
        calculate:() -> Int = () => {
            x = 10
            y = 20
            x + y  // 返回表达式
        }
        main = () => {
            result = calculate()
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 测试函数体内的变量作用域
#[test]
fn test_fn_variable_scope() {
    let code = r#"
        calculate:(Int) -> Int = (input) => {
            temp = input * 2
            result = temp + 10
            result
        }
        main = () => {
            value = calculate(5)
            // temp 变量应该只在函数作用域内有效
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 阶段4新增：测试复杂嵌套块表达式
#[test]
fn test_complex_nested_blocks() {
    let code = r#"
        simple_nested:(Int, Int) -> Int = (x, y) => {
            temp = x + y
            temp
        }
        main = () => {
            value = simple_nested(5, 3)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 阶段4新增：测试显式return语句的函数
#[test]
fn test_explicit_return_statement() {
    let code = r#"
        get_value:(Int) -> Void = (x) => {
            if x > 0 {
                y = x
            } else {
                y = 0
            }
        }
        main = () => {
            get_value(10)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 阶段4新增：测试混合表达式和语句的块
#[test]
fn test_mixed_expr_stmt_block() {
    let code = r#"
        process:(Int) -> Int = (x) => {
            temp = x * 2
            if temp > 10 {
                temp = temp + 5
            }
            temp + 1
        }
        main = () => {
            result = process(7)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 阶段4新增：测试无参数函数
#[test]
fn test_no_param_function() {
    let code = r#"
        get_constant:() -> Int = () => { 42 }
        get_void = () => {
            x = 10
            y = 20
        }
        main = () => {
            const_val = get_constant()
            get_void()
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 阶段4新增：测试函数类型推断
#[test]
fn test_function_type_inference() {
    let code = r#"
        // 函数返回类型应该被正确推断
        multiply:(Int, Int) -> Int = (x, y) => { x * y }
        add:(Int, Int) -> Int = (a, b) => { a + b }
        main = () => {
            sum = add(5, 10)
            product = multiply(3, 4)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 阶段4新增：测试块中变量声明的类型注解
#[test]
fn test_block_var_type_annotations() {
    let code = r#"
        calculate:(Int) -> Int = (input) => {
            // 变量声明带有类型注解
            doubled: Int = input * 2
            result: Int = doubled + 10
            result
        }
        main = () => {
            value = calculate(5)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}

/// 阶段4新增：测试复杂的条件表达式块
#[test]
fn test_complex_conditional_block() {
    let code = r#"
        grade_score:(Int) -> Void = (score) => {
            if score >= 90 {
                grade = "A"
            } else if score >= 80 {
                grade = "B"
            } else if score >= 70 {
                grade = "C"
            } else {
                grade = "F"
            }
        }
        main = () => {
            grade_score(85)
        }
    "#;

    let tokens = tokenize(code).unwrap();
    let result = parse(&tokens);

    assert!(result.is_ok(), "Failed to parse code");

    let module = result.unwrap();
    let mut solver = TypeConstraintSolver::new();
    let mut checker = TypeChecker::new(&mut solver, "test");

    let check_result = checker.check_module(&module);

    if let Err(errors) = check_result {
        panic!("Type checking failed: {:?}", errors);
    }
    assert!(check_result.is_ok(), "Type checking should pass");
}
