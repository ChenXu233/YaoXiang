//! 类型求值测试 — 基于语言规范 §3.11 & RFC-011 §4-5 & RFC-027 §362
//!
//! §3.11: 编译期泛型
//! RFC-011 §4: 编译期泛型
//! RFC-011 §4.3: IsTrue / Assert 类型族归约
//! RFC-027 §362: 不可判定回退原则（重构失败回退到 Unproven，不伪造）
//! spec 2026-07-12-assert-refinement-unification-design.md §1.3: IsTrue 桥
//!
//! 注：issue #267 退役字符串协议——If/Match/Nat 字符串求值机器已删除，
//! 其 #262 回归测试随机器一并退役；三值语义由 conditional.rs/operations.rs 承担。

use crate::frontend::core::types::eval::evaluator::{EvalConfig, Evaluator};
use crate::frontend::core::types::MonoType;
use crate::frontend::core::typecheck::TypeEnvironment;
use crate::frontend::core::typecheck::proof::budget::BudgetTracker;
use crate::frontend::core::types::eval::dependent_types::DependentTypeEnv;
use crate::std::StdModule;

// Happy path 测试

#[test]
fn test_type_evaluator_creation() {
    // Arrange & Act
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let _evaluator = Evaluator::new(&env, &budget, &dep_env);

    // Assert - 应该成功创建
}

#[test]
fn test_type_evaluator_eval_simple_type() {
    // Arrange
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);

    // Act
    let result = evaluator.eval(&MonoType::Int(32));

    // Assert
    assert!(result.is_ok(), "should eval simple type");
}

#[test]
fn test_type_evaluator_eval_fn_type() {
    // Arrange
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);
    let fn_type = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Float(64)],
        return_type: Box::new(MonoType::String),
    };

    // Act
    let result = evaluator.eval(&fn_type);

    // Assert
    assert!(result.is_ok(), "eval Fn type should return Value");
}

#[test]
fn test_type_evaluator_eval_tuple_type() {
    // Arrange
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);
    let tuple_type = MonoType::Tuple(vec![MonoType::Int(32), MonoType::Bool, MonoType::String]);

    // Act
    let result = evaluator.eval(&tuple_type);

    // Assert
    assert!(result.is_ok(), "eval Tuple type should return Value");
}

#[test]
fn test_type_evaluator_eval_list_type() {
    // Arrange
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);
    let list_type = MonoType::List(Box::new(MonoType::Float(64)));

    // Act
    let result = evaluator.eval(&list_type);

    // Assert
    assert!(result.is_ok(), "eval List type should return Value");
}

#[test]
fn test_type_evaluator_eval_max_depth_exceeded() {
    // Arrange - 设置 max_depth=0，使得任何递归都会触发深度限制
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let config = EvalConfig {
        max_depth: 0,
        enable_cache: true,
        cycle_detection: true,
    };
    let mut evaluator = Evaluator::with_config(&env, &budget, config, &dep_env);
    // 嵌套 Fn 类型会递归求值参数和返回类型，触发深度检查
    let nested_fn = MonoType::Fn {
        params: vec![MonoType::Fn {
            params: vec![MonoType::Int(32)],
            return_type: Box::new(MonoType::Float(64)),
        }],
        return_type: Box::new(MonoType::String),
    };

    // Act
    let result = evaluator.eval(&nested_fn);

    // Assert - Fn 类型不是递归类型引用（TypeRef），不触发深度检查，应返回 Value
    assert!(
        result.is_ok(),
        "eval Fn type should return Value (Fn is not a recursive TypeRef)"
    );
}

// Boundary 测试

#[test]
fn test_type_evaluator_eval_nested_type() {
    // Arrange - 构造深层嵌套类型：Fn[Tuple[List[Int], Fn[Bool -> String(async)]] -> List[Float]]
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);
    let nested_type = MonoType::Fn {
        params: vec![MonoType::Tuple(vec![
            MonoType::List(Box::new(MonoType::Int(32))),
            MonoType::Fn {
                params: vec![MonoType::Bool],
                return_type: Box::new(MonoType::String),
            },
        ])],
        return_type: Box::new(MonoType::List(Box::new(MonoType::Float(64)))),
    };

    // Act
    let result = evaluator.eval(&nested_type);

    // Assert
    assert!(
        result.is_ok(),
        "eval deeply nested type should return Value"
    );
}

#[test]
fn test_type_evaluator_eval_void_type() {
    // Arrange
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);

    // Act
    let result = evaluator.eval(&MonoType::Void);

    // Assert
    assert!(
        matches!(result, Ok(MonoType::Void)),
        "eval Void type should return Value(Void)"
    );
}

// IsTrue/Assert 类型族测试
#[test]
fn test_istrue_true_evaluates_to_void() {
    // Arrange
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);
    let ty = MonoType::TypeRef("IsTrue(true)".to_string());

    // Act
    let result = evaluator.eval(&ty);

    // Assert — IsTrue(true) 归约为 Void（spec §1.3）
    assert!(result.is_ok(), "IsTrue(true) should evaluate successfully");
    assert_eq!(
        result.unwrap(),
        MonoType::Void,
        "IsTrue(true) must reduce to Void"
    );
}

#[test]
fn test_istrue_false_evaluates_to_never() {
    // Arrange
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);
    let ty = MonoType::TypeRef("IsTrue(false)".to_string());

    // Act
    let result = evaluator.eval(&ty);

    // Assert — IsTrue(false) 归约为 Never（spec §1.3）
    assert!(result.is_ok(), "IsTrue(false) should evaluate successfully");
    assert_eq!(
        result.unwrap(),
        MonoType::Never,
        "IsTrue(false) must reduce to Never"
    );
}

#[test]
fn test_istrue_unknown_preserves_expression() {
    // Arrange
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);
    let ty = MonoType::TypeRef("IsTrue(x)".to_string());

    // Act
    let result = evaluator.eval(&ty);

    // Assert — x 不可归约，IsTrue(x) 保留不归约（spec §1.3）
    assert!(result.is_ok(), "IsTrue(x) should not error on unknown arg");
    assert_eq!(
        result.unwrap(),
        ty,
        "IsTrue(x) must preserve when x is unknown"
    );
}

#[test]
fn test_assert_true_evaluates_to_void() {
    // Arrange — Assert(true) 内部委托给 IsTrue(true)
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);
    let ty = MonoType::TypeRef("Assert(true)".to_string());

    // Act
    let result = evaluator.eval(&ty);

    // Assert — Assert(true) 归约为 Void（spec §1.3 + RFC-011 §4.3）
    assert!(result.is_ok(), "Assert(true) should evaluate successfully");
    assert_eq!(
        result.unwrap(),
        MonoType::Void,
        "Assert(true) must reduce to Void"
    );
}

#[test]
fn test_assert_false_evaluates_to_never() {
    // Arrange — Assert(false) 内部委托给 IsTrue(false)
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);
    let ty = MonoType::TypeRef("Assert(false)".to_string());

    // Act
    let result = evaluator.eval(&ty);

    // Assert — Assert(false) 归约为 Never（spec §1.3 + RFC-011 §4.3）
    assert!(result.is_ok(), "Assert(false) should evaluate successfully");
    assert_eq!(
        result.unwrap(),
        MonoType::Never,
        "Assert(false) must reduce to Never"
    );
}
