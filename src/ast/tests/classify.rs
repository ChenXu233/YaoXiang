//! AST 层纯判据的单元测试 — 与 `src/ast/mod.rs` 一一对应
//!
//! 规范来源:
//! - `docs/src/reference/language-spec/type-system.md` §7.1 编译期值参数
//! - `docs/src/rfc/accepted/011-generic-type-system.md` §4.1
//!
//! 覆盖: 唯一分类器 `classify_compile_time_value_params` 的真值表（形态粗筛 + 用途精筛
//! + 命名空间解析 + 单调不动点）。

use crate::ast;
use crate::util::span::Span;

/// 夹具用的注入问答：`Type`/`MetaType` 是类型命名空间；`Int`/`Bool`/`Float` 是
/// 编译期值参数候选类型；`Terminates` 是谓词。
fn spec_param(
    name: &str,
    ty: ast::Type,
) -> ast::Param {
    ast::Param {
        name: name.to_string(),
        ty: Some(ty),
        is_mut: false,
        span: Span::dummy(),
    }
}

fn name_ty(name: &str) -> ast::Type {
    ast::Type::Name {
        name: name.to_string(),
        span: Span::dummy(),
    }
}

fn classify_spec(
    params: &[ast::Param],
    type_positions: &[&ast::Type],
    type_names: &[&str],
) -> std::collections::HashSet<String> {
    let cls = ast::ParamClassifier {
        is_const_param_type: &|n| matches!(n, "Int" | "Bool" | "Float"),
        is_type_name: &|n| type_names.contains(&n),
        is_predicate: &|n| n == "Terminates",
    };
    ast::classify_compile_time_value_params(params, type_positions, &cls)
}

fn struct_with_field(
    field: &str,
    ty: ast::Type,
) -> ast::Type {
    ast::Type::Struct {
        body: vec![ast::TypeBodyItem::Field(ast::StructField {
            name: field.to_string(),
            ty,
            default: None,
            is_mut: false,
        })],
    }
}

#[test]
fn test_compile_time_value_param_spec_table() {
    // Arrange — SPEC §7.1 的四行真值表，逐行一份签名。
    // ① `add: (a: Int, b: Int) -> Int`：a/b 都只在值位置出现。
    let add_params = vec![
        spec_param("a", name_ty("Int")),
        spec_param("b", name_ty("Int")),
    ];
    let add_ret = name_ty("Int");
    // ② `Array: (T: Type, N: Int) -> Type = { data: Array(T, N) }`：N 在类型构造实参位。
    let array_params = vec![
        spec_param("T", name_ty("Type")),
        spec_param("N", name_ty("Int")),
    ];
    let array_body = struct_with_field(
        "data",
        ast::Type::Generic {
            name: "Array".to_string(),
            name_span: Span::dummy(),
            args: vec![name_ty("T"), name_ty("N")],
        },
    );
    // ③ `factorial: (N: Int) -> (k: N) -> Int`：N 作内层参数 k 的类型。
    let factorial_params = vec![spec_param("N", name_ty("Int"))];
    let factorial_inner = ast::Type::Fn {
        params: vec![spec_param("k", name_ty("N"))],
        return_type: Box::new(name_ty("Int")),
    };
    // ④ `Foo: (T: Type, N: Int) -> Type = { x: T }`：N 未在类型体引用 ⇒ 落空。
    let foo_params = vec![
        spec_param("T", name_ty("Type")),
        spec_param("N", name_ty("Int")),
    ];
    let foo_body = struct_with_field("x", name_ty("T"));

    // Act
    let add = classify_spec(&add_params, &[&add_ret], &[]);
    let array = classify_spec(&array_params, &[&array_body], &[]);
    let factorial = classify_spec(&factorial_params, &[&factorial_inner], &[]);
    let foo = classify_spec(&foo_params, &[&foo_body], &[]);

    // Assert — 逐行对照 SPEC 表：只有 ②③ 的 N 是编译期值参数
    assert!(
        add.is_empty(),
        "① `(a: Int, b: Int)` 的 a/b 只在值位置出现 ⇒ 都不是编译期值参数，got: {add:?}"
    );
    assert!(
        array.contains("N"),
        "② N 出现在类型构造实参位 `Array(T, N)` ⇒ 编译期值参数，got: {array:?}"
    );
    assert!(
        !array.contains("T"),
        "② T 标注是 `Type`（类型参数，非候选）⇒ 不进编译期值参数名单，got: {array:?}"
    );
    assert!(
        factorial.contains("N"),
        "③ N 作内层形参 k 的类型 ⇒ 编译期值参数，got: {factorial:?}"
    );
    assert!(
        foo.is_empty(),
        "④ N 未在类型体引用 ⇒ 落空，退化为运行期值参数，got: {foo:?}"
    );
}

// ─────────────────────────────────────────────────────────────
// 用途精筛的**扫描面**（`ast::name_used_as_type`）
//
// 规范：type-system.md §7.1「类型位置指声明中的全部 `Type` 子树」——形参标注、
// 返回类型、内层 `Fn`、`Assert` 谓词（`ConstExpr`）、`Type[T]`（`MetaType`）、
// 括号类型（`Paren` / `NamedParen`）都是同一棵树的子树，判定时一次递归收口。
// 以下四条钉住四个此前落 `_ => false` 的变体（#528 §2-1 的共同缺口）。
// ─────────────────────────────────────────────────────────────

/// 构造 `Assert(N > 0)` 形态的类型（SPEC §7.1 把 `Assert` 列为类型位置实例）
fn assert_gt_zero(operand: &str) -> ast::Type {
    ast::Type::Generic {
        name: "Assert".to_string(),
        name_span: Span::dummy(),
        args: vec![ast::Type::ConstExpr(Box::new(ast::Expr::BinOp {
            op: ast::BinOp::Gt,
            left: Box::new(ast::Expr::Var(operand.to_string(), Span::dummy())),
            right: Box::new(ast::Expr::Lit(ast::Literal::Int(0), Span::dummy())),
            span: Span::dummy(),
        }))],
    }
}

/// 规范：SPEC §7.1「类型位置指声明中的全部 `Type` 子树」——`Assert` 谓词的
/// 操作数是类型位置上的名字引用。
///
/// 格 2：`f: (N: Int) -> (x: Assert(N > 0)) -> Int` 的 `N` 必须判定为「被当类型用」。
/// 补 `ConstExpr` 臂之前本用例必红（该变体落 `_ => false`）。
#[test]
fn test_name_used_as_type_scans_const_expr() {
    // Arrange — `f: (N: Int) -> (x: Assert(N > 0)) -> Int`
    let ann = ast::Type::Fn {
        params: vec![spec_param("N", name_ty("Int"))],
        return_type: Box::new(ast::Type::Fn {
            params: vec![spec_param("x", assert_gt_zero("N"))],
            return_type: Box::new(name_ty("Int")),
        }),
    };

    // Act
    let referenced = ast::name_used_as_type("N", &ann, &|_| false);

    // Assert
    assert!(
        referenced,
        "`Assert(N > 0)` 里的 N 是类型位置引用 ⇒ 必须判为被当类型用"
    );
}

/// 规范：SPEC §7.1 —— `Type[T]` 的实参是类型位置；`Type` 关键字本身不是候选名。
#[test]
fn test_name_used_as_type_scans_meta_type_args() {
    // Arrange — `Type[N]` 形态的元类型实参位（`args`）
    let ty = ast::Type::MetaType {
        name_span: Span::dummy(),
        args: vec![name_ty("N")],
    };

    // Act
    let referenced = ast::name_used_as_type("N", &ty, &|_| false);

    // Assert
    assert!(
        referenced,
        "元类型实参 `Type[N]` 的 N 是类型位置引用 ⇒ 必须判为被当类型用"
    );
}

/// 规范：SPEC §7.1 —— 括号类型透明（RFC-004：`(T)` 是完整类型，内层同义）。
#[test]
fn test_name_used_as_type_scans_paren_inner() {
    // Arrange — `(N)`：括号未构成参数组 ⇒ `Type::Paren`
    let ty = ast::Type::Paren(Box::new(name_ty("N")));

    // Act
    let referenced = ast::name_used_as_type("N", &ty, &|_| false);

    // Assert
    assert!(
        referenced,
        "括号类型 `(N)` 的内层是类型位置引用 ⇒ 必须判为被当类型用"
    );
}

/// 规范：SPEC §7.1 与 RFC-027 §3 —— `(r: P(...))` 的 `r` 是**返回形式参数名的
/// 声明**（binder），不是类型引用；只有内层类型是。
#[test]
fn test_name_used_as_type_scans_named_paren_inner_but_not_binder() {
    // Arrange — `(r: N)`：binder 名 `r`，内层类型 `N`
    let ty = ast::Type::NamedParen {
        param: "r".to_string(),
        param_span: Span::dummy(),
        inner: Box::new(name_ty("N")),
    };

    // Act
    let binder_referenced = ast::name_used_as_type("r", &ty, &|_| false);
    let inner_referenced = ast::name_used_as_type("N", &ty, &|_| false);

    // Assert
    assert!(
        !binder_referenced,
        "binder 名 r 是声明而非引用 ⇒ 不得判为被当类型用"
    );
    assert!(
        inner_referenced,
        "具名括号的内层 N 是类型位置引用 ⇒ 必须判为被当类型用"
    );
}
