//! 解析器旧语法拒绝测试
//!
//! 验证解析器正确拒绝旧语法并接受新语法

use crate::frontend::lexer::tokenize;
use crate::frontend::parser::parse;

#[test]
fn test_old_syntax_rejection() {
    // 旧语法应该被拒绝
    let old_syntax = r#"
        add(Int, Int) -> Int = (a, b) => a + b
    "#;

    let tokens = tokenize(old_syntax).unwrap();
    let result = parse(&tokens);

    // 应该返回错误
    assert!(result.is_err(), "旧语法应该被拒绝");
}

#[test]
fn test_new_syntax_acceptance() {
    // 新语法应该被接受（通过解析）
    let new_syntax = r#"
        add:(Int, Int) -> Int = (a, b) => { a + b }
    "#;

    let tokens = tokenize(new_syntax).unwrap();
    let result = parse(&tokens);

    // 应该成功通过解析（即使后续类型检查可能失败）
    assert!(result.is_ok(), "新语法应该通过解析");
}

#[test]
fn test_old_syntax_examples() {
    // 测试各种旧语法模式
    let old_syntaxes = vec![
        r#"add(Int, Int) -> Int = (a, b) => a + b"#,
        r#"square(Int) -> Int = (x) => x * x"#,
        // 注意: main() -> Int = () => { 42 } 这种模式还需要进一步处理
    ];

    for syntax in old_syntaxes {
        let tokens = tokenize(syntax).unwrap();
        let result = parse(&tokens);

        assert!(result.is_err(), "旧语法 '{}' 应该被拒绝", syntax);
    }
}

#[test]
fn test_new_syntax_examples() {
    // 测试各种新语法模式
    let new_syntaxes = vec![
        r#"add:(Int, Int) -> Int = (a, b) => { a + b }"#,
        r#"main:() -> Int = () => { 42 }"#,
        r#"square:(Int) -> Int = (x) => { x * x }"#,
        r#"main = () => { 42 }"#, // 省略形式
    ];

    for syntax in new_syntaxes {
        let tokens = tokenize(syntax).unwrap();
        let result = parse(&tokens);

        assert!(result.is_ok(), "新语法 '{}' 应该通过解析", syntax);
    }
}
