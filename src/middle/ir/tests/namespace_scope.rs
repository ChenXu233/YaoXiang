//! 谓词命名空间的**范围**判据（F14-A / RFC-039 D62 修复轮）
//!
//! 规范：`TypeLevelNamespace::scan` 的契约是「多文件项目传**全部单元**」——
//! 走查期（Registry / Typecheck）与生成期（IrGeneration）必须消费**同一份**并集；
//! 生成期不得按本单元自扫（`scan(once(module))`）。
//!
//! 病史：t14 复审 F14-A。生成期曾按本单元自扫，范围比走查期窄；触发条件是
//! #527 跨模块谓词注册落地——之后生成期仍按本单元判 ⇒ 谓词应用实参位被当类型
//! 引用 ⇒ 真值参数误登 `generic_params` ⇒ 单态化删掉该函数 ⇒ 调用侧 E6006/静默缺失，
//! 而走查期判据是对的 ⇒ 两阶段裁决分歧。当前源码形态**不可观测**（跨模块谓词
//! 用于形参位会先在类型检查期被拒），故本文件用**单元级构造**证明
//! 「生成器消费的就是调用方给的那一份」：
//! - 给并集（谓词在另一单元声明）⇒ `generic_params` 为空（与走查期同判）；
//! - 给单元素集 ⇒ `Some(["N"])`（修复前的自扫结果；证明生成期不再自扫）。

use crate::ast;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_with_probe;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::{SemanticProbe, TypeCheckResult};
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::ModuleIR;
use crate::middle::ir::ir_gen::AstToIrGenerator;

/// 词法 + 语法（语义探针入口与生产一致）；语法错误直接失败并列出。
fn parse_module(source: &str) -> ast::Module {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse_with_probe(&tokens, &SemanticProbe);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误，实际: {:?}",
        parsed.errors
    );
    parsed.module
}

/// 用给定命名空间生成一份 IR，取 `f` 的 `generic_params`。
fn f_generic_params(
    module: &ast::Module,
    namespace: TypeLevelNamespace,
) -> Option<Vec<String>> {
    let type_result = TypeCheckResult::default();
    let mut generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::with_std(),
        None,
        namespace,
    );
    let ir: ModuleIR = generator
        .generate_module_ir(module)
        .expect("IR 生成应成功（夹具不触发 IR 级诊断）");
    ir.functions
        .iter()
        .find(|f| f.name == "f")
        .map(|f| f.generic_params.clone())
        .expect("IR 里应有函数 f")
}

/// 规范 / 病史：生成期消费**调用方给的**命名空间——并集判「谓词实参是值」，
/// 单元素集判「谓词实参是类型」（后者正是修复前按本单元自扫的结果）。
#[test]
fn test_generation_consumes_caller_namespace_scope() {
    // Arrange — 谓词 Eq 声明在另一单元；本单元只使用它
    let predicate_unit = parse_module("Eq: (a: Int, b: Int) -> Type = { a == b }\n");
    let using_unit = parse_module(concat!(
        "f: (N: Int, n: Eq(n, N)) -> Int = { n }\n",
        "main: () -> Void = {\n",
        "}\n",
    ));
    let union = TypeLevelNamespace::scan([&predicate_unit, &using_unit]);
    let this_unit_only = TypeLevelNamespace::scan_one(&using_unit);

    // Act
    let with_union = f_generic_params(&using_unit, union.clone());
    let with_single = f_generic_params(&using_unit, this_unit_only);

    // Assert — 并集：谓词可见 ⇒ Eq(n, N) 的实参是值 ⇒ 没有编译期值参数
    assert_eq!(
        with_union, None,
        "并集里 Eq 是谓词 ⇒ N 不是编译期值参数 ⇒ generic_params 必须为空"
    );
    // 负控：同一份 AST、只给本单元 ⇒ 自扫的老结果（证明生成期确实消费入参）
    assert_eq!(
        with_single,
        Some(vec!["N".to_string()]),
        "只给本单元时 Eq 不可见 ⇒ 修复前的自扫结果；生成器若仍自扫，前一条断言不会随入参变化"
    );
}

/// 规范 / 病史：**生成期与走查期同源**——同一份命名词空间喂给两阶段，判定必须一致。
///
/// 走查期用唯一分类器（`ast::classify_compile_time_value_params`，与 sema 四处同一
/// 实现）回答「N 是不是编译期值参数」；生成期读同一份事实派生 `generic_params`。
/// 两者对同一入参必须同判：并集 ⇒ 都不是 q=0；单元素集 ⇒ 都是 q=0。
#[test]
fn test_generation_and_walk_agree_on_same_namespace() {
    // Arrange
    let predicate_unit = parse_module("Eq: (a: Int, b: Int) -> Type = { a <= b }\n");
    let using_unit = parse_module("f: (N: Int, n: Eq(n, N)) -> Int = { n }\n");
    let union = TypeLevelNamespace::scan([&predicate_unit, &using_unit]);
    let this_unit_only = TypeLevelNamespace::scan_one(&using_unit);
    let ann = using_unit
        .items
        .iter()
        .find_map(|stmt| match &stmt.kind {
            ast::StmtKind::Assign {
                type_annotation: Some(ty),
                ..
            } => Some(ty.clone()),
            _ => None,
        })
        .expect("夹具里 f 应有类型注解");
    let flat = ann.fn_params_flat();

    // Act — 走查期：唯一分类器；生成期：同一命名空间下的 generic_params
    let walk_with_union = classify_with(&flat, &ann, &union);
    let gen_with_union = f_generic_params(&using_unit, union);
    let walk_single = classify_with(&flat, &ann, &this_unit_only);
    let gen_single = f_generic_params(&using_unit, this_unit_only);

    // Assert — 两阶段对同一入参同判
    assert!(
        !walk_with_union.contains("N") && gen_with_union.is_none(),
        "并集：走查期 N 不是 q=0、生成期 generic_params 为空；实际 走查={walk_with_union:?} 生成={gen_with_union:?}"
    );
    assert!(
        walk_single.contains("N") && gen_single == Some(vec!["N".to_string()]),
        "单元素集：走查期 N 是 q=0、生成期 generic_params=[\"N\"]；实际 走查={walk_single:?} 生成={gen_single:?}"
    );
}

/// 走查期的判定：唯一分类器 + 该命名空间答谓词问题。
fn classify_with(
    params: &[ast::Param],
    ann: &ast::Type,
    namespace: &TypeLevelNamespace,
) -> std::collections::HashSet<String> {
    let is_const_param_type =
        |n: &str| crate::frontend::core::sema::probe::is_const_param_type_name(n);
    let no = |_: &str| false;
    let is_predicate = |n: &str| namespace.predicates.contains(n);
    let cls = ast::ParamClassifier {
        is_const_param_type: &is_const_param_type,
        is_type_name: &no,
        is_predicate: &is_predicate,
    };
    ast::classify_compile_time_value_params(params, &[ann], &cls)
}
