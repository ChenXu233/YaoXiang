//! `Type::collect_name_refs` 的括号透明性测试 — 与 `src/ast/mod.rs` 一一对应
//!
//! 规范来源：
//! - RFC-004 括号语义：`Paren` / `NamedParen` 声明「这是一个完整类型」，
//!   其余环节（类型检查 / 单态化 / 解释器 / **名字收集**）一律透明——递归内层即可。
//! - RFC-027 §3 返回位 binder：`NamedParen` 的 `param` 是**声明**不是引用，不进引用集。
//!
//! 缺口由来（#528 6.8.11）：本遍历末尾的 `_ => {}` 把两个括号变体一并吞掉 ⇒
//! 括号包裹的类型名不被计入引用 ⇒ 死代码可达性 / 未用导入检测误报（W1002 / W1003），
//! 多约束并集槽位成形（`type_position_names`）同样漏名。

use std::collections::HashSet;

use crate::ast::{Expr, Type};
use crate::util::plan_id::PlanId;
use crate::util::span::Span;

fn name_ty(name: &str) -> Type {
    Type::Name {
        name: name.to_string(),
        span: Span::dummy(),
    }
}

fn refs_of(ty: &Type) -> HashSet<String> {
    let mut out = HashSet::new();
    ty.collect_name_refs(&mut out);
    out
}

/// 括号包裹的单个类型名：透明 ⇒ 内层名字必须计入。
#[test]
fn test_collect_name_refs_paren_inner_name() {
    // Arrange — `(Bar)`：括号只声明完整类型，内层与无括号同义
    let ty = Type::Paren(Box::new(name_ty("Bar")));

    // Act
    let refs = refs_of(&ty);

    // Assert
    assert!(
        refs.contains("Bar"),
        "Paren 的内层类型名必须计入引用；实际: {refs:?}"
    );
}

/// 嵌套在泛型实参位的括号：`Vec((Bar))`。
#[test]
fn test_collect_name_refs_generic_with_paren_arg() {
    // Arrange — 括号出现在 Generic 的实参位
    let ty = Type::Generic {
        name: "Vec".to_string(),
        name_span: Span::dummy(),
        args: vec![Type::Paren(Box::new(name_ty("Bar")))],
    };

    // Act
    let refs = refs_of(&ty);

    // Assert
    assert!(
        refs.contains("Vec") && refs.contains("Bar"),
        "泛型名与括号内的实参类型名都必须计入；实际: {refs:?}"
    );
}

/// `NamedParen`：内层计入，**binder 名不计**（RFC-027 §3：binder 是声明）。
#[test]
fn test_collect_name_refs_named_paren_inner_but_not_binder() {
    // Arrange — `(r: Bar)`：`r` 是返回形式参数名的声明，`Bar` 是类型引用
    let ty = Type::NamedParen {
        param: "r".to_string(),
        param_span: Span::dummy(),
        inner: Box::new(name_ty("Bar")),
    };

    // Act
    let refs = refs_of(&ty);

    // Assert
    assert!(
        refs.contains("Bar"),
        "NamedParen 的内层类型名必须计入引用；实际: {refs:?}"
    );
    assert!(
        !refs.contains("r"),
        "binder 名是声明不是引用，不得计入引用集；实际: {refs:?}"
    );
}

/// `NamedParen` 内层谓词应用里的 binder **使用**仍是标识符引用——
/// 与「binder 声明不计」是两件事，防止把两者混为一谈。
#[test]
fn test_collect_name_refs_named_paren_collects_predicate_heads_and_uses() {
    // Arrange — `(r: IsPositive(r))`：谓词头与被引用的 binder 名字都是表达式叶
    let ty = Type::NamedParen {
        param: "r".to_string(),
        param_span: Span::dummy(),
        inner: Box::new(Type::Generic {
            name: "IsPositive".to_string(),
            name_span: Span::dummy(),
            args: vec![Type::ConstExpr(Box::new(Expr::Var(
                "r".to_string(),
                Span::dummy(),
            )))],
        }),
    };

    // Act
    let refs = refs_of(&ty);

    // Assert
    assert!(
        refs.contains("IsPositive") && refs.contains("r"),
        "谓词头与谓词里对 binder 的使用都必须计入（保守收集）；实际: {refs:?}"
    );
}
/// `x as Foo` 的 `target_type` 是**类型位置**：这个名字必须计入引用。
#[test]
fn test_collect_name_refs_cast_target_type() {
    // Arrange — 类型树的 ConstExpr 里出现 `x as Foo`
    let ty = Type::ConstExpr(Box::new(Expr::Cast {
        expr: Box::new(Expr::Var("x".to_string(), Span::dummy())),
        target_type: name_ty("Foo"),
        span: Span::dummy(),
    }));

    // Act
    let refs = refs_of(&ty);

    // Assert
    assert!(
        refs.contains("Foo"),
        "`as Foo` 的目标类型名必须计入引用（否则未用导入/未用类型误报）；实际: {refs:?}"
    );
    assert!(
        refs.contains("x"),
        "被转换的表达式里的标识符照常计入；实际: {refs:?}"
    );
}

/// 具名实参：`f(x: Foo)` 的**值**是引用，**键**（形参名）不是。
#[test]
fn test_collect_name_refs_named_call_arguments() {
    // Arrange — ConstExpr 里出现具名实参调用
    let ty = Type::ConstExpr(Box::new(Expr::Call {
        func: Box::new(Expr::Var("f".to_string(), Span::dummy())),
        args: vec![],
        named_args: vec![("x".to_string(), Expr::Var("Foo".to_string(), Span::dummy()))],
        span: Span::dummy(),
        plan_id: PlanId::UNPLANNED,
    }));

    // Act
    let refs = refs_of(&ty);

    // Assert
    assert!(
        refs.contains("Foo"),
        "具名实参的值里的标识符必须计入引用；实际: {refs:?}"
    );
    assert!(
        !refs.contains("x"),
        "具名实参的键是**形参名**（被调方命名空间的局部名），不是引用；实际: {refs:?}"
    );
}
