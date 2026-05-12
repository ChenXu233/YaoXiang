#![allow(clippy::result_large_err)]

//! 语句检查器
//!
//! 合并原 checking/BodyChecker 和 inference/StmtInferrer
//! 使用统一的 ScopeManager 管理变量作用域

use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

use std::collections::HashMap;
use crate::frontend::module::{Export, ExportKind, ModuleInfo};
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::core::types::base::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::core::parser::ast::{Block, Expr, Param, Stmt};

use super::scope::ScopeManager;

/// 语句检查器
///
/// 负责检查函数体中的语句和表达式的类型正确性。
/// 使用统一的 ScopeManager 实现作用域管理。
///
/// ## 错误收集模式
///
/// 支持两种错误处理模式：
/// - **短路模式**（默认）：遇到错误立即返回，保持向后兼容
/// - **收集模式**：收集所有错误后统一返回，用于 LSP 支持
///
/// 通过 `set_collect_all_errors(true)` 启用收集模式。
pub struct StatementChecker {
    /// 约束求解器
    solver: TypeConstraintSolver,
    /// 统一作用域管理器
    scope: ScopeManager,
    /// 已检查的函数
    checked_functions: HashMap<String, bool>,
    /// 重载候选存储
    overload_candidates:
        HashMap<String, Vec<crate::frontend::core::typecheck::overload::OverloadCandidate>>,
    /// Native 函数签名表
    native_signatures: HashMap<String, MonoType>,
    /// 模块注册表（用于在函数体/块作用域中处理 use 语句）
    module_registry: ModuleRegistry,
    /// 是否在顶层作用域（模块级，非函数内部）
    is_top_level: bool,
    /// 累积的错误（收集模式下使用）
    collected_errors: Vec<Diagnostic>,
    /// 是否启用错误收集模式（收集所有错误而非短路返回）
    collect_all_errors: bool,
    /// 保存函数体的变量（在退出函数作用域后保留）
    function_local_vars: HashMap<String, PolyType>,
    /// 当前函数的 Result 错误类型栈（用于 `?` 运算符约束）
    result_err_stack: Vec<Option<MonoType>>,
}

impl StatementChecker {
    /// 创建新的语句检查器
    pub fn new(solver: &mut TypeConstraintSolver) -> Self {
        Self {
            solver: solver.clone(),
            scope: ScopeManager::new(),
            checked_functions: HashMap::new(),
            overload_candidates: HashMap::new(),
            native_signatures: HashMap::new(),
            module_registry: ModuleRegistry::with_std(),
            is_top_level: true,
            collected_errors: Vec::new(),
            collect_all_errors: false,
            function_local_vars: HashMap::new(),
            result_err_stack: Vec::new(),
        }
    }

    fn current_result_err(&self) -> Option<MonoType> {
        self.result_err_stack.last().cloned().flatten()
    }

    /// 设置是否启用错误收集模式
    ///
    /// 启用后，类型检查不会在遇到第一个错误时短路返回，
    /// 而是尽可能多地收集错误，最终统一返回。
    /// 这对于 LSP 诊断非常重要，因为用户需要看到所有错误。
    pub fn set_collect_all_errors(
        &mut self,
        collect: bool,
    ) {
        self.collect_all_errors = collect;
    }

    /// 获取是否启用了错误收集模式
    pub fn is_collect_all_errors(&self) -> bool {
        self.collect_all_errors
    }

    /// 获取累积的错误
    pub fn collected_errors(&self) -> &[Diagnostic] {
        &self.collected_errors
    }

    /// 取出所有累积的错误（消耗）
    pub fn drain_collected_errors(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.collected_errors)
    }

    /// 检查是否有累积的错误
    pub fn has_collected_errors(&self) -> bool {
        !self.collected_errors.is_empty()
    }

    /// 收集错误（在收集模式下添加到列表，否则忽略）
    fn collect_error(
        &mut self,
        error: Diagnostic,
    ) {
        self.collected_errors.push(error);
    }

    /// 设置 native 函数签名表
    pub fn set_native_signatures(
        &mut self,
        signatures: HashMap<String, MonoType>,
    ) {
        self.native_signatures = signatures;
    }

    /// 设置模块注册表
    pub fn set_module_registry(
        &mut self,
        registry: ModuleRegistry,
    ) {
        self.module_registry = registry;
    }

    fn default_callable_type(&mut self) -> MonoType {
        MonoType::Fn {
            params: vec![self.solver.new_var()],
            return_type: Box::new(self.solver.new_var()),
            is_async: false,
        }
    }

    fn export_type(
        &mut self,
        export: &Export,
    ) -> MonoType {
        match export.kind {
            ExportKind::SubModule => {
                if let Some(sub_module) = self.module_registry.get(&export.full_path).cloned() {
                    return self.module_as_struct_type(&sub_module, &export.name);
                }
                self.default_callable_type()
            }
            _ => self
                .native_signatures
                .get(&export.full_path)
                .cloned()
                .or_else(|| self.native_signatures.get(&export.name).cloned())
                .unwrap_or_else(|| self.default_callable_type()),
        }
    }

    fn module_as_struct_type(
        &mut self,
        module: &ModuleInfo,
        name: &str,
    ) -> MonoType {
        let mut fields = Vec::new();
        for export in module.exports.values() {
            fields.push((export.name.clone(), self.export_type(export)));
        }

        MonoType::Struct(crate::frontend::core::types::base::mono::StructType {
            name: name.to_string(),
            fields,
            methods: HashMap::new(),
            field_mutability: Vec::new(),
            field_has_default: Vec::new(),
            interfaces: vec![],
        })
    }

    fn import_binding(
        &mut self,
        binding_name: &str,
        export: &Export,
    ) {
        let ty = self.export_type(export);
        self.scope
            .add_var(binding_name.to_string(), PolyType::mono(ty));
    }

    fn process_use_stmt(
        &mut self,
        path: &str,
        items: &Option<Vec<String>>,
        alias: &Option<Vec<String>>,
    ) {
        let Some(module) = self.module_registry.get(path).cloned() else {
            return;
        };

        let selected_exports: Vec<Export> = match items {
            Some(item_names) => item_names
                .iter()
                .filter_map(|item| module.exports.get(item).cloned())
                .collect(),
            None => module.exports.values().cloned().collect(),
        };

        match (items.as_ref(), alias.as_ref()) {
            // use path
            (None, None) => {
                let module_alias = path.split('.').next_back().unwrap_or(path);
                let module_ty = self.module_as_struct_type(&module, module_alias);
                self.scope
                    .add_var(module_alias.to_string(), PolyType::mono(module_ty));
            }
            // use path as alias
            (None, Some(aliases)) if aliases.len() == 1 => {
                let module_alias = &aliases[0];
                let module_ty = self.module_as_struct_type(&module, module_alias);
                self.scope
                    .add_var(module_alias.to_string(), PolyType::mono(module_ty));
            }
            // use path.{a, b}
            (Some(item_names), None) => {
                for item_name in item_names {
                    if let Some(export) = module.exports.get(item_name).cloned() {
                        self.import_binding(item_name, &export);
                    }
                }
            }
            // use path.{a, b} as aa, bb
            (Some(item_names), Some(aliases)) if item_names.len() == aliases.len() => {
                for (item_name, alias_name) in item_names.iter().zip(aliases.iter()) {
                    if let Some(export) = module.exports.get(item_name).cloned() {
                        self.import_binding(alias_name, &export);
                    }
                }
            }
            // 不合法别名数量：按原名导入
            (Some(item_names), Some(_)) => {
                for item_name in item_names {
                    if let Some(export) = module.exports.get(item_name).cloned() {
                        self.import_binding(item_name, &export);
                    }
                }
            }
            _ => {
                for export in selected_exports {
                    self.import_binding(&export.name, &export);
                }
            }
        }
    }

    /// 获取求解器
    pub fn solver(&mut self) -> &mut TypeConstraintSolver {
        &mut self.solver
    }

    /// 添加变量到当前作用域
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.scope.add_var(name, poly);
    }

    /// 获取变量（从最内层作用域开始查找）
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.scope.get_var(name)
    }

    /// 获取所有变量（从所有作用域，内层覆盖外层）
    /// 包含函数体变量（在退出作用域后保留）
    pub fn vars(&self) -> HashMap<String, PolyType> {
        let mut result = self.scope.vars();
        // 合并退出作用域前保存的变量
        for (name, poly) in &self.function_local_vars {
            if !result.contains_key(name) {
                result.insert(name.clone(), poly.clone());
            }
        }
        result
    }

    /// 检查变量是否存在于任何作用域中
    pub fn var_exists_in_any_scope(
        &self,
        name: &str,
    ) -> bool {
        self.scope.var_in_any_scope(name)
    }

    /// 检查变量是否存在于当前作用域
    pub fn var_exists_in_current_scope(
        &self,
        name: &str,
    ) -> bool {
        self.scope.var_in_current_scope(name)
    }

    /// 变量赋值操作 - 统一处理所有作用域的变量赋值
    ///
    /// 统一变量类型并写回 scope，确保后续类型推断能获取最新类型。
    /// 修复了之前在当前作用域赋值时未写回导致 for 循环等场景类型丢失的问题。
    /// 关键：直接使用右侧表达式的类型（new_ty），而不是依赖 solver.resolve()。
    fn assign_var(
        &mut self,
        name: &str,
        new_ty: MonoType,
    ) {
        // 直接使用右侧表达式的类型更新变量
        // 注意：new_ty 已经是解析后的正确类型（如 List<Int>），不需要额外 resolve
        self.scope.update_var(name, PolyType::mono(new_ty));
    }

    /// 进入新的作用域
    pub fn enter_scope(&mut self) {
        self.scope.enter_scope();
    }

    /// 退出当前作用域
    pub fn exit_scope(&mut self) {
        self.scope.exit_scope();
    }

    /// 检查函数定义
    ///
    /// 在收集模式下，遇到错误不会短路返回，而是继续检查后续语句，
    /// 尽可能收集所有错误。
    pub fn check_fn_def(
        &mut self,
        name: &str,
        params: &[Param],
        body: &Block,
    ) -> Result<(), Box<Diagnostic>> {
        // 检查是否已经检查过
        if self.checked_functions.contains_key(name) {
            return Ok(());
        }

        // 标记为已检查
        self.checked_functions.insert(name.to_string(), true);

        // 保存当前顶层状态，进入函数后不再是顶层
        let was_top_level = self.is_top_level;
        self.is_top_level = false;

        // 创建函数作用域
        self.scope.enter_scope();

        // 添加参数到函数作用域
        for param in params {
            let param_ty = param
                .ty
                .as_ref()
                .map(|t| MonoType::from(t.clone()))
                .unwrap_or_else(|| self.solver.new_var());
            self.scope
                .add_var(param.name.clone(), PolyType::mono(param_ty));
        }

        if self.collect_all_errors {
            // 收集模式：收集所有错误，不短路
            let mut first_err = None;
            for stmt in &body.stmts {
                if let Err(e) = self.check_stmt(stmt) {
                    if first_err.is_none() {
                        first_err = Some(e.clone());
                    }
                    self.collect_error(*e);
                }
            }

            // 检查返回表达式
            if let Some(expr) = &body.expr {
                if let Err(e) = self.check_expr(expr) {
                    if first_err.is_none() {
                        first_err = Some(e.clone());
                    }
                    self.collect_error(*e);
                }
            }

            // 退出函数作用域前，保存所有变量（解决退出作用域后变量丢失的问题）
            for (name, poly) in self.scope.vars() {
                self.function_local_vars.insert(name, poly);
            }

            // 退出函数作用域
            self.scope.exit_scope();
            self.is_top_level = was_top_level;

            match first_err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        } else {
            // 短路模式：遇到第一个错误立即返回
            let mut err = None;
            for stmt in &body.stmts {
                if let Err(e) = self.check_stmt(stmt) {
                    err = Some(e);
                    break;
                }
            }

            // 检查返回表达式
            if err.is_none() {
                if let Some(expr) = &body.expr {
                    if let Err(e) = self.check_expr(expr) {
                        err = Some(e);
                    }
                }
            }

            // 退出函数作用域前，保存所有变量（解决退出作用域后变量丢失的问题）
            for (name, poly) in self.scope.vars() {
                self.function_local_vars.insert(name, poly);
            }

            // 退出函数作用域
            self.scope.exit_scope();
            self.is_top_level = was_top_level;

            match err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
    }

    /// 检查语句
    pub fn check_stmt(
        &mut self,
        stmt: &Stmt,
    ) -> Result<(), Box<Diagnostic>> {
        match &stmt.kind {
            crate::frontend::core::parser::ast::StmtKind::Expr(expr) => self.check_expr_stmt(expr),
            crate::frontend::core::parser::ast::StmtKind::Binding {
                name,
                type_name,
                generic_params: _,
                type_annotation,
                eval: _,
                params,
                body: (stmts, expr),
                is_pub: _,
                method_type,
            } => {
                // 根据是否有 type_name 来区分方法绑定和其他绑定
                // 注意：不能根据 params 是否为空来判断，因为空参数的函数也是函数
                let body_block = Block {
                    stmts: stmts.to_vec(),
                    expr: expr.clone(),
                    span: stmt.span,
                };
                if type_name.is_some() {
                    // 方法绑定：使用 method_type 作为签名
                    // method_type 包含完整的 (params) -> ReturnType 签名
                    let type_ann = method_type.as_ref();
                    self.check_fn_stmt(name, type_ann, params, stmts, body_block, stmt.span)
                } else {
                    // 函数绑定（包括空参数的函数）
                    // 使用 type_annotation 作为签名
                    self.check_fn_stmt(
                        name,
                        type_annotation.as_ref(),
                        params,
                        stmts,
                        body_block,
                        stmt.span,
                    )
                }
            }
            crate::frontend::core::parser::ast::StmtKind::Var {
                name,
                type_annotation,
                initializer,
                ..
            } => self.check_var_stmt(
                name,
                type_annotation.as_ref(),
                None,
                &[],
                initializer.as_deref(),
            ),
            crate::frontend::core::parser::ast::StmtKind::For {
                var,
                var_mut,
                iterable,
                body,
                ..
            } => self.check_for_stmt(var, *var_mut, iterable, body),
            crate::frontend::core::parser::ast::StmtKind::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                span,
            } => {
                let elif_refs: Vec<(&Expr, &Block)> = elif_branches
                    .iter()
                    .map(|(e, b)| (e.as_ref(), b.as_ref()))
                    .collect();
                self.check_if_stmt(
                    condition,
                    then_branch,
                    &elif_refs,
                    else_branch.as_deref(),
                    *span,
                )
            }
            crate::frontend::core::parser::ast::StmtKind::Use {
                path, items, alias, ..
            } => {
                self.process_use_stmt(path, items, alias);
                Ok(())
            }
            // 错误恢复占位符：报告错误但不 panic
            crate::frontend::core::parser::ast::StmtKind::Error(span) => Err(Box::new(
                ErrorCodeDefinition::invalid_syntax("缺失语句")
                    .at(*span)
                    .build(),
            )),
            _ => Ok(()),
        }
    }
    /// 检查表达式语句
    fn check_expr_stmt(
        &mut self,
        expr: &Expr,
    ) -> Result<(), Box<Diagnostic>> {
        match expr {
            Expr::FnDef {
                name, params, body, ..
            } => {
                self.check_fn_def(name, params, body)?;
                Ok(())
            }
            Expr::BinOp {
                op: crate::frontend::core::parser::ast::BinOp::Assign,
                left,
                right,
                ..
            } => {
                let right_ty = self.check_expr(right)?;

                if let Expr::Var(name, _) = left.as_ref() {
                    if self.scope.var_in_any_scope(name) {
                        // 统一变量类型并写回 scope，确保后续类型推断正确
                        self.assign_var(name, right_ty);
                    } else {
                        // 新变量：创建类型变量并统一
                        let ty = self.solver.new_var();
                        let _ = self.solver.unify(&ty, &right_ty);
                        self.scope.add_var(name.clone(), PolyType::mono(ty));
                    }
                }
                Ok(())
            }
            _ => {
                self.check_expr(expr)?;
                Ok(())
            }
        }
    }

    /// 检查函数语句
    fn check_fn_stmt(
        &mut self,
        name: &str,
        type_annotation: Option<&crate::frontend::core::parser::ast::Type>,
        params: &[Param],
        _stmts: &[Stmt],
        body: Block,
        _span: crate::util::span::Span,
    ) -> Result<(), Box<Diagnostic>> {
        // 检查是否与结构体重名
        if let Some(existing) = self.scope.get_var(name) {
            if let MonoType::Struct(_) = &existing.body {
                return Err(Box::new(
                    ErrorCodeDefinition::duplicate_definition(name)
                        .at(_span)
                        .build(),
                ));
            }
        }

        // 将函数自身注册到变量环境中
        if let Some(type_ann) = type_annotation {
            if let crate::frontend::core::parser::ast::Type::Fn {
                params: param_types,
                return_type,
            } = type_ann
            {
                let fn_param_types: Vec<MonoType> = param_types
                    .iter()
                    .map(|t| MonoType::from(t.clone()))
                    .collect();
                let fn_return_type = MonoType::from(*return_type.clone());
                let fn_type = MonoType::Fn {
                    params: fn_param_types,
                    return_type: Box::new(fn_return_type),
                    is_async: false,
                };
                self.scope
                    .add_var(name.to_string(), PolyType::mono(fn_type));
            }
        } else {
            let param_types: Vec<MonoType> = params
                .iter()
                .map(|p| {
                    p.ty.as_ref()
                        .map(|t| MonoType::from(t.clone()))
                        .unwrap_or_else(|| self.solver.new_var())
                })
                .collect();

            let fn_type = MonoType::Fn {
                params: param_types,
                return_type: Box::new(self.solver.new_var()),
                is_async: false,
            };
            self.scope
                .add_var(name.to_string(), PolyType::mono(fn_type));
        }

        // 进入函数 Result 上下文（用于 `?` 运算符检查）
        let fn_result_err = type_annotation.and_then(|t| match t {
            crate::frontend::core::parser::ast::Type::Fn { return_type, .. } => {
                let ret_mono = MonoType::from((**return_type).clone());
                match ret_mono {
                    MonoType::Result(_, err) => Some((*err).clone()),
                    _ => None,
                }
            }
            _ => None,
        });
        self.result_err_stack.push(fn_result_err);

        // 处理类型注解
        if let Some(crate::frontend::core::parser::ast::Type::Fn { return_type, .. }) =
            type_annotation
        {
            let fn_def_expr = Expr::FnDef {
                name: name.to_string(),
                params: params.to_vec(),
                return_type: Some(*return_type.clone()),
                body: Box::new(body.clone()),
                is_async: false,
                span: _span,
            };
            let _ = self.check_expr(&fn_def_expr);
        }

        let out = self.check_fn_def(name, params, &body);

        // 退出函数 Result 上下文
        let _ = self.result_err_stack.pop();

        out
    }

    /// 检查变量语句
    ///
    /// 处理 Binding 类型的变量声明，支持编译期求值标记（eval）。
    fn check_var_stmt(
        &mut self,
        name: &str,
        type_annotation: Option<&crate::frontend::core::parser::ast::Type>,
        eval: Option<crate::frontend::core::parser::ast::EvalMode>,
        prelude_stmts: &[Stmt],
        initializer: Option<&Expr>,
    ) -> Result<(), Box<Diagnostic>> {
        // 处理 prelude 语句（编译期求值部分）
        for stmt in prelude_stmts {
            self.check_stmt(stmt)?;
        }

        let ty = match (initializer, type_annotation) {
            (Some(init_expr), Some(type_ann)) => {
                let init_ty = self.check_expr(init_expr)?;
                let ann_ty = MonoType::from(type_ann.clone());
                let _ = self.solver.unify(&init_ty, &ann_ty);
                ann_ty
            }
            (Some(init_expr), None) => self.check_expr(init_expr)?,
            (None, Some(type_ann)) => MonoType::from(type_ann.clone()),
            (None, None) => self.solver.new_var(),
        };

        // 编译期求值标记：Eager 模式立即求值，Block 模式延迟，Auto 根据上下文推断
        // 目前 eval 字段主要用于语义高亮和 IDE 支持，求值逻辑在 IR 生成阶段处理
        let _ = eval;

        if self.scope.var_in_current_scope(name) {
            // 统一变量类型并写回 scope，确保后续类型推断正确
            self.assign_var(name, ty);
            return Ok(());
        }

        if self.scope.var_in_any_scope(name) {
            self.assign_var(name, ty);
            return Ok(());
        }

        self.scope.add_var(name.to_string(), PolyType::mono(ty));
        Ok(())
    }

    /// 检查 for 语句
    ///
    /// 在收集模式下，循环体内的错误会被收集而非短路。
    fn check_for_stmt(
        &mut self,
        var: &str,
        var_mut: bool,
        iterable: &Expr,
        body: &Block,
    ) -> Result<(), Box<Diagnostic>> {
        let iter_ty = self.check_expr(iterable)?;
        let elem_ty = match iter_ty {
            MonoType::List(elem) => *elem,
            MonoType::Range { elem_type } => *elem_type,
            MonoType::String => MonoType::Char,
            MonoType::Dict(key_ty, value_ty) => MonoType::Tuple(vec![*key_ty, *value_ty]),
            MonoType::Tuple(_) => self.solver.new_var(),
            _ => self.solver.new_var(),
        };

        self.scope.enter_scope();

        // 遮蔽检查
        if self.scope.var_in_any_scope(var) {
            self.scope.exit_scope();
            return Err(Box::new(
                ErrorCodeDefinition::variable_shadowing(var).build(),
            ));
        }

        self.scope.add_var(var.to_string(), PolyType::mono(elem_ty));

        let _ = var_mut;

        if self.collect_all_errors {
            let mut first_err = None;
            for stmt in &body.stmts {
                if let Err(e) = self.check_stmt(stmt) {
                    if first_err.is_none() {
                        first_err = Some(e.clone());
                    }
                    self.collect_error(*e);
                }
            }
            if let Some(expr) = &body.expr {
                if let Err(e) = self.check_expr(expr) {
                    if first_err.is_none() {
                        first_err = Some(e.clone());
                    }
                    self.collect_error(*e);
                }
            }
            self.scope.exit_scope();
            match first_err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        } else {
            let mut err = None;
            for stmt in &body.stmts {
                if let Err(e) = self.check_stmt(stmt) {
                    err = Some(e);
                    break;
                }
            }
            if err.is_none() {
                if let Some(expr) = &body.expr {
                    if let Err(e) = self.check_expr(expr) {
                        err = Some(e);
                    }
                }
            }
            self.scope.exit_scope();
            match err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
    }

    /// 检查 if 语句
    fn check_if_stmt(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        elif_branches: &[(&Expr, &Block)],
        else_branch: Option<&Block>,
        _stmt_span: crate::util::span::Span,
    ) -> Result<(), Box<Diagnostic>> {
        let cond_ty = self.check_expr(condition)?;
        if cond_ty != MonoType::Bool {
            return Err(Box::new(
                ErrorCodeDefinition::type_mismatch("bool", &format!("{}", cond_ty))
                    .at(_stmt_span)
                    .build(),
            ));
        }

        self.check_block(then_branch)?;

        for (elif_cond, _) in elif_branches {
            let elif_cond_ty = self.check_expr(elif_cond)?;
            if elif_cond_ty != MonoType::Bool {
                return Err(Box::new(
                    ErrorCodeDefinition::type_mismatch("bool", &format!("{}", elif_cond_ty))
                        .at(_stmt_span)
                        .build(),
                ));
            }
        }

        for (_, elif_block) in elif_branches {
            self.check_block(elif_block)?;
        }

        if let Some(else_block) = else_branch {
            self.check_block(else_block)?;
        }

        Ok(())
    }

    /// 检查代码块（创建独立作用域）
    ///
    /// 在收集模式下，代码块内的错误会被收集而非短路。
    fn check_block(
        &mut self,
        block: &Block,
    ) -> Result<(), Box<Diagnostic>> {
        self.scope.enter_scope();

        if self.collect_all_errors {
            let mut first_err = None;
            for stmt in &block.stmts {
                if let Err(e) = self.check_stmt(stmt) {
                    if first_err.is_none() {
                        first_err = Some(e.clone());
                    }
                    self.collect_error(*e);
                }
            }
            if let Some(expr) = &block.expr {
                if let Err(e) = self.check_expr(expr) {
                    if first_err.is_none() {
                        first_err = Some(e.clone());
                    }
                    self.collect_error(*e);
                }
            }
            self.scope.exit_scope();
            match first_err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        } else {
            let mut err = None;
            for stmt in &block.stmts {
                if let Err(e) = self.check_stmt(stmt) {
                    err = Some(e);
                    break;
                }
            }
            if err.is_none() {
                if let Some(expr) = &block.expr {
                    if let Err(e) = self.check_expr(expr) {
                        err = Some(e);
                    }
                }
            }
            self.scope.exit_scope();
            match err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
    }

    /// 检查表达式
    ///
    /// 直接使用同一个 ScopeManager 和 Solver，确保变量状态正确传递。
    /// 这是方案 C 的核心。
    /// 直接使用同一个 scope 和 solver，确保类型状态正确传递
    pub fn check_expr(
        &mut self,
        expr: &Expr,
    ) -> Result<MonoType, Box<Diagnostic>> {
        match expr {
            // 变量：直接从 scope 中读取
            Expr::Var(name, span) => {
                if let Some(poly) = self.scope.get_var(name).cloned() {
                    // 直接返回 scope 中的类型
                    Ok(poly.body)
                } else {
                    Err(Box::new(
                        ErrorCodeDefinition::unknown_variable(name)
                            .at(*span)
                            .build(),
                    ))
                }
            }
            // 列表字面量：直接处理
            Expr::List(elems, _) => {
                if elems.is_empty() {
                    let elem_ty = self.solver.new_var();
                    Ok(MonoType::List(Box::new(elem_ty)))
                } else {
                    let mut iter = elems.iter();
                    let first = iter.next().expect("non-empty list");
                    let mut elem_ty = self.check_expr(first)?;
                    for e in iter {
                        let ty = self.check_expr(e)?;
                        let _ = self.solver.unify(&elem_ty, &ty);
                        elem_ty = self.solver.resolve_type(&elem_ty);
                    }
                    Ok(MonoType::List(Box::new(elem_ty)))
                }
            }
            // 二元运算 = 赋值：直接处理
            Expr::BinOp {
                op,
                left,
                right,
                span: _,
            } => {
                use crate::frontend::core::parser::ast::BinOp;
                let right_ty = self.check_expr(right)?;

                if matches!(op, BinOp::Assign) {
                    if let Expr::Var(var_name, _) = left.as_ref() {
                        self.assign_var(var_name, right_ty);
                    }
                    return Ok(MonoType::Void);
                }

                // 其他二元运算：直接处理
                let left_ty = self.check_expr(left)?;

                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        if let (MonoType::Int(_), MonoType::Int(_)) = (&left_ty, &right_ty) {
                            Ok(left_ty)
                        } else if let (MonoType::Float(_), MonoType::Float(_)) =
                            (&left_ty, &right_ty)
                        {
                            Ok(left_ty)
                        } else if let (MonoType::String, MonoType::String) = (&left_ty, &right_ty) {
                            Ok(MonoType::String)
                        } else if let (MonoType::List(left_elem), MonoType::List(right_elem)) =
                            (&left_ty, &right_ty)
                        {
                            let _ = self.solver.unify(left_elem, right_elem);
                            let elem_ty = self.solver.resolve_type(left_elem);
                            Ok(MonoType::List(Box::new(elem_ty)))
                        } else {
                            Ok(self.solver.new_var())
                        }
                    }
                    BinOp::Range => {
                        let elem_ty = if left_ty == right_ty {
                            left_ty
                        } else {
                            let _ = self.solver.unify(&left_ty, &right_ty);
                            left_ty
                        };
                        Ok(MonoType::Range {
                            elem_type: Box::new(elem_ty),
                        })
                    }
                    _ => {
                        // 其他操作符委托给 ExpressionInferrer
                        let current_result_err = self.current_result_err();
                        let mut inferrer =
                            super::ExpressionInferrer::with_native_signatures_and_result_err(
                                &mut self.scope,
                                &mut self.solver,
                                &self.overload_candidates,
                                &self.native_signatures,
                                current_result_err,
                            );
                        inferrer.infer_expr(expr).map_err(Box::new)
                    }
                }
            }
            // 其他表达式：委托给 ExpressionInferrer
            _ => {
                let current_result_err = self.current_result_err();
                let mut inferrer = super::ExpressionInferrer::with_native_signatures_and_result_err(
                    &mut self.scope,
                    &mut self.solver,
                    &self.overload_candidates,
                    &self.native_signatures,
                    current_result_err,
                );
                inferrer.infer_expr(expr).map_err(Box::new)
            }
        }
    }
}
