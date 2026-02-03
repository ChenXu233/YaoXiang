//! RFC-007 旧语法拒绝测试
//!
//! 验证解析器正确拒绝已废弃的旧语法并接受 RFC-007 新语法
//!
//! 旧语法（已不再支持）：
//!   name(ParamTypes) -> ReturnType = Lambda
//!
//! RFC-007 新语法：
//!   name: (param: Type) -> ReturnType = body

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;

#[cfg(test)]
mod old_syntax_rejection_tests {
    use super::*;

    #[test]
    fn test_old_syntax_rejection() {
        // 旧语法应该在解析层被拒绝
        // 旧语法特征：name(Types) -> Ret = ...
        let old_syntaxes = vec![
            r#"add(Int, Int) -> Int = (a, b) => a + b"#,
            r#"main() -> Int = () => 42"#,
            r#"square(Int) -> Int = (x) => x * x"#,
            r#"factorial(Int) -> Int = (n) => if n <= 1 { 1 } else { n * factorial(n - 1) }"#,
        ];

        for syntax in old_syntaxes {
            let tokens = tokenize(syntax).unwrap();
            let result = parse(&tokens);

            assert!(result.is_err(), "旧语法 '{}' 应该被拒绝", syntax);
        }
    }

    #[test]
    fn test_rfc007_new_syntax_acceptance() {
        // RFC-007 新语法应该被接受（通过解析）
        let new_syntaxes = vec![
            // 完整形式：签名完整 + Lambda 头完整
            r#"add: (a: Int, b: Int) -> Int = (a, b) => { return a + b }"#,
            // 省略 Lambda 头
            r#"add: (a: Int, b: Int) -> Int = { return a + b }"#,
            // 直接表达式
            r#"add: (a: Int, b: Int) -> Int = a + b"#,
            // 空参函数
            r#"main: () -> Void = () => { println("Hello") }"#,
            r#"main: () -> Void = { println("Hello") }"#,
            r#"main: () -> Void = {}"#,
            // 最简形式
            r#"add = (a, b) => a + b"#,
            r#"main = { println("Hello") }"#,
            // 泛型函数
            r#"identity: [T](x: T) -> T = x"#,
        ];

        for syntax in new_syntaxes {
            let tokens = tokenize(syntax).unwrap();
            let result = parse(&tokens);

            assert!(
                result.is_ok(),
                "RFC-007 新语法 '{}' 应该通过解析，错误: {:?}",
                syntax,
                result.err()
            );
        }
    }

    #[test]
    fn test_rfc007_syntax_table_examples() {
        // RFC-007 语法规则表中的所有场景
        let syntax_table_examples = vec![
            // | 完整形式 | name: (a: Type, b) -> Ret = (a, b) => { return ... } |
            r#"test: (a: Int, b: Int) -> Int = (a, b) => { return a + b }"#,
            // | 省略 Lambda 头 | name: (a: Type, b) -> Ret = { return ... } |
            r#"test: (a: Int, b: Int) -> Int = { return a + b }"#,
            // | 最简形式 | name = (a, b) => { return ... } |
            r#"test = (a, b) => { return a + b }"#,
            // | 空参完整 | name: () -> Void = () => { return ... } |
            r#"test: () -> Void = () => { return println("hi") }"#,
            // | 空参简写 | name: () -> Void = { return ... } |
            r#"test: () -> Void = { return println("hi") }"#,
            // | 空参最简 | name = { return ... } |
            r#"test = { return println("hi") }"#,
        ];

        for syntax in syntax_table_examples {
            let tokens = tokenize(syntax).unwrap();
            let result = parse(&tokens);

            assert!(
                result.is_ok(),
                "RFC-007 语法规则表示例 '{}' 应该通过解析，错误: {:?}",
                syntax,
                result.err()
            );
        }
    }

    #[test]
    fn test_parameter_name_mismatch_rejection() {
        // RFC-007: 当同时提供签名参数和 lambda 参数时，参数名必须匹配
        // 如果不匹配，应该产生明确的错误

        let mismatch_cases = vec![
            // 参数名不匹配
            (
                r#"add: (a: Int, b: Int) -> Int = (x, y) => x + y"#,
                "参数名不匹配：签名 (a, b)，lambda (x, y)",
            ),
            // 单参数不匹配
            (
                r#"square: (n: Int) -> Int = (x) => x * x"#,
                "参数名不匹配：签名 (n)，lambda (x)",
            ),
            // 部分匹配也应该报错
            (
                r#"test: (a: Int, b: Int, c: Int) -> Int = (a, x, c) => a + x + c"#,
                "参数名不匹配：签名 (a, b, c)，lambda (a, x, c)",
            ),
        ];

        for (syntax, desc) in mismatch_cases {
            let tokens = tokenize(syntax).unwrap();
            let result = parse(&tokens);

            assert!(
                result.is_err(),
                "{} - 代码 '{}' 应该因参数名不匹配被拒绝",
                desc,
                syntax
            );

            // 检查错误信息是否包含 "mismatch" 关键词
            if let Err(error) = result {
                let msg = format!("{:?}", error);
                assert!(
                    msg.contains("mismatch") || msg.contains("Mismatch"),
                    "{} - 错误信息应该包含 'mismatch' 关键词，实际: {}",
                    desc,
                    msg
                );
            }
        }
    }

    #[test]
    fn test_parameter_count_mismatch_rejection() {
        // RFC-007: 参数数量也必须匹配
        let count_mismatch_cases = vec![
            // 签名有 2 个参数，lambda 只有 1 个
            (
                r#"add: (a: Int, b: Int) -> Int = (a) => a"#,
                "参数数量不匹配：签名 2 个，lambda 1 个",
            ),
            // 签名有 1 个参数，lambda 有 2 个
            (
                r#"double: (x: Int) -> Int = (x, y) => x + y"#,
                "参数数量不匹配：签名 1 个，lambda 2 个",
            ),
            // 签名无参数，lambda 有参数
            (
                r#"main: () -> Void = (x) => println(x)"#,
                "参数数量不匹配：签名 0 个，lambda 1 个",
            ),
        ];

        for (syntax, desc) in count_mismatch_cases {
            let tokens = tokenize(syntax).unwrap();
            let result = parse(&tokens);

            assert!(
                result.is_err(),
                "{} - 代码 '{}' 应该因参数数量不匹配被拒绝",
                desc,
                syntax
            );
        }
    }

    #[test]
    fn test_matching_parameter_names_accepted() {
        // 参数名匹配时应该被接受
        let matching_cases = vec![
            // 完全匹配
            r#"add: (a: Int, b: Int) -> Int = (a, b) => a + b"#,
            r#"square: (n: Int) -> Int = (n) => n * n"#,
            r#"test: (x: Int, y: Int, z: Int) -> Int = (x, y, z) => x + y + z"#,
            // 空参匹配
            r#"main: () -> Void = () => println("Hello")"#,
        ];

        for syntax in matching_cases {
            let tokens = tokenize(syntax).unwrap();
            let result = parse(&tokens);

            assert!(
                result.is_ok(),
                "参数名匹配的代码 '{}' 应该通过解析，错误: {:?}",
                syntax,
                result.err()
            );
        }
    }
}
