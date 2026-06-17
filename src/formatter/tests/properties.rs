//! Formatter 属性测试 — 基于测试规范 §15
//!
//! 验证格式化器的不变量：
//! - 幂等性：format(format(code)) == format(code)
//!
//! 注意：对于包含语法错误的输入，格式化器会返回 Err。
//! 因此我们只验证能成功解析的输入。

use proptest::prelude::*;
use crate::formatter::{format_source, FormatOptions};

fn default_options() -> FormatOptions {
    FormatOptions::default()
}

proptest! {
    // 属性：对于任意能成功解析的源代码，format(format(code)) == format(code)
    // 即格式化器是幂等的
    #[test]
    fn test_format_idempotent(source in ".*") {
        let opts = default_options();
        // Arrange: 如果解析有错误，format_source 会返回 Err，直接跳过
        if let Ok(formatted1) = format_source(&source, &opts) {
            // Act: 对格式化后的代码再次格式化
            let formatted2 = format_source(&formatted1, &opts)
                .expect("formatted1 should be valid syntax");

            // Assert: 两次格式化结果应该相同
            prop_assert_eq!(
                formatted1, formatted2,
                "Format should be idempotent for source: {:?}", source
            );
        }
    }
}
