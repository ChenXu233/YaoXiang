//! 裸名歧义的标准库短名（E1116）— issue #577，用户裁决乙案
//!
//! 条款门（三扇门之一）：issue #577 用户裁决（2026-11-28）——裸名命中 ≥2 个
//! Std 单元的同名导出 ⇒ 编译期报错，强制显式消歧；与 #477（用户定义优先 +
//! 限定名消歧）同一哲学。判据本体：
//! - 生产端：`add_native_function_types`（src/frontend/core/sema/mod.rs）按
//!   `native_names()`（与 lower 侧 `short_to_qualified_map` 同一遍历口径）枚举
//!   「≥2 个 Std 单元导出同一 Function/Constant 短名」的交集合，登记进
//!   `env.ambiguous_std_short_names`；歧义短名跳过 native_signatures /
//!   native_arity / native_param_names 三表的短名键（全限定名键照常）。
//! - 消费端：表达式推断器 Var 臂——裸名查不到条目且命中歧义表 ⇒ E1116。
//!
//! 本文件测试**直驱**生产端：`TypeChecker::new` 内部跑真实
//! `add_native_function_types`（checker.rs 注册段），不手工构造歧义表。
//!
//! 覆盖：
//! 1. 歧义短名不进三表短名键、进歧义表且单元列表有序（生产端事实）；
//! 2. 裸名引用歧义短名报 E1116（消费端行为）；
//! 3. 防误伤：use 项导入 / 本地同名定义 / 单一导出短名 / 限定名调用全部放行。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::lower::AstToIrGenerator;

/// 解析源码 → 模块（失败即 panic，夹具自身错误必须显式）。
fn parse_module(source: &str) -> crate::ast::Module {
    let tokens = tokenize(source).unwrap_or_else(|e| panic!("词法失败: {e:?}\n源码:\n{source}"));
    let parsed = parse(&tokens);
    assert!(!parsed.has_errors, "解析失败: {:?}", parsed.errors);
    parsed.module
}

/// 跑「类型检查 + IR 生成」两阶段，收集诊断码（按出现序）。
///
/// 类型检查有诊断时短路（IR 前提不成立）；两阶段口径与 native_param_names
/// 同款（check 阶段短路返回的形态是本主题的主形态——E1116 在 check 发射）。
fn diagnostics_via_pipeline(source: &str) -> Vec<String> {
    let module = parse_module(source);
    let mut checker = TypeChecker::new("ambiguous-std-probe");
    let result = checker.check_module(&module, &TypeLevelNamespace::scan_one(&module));
    let mut codes: Vec<String> = result.diagnostics.iter().map(|d| d.code.clone()).collect();
    if !codes.is_empty() {
        return codes;
    }
    let mut generator = AstToIrGenerator::new_with_type_result(
        &result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::scan_one(&module),
    );
    if let Err(diags) = generator.generate_module_ir(&module) {
        codes.extend(diags.iter().map(|d| d.code.clone()));
    }
    codes
}

/// 顶层值引用形态（Var 臂全覆盖的最小夹具）：值引用不经调用实参推断。
fn bare_reference(name: &str) -> String {
    format!("main: () -> Void = {{\n    r = {name}\n}}\n")
}

#[test]
fn test_ambiguous_short_names_registered_and_skipped_from_short_key_tables() {
    // Arrange：TypeChecker::new 直驱 add_native_function_types（真实生产端）
    let mut checker = TypeChecker::new("ambiguous-std-probe");

    // Act：读 env 三表与歧义表
    let env = checker.env();
    let concat_units = env
        .ambiguous_std_short_names
        .get("concat")
        .cloned()
        .unwrap_or_default();
    let has_bare_sig = env.native_signatures.contains_key("concat");
    let has_bare_arity = env.native_arity.contains_key("concat");
    let has_full_sig = env.native_signatures.contains_key("std.string.concat");

    // Assert：歧义短名进歧义表（单元 ≥2 且字典序）、三表短名键跳过、全名键照常
    assert!(
        concat_units.len() >= 2,
        "concat 必须登记为歧义短名（std.string 与 std.list 双导出），实际: {concat_units:?}"
    );
    let mut sorted = concat_units.clone();
    sorted.sort();
    assert_eq!(
        concat_units, sorted,
        "歧义单元列表必须字典序（E1116 文案逐字确定性）"
    );
    assert!(
        !has_bare_sig && !has_bare_arity,
        "歧义短名不得注册短名键（没有权威赢家就不该有静默赢家）"
    );
    assert!(has_full_sig, "全限定名键必须照常注册（限定调用解析依赖它）");
}

#[test]
fn test_bare_reference_to_ambiguous_short_name_reports_e1116() {
    // Arrange：裸名 concat 值引用（无本地定义、无 use 导入、无限定）
    let source = bare_reference("concat");

    // Act
    let codes = diagnostics_via_pipeline(&source);

    // Assert：E1116 发射，且不是「未知变量」（E1001/E1027）——歧义名有自己的码
    assert!(
        codes.iter().any(|c| c == "E1116"),
        "裸名引用歧义短名必须报 E1116，实际诊断: {codes:?}"
    );
    assert!(
        !codes.iter().any(|c| c == "E1001" || c == "E1027"),
        "歧义短名不得被报成未知变量（码义必须精确），实际诊断: {codes:?}"
    );
}

#[test]
fn test_use_item_import_disambiguates_bare_reference() {
    // Arrange：`use std.string.{concat}` 项导入后裸名引用——显式消歧形态
    let source = "use std.string.{concat}\nmain: () -> Void = {\n    r = concat\n}\n";

    // Act
    let codes = diagnostics_via_pipeline(source);

    // Assert：不得报 E1116（项导入已消歧；此前的 E5003 等其他诊断不属于本主题）
    assert!(
        !codes.iter().any(|c| c == "E1116"),
        "项导入消歧后不得报 E1116，实际诊断: {codes:?}"
    );
}

#[test]
fn test_local_definition_shadows_ambiguous_short_name() {
    // Arrange：本单元顶层定义同名函数（#477 用户定义优先语义）
    let source = "main: () -> String = () => {\n    concat(\"foo\", \"bar\")\n}\nconcat: (String, String) -> String = (a, b) => a + b\nmain()";

    // Act
    let codes = diagnostics_via_pipeline(source);

    // Assert：用户定义优先，不得报 E1116（#477 语义不变）
    assert!(
        !codes.iter().any(|c| c == "E1116"),
        "本地同名定义必须遮蔽歧义短名（#477 用户定义优先），实际诊断: {codes:?}"
    );
}

#[test]
fn test_single_export_short_name_resolves_without_e1116() {
    // Arrange：println 只由 std.io 导出（非歧义短名）——对照面
    let source = "main: () -> Void = () => {\n    println(\"hi\")\n}\nmain()";

    // Act
    let codes = diagnostics_via_pipeline(source);

    // Assert：单一导出不得被 E1116 误伤
    assert!(
        !codes.iter().any(|c| c == "E1116"),
        "单一导出短名（println）不得报 E1116，实际诊断: {codes:?}"
    );
}

#[test]
fn test_qualified_call_to_ambiguous_export_resolves() {
    // Arrange：限定名 string.concat(...) 调用（FieldAccess 形态不经裸名臂）
    let source = "use std.string\nmain: () -> String = () => {\n    string.concat(\"foo\", \"bar\")\n}\nmain()";

    // Act
    let codes = diagnostics_via_pipeline(source);

    // Assert：限定名调用零变化
    assert!(
        !codes.iter().any(|c| c == "E1116"),
        "限定名调用不得报 E1116，实际诊断: {codes:?}"
    );
}
