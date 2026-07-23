#![allow(clippy::result_large_err)]

//! 语句检查器
//!
//! 合并原 checking/BodyChecker 和 inference/StmtInferrer
//! 使用统一的 ScopeManager 管理变量作用域

use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

use std::collections::HashMap;
use crate::frontend::module::{Export, ExportKind, ModuleInfo};
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::core::types::{MonoType, PolyType, TraitTable, TypeConstraintSolver};
use crate::frontend::core::parser::ast::{classify_generic_params, Block, Expr, Param, Stmt};
use crate::middle::passes::mono::instance::InstantiationRequest;

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
        HashMap<String, Vec<crate::frontend::core::typecheck::passes::overload::OverloadCandidate>>,
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
    /// 当前函数的预期返回类型（用于 return 语句的类型检查）
    expected_return_type: Option<MonoType>,
    /// 泛型类型定义模板表（从 TypeEnvironment 同步）
    generic_type_defs: std::collections::HashMap<
        String,
        crate::frontend::core::typecheck::environment::GenericTypeDef,
    >,
    /// 方法绑定表: "Type.method" -> MonoType
    method_bindings: HashMap<String, MonoType>,
    /// 类型定义表: type_name -> MonoType(Struct)
    /// 用于 TypeRef → Struct 解析
    type_defs: HashMap<String, MonoType>,
    /// 类型名集合（从 TypeEnvironment.types 同步），语义区分 base 是类型还是值（issue #180 F 组）
    type_names: std::collections::HashSet<String>,
    /// 实例化请求（收集所有泛型函数实例化需求）
    pub instantiation_requests: Vec<InstantiationRequest>,
    /// 流敏感假设集 Γ（可选 — None 在测试或未启用证明管道时使用）
    gamma: Option<crate::frontend::core::typecheck::proof::assumptions::FlowSensitiveGamma>,
    /// 依赖类型环境（类型族注册与查找）
    dep_env: crate::frontend::core::types::eval::dependent_types::DependentTypeEnv,
    /// Trait 表（用于 classify_generic_params 判定 annotation 是否为 trait）
    trait_table: TraitTable,
    /// 证明函数基类型表: "IsPositive" -> Int(64)（RFC-027 Phase 2.5）
    proof_fn_bases: HashMap<String, MonoType>,
}

impl StatementChecker {
    /// 创建新的语句检查器
    pub fn new(
        solver: &mut TypeConstraintSolver,
        gamma: Option<crate::frontend::core::typecheck::proof::assumptions::FlowSensitiveGamma>,
        dep_env: crate::frontend::core::types::eval::dependent_types::DependentTypeEnv,
        trait_table: TraitTable,
    ) -> Self {
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
            expected_return_type: None,
            generic_type_defs: std::collections::HashMap::new(),
            method_bindings: HashMap::new(),
            type_defs: HashMap::new(),
            type_names: std::collections::HashSet::new(),
            instantiation_requests: Vec::new(),
            gamma,
            dep_env,
            trait_table,
            proof_fn_bases: HashMap::new(),
        }
    }

    /// 设置类型定义表
    pub fn set_type_defs(
        &mut self,
        defs: HashMap<String, MonoType>,
    ) {
        self.type_defs = defs;
    }

    /// 设置类型名集合（值空间 schema 校验区分类型/值 base，issue #180 F 组）
    pub fn set_type_names(
        &mut self,
        names: std::collections::HashSet<String>,
    ) {
        self.type_names = names;
    }

    /// 设置 Trait 表
    pub fn set_trait_table(
        &mut self,
        trait_table: TraitTable,
    ) {
        self.trait_table = trait_table;
    }

    /// 解析 TypeRef 为实际的类型定义
    ///
    /// 如果 `ty` 是 `TypeRef("Circle")` 且 `Circle` 在 `type_defs` 中定义为
    /// `Struct { ... }`，则返回该 Struct 类型。对于内置类型名也进行解析。
    fn resolve_type_ref_type(
        &self,
        ty: &MonoType,
    ) -> MonoType {
        match ty {
            MonoType::TypeRef(name) => {
                // 先查内置类型名
                if let Some(builtin) = MonoType::from_builtin_name(name) {
                    return builtin;
                }
                // 再查用户类型定义
                if let Some(struct_ty) = self.type_defs.get(name) {
                    return struct_ty.clone();
                }
                ty.clone()
            }
            _ => ty.clone(),
        }
    }

    /// 设置泛型类型定义模板表
    pub fn set_generic_type_defs(
        &mut self,
        defs: std::collections::HashMap<
            String,
            crate::frontend::core::typecheck::environment::GenericTypeDef,
        >,
    ) {
        self.generic_type_defs = defs;
    }

    /// 设置方法绑定表
    pub fn set_method_bindings(
        &mut self,
        bindings: HashMap<String, MonoType>,
    ) {
        self.method_bindings = bindings;
    }

    /// 设置证明函数基类型表（RFC-027 Phase 2.5）
    pub fn set_proof_fn_bases(
        &mut self,
        bases: HashMap<String, MonoType>,
    ) {
        self.proof_fn_bases = bases;
    }

    /// 尝试实例化泛型类型
    ///
    /// 当 type_annotation 为 `List(Int)` 时，查找 `List` 的泛型模板，
    /// 将类型参数 `T` 替换为 `Int`，返回展开后的结构体类型。
    fn try_instantiate_generic_type(
        &self,
        type_ann: &crate::frontend::core::parser::ast::Type,
    ) -> Option<MonoType> {
        use crate::frontend::core::typecheck::TypeEnvironment;
        match type_ann {
            crate::frontend::core::parser::ast::Type::Generic { name, args, .. } => {
                let def = self.generic_type_defs.get(name)?;
                let arg_types: Vec<MonoType> =
                    args.iter().map(|a| MonoType::from(a.clone())).collect();
                TypeEnvironment::instantiate_generic_type(def, &arg_types).ok()
            }
            _ => None,
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

        MonoType::Struct(crate::frontend::core::types::mono::StructType {
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
        self.scope.add_var(
            binding_name.to_string(),
            PolyType::mono(ty),
            false,
            crate::util::span::Span::default(),
        );
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
                self.scope.add_var(
                    module_alias.to_string(),
                    PolyType::mono(module_ty),
                    false,
                    crate::util::span::Span::default(),
                );
            }
            // use path as alias
            (None, Some(aliases)) if aliases.len() == 1 => {
                let module_alias = &aliases[0];
                let module_ty = self.module_as_struct_type(&module, module_alias);
                self.scope.add_var(
                    module_alias.to_string(),
                    PolyType::mono(module_ty),
                    false,
                    crate::util::span::Span::default(),
                );
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
        is_mut: bool,
        definition_span: crate::util::span::Span,
    ) {
        self.scope.add_var(name, poly, is_mut, definition_span);
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
        self.check_fn_def_with_subst(name, params, body, &std::collections::HashMap::new())
    }

    /// 带 const 替换的 check_fn_def（const 泛型参数名 → 底层类型的 MonoType）
    fn check_fn_def_with_subst(
        &mut self,
        name: &str,
        params: &[Param],
        body: &Block,
        const_subst: &std::collections::HashMap<String, MonoType>,
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

        // 添加参数到函数作用域，const 泛型引用用 subst 替换
        for param in params {
            let param_ty = param
                .ty
                .as_ref()
                .map(|t| MonoType::from(t.clone()))
                .unwrap_or_else(|| self.solver.new_var());
            let param_ty = Self::substitute_type_refs(param_ty, const_subst);
            self.scope.add_var(
                param.name.clone(),
                PolyType::mono(param_ty),
                param.is_mut,
                crate::util::span::Span::default(),
            );
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
            crate::frontend::core::parser::ast::StmtKind::Assign {
                target,
                type_annotation,
                signature_params,
                value,
                is_mut,
                span: stmt_span,
                ..
            } => {
                use crate::frontend::core::parser::ast::Expr;
                let (name, _type_name) = match target.as_ref() {
                    Expr::Var(n, _) => (n.clone(), None),
                    Expr::FieldAccess { expr, field, .. } => {
                        if let Expr::Var(tn, _) = expr.as_ref() {
                            (field.clone(), Some(tn.clone()))
                        } else {
                            (field.clone(), None)
                        }
                    }
                    _ => return Ok(()),
                };
                // 值空间 schema 校验（issue #180 F 组，全受限）：
                // target 是 base.field 且 base 是值（实例）→ 字段必须在类型 schema 内。
                if let Expr::FieldAccess { expr, field, .. } = target.as_ref() {
                    // base 是类型 → 类型空间（pass2 已登记），此处不重复校验
                    let base_is_type = if let Expr::Var(base, _) = expr.as_ref() {
                        self.type_names.contains(base)
                    } else {
                        false
                    };
                    if !base_is_type {
                        if let Ok(base_ty) = self.check_expr(expr) {
                            // 剥 Ref 层：&mut File -> File（同 expressions.rs 字段访问）
                            let mut resolved = self.solver.resolve_type(&base_ty);
                            while let MonoType::Ref { inner, .. } = resolved {
                                resolved = *inner;
                            }
                            let resolved = self.solver.resolve_type(&resolved);
                            let in_schema = |st: &crate::frontend::core::types::StructType| {
                                st.fields.iter().any(|(n, _)| n == field)
                                    || st.methods.contains_key(field)
                            };
                            let schema_ok = match &resolved {
                                MonoType::Struct(st) => in_schema(st),
                                MonoType::TypeRef(tn) => self
                                    .type_defs
                                    .get(tn)
                                    .map(|t| self.solver.resolve_type(t))
                                    .and_then(|t| match t {
                                        MonoType::Struct(st) => Some(in_schema(&st)),
                                        _ => None,
                                    })
                                    .unwrap_or(false),
                                _ => false,
                            };
                            if !schema_ok {
                                let type_name = match &resolved {
                                    MonoType::Struct(st) => st.name.clone(),
                                    MonoType::TypeRef(tn) => tn.clone(),
                                    other => format!("{}", other),
                                };
                                return Err(Box::new(
                                    crate::util::diagnostic::ErrorCodeDefinition::field_not_found(
                                        field, &type_name,
                                    )
                                    .build(),
                                ));
                            }
                        }
                    }
                }
                // 从 value 提取 Lambda params/body
                let (params, body_stmts) = match value {
                    Some(v) => {
                        if let Expr::Lambda { params, body, .. } = v.as_ref() {
                            (params.clone(), body.stmts.clone())
                        } else if let Expr::Block(block) = v.as_ref() {
                            (Vec::new(), block.stmts.clone())
                        } else {
                            return self.check_var_stmt(
                                &name,
                                type_annotation.as_ref(),
                                &[],
                                Some(v.as_ref()),
                                *is_mut,
                            );
                        }
                    }
                    None => {
                        return self.check_var_stmt(
                            &name,
                            type_annotation.as_ref(),
                            &[],
                            None,
                            *is_mut,
                        );
                    }
                };
                let body_block = Block {
                    stmts: body_stmts.clone(),
                    span: stmt.span,
                };
                let result = self.check_fn_stmt(
                    &name,
                    type_annotation.as_ref(),
                    signature_params,
                    &params,
                    &body_stmts,
                    body_block,
                    *stmt_span,
                );
                result
            }
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
            // 元组解构赋值
            crate::frontend::core::parser::ast::StmtKind::DestructureAssign {
                names,
                rhs,
                span,
            } => {
                let rhs_ty = self.check_expr(rhs)?;
                let resolved_ty = self.solver.resolve_type(&rhs_ty);
                match &resolved_ty {
                    MonoType::Tuple(elem_types) => {
                        if elem_types.len() != names.len() {
                            return Err(Box::new(
                                ErrorCodeDefinition::type_mismatch(
                                    &format!("Tuple({})", names.len()),
                                    &format!("Tuple({})", elem_types.len()),
                                )
                                .at(*span)
                                .build(),
                            ));
                        }
                        for (name, elem_ty) in names.iter().zip(elem_types.iter()) {
                            self.scope.add_var(
                                name.name.clone(),
                                PolyType::mono(elem_ty.clone()),
                                false,
                                crate::util::span::Span::default(),
                            );
                        }
                        Ok(())
                    }
                    _ => {
                        // RHS 不是元组类型，为每个名称创建新类型变量
                        for name in names {
                            let ty = self.solver.new_var();
                            self.scope.add_var(
                                name.name.clone(),
                                PolyType::mono(ty),
                                false,
                                crate::util::span::Span::default(),
                            );
                        }
                        Ok(())
                    }
                }
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
                        self.scope.add_var(
                            name.clone(),
                            PolyType::mono(ty),
                            false,
                            crate::util::span::Span::default(),
                        );
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
    #[allow(clippy::too_many_arguments)]
    fn check_fn_stmt(
        &mut self,
        name: &str,
        type_annotation: Option<&crate::frontend::core::parser::ast::Type>,
        signature_params: &[Param],
        params: &[Param],
        _stmts: &[Stmt],
        body: Block,
        _span: crate::util::span::Span,
    ) -> Result<(), Box<Diagnostic>> {
        let generic_params =
            classify_generic_params(signature_params, &|name| self.trait_table.has_trait(name));
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

        // 提取 Type 级别的泛型参数
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
        // 与 checker.rs 中 collect_function_signature 相同逻辑
        let const_generic_params: Vec<_> = generic_params
            .iter()
            .filter(|p| {
                matches!(
                    p.kind,
                    crate::frontend::core::parser::ast::GenericParamKind::Const { .. }
                )
            })
            .collect();

        let mut const_binders: Vec<crate::frontend::core::types::const_data::ConstVarDef> =
            Vec::new();
        if !const_generic_params.is_empty() {
            let candidate_names: std::collections::HashSet<String> = const_generic_params
                .iter()
                .map(|p| p.name.clone())
                .collect();
            let mut used_as_const = std::collections::HashSet::new();

            // 扫描内层 Fn 的 params 判断 const 用途
            if let Some(crate::frontend::core::parser::ast::Type::Fn { return_type, .. }) =
                type_annotation
            {
                if let crate::frontend::core::parser::ast::Type::Fn {
                    params: inner_params,
                    ..
                } = return_type.as_ref()
                {
                    for p in inner_params {
                        crate::frontend::core::typecheck::checker::collect_used_in_type(
                            p,
                            &candidate_names,
                            &mut used_as_const,
                        );
                    }
                }
                crate::frontend::core::typecheck::checker::collect_used_in_type(
                    return_type,
                    &candidate_names,
                    &mut used_as_const,
                );
            }

            let type_param_names: Vec<String> =
                type_generic_params.iter().map(|p| p.name.clone()).collect();
            for (i, gp) in const_generic_params.iter().enumerate() {
                if used_as_const.contains(&gp.name) {
                    if let crate::frontend::core::parser::ast::GenericParamKind::Const {
                        const_type,
                    } = &gp.kind
                    {
                        let type_name = match const_type.as_ref() {
                            crate::frontend::core::parser::ast::Type::Name { name, .. } => {
                                name.clone()
                            }
                            crate::frontend::core::parser::ast::Type::Int(_) => "Int".to_string(),
                            crate::frontend::core::parser::ast::Type::Float(_) => {
                                "Float".to_string()
                            }
                            crate::frontend::core::parser::ast::Type::Bool => "Bool".to_string(),
                            _ => "Int".to_string(),
                        };
                        let kind = crate::frontend::core::types::const_data::ConstKind::from_ast_type_name(&type_name)
                            .unwrap_or(crate::frontend::core::types::const_data::ConstKind::Int(None));
                        let idx = type_param_names.len() + i;
                        const_binders.push(
                            crate::frontend::core::types::const_data::ConstVarDef::new(
                                gp.name.clone(),
                                kind,
                                idx,
                            ),
                        );
                    }
                }
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

                // 泛型函数处理：剥离类型级参数，替换 TypeRef 为类型变量
                let (final_params, final_ret) = if !type_generic_params.is_empty()
                    && fn_param_types.len() >= type_generic_params.len()
                {
                    let mut subst = std::collections::HashMap::new();
                    for gp in &type_generic_params {
                        let fresh_var = self.solver.new_var();
                        subst.insert(gp.name.clone(), fresh_var);
                    }

                    // 添加 const 参数名到 subst
                    for cb in &const_binders {
                        let base_ty = match cb.kind {
                            crate::frontend::core::types::const_data::ConstKind::Int(_) => {
                                MonoType::Int(64)
                            }
                            crate::frontend::core::types::const_data::ConstKind::Bool => {
                                MonoType::Bool
                            }
                            crate::frontend::core::types::const_data::ConstKind::Float(_) => {
                                MonoType::Float(64)
                            }
                        };
                        subst.insert(cb.name.clone(), base_ty);
                    }

                    let inner_fn_ty = Self::substitute_type_refs(fn_return_type.clone(), &subst);
                    match inner_fn_ty {
                        MonoType::Fn {
                            params: inner_params,
                            return_type: inner_ret,
                            ..
                        } => (inner_params, *inner_ret),
                        // return_type 不是 Fn（可能是单值泛型），保持原样
                        _ => (fn_param_types, fn_return_type),
                    }
                } else if !const_binders.is_empty() {
                    // 没有 Type 泛型但有 const 泛型：替换 param_types 和 return_type 中的 const ref
                    let mut subst = std::collections::HashMap::new();
                    for cb in &const_binders {
                        let base_ty = match cb.kind {
                            crate::frontend::core::types::const_data::ConstKind::Int(_) => {
                                MonoType::Int(64)
                            }
                            crate::frontend::core::types::const_data::ConstKind::Bool => {
                                MonoType::Bool
                            }
                            crate::frontend::core::types::const_data::ConstKind::Float(_) => {
                                MonoType::Float(64)
                            }
                        };
                        subst.insert(cb.name.clone(), base_ty);
                    }
                    let substituted_params: Vec<MonoType> = fn_param_types
                        .iter()
                        .map(|t| Self::substitute_type_refs(t.clone(), &subst))
                        .collect();
                    let substituted_ret =
                        Self::substitute_type_refs(fn_return_type.clone(), &subst);
                    (substituted_params, substituted_ret)
                } else {
                    (fn_param_types, fn_return_type)
                };

                let fn_type = MonoType::Fn {
                    params: final_params,
                    return_type: Box::new(final_ret),
                };
                let poly = if const_binders.is_empty() {
                    PolyType::mono(fn_type)
                } else {
                    PolyType::new_with_const(Vec::new(), const_binders.clone(), fn_type)
                };
                self.scope.add_var(
                    name.to_string(),
                    poly,
                    false,
                    crate::util::span::Span::default(),
                );
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
            };
            self.scope.add_var(
                name.to_string(),
                PolyType::mono(fn_type),
                false,
                crate::util::span::Span::default(),
            );
        }

        // 从函数签名提取最内层返回类型（curried 函数的值级返回类型）
        let innermost_ret = type_annotation.and_then(|t| {
            if let crate::frontend::core::parser::ast::Type::Fn { return_type, .. } = t {
                Some(MonoType::from(
                    innermost_return_type(return_type.as_ref()).clone(),
                ))
            } else {
                None
            }
        });

        // Result 错误类型（用于 `?` 运算符检查）
        let fn_result_err = innermost_ret.as_ref().and_then(|ret| match ret {
            MonoType::Result(_, err) => Some((**err).clone()),
            _ => None,
        });
        self.result_err_stack.push(fn_result_err);

        // 预期返回类型（用于 return 语句类型检查）
        self.expected_return_type = innermost_ret;

        // 当 body 的参数缺少类型标注时，从函数签名中补全
        // 例如: Point.getX: (self: &Point) -> Float = (self) => { ... }
        // 此时 body 的 params 为 [Param { name: "self", ty: None }]
        // 需要从 type_annotation 的 Fn params 中获取类型
        let owned_merged_params: Vec<Param>;
        let params = if let Some(crate::frontend::core::parser::ast::Type::Fn {
            params: sig_param_types,
            return_type,
            ..
        }) = type_annotation
        {
            // 当 lambda 参数缺类型标注时，从最内层 Fn（值级参数层）补全类型
            let value_param_types = innermost_fn_param_types(sig_param_types, return_type);
            let needs_merge = params.iter().any(|p| p.ty.is_none())
                && !params.is_empty()
                && value_param_types.len() >= params.len();
            if needs_merge {
                owned_merged_params = params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        if p.ty.is_none() {
                            if let Some(sig_ty) = value_param_types.get(i) {
                                Param {
                                    name: p.name.clone(),
                                    ty: Some(sig_ty.clone()),
                                    is_mut: p.is_mut,
                                    span: p.span,
                                }
                            } else {
                                p.clone()
                            }
                        } else {
                            p.clone()
                        }
                    })
                    .collect();
                &owned_merged_params
            } else {
                params
            }
        } else {
            params
        };

        // 补充 curry 后续组的值参数（如 `factorial: (N: Int) -> (n: N) -> Int` 的 `n`）
        // signature_params 现含全部 curry 组带名参数；第一组已被 extract_generic_params 处理，
        // 后续组值参数（不在 generic_params 名单）需补进 params 供 check_fn_def 绑定进作用域。
        let generic_names: std::collections::HashSet<&str> =
            generic_params.iter().map(|p| p.name.as_str()).collect();
        let mut params: Vec<Param> = params.to_vec();
        for p in signature_params {
            if !generic_names.contains(p.name.as_str())
                && !params.iter().any(|ep| ep.name == p.name)
            {
                params.push(p.clone());
            }
        }

        // 如果有 const 泛型参数，构建 subst 传给 check_fn_def_with_subst
        let const_subst = if !const_binders.is_empty() {
            let mut subst = std::collections::HashMap::new();
            for cb in &const_binders {
                let base_ty = match cb.kind {
                    crate::frontend::core::types::const_data::ConstKind::Int(_) => {
                        MonoType::Int(64)
                    }
                    crate::frontend::core::types::const_data::ConstKind::Bool => MonoType::Bool,
                    crate::frontend::core::types::const_data::ConstKind::Float(_) => {
                        MonoType::Float(64)
                    }
                };
                subst.insert(cb.name.clone(), base_ty);
            }
            subst
        } else {
            std::collections::HashMap::new()
        };

        let out = self.check_fn_def_with_subst(name, &params, &body, &const_subst);

        // Clear expected return type after function body checking
        self.expected_return_type = None;

        // 退出函数 Result 上下文
        let _ = self.result_err_stack.pop();

        out
    }

    /// 替换 MonoType 中的 TypeRef 名称为对应的类型变量
    ///
    /// 用于泛型函数类型推断：将 TypeRef("T") 替换为 solver 中的新类型变量。
    fn substitute_type_refs(
        ty: MonoType,
        subst: &std::collections::HashMap<String, MonoType>,
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
            other => other,
        }
    }

    /// 检查变量语句
    ///
    /// 处理 Binding 类型的变量声明。
    fn check_var_stmt(
        &mut self,
        name: &str,
        type_annotation: Option<&crate::frontend::core::parser::ast::Type>,
        prelude_stmts: &[Stmt],
        initializer: Option<&Expr>,
        is_mut: bool,
    ) -> Result<(), Box<Diagnostic>> {
        // 处理 prelude 语句（编译期求值部分）
        for stmt in prelude_stmts {
            self.check_stmt(stmt)?;
        }

        let ty = match (initializer, type_annotation) {
            (Some(init_expr), Some(type_ann)) => {
                let init_ty = self.check_expr(init_expr)?;
                // Try generic type instantiation for List(Int) → struct expansion
                let ann_ty = self
                    .try_instantiate_generic_type(type_ann)
                    .unwrap_or_else(|| MonoType::from(type_ann.clone()));
                // RFC-027 Phase 2.5: 证明函数类型解析（从 AST Type 直接提取）
                // IsPositive(5) → Refined { base: Int(64), constraint: Call("IsPositive", [5]) }
                let ann_ty = if let crate::frontend::core::parser::ast::Type::Generic {
                    name,
                    args,
                    ..
                } = type_ann
                {
                    if !args.is_empty() {
                        if let Some(base) = self.proof_fn_bases.get(name) {
                            let constraint_args: Vec<crate::frontend::core::types::const_data::ConstExpr> = args
                                .iter()
                                .filter_map(|a| {
                                    if let crate::frontend::core::parser::ast::Type::ConstExpr(expr) = a {
                                        crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(expr)
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            let constraint =
                                crate::frontend::core::types::const_data::ConstExpr::Call {
                                    func: name.clone(),
                                    args: constraint_args,
                                };
                            MonoType::Refined {
                                base: Box::new(base.clone()),
                                constraint,
                            }
                        } else {
                            ann_ty
                        }
                    } else {
                        ann_ty
                    }
                } else {
                    ann_ty
                };
                // Check type assignment compatibility:
                // - Float cannot be assigned to Int (no implicit narrowing)
                //   Resolve TypeRef("Int") to Int(64) for comparison (§3.2: Int defaults to 8 bytes)
                // RFC-027: Refined 类型用 base 做 unify
                let resolved_ann = match &ann_ty {
                    MonoType::Refined { base, .. } => *base.clone(),
                    _ => ann_ty.clone(),
                };
                if matches!(
                    (&resolved_ann, &init_ty),
                    (MonoType::Int(_), MonoType::Float(_))
                ) {
                    return Err(Box::new(
                        ErrorCodeDefinition::type_mismatch(
                            &format!("{}", ann_ty),
                            &format!("{}", init_ty),
                        )
                        .build(),
                    ));
                }
                // Resolve TypeRef("Circle") → Struct(Circle) for the source type,
                // enabling interface assignment checks like d: Drawable = c.
                // The annotation type is NOT resolved when it's a struct/interface TypeRef,
                // so the solver can detect the Struct vs TypeRef pattern.
                let resolved_init = self.resolve_type_ref_type(&init_ty);
                // RFC-027: Refined 类型用 base 做 unify
                let resolved_ann = match &ann_ty {
                    MonoType::Refined { base, .. } => *base.clone(),
                    _ => ann_ty.clone(),
                };
                // Check Int → Float subtype (widening conversion is always safe)
                let is_int_to_float = matches!(
                    (&resolved_ann, &resolved_init),
                    (MonoType::Float(_), MonoType::Int(_))
                );
                if !is_int_to_float {
                    let unify_result = self.solver.unify(&resolved_init, &resolved_ann);
                    if unify_result.is_err() {
                        // Unify failed — check structural subtyping (interface assignment)
                        let is_structural_subtype = matches!(
                            (&resolved_init, &resolved_ann),
                            (MonoType::Struct(s), MonoType::TypeRef(iface)) if s.interfaces.contains(iface)
                        );
                        // 泛型类型构造：当 init 是泛型结构体（含 TypeRef 字段）且
                        // annotation 是实例化后的结构体时，跳过 unify 直接使用 annotation 类型
                        let is_generic_constructor = match (&resolved_init, &resolved_ann) {
                            (MonoType::Struct(s_init), MonoType::Struct(s_ann)) => {
                                s_init.name == s_ann.name
                                    && self.generic_type_defs.contains_key(&s_init.name)
                            }
                            _ => false,
                        };
                        if !is_structural_subtype && !is_generic_constructor {
                            return Err(Box::new(
                                ErrorCodeDefinition::type_mismatch(
                                    &format!("{}", ann_ty),
                                    &format!("{}", init_ty),
                                )
                                .build(),
                            ));
                        }
                    }
                }
                // 类型构造器：当 type_ann 是 Type(MetaType) 且 init_ty 是 Struct 时，
                // 存 Struct 类型而不是 MetaType，使 Point(1.0, 2.0) 自然工作
                if matches!(ann_ty, MonoType::MetaType { .. })
                    && matches!(resolved_init, MonoType::Struct(_))
                {
                    resolved_init
                } else {
                    ann_ty
                }
            }
            (Some(init_expr), None) => self.check_expr(init_expr)?,
            (None, Some(type_ann)) => self
                .try_instantiate_generic_type(type_ann)
                .unwrap_or_else(|| MonoType::from(type_ann.clone())),
            (None, None) => self.solver.new_var(),
        };

        if self.scope.var_in_current_scope(name) {
            // mut 变量被重新赋值 → kill Γ 中依赖该变量的假设
            if let Some(gamma) = &mut self.gamma {
                if is_mut {
                    gamma.kill(name);
                }
            }
            // 统一变量类型并写回 scope，确保后续类型推断正确
            self.assign_var(name, ty);
            return Ok(());
        }

        if self.scope.var_in_any_scope(name) {
            // mut 变量被重新赋值 → kill Γ 中依赖该变量的假设
            if let Some(gamma) = &mut self.gamma {
                if is_mut {
                    gamma.kill(name);
                }
            }
            self.assign_var(name, ty);
            return Ok(());
        }

        self.scope.add_var(
            name.to_string(),
            PolyType::mono(ty),
            is_mut,
            crate::util::span::Span::default(),
        );
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

        self.scope.add_var(
            var.to_string(),
            PolyType::mono(elem_ty),
            var_mut,
            crate::util::span::Span::default(),
        );

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
                        inferrer.set_method_bindings(&self.method_bindings);
                        inferrer.set_type_defs(&self.type_defs);
                        inferrer.set_generic_type_defs(&self.generic_type_defs);
                        inferrer.set_dep_env(&self.dep_env);
                        if let Some(gamma) = &mut self.gamma {
                            inferrer.set_gamma(gamma);
                        }
                        let result = inferrer.infer_expr(expr).map_err(Box::new);
                        self.instantiation_requests
                            .extend(inferrer.instantiation_requests);
                        result
                    }
                }
            }
            // 其他表达式：委托给 ExpressionInferrer
            _ => {
                let current_result_err = self.current_result_err();
                let mut inferrer = super::ExpressionInferrer::with_full_context(
                    &mut self.scope,
                    &mut self.solver,
                    &self.overload_candidates,
                    &self.native_signatures,
                    current_result_err,
                    self.expected_return_type.clone(),
                    &self.method_bindings,
                );
                inferrer.set_type_defs(&self.type_defs);
                inferrer.set_generic_type_defs(&self.generic_type_defs);
                inferrer.set_dep_env(&self.dep_env);
                if let Some(gamma) = &mut self.gamma {
                    inferrer.set_gamma(gamma);
                }
                let result = inferrer.infer_expr(expr).map_err(Box::new);
                self.instantiation_requests
                    .extend(inferrer.instantiation_requests);
                result
            }
        }
    }
}

/// 从最内层 Fn 类型中提取参数类型（值级参数层）
///
/// `(T: Type) -> ((x: Int) -> Int)` → `[Int]`（最内层 Fn 的参数）
/// 非嵌套场景 `(x: Int) -> Int` → `[Int]`
fn innermost_fn_param_types(
    outer_params: &[crate::frontend::core::parser::ast::Type],
    return_type: &crate::frontend::core::parser::ast::Type,
) -> Vec<crate::frontend::core::parser::ast::Type> {
    if let crate::frontend::core::parser::ast::Type::Fn {
        params: inner_params,
        return_type: inner_ret,
    } = return_type
    {
        innermost_fn_param_types(inner_params, inner_ret)
    } else {
        outer_params.to_vec()
    }
}

/// 从嵌套 Fn 类型中提取最内层的返回类型
///
/// `(Int) -> ((Int) -> Int)` → `Int`
/// `Int` → `Int`（非 Fn 直接返回自身）
fn innermost_return_type(
    ty: &crate::frontend::core::parser::ast::Type
) -> &crate::frontend::core::parser::ast::Type {
    if let crate::frontend::core::parser::ast::Type::Fn { return_type, .. } = ty {
        innermost_return_type(return_type.as_ref())
    } else {
        ty
    }
}
