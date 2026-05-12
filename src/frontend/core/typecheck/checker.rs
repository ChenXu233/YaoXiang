//! 类型检查器模块
//!
//! 包含 TypeChecker 的完整实现

use std::collections::{HashMap, HashSet};

use crate::frontend::core::parser::ast::Module;
use crate::frontend::core::types::base::{MonoType, PolyType};
use crate::frontend::core::typecheck::traits::auto_derive;
use crate::frontend::core::types::computation::const_generics::{ConstFunction, ConstExpr};

use super::inference;
use super::semantic_db;
use super::spawn_placement;
use super::types::TypeCheckResult;
use super::environment::TypeEnvironment;
use super::{add_builtin_types, add_std_traits, add_native_function_types};
use crate::frontend::core::parser::ast::Expr;
use super::Diagnostic;
use crate::util::diagnostic::ErrorCodeDefinition;

/// 类型检查器
///
/// 负责模块级类型检查编排，协调前置收集和函数体检查
pub struct TypeChecker {
    /// 当前环境
    env: TypeEnvironment,
    /// 语句检查器
    body_checker: Option<inference::StatementChecker>,
    /// 语义信息收集（typecheck 阶段同时产出）
    semantic_db: semantic_db::SemanticDB,
}

impl TypeChecker {
    /// 创建新的类型检查器
    pub fn new(module_name: &str) -> Self {
        let mut env = TypeEnvironment::new_with_module(module_name.to_string());
        add_builtin_types(&mut env);
        add_std_traits(&mut env);
        add_native_function_types(&mut env);

        // 注册预定义的 const 函数
        Self::register_predefined_const_functions(&mut env);

        Self {
            env,
            body_checker: None,
            semantic_db: semantic_db::SemanticDB::new(),
        }
    }

    /// 注册预定义的 const 函数
    /// 这些函数用于值依赖类型的编译期求值
    fn register_predefined_const_functions(env: &mut TypeEnvironment) {
        use crate::frontend::core::types::computation::const_generics::ConstBinOp;

        // 注册 factorial 函数
        let factorial = ConstFunction::new(
            "factorial".to_string(),
            vec!["n".to_string()],
            ConstExpr::If {
                condition: Box::new(ConstExpr::BinOp {
                    op: ConstBinOp::Lte,
                    lhs: Box::new(ConstExpr::Var("n".to_string())),
                    rhs: Box::new(ConstExpr::Int(1)),
                }),
                true_branch: Box::new(ConstExpr::Int(1)),
                false_branch: Box::new(ConstExpr::BinOp {
                    op: ConstBinOp::Mul,
                    lhs: Box::new(ConstExpr::Var("n".to_string())),
                    rhs: Box::new(ConstExpr::Call {
                        name: "factorial".to_string(),
                        args: vec![ConstExpr::BinOp {
                            op: ConstBinOp::Sub,
                            lhs: Box::new(ConstExpr::Var("n".to_string())),
                            rhs: Box::new(ConstExpr::Int(1)),
                        }],
                    }),
                }),
            },
        );
        env.add_const_function("factorial".to_string(), factorial);

        // 注册 fibonacci 函数
        let fibonacci = ConstFunction::new(
            "fibonacci".to_string(),
            vec!["n".to_string()],
            ConstExpr::If {
                condition: Box::new(ConstExpr::BinOp {
                    op: ConstBinOp::Lte,
                    lhs: Box::new(ConstExpr::Var("n".to_string())),
                    rhs: Box::new(ConstExpr::Int(1)),
                }),
                true_branch: Box::new(ConstExpr::Var("n".to_string())),
                false_branch: Box::new(ConstExpr::BinOp {
                    op: ConstBinOp::Add,
                    lhs: Box::new(ConstExpr::Call {
                        name: "fibonacci".to_string(),
                        args: vec![ConstExpr::BinOp {
                            op: ConstBinOp::Sub,
                            lhs: Box::new(ConstExpr::Var("n".to_string())),
                            rhs: Box::new(ConstExpr::Int(1)),
                        }],
                    }),
                    rhs: Box::new(ConstExpr::Call {
                        name: "fibonacci".to_string(),
                        args: vec![ConstExpr::BinOp {
                            op: ConstBinOp::Sub,
                            lhs: Box::new(ConstExpr::Var("n".to_string())),
                            rhs: Box::new(ConstExpr::Int(2)),
                        }],
                    }),
                }),
            },
        );
        env.add_const_function("fibonacci".to_string(), fibonacci);
    }

    /// 获取环境引用
    pub fn env(&mut self) -> &mut TypeEnvironment {
        &mut self.env
    }

    /// 获取模块名称
    pub fn module_name(&self) -> &str {
        &self.env.module_name
    }

    /// 提取收集到的语义信息
    pub fn take_semantic_db(&mut self) -> semantic_db::SemanticDB {
        std::mem::take(&mut self.semantic_db)
    }

    /// 添加错误
    fn add_error(
        &mut self,
        error: Diagnostic,
    ) {
        self.env.errors.add_error(error);
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        self.env.errors.has_errors()
    }

    /// 添加变量绑定
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.env.add_var(name, poly);
    }

    /// 获取错误列表
    pub fn errors(&self) -> &[Diagnostic] {
        self.env.errors.errors()
    }

    /// 检查单个语句（委托给 StatementChecker）
    pub fn check_stmt(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) -> Result<(), Box<Diagnostic>> {
        self.body_checker_mut().check_stmt(stmt)
    }

    /// 检查整个模块
    ///
    /// 在收集模式下，将收集所有错误后统一返回。
    pub fn check_module(
        &mut self,
        module: &Module,
    ) -> Result<TypeCheckResult, Vec<Diagnostic>> {
        self.check_module_impl(module, false)
    }

    /// 检查整个模块（收集所有错误模式）
    ///
    /// 启用错误收集模式后，类型检查器会尽可能多地收集错误，
    /// 而不是在第一个错误处停止。适用于 LSP 诊断场景。
    pub fn check_module_collect_all(
        &mut self,
        module: &Module,
    ) -> Result<TypeCheckResult, Vec<Diagnostic>> {
        self.check_module_impl(module, true)
    }

    /// 检查整个模块的内部实现
    fn check_module_impl(
        &mut self,
        module: &Module,
        collect_all: bool,
    ) -> Result<TypeCheckResult, Vec<Diagnostic>> {
        // 第一遍：收集所有类型定义
        for stmt in &module.items {
            if let crate::frontend::core::parser::ast::StmtKind::Binding {
                name,
                type_name,
                method_type: _,
                type_annotation,
                generic_params,
                params,
                body,
                ..
            } = &stmt.kind
            {
                if crate::frontend::core::parser::ast::classify_binding_semantic_kind(
                    type_name.as_ref(),
                    type_annotation.as_ref(),
                    params,
                    &body.0,
                    body.1.as_deref(),
                ) == crate::frontend::core::parser::ast::BindingSemanticKind::TypeConstructor
                {
                    // 这是一个类型定义
                    if let Some(type_annotation) = type_annotation {
                        // 从 GenericParam 中提取名称
                        let param_names: Vec<String> =
                            generic_params.iter().map(|p| p.name.clone()).collect();
                        self.add_type_definition(name, type_annotation, &param_names, stmt.span);
                    }
                }
            }
        }

        // 第二遍：收集所有函数签名（使其可被前向引用）
        for stmt in &module.items {
            self.collect_function_signature(stmt);
        }

        // 收集所有导出项
        self.collect_exports(module);

        // RFC-001/008: `spawn` 仅允许在 `@block` 作用域内使用（编译期约束）
        for err in spawn_placement::check_spawn_placement(module) {
            self.add_error(err);
        }

        // 初始化函数体检查器
        let mut body_checker = inference::StatementChecker::new(self.env.solver());
        // 设置 native 函数签名表
        body_checker.set_native_signatures(self.env.native_signatures.clone());
        // 设置模块注册表，支持函数体/块作用域 use
        body_checker.set_module_registry(self.env.module_registry.clone());
        // 如果启用收集模式，设置收集所有错误
        if collect_all {
            body_checker.set_collect_all_errors(true);
        }
        *self.body_checker_mut() = body_checker;

        // 将环境中的变量同步到 body_checker
        for (name, poly) in self.env.vars.clone() {
            self.body_checker_mut().add_var(name, poly);
        }

        // 第三遍：检查所有语句（包括函数体）
        for stmt in &module.items {
            if let Err(e) = self.body_checker_mut().check_stmt(stmt) {
                self.add_error(*e);
            }
        }

        // 收集 body_checker 中累积的错误（收集模式下产生的）
        if let Some(ref mut bc) = self.body_checker {
            for err in bc.drain_collected_errors() {
                self.env.errors.add_error(err);
            }
        }

        // 求解所有约束
        let solve_result = self.env.solver().solve();
        if let Err(constraint_errors) = solve_result {
            for e in constraint_errors {
                self.add_error(
                    ErrorCodeDefinition::type_mismatch(
                        &format!("{}", e.error.left),
                        &format!("{}", e.error.right),
                    )
                    .at(e.span)
                    .build(),
                );
            }
        }

        // 语义收集：遍历 AST 构建 SemanticDB
        // 即便类型检查存在错误（如语法或类型错误），我们也要尽可能收集当前的语义 token，保证代码染色等功能
        self.collect_semantic_tokens(module);

        // 如果有错误，返回所有错误
        if self.has_errors() {
            return Err(self.errors().to_vec());
        }

        // 构建类型检查结果
        // 合并 StatementChecker 中的局部变量类型到 bindings
        let mut bindings = self.env.vars.clone();
        let mut local_var_types = HashMap::new();

        // 从 body_checker.vars 获取局部变量类型
        if let Some(ref bc) = self.body_checker {
            for (name, poly) in bc.vars() {
                // 只添加 env.vars 中不存在的局部变量类型
                if !bindings.contains_key(&name) {
                    bindings.insert(name.clone(), poly.clone());
                }
                // 收集局部变量的 MonoType（用于 IR 生成器错误消息）
                local_var_types.insert(name, poly.body);
            }
        }

        // 同时从 env.vars 收集非全局绑定（函数）的局部变量
        for (name, poly) in &self.env.vars {
            // 排除函数（函数名首字母小写或者是已知的函数）
            let is_function = matches!(
                poly.body,
                crate::frontend::core::types::base::MonoType::Fn { .. }
            );
            if !is_function && !local_var_types.contains_key(name) {
                local_var_types.insert(name.clone(), poly.body.clone());
            }
        }

        // 注意：由于 body_checker.solver 是克隆的，无法通过 solver.resolve() 来解析类型变量。
        // 幸运的是，assign_var 方法已经将更新后的类型写回到了 scope 中，
        // 所以这里直接使用 scope 中的类型即可，不需要额外 resolve。
        // （注：如果后续需要支持更复杂的泛型推导，可能需要重新设计 solver 的共享机制）

        let result = TypeCheckResult {
            module_name: self.env.module_name.clone(),
            bindings,
            local_var_types,
            semantic_db: std::mem::take(&mut self.semantic_db),
        };

        Ok(result)
    }

    /// 获取 body_checker 的可变引用
    fn body_checker_mut(&mut self) -> &mut inference::StatementChecker {
        if self.body_checker.is_none() {
            let mut body_checker = inference::StatementChecker::new(self.env.solver());
            // 设置 native 函数签名表
            body_checker.set_native_signatures(self.env.native_signatures.clone());
            // 设置模块注册表，支持函数体/块作用域 use
            body_checker.set_module_registry(self.env.module_registry.clone());
            self.body_checker = Some(body_checker);
        }
        self.body_checker.as_mut().unwrap()
    }

    /// 收集函数签名（第一遍扫描）
    fn collect_function_signature(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) {
        match &stmt.kind {
            crate::frontend::core::parser::ast::StmtKind::Expr(expr) => {
                // 处理函数定义表达式
                if let crate::frontend::core::parser::ast::Expr::FnDef {
                    name,
                    params,
                    return_type,
                    is_async,
                    ..
                } = expr.as_ref()
                {
                    let fn_ty = MonoType::Fn {
                        params: params
                            .iter()
                            .map(|p| {
                                p.ty.as_ref()
                                    .map(|t| MonoType::from(t.clone()))
                                    .unwrap_or_else(|| self.env.solver().new_var())
                            })
                            .collect(),
                        return_type: Box::new(
                            return_type
                                .as_ref()
                                .map(|t| MonoType::from(t.clone()))
                                .unwrap_or_else(|| self.env.solver().new_var()),
                        ),
                        is_async: *is_async,
                    };
                    self.env.add_var(name.clone(), PolyType::mono(fn_ty));
                }
                // 处理 Lambda 赋值 (name = (params) => body)
                else if let crate::frontend::core::parser::ast::Expr::BinOp {
                    op: crate::frontend::core::parser::ast::BinOp::Assign,
                    left,
                    right,
                    ..
                } = expr.as_ref()
                {
                    if let crate::frontend::core::parser::ast::Expr::Var(name, _) = left.as_ref() {
                        if let crate::frontend::core::parser::ast::Expr::Lambda { params, .. } =
                            right.as_ref()
                        {
                            let fn_ty = MonoType::Fn {
                                params: params
                                    .iter()
                                    .map(|p| {
                                        p.ty.as_ref()
                                            .map(|t| MonoType::from(t.clone()))
                                            .unwrap_or_else(|| self.env.solver().new_var())
                                    })
                                    .collect(),
                                return_type: Box::new(self.env.solver().new_var()),
                                is_async: false,
                            };
                            self.env.add_var(name.clone(), PolyType::mono(fn_ty));
                        }
                    }
                }
            }
            crate::frontend::core::parser::ast::StmtKind::Binding {
                name,
                type_name,
                method_type,
                type_annotation,
                params,
                is_pub,
                ..
            } => {
                // 处理统一函数语法
                // 方法绑定使用 method_type，普通函数使用 type_annotation
                let (param_types, return_type) = if let Some(meth_ty) = method_type {
                    // 方法绑定：优先使用 method_type 中的签名
                    if let crate::frontend::core::parser::ast::Type::Fn {
                        params: param_tys,
                        return_type,
                    } = meth_ty
                    {
                        let pts: Vec<MonoType> = param_tys
                            .iter()
                            .map(|t| MonoType::from(t.clone()))
                            .collect();
                        (pts, MonoType::from(*return_type.clone()))
                    } else {
                        // method_type 不是 Fn 类型，回退到 type_annotation 或 params
                        if let Some(type_ann) = type_annotation {
                            if let crate::frontend::core::parser::ast::Type::Fn {
                                params: param_tys,
                                return_type,
                            } = type_ann
                            {
                                let pts: Vec<MonoType> = param_tys
                                    .iter()
                                    .map(|t| MonoType::from(t.clone()))
                                    .collect();
                                (pts, MonoType::from(*return_type.clone()))
                            } else {
                                let pts: Vec<MonoType> = params
                                    .iter()
                                    .map(|p| {
                                        p.ty.as_ref()
                                            .map(|t| MonoType::from(t.clone()))
                                            .unwrap_or_else(|| self.env.solver().new_var())
                                    })
                                    .collect();
                                (pts, self.env.solver().new_var())
                            }
                        } else {
                            let pts: Vec<MonoType> = params
                                .iter()
                                .map(|p| {
                                    p.ty.as_ref()
                                        .map(|t| MonoType::from(t.clone()))
                                        .unwrap_or_else(|| self.env.solver().new_var())
                                })
                                .collect();
                            (pts, self.env.solver().new_var())
                        }
                    }
                } else if let Some(type_ann) = type_annotation {
                    if let crate::frontend::core::parser::ast::Type::Fn {
                        params: param_tys,
                        return_type,
                    } = type_ann
                    {
                        let pts: Vec<MonoType> = param_tys
                            .iter()
                            .map(|t| MonoType::from(t.clone()))
                            .collect();
                        (pts, MonoType::from(*return_type.clone()))
                    } else {
                        let pts: Vec<MonoType> = params
                            .iter()
                            .map(|p| {
                                p.ty.as_ref()
                                    .map(|t| MonoType::from(t.clone()))
                                    .unwrap_or_else(|| self.env.solver().new_var())
                            })
                            .collect();
                        (pts, self.env.solver().new_var())
                    }
                } else {
                    let pts: Vec<MonoType> = params
                        .iter()
                        .map(|p| {
                            p.ty.as_ref()
                                .map(|t| MonoType::from(t.clone()))
                                .unwrap_or_else(|| self.env.solver().new_var())
                        })
                        .collect();
                    (pts, self.env.solver().new_var())
                };

                let fn_ty = MonoType::Fn {
                    params: param_types.clone(),
                    return_type: Box::new(return_type),
                    is_async: false,
                };

                // 如果有 type_name（显式方法绑定），使用 add_fn_binding
                if type_name.is_some() {
                    self.env
                        .add_fn_binding(name, type_name.as_deref(), fn_ty.clone());
                } else {
                    // 否则使用普通的 add_var
                    self.env
                        .add_var(name.clone(), PolyType::mono(fn_ty.clone()));
                }

                // 处理 pub 自动绑定
                if *is_pub {
                    self.auto_bind_to_type(name, &param_types, fn_ty);
                }
            }
            crate::frontend::core::parser::ast::StmtKind::Use {
                path, items, alias, ..
            } => {
                // 计算导入模式
                // use std.io → register as "io.print"
                // use std.io as str → register as "str.print"
                // use std.{print} → register as "print"
                // use std.{print} as p → register as "p"
                // use std.{print, read} → register as "print", "read"
                // use std.{print, read} as p, r → register as "p", "r"
                let import_all = items.is_none();
                let aliases = alias.as_ref();

                // 通过 ModuleRegistry 查找模块导出，不再硬编码特定模块
                if let Some(module) = self.env.module_registry.get(path).cloned() {
                    let items_ref = items.as_ref();

                    // 收集需要导入的导出
                    let mut exports_to_import: Vec<&crate::frontend::module::Export> = Vec::new();
                    for export in module.exports.values() {
                        let should_import = import_all
                            || items_ref.is_some_and(|i| i.iter().any(|s| s == &export.name));
                        if should_import {
                            exports_to_import.push(export);
                        }
                    }

                    // 根据别名情况注册
                    match (items.as_ref(), aliases) {
                        // use path (无 items，无 alias) → 提取 path 最后部分作为模块别名
                        (None, None) => {
                            let module_alias = path.split('.').next_back().unwrap_or(path);
                            // 首先将模块本身注册为 Struct 类型（包含所有导出作为字段）
                            self.register_module_as_struct(path, module_alias, &module);
                            // 然后注册每个导出
                            for export in exports_to_import {
                                self.register_use_export(module_alias, export, true);
                            }
                        }
                        // use path as alias → 整个模块用别名注册
                        (None, Some(aliases)) if aliases.len() == 1 => {
                            let alias_name = &aliases[0];
                            for export in exports_to_import {
                                self.register_use_export(alias_name, export, true);
                            }
                        }
                        // use path.{a, b} 无 alias → 展平注册
                        (Some(item_names), None) => {
                            for (item_name, export) in
                                item_names.iter().zip(exports_to_import.iter())
                            {
                                self.register_use_export(item_name, export, false);
                            }
                        }
                        // use path.{a, b} as alias1, alias2 → 嵌套注册
                        (Some(item_names), Some(aliases)) if item_names.len() == aliases.len() => {
                            for (alias_name, export) in aliases.iter().zip(exports_to_import.iter())
                            {
                                self.register_use_export(alias_name, export, true);
                            }
                        }
                        // 其他情况：报错或回退
                        _ => {
                            for export in exports_to_import {
                                self.register_use_export(path, export, false);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// 将模块注册为 Struct 类型（包含所有导出作为字段）
    fn register_module_as_struct(
        &mut self,
        _module_path: &str,
        module_alias: &str,
        module: &crate::frontend::module::ModuleInfo,
    ) {
        let mut fields = Vec::new();
        for field_name in module.exports.keys() {
            let field_ty = MonoType::Fn {
                params: vec![self.env.solver().new_var()],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            };
            fields.push((field_name.clone(), field_ty));
        }
        let module_ty = MonoType::Struct(crate::frontend::core::types::base::mono::StructType {
            name: module_alias.to_string(),
            fields,
            methods: HashMap::new(),
            field_mutability: Vec::new(),
            field_has_default: Vec::new(),
            interfaces: vec![],
        });
        self.env
            .add_var(module_alias.to_string(), PolyType::mono(module_ty));
    }

    /// 注册单个导出
    /// - `use_alias`: 是否使用别名模式，为 true 时注册名为 prefix，否则为 export.name
    fn register_use_export(
        &mut self,
        prefix: &str,
        export: &crate::frontend::module::Export,
        use_alias: bool,
    ) {
        let register_name = if use_alias {
            prefix.to_string()
        } else {
            export.name.clone()
        };

        match export.kind {
            crate::frontend::module::ExportKind::SubModule => {
                // 子模块作为命名空间
                let sub_module_path = export.full_path.clone();
                let mut fields = Vec::new();
                if let Some(sub_module) = self.env.module_registry.get(&sub_module_path).cloned() {
                    for field_name in sub_module.exports.keys() {
                        let field_ty = MonoType::Fn {
                            params: vec![self.env.solver().new_var()],
                            return_type: Box::new(MonoType::Void),
                            is_async: false,
                        };
                        fields.push((field_name.clone(), field_ty));
                    }
                }
                let module_ty =
                    MonoType::Struct(crate::frontend::core::types::base::mono::StructType {
                        name: export.name.clone(),
                        fields,
                        methods: HashMap::new(),
                        field_mutability: Vec::new(),
                        field_has_default: Vec::new(),
                        interfaces: vec![],
                    });
                self.env.add_var(register_name, PolyType::mono(module_ty));
            }
            _ => {
                // 如果变量已存在（比如已经是 Struct 类型），则跳过
                if self.env.get_var(&register_name).is_some() {
                    return;
                }
                let fn_ty = MonoType::Fn {
                    params: vec![self.env.solver().new_var()],
                    return_type: Box::new(MonoType::Void),
                    is_async: false,
                };
                self.env.add_var(register_name, PolyType::mono(fn_ty));
            }
        }
    }

    /// 添加类型定义
    fn add_type_definition(
        &mut self,
        name: &str,
        definition: &crate::frontend::core::parser::ast::Type,
        generic_params: &[String],
        span: crate::util::span::Span,
    ) {
        // RFC-010 Easter Egg: Type: Type = Type
        // 当用户尝试定义 Type 自身时，触发彩蛋
        if name == "Type" {
            // 检查 definition 是否引用了 Type
            let is_type_self_ref = match definition {
                // 情况1: definition 是 MetaType（Type: Type = ...）
                crate::frontend::core::parser::ast::Type::MetaType { .. } => true,
                // 情况2: definition 是 Name("Type")（Type: Type = Type）
                crate::frontend::core::parser::ast::Type::Name { name, .. } => name == "Type",
                // 情况3: definition 是 Generic { name: "Type", ... }（Type: Type = Type[T]）
                crate::frontend::core::parser::ast::Type::Generic {
                    name: type_name, ..
                } => type_name == "Type",
                // 情况4: definition 是 NamedStruct { name: "Type", ... }
                crate::frontend::core::parser::ast::Type::NamedStruct {
                    name: type_name, ..
                } => type_name == "Type",
                _ => false,
            };

            if is_type_self_ref {
                // 检查 type_annotation 是否有泛型参数（这表示 Type: Type[T] = ...）
                if !generic_params.is_empty() {
                    // E1091: 泛型元类型自引用错误
                    let decl = format!("Type: Type({}) = ...", generic_params.join(", "));
                    self.add_error(
                        ErrorCodeDefinition::invalid_generic_self_reference(&decl)
                            .at(span)
                            .build(),
                    );
                    return;
                }

                // E1090: Type: Type = Type 彩蛋（Note 级别）
                self.add_error(
                    ErrorCodeDefinition::type_self_reference_easter_egg()
                        .at(span)
                        .severity(crate::util::diagnostic::Severity::Info)
                        .build(),
                );
                return;
            }
        }

        let poly = PolyType::mono(MonoType::from(definition.clone()));
        self.env.add_type(name.to_string(), poly);

        // 自动为 Record 类型派生标准库 traits
        self.auto_derive_traits(name, definition);
    }

    /// 为 Record 类型自动派生标准库 traits
    ///
    /// 规则：
    /// 1. Record 的所有字段都实现了某 trait → 自动派生该 trait
    /// 2. 显式定义的方法会覆盖自动派生
    fn auto_derive_traits(
        &mut self,
        type_name: &str,
        definition: &crate::frontend::core::parser::ast::Type,
    ) {
        // 提取字段列表
        let fields = match definition {
            crate::frontend::core::parser::ast::Type::NamedStruct { fields, .. } => fields,
            crate::frontend::core::parser::ast::Type::Struct { fields, .. } => fields,
            _ => return, // 非 Record 类型不自动派生
        };

        // 获取 trait 表的引用（用于检查）
        let trait_table = &self.env.trait_table;

        // 为每个内置可派生 trait 尝试自动派生
        let mut impls_to_add = Vec::new();

        for trait_name in auto_derive::BUILTIN_DERIVES {
            // 检查是否可以自动派生
            let can_derive = auto_derive::can_auto_derive(trait_table, trait_name, fields);

            if can_derive {
                // 检查是否已有显式实现
                if !self.env.has_trait_impl(trait_name, type_name) {
                    // 生成自动派生实现
                    if let Some(impl_) = auto_derive::generate_auto_derive(type_name, trait_name) {
                        impls_to_add.push(impl_);
                    }
                }
            }
        }

        // 批量添加实现（避免借用冲突）
        for impl_ in impls_to_add {
            self.env.add_trait_impl(impl_);
        }
    }

    /// 自动将函数绑定到类型
    /// pub 函数的默认行为：绑定到第一个参数的类型
    /// 例如: pub distance: (p1: Point, p2: Point) -> Float 自动绑定为 Point.distance
    fn auto_bind_to_type(
        &mut self,
        fn_name: &str,
        param_types: &[MonoType],
        fn_type: MonoType,
    ) {
        if param_types.is_empty() {
            // 无参数函数无法自动绑定（工厂函数模式需要特殊处理）
            return;
        }

        // 获取第一个参数的类型名称
        let first_param_ty = &param_types[0];
        let type_name = match first_param_ty {
            MonoType::TypeRef(name) => name.clone(),
            _ => return, // 无法确定绑定目标类型
        };

        // 检查该类型是否在当前模块中定义
        if self.env.types.contains_key(&type_name) {
            // 绑定方法到类型
            self.env.add_method_binding(&type_name, fn_name, fn_type);
        }
    }

    /// 收集模块的所有导出项
    fn collect_exports(
        &mut self,
        module: &Module,
    ) {
        use crate::frontend::core::parser::ast::StmtKind;
        for stmt in &module.items {
            if let StmtKind::Binding {
                name,
                type_name,
                type_annotation,

                body,
                is_pub,
                ..
            } = &stmt.kind
            {
                // 方法绑定
                let is_method = type_name.is_some();

                // 函数定义：body 有 tail expression
                let has_body = body.1.is_some() || !body.0.is_empty();
                // 类型定义：没有 body 且有 type_annotation
                let is_type_def = !has_body && type_annotation.is_some();

                // 类型定义始终导出，方法绑定始终导出，函数仅 pub 导出
                if is_type_def || is_method || *is_pub {
                    if is_method {
                        // 方法绑定导出为 Type.method 格式
                        if let Some(ty_name) = type_name {
                            self.env.add_export(&format!("{}.{}", ty_name, name));
                        }
                    } else {
                        self.env.add_export(name);
                    }
                }
            }
        }
    }

    // ============ 语义信息收集 ============

    /// 从已完成类型检查的 AST 收集语义 tokens
    ///
    /// 利用 typecheck 阶段已有的类型信息，一次遍历产出语义数据。
    /// 收集规则：
    /// - StmtKind::Binding   → Function/Type (定义，区分 type_annotation)
    /// - StmtKind::Var       → Variable (定义)
    /// - StmtKind::Binding   → Method/Type/Function (通过字段区分)
    /// - StmtKind::Use       → Namespace (引用)
    /// - Param               → Parameter (定义)
    /// - Expr::Var           → Variable (引用)
    /// - Expr::Call          → Function (引用)
    /// - Expr::FieldAccess   → Property (引用)
    /// - Expr::Cast          → Type (引用)
    fn constructor_names_from_module(module: &Module) -> HashSet<String> {
        use crate::frontend::core::parser::ast::{StmtKind, Type};

        let mut names = HashSet::new();
        for stmt in &module.items {
            if let StmtKind::Binding {
                type_annotation: Some(Type::Variant(variants)),
                ..
            } = &stmt.kind
            {
                for v in variants {
                    names.insert(v.name.clone());
                }
            }
        }
        names
    }

    fn add_use_module_root(
        &self,
        imported_module_roots: &mut HashSet<String>,
        path: &str,
        items: &Option<Vec<String>>,
        alias: &Option<Vec<String>>,
    ) {
        if items.is_some() {
            return;
        }

        if self.env.module_registry.has_module(path) {
            if let Some(aliases) = alias {
                if aliases.len() == 1 {
                    imported_module_roots.insert(aliases[0].clone());
                    return;
                }
            }

            if let Some(last) = path.split('.').next_back() {
                if !last.is_empty() {
                    imported_module_roots.insert(last.to_string());
                }
            }
            return;
        }

        // use std.io.print / use std.io.print as p 属于符号导入，不是命名空间根
        if let Some(dot_pos) = path.rfind('.') {
            let module_path = &path[..dot_pos];
            if self.env.module_registry.has_module(module_path) {
                return;
            }
        }

        // 回退策略：未知路径按旧行为处理
        if let Some(aliases) = alias {
            if aliases.len() == 1 {
                imported_module_roots.insert(aliases[0].clone());
                return;
            }
        }
        if let Some(last) = path.split('.').next_back() {
            if !last.is_empty() {
                imported_module_roots.insert(last.to_string());
            }
        }
    }

}
include!("semantic_tokens_impl.rs");
