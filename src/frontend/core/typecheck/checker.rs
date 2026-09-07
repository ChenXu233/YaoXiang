//! 类型检查器模块
//!
//! 包含 TypeChecker 的完整实现

use std::collections::{HashMap, HashSet};

use crate::frontend::core::parser::ast::{
    classify_generic_params, Expr, GenericParamKind, Module, Param, TypeBodyItem,
};
use crate::frontend::core::types::{MonoType, PolyType, TraitTable};
use crate::frontend::core::types::const_data::{ConstExpr, ConstValue, BinOp, ConstKind, ConstVarDef};
use crate::frontend::core::types::eval::const_eval::{ConstFunction, convert_expr_to_const_expr};
use crate::frontend::core::typecheck::predicate_resolver::PredicateResolver;
use crate::frontend::core::typecheck::proof::verdict::ProofResult;
use crate::frontend::core::types::eval::dependent_types::DependentTypeEnv;
use crate::std::StdModule;

use super::inference;
use super::semantic_db;
use crate::frontend::core::spawn;
use super::types::TypeCheckResult;
use super::environment::TypeEnvironment;
use super::environment::ImplementationProof;
use super::{add_builtin_types, add_std_traits, add_native_function_types};
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
    /// 用户模块命名空间别名表（别名 → 模块限定键），由 `use lib` / `use lib as l` 整体导入登记。
    /// 模块解析归 typecheck 所有：IR 生成直接消费此表，不再自行从 AST 重新推导。
    module_namespaces: HashMap<String, String>,
    /// RFC-004: 类型体绑定的待登记队列（类型名, 绑定, span）。
    /// pass1 收集类型定义，此时被绑函数可能尚未注册（pass2 收集签名），
    /// 故 External/DefaultExternal 延迟到 pass2 之后统一登记。
    pending_body_bindings: Vec<(
        String,
        crate::frontend::core::parser::ast::TypeBodyBinding,
        crate::util::span::Span,
    )>,
    /// RFC-011a: 已注册类型定义的 AST 体项（接口展开需要原始应用项；
    /// MonoType 转换会丢弃 Expr/Binding 项，故单独留存）。
    type_definition_bodies: HashMap<String, Vec<crate::frontend::core::parser::ast::TypeBodyItem>>,
    /// RFC-011a: 类型体中的接口实例化待决记录（阶段 2 延迟完整性检查）。
    pending_interface_instantiations: Vec<PendingInterfaceInstantiation>,
    /// RFC-011a §3: 已声明方法签名 (类型名, 方法名) -> 签名。
    /// 同签名重复声明 = 覆盖 → E1100；不同签名 = 重载 → 放行。
    declared_methods: HashMap<(String, String), MonoType>,
}

/// RFC-011a: 类型体应用项 `Animal(Dog)` 的待决接口实例化。
struct PendingInterfaceInstantiation {
    impl_type: String,
    interface_name: String,
    args: Vec<crate::frontend::core::parser::ast::Type>,
    span: crate::util::span::Span,
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
            module_namespaces: HashMap::new(),
            pending_body_bindings: Vec::new(),
            type_definition_bodies: HashMap::new(),
            pending_interface_instantiations: Vec::new(),
            declared_methods: HashMap::new(),
        }
    }

    /// RFC-011a: 已通过的接口实现证明（编译期，运行时擦除）。
    /// LSP/阶段 3 动态分发的类型收集据此枚举某接口的全部实现类型。
    pub fn implementation_proofs(&self) -> &[ImplementationProof] {
        &self.env.implementation_proofs
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

    /// 仅收集模块的顶层签名（RFC-029 多文件编排）。
    ///
    /// 跑 pass1（类型定义）+ pass2（函数/绑定签名），**不检查函数体**。
    /// 用于在构建 Registry 时提取每个文件顶层绑定的真实 `MonoType`。
    /// 函数体里的跨文件引用留到完整三遍（pass3）才检查，彼时 Registry 已就绪。
    pub fn collect_signatures(
        &mut self,
        module: &Module,
    ) {
        // pass1: 类型定义
        for stmt in &module.items {
            // #324：模块级阶段挂当前语句 span，诊断自动获得位置
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            if let crate::frontend::core::parser::ast::StmtKind::TypeDefinition {
                name,
                signature_params,
                definition,
                ..
            } = &stmt.kind
            {
                self.add_type_definition(name, definition, signature_params, stmt.span);
            }
        }
        // pass2: 函数/绑定签名（使其可被前向引用）
        for stmt in &module.items {
            // #324：模块级阶段挂当前语句 span，诊断自动获得位置
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            self.collect_function_signature(stmt);
        }
        // RFC-004: 函数签名就位后登记类型体绑定
        self.flush_pending_body_bindings();
    }

    /// 检查整个模块的内部实现
    fn check_module_impl(
        &mut self,
        module: &Module,
        collect_all: bool,
    ) -> TypeCheckResult {
        // 第一遍：收集所有类型定义
        for stmt in &module.items {
            // #324：模块级阶段挂当前语句 span，诊断自动获得位置
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            if let crate::frontend::core::parser::ast::StmtKind::TypeDefinition {
                name,
                signature_params,
                definition,
                ..
            } = &stmt.kind
            {
                self.add_type_definition(name, definition, signature_params, stmt.span);
            }
        }

        // 第二遍：收集所有函数签名（使其可被前向引用）
        for stmt in &module.items {
            // #324：模块级阶段挂当前语句 span，诊断自动获得位置
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            self.collect_function_signature(stmt);
        }

        // RFC-004: 函数签名就位后登记类型体绑定
        self.flush_pending_body_bindings();

        // RFC-011a 阶段2: 接口实例化完整性检查（Self 替换 + 签名匹配 + 实现证明）
        self.finalize_interface_instantiations();

        // 收集所有导出项
        self.collect_exports(module);

        // RFC-024: spawn 位置检查
        for err in spawn::placement::check_spawn_placement(module) {
            self.add_error(err);
        }

        // 初始化函数体检查器
        let trait_table = self.env.trait_table.clone();
        let mut body_checker = inference::StatementChecker::new(
            self.env.solver(),
            None,
            self.dependent_type_env.clone(),
            trait_table,
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
        // RFC-027 Phase 2.5: 构建证明函数基类型表
        let proof_fn_bases: HashMap<String, MonoType> = self
            .env
            .vars
            .iter()
            .filter_map(|(name, poly)| {
                if let MonoType::Fn {
                    params,
                    return_type,
                } = &poly.body
                {
                    if matches!(return_type.as_ref(), MonoType::MetaType { .. }) {
                        return params.first().map(|base| (name.clone(), base.clone()));
                    }
                }
                None
            })
            .collect();
        body_checker.set_proof_fn_bases(proof_fn_bases);
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
            // #324：模块级阶段挂当前语句 span，诊断自动获得位置
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
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
        // #256：类型账本从推断层移交，供 Move/Dup 分类
        // #265：经证明管线入口 check_ownership，分支守卫注入假设栈
        let (release_plan, escaped_refs) = {
            let ledger = self
                .body_checker
                .as_ref()
                .map(|bc| bc.var_type_ledger().clone())
                .unwrap_or_default();
            let mut proof_ctx =
                crate::frontend::core::typecheck::proof::context::ProofContext::new(&self.env);
            let (ownership_results, plan, escaped_refs) = super::layers::ownership::check_ownership(
                &mut proof_ctx,
                module,
                &self.env,
                &ledger,
            );
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

        // 从 body_checker.vars 获取局部变量类型，并合并三链模型的 globals（#295 重构：
        // 模块级绑定（如 `result = id(42)`）在 ScopeManager.globals，不在 vars() 中）
        if let Some(ref bc) = self.body_checker {
            for (name, poly) in bc.vars() {
                // 只添加 env.vars 中不存在的局部变量类型
                if !bindings.contains_key(&name) {
                    bindings.insert(name.clone(), poly.clone());
                }
                // 收集局部变量的 MonoType（用于 IR 生成器错误消息）
                local_var_types.insert(name, poly.body);
            }
            for (name, info) in bc.scope_globals() {
                if !bindings.contains_key(name) {
                    bindings.insert(name.clone(), info.poly.clone());
                }
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

        // RFC-011a §6: 从 body_checker 收集存在类型强制点（ir_gen 包装注入用）
        let existential_coercions = if let Some(ref bc) = self.body_checker {
            bc.existential_coercions.clone()
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
            proof_calls, // Phase 2.5 预留：证明调用收集
            release_plan,
            escaped_refs,
            instantiation_requests,
            existential_coercions,
            implementation_proofs: self.env.implementation_proofs.clone(),
            module_namespaces: std::mem::take(&mut self.module_namespaces),
        }
    }

    /// 获取 body_checker 的可变引用
    fn body_checker_mut(&mut self) -> &mut inference::StatementChecker {
        if self.body_checker.is_none() {
            let trait_table = self.env.trait_table.clone();
            let mut body_checker = inference::StatementChecker::new(
                self.env.solver(),
                None,
                self.dependent_type_env.clone(),
                trait_table,
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

                    // RFC-027: 解析类型标注中的编译期谓词（#263：诊断汇入后上报）
                    let mut refined_diags = Vec::new();
                    let fn_ty = match fn_ty {
                        MonoType::Fn {
                            params,
                            return_type,
                        } => MonoType::Fn {
                            params: params
                                .into_iter()
                                .map(|p| self.resolve_type_annotation(&p, &mut refined_diags))
                                .collect(),
                            return_type: Box::new(
                                self.resolve_type_annotation(&return_type, &mut refined_diags),
                            ),
                        },
                        other => other,
                    };
                    self.env.errors.extend_errors(refined_diags);

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
                            let mut refined_diags = Vec::new();
                            let fn_ty =
                                match fn_ty {
                                    MonoType::Fn {
                                        params,
                                        return_type,
                                    } => MonoType::Fn {
                                        params: params
                                            .into_iter()
                                            .map(|p| {
                                                self.resolve_type_annotation(&p, &mut refined_diags)
                                            })
                                            .collect(),
                                        return_type: Box::new(self.resolve_type_annotation(
                                            &return_type,
                                            &mut refined_diags,
                                        )),
                                    },
                                    other => other,
                                };
                            self.env.errors.extend_errors(refined_diags);

                            self.env.add_var(name.clone(), PolyType::mono(fn_ty));
                        }
                    }
                }
            }
            crate::frontend::core::parser::ast::StmtKind::Assign {
                target,
                type_annotation,
                signature_params,
                value,
                is_pub,
                ..
            } if value.as_ref().is_some_and(|v| {
                matches!(
                    v.as_ref(),
                    crate::frontend::core::parser::ast::Expr::Lambda { .. }
                        | crate::frontend::core::parser::ast::Expr::Block(..)
                )
            }) || (value.is_none() && type_annotation.is_some()) =>
            {
                // 从 target 提取 name 和 type_name
                let (name, type_name) = match target.as_ref() {
                    crate::frontend::core::parser::ast::Expr::Var(n, _) => (n.clone(), None),
                    crate::frontend::core::parser::ast::Expr::FieldAccess {
                        expr, field, ..
                    } => {
                        if let crate::frontend::core::parser::ast::Expr::Var(tn, _) = expr.as_ref()
                        {
                            {
                                // 语义分流（issue #180 F 组）：base 是类型才是方法定义；
                                // base 是值（实例）→ 字段赋值，pass3 处理，不当方法定义收集。
                                let ty_name = matches!(
                                    self.env.resolve_base_kind(tn),
                                    crate::frontend::core::typecheck::environment::BaseKind::TypeSpace
                                )
                                .then(|| tn.clone());
                                (field.clone(), ty_name)
                            }
                        } else {
                            (field.clone(), None)
                        }
                    }
                    _ => {
                        // 无法从 target 提取名字，跳过
                        return;
                    }
                };
                // 从 value 提取 params 和 body（Lambda 形式）
                let (params, _): (Vec<_>, Vec<_>) = match value {
                    Some(expr) => {
                        if let crate::frontend::core::parser::ast::Expr::Lambda {
                            params: p,
                            body: b,
                            ..
                        } = expr.as_ref()
                        {
                            (p.clone(), b.stmts.clone())
                        } else if let crate::frontend::core::parser::ast::Expr::Block(b) =
                            expr.as_ref()
                        {
                            (Vec::new(), b.stmts.clone())
                        } else {
                            (Vec::new(), Vec::new())
                        }
                    }
                    None => (Vec::new(), Vec::new()),
                };
                let method_type = type_annotation.as_ref();
                let generic_params =
                    classify_generic_params(signature_params, &|name| self.env.has_trait(name));
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

                // === 函数 const 泛型判定（用途分析） ===
                // 候选 = generic_params 中 annotation 为具体类型的参数（形态粗筛产物）。
                // 扫描 curry 后续组的类型标注（内层 Fn 的 params），
                // 被引用的候选 → const。精筛唯一实现在 const_param.rs（RFC-011 §4.1）。
                let candidate_names: HashSet<String> = generic_params
                    .iter()
                    .filter(|p| {
                        matches!(
                            p.kind,
                            crate::frontend::core::parser::ast::GenericParamKind::Const { .. }
                        )
                    })
                    .map(|p| p.name.clone())
                    .collect();

                let mut used_as_const = HashSet::new();
                if !candidate_names.is_empty() {
                    // 扫描内层 Fn 的 params：对于 (N: Int) -> (n: N) -> Int，
                    // type_annotation.return_type 是 Fn { params: [Type::Name("N")], ... }。
                    // N 出现在内层 params 中 → const。
                    if let Some(crate::frontend::core::parser::ast::Type::Fn {
                        return_type, ..
                    }) = type_annotation
                    {
                        if let crate::frontend::core::parser::ast::Type::Fn {
                            params: inner_params,
                            ..
                        } = return_type.as_ref()
                        {
                            for p in inner_params {
                                collect_used_in_type(p, &candidate_names, &mut used_as_const);
                            }
                        }
                        // 也扫描 return_type 自身（深度嵌套场景）
                        collect_used_in_type(return_type, &candidate_names, &mut used_as_const);
                    }
                }

                let resolved_const =
                    crate::frontend::core::typecheck::const_param::resolve_const_candidates(
                        &generic_params,
                        signature_params,
                        &used_as_const,
                        type_generic_params.len(),
                    );
                let const_binders: Vec<ConstVarDef> = resolved_const.const_binders;

                let (final_param_types, final_return_type) = if !type_generic_params.is_empty()
                    && param_types.len() >= type_generic_params.len()
                {
                    // 为每个泛型类型参数创建新的类型变量
                    let mut subst = HashMap::new();
                    for gp in &type_generic_params {
                        let fresh_var = self.env.solver().new_var();
                        subst.insert(gp.name.clone(), fresh_var);
                    }

                    // 添加 const 参数名到 subst，使内层 Fn 中的 TypeRef("N") 解析为底层类型
                    for cb in &const_binders {
                        let base_ty = match cb.kind {
                            ConstKind::Int(_) => MonoType::Int(64),
                            ConstKind::Bool => MonoType::Bool,
                            ConstKind::Float(_) => MonoType::Float(64),
                        };
                        subst.insert(cb.name.clone(), base_ty);
                    }

                    // 剥离类型级参数，使用 return_type 作为实际函数类型
                    let inner_fn_ty = return_type.clone().substitute(&subst);

                    match inner_fn_ty {
                        MonoType::Fn {
                            params: inner_params,
                            return_type: inner_ret,
                            ..
                        } => (inner_params, *inner_ret),
                        // return_type 不是 Fn（可能是单值泛型），保持原样
                        _ => (param_types, return_type),
                    }
                } else if !const_binders.is_empty() {
                    // 没有 Type 泛型但有 const 泛型（如 factorial: (N: Int) -> ...）
                    // 替换 return_type 中的 TypeRef("N") 为 Int
                    let mut subst = HashMap::new();
                    for cb in &const_binders {
                        let base_ty = match cb.kind {
                            ConstKind::Int(_) => MonoType::Int(64),
                            ConstKind::Bool => MonoType::Bool,
                            ConstKind::Float(_) => MonoType::Float(64),
                        };
                        subst.insert(cb.name.clone(), base_ty);
                    }
                    let substituted_ret = return_type.clone().substitute(&subst);
                    (param_types, substituted_ret)
                } else {
                    (param_types, return_type)
                };

                let fn_ty = MonoType::Fn {
                    params: final_param_types.clone(),
                    return_type: Box::new(final_return_type),
                };

                // RFC-027: 解析类型标注中的编译期谓词（如 Positive(5) -> Refined）
                let mut refined_diags = Vec::new();
                let fn_ty = match fn_ty {
                    MonoType::Fn {
                        params,
                        return_type,
                    } => MonoType::Fn {
                        params: params
                            .into_iter()
                            .map(|p| self.resolve_type_annotation(&p, &mut refined_diags))
                            .collect(),
                        return_type: Box::new(
                            self.resolve_type_annotation(&return_type, &mut refined_diags),
                        ),
                    },
                    other => other,
                };
                self.env.errors.extend_errors(refined_diags);

                // 如果有 type_name（显式方法绑定），使用 add_fn_binding
                if type_name.is_some() {
                    // RFC-011a §3: 同签名重复声明 = 覆盖 → E1100；不同签名 = 重载 → 放行
                    let tn = type_name.as_deref().unwrap_or_default();
                    let key = (tn.to_string(), name.clone());
                    if let Some(prev) = self.declared_methods.get(&key) {
                        if *prev == fn_ty {
                            self.add_error(
                                ErrorCodeDefinition::interface_method_duplicate(tn, &name)
                                    .at(stmt.span)
                                    .build(),
                            );
                        }
                    }
                    self.declared_methods.insert(key, fn_ty.clone());
                    self.env
                        .add_fn_binding(&name, type_name.as_deref(), fn_ty.clone());
                } else {
                    // 如果函数有 const 泛型参数，存进 PolyType.const_binders
                    let poly = if const_binders.is_empty() {
                        PolyType::mono(fn_ty.clone())
                    } else {
                        // type_binders 从 solver 的新变量管理，PolyType 中 type_binders 留空
                        // （函数类型 Level 的泛型由 solver 处理，PolyType 仅存 const_binders）
                        PolyType::new_with_const(Vec::new(), const_binders.clone(), fn_ty.clone())
                    };
                    self.env.add_var(name.clone(), poly);
                }

                // 处理 pub 自动绑定
                if *is_pub {
                    self.auto_bind_to_type(&name, &final_param_types, fn_ty);
                }
            }
            crate::frontend::core::parser::ast::StmtKind::Use {
                path,
                items,
                alias,
                item_aliases,
                ..
            } => {
                // 计算导入模式
                // use std.io → register as "io.print"
                // use std.io as str → register as "str.print"
                // use std.{print} → register as "print"
                // use std.{print as p} → register as "p"（#245 内联别名）
                // use std.{print, read} → register as "print", "read"
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
                            // 登记用户模块命名空间别名（std 走 is_std_submodule 机制，不入此表）
                            if !(path == "std" || path.starts_with("std.")) {
                                self.module_namespaces
                                    .insert(module_alias.to_string(), path.clone());
                            }
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
                            // 登记用户模块命名空间别名（std 走 is_std_submodule 机制，不入此表）
                            if !(path == "std" || path.starts_with("std.")) {
                                self.module_namespaces
                                    .insert(alias_name.clone(), path.clone());
                            }
                            for export in exports_to_import {
                                self.register_use_export(alias_name, export, true);
                            }
                        }
                        // use path.{a, b} / use path.{a as x}（#245：仅内联别名）。
                        // 按 item 名查导出——exports 是 HashMap 无序，zip 会错配。
                        (Some(item_names), _) => {
                            for (i, item_name) in item_names.iter().enumerate() {
                                let Some(export) = module.exports.get(item_name) else {
                                    continue;
                                };
                                let local_name = item_aliases
                                    .as_ref()
                                    .and_then(|v| v.get(i))
                                    .and_then(|a| a.as_ref());
                                match local_name {
                                    Some(local) => self.register_use_export(local, export, true),
                                    None => self.register_use_export(item_name, export, false),
                                }
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
            // 外部绑定: Type.method = function 或 Type.method = function[pos]
            crate::frontend::core::parser::ast::StmtKind::Assign {
                target,
                value: Some(val),
                span,
                ..
            } => {
                let (type_name, method_name) = match target.as_ref() {
                    Expr::FieldAccess { expr, field, .. } => {
                        if let Expr::Var(tn, _) = expr.as_ref() {
                            (tn.clone(), field.clone())
                        } else {
                            return;
                        }
                    }
                    _ => return,
                };
                // 语义分流（issue #180 F 组）：base 必须是已注册类型才在 pass2 登记类型空间方法。
                // base 是值（实例）→ 不在此登记，由 pass3 check_stmt 做 schema 校验。
                if !matches!(
                    self.env.resolve_base_kind(&type_name),
                    crate::frontend::core::typecheck::environment::BaseKind::TypeSpace
                ) {
                    return;
                }
                // 从 value 提取 func_name 和 positions
                let (func_name, positions) = match val.as_ref() {
                    Expr::Var(fn_name, _) => (fn_name.clone(), vec![0]),
                    Expr::Index {
                        expr: inner, index, ..
                    } => {
                        let fn_name = if let Expr::Var(n, _) = inner.as_ref() {
                            n.clone()
                        } else {
                            return;
                        };
                        let positions = Self::extract_positions(index);
                        (fn_name, positions)
                    }
                    _ => return,
                };
                if let Some(poly) = self.env.get_var(&func_name) {
                    let total = match &poly.body {
                        MonoType::Fn { params, .. } => params.len(),
                        _ => 0,
                    };
                    let fn_ty = poly.body.clone();
                    if let Some(positions) =
                        self.normalize_binding_positions(&positions, total, *span)
                    {
                        let method_ty = Self::method_type_after_binding(&fn_ty, &positions);
                        self.env
                            .add_method_binding(&type_name, &method_name, method_ty);
                    }
                }
            }
            _ => {}
        }
    }

    /// RFC-004: 归一化绑定位置（负索引从末尾计数，[-1] = 最后一个参数）并校验有效性。
    /// total 为被绑函数参数个数；未知（0）时跳过归一化与校验。无效位置报 E1064。
    fn normalize_binding_positions(
        &mut self,
        positions: &[i64],
        total: usize,
        span: crate::util::span::Span,
    ) -> Option<Vec<i64>> {
        if total == 0 {
            return Some(positions.to_vec());
        }
        let total_i = total as i64;
        let normalized: Vec<i64> = positions
            .iter()
            .map(|&p| if p < 0 { p + total_i } else { p })
            .collect();
        if normalized.iter().any(|&p| p < 0 || p >= total_i) {
            self.add_error(
                ErrorCodeDefinition::invalid_binding_position(&format!("{:?}", positions), total)
                    .at(span)
                    .build(),
            );
            return None;
        }
        Some(normalized)
    }

    /// RFC-004: 绑定后的方法类型——单位绑定时保留全签名（实例占住绑定参数位），
    /// 多位绑定时挖掉被绑参数，剩余参数由调用点填充。
    fn method_type_after_binding(
        fn_ty: &MonoType,
        positions: &[i64],
    ) -> MonoType {
        if positions.len() <= 1 {
            return fn_ty.clone();
        }
        match fn_ty {
            MonoType::Fn {
                params,
                return_type,
            } => {
                let new_params: Vec<MonoType> = params
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !positions.contains(&(*i as i64)))
                    .map(|(_, p)| p.clone())
                    .collect();
                MonoType::Fn {
                    params: new_params,
                    return_type: return_type.clone(),
                }
            }
            other => other.clone(),
        }
    }

    /// RFC-004: 登记类型体绑定的待处理队列（须在函数签名收集 pass2 之后调用）。
    pub fn flush_pending_body_bindings(&mut self) {
        let pending = std::mem::take(&mut self.pending_body_bindings);
        for (type_name, binding, span) in pending {
            match &binding.kind {
                crate::frontend::core::parser::ast::BindingKind::External {
                    function,
                    positions,
                } => {
                    self.register_external_method_binding(
                        &type_name,
                        &binding.name,
                        function,
                        positions,
                        span,
                    );
                }
                crate::frontend::core::parser::ast::BindingKind::DefaultExternal { function } => {
                    self.register_external_method_binding(
                        &type_name,
                        &binding.name,
                        function,
                        &[0],
                        span,
                    );
                }
                crate::frontend::core::parser::ast::BindingKind::Anonymous { .. } => {}
            }
        }
    }

    /// RFC-011a 阶段1: 判断类型体应用项是否为接口实例化形态——
    /// head 命中已注册类型构造器（generic_type_defs）且实参全为类型形态。
    fn is_interface_application(
        &self,
        ty: &crate::frontend::core::parser::ast::Type,
    ) -> bool {
        use crate::frontend::core::parser::ast::Type;
        if let Type::Generic {
            name: head, args, ..
        } = ty
        {
            let all_type_args = args.iter().all(|a| {
                matches!(
                    a,
                    Type::Name { .. }
                        | Type::Generic { .. }
                        | Type::MetaType { .. }
                        | Type::Ref { .. }
                )
            });
            return all_type_args && self.env.generic_type_defs.contains_key(head);
        }
        false
    }

    /// RFC-011a: 类型实参是否引用给定参数名（抽象引用 → 继承链接，不记待决实例化）
    fn type_arg_references(
        ty: &crate::frontend::core::parser::ast::Type,
        params: &[String],
    ) -> bool {
        use crate::frontend::core::parser::ast::Type;
        match ty {
            Type::Name { name, .. } => params.contains(name),
            Type::Generic { args, .. } => args.iter().any(|a| Self::type_arg_references(a, params)),
            Type::Ref { inner, .. } => Self::type_arg_references(inner, params),
            _ => false,
        }
    }

    /// RFC-011a 阶段2: 接口实例化完整性检查（须在 pass2 与类型体绑定登记之后调用）。
    /// 五步流程（§1）：识别（pass1 已记录）→ Self 替换展开 → 签名匹配 →
    /// 通过则登记 ImplementationProof → 失败报编译错误。
    pub fn finalize_interface_instantiations(&mut self) {
        let pending = std::mem::take(&mut self.pending_interface_instantiations);
        for inst in pending {
            let args: Vec<MonoType> = inst
                .args
                .iter()
                .map(|a| MonoType::from(a.clone()))
                .collect();
            let _ = self.check_interface_instantiation(
                &inst.impl_type,
                &inst.interface_name,
                &args,
                inst.span,
            );
        }
    }

    /// RFC-011a: 展开接口成员列表（递归支持接口继承，Self 延迟替换）。
    /// 返回 (方法名, 替换后签名)；同名成员后者覆盖前者。
    fn expand_interface_members(
        &mut self,
        interface_name: &str,
        args: &[MonoType],
        visiting: &mut Vec<String>,
        span: crate::util::span::Span,
    ) -> Result<Vec<(String, MonoType)>, ()> {
        use crate::frontend::core::parser::ast::Type;
        if visiting.iter().any(|v| v == interface_name) {
            // 循环继承：终止该分支展开
            return Ok(Vec::new());
        }
        visiting.push(interface_name.to_string());

        let Some(param_names) = self
            .env
            .generic_type_defs
            .get(interface_name)
            .map(|d| d.type_param_names.clone())
        else {
            self.add_error(
                ErrorCodeDefinition::unknown_interface(interface_name)
                    .at(span)
                    .build(),
            );
            return Err(());
        };
        if param_names.len() != args.len() {
            self.add_error(
                ErrorCodeDefinition::interface_arity_mismatch(
                    interface_name,
                    param_names.len(),
                    args.len(),
                )
                .at(span)
                .build(),
            );
            return Err(());
        }
        let Some(body) = self.type_definition_bodies.get(interface_name).cloned() else {
            visiting.pop();
            return Ok(Vec::new());
        };

        let mut members: Vec<(String, MonoType)> = Vec::new();
        for item in &body {
            match item {
                TypeBodyItem::Field(f) => {
                    let ty = MonoType::from(f.ty.clone());
                    let ty = TypeEnvironment::replace_type_params(&ty, &param_names, args);
                    if !matches!(ty, MonoType::Fn { .. }) {
                        // RFC-011a §1: 接口成员必须是方法（函数类型字段）
                        self.add_error(
                            ErrorCodeDefinition::interface_method_mismatch(
                                interface_name,
                                &f.name,
                                "函数类型",
                                &ty.to_string(),
                            )
                            .at(span)
                            .build(),
                        );
                        return Err(());
                    }
                    members.push((f.name.clone(), ty));
                }
                TypeBodyItem::Expr(Type::Generic {
                    name: head,
                    args: nested_args,
                    ..
                }) => {
                    // 接口继承：先替换当前接口类型参数再递归展开（Self 延迟替换）
                    let nested: Vec<MonoType> = nested_args
                        .iter()
                        .map(|a| {
                            let m = MonoType::from(a.clone());
                            TypeEnvironment::replace_type_params(&m, &param_names, args)
                        })
                        .collect();
                    match self.expand_interface_members(head, &nested, visiting, span) {
                        Ok(sub) => members.extend(sub),
                        Err(()) => return Err(()),
                    }
                }
                _ => {}
            }
        }
        visiting.pop();
        Ok(members)
    }

    /// RFC-011a 阶段2: 对单个接口实例化做完整性检查并登记实现证明。
    fn check_interface_instantiation(
        &mut self,
        impl_type: &str,
        interface_name: &str,
        args: &[MonoType],
        span: crate::util::span::Span,
    ) -> Result<(), ()> {
        let mut visiting = Vec::new();
        let members = self.expand_interface_members(interface_name, args, &mut visiting, span)?;

        // §1.2 命名空间共享：接口方法名与实现类型已有字段同名 → 冲突
        let impl_field_names: Vec<String> = match self.env.types.get(impl_type) {
            Some(poly) => match &poly.body {
                MonoType::Struct(s) => s.fields.iter().map(|(n, _)| n.clone()).collect(),
                _ => Vec::new(),
            },
            None => Vec::new(),
        };
        for (member, _) in &members {
            if impl_field_names.contains(member) {
                self.add_error(
                    ErrorCodeDefinition::interface_member_conflict(impl_type, member)
                        .at(span)
                        .build(),
                );
                return Err(());
            }
        }

        // 完整性检查：每个接口成员须有同名实现且 Self 替换后签名一致
        let mut methods: Vec<String> = Vec::new();
        for (member, expected) in &members {
            let Some(found) = self.env.get_method_binding(impl_type, member).cloned() else {
                self.add_error(
                    ErrorCodeDefinition::interface_method_missing(
                        impl_type,
                        interface_name,
                        member,
                    )
                    .at(span)
                    .build(),
                );
                return Err(());
            };
            // impl 签名中的 Self 是 impl 类型的别名（RFC-011a §3：impl 签名与接口
            // 成员经 Self↦impl 类型替换后完全一致）——两侧用同一实参替换后再比较，
            // impl 写 &Self 或 &Dog 均与接口 &Self 匹配
            let found = TypeEnvironment::replace_type_params(&found, &["Self".to_string()], args);
            if &found != expected {
                self.add_error(
                    ErrorCodeDefinition::interface_method_mismatch(
                        impl_type,
                        member,
                        &expected.to_string(),
                        &found.to_string(),
                    )
                    .at(span)
                    .build(),
                );
                return Err(());
            }
            methods.push(member.clone());
        }

        // 通过 → 生成实现证明（纯编译期概念，运行时擦除，§5.3/§6.4）
        self.env.implementation_proofs.push(ImplementationProof {
            type_name: impl_type.to_string(),
            interface_name: interface_name.to_string(),
            methods,
        });

        // 接口名追加进实现类型的 interfaces 面（LSP/阶段3 类型收集消费）。
        // types 与 vars 必须同步：构造器调用（Dog("Rex")）返回的是 vars 里的
        // StructType，只改 types 会让 structured-subtyping 臂（solver.rs:626）
        // 拿到空 interfaces，List(Animal)/标量存在类型赋值全部 E1002。
        let poly = self.env.types.get(impl_type);
        if let Some(poly) = poly {
            if let MonoType::Struct(s) = &poly.body {
                let mut s = s.clone();
                if !s.interfaces.iter().any(|i| i == interface_name) {
                    s.interfaces.push(interface_name.to_string());
                    let updated = PolyType {
                        type_binders: poly.type_binders.clone(),
                        const_binders: poly.const_binders.clone(),
                        body: MonoType::Struct(s),
                    };
                    self.env
                        .types
                        .insert(impl_type.to_string(), updated.clone());
                    self.env.vars.insert(impl_type.to_string(), updated);
                }
            }
        }
        Ok(())
    }

    /// RFC-004: 将 `Type.method = fn[pos]` 形态的绑定登记到 method_bindings。
    /// 函数尚未注册（前向引用）时静默跳过——与既有宽松行为一致。
    fn register_external_method_binding(
        &mut self,
        type_name: &str,
        method_name: &str,
        func_name: &str,
        positions: &[i64],
        span: crate::util::span::Span,
    ) {
        let found = self.env.get_var(func_name).map(|poly| {
            (
                match &poly.body {
                    MonoType::Fn { params, .. } => params.len(),
                    _ => 0,
                },
                poly.body.clone(),
            )
        });
        let Some((total, fn_ty)) = found else {
            return;
        };
        if let Some(positions) = self.normalize_binding_positions(positions, total, span) {
            let method_ty = Self::method_type_after_binding(&fn_ty, &positions);
            self.env
                .add_method_binding(type_name, method_name, method_ty);
        }
    }

    /// 从 Index 表达式的 index 部分提取位置列表
    fn extract_positions(index: &crate::frontend::core::parser::ast::Expr) -> Vec<i64> {
        crate::frontend::core::parser::ast::Expr::extract_binding_positions(index)
    }

    /// 将模块注册为 Struct 类型（包含所有导出作为字段）
    /// native/未知导出的宽松类型占位（fresh var -> Void）。
    ///
    /// ponytail: std native FFI 边界本就丢类型精度，用宽松占位是诚实的永久状态，
    /// 不是临时 hack。用户模块走 export.mono_type 的完整真实类型。
    fn loose_native_type(&mut self) -> MonoType {
        MonoType::Fn {
            params: vec![self.env.solver().new_var()],
            return_type: Box::new(MonoType::Void),
        }
    }

    /// 导出的注册类型：有真实类型用真实的（用户模块），否则走宽松占位（native）。
    fn export_register_type(
        &mut self,
        export: &crate::frontend::module::Export,
    ) -> MonoType {
        export
            .mono_type
            .clone()
            .unwrap_or_else(|| self.loose_native_type())
    }

    fn register_module_as_struct(
        &mut self,
        _module_path: &str,
        module_alias: &str,
        module: &crate::frontend::module::ModuleInfo,
    ) {
        let mut fields = Vec::new();
        for export in module.exports.values() {
            let field_ty = self.export_register_type(export);
            fields.push((export.name.clone(), field_ty));
        }
        let module_ty = MonoType::Struct(crate::frontend::core::types::mono::StructType {
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
                    for sub_export in sub_module.exports.values() {
                        let field_ty = self.export_register_type(sub_export);
                        fields.push((sub_export.name.clone(), field_ty));
                    }
                }
                let module_ty = MonoType::Struct(crate::frontend::core::types::mono::StructType {
                    name: export.name.clone(),
                    fields,
                    methods: HashMap::new(),
                    field_mutability: Vec::new(),
                    field_has_default: Vec::new(),
                    interfaces: vec![],
                });
                self.env.add_var(register_name, PolyType::mono(module_ty));
            }
            crate::frontend::module::ExportKind::Type => {
                // 类型导出：镜像本地类型定义，同时注册到类型空间与值空间，
                // 使构造（Point(...)）与字段访问（p.x）能解析结构体。
                if self.env.get_var(&register_name).is_some() {
                    return;
                }
                let ty = self.export_register_type(export);
                let poly = PolyType::mono(ty);
                self.env.add_type(register_name.clone(), poly.clone());
                self.env.add_var(register_name, poly);
            }
            _ => {
                // 如果变量已存在（比如已经是 Struct 类型），则跳过
                if self.env.get_var(&register_name).is_some() {
                    return;
                }
                let ty = self.export_register_type(export);
                self.env.add_var(register_name, PolyType::mono(ty));
            }
        }
    }

    /// 添加类型定义
    fn add_type_definition(
        &mut self,
        name: &str,
        definition: &crate::frontend::core::parser::ast::Type,
        signature_params: &[Param],
        span: crate::util::span::Span,
    ) {
        let generic_params =
            classify_generic_params(signature_params, &|name| self.env.has_trait(name));
        let param_names: Vec<String> = generic_params.iter().map(|p| p.name.clone()).collect();
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
                    let decl = format!("Type: Type({}) = ...", param_names.join(", "));
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
                })
            }
            _ => poly.body.clone(),
        });
        self.env.add_type(name.to_string(), poly.clone());
        // 同时注册到 vars，使类型名可以在表达式中使用（如 Point(1.0, 2.0) 构造器调用）
        self.env.add_var(name.to_string(), poly.clone());

        // RFC-004: 类型体内的方法绑定登记到 method_bindings（与语句级 Type.method = fn[pos]
        // 同一语义）。此前类型体 Binding 项只有 IR 侧登记，方法调用在类型检查层报 E1042。
        if let crate::frontend::core::parser::ast::Type::Struct { body } = definition {
            for item in body {
                if let crate::frontend::core::parser::ast::TypeBodyItem::Binding(b) = item {
                    match &b.kind {
                        crate::frontend::core::parser::ast::BindingKind::External { .. }
                        | crate::frontend::core::parser::ast::BindingKind::DefaultExternal {
                            ..
                        } => {
                            // 被绑函数可能在其后定义，延迟到 pass2 之后登记
                            self.pending_body_bindings
                                .push((name.to_string(), b.clone(), span));
                        }
                        crate::frontend::core::parser::ast::BindingKind::Anonymous {
                            params,
                            return_type,
                            positions,
                            ..
                        } => {
                            // 匿名绑定：函数类型来自内置声明的注解
                            let fn_ty = MonoType::Fn {
                                params: params
                                    .iter()
                                    .filter_map(|p| p.ty.as_ref())
                                    .map(|t| MonoType::from(t.clone()))
                                    .collect(),
                                return_type: Box::new(MonoType::from((**return_type).clone())),
                            };
                            if let Some(positions) =
                                self.normalize_binding_positions(positions, params.len(), span)
                            {
                                let method_ty = Self::method_type_after_binding(&fn_ty, &positions);
                                self.env.add_method_binding(name, &b.name, method_ty);
                            }
                        }
                    }
                }
            }
        }

        // RFC-027 Phase 2.5: 带约束表达式的类型定义同时注册为 proof 函数
        // IsPositive: (x: Int) -> Type = { x > 0 }
        // → 注册 `IsPositive` 为 `Fn { params: [Int], return_type: MetaType }`
        if let crate::frontend::core::parser::ast::Type::Struct { body } = definition {
            let has_constraint = body.iter().any(|item| {
                matches!(
                    item,
                    crate::frontend::core::parser::ast::TypeBodyItem::Expr(
                        crate::frontend::core::parser::ast::Type::ConstExpr(_)
                    )
                )
            });
            if has_constraint && !signature_params.is_empty() {
                let param_types: Vec<MonoType> = signature_params
                    .iter()
                    .filter_map(|p| p.ty.as_ref().map(|t| MonoType::from(t.clone())))
                    .collect();
                let fn_ty = MonoType::Fn {
                    params: param_types,
                    return_type: Box::new(MonoType::MetaType {
                        universe_level: crate::frontend::core::types::mono::UniverseLevel::type0(),
                        type_params: vec![],
                    }),
                };
                self.env.add_var(name.to_string(), PolyType::mono(fn_ty));
            }
        }

        // RFC-011a 阶段1: 类型体 AST 项留存（接口展开需要原始应用项，MonoType 转换会丢弃）
        if let crate::frontend::core::parser::ast::Type::Struct { body } = definition {
            self.type_definition_bodies
                .insert(name.to_string(), body.clone());
        }

        // RFC-011a 阶段1: 类型体应用项语义改判。
        // `Name(args)`（Expr(Generic)）命中已注册类型构造器且实参全为类型 → 接口实例化，
        // 记入待决队列，阶段 2 统一做 Self 替换 + 完整性检查；实参引用本声明的类型参数
        // （如接口继承体内的 Animal(Self)）为抽象链接，不记待检查（展开时延迟替换）。
        // 其余应用项维持 const 约束路径（Assert(N > 0) 等），行为不变。
        if let crate::frontend::core::parser::ast::Type::Struct { body } = definition {
            let enclosing_params: Vec<String> =
                generic_params.iter().map(|p| p.name.clone()).collect();
            for item in body {
                if let crate::frontend::core::parser::ast::TypeBodyItem::Expr(ty) = item {
                    if self.is_interface_application(ty) {
                        if let crate::frontend::core::parser::ast::Type::Generic {
                            name: head,
                            args,
                            ..
                        } = ty
                        {
                            let abstract_ref = args
                                .iter()
                                .any(|a| Self::type_arg_references(a, &enclosing_params));
                            if !abstract_ref {
                                self.pending_interface_instantiations.push(
                                    PendingInterfaceInstantiation {
                                        impl_type: name.to_string(),
                                        interface_name: head.clone(),
                                        args: args.clone(),
                                        span,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        // 如果是泛型类型构造器（有泛型参数），存储模板信息用于类型实例化
        if !generic_params.is_empty() {
            use crate::frontend::core::typecheck::environment::GenericTypeDef;
            use crate::frontend::core::types::var::TypeVar;

            let type_param_names: Vec<String> = generic_params
                .iter()
                .filter(|p| matches!(p.kind, GenericParamKind::Type))
                .map(|p| p.name.clone())
                .collect();

            // const 判定：用途分析是唯一裁判（RFC-011 §4.1 精筛唯一实现在 const_param.rs）。
            let candidate_names: HashSet<String> = generic_params
                .iter()
                .filter(|p| matches!(p.kind, GenericParamKind::Const { .. }))
                .map(|p| p.name.clone())
                .collect();
            let used_as_const = collect_used_const_names(&candidate_names, definition);

            let resolved_const =
                crate::frontend::core::typecheck::const_param::resolve_const_candidates(
                    &generic_params,
                    signature_params,
                    &used_as_const,
                    type_param_names.len(),
                );

            // #297/F：落空 const 候选（标注具体类型但未在类型体任何类型位置引用）
            // 不能静默丢弃——否则实例化 arity 对不上，调用侧报风马牛不相及的 E1010。
            // 按 RFC-011 §4.1 勘误推荐：声明侧直接报错。
            for p in &resolved_const.fallen {
                self.add_error(
                    ErrorCodeDefinition::unused_const_param(&p.name, name)
                        .at(span)
                        .build(),
                );
            }

            let mut const_binders: Vec<ConstVarDef> = resolved_const.const_binders;

            // 从类型体收集待定证明义务到 const 参数
            // （接口实例化应用项已被分流，不走 const 约束路径）
            if let crate::frontend::core::parser::ast::Type::Struct { body } = definition {
                for item in body {
                    match item {
                        TypeBodyItem::Expr(ty) if !self.is_interface_application(ty) => {
                            process_body_expr_item(ty, &mut const_binders);
                        }
                        TypeBodyItem::Expr(_) => {}
                        TypeBodyItem::Field(f) => {
                            process_body_expr_item(&f.ty, &mut const_binders);
                        }
                        _ => {}
                    }
                }
            }

            let type_binders: Vec<TypeVar> =
                (0..type_param_names.len()).map(TypeVar::new).collect();

            let def = GenericTypeDef {
                poly: PolyType {
                    type_binders,
                    const_binders,
                    body: poly.body.clone(),
                },
                type_param_names,
            };
            self.env.add_generic_type_def(name.to_string(), def);
        }

        // 自动为 Record 类型派生标准库 traits
        self.auto_derive_traits(name, definition);
    }
}

/// 处理类型体中的类型表达式项，收集 const 约束到对应的 const binder。
/// 遍历类型表达式中的 Generic 类型（如 Assert(N < 100)），
/// 将含自由 const 变量的 ConstExpr 参数关联到对应 const binder 的 constraints。
fn process_body_expr_item(
    ty: &crate::frontend::core::parser::ast::Type,
    const_binders: &mut [ConstVarDef],
) {
    use crate::frontend::core::parser::ast::Type;
    if let Type::Generic { args, .. } = ty {
        for arg in args {
            if let Type::ConstExpr(expr) = arg {
                if let Some(const_expr) = convert_expr_to_const_expr(expr) {
                    if let Some(cv) = find_const_var_in_expr(expr, const_binders) {
                        const_binders[cv.index()].constraints.push(const_expr);
                    }
                }
            }
        }
    }
}

/// 在 Expr 树中查找引用的 const 变量名，返回对应的 ConstVar index
fn find_const_var_in_expr(
    expr: &Expr,
    const_binders: &[ConstVarDef],
) -> Option<crate::frontend::core::types::var::ConstVar> {
    match expr {
        Expr::Var(name, _) => const_binders
            .iter()
            .position(|b| b.name == *name)
            .map(crate::frontend::core::types::var::ConstVar::new),
        Expr::BinOp { left, right, .. } => find_const_var_in_expr(left, const_binders)
            .or_else(|| find_const_var_in_expr(right, const_binders)),
        Expr::UnOp { expr, .. } => find_const_var_in_expr(expr, const_binders),
        _ => None,
    }
}

/// 用途分析：扫描类型定义体，找出候选参数中哪些**被当编译期值/类型使用**。
///
/// 这是 const 判定的唯一裁判——取代旧的"CONST_PARAM_TYPES 名单 + 大小写猜测"。
/// 用途 = 参数名出现在以下任一编译期位置：
/// - `Array(T, N)` 里作为泛型实参的裸 `Name`（Generic.args）
/// - `Assert(N > 0)` 里的 `ConstExpr` 表达式引用
/// - `length: N` 里作为字段类型标注的裸 `Name`
fn collect_used_const_names(
    candidates: &HashSet<String>,
    definition: &crate::frontend::core::parser::ast::Type,
) -> HashSet<String> {
    use crate::frontend::core::parser::ast::Type;
    let mut found = HashSet::new();
    if let Type::Struct { body } = definition {
        for item in body {
            match item {
                TypeBodyItem::Expr(ty) => collect_used_in_type(ty, candidates, &mut found),
                TypeBodyItem::Field(f) => collect_used_in_type(&f.ty, candidates, &mut found),
                _ => {}
            }
        }
    }
    found
}

/// 递归扫描类型表达式，收集被引用的候选参数名。
pub(crate) fn collect_used_in_type(
    ty: &crate::frontend::core::parser::ast::Type,
    candidates: &HashSet<String>,
    found: &mut HashSet<String>,
) {
    use crate::frontend::core::parser::ast::Type;
    match ty {
        Type::Name { name, .. } if candidates.contains(name) => {
            found.insert(name.clone());
        }
        Type::ConstExpr(expr) => collect_used_in_expr(expr, candidates, found),
        Type::Generic { args, .. } => {
            for arg in args {
                collect_used_in_type(arg, candidates, found);
            }
        }
        Type::Fn {
            params,
            return_type,
        } => {
            for p in params {
                collect_used_in_type(p, candidates, found);
            }
            collect_used_in_type(return_type, candidates, found);
        }
        Type::Option(inner) | Type::Ptr(inner) => {
            collect_used_in_type(inner, candidates, found);
        }
        Type::Ref { inner, .. } => collect_used_in_type(inner, candidates, found),
        Type::Result(a, b) => {
            collect_used_in_type(a, candidates, found);
            collect_used_in_type(b, candidates, found);
        }
        Type::Tuple(types) | Type::Sum(types) => {
            for t in types {
                collect_used_in_type(t, candidates, found);
            }
        }
        _ => {}
    }
}

/// 递归扫描编译期表达式（如 `N > 0`），收集被引用的候选参数名。
fn collect_used_in_expr(
    expr: &Expr,
    candidates: &HashSet<String>,
    found: &mut HashSet<String>,
) {
    match expr {
        Expr::Var(name, _) if candidates.contains(name) => {
            found.insert(name.clone());
        }
        Expr::BinOp { left, right, .. } => {
            collect_used_in_expr(left, candidates, found);
            collect_used_in_expr(right, candidates, found);
        }
        Expr::UnOp { expr, .. } => collect_used_in_expr(expr, candidates, found),
        _ => {}
    }
}
impl TypeChecker {
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
        let fields: Vec<crate::frontend::core::parser::ast::StructField> = match definition {
            crate::frontend::core::parser::ast::Type::NamedStruct { fields, .. } => fields.clone(),
            crate::frontend::core::parser::ast::Type::Struct { body } => body
                .iter()
                .filter_map(|it| {
                    if let crate::frontend::core::parser::ast::TypeBodyItem::Field(f) = it {
                        Some(f.clone())
                    } else {
                        None
                    }
                })
                .collect(),
            _ => return,
        };

        // 获取 trait 表的引用（用于检查）
        let trait_table = &self.env.trait_table;

        // 为每个内置可派生 trait 尝试自动派生
        let mut impls_to_add = Vec::new();

        for trait_name in TraitTable::BUILTIN_DERIVES {
            // 检查是否可以自动派生
            let can_derive = trait_table.can_auto_derive(trait_name, &fields);

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
            // #324：模块级阶段挂当前语句 span，诊断自动获得位置
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            if let StmtKind::Assign { target, is_pub, .. } = &stmt.kind {
                let Some((name, type_name)) = target.receiver_parts() else {
                    continue;
                };
                let is_method = type_name.is_some();
                if is_method || *is_pub {
                    if is_method {
                        if let Some(ty_name) = type_name {
                            self.env.add_export(&format!("{}.{}", ty_name, name));
                        }
                    } else {
                        self.env.add_export(&name);
                    }
                }
            }
            if let StmtKind::TypeDefinition { name, .. } = &stmt.kind {
                // 类型定义始终导出
                self.env.add_export(name);
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
    fn constructor_names_from_module(_module: &Module) -> HashSet<String> {
        // Variant 语法已废弃（RFC-010，issue #203）。
        // 类型定义的构造器识别由下游类型系统统一处理，此处保留空集合作占位，
        // 避免破坏 semantic_tokens 中 EnumMember 识别链路。
        HashSet::new()
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
    // ============ RFC-027 阶段 1：编译期谓词集成 ============

    /// 解析类型标注：如果是编译期谓词调用，正格化为 Refined（#263：非法用法写诊断汇入 diags，不静默）
    ///
    /// Generic("Positive", [arg]) -> 尝试 PredicateResolver::try_resolve
    /// 如果不是已知的编译期谓词，检查是否是证明函数
    fn resolve_type_annotation(
        &self,
        ty: &MonoType,
        diags: &mut Vec<Diagnostic>,
    ) -> MonoType {
        match ty {
            MonoType::Generic { name, args } if !args.is_empty() => {
                // 尝试原有 PredicateResolver（三值结果，#263）
                match PredicateResolver::try_resolve(&self.env, name, args) {
                    Some(Ok(refined)) => return refined,
                    Some(Err(err)) => {
                        // #263：已注册谓词非法用法——汇入诊断，绝不静默放行
                        diags.push(Self::refined_usage_diagnostic(name, &err));
                        return ty.clone();
                    }
                    None => {} // 不是谓词——继续证明函数路径
                }
                // Phase 2.5: 检查是否是证明函数（源码定义的返回 Type 的函数）
                match self.lookup_proof_fn_base_type(name, args) {
                    Some(Ok(base)) => {
                        // 将参数转换为 ConstExpr
                        let const_args: Option<Vec<ConstExpr>> = args
                            .iter()
                            .map(|a| self.mono_type_to_const_expr(a))
                            .collect();
                        match const_args {
                            Some(const_args) => {
                                let constraint = ConstExpr::Call {
                                    func: name.clone(),
                                    args: const_args,
                                };
                                MonoType::Refined {
                                    base: Box::new(base),
                                    constraint,
                                }
                            }
                            None => {
                                // #263：证明函数实参形态不可转换——约束无法生成，汇入诊断（E1092）
                                diags
                                    .push(ErrorCodeDefinition::refined_arg_not_const(name).build());
                                ty.clone()
                            }
                        }
                    }
                    Some(Err(err)) => {
                        // #263：证明函数实参个数不匹配——汇入诊断（E1093）
                        diags.push(Self::refined_usage_diagnostic(name, &err));
                        ty.clone()
                    }
                    None => ty.clone(), // 非证明函数——保持原样
                }
            }
            _ => ty.clone(),
        }
    }

    /// 将谓词/证明函数用法非法映射为诊断（#263：结构化错误，i18n 文案不混排）
    fn refined_usage_diagnostic(
        name: &str,
        err: &crate::frontend::core::typecheck::predicate_resolver::PredicateResolveError,
    ) -> Diagnostic {
        use crate::frontend::core::typecheck::predicate_resolver::PredicateResolveError;
        match err {
            PredicateResolveError::ArityMismatch { expected, found } => {
                ErrorCodeDefinition::refined_arity_mismatch(name, *expected, *found).build()
            }
            PredicateResolveError::ArgNotConst => {
                ErrorCodeDefinition::refined_arg_not_const(name).build()
            }
        }
    }

    /// 查找证明函数的基类型（三值结果，#263）
    ///
    /// 检查 `name` 是否在环境中定义为返回 Type 的函数：
    /// - `None`：不是证明函数（调用方继续其他解析路径）
    /// - `Some(Ok(base))`：是证明函数，返回第一个参数的类型作为基类型
    /// - `Some(Err(err))`：是证明函数但实参个数不匹配（#263：不得静默放行）
    fn lookup_proof_fn_base_type(
        &self,
        name: &str,
        args: &[MonoType],
    ) -> Option<
        Result<
            MonoType,
            crate::frontend::core::typecheck::predicate_resolver::PredicateResolveError,
        >,
    > {
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
                // #263：实参个数不匹配是非法用法，不得与「不是证明函数」混淆
                if params.len() != args.len() {
                    return Some(Err(
                        crate::frontend::core::typecheck::predicate_resolver::PredicateResolveError::ArityMismatch {
                            expected: params.len(),
                            found: args.len(),
                        },
                    ));
                }
                // 返回第一个参数的类型作为基类型（此处 params 与 args 等长且非空）
                return Some(Ok(params[0].clone()));
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
    #[allow(dead_code)]
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
        &mut self,
        module: &Module,
        proof_calls: &mut Vec<crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall>,
    ) {
        use crate::frontend::core::typecheck::proof::dep_graph::TypeDepGraph;

        let mut dep_graph = TypeDepGraph::new();
        let mut shared_ctx =
            crate::frontend::core::typecheck::proof::context::ProofContext::new(&self.env);
        // #263：shared_ctx 持有 &self.env，精化解析诊断先汇入 sink，
        // 待 shared_ctx 释放后再写入 env.errors
        let mut refined_diags = Vec::new();

        // 阶段 1：构建依赖图 + 初始绑定检查
        for stmt in &module.items {
            // #324：模块级阶段挂当前语句 span，诊断自动获得位置
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            self.build_dep_graph_and_check_init(
                stmt,
                &mut dep_graph,
                &mut shared_ctx,
                proof_calls,
                &mut refined_diags,
            );
        }

        // 阶段 2：遍历赋值点，生成 VC
        for stmt in &module.items {
            // #324：模块级阶段挂当前语句 span，诊断自动获得位置
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            self.check_assignments_with_deps(stmt, &dep_graph, &mut shared_ctx, proof_calls);
        }

        // #263：精化解析诊断汇入（shared_ctx 已不再被使用，借用结束）
        self.env.errors.extend_errors(refined_diags);
    }

    /// 阶段 1：递归遍历语句树——构建依赖图 + 检查初始化绑定
    fn build_dep_graph_and_check_init(
        &self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        dep_graph: &mut crate::frontend::core::typecheck::proof::dep_graph::TypeDepGraph,
        shared_ctx: &mut crate::frontend::core::typecheck::proof::context::ProofContext<'_>,
        proof_calls: &mut Vec<crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall>,
        diags: &mut Vec<Diagnostic>,
    ) {
        use crate::frontend::core::parser::ast::StmtKind;

        match &stmt.kind {
            StmtKind::Assign {
                target,
                type_annotation: Some(type_ann),
                ..
            } => {
                let name = match target.as_ref() {
                    Expr::Var(n, _) => n.clone(),
                    _ => return,
                };
                let mono_ty = MonoType::from(type_ann.clone());
                let resolved_ty = self.resolve_type_annotation(&mono_ty, diags);
                if let MonoType::Refined { constraint, .. } = &resolved_ty {
                    // RFC-027 Phase 2.5: Call 约束直接生成 proof call
                    if let crate::frontend::core::types::const_data::ConstExpr::Call {
                        func,
                        args,
                    } = constraint
                    {
                        let call_args: Vec<crate::frontend::core::types::ConstValue> = args
                            .iter()
                            .filter_map(|a| {
                                if let crate::frontend::core::types::const_data::ConstExpr::Lit(v) =
                                    a
                                {
                                    Some(v.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        proof_calls.push(
                            crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall {
                                func_name: func.clone(),
                                args: call_args,
                            },
                        );
                    }
                    let free_vars = Self::extract_free_vars(constraint);
                    for fv in &free_vars {
                        if fv != &name {
                            dep_graph.add_dep(&name, fv);
                        }
                    }
                }
            }
            StmtKind::Assign {
                value: Some(expr), ..
            } => {
                if let crate::frontend::core::parser::ast::Expr::Lambda { body, .. } = expr.as_ref()
                {
                    for s in &body.stmts {
                        self.build_dep_graph_and_check_init(
                            s,
                            dep_graph,
                            shared_ctx,
                            proof_calls,
                            diags,
                        );
                    }
                } else if let crate::frontend::core::parser::ast::Expr::Block(block) = expr.as_ref()
                {
                    for s in &block.stmts {
                        self.build_dep_graph_and_check_init(
                            s,
                            dep_graph,
                            shared_ctx,
                            proof_calls,
                            diags,
                        );
                    }
                }
            }
            StmtKind::Expr(expr) => {
                self.build_dep_graph_from_expr(
                    expr.as_ref(),
                    dep_graph,
                    shared_ctx,
                    proof_calls,
                    diags,
                );
            }
            StmtKind::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                for s in &then_branch.stmts {
                    self.build_dep_graph_and_check_init(
                        s,
                        dep_graph,
                        shared_ctx,
                        proof_calls,
                        diags,
                    );
                }
                for (_, body) in else_if_branches {
                    for s in &body.stmts {
                        self.build_dep_graph_and_check_init(
                            s,
                            dep_graph,
                            shared_ctx,
                            proof_calls,
                            diags,
                        );
                    }
                }
                if let Some(else_body) = else_branch {
                    for s in &else_body.stmts {
                        self.build_dep_graph_and_check_init(
                            s,
                            dep_graph,
                            shared_ctx,
                            proof_calls,
                            diags,
                        );
                    }
                }
            }
            StmtKind::For { body, .. } => {
                for s in &body.stmts {
                    self.build_dep_graph_and_check_init(
                        s,
                        dep_graph,
                        shared_ctx,
                        proof_calls,
                        diags,
                    );
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
        diags: &mut Vec<Diagnostic>,
    ) {
        match expr {
            crate::frontend::core::parser::ast::Expr::Block(block) => {
                for s in &block.stmts {
                    self.build_dep_graph_and_check_init(
                        s,
                        dep_graph,
                        shared_ctx,
                        proof_calls,
                        diags,
                    );
                }
            }
            crate::frontend::core::parser::ast::Expr::While { body, .. } => {
                for s in &body.stmts {
                    self.build_dep_graph_and_check_init(
                        s,
                        dep_graph,
                        shared_ctx,
                        proof_calls,
                        diags,
                    );
                }
            }
            crate::frontend::core::parser::ast::Expr::For { body, .. } => {
                for s in &body.stmts {
                    self.build_dep_graph_and_check_init(
                        s,
                        dep_graph,
                        shared_ctx,
                        proof_calls,
                        diags,
                    );
                }
            }
            crate::frontend::core::parser::ast::Expr::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                for s in &then_branch.stmts {
                    self.build_dep_graph_and_check_init(
                        s,
                        dep_graph,
                        shared_ctx,
                        proof_calls,
                        diags,
                    );
                }
                for (_, body) in else_if_branches {
                    for s in &body.stmts {
                        self.build_dep_graph_and_check_init(
                            s,
                            dep_graph,
                            shared_ctx,
                            proof_calls,
                            diags,
                        );
                    }
                }
                if let Some(else_body) = else_branch {
                    for s in &else_body.stmts {
                        self.build_dep_graph_and_check_init(
                            s,
                            dep_graph,
                            shared_ctx,
                            proof_calls,
                            diags,
                        );
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
        use crate::frontend::core::parser::ast::{StmtKind, Expr};

        match &stmt.kind {
            // 赋值语句：x = expr（有 value 的 Assign，target 是 Var）
            StmtKind::Assign {
                target,
                value: Some(v),
                ..
            } => {
                if let Expr::Var(name, _) = target.as_ref() {
                    let affected = dep_graph.affected_by(name);
                    if !affected.is_empty() {
                        self.generate_vc_for_dependants(name, &affected, shared_ctx, proof_calls);
                    }
                }
                // 递归处理 Lambda/Block 函数体
                if let Expr::Lambda { body, .. } = v.as_ref() {
                    for s in &body.stmts {
                        self.check_assignments_with_deps(s, dep_graph, shared_ctx, proof_calls);
                    }
                } else if let Expr::Block(block) = v.as_ref() {
                    for s in &block.stmts {
                        self.check_assignments_with_deps(s, dep_graph, shared_ctx, proof_calls);
                    }
                }
            }
            StmtKind::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                for s in &then_branch.stmts {
                    self.check_assignments_with_deps(s, dep_graph, shared_ctx, proof_calls);
                }
                for (_, body) in else_if_branches {
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
}
include!("checker/semantic_tokens.rs");
