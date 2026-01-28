//! 类型系统核心数据结构测试

use std::collections::HashMap;

use crate::frontend::parser::ast;
use crate::frontend::typecheck::types::*;
use crate::util::span::Span;

fn create_dummy_span() -> Span {
    Span::dummy()
}

// =========================================================================
// TypeVar 测试
// =========================================================================

#[test]
fn test_type_var_creation() {
    let tv = TypeVar::new(0);
    assert_eq!(tv.index(), 0);
}

#[test]
fn test_type_var_index() {
    let tv = TypeVar::new(42);
    assert_eq!(tv.index(), 42);
}

#[test]
fn test_type_var_display_short() {
    let tv = TypeVar::new(5);
    let display = format!("{}", tv);
    assert_eq!(display, "t5");
}

#[test]
fn test_type_var_clone() {
    let tv1 = TypeVar::new(10);
    let tv2 = tv1.clone();
    assert_eq!(tv1.index(), tv2.index());
    assert_eq!(tv1, tv2);
}

#[test]
fn test_type_var_eq() {
    let tv1 = TypeVar::new(1);
    let tv2 = TypeVar::new(1);
    let tv3 = TypeVar::new(2);
    assert_eq!(tv1, tv2);
    assert_ne!(tv1, tv3);
}

// =========================================================================
// TypeBinding 测试
// =========================================================================

#[test]
fn test_type_binding_unbound() {
    let binding = TypeBinding::Unbound;
    assert!(matches!(binding, TypeBinding::Unbound));
}

#[test]
fn test_type_binding_bound() {
    let mono = MonoType::Int(64);
    let binding = TypeBinding::Bound(mono);
    assert!(matches!(binding, TypeBinding::Bound(MonoType::Int(64))));
}

#[test]
fn test_type_binding_link() {
    let tv1 = TypeVar::new(1);
    let _tv2 = TypeVar::new(2);
    let binding = TypeBinding::Link(tv1);
    assert!(matches!(binding, TypeBinding::Link(t) if t.index() == 1));
}

#[test]
fn test_type_binding_clone() {
    let binding1 = TypeBinding::Unbound;
    let binding2 = binding1.clone();
    assert!(matches!(binding2, TypeBinding::Unbound));

    let binding3 = TypeBinding::Bound(MonoType::Bool);
    let binding4 = binding3.clone();
    assert!(matches!(binding4, TypeBinding::Bound(MonoType::Bool)));
}

// =========================================================================
// MonoType 测试 - 基本类型
// =========================================================================

#[test]
fn test_mono_type_void() {
    let ty = MonoType::Void;
    assert!(matches!(ty, MonoType::Void));
    assert!(!ty.is_numeric());
}

#[test]
fn test_mono_type_bool() {
    let ty = MonoType::Bool;
    assert!(matches!(ty, MonoType::Bool));
    assert!(!ty.is_numeric());
}

#[test]
fn test_mono_type_int() {
    let ty = MonoType::Int(64);
    assert!(matches!(ty, MonoType::Int(64)));
    assert!(ty.is_numeric());
}

#[test]
fn test_mono_type_float() {
    let ty = MonoType::Float(64);
    assert!(matches!(ty, MonoType::Float(64)));
    assert!(ty.is_numeric());
}

#[test]
fn test_mono_type_char() {
    let ty = MonoType::Char;
    assert!(matches!(ty, MonoType::Char));
    assert!(!ty.is_numeric());
}

#[test]
fn test_mono_type_string() {
    let ty = MonoType::String;
    assert!(matches!(ty, MonoType::String));
    assert!(!ty.is_numeric());
}

#[test]
fn test_mono_type_bytes() {
    let ty = MonoType::Bytes;
    assert!(matches!(ty, MonoType::Bytes));
    assert!(!ty.is_numeric());
}

// =========================================================================
// MonoType 测试 - 容器类型
// =========================================================================

#[test]
fn test_mono_type_list() {
    let ty = MonoType::List(Box::new(MonoType::Int(64)));
    assert!(matches!(ty, MonoType::List(_)));
    assert!(ty.is_indexable());
}

#[test]
fn test_mono_type_dict() {
    let ty = MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(64)));
    assert!(matches!(ty, MonoType::Dict(_, _)));
    assert!(ty.is_indexable());
}

#[test]
fn test_mono_type_tuple() {
    let ty = MonoType::Tuple(vec![MonoType::Int(64), MonoType::String]);
    assert!(matches!(ty, MonoType::Tuple(ref t) if t.len() == 2));
    assert!(ty.is_indexable());
}

#[test]
fn test_mono_type_tuple_empty() {
    let ty = MonoType::Tuple(vec![]);
    assert!(matches!(ty, MonoType::Tuple(t) if t.is_empty()));
}

#[test]
fn test_mono_type_set() {
    let ty = MonoType::Set(Box::new(MonoType::Int(64)));
    assert!(matches!(ty, MonoType::Set(_)));
}

#[test]
fn test_mono_type_fn() {
    let ty = MonoType::Fn {
        params: vec![MonoType::Int(64)],
        return_type: Box::new(MonoType::String),
        is_async: false,
    };
    assert!(matches!(ty, MonoType::Fn { params, is_async: false, .. }
        if params.len() == 1));
}

#[test]
fn test_mono_type_fn_async() {
    let ty = MonoType::Fn {
        params: vec![],
        return_type: Box::new(MonoType::Void),
        is_async: true,
    };
    assert!(matches!(ty, MonoType::Fn { is_async: true, .. }));
}

#[test]
fn test_mono_type_fn_multiple_params() {
    let ty = MonoType::Fn {
        params: vec![MonoType::Int(64), MonoType::String, MonoType::Bool],
        return_type: Box::new(MonoType::Float(64)),
        is_async: false,
    };
    assert!(matches!(ty, MonoType::Fn { params, .. } if params.len() == 3));
}

// =========================================================================
// MonoType 测试 - 结构体和枚举
// =========================================================================

#[test]
fn test_mono_type_struct() {
    let ty = MonoType::Struct(StructType {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MonoType::Int(64)),
            ("y".to_string(), MonoType::Int(64)),
        ],
        methods: HashMap::new(),
    });
    assert!(matches!(ty, MonoType::Struct(s) if s.name == "Point"));
}

#[test]
fn test_mono_type_struct_empty() {
    let ty = MonoType::Struct(StructType {
        name: "Empty".to_string(),
        fields: vec![],
        methods: HashMap::new(),
    });
    assert!(matches!(ty, MonoType::Struct(s) if s.fields.is_empty()));
}

#[test]
fn test_mono_type_enum() {
    let ty = MonoType::Enum(EnumType {
        name: "Color".to_string(),
        variants: vec!["Red".to_string(), "Green".to_string(), "Blue".to_string()],
    });
    assert!(matches!(ty, MonoType::Enum(ref e) if e.name == "Color"));
    assert_eq!(ty.type_name(), "Color");
}

// =========================================================================
// MonoType 测试 - 类型变量和引用
// =========================================================================

#[test]
fn test_mono_type_type_var() {
    let tv = TypeVar::new(0);
    let ty = MonoType::TypeVar(tv);
    assert!(matches!(ty, MonoType::TypeVar(t) if t.index() == 0));
}

#[test]
fn test_mono_type_type_var_from() {
    let ty = MonoType::Void;
    let tv = ty.type_var();
    assert!(tv.is_none());
}

#[test]
fn test_mono_type_type_ref() {
    let ty = MonoType::TypeRef("MyCustomType".to_string());
    assert!(matches!(ty, MonoType::TypeRef(s) if s == "MyCustomType"));
}

// =========================================================================
// MonoType 测试 - 类型检查方法
// =========================================================================

#[test]
fn test_mono_type_is_numeric_int() {
    let ty = MonoType::Int(64);
    assert!(ty.is_numeric());
}

#[test]
fn test_mono_type_is_numeric_float() {
    let ty = MonoType::Float(64);
    assert!(ty.is_numeric());
}

#[test]
fn test_mono_type_is_numeric_bool() {
    let ty = MonoType::Bool;
    assert!(!ty.is_numeric());
}

#[test]
fn test_mono_type_is_numeric_string() {
    let ty = MonoType::String;
    assert!(!ty.is_numeric());
}

#[test]
fn test_mono_type_is_indexable_list() {
    let ty = MonoType::List(Box::new(MonoType::Int(64)));
    assert!(ty.is_indexable());
}

#[test]
fn test_mono_type_is_indexable_dict() {
    let ty = MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(64)));
    assert!(ty.is_indexable());
}

#[test]
fn test_mono_type_is_indexable_string() {
    let ty = MonoType::String;
    assert!(ty.is_indexable());
}

#[test]
fn test_mono_type_is_indexable_tuple() {
    let ty = MonoType::Tuple(vec![MonoType::Int(64)]);
    assert!(ty.is_indexable());
}

#[test]
fn test_mono_type_is_indexable_int() {
    let ty = MonoType::Int(64);
    assert!(!ty.is_indexable());
}

// =========================================================================
// MonoType 测试 - 类型名称
// =========================================================================

#[test]
fn test_mono_type_type_name_void() {
    let ty = MonoType::Void;
    assert_eq!(ty.type_name(), "void");
}

#[test]
fn test_mono_type_type_name_bool() {
    let ty = MonoType::Bool;
    assert_eq!(ty.type_name(), "bool");
}

#[test]
fn test_mono_type_type_name_int() {
    let ty = MonoType::Int(64);
    assert_eq!(ty.type_name(), "int64");
}

#[test]
fn test_mono_type_type_name_float() {
    let ty = MonoType::Float(64);
    assert_eq!(ty.type_name(), "float64");
}

#[test]
fn test_mono_type_type_name_char() {
    let ty = MonoType::Char;
    assert_eq!(ty.type_name(), "char");
}

#[test]
fn test_mono_type_type_name_string() {
    let ty = MonoType::String;
    assert_eq!(ty.type_name(), "string");
}

#[test]
fn test_mono_type_type_name_bytes() {
    let ty = MonoType::Bytes;
    assert_eq!(ty.type_name(), "bytes");
}

#[test]
fn test_mono_type_type_name_list() {
    let ty = MonoType::List(Box::new(MonoType::Int(64)));
    assert_eq!(ty.type_name(), "List<int64>");
}

#[test]
fn test_mono_type_type_name_dict() {
    let ty = MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(64)));
    assert_eq!(ty.type_name(), "Dict<string, int64>");
}

#[test]
fn test_mono_type_type_name_tuple() {
    let ty = MonoType::Tuple(vec![MonoType::Int(64), MonoType::String]);
    assert_eq!(ty.type_name(), "(int64, string)");
}

#[test]
fn test_mono_type_type_name_fn() {
    let ty = MonoType::Fn {
        params: vec![MonoType::Int(64)],
        return_type: Box::new(MonoType::String),
        is_async: false,
    };
    assert_eq!(ty.type_name(), "fn(int64) -> string");
}

#[test]
fn test_mono_type_type_name_type_var() {
    let ty = MonoType::TypeVar(TypeVar::new(5));
    assert_eq!(ty.type_name(), "t5");
}

#[test]
fn test_mono_type_type_name_type_ref() {
    let ty = MonoType::TypeRef("MyType".to_string());
    assert_eq!(ty.type_name(), "MyType");
}

// =========================================================================
// PolyType 测试
// =========================================================================

#[test]
fn test_poly_type_mono() {
    let poly = PolyType::mono(MonoType::Int(64));
    assert!(poly.type_binders.is_empty());
    assert_eq!(poly.body, MonoType::Int(64));
}

#[test]
fn test_poly_type_new() {
    let tv1 = TypeVar::new(0);
    let tv2 = TypeVar::new(1);
    let poly = PolyType::new(
        vec![tv1, tv2],
        MonoType::Fn {
            params: vec![MonoType::TypeVar(tv1)],
            return_type: Box::new(MonoType::TypeVar(tv2)),
            is_async: false,
        },
    );
    assert_eq!(poly.type_binders.len(), 2);
}

#[test]
fn test_poly_type_clone() {
    let poly1 = PolyType::mono(MonoType::Bool);
    let poly2 = poly1.clone();
    assert_eq!(poly1, poly2);
}

// =========================================================================
// TypeConstraint 测试
// =========================================================================

#[test]
fn test_type_constraint_new() {
    let left = MonoType::Int(64);
    let right = MonoType::Float(64);
    let span = create_dummy_span();
    let constraint = TypeConstraint::new(left, right, span);

    assert_eq!(constraint.left, MonoType::Int(64));
    assert_eq!(constraint.right, MonoType::Float(64));
}

#[test]
fn test_type_constraint_clone() {
    let left = MonoType::Int(64);
    let right = MonoType::Float(64);
    let span = create_dummy_span();
    let constraint1 = TypeConstraint::new(left, right, span);
    let constraint2 = constraint1.clone();

    assert_eq!(constraint1.left, constraint2.left);
    assert_eq!(constraint1.right, constraint2.right);
}

// =========================================================================
// TypeConstraintSolver 测试
// =========================================================================

#[test]
fn test_type_constraint_solver_new() {
    let _solver = TypeConstraintSolver::new();
    // Solver should be ready to use
}

#[test]
fn test_type_constraint_solver_new_var() {
    let mut solver = TypeConstraintSolver::new();
    let tv = solver.new_var();
    assert!(matches!(tv, MonoType::TypeVar(_)));
}

#[test]
fn test_type_constraint_solver_generalize() {
    let mut solver = TypeConstraintSolver::new();
    let ty = MonoType::Int(64);
    let poly = solver.generalize(&ty);
    assert!(poly.type_binders.is_empty());
}

// =========================================================================
// From<ast::Type> 测试
// =========================================================================

#[test]
fn test_from_ast_type_name() {
    let ast_type = ast::Type::Name("MyType".to_string());
    let mono: MonoType = ast_type.into();
    assert!(matches!(mono, MonoType::TypeRef(s) if s == "MyType"));
}

#[test]
fn test_from_ast_type_int() {
    let ast_type = ast::Type::Int(32);
    let mono: MonoType = ast_type.into();
    assert!(matches!(mono, MonoType::Int(32)));
}

#[test]
fn test_from_ast_type_float() {
    let ast_type = ast::Type::Float(32);
    let mono: MonoType = ast_type.into();
    assert!(matches!(mono, MonoType::Float(32)));
}

#[test]
fn test_from_ast_type_void() {
    let ast_type = ast::Type::Void;
    let mono: MonoType = ast_type.into();
    assert!(matches!(mono, MonoType::Void));
}

#[test]
fn test_from_ast_type_bool() {
    let ast_type = ast::Type::Bool;
    let mono: MonoType = ast_type.into();
    assert!(matches!(mono, MonoType::Bool));
}

#[test]
fn test_from_ast_type_char() {
    let ast_type = ast::Type::Char;
    let mono: MonoType = ast_type.into();
    assert!(matches!(mono, MonoType::Char));
}

#[test]
fn test_from_ast_type_string() {
    let ast_type = ast::Type::String;
    let mono: MonoType = ast_type.into();
    assert!(matches!(mono, MonoType::String));
}

#[test]
fn test_from_ast_type_bytes() {
    let ast_type = ast::Type::Bytes;
    let mono: MonoType = ast_type.into();
    assert!(matches!(mono, MonoType::Bytes));
}

#[test]
fn test_from_ast_type_tuple() {
    let ast_type = ast::Type::Tuple(vec![ast::Type::Int(64), ast::Type::String]);
    let mono: MonoType = ast_type.into();
    assert!(matches!(mono, MonoType::Tuple(t) if t.len() == 2));
}

#[test]
fn test_from_ast_type_list() {
    let ast_type = ast::Type::List(Box::new(ast::Type::Int(64)));
    let mono: MonoType = ast_type.into();
    assert!(matches!(mono, MonoType::List(_)));
}

#[test]
fn test_from_ast_type_dict() {
    let ast_type = ast::Type::Dict(Box::new(ast::Type::String), Box::new(ast::Type::Int(64)));
    let mono: MonoType = ast_type.into();
    assert!(matches!(mono, MonoType::Dict(_, _)));
}

#[test]
fn test_from_ast_type_fn() {
    let ast_type = ast::Type::Fn {
        params: vec![ast::Type::Int(64)],
        return_type: Box::new(ast::Type::String),
    };
    let mono: MonoType = ast_type.into();
    assert!(matches!(mono, MonoType::Fn { params, is_async: false, .. }
        if params.len() == 1));
}

#[test]
fn test_from_ast_type_generic() {
    let ast_type = ast::Type::Generic {
        name: "List".to_string(),
        args: vec![ast::Type::Int(64)],
    };
    let mono: MonoType = ast_type.into();
    assert!(matches!(mono, MonoType::TypeRef(s) if s == "List<int64>"));
}

// =========================================================================
// Eq 和 Clone 测试
// =========================================================================

#[test]
fn test_mono_type_eq() {
    let ty1 = MonoType::Int(64);
    let ty2 = MonoType::Int(64);
    let ty3 = MonoType::Int(32);
    assert_eq!(ty1, ty2);
    assert_ne!(ty1, ty3);
}

#[test]
fn test_mono_type_clone() {
    let ty1 = MonoType::List(Box::new(MonoType::Int(64)));
    let ty2 = ty1.clone();
    assert_eq!(ty1, ty2);
}

// =========================================================================
// Debug 和 Display 测试
// =========================================================================

#[test]
fn test_mono_type_debug() {
    let ty = MonoType::Int(64);
    let debug = format!("{:?}", ty);
    assert!(debug.contains("Int"));
}

#[test]
fn test_mono_type_display() {
    let ty = MonoType::Int(64);
    let display = format!("{}", ty);
    assert_eq!(display, "int64");
}

#[test]
fn test_type_var_debug() {
    let tv = TypeVar::new(0);
    let debug = format!("{:?}", tv);
    assert!(debug.contains("TypeVar"));
}

#[test]
fn test_type_var_display_value() {
    let tv = TypeVar::new(10);
    let display = format!("{}", tv);
    assert_eq!(display, "t10");
}
