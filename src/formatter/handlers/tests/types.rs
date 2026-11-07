//! 类型格式化处理器测试
//!
//! 对应 formatter 规范 §9 (type annotations)

use crate::formatter::context::FormatContext;
use crate::formatter::handlers::types::format_type;
use crate::formatter::options::FormatOptions;
use crate::formatter::source_map::SourceMap;
use crate::ast::*;
use crate::util::span::Span;

fn default_ctx() -> FormatContext {
    FormatContext::new(FormatOptions::default())
}

fn default_source_map() -> SourceMap {
    SourceMap::build("")
}

/// 内建标量在 AST 里只有 Name 一种写法（6.3 删除了 Int/Float/… 专用变体），
/// 格式化必须原样输出名字，不得再造 i32 / i64 这类实现内拼写。
fn name_type(name: &str) -> Type {
    Type::Name {
        name: name.to_string(),
        span: Span::dummy(),
    }
}

#[test]
fn test_format_type_int() {
    assert_eq!(
        format_type(&name_type("Int32"), &default_ctx(), &default_source_map()),
        "Int32",
        "宽度显式内建名应原样输出"
    );
    assert_eq!(
        format_type(&name_type("Int64"), &default_ctx(), &default_source_map()),
        "Int64",
        "Int64 应原样输出（实现内拼写 i64 已删除）"
    );
}
