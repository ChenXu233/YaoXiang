//! 表达式推断测试 — 基于语言规范 §3.2 & §4 & RFC-010
//!
//! §3.2: 原类型（Int 默认 8 字节 = Int(64)）
//! §4.1-§4.10: 表达式分类
//! RFC-010: 统一类型语法
//! RFC-029: 模块限定名 `{module_key}.{bare}`（`lower/module_ir.rs` 的 `qualify_names` 与
//!   `DefPath::project` 同源）
//! RFC-039 §3.4.8 ①: 跨模块限定调用 `lib.f(x)` 发泛型实例化请求（generic_id
//!   用限定名，与 merged-IR 的函数表键对齐）
//! issue #509 残留④（WBS 3.21.5）：二元运算 Never 短路判据收敛为共享纯函数
//!   （expressions::never_short_circuit）——裁决表 pin（两遍历器同输入同裁决）

use crate::ast::Expr;
use crate::frontend::core::sema::inference::expressions::ExpressionInferrer;
use crate::frontend::core::sema::inference::scope::ScopeManager;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::types::{MonoType, TypeConstraintSolver};
use crate::util::span::Span;

use std::collections::HashMap;

use crate::frontend::core::sema::TypeCheckResult;
use crate::frontend::module::ModuleInfo;
use crate::middle::passes::mono::instance::InstantiationRequest;

/// 按多文件编排的导出提取管线（tokenize → parse → extract_module_info）构造
/// 模块导出面——full_path 为 `{module_key}.{bare}`，与 `build_registry_from` 同源。
fn module_info_from_source(
    module_key: &str,
    source: &str,
) -> ModuleInfo {
    let tokens = crate::frontend::core::lexer::tokenize(source).expect("模块源码必须可词法分析");
    let parsed = crate::frontend::core::parser::parse(&tokens);
    assert!(
        !parsed.has_errors,
        "模块源码必须可解析：{:?}",
        parsed.errors
    );
    crate::frontend::module::orchestrator::extract_module_info(
        module_key,
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    )
}

/// 以真实 tokenize → parse → check 管线检查入口源码；`modules` 预先注册进
/// 检查器的模块注册表（等价于多文件编排预构建的用户模块集）。
fn check_entry_with_modules(
    source: &str,
    modules: Vec<ModuleInfo>,
) -> TypeCheckResult {
    let tokens = crate::frontend::core::lexer::tokenize(source).expect("入口源码必须可词法分析");
    let parsed = crate::frontend::core::parser::parse(&tokens);
    assert!(
        !parsed.has_errors,
        "入口源码必须可解析：{:?}",
        parsed.errors
    );
    let mut checker = crate::frontend::core::sema::TypeChecker::new("main");
    for module in modules {
        checker.env().module_registry.register(module);
    }
    checker.check_module_collect_all(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    )
}

/// 实例化请求快照：`(generic_id 名, 声明序类型参数名, 类型实参)`。
fn request_triples(requests: &[InstantiationRequest]) -> Vec<(String, Vec<String>, Vec<MonoType>)> {
    requests
        .iter()
        .map(|r| {
            (
                r.generic_id().name().to_string(),
                r.generic_id().type_params().to_vec(),
                r.type_args().to_vec(),
            )
        })
        .collect()
}

/// Test context that owns the dependencies borrowed by ExpressionInferrer.
struct TestContext {
    scope: ScopeManager,
    solver: TypeConstraintSolver,
    overload_candidates:
        HashMap<String, Vec<crate::frontend::core::sema::passes::overload::OverloadCandidate>>,
}

impl TestContext {
    fn new() -> Self {
        Self {
            scope: ScopeManager::new(),
            solver: TypeConstraintSolver::default(),
            overload_candidates: HashMap::new(),
        }
    }

    fn inferrer(&mut self) -> ExpressionInferrer<'_> {
        ExpressionInferrer::new(&mut self.scope, &mut self.solver, &self.overload_candidates)
    }
}

// Happy path 测试

#[test]
fn test_expression_inferrer_creation() {
    // Arrange
    let mut ctx = TestContext::new();
    let mut inferrer = ctx.inferrer();

    // Assert - 验证创建后能正常推断
    let expr = Expr::Lit(
        crate::frontend::core::lexer::tokens::Literal::Int(0),
        Span::dummy(),
    );
    let result = inferrer.infer_expr(&expr);
    assert!(
        result.is_ok(),
        "newly created inferrer should handle basic expressions"
    );
}

/// §3.2: 整数字面量默认推断为 Int(64)（8 字节）
#[test]
fn test_infer_integer_literal() {
    // Arrange
    let mut ctx = TestContext::new();
    let mut inferrer = ctx.inferrer();
    let expr = Expr::Lit(
        crate::frontend::core::lexer::tokens::Literal::Int(42),
        Span::dummy(),
    );

    // Act
    let result = inferrer.infer_expr(&expr);

    // Assert - 规范 §3.2：Int 默认 8 字节 = Int(64)
    assert!(result.is_ok(), "should infer integer literal");
    assert_eq!(
        result.unwrap(),
        MonoType::Int(64),
        "integer literal should be Int(64) per spec §3.2"
    );
}

// Error path 测试

/// §4: 未定义变量应报错
#[test]
fn test_infer_undefined_variable() {
    // Arrange
    let mut ctx = TestContext::new();
    let mut inferrer = ctx.inferrer();
    let expr = Expr::Var("undefined_var".to_string(), Span::dummy());

    // Act
    let result = inferrer.infer_expr(&expr);

    // Assert
    assert!(result.is_err(), "should fail for undefined variable");
}

// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// RFC-039 §3.4.8 ①：跨模块限定调用 lib.f(x) 的泛型实例化请求
// ─────────────────────────────────────────────────────────────────────────────

/// RFC-039 §3.4.8 ① / RFC-029：`use lib` 后限定调用泛型函数 `lib.f(x)` 必须
/// 发出实例化请求，且 `generic_id` 用与 merged-IR 同源的限定名 `lib.f`
///（`lower/module_ir.rs` 的 `qualify_names` 把每个模块的顶层函数重写为 `{module_key}.{bare}`；
/// 用裸名 `f` 在 merged-IR 的函数表里匹配不到，请求会被下游静默跳过）。
///
/// D2 裁甲（2026-10-10）：用户模块导出随 `Export::type_params` 携带声明期
/// 类型参数名快照，故 `generic_id` 的类型参数名由原先的空表变为 ["T"]——
/// 声明名是权威来源，mono 单态化按它给特化版本命名。
#[test]
fn test_instantiation_request_qualified_module_call_uses_module_qualified_name() {
    // Arrange：lib 导出泛型 f: (T) -> T；入口整体导入后限定调用
    let lib = module_info_from_source("lib", "f: (T: Type) -> (x: T) -> T = (x) => x\n");
    let source = "use lib\nv = lib.f(42)\n";

    // Act：真实 tokenize → parse → check 管线
    let result = check_entry_with_modules(source, vec![lib]);

    // Assert：请求携带限定名 lib.f，实参已解成 Int(64)
    assert!(
        result.diagnostics.is_empty(),
        "限定调用不应产生诊断，实际：{:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        request_triples(&result.instantiation_requests),
        vec![(
            "lib.f".to_string(),
            vec!["T".to_string()],
            vec![MonoType::Int(64)],
        )],
        "限定调用的 generic_id 必须是限定名 lib.f 且携带导出快照的声明名 T"
    );
}

/// RFC-039 §3.4.8 ①：限定名必须能按名解析出被调方的声明序类型参数名——
/// `use std.list` 的 `list.len` 声明为 `(A: Type) -> (&Vec(A)) -> Int`，
/// 请求的 `generic_id` 名 = `std.list.len`、类型参数名 = ["A"]。
#[test]
fn test_instantiation_request_qualified_std_call_resolves_declared_type_params() {
    // Arrange：std.list 由 with_std 预载（含 yx 表面导出），入口限定调用 len
    let source = "use std.list\nxs = [1, 2]\nv = list.len(xs)\n";

    // Act
    let result = check_entry_with_modules(source, Vec::new());

    // Assert：限定名 + 声明名 A（限定名解析不出声明名时 arity 无从判定，请求发不出）
    assert!(
        result.diagnostics.is_empty(),
        "限定调用不应产生诊断，实际：{:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        request_triples(&result.instantiation_requests),
        vec![(
            "std.list.len".to_string(),
            vec!["A".to_string()],
            vec![MonoType::Int(64)]
        )],
        "限定名 std.list.len 必须解析出声明序类型参数名 [\"A\"]"
    );
}

/// 行为守恒（验收红线）：无 `use` 的单文件路径一字不改——裸 `Var` 调用仍发
/// **裸名**请求（不带模块前缀），声明名与实参照旧。
#[test]
fn test_instantiation_request_bare_call_keeps_unqualified_name() {
    // Arrange：本模块定义泛型 identity，无模块别名世界
    let source = "identity: (T: Type) -> (x: T) -> T = (x) => x\nv = identity(42)\n";

    // Act
    let result = check_entry_with_modules(source, Vec::new());

    // Assert
    assert!(
        result.diagnostics.is_empty(),
        "泛型裸调用不应产生诊断，实际：{:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        request_triples(&result.instantiation_requests),
        vec![(
            "identity".to_string(),
            vec!["T".to_string()],
            vec![MonoType::Int(64)]
        )],
        "裸 Var 调用的请求名保持裸名 identity（不得被限定化）"
    );
}

/// #511：接口约束形态 `(T: Add)`（type-system.md:151）的跨单元请求必须携带
/// 声明名——修前登记口只认 `(T: Type)`（MetaType），`Export::type_params`
/// 为 None ⇒ 请求 `type_params` 为空表，走签名反推兜底（#510 探针实证）。
#[test]
fn test_instantiation_request_qualified_call_carries_interface_constraint_param_name() {
    // Arrange：lib 导出接口约束泛型 f，入口整体导入后限定调用
    let lib = module_info_from_source("lib", "f: (T: Add) -> (x: T) -> T = (x) => x\n");
    let source = "use lib\nv = lib.f(42)\n";

    // Act：真实 tokenize → parse → check 管线
    let result = check_entry_with_modules(source, vec![lib]);

    // Assert：请求携带限定名 lib.f 且带声明名 ["T"]
    assert!(
        result.diagnostics.is_empty(),
        "限定调用不应产生诊断，实际：{:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        request_triples(&result.instantiation_requests),
        vec![(
            "lib.f".to_string(),
            vec!["T".to_string()],
            vec![MonoType::Int(64)],
        )],
        "(T: Add) 接口约束的跨单元请求必须携带导出快照的声明名 [\"T\"]（不得退回空表反推）"
    );
}

/// #511 行为守恒锚：const 泛型限定调用 `lib.f(42)(5)` **不发实例化请求**——
/// const 形态的单态化走 const_binders → mono 编译期求值面，不经
/// `instantiation_requests`（单模块对照实测同样零请求）。#511 的登记口扩容
/// 只改名字表，不得让 const 调用意外开始发请求；`(N: Int)` 的声明名兑现面
/// 是导出携带（`Export::type_params`，见 orchestrator 的
/// `export_type_params_tests`）。
#[test]
fn test_instantiation_request_qualified_const_call_stays_request_free() {
    // Arrange：lib 导出 const 泛型 f，入口整体导入后两段 curry 调用
    //（首段 42 是编译期 const 实参，末段 5 是运行时值实参）
    let lib = module_info_from_source("lib", "f: (N: Int) -> (n: N) -> Int = (n) => n\n");
    let source = "use lib\nv = lib.f(42)(5)\n";

    // Act：真实 tokenize → parse → check 管线
    let result = check_entry_with_modules(source, vec![lib]);

    // Assert：零诊断 + 零请求（守恒，与单模块形态一致）
    assert!(
        result.diagnostics.is_empty(),
        "const 泛型限定调用不应产生诊断，实际：{:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert!(
        result.instantiation_requests.is_empty(),
        "const 泛型调用不得发实例化请求（其单态化不经该通道），实际：{:?}",
        request_triples(&result.instantiation_requests)
    );
}

/// #511 / 3.20.5 t3-low② 收口：混合签名（约束段 `(T: Add)` + 值依赖 const 段
/// `(N: Int)`）的跨单元**全柯里调用**与纯 const 形态同构——**零请求**。实测
/// （探针先行）其单态化不经 `instantiation_requests` 通道；t3 提出的
/// 「param_names.position 取 last_type_args[idx] 对齐性」疑虑因此在请求通道
/// 上不成立（该疑虑所在的 E1106 约束复检路径系结构性死码，已经 2026-10-31
/// 用户裁决甲案删除，见 RFC-011b 附录B 与 issue #519）。
/// 未覆盖边界：混合签名的部分应用（调用停在 const 段之前）无测试。
#[test]
fn test_instantiation_request_qualified_mixed_signature_call_stays_request_free() {
    // Arrange：lib 导出混合签名 f（约束段 T + const 段 N，返回类型引用 N）；
    // 约束段不吃调用位（T 从值实参类型反推），调用位 = N 的 const 实参 + x + n
    let lib = module_info_from_source(
        "lib",
        "f: (T: Add) -> (N: Int) -> (x: T) -> (n: N) -> N = (x, n) => n\n",
    );
    let source = "use lib\nv = lib.f(7)(1)(2)\n";

    // Act：真实 tokenize → parse → check 管线
    let result = check_entry_with_modules(source, vec![lib]);

    // Assert：零诊断 + 零请求（与纯 const 形态守恒一致）
    assert!(
        result.diagnostics.is_empty(),
        "混合签名限定调用不应产生诊断，实际：{:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert!(
        result.instantiation_requests.is_empty(),
        "混合签名全柯里调用不得发实例化请求（与纯 const 形态守恒一致），实际：{:?}",
        request_triples(&result.instantiation_requests)
    );
}

/// 边界：被调方是 FieldAccess 但接收者是**值**（方法调用），不是导入模块别名——
/// 不产生任何实例化请求（请求只属于模块限定调用 `lib.f(x)`）。
#[test]
fn test_instantiation_request_value_receiver_method_call_not_collected() {
    // Arrange：Point 类型上声明方法 get_x，值为接收者调用 p.get_x()
    let source =
        "Point: Type = { x: Int }\nPoint.get_x: (self: &Point) -> Int = (self) => self.x\n\
                p = Point(1)\nv = p.get_x()\n";

    // Act
    let result = check_entry_with_modules(source, Vec::new());

    // Assert：值接收者方法调用不是模块限定调用，零请求
    assert!(
        result.diagnostics.is_empty(),
        "方法调用不应产生诊断，实际：{:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    assert!(
        result.instantiation_requests.is_empty(),
        "值接收者方法调用不得产生实例化请求，实际：{:?}",
        request_triples(&result.instantiation_requests)
    );
}

/// RFC-039 §3.4.8 ①（钉测试）：跨模块限定调用的**声明名前**权威来源——导出快照。
///
/// 探针形态：被调泛型函数 read 的类型参数 T 只出现在嵌套/借用位置
/// （形参 x: &T，整份签名里没有裸 T）——类型实参本应由形参位的借用解出，
/// 而声明序声明名 ["T"] 只能来自被调模块的导出快照；签名形态反推在这类
/// 形态上什么也收集不到。用户模块导出硬编码 type_params: None 时请求不发出
/// （mono 不特化 → 运行时报 E6006）。
#[test]
fn test_qualified_generic_call_borrowed_param_uses_export_declared_type_params() {
    // Arrange：lib 导出泛型 read: (T) -> (&T) -> T；类型参数只出现在借用内层
    let lib = module_info_from_source("lib", "read: (T: Type) -> (x: &T) -> T = (x) => x\n");
    let source = "use lib\nv = lib.read(7)\n";

    // Act：真实 tokenize → parse → check 管线
    let result = check_entry_with_modules(source, vec![lib]);

    // Assert：诊断为空 + 请求带限定名 lib.read 与声明序声明名 ["T"]
    assert!(
        result.diagnostics.is_empty(),
        "限定调用不应产生诊断，实际：{:?}",
        result
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );
    let triples = request_triples(&result.instantiation_requests);
    assert_eq!(
        triples.len(),
        1,
        "被调泛型函数的类型参数只在借用内层时，仍必须发出恰好一条实例化请求，实际：{triples:?}"
    );
    assert_eq!(
        triples[0].0, "lib.read",
        "generic_id 名必须是限定名 lib.read（与 merged-IR 函数表键同源），实际：{:?}",
        triples[0].0
    );
    assert_eq!(
        triples[0].1,
        vec!["T".to_string()],
        "声明序类型参数名必须由导出快照携带（用户模块 Export::type_params），实际：{:?}",
        triples[0].1
    );
}

// ============ issue #509 残留④锚（WBS 3.21.5）============

/// 判据（issue #509 残留④，门 = 病史 + 证明）：二元运算 Never 短路判据原为
/// `ExpressionInferrer::infer_binary` 与 `StatementChecker` 二元运算臂两份
/// 逐字拷贝（仅注释互指），一处改另一处忘改即两路径裁决分歧（#469 处 3 引入、
/// #509 收敛为共享纯函数）。本测试钉住裁决表——两个遍历器现在调用**同一个**
/// 函数，同输入必得同裁决；任何一侧的裁决变化都必然先过这张表。
#[test]
fn test_never_short_circuit_criterion_pins_shared_verdict_for_both_traversers() {
    use crate::frontend::core::sema::inference::expressions::{
        never_short_circuit, NeverShortCircuit,
    };

    // Arrange：四个输入形态——两侧 Never / 仅左 / 仅右 / 两侧具体类型
    let never = MonoType::Never;
    let int = MonoType::Int(64);

    // Act & Assert：裁决表（两遍历器共用的唯一实现）
    assert!(
        matches!(
            never_short_circuit(&never, &never),
            Some(NeverShortCircuit::Both)
        ),
        "两侧都是 Never 必须裁决 Both（调用方发 fresh var 推迟）"
    );
    assert!(
        matches!(
            never_short_circuit(&never, &int),
            Some(NeverShortCircuit::TakeRight)
        ),
        "仅左侧 Never 必须裁决 TakeRight（结果取右侧具体形态）"
    );
    assert!(
        matches!(
            never_short_circuit(&int, &never),
            Some(NeverShortCircuit::TakeLeft)
        ),
        "仅右侧 Never 必须裁决 TakeLeft（结果取左侧具体形态）"
    );
    assert!(
        never_short_circuit(&int, &int).is_none(),
        "两侧都非 Never 必须返回 None（按正常推断走，不短路）"
    );
}
