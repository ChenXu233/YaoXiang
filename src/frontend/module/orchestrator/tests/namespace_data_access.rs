//! namespace 数据访问测试 — #396/#397（`use lib;` 后 `lib.member` 表达式位置取值）
//!
//! 规范来源：语言参考 modules.md §3.1（顶层绑定一律可导入——整体导入经模块名
//! 访问；成员缺失报 E1043）+ RFC-029 模块语义（名字解析归 Resolver / 导出面
//! 语义）+ RFC-013 码表（E1043 module_has_no_export）。
//!
//! 覆盖:
//! - 有标注/无标注常量的 namespace 数据访问在 typecheck 全链干净（取值执行见
//!   multifile 集成测试）
//! - 访问模块未导出的成员报 E1043（模块语义 + 可用导出清单），不再借道 E1042
//! - 函数成员作一等值（`f = lib.greet`）typecheck 干净
//! - 无标注绑定进导出面（#397 收割）：前向引用与跨模块初始化式均不退化

use std::fs;

use tempfile::TempDir;

use super::super::check_project;

/// 两文件项目：lib.yx（导出面）+ main.yx（入口，内容由用例写入）
fn setup_project(
    lib_source: &str,
    main_source: &str,
) -> TempDir {
    let tmp = TempDir::new().expect("create tempdir");
    fs::write(tmp.path().join("lib.yx"), lib_source).expect("write lib.yx");
    fs::write(tmp.path().join("main.yx"), main_source).expect("write main.yx");
    tmp
}

fn all_codes(
    files: &[(std::path::PathBuf, Vec<crate::util::diagnostic::Diagnostic>)]
) -> Vec<String> {
    files
        .iter()
        .flat_map(|(_, diags)| diags.iter())
        .map(|d| d.code.clone())
        .collect()
}

#[test]
fn test_namespace_typed_member_access_is_clean() {
    // Arrange：lib 导出有标注常量与函数；main 用整体导入访问数据成员
    let tmp = setup_project(
        "typed_v: Int = 42\ngreet: () -> Int = () => { 5 }\n",
        "use lib;\n\nmain = () => {\n  print(lib.typed_v)\n  print(lib.greet())\n}\n",
    );

    // Act
    let files = check_project(
        &tmp.path().join("main.yx"),
        crate::frontend::config::CompileConfig::default(),
    )
    .expect("check project");

    // Assert：typecheck 侧不应有任何诊断（IR 降级由集成测试真跑验证）
    assert!(
        all_codes(&files).is_empty(),
        "有标注成员的 namespace 数据访问应无诊断，实际: {:?}",
        all_codes(&files)
    );
}

#[test]
fn test_namespace_missing_member_reports_e1043() {
    // Arrange：访问模块上不存在的成员
    let tmp = setup_project(
        "typed_v: Int = 42\n",
        "use lib;\n\nmain = () => {\n  print(lib.nope)\n}\n",
    );

    // Act
    let files = check_project(
        &tmp.path().join("main.yx"),
        crate::frontend::config::CompileConfig::default(),
    )
    .expect("check project");

    // Assert：模块语义报 E1043（不再报 struct 语义的 E1042）
    let codes = all_codes(&files);
    assert!(
        codes.iter().any(|c| c == "E1043"),
        "模块未导出成员应报 E1043，实际: {codes:?}"
    );
    assert!(
        !codes.iter().any(|c| c == "E1042"),
        "模块成员缺失不得借道 E1042（模块不是 struct），实际: {codes:?}"
    );
}

#[test]
fn test_std_submodule_missing_member_reports_e1043() {
    // Arrange：std 子模块别名（io）上取不存在的成员
    let tmp = setup_project(
        "typed_v: Int = 42\n",
        "use std.io;\n\nmain = () => {\n  print(io.nope)\n}\n",
    );

    // Act
    let files = check_project(
        &tmp.path().join("main.yx"),
        crate::frontend::config::CompileConfig::default(),
    )
    .expect("check project");

    // Assert：std 子模块同样是模块语义
    let codes = all_codes(&files);
    assert!(
        codes.iter().any(|c| c == "E1043"),
        "std 子模块成员缺失应报 E1043，实际: {codes:?}"
    );
}

// === #397：无标注顶层绑定入导出面 ===
