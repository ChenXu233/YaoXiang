//! 类型检查器模块
//!
//! 包含 TypeChecker 的完整实现

use std::collections::{HashMap, HashSet};

use crate::frontend::core::parser::ast::Module;
use crate::frontend::core::types::{MonoType, PolyType, TraitTable};
use crate::frontend::core::types::eval::const_eval::ConstFunction;
use crate::frontend::core::types::const_data::{ConstExpr, ConstValue, BinOp};
use crate::frontend::core::typecheck::predicate_resolver::PredicateResolver;
use crate::frontend::core::typecheck::proof::context::ProofContext;
use crate::frontend::core::typecheck::proof::verdict::ProofResult;
use crate::frontend::core::typecheck::layers::predicate::check_predicate;
use crate::frontend::core::types::eval::dependent_types::DependentTypeEnv;
use crate::std::StdModule;

use super::inference;
use super::semantic_db;
use crate::frontend::core::spawn;
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
    /// 依赖类型环境（类型族注册与查找）
    pub dependent_type_env: DependentTypeEnv,
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

        // 初始化依赖类型环境并通过 std::assert 注册类型族
        let mut dependent_type_env = DependentTypeEnv::new();
        crate::std::assert::AssertModule.register_type_families(&mut dependent_type_env);

        Self {
            env,
            body_checker: None,
            semantic_db: semantic_db::SemanticDB::new(),
            dependent_type_env,
        }
    }

    /// 注册预定义的 const 函数
    /// 这些函数用于值依赖类型的编译期求值
    fn register_predefined_const_functions(env: &mut TypeEnvironment) {
        // 注册 factorial 函数
        let factorial = ConstFunction::new(
            "factorial".to_string(),
            vec!["n".to_string()],
            ConstExpr::If {
                condition: Box::new(ConstExpr::BinOp {
                    op: BinOp::Le,
                    left: Box::new(ConstExpr::NamedVar("n".to_string())),
                    right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
                }),
                then_branch: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
                else_branch: Box::new(ConstExpr::BinOp {
                    op: BinOp::Mul,
                    left: Box::new(ConstExpr::NamedVar("n".to_string())),
                    right: Box::new(ConstExpr::Call {
                        func: "factorial".to_string(),
                        args: vec![ConstExpr::BinOp {
                            op: BinOp::Sub,
                            left: Box::new(ConstExpr::NamedVar("n".to_string())),
                            right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
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
                    op: BinOp::Le,
                    left: Box::new(ConstExpr::NamedVar("n".to_string())),
                    right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
                }),
                then_branch: Box::new(ConstExpr::NamedVar("n".to_string())),
                else_branch: Box::new(ConstExpr::BinOp {
                    op: BinOp::Add,
                    left: Box::new(ConstExpr::Call {
                        func: "fibonacci".to_string(),
                        args: vec![ConstExpr::BinOp {
                            op: BinOp::Sub,
                            left: Box::new(ConstExpr::NamedVar("n".to_string())),
                            right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
                        }],
                    }),
                    right: Box::new(ConstExpr::Call {
                        func: "fibonacci".to_string(),
                        args: vec![ConstExpr::BinOp {
                            op: BinOp::Sub,
                            left: Box::new(ConstExpr::NamedVar("n".to_string())),
                            right: Box::new(ConstExpr::Lit(ConstValue::Int(2))),
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
    ) -> TypeCheckResult {
        self.check_module_impl(module, false)
    }

    /// 检查整个模块（收集所有错误模式）
    ///
    /// 启用错误收集模式后，类型检查器会尽可能多地收集错误，
    /// 而不是在第一个错误处停止。适用于 LSP 诊断场景。
    pub fn check_module_collect_all(
        &mut self,
        module: &Module,
    ) -> TypeCheckResult {
        self.check_module_impl(module, true)
    }

    /// 检查整个模块的内部实现
    fn check_module_impl(
        &mut self,
        module: &Module,
        collect_all: bool,
    ) -> TypeCheckResult {
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
                    body,
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

        // RFC-024: spawn 位置检查
        for err in spawn::placement::check_spawn_placement(module) {
            self.add_error(err);
        }

        // 初始化函数体检查器
        let mut body_checker = inference::StatementChecker::new(
            self.env.solver(),
            None,
            self.dependent_type_env.clone(),
        );
        // 设置 native 函数签名表
        body_checker.set_native_signatures(self.env.native_signatures.clone());
        // 设置模块注册表，支持函数体/块作用域 use
        body_checker.set_module_registry(self.env.module_registry.clone());
        // 设置泛型类型定义模板表
        body_checker.set_generic_type_defs(self.env.generic_type_defs.clone());
        // 设置方法绑定表
        body_checker.set_method_bindings(self.env.method_bindings.clone());
        // 设置类型定义表（用于 TypeRef → Struct 解析）
        let type_defs: HashMap<String, MonoType> = self
            .env
            .types
            .iter()
            .map(|(name, poly)| (name.clone(), poly.body.clone()))
            .collect();
        body_checker.set_type_defs(type_defs);
        // 如果启用收集模式，设置收集所有错误
        if collect_all {
            body_checker.set_collect_all_errors(true);
        }
        *self.body_checker_mut() = body_checker;

        // 将环境中的变量同步到 body_checker
        for (name, poly) in self.env.vars.clone() {
            self.body_checker_mut()
                .add_var(name, poly, false, crate::util::span::Span::default());
        }

        // 第三遍：检查所有语句（包括函数体）
        for stmt in &module.items {
            if let Err(e) = self.body_checker_mut().check_stmt(stmt) {
                self.add_error(*e);
            }
        }

        // Phase 2.5: 检查精化类型绑定
        // 遍历所有语句，对变量绑定的精化类型执行证明检查
        let mut proof_calls = Vec::new();
        self.collect_refined_binding_checks(module, &mut proof_calls);

        // 收集 body_checker 中累积的错误（收集模式下产生的）
        if let Some(ref mut bc) = self.body_checker {
            for err in bc.drain_collected_errors() {
                self.env.errors.add_error(err);
            }
        }

        // RFC-027: 终止检查 — 在类型检查之后、约束求解之前运行
        // 分析循环和递归函数，自动证明终止性
        let term_results = {
            let mut term_checker = super::layers::termination::TerminationChecker::new();
            term_checker.check_module(module, self.env())
        };
        for result in term_results {
            match result.into_result() {
                Ok(()) => {} // 证明通过，无需诊断
                Err(diag) => self.add_error(diag),
            }
        }

        // RFC-027: 所有权检查 — 在终止检查之后、约束求解之前运行
        // 分析借用令牌冲突、Move/Drop/Clone/Mut 语义（RFC-009a §系统谓词清单）
        let (release_plan, escaped_refs) = {
            let mut ownership_checker = super::layers::ownership::OwnershipChecker::new();
            let (ownership_results, plan, escaped_refs) =
                ownership_checker.check_module(module, self.env());
            for result in ownership_results {
                match result {
                    ProofResult::Proved => {}
                    ProofResult::Disproved(model) => {
                        self.add_error(model.into_diagnostic());
                    }
                    ProofResult::Unproven { .. } => {}
                }
            }
            (plan, escaped_refs)
        };

        // 求解所有约束
        let solve_result = self.env.solver().solve();
        if let Err(constraint_errors) = solve_result {
            for e in constraint_errors {
                let mut diag = e.error;
                diag.span = Some(e.span);
                self.add_error(diag);
            }
        }

        // 语义收集：遍历 AST 构建 SemanticDB
        // 即便类型检查存在错误（如语法或类型错误），我们也要尽可能收集当前的语义 token，保证代码染色等功能
        self.collect_semantic_tokens(module);

        // 收集错误（无论有无错误都收进 result.diagnostics）
        let diagnostics = self.errors().to_vec();

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
            let is_function =
                matches!(poly.body, crate::frontend::core::types::MonoType::Fn { .. });
            if !is_function && !local_var_types.contains_key(name) {
                local_var_types.insert(name.clone(), poly.body.clone());
            }
        }

        // 注意：由于 body_checker.solver 是克隆的，无法通过 solver.resolve() 来解析类型变量。
        // 幸运的是，assign_var 方法已经将更新后的类型写回到了 scope 中，
        // 所以这里直接使用 scope 中的类型即可，不需要额外 resolve。
        // （注：如果后续需要支持更复杂的泛型推导，可能需要重新设计 solver 的共享机制）

        // 从 body_checker 收集实例化请求
        let instantiation_requests = if let Some(ref bc) = self.body_checker {
            bc.instantiation_requests.clone()
        } else {
            Vec::new()
        };

        TypeCheckResult {
            module_name: self.env.module_name.clone(),
            diagnostics,
            bindings,
            local_var_types,
            semantic_db: std::mem::take(&mut self.semantic_db),
            trait_table: self.env.trait_table.clone(),
            proof_calls, // Phase 2.5: 由 check_refined_binding 收集
            release_plan,
            escaped_refs,
            instantiation_requests,
        }
    }

    /// 获取 body_checker 的可变引用
    fn body_checker_mut(&mut self) -> &mut inference::StatementChecker {
        if self.body_checker.is_none() {
            let mut body_checker = inference::StatementChecker::new(
                self.env.solver(),
                None,
                self.dependent_type_env.clone(),
            );
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
                    };

                    // RFC-027: 解析类型标注中的编译期谓词
                    let fn_ty = match fn_ty {
                        MonoType::Fn {
                            params,
                            return_type,
                        } => MonoType::Fn {
                            params: params
                                .into_iter()
                                .map(|p| self.resolve_type_annotation(&p))
                                .collect(),
                            return_type: Box::new(self.resolve_type_annotation(&return_type)),
                        },
                        other => other,
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
                            };

                            // RFC-027: 解析类型标注中的编译期谓词
                            let fn_ty = match fn_ty {
                                MonoType::Fn {
                                    params,
                                    return_type,
                                } => MonoType::Fn {
                                    params: params
                                        .into_iter()
                                        .map(|p| self.resolve_type_annotation(&p))
                                        .collect(),
                                    return_type: Box::new(
                                        self.resolve_type_annotation(&return_type),
                                    ),
                                },
                                other => other,
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
                generic_params,
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

                // 泛型函数处理：
                // 当 generic_params 包含 Type 级别的参数时，外层 Fn 的前 N 个参数
                // 是类型级参数（如 (T: Type)），return_type 才是实际的值级函数类型。
                // 需要剥离类型级参数，并将 TypeRef("T") 替换为新的类型变量。
                let type_generic_params: Vec<_> = generic_params
                    .iter()
                    .filter(|p| {
                        matches!(
                            p.kind,
                            crate::frontend::core::parser::ast::GenericParamKind::Type
                        )
                    })
                    .collect();

                let (final_param_types, final_return_type) = if !type_generic_params.is_empty()
                    && param_types.len() >= type_generic_params.len()
                {
                    // 为每个泛型类型参数创建新的类型变量
                    let mut subst = HashMap::new();
                    for gp in &type_generic_params {
                        let fresh_var = self.env.solver().new_var();
                        subst.insert(gp.name.clone(), fresh_var);
                    }

                    // 剥离类型级参数，使用 return_type 作为实际函数类型
                    let inner_fn_ty = Self::substitute_type_refs(return_type.clone(), &subst);

                    match inner_fn_ty {
                        MonoType::Fn {
                            params: inner_params,
                            return_type: inner_ret,
                            ..
                        } => (inner_params, *inner_ret),
                        // return_type 不是 Fn（可能是单值泛型），保持原样
                        _ => (param_types, return_type),
                    }
                } else {
                    (param_types, return_type)
                };

                let fn_ty = MonoType::Fn {
                    params: final_param_types.clone(),
                    return_type: Box::new(final_return_type),
                };

                // RFC-027: 解析类型标注中的编译期谓词（如 Positive(5) -> Refined）
                let fn_ty = match fn_ty {
                    MonoType::Fn {
                        params,
                        return_type,
                    } => MonoType::Fn {
                        params: params
                            .into_iter()
                            .map(|p| self.resolve_type_annotation(&p))
                            .collect(),
                        return_type: Box::new(self.resolve_type_annotation(&return_type)),
                    },
                    other => other,
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
                    self.auto_bind_to_type(name, &final_param_types, fn_ty);
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
            crate::frontend::core::parser::ast::StmtKind::ExternalBindingStmt {
                type_name,
                method_name,
                binding,
            } => {
                // 外部方法绑定: Point.distance = distance[0] 或 Point.calc = calculate[1, 2]
                // 查找函数的类型并注册为方法绑定
                let (func_name, positions) = match binding {
                    crate::frontend::core::parser::ast::BindingKind::External {
                        function,
                        positions,
                    } => (function.clone(), positions.clone()),
                    crate::frontend::core::parser::ast::BindingKind::DefaultExternal {
                        function,
                    } => (function.clone(), vec![0]),
                    _ => return,
                };
                if let Some(poly) = self.env.get_var(&func_name) {
                    let method_ty = if positions.len() <= 1 {
                        // Single position [0] or default: use the full function type directly
                        poly.body.clone()
                    } else {
                        // Multi-position [1, 2]: filter out bound params
                        // The resulting method type has params minus the bound positions
                        match &poly.body {
                            MonoType::Fn {
                                params,
                                return_type,
                            } => {
                                let mut new_params: Vec<MonoType> = Vec::new();
                                for (i, p) in params.iter().enumerate() {
                                    if !positions.contains(&(i as i64)) {
                                        new_params.push(p.clone());
                                    }
                                }
                                MonoType::Fn {
                                    params: new_params,
                                    return_type: return_type.clone(),
                                }
                            }
                            other => other.clone(),
                        }
                    };
                    self.env
                        .add_method_binding(type_name, method_name, method_ty);
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
            };
            fields.push((field_name.clone(), field_ty));
        }
        let module_ty = MonoType::Struct(crate::frontend::core::types::mono::StructType {
            name: module_alias.to_string(),
            fields,
            methods: HashMap::new(),
            field_mutability: Vec::new(),
            field_has_default: Vec::new(),
            interfaces: vec![],
            constraints: Vec::new(),
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
                        };
                        fields.push((field_name.clone(), field_ty));
                    }
                }
                let module_ty = MonoType::Struct(crate::frontend::core::types::mono::StructType {
                    name: export.name.clone(),
                    fields,
                    methods: HashMap::new(),
                    field_mutability: Vec::new(),
                    field_has_default: Vec::new(),
                    interfaces: vec![],
                    constraints: Vec::new(),
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
                // 无泛型参数 → 静默跳过（#161 宇宙分层决策）
                return;
            }
        }

        let poly = PolyType::mono(MonoType::from(definition.clone()));
        // Inject the type name into StructType if it's missing (plain Type::Struct has no name)
        let poly = PolyType::mono(match &poly.body {
            MonoType::Struct(s) if s.name.is_empty() => {
                MonoType::Struct(crate::frontend::core::types::mono::StructType {
                    name: name.to_string(),
                    fields: s.fields.clone(),
                    methods: s.methods.clone(),
                    field_mutability: s.field_mutability.clone(),
                    field_has_default: s.field_has_default.clone(),
                    interfaces: s.interfaces.clone(),
                    constraints: s.constraints.clone(),
                })
            }
            _ => poly.body.clone(),
        });
        self.env.add_type(name.to_string(), poly.clone());

        // 如果是泛型类型构造器（有泛型参数），存储模板信息用于类型实例化
        if !generic_params.is_empty() {
            use crate::frontend::core::typecheck::environment::GenericTypeDef;
            use crate::frontend::core::types::var::TypeVar;

            let type_param_names: Vec<String> = generic_params.to_vec();
            let type_binders: Vec<TypeVar> =
                (0..type_param_names.len()).map(TypeVar::new).collect();

            let def = GenericTypeDef {
                poly: PolyType {
                    type_binders,
                    const_binders: vec![],
                    body: poly.body.clone(),
                },
                type_param_names,
            };
            self.env.add_generic_type_def(name.to_string(), def);
        }

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

        for trait_name in TraitTable::BUILTIN_DERIVES {
            // 检查是否可以自动派生
            let can_derive = trait_table.can_auto_derive(trait_name, fields);

            if can_derive {
                // 检查是否已有显式实现
                if !self.env.has_trait_impl(trait_name, type_name) {
                    // 生成自动派生实现
                    if let Some(impl_) = TraitTable::generate_auto_derive(type_name, trait_name) {
                        impls_to_add.push(impl_);
                    }
                }
            }
        }

        // 批量添加实现（避免借用冲突）
        for impl_ in impls_to_add {
            // auto_derive 已有 has_trait_impl 前置检查，这里不会冲突
            let _ = self.env.add_trait_impl(impl_);
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

                // 函数定义：body 有语句
                let has_body = !body.is_empty();
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

    /// 替换 MonoType 中的 TypeRef 名称为对应的类型变量
    ///
    /// 用于泛型函数类型推断：将 TypeRef("T") 替换为 solver 中的新类型变量。
    fn substitute_type_refs(
        ty: MonoType,
        subst: &HashMap<String, MonoType>,
    ) -> MonoType {
        match ty {
            MonoType::TypeRef(name) => subst.get(&name).cloned().unwrap_or(MonoType::TypeRef(name)),
            MonoType::Fn {
                params,
                return_type,
            } => MonoType::Fn {
                params: params
                    .into_iter()
                    .map(|p| Self::substitute_type_refs(p, subst))
                    .collect(),
                return_type: Box::new(Self::substitute_type_refs(*return_type, subst)),
            },
            MonoType::List(inner) => {
                MonoType::List(Box::new(Self::substitute_type_refs(*inner, subst)))
            }
            MonoType::Option(inner) => {
                MonoType::Option(Box::new(Self::substitute_type_refs(*inner, subst)))
            }
            MonoType::Result(ok, err) => MonoType::Result(
                Box::new(Self::substitute_type_refs(*ok, subst)),
                Box::new(Self::substitute_type_refs(*err, subst)),
            ),
            MonoType::Tuple(types) => MonoType::Tuple(
                types
                    .into_iter()
                    .map(|t| Self::substitute_type_refs(t, subst))
                    .collect(),
            ),
            MonoType::Dict(k, v) => MonoType::Dict(
                Box::new(Self::substitute_type_refs(*k, subst)),
                Box::new(Self::substitute_type_refs(*v, subst)),
            ),
            MonoType::Arc(inner) => {
                MonoType::Arc(Box::new(Self::substitute_type_refs(*inner, subst)))
            }
            MonoType::Range { elem_type } => MonoType::Range {
                elem_type: Box::new(Self::substitute_type_refs(*elem_type, subst)),
            },
            // 其他类型（Int, Float, Bool, String, Void, Struct, Enum, TypeVar, TypeRef 等）保持不变
            other => other,
        }
    }

    // ============ RFC-027 阶段 1：编译期谓词集成 ============

    /// 解析类型标注：如果是编译期谓词调用，正格化为 Refined
    ///
    /// Generic("Positive", [arg]) -> 尝试 PredicateResolver::try_resolve
    /// 如果不是已知的编译期谓词，检查是否是证明函数
    fn resolve_type_annotation(
        &self,
        ty: &MonoType,
    ) -> MonoType {
        match ty {
            MonoType::Generic { name, args } if !args.is_empty() => {
                // 尝试原有 PredicateResolver
                if let Some(refined) = PredicateResolver::try_resolve(&self.env, name, args) {
                    return refined;
                }
                // Phase 2.5: 检查是否是证明函数（源码定义的返回 Type 的函数）
                if let Some(base) = self.lookup_proof_fn_base_type(name, args) {
                    let constraint = ConstExpr::Call {
                        func: name.clone(),
                        args: args
                            .iter()
                            .map(|a| {
                                self.mono_type_to_const_expr(a)
                                    .unwrap_or(ConstExpr::Lit(ConstValue::Int(0)))
                            })
                            .collect(),
                    };
                    return MonoType::Refined {
                        base: Box::new(base),
                        constraint,
                    };
                }
                ty.clone()
            }
            _ => ty.clone(),
        }
    }

    /// 查找证明函数的基类型
    ///
    /// 检查 `name` 是否在环境中定义为返回 Type 的函数。
    /// 如果是，返回第一个参数的类型作为基类型。
    fn lookup_proof_fn_base_type(
        &self,
        name: &str,
        args: &[MonoType],
    ) -> Option<MonoType> {
        // 查找函数定义
        let poly = self.env.get_var(name)?;
        let fn_ty = &poly.body;

        // 检查是否是函数类型，且返回类型是 MetaType（Type）
        if let MonoType::Fn {
            params,
            return_type,
        } = fn_ty
        {
            // 检查返回类型是否是 MetaType（表示返回 Type）
            if matches!(return_type.as_ref(), MonoType::MetaType { .. })
                || matches!(return_type.as_ref(), MonoType::TypeRef(ref name) if name == "Type")
            {
                // 检查参数数量是否匹配
                if params.len() == args.len() {
                    // 返回第一个参数的类型作为基类型
                    return params.first().cloned();
                }
            }
        }
        None
    }

    /// 将 MonoType 转换为 ConstExpr
    ///
    /// 用于将类型参数转换为约束表达式中的常量表达式。
    fn mono_type_to_const_expr(
        &self,
        ty: &MonoType,
    ) -> Option<ConstExpr> {
        match ty {
            // 字面量值
            MonoType::Literal { value, .. } => Some(ConstExpr::Lit(value.clone())),
            // 变量引用
            MonoType::TypeRef(name) => Some(ConstExpr::NamedVar(name.clone())),
            // 递归处理 Generic 中的参数
            MonoType::Generic { name: _, args } if args.len() == 1 => {
                self.mono_type_to_const_expr(&args[0])
            }
            _ => None,
        }
    }

    /// 从表达式中提取常量值
    ///
    /// 用于从初始化器中提取值，以便在精化类型检查中使用。
    fn extract_const_value(
        &self,
        expr: &Expr,
    ) -> Option<crate::frontend::core::types::ConstValue> {
        match expr {
            Expr::Lit(literal, _) => match literal {
                crate::frontend::core::parser::ast::Literal::Int(n) => {
                    Some(crate::frontend::core::types::ConstValue::Int(*n))
                }
                crate::frontend::core::parser::ast::Literal::Float(f) => {
                    Some(crate::frontend::core::types::ConstValue::Float(*f as f32))
                }
                crate::frontend::core::parser::ast::Literal::Bool(b) => {
                    Some(crate::frontend::core::types::ConstValue::Bool(*b))
                }
                _ => None,
            },
            // 处理一元负号：-1
            Expr::UnOp {
                op: crate::frontend::core::parser::ast::UnOp::Neg,
                expr: inner,
                ..
            } => {
                if let Some(crate::frontend::core::types::ConstValue::Int(n)) =
                    self.extract_const_value(inner)
                {
                    Some(crate::frontend::core::types::ConstValue::Int(-n))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 两阶段精化类型检查（RFC-027 Phase 3.1）
    ///
    /// 阶段 1：遍历模块，构建 TypeDepGraph + 检查初始化绑定
    /// 阶段 2：遍历赋值点，查询依赖图，生成 VC
    fn collect_refined_binding_checks(
        &self,
        module: &Module,
        proof_calls: &mut Vec<crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall>,
    ) {
        use crate::frontend::core::typecheck::proof::dep_graph::TypeDepGraph;

        let mut dep_graph = TypeDepGraph::new();
        let mut shared_ctx =
            crate::frontend::core::typecheck::proof::context::ProofContext::new(&self.env);

        // 阶段 1：构建依赖图 + 初始绑定检查
        for stmt in &module.items {
            self.build_dep_graph_and_check_init(stmt, &mut dep_graph, &mut shared_ctx, proof_calls);
        }

        // 阶段 2：遍历赋值点，生成 VC
        for stmt in &module.items {
            self.check_assignments_with_deps(stmt, &dep_graph, &mut shared_ctx, proof_calls);
        }
    }

    /// 阶段 1：递归遍历语句树——构建依赖图 + 检查初始化绑定
    fn build_dep_graph_and_check_init(
        &self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        dep_graph: &mut crate::frontend::core::typecheck::proof::dep_graph::TypeDepGraph,
        shared_ctx: &mut crate::frontend::core::typecheck::proof::context::ProofContext<'_>,
        proof_calls: &mut Vec<crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall>,
    ) {
        use crate::frontend::core::parser::ast::StmtKind;

        match &stmt.kind {
            StmtKind::Var {
                name,
                type_annotation: Some(type_ann),
                initializer,
                ..
            } => {
                let mono_ty = MonoType::from(type_ann.clone());
                let resolved_ty = self.resolve_type_annotation(&mono_ty);

                if let MonoType::Refined { constraint, .. } = &resolved_ty {
                    // 构建依赖图：提取约束中的自由变量 → 记录 dep
                    let free_vars = Self::extract_free_vars(constraint);
                    for fv in &free_vars {
                        if fv != name {
                            dep_graph.add_dep(name, fv);
                        }
                    }

                    // 检查初始化绑定
                    let mut bindings = HashMap::new();
                    if let Some(init_expr) = initializer {
                        if let Some(const_val) = self.extract_const_value(init_expr) {
                            bindings.insert(name.clone(), const_val);
                        }
                    }
                    let proof_result =
                        crate::frontend::core::typecheck::layers::predicate::check_predicate(
                            shared_ctx,
                            &resolved_ty,
                            &bindings,
                        );
                    if let ProofResult::Unproven {
                        proof_calls: calls, ..
                    } = &proof_result
                    {
                        if !calls.is_empty() {
                            proof_calls.extend(calls.clone());
                        }
                    }
                }
            }
            StmtKind::Binding { body: stmts, .. } => {
                for s in stmts {
                    self.build_dep_graph_and_check_init(s, dep_graph, shared_ctx, proof_calls);
                }
            }
            StmtKind::Expr(expr) => {
                self.build_dep_graph_from_expr(expr.as_ref(), dep_graph, shared_ctx, proof_calls);
            }
            StmtKind::If {
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                for s in &then_branch.stmts {
                    self.build_dep_graph_and_check_init(s, dep_graph, shared_ctx, proof_calls);
                }
                for (_, body) in elif_branches {
                    for s in &body.stmts {
                        self.build_dep_graph_and_check_init(s, dep_graph, shared_ctx, proof_calls);
                    }
                }
                if let Some(else_body) = else_branch {
                    for s in &else_body.stmts {
                        self.build_dep_graph_and_check_init(s, dep_graph, shared_ctx, proof_calls);
                    }
                }
            }
            StmtKind::For { body, .. } => {
                for s in &body.stmts {
                    self.build_dep_graph_and_check_init(s, dep_graph, shared_ctx, proof_calls);
                }
            }
            _ => {}
        }
    }

    /// 从表达式递归构建依赖图（处理 While/Block/For 等包含语句的表达式）
    fn build_dep_graph_from_expr(
        &self,
        expr: &crate::frontend::core::parser::ast::Expr,
        dep_graph: &mut crate::frontend::core::typecheck::proof::dep_graph::TypeDepGraph,
        shared_ctx: &mut crate::frontend::core::typecheck::proof::context::ProofContext<'_>,
        proof_calls: &mut Vec<crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall>,
    ) {
        match expr {
            crate::frontend::core::parser::ast::Expr::Block(block) => {
                for s in &block.stmts {
                    self.build_dep_graph_and_check_init(s, dep_graph, shared_ctx, proof_calls);
                }
            }
            crate::frontend::core::parser::ast::Expr::While { body, .. } => {
                for s in &body.stmts {
                    self.build_dep_graph_and_check_init(s, dep_graph, shared_ctx, proof_calls);
                }
            }
            crate::frontend::core::parser::ast::Expr::For { body, .. } => {
                for s in &body.stmts {
                    self.build_dep_graph_and_check_init(s, dep_graph, shared_ctx, proof_calls);
                }
            }
            crate::frontend::core::parser::ast::Expr::If {
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                for s in &then_branch.stmts {
                    self.build_dep_graph_and_check_init(s, dep_graph, shared_ctx, proof_calls);
                }
                for (_, body) in elif_branches {
                    for s in &body.stmts {
                        self.build_dep_graph_and_check_init(s, dep_graph, shared_ctx, proof_calls);
                    }
                }
                if let Some(else_body) = else_branch {
                    for s in &else_body.stmts {
                        self.build_dep_graph_and_check_init(s, dep_graph, shared_ctx, proof_calls);
                    }
                }
            }
            _ => {}
        }
    }

    /// 阶段 2：遍历赋值点，查询依赖图，生成 VC
    fn check_assignments_with_deps(
        &self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        dep_graph: &crate::frontend::core::typecheck::proof::dep_graph::TypeDepGraph,
        shared_ctx: &mut crate::frontend::core::typecheck::proof::context::ProofContext<'_>,
        proof_calls: &mut Vec<crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall>,
    ) {
        use crate::frontend::core::parser::ast::StmtKind;

        match &stmt.kind {
            // 赋值语句：x = expr（有 initializer 的 Var）
            StmtKind::Var {
                name,
                initializer: Some(_),
                ..
            } => {
                let affected = dep_graph.affected_by(name);
                if !affected.is_empty() {
                    self.generate_vc_for_dependants(name, &affected, shared_ctx, proof_calls);
                }
            }
            // 表达式赋值：i += 1（BinOp::Assign in expression position）
            StmtKind::Expr(expr) => {
                self.check_assign_expr_with_deps(expr.as_ref(), dep_graph, shared_ctx, proof_calls);
            }
            // 递归处理子语句
            StmtKind::Binding { body: stmts, .. } => {
                for s in stmts {
                    self.check_assignments_with_deps(s, dep_graph, shared_ctx, proof_calls);
                }
            }
            StmtKind::If {
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                for s in &then_branch.stmts {
                    self.check_assignments_with_deps(s, dep_graph, shared_ctx, proof_calls);
                }
                for (_, body) in elif_branches {
                    for s in &body.stmts {
                        self.check_assignments_with_deps(s, dep_graph, shared_ctx, proof_calls);
                    }
                }
                if let Some(else_body) = else_branch {
                    for s in &else_body.stmts {
                        self.check_assignments_with_deps(s, dep_graph, shared_ctx, proof_calls);
                    }
                }
            }
            StmtKind::For { body, .. } => {
                for s in &body.stmts {
                    self.check_assignments_with_deps(s, dep_graph, shared_ctx, proof_calls);
                }
            }
            _ => {}
        }
    }

    /// 检查表达式赋值中的依赖触发（处理 i += 1 等 BinOp::Assign）
    fn check_assign_expr_with_deps(
        &self,
        expr: &crate::frontend::core::parser::ast::Expr,
        dep_graph: &crate::frontend::core::typecheck::proof::dep_graph::TypeDepGraph,
        shared_ctx: &mut crate::frontend::core::typecheck::proof::context::ProofContext<'_>,
        proof_calls: &mut Vec<crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall>,
    ) {
        match expr {
            crate::frontend::core::parser::ast::Expr::BinOp {
                op: crate::frontend::core::parser::ast::BinOp::Assign,
                left,
                ..
            } => {
                if let crate::frontend::core::parser::ast::Expr::Var(var_name, _) = left.as_ref() {
                    let affected = dep_graph.affected_by(var_name);
                    if !affected.is_empty() {
                        self.generate_vc_for_dependants(
                            var_name,
                            &affected,
                            shared_ctx,
                            proof_calls,
                        );
                    }
                }
            }
            // 递归处理子表达式中的语句（While/For body 等）
            crate::frontend::core::parser::ast::Expr::Block(block) => {
                for s in &block.stmts {
                    self.check_assignments_with_deps(s, dep_graph, shared_ctx, proof_calls);
                }
            }
            crate::frontend::core::parser::ast::Expr::While { body, .. } => {
                for s in &body.stmts {
                    self.check_assignments_with_deps(s, dep_graph, shared_ctx, proof_calls);
                }
            }
            crate::frontend::core::parser::ast::Expr::For { body, .. } => {
                for s in &body.stmts {
                    self.check_assignments_with_deps(s, dep_graph, shared_ctx, proof_calls);
                }
            }
            _ => {}
        }
    }

    /// 为依赖变量生成验证条件并送入证明管道
    ///
    /// 当 x 被赋值时，对每个依赖 x 的变量 v：
    /// 1. 从 TypeEnvironment 查找 v 的类型标注
    /// 2. 调用 check_predicate() 验证约束
    fn generate_vc_for_dependants(
        &self,
        assigned_var: &str,
        affected: &[&str],
        shared_ctx: &crate::frontend::core::typecheck::proof::context::ProofContext<'_>,
        proof_calls: &mut Vec<crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall>,
    ) {
        for dependant in affected {
            // 从环境中查找 dependant 的类型
            if let Some(poly_ty) = self.env.get_var(dependant) {
                let mono_ty = &poly_ty.body;

                // 只处理 Refined 类型
                if let MonoType::Refined { constraint, .. } = mono_ty {
                    // 构造 bindings：变量值未知 → SMT 符号化处理
                    let bindings = HashMap::new();

                    let proof_result =
                        crate::frontend::core::typecheck::layers::predicate::check_predicate(
                            shared_ctx, mono_ty, &bindings,
                        );

                    match &proof_result {
                        ProofResult::Proved => {
                            // VC 成立
                        }
                        ProofResult::Disproved(model) => {
                            tracing::warn!(
                                "VC 失败：变量 {} 被赋值后，{} 不满足类型 {}: 反例 {:?}",
                                assigned_var,
                                dependant,
                                constraint,
                                model.assignments,
                            );
                        }
                        ProofResult::Unproven {
                            proof_calls: calls, ..
                        } => {
                            if !calls.is_empty() {
                                proof_calls.extend(calls.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    /// 从 ConstExpr 中提取所有自由变量名
    ///
    /// 遍历约束表达式树，收集所有 NamedVar 引用。
    /// 用于构建 TypeDepGraph 时判断"x 的类型标注引用了 y"。
    fn extract_free_vars(
        expr: &crate::frontend::core::types::const_data::ConstExpr
    ) -> Vec<String> {
        let mut vars = Vec::new();
        Self::collect_free_vars(expr, &mut vars);
        vars
    }

    fn collect_free_vars(
        expr: &crate::frontend::core::types::const_data::ConstExpr,
        out: &mut Vec<String>,
    ) {
        match expr {
            crate::frontend::core::types::const_data::ConstExpr::NamedVar(name) => {
                out.push(name.clone());
            }
            crate::frontend::core::types::const_data::ConstExpr::Var(var) => {
                out.push(var.to_string());
            }
            crate::frontend::core::types::const_data::ConstExpr::BinOp { left, right, .. } => {
                Self::collect_free_vars(left, out);
                Self::collect_free_vars(right, out);
            }
            crate::frontend::core::types::const_data::ConstExpr::UnOp { expr: inner, .. } => {
                Self::collect_free_vars(inner, out);
            }
            crate::frontend::core::types::const_data::ConstExpr::Call { args, .. } => {
                for a in args {
                    Self::collect_free_vars(a, out);
                }
            }
            // Lit, If, Range 不含变量引用
            _ => {}
        }
    }

    /// 对绑定点的精化类型执行谓词检查
    ///
    /// 仅处理 Refined 类型，非 Refined 直接返回 Proved。
    /// 构造 ProofContext 后调用 Layer 3 的 check_predicate。
    #[allow(dead_code)] // Phase 2.5 预留：精化类型绑定点谓词检查
    fn check_refined_binding(
        &self,
        ty: &MonoType,
        bindings: &std::collections::HashMap<String, crate::frontend::core::types::ConstValue>,
    ) -> ProofResult {
        // 只处理 Refined 类型
        if !matches!(ty, MonoType::Refined { .. }) {
            return ProofResult::Proved;
        }

        // 构造 ProofContext
        let ctx = ProofContext::new(&self.env);

        // 调用 Layer 3
        check_predicate(&ctx, ty, bindings)
    }
}
include!("checker/semantic_tokens.rs");
