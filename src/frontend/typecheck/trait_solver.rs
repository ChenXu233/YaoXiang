//! Trait约束求解器（RFC-011）
//!
//! 实现trait约束的证明算法，确保类型满足trait约束

use crate::frontend::parser::ast;
use crate::frontend::typecheck::traits::{TraitDef, TraitEnvironment, TraitImpl, TraitRef};
use crate::util::span::Span;
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Trait约束
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitConstraint {
    /// 被约束的类型
    pub ty: String,
    /// Trait引用
    pub trait_ref: TraitRef,
    /// 位置信息
    pub span: Span,
}

/// Trait约束求解错误
#[derive(Debug, Clone)]
pub enum TraitError {
    /// 找不到trait定义
    TraitNotFound {
        trait_name: String,
        span: Span,
    },
    /// 找不到impl
    ImplNotFound {
        ty: String,
        trait_name: String,
        span: Span,
    },
    /// 约束无法满足
    CannotSatisfy {
        constraint: TraitConstraint,
        reason: String,
        span: Span,
    },
    /// 循环约束
    CycleDetected {
        constraints: Vec<TraitConstraint>,
        span: Span,
    },
}

/// Trait义务（需要证明的约束）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitObligation {
    /// 被约束的类型
    pub ty: String,
    /// Trait引用
    pub trait_ref: TraitRef,
    /// 位置信息
    pub span: Span,
}

/// Trait约束求解器
pub struct TraitSolver {
    /// 已证明的义务
    proven: HashSet<TraitObligation>,
    /// 正在证明的义务（用于循环检测）
    in_progress: HashSet<TraitObligation>,
    /// 环境
    env: TraitEnvironment,
}

impl TraitSolver {
    /// 创建新的求解器
    pub fn new(env: TraitEnvironment) -> Self {
        TraitSolver {
            proven: HashSet::new(),
            in_progress: HashSet::new(),
            env,
        }
    }

    /// 证明一个义务
    pub fn prove_obligation(
        &mut self,
        obligation: TraitObligation,
    ) -> Result<(), TraitError> {
        // 检查是否已经证明
        if self.proven.contains(&obligation) {
            return Ok(());
        }

        // 检查是否正在证明（循环检测）
        if self.in_progress.contains(&obligation) {
            return Err(TraitError::CycleDetected {
                constraints: vec![obligation.clone()],
                span: obligation.span,
            });
        }

        // 标记为正在证明
        self.in_progress.insert(obligation.clone());

        // 1. 查找trait定义
        let trait_def = self.env.get_trait(&obligation.trait_ref.name)
            .ok_or_else(|| TraitError::TraitNotFound {
                trait_name: obligation.trait_ref.name.clone(),
                span: obligation.span,
            })?;

        // 2. 尝试从impl证明
        if let Some(impls) = self.env.get_impl(&obligation.trait_ref.name, &obligation.ty) {
            for impl_ in impls {
                if self.validate_impl(impl_, &obligation)? {
                    self.proven.insert(obligation);
                    self.in_progress.remove(&obligation);
                    return Ok(());
                }
            }
        }

        // 3. 尝试从supertrait证明
        if !trait_def.super_traits.is_empty() {
            for super_trait in &trait_def.super_traits {
                let super_obligation = TraitObligation {
                    ty: obligation.ty.clone(),
                    trait_ref: super_trait.clone(),
                    span: obligation.span,
                };

                match self.prove_obligation(super_obligation) {
                    Ok(_) => {
                        self.proven.insert(obligation);
                        self.in_progress.remove(&obligation);
                        return Ok(());
                    }
                    Err(e) => {
                        // 继续尝试其他supertrait
                        continue;
                    }
                }
            }
        }

        // 无法证明
        self.in_progress.remove(&obligation);
        Err(TraitError::ImplNotFound {
            ty: obligation.ty,
            trait_name: obligation.trait_ref.name,
            span: obligation.span,
        })
    }

    /// 验证impl是否满足约束
    fn validate_impl(
        &self,
        impl_: &TraitImpl,
        obligation: &TraitObligation,
    ) -> Result<bool, TraitError> {
        // 检查impl是否匹配义务
        if impl_.trait_ref.name != obligation.trait_ref.name {
            return Ok(false);
        }

        if impl_.for_type != obligation.ty {
            return Ok(false);
        }

        // 检查泛型参数是否匹配
        if impl_.trait_ref.args.len() != obligation.trait_ref.args.len() {
            return Ok(false);
        }

        // TODO: 检查泛型参数是否满足约束
        // 这里需要更复杂的类型检查

        Ok(true)
    }

    /// 批量证明义务
    pub fn prove_all(
        &mut self,
        obligations: Vec<TraitObligation>,
    ) -> Result<(), TraitError> {
        for obligation in obligations {
            self.prove_obligation(obligation)?;
        }
        Ok(())
    }

    /// 检查是否已证明
    pub fn is_proven(&self, obligation: &TraitObligation) -> bool {
        self.proven.contains(obligation)
    }

    /// 从AST中收集trait约束
    pub fn collect_constraints_from_ast(
        &self,
        ast_item: &ast::Item,
    ) -> Vec<TraitConstraint> {
        let mut constraints = Vec::new();

        match ast_item {
            ast::Item::Fn { params, .. } => {
                for param in params {
                    if let Some(ref ty) = param.ty {
                        self.collect_constraints_from_type(ty, &mut constraints);
                    }
                }
            }
            ast::Item::Type { ty, .. } => {
                if let Some(ref type_annotation) = ty {
                    self.collect_constraints_from_type(type_annotation, &mut constraints);
                }
            }
            _ => {}
        }

        constraints
    }

    /// 从类型中收集约束
    fn collect_constraints_from_type(
        &self,
        ty: &ast::Type,
        constraints: &mut Vec<TraitConstraint>,
    ) {
        match ty {
            ast::Type::GenericParam { name, bounds } => {
                for bound in bounds {
                    constraints.push(TraitConstraint {
                        ty: name.clone(),
                        trait_ref: TraitRef {
                            name: bound.clone(),
                            args: Vec::new(),
                        },
                        span: Span::dummy(), // TODO: 需要真实的span
                    });
                }
            }
            ast::Type::GenericParams(params) => {
                for param in params {
                    for bound in &param.bounds {
                        constraints.push(TraitConstraint {
                            ty: param.name.clone(),
                            trait_ref: TraitRef {
                                name: bound.clone(),
                                args: Vec::new(),
                            },
                            span: Span::dummy(), // TODO: 需要真实的span
                        });
                    }
                }
            }
            _ => {}
        }
    }
}

impl fmt::Display for TraitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TraitError::TraitNotFound { trait_name, .. } => {
                write!(f, "trait '{}' not found", trait_name)
            }
            TraitError::ImplNotFound { ty, trait_name, .. } => {
                write!(f, "impl of trait '{}' for type '{}' not found", trait_name, ty)
            }
            TraitError::CannotSatisfy { reason, .. } => {
                write!(f, "cannot satisfy constraint: {}", reason)
            }
            TraitError::CycleDetected { .. } => {
                write!(f, "cycle detected in trait constraints")
            }
        }
    }
}
