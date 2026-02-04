//! Derive 宏支持
//!
//! 实现 RFC-011 Derive 功能：
//! - `#[derive(Trait)]` 语法解析
//! - 自动生成 Trait 实现

use std::collections::HashMap;
use crate::frontend::core::parser::ast::{Type, Expr, Stmt, StmtKind, Param};
use crate::frontend::type_level::trait_bounds::TraitTable;

/// Derive 属性信息
#[derive(Debug, Clone)]
pub struct DeriveAttribute {
    pub traits: Vec<String>,
}

/// Derive 解析结果
#[derive(Debug, Clone)]
pub struct DeriveParseResult {
    pub derives: Vec<String>,
    pub remaining: Vec<super::super::core::lexer::tokens::Token>,
}

/// Derive 属性解析器
#[derive(Debug, Default)]
pub struct DeriveParser;

impl DeriveParser {
    /// 创建新的解析器
    pub fn new() -> Self {
        Self
    }

    /// 检查是否是 derive 属性
    pub fn is_derive_attr(tokens: &[super::super::core::lexer::tokens::Token]) -> bool {
        tokens.first().map(|t| {
            matches!(&t.kind, super::super::core::lexer::tokens::TokenKind::Identifier(n) if n == "derive")
        }).unwrap_or(false)
    }

    /// 简单解析 derive 属性
    /// 返回 derive 的 trait 列表
    pub fn parse_derive(
        tokens: &[super::super::core::lexer::tokens::Token]
    ) -> Option<Vec<String>> {
        let mut traits = Vec::new();
        let mut in_parens = false;
        let mut current = String::new();

        for token in tokens {
            match &token.kind {
                super::super::core::lexer::tokens::TokenKind::Identifier(name) => {
                    if in_parens {
                        if !current.is_empty() {
                            current.push(',');
                        }
                        current.push_str(name);
                    }
                }
                super::super::core::lexer::tokens::TokenKind::LParen => {
                    in_parens = true;
                    current.clear();
                }
                super::super::core::lexer::tokens::TokenKind::RParen => {
                    if !current.is_empty() {
                        traits.push(current.clone());
                    }
                    in_parens = false;
                    current.clear();
                }
                super::super::core::lexer::tokens::TokenKind::Comma if in_parens => {
                    if !current.is_empty() {
                        traits.push(current.clone());
                        current.clear();
                    }
                }
                _ => {}
            }
        }

        if traits.is_empty() && !current.is_empty() {
            traits.push(current);
        }

        if traits.is_empty() {
            None
        } else {
            Some(traits)
        }
    }
}

/// Derive 代码生成器
#[derive(Debug)]
pub struct DeriveGenerator<'a> {
    /// Trait 表
    trait_table: &'a TraitTable,
    /// 已知的派生 trait
    known_derives: HashMap<String, DeriveImpl>,
}

impl<'a> DeriveGenerator<'a> {
    /// 创建新的生成器
    pub fn new(trait_table: &'a TraitTable) -> Self {
        Self {
            trait_table,
            known_derives: Self::init_known_derives(),
        }
    }

    /// 初始化内置的派生实现
    fn init_known_derives() -> HashMap<String, DeriveImpl> {
        let mut derives = HashMap::new();
        derives.insert("Clone".to_string(), DeriveImpl::Clone);
        derives.insert("Copy".to_string(), DeriveImpl::Copy);
        derives
    }

    /// 生成 derive 实现
    pub fn generate_impls(
        &self,
        struct_name: &str,
        derive_traits: &[String],
        _fields: &[(String, Type)],
    ) -> Vec<Stmt> {
        let mut impl_stmts = Vec::new();

        for trait_name in derive_traits {
            if let Some(derive) = self.known_derives.get(trait_name) {
                let impl_stmt = derive.generate_impl(struct_name);
                impl_stmts.push(impl_stmt);
            }
        }

        impl_stmts
    }

    /// 检查 trait 是否可以被 derive
    pub fn can_derive(
        &self,
        trait_name: &str,
    ) -> bool {
        self.known_derives.contains_key(trait_name)
    }

    /// 获取支持的 derive trait 列表
    pub fn supported_traits(&self) -> Vec<&str> {
        self.known_derives.keys().map(|s| s.as_str()).collect()
    }
}

/// Derive 实现类型
#[derive(Debug)]
pub enum DeriveImpl {
    Clone,
    Copy,
}

impl DeriveImpl {
    /// 生成 impl 语句
    fn generate_impl(
        &self,
        struct_name: &str,
    ) -> Stmt {
        let (trait_name, method) = match self {
            Self::Clone => ("Clone".to_string(), self.generate_clone_method(struct_name)),
            Self::Copy => ("Copy".to_string(), self.generate_copy_method(struct_name)),
        };

        Stmt {
            kind: StmtKind::TraitImpl(crate::frontend::core::parser::ast::TraitImpl {
                trait_name,
                for_type: Type::Name(struct_name.to_string()),
                methods: vec![method],
                span: crate::util::span::Span::dummy(),
            }),
            span: crate::util::span::Span::dummy(),
        }
    }

    /// 生成 Clone 方法
    fn generate_clone_method(
        &self,
        struct_name: &str,
    ) -> crate::frontend::core::parser::ast::MethodImpl {
        // 构造新实例: StructName { x: self.x, y: self.y, ... }
        // 这里简化处理：假设字段名就是变量名
        let struct_init = Expr::Var(struct_name.to_string(), crate::util::span::Span::dummy());

        crate::frontend::core::parser::ast::MethodImpl {
            name: "clone".to_string(),
            params: vec![Param {
                name: "self".to_string(),
                ty: Some(Type::Name("Self".to_string())),
                span: crate::util::span::Span::dummy(),
            }],
            return_type: Some(Type::Name(struct_name.to_string())),
            body: (vec![], Some(Box::new(struct_init))),
            span: crate::util::span::Span::dummy(),
        }
    }

    /// 生成 Copy 方法
    fn generate_copy_method(
        &self,
        _struct_name: &str,
    ) -> crate::frontend::core::parser::ast::MethodImpl {
        // 返回 self
        let self_var = Expr::Var("self".to_string(), crate::util::span::Span::dummy());

        crate::frontend::core::parser::ast::MethodImpl {
            name: "copy".to_string(),
            params: vec![Param {
                name: "self".to_string(),
                ty: Some(Type::Name("Self".to_string())),
                span: crate::util::span::Span::dummy(),
            }],
            return_type: Some(Type::Name("Self".to_string())),
            body: (vec![], Some(Box::new(self_var))),
            span: crate::util::span::Span::dummy(),
        }
    }
}
