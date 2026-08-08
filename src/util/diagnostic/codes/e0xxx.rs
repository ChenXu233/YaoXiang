//! 错误码定义
//!
//! E0xxx: 词法和语法分析阶段的错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E0xxx 错误码列表
pub static E0XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E0001",
        category: ErrorCategory::Lexer,
    },
    ErrorCodeDefinition {
        code: "E0002",
        category: ErrorCategory::Lexer,
    },
    ErrorCodeDefinition {
        code: "E0003",
        category: ErrorCategory::Lexer,
    },
    ErrorCodeDefinition {
        code: "E0004",
        category: ErrorCategory::Lexer,
    },
    ErrorCodeDefinition {
        code: "E0010",
        category: ErrorCategory::Parser,
    },
    ErrorCodeDefinition {
        code: "E0011",
        category: ErrorCategory::Parser,
    },
    ErrorCodeDefinition {
        code: "E0012",
        category: ErrorCategory::Parser,
    },
    ErrorCodeDefinition {
        code: "E0013",
        category: ErrorCategory::Parser,
    },
    ErrorCodeDefinition {
        code: "E0014",
        category: ErrorCategory::Parser,
    },
    ErrorCodeDefinition {
        code: "E0016",
        category: ErrorCategory::Parser,
    },
    ErrorCodeDefinition {
        code: "E0018",
        category: ErrorCategory::Parser,
    },
];

// 快捷方法（code_helpers! 生成）
impl ErrorCodeDefinition {
    code_helpers! {
    /// E0001 无效字符
    ("E0001", invalid_character(char: &str) => .param("char", char)),
    /// E0002 无效数字字面量
    ("E0002", invalid_number_literal(literal: &str) => .param("literal", literal)),
    /// E0003 未终止的字符串
    ("E0003", unterminated_string(line: usize) => .param("line", line.to_string())),
    /// E0004 无效字符字面量
    ("E0004", invalid_char_literal(literal: &str) => .param("literal", literal)),
    /// E0010 期望的令牌
    ("E0010", expected_token(expected: &str, found: &str) => .param("expected", expected) .param("found", found)),
    /// E0011 意外的令牌
    ("E0011", unexpected_token(token: &str) => .param("token", token)),
    /// E0012 无效语法
    ("E0012", invalid_syntax(reason: &str) => .param("reason", reason)),
    /// E0013 不匹配的括号
    ("E0013", mismatched_brackets(bracket_type: &str, open_line: usize, open_col: usize) => .param("bracket_type", bracket_type) .param("open_line", open_line.to_string()) .param("open_col", open_col.to_string())),
    /// E0014 缺少分号
    ("E0014", missing_semicolon(statement: &str) => .param("statement", statement)),
    /// E0016 期望表达式
    ("E0016", expected_expression(context: &str) => .param("context", context)),
    /// E0018 关键字作变量名
    ("E0018", keyword_as_name(keyword: &str) => .param("keyword", keyword)),
    }
}
