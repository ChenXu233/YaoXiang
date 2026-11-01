//! 探针路径回归：const 泛型参数判定只听从注入探针（RFC-039 P6 6.4 / WBS 6.6.9）
//!
//! 判据：parser **不持有任何名字表**——const 参数类型、约束名、谓词名的判定一律经
//! `TypeEnvProbe`。本文件的用例把「L2 表压过探针」的旧形态钉成历史：6.4 曾用
//! `BuiltinProbe`（内置封闭集镜像）+ `ast::CONST_PARAM_TYPES`（不持表探针的单表兜底）
//! 让默认入口也能判 const 参数，代价是 L2 存了 L3 知识的三份副本（`check-boundary.py`
//! B6 的两个命中点）。WBS 6.6.9 删除这些副本后，**探针的答案就是唯一判据**。
//!
//! 本文件**不得**引用 L3（`check-boundary.py` B2 不豁免 tests）：需要「持有内置名
//! 知识」的探针一律用本文件的本地 stub 表达。

use crate::ast::{extract_generic_param_names, Param, Type};
use crate::frontend::core::parser::{NameKind, NullProbe, TypeEnvProbe};
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
    probe: &dyn TypeEnvProbe,
) -> Vec<String> {
    extract_generic_param_names(param_list, probe)
        .into_iter()
        .map(|p| p.name)
        .collect()
}

/// 「持有内置名知识」的探针 stub：只认 `Int` 可作 const 参数类型。
///
/// 这个集合的取值是**任意的**：用例断言的是「parser 原样采纳探针的答案」，不是
/// 「Int 一定可作 const 参数」（后者是 L3 的知识，由 L3 的对账用例守）。
struct IntConstParamProbe;

impl TypeEnvProbe for IntConstParamProbe {
    fn is_const_param_type(
        &self,
        name: &str,
    ) -> bool {
        name == "Int"
    }

    fn kind_of(
        &self,
        name: &str,
    ) -> NameKind {
        if name == "Int" {
            NameKind::Builtin
        } else {
            NameKind::UserType
        }
    }
}

#[test]
fn test_probe_accepted_const_type_is_registered() {
    // Arrange：`(N: Int) -> (n: N) -> Int` 拍平后的签名形参（ir_gen 的输入形态）
    let params = vec![param("N", "Int"), param("n", "N")];

    // Act
    let names = names_of(&params, &IntConstParamProbe);

    // Assert：探针认 `Int` ⇒ N 必须被登记为泛型参数（const 参数）。
    // 6.4 的判据是「L2 单表 ∩ 内置名定种」；6.6.9 起只剩探针这一问。
    assert_eq!(
        names,
        vec!["N".to_string()],
        "探针认 Int 可作 const 参数类型时必须登记 (N: Int)，实际 {names:?}"
    );
}

#[test]
fn test_probe_accepted_const_type_without_type_use_is_not_registered() {
    // Arrange：`plain: (a: Int) -> Int` —— a 未被当作类型引用，是普通值参数
    let params = vec![param("a", "Int")];

    // Act
    let names = names_of(&params, &IntConstParamProbe);

    // Assert：不得把普通值参数登记成泛型参数（#351 判据：登记了会被单态化删掉）
    assert!(
        names.is_empty(),
        "普通值参数 a 不得登记为泛型参数，实际 {names:?}"
    );
}

#[test]
fn test_null_probe_registers_no_const_param() {
    // Arrange：默认入口的探针（`parse()` 只带 NullProbe）
    let params = vec![param("N", "Int"), param("n", "N")];

    // Act
    let names = names_of(&params, &NullProbe);

    // Assert：NullProbe 不提供任何类型知识 ⇒ 不登记 const 参数。
    // 这正是 6.6.9「默认入口不得静默降级」的反面证据：生产入口必须显式注入
    // 语义探针，否则 `(N: Int)` 会被当成普通值参数静默解析。
    assert!(
        names.is_empty(),
        "NullProbe 不携带类型知识，不得登记 const 参数，实际 {names:?}"
    );
}

#[test]
fn test_injected_probe_replaces_default_no_knowledge() {
    // Arrange：同一份形参，两个探针的 const 参数判定
    let params = vec![param("N", "Int"), param("n", "N")];

    // Act
    let with_stub = names_of(&params, &IntConstParamProbe);
    let with_null = names_of(&params, &NullProbe);

    // Assert：判定完全由注入探针决定（6.4 的「两个不持表探针判定一致」已随
    // 单表兜底删除——一致性不再是判据，**探针主权**才是）
    assert_eq!(
        with_stub,
        vec!["N".to_string()],
        "注入持表探针时必须登记 const 参数，实际 {with_stub:?}"
    );
    assert!(
        with_null.is_empty(),
        "NullProbe 下不得登记 const 参数，实际 {with_null:?}"
    );
}

// ==================== 内置封闭集清零（WBS 6.6.9）====================
//
// 下面两条用例是**红判据**：它们断言「探针说不，parser 就闭嘴」与「探针说是，
// parser 不得复核」。在 6.4 的形态下（L2 `CONST_PARAM_TYPES` 单表兜底）两条都红：
// 前者被 L2 表压过、后者被内置名表复核掉。删表后转绿。

/// 明确否认一切 const 参数类型的探针（其余判定走默认）。
struct DenyConstParamProbe;

impl TypeEnvProbe for DenyConstParamProbe {
    fn is_const_param_type(
        &self,
        _name: &str,
    ) -> bool {
        false
    }
}

/// 只认一个**非内置名** `Nominal` 的探针：证明 parser 不拿内置名表复核探针的答案。
struct NominalConstParamProbe;

impl TypeEnvProbe for NominalConstParamProbe {
    fn is_const_param_type(
        &self,
        name: &str,
    ) -> bool {
        name == "Nominal"
    }
}

#[test]
fn test_extract_generic_param_names_follows_injected_probe_only() {
    // Arrange：`(N: Int) -> (n: N)` 的拍平形参 + 一个**否认** Int 可作 const 参数的探针
    let params = vec![param("N", "Int"), param("n", "N")];

    // Act
    let names = names_of(&params, &DenyConstParamProbe);

    // Assert：探针说不 ⇒ 不得登记 N。若 parser 还留着自己的封闭集
    // （`CONST_PARAM_TYPES` 单表兜底），那张表会压过探针把 N 登记出来。
    assert!(
        names.is_empty(),
        "探针否认 Int 可作 const 参数类型时 parser 不得登记 N（内置封闭集必须清零），实际 {names:?}"
    );
}

#[test]
fn test_extract_generic_param_names_accepts_probe_declared_const_type() {
    // Arrange：标注是**非内置名** `Nominal`，但探针判它为可作 const 参数的类型
    let params = vec![param("N", "Nominal"), param("n", "N")];

    // Act
    let names = names_of(&params, &NominalConstParamProbe);

    // Assert：parser 原样采纳探针的答案，不得拿内置名表复核（`Nominal` 不在任何内置名表里）
    assert_eq!(
        names,
        vec!["N".to_string()],
        "探针声明 Nominal 可作 const 参数类型时 N 必须登记（parser 不得复核内置名表），实际 {names:?}"
    );
}
