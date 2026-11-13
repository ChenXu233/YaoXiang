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
use crate::frontend::core::sema::TypeEnvironment;
use crate::frontend::core::sema::proof::budget::BudgetTracker;
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
    let _evaluator = Evaluator::new(&env.types, &budget, &dep_env);

    // Assert - 应该成功创建
}

#[test]
fn test_type_evaluator_eval_simple_type() {
    // Arrange
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env.types, &budget, &dep_env);

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
    let mut evaluator = Evaluator::new(&env.types, &budget, &dep_env);
    let fn_type = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Float(64)],
        return_type: Box::new(MonoType::make_string()),
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
    let mut evaluator = Evaluator::new(&env.types, &budget, &dep_env);
    let tuple_type = MonoType::make_tuple(vec![
        MonoType::Int(32),
        MonoType::Bool,
        MonoType::make_string(),
    ]);

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
    let mut evaluator = Evaluator::new(&env.types, &budget, &dep_env);
    let list_type = MonoType::make_list(MonoType::Float(64));

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
    let mut evaluator = Evaluator::with_config(&env.types, &budget, config, &dep_env);
    // 嵌套 Fn 类型会递归求值参数和返回类型，触发深度检查
    let nested_fn = MonoType::Fn {
        params: vec![MonoType::Fn {
            params: vec![MonoType::Int(32)],
            return_type: Box::new(MonoType::Float(64)),
        }],
        return_type: Box::new(MonoType::make_string()),
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
    let mut evaluator = Evaluator::new(&env.types, &budget, &dep_env);
    let nested_type = MonoType::Fn {
        params: vec![MonoType::make_tuple(vec![
            MonoType::make_list(MonoType::Int(32)),
            MonoType::Fn {
                params: vec![MonoType::Bool],
                return_type: Box::new(MonoType::make_string()),
            },
        ])],
        return_type: Box::new(MonoType::make_list(MonoType::Float(64))),
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
    let mut evaluator = Evaluator::new(&env.types, &budget, &dep_env);

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
    let mut evaluator = Evaluator::new(&env.types, &budget, &dep_env);
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
    let mut evaluator = Evaluator::new(&env.types, &budget, &dep_env);
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
    let mut evaluator = Evaluator::new(&env.types, &budget, &dep_env);
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
    let mut evaluator = Evaluator::new(&env.types, &budget, &dep_env);
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
    let mut evaluator = Evaluator::new(&env.types, &budget, &dep_env);
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

// ---- 一元取负溢出（issue #559：裸 `-*n` 改 checked_neg，与 const_eval.rs #507 口径一致）----

#[test]
fn test_eval_unop_neg_i128_min_returns_arithmetic_error_not_panic() {
    use std::collections::HashMap;

    use crate::frontend::core::types::const_data::{ConstExpr, ConstValue, UnOp};
    use crate::frontend::core::types::eval::evaluator::EvalError;

    // Arrange — n = i128::MIN 的一元取负：裸 `-*n` 在溢出检查构建下 panic
    // （实测探针 `x = -(0 - i128::MAX - 1)` 经赋值折叠面 exit 101，2026-11-14）。
    // 修后走 checked_neg 报本面既有 ArithmeticError 通道（#507/#468 同构造）。
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);
    let expr = ConstExpr::UnOp {
        op: UnOp::Neg,
        expr: Box::new(ConstExpr::Lit(ConstValue::Int(i128::MIN))),
    };

    // Act
    let result = evaluator.eval_expr(&expr, &HashMap::new());

    // Assert — Err(ArithmeticError)，不得 panic
    assert!(
        matches!(result, Err(EvalError::ArithmeticError(_))),
        "Neg(i128::MIN) 应报 ArithmeticError 而非 panic（#559），实际: {result:?}"
    );
}

#[test]
fn test_eval_unop_neg_values_in_range_unchanged() {
    use std::collections::HashMap;

    use crate::frontend::core::types::const_data::{ConstExpr, ConstValue, UnOp};

    // Arrange — 对照锚：值域内的取负修前修后行为不变（普通值 / -i128::MAX 边界 /
    // i64::MIN 形态），#559 只堵 MIN 溢出点。
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);
    let cases: [(i128, i128); 3] = [
        (-5, 5),
        (-i128::MAX, i128::MAX),
        (-9223372036854775808, 9223372036854775808),
    ];

    // Act + Assert — 三个形态逐一折叠为精确结果
    for (input, expected) in cases {
        let expr = ConstExpr::UnOp {
            op: UnOp::Neg,
            expr: Box::new(ConstExpr::Lit(ConstValue::Int(input))),
        };
        let result = evaluator.eval_expr(&expr, &HashMap::new());
        assert_eq!(
            result,
            Ok(ConstValue::Int(expected)),
            "Neg({input}) 应折叠为 Int({expected})，行为不变（#559）"
        );
    }
}

#[test]
fn test_eval_binop_sub_chain_constructs_i128_min_exactly() {
    use std::collections::HashMap;

    use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};

    // Arrange — `0 - i128::MAX - 1`：裸 Sub 两步都在值域内（-i128::MAX 精确、
    // MIN 恰好可表示），不求值溢出、精确产出 i128::MIN——这是 #559 取负臂
    // 溢出分支的可达性根基（MIN 词法不可写，只能经算术算出来）。
    let env = TypeEnvironment::new();
    let budget = BudgetTracker::new();
    let mut dep_env = DependentTypeEnv::new();
    crate::std::assert::AssertModule.register_type_families(&mut dep_env);
    let mut evaluator = Evaluator::new(&env, &budget, &dep_env);
    let inner = ConstExpr::BinOp {
        op: BinOp::Sub,
        left: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(i128::MAX))),
    };
    let expr = ConstExpr::BinOp {
        op: BinOp::Sub,
        left: Box::new(inner),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
    };

    // Act
    let result = evaluator.eval_expr(&expr, &HashMap::new());

    // Assert — 精确折叠为 i128::MIN，无 panic
    assert_eq!(
        result,
        Ok(ConstValue::Int(i128::MIN)),
        "0 - i128::MAX - 1 应精确折叠为 i128::MIN（#559 可达性根基）"
    );
}
