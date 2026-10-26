//! 命名实参槽位绑定诊断测试 — 基于 #452/#454 槽位绑定判据与 issue #469 处 1
//!
//! 判据（issue #469 处 1）：诊断 help 与 locales 同源——同一次编译的 help
//! 语言必须一致。E1010「命名实参缺必需形参」形态的 help 不得在发射点
//! 硬编某语言成品串；缺失形参名的信息不得丢失（不得为换语言丢内容）。

use crate::frontend::core::parser::ast::Expr;
use crate::frontend::core::typecheck::inference::arg_binding::bind_args_to_params;
use crate::util::span::{Position, Span};

/// 构造调用点 span（判据渲染位置无关，只需非 dummy 以通过 span 强制）
fn call_span() -> Span {
    Span::new(Position::new(3, 5), Position::new(3, 25))
}

/// 构造一个位置无关的变量表达式（bind_args_to_params 只读其 span，形态不限）
fn placeholder_expr() -> Expr {
    Expr::Var("1".to_string(), call_span())
}

#[test]
fn test_missing_required_param_help_has_no_hardcoded_language() {
    // Arrange：钉住 en 本地化（默认即 en；显式设定使断言与环境配置无关，
    // 先例见 rfc027_return_refinement.rs 同款写法）。zh 环境下 locales 的
    // zh 模板本就是中文——无中文判据只对 en 通道有意义。
    // 形参表 [x, y, z]（全必需），命名实参只给 x、z——缺 y（issue #469 样例形态）
    crate::util::i18n::set_lang_from_string("en".to_string());
    let params = vec!["x".to_string(), "y".to_string(), "z".to_string()];
    let optional = vec![false, false, false];
    let named_args = vec![
        ("x".to_string(), placeholder_expr()),
        ("z".to_string(), placeholder_expr()),
    ];

    // Act
    let err = bind_args_to_params(
        "Temp.tri",
        &params,
        &[],
        &named_args,
        0,
        &optional,
        call_span(),
    )
    .expect_err("缺必需形参必须返回 E1010 诊断");

    // Assert：help 不得含 CJK 统一表意文字（发射点硬编中文即此判据的失败态）
    assert!(
        !err.help
            .chars()
            .any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c)),
        "help 必须与 locales 同源（同一次编译语言一致），不得硬编中文成品串。Got: '{}'",
        err.help
    );
    // Assert：缺失形参名是本形态特有的信息，迁移时不得丢失
    assert!(
        err.help.contains('y'),
        "help 必须列出缺失的形参名。Got: '{}'",
        err.help
    );
    assert_eq!(
        err.code, "E1010",
        "缺必需形参（命名实参形态）应保持 E1010 码"
    );
}
