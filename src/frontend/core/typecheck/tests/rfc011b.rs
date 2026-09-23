//! 运算符接口测试 — 基于 RFC-011b 运算符重载与接口驱动运算符（#341）
//!
//! RFC-011b: docs/src/design/rfc/accepted/011b-operator-overloading.md
//!
//! M1 覆盖（接口实现登记表地基）：
//! - 七个运算符接口的编译器侧声明：类型体内 `Add(Point, Point, Point)`
//!   经 RFC-011a 既有管线检查（E1095–E1100 免费），通过后同时产出
//!   ImplementationProof 与登记表条目（带类型实参维度）
//! - 核心默认登记（native 条目）：算术矩阵 + 基础类型 Equal
//! - `T: Add` 约束名复活（classify_generic_params 的 is_trait 扩展）

use crate::frontend::core::typecheck::operator_interfaces as oi;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;

use super::rfc011a::check_source_with_checker;

const POINT_ADD_FULL: &str = r#"
    Point: Type = {
        x: Float,
        y: Float,
        Add(Point, Point, Point),
    }
    Point.add: (self: &Point, other: &Point) -> Point = {
        return Point(self.x + other.x, self.y + other.y)
    }
"#;

/// 规范：运算符接口实例化成功 → 证明 + 登记表条目
///
/// 接口声明无需用户书写（编译器侧规格注册）；`Add(Point, Point, Point)`
/// 要求 `Point.add: (self: &Self, other: &R) -> O` 经实参替换后签名一致。
#[test]
fn test_rfc011b_operator_instantiation_generates_proof_and_registry() {
    let (result, checker) = check_source_with_checker(POINT_ADD_FULL);
    assert!(
        result.diagnostics.is_empty(),
        "complete operator implementation should pass: {:?}",
        result.diagnostics
    );
    let proof = checker
        .implementation_proofs()
        .iter()
        .find(|p| p.type_name == "Point" && p.interface_name == "Add")
        .expect("Point should have an Add proof");
    assert!(proof.methods.contains(&"add".to_string()));

    let registry = checker.interface_impl_registry();
    let entries = registry
        .get("Add")
        .expect("Add registry should exist (native + user entries)");
    let user = entries
        .iter()
        .find(|e| !e.native && e.impl_type == "Point")
        .expect("user entry for Point should be registered");
    assert_eq!(user.methods, vec!["add".to_string()]);
    // 三实参维度：登记表条目携带 (Point, Point, Point)
    assert_eq!(user.args.len(), 3);
    assert!(entries.iter().any(|e| e.native), "native entries present");

    // TypeCheckResult 镜像
    assert!(result
        .interface_impl_registry
        .get("Add")
        .is_some_and(|l| l.iter().any(|e| !e.native && e.impl_type == "Point")));
}

/// 规范：运算符方法未实现 → E1098
#[test]
fn test_rfc011b_missing_operator_method_reports_e1098() {
    let source = r#"
        Point: Type = {
            x: Float,
            y: Float,
            Add(Point, Point, Point),
        }
    "#;
    let result = check_source_with_checker(source).0;
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1098"),
        "missing add method should report E1098: {:?}",
        result.diagnostics
    );
}

/// 规范：方法签名与接口不符（返回类型错）→ E1099
#[test]
fn test_rfc011b_operator_signature_mismatch_reports_e1099() {
    let source = r#"
        Point: Type = {
            x: Float,
            y: Float,
            Add(Point, Point, Point),
        }
        Point.add: (self: &Point, other: &Point) -> Float = {
            return self.x + other.x
        }
    "#;
    let result = check_source_with_checker(source).0;
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1099"),
        "wrong return type should report E1099: {:?}",
        result.diagnostics
    );
}

/// 规范：实参数量不符（算术接口三参数）→ E1096
#[test]
fn test_rfc011b_operator_arity_mismatch_reports_e1096() {
    let source = r#"
        Point: Type = {
            x: Float,
            Add(Point, Point),
        }
    "#;
    let result = check_source_with_checker(source).0;
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1096"),
        "two-arg Add instantiation should report E1096: {:?}",
        result.diagnostics
    );
}

/// 规范：接口成员名与字段名冲突 → E1097
#[test]
fn test_rfc011b_operator_member_conflict_reports_e1097() {
    let source = r#"
        Point: Type = {
            add: Float,
            Add(Point, Point, Point),
        }
    "#;
    let result = check_source_with_checker(source).0;
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1097"),
        "field named add should conflict: {:?}",
        result.diagnostics
    );
}

/// 规范：`T: Add` 约束名复活——带运算符约束的泛型签名零诊断
///
/// 此前 `is_trait` 只查旧 trait 表，`Add` 不在其中，`T` 被误判为
/// 普通值参数（RFC-011b 动机 §1 的「悬空约束」根源）。
#[test]
fn test_rfc011b_operator_constraint_name_accepted() {
    let source = r#"
        combine: (T: Add + Multiply) -> ((a: T, b: T, c: T) -> T) = (a, b, c) => a
        main: () -> Void = {
            return
        }
    "#;
    let result = check_source_with_checker(source).0;
    assert!(
        result.diagnostics.is_empty(),
        "T: Add + Multiply constraint should classify T as a type param: {:?}",
        result.diagnostics
    );
}

/// 规范：Equal 接口显式实例化（覆盖自动派生的形态在 M2 接线）
#[test]
fn test_rfc011b_equal_instantiation_flow() {
    let source = r#"
        Vec3: Type = {
            x: Float,
            y: Float,
            z: Float,
            Equal(Vec3, Vec3),
        }
        Vec3.equal: (self: &Vec3, other: &Vec3) -> Bool = {
            return self.x == other.x
        }
    "#;
    let (result, checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "explicit Equal implementation should pass: {:?}",
        result.diagnostics
    );
    let registry = checker.interface_impl_registry();
    assert!(registry
        .get("Equal")
        .is_some_and(|l| l.iter().any(|e| !e.native && e.impl_type == "Vec3")));
}

/// 登记表查询：checker 初始化即含核心默认登记（native 条目）
#[test]
fn test_rfc011b_checker_boots_with_native_entries() {
    // native 登记经查询 API 验证混合算术矩阵与 Equal 同型条目
    let mut env = crate::frontend::core::typecheck::environment::TypeEnvironment::new();
    oi::register_interface_defs(&mut env);
    oi::register_native_entries(&mut env);
    let r = &env.interface_impl_registry;
    assert!(oi::query_prefix(r, "Add", &[mono_int(), mono_float()]).is_some());
    assert!(oi::query_exact(
        r,
        "Equal",
        &[
            crate::frontend::core::types::MonoType::Bool,
            crate::frontend::core::types::MonoType::Bool
        ]
    )
    .is_some());
}

fn mono_int() -> crate::frontend::core::types::MonoType {
    crate::frontend::core::types::MonoType::Int(64)
}
fn mono_float() -> crate::frontend::core::types::MonoType {
    crate::frontend::core::types::MonoType::Float(64)
}

/// 入口完整性：源码级跑一遍 tokenize→parse→check（parse 冒烟）
#[test]
fn test_rfc011b_parse_smoke() {
    let tokens = tokenize(POINT_ADD_FULL).expect("tokenize failed");
    let result = parse(&tokens);
    assert!(!result.has_errors, "parse failed: {:?}", result.errors);
}
