use crate::frontend::parser::parse;
use crate::frontend::lexer::tokenize;

fn check_syntax(input: &str) -> bool {
    match tokenize(input) {
        Ok(tokens) => {
            match parse(&tokens) {
                Ok(_) => true,
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

// ============================================================================
// 语法说明
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
// - 参数类型：有标注用标注，无标注尝试推断，无法推断则报错
// - 返回类型：从 return 表达式或函数体推断，无法推断则报错
//
// ============================================================================
// 【标准语法】标识符: 类型 = 表达式
// 这是 YaoXiang 的官方标准语法，所有新代码应使用此形式。
//
// 规则：
// 1. 声明格式：name: (ParamTypes) -> ReturnType = Lambda
// 2. 单参数可省略括号：name: ParamType -> ReturnType = Lambda
// 3. 无参函数：name: () -> ReturnType = Lambda
// 4. 无类型标注：解析层放过，类型检查层推断
//
// 【旧语法（不推荐）】name(Params) -> Ret = Lambda
// 这是为了向后兼容而保留的旧语法糖，不推荐在新代码中使用。
//
// ============================================================================

#[test]
fn test_standard_syntax() {
    // ========== 标准语法：标识符: 类型 = 表达式 ==========
    // 这是官方推荐的语法形式

    // 标准形式：多参数
    assert!(check_syntax("add: (Int, Int) -> Int = (a, b) => a + b"));
    assert!(check_syntax("sub: (Int, Int) -> Int = (a, b) => a - b"));

    // 单参数函数：类型简写（省略括号）
    assert!(check_syntax("inc: Int -> Int = x => x + 1"));

    // 无参函数
    assert!(check_syntax("empty1: () -> Void = () => {}"));
    assert!(check_syntax("get_answer: () -> Int = () => 42"));

    // 多参数函数
    assert!(check_syntax("mul: (Int, Int) -> Int = (a, b) => a * b"));

    // 柯里化函数（右结合）
    assert!(check_syntax("add_curried: Int -> Int -> Int = a => b => a + b"));
}

#[test]
fn test_legacy_syntax() {
    // ========== 旧语法（不推荐，仅向后兼容） ==========
    //
    // 形式：name(Params) -> Ret = Lambda
    // 这是旧版 MoonBit 风格的语法，为了兼容旧代码而保留。
    //
    // 不推荐原因：
    // 1. 与标准语法不一致，增加学习成本
    // 2. 参数类型位置不统一
    // 3. 解析器需要额外处理两种形式
    //
    // 注意：旧语法可以省略返回类型让类型检查器推断

    // 多参数旧语法
    assert!(check_syntax("mul(Int, Int) -> Int = (a, b) => a * b"));

    // 单参数旧语法
    assert!(check_syntax("square(Int) -> Int = (x) => x * x"));

    // 无参旧语法（可省略括号内内容）
    assert!(check_syntax("empty2() -> Void = () => {}"));
    assert!(check_syntax("get_random() -> Int = () => 42"));

    // 旧语法 + Void 返回
    assert!(check_syntax("say_hello() -> Void = () => print(\"hi\")"));

    // 旧语法无参省略类型标注（解析放过，类型检查推断）
    assert!(check_syntax("empty3() = () => {}"));
    assert!(check_syntax("main = () => {}"));
}

#[test]
fn test_inference_syntax() {
    // ========== 类型推断语法（解析层放过） ==========
    //
    // 这些语法形式在解析层全部放过，由类型检查层进行推断
    // 如果推断成功则通过，否则报错

    // 无类型标注的新语法（参数无法推断，会在类型检查层报错）
    // 但解析层放行 - 单参数需要括号
    assert!(check_syntax("add = (a, b) => a + b"));
    assert!(check_syntax("square = (x) => x * x"));
    assert!(check_syntax("foo = (x) => x"));

    // 无类型标注 + return 语句
    assert!(check_syntax("add2 = (a, b) => { return a + b; }"));
    assert!(check_syntax("square2 = (x) => { return x * x; }"));
    assert!(check_syntax("get_val = () => { return 42; }"));

    // 纯表达式（无 return）
    assert!(check_syntax("get_num = () => 42"));
    // 单参数带类型标注在 Lambda 中需要括号
    assert!(check_syntax("identity = (x: Int) => x"));
    assert!(check_syntax("double = (x: Int) => x * 2"));
}

#[test]
fn test_invalid_syntax() {
    // ========== 无效语法测试 ==========
    // 这些语法形式是无效的，应该被解析器拒绝

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
    assert!(!check_syntax("bad_parens: Int, Int -> Int = (a, b) => a + b"));

    // 无效的参数形式
    assert!(!check_syntax("bad_param: (Int)Int -> Int = (a) => a"));
}

#[test]
fn test_lambda_syntax() {
    // ========== Lambda 表达式语法 ==========

    // 单参数 Lambda（可省略括号）
    assert!(check_syntax("inc: Int -> Int = x => x + 1"));

    // 柯里化 Lambda（多箭头，右结合）
    assert!(check_syntax("add: Int -> Int -> Int = a => b => a + b"));
}

#[test]
fn test_return_syntax() {
    // ========== return 语句语法 ==========
    // return 语句用于从函数体中返回一个值

    // 标准语法 + return 语句
    assert!(check_syntax("add: (Int, Int) -> Int = (a, b) => { return a + b; }"));
    assert!(check_syntax("square: Int -> Int = (x) => { return x * x; }"));
    assert!(check_syntax("get_value: () -> Int = () => { return 42; }"));
    assert!(check_syntax("log: (String) -> Void = (msg) => { print(msg); return; }"));

    // 标准语法 + return 语句（多行函数体）
    assert!(check_syntax("fact: Int -> Int = (n) => {
        if n <= 1 {
            return 1;
        }
        return n * fact(n - 1);
    }"));

    // 标准语法 + 混合表达式和 return
    assert!(check_syntax("max: (Int, Int) -> Int = (a, b) => {
        if a > b {
            return a;
        }
        a + b
    }"));

    // 旧语法 + return 语句
    assert!(check_syntax("mul(Int, Int) -> Int = (a, b) => { return a * b; }"));
    assert!(check_syntax("square2(Int) -> Int = (x) => { return x * x; }"));
    assert!(check_syntax("get_random2() -> Int = () => { return 42; }"));

    // 旧语法 + return + Void
    assert!(check_syntax("say_hello2() -> Void = () => { print(\"hi\"); return; }"));

    // return 语句的位置测试
    assert!(check_syntax("early_return: Int -> Int = (x) => { if x < 0 { return 0; } x }"));
    assert!(check_syntax("multiple_returns: Int -> Int = (x) => {
        if x < 0 { return 0; }
        if x == 0 { return 1; }
        return x;
    }"));
}

#[test]
fn test_type_inference_cases() {
    // ========== 类型推断测试（解析层） ==========
    // 以下语法在解析层全部放行，类型检查层负责推断

    // 新语法无标注 - 单参数需要括号
    assert!(check_syntax("id = (x) => x"));
    assert!(check_syntax("apply = (f, x) => f(x)"));
    assert!(check_syntax("const = () => 42"));
    assert!(check_syntax("nop = () => {}"));

    // 旧语法无标注 - 解析放行
    assert!(check_syntax("id2() = (x) => x"));
    assert!(check_syntax("apply2() = (f, x) => f(x)"));
    assert!(check_syntax("const2() = () => 42"));
    assert!(check_syntax("nop2() = () => {}"));
    assert!(check_syntax("add3() = (a, b) => a + b"));

    // 混合形式 - 解析放行
    assert!(check_syntax("partial(Int) = (x) => x"));
    // 单参数带类型标注在旧语法中
    assert!(check_syntax("partial2() = (x: Int) => x"));
}
