//! 字面量类型验证
//!
//! 实现 Const 泛型的字面量类型验证和类型检查。
//!
//! 支持：
//! - 字面量类型到 ConstValue 的转换
//! - Const 参数的类型验证
//! - 字面量类型约束检查

use crate::frontend::core::parser::ast::{Type, GenericParam, GenericParamKind};
use crate::frontend::core::type_system::{ConstValue, ConstKind, MonoType};
use std::collections::HashMap;

/// Integer types for const parameters
const INT_TYPES: &[&str] = &["Int", "I8", "I16", "I32", "I64", "U8", "U16", "U32", "U64"];
/// Float types for const parameters
const FLOAT_TYPES: &[&str] = &["Float", "F32", "F64"];

/// 字面量类型信息
#[derive(Debug, Clone)]
pub struct LiteralTypeInfo {
    /// 字面量名称
    pub name: String,
    /// 对应的 ConstValue
    pub value: ConstValue,
    /// 基础类型
    pub base_type: MonoType,
}

/// Helper to convert type name to ConstKind
fn type_name_to_const_kind(name: &str) -> Option<ConstKind> {
    if INT_TYPES.contains(&name) {
        Some(ConstKind::Int(None))
    } else if name == "Bool" {
        Some(ConstKind::Bool)
    } else if FLOAT_TYPES.contains(&name) {
        Some(ConstKind::Float(None))
    } else if name == "Char" {
        Some(ConstKind::Int(None))
    } else {
        None
    }
}

/// 字面量类型验证器
#[derive(Debug, Clone, Default)]
pub struct LiteralTypeValidator {
    /// 注册的字面量类型
    literal_types: HashMap<String, LiteralTypeInfo>,
    /// Const 参数绑定
    const_params: HashMap<String, ConstValue>,
}

impl LiteralTypeValidator {
    /// 创建新的验证器
    pub fn new() -> Self {
        Self {
            literal_types: HashMap::new(),
            const_params: HashMap::new(),
        }
    }

    /// 注册字面量类型
    pub fn register_literal_type(
        &mut self,
        name: String,
        value: ConstValue,
        base_type: MonoType,
    ) {
        self.literal_types.insert(
            name.clone(),
            LiteralTypeInfo {
                name,
                value,
                base_type,
            },
        );
    }

    /// 绑定 Const 参数
    pub fn bind_const_param(
        &mut self,
        name: String,
        value: ConstValue,
    ) {
        self.const_params.insert(name, value);
    }

    /// 解析 AST 类型为字面量类型信息
    pub fn parse_literal_type<'a>(
        &'a self,
        ty: &'a Type,
    ) -> Option<(String, ConstValue)> {
        match ty {
            Type::Literal { name, base_type: _ } => {
                // 首先检查是否是已注册的 Const 参数
                if let Some(value) = self.const_params.get(name) {
                    return Some((name.clone(), value.clone()));
                }
                // 然后检查是否是已注册的字面量类型
                if let Some(info) = self.literal_types.get(name) {
                    return Some((info.name.clone(), info.value.clone()));
                }
                // 尝试从名称解析
                if let Some(value) = ConstValue::from_literal_name(name) {
                    return Some((name.clone(), value));
                }
                None
            }
            Type::Name(name) => {
                // 检查是否是已注册的 Const 参数
                if let Some(value) = self.const_params.get(name) {
                    return Some((name.clone(), value.clone()));
                }
                None
            }
            _ => None,
        }
    }

    /// 验证类型是否是有效的 Const 类型
    pub fn validate_const_type(
        &self,
        ty: &Type,
    ) -> Option<ConstKind> {
        match ty {
            Type::Name(name) => type_name_to_const_kind(name),
            Type::Literal { name, .. } => ConstValue::from_literal_name(name).map(|v| v.kind()),
            _ => None,
        }
    }

    /// 检查值是否是给定类型的有效值
    pub fn matches_type(
        &self,
        value: &ConstValue,
        kind: &ConstKind,
    ) -> bool {
        kind.matches(value)
    }

    /// 获取所有注册的 Const 参数
    pub fn const_params(&self) -> &HashMap<String, ConstValue> {
        &self.const_params
    }

    /// 获取所有注册的字面量类型
    pub fn literal_types(&self) -> &HashMap<String, LiteralTypeInfo> {
        &self.literal_types
    }

    /// 清除所有绑定
    pub fn clear(&mut self) {
        self.literal_types.clear();
        self.const_params.clear();
    }
}

/// 从 AST GenericParam 提取 Const 参数信息
pub fn extract_const_param_info(param: &GenericParam) -> Option<(String, ConstKind)> {
    match &param.kind {
        GenericParamKind::Const { const_type } => {
            let name = param.name.clone();
            if let Type::Name(type_name) = const_type.as_ref() {
                type_name_to_const_kind(type_name).map(|kind| (name, kind))
            } else {
                None
            }
        }
        GenericParamKind::Type => None,
        GenericParamKind::Platform => None, // 平台参数不是常量参数
    }
}

/// 将 AST 类型转换为 MonoType
pub fn ast_type_to_mono_type(ty: &Type) -> Option<MonoType> {
    match ty {
        Type::Name(name) => Some(MonoType::TypeRef(name.clone())),
        Type::Int(n) => Some(MonoType::Int(*n)),
        Type::Float(n) => Some(MonoType::Float(*n)),
        Type::Char => Some(MonoType::Char),
        Type::String => Some(MonoType::String),
        Type::Bytes => Some(MonoType::Bytes),
        Type::Bool => Some(MonoType::Bool),
        Type::Void => Some(MonoType::Void),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_type_parsing() {
        let mut validator = LiteralTypeValidator::new();

        // 注册字面量类型
        validator.register_literal_type("N".to_string(), ConstValue::Int(10), MonoType::Int(64));

        // 解析已注册的字面量
        let ty = Type::Literal {
            name: "N".to_string(),
            base_type: Box::new(Type::Name("Int".to_string())),
        };
        let result = validator.parse_literal_type(&ty);
        assert!(result.is_some());
        let (name, value) = result.unwrap();
        assert_eq!(name, "N");
        assert_eq!(value, ConstValue::Int(10));
    }

    #[test]
    fn test_const_param_binding() {
        let mut validator = LiteralTypeValidator::new();

        // 绑定 Const 参数
        validator.bind_const_param("n".to_string(), ConstValue::Int(5));

        // 解析 Const 参数
        let ty = Type::Name("n".to_string());
        let result = validator.parse_literal_type(&ty);
        assert!(result.is_some());
        let (name, value) = result.unwrap();
        assert_eq!(name, "n");
        assert_eq!(value, ConstValue::Int(5));
    }

    #[test]
    fn test_validate_const_type() {
        let validator = LiteralTypeValidator::new();

        // 验证 Int 类型
        let ty = Type::Name("Int".to_string());
        let kind = validator.validate_const_type(&ty);
        assert_eq!(kind, Some(ConstKind::Int(None)));

        // 验证 Bool 类型
        let ty = Type::Name("Bool".to_string());
        let kind = validator.validate_const_type(&ty);
        assert_eq!(kind, Some(ConstKind::Bool));
    }

    #[test]
    fn test_matches_type() {
        let validator = LiteralTypeValidator::new();

        // Int 值匹配 Int 类型
        assert!(validator.matches_type(&ConstValue::Int(5), &ConstKind::Int(None)));

        // Bool 值匹配 Bool 类型
        assert!(validator.matches_type(&ConstValue::Bool(true), &ConstKind::Bool));

        // Int 值不匹配 Bool 类型
        assert!(!validator.matches_type(&ConstValue::Int(5), &ConstKind::Bool));
    }
}
