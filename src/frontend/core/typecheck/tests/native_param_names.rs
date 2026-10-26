//! 手注 FFI 的形参名表同源 + 实参绑定判据单一源 — WBS 3.17.1 / issue #456
//!
//! 规范来源：
//! - 用户 2026-10-14 裁决（实参绑定规则，主流约定，禁止静默择一）：① 位置实参占前导
//!   槽位；② 命名实参按形参名入槽（未知名 → E1014）；③ 位置与命名争同槽 → E1111；
//!   ④ 命名之后不得再出现位置实参 → E1111；⑤ 同一形参重复命名 → E1015；
//!   ⑥ 必需形参必须全部有实参 → E1010。判据唯一实现 = `inference::arg_binding`。
//! - 3.16.1 / t14：形参名与必需下限只能从 `signature::export_signature_param_info`
//!   这一唯一出口取（同一签名串、同一解析入口）。
//!
//! 覆盖：
//! 1. **手注 FFI**（`TypeEnvironment::add_native_signature`）与 stdlib 导出同源：
//!    注册时同灌形参名表（修复前该表为空 ⇒ 命名实参被静默丢弃、零诊断）；
//! 2. 四形态各一判据，且**与普通用户函数同码**（同一判据、同一码）；
//! 3. 可选位（`?name`）与必需下限同出一源：省略可选位的命名调用放行、缺必需位报 E1010。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::environment::ExportParamInfo;
use crate::frontend::core::typecheck::{MonoType, PolyType};
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::core::ir_gen::AstToIrGenerator;

/// 宿主嵌入形态同 `Native`：命名空间根是 **Struct 变量**（FieldAccess 可解析），
/// 成员是两个经 `add_native_signature`（**签名串**入参）注册的 native 函数。
const HOST_NATIVE_ADD_SIG: &str = "(a: Int, b: Int) -> Int";
const HOST_NATIVE_FLAG_SIG: &str = "(cond: Bool, ?msg: String) -> Void";

/// 造一个"已手注注册两个宿主 native"的类型检查器。
fn host_checker() -> TypeChecker {
    let mut checker = TypeChecker::new("host-probe");
    let add_ty = MonoType::Fn {
        params: vec![MonoType::Int(64), MonoType::Int(64)],
        return_type: Box::new(MonoType::Int(64)),
    };
    let flag_ty = MonoType::Fn {
        params: vec![MonoType::Bool, MonoType::make_string()],
        return_type: Box::new(MonoType::Void),
    };
    checker.env().add_var(
        "host".to_string(),
        PolyType::mono(MonoType::Struct(crate::frontend::core::types::StructType {
            name: "host".to_string(),
            fields: vec![("add".to_string(), add_ty), ("flag".to_string(), flag_ty)],
            methods: std::collections::HashMap::new(),
            field_mutability: vec![false, false],
            field_has_default: vec![false, false],
            interfaces: Vec::new(),
        })),
    );
    checker
        .env()
        .add_native_signature("host.add", HOST_NATIVE_ADD_SIG)
        .expect("宿主签名串必须可解析");
    checker
        .env()
        .add_native_signature("host.flag", HOST_NATIVE_FLAG_SIG)
        .expect("宿主签名串必须可解析");
    checker
}

/// 跑"类型检查 + IR 生成"两阶段，收集两阶段的诊断码（按出现序）。
///
/// 为什么两阶段都要跑：普通用户函数的命名实参未知名/争槽等判定发生在 IR 生成
/// （`arg_binding::bind_args_to_params` 的调用点之一）——只看 typecheck 会漏，
/// 跨调用面比较必须按**流水线**口径。类型检查有诊断时短路（后续阶段前提不成立）。
fn diagnostics_via_pipeline(
    mut checker: TypeChecker,
    module: &crate::frontend::core::parser::ast::Module,
) -> Vec<String> {
    let result = checker.check_module(module);
    let mut codes: Vec<String> = result.diagnostics.iter().map(|d| d.code.clone()).collect();
    if !codes.is_empty() {
        return codes;
    }
    let mut generator =
        AstToIrGenerator::new_with_type_result(&result, ModuleRegistry::with_std(), None);
    if let Err(diags) = generator.generate_module_ir(module) {
        codes.extend(diags.iter().map(|d| d.code.clone()));
    }
    codes
}

/// 解析源码 → 模块（失败即 panic，夹具自身错误必须显式）。
fn parse_module(source: &str) -> crate::frontend::core::parser::ast::Module {
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("词法失败: {e:?}\n源码:\n{source}"));
    let parsed = parse(&tokens);
    assert!(!parsed.has_errors, "解析失败: {:?}", parsed.errors);
    parsed.module
}

/// 手注面：**只跑类型检查**（类型检查 + IR 两阶段的诊断口径差异见下）。
///
/// 宿主 native 只进 `TypeEnvironment` 的两张表，不进 `ModuleRegistry`——
/// IR 生成按 registry 解析命名空间成员，故宿主 native 在 IR 阶段本来就不可解析
/// （`E3006`，先于本任务的嵌入面限制，与命名实参无关）。因此手注面的四条判据
/// 必须在**类型检查阶段**观测：那正是 `add_native_signature` 补齐形参名表的作用面。
fn check_host_native(source: &str) -> Vec<String> {
    let module = parse_module(source);
    let mut checker = host_checker();
    checker
        .check_module(&module)
        .diagnostics
        .iter()
        .map(|d| d.code.clone())
        .collect()
}

/// 普通用户函数（对照面）——同一流水线口径。
fn check_source(source: &str) -> Vec<String> {
    let module = parse_module(source);
    diagnostics_via_pipeline(TypeChecker::new("plain-probe"), &module)
}

/// 用户函数面：`add: (a: Int, b: Int) -> Int`
const USER_ADD: &str = "\nadd: (a: Int, b: Int) -> Int = (a, b) => a + b\n";

/// 用户函数面的统一外壳：把调用放进函数体（与手注面同一形态）。
fn user_body(call: &str) -> String {
    format!("{USER_ADD}\nmain: () -> Void = {{\n    r = {call}\n}}\nmain()\n")
}

#[test]
fn test_add_native_signature_fills_param_names_from_single_exit() {
    // Arrange：两个手注签名（一个全必需、一个带可选位）
    let mut checker = host_checker();

    // Act
    let env = checker.env();
    let add_info = env.native_param_names.get("host.add").cloned();
    let flag_info = env.native_param_names.get("host.flag").cloned();
    let sig_has_add = env.native_signatures.contains_key("host.add");
    let sig_has_flag = env.native_signatures.contains_key("host.flag");

    // Assert：形参名表与签名表**同键同源**（形参名 + 必需下限来自唯一解析出口）
    assert_eq!(
        add_info,
        Some(ExportParamInfo {
            names: vec!["a".to_string(), "b".to_string()],
            required_min: 2,
        }),
        "手注签名必须灌装形参名表：host.add"
    );
    assert_eq!(
        flag_info,
        Some(ExportParamInfo {
            names: vec!["cond".to_string(), "msg".to_string()],
            required_min: 1,
        }),
        "可选位（?msg）不影响形参名，但必需下限必须是 1：host.flag"
    );
    assert!(
        sig_has_add && sig_has_flag,
        "两个手注签名都必须进签名表（否则后续断言的前提不成立）"
    );
    assert!(
        add_info.is_some() && flag_info.is_some(),
        "形参名表必须与签名表同键（否则消费者按同一名字查两张表只得到一半事实）"
    );
}

#[test]
fn test_host_native_unknown_param_name_reports_e1014() {
    // Arrange：手注 native 的形参名是 a / b；调用点写了不存在的 z
    let host_src = "v = host.add(a = 1, z = 2)\n";

    // Act
    let host = check_host_native(host_src);
    let user = check_source(&user_body("add(a = 1, z = 2)"));

    // Assert：手注 FFI 与用户函数同码（修复前手注面零诊断 = 命名实参被静默丢弃）
    assert!(
        host.iter().any(|c| c == "E1014"),
        "手注 FFI 未知名应报 E1014，实际诊断: {host:?}"
    );
    assert_eq!(host, user, "手注 FFI 与用户函数必须同码同判（同一判据）");
}

#[test]
fn test_host_native_missing_required_arg_reports_e1010() {
    // Arrange：两个必需形参只给一个
    let host_src = "v = host.add(a = 1)\n";

    // Act
    let host = check_host_native(host_src);
    let user = check_source(&user_body("add(a = 1)"));

    // Assert
    assert!(
        host.iter().any(|c| c == "E1010"),
        "手注 FFI 缺参应报 E1010，实际诊断: {host:?}"
    );
    assert_eq!(host, user, "手注 FFI 与用户函数必须同码同判（同一判据）");
}

#[test]
fn test_host_native_positional_named_slot_conflict_reports_e1111() {
    // Arrange：位置实参占了 a，命名实参又指定 a
    let host_src = "v = host.add(1, a = 2)\n";

    // Act
    let host = check_host_native(host_src);
    let user = check_source(&user_body("add(1, a = 2)"));

    // Assert
    assert!(
        host.iter().any(|c| c == "E1111"),
        "手注 FFI 争槽应报 E1111，实际诊断: {host:?}"
    );
    assert_eq!(host, user, "手注 FFI 与用户函数必须同码同判（同一判据）");
}

#[test]
fn test_host_native_positional_after_named_reports_e1111() {
    // Arrange：命名实参之后又写位置实参
    let host_src = "v = host.add(a = 1, 2)\n";

    // Act
    let host = check_host_native(host_src);
    let user = check_source(&user_body("add(a = 1, 2)"));

    // Assert
    assert!(
        host.iter().any(|c| c == "E1111"),
        "手注 FFI 命名后接位置应报 E1111，实际诊断: {host:?}"
    );
    assert_eq!(host, user, "手注 FFI 与用户函数必须同码同判（同一判据）");
}

#[test]
fn test_host_native_duplicate_named_reports_e1015() {
    // Arrange：同一形参被命名两次（判据⑤）
    let host_src = "v = host.add(a = 1, a = 2)\n";

    // Act
    let host = check_host_native(host_src);
    let user = check_source(&user_body("add(a = 1, a = 2)"));

    // Assert
    assert!(
        host.iter().any(|c| c == "E1015"),
        "手注 FFI 重复命名应报 E1015，实际诊断: {host:?}"
    );
    assert_eq!(host, user, "手注 FFI 与用户函数必须同码同判（同一判据）");
}

#[test]
fn test_host_native_valid_named_args_pass() {
    // Arrange：形参名正确、覆盖全部必需位
    let host_src = "v = host.add(b = 2, a = 1)\n";

    // Act
    let host = check_host_native(host_src);

    // Assert：合法命名调用不得被误报
    assert!(
        host.is_empty(),
        "手注 FFI 合法命名实参应无诊断，实际: {host:?}"
    );
}

#[test]
fn test_host_native_optional_position_may_be_omitted() {
    // Arrange：`?msg` 是可选位——只给必需位 cond
    let omitted = "v = host.flag(cond = 1 == 1)\n";
    let given = "v = host.flag(cond = 1 == 1, msg = \"boom\")\n";

    // Act
    let omitted_diags = check_host_native(omitted);
    let given_diags = check_host_native(given);

    // Assert：可选位省略放行；显式给也放行（同一张表给出的 required_min 生效）
    assert!(
        omitted_diags.is_empty(),
        "省略可选位（?msg）的命名调用应放行，实际: {omitted_diags:?}"
    );
    assert!(
        given_diags.is_empty(),
        "显式给可选位的命名调用应放行，实际: {given_diags:?}"
    );
}

#[test]
fn test_host_native_missing_required_but_optional_given_reports_e1010() {
    // Arrange：只给可选位 msg，必需的 cond 缺席
    let source = "v = host.flag(msg = \"boom\")\n";

    // Act
    let diagnostics = check_host_native(source);

    // Assert：必需下限来自同一出口（cond 必需、msg 可选）
    assert!(
        diagnostics.iter().any(|c| c == "E1010"),
        "缺必需位 cond 应报 E1010（msg 可选不能顶替必需位），实际: {diagnostics:?}"
    );
}
/// stdlib 原生导出面（第三个调用面）：`assert.assert: (cond: Bool, ?msg: String) -> Never`。
///
/// 这条路径的形参名/必需下限一直由 `typecheck/mod.rs` 经同一出口灌装（3.16.1 已修）；
/// 本组测试是「三调用面同一判据、同一码」的对照基线：手注 FFI 修好后必须与它同码。
fn stdlib_assert(call: &str) -> Vec<String> {
    check_source(&format!(
        "use std.assert\n\nmain: () -> Void = {{\n    r = {call}\n}}\nmain()\n"
    ))
}

#[test]
fn test_stdlib_native_unknown_param_name_reports_e1014() {
    // Arrange：stdlib 原生导出的形参名是 cond / msg；写了不存在的 bogus
    // Act
    let stdlib = stdlib_assert("assert.assert(bogus = true)");
    let host = check_host_native("v = host.add(a = 1, z = 2)\n");
    let user = check_source(&user_body("add(a = 1, z = 2)"));

    // Assert：三调用面同码（同一判据）
    assert!(
        stdlib.iter().any(|c| c == "E1014"),
        "stdlib 原生导出未知名应报 E1014，实际: {stdlib:?}"
    );
    assert_eq!(stdlib, host, "stdlib 原生面与手注 FFI 面必须同码");
    assert_eq!(stdlib, user, "stdlib 原生面与用户函数面必须同码");
}

#[test]
fn test_stdlib_native_positional_named_conflict_reports_e1111() {
    // Arrange：位置实参占 cond，命名实参又指定 cond
    // Act
    let stdlib = stdlib_assert("assert.assert(true, cond = false)");
    let host = check_host_native("v = host.add(1, a = 2)\n");
    let user = check_source(&user_body("add(1, a = 2)"));

    // Assert
    assert!(
        stdlib.iter().any(|c| c == "E1111"),
        "stdlib 原生导出争槽应报 E1111，实际: {stdlib:?}"
    );
    assert_eq!(stdlib, host, "stdlib 原生面与手注 FFI 面必须同码");
    assert_eq!(stdlib, user, "stdlib 原生面与用户函数面必须同码");
}

#[test]
fn test_stdlib_native_optional_position_may_be_omitted() {
    // Arrange：`?msg` 可选——只给 cond（必需位）
    // Act
    let diagnostics = stdlib_assert("assert.assert(cond = 1 == 1)");

    // Assert：可选位省略放行（与手注面同一张表、同一判据）
    assert!(
        diagnostics.is_empty(),
        "stdlib 原生导出省略可选位应放行，实际: {diagnostics:?}"
    );
}
