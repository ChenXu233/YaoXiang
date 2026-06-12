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
        code: "E0015",
        category: ErrorCategory::Parser,
    },
    ErrorCodeDefinition {
        code: "E0016",
        category: ErrorCategory::Parser,
    },
    ErrorCodeDefinition {
        code: "E0017",
        category: ErrorCategory::Parser,
    },
    ErrorCodeDefinition {
        code: "E0018",
        category: ErrorCategory::Parser,
    },
    ErrorCodeDefinition {
        code: "E0019",
        category: ErrorCategory::Parser,
    },
    ErrorCodeDefinition {
        code: "E0020",
        category: ErrorCategory::Parser,
    },
    ErrorCodeDefinition {
        code: "E0021",
        category: ErrorCategory::Parser,
    },
    ErrorCodeDefinition {
        code: "E0022",
        category: ErrorCategory::Parser,
    },
];

// E0xxx 快捷方法
impl ErrorCodeDefinition {
    /// E0001 无效字符
    pub fn invalid_character(char: &str) -> DiagnosticBuilder {
        let def = Self::find("E0001").unwrap();
        def.builder().param("char", char)
    }

    /// E0002 无效数字字面量
    pub fn invalid_number_literal(literal: &str) -> DiagnosticBuilder {
        let def = Self::find("E0002").unwrap();
        def.builder().param("literal", literal)
    }

    /// E0003 未终止的字符串
    pub fn unterminated_string(line: usize) -> DiagnosticBuilder {
        let def = Self::find("E0003").unwrap();
        def.builder().param("line", line.to_string())
    }

    /// E0004 无效字符字面量
    pub fn invalid_char_literal(literal: &str) -> DiagnosticBuilder {
        let def = Self::find("E0004").unwrap();
        def.builder().param("literal", literal)
    }

    /// E0010 期望的令牌
    pub fn expected_token(
        expected: &str,
        found: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E0010").unwrap();
        def.builder()
            .param("expected", expected)
            .param("found", found)
    }

    /// E0011 意外的令牌
    pub fn unexpected_token(token: &str) -> DiagnosticBuilder {
        let def = Self::find("E0011").unwrap();
        def.builder().param("token", token)
    }

    /// E0012 无效语法
    pub fn invalid_syntax(reason: &str) -> DiagnosticBuilder {
        let def = Self::find("E0012").unwrap();
        def.builder().param("reason", reason)
    }

    /// E0013 不匹配的括号
    pub fn mismatched_brackets(
        bracket_type: &str,
        open_line: usize,
        open_col: usize,
    ) -> DiagnosticBuilder {
        let def = Self::find("E0013").unwrap();
        def.builder()
            .param("bracket_type", bracket_type)
            .param("open_line", open_line.to_string())
            .param("open_col", open_col.to_string())
    }

    /// E0014 缺少分号
    pub fn missing_semicolon(statement: &str) -> DiagnosticBuilder {
        let def = Self::find("E0014").unwrap();
        def.builder().param("statement", statement)
    }

    /// E0015 期望标识符
    pub fn expected_identifier(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E0015").unwrap();
        def.builder().param("name", name)
    }

    /// E0016 期望表达式
    pub fn expected_expression(context: &str) -> DiagnosticBuilder {
        let def = Self::find("E0016").unwrap();
        def.builder().param("context", context)
    }

    /// E0017 旧语法不兼容
    pub fn old_syntax_rejected(
        old: &str,
        new: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E0017").unwrap();
        def.builder().param("old", old).param("new", new)
    }

    /// E0018 关键字作变量名
    pub fn keyword_as_name(keyword: &str) -> DiagnosticBuilder {
        let def = Self::find("E0018").unwrap();
        def.builder().param("keyword", keyword)
    }

    /// E0019 参数数量不匹配
    pub fn param_count_mismatch(
        expected: usize,
        found: usize,
    ) -> DiagnosticBuilder {
        let def = Self::find("E0019").unwrap();
        def.builder()
            .param("expected", expected.to_string())
            .param("found", found.to_string())
    }

    /// E0020 参数名称不匹配
    pub fn param_name_mismatch(
        position: usize,
        expected: &str,
        found: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E0020").unwrap();
        def.builder()
            .param("position", position.to_string())
            .param("expected", expected)
            .param("found", found)
    }

    /// E0021 期望 token 在特定上下文后
    pub fn expected_token_after(
        token: &str,
        context: &str,
    ) -> DiagnosticBuilder {
        let def = Self::find("E0021").unwrap();
        def.builder()
            .param("token", token)
            .param("context", context)
    }

    /// E0022 枚举必须使用花括号语法
    pub fn enum_brace_syntax() -> DiagnosticBuilder {
        let def = Self::find("E0022").unwrap();
        def.builder()
    }
}
