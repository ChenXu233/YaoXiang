//! 方法定义混编 IR 名的单一权威来源（RFC-011a §3 / RFC-039 D21 / WBS 7c.3）
//!
//! 规范来源：
//! - RFC-011a §3：同名方法重载的定义序混编（首定义裸名，后续 `#N`），调用点决议按名派发
//! - RFC-039 D21：`method_def_ordinals` 归 04（P7 批 c）
//! - `docs/src/dev/architecture/04-ssa.md` §span 键控的额外机制「机制三」：
//!   定义名由 typecheck 一次性产出、IR 生成只查不改
//!
//! 判据：方法声明的混编名按**声明身份**（语句 `PlanId`）从
//! `TypeCheckResult.method_def_ir_names` 读取；同名重载即使声明顺序变化，
//! 调用目标仍落在决议选中的那个定义上——定义名与调用名同源，不靠遍历顺序巧合。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::TypeCheckResult;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{ConstValue, Instruction, ModuleIR, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::util::plan_id::PlanId;

/// 真实链路产物：typecheck 结果 + IR 模块 + 方法声明的 PlanId（按源码序）。
struct LowerProbe {
    type_result: TypeCheckResult,
    module: ModuleIR,
    module_ast: crate::ast::Module,
    method_decl_plan_ids: Vec<PlanId>,
}

/// 源码 → lex→parse→typecheck→ir_gen 真实生成序列（与语料判据同一路径）。
fn lower_method_probe(source: &str) -> LowerProbe {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<method-def-names-probe>");
    let type_result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    let method_decl_plan_ids = method_decl_plan_ids(&parsed.module, "Temp", "pick");
    let mut generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::scan_one(&parsed.module),
    );
    let module = generator
        .generate_module_ir(&parsed.module)
        .expect("夹具的 IR 生成必须成功");
    LowerProbe {
        type_result,
        module,
        module_ast: parsed.module,
        method_decl_plan_ids,
    }
}

/// 收集 `Type.method` 形态方法声明的语句 PlanId（源码序）。
fn method_decl_plan_ids(
    module: &crate::ast::Module,
    type_name: &str,
    method_name: &str,
) -> Vec<PlanId> {
    use crate::ast::{Expr, StmtKind};
    module
        .items
        .iter()
        .filter(|stmt| {
            let StmtKind::Assign { target, .. } = &stmt.kind else {
                return false;
            };
            let Expr::FieldAccess { expr, field, .. } = target.as_ref() else {
                return false;
            };
            field == method_name && matches!(expr.as_ref(), Expr::Var(name, _) if name == type_name)
        })
        .map(|stmt| stmt.plan_id)
        .collect()
}

/// IR 里全部函数定义名。
fn function_names(module: &ModuleIR) -> Vec<String> {
    module.functions.iter().map(|f| f.name.clone()).collect()
}

/// IR 里全部 `Call` 的被调名（函数体 + init 段）。
fn call_names(module: &ModuleIR) -> Vec<String> {
    let of = |instr: &Instruction| match instr {
        Instruction::Call {
            func: Operand::Const(ConstValue::String(name)),
            ..
        } => Some(name.clone()),
        _ => None,
    };
    let from_fns = module.functions.iter().flat_map(|f| f.all_instructions());
    let from_init = module.init.iter();
    from_fns.chain(from_init).filter_map(of).collect()
}

/// 声明序 Int → Float：首定义取裸名，第二定义取 `#1`，调用落在决议选中的定义上。
#[test]
fn test_method_ir_names_follow_declaration_identity_for_overloads() {
    // Arrange：Int 重载先声明、Float 后声明；入口用命名实参选中 Int
    let source = r#"
Temp: Type = { v: Int }
Temp.pick: (self: &Temp, n: Int) -> Int = (self, n) => { return n }
Temp.pick: (self: &Temp, f: Float) -> Float = (self, f) => { return f * 2.0 }
helper: (x: Int) -> Int = (x) => x
main: () -> Int = {
    t = Temp(1)
    return t.pick(n = 3)
}
main()
"#;

    // Act
    let probe = lower_method_probe(source);

    // Assert：定义身份映射 = 声明序（裸名 / #1），且按 PlanId 索引
    assert_eq!(
        probe.method_decl_plan_ids.len(),
        2,
        "夹具必须恰有两条 Temp.pick 声明，实际: {:?}",
        probe.method_decl_plan_ids
    );
    let int_name = probe
        .type_result
        .method_def_ir_names
        .get(&probe.method_decl_plan_ids[0])
        .expect("Int 重载声明必须登记混编名");
    let float_name = probe
        .type_result
        .method_def_ir_names
        .get(&probe.method_decl_plan_ids[1])
        .expect("Float 重载声明必须登记混编名");
    assert_eq!(
        (int_name.as_str(), float_name.as_str()),
        ("Temp.pick", "Temp.pick#1"),
        "首定义取裸名、第二定义取 #1（声明序），实际: {int_name:?} / {float_name:?}"
    );

    // Assert：函数定义名与权威映射逐字一致（lowering 只读消费）
    let names = function_names(&probe.module);
    assert!(
        names.contains(&"Temp.pick".to_string()) && names.contains(&"Temp.pick#1".to_string()),
        "IR 定义名必须等于 typecheck 的混编名，实际定义: {names:?}"
    );
    assert!(
        names.contains(&"helper".to_string()),
        "普通函数不受方法定义名映射影响（照旧裸名），实际定义: {names:?}"
    );

    // Assert：调用目标 = 决议选中的 Int 声明的混编名（同源，不靠遍历顺序）
    let calls = call_names(&probe.module);
    assert!(
        calls.iter().any(|n| n == int_name),
        "命名实参选中 Int 重载，调用名必须是该声明的混编名 {int_name:?}，实际调用: {calls:?}"
    );
}

/// 声明序反转（Float 先、Int 后）：调用 Int 必须落在 `#1` 上——顺序变化不改错目标。
#[test]
fn test_reordered_overload_declarations_keep_call_target() {
    // Arrange：与上一夹具同两个重载，仅声明顺序反转；入口仍用命名实参选中 Int
    let source = r#"
Temp: Type = { v: Int }
Temp.pick: (self: &Temp, f: Float) -> Float = (self, f) => { return f * 2.0 }
Temp.pick: (self: &Temp, n: Int) -> Int = (self, n) => { return n }
main: () -> Int = {
    t = Temp(1)
    return t.pick(n = 3)
}
main()
"#;

    // Act
    let probe = lower_method_probe(source);

    // Assert：PlanId 映射随声明序（Float 声明 = 裸名，Int 声明 = #1）
    let float_name = probe
        .type_result
        .method_def_ir_names
        .get(&probe.method_decl_plan_ids[0])
        .expect("Float 重载声明必须登记混编名");
    let int_name = probe
        .type_result
        .method_def_ir_names
        .get(&probe.method_decl_plan_ids[1])
        .expect("Int 重载声明必须登记混编名");
    assert_eq!(
        (float_name.as_str(), int_name.as_str()),
        ("Temp.pick", "Temp.pick#1"),
        "混编名按声明身份分配，实际: {float_name:?} / {int_name:?}"
    );

    // Assert：调用目标跟着**决议选中的声明身份**走（#1），不是按裸名猜
    let calls = call_names(&probe.module);
    assert!(
        calls.iter().any(|n| n == int_name),
        "声明序反转后，Int 调用的目标必须是 Int 声明的混编名 {int_name:?}，实际调用: {calls:?}"
    );
    assert!(
        !calls.iter().any(|n| n == float_name),
        "不得按顺序巧合落到 Float 声明的名字 {float_name:?}，实际调用: {calls:?}"
    );
}

/// 阴性对照：单定义方法登记裸名；普通函数不进入定义身份映射。
#[test]
fn test_single_method_and_plain_function_keep_bare_names() {
    // Arrange：Temp.only 单定义 + 普通函数 helper
    let source = r#"
Temp: Type = { v: Int }
Temp.only: (self: &Temp) -> Int = (self) => { return self.v }
helper: (x: Int) -> Int = (x) => x
main: () -> Int = {
    t = Temp(7)
    return t.only()
}
main()
"#;

    // Act
    let probe = lower_method_probe(source);

    // Assert：单定义方法也按声明身份登记，名字取裸名；普通函数不在映射里
    let names = function_names(&probe.module);
    assert!(
        names.contains(&"Temp.only".to_string()) && names.contains(&"helper".to_string()),
        "方法保持裸名、普通函数照旧，实际定义: {names:?}"
    );
    let mapped: Vec<&str> = probe
        .type_result
        .method_def_ir_names
        .values()
        .map(|s| s.as_str())
        .collect();
    assert_eq!(
        mapped,
        vec!["Temp.only"],
        "定义身份映射只含方法定义（裸名），不得收录普通函数 helper，实际: {:?}",
        probe.type_result.method_def_ir_names
    );
}

/// 消费端只读权威映射：映射缺失时必须响亮失败（E3005），不得按顺序回退裸名。
///
/// 这条直驱判据把「定义名只有一个来源」变成可失败的性质：旧实现（lowering
/// 自带定义序计数器）在映射清空后仍会成功生成并给出裸名/#N——本测试会红。
#[test]
fn test_missing_authoritative_method_name_fails_loudly() {
    // Arrange：真实链路产出；清空 typecheck 的定义身份映射，模拟两侧不同源
    let source = r#"
Temp: Type = { v: Int }
Temp.only: (self: &Temp) -> Int = (self) => { return self.v }
main: () -> Int = {
    t = Temp(7)
    return t.only()
}
main()
"#;
    let mut probe = lower_method_probe(source);
    probe.type_result.method_def_ir_names.clear();

    // Act：同一 TypeCheckResult 重新生成 IR
    let mut generator = AstToIrGenerator::new_with_type_result(
        &probe.type_result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::scan_one(&probe.module_ast),
    );
    let result = generator.generate_module_ir(&probe.module_ast);

    // Assert：不得回退裸名静默成功——必须报内部错误 E3005 并指回声明位置
    let diagnostics = result.expect_err("映射缺失时 IR 生成必须失败，不得回退裸名");
    assert!(
        diagnostics.iter().any(|d| d.code == "E3005"),
        "缺失定义名必须报 E3005，实际诊断: {:?}",
        diagnostics
    );
}
