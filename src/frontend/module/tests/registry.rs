//! WBS 4.10 / #493：「纯 yx 导出」判据必须落在**导出粒度**（模块粒度会判错）
//!
//! 规范来源:
//! - docs/src/rfc/accepted/036-module-system.md §4（嵌入纯 YaoXiang std 模块：
//!   std.test / std.json / std.result 等以虚拟路径作种子模块，和 native 模块同级注册）
//! - docs/src/dev/architecture/07-equivalence-oracle.md（静默错值 / 静默不动作属 C 类）
//!
//! 为什么是导出粒度：std.result 是 native（is_ok/unwrap/… 工具族）与纯 yx
//! （Result 类型 + Try 方法）的**混合面**，而嵌入模块本身也带 ModuleSource::Std
//! ——任何按模块来源的回答都会在这里判错一半。

use crate::frontend::module::registry::ModuleRegistry;

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

/// 纯 yx 模块与 native 模块的分野：std.json.parse 是；std.io.println 不是。
#[test]
fn test_is_yx_std_export_discriminates_pure_yx_and_native_modules() {
    // Arrange
    let registry = ModuleRegistry::with_std();

    // Act
    let yx_export = registry.is_yx_std_export("std.json.parse");
    let native_export = registry.is_yx_std_export("std.io.println");
    let unknown = registry.is_yx_std_export("std.json.no_such_export");

    // Assert
    assert!(
        yx_export,
        "std.json.parse 定义在嵌入源 src/std/json.yx 里，必须判为纯 yx 导出",
    );
    assert!(
        !native_export,
        "std.io.println 是 native 导出，不得判为纯 yx 导出（#492 面不并入本判据）",
    );
    assert!(
        !unknown,
        "未注册的导出名不得被当作纯 yx 导出（判据只认注册表里标记过的名字）",
    );
}
