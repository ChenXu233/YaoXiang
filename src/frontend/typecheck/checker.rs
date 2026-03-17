//! 类型检查器模块
//!
//! 包含 TypeChecker 的完整实现

use std::collections::{HashMap, HashSet};

use crate::frontend::core::parser::ast::Module;
use crate::frontend::core::type_system::{MonoType, PolyType};
use crate::frontend::type_level::auto_derive;

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

    fn semantic_token_type_for_export(
        export: &crate::frontend::module::Export
    ) -> semantic_db::SemanticTokenType {
        match export.kind {
            crate::frontend::module::ExportKind::Function => {
                semantic_db::SemanticTokenType::Function
            }
            crate::frontend::module::ExportKind::SubModule => {
                semantic_db::SemanticTokenType::Namespace
            }
            crate::frontend::module::ExportKind::Type => semantic_db::SemanticTokenType::Type,
            crate::frontend::module::ExportKind::Constant => {
                semantic_db::SemanticTokenType::Variable
            }
        }
    }

    fn collect_use_stmt_tokens(
        &mut self,
        file_path: &str,
        path: &str,
        path_parts: &[crate::frontend::core::parser::ast::SpannedIdent],
        items: &Option<Vec<String>>,
        alias: &Option<Vec<String>>,
    ) {
        // use path.{...} 的 path 部分始终是模块命名空间
        if items.is_some() {
            for part in path_parts {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: part.name.clone(),
                        token_type: semantic_db::SemanticTokenType::Namespace,
                        modifiers: vec![],
                        span: part.span,
                    },
                );
            }
            return;
        }

        // use path as alias：path 是命名空间，alias 是目标符号名（由别名决定）
        if alias.is_some() {
            for part in path_parts {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: part.name.clone(),
                        token_type: semantic_db::SemanticTokenType::Namespace,
                        modifiers: vec![],
                        span: part.span,
                    },
                );
            }
            return;
        }

        if self.env.module_registry.has_module(path) {
            for part in path_parts {
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: part.name.clone(),
                        token_type: semantic_db::SemanticTokenType::Namespace,
                        modifiers: vec![],
                        span: part.span,
                    },
                );
            }
            return;
        }

        if let Ok(export) = self.env.module_registry.resolve_export(path) {
            for (idx, part) in path_parts.iter().enumerate() {
                let token_type = if idx + 1 == path_parts.len() {
                    Self::semantic_token_type_for_export(export)
                } else {
                    semantic_db::SemanticTokenType::Namespace
                };
                self.semantic_db.add_token(
                    file_path,
                    semantic_db::SemanticToken {
                        name: part.name.clone(),
                        token_type,
                        modifiers: vec![],
                        span: part.span,
                    },
                );
            }
            return;
        }

        // fallback
        for part in path_parts {
            self.semantic_db.add_token(
                file_path,
                semantic_db::SemanticToken {
                    name: part.name.clone(),
                    token_type: semantic_db::SemanticTokenType::Namespace,
                    modifiers: vec![],
                    span: part.span,
                },
            );
        }
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
        imported_module_roots: &mut HashSet<String>,
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
        let mut imported_module_roots = HashSet::new();
        imported_module_roots.insert("std".to_string());

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
                    let mut fn_roots = imported_module_roots.clone();
                    for body_stmt in &body.0 {
                        self.collect_stmt_tokens(
                            &fp,
                            body_stmt,
                            scope_idx,
                            &mut declared,
                            &constructor_names,
                            &mut fn_roots,
                        );
                    }
                    if let Some(ret_expr) = &body.1 {
                        self.collect_expr_tokens(
                            &fp,
                            ret_expr,
                            scope_idx,
                            &mut declared,
                            &constructor_names,
                            &mut fn_roots,
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
                            &mut imported_module_roots,
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
                StmtKind::Use {
                    path,
                    path_parts,
                    items,
                    alias,
                    ..
                } => {
                    self.collect_use_stmt_tokens(&fp, path, path_parts, items, alias);
                    self.add_use_module_root(&mut imported_module_roots, path, items, alias);
                }
                StmtKind::Expr(expr) => {
                    self.collect_expr_tokens(
                        &fp,
                        expr,
                        0,
                        &mut declared,
                        &constructor_names,
                        &mut imported_module_roots,
                    );
                }
                StmtKind::For {
                    var,
                    var_span,
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
                            span: *var_span,
                        },
                    );
                    self.collect_expr_tokens(
                        &fp,
                        iterable,
                        0,
                        &mut declared,
                        &constructor_names,
                        &mut imported_module_roots,
                    );
                    for body_stmt in &body.stmts {
                        self.collect_stmt_tokens(
                            &fp,
                            body_stmt,
                            0,
                            &mut declared,
                            &constructor_names,
                            &mut imported_module_roots,
                        );
                    }
                    if let Some(ret_expr) = &body.expr {
                        self.collect_expr_tokens(
                            &fp,
                            ret_expr,
                            0,
                            &mut declared,
                            &constructor_names,
                            &mut imported_module_roots,
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
        imported_module_roots: &mut HashSet<String>,
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
            StmtKind::Use {
                path,
                path_parts,
                items,
                alias,
                ..
            } => {
                self.collect_use_stmt_tokens(file_path, path, path_parts, items, alias);
                self.add_use_module_root(imported_module_roots, path, items, alias);
            }
            StmtKind::For {
                var,
                var_span,
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
                        span: *var_span,
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
            StmtKind::If {
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

                let mut then_roots = imported_module_roots.clone();
                for s in &then_branch.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut then_roots,
                    );
                }
                if let Some(r) = &then_branch.expr {
                    self.collect_expr_tokens(
                        file_path,
                        r,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut then_roots,
                    );
                }

                for (elif_cond, elif_block) in elif_branches {
                    self.collect_expr_tokens(
                        file_path,
                        elif_cond,
                        scope_idx,
                        declared,
                        constructor_names,
                        imported_module_roots,
                    );

                    let mut elif_roots = imported_module_roots.clone();
                    for s in &elif_block.stmts {
                        self.collect_stmt_tokens(
                            file_path,
                            s,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut elif_roots,
                        );
                    }
                    if let Some(r) = &elif_block.expr {
                        self.collect_expr_tokens(
                            file_path,
                            r,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut elif_roots,
                        );
                    }
                }

                if let Some(else_block) = else_branch {
                    let mut else_roots = imported_module_roots.clone();
                    for s in &else_block.stmts {
                        self.collect_stmt_tokens(
                            file_path,
                            s,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut else_roots,
                        );
                    }
                    if let Some(r) = &else_block.expr {
                        self.collect_expr_tokens(
                            file_path,
                            r,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut else_roots,
                        );
                    }
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
                let mut fn_roots = imported_module_roots.clone();
                for body_stmt in &body.0 {
                    self.collect_stmt_tokens(
                        file_path,
                        body_stmt,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut fn_roots,
                    );
                }
                if let Some(ret_expr) = &body.1 {
                    self.collect_expr_tokens(
                        file_path,
                        ret_expr,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut fn_roots,
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
        imported_module_roots: &mut HashSet<String>,
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
                let mut then_roots = imported_module_roots.clone();
                for s in &then_branch.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut then_roots,
                    );
                }
                if let Some(r) = &then_branch.expr {
                    self.collect_expr_tokens(
                        file_path,
                        r,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut then_roots,
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
                    let mut elif_roots = imported_module_roots.clone();
                    for s in &block.stmts {
                        self.collect_stmt_tokens(
                            file_path,
                            s,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut elif_roots,
                        );
                    }
                    if let Some(r) = &block.expr {
                        self.collect_expr_tokens(
                            file_path,
                            r,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut elif_roots,
                        );
                    }
                }
                if let Some(block) = else_branch {
                    let mut else_roots = imported_module_roots.clone();
                    for s in &block.stmts {
                        self.collect_stmt_tokens(
                            file_path,
                            s,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut else_roots,
                        );
                    }
                    if let Some(r) = &block.expr {
                        self.collect_expr_tokens(
                            file_path,
                            r,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut else_roots,
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
                let mut while_roots = imported_module_roots.clone();
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut while_roots,
                    );
                }
                if let Some(r) = &body.expr {
                    self.collect_expr_tokens(
                        file_path,
                        r,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut while_roots,
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
                let mut for_roots = imported_module_roots.clone();
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut for_roots,
                    );
                }
                if let Some(r) = &body.expr {
                    self.collect_expr_tokens(
                        file_path,
                        r,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut for_roots,
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
                let mut lambda_roots = imported_module_roots.clone();
                for s in &body.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        lambda_scope_idx,
                        declared,
                        constructor_names,
                        &mut lambda_roots,
                    );
                }
                if let Some(r) = &body.expr {
                    self.collect_expr_tokens(
                        file_path,
                        r,
                        lambda_scope_idx,
                        declared,
                        constructor_names,
                        &mut lambda_roots,
                    );
                }
            }
            Expr::Block(block) => {
                let mut block_roots = imported_module_roots.clone();
                for s in &block.stmts {
                    self.collect_stmt_tokens(
                        file_path,
                        s,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut block_roots,
                    );
                }
                if let Some(r) = &block.expr {
                    self.collect_expr_tokens(
                        file_path,
                        r,
                        scope_idx,
                        declared,
                        constructor_names,
                        &mut block_roots,
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
                    let mut arm_roots = imported_module_roots.clone();
                    for s in &arm.body.stmts {
                        self.collect_stmt_tokens(
                            file_path,
                            s,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut arm_roots,
                        );
                    }
                    if let Some(r) = &arm.body.expr {
                        self.collect_expr_tokens(
                            file_path,
                            r,
                            scope_idx,
                            declared,
                            constructor_names,
                            &mut arm_roots,
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
