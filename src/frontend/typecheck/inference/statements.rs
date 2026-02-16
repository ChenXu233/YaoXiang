#![allow(clippy::result_large_err)]

//! 语句类型推断
//!
//! 实现各种语句的类型推断

use crate::util::diagnostic::Result;
use crate::frontend::core::parser::ast;
use crate::frontend::core::type_system::MonoType;
use std::collections::HashMap;

/// 语句类型推断器
pub struct StmtInferrer {
    /// 变量作用域栈
    scopes: Vec<HashMap<String, MonoType>>,
    next_type_var: usize,
}

impl Default for StmtInferrer {
    fn default() -> Self {
        Self::new()
    }
}

impl StmtInferrer {
    /// 创建新的语句推断器
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()], // 全局作用域
            next_type_var: 0,
        }
    }

    fn fresh_type_var(&mut self) -> MonoType {
        let var = crate::frontend::core::type_system::var::TypeVar::new(self.next_type_var);
        self.next_type_var += 1;
        MonoType::TypeVar(var)
    }

    /// 进入新的作用域
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// 退出作用域
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// 添加变量到当前作用域
    pub fn add_var(
        &mut self,
        name: String,
        ty: MonoType,
    ) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    /// 获取变量
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&MonoType> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }

    /// 推断声明语句类型
    pub fn infer_declaration(
        &mut self,
        name: &str,
        ty: Option<MonoType>,
    ) -> Result<MonoType> {
        let var_type = match ty {
            Some(t) => t,
            None => {
                // 如果没有类型注解，创建类型变量
                self.fresh_type_var()
            }
        };

        self.add_var(name.to_string(), var_type.clone());
        Ok(var_type)
    }

    /// 推断赋值语句类型
    pub fn infer_assignment(
        &mut self,
        _lhs: &ast::Expr,
        _rhs: &ast::Expr,
    ) -> Result<MonoType> {
        // 赋值语句返回void类型
        Ok(MonoType::Void)
    }

    /// 推断块语句类型
    pub fn infer_block(
        &mut self,
        block: &ast::Block,
    ) -> Result<MonoType> {
        // 进入块作用域
        self.enter_scope();

        // 推断块中的所有语句
        for stmt in &block.stmts {
            // TODO: 实现语句推断
            let _ = stmt;
        }

        // 推断块表达式
        let block_type = if let Some(_expr) = &block.expr {
            // TODO: 推断表达式类型
            MonoType::Void
        } else {
            MonoType::Void
        };

        // 退出块作用域
        self.exit_scope();

        Ok(block_type)
    }

    /// 推断语句类型
    pub fn infer_stmt(
        &mut self,
        stmt: &ast::Stmt,
    ) -> Result<MonoType> {
        match &stmt.kind {
            ast::StmtKind::Expr(_expr) => {
                // TODO: 推断表达式类型
                Ok(MonoType::Void)
            }
            ast::StmtKind::Fn {
                name,
                generic_params: _,
                type_annotation,
                params: _,
                body: _,
                is_pub: _,
            } => {
                // TODO: 推断函数类型
                let fn_type = type_annotation
                    .as_ref()
                    .map(|t| MonoType::from(t.clone()))
                    .unwrap_or_else(|| MonoType::Void);

                self.add_var(name.clone(), fn_type);
                Ok(MonoType::Void)
            }
            _ => Ok(MonoType::Void),
        }
    }
}
