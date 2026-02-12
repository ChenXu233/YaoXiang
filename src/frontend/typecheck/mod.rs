//! 类型检查器模块
//!
//! 实现 YaoXiang 语言的类型检查器，支持：
//! - Hindley-Milner 类型推断
//! - 泛型函数和泛型类型
//! - 完整的错误收集
//! - RFC-004/010/011 支撑

use std::collections::{HashMap, HashSet};

use crate::frontend::core::parser::ast::{Module, Expr};

// 导入推断模块
pub mod inference;

// 导入检查模块
pub mod checking;

// 导入特化模块
pub mod specialization;

// 导入旧特化模块（包含 GenericSpecializer）
mod specialize;

// 导入特质模块
pub mod traits;

// 导入 GAT 模块
pub mod gat;

// 导入重载解析模块（在 TypeEnvironment 之前声明）
pub mod overload;

// 导入类型求值器模块
pub mod type_eval;

// 导入测试模块
#[cfg(test)]
mod tests;

// 使用 core 层的类型系统（显式导出以避免 ambiguous glob re-exports）
pub use crate::frontend::core::type_system::{
    MonoType, PolyType, TypeVar, TypeBinding, StructType, EnumType, TypeConstraint,
    TypeConstraintSolver, SendSyncConstraint, SendSyncSolver, TypeMismatch, TypeConstraintError,
    ConstValue, ConstExpr, ConstKind, ConstVarDef,
};

// 重新导出推断、检查、特化等模块
pub use inference::*;
pub use checking::*;
pub use specialization::*;
pub use specialize::*;
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
}

/// 类型检查器
///
/// 负责模块级类型检查编排，协调前置收集和函数体检查
pub struct TypeChecker {
    /// 当前环境
    env: TypeEnvironment,
    /// 已检查的函数签名（用于递归检测）
    checked_functions: HashMap<String, bool>,
    /// 当前函数的返回类型
    current_return_type: Option<MonoType>,
    /// 泛型函数缓存
    generic_cache: HashMap<String, HashMap<String, PolyType>>,
    /// 函数体检查器
    body_checker: Option<checking::BodyChecker>,
}

impl TypeChecker {
    /// 创建新的类型检查器
    pub fn new(module_name: &str) -> Self {
        let mut env = TypeEnvironment::new_with_module(module_name.to_string());
        add_builtin_types(&mut env);
        add_std_traits(&mut env);

        Self {
            env,
            checked_functions: HashMap::new(),
            current_return_type: None,
            generic_cache: HashMap::new(),
            body_checker: None,
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

    /// 检查单个语句（委托给 BodyChecker）
    pub fn check_stmt(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) -> Result<(), Diagnostic> {
        self.body_checker_mut().check_stmt(stmt)
    }

    /// 获取函数体检查器
    fn body_checker(&mut self) -> &mut checking::BodyChecker {
        if self.body_checker.is_none() {
            self.body_checker = Some(checking::BodyChecker::new(self.env.solver()));
        }
        self.body_checker.as_mut().unwrap()
    }

    /// 检查整个模块
    pub fn check_module(
        &mut self,
        module: &Module,
    ) -> Result<TypeCheckResult, Vec<Diagnostic>> {
        // 第一遍：收集所有类型定义
        for stmt in &module.items {
            if let crate::frontend::core::parser::ast::StmtKind::TypeDef { name, definition } =
                &stmt.kind
            {
                self.add_type_definition(name, definition, stmt.span);
            }
        }

        // 第二遍：收集所有函数签名（使其可被前向引用）
        for stmt in &module.items {
            self.collect_function_signature(stmt);
        }

        // 收集所有导出项
        self.collect_exports(module);

        // 初始化函数体检查器
        let body_checker = checking::BodyChecker::new(self.env.solver());
        *self.body_checker_mut() = body_checker;

        // 将环境中的变量同步到 body_checker
        for (name, poly) in self.env.vars.clone() {
            self.body_checker_mut().add_var(name, poly);
        }

        // 第三遍：检查所有语句（包括函数体）
        for stmt in &module.items {
            if let Err(e) = self.body_checker_mut().check_stmt(stmt) {
                self.add_error(e);
            }
        }

        // 求解所有约束
        self.env.solver().solve().map_err(|e| {
            e.into_iter()
                .map(|e| {
                    ErrorCodeDefinition::type_mismatch(
                        &format!("{}", e.error.left),
                        &format!("{}", e.error.right),
                    ).at(e.span).build(I18nRegistry::en())
                })
                .collect::<Vec<_>>()
        })?;

        // 如果有错误，返回所有错误
        if self.has_errors() {
            return Err(self.errors().to_vec());
        }

        // 构建类型检查结果
        let result = TypeCheckResult {
            module_name: self.env.module_name.clone(),
            bindings: self.env.vars.clone(),
        };

        Ok(result)
    }

    /// 获取 body_checker 的可变引用
    fn body_checker_mut(&mut self) -> &mut checking::BodyChecker {
        if self.body_checker.is_none() {
            self.body_checker = Some(checking::BodyChecker::new(self.env.solver()));
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
                path,
                items: _,
                alias: _,
            } => {
                // 处理 use std.io - 添加 print/println 函数
                if path == "std.io" {
                    let print_ty = MonoType::Fn {
                        params: vec![self.env.solver().new_var()],
                        return_type: Box::new(MonoType::Void),
                        is_async: false,
                    };
                    self.env
                        .add_var("print".to_string(), PolyType::mono(print_ty.clone()));
                    self.env
                        .add_var("println".to_string(), PolyType::mono(print_ty));
                }
            }
            _ => {}
        }
    }

    /// 添加类型定义
    fn add_type_definition(
        &mut self,
        name: &str,
        definition: &crate::frontend::core::parser::ast::Type,
        _span: crate::util::span::Span,
    ) {
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
            crate::frontend::core::parser::ast::Type::Struct(fields) => fields,
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
}

/// 检查模块
#[allow(unused_variables)]
pub fn check_module(
    ast: &Module,
    env: &mut Option<TypeEnvironment>,
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
    let result = checker.check_module(ast)?;

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
    // 克隆环境变量，避免借用冲突
    let vars_clone = env.vars.clone();
    let overload_candidates_clone = env.overload_candidates.clone();
    let mut inferrer = crate::frontend::typecheck::inference::ExprInferrer::new(
        env.solver(),
        &overload_candidates_clone,
    );
    // 添加环境中的变量到推断器
    for (name, poly) in vars_clone {
        inferrer.add_var(name, poly);
    }
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
