//! RFC-039 D53 / WBS 8.9.1：参数位形态归一 —— 未命名带型参数的语义面。
//!
//! 产生式与解析形状见 `parser::statements::tests::types` 的同批用例
//! （`test_bare_type_params_parse_as_unnamed_typed` 等）；本文件只钉**语义**面：
//! 裸类型项真的进了类型检查（而不是退化成 HM 推断的无标注形参）。
//!
//! 条款门依据：
//! - SPEC type-system §3.5：`Param ::= Identifier ':' TypeExpr | TypeExpr`，
//!   裸标识符按类型命名空间解析，解析不到已声明类型报错；
//! - SPEC syntax §2.3：未命名参数不可被命名实参引用（没有名字可引用），
//!   只能按位置传递。
//!
//! 解析+检查一次到位的 helper 复用 `rfc011a` 的 `pub(crate)` 版本——
//! 本仓 sema 测试已有十余份同形 `check_source`，不再复制第十三份。

use super::rfc011a::check_source_with_checker;

/// 裸类型项解析不到类型 ⇒ **构造期**拒绝（E1003），且诊断落在标识符本身。
///
/// D53 之前该形态被读成「名为 Bogus 的无标注形参」（`ty = None`、HM 推断），
/// 标注对类型零约束、check 静默通过。
#[test]
fn test_bare_param_unknown_type_reports_e1003_at_identifier() {
    // Arrange
    let src = "f: (Bogus) -> Int = (x) => 1";

    // Act
    let (result, _checker) = check_source_with_checker(src);

    // Assert
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|d| d.code == "E1003")
        .unwrap_or_else(|| {
            panic!(
                "未命名形参的未知类型应报 E1003，实际: {:#?}",
                result.diagnostics
            )
        });
    assert!(
        diagnostic.message.contains("Bogus"),
        "诊断消息应点名未解析的类型 'Bogus'，实际: {}",
        diagnostic.message
    );
    let span = diagnostic
        .span
        .unwrap_or_else(|| panic!("E1003 必须带 span（构造期诊断），实际: {diagnostic:?}"));
    assert_eq!(span.start.line, 1, "诊断应指向源码第 1 行，实际: {span:?}");
    assert_eq!(
        span.start.column, 5,
        "诊断应指向标识符 'Bogus' 的起始列（1 基），实际: {span:?}"
    );
}

/// 裸类型项是**真标注**：声明的元数因此生效，少给实参报 E1010。
///
/// D53 之前 `(Int, Int)` 的两个形参 `ty = None`，注册签名把它们整段丢掉
/// ⇒ `add()` / `add(1, 2, 3)` 都被静默接受（零元函数 + HM 推断）。
#[test]
fn test_bare_params_enforce_declared_arity() {
    // Arrange
    let src = "\
        add: (Int, Int) -> Int = (a, b) => a + b\n\
        main: () -> Void = {\n\
            x = add()\n\
        }\n";

    // Act
    let (result, _checker) = check_source_with_checker(src);

    // Assert
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| { d.code == "E1010" && d.message.contains("add") && d.message.contains('2') }),
        "裸类型项声明的元数必须生效：add() 应报 E1010（期望 2 个实参），实际: {:#?}",
        result.diagnostics
    );
}

// 「未命名形参不可被命名实参引用」（SPEC syntax §2.3）的 E1014 在**调用绑定**面
// （`arg_binding::bind_args_to_params` 的 lower 侧调用点）发射，typecheck 的单测
// 面看不到——该性质由语料用例
// `tests/yaoxiang/06-compile-errors/fn-binding/named_arg_on_unnamed_param_err.yx`
// 在完整管道上锁定，此处不复制一份半管道断言。

/// 混合形态按位置调用照常通过：`(p: Point, Surface)` 的两位各占一个实参位。
#[test]
fn test_mixed_named_and_bare_params_accept_positional_call() {
    // Arrange
    let src = "\
        Point: Type = { x: Int }\n\
        Surface: Type = { w: Int }\n\
        mk: (p: Point, Surface) -> Int = (p, s) => p.x + s.w\n\
        main: () -> Void = {\n\
            v = mk(Point(1), Surface(2))\n\
        }\n";

    // Act
    let (result, _checker) = check_source_with_checker(src);

    // Assert
    assert!(
        result.diagnostics.is_empty(),
        "按位置传参的混合形参签名应零诊断，实际: {:#?}",
        result.diagnostics
    );
}
