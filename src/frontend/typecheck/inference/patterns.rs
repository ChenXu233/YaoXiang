#![allow(clippy::result_large_err)]

//! 模式匹配推断
//!
//! 实现模式匹配的类型推断

use crate::util::diagnostic::Result;
use crate::frontend::core::parser::ast;
use crate::frontend::core::type_system::MonoType;

/// 模式匹配推断器
pub struct PatternInferrer {
    next_type_var: usize,
}

impl Default for PatternInferrer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternInferrer {
    /// 创建新的模式推断器
    pub fn new() -> Self {
        Self { next_type_var: 0 }
    }

    fn fresh_type_var(&mut self) -> MonoType {
        let var = crate::frontend::core::type_system::var::TypeVar::new(self.next_type_var);
        self.next_type_var += 1;
        MonoType::TypeVar(var)
    }

    /// 推断模式类型
    #[allow(clippy::only_used_in_recursion)]
    pub fn infer_pattern(
        &mut self,
        pattern: &ast::Pattern,
    ) -> Result<MonoType> {
        match pattern {
            ast::Pattern::Wildcard => {
                // 通配符模式可以匹配任何类型，返回类型变量
                Ok(self.fresh_type_var())
            }
            ast::Pattern::Literal(lit) => {
                // 字面量模式根据字面量类型推断
                match lit {
                    crate::frontend::core::lexer::tokens::Literal::Int(_) => Ok(MonoType::Int(64)),
                    crate::frontend::core::lexer::tokens::Literal::Float(_) => {
                        Ok(MonoType::Float(64))
                    }
                    crate::frontend::core::lexer::tokens::Literal::Bool(_) => Ok(MonoType::Bool),
                    crate::frontend::core::lexer::tokens::Literal::Char(_) => Ok(MonoType::Char),
                    crate::frontend::core::lexer::tokens::Literal::String(_) => {
                        Ok(MonoType::String)
                    }
                }
            }
            ast::Pattern::Identifier(name) => {
                // 标识符模式返回类型变量
                let _ = name;
                Ok(self.fresh_type_var())
            }
            ast::Pattern::Tuple(patterns) => {
                // 元组模式：推断每个元素的类型
                let mut element_types = Vec::new();
                for pattern in patterns {
                    let ty = self.infer_pattern(pattern)?;
                    element_types.push(ty);
                }
                Ok(MonoType::Tuple(element_types))
            }
            ast::Pattern::Struct { name, fields } => {
                // 结构体模式：推断字段类型
                let mut field_types = Vec::new();
                let mut field_mutability = Vec::new();
                for (field_name, is_mut, pattern) in fields {
                    let field_ty = self.infer_pattern(pattern)?;
                    field_types.push((field_name.clone(), field_ty));
                    field_mutability.push(*is_mut);
                }
                Ok(MonoType::Struct(
                    crate::frontend::core::type_system::StructType {
                        name: name.clone(),
                        fields: field_types,
                        methods: std::collections::HashMap::new(),
                        field_mutability,
                        field_has_default: Vec::new(),
                    },
                ))
            }
            ast::Pattern::Or(patterns) => {
                // OR模式：所有分支必须有相同类型，取第一个
                if let Some(first) = patterns.first() {
                    self.infer_pattern(first)
                } else {
                    Ok(MonoType::Void)
                }
            }
            _ => {
                // 其他模式返回类型变量
                Ok(self.fresh_type_var())
            }
        }
    }

    /// 推断字面量模式类型
    pub fn infer_literal_pattern(
        &mut self,
        _pattern: &ast::Pattern,
    ) -> Result<MonoType> {
        Ok(MonoType::Bool) // 默认返回布尔类型
    }

    /// 推断绑定模式类型
    pub fn infer_binding_pattern(
        &mut self,
        _name: &str,
    ) -> Result<MonoType> {
        Ok(self.fresh_type_var()) // 返回类型变量
    }

    /// 推断通配符模式类型
    pub fn infer_wildcard_pattern(&mut self) -> Result<MonoType> {
        Ok(self.fresh_type_var()) // 返回类型变量
    }
}
