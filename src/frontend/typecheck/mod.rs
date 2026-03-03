//! 类型检查器模块
//!
//! 实现 YaoXiang 语言的类型检查器，支持：
//! - Hindley-Milner 类型推断
//! - 泛型函数和泛型类型
//! - 完整的错误收集
//! - RFC-004/010/011 支撑

use std::collections::{HashMap, HashSet};

use crate::frontend::core::parser::ast::{Module, Expr};

// 导入推断模块（合并了原 checking/ 模块）
pub mod inference;

// 导入特化模块
pub mod specialization;

// 导入特质模块
pub mod traits;

// 导入 GAT 模块
pub mod gat;

// 导入重载解析模块（在 TypeEnvironment 之前声明）
pub mod overload;

// 导入类型求值器模块
pub mod type_eval;

// 语义信息数据库
pub mod semantic_db;

// 死代码分析器
pub mod dead_code;

// 导入测试模块
#[cfg(test)]
mod tests;

// 使用 core 层的类型系统（显式导出以避免 ambiguous glob re-exports）
pub use crate::frontend::core::type_system::{
    MonoType, PolyType, TypeVar, TypeBinding, StructType, EnumType, TypeConstraint,
    TypeConstraintSolver, SendSyncConstraint, SendSyncSolver, TypeMismatch, TypeConstraintError,
    ConstValue, ConstExpr, ConstKind, ConstVarDef, UniverseLevel,
};

// 重新导出推断、特化等模块
pub use inference::*;
pub use specialization::*;
pub use gat::*;
pub use overload::*;
pub use type_eval::*;

// 导入诊断系统
pub use crate::util::diagnostic::{Diagnostic, ErrorCollector, ErrorCodeDefinition, I18nRegistry};

/// 类型推断结果
pub type TypeResult<T> = Result<T, Diagnostic>;

/// 类型错误收集器
pub type TypeErrorCollector = ErrorCollector<Diagnostic>;

// 类型环境
#[derive(Debug, Default)]
pub struct TypeEnvironment {
    pub vars: HashMap<String, PolyType>,
    pub types: HashMap<String, PolyType>,
    pub solver: TypeConstraintSolver,
    pub errors: TypeErrorCollector,
    /// 导入追踪 - 模块导入信息
    /// 包含源模块ID用于访问控制
    pub imports: Vec<ImportInfo>,
    /// 当前模块的导出项
    pub exports: HashSet<String>,
    /// 方法绑定关系: "Type.method" -> FunctionType
    /// 用于存储显式绑定和 pub 自动绑定
    pub method_bindings: HashMap<String, crate::frontend::core::type_system::MonoType>,
    /// 模块名称
    pub module_name: String,
    /// 重载候选存储: 函数名 -> 多个重载版本
    /// 用于支持函数重载解析
    pub overload_candidates: HashMap<String, Vec<overload::OverloadCandidate>>,
    /// Trait 表：存储所有已解析的 Trait 定义和实现
    pub trait_table: super::type_level::trait_bounds::TraitTable,
    /// Native 函数签名表：存储已注册的 native 函数类型签名
    /// Key: 函数名（如 "std.io.println"），Value: 函数类型
    pub native_signatures: HashMap<String, MonoType>,
    /// 模块注册表 - 提供统一的模块查询接口
    pub module_registry: crate::frontend::module::registry::ModuleRegistry,
}

impl TypeEnvironment {
    /// 创建新的类型环境
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建新的类型环境（带模块名）
    pub fn new_with_module(module_name: String) -> Self {
        Self {
            module_name,
            trait_table: super::type_level::trait_bounds::TraitTable::default(),
            module_registry: crate::frontend::module::registry::ModuleRegistry::with_std(),
            ..Self::default()
        }
    }

    /// 添加变量绑定
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.vars.insert(name, poly);
    }

    /// 获取变量类型
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.vars.get(name)
    }

    /// 获取求解器
    pub fn solver(&mut self) -> &mut TypeConstraintSolver {
        &mut self.solver
    }

    /// 添加类型定义
    pub fn add_type(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.types.insert(name, poly);
    }

    /// 获取类型定义
    pub fn get_type(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.types.get(name)
    }

    /// 添加方法绑定
    /// 例如: Point.distance = distance 存储为 "Point.distance" -> fn_type
    pub fn add_method_binding(
        &mut self,
        type_name: &str,
        method_name: &str,
        fn_type: MonoType,
    ) {
        let key = format!("{}.{}", type_name, method_name);
        self.method_bindings.insert(key.clone(), fn_type);
        // 方法绑定也导出
        self.exports.insert(key);
    }

    /// 获取方法绑定
    pub fn get_method_binding(
        &self,
        type_name: &str,
        method_name: &str,
    ) -> Option<&MonoType> {
        let key = format!("{}.{}", type_name, method_name);
        self.method_bindings.get(&key)
    }

    /// 添加导出项
    pub fn add_export(
        &mut self,
        name: &str,
    ) {
        self.exports.insert(name.to_string());
    }

    /// 检查是否是导出项
    pub fn is_exported(
        &self,
        name: &str,
    ) -> bool {
        self.exports.contains(name)
    }

    /// 检查名称是否可见（可从当前模块访问）
    ///
    /// 一个名称在以下情况下可见：
    /// 1. 在当前模块中定义
    /// 2. 被当前模块导出
    /// 3. 从导入了该名称的模块导入
    pub fn is_visible(
        &self,
        name: &str,
    ) -> bool {
        // 当前模块定义的变量总是可见的
        if self.vars.contains_key(name) {
            return true;
        }
        // 当前模块定义的类型总是可见的
        if self.types.contains_key(name) {
            return true;
        }
        // 当前模块导出的内容可见
        if self.exports.contains(name) {
            return true;
        }
        false
    }

    // ============ Trait 相关方法 ============

    /// 添加 Trait 定义
    pub fn add_trait(
        &mut self,
        definition: super::type_level::trait_bounds::TraitDefinition,
    ) {
        self.trait_table.add_trait(definition);
    }

    /// 获取 Trait 定义
    pub fn get_trait(
        &self,
        name: &str,
    ) -> Option<&super::type_level::trait_bounds::TraitDefinition> {
        self.trait_table.get_trait(name)
    }

    /// 检查 Trait 是否已定义
    pub fn has_trait(
        &self,
        name: &str,
    ) -> bool {
        self.trait_table.has_trait(name)
    }

    /// 添加 Trait 实现
    pub fn add_trait_impl(
        &mut self,
        impl_: super::type_level::trait_bounds::TraitImplementation,
    ) {
        self.trait_table.add_impl(impl_);
    }

    /// 检查类型是否实现了 Trait
    pub fn has_trait_impl(
        &self,
        trait_name: &str,
        for_type: &str,
    ) -> bool {
        self.trait_table.has_impl(trait_name, for_type)
    }

    /// 获取 Trait 实现
    pub fn get_trait_impl(
        &self,
        trait_name: &str,
        for_type: &str,
    ) -> Option<&super::type_level::trait_bounds::TraitImplementation> {
        self.trait_table.get_impl(trait_name, for_type)
    }
    /// 注册 native 函数签名
    pub fn add_native_signature(
        &mut self,
        name: &str,
        sig: MonoType,
    ) {
        self.native_signatures.insert(name.to_string(), sig);
    }

    /// 获取 native 函数签名
    pub fn get_native_signature(
        &self,
        name: &str,
    ) -> Option<&MonoType> {
        self.native_signatures.get(name)
    }

    /// 检查是否是已注册的 native 函数
    pub fn is_native_function(
        &self,
        name: &str,
    ) -> bool {
        self.native_signatures.contains_key(name)
    }
}

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

        Self {
            env,
            body_checker: None,
            semantic_db: semantic_db::SemanticDB::new(),
        }
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
            if let crate::frontend::core::parser::ast::StmtKind::TypeDef {
                name,
                definition,
                generic_params,
                ..
            } = &stmt.kind
            {
                self.add_type_definition(name, definition, generic_params, stmt.span);
            }
        }

        // 第二遍：收集所有函数签名（使其可被前向引用）
        for stmt in &module.items {
            self.collect_function_signature(stmt);
        }

        // 收集所有导出项
        self.collect_exports(module);

        // 初始化函数体检查器
        let mut body_checker = inference::StatementChecker::new(self.env.solver());
        // 设置 native 函数签名表
        body_checker.set_native_signatures(self.env.native_signatures.clone());
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
                crate::frontend::core::type_system::MonoType::Fn { .. }
            );
            if !is_function && !local_var_types.contains_key(name) {
                local_var_types.insert(name.clone(), poly.body.clone());
            }
        }

        // 注意：由于 body_checker.solver 是克隆的，无法通过 solver.resolve() 来解析类型变量。
        // 幸运的是，assign_var 方法已经将更新后的类型写回到了 scope 中，
        // 所以这里直接使用 scope 中的类型即可，不需要额外 resolve。
        // （注：如果后续需要支持更复杂的泛型推导，可能需要重新设计 solver 的共享机制）

        // 语义收集：遍历 AST 构建 SemanticDB
        // 一次遍历，多处使用 —— 利用 typecheck 已有的类型信息
        self.collect_semantic_tokens(module);

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
            crate::frontend::core::parser::ast::StmtKind::Fn {
                name,
                type_annotation,
                params,
                is_pub,
                ..
            } => {
                // 处理统一函数语法
                let (param_types, return_type) = if let Some(type_ann) = type_annotation {
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
                self.env
                    .add_var(name.clone(), PolyType::mono(fn_ty.clone()));

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
        let module_ty = MonoType::Struct(crate::frontend::core::type_system::mono::StructType {
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
                    MonoType::Struct(crate::frontend::core::type_system::mono::StructType {
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
                    let decl = format!("Type: Type[{}] = ...", generic_params.join(", "));
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

        for trait_name in super::type_level::auto_derive::BUILTIN_DERIVES {
            // 检查是否可以自动派生
            let can_derive =
                super::type_level::auto_derive::can_auto_derive(trait_table, trait_name, fields);

            if can_derive {
                // 检查是否已有显式实现
                if !self.env.has_trait_impl(trait_name, type_name) {
                    // 生成自动派生实现
                    if let Some(impl_) =
                        super::type_level::auto_derive::generate_auto_derive(type_name, trait_name)
                    {
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
        for stmt in &module.items {
            match &stmt.kind {
                // pub 函数导出函数名
                crate::frontend::core::parser::ast::StmtKind::Fn { name, is_pub, .. }
                    if *is_pub =>
                {
                    self.env.add_export(name);
                }
                // 类型定义默认导出
                crate::frontend::core::parser::ast::StmtKind::TypeDef { name, .. } => {
                    self.env.add_export(name);
                }
                // 方法绑定导出为 Type.method
                crate::frontend::core::parser::ast::StmtKind::MethodBind {
                    type_name,
                    method_name,
                    ..
                } => {
                    self.env
                        .add_export(&format!("{}.{}", type_name, method_name));
                }
                _ => {}
            }
        }
    }

    // ============ 语义信息收集 ============

    /// 从已完成类型检查的 AST 收集语义 tokens
    ///
    /// 利用 typecheck 阶段已有的类型信息，一次遍历产出语义数据。
    /// 收集规则：
    /// - StmtKind::Fn        → Function (定义)
    /// - StmtKind::TypeDef   → Type (定义)
    /// - StmtKind::Var       → Variable (定义)
    /// - StmtKind::MethodBind→ Method (定义)
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
            if let StmtKind::TypeDef { definition, .. } = &stmt.kind {
                if let Type::Variant(variants) = definition {
                    for v in variants {
                        names.insert(v.name.clone());
                    }
                }
            }
        }
        names
    }

    fn imported_module_roots_from_module(module: &Module) -> HashSet<String> {
        use crate::frontend::core::parser::ast::StmtKind;

        let mut roots = HashSet::new();
        roots.insert("std".to_string());
        for stmt in &module.items {
            if let StmtKind::Use {
                path, path_parts, ..
            } = &stmt.kind
            {
                if let Some(first) = path_parts.first() {
                    roots.insert(first.name.clone());
                } else if let Some(first) = path.split('.').next() {
                    if !first.is_empty() {
                        roots.insert(first.to_string());
                    }
                }
            }
        }
        roots
    }

    fn collect_type_tokens(
        &mut self,
        file_path: &str,
        ty: &crate::frontend::core::parser::ast::Type,
    ) {
        use crate::frontend::core::parser::ast::Type;

        match ty {
            Type::Name { name, span } => {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::Type,
                        modifiers: vec![],
                        span: *span,
                    },
                );
            }
            Type::Generic {
                name,
                name_span,
                args,
            } => {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::Type,
                        modifiers: vec![],
                        span: *name_span,
                    },
                );
                for arg in args {
                    self.collect_type_tokens(file_path, arg);
                }
            }
            Type::NamedStruct {
                name,
                name_span,
                fields,
            } => {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::Type,
                        modifiers: vec![],
                        span: *name_span,
                    },
                );
                for f in fields {
                    self.collect_type_tokens(file_path, &f.ty);
                }
            }
            Type::Struct {
                fields, bindings, ..
            } => {
                for f in fields {
                    self.collect_type_tokens(file_path, &f.ty);
                }
                for b in bindings {
                    match &b.kind {
                        crate::frontend::core::parser::ast::BindingKind::Anonymous {
                            params,
                            return_type,
                            ..
                        } => {
                            for p in params {
                                if let Some(t) = &p.ty {
                                    self.collect_type_tokens(file_path, t);
                                }
                            }
                            self.collect_type_tokens(file_path, return_type);
                        }
                        crate::frontend::core::parser::ast::BindingKind::External { .. }
                        | crate::frontend::core::parser::ast::BindingKind::DefaultExternal {
                            ..
                        } => {}
                    }
                }
            }
            Type::Union(variants) => {
                for (_name, maybe_ty) in variants {
                    if let Some(t) = maybe_ty {
                        self.collect_type_tokens(file_path, t);
                    }
                }
            }
            Type::Variant(variants) => {
                for v in variants {
                    for (_param_name, t) in &v.params {
                        self.collect_type_tokens(file_path, t);
                    }
                }
            }
            Type::Tuple(types) => {
                for t in types {
                    self.collect_type_tokens(file_path, t);
                }
            }
            Type::Fn {
                params,
                return_type,
            } => {
                for t in params {
                    self.collect_type_tokens(file_path, t);
                }
                self.collect_type_tokens(file_path, return_type);
            }
            Type::Option(inner) => self.collect_type_tokens(file_path, inner),
            Type::Result(ok, err) => {
                self.collect_type_tokens(file_path, ok);
                self.collect_type_tokens(file_path, err);
            }
            Type::AssocType {
                host_type,
                assoc_name,
                assoc_name_span,
                assoc_args,
            } => {
                self.collect_type_tokens(file_path, host_type);
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: assoc_name.clone(),
                        token_type: semantic_db::SemanticTokenType::Type,
                        modifiers: vec![],
                        span: *assoc_name_span,
                    },
                );
                for t in assoc_args {
                    self.collect_type_tokens(file_path, t);
                }
            }
            Type::Sum(types) => {
                for t in types {
                    self.collect_type_tokens(file_path, t);
                }
            }
            Type::Literal {
                name,
                name_span,
                base_type,
            } => {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::Type,
                        modifiers: vec![],
                        span: *name_span,
                    },
                );
                self.collect_type_tokens(file_path, base_type);
            }
            Type::Ptr(inner) => self.collect_type_tokens(file_path, inner),
            Type::MetaType { name_span, args } => {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: "Type".to_string(),
                        token_type: semantic_db::SemanticTokenType::Type,
                        modifiers: vec![],
                        span: *name_span,
                    },
                );
                for t in args {
                    self.collect_type_tokens(file_path, t);
                }
            }
            Type::Int(_)
            | Type::Float(_)
            | Type::Char
            | Type::String
            | Type::Bytes
            | Type::Bool
            | Type::Void
            | Type::Enum(_) => {}
        }
    }

    fn is_struct_binding(
        &self,
        name: &str,
    ) -> bool {
        self.env
            .get_var(name)
            .is_some_and(|poly| matches!(poly.body, MonoType::Struct(_)))
    }

    fn collect_call_target_tokens(
        &mut self,
        file_path: &str,
        expr: &Expr,
        scope_idx: usize,
        declared: &mut HashMap<usize, HashSet<String>>,
        constructor_names: &HashSet<String>,
        imported_module_roots: &HashSet<String>,
        is_terminal: bool,
    ) {
        use semantic_db::SemanticTokenType;

        match expr {
            Expr::FieldAccess {
                expr: inner,
                field,
                span,
            } => {
                self.collect_call_target_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                    false,
                );

                let is_module_path = Self::is_module_path_expr(inner, imported_module_roots);
                let token_type = if is_terminal {
                    if is_module_path {
                        SemanticTokenType::Function
                    } else {
                        SemanticTokenType::Method
                    }
                } else if is_module_path {
                    SemanticTokenType::Namespace
                } else {
                    SemanticTokenType::Property
                };

                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: field.clone(),
                        token_type,
                        modifiers: vec![],
                        span: *span,
                    },
                );
            }
            Expr::Var(name, span) => {
                let token_type = if imported_module_roots.contains(name) {
                    SemanticTokenType::Namespace
                } else if constructor_names.contains(name) {
                    SemanticTokenType::EnumMember
                } else if self.is_struct_binding(name) {
                    SemanticTokenType::Type
                } else if is_terminal {
                    SemanticTokenType::Function
                } else if let Some(poly) = self.env.get_var(name) {
                    if matches!(poly.body, MonoType::Fn { .. }) {
                        SemanticTokenType::Function
                    } else {
                        SemanticTokenType::Variable
                    }
                } else {
                    SemanticTokenType::Variable
                };

                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type,
                        modifiers: vec![],
                        span: *span,
                    },
                );
            }
            _ => {
                self.collect_expr_tokens(
                    file_path,
                    expr,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
        }
    }

    fn is_module_path_expr(
        expr: &Expr,
        imported_module_roots: &HashSet<String>,
    ) -> bool {
        match expr {
            Expr::Var(name, _) => imported_module_roots.contains(name),
            Expr::FieldAccess { expr: inner, .. } => {
                Self::is_module_path_expr(inner, imported_module_roots)
            }
            _ => false,
        }
    }

    fn collect_semantic_tokens(
        &mut self,
        module: &Module,
    ) {
        use crate::frontend::core::parser::ast::StmtKind;
        use semantic_db::{
            SemanticToken, SemanticTokenType, SemanticTokenModifier, ScopeInfo, ScopeKind,
        };

        let fp = self.env.module_name.clone();

        let mut declared: HashMap<usize, HashSet<String>> = HashMap::new();
        declared.insert(0, HashSet::new());
        let constructor_names = Self::constructor_names_from_module(module);
        let imported_module_roots = Self::imported_module_roots_from_module(module);

        // 添加全局作用域
        self.semantic_db.add_scope(
            &fp,
            ScopeInfo {
                span: module.span,
                parent: None,
                symbols: Vec::new(),
                kind: ScopeKind::Global,
            },
        );

        let mut global_symbols = Vec::new();

        for stmt in &module.items {
            match &stmt.kind {
                StmtKind::Fn {
                    name,
                    params,
                    is_pub,
                    generic_params,
                    type_annotation,
                    body,
                    ..
                } => {
                    // 函数名 → Function (定义)
                    let mut modifiers = vec![SemanticTokenModifier::Declaration];
                    if *is_pub {
                        modifiers.push(SemanticTokenModifier::Public);
                    }
                    if !generic_params.is_empty() {
                        modifiers.push(SemanticTokenModifier::Generic);
                    }
                    self.semantic_db.add_token(
                        &fp,
                        SemanticToken {
                            name: name.clone(),
                            token_type: SemanticTokenType::Function,
                            modifiers,
                            span: stmt.span,
                        },
                    );
                    global_symbols.push(name.clone());

                    // 参数 → Parameter (定义)
                    for param in params {
                        self.semantic_db.add_token(
                            &fp,
                            SemanticToken {
                                name: param.name.clone(),
                                token_type: SemanticTokenType::Parameter,
                                modifiers: vec![SemanticTokenModifier::Declaration],
                                span: param.span,
                            },
                        );
                    }

                    // 泛型参数 → TypeParameter (定义)
                    for gp in generic_params {
                        let gp_name = match &gp.kind {
                            crate::frontend::core::parser::ast::GenericParamKind::Type => {
                                gp.name.clone()
                            }
                            _ => gp.name.clone(),
                        };
                        self.semantic_db.add_token(
                            &fp,
                            SemanticToken {
                                name: gp_name,
                                token_type: SemanticTokenType::TypeParameter,
                                modifiers: vec![SemanticTokenModifier::Declaration],
                                span: stmt.span, // 泛型参数暂用语句 span
                            },
                        );
                    }

                    // 函数体作用域
                    // 泛型约束中的类型引用
                    for gp in generic_params {
                        for c in &gp.constraints {
                            self.collect_type_tokens(&fp, c);
                        }
                    }

                    // 函数签名中的类型引用
                    if let Some(ty) = type_annotation {
                        self.collect_type_tokens(&fp, ty);
                    }

                    let scope_idx = self
                        .semantic_db
                        .get_scopes(&fp)
                        .map(|s| s.len())
                        .unwrap_or(0);
                    declared.insert(scope_idx, params.iter().map(|p| p.name.clone()).collect());
                    self.semantic_db.add_scope(
                        &fp,
                        ScopeInfo {
                            span: stmt.span,
                            parent: Some(0), // 全局作用域
                            symbols: params.iter().map(|p| p.name.clone()).collect(),
                            kind: ScopeKind::Function,
                        },
                    );

                    // 递归收集函数体中的表达式
                    for body_stmt in &body.0 {
                        self.collect_stmt_tokens(
                            &fp,
                            body_stmt,
                            scope_idx,
                            &mut declared,
                            &constructor_names,
                            &imported_module_roots,
                        );
                    }
                    if let Some(ret_expr) = &body.1 {
                        self.collect_expr_tokens(
                            &fp,
                            ret_expr,
                            scope_idx,
                            &mut declared,
                            &constructor_names,
                            &imported_module_roots,
                        );
                    }
                }
                StmtKind::TypeDef {
                    name,
                    name_span,
                    definition,
                    generic_params,
                } => {
                    // 类型名 → Type (定义)
                    let mut modifiers = vec![SemanticTokenModifier::Declaration];
                    if !generic_params.is_empty() {
                        modifiers.push(SemanticTokenModifier::Generic);
                    }
                    self.semantic_db.add_token(
                        &fp,
                        SemanticToken {
                            name: name.clone(),
                            token_type: SemanticTokenType::Type,
                            modifiers,
                            span: *name_span,
                        },
                    );
                    global_symbols.push(name.clone());

                    // ç±»åž‹å®šä¹‰ä½“ä¸­çš„ç±»åž‹å¼•ç”¨
                    self.collect_type_tokens(&fp, definition);

                    // Variant constructors â†’ EnumMember (å®šä¹‰)
                    if let crate::frontend::core::parser::ast::Type::Variant(variants) = definition
                    {
                        for v in variants {
                            self.semantic_db.add_token(
                                &fp,
                                SemanticToken {
                                    name: v.name.clone(),
                                    token_type: SemanticTokenType::EnumMember,
                                    modifiers: vec![SemanticTokenModifier::Declaration],
                                    span: v.name_span,
                                },
                            );
                        }
                    }
                }
                StmtKind::Var {
                    name,
                    name_span,
                    type_annotation,
                    initializer,
                    ..
                } => {
                    // 变量名 → Variable (定义)
                    let is_declaration = declared.entry(0).or_default().insert(name.clone());
                    let modifiers = if is_declaration {
                        vec![SemanticTokenModifier::Declaration]
                    } else {
                        vec![SemanticTokenModifier::Mutable]
                    };
                    self.semantic_db.add_token(
                        &fp,
                        SemanticToken {
                            name: name.clone(),
                            token_type: SemanticTokenType::Variable,
                            modifiers,
                            span: *name_span,
                        },
                    );
                    if is_declaration {
                        global_symbols.push(name.clone());
                    }

                    if let Some(ty) = type_annotation {
                        self.collect_type_tokens(&fp, ty);
                    }

                    if let Some(init) = initializer {
                        self.collect_expr_tokens(
                            &fp,
                            init,
                            0,
                            &mut declared,
                            &constructor_names,
                            &imported_module_roots,
                        );
                    }
                }
                StmtKind::MethodBind {
                    type_name,
                    method_name,
                    method_type,
                    params,
                    ..
                } => {
                    // 类型名.方法名 → Method (定义)
                    self.semantic_db.add_token(
                        &fp,
                        SemanticToken {
                            name: format!("{}.{}", type_name, method_name),
                            token_type: SemanticTokenType::Method,
                            modifiers: vec![SemanticTokenModifier::Declaration],
                            span: stmt.span,
                        },
                    );

                    // 参数 → Parameter (定义)
                    for param in params {
                        self.semantic_db.add_token(
                            &fp,
                            SemanticToken {
                                name: param.name.clone(),
                                token_type: SemanticTokenType::Parameter,
                                modifiers: vec![SemanticTokenModifier::Declaration],
                                span: param.span,
                            },
                        );
                    }

                    self.collect_type_tokens(&fp, method_type);
                }
                StmtKind::Use { path_parts, .. } => {
                    // 模块路径按段着色：`std.io` → `std`、`io`
                    for part in path_parts {
                        self.semantic_db.add_token(
                            &fp,
                            SemanticToken {
                                name: part.name.clone(),
                                token_type: SemanticTokenType::Namespace,
                                modifiers: vec![],
                                span: part.span,
                            },
                        );
                    }
                }
                StmtKind::Expr(expr) => {
                    self.collect_expr_tokens(
                        &fp,
                        expr,
                        0,
                        &mut declared,
                        &constructor_names,
                        &imported_module_roots,
                    );
                }
                StmtKind::For {
                    var,
                    iterable,
                    body,
                    ..
                } => {
                    declared.entry(0).or_default().insert(var.clone());
                    self.semantic_db.add_token(
                        &fp,
                        SemanticToken {
                            name: var.clone(),
                            token_type: SemanticTokenType::Variable,
                            modifiers: vec![SemanticTokenModifier::Declaration],
                            span: stmt.span,
                        },
                    );
                    self.collect_expr_tokens(
                        &fp,
                        iterable,
                        0,
                        &mut declared,
                        &constructor_names,
                        &imported_module_roots,
                    );
                    for body_stmt in &body.stmts {
                        self.collect_stmt_tokens(
                            &fp,
                            body_stmt,
                            0,
                            &mut declared,
                            &constructor_names,
                            &imported_module_roots,
                        );
                    }
                    if let Some(ret_expr) = &body.expr {
                        self.collect_expr_tokens(
                            &fp,
                            ret_expr,
                            0,
                            &mut declared,
                            &constructor_names,
                            &imported_module_roots,
                        );
                    }
                }
                StmtKind::If { .. } | StmtKind::Error(_) | StmtKind::ExternalBindingStmt { .. } => {
                }
            }
        }

        // 更新全局作用域的符号列表
        if let Some(file_info) = self.semantic_db.get_file_info(&self.env.module_name) {
            if !file_info.scopes.is_empty() {
                // We need mutable access; use set_file_info approach or direct access
                // For simplicity, we recorded symbols inline already
            }
        }
    }

    /// 收集语句中的语义 tokens（递归）
    fn collect_stmt_tokens(
        &mut self,
        file_path: &str,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        scope_idx: usize,
        declared: &mut HashMap<usize, HashSet<String>>,
        constructor_names: &HashSet<String>,
        imported_module_roots: &HashSet<String>,
    ) {
        use crate::frontend::core::parser::ast::StmtKind;
        use semantic_db::SemanticTokenModifier;

        match &stmt.kind {
            StmtKind::Var {
                name,
                name_span,
                type_annotation,
                initializer,
                ..
            } => {
                let is_declaration = declared.entry(scope_idx).or_default().insert(name.clone());
                let modifiers = if is_declaration {
                    vec![SemanticTokenModifier::Declaration]
                } else {
                    vec![SemanticTokenModifier::Mutable]
                };
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::LocalVariable,
                        modifiers,
                        span: *name_span,
                    },
                );
                if let Some(ty) = type_annotation {
                    self.collect_type_tokens(file_path, ty);
                }
                if let Some(init) = initializer {
                    self.collect_expr_tokens(
                        file_path,
                        init,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            StmtKind::Expr(expr) => {
                self.collect_expr_tokens(
                    file_path,
                    expr,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
            StmtKind::For {
                var,
                iterable,
                body,
                ..
            } => {
                declared.entry(scope_idx).or_default().insert(var.clone());
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: var.clone(),
                        token_type: semantic_db::SemanticTokenType::Variable,
                        modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                        span: stmt.span,
                    },
                );
                self.collect_expr_tokens(
                    file_path,
                    iterable,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                for body_stmt in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        body_stmt,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
                if let Some(ret_expr) = &body.expr {
                    self.collect_expr_tokens(
                        file_path,
                        ret_expr,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            StmtKind::Fn {
                name,
                params,
                generic_params,
                type_annotation,
                body,
                ..
            } => {
                // 嵌套函数
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::Function,
                        modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                        span: stmt.span,
                    },
                );
                for param in params {
                    self.semantic_db.add_token(
                        file_path,
                        semantic_db::SemanticToken {
                            name: param.name.clone(),
                            token_type: semantic_db::SemanticTokenType::Parameter,
                            modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                            span: param.span,
                        },
                    );
                }
                for gp in generic_params {
                    for c in &gp.constraints {
                        self.collect_type_tokens(file_path, c);
                    }
                }
                if let Some(ty) = type_annotation {
                    self.collect_type_tokens(file_path, ty);
                }
                for body_stmt in &body.0 {
                    self.collect_stmt_tokens(
                        file_path,
                        body_stmt,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
                if let Some(ret_expr) = &body.1 {
                    self.collect_expr_tokens(
                        file_path,
                        ret_expr,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            _ => {}
        }
    }

    /// 收集表达式中的语义 tokens（递归）
    fn collect_expr_tokens(
        &mut self,
        file_path: &str,
        expr: &Expr,
        scope_idx: usize,
        declared: &mut HashMap<usize, HashSet<String>>,
        constructor_names: &HashSet<String>,
        imported_module_roots: &HashSet<String>,
    ) {
        use crate::frontend::core::parser::ast::Expr;

        match expr {
            Expr::Var(name, span) => {
                // 判断是函数引用还是变量引用
                let token_type = if imported_module_roots.contains(name) {
                    semantic_db::SemanticTokenType::Namespace
                } else if constructor_names.contains(name) {
                    semantic_db::SemanticTokenType::EnumMember
                } else if let Some(poly) = self.env.get_var(name) {
                    if matches!(poly.body, MonoType::Fn { .. }) {
                        semantic_db::SemanticTokenType::Function
                    } else {
                        semantic_db::SemanticTokenType::Variable
                    }
                } else {
                    semantic_db::SemanticTokenType::Variable
                };
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type,
                        modifiers: vec![],
                        span: *span,
                    },
                );
            }
            Expr::Call { func, args, .. } => {
                self.collect_call_target_tokens(
                    file_path,
                    func,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                    true,
                );
                for arg in args {
                    self.collect_expr_tokens(
                        file_path,
                        arg,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::FieldAccess {
                expr: inner,
                field,
                span,
            } => {
                let is_module_path = Self::is_module_path_expr(inner, imported_module_roots);
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: field.clone(),
                        token_type: if is_module_path {
                            semantic_db::SemanticTokenType::Namespace
                        } else {
                            semantic_db::SemanticTokenType::Property
                        },
                        modifiers: vec![],
                        span: *span,
                    },
                );
            }
            Expr::Cast {
                expr: inner,
                target_type,
                span: _,
            } => {
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                // Cast 目标类型 → Type (引用)
                self.collect_type_tokens(file_path, target_type);
            }
            Expr::BinOp { left, right, .. } => {
                self.collect_expr_tokens(
                    file_path,
                    left,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                self.collect_expr_tokens(
                    file_path,
                    right,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
            Expr::UnOp { expr: inner, .. } => {
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
            Expr::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.collect_expr_tokens(
                    file_path,
                    condition,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                for s in &then_branch.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
                if let Some(r) = &then_branch.expr {
                    self.collect_expr_tokens(
                        file_path,
                        r,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
                for (cond, block) in elif_branches {
                    self.collect_expr_tokens(
                        file_path,
                        cond,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                    for s in &block.stmts {
                        self.collect_stmt_tokens(
                            file_path,
                            s,
                            scope_idx,
                            declared,
                            constructor_names,
                            imported_module_roots,
                        );
                    }
                    if let Some(r) = &block.expr {
                        self.collect_expr_tokens(
                            file_path,
                            r,
                            scope_idx,
                            declared,
                            constructor_names,
                            imported_module_roots,
                        );
                    }
                }
                if let Some(block) = else_branch {
                    for s in &block.stmts {
                        self.collect_stmt_tokens(
                            file_path,
                            s,
                            scope_idx,
                            declared,
                            constructor_names,
                            imported_module_roots,
                        );
                    }
                    if let Some(r) = &block.expr {
                        self.collect_expr_tokens(
                            file_path,
                            r,
                            scope_idx,
                            declared,
                            constructor_names,
                            imported_module_roots,
                        );
                    }
                }
                // 后续还有其他类似的块...
            }
            Expr::While {
                condition, body, ..
            } => {
                self.collect_expr_tokens(
                    file_path,
                    condition,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
                if let Some(r) = &body.expr {
                    self.collect_expr_tokens(
                        file_path,
                        r,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::For {
                iterable,
                body,
                var,
                span,
                ..
            } => {
                declared.entry(scope_idx).or_default().insert(var.clone());
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: var.clone(),
                        token_type: semantic_db::SemanticTokenType::Variable,
                        modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                        span: *span,
                    },
                );
                self.collect_expr_tokens(
                    file_path,
                    iterable,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
                if let Some(r) = &body.expr {
                    self.collect_expr_tokens(
                        file_path,
                        r,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::Lambda { params, body, span } => {
                // Lambda 参数
                for param in params {
                    self.semantic_db.add_token(
                        file_path,
                        semantic_db::SemanticToken {
                            name: param.name.clone(),
                            token_type: semantic_db::SemanticTokenType::Parameter,
                            modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                            span: param.span,
                        },
                    );
                }
                // Lambda 作用域
                let lambda_scope_idx = self
                    .semantic_db
                    .get_scopes(file_path)
                    .map(|s| s.len())
                    .unwrap_or(0);
                declared.insert(
                    lambda_scope_idx,
                    params.iter().map(|p| p.name.clone()).collect(),
                );
                self.semantic_db.add_scope(
                    file_path,
                    semantic_db::ScopeInfo {
                        span: *span,
                        parent: None, // 简化：不追踪精确父级
                        symbols: params.iter().map(|p| p.name.clone()).collect(),
                        kind: semantic_db::ScopeKind::Lambda,
                    },
                );
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        lambda_scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
                if let Some(r) = &body.expr {
                    self.collect_expr_tokens(
                        file_path,
                        r,
                        lambda_scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::Block(block) => {
                for s in &block.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
                if let Some(r) = &block.expr {
                    self.collect_expr_tokens(
                        file_path,
                        r,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::Return(Some(inner), _) => {
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
            Expr::Match {
                expr: inner, arms, ..
            } => {
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                for arm in arms {
                    for s in &arm.body.stmts {
                        self.collect_stmt_tokens(
                            file_path,
                            s,
                            scope_idx,
                            declared,
                            constructor_names,
                            imported_module_roots,
                        );
                    }
                    if let Some(r) = &arm.body.expr {
                        self.collect_expr_tokens(
                            file_path,
                            r,
                            scope_idx,
                            declared,
                            constructor_names,
                            imported_module_roots,
                        );
                    }
                }
            }
            Expr::Tuple(elements, _) | Expr::List(elements, _) => {
                for elem in elements {
                    self.collect_expr_tokens(
                        file_path,
                        elem,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::Dict(pairs, _) => {
                for (k, v) in pairs {
                    self.collect_expr_tokens(
                        file_path,
                        k,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                    self.collect_expr_tokens(
                        file_path,
                        v,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::Index {
                expr: inner, index, ..
            } => {
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                self.collect_expr_tokens(
                    file_path,
                    index,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
            Expr::Try { expr: inner, .. } | Expr::Ref { expr: inner, .. } => {
                self.collect_expr_tokens(
                    file_path,
                    inner,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
            }
            Expr::FnDef {
                name,
                params,
                return_type,
                body,
                span,
                ..
            } => {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: name.clone(),
                        token_type: semantic_db::SemanticTokenType::Function,
                        modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                        span: *span,
                    },
                );
                for param in params {
                    self.semantic_db.add_token(
                        file_path,
                        semantic_db::SemanticToken {
                            name: param.name.clone(),
                            token_type: semantic_db::SemanticTokenType::Parameter,
                            modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                            span: param.span,
                        },
                    );
                    if let Some(t) = &param.ty {
                        self.collect_type_tokens(file_path, t);
                    }
                }
                if let Some(t) = return_type {
                    self.collect_type_tokens(file_path, t);
                }
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
                if let Some(r) = &body.expr {
                    self.collect_expr_tokens(
                        file_path,
                        r,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::ListComp {
                element,
                iterable,
                condition,
                var,
                span,
                ..
            } => {
                declared.entry(scope_idx).or_default().insert(var.clone());
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: var.clone(),
                        token_type: semantic_db::SemanticTokenType::Variable,
                        modifiers: vec![semantic_db::SemanticTokenModifier::Declaration],
                        span: *span,
                    },
                );
                self.collect_expr_tokens(
                    file_path,
                    element,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                self.collect_expr_tokens(
                    file_path,
                    iterable,
                    scope_idx,
                    declared,
                    constructor_names,
                    imported_module_roots,
                );
                if let Some(cond) = condition {
                    self.collect_expr_tokens(
                        file_path,
                        cond,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            Expr::FString { segments, .. } => {
                for seg in segments {
                    if let crate::frontend::core::parser::ast::FStringSegment::Interpolation {
                        expr,
                        ..
                    } = seg
                    {
                        self.collect_expr_tokens(
                            file_path,
                            expr,
                            scope_idx,
                            declared,
                            constructor_names,
                            imported_module_roots,
                        );
                    }
                }
            }
            Expr::Unsafe { body, .. } => {
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
                if let Some(r) = &body.expr {
                    self.collect_expr_tokens(
                        file_path,
                        r,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );
                }
            }
            // 字面量、Error、Break、Continue 等不需要收集
            _ => {}
        }
    }
}

/// 检查模块
#[allow(unused_variables)]
pub fn check_module(
    ast: &Module,
    env: &mut Option<TypeEnvironment>,
) -> Result<TypeCheckResult, Vec<Diagnostic>> {
    check_module_inner(ast, env, false)
}

/// 检查模块（收集所有错误模式）
///
/// 与 `check_module` 相同，但启用错误收集模式。
/// 类型检查器会尽可能多地收集错误，适用于 LSP 诊断。
#[allow(unused_variables)]
pub fn check_module_collect_all(
    ast: &Module,
    env: &mut Option<TypeEnvironment>,
) -> Result<TypeCheckResult, Vec<Diagnostic>> {
    check_module_inner(ast, env, true)
}

/// 模块检查内部实现
#[allow(unused_variables)]
fn check_module_inner(
    ast: &Module,
    env: &mut Option<TypeEnvironment>,
    collect_all: bool,
) -> Result<TypeCheckResult, Vec<Diagnostic>> {
    // 使用 TypeChecker 进行完整的模块检查
    let mut checker = TypeChecker::new("main");

    // 如果提供了外部环境，将其变量和类型导入到 checker 中
    if let Some(ref mut ext_env) = env {
        for (name, poly) in &ext_env.vars {
            checker.env().add_var(name.clone(), poly.clone());
        }
        for (name, poly) in &ext_env.types {
            checker.env().add_type(name.clone(), poly.clone());
        }
    }

    // 执行模块检查
    let result = if collect_all {
        checker.check_module_collect_all(ast)?
    } else {
        checker.check_module(ast)?
    };

    // 将 exports 和 method_bindings 导回传入的环境
    if let Some(ref mut ext_env) = env {
        ext_env.exports = checker.env.exports.clone();
        ext_env.method_bindings = checker.env.method_bindings.clone();
    }

    Ok(result)
}

/// 检查单个表达式
pub fn infer_expression(
    expr: &Expr,
    env: &mut TypeEnvironment,
) -> Result<MonoType, Vec<Diagnostic>> {
    // 创建共享 ScopeManager 并添加环境变量
    let mut scope = inference::ScopeManager::new();
    for (name, poly) in env.vars.clone() {
        scope.add_var(name, poly);
    }
    let overload_candidates_clone = env.overload_candidates.clone();
    let native_signatures_clone = env.native_signatures.clone();
    let mut inferrer = inference::ExpressionInferrer::with_native_signatures(
        &mut scope,
        env.solver(),
        &overload_candidates_clone,
        &native_signatures_clone,
    );
    inferrer.infer_expr(expr).map_err(|diag| vec![diag])
}

/// 添加内置类型到环境
pub fn add_builtin_types(env: &mut TypeEnvironment) {
    env.types
        .insert("int".to_string(), PolyType::mono(MonoType::Int(32)));
    env.types
        .insert("float".to_string(), PolyType::mono(MonoType::Float(64)));
    env.types
        .insert("bool".to_string(), PolyType::mono(MonoType::Bool));
    env.types
        .insert("string".to_string(), PolyType::mono(MonoType::String));
    env.types
        .insert("void".to_string(), PolyType::mono(MonoType::Void));
    env.types
        .insert("char".to_string(), PolyType::mono(MonoType::Char));
}

/// 注册标准库 native 函数类型签名到类型环境
///
/// 这些签名用于类型检查 `Native("...")` 表达式，确保调用签名匹配。
/// 通过 ModuleRegistry 自动发现所有 std 模块的 native 函数。
pub fn add_native_function_types(env: &mut TypeEnvironment) {
    use crate::frontend::module::registry::ModuleRegistry;
    use crate::frontend::module::ExportKind;

    let registry = ModuleRegistry::with_std();

    // 遍历所有 std 子模块，自动注册导出的函数
    for submodule_name in registry.std_submodule_names() {
        let module_path = format!("std.{}", submodule_name);
        if let Some(module) = registry.get(&module_path) {
            for export in module.exports.values() {
                // 使用签名字符串解析出正确的函数类型
                let fn_ty = match export.kind {
                    ExportKind::Function | ExportKind::Constant => {
                        parse_signature(&export.signature, env)
                    }
                    _ => continue,
                };

                // 注册完全限定名
                env.native_signatures
                    .insert(export.full_path.clone(), fn_ty.clone());

                // 注册短名称
                env.native_signatures.insert(export.name.clone(), fn_ty);
            }
        }
    }

    // 同时将这些 native 函数注册为变量，使其可在类型推断时查找
    for (name, sig) in &env.native_signatures.clone() {
        env.add_var(name.clone(), PolyType::mono(sig.clone()));
    }
}

/// 解析函数签名字符串为 MonoType
///
/// 格式: "[T](param1: Type1, param2: Type2) -> ReturnType"
/// 支持泛型前缀 [T]、函数类型参数 (item: T) -> T
/// 例如: "[T](list: List<T>, fn: (item: T) -> T) -> List<T>"
fn parse_signature(
    signature: &str,
    env: &mut TypeEnvironment,
) -> MonoType {
    let signature = signature.trim();

    // 解析可选的泛型参数前缀 [T] 或 [T, U]
    let (generic_params, rest) = parse_generic_prefix(signature);

    // 如果不以 ( 开头且没有泛型前缀，视为常量类型签名（如 "Float"）
    if !rest.starts_with('(') && generic_params.is_empty() {
        return parse_type_str_with_generics(rest, &generic_params);
    }

    // 检查泛型参数是否有重复
    {
        let mut seen = HashSet::new();
        for gp in &generic_params {
            if !seen.insert(gp.as_str()) {
                let diag = ErrorCodeDefinition::invalid_signature_duplicate_param(gp).build();
                eprintln!("[Error] {}: {}", diag.code, diag.message);
                return MonoType::Fn {
                    params: vec![env.solver().new_var()],
                    return_type: Box::new(MonoType::Void),
                    is_async: false,
                };
            }
        }
    }

    // 验证括号：必须以 ( 开头
    if !rest.starts_with('(') {
        let diag = ErrorCodeDefinition::invalid_signature("must start with '('").build();
        eprintln!("[Error] {}: {}", diag.code, diag.message);
        return MonoType::Fn {
            params: vec![env.solver().new_var()],
            return_type: Box::new(MonoType::Void),
            is_async: false,
        };
    }

    // 找到与首个 ( 匹配的 )
    let closing_paren = find_matching_close(rest, 0);
    let Some(closing_paren) = closing_paren else {
        let diag = ErrorCodeDefinition::invalid_signature("unmatched '('").build();
        eprintln!("[Error] {}: {}", diag.code, diag.message);
        return MonoType::Fn {
            params: vec![env.solver().new_var()],
            return_type: Box::new(MonoType::Void),
            is_async: false,
        };
    };

    let params_str = &rest[1..closing_paren];
    let after_params = rest[closing_paren + 1..].trim();

    // 验证签名格式：匹配的 ) 之后必须有 ->
    if !after_params.starts_with("->") {
        let diag = ErrorCodeDefinition::invalid_signature_missing_arrow().build();
        eprintln!("[Error] {}: {}", diag.code, diag.message);
        return MonoType::Fn {
            params: vec![env.solver().new_var()],
            return_type: Box::new(MonoType::Void),
            is_async: false,
        };
    }

    let return_str = after_params[2..].trim();

    // 解析参数（并验证参数名）
    let (params, param_names) = parse_params_with_names(params_str, &generic_params);

    // 检查参数名是否重复
    {
        let mut seen = HashSet::new();
        for name in &param_names {
            if !name.is_empty() && !seen.insert(name.as_str()) {
                let diag = ErrorCodeDefinition::invalid_signature_duplicate_param(name).build();
                eprintln!("[Error] {}: {}", diag.code, diag.message);
                return MonoType::Fn {
                    params: vec![env.solver().new_var()],
                    return_type: Box::new(MonoType::Void),
                    is_async: false,
                };
            }
        }
    }

    // 检查参数名是否与泛型参数同名
    for name in &param_names {
        if !name.is_empty() && generic_params.contains(name) {
            let diag = ErrorCodeDefinition::invalid_signature_param_shadows_generic(name).build();
            eprintln!("[Error] {}: {}", diag.code, diag.message);
            return MonoType::Fn {
                params: vec![env.solver().new_var()],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            };
        }
    }

    // 解析返回类型
    let return_type = Box::new(parse_type_str_with_generics(return_str, &generic_params));

    MonoType::Fn {
        params,
        return_type,
        is_async: false,
    }
}

/// 解析泛型参数前缀 [T] 或 [T, U]
/// 返回 (泛型参数列表, 剩余字符串)
fn parse_generic_prefix(s: &str) -> (Vec<String>, &str) {
    let s = s.trim();
    if s.starts_with('[') {
        if let Some(close) = s.find(']') {
            let inner = &s[1..close];
            let params: Vec<String> = inner
                .split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();
            return (params, s[close + 1..].trim());
        }
    }
    (Vec::new(), s)
}

/// 找到从 pos 开始的 ( 对应的匹配 )，正确处理嵌套
fn find_matching_close(
    s: &str,
    pos: usize,
) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.get(pos) != Some(&b'(') {
        return None;
    }
    let mut depth: i32 = 0;
    for (i, &byte) in bytes.iter().enumerate().skip(pos) {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// 解析参数字符串，返回类型列表和参数名列表
fn parse_params_with_names(
    params_str: &str,
    generic_params: &[String],
) -> (Vec<MonoType>, Vec<String>) {
    if params_str.trim().is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut params = Vec::new();
    let mut names = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0;

    for (i, c) in params_str.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let param = params_str[start..i].trim();
                if !param.is_empty() {
                    let (ty, name) = parse_param_with_name(param, generic_params);
                    params.push(ty);
                    names.push(name);
                }
                start = i + 1;
            }
            _ => {}
        }
    }

    // 最后一个参数
    let param = params_str[start..].trim();
    if !param.is_empty() {
        let (ty, name) = parse_param_with_name(param, generic_params);
        params.push(ty);
        names.push(name);
    }

    (params, names)
}

/// 解析单个参数，返回 (类型, 参数名)
/// 支持 "name: Type" 格式和函数类型 "name: (item: T) -> T"
fn parse_param_with_name(
    param: &str,
    generic_params: &[String],
) -> (MonoType, String) {
    let param = param.trim();

    // 找到顶层的冒号（在括号/尖括号外面的第一个冒号）
    let mut depth: i32 = 0;
    let mut colon_pos = None;
    for (i, c) in param.char_indices() {
        match c {
            '(' | '<' | '[' => depth += 1,
            ')' | '>' | ']' => depth = depth.saturating_sub(1),
            ':' if depth == 0 => {
                colon_pos = Some(i);
                break;
            }
            _ => {}
        }
    }

    if let Some(pos) = colon_pos {
        let name = param[..pos].trim().to_string();
        let type_str = param[pos + 1..].trim();
        let ty = parse_type_str_with_generics(type_str, generic_params);
        (ty, name)
    } else {
        let ty = parse_type_str_with_generics(param, generic_params);
        (ty, String::new())
    }
}

/// 解析类型字符串为 MonoType，支持泛型参数引用和函数类型
fn parse_type_str_with_generics(
    type_str: &str,
    generic_params: &[String],
) -> MonoType {
    let type_str = type_str.trim();

    // 处理函数类型: (item: T) -> T 或元组类型: (String, Int)
    if type_str.starts_with('(') {
        // 找到匹配的 )
        if let Some(close) = find_matching_close(type_str, 0) {
            let after = type_str[close + 1..].trim();
            if let Some(after_arrow) = after.strip_prefix("->") {
                // 这是函数类型: (params) -> ReturnType
                let params_part = &type_str[1..close];
                let return_part = after_arrow.trim();

                let (fn_params, _fn_param_names) =
                    parse_params_with_names(params_part, generic_params);
                let fn_return = parse_type_str_with_generics(return_part, generic_params);

                return MonoType::Fn {
                    params: fn_params,
                    return_type: Box::new(fn_return),
                    is_async: false,
                };
            } else if after.is_empty() {
                // 没有 ->，是元组类型: (String, Int)
                let inner = &type_str[1..close];
                let elements = split_by_top_level_comma(inner);
                let tuple_types: Vec<MonoType> = elements
                    .iter()
                    .map(|s| parse_type_str_with_generics(s, generic_params))
                    .collect();
                return MonoType::Tuple(tuple_types);
            }
        }
    }

    // 处理泛型类型: List<T>, Dict<String, Int>
    if let Some(angle_bracket) = type_str.find('<') {
        let base = &type_str[..angle_bracket];
        let inner_start = angle_bracket + 1;
        let inner_end = type_str.len() - 1;

        if inner_end > inner_start && type_str.ends_with('>') {
            let inner = &type_str[inner_start..inner_end];

            match base {
                "List" => {
                    let inner_types = split_by_top_level_comma(inner);
                    if inner_types.len() == 1 {
                        let inner_type =
                            Box::new(parse_type_str_with_generics(inner_types[0], generic_params));
                        return MonoType::List(inner_type);
                    }
                }
                "Dict" => {
                    let parts: Vec<&str> = split_by_top_level_comma(inner);
                    if parts.len() == 2 {
                        let k = Box::new(parse_type_str_with_generics(parts[0], generic_params));
                        let v = Box::new(parse_type_str_with_generics(parts[1], generic_params));
                        return MonoType::Dict(k, v);
                    }
                }
                "Set" => {
                    let inner_types = split_by_top_level_comma(inner);
                    if inner_types.len() == 1 {
                        let inner_type =
                            Box::new(parse_type_str_with_generics(inner_types[0], generic_params));
                        return MonoType::Set(inner_type);
                    }
                }
                _ => {}
            }
        }
    }

    // 检查是否是泛型参数引用
    if generic_params.iter().any(|gp| gp == type_str) {
        // 泛型参数 → 使用 TypeRef 表示（类型检查时将其视为 Any）
        return MonoType::TypeRef(type_str.to_string());
    }

    // 基本类型
    match type_str {
        "Void" | "void" => MonoType::Void,
        "Bool" | "bool" => MonoType::Bool,
        "Int" | "int" => MonoType::Int(32),
        "Float" | "float" => MonoType::Float(64),
        "Char" | "char" => MonoType::Char,
        "String" | "string" => MonoType::String,
        "Bytes" | "bytes" => MonoType::Bytes,
        "Any" => MonoType::TypeRef("Any".to_string()),
        _ => {
            // 未知类型 → 创建 TypeRef（可能是自定义类型）
            MonoType::TypeRef(type_str.to_string())
        }
    }
}

/// 按顶层逗号分割字符串，正确处理嵌套的 < > ( )
fn split_by_top_level_comma(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0;

    for (i, c) in s.char_indices() {
        match c {
            '<' | '(' => depth += 1,
            '>' | ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let part = s[start..i].trim();
                if !part.is_empty() {
                    result.push(part);
                }
                start = i + 1;
            }
            _ => {}
        }
    }

    // 最后一个元素
    let part = s[start..].trim();
    if !part.is_empty() {
        result.push(part);
    }

    result
}

/// 添加标准库 traits 到环境
pub fn add_std_traits(env: &mut TypeEnvironment) {
    // 初始化标准库 trait 定义
    super::type_level::std_traits::init_std_traits(&mut env.trait_table);

    // 初始化 primitive 类型的 trait 实现
    super::type_level::std_traits::init_primitive_impls(&mut env.trait_table);
}

/// 类型检查结果
#[derive(Debug, Clone, Default)]
pub struct TypeCheckResult {
    pub module_name: String,
    pub bindings: HashMap<String, PolyType>,
    /// 局部变量的类型信息（用于 IR 生成器显示错误消息）
    /// Key 是变量名，Value 是推断出的具体类型
    pub local_var_types: HashMap<String, MonoType>,
    /// 语义信息数据库（typecheck 阶段产出）
    pub semantic_db: semantic_db::SemanticDB,
}

/// 导入信息
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// 导入路径（如 "std.io"）
    pub path: String,
    /// 导入的具体项（如 ["print", "println"]），None 表示全部
    pub items: Option<Vec<String>>,
    /// 模块别名
    pub alias: Option<String>,
}
