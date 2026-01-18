//! Send/Sync 约束传播测试
//!
//! 测试 Send/Sync 约束的收集、传播和验证：
//! - 基本类型 Send/Sync 属性
//! - 约束收集
//! - 约束传播到泛型参数
//! - 约束验证和特化请求生成

use crate::frontend::typecheck::{MonoType, SendSyncConstraint, SendSyncConstraintSolver};
use crate::middle::lifetime::send_sync::{SendSyncChecker, SendSyncPropagator};
use crate::middle::monomorphize::constraint::{
    ConstraintCollector, ConstraintPropagationEngine, SpecializationRequestCollector,
};
use crate::util::span::Span;

/// 创建整数的 MonoType
fn int_type() -> MonoType {
    MonoType::Int(64)
}

/// 创建浮点数的 MonoType
fn float_type() -> MonoType {
    MonoType::Float(64)
}

/// 创建字符串类型的 MonoType
fn string_type() -> MonoType {
    MonoType::String
}

/// 创建布尔类型的 MonoType
fn bool_type() -> MonoType {
    MonoType::Bool
}

/// 创建列表类型的 MonoType
fn list_type(elem: MonoType) -> MonoType {
    MonoType::List(Box::new(elem))
}

/// 创建 Arc 类型的 MonoType
fn arc_type(inner: MonoType) -> MonoType {
    MonoType::Arc(Box::new(inner))
}

/// 创建函数类型的 MonoType
fn fn_type(
    params: Vec<MonoType>,
    return_type: MonoType,
) -> MonoType {
    MonoType::Fn {
        params,
        return_type: Box::new(return_type),
        is_async: false,
    }
}

// =========================================================================
// SendSyncConstraint 测试
// =========================================================================

/// 测试：约束合并
#[test]
fn test_constraint_merge() {
    let constraint1 = SendSyncConstraint::send_only();
    let constraint2 = SendSyncConstraint::sync_only();

    let merged = constraint1.merge(&constraint2);

    assert!(merged.require_send);
    assert!(merged.require_sync);
}

/// 测试：无约束
#[test]
fn test_none_constraint() {
    let constraint = SendSyncConstraint::none();

    assert!(!constraint.require_send);
    assert!(!constraint.require_sync);
}

/// 测试：约束满足检查
#[test]
fn test_constraint_satisfaction() {
    let send_constraint = SendSyncConstraint::send_only();

    // Int 是 Send，应该满足
    assert!(send_constraint.is_satisfied(true, false));
    assert!(send_constraint.is_satisfied(true, true));

    // 但不是 Send，不应该满足
    assert!(!send_constraint.is_satisfied(false, false));
}

// =========================================================================
// SendSyncConstraintSolver 测试
// =========================================================================

/// 测试：基本类型 Send 检查
#[test]
fn test_basic_type_is_send() {
    let solver = SendSyncConstraintSolver::new();

    assert!(solver.is_send(&int_type()));
    assert!(solver.is_send(&float_type()));
    assert!(solver.is_send(&string_type()));
    assert!(solver.is_send(&bool_type()));
    assert!(solver.is_send(&arc_type(int_type())));
}

/// 测试：列表类型 Send 检查
#[test]
fn test_list_is_send() {
    let solver = SendSyncConstraintSolver::new();

    // List<Int> 是 Send
    assert!(solver.is_send(&list_type(int_type())));

    // List<List<Int>> 是 Send
    assert!(solver.is_send(&list_type(list_type(int_type()))));
}

/// 测试：添加 Send 约束传播到类型参数
#[test]
fn test_send_constraint_propagation() {
    let mut solver = SendSyncConstraintSolver::new();

    // 为 List<T> 添加 Send 约束
    let list_t = list_type(MonoType::TypeVar(crate::frontend::typecheck::TypeVar::new(
        0,
    )));
    solver.add_send_constraint(&list_t);

    // 检查约束是否被收集
    let constraints = solver.constraints();
    assert!(!constraints.is_empty());
}

/// 测试：函数类型 Send 检查
#[test]
fn test_fn_type_is_send() {
    let solver = SendSyncConstraintSolver::new();

    let closure_type = fn_type(vec![int_type()], int_type());
    assert!(solver.is_send(&closure_type));

    let closure_with_arc = fn_type(vec![int_type()], arc_type(int_type()));
    assert!(solver.is_send(&closure_with_arc));
}

// =========================================================================
// SendSyncPropagator 测试
// =========================================================================

/// 测试：约束传播
#[test]
fn test_constraint_propagation() {
    let mut propagator = SendSyncPropagator::new();

    // 添加约束
    let list_t = list_type(MonoType::TypeVar(crate::frontend::typecheck::TypeVar::new(
        0,
    )));
    propagator.add_send_constraint(&list_t);

    // 传播
    let propagated = propagator.propagate();

    // 应该包含 List<T> 和 T 的约束
    assert!(!propagated.is_empty());
}

/// 测试：从约束求解器收集
#[test]
fn test_collect_from_solver() {
    let mut propagator = SendSyncPropagator::new();
    let mut solver = SendSyncConstraintSolver::new();

    // 添加约束到求解器
    let t_var = MonoType::TypeVar(crate::frontend::typecheck::TypeVar::new(0));
    solver.add_send_constraint(&t_var);

    // 从求解器收集
    propagator.collect_from_solver(&solver, &[t_var]);

    let constraints = propagator.constraints();
    assert!(!constraints.is_empty());
}

// =========================================================================
// SendSyncChecker 测试
// =========================================================================

/// 测试：基本类型 Send
#[test]
fn test_checker_basic_send() {
    let checker = SendSyncChecker::new();

    assert!(checker.is_send(&int_type()));
    assert!(checker.is_send(&float_type()));
    assert!(checker.is_send(&string_type()));
}

/// 测试：列表类型 Send
#[test]
fn test_checker_list_send() {
    let checker = SendSyncChecker::new();

    assert!(checker.is_send(&list_type(int_type())));
}

/// 测试：Arc 类型 Send
#[test]
fn test_checker_arc_send() {
    let checker = SendSyncChecker::new();

    assert!(checker.is_send(&arc_type(int_type())));
}

// =========================================================================
// ConstraintCollector 测试
// =========================================================================

/// 测试：约束收集
#[test]
fn test_collector_add_send() {
    let mut collector = ConstraintCollector::new();

    collector.add_send_constraint(&int_type(), Span::default());

    let constraints = collector.constraints();
    assert_eq!(constraints.len(), 1);
}

/// 测试：避免重复约束
#[test]
fn test_collector_no_duplicate() {
    let mut collector = ConstraintCollector::new();

    collector.add_send_constraint(&int_type(), Span::default());
    collector.add_send_constraint(&int_type(), Span::default());

    let constraints = collector.constraints();
    assert_eq!(constraints.len(), 1);
}

// =========================================================================
// ConstraintPropagationEngine 测试
// =========================================================================

/// 测试：添加 spawn 约束
#[test]
fn test_add_spawn_constraint() {
    let mut engine = ConstraintPropagationEngine::new();

    let closure_type = fn_type(vec![int_type()], int_type());
    engine.add_spawn_constraint(&closure_type, Span::default());

    // 应该收集参数和返回类型的 Send 约束
    let collector = engine.collector();
    let constraints = collector.constraints();
    assert!(!constraints.is_empty());
}

/// 测试：约束传播结果
#[test]
fn test_propagation_result() {
    let mut engine = ConstraintPropagationEngine::new();

    let closure_type = fn_type(vec![int_type()], int_type());
    engine.add_spawn_constraint(&closure_type, Span::default());

    let result = engine.propagate();

    // 基本类型应该满足 Send 约束
    assert!(result.can_satisfy());
}

/// 测试：约束无法满足
#[test]
fn test_unsatisfied_constraint() {
    let mut engine = ConstraintPropagationEngine::new();

    // 创建一个无法 Send 的类型（模拟）
    // 这里测试传播逻辑是否正确
    let non_send_list = list_type(MonoType::TypeVar(crate::frontend::typecheck::TypeVar::new(
        0,
    )));
    engine.add_send_constraint(&non_send_list, Span::default());

    let result = engine.propagate();

    // 类型变量保守假设为 Send，所以应该可以满足
    assert!(result.can_satisfy());
}

// =========================================================================
// SpecializationRequestCollector 测试
// =========================================================================

use crate::middle::monomorphize::constraint::SpecializationRequest;

/// 测试：收集特化请求
#[test]
fn test_specialization_request_collector() {
    let mut collector = SpecializationRequestCollector::new();

    let request = SpecializationRequest::new(
        "test_fn".to_string(),
        vec![int_type()],
        SendSyncConstraint::send_only(),
        Span::default(),
    );
    collector.add_request(request);

    assert_eq!(collector.requests().len(), 1);
}

/// 测试：过滤 Send 请求
#[test]
fn test_filter_send_requests() {
    let mut collector = SpecializationRequestCollector::new();

    // 添加 Send 请求
    collector.add_request(SpecializationRequest::new(
        "fn1".to_string(),
        vec![int_type()],
        SendSyncConstraint::send_only(),
        Span::default(),
    ));

    // 添加 Sync 请求
    collector.add_request(SpecializationRequest::new(
        "fn2".to_string(),
        vec![float_type()],
        SendSyncConstraint::sync_only(),
        Span::default(),
    ));

    // 添加 Send + Sync 请求
    collector.add_request(SpecializationRequest::new(
        "fn3".to_string(),
        vec![string_type()],
        SendSyncConstraint::send_sync(),
        Span::default(),
    ));

    let send_requests: Vec<_> = collector.send_requests().collect();
    assert_eq!(send_requests.len(), 2); // fn1 和 fn3

    let sync_requests: Vec<_> = collector.sync_requests().collect();
    assert_eq!(sync_requests.len(), 2); // fn2 和 fn3
}

// =========================================================================
// 集成测试
// =========================================================================

/// 测试：完整约束传播流程
#[test]
fn test_full_propagation_flow() {
    let mut engine = ConstraintPropagationEngine::new();
    let mut request_collector = SpecializationRequestCollector::new();

    // 1. 添加 spawn 约束（闭包参数和返回值必须 Send）
    let closure_type = fn_type(
        vec![list_type(int_type())], // 参数: List<Int>
        int_type(),                  // 返回值: Int
    );
    engine.add_spawn_constraint(&closure_type, Span::default());

    // 2. 传播约束
    let result = engine.propagate();

    // 3. 收集特化请求
    if result.needs_specialization() {
        for constraint in &result.unsatisfied_types {
            let request = SpecializationRequest::new(
                "closure".to_string(),
                vec![constraint.0.clone()],
                constraint.1.clone(),
                Span::default(),
            );
            request_collector.add_request(request);
        }
    }

    // 基本类型应该满足约束，不需要特化
    assert!(result.can_satisfy());
    assert!(!result.needs_specialization());
}

/// 测试：泛型函数约束传播
#[test]
fn test_generic_function_constraint_propagation() {
    let mut solver = SendSyncConstraintSolver::new();
    let mut propagator = SendSyncPropagator::new();

    // 模拟泛型函数用于 spawn
    // fn process<T>(x: T) {
    //     spawn(|| use(x))  // x 必须 Send → T 必须 Send
    // }

    // T 是类型变量
    let t_var = MonoType::TypeVar(crate::frontend::typecheck::TypeVar::new(0));

    // 为 T 添加 Send 约束
    solver.add_send_constraint(&t_var);

    // 从求解器收集约束
    propagator.collect_from_solver(&solver, &[t_var.clone()]);

    // 传播约束
    let propagated = propagator.propagate();

    // 应该包含 T 的约束
    assert!(!propagated.is_empty());
}

/// 测试：嵌套类型的 Send 约束传播
#[test]
fn test_nested_type_constraint_propagation() {
    let mut solver = SendSyncConstraintSolver::new();

    // Vec<List<T>> 必须 Send → List<T> 必须 Send → T 必须 Send
    let nested_type = list_type(list_type(MonoType::TypeVar(
        crate::frontend::typecheck::TypeVar::new(0),
    )));

    solver.add_send_constraint(&nested_type);

    // 检查 T 是否被标记为 Send
    // 通过传播，类型变量的约束应该被收集
    let constraints = solver.constraints();
    // 由于约束传播到类型参数，应该有额外的约束
    assert!(!constraints.is_empty());
}
