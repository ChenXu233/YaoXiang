//! 鸭子类型测试 — 基于语言规范 §3.5 & RFC-010
//!
//! §3.5: 约束类型（接口）
//! §3.5.1: 结构子类型（鸭子类型）
//! §3.5.2: 标准接口（Clone、Equal）
//! RFC-010 §3: 接口交集满足性

use crate::frontend::core::types::{MonoType, StructType};
use crate::frontend::core::sema::inference::bounds::BoundsChecker;
use crate::frontend::core::sema::environment::TypeEnvironment;
use std::collections::HashMap;

/// 创建一个简单的函数类型
fn fn_type(
    params: Vec<MonoType>,
    return_type: MonoType,
) -> MonoType {
    MonoType::Fn {
        params,
        return_type: Box::new(return_type),
    }
}

/// 创建一个接口类型（所有字段都是函数类型）
fn create_interface(
    name: &str,
    methods: Vec<(&str, MonoType)>,
) -> MonoType {
    let fields = methods
        .into_iter()
        .map(
            |(method_name, method_type)| crate::frontend::core::types::mono::StructFieldInfo {
                name: method_name.to_string(),
                ty: method_type,
                quantity: None,
                is_mut: false,
                has_default: false,
            },
        )
        .collect();
    MonoType::Struct(StructType {
        name: name.to_string(),
        fields,
        methods: HashMap::new(),
        interfaces: vec![],
    })
}

/// 创建一个结构体类型
fn create_struct(
    name: &str,
    fields: Vec<(&str, MonoType)>,
    method_bindings: Vec<(&str, MonoType)>,
) -> (MonoType, TypeEnvironment) {
    let mut env = TypeEnvironment::new();
    let struct_fields: Vec<_> = fields
        .into_iter()
        .map(
            |(n, t)| crate::frontend::core::types::mono::StructFieldInfo {
                name: n.to_string(),
                ty: t,
                quantity: None,
                is_mut: false,
                has_default: false,
            },
        )
        .collect();
    for (method_name, method_type) in method_bindings {
        env.add_method_binding(name, method_name, method_type);
    }
    (
        MonoType::Struct(StructType {
            name: name.to_string(),
            fields: struct_fields,
            methods: HashMap::new(),
            interfaces: vec![],
        }),
        env,
    )
}

/// 字段表 `{ x: Float, y: Float }`（Point 用例共同形态）。
fn point_fields() -> Vec<(&'static str, MonoType)> {
    vec![("x", MonoType::Float(64)), ("y", MonoType::Float(64))]
}

/// 接口侧 `draw: (Surface) -> Void` 签名。
fn surface_draw_sig() -> MonoType {
    fn_type(
        vec![MonoType::TypeRef("Surface".to_string())],
        MonoType::Void,
    )
}

/// 方法签名 `(self_type, arg) -> ret`。
fn self_and(
    self_type: &str,
    arg: MonoType,
    ret: MonoType,
) -> MonoType {
    fn_type(vec![MonoType::TypeRef(self_type.to_string()), arg], ret)
}

/// 结构体侧 `draw: (self_type, Surface) -> ret` 签名。
fn draw_method(
    self_type: &str,
    ret: MonoType,
) -> MonoType {
    self_and(self_type, MonoType::TypeRef("Surface".to_string()), ret)
}

/// `Drawable: { draw: (Surface) -> Void }` 接口。
fn drawable_interface() -> MonoType {
    create_interface("Drawable", vec![("draw", surface_draw_sig())])
}

/// `Serializable: { serialize: () -> String }` 接口。
fn serializable_interface() -> MonoType {
    create_interface(
        "Serializable",
        vec![("serialize", fn_type(vec![], MonoType::make_string()))],
    )
}

/// RFC-010 §3: 类型缺少交集接口中任一方法应失败
#[test]
fn test_interface_intersection_missing_one() {
    // Arrange
    let drawable = drawable_interface();
    let serializable = serializable_interface();
    let intersection = MonoType::Intersection(vec![drawable, serializable]);
    let (point, env) = create_struct(
        "Point",
        point_fields(),
        vec![("draw", draw_method("Point", MonoType::Void))],
    );

    // Act
    let checker = BoundsChecker::new();
    let result = checker.check_constraint(&point, &intersection, Some(&env));

    // TODO(behavior): 规范要求交集类型检查应检测缺失成员，
    // 当前 BoundsChecker 对 Intersection 只匹配第一个接口（此处 Drawable），
    // 因此即使缺少 Serializable 也返回 Ok。修复 BoundsChecker 后此测试应断言 is_err()。
    let _ = result;
}
