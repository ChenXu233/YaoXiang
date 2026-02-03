//! Syntax validation tests - 语法验证测试
//!
//! RFC-007 函数定义语法统一方案测试

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;

fn check_syntax(input: &str) -> bool {
    match tokenize(input) {
        Ok(tokens) => match parse(&tokens) {
            Ok(_) => true,
            Err(_) => false,
        },
        Err(_) => false,
    }
}

// ============================================================================
// RFC-007 函数定义语法统一方案
// ============================================================================
//
// YaoXiang 采用双层处理策略：解析层宽松放过，类型检查层严格推断。
//
// 【解析层规则】
// - 解析器只验证语法结构，不进行类型推断
// - 缺少类型标注的声明，类型标注字段为 None
// - 所有符合基础语法结构的声明都能通过解析
//
// 【类型检查层规则】
// - 参数类型：有标注用标注，无标注尝试 HM 推断，无法推断则报错
// - 返回类型：从 return 表达式或函数体推断，无法推断则报错
//
// ============================================================================
// 【RFC-007 统一语法】name: (params) -> Return = body
//
// 语法规则：
// 1. 完整形式：name: (a: Int, b: Int) -> Int = (a, b) => { return a + b }
// 2. 省略 Lambda 头：name: (a: Int, b: Int) -> Int = { return a + b }
// 3. 省略参数类型：name: (a, b) -> Int = (a, b) => { return a + b }
// 4. 最简形式：name = (a, b) => { return a + b }
// 5. 空参函数：name: () -> Void = { ... }
// 6. 表达式形式：name: (a: Int, b: Int) -> Int = a + b
//
// 注意：
// - {} 块内必须使用 return 返回值（除返回 Void 外）
// - -> 是函数类型的标志，不能省略
// - 旧语法 name(Params) -> Ret = ... 已不再支持
//
// ============================================================================

#[cfg(test)]
mod syntax_validation_tests {
    use super::*;

    #[test]
    fn test_rfc007_complete_form() {
        // ========== RFC-007 完整形式：签名完整 + Lambda 头完整 ==========
        // name: (a: Type, b: Type) -> Ret = (a, b) => { return ... }

        // 完整形式：多参数
        assert!(check_syntax(
            "add: (a: Int, b: Int) -> Int = (a, b) => a + b"
        ));
        assert!(check_syntax(
            "add: (a: Int, b: Int) -> Int = (a, b) => { return a + b }"
        ));
        assert!(check_syntax(
            "sub: (a: Int, b: Int) -> Int = (a, b) => a - b"
        ));

        // 单参数函数（签名中类型简写）
        assert!(check_syntax("inc: Int -> Int = x => x + 1"));

        // 空参函数
        assert!(check_syntax("empty1: () -> Void = () => {}"));
        assert!(check_syntax("get_answer: () -> Int = () => 42"));

        // 柯里化函数（右结合）
        assert!(check_syntax(
            "add_curried: Int -> Int -> Int = a => b => a + b"
        ));
    }

    #[test]
    fn test_rfc007_omit_lambda_head() {
        // ========== RFC-007 省略 Lambda 头：签名已声明参数 ==========
        // name: (a: Type, b: Type) -> Ret = { return ... }
        // name: (a: Type, b: Type) -> Ret = expression

        // 省略 Lambda 头，使用块体
        assert!(check_syntax(
            "add: (a: Int, b: Int) -> Int = { return a + b }"
        ));
        assert!(check_syntax(
            "mul: (a: Int, b: Int) -> Int = { return a * b }"
        ));

        // 省略 Lambda 头，直接表达式
        assert!(check_syntax("add: (a: Int, b: Int) -> Int = a + b"));
        assert!(check_syntax("mul: (a: Int, b: Int) -> Int = a * b"));
    }

    #[test]
    fn test_rfc007_omit_param_types() {
        // ========== RFC-007 省略参数类型：HM 推断参数类型 ==========
        // name: (a, b) -> Ret = (a, b) => { return ... }

        // 签名中参数无类型标注
        assert!(check_syntax(
            "add: (a, b) -> Int = (a, b) => { return a + b }"
        ));
        assert!(check_syntax("square: (x) -> Int = (x) => { return x * x }"));
    }

    #[test]
    fn test_rfc007_minimal_form() {
        // ========== RFC-007 最简形式：HM 完全推断 ==========
        // name = (a, b) => { return ... }

        // 无类型标注（参数由 HM 推断）
        assert!(check_syntax("add = (a, b) => a + b"));
        assert!(check_syntax("square = (x) => x * x"));
        assert!(check_syntax("foo = (x) => x"));

        // 无类型标注 + return 语句
        assert!(check_syntax("add2 = (a, b) => { return a + b }"));
        assert!(check_syntax("square2 = (x) => { return x * x }"));
        assert!(check_syntax("get_val = () => { return 42 }"));

        // 表达式形式（无 return）
        assert!(check_syntax("get_num = () => 42"));

        // Lambda 参数带类型标注
        assert!(check_syntax("identity = (x: Int) => x"));
        assert!(check_syntax("double = (x: Int) => x * 2"));
    }

    #[test]
    fn test_rfc007_empty_params() {
        // ========== RFC-007 空参函数 ==========
        // name: () -> Void = { ... }

        // 完整形式
        assert!(check_syntax(
            "main: () -> Void = () => { println(\"Hello\") }"
        ));

        // 省略 Lambda 头
        assert!(check_syntax("main: () -> Void = { println(\"Hello\") }"));
        assert!(check_syntax("main: () -> Void = {}"));

        // 最简形式
        assert!(check_syntax("main = { println(\"Hello\") }"));
    }

    #[test]
    fn test_invalid_syntax() {
        // ========== 无效语法测试 ==========
        // 这些语法形式是无效的，应该被解析器拒绝

        // 旧语法已不再支持：name(Params) -> Ret = ...
        assert!(!check_syntax("add(Int, Int) -> Int = (a, b) => a + b"));

        // 缺少 '=' 符号（类型标注后直接跟 lambda 而没有 =）
        assert!(!check_syntax("neg: Int -> Int (a) => -a"));

        // 缺少 '=>' 符号 - 这个实际上是有效的变量声明语法
        // `inc: Int -> Int = a + 1` 声明了一个类型为 Int -> Int 的变量，值为 a + 1
        // 这是有效的语法，类型检查会报错但解析会通过
        assert!(check_syntax("inc: Int -> Int = a + 1"));

        // 缺少函数体
        assert!(!check_syntax("dec: Int -> Int = (a) => "));

        // 参数体不完整
        assert!(!check_syntax("double: Int -> Int =  => x * 2;"));

        // 无效的括号形式
        assert!(!check_syntax(
            "bad_parens: Int, Int -> Int = (a, b) => a + b"
        ));

        // 无效的参数形式
        assert!(!check_syntax("bad_param: (Int)Int -> Int = (a) => a"));
    }

    #[test]
    fn test_rfc007_generic_functions() {
        // ========== RFC-007 泛型函数 ==========
        // 使用 RFC-011 泛型语法 [T]

        // 完整形式
        assert!(check_syntax(
            "identity: [T](x: T) -> T = (x) => { return x }"
        ));

        // 省略 Lambda 头
        assert!(check_syntax("identity: [T](x: T) -> T = { return x }"));

        // 直接表达式
        assert!(check_syntax("identity: [T](x: T) -> T = x"));

        // 带约束的泛型
        assert!(check_syntax("clone: [T: Clone](x: T) -> T = x"));
    }

    #[test]
    fn test_lambda_syntax() {
        // ========== RFC-007 Lambda 表达式语法 ==========
        // | 语法形式 | 返回方式 |
        // |---------|----------|
        // | { statements } | 无 return → Void；有 return → return 的类型 |
        // | expression | 直接返回表达式值 |

        // 单参数 Lambda（可省略括号）
        assert!(check_syntax("inc: Int -> Int = x => x + 1"));

        // 柯里化 Lambda（多箭头，右结合）
        assert!(check_syntax("add: Int -> Int -> Int = a => b => a + b"));

        // 块形式：无 return → Void
        assert!(check_syntax("test = () => { println(\"hi\") }"));

        // 块形式：有 return → return 的类型
        assert!(check_syntax("test = () => { return 42 }"));

        // 表达式形式：直接返回表达式值
        assert!(check_syntax("test = () => 42"));
    }

    #[test]
    fn test_return_syntax() {
        // ========== RFC-007 return 语句语法 ==========
        // {} 块内必须使用 return 返回值（除返回 Void 外）

        // RFC-007 新语法 + return 语句
        assert!(check_syntax(
            "add: (a: Int, b: Int) -> Int = (a, b) => { return a + b }"
        ));
        assert!(check_syntax(
            "square: (x: Int) -> Int = (x) => { return x * x }"
        ));
        assert!(check_syntax("square: Int -> Int = x => { return x * x }"));
        assert!(check_syntax("get_value: () -> Int = () => { return 42 }"));
        assert!(check_syntax(
            "log: (msg: String) -> Void = (msg) => { print(msg); return }"
        ));

        // 多行函数体 + return
        assert!(check_syntax(
            "fact: (n: Int) -> Int = (n) => {
        if n <= 1 {
            return 1
        }
        return n * fact(n - 1)
    }"
        ));

        // 混合表达式和 return
        assert!(check_syntax(
            "max: (a: Int, b: Int) -> Int = (a, b) => {
        if a > b {
            return a
        }
        a + b
    }"
        ));

        // 早期 return
        assert!(check_syntax(
            "early_return: (x: Int) -> Int = (x) => { if x < 0 { return 0 } x }"
        ));
        assert!(check_syntax(
            "multiple_returns: (x: Int) -> Int = (x) => {
        if x < 0 { return 0 }
        if x == 0 { return 1 }
        return x
    }"
        ));
    }

    #[test]
    fn test_rfc007_recursive_function() {
        // ========== RFC-007 递归函数 ==========
        // 使用花括号形式的 if 语句

        assert!(check_syntax(
            "factorial: (n: Int) -> Int = (n) => {
            if n <= 1 { return 1 } else { return n * factorial(n - 1) }
        }"
        ));

        assert!(check_syntax(
            "fib: (n: Int) -> Int = (n) => {
            if n <= 1 { return n } else { return fib(n - 1) + fib(n - 2) }
        }"
        ));
    }

    #[test]
    fn test_rfc007_direct_expression_body() {
        // ========== RFC-007 直接表达式形式 ==========
        // 无需 return，直接返回表达式值

        assert!(check_syntax("add: (a: Int, b: Int) -> Int = a + b"));
        assert!(check_syntax("main: () -> Void = println(\"Hello\")"));
        assert!(check_syntax("identity: [T](x: T) -> T = x"));
    }

    #[test]
    fn test_type_inference_cases() {
        // ========== RFC-007 类型推断测试（解析层） ==========
        // 以下语法在解析层全部放行，类型检查层负责 HM 推断

        // 最简形式：无类型标注
        assert!(check_syntax("id = (x) => x"));
        assert!(check_syntax("apply = (f, x) => f(x)"));
        assert!(check_syntax("apply = (f: Int, x: Int) => f + x"));
        assert!(check_syntax("const = () => 42"));
        assert!(check_syntax("nop = () => {}"));
    }
}
