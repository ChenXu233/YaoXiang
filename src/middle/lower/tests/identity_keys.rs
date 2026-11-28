//! lower 两张类型表的身份视图守卫（L3-1 收尾，D64）
//!
//! 判据：`struct_definitions` / `sum_type_variants` 是「本地键 + 身份视图」
//! 两笔同源写入（唯一写入口 `register_struct_definition` / 构造期一次装配）；
//! 身份已知的查询命中该定义**自己的**字段/变体序，同名类型（a.T / b.T）不共用
//! 条目。身份不得从名字串反解（设计 §2.5 执行口径）。
//!
//! 文件 ↔ 被测源：`src/middle/lower/mod.rs`（register_struct_definition /
//! sum_variants_for）、`src/middle/lower/query.rs`（resolve_field_index 的身份臂）。

use crate::ast::{self, StructField};
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::types::TypeCheckResult;
use crate::frontend::core::types::identity::Nominal;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::lower::AstToIrGenerator;
use crate::util::span::Span;

/// 单字段结构体的字段表（a.T 的 shapes）。
fn one_field() -> Vec<StructField> {
    vec![StructField::new(
        "v".to_string(),
        false,
        ast::Type::Name {
            name: "Float".to_string(),
            span: Span::dummy(),
        },
    )]
}

/// 两字段结构体的字段表（b.T 的 shapes：x 在前、y 在后）。
fn two_fields() -> Vec<StructField> {
    vec![
        StructField::new(
            "x".to_string(),
            false,
            ast::Type::Name {
                name: "Float".to_string(),
                span: Span::dummy(),
            },
        ),
        StructField::new(
            "y".to_string(),
            false,
            ast::Type::Name {
                name: "Float".to_string(),
                span: Span::dummy(),
            },
        ),
    ]
}

/// 身份视图与裸名视图同笔写：同名定义各占一条身份条目，裸名视图保留既有
/// last-wins 兼容面（它不是身份判据，只是写名查询的既有入口）。
#[test]
fn test_lower_struct_identity_view_keeps_same_named_definitions_apart() {
    // Arrange — 同一生成器登记两个同名类型（不同模块键）
    let type_result = TypeCheckResult::default();
    let mut generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::new(),
        Some("main".to_string()),
        TypeLevelNamespace::default(),
    );
    let a_t = Nominal::item("a", "T").expect("a.T");
    let b_t = Nominal::item("b", "T").expect("b.T");
    generator.register_struct_definition(a_t.clone(), one_field());
    generator.register_struct_definition(b_t.clone(), two_fields());

    // Assert — 身份视图两条、裸名视图一条；每个身份都命中自己的字段序
    assert_eq!(
        generator.struct_definitions_by_identity.len(),
        2,
        "同名类型必须各占一条身份条目"
    );
    assert_eq!(
        generator.struct_definitions.len(),
        1,
        "裸名视图保留既有 last-wins 兼容面（写名域，不是身份判据）"
    );
    assert!(
        generator
            .struct_definitions_by_identity
            .values()
            .all(|fields| fields.iter().any(|f| f.name == "v")
                || fields.iter().any(|f| f.name == "y")),
        "身份视图的每条都必须是登记时的字段表"
    );
    let a_fields = generator
        .struct_fields_by_identity(&a_t)
        .expect("a.T 的身份视图必须存在");
    assert!(
        a_fields.iter().any(|f| f.name == "v") && a_fields.len() == 1,
        "a.T 的字段序必须来自 a 的定义（v 一个字段），实际: {:?}",
        a_fields
            .iter()
            .map(|f| (&f.name, f.quantity))
            .collect::<Vec<_>>()
    );
    let b_fields = generator
        .struct_fields_by_identity(&b_t)
        .expect("b.T 的身份视图必须存在");
    assert_eq!(
        b_fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        vec!["x", "y"],
        "b.T 的字段序必须来自 b 的定义"
    );
    // 守卫：身份视图的每个裸名键都能在裸名视图里找到（两笔同源，不出现分叉）
    for identity in generator.struct_definitions_by_identity.keys() {
        assert!(
            generator.struct_definitions.contains_key(identity.bare()),
            "身份 {} 的裸名键必须同时写进裸名视图",
            identity.project()
        );
    }
}

/// 和类型身份视图：本地键经 typecheck 的派生索引反查身份；带模块的身份不回退裸名。
#[test]
fn test_lower_sum_identity_view_resolves_by_identity_not_bare_name() {
    // Arrange — 两个同名和类型（a.Color / b.Color）各一组变体，本地键为别名
    let mut type_result = TypeCheckResult::default();
    let flat = |name: &str| Nominal::item("", name).expect("平路径身份");
    let a_color = Nominal::item("a", "Color").expect("a.Color");
    let b_color = Nominal::item("b", "Color").expect("b.Color");
    type_result
        .type_identity_index
        .insert(a_color.clone(), "ColorA".to_string());
    type_result
        .type_identity_index
        .insert(b_color.clone(), "ColorB".to_string());
    type_result
        .sum_types
        .insert("ColorA".to_string(), vec![variant("red"), variant("blue")]);
    type_result.sum_types.insert(
        "ColorB".to_string(),
        vec![variant("green"), variant("cyan")],
    );
    let generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::new(),
        Some("main".to_string()),
        TypeLevelNamespace::default(),
    );

    // Assert — 身份查询各得自己的变体序；裸名（平路径）仍可查
    let a_variants = generator
        .sum_variants_for(&a_color)
        .expect("a.Color 的变体必须按身份可查");
    assert_eq!(
        a_variants.iter().map(|v| v.0.as_str()).collect::<Vec<_>>(),
        vec!["red", "blue"],
        "a.Color 的变体序必须来自 a 的声明"
    );
    let b_variants = generator
        .sum_variants_for(&b_color)
        .expect("b.Color 的变体必须按身份可查");
    assert_eq!(
        b_variants.iter().map(|v| v.0.as_str()).collect::<Vec<_>>(),
        vec!["green", "cyan"],
        "b.Color 的变体序必须来自 b 的声明"
    );
    assert!(
        generator.sum_variants_for(&flat("ColorA")).is_some(),
        "平路径身份（单文件/本地键同名形态）仍按本地键可查"
    );
    assert!(
        generator
            .sum_variants_for(&Nominal::item("c", "Color").unwrap())
            .is_none(),
        "带模块的未知身份不得回退裸名（否则同名串味重现）"
    );
}

/// 和类型变体定义的最小夹具。
fn variant(name: &str) -> crate::frontend::core::sema::environment::SumVariantDef {
    crate::frontend::core::sema::environment::SumVariantDef {
        name: name.to_string(),
        params: Vec::new(),
    }
}
