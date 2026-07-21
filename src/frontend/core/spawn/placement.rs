//! `spawn` placement checker.
//!
//! Recursively walks the AST to find `spawn` expressions and report diagnostics.

use crate::frontend::core::parser::ast::{Block, Expr, FStringSegment, Module, Stmt, StmtKind};
use crate::util::diagnostic::Diagnostic;

pub fn check_spawn_placement(module: &Module) -> Vec<Diagnostic> {
    let mut checker = SpawnPlacementChecker::new();
    checker.check_module(module);
    checker.errors
}

#[derive(Debug, Default)]
struct SpawnPlacementChecker {
    errors: Vec<Diagnostic>,
}

impl SpawnPlacementChecker {
    fn new() -> Self {
        Self::default()
    }

    fn check_module(
        &mut self,
        module: &Module,
    ) {
        for stmt in &module.items {
            self.check_stmt(stmt);
        }
    }

    fn check_block(
        &mut self,
        block: &Block,
    ) {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
    }

    fn check_stmt(
        &mut self,
        stmt: &Stmt,
    ) {
        match &stmt.kind {
            StmtKind::Expr(expr) => self.check_expr(expr),
            StmtKind::Var { initializer, .. } => {
                if let Some(init) = initializer {
                    self.check_expr(init);
                }
            }
            StmtKind::For { iterable, body, .. } => {
                self.check_expr(iterable);
                self.check_block(body);
            }
            StmtKind::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.check_expr(condition);
                self.check_block(then_branch);
                for (elif_cond, elif_body) in elif_branches {
                    self.check_expr(elif_cond);
                    self.check_block(elif_body);
                }
                if let Some(else_body) = else_branch {
                    self.check_block(else_body);
                }
            }
            StmtKind::Binding { body, .. } => {
                for s in body {
                    self.check_stmt(s);
                }
            }
            StmtKind::DestructureAssign { rhs, .. } => {
                self.check_expr(rhs);
            }
            StmtKind::TypeDefinition { .. } => {}
            StmtKind::Use { .. } | StmtKind::ExternalBindingStmt { .. } | StmtKind::Error(_) => {}
            StmtKind::Return(expr_opt) => {
                if let Some(expr) = expr_opt {
                    self.check_expr(expr);
                }
            }
        }
    }

    fn check_expr(
        &mut self,
        expr: &Expr,
    ) {
        match expr {
            Expr::Lit(..) | Expr::Var(..) | Expr::Break(..) | Expr::Continue(..) => {}

            Expr::BinOp { left, right, .. } => {
                self.check_expr(left);
                self.check_expr(right);
            }
            Expr::UnOp { expr, .. } => self.check_expr(expr),
            Expr::Call {
                func,
                args,
                named_args,
                ..
            } => {
                self.check_expr(func);
                for a in args {
                    self.check_expr(a);
                }
                for (_name, val) in named_args {
                    self.check_expr(val);
                }
            }
            Expr::FnDef { body, .. } => self.check_block(body),
            Expr::Lambda { body, .. } => self.check_block(body),
            Expr::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.check_expr(condition);
                self.check_block(then_branch);
                for (elif_cond, elif_body) in elif_branches {
                    self.check_expr(elif_cond);
                    self.check_block(elif_body);
                }
                if let Some(else_body) = else_branch {
                    self.check_block(else_body);
                }
            }
            Expr::Match { expr, arms, .. } => {
                self.check_expr(expr);
                for arm in arms {
                    self.check_block(&arm.body);
                }
            }
            Expr::While {
                condition, body, ..
            } => {
                self.check_expr(condition);
                self.check_block(body);
            }
            Expr::For { iterable, body, .. } => {
                self.check_expr(iterable);
                self.check_block(body);
            }
            Expr::Block(block) => self.check_block(block),
            Expr::Return(expr_opt, ..) => {
                if let Some(e) = expr_opt {
                    self.check_expr(e);
                }
            }
            Expr::Cast { expr, .. } => self.check_expr(expr),
            Expr::Tuple(elems, ..) | Expr::List(elems, ..) => {
                for e in elems {
                    self.check_expr(e);
                }
            }
            Expr::ListComp {
                element,
                iterable,
                condition,
                ..
            } => {
                self.check_expr(element);
                self.check_expr(iterable);
                if let Some(cond) = condition {
                    self.check_expr(cond);
                }
            }
            Expr::Dict(pairs, ..) => {
                for (k, v) in pairs {
                    self.check_expr(k);
                    self.check_expr(v);
                }
            }
            Expr::Index { expr, index, .. } => {
                self.check_expr(expr);
                self.check_expr(index);
            }
            Expr::FieldAccess { expr, .. } => self.check_expr(expr),
            Expr::Try { expr, .. } => self.check_expr(expr),
            Expr::Ref { expr, .. } => self.check_expr(expr),
            Expr::Unsafe { body, .. } => self.check_block(body),

            Expr::Spawn { body, .. } => {
                self.check_block(body);
            }

            Expr::FString { segments, .. } => {
                for seg in segments {
                    if let FStringSegment::Interpolation { expr, .. } = seg {
                        self.check_expr(expr);
                    }
                }
            }

            Expr::Error(_) => {}
            Expr::Borrow { expr, .. } => self.check_expr(expr),
            Expr::SpawnFor { body, iterable, .. } => {
                self.check_expr(iterable);
                self.check_block(body);
            }
        }
    }
}
