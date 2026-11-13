//! `aggregate_instantiation_requests` 的 helper 级单测（t4 R4）。
//!
//! 该函数是 t2 引入的驱动侧聚合口，此前只有端到端用例覆盖；这里直接喂
//! 「单元 + 单元 IR + typecheck 产物」三件套，钉住两条限定化契约：
//! 1. 请求名与 IR 调用指令**同源**——本单元同名顶层函数按单元键限定化
//!    （`pick` → `main.pick`），跨模块限定名（`lib.f`）与 std 名原样保留；
//! 2. `containing_fn` 的裸名同样按单元键限定化（deferred 桶键与 merged IR 同源），
//!    且**只在**该单元确有这个函数时才改（其余形态保持原样，不猜）。

use crate::driver::helpers::aggregate_instantiation_requests;
use crate::driver::unit::Unit;
use crate::frontend::core::sema::TypeCheckResult;
use crate::frontend::core::types::MonoType;
use crate::middle::ir::ir::{FunctionBody, FunctionIR, ModuleIR};
use crate::middle::passes::mono::instance::{GenericFunctionId, InstantiationRequest};
use crate::util::span::Span;

/// 调聚合函数（helper 级测试用空 AST 表 + 空注册表：这些用例只钉单元键限定化，
/// 不涉及别名通道；别名通道由 `test_aggregate_resolves_named_import_...` 单独钉）。
fn aggregate(
    units: &[Unit],
    unit_irs: &[Option<ModuleIR>],
    type_results: &[Option<&TypeCheckResult>],
) -> Vec<InstantiationRequest> {
    let asts: Vec<Option<crate::frontend::core::parser::Module>> = vec![None; units.len()];
    let registry = crate::frontend::module::registry::ModuleRegistry::new();
    aggregate_instantiation_requests(units, unit_irs, type_results, &asts, &registry)
}
/// 造一个「只有一个空函数体函数」的单元 IR（函数名即该单元的函数表键）。
fn unit_ir(function_names: &[&str]) -> ModuleIR {
    let functions: Vec<FunctionIR> = function_names
        .iter()
        .map(|name| FunctionIR {
            def: None,
            name: (*name).to_string(),
            params: Vec::new(),
            return_type: MonoType::Void,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: Vec::new(),
                entry: 0,
                locals: Vec::new(),
            },
        })
        .collect();
    ModuleIR {
        functions,
        ..Default::default()
    }
}

/// 造一条实参为 `Int(64)` 的请求（名字与容器按参数给）。
fn request(
    name: &str,
    containing_fn: Option<&str>,
) -> InstantiationRequest {
    let mut req = InstantiationRequest::new(
        GenericFunctionId::new(name.to_string(), vec!["T".to_string()]),
        vec![crate::frontend::core::types::mono::GenericArg::Type(
            MonoType::Int(64),
        )],
        Span::default(),
    );
    req.containing_fn = containing_fn.map(str::to_owned);
    req
}

/// 造带指定请求的 typecheck 产物。
fn type_result(requests: Vec<InstantiationRequest>) -> TypeCheckResult {
    // 用 Default + 赋值（结构体字面量需要 `pub(super)` 的 proof_calls 可见，
    // 本测试在 typecheck 树外，只能走公开构造路径）
    let mut result = TypeCheckResult::default();
    result.instantiation_requests = requests;
    result
}

/// 请求名：本单元同名顶层函数 → 按单元键限定化（与 IR 调用指令同源）。
#[test]
fn test_aggregate_qualifies_local_request_name_with_unit_key() {
    // Arrange：main 单元定义了 pick，请求名是裸名 pick（同模块调用）
    let units = vec![Unit::new("main", "main.yx", "")];
    let irs = vec![Some(unit_ir(&["main.pick", "main.main"]))];
    let result = type_result(vec![request("pick", Some("main"))]);
    let results = vec![Some(&result)];

    // Act
    let aggregated = aggregate(&units, &irs, &results);

    // Assert
    assert_eq!(
        aggregated
            .iter()
            .map(|r| (r.generic_id().name().to_string(), r.containing_fn.clone()))
            .collect::<Vec<_>>(),
        vec![("main.pick".to_string(), Some("main.main".to_string()))],
        "本单元同名函数的请求名与容器名都必须按单元键限定化"
    );
}

/// t10：**具名导入**的请求名必须经别名通道解析到导入目标（`pick` → `lib.pick`），
/// 而不是被单元键缀成 `main.pick`（不存在）——后者正是 t6 回归 E3018 的成因。
#[test]
fn test_aggregate_resolves_named_import_request_through_alias_channel() {
    // Arrange：main 单元 use lib.{pick}，请求名是裸名 pick；lib 单元定义 lib.pick
    let units = vec![
        Unit::new("main", "main.yx", ""),
        Unit::new("lib", "lib.yx", ""),
    ];
    let irs = vec![Some(unit_ir(&["main.main"])), Some(unit_ir(&["lib.pick"]))];
    // 入口单元的真实 `use lib.{pick}` AST + 同一 registry：聚合函数据此调
    // ir_gen 的 import_aliases_from 取别名表（与生成期同源）
    let tokens =
        crate::frontend::core::lexer::tokenize("use lib.{pick}\n").expect("use 行可词法分析");
    let parsed = crate::frontend::core::parser::parse(&tokens);
    assert!(!parsed.has_errors, "use 行必须可解析：{:?}", parsed.errors);
    let asts = vec![Some(parsed.module), None];
    let mut registry = crate::frontend::module::registry::ModuleRegistry::new();
    let mut lib = crate::frontend::module::ModuleInfo::new(
        "lib".to_string(),
        crate::frontend::module::ModuleSource::User,
    );
    lib.add_export(crate::frontend::module::Export {
        name: "pick".to_string(),
        full_path: "lib.pick".to_string(),
        kind: crate::frontend::module::ExportKind::Function,
        signature: String::new(),
        mono_type: None,
        type_params: None,
        param_names: None,
        type_payload: None,
        definition_span: None,
        predicate_def: None,
        proof_fn_params: None,
        const_binders: Vec::new(), // 测试桩导出无登记面（#539 载荷面）
        call_site_params: Vec::new(),
    });
    registry.register(lib);
    let result = type_result(vec![request("pick", None)]);
    let results = vec![Some(&result), None];

    // Act
    let aggregated = aggregate_instantiation_requests(&units, &irs, &results, &asts, &registry);

    // Assert：请求名 = lib.pick（别名通道），不是 main.pick（单元键误缀）
    assert_eq!(
        aggregated
            .iter()
            .map(|r| r.generic_id().name().to_string())
            .collect::<Vec<_>>(),
        vec!["lib.pick".to_string()],
        "具名导入的请求名必须经别名通道解析到 lib.pick（与 IR 调用指令同源）"
    );
}
