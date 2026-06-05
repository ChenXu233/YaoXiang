//! 类型约束测试 — 基于语言规范 §8.7 (Send/Sync 约束)
//!
//! §8.7: Send/Sync 约束语义
//!   - 基本类型默认 Send + Sync
//!   - Arc<T> 默认 Send + Sync
//!   - Rc<...> TypeRef 不是 Send/Sync
//!   - 结构体/联合/交集需要所有成员 Send/Sync
//!   - 约束通过容器类型传播

use crate::frontend::core::types::base::{
    MonoType, SendSyncConstraint, SendSyncSolver, StructType, TypeVar,
};
use std::collections::HashMap;

fn empty_struct(name: &str) -> MonoType {
    MonoType::Struct(StructType {
        name: name.to_string(),
        fields: vec![],
        methods: HashMap::new(),
        field_mutability: vec![],
        field_has_default: vec![],
        interfaces: vec![],
    })
}

fn new_var(idx: usize) -> MonoType {
    MonoType::TypeVar(TypeVar::new(idx))
}

// ===================================================================
// §8.7: SendSyncConstraint 基础
// ===================================================================

#[test]
fn test_send_sync_constraint_new() {
    let c = SendSyncConstraint::new(true, false);
    assert!(c.require_send && !c.require_sync);
}

#[test]
fn test_send_sync_constraint_constructors() {
    assert!(
        SendSyncConstraint::send_only().require_send
            && !SendSyncConstraint::send_only().require_sync
    );
    assert!(
        !SendSyncConstraint::sync_only().require_send
            && SendSyncConstraint::sync_only().require_sync
    );
    assert!(
        SendSyncConstraint::send_sync().require_send
            && SendSyncConstraint::send_sync().require_sync
    );
    assert!(!SendSyncConstraint::none().require_send && !SendSyncConstraint::none().require_sync);
}

#[test]
fn test_send_sync_constraint_merge_union() {
    let merged = SendSyncConstraint::send_only().merge(&SendSyncConstraint::sync_only());
    assert!(merged.require_send && merged.require_sync);
}

#[test]
fn test_send_sync_constraint_merge_idempotent() {
    let merged = SendSyncConstraint::send_sync().merge(&SendSyncConstraint::send_only());
    assert!(merged.require_send && merged.require_sync);
}

#[test]
fn test_send_sync_constraint_is_satisfied() {
    let s = SendSyncConstraint::send_sync();
    assert!(s.is_satisfied(true, true));
    assert!(!s.is_satisfied(true, false));
    assert!(!s.is_satisfied(false, true));
    assert!(!s.is_satisfied(false, false));
}

// ===================================================================
// §8.7: 基本类型固有 Send/Sync
// ===================================================================

#[test]
fn test_primitives_are_inherently_send_sync() {
    let solver = SendSyncSolver::new();
    for ty in &[
        MonoType::Int(32),
        MonoType::Int(64),
        MonoType::Float(32),
        MonoType::Float(64),
        MonoType::Bool,
        MonoType::Char,
        MonoType::String,
        MonoType::Bytes,
        MonoType::Void,
    ] {
        assert!(solver.is_send(ty), "expected send: {:?}", ty);
        assert!(solver.is_sync(ty), "expected sync: {:?}", ty);
    }
}

// ===================================================================
// §8.7: Arc/Weak 固有 Send/Sync
// ===================================================================

#[test]
fn test_arc_weak_are_send_sync() {
    let solver = SendSyncSolver::new();
    assert!(solver.is_send(&MonoType::Arc(Box::new(MonoType::Int(32)))));
    assert!(solver.is_sync(&MonoType::Arc(Box::new(MonoType::Int(32)))));
    assert!(solver.is_send(&MonoType::Weak(Box::new(MonoType::String))));
    assert!(solver.is_sync(&MonoType::Weak(Box::new(MonoType::String))));
}

// ===================================================================
// §8.7: 枚举固有 Send/Sync
// ===================================================================

#[test]
fn test_enum_is_send_sync() {
    let solver = SendSyncSolver::new();
    let e = empty_struct("E");
    assert!(solver.is_send(&e));
    assert!(solver.is_sync(&e));
}

// ===================================================================
// §8.7: 结构体——所有字段需满足 Send/Sync
// ===================================================================

#[test]
fn test_struct_send_all_fields_send() {
    let solver = SendSyncSolver::new();
    let s = MonoType::Struct(StructType {
        name: "AllSend".to_string(),
        fields: vec![
            ("a".to_string(), MonoType::Int(32)),
            ("b".to_string(), MonoType::String),
        ],
        methods: HashMap::new(),
        field_mutability: vec![false, false],
        field_has_default: vec![false, false],
        interfaces: vec![],
    });
    // All fields are Send → struct is Send
    assert!(solver.is_send(&s));
    assert!(solver.is_sync(&s));
}

#[test]
fn test_struct_send_mixed_fields() {
    let solver = SendSyncSolver::new();
    let tv = new_var(0);
    let s = MonoType::Struct(StructType {
        name: "Mixed".to_string(),
        fields: vec![
            ("safe".to_string(), MonoType::Int(32)), // Send
            ("unsafe".to_string(), tv.clone()),      // TypeVar → not inherently Send
        ],
        methods: HashMap::new(),
        field_mutability: vec![false, false],
        field_has_default: vec![false, false],
        interfaces: vec![],
    });
    // TypeVar is not inherently Send, so struct with TypeVar field is not Send
    assert!(!solver.is_send(&s));
}

// ===================================================================
// §8.7: 元组——所有元素需满足 Send/Sync
// ===================================================================

#[test]
fn test_tuple_send_all_elements_send() {
    let solver = SendSyncSolver::new();
    let t = MonoType::Tuple(vec![MonoType::Int(32), MonoType::Bool]);
    assert!(solver.is_send(&t));
    assert!(solver.is_sync(&t));
}

#[test]
fn test_tuple_send_with_type_var() {
    let solver = SendSyncSolver::new();
    let t = MonoType::Tuple(vec![MonoType::Int(32), new_var(0)]);
    assert!(!solver.is_send(&t));
}

// ===================================================================
// §8.7: 联合/交集——所有成员需满足 Send/Sync
// ===================================================================

#[test]
fn test_union_send_all_members_send() {
    let solver = SendSyncSolver::new();
    let u = MonoType::Union(vec![MonoType::Int(32), MonoType::String]);
    assert!(solver.is_send(&u));
    assert!(solver.is_sync(&u));
}

#[test]
fn test_union_send_with_non_send_member() {
    let solver = SendSyncSolver::new();
    let u = MonoType::Union(vec![MonoType::Int(32), new_var(0)]);
    assert!(!solver.is_send(&u));
}

#[test]
fn test_intersection_send_all_members_send() {
    let solver = SendSyncSolver::new();
    let i = MonoType::Intersection(vec![MonoType::Int(32), MonoType::Bool]);
    assert!(solver.is_send(&i));
}

// ===================================================================
// §8.7: 容器类型——约束传播到元素
// ===================================================================

#[test]
fn test_list_send_propagation() {
    let solver = SendSyncSolver::new();
    assert!(solver.is_send(&MonoType::List(Box::new(MonoType::Int(32)))));
    assert!(!solver.is_send(&MonoType::List(Box::new(new_var(0)))));
}

#[test]
fn test_set_send_propagation() {
    let solver = SendSyncSolver::new();
    assert!(solver.is_send(&MonoType::Set(Box::new(MonoType::Int(32)))));
    assert!(!solver.is_send(&MonoType::Set(Box::new(new_var(0)))));
}

#[test]
fn test_range_send_propagation() {
    let solver = SendSyncSolver::new();
    let r = MonoType::Range {
        elem_type: Box::new(MonoType::Int(32)),
    };
    assert!(solver.is_send(&r));
}

#[test]
fn test_dict_send_propagation() {
    let solver = SendSyncSolver::new();
    let d = MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(32)));
    assert!(solver.is_send(&d));
    let d2 = MonoType::Dict(Box::new(new_var(0)), Box::new(MonoType::Int(32)));
    assert!(!solver.is_send(&d2));
}

#[test]
fn test_fn_send_propagation() {
    let solver = SendSyncSolver::new();
    let f = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::Bool),
    };
    assert!(solver.is_send(&f));
    let f2 = MonoType::Fn {
        params: vec![new_var(0)],
        return_type: Box::new(MonoType::Bool),
    };
    assert!(!solver.is_send(&f2));
}

// ===================================================================
// §8.7: TypeRef — Rc 不是 Send/Sync
// ===================================================================

#[test]
fn test_rc_typeref_not_send() {
    let solver = SendSyncSolver::new();
    assert!(!solver.is_send(&MonoType::TypeRef("Rc(Int)".to_string())));
    assert!(!solver.is_sync(&MonoType::TypeRef("Rc(Int)".to_string())));
}

#[test]
fn test_other_typeref_is_send() {
    let solver = SendSyncSolver::new();
    assert!(solver.is_send(&MonoType::TypeRef("MySendType".to_string())));
    assert!(solver.is_sync(&MonoType::TypeRef("MySyncType".to_string())));
}

// ===================================================================
// §8.7: 类型变量——默认不是 Send/Sync
// ===================================================================

#[test]
fn test_type_var_not_inherently_send_sync() {
    let solver = SendSyncSolver::new();
    assert!(!solver.is_send(&new_var(0)));
    assert!(!solver.is_sync(&new_var(1)));
}

// ===================================================================
// §8.7: 约束传播——add_constraint 到所有容器类型
// ===================================================================

#[test]
fn test_constraint_propagation_through_union() {
    let mut solver = SendSyncSolver::new();
    let v0 = new_var(0);
    let v1 = new_var(1);
    let union = MonoType::Union(vec![v0.clone(), v1.clone()]);
    solver.add_send_constraint(&union);
    assert!(solver.is_send(&v0));
    assert!(solver.is_send(&v1));
}

#[test]
fn test_constraint_propagation_through_intersection() {
    let mut solver = SendSyncSolver::new();
    let v0 = new_var(0);
    let v1 = new_var(1);
    let inter = MonoType::Intersection(vec![v0.clone(), v1.clone()]);
    solver.add_send_constraint(&inter);
    assert!(solver.is_send(&v0));
    assert!(solver.is_send(&v1));
}

#[test]
fn test_constraint_propagation_through_arc_weak() {
    let mut solver = SendSyncSolver::new();
    let v0 = new_var(0);
    solver.add_send_constraint(&MonoType::Arc(Box::new(v0.clone())));
    assert!(solver.is_send(&v0));

    let mut solver2 = SendSyncSolver::new();
    let v1 = new_var(1);
    solver2.add_send_constraint(&MonoType::Weak(Box::new(v1.clone())));
    assert!(solver2.is_send(&v1));
}

#[test]
fn test_constraint_propagation_through_dict() {
    let mut solver = SendSyncSolver::new();
    let k = new_var(0);
    let v = new_var(1);
    solver.add_send_constraint(&MonoType::Dict(Box::new(k.clone()), Box::new(v.clone())));
    assert!(solver.is_send(&k));
    assert!(solver.is_send(&v));
}

#[test]
fn test_constraint_propagation_through_set() {
    let mut solver = SendSyncSolver::new();
    let v0 = new_var(0);
    solver.add_send_constraint(&MonoType::Set(Box::new(v0.clone())));
    assert!(solver.is_send(&v0));
}

#[test]
fn test_constraint_propagation_through_tuple() {
    let mut solver = SendSyncSolver::new();
    let v0 = new_var(0);
    let v1 = new_var(1);
    solver.add_send_constraint(&MonoType::Tuple(vec![v0.clone(), v1.clone()]));
    assert!(solver.is_send(&v0));
    assert!(solver.is_send(&v1));
}

#[test]
fn test_constraint_propagation_through_fn() {
    let mut solver = SendSyncSolver::new();
    let p = new_var(0);
    let r = new_var(1);
    solver.add_send_constraint(&MonoType::Fn {
        params: vec![p.clone()],
        return_type: Box::new(r.clone()),
    });
    assert!(solver.is_send(&p));
    assert!(solver.is_send(&r));
}

// ===================================================================
// §8.7: add_constraint 合并
// ===================================================================

#[test]
fn test_constraint_merging_on_same_var() {
    let mut solver = SendSyncSolver::new();
    let v = new_var(0);
    solver.add_send_constraint(&v); // Send only
    solver.add_sync_constraint(&v); // Sync only → merge → Send+Sync
    let c = solver.get_constraint(&v);
    assert!(c.require_send && c.require_sync);
}

#[test]
fn test_constraints_accessor() {
    let mut solver = SendSyncSolver::new();
    solver.add_send_constraint(&new_var(5));
    solver.add_sync_constraint(&new_var(7));
    let map = solver.constraints();
    assert_eq!(map.len(), 2);
    assert!(map.contains_key(&5));
    assert!(map.contains_key(&7));
}
