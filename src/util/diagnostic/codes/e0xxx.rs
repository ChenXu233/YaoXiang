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
    // E0019 解析递归深度超限（issue #540：深嵌套表达式此前让编译器栈溢出
    // 崩溃，0xC00000FD 零诊断）。闸值 1024 的登记依据见
    // parser_state.rs 的 MAX_PARSE_DEPTH 文档（实测括号嵌套 600 层通过、
    // 崩溃起点 ~650 ⇒ 下限 ~640；大栈线程 64MB 崩溃阈值 ~8500 ⇒ 上限
    // 8499）；语义是「解析递归（语句/表达式嵌套）超限」——覆盖 parser
    // 真递归形态（一元链/调用嵌套/括号/块/列表等）。扁平二元链在 parser
    // 是迭代、不经本闸，由 IR 层既有 E3014 寄存器上限拒绝。
    ("E0019", Parser, false, expression_too_deep(limit: usize) => .param("limit", limit.to_string())),
});
