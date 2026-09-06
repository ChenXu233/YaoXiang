//! 错误码定义（define_codes! 单源条目，#326）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

define_codes!(E0XXX, {
    // E0001 无效字符
    ("E0001", Lexer, false, invalid_character(char: &str) => .param("char", char)),
    // E0002 无效数字字面量
    ("E0002", Lexer, false, invalid_number_literal(literal: &str) => .param("literal", literal)),
    // E0003 未终止的字符串
    ("E0003", Lexer, false, unterminated_string(line: usize) => .param("line", line.to_string())),
    // E0004 无效字符字面量
    ("E0004", Lexer, false, invalid_char_literal(literal: &str) => .param("literal", literal)),
    // E0010 期望的令牌
    ("E0010", Parser, false, expected_token(expected: &str, found: &str) => .param("expected", expected) .param("found", found)),
    // E0011 意外的令牌
    ("E0011", Parser, false, unexpected_token(token: &str) => .param("token", token)),
    // E0012 无效语法
    ("E0012", Parser, false, invalid_syntax(reason: &str) => .param("reason", reason)),
    // E0013 不匹配的括号
    ("E0013", Parser, false, mismatched_brackets(bracket_type: &str, open_line: usize, open_col: usize) => .param("bracket_type", bracket_type) .param("open_line", open_line.to_string()) .param("open_col", open_col.to_string())),
    // E0014 缺少分号
    ("E0014", Parser, false, missing_semicolon(statement: &str) => .param("statement", statement)),
    // E0016 期望表达式
    ("E0016", Parser, false, expected_expression(context: &str) => .param("context", context)),
    // E0018 关键字作变量名
    ("E0018", Parser, false, keyword_as_name(keyword: &str) => .param("keyword", keyword)),
});
