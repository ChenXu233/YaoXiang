//! 方法死代码判据的驱动级钉测试（WBS 4.9.6 / issue #450，用户 2026-10-16 裁决 A）
//!
//! 判据（D 类修复：删掉误报即改变诊断对外面）：
//! ① 无项目根路径下**被调用**的方法（含重载决议生效）不得报 W1005；
//! ② 同名重载**各自独立判定**——逐定义一条、不互相覆盖（旧实现 `all_defs` 按
//!    full name 键控，两个重载只剩一条，于是「2 个重载报 1 条」）；
//! ③ **未被调用**的方法仍须如实报 W1005（不得借修复放宽判据）。
//!
//! 红态（修复前，实测）：语料 `tests/yaoxiang/02-type-system/functions/
//! method_call_args.yx` 3 条 W1005（三个方法都被调用）、`method_overload.yx`
//! 1 条（两个重载都被调用，却只报 1 条）、`06-compile-errors/call-args/
//! method_overload_ambiguous_err.yx` 1 条（两个重载各占一条定义，只报 1 条）。
//! 夹具形态与语料同构：**无 manifest**（rootless / Script 语义，无包内引用池）。

use super::check_rootless_mono::{check_codes, make_rootless_project, run_check};
use super::ref_pool::assert_warnings;

/// 4.9.6 / #450 ①：被调用的方法不得报 W1005（无项目根路径）。
#[test]
fn test_called_method_is_not_reported_unused() {
    // Arrange：Temp.get 被 main 调用（同文件、无 manifest ⇒ 只按文件内定义-使用图判定）
    let src = "Temp: Type = { value: Float }\n\nTemp.get: (self: &Temp) -> Float = (self) => {\n    return self.value\n}\n\nmain: () -> Void = {\n    t = Temp(1.0)\n    x = t.get()\n}\n";
    let (_dir, entry) = make_rootless_project(&[("main.yx", src)]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：方法被调用 ⇒ 无 W1005（类型与函数也不该报）
    let codes = check_codes(&outcome, "main.yx");
    assert!(
        !codes.iter().any(|c| c == "W1005"),
        "被调用的方法不得报 W1005，实际: {codes:?}"
    );
}

/// 4.9.6 / #450 ②：同名重载各自独立判定——两个都未被调用时**各报一条**。
#[test]
fn test_same_named_overloads_report_independently() {
    // Arrange：Temp.to 两个重载都未被调用（逐定义一条，不互相覆盖）
    let src = "Temp: Type = { value: Float }\n\nTemp.to: (self: &Temp) -> Float = (self) => {\n    return self.value\n}\n\nTemp.to: (self: &Temp) -> String = (self) => {\n    return \"hot\"\n}\n\nmain: () -> Void = {\n    t = Temp(1.0)\n}\n";
    let (_dir, entry) = make_rootless_project(&[("main.yx", src)]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：两个重载各报一条 W1005（旧实现按 full name 键控只剩一条）
    let codes = check_codes(&outcome, "main.yx");
    let unused_methods = codes.iter().filter(|c| *c == "W1005").count();
    assert_eq!(
        unused_methods, 2,
        "两个同名重载都未被调用时必须各报一条 W1005，实际: {codes:?}"
    );
}

/// 4.9.6 / #450 ③：未被调用的方法仍须如实报 W1005（修复不得放宽判据）。
#[test]
fn test_uncalled_method_still_reported() {
    // Arrange：Temp.get 无任何调用点
    let src = "Temp: Type = { value: Float }\n\nTemp.get: (self: &Temp) -> Float = (self) => {\n    return self.value\n}\n\nmain: () -> Void = {\n    t = Temp(1.0)\n}\n";
    let (_dir, entry) = make_rootless_project(&[("main.yx", src)]);

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：未被调用的方法必须报 W1005
    let codes = check_codes(&outcome, "main.yx");
    assert!(
        codes.iter().any(|c| c == "W1005"),
        "未被调用的方法必须报 W1005，实际: {codes:?}"
    );
}

/// 4.9.6 / #450 ①+③ 逐候选（验收③「含未被重载决议命中」）：
/// 一组同名重载**仅一个被调用**时，未被决议命中的兄弟重载仍须报 W1005——
/// 且必须报**正确的那一个**（行号断言 = 声明序 ↔ 决议名 #k 的配对实测）。
#[test]
fn test_partially_called_overload_group_reports_the_unhit_sibling() {
    // Arrange：两个 Temp.to 重载（第 3 行 ->Float 声明序 0、第 7 行 ->String 声明序 1），
    // 调用点带类型注解 ⇒ 重载决议生效（D20 决议表记下被选中的那一个）
    let call_first = "Temp: Type = { value: Float }\n\nTemp.to: (self: &Temp) -> Float = (self) => {\n    return self.value\n}\n\nTemp.to: (self: &Temp) -> String = (self) => {\n    return \"hot\"\n}\n\nmain: () -> Void = {\n    t = Temp(1.0)\n    a: Float = t.to()\n}\n";
    let call_second = "Temp: Type = { value: Float }\n\nTemp.to: (self: &Temp) -> Float = (self) => {\n    return self.value\n}\n\nTemp.to: (self: &Temp) -> String = (self) => {\n    return \"hot\"\n}\n\nmain: () -> Void = {\n    t = Temp(1.0)\n    b: String = t.to()\n}\n";
    let (_d1, first_entry) = make_rootless_project(&[("main.yx", call_first)]);
    let (_d2, second_entry) = make_rootless_project(&[("main.yx", call_second)]);

    // Act：分别调用声明序 0 / 声明序 1 的候选
    let out_first = run_check(&first_entry).expect("check 必须返回产物");
    let out_second = run_check(&second_entry).expect("check 必须返回产物");

    // Assert：命中 ->Float（:3）时兄弟 ->String（:7）报；命中 ->String（:7）时兄弟
    // ->Float（:3）报——行号方向即配对方向，配错会在此翻红
    assert_warnings(&out_first, "main.yx", &[("W1005", 7)], &[("W1005", 3)]);
    assert_warnings(&out_second, "main.yx", &[("W1005", 3)], &[("W1005", 7)]);
}
