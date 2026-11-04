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

#[test]
fn test_compile_time_value_param_scans_every_type_subtree() {
    // Arrange — 名字只出现在**返回位**的类型实参里（形参标注里不出现）. 这是
    // WBS 6.8.3 F-1 的形态：旧的三套切法各漏一处，位次因此分叉。
    let params = vec![spec_param("a", name_ty("Int"))];
    let ret = ast::Type::Generic {
        name: "Vec".to_string(),
        name_span: Span::dummy(),
        args: vec![name_ty("a")],
    };

    // Act
    let confirmed = classify_spec(&params, &[&ret], &["Vec"]);

    // Assert — 返回位也是 Type 子树，必须扫到（漏扫即把编译期值参数当运行期值参数）
    assert!(
        confirmed.contains("a"),
        "返回位的类型实参也是「类型位置」：漏扫会把编译期值参数误判成运行期值参数，got: {confirmed:?}"
    );
}

#[test]
fn test_compile_time_value_param_respects_type_namespace() {
    // Arrange — 形参名与**已声明类型名**同形：类型位置上的那个名字解析到类型命名空间，
    // 与形参无关（SPEC §7.1：形参名与类型名同形不构成遮蔽）。
    let params = vec![spec_param("Point", name_ty("Int"))];
    let ret = name_ty("Point");

    // Act
    let confirmed = classify_spec(&params, &[&ret], &["Point"]);

    // Assert — 解析到已声明类型 ⇒ 该候选落空（退化为运行期值参数）
    assert!(
        confirmed.is_empty(),
        "类型位置上的名字解析到已声明类型时与形参无关，got: {confirmed:?}"
    );
}

#[test]
fn test_compile_time_value_param_ignores_predicate_argument_positions() {
    // Arrange — 值参数只出现在**谓词应用的实参位**（`Terminates(b)` 的 b 是值不是类型）。
    let params = vec![spec_param("b", name_ty("Int"))];
    let ret = ast::Type::Generic {
        name: "Terminates".to_string(),
        name_span: Span::dummy(),
        args: vec![name_ty("b")],
    };

    // Act
    let confirmed = classify_spec(&params, &[&ret], &["Terminates"]);

    // Assert — 谓词实参位不算类型引用
    assert!(
        confirmed.is_empty(),
        "谓词应用的实参位是值不是类型，不得把 b 判成编译期值参数，got: {confirmed:?}"
    );
}
