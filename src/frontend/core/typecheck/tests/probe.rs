//! 探针权威对账（RFC-039 P6 6.4 / WBS 6.6.9）
//!
//! **口径变更（6.6.9）**：本模块原先对账「parser 的 L2 封闭集镜像（`BuiltinProbe` /
//! `CONSTRAINT_NAMES` / `PREDICATE_NAMES`）↔ L3 权威表」。6.6.9 删除了 L2 镜像
//! （知识只剩一份），故对账改为「**L3 权威表 ↔ 语言级规格清单 ↔ 探针实现**」：
//! 任一侧漂移（表里加名/删名、探针不再转发、探针把集合放宽）都在这里红。
//!
//! - 约束名：`operator_interfaces::SPECS` ↔ 规格清单（7 名）↔ `is_constraint_name` / 探针
//! - 内置谓词：`BUILTIN_PREDICATES` ↔ 规格清单（1 名）↔ `is_builtin_predicate_name` / 探针
//! - const 参数类型名：`CONST_PARAM_TYPES` ↔ 规格清单（5 名）↔ `SemanticProbe`，
//!   且**不得**宽化成 `kind_of == Builtin`（那是 12 个规范名 + 别名）
//! - 定种：`SemanticProbe::kind_of` ↔ 上述三张表 + `from_builtin_name`

use crate::frontend::core::parser::ast::extract_generic_param_names;
use crate::frontend::core::parser::{NameKind, NullProbe, TypeEnvProbe};
use crate::frontend::core::typecheck::operator_interfaces::SPECS;
use crate::frontend::core::typecheck::probe::{
    is_const_param_type_name, BUILTIN_PREDICATES, CONST_PARAM_TYPES,
};
use crate::frontend::core::typecheck::{is_builtin_predicate_name, is_constraint_name, SemanticProbe};

/// 语言级规格清单：运算符接口（约束名）的 7 个名字（RFC-011b）。
///
/// 这是**期望值**（规格），不是实现镜像：表里多一个/少一个名字都必须红。
const SPEC_CONSTRAINT_NAMES: &[&str] = &[
    "Add", "Subtract", "Multiply", "Divide", "Modulo", "Equal", "Index",
];

/// 语言级规格清单：内置谓词（RFC-027 §6.9）。
const SPEC_PREDICATE_NAMES: &[&str] = &["Terminates"];

/// 语言级规格清单：可作 const 泛型参数的类型名（RFC-011 §4.1 的粗筛集合）。
const SPEC_CONST_PARAM_TYPES: &[&str] = &["Int", "Bool", "Float", "Char", "String"];

#[test]
fn test_constraint_names_match_operator_interface_table() {
    // Arrange：运算符接口表的全部名字 + 探针
    let authority: Vec<&str> = SPECS.iter().map(|s| s.name).collect();
    let semantic = SemanticProbe;

    // Act
    let mut accepted_by_probe: Vec<&str> = authority
        .iter()
        .copied()
        .filter(|n| semantic.is_constraint(n))
        .collect();
    let mut spec: Vec<&str> = SPEC_CONSTRAINT_NAMES.to_vec();

    // Assert：表 = 规格 = 探针接受面（三向，任一漂移即红）
    assert_eq!(
        accepted_by_probe.len(),
        authority.len(),
        "探针必须接受运算符接口表的每个名字，实际 {accepted_by_probe:?}"
    );
    accepted_by_probe.sort_unstable();
    let mut authority_sorted = authority.clone();
    authority_sorted.sort_unstable();
    spec.sort_unstable();
    assert_eq!(
        accepted_by_probe, authority_sorted,
        "探针接受面必须与 operator_interfaces::SPECS 逐名一致"
    );
    assert_eq!(
        authority_sorted, spec,
        "SPECS 必须与语言级规格清单（7 个运算符接口）逐名一致"
    );
    // 负向：普通标识符不是约束名（防「全 true」把判据做废）
    assert!(
        !is_constraint_name("Positive") && !semantic.is_constraint("Positive"),
        "Positive 是用户声明名，不是约束名"
    );
}

#[test]
fn test_predicate_names_match_typecheck_judgement() {
    // Arrange：内置谓词的权威表 + 探针
    let semantic = SemanticProbe;

    // Act & Assert：表 = 规格 = 窄入口 = 探针，四向逐名一致
    assert_eq!(
        BUILTIN_PREDICATES, SPEC_PREDICATE_NAMES,
        "内置谓词表必须与语言级规格清单逐名一致"
    );
    for name in BUILTIN_PREDICATES {
        assert!(
            is_builtin_predicate_name(name),
            "{name} 必须在内置谓词单一判定口里"
        );
        assert!(semantic.is_predicate(name), "探针必须把 {name} 判为谓词");
        assert!(
            matches!(semantic.kind_of(name), NameKind::Predicate),
            "{name} 的定种必须是 Predicate"
        );
    }
    // 负向：普通标识符不是谓词（防「全 true」把判据做废）
    assert!(
        !is_builtin_predicate_name("Positive") && !semantic.is_predicate("Positive"),
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
    // NullProbe 不携带任何类型知识（生产入口必须显式注入语义探针）
    assert!(
        matches!(null.kind_of("Int"), NameKind::UserType)
            && matches!(null.kind_of("Add"), NameKind::UserType),
        "NullProbe 不携带任何类型知识，定种恒为 UserType"
    );
}

#[test]
fn test_semantic_probe_const_param_types_match_spec_without_widening() {
    // Arrange：L3 的 const 参数名表 + 探针 + 内置名全集（12 个规范拼写）
    let semantic = SemanticProbe;
    let all_builtin_spellings = [
        "Never", "Void", "()", "Bool", "Int", "Int8", "Int16", "Int32", "Int64", "Float",
        "Float32", "Float64", "Char", "String", "Bytes",
    ];

    // Act & Assert ①：表 = 规格清单（任一漂移即红）
    assert_eq!(
        CONST_PARAM_TYPES, SPEC_CONST_PARAM_TYPES,
        "CONST_PARAM_TYPES 必须与语言级规格清单逐名一致"
    );
    // Assert ②：窄入口 = 探针接受面
    for name in CONST_PARAM_TYPES {
        assert!(
            is_const_param_type_name(name) && semantic.is_const_param_type(name),
            "{name} 在表里，窄入口与探针都必须接受"
        );
    }
    // Assert ③：**不得宽化成 kind_of == Builtin**（6.6.9 的硬判据）
    for name in all_builtin_spellings {
        let is_builtin = matches!(semantic.kind_of(name), NameKind::Builtin);
        let is_const_param = semantic.is_const_param_type(name);
        if !CONST_PARAM_TYPES.contains(&name) {
            assert!(
                is_builtin && !is_const_param,
                "{name} 是内置名但不是 const 参数类型：探针不得放行（集合不许宽化）"
            );
        }
    }
}

#[test]
fn test_semantic_probe_and_null_probe_differ_on_const_param() {
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
                type_annotation, ..
            } => {
                let params = type_annotation
                    .as_ref()
                    .map(crate::frontend::core::parser::ast::Type::fn_params_flat)
                    .unwrap_or_default();
                if params.is_empty() {
                    None
                } else {
                    Some(params)
                }
            }
            _ => None,
        })
        .expect("应有一条带签名形参的赋值语句");

    // Act：语义探针（生产入口的形态）vs NullProbe（默认语法入口）
    let with_semantic: Vec<String> = extract_generic_param_names(&params, &SemanticProbe)
        .into_iter()
        .map(|p| p.name)
        .collect();
    let with_null: Vec<String> = extract_generic_param_names(&params, &NullProbe)
        .into_iter()
        .map(|p| p.name)
        .collect();

    // Assert：语义探针下 const 参数 N 必须登记；NullProbe 下不登记——
    // 这正是「默认入口不得静默降级」的另一面：生产入口必须注入语义探针。
    assert_eq!(
        with_semantic,
        vec!["N".to_string()],
        "注入语义探针时 (N: Int) 必须登记为 const 参数"
    );
    assert!(
        with_null.is_empty(),
        "NullProbe 不携带类型知识，不得登记 const 参数，实际 {with_null:?}"
    );
}
