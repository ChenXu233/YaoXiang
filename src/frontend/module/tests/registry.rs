//! 模块注册表测试 — 规范来源：docs/src/reference/language-spec/modules.md §2.2/§3.1、
//! docs/src/rfc/accepted/036-module-system.md §4（嵌入纯 YaoXiang std 模块）、
//! docs/src/dev/architecture/07-equivalence-oracle.md
//!
//! 覆盖：
//! - #493：「纯 yx 导出」判据必须落在**导出粒度**（模块粒度会判错）
//! - #494（WBS 3.6.6）：「限定名指向什么」单一查询接口
//!   `ModuleRegistry::qualified_name_is` 的**面板 pin**——形态全集 × 三关注面
//!   （NativeName / FunctionCodeExport / FunctionOrMethodDef）的答案表。
//!   三关注面对同一形态答案可以相反（native 函数导出：NativeName=是 /
//!   FunctionCodeExport=否 / FunctionOrMethodDef=是），收敛后判据只此一份；
//!   判据被改错时本面板与 lower 侧消费臂 pin（`src/middle/lower/expr/tests/`）
//!   同红，证明同源接线。
//!
//! 为什么是导出粒度（#493）：std.result 是 native（is_ok/unwrap/… 工具族）与
//! 纯 yx（Result 类型 + Try 方法）的**混合面**，而嵌入模块本身也带
//! ModuleSource::Std——任何按模块来源的回答都会在这里判错一半。

use crate::frontend::module::registry::{ModuleRegistry, QualifiedNameConcern};
use crate::frontend::module::symbol::DefPath;

/// 混合面：同一个 std.result 里，yx 定义的导出答「是」、native 导出答「否」。
#[test]
fn test_is_yx_std_export_export_granular_on_mixed_surface() {
    // Arrange
    let registry = ModuleRegistry::with_std();

    // Act
    let yx_type_export = registry.is_yx_std_export("std.result.Result");
    let native_tool = registry.is_yx_std_export("std.result.is_ok");

    // Assert
    assert!(
        yx_type_export,
        "std.result.Result 由嵌入源 src/std/result.yx 定义，必须判为纯 yx 导出",
    );
    assert!(
        !native_tool,
        "std.result.is_ok 是 native 导出，不得判为纯 yx 导出（模块粒度会在此判错）",
    );
}

// === #494 面板 pin：qualified_name_is 的形态全集 × 三关注面答案表 ===

use crate::frontend::core::types::mono::MonoType;
use crate::frontend::module::{Export, ExportKind, ModuleInfo, ModuleSource};

/// with_std 之外再注册一个用户模块 `lib`（函数/常量/类型导出 + 方法绑定），
/// 覆盖面板的用户/vendor 形态与方法 def 形态。
fn registry_with_user_module() -> ModuleRegistry {
    let mut registry = ModuleRegistry::with_std();
    let mut lib = ModuleInfo::new("lib".to_string(), ModuleSource::User);
    for (name, kind) in [
        ("greet", ExportKind::Function),
        ("PI", ExportKind::Constant),
        ("Point", ExportKind::Type),
    ] {
        lib.add_export(Export {
            name: name.to_string(),
            full_path: DefPath::item("lib", name).project(),
            kind,
            signature: String::new(),
            mono_type: Some(MonoType::Int(64)),
            type_params: None,
            param_names: None,
            type_payload: None,
            definition_span: None,
            predicate_def: None,
            proof_fn_params: None,
            const_binders: Vec::new(),
            call_site_params: Vec::new(),
        });
    }
    lib.method_bindings
        .insert("Point.get_x".to_string(), MonoType::Int(64));
    registry.register(lib);
    registry
}

/// **原生名**关注面的形态答案表（原 `is_native_name` 判据，语义原样）：
/// Std 模块的已注册导出都答「是」（含常量与子模块导出），用户模块导出、
/// 方法 def、未知名、无点裸名答「否」。
#[test]
fn test_qualified_name_native_name_answers_by_form() {
    // Arrange：std 全家 + 用户模块 lib + 未知名/裸名形态
    let registry = registry_with_user_module();

    // Act + Assert：每个形态的答案逐一钉死
    for (name, expected) in [
        ("std.io.println", true),   // native 函数导出
        ("std.math.PI", true),      // native 常量导出
        ("std.io", true),           // 子模块导出（has_export 不看类别）
        ("std.json.parse", true),   // 纯 yx std 函数导出（仍是 Std 模块导出）
        ("lib.greet", false),       // 用户模块函数导出（来源非 Std）
        ("lib.PI", false),          // 用户模块常量导出
        ("lib.Point", false),       // 用户模块类型导出
        ("lib.Point.get_x", false), // 方法 def（lib.Point 不是模块）
        ("ghost.thing", false),     // 未知名
        ("println", false),         // 无点裸名
    ] {
        // Act：查询该形态在 NativeName 关注面的答案
        let actual = registry.qualified_name_is(name, QualifiedNameConcern::NativeName);
        // Assert：答案必须与面板一致
        assert_eq!(
            actual, expected,
            "NativeName 对 {name} 的回答必须为 {expected}（#494 面板）"
        );
    }
}

/// **函数代码**关注面的形态答案表（原 lower 侧 `qualified_export_is_code`
/// 判据 #479/#493，语义原样）：Function 导出且实现是 yx 源码答「是」；
/// native 导出（Std 且非 yx）、常量、类型、子模块、方法 def、未知名答「否」。
#[test]
fn test_qualified_name_function_code_export_answers_by_form() {
    // Arrange
    let registry = registry_with_user_module();

    // Act + Assert
    for (name, expected) in [
        ("std.io.println", false),  // native 函数导出（Std 且非 yx ⇒ 非代码）
        ("std.math.PI", false),     // native 常量（类别非 Function）
        ("std.io", false),          // 子模块导出（类别非 Function）
        ("std.json.parse", true),   // 纯 yx std 函数导出（Function + yx）
        ("std.list.map", true),     // 纯 yx std 函数导出（泛型，#493 修复面）
        ("lib.greet", true),        // 用户模块函数导出（非 Std ⇒ 代码）
        ("lib.PI", false),          // 用户模块常量（类别非 Function）
        ("lib.Point", false),       // 用户模块类型（类别非 Function）
        ("lib.Point.get_x", false), // 方法 def（不是导出）
        ("ghost.thing", false),     // 未知名
        ("println", false),         // 无点裸名
    ] {
        // Act：查询该形态在 FunctionCodeExport 关注面的答案
        let actual = registry.qualified_name_is(name, QualifiedNameConcern::FunctionCodeExport);
        // Assert：答案必须与面板一致
        assert_eq!(
            actual, expected,
            "FunctionCodeExport 对 {name} 的回答必须为 {expected}（#494 面板）"
        );
    }
}

/// **可调用 def**关注面的形态答案表（原 lower 侧命名空间臂判据，语义原样）：
/// 项目符号表里已 intern 的 Function/Method 绑定答「是」——不看模块表、
/// 不看实现来源（native 函数导出也答「是」）；常量/类型 def、子模块导出
/// （注册期跳过 intern）、未知名、无点裸名答「否」。
#[test]
fn test_qualified_name_function_or_method_def_answers_by_form() {
    // Arrange
    let registry = registry_with_user_module();

    // Act + Assert
    for (name, expected) in [
        ("std.io.println", true),  // 注册期 intern 为 Function def
        ("std.math.PI", false),    // intern 为 Constant def
        ("std.io", false),         // 子模块导出：intern_module 跳过，未 intern
        ("std.json.parse", true),  // 注册期 intern 为 Function def
        ("lib.greet", true),       // 注册期 intern 为 Function def
        ("lib.PI", false),         // intern 为 Constant def
        ("lib.Point", false),      // intern 为 Type def
        ("lib.Point.get_x", true), // 方法绑定 intern 为 Method def（非导出）
        ("ghost.thing", false),    // 未知名
        ("println", false),        // 无点裸名
    ] {
        // Act：查询该形态在 FunctionOrMethodDef 关注面的答案
        let actual = registry.qualified_name_is(name, QualifiedNameConcern::FunctionOrMethodDef);
        // Assert：答案必须与面板一致
        assert_eq!(
            actual, expected,
            "FunctionOrMethodDef 对 {name} 的回答必须为 {expected}（#494 面板）"
        );
    }
}
