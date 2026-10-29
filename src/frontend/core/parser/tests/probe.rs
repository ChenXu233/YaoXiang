//! 探针路径回归：默认探针下的 const 泛型参数判定（RFC-039 P6 6.4 修复轮）
//!
//! 回归事实：`extract_generic_param_names` 的 const 参数 guard 曾要求
//! `probe.kind_of(name) == NameKind::Builtin`，而 L2 的默认探针不持有内置类型名表
//! （`MonoType::from_builtin_name` 在 L3），永不返回 `Builtin` ⇒ 默认入口把
//! `(N: Int)` 这类 const 参数静默漏判，与 6.4 之前不一致。
//!
//! 修法：trait 增 `knows_builtin_types()`——不持有内置名表的探针返回 false，
//! const 参数 guard 回退到 `CONST_PARAM_TYPES` 单表（= 6.4 前行为）；L3 的语义
//! 探针返回 true，双知识（内置名定种 ∩ const 可接受集合）保持接线。

use crate::frontend::core::parser::ast::{extract_generic_param_names, Param, Type};
use crate::frontend::core::parser::{BuiltinProbe, NullProbe, TypeEnvProbe};
use crate::util::span::Span;

/// 构造一个带类型标注的形参（探针判定的输入只依赖 name/ty）。
fn param(
    name: &str,
    ty: &str,
) -> Param {
    Param {
        name: name.to_string(),
        ty: Some(Type::Name {
            name: ty.to_string(),
            span: Span::dummy(),
        }),
        is_mut: false,
        span: Span::dummy(),
    }
}

fn names_of(
    param_list: &[Param],
    probe: &dyn crate::frontend::core::parser::TypeEnvProbe,
) -> Vec<String> {
    extract_generic_param_names(param_list, probe)
        .into_iter()
        .map(|p| p.name)
        .collect()
}

#[test]
fn test_default_probe_registers_const_param_for_builtin_annotation() {
    // Arrange：`(N: Int) -> (n: N) -> Int` 拍平后的签名形参（ir_gen 的输入形态）
    let params = vec![param("N", "Int"), param("n", "N")];

    // Act
    let names = names_of(&params, &BuiltinProbe);

    // Assert：N 必须被登记为泛型参数（const 参数）；默认探针不持有内置名表，
    // 故 guard 走 CONST_PARAM_TYPES 单表回退（与 6.4 前一致）
    assert!(
        !BuiltinProbe.knows_builtin_types(),
        "L2 默认探针不持有内置类型名表"
    );
    assert_eq!(
        names,
        vec!["N".to_string()],
        "默认探针路径必须登记 (N: Int) 这个 const 泛型参数，实际 {names:?}"
    );
}

#[test]
fn test_default_probe_does_not_register_plain_value_param() {
    // Arrange：`plain: (a: Int) -> Int` —— a 未被当作类型引用，是普通值参数
    let params = vec![param("a", "Int")];

    // Act
    let names = names_of(&params, &BuiltinProbe);

    // Assert：不得把普通值参数登记成泛型参数（#351 判据）
    assert!(
        names.is_empty(),
        "普通值参数 a 不得登记为泛型参数，实际 {names:?}"
    );
}

#[test]
fn test_null_probe_and_default_probe_agree_on_const_param() {
    // Arrange
    let params = vec![param("N", "Int"), param("n", "N")];

    // Act
    let with_default = names_of(&params, &BuiltinProbe);
    let with_null = names_of(&params, &NullProbe);

    // Assert：两个不持有内置名表的探针判定一致（都回退到 CONST_PARAM_TYPES 单表）
    assert_eq!(
        with_default, with_null,
        "NullProbe 与 BuiltinProbe 对 const 参数必须判定一致"
    );
}
