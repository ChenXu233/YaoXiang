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

// ===== M2: Equal 接线 =====

/// 规范（RFC-011b §Equal 规则1）：全字段可比的记录自动派生 `==`——
/// 无任何接口实例化，类型检查放行且不产生 OperatorDispatch（原生路径）
#[test]
fn test_rfc011b_equal_auto_derived_without_instantiation() {
    let source = r#"
        Point: Type = {
            x: Float,
            y: Float,
        }
        main: () -> Void = {
            a = Point(1.0, 2.0)
            b = Point(1.0, 2.0)
            c = a == b
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "auto-derived equality should pass: {:?}",
        result.diagnostics
    );
    assert!(
        result.operator_dispatches.is_empty(),
        "auto-derivation is native — no dispatch entry expected"
    );
}

/// 规范（规则2）：显式 `Equal(T, T)` + `T.equal` 方法覆盖自动派生——
/// 产生 OperatorDispatch（ir_gen 派发方法调用），`!=` 记 negate
#[test]
fn test_rfc011b_equal_explicit_overrides_with_dispatch() {
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
        main: () -> Void = {
            v1 = Vec3(1.0, 2.0, 3.0)
            v2 = Vec3(1.0, 9.9, 3.0)
            eq = v1 == v2
            ne = v1 != v2
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "explicit Equal implementation should pass: {:?}",
        result.diagnostics
    );
    let eq_dispatch = result
        .operator_dispatches
        .iter()
        .find(|d| d.method == "equal" && !d.negate)
        .expect("== dispatch entry expected");
    assert_eq!(eq_dispatch.type_name, "Vec3");
    assert!(
        result
            .operator_dispatches
            .iter()
            .any(|d| d.method == "equal" && d.negate),
        "!= should dispatch with negate"
    );
}

/// 规范（规则3/4）：含 `&mut` 线性令牌字段的类型不可比较 → E1101（指明字段）
#[test]
fn test_rfc011b_linear_token_field_rejects_equality() {
    let source = r#"
        Token: Type = {
            name: String,
            lock: &mut Int,
        }
        main: () -> Void = {
            a = Token("x", 1)
            b = Token("y", 2)
            c = a == b
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    let diag = result
        .diagnostics
        .iter()
        .find(|d| d.code == "E1101")
        .expect("linear token field should reject equality with E1101");
    assert!(
        diag.message.contains("lock"),
        "diagnostic should name the offending field: {}",
        diag.message
    );
}

/// 规范（规则3）：字段不可比的记录可手写 `Equal` 自定义比较（覆盖）
#[test]
fn test_rfc011b_custom_equal_rescues_incomparable_fields() {
    let source = r#"
        Callback: Type = {
            id: Int,
            fn_ref: (Int) -> Int,
            Equal(Callback, Callback),
        }
        Callback.equal: (self: &Callback, other: &Callback) -> Bool = {
            return self.id == other.id
        }
        main: () -> Void = {
            a = Callback(1, (x) => x)
            b = Callback(1, (x) => x)
            c = a == b
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    // 字段 fn_ref 不可比，但显式 Equal 覆盖在结构推导之前命中 → 放行
    assert!(
        result.diagnostics.is_empty(),
        "explicit Equal should rescue incomparable fields: {:?}",
        result.diagnostics
    );
}

// ===== M3: 算术接口接线 =====

/// 规范（RFC-011b §运行时行为/向后兼容）：`1 + 2.5` 混合算术 widening——
/// native 登记 `Add(Int, Float, Float)` 的快路径，结果 Float
#[test]
fn test_rfc011b_mixed_arithmetic_widens_to_float() {
    let source = r#"
        main: () -> Void = {
            a = 1 + 2.5
            b = 2.5 + 1
            c = 7 - 2.5
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "mixed arithmetic should pass: {:?}",
        result.diagnostics
    );
    assert_eq!(
        result.local_var_types.get("a"),
        Some(&crate::frontend::core::types::MonoType::Float(64)),
        "1 + 2.5 should infer Float"
    );
}

/// 规范（RFC-011b §示例）：`Point + Point` 用户重载——派发方法调用，
/// 结果类型来自登记条目的 O 位
#[test]
fn test_rfc011b_user_add_dispatch_recorded() {
    let source = r#"
        Point: Type = {
            x: Float,
            y: Float,
            Add(Point, Point, Point),
        }
        Point.add: (self: &Point, other: &Point) -> Point = {
            return Point(self.x + other.x, self.y + other.y)
        }
        main: () -> Void = {
            a = Point(1.0, 2.0)
            b = Point(3.0, 4.0)
            c = a + b
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "user add should pass: {:?}",
        result.diagnostics
    );
    let d = result
        .operator_dispatches
        .iter()
        .find(|d| d.method == "add" && !d.negate)
        .expect("add dispatch entry expected");
    assert_eq!(d.type_name, "Point");
}

/// 规范（RFC-011b §孤儿规则）：Self 位不是声明类型本身 → E1104
#[test]
fn test_rfc011b_orphan_rule_rejects_e1104() {
    let source = r#"
        Point: Type = {
            x: Float,
            y: Float,
            Add(Int, Point, Point),
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1104"),
        "Self position must be the declaring type: {:?}",
        result.diagnostics
    );
}

/// 规范：未登记的组合（Point 无 Add 实例化）→ E1002 且提示接口
#[test]
fn test_rfc011b_unregistered_add_reports_e1002() {
    let source = r#"
        Point: Type = {
            x: Float,
            y: Float,
        }
        main: () -> Void = {
            a = Point(1.0, 2.0)
            b = Point(3.0, 4.0)
            c = a + b
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    let diag = result
        .diagnostics
        .iter()
        .find(|d| d.code == "E1002")
        .expect("unregistered add should report E1002");
    assert!(
        diag.message.contains("Point"),
        "diagnostic should mention operand types: {}",
        diag.message
    );
}

// ===== M4: Index 接线 =====

/// 规范（RFC-011b §索引接口）：实现 Index 的用户容器可下标访问，
/// 结果类型来自登记条目的 Value 位
#[test]
fn test_rfc011b_user_index_typecheck() {
    let source = r#"
        Grid: Type = {
            cells: List(Float),
            Index(Grid, Int, Float),
        }
        Grid.index: (self: &Grid, key: &Int) -> Float = {
            return self.cells[key]
        }
        main: () -> Void = {
            g = Grid([7.0, 8.0, 9.0])
            v = g[1]
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "user container index should pass: {:?}",
        result.diagnostics
    );
    assert_eq!(
        result.local_var_types.get("v"),
        Some(&crate::frontend::core::types::MonoType::Float(64)),
        "g[1] should infer Float from Index entry's Value position"
    );
    let d = result
        .operator_dispatches
        .iter()
        .find(|d| d.method == "index")
        .expect("index dispatch entry expected");
    assert_eq!(d.type_name, "Grid");
}

/// 规范（RFC-011b §实例化级重载）：同一类型的同名接口多实例化按
/// 方法签名共存（Int 键 + 元组键）
#[test]
fn test_rfc011b_index_overload_coexists() {
    let source = r#"
        Grid: Type = {
            cells: List(Float),
            Index(Grid, Int, Float),
            Index(Grid, Tuple(Int, Int), Float),
        }
        Grid.index: (self: &Grid, key: &Int) -> Float = {
            return self.cells[key]
        }
        Grid.index: (self: &Grid, key: &Tuple(Int, Int)) -> Float = {
            return self.cells[key.0]
        }
        main: () -> Void = {
            g = Grid([7.0, 8.0, 9.0])
            a = g[1]
            b = g[0, 2]
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "two Index instantiations should coexist: {:?}",
        result.diagnostics
    );
}

/// 规范：未实现 Index 的类型下标访问 → E1002（提示实现 Index 接口）
#[test]
fn test_rfc011b_unregistered_index_reports_e1002() {
    let source = r#"
        Point: Type = {
            x: Float,
            y: Float,
        }
        main: () -> Void = {
            p = Point(1.0, 2.0)
            v = p[0]
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1002"),
        "unregistered index should report E1002: {:?}",
        result.diagnostics
    );
}

// ===== RFC-010 变体构造（阶段 2 第一块）=====

/// 规范（RFC-010 判定规则）：字段全为函数且返回自身 → 判定为和类型；
/// `Result(Int, String)` 值位置实例化为 Generic 形态（类型自足）
#[test]
fn test_rfc010_sum_type_detection_and_instantiation() {
    let source = r#"
        Result: (T: Type, E: Type) -> Type = {
            ok: (T) -> Result(T, E),
            err: (E) -> Result(T, E),
        }
        main: () -> Void = {
            x = Result(Int, String)
            return
        }
    "#;
    let (result, mut checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "sum type declaration + instantiation should pass: {:?}",
        result.diagnostics
    );
    let variants = checker
        .env()
        .sum_types
        .get("Result")
        .expect("Result should be detected as sum type");
    assert_eq!(variants.len(), 2);
    assert_eq!(variants[0].name, "ok");
    assert_eq!(variants[1].name, "err");
}

/// 规范（判定规则·反例）：字段返回非自身 → 不判定为和类型（普通记录）
#[test]
fn test_rfc010_not_sum_type_when_return_differs() {
    let source = r#"
        Wrapper: Type = {
            make: () -> Int,
        }
        main: () -> Void = {
            return
        }
    "#;
    let (_result, mut checker) = check_source_with_checker(source);
    assert!(
        !checker.env().sum_types.contains_key("Wrapper"),
        "returning Int is not sum type"
    );
}

/// 规范（判定规则·反例）：存在方法绑定 → 不判定（全有或全无）
#[test]
fn test_rfc010_not_sum_type_with_binding() {
    let source = r#"
        helper: () -> Int = 1
        Weird: Type = {
            make: () -> Weird,
            make = helper[0],
        }
        main: () -> Void = {
            return
        }
    "#;
    let (_result, mut checker) = check_source_with_checker(source);
    assert!(
        !checker.env().sum_types.contains_key("Weird"),
        "binding item disqualifies sum type"
    );
}

// ===== RFC-010 变体构造（W2/W3）=====

/// 规范（RFC-010 调用形态/推断）：类型限定调用类型自足——
/// `Result(Int, String).ok(5)` 推断为 Generic{Result,[Int,String]}，
/// 并记录 VariantCtorCall（span 键控，ir_gen 生成 CreateVariant）
#[test]
fn test_rfc010_variant_ctor_type_inferred() {
    let source = r#"
        Result: (T: Type, E: Type) -> Type = {
            ok: (T) -> Result(T, E),
            err: (E) -> Result(T, E),
        }
        main: () -> Void = {
            r = Result(Int, String).ok(5)
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "variant construction should pass: {:?}",
        result.diagnostics
    );
    assert_eq!(
        result.local_var_types.get("r"),
        Some(&crate::frontend::core::types::MonoType::Generic {
            name: "Result".to_string(),
            args: vec![
                crate::frontend::core::types::MonoType::Int(64),
                crate::frontend::core::types::MonoType::make_string(),
            ],
        }),
    );
    let d = result
        .variant_ctor_calls
        .iter()
        .find(|c| c.type_name == "Result")
        .expect("variant ctor call expected");
    assert_eq!(d.variant_index, 0, "ok is declaration-order 0");
    assert_eq!(d.payload_count, 1);
}

/// 规范（零载荷变体）：`Color.red()` 函数调用形态，构造点 payload_count = 0
#[test]
fn test_rfc010_zero_payload_variant() {
    let source = r#"
        Color: Type = {
            red: () -> Color,
            green: () -> Color,
            blue: () -> Color,
        }
        main: () -> Void = {
            c = Color.green()
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "zero-payload variant should pass: {:?}",
        result.diagnostics
    );
    let d = result
        .variant_ctor_calls
        .iter()
        .find(|c| c.type_name == "Color")
        .expect("Color ctor call expected");
    assert_eq!(d.variant_index, 1, "green is declaration-order 1");
    assert_eq!(d.payload_count, 0);
}

/// 规范（字段升格）：和类型值的变体名字段访问 → E1105
#[test]
fn test_rfc010_variant_field_access_rejected_e1105() {
    let source = r#"
        Color: Type = {
            red: () -> Color,
        }
        main: () -> Void = {
            c = Color.red()
            x = c.red
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1105"),
        "variant-as-field should report E1105: {:?}",
        result.diagnostics
    );
}

/// 规范（无一等构造器值）：`Result(Int, String).ok` 不跟调用 → E1105
#[test]
fn test_rfc010_ctor_without_call_rejected_e1105() {
    let source = r#"
        Result: (T: Type, E: Type) -> Type = {
            ok: (T) -> Result(T, E),
            err: (E) -> Result(T, E),
        }
        main: () -> Void = {
            f = Result(Int, String).ok
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1105"),
        "ctor without call should report E1105: {:?}",
        result.diagnostics
    );
}

/// 规范（相等放行）：和类型值参与 == 编译期放行（运行时按身份+变体+载荷）
#[test]
fn test_rfc010_sum_type_equality_allowed() {
    let source = r#"
        Result: (T: Type, E: Type) -> Type = {
            ok: (T) -> Result(T, E),
            err: (E) -> Result(T, E),
        }
        main: () -> Void = {
            a = Result(Int, String).ok(5)
            b = Result(Int, String).ok(5)
            e = a == b
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "sum type equality should pass typecheck: {:?}",
        result.diagnostics
    );
}

// ===== RFC-010b: match 变体解构 =====

/// 规范（RFC-010b）：变体解构类型检查——变体集按 scrutinee 消歧、
/// 载荷绑定进臂作用域、各臂类型 unify
#[test]
fn test_rfc010b_match_variant_destructuring() {
    let source = r#"
        Result: (T: Type, E: Type) -> Type = {
            ok: (T) -> Result(T, E),
            err: (E) -> Result(T, E),
        }
        main: () -> Void = {
            a = Result(Int, String).ok(21)
            v = match a {
                ok(value) => value * 2,
                err(e) => 0,
            }
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "variant destructuring should pass: {:?}",
        result.diagnostics
    );
    assert_eq!(
        result.local_var_types.get("v"),
        Some(&crate::frontend::core::types::MonoType::Int(64)),
        "payload bound as Int, arm type is Int"
    );
}

/// 规范（穷尽性）：漏 err 变体且无兜底臂 → E1030（列出缺失变体）
#[test]
fn test_rfc010b_non_exhaustive_reports_e1030() {
    let source = r#"
        Result: (T: Type, E: Type) -> Type = {
            ok: (T) -> Result(T, E),
            err: (E) -> Result(T, E),
        }
        main: () -> Void = {
            a = Result(Int, String).ok(5)
            v = match a {
                ok(v) => v,
            }
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    let d = result
        .diagnostics
        .iter()
        .find(|d| d.code == "E1030")
        .expect("missing variant should report E1030");
    assert!(
        d.message.contains("err"),
        "diagnostic should name the missing variant: {}",
        d.message
    );
}

/// 规范（不可达）：重复变体臂 → E1031
#[test]
fn test_rfc010b_duplicate_variant_reports_e1031() {
    let source = r#"
        Result: (T: Type, E: Type) -> Type = {
            ok: (T) -> Result(T, E),
            err: (E) -> Result(T, E),
        }
        main: () -> Void = {
            a = Result(Int, String).ok(5)
            v = match a {
                ok(v) => v,
                ok(w) => w,
                err(e) => 0,
            }
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1031"),
        "duplicate variant arm should report E1031: {:?}",
        result.diagnostics
    );
}

// ============================================================================
// 议题 1：泛型柯里化方法体参数对齐（期望类型下传）
// ============================================================================
// 方法绑定 `Type.method: (T: Type, E: Type) -> ((self: ...) -> R) = (self) => ...`
// 的函数体走 check_fn_stmt → check_fn_body：签名剥层 + T/E 替换为新鲜类型
// 变量后的「值级签名」按位置下传给参数绑定。lambda 参数不再从 AST 标注抄
// 悬空名字（旧 AST 补型路径已删除）。

/// result.yx 挂起形态：泛型柯里化方法体里 self 拿到 `&Result(T, E)`，
/// match 变体分发成功，零诊断。
#[test]
fn test_rfc011b_generic_curried_method_body_self_typed() {
    let source = r#"
        Result: (T: Type, E: Type) -> Type = {
            ok: (T) -> Result(T, E),
            err: (E) -> Result(T, E),
        }
        Result.is_failure: (T: Type, E: Type) -> ((self: &Result(T, E)) -> Bool) = (self) => {
            match self {
                err(_) => true,
                ok(_) => false,
            }
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "generic curried method body should typecheck: {:?}",
        result.diagnostics
    );
}

/// from_error：泛型方法体内变体构造（T/E 是新鲜类型变量时载荷替换成立）。
#[test]
fn test_rfc011b_generic_method_body_variant_ctor() {
    let source = r#"
        Result: (T: Type, E: Type) -> Type = {
            ok: (T) -> Result(T, E),
            err: (E) -> Result(T, E),
        }
        Result.from_error: (T: Type, E: Type) -> ((e: E) -> Result(T, E)) = (e) => {
            Result(T, E).err(e)
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "variant ctor with type-variable args should typecheck: {:?}",
        result.diagnostics
    );
}

/// 带注解 lambda 的 let 绑定：参数从注解的值级签名对齐（非泛型场景，
/// 泛型实例化路径由 try_instantiate_generic_type 处理，此处保底行为不变）。
#[test]
fn test_rfc011b_annotated_lambda_binding_params_aligned() {
    let source = r#"
        main: () -> Void = {
            f: (x: Int) -> Int = (x) => { return x + 1 }
            f(1)
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "annotated lambda binding should typecheck: {:?}",
        result.diagnostics
    );
}

/// 隔离 A：泛型柯里化方法体不含 match——只验值级参数下传。
#[test]
fn test_dbg_curried_body_no_match() {
    let source = r#"
        Result: (T: Type, E: Type) -> Type = {
            ok: (T) -> Result(T, E),
            err: (E) -> Result(T, E),
        }
        Result.is_failure: (T: Type, E: Type) -> ((self: &Result(T, E)) -> Bool) = (self) => {
            return true
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "no-match body should typecheck: {:?}",
        result.diagnostics
    );
}

/// 隔离 B：match 泛型和类型实例的基线（非方法体，&self 无关）。
#[test]
fn test_dbg_match_baseline() {
    let source = r#"
        Result: (T: Type, E: Type) -> Type = {
            ok: (T) -> Result(T, E),
            err: (E) -> Result(T, E),
        }
        main: () -> Void = {
            r = Result(Int, String).ok(5)
            v = match r {
                ok(x) => x,
                err(_) => 0,
            }
            return
        }
    "#;
    let (result, _checker) = check_source_with_checker(source);
    assert!(
        result.diagnostics.is_empty(),
        "match baseline should typecheck: {:?}",
        result.diagnostics
    );
}
