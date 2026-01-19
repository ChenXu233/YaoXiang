//! 语句和函数类型检查
//!
//! 实现语句的类型检查和函数定义的类型验证

use super::super::lexer::tokens::Literal;
use super::super::parser::ast;
use super::errors::{TypeError, TypeResult};
use super::infer::TypeInferrer;
use super::types::{MonoType, PolyType, TypeConstraintSolver, TypeVar};
use crate::middle;
use crate::util::span::Span;
use crate::util::i18n::{t_cur, MSG};
use std::collections::{HashMap, HashSet};
use tracing::debug;

/// 类型检查器
///
/// 负责检查模块、函数和语句的类型正确性
#[derive(Debug)]
pub struct TypeChecker<'a> {
    /// 类型推断器
    inferrer: TypeInferrer<'a>,
    /// 已检查的函数签名（用于递归检测）
    checked_functions: HashMap<String, bool>,
    /// 当前函数的返回类型
    current_return_type: Option<MonoType>,
    /// 泛型函数缓存
    generic_cache: HashMap<String, HashMap<String, PolyType>>,
    /// 收集的错误
    errors: Vec<TypeError>,
    /// 导入追踪 - 模块导入信息
    imports: Vec<ImportInfo>,
    /// 当前模块的导出项
    exports: HashSet<String>,
    /// 模块名称
    module_name: String,
}

/// 类型检查结果
///
/// 包含类型检查后的所有信息，用于后续 IR 生成
#[derive(Debug, Clone, Default)]
pub struct TypeCheckResult {
    /// 导入列表
    pub imports: Vec<ImportInfo>,
    /// 导出项
    pub exports: HashSet<String>,
    /// 模块名称
    pub module_name: String,
    /// 类型检查器收集的变量绑定（用于 IR 生成）
    /// key: 变量名, value: 多态类型
    pub bindings: HashMap<String, PolyType>,
}

/// 导入信息
///
/// 记录 use 语句导入的内容
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// 导入路径
    pub path: String,
    /// 导入项名称（None 表示导入整个模块）
    pub items: Option<Vec<String>>,
    /// 别名（如果指定）
    pub alias: Option<String>,
    /// 源 span（用于错误定位）
    pub span: Span,
    /// 是否为公开导入
    pub is_public: bool,
}

impl<'a> TypeChecker<'a> {
    /// 创建新的类型检查器
    pub fn new(
        solver: &'a mut TypeConstraintSolver,
        module_name: &str,
    ) -> Self {
        TypeChecker {
            inferrer: TypeInferrer::new(solver),
            checked_functions: HashMap::new(),
            current_return_type: None,
            generic_cache: HashMap::new(),
            errors: Vec::new(),
            imports: Vec::new(),
            exports: HashSet::new(),
            module_name: module_name.to_string(),
        }
    }

    /// 获取导入列表
    pub fn imports(&self) -> &[ImportInfo] {
        &self.imports
    }

    /// 获取模块名称
    pub fn module_name(&self) -> &str {
        &self.module_name
    }

    /// 获取错误列表
    pub fn errors(&self) -> &[TypeError] {
        &self.errors
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 添加变量绑定
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.inferrer.add_var(name, poly);
    }

    /// 添加错误
    fn add_error(
        &mut self,
        error: TypeError,
    ) {
        self.errors.push(error);
    }

    // =========================================================================
    // 模块检查
    // =========================================================================

    /// 检查整个模块
    ///
    /// 只执行类型检查，不生成 IR
    /// 返回类型检查结果，供后续 IR 生成使用
    pub fn check_module(
        &mut self,
        module: &ast::Module,
    ) -> Result<TypeCheckResult, Vec<TypeError>> {
        // 首先收集所有类型定义
        for stmt in &module.items {
            if let ast::StmtKind::TypeDef { name, definition } = &stmt.kind {
                self.add_type_definition(name, definition, stmt.span);
            }
        }

        // 然后检查所有语句
        for stmt in &module.items {
            if let Err(e) = self.check_stmt(stmt) {
                self.add_error(e);
            }
        }

        // 求解所有约束
        self.inferrer.solver().solve().map_err(|e| {
            e.into_iter()
                .map(|e| TypeError::TypeMismatch {
                    expected: e.error.left,
                    found: e.error.right,
                    span: e.span,
                })
                .collect::<Vec<_>>()
        })?;

        // 如果有错误，返回所有错误
        if self.has_errors() {
            return Err(self.errors.clone());
        }

        // 构建类型检查结果
        let result = TypeCheckResult {
            imports: self.imports.clone(),
            exports: self.exports.clone(),
            module_name: self.module_name.clone(),
            bindings: self.inferrer.get_all_bindings(),
        };

        Ok(result)
    }

    /// 添加类型定义
    fn add_type_definition(
        &mut self,
        name: &str,
        definition: &ast::Type,
        _span: Span,
    ) {
        let poly = PolyType::mono(MonoType::from(definition.clone()));
        self.inferrer.add_var(name.to_string(), poly);
    }

    // =========================================================================
    // 语句检查
    // =========================================================================

    /// 检查语句
    #[allow(clippy::result_large_err)]
    pub fn check_stmt(
        &mut self,
        stmt: &ast::Stmt,
    ) -> TypeResult<()> {
        match &stmt.kind {
            ast::StmtKind::Expr(expr) => {
                if let ast::Expr::FnDef {
                    name,
                    params,
                    return_type,
                    body,
                    is_async,
                    span: _,
                } = expr.as_ref()
                {
                    // 有返回类型标注表示有完整类型签名
                    let is_annotated = return_type.is_some();
                    self.check_fn_def(
                        name,
                        params,
                        return_type.as_ref(),
                        body,
                        *is_async,
                        None,
                        is_annotated,
                    )?;
                    Ok(())
                } else if let ast::Expr::BinOp {
                    op: ast::BinOp::Assign,
                    left,
                    right: _,
                    ..
                } = expr.as_ref()
                {
                    // 对于赋值表达式，先将变量添加到作用域，再推断类型
                    if let ast::Expr::Var(name, _) = left.as_ref() {
                        // 为变量创建类型变量并添加到作用域
                        let ty = self.inferrer.solver().new_var();
                        let poly = PolyType::mono(ty);
                        self.inferrer.add_var(name.clone(), poly);
                    }
                    // 推断整个赋值表达式
                    self.inferrer.infer_expr(expr)?;
                    Ok(())
                } else {
                    self.inferrer.infer_expr(expr)?;
                    Ok(())
                }
            }
            ast::StmtKind::Fn {
                name,
                type_annotation,
                params,
                body: (stmts, expr),
            } => {
                debug!("{}", t_cur(MSG::TypeCheckProcessFn, Some(&[&name])));

                // 1. Extract params and return type from annotation if available
                let (annotated_params, annotated_return) = if let Some(ast::Type::Fn {
                    params,
                    return_type,
                    ..
                }) = type_annotation
                {
                    (Some(params), Some(return_type.as_ref()))
                } else {
                    (None, None)
                };

                // 2. Prepare parameter types for the function signature
                let param_types: Vec<MonoType> = if let Some(a_params) = annotated_params {
                    if a_params.len() == params.len() {
                        a_params.iter().map(|t| MonoType::from(t.clone())).collect()
                    } else {
                        // Mismatch in count, fallback to params or error?
                        // For now, fallback to inferring from params if counts don't match,
                        // but ideally this should be an error caught by parser/validation.
                        // Given parser checks count, we can assume they match or use params.
                        params
                            .iter()
                            .map(|p| {
                                if let Some(ty) = &p.ty {
                                    MonoType::from(ty.clone())
                                } else {
                                    self.inferrer.solver().new_var()
                                }
                            })
                            .collect()
                    }
                } else {
                    params
                        .iter()
                        .map(|p| {
                            if let Some(ty) = &p.ty {
                                MonoType::from(ty.clone())
                            } else {
                                self.inferrer.solver().new_var()
                            }
                        })
                        .collect()
                };

                // 3. Prepare return type for the function signature
                // Handle "_" as "inferred" (None)
                let expected_return_type = if let Some(ty) = annotated_return {
                    if let ast::Type::Name(n) = ty {
                        if n == "_" {
                            None
                        } else {
                            Some(ty)
                        }
                    } else {
                        Some(ty)
                    }
                } else {
                    None
                };

                let return_ty = if let Some(ty) = expected_return_type {
                    MonoType::from(ty.clone())
                } else {
                    self.inferrer.solver().new_var()
                };

                // 4. Construct the inferred function type
                let fn_type = MonoType::Fn {
                    params: param_types.clone(),
                    return_type: Box::new(return_ty),
                    is_async: false,
                };

                // 5. If there is an annotation, constrain the inferred type to it
                if let Some(ann) = type_annotation {
                    let ann_ty = MonoType::from(ann.clone());
                    self.inferrer
                        .solver()
                        .add_constraint(fn_type.clone(), ann_ty, stmt.span);
                }

                // 6. Register the function in the scope
                self.inferrer.add_var(name.clone(), PolyType::mono(fn_type));

                let body = ast::Block {
                    stmts: stmts.clone(),
                    expr: expr.clone(),
                    span: stmt.span,
                };

                // Pass the constrained param_types to check_fn_def so inner scope matches outer signature
                debug!(
                    "{}",
                    t_cur(
                        MSG::TypeCheckCallFnDef,
                        Some(&[&annotated_params.is_some()])
                    )
                );
                self.check_fn_def(
                    name,
                    params,
                    expected_return_type,
                    &body,
                    false,
                    Some(param_types),
                    annotated_params.is_some(),
                )?;
                Ok(())
            }
            ast::StmtKind::Var {
                name,
                type_annotation,
                initializer,
                is_mut: _,
            } => self.check_var(
                name,
                type_annotation.as_ref(),
                initializer.as_deref(),
                stmt.span,
            ),
            ast::StmtKind::For {
                var,
                iterable,
                body,
                ..
            } => {
                // 类型检查 for 循环
                // 1. 推断 iterable 的类型（应该是可迭代的）
                let _iter_ty = self.inferrer.infer_expr(iterable)?;

                // 2. 在循环体中，var 的类型取决于 iterable 的元素类型
                // 暂时添加一个类型变量
                let var_ty = self.inferrer.solver().new_var();
                let poly = PolyType::mono(var_ty);
                self.inferrer.add_var(var.clone(), poly);

                // 3. 类型检查循环体
                let _body_ty = self.inferrer.infer_block(body, false, None)?;
                Ok(())
            }
            ast::StmtKind::TypeDef { name, definition } => {
                self.check_type_def(name, definition, stmt.span)
            }
            ast::StmtKind::Module { name, items } => {
                self.check_module_alias(name, items, stmt.span)
            }
            ast::StmtKind::Use { path, items, alias } => {
                self.check_use(path, items.as_deref(), alias.as_deref(), stmt.span)
            }
        }
    }

    /// 检查变量声明: `name[: type] [= expr]`
    #[allow(clippy::result_large_err)]
    fn check_var(
        &mut self,
        name: &str,
        type_annotation: Option<&ast::Type>,
        initializer: Option<&ast::Expr>,
        span: Span,
    ) -> TypeResult<()> {
        if let Some(init) = initializer {
            let init_ty = self.inferrer.infer_expr(init)?;

            if let Some(ann) = type_annotation {
                let ann_ty = MonoType::from(ann.clone());
                self.inferrer
                    .solver()
                    .add_constraint(init_ty.clone(), ann_ty, span);
            }

            // 泛化 initializer 的类型
            let poly = self.inferrer.solver().generalize(&init_ty);
            self.inferrer.add_var(name.to_string(), poly);
        } else if let Some(ann) = type_annotation {
            // 没有初始化时，使用类型注解
            let ty = MonoType::from(ann.clone());
            self.inferrer.add_var(name.to_string(), PolyType::mono(ty));
        } else {
            // 没有任何信息，创建新类型变量
            let ty = self.inferrer.solver().new_var();
            self.inferrer.add_var(name.to_string(), PolyType::mono(ty));
        }

        Ok(())
    }

    /// 检查类型定义
    #[allow(clippy::result_large_err)]
    fn check_type_def(
        &mut self,
        name: &str,
        definition: &ast::Type,
        _span: Span,
    ) -> TypeResult<()> {
        let ty = MonoType::from(definition.clone());
        self.inferrer.add_var(name.to_string(), PolyType::mono(ty));
        Ok(())
    }

    /// 检查模块别名
    #[allow(clippy::result_large_err)]
    fn check_module_alias(
        &mut self,
        _name: &str,
        _items: &[ast::Stmt],
        _span: Span,
    ) -> TypeResult<()> {
        // TODO: 实现模块别名检查
        Ok(())
    }

    /// 检查 use 语句
    #[allow(clippy::result_large_err)]
    fn check_use(
        &mut self,
        path: &str,
        items: Option<&[String]>,
        alias: Option<&str>,
        span: Span,
    ) -> TypeResult<()> {
        // 1. 验证模块路径语法
        if !self.is_valid_module_path(path) {
            self.add_error(TypeError::import_error(
                format!("无效的模块路径: {}", path),
                span,
            ));
            return Ok(());
        }

        // 2. 处理标准库导入
        // 解析后的路径使用 "::" 分隔符，例如 "std.io"
        if path.starts_with("std.") {
            debug!("Detected std import: {}", path);
            return self.import_stdlib(path, items, alias, span);
        }

        // 3. 验证选择性导入语法
        if let Some(import_items) = items {
            if import_items.is_empty() {
                self.add_error(TypeError::import_error("空的导入列表".to_string(), span));
                return Ok(());
            }

            // 检查重复导入
            let mut seen = HashSet::new();
            for item in import_items {
                if !seen.insert(item) {
                    self.add_error(TypeError::import_error(format!("重复导入: {}", item), span));
                }
            }
        }

        // 4. 验证别名语法
        if let Some(alias_name) = alias {
            if alias_name.is_empty() {
                self.add_error(TypeError::import_error("空的别名".to_string(), span));
            } else if !alias_name.chars().next().unwrap().is_ascii_lowercase() {
                self.add_error(TypeError::import_error("别名必须小写".to_string(), span));
            }
        }

        // 5. 记录导入信息
        let import_info = ImportInfo {
            path: path.to_string(),
            items: items.map(|v| v.to_vec()),
            alias: alias.map(|s| s.to_string()),
            span,
            is_public: false,
        };
        self.imports.push(import_info);

        debug!("Recorded use statement: path={}, items={:?}", path, items);

        Ok(())
    }

    /// 导入标准库模块
    #[allow(clippy::result_large_err)]
    fn import_stdlib(
        &mut self,
        path: &str,
        items: Option<&[String]>,
        _alias: Option<&str>,
        span: Span,
    ) -> TypeResult<()> {
        // 提取模块名：std.io -> io
        let module = path.strip_prefix("std.").unwrap_or(path);

        // 定义标准库函数签名
        let stdlib_functions: HashMap<&str, PolyType> = [
            // std.io
            (
                "print",
                PolyType::new(
                    vec![TypeVar::new(0)],
                    MonoType::Fn {
                        params: vec![MonoType::TypeVar(TypeVar::new(0))],
                        return_type: Box::new(MonoType::Void),
                        is_async: false,
                    },
                ),
            ),
            (
                "println",
                PolyType::new(
                    vec![TypeVar::new(0)],
                    MonoType::Fn {
                        params: vec![MonoType::TypeVar(TypeVar::new(0))],
                        return_type: Box::new(MonoType::Void),
                        is_async: false,
                    },
                ),
            ),
            (
                "read_line",
                PolyType::mono(MonoType::Fn {
                    params: vec![],
                    return_type: Box::new(MonoType::String),
                    is_async: false,
                }),
            ),
            (
                "read_file",
                PolyType::mono(MonoType::Fn {
                    params: vec![MonoType::String],
                    return_type: Box::new(MonoType::String),
                    is_async: false,
                }),
            ),
            (
                "write_file",
                PolyType::mono(MonoType::Fn {
                    params: vec![MonoType::String, MonoType::String],
                    return_type: Box::new(MonoType::Bool),
                    is_async: false,
                }),
            ),
        ]
        .into_iter()
        .collect();

        // 根据模块选择要导入的函数
        let functions_to_import: Vec<&str> = match module {
            "io" => vec!["print", "println", "read_line", "read_file", "write_file"],
            _ => {
                self.add_error(TypeError::import_error(
                    format!("未知标准库模块: {}", module),
                    span,
                ));
                return Ok(());
            }
        };

        // 导入指定的函数或所有函数
        let items_to_import = items.map(|v| v.iter().map(|s| s.as_str()).collect::<Vec<_>>());

        let target_functions: Vec<&str> = if let Some(items) = items_to_import {
            // 只导入指定的函数
            for item in &items {
                if !functions_to_import.contains(item) {
                    self.add_error(TypeError::import_error(
                        format!("std.{} 中不存在函数: {}", module, item),
                        span,
                    ));
                }
            }
            items
        } else {
            // 导入模块中的所有函数
            functions_to_import.to_vec()
        };

        // 将函数添加到符号表
        for func_name in &target_functions {
            if let Some(poly_type) = stdlib_functions.get(func_name) {
                self.inferrer
                    .add_var(func_name.to_string(), poly_type.clone());
            }
        }

        // 记录导入信息
        let import_info = ImportInfo {
            path: path.to_string(),
            items: Some(target_functions.iter().map(|s| s.to_string()).collect()),
            alias: None,
            span,
            is_public: false,
        };
        self.imports.push(import_info);

        debug!("Imported stdlib: path={}, items={:?}", path, items);

        Ok(())
    }

    /// 验证模块路径语法
    fn is_valid_module_path(
        &self,
        path: &str,
    ) -> bool {
        if path.is_empty() {
            return false;
        }
        // 路径使用 "." 分隔符
        let parts: Vec<&str> = path.split(".").collect();
        for part in parts {
            if part.is_empty() {
                return false;
            }
            if let Some(first_char) = part.chars().next() {
                if !first_char.is_ascii_lowercase() && first_char != '_' {
                    return false;
                }
            }
            for c in part.chars() {
                if !c.is_ascii_alphanumeric() && c != '_' {
                    return false;
                }
            }
        }
        true
    }

    /// 记录导出项
    pub fn record_export(
        &mut self,
        name: &str,
    ) {
        self.exports.insert(name.to_string());
    }

    /// 检查名称是否已导出
    pub fn is_exported(
        &self,
        name: &str,
    ) -> bool {
        self.exports.contains(name)
    }

    /// 获取导出项
    pub fn exported_items(&self) -> &HashSet<String> {
        &self.exports
    }

    /// 标记导入为公开导入
    pub fn mark_import_public(
        &mut self,
        index: usize,
    ) -> bool {
        if index < self.imports.len() {
            self.imports[index].is_public = true;
            true
        } else {
            false
        }
    }

    // =========================================================================
    // 函数检查
    // =========================================================================

    /// 检查函数定义
    ///
    /// 实现完整的类型推断规则：
    /// - 参数类型推断：有标注用标注，无标注尝试推断，Lambda 参数无法推断则拒绝
    /// - 返回类型推断：有标注用标注，无标注则从函数体推断
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::result_large_err)]
    pub fn check_fn_def(
        &mut self,
        name: &str,
        params: &[ast::Param],
        return_type: Option<&ast::Type>,
        body: &ast::Block,
        is_async: bool,
        external_param_types: Option<Vec<MonoType>>,
        is_annotated: bool,
    ) -> TypeResult<middle::FunctionIR> {
        // 保存当前返回类型
        let prev_return_type = self.current_return_type.take();
        let prev_inferrer_return_type = self.inferrer.current_return_type.take();

        let _has_external_types = external_param_types.is_some();

        // 创建参数类型列表
        let param_types: Vec<MonoType> = if let Some(types) = external_param_types {
            types
        } else {
            params
                .iter()
                .map(|p| {
                    if let Some(ty) = &p.ty {
                        MonoType::from(ty.clone())
                    } else {
                        self.inferrer.solver().new_var()
                    }
                })
                .collect()
        };

        // 跟踪未类型化的参数（使用 param_types 中对应位置的类型变量）
        let mut untyped_params: Vec<(String, usize, MonoType)> = Vec::new();
        for (i, param) in params.iter().enumerate() {
            if param.ty.is_none() {
                // 直接使用 param_types 中对应位置的类型变量
                untyped_params.push((param.name.clone(), i, param_types[i].clone()));
            }
        }

        // 处理返回类型
        let (return_ty, _inferred_return_ty) = if let Some(ty) = return_type {
            // 有标注类型，使用标注类型
            (MonoType::from(ty.clone()), None)
        } else {
            // 无标注类型，需要从函数体推断
            let inferred = self.inferrer.solver().new_var();
            (inferred.clone(), Some(inferred))
        };

        // 设置当前返回类型
        self.current_return_type = Some(return_ty.clone());
        self.inferrer.current_return_type = Some(return_ty.clone());

        // 进入函数体作用域
        self.inferrer.enter_scope();

        // 添加参数到作用域
        // 注意：不要使用 generalize/instantiate，因为参数类型需要在函数体内被约束
        for (param, param_ty) in params.iter().zip(param_types.iter()) {
            let poly = PolyType::mono(param_ty.clone());
            self.inferrer.add_var(param.name.clone(), poly);
        }

        // 推断函数体
        // infer_block 会调用 infer_expr，后者会调用 infer_return
        // infer_return 会使用 self.inferrer.current_return_type 进行检查
        let body_ty = self.inferrer.infer_block(body, true, None)?;

        // 检查隐式返回（最后表达式或 Void）
        if let Some(expr) = &body.expr {
            // 如果最后表达式是 `return`，则该 return 已在 infer_return 中
            // 对当前返回类型添加了约束，因此不需要（也不应）再将
            // 块的类型与返回类型约束在一起 — 否则会把 `Void` 约束到返回类型。
            if let ast::Expr::Return(_, _) = expr.as_ref() {
                // diverging via explicit return; nothing to do here
            } else {
                // 有最后表达式，约束其类型为返回类型
                self.inferrer.solver().add_constraint(
                    body_ty.clone(),
                    return_ty.clone(),
                    body.span,
                );
            }
        } else {
            // 无最后表达式，检查是否是发散的（以 return 结尾）
            // 如果不是发散的，则隐式返回 Void
            // Consider the function diverging if any top-level statement in the
            // block is an explicit `return`.
            let is_diverging = body.stmts.iter().any(|s| match &s.kind {
                ast::StmtKind::Expr(e) => matches!(e.as_ref(), ast::Expr::Return(_, _)),
                _ => false,
            });

            if !is_diverging {
                self.inferrer
                    .solver()
                    .add_constraint(MonoType::Void, return_ty.clone(), body.span);
            }
        }

        // 参数类型推断规则：检查未类型化参数
        // 如果没有外部类型标注，且参数本身没有类型标注，则报错（不支持从使用推断参数类型）
        debug!(
            "{}",
            t_cur(
                MSG::TypeCheckAnnotated,
                Some(&[
                    &format!("{}", is_annotated),
                    &format!("{}", untyped_params.len())
                ])
            )
        );
        if !is_annotated {
            for (param_name, param_idx, _param_ty) in &untyped_params {
                debug!("{}", t_cur(MSG::TypeCheckAddError, Some(&[param_name])));
                self.add_error(TypeError::CannotInferParamType {
                    name: param_name.clone(),
                    span: params[*param_idx].span,
                });
            }
        }

        // 退出函数体作用域
        self.inferrer.exit_scope();

        // 恢复之前的返回类型
        self.current_return_type = prev_return_type;
        self.inferrer.current_return_type = prev_inferrer_return_type;

        // 生成函数 IR
        let fn_ir = middle::FunctionIR {
            name: name.to_string(),
            params: param_types,
            return_type: return_ty,
            is_async,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: 0,
        };

        Ok(fn_ir)
    }

    /// 检查类型变量是否未约束（未被使用）
    ///
    /// 如果类型变量仍然是 Unbound 状态，返回 true
    fn is_unconstrained_var(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            MonoType::TypeVar(id) => self.inferrer.solver_ref().is_unconstrained(*id),
            _ => false,
        }
    }

    /// 检查函数调用
    #[allow(clippy::result_large_err)]
    fn check_fn_call(
        &mut self,
        func: &ast::Expr,
        args: &[ast::Expr],
        span: Span,
    ) -> TypeResult<MonoType> {
        self.inferrer.infer_call(func, args, span)
    }
}

// =========================================================================
// 类型环境扩展
// =========================================================================

/// 扩展类型环境，支持更多操作
#[derive(Debug, Default)]
pub struct ExtendedTypeEnvironment {
    /// 变量绑定
    vars: HashMap<String, PolyType>,
    /// 类型定义
    types: HashMap<String, PolyType>,
    /// 求解器
    solver: TypeConstraintSolver,
    /// 错误列表
    errors: Vec<TypeError>,
}

impl ExtendedTypeEnvironment {
    /// 创建新的扩展类型环境
    pub fn new() -> Self {
        ExtendedTypeEnvironment {
            vars: HashMap::new(),
            types: HashMap::new(),
            solver: TypeConstraintSolver::new(),
            errors: Vec::new(),
        }
    }

    /// 添加变量
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.vars.insert(name, poly);
    }

    /// 添加类型
    pub fn add_type(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.types.insert(name, poly);
    }

    /// 获取变量
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.vars.get(name)
    }

    /// 获取类型
    pub fn get_type(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.types.get(name)
    }

    /// 添加错误
    pub fn add_error(
        &mut self,
        error: TypeError,
    ) {
        self.errors.push(error);
    }

    /// 获取错误
    pub fn errors(&self) -> &[TypeError] {
        &self.errors
    }

    /// 是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// 检查字面量类型
pub fn infer_literal_type(lit: &Literal) -> MonoType {
    match lit {
        Literal::Int(_) => MonoType::Int(64),
        Literal::Float(_) => MonoType::Float(64),
        Literal::Bool(_) => MonoType::Bool,
        Literal::Char(_) => MonoType::Char,
        Literal::String(_) => MonoType::String,
    }
}

/// 获取二元运算的结果类型
pub fn binop_result_type(
    op: &ast::BinOp,
    left: &MonoType,
    right: &MonoType,
) -> Option<MonoType> {
    match op {
        ast::BinOp::Add | ast::BinOp::Sub | ast::BinOp::Mul | ast::BinOp::Div | ast::BinOp::Mod => {
            if left == right && left.is_numeric() {
                Some(left.clone())
            } else {
                None
            }
        }
        ast::BinOp::Eq
        | ast::BinOp::Neq
        | ast::BinOp::Lt
        | ast::BinOp::Le
        | ast::BinOp::Gt
        | ast::BinOp::Ge => {
            if left == right {
                Some(MonoType::Bool)
            } else {
                None
            }
        }
        ast::BinOp::And | ast::BinOp::Or => {
            if *left == MonoType::Bool && *right == MonoType::Bool {
                Some(MonoType::Bool)
            } else {
                None
            }
        }
        ast::BinOp::Assign => Some(MonoType::Void),
        ast::BinOp::Range => None, // 范围运算暂时不支持类型检查
    }
}

/// 获取一元运算的结果类型
pub fn unop_result_type(
    op: &ast::UnOp,
    expr: &MonoType,
) -> Option<MonoType> {
    match op {
        ast::UnOp::Neg | ast::UnOp::Pos => {
            if expr.is_numeric() {
                Some(expr.clone())
            } else {
                None
            }
        }
        ast::UnOp::Not => {
            if *expr == MonoType::Bool {
                Some(MonoType::Bool)
            } else {
                None
            }
        }
    }
}
