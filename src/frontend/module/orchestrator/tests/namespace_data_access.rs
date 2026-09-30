//! namespace 数据访问测试 — #396（`use lib;` 后 `lib.member` 表达式位置取值）
//!
//! 覆盖:
//! - 有标注常量的 namespace 数据访问在 typecheck 全链干净（IR 降级见集成测试）
//! - 访问模块未导出的成员报 E1043（模块语义 + 可用导出清单），不再借道 E1042
//! - 函数成员作一等值（`f = lib.greet`）typecheck 干净

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
    let files = check_project(&tmp.path().join("main.yx")).expect("check project");
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
    let files = check_project(&tmp.path().join("main.yx")).expect("check project");
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
fn test_namespace_function_as_value_is_clean() {
    // Arrange：`lib.greet` 不带调用，作一等值绑定
    let tmp = setup_project(
        "greet: () -> Int = () => { 5 }\n",
        "use lib;\n\nmain = () => {\n  f = lib.greet\n  print(f())\n}\n",
    );
    // Act
    let files = check_project(&tmp.path().join("main.yx")).expect("check project");
    // Assert
    assert!(
        all_codes(&files).is_empty(),
        "函数成员作一等值应无诊断，实际: {:?}",
        all_codes(&files)
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
    let files = check_project(&tmp.path().join("main.yx")).expect("check project");
    // Assert：std 子模块同样是模块语义
    let codes = all_codes(&files);
    assert!(
        codes.iter().any(|c| c == "E1043"),
        "std 子模块成员缺失应报 E1043，实际: {codes:?}"
    );
}

// === #397：无标注顶层绑定入导出面 ===

#[test]
fn test_namespace_unannotated_member_access_is_clean() {
    // Arrange：lib 导出无标注常量（无标注 = 借推断进导出面）
    let tmp = setup_project(
        "untyped_v = 7\n",
        "use lib;\n\nmain = () => {\n  print(lib.untyped_v)\n}\n",
    );
    // Act
    let files = check_project(&tmp.path().join("main.yx")).expect("check project");
    // Assert：typecheck 全链无诊断（取值执行由集成测试真跑验证）
    assert!(
        all_codes(&files).is_empty(),
        "无标注成员的 namespace 数据访问应无诊断，实际: {:?}",
        all_codes(&files)
    );
}

#[test]
fn test_unannotated_forward_reference_still_exported() {
    // Arrange：无标注绑定引用**后置**绑定（前向引用——收割按 pass3 同款
    // 占位机制解析，不因声明序退化）
    let tmp = setup_project(
        "b = a + 1\na: Int = 2\n",
        "use lib;\n\nmain = () => {\n  print(lib.b)\n}\n",
    );
    // Act
    let files = check_project(&tmp.path().join("main.yx")).expect("check project");
    // Assert
    assert!(
        all_codes(&files).is_empty(),
        "前向引用的无标注绑定应正常导出，实际: {:?}",
        all_codes(&files)
    );
}

#[test]
fn test_unannotated_cross_module_initializer_exported() {
    // Arrange：无标注绑定初始化式引用**其他模块**的名字。收割前先在
    // body_checker 作用域加工 use（块级 use 同一入口），推断可用
    let tmp = setup_project(
        "use other;\n\nv = other.greet()\n",
        "use lib;\n\nmain = () => {\n  print(lib.v)\n}\n",
    );
    fs::write(
        tmp.path().join("other.yx"),
        "greet: () -> Int = () => { 5 }\n",
    )
    .expect("write other.yx");
    // Act
    let files = check_project(&tmp.path().join("main.yx")).expect("check project");
    // Assert：lib 自身与导入方全链无诊断（执行级验证见 multifile 集成测试）
    assert!(
        all_codes(&files).is_empty(),
        "跨模块初始化式的无标注绑定应正常导出，实际: {:?}",
        all_codes(&files)
    );
}
