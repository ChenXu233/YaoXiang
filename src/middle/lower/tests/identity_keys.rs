//! lower 两张类型表的身份视图守卫（L3-1 收尾，D64）
//!
//! 判据：`struct_definitions` / `sum_type_variants` 是「本地键 + 身份视图」
//! 两笔同源写入（唯一写入口 `register_struct_definition` / 构造期一次装配）；
//! 身份已知的查询命中该定义**自己的**字段/变体序，同名类型（a.T / b.T）不共用
//! 条目。身份不得从名字串反解（设计 §2.5 执行口径）。
//!
//! 文件 ↔ 被测源：`src/middle/lower/mod.rs`（register_struct_definition /
//! sum_variants_for / identity_for_written）、`src/middle/lower/query.rs`
//! （resolve_field_index 的身份臂）、`src/frontend/core/sema/checker.rs`
//! （TypeCheckResult 携带的正向身份表 = 运输面）。

use crate::ast::{self, StructField};
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_with_probe;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::{SemanticProbe, TypeLevelNamespace};
use crate::frontend::core::sema::types::TypeCheckResult;
use crate::frontend::core::types::identity::Nominal;
use crate::frontend::module::orchestrator;
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::module::ModuleInfo;
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

/// 和类型身份视图：本地键经 typecheck 同笔携带的**正向表**取身份；带模块的身份不回退裸名。
#[test]
fn test_lower_sum_identity_view_resolves_by_identity_not_bare_name() {
    // Arrange — 两个同名和类型（a.Color / b.Color）各一组变体，本地键为别名
    let mut type_result = TypeCheckResult::default();
    let flat = |name: &str| Nominal::item("", name).expect("平路径身份");
    let a_color = Nominal::item("a", "Color").expect("a.Color");
    let b_color = Nominal::item("b", "Color").expect("b.Color");
    type_result
        .type_identity_by_local_key
        .insert("ColorA".to_string(), a_color.clone());
    type_result
        .type_identity_by_local_key
        .insert("ColorB".to_string(), b_color.clone());
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

/// 消费侧：别名双键都解析到同一身份（第二键不落空）；身份存在时不得回退 flat。
///
/// 三形态：别名双键（T/TB → 同一 a.T）、双登记同键（首个为准，见生产者侧
/// environment tests）、flat（无身份 → 调用方回退平路径）。
#[test]
fn test_lower_written_name_identity_resolves_alias_and_definition_keys() {
    // Arrange：同一身份 a.T 以定义名 T 与别名 TB 两个写名登记；变体表只在 T 下
    let mut type_result = TypeCheckResult::default();
    let identity = Nominal::item("a", "T").expect("a.T");
    type_result
        .type_identity_by_local_key
        .insert("T".to_string(), identity.clone());
    type_result
        .type_identity_by_local_key
        .insert("TB".to_string(), identity.clone());
    type_result
        .sum_types
        .insert("T".to_string(), vec![variant("red"), variant("blue")]);
    let generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::new(),
        Some("main".to_string()),
        TypeLevelNamespace::default(),
    );

    // Assert：两个写名都取到同一身份（第二键不落空），且不是 flat 回退
    assert_eq!(
        generator.identity_for_written("T"),
        Some(identity.clone()),
        "定义名键必须解析到身份"
    );
    assert_eq!(
        generator.identity_for_written("TB"),
        Some(identity.clone()),
        "别名第二键不得落空（旧实现按 value 反查索引时它没有条目）"
    );
    assert!(
        !generator
            .identity_for_written("TB")
            .expect("alias key must resolve")
            .module()
            .is_empty(),
        "身份存在时不得回落 flat"
    );
    let variants = generator
        .sum_variants_for(&identity)
        .expect("身份查询必须命中变体表");
    assert_eq!(
        variants.iter().map(|v| v.0.as_str()).collect::<Vec<_>>(),
        vec!["red", "blue"],
        "同一身份的变体表由首个登记键给出"
    );

    // flat 形态：无身份条目的写名 → 消费侧回退平路径（合法回退，不是猜测）
    assert_eq!(
        generator.identity_for_written("Local"),
        None,
        "flat 写名不在身份表，调用方按平路径回退"
    );
    assert!(
        Nominal::flat("Local").is_some(),
        "flat 回退目标必须是合法平路径"
    );
}

/// 和类型变体定义的最小夹具。
fn variant(name: &str) -> crate::frontend::core::sema::environment::SumVariantDef {
    crate::frontend::core::sema::environment::SumVariantDef {
        name: name.to_string(),
        params: Vec::new(),
    }
}

/// 生产导出构造点（生产唯一导出构造面）：一段源码 → 模块导出面。
fn lib_module(source: &str) -> ModuleInfo {
    let tokens = tokenize(source).expect("lib 源码必须可词法分析");
    let parsed = parse_with_probe(&tokens, &SemanticProbe);
    assert!(
        !parsed.has_errors,
        "lib 源码必须可解析：{:?}",
        parsed.errors
    );
    orchestrator::extract_module_info(
        "lib",
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    )
}

/// 生产导入入口（生产唯一导入写入面）：入口源码 + 已注册模块集 → 类型检查结果。
fn check_entry(
    source: &str,
    modules: Vec<ModuleInfo>,
) -> TypeCheckResult {
    let tokens = tokenize(source).expect("入口源码必须可词法分析");
    let parsed = parse_with_probe(&tokens, &SemanticProbe);
    assert!(
        !parsed.has_errors,
        "入口源码必须可解析：{:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("entry");
    for module in modules {
        checker.env().module_registry.register(module);
    }
    checker.check_module_collect_all(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    )
}

/// 生产端 → 运输面 → 消费端**同一条链**的直驱判据：别名写名必须带定义身份。
///
/// 判据门：**证明**——① 性质：「本单元写名（本地键）→ 类型身份」这条映射由唯一
/// 生产者（`TypeEnvironment::add_type`）写就，并经 `TypeCheckResult` 原样运到
/// 消费端（lower）；别名导入的**第二个写名**在运输中不得丢失（D64：身份不得从
/// 名字串反解，身份存在时不得回落平路径）。② 为什么必须成立：别名写名只经
/// `use lib.{T as TB}` 出现一次，运输丢键后 lower 的构造/变体查询只能回退平路径
/// ——同名不同模块的类型因此串味（D64 明确禁止的静默面）。③ 为什么这里是最便宜
/// 的观察点：它是**唯一**同时握着生产入口（`extract_module_info` + `add_type`）、
/// 运输面（`TypeCheckResult`）与消费面（`identity_for_written`）的位置；两端
/// 各自单测（environment 的正向表 / lower 的身份视图）都证明不了中间那一步没丢。
///
/// 换一个正确的替代实现（换一种运输机制，只要别名写名仍解析到定义身份）它不会
/// 红——它钉的是性质，不是实现形状。
#[test]
fn test_aliased_import_write_name_keeps_definition_identity_end_to_end() {
    // Arrange — lib 声明 T 并以生产导出构造点产出模块导出面；入口以别名 TB 导入
    let lib = lib_module("T: Type = {\n    x: Int,\n}\n");
    let result = check_entry(
        "use lib.{T as TB}\n\nmain: () -> Void = {\n    t = TB(1)\n}\n",
        vec![lib],
    );

    // Assert — 导入面按 (提供模块, 定义名) 登记，别名导入本身不得有诊断
    assert!(
        result.diagnostics.is_empty(),
        "别名导入的类型必须无诊断：{:?}",
        result.diagnostics
    );

    // Act — 用**这份真实的**类型检查结果构造 lower 消费端
    let generator = AstToIrGenerator::new_with_type_result(
        &result,
        ModuleRegistry::new(),
        Some("entry".to_string()),
        TypeLevelNamespace::default(),
    );

    // Assert — 别名写名经运输后仍解析到定义身份（不得落空、不得回落 flat）
    let identity = generator
        .identity_for_written("TB")
        .expect("别名写名 TB 必须经 TypeCheckResult 运到消费端");
    assert_eq!(
        identity.module(),
        "lib",
        "别名写名必须解析到定义模块 lib，实际身份：{identity:?}"
    );
    assert_eq!(
        identity.bare(),
        "T",
        "别名写名必须解析到定义名 T，实际身份：{identity:?}"
    );
}
