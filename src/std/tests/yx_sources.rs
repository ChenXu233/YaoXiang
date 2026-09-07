//! 嵌入 std 源表（yx_sources）测试
//!
//! 规范来源: RFC-036 §4（嵌入二进制的纯 YaoXiang 标准库）+ #327
//! （运行时错误默认携带栈帧与源码上下文——虚拟路径回填 SourceMap 的依据）

use super::super::yx_sources::embedded_source_by_virtual_path;

/// #327：虚拟路径（<std/test>）命中嵌入源，用户路径/native 模块/残缺形态不命中
#[test]
fn test_embedded_source_by_virtual_path_resolves_std_and_rejects_others() {
    // Arrange：嵌入表仅含 std/test.yx（STD_YX_FILES）

    // Act + Assert：嵌入虚拟路径回填源文本
    let src = embedded_source_by_virtual_path("<std/test>");
    assert!(
        !src.unwrap_or_default().is_empty(),
        "<std/test> should resolve to non-empty embedded source"
    );

    // Act + Assert：非嵌入路径（用户文件/native 模块）与形态残缺输入不命中、不 panic
    assert!(
        embedded_source_by_virtual_path("src/main.yx").is_none(),
        "user file path must not resolve"
    );
    assert!(
        embedded_source_by_virtual_path("<std/assert>").is_none(),
        "native module (std.assert) must not resolve"
    );
    assert!(
        embedded_source_by_virtual_path("<std/test").is_none(),
        "missing closing angle must not resolve"
    );
    assert!(
        embedded_source_by_virtual_path("std/test>").is_none(),
        "missing opening angle must not resolve"
    );
}
