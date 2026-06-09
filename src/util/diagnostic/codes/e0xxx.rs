//! 错误码定义
//! E0xxx: 词法和语法分析阶段的错误码

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

/// E0xxx 错误码列表
pub static E0XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition { code: "E0001", category: ErrorCategory::Lexer },
    ErrorCodeDefinition { code: "E0002", category: ErrorCategory::Lexer },
    ErrorCodeDefinition { code: "E0003", category: ErrorCategory::Lexer },
    ErrorCodeDefinition { code: "E0004", category: ErrorCategory::Lexer },
    ErrorCodeDefinition { code: "E0010", category: ErrorCategory::Parser },
    ErrorCodeDefinition { code: "E0011", category: ErrorCategory::Parser },
    ErrorCodeDefinition { code: "E0012", category: ErrorCategory::Parser },
    ErrorCodeDefinition { code: "E0013", category: ErrorCategory::Parser },
    ErrorCodeDefinition { code: "E0014", category: ErrorCategory::Parser },
    ErrorCodeDefinition { code: "E0015", category: ErrorCategory::Parser },
    ErrorCodeDefinition { code: "E0016", category: ErrorCategory::Parser },
    ErrorCodeDefinition { code: "E0017", category: ErrorCategory::Parser },
    ErrorCodeDefinition { code: "E0018", category: ErrorCategory::Parser },
    ErrorCodeDefinition { code: "E0019", category: ErrorCategory::Parser },
    ErrorCodeDefinition { code: "E0020", category: ErrorCategory::Parser },
    ErrorCodeDefinition { code: "E0021", category: ErrorCategory::Parser },
    ErrorCodeDefinition { code: "E0022", category: ErrorCategory::Parser },
];

impl ErrorCodeDefinition {
    pub fn invalid_character(char: &str) -> DiagnosticBuilder { Self::find("E0001").unwrap().builder().param("char", char) }
    pub fn invalid_number_literal(literal: &str) -> DiagnosticBuilder { Self::find("E0002").unwrap().builder().param("literal", literal) }
    pub fn unterminated_string(line: usize) -> DiagnosticBuilder { Self::find("E0003").unwrap().builder().param("line", line.to_string()) }
    pub fn invalid_char_literal(literal: &str) -> DiagnosticBuilder { Self::find("E0004").unwrap().builder().param("literal", literal) }
    pub fn expected_token(expected: &str, found: &str) -> DiagnosticBuilder { Self::find("E0010").unwrap().builder().param("expected", expected).param("found", found) }
    pub fn unexpected_token(token: &str) -> DiagnosticBuilder { Self::find("E0011").unwrap().builder().param("token", token) }
    pub fn invalid_syntax(reason: &str) -> DiagnosticBuilder { Self::find("E0012").unwrap().builder().param("reason", reason) }
    pub fn mismatched_brackets(bracket_type: &str, open_line: usize, open_col: usize) -> DiagnosticBuilder { Self::find("E0013").unwrap().builder().param("bracket_type", bracket_type).param("open_line", open_line.to_string()).param("open_col", open_col.to_string()) }
    pub fn missing_semicolon(statement: &str) -> DiagnosticBuilder { Self::find("E0014").unwrap().builder().param("statement", statement) }
    pub fn expected_identifier(name: &str) -> DiagnosticBuilder { Self::find("E0015").unwrap().builder().param("name", name) }
    pub fn expected_expression(context: &str) -> DiagnosticBuilder { Self::find("E0016").unwrap().builder().param("context", context) }
    pub fn old_syntax_rejected(old: &str, new: &str) -> DiagnosticBuilder { Self::find("E0017").unwrap().builder().param("old", old).param("new", new) }
    pub fn keyword_as_name(keyword: &str) -> DiagnosticBuilder { Self::find("E0018").unwrap().builder().param("keyword", keyword) }
    pub fn param_count_mismatch(expected: usize, found: usize) -> DiagnosticBuilder { Self::find("E0019").unwrap().builder().param("expected", expected.to_string()).param("found", found.to_string()) }
    pub fn param_name_mismatch(position: usize, expected: &str, found: &str) -> DiagnosticBuilder { Self::find("E0020").unwrap().builder().param("position", position.to_string()).param("expected", expected).param("found", found) }
    pub fn expected_token_after(token: &str, context: &str) -> DiagnosticBuilder { Self::find("E0021").unwrap().builder().param("token", token).param("context", context) }
    pub fn enum_brace_syntax() -> DiagnosticBuilder { Self::find("E0022").unwrap().builder() }
}
