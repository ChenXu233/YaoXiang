//! 自动trait派生（RFC-011）
//!
//! 支持 `#[derive(Clone, Debug, Display)]` 语法，
//! 自动为类型生成简单的trait实现

use crate::frontend::parser::ast;
use crate::frontend::typecheck::traits::{TraitImpl, TraitMethod, TraitRef};
use crate::util::span::Span;
use std::collections::HashSet;
use std::fmt;

/// 可自动派生的trait
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeriveTrait {
    Clone,
    Debug,
    Display,
    PartialEq,
    PartialOrd,
    Copy,
}

/// 自动派生的错误
#[derive(Debug, Clone)]
pub enum DeriveError {
    /// 不支持的trait
    UnsupportedTrait {
        trait_name: String,
        span: Span,
    },
    /// 循环依赖
    CyclicDependency {
        type_name: String,
        span: Span,
    },
    /// 字段类型不支持
    UnsupportedFieldType {
        field_name: String,
        field_type: String,
        span: Span,
    },
}

impl DeriveTrait {
    /// 从字符串解析DeriveTrait
    pub fn from_str(name: &str) -> Option<Self> {
        match name {
            "Clone" => Some(DeriveTrait::Clone),
            "Debug" => Some(DeriveTrait::Debug),
            "Display" => Some(DeriveTrait::Display),
            "PartialEq" => Some(DeriveTrait::PartialEq),
            "PartialOrd" => Some(DeriveTrait::PartialOrd),
            "Copy" => Some(DeriveTrait::Copy),
            _ => None,
        }
    }

    /// 获取trait名称
    pub fn name(&self) -> &'static str {
        match self {
            DeriveTrait::Clone => "Clone",
            DeriveTrait::Debug => "Debug",
            DeriveTrait::Display => "Display",
            DeriveTrait::PartialEq => "PartialEq",
            DeriveTrait::PartialOrd => "PartialOrd",
            DeriveTrait::Copy => "Copy",
        }
    }
}

/// 自动派生器
pub struct Deriver {
    /// 已派生的类型（用于循环检测）
    derived: HashSet<String>,
}

impl Deriver {
    /// 创建新的派生器
    pub fn new() -> Self {
        Deriver {
            derived: HashSet::new(),
        }
    }

    /// 为类型派生trait
    pub fn derive(
        &mut self,
        ty_name: &str,
        fields: &[(String, ast::Type)],
        traits: &[DeriveTrait],
    ) -> Result<Vec<TraitImpl>, DeriveError> {
        // 循环检测
        if self.derived.contains(ty_name) {
            return Err(DeriveError::CyclicDependency {
                type_name: ty_name.to_string(),
                span: Span::dummy(), // TODO: 需要真实的span
            });
        }

        self.derived.insert(ty_name.to_string());

        let mut impls = Vec::new();

        for trait_ in traits {
            match self.derive_trait(ty_name, fields, *trait_) {
                Ok(impl_) => impls.push(impl_),
                Err(e) => {
                    self.derived.remove(ty_name);
                    return Err(e);
                }
            }
        }

        self.derived.remove(ty_name);

        Ok(impls)
    }

    /// 派生特定的trait
    fn derive_trait(
        &self,
        ty_name: &str,
        fields: &[(String, ast::Type)],
        trait_: DeriveTrait,
    ) -> Result<TraitImpl, DeriveError> {
        match trait_ {
            DeriveTrait::Clone => self.derive_clone(ty_name, fields),
            DeriveTrait::Debug => self.derive_debug(ty_name, fields),
            DeriveTrait::Display => self.derive_display(ty_name, fields),
            DeriveTrait::PartialEq => self.derive_partial_eq(ty_name, fields),
            DeriveTrait::PartialOrd => self.derive_partial_ord(ty_name, fields),
            DeriveTrait::Copy => self.derive_copy(ty_name, fields),
        }
    }

    /// 派生Clone trait
    fn derive_clone(
        &self,
        ty_name: &str,
        fields: &[(String, ast::Type)],
    ) -> Result<TraitImpl, DeriveError> {
        // Clone需要所有字段都实现Clone
        for (field_name, field_type) in fields {
            if !self.supports_trait(field_type, &DeriveTrait::Clone) {
                return Err(DeriveError::UnsupportedFieldType {
                    field_name: field_name.clone(),
                    field_type: format!("{:?}", field_type),
                    span: Span::dummy(), // TODO: 需要真实的span
                });
            }
        }

        // 生成clone方法
        let body = format!(
            "fn clone(self) -> {} {{ {} {{ {} }} }}",
            ty_name,
            ty_name,
            fields
                .iter()
                .map(|(name, _)| format!("{}: self.{},", name, name))
                .collect::<Vec<_>>()
                .join(" ")
        );

        Ok(TraitImpl {
            for_type: ty_name.to_string(),
            trait_ref: TraitRef {
                name: "Clone".to_string(),
                args: Vec::new(),
            },
            methods: vec![ImplMethod {
                name: "clone".to_string(),
                body,
            }],
            span: Span::dummy(), // TODO: 需要真实的span
        })
    }

    /// 派生Debug trait
    fn derive_debug(
        &self,
        ty_name: &str,
        fields: &[(String, ast::Type)],
    ) -> Result<TraitImpl, DeriveError> {
        // Debug需要所有字段都实现Debug
        for (field_name, field_type) in fields {
            if !self.supports_trait(field_type, &DeriveTrait::Debug) {
                return Err(DeriveError::UnsupportedFieldType {
                    field_name: field_name.clone(),
                    field_type: format!("{:?}", field_type),
                    span: Span::dummy(), // TODO: 需要真实的span
                });
            }
        }

        let body = format!(
            "fn fmt(self, f) {{ write!(f, \"{}({}))\" {}) }}",
            ty_name,
            fields
                .iter()
                .map(|(name, _)| format!("{}: {{:?}}", name))
                .collect::<Vec<_>>()
                .join(", "),
            if fields.is_empty() {
                "".to_string()
            } else {
                ", " + &fields
                    .iter()
                    .map(|(name, _)| format!("self.{}", name))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );

        Ok(TraitImpl {
            for_type: ty_name.to_string(),
            trait_ref: TraitRef {
                name: "Debug".to_string(),
                args: Vec::new(),
            },
            methods: vec![ImplMethod {
                name: "fmt".to_string(),
                body,
            }],
            span: Span::dummy(), // TODO: 需要真实的span
        })
    }

    /// 派生Display trait
    fn derive_display(
        &self,
        ty_name: &str,
        fields: &[(String, ast::Type)],
    ) -> Result<TraitImpl, DeriveError> {
        // Display要求所有字段都实现Display
        for (field_name, field_type) in fields {
            if !self.supports_trait(field_type, &DeriveTrait::Display) {
                return Err(DeriveError::UnsupportedFieldType {
                    field_name: field_name.clone(),
                    field_type: format!("{:?}", field_type),
                    span: Span::dummy(), // TODO: 需要真实的span
                });
            }
        }

        let body = format!(
            "fn fmt(self, f) {{ write!(f, \"{}({}))\" {}) }}",
            ty_name,
            fields
                .iter()
                .map(|(name, _)| format!("{}: {{}}", name))
                .collect::<Vec<_>>()
                .join(", "),
            if fields.is_empty() {
                "".to_string()
            } else {
                ", " + &fields
                    .iter()
                    .map(|(name, _)| format!("self.{}", name))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );

        Ok(TraitImpl {
            for_type: ty_name.to_string(),
            trait_ref: TraitRef {
                name: "Display".to_string(),
                args: Vec::new(),
            },
            methods: vec![ImplMethod {
                name: "fmt".to_string(),
                body,
            }],
            span: Span::dummy(), // TODO: 需要真实的span
        })
    }

    /// 派生PartialEq trait
    fn derive_partial_eq(
        &self,
        ty_name: &str,
        fields: &[(String, ast::Type)],
    ) -> Result<TraitImpl, DeriveError> {
        // PartialEq需要所有字段都实现PartialEq
        for (field_name, field_type) in fields {
            if !self.supports_trait(field_type, &DeriveTrait::PartialEq) {
                return Err(DeriveError::UnsupportedFieldType {
                    field_name: field_name.clone(),
                    field_type: format!("{:?}", field_type),
                    span: Span::dummy(), // TODO: 需要真实的span
                });
            }
        }

        let body = format!(
            "fn eq(self, other) {{ {} }}",
            if fields.is_empty() {
                "true".to_string()
            } else {
                format!(
                    "{}",
                    fields
                        .iter()
                        .map(|(name, _)| format!("self.{} == other.{}", name, name))
                        .collect::<Vec<_>>()
                        .join(" && ")
                )
            }
        );

        Ok(TraitImpl {
            for_type: ty_name.to_string(),
            trait_ref: TraitRef {
                name: "PartialEq".to_string(),
                args: Vec::new(),
            },
            methods: vec![ImplMethod {
                name: "eq".to_string(),
                body,
            }],
            span: Span::dummy(), // TODO: 需要真实的span
        })
    }

    /// 派生PartialOrd trait
    fn derive_partial_ord(
        &self,
        ty_name: &str,
        fields: &[(String, ast::Type)],
    ) -> Result<TraitImpl, DeriveError> {
        // PartialOrd需要所有字段都实现PartialOrd
        for (field_name, field_type) in fields {
            if !self.supports_trait(field_type, &DeriveTrait::PartialOrd) {
                return Err(DeriveError::UnsupportedFieldType {
                    field_name: field_name.clone(),
                    field_type: format!("{:?}", field_type),
                    span: Span::dummy(), // TODO: 需要真实的span
                });
            }
        }

        // 简单的字典序比较
        let body = format!(
            "fn cmp(self, other) {{ {} }}",
            if fields.is_empty() {
                "Ordering::Equal".to_string()
            } else {
                format!(
                    "match {} {{ Ordering::Equal => {}, Ordering::Less | Ordering::Greater => result }}",
                    fields
                        .iter()
                        .map(|(name, _)| format!("self.{}.cmp(&other.{})", name, name))
                        .collect::<Vec<_>>()
                        .join(".then(|| ").to_string() + ")",
                    fields
                        .iter()
                        .map(|(_, _)| "result".to_string())
                        .collect::<Vec<_>>()
                        .join(", ").to_string() + ")"
                )
            }
        );

        Ok(TraitImpl {
            for_type: ty_name.to_string(),
            trait_ref: TraitRef {
                name: "PartialOrd".to_string(),
                args: Vec::new(),
            },
            methods: vec![ImplMethod {
                name: "cmp".to_string(),
                body,
            }],
            span: Span::dummy(), // TODO: 需要真实的span
        })
    }

    /// 派生Copy trait
    fn derive_copy(
        &self,
        ty_name: &str,
        fields: &[(String, ast::Type)],
    ) -> Result<TraitImpl, DeriveError> {
        // Copy要求所有字段都实现Copy
        for (field_name, field_type) in fields {
            if !self.supports_trait(field_type, &DeriveTrait::Copy) {
                return Err(DeriveError::UnsupportedFieldType {
                    field_name: field_name.clone(),
                    field_type: format!("{:?}", field_type),
                    span: Span::dummy(), // TODO: 需要真实的span
                });
            }
        }

        // Copy不需要方法，它是一个标记trait
        Ok(TraitImpl {
            for_type: ty_name.to_string(),
            trait_ref: TraitRef {
                name: "Copy".to_string(),
                args: Vec::new(),
            },
            methods: vec![],
            span: Span::dummy(), // TODO: 需要真实的span
        })
    }

    /// 检查类型是否支持特定trait
    fn supports_trait(&self, ty: &ast::Type, trait_: &DeriveTrait) -> bool {
        match ty {
            ast::Type::Name(name) => {
                // 基本类型都有一些trait实现
                matches!(
                    name.as_str(),
                    "Int" | "Float" | "Bool" | "String" | "Char"
                )
            }
            ast::Type::Int(_) | ast::Type::Float(_) | ast::Type::Bool | ast::Type::String | ast::Type::Char => true,
            _ => false,
        }
    }
}

impl Default for Deriver {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DeriveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeriveError::UnsupportedTrait { trait_name, .. } => {
                write!(f, "unsupported derive trait: {}", trait_name)
            }
            DeriveError::CyclicDependency { type_name, .. } => {
                write!(f, "cyclic dependency in type: {}", type_name)
            }
            DeriveError::UnsupportedFieldType {
                field_name,
                field_type,
                ..
            } => {
                write!(
                    f,
                    "field '{}' of type '{}' does not support derive",
                    field_name, field_type
                )
            }
        }
    }
}
