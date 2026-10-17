//! TypeEnvironment 测试 — 基于语言规范 §3 & RFC-010/011
//!
//! §3.1-§3.17: 类型系统环境管理
//! RFC-010: 统一类型语法
//! RFC-011: 泛型系统设计

use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::frontend::core::types::{MonoType, PolyType};

// Happy path 测试

/// WBS 4.9.7 / issue #464（修复轮 F3）：候选三元组（候选 / 混编 IR 名 / 声明期形参名）
/// 的**同序落表**与**等长不变式**——声明期登记与跨单元注入共用同一写入点，
/// 快照出口校验等长；错位不得静默流到别的编译单元（错位 = 调用点拿别人的名字派发）。
#[test]
fn test_method_overload_triple_stays_aligned_across_registration_and_injection() {
    // Arrange：声明期登记两个同名候选（签名不同 ⇒ 各自占一个声明序）
    let mut source = TypeEnvironment::new();
    let first = MonoType::Fn {
        params: vec![MonoType::Int(64)],
        return_type: Box::new(MonoType::Float(64)),
    };
    let second = MonoType::Fn {
        params: vec![MonoType::make_string()],
        return_type: Box::new(MonoType::Float(64)),
    };
    source.add_method_overload_with_params("Temp.to", first.clone(), vec!["x".to_string()]);
    source.add_method_overload_with_params("Temp.to", second.clone(), vec!["s".to_string()]);
    // 同签名重复登记不占新声明序（既有去重口径不得因改写而变）
    source.add_method_overload_with_params("Temp.to", first.clone(), vec!["x".to_string()]);

    // Act：快照（跨单元通路的出口）→ 注入另一个环境（通路的入口）
    let snapshot = source.method_overload_snapshot();
    let mut target = TypeEnvironment::new();
    target.inject_method_overloads(snapshot.clone());

    // Assert：逐位对齐——候选序 = 声明序、混编名 0=裸名其后 #序、形参名同位
    let (key, group) = &snapshot[0];
    assert_eq!(key, "Temp.to", "快照键 = 登记键，实际: {key}");
    assert_eq!(
        group.candidates,
        vec![first, second],
        "候选必须按声明序排列（重复登记不得占新序），实际: {:?}",
        group.candidates
    );
    assert_eq!(
        group.ir_names,
        vec!["Temp.to".to_string(), "Temp.to#1".to_string()],
        "混编名必须与候选同序（0=裸名、其后 #序），实际: {:?}",
        group.ir_names
    );
    assert_eq!(
        group.param_names,
        vec![vec!["x".to_string()], vec!["s".to_string()]],
        "声明期形参名必须与候选同序，实际: {:?}",
        group.param_names
    );
    // 注入端与快照端逐表一致（跨单元通路的往返不得改序、不得丢位）
    let injected: Vec<(String, _)> = target.method_overload_snapshot();
    assert_eq!(
        injected.len(),
        1,
        "注入后目标环境应恰有 1 个键位，实际: {:?}",
        injected.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>()
    );
    assert_eq!(
        injected[0].1,
        group.clone(),
        "注入不得改变三元组的顺序与配对"
    );
}

#[test]
fn test_environment_new_creates_empty() {
    // Arrange & Act
    let env = TypeEnvironment::new();

    // Assert
    assert!(env.vars.is_empty(), "vars should be empty");
    assert!(env.types.is_empty(), "types should be empty");
    assert!(env.imports.is_empty(), "imports should be empty");
}

#[test]
fn test_environment_new_with_module() {
    // Arrange & Act
    let env = TypeEnvironment::new_with_module("test_module".to_string());

    // Assert
    assert_eq!(env.module_name, "test_module");
}

#[test]
fn test_environment_add_var() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act
    env.add_var("x".to_string(), PolyType::mono(MonoType::Int(32)));

    // Assert
    assert!(env.vars.contains_key("x"), "should contain variable x");
}

#[test]
fn test_environment_add_type() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act - 使用简单的类型引用
    env.add_type(
        "MyType".to_string(),
        PolyType::mono(MonoType::TypeRef("MyType".to_string())),
    );

    // Assert
    assert!(
        env.types.contains_key("MyType"),
        "should contain type MyType"
    );
}

#[test]
fn test_environment_has_solver() {
    // Arrange & Act
    let _env = TypeEnvironment::new();

    // Assert - solver 应该存在
    // 具体断言取决于 TypeConstraintSolver 的实现
}

// Error path 测试

#[test]
fn test_environment_duplicate_var_allowed() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act - 添加同名变量应该覆盖
    env.add_var("x".to_string(), PolyType::mono(MonoType::Int(32)));
    env.add_var("x".to_string(), PolyType::mono(MonoType::Float(64)));

    // Assert
    let var = env.vars.get("x").unwrap();
    assert_eq!(
        *var,
        PolyType::mono(MonoType::Float(64)),
        "should overwrite with latest"
    );
}

// Boundary 测试

#[test]
fn test_environment_with_many_vars() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act - 添加大量变量
    for i in 0..1000 {
        env.add_var(format!("var_{}", i), PolyType::mono(MonoType::Int(32)));
    }

    // Assert
    assert_eq!(env.vars.len(), 1000, "should have 1000 variables");
}

#[test]
fn test_environment_with_many_types() {
    // Arrange
    let mut env = TypeEnvironment::new();

    // Act - 添加大量类型
    for i in 0..1000 {
        env.add_type(format!("Type_{}", i), PolyType::mono(MonoType::Int(32)));
    }

    // Assert
    assert_eq!(env.types.len(), 1000, "should have 1000 types");
}

#[test]
fn test_resolve_base_kind_type_vs_value() {
    use crate::frontend::core::typecheck::environment::BaseKind;
    let mut env = TypeEnvironment::new();
    // Point 是类型
    env.add_type(
        "Point".to_string(),
        PolyType::mono(MonoType::TypeRef("Point".to_string())),
    );
    // p 是变量
    env.add_var(
        "p".to_string(),
        PolyType::mono(MonoType::TypeRef("Point".to_string())),
    );

    assert_eq!(env.resolve_base_kind("Point"), BaseKind::TypeSpace);
    assert_eq!(env.resolve_base_kind("p"), BaseKind::ValueSpace);
    assert_eq!(env.resolve_base_kind("nope"), BaseKind::Unknown);
}
// ===== WBS 3.4.1：const 约束 Unproven 不得静默 =====
// 规范来源：RFC-011 §4.3 编译期验证 + RFC-027 §4/§9「Unproven 不得 silent pass」。
// environment.rs 的 Unproven 空臂（审计：全仓零登记的活体静默丢弃点）——
// const 实参不可编译期求值时，约束检查被跳过且零信号。修复后：
// 事实经 InstantiatedGeneric.unevaluable_constraints 上达，调用方发射 W1063。
// 边界：Unproven 只在**字面量实参**下可达（Layer 1 拒绝一切非 Literal 实参）；
// 两支成因 = 约束求值失败（引用未绑定变量）/ 求值结果非 Bool。

/// 夹具：带约束的泛型类型 `BadArray: (T: Type, N: Int) -> Type`，
/// 约束表达式由参数给出（`N > 0` 可判定；引用未绑定变量则 eval 失败）。
fn constrained_type_def(
    constraint: crate::frontend::core::types::const_data::ConstExpr
) -> crate::frontend::core::typecheck::environment::GenericTypeDef {
    use crate::frontend::core::typecheck::environment::GenericTypeDef;
    use crate::frontend::core::types::const_data::{ConstKind, ConstVarDef};
    use crate::frontend::core::types::var::TypeVar;

    let mut binder = ConstVarDef::new("N".to_string(), ConstKind::Int(None), 0);
    binder.constraints.push(constraint);
    GenericTypeDef {
        poly: PolyType::new_with_const(
            vec![TypeVar::new(0)],
            vec![binder],
            MonoType::TypeRef("T".to_string()),
        ),
        type_param_names: vec!["T".to_string()],
    }
}

/// `N > 0` 约束（可判定形态）。
fn evaluable_constraint() -> crate::frontend::core::types::const_data::ConstExpr {
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};
    ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("N".to_string())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    }
}

/// 字面量实参 `N = 5`（过 Layer 1 的唯一形态——非字面量被 Layer 1 拒绝）。
fn literal_arg_five() -> MonoType {
    MonoType::Literal {
        name: "5".to_string(),
        base_type: Box::new(MonoType::Int(64)),
        value: crate::frontend::core::types::const_data::ConstValue::Int(5),
    }
}

#[test]
fn test_instantiate_generic_type_unevaluable_constraint_is_surfaced() {
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr};

    // Arrange: 字面量实参 N=5，但约束 `N > M` 引用未绑定变量 M——
    // eval 失败（Unproven 可达支一：约束求值失败）
    let def = constrained_type_def(ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("N".to_string())),
        right: Box::new(ConstExpr::NamedVar("M".to_string())),
    });
    let args = vec![MonoType::Int(64), literal_arg_five()];

    // Act
    let result = TypeEnvironment::instantiate_generic_type(&def, &args);

    // Assert: 实例化放行，但不可求值约束必须作为事实上达（3.4.1：不得静默）
    let inst = result.unwrap_or_else(|e| panic!("实例化应放行（W 而非 E），实际 Err: {e}"));
    assert_eq!(
        inst.unevaluable_constraints.len(),
        1,
        "约束引用未绑定变量不可求值，必须上达一条事实，实际 {:?}",
        inst.unevaluable_constraints
    );
    assert!(
        inst.unevaluable_constraints[0].contains('N'),
        "事实应指明不可求值的 const 参数名，实际: {}",
        inst.unevaluable_constraints[0]
    );
}

#[test]
fn test_instantiate_generic_type_non_bool_constraint_is_surfaced() {
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};

    // Arrange: 字面量实参 N=5，约束 `N + 1` 求值产 Int（非 Bool）——
    // Unproven 可达支二：求值结果非布尔
    let def = constrained_type_def(ConstExpr::BinOp {
        op: BinOp::Add,
        left: Box::new(ConstExpr::NamedVar("N".to_string())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
    });
    let args = vec![MonoType::Int(64), literal_arg_five()];

    // Act
    let inst = TypeEnvironment::instantiate_generic_type(&def, &args)
        .unwrap_or_else(|e| panic!("实例化应放行，实际 Err: {e}"));

    // Assert
    assert_eq!(
        inst.unevaluable_constraints.len(),
        1,
        "约束求值结果非 Bool 属不可判定，必须上达事实，实际 {:?}",
        inst.unevaluable_constraints
    );
}

#[test]
fn test_instantiate_generic_type_literal_constraint_stays_silent() {
    // Arrange: const 实参为字面量 5（N=5 > 0 可求值且满足）
    let def = constrained_type_def(evaluable_constraint());
    let args = vec![MonoType::Int(64), literal_arg_five()];

    // Act
    let inst = TypeEnvironment::instantiate_generic_type(&def, &args)
        .unwrap_or_else(|e| panic!("字面量满足约束应放行，实际 Err: {e}"));

    // Assert: 约束可判定且满足——零事实（正常路径不产生信号）
    assert!(
        inst.unevaluable_constraints.is_empty(),
        "字面量约束可判定，不应上达事实，实际 {:?}",
        inst.unevaluable_constraints
    );
}

#[test]
fn test_instantiate_generic_type_non_literal_arg_rejected_by_layer1() {
    // Arrange: const 实参为 MonoType::Int（非字面量）——Layer 1 边界：
    // 一切非 Literal 实参报 E1002，Unproven 只在 Literal 实参下可达
    let def = constrained_type_def(evaluable_constraint());
    let args = vec![MonoType::Int(64), MonoType::Int(64)];

    // Act（#324：诊断构建需 span 上下文——生产由 checker walk 提供，
    // 单测直调须自挂，先例 checker.rs 的 push_current_span 兜底）
    let _span_guard =
        crate::util::diagnostic::push_current_span(crate::util::span::Span::default());
    let result = TypeEnvironment::instantiate_generic_type(&def, &args);

    // Assert
    assert!(
        result.is_err(),
        "非字面量 const 实参必须被 Layer 1 拒绝（E1002），实际放行: {:?}",
        result.map(|i| i.ty)
    );
}
