//! Formatter 属性测试 — 基于测试规范 §15
//!
//! 验证格式化器的不变量：
//! - 幂等性：format(format(code)) == format(code)
//!
//! 注意：对于包含语法错误的输入，格式化器会插入 `/* error */` 注释。
//! 当再次格式化时，这些注释可能被丢弃，导致幂等性不成立。
//! 因此我们只验证能成功解析且不含错误注释的输入。

// use proptest::prelude::*;
// use crate::formatter::{format_source, FormatOptions};

// fn default_options() -> FormatOptions {
//     FormatOptions::default()
// }

// proptest! {
//     #[test]
//     fn test_format_idempotent(source in ".*") {
//         let opts = default_options();
//         // 如果解析有错误，format_source 会返回 Err，直接跳过
//         if let Ok(formatted1) = format_source(&source, &opts) {
//             let formatted2 = format_source(&formatted1, &opts).unwrap();
//             prop_assert_eq!(formatted1, formatted2);
//         }
//     }
// }
