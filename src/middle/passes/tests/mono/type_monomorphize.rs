// 测试文件已简化，需要重新创建完整版本
// 这个文件包含了基本的导入和辅助函数
// 原始文件在测试过程中被意外删除

use crate::frontend::typecheck::{MonoType, TypeVar};

/// 创建类型变量的辅助函数
fn make_type_var(index: usize) -> TypeVar {
    TypeVar::new(index)
}

/// 创建类型变量的 MonoType
fn type_var(index: usize) -> MonoType {
    MonoType::TypeVar(make_type_var(index))
}

/// 创建整数的辅助函数
fn int_type() -> MonoType {
    MonoType::Int(64)
}

/// 创建浮点数的辅助函数
fn float_type() -> MonoType {
    MonoType::Float(64)
}

/// 创建字符串类型的辅助函数
fn string_type() -> MonoType {
    MonoType::String
}

/// 创建 Arc 类型的辅助函数
fn arc_type(inner: MonoType) -> MonoType {
    MonoType::Arc(Box::new(inner))
}

/// 创建列表类型的辅助函数
fn list_type(elem: MonoType) -> MonoType {
    MonoType::List(Box::new(elem))
}

/// 创建字典类型的辅助函数
fn dict_type(
    key: MonoType,
    value: MonoType,
) -> MonoType {
    MonoType::Dict(Box::new(key), Box::new(value))
}
