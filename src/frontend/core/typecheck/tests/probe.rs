//! 探针表对账（RFC-039 P6 6.4 / 03 §5.4）
//!
//! parser 的默认探针 `BuiltinProbe` 是编译器**内置封闭集**在 L2 的镜像；本模块把
//! 它与 L3 的权威表逐名对账，任一侧漂移即红：
//!
//! - 约束名 ↔ `operator_interfaces::SPECS`（运算符接口表）
//! - 内置谓词名 ↔ `is_builtin_predicate_name`（内置谓词单一判定口）
//! - 定种（`NameKind`）↔ `MonoType::from_builtin_name` 的内置名像

use crate::frontend::core::parser::probe::{
    BuiltinProbe, NameKind, NullProbe, TypeEnvProbe, CONSTRAINT_NAMES, PREDICATE_NAMES,
};
use crate::frontend::core::typecheck::operator_interfaces::SPECS;
use crate::frontend::core::typecheck::probe::BUILTIN_PREDICATES;
use crate::frontend::core::typecheck::{is_builtin_predicate_name, SemanticProbe};

#[test]
fn test_builtin_probe_constraint_names_match_operator_interface_table() {
    // Arrange：L3 权威表（运算符接口）与 L2 默认探针的封闭集
    let authority: Vec<&str> = SPECS.iter().map(|s| s.name).collect();
    let builtin = BuiltinProbe;
    let semantic = SemanticProbe;

    // Act
    let accepted_by_default: Vec<&str> = CONSTRAINT_NAMES
        .iter()
        .copied()
        .filter(|n| builtin.is_constraint(n))
        .collect();
    let accepted_by_l3: Vec<&str> = authority
        .iter()
        .copied()
        .filter(|n| semantic.is_constraint(n))
        .collect();

    // Assert：双向逐名一致（任何一侧加名/删名都会在这里红）
    assert_eq!(
        accepted_by_default.len(),
        CONSTRAINT_NAMES.len(),
        "默认探针必须接受自己封闭集里的每个约束名"
    );
    assert_eq!(
        accepted_by_l3, authority,
        "L3 探针必须接受运算符接口表的每个名字"
    );
    let mut a = accepted_by_default;
    let mut b = authority.clone();
    a.sort_unstable();
    b.sort_unstable();
    assert_eq!(
        a, b,
        "parser 的 CONSTRAINT_NAMES 与 operator_interfaces::SPECS 必须逐名一致（WBS 6.4 的单一源对账）"
    );
}

#[test]
fn test_builtin_probe_predicate_names_match_typecheck_judgement() {
    // Arrange：内置谓词的两个来源
    let builtin = BuiltinProbe;
    let semantic = SemanticProbe;

    // Act & Assert：双向逐名一致
    for name in PREDICATE_NAMES {
        assert!(
            builtin.is_predicate(name),
            "默认探针必须把封闭集里的 {name} 判为谓词"
        );
        assert!(semantic.is_predicate(name), "L3 探针必须把 {name} 判为谓词");
        assert!(
            is_builtin_predicate_name(name),
            "{name} 必须在内置谓词单一判定口里"
        );
    }
    for name in BUILTIN_PREDICATES {
        assert!(
            PREDICATE_NAMES.contains(name),
            "内置谓词 {name} 必须同步进 parser 的默认封闭集"
        );
    }
    assert_eq!(
        PREDICATE_NAMES.len(),
        BUILTIN_PREDICATES.len(),
        "两侧内置谓词数必须一致"
    );
    // 负向：普通标识符不是谓词（防「全 true」把判据做废）
    assert!(
        !builtin.is_predicate("Positive"),
        "Positive 是用户声明名，不是内置谓词"
    );
}

#[test]
fn test_semantic_probe_kind_of_matches_builtin_name_table() {
    // Arrange
    let semantic = SemanticProbe;
    let null = NullProbe;

    // Act & Assert：内置名 → Builtin；约束名 / 谓词 / 用户类型各自定种
    assert!(
        matches!(semantic.kind_of("Int"), NameKind::Builtin),
        "Int 是内置名，必须定种为 Builtin"
    );
    assert!(
        matches!(semantic.kind_of("Int64"), NameKind::Builtin),
        "Int64 是内置名，必须定种为 Builtin"
    );
    assert!(
        matches!(semantic.kind_of("Add"), NameKind::Constraint),
        "Add 是约束名，必须定种为 Constraint"
    );
    assert!(
        matches!(semantic.kind_of("Terminates"), NameKind::Predicate),
        "Terminates 是内置谓词，必须定种为 Predicate"
    );
    assert!(
        matches!(semantic.kind_of("MyType"), NameKind::UserType),
        "未知名只能是 UserType（不得瞎猜 Builtin）"
    );
    // 默认探针按自己的封闭集定种：谓词 / 约束名可判，内置类型名不可判（表在 L3）
    assert!(
        matches!(BuiltinProbe.kind_of("Terminates"), NameKind::Predicate),
        "默认探针必须把封闭集里的谓词定种为 Predicate"
    );
    assert!(
        matches!(BuiltinProbe.kind_of("Add"), NameKind::Constraint),
        "默认探针必须把封闭集里的约束名定种为 Constraint"
    );
    assert!(
        matches!(BuiltinProbe.kind_of("Int"), NameKind::UserType),
        "默认探针不持有内置名表，内置名只能定种为 UserType"
    );
    // NullProbe 不携带任何类型知识
    assert!(
        matches!(null.kind_of("Int"), NameKind::UserType),
        "NullProbe 不携带任何类型知识，定种恒为 UserType"
    );
}

#[test]
fn test_semantic_probe_and_default_probe_agree_on_same_source_const_param() {
    // Arrange：同一份源码的拍平签名形参（`(N: Int)` 是 const 泛型参数，
    // `n: N` 把 N 当类型引用——正是 const 参数判定的触发形态）
    let tokens = crate::frontend::core::lexer::tokenize("f: (N: Int, n: N) -> Int = 1")
        .expect("源码应可词法分析");
    let module = crate::frontend::core::parser::parse(&tokens).module;
    let params = module
        .items
        .iter()
        .find_map(|s| match &s.kind {
            crate::frontend::core::parser::ast::StmtKind::Assign {
                signature_params, ..
            } if !signature_params.is_empty() => Some(signature_params.clone()),
            _ => None,
        })
        .expect("应有一条带签名形参的赋值语句");

    // Act
    let with_semantic: Vec<String> =
        crate::frontend::core::parser::ast::extract_generic_param_names(&params, &SemanticProbe)
            .into_iter()
            .map(|p| p.name)
            .collect();
    let with_default: Vec<String> =
        crate::frontend::core::parser::ast::extract_generic_param_names(
            &params,
            &crate::frontend::core::parser::BuiltinProbe,
        )
        .into_iter()
        .map(|p| p.name)
        .collect();

    // Assert：持表探针（双知识全强度）与不持表探针（单表回退）对同一源码判定一致
    assert_eq!(
        with_semantic, with_default,
        "同一源码下 SemanticProbe 与默认探针的 const 参数判定必须一致"
    );
    assert_eq!(
        with_default,
        vec!["N".to_string()],
        "const 参数 N 必须被登记（6.4 前行为），实际 {with_default:?}"
    );
    assert!(
        SemanticProbe.knows_builtin_types(),
        "L3 语义探针持有内置类型名表"
    );
    assert!(
        !crate::frontend::core::parser::BuiltinProbe.knows_builtin_types(),
        "L2 默认探针不持有内置类型名表"
    );
}
