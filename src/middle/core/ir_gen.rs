//! AST 到 IR 的代码生成器
//!
//! 将抽象语法树（AST）转换为中间表示（IR）。
//! 这是编译流程的第二步：解析 → 类型检查 → IR 生成 → 代码生成
//!
//! ## 设计原则
//!
//! 1. 单一职责：只负责 AST → IR 转换，不关心类型检查或代码生成
//! 2. 简洁直接：IR 结构简单，生成逻辑清晰
//! 3. 可测试性：独立的模块便于单元测试

use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::parser::ast::{self, Expr};
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::typecheck::{MonoType, PolyType, TypeCheckResult};
use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, ModuleIR, Operand};
use crate::tlog;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::i18n::MSG;
use crate::util::span::Span;
use std::collections::HashMap;
use std::sync::LazyLock;

/// 缓存所有 native 函数/常量名（通过 ModuleRegistry 自动发现）
static NATIVE_NAMES: LazyLock<Vec<String>> =
    LazyLock::new(|| ModuleRegistry::with_std().native_names());

/// 缓存短名称到完整名称的映射（通过 ModuleRegistry 自动发现）
/// 例如：print -> std.io.print, abs -> std.math.abs
static SHORT_TO_QUALIFIED: LazyLock<HashMap<String, String>> =
    LazyLock::new(|| ModuleRegistry::with_std().short_to_qualified_map());

/// 缓存 std 子模块名称列表（通过 ModuleRegistry 自动发现）
static STD_SUBMODULES: LazyLock<Vec<String>> =
    LazyLock::new(|| ModuleRegistry::with_std().std_submodule_names());

/// 检查是否是命名空间调用（如 std.io.println 或 io.println）
fn is_namespace_call(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::Var(name, _) => name == "std" || is_std_module(name),
        ast::Expr::FieldAccess { expr, .. } => is_namespace_call(expr),
        _ => false,
    }
}

/// 提取完整的命名空间路径（如 std.io.println 或 io.println -> std.io.println）
fn extract_namespace_path(
    expr: &ast::Expr,
    field: &str,
) -> String {
    match expr {
        ast::Expr::Var(name, _) => {
            if name == "std" {
                format!("std.{}", field)
            } else if is_std_module(name) {
                format!("std.{}.{}", name, field)
            } else {
                format!("{}.{}", name, field)
            }
        }
        ast::Expr::FieldAccess {
            expr,
            field: sub_field,
            ..
        } => {
            let prefix = extract_namespace_path(expr, sub_field);
            format!("{}.{}", prefix, field)
        }
        _ => field.to_string(),
    }
}

/// 检查完整的命名空间路径是否是 native 函数/常量
fn is_native_name(full_path: &str) -> bool {
    NATIVE_NAMES.iter().any(|n| n == full_path)
}

/// 检查变量名是否是 std 模块的子模块
/// 通过 ModuleRegistry 动态查询，不再硬编码模块名称
fn is_std_module(name: &str) -> bool {
    STD_SUBMODULES.iter().any(|m| m == name)
}

/// 将模块变量和字段组合成完整路径（如 io.println -> std.io.println）
fn resolve_module_access(
    module_name: &str,
    field: &str,
) -> Option<String> {
    if is_std_module(module_name) {
        let full_path = format!("std.{}", field);
        if is_native_name(&full_path) {
            return Some(full_path);
        }
    }
    None
}

/// 符号表条目
#[derive(Debug, Clone)]
struct SymbolEntry {
    local_idx: usize,
}

/// IR 生成器配置
#[derive(Debug, Default, Clone)]
pub struct IrGeneratorConfig {
    /// 是否生成调试信息
    pub generate_debug_info: bool,
}

impl IrGeneratorConfig {
    /// 创建默认配置
    pub fn new() -> Self {
        Self::default()
    }
}

/// AST 到 IR 的生成器
///
/// 将 AST 节点转换为 IR 指令序列。
#[derive(Debug)]
pub struct AstToIrGenerator {
    /// 符号表（用于变量解析）
    symbols: Vec<HashMap<String, SymbolEntry>>,
    /// 类型检查结果（包含变量绑定信息）
    type_result: Option<Box<TypeCheckResult>>,
    /// 下一个临时寄存器编号
    next_temp: usize,
    /// 当前函数中可变局部变量的索引集合
    current_mut_locals: std::collections::HashSet<usize>,
    /// 模块级别的可变局部变量映射 (function_name -> set of mutable local indices)
    module_mut_locals: HashMap<String, std::collections::HashSet<usize>>,
    /// 当前函数中循环绑定变量的索引集合（这些变量的 Store 是绑定操作，不是修改）
    current_loop_binding_locals: std::collections::HashSet<usize>,
    /// 模块级别的循环绑定变量映射 (function_name -> set of loop binding local indices)
    module_loop_binding_locals: HashMap<String, std::collections::HashSet<usize>>,
    /// 当前函数的局部变量名列表（按索引顺序）
    current_local_names: Vec<String>,
    /// 模块级别的局部变量名映射 (function_name -> 变量名列表)
    module_local_names: HashMap<String, Vec<String>>,
    /// 局部变量类型追踪（用于错误消息中显示实际类型）
    local_var_types: HashMap<String, String>,
    /// 用户声明的 native 函数绑定
    native_bindings: Vec<crate::std::ffi::NativeBinding>,
    /// 结构体定义映射（类型名 -> 字段列表）
    /// 用于构造器调用时填充默认值
    struct_definitions: HashMap<String, Vec<crate::frontend::core::parser::ast::StructField>>,
    /// 类型绑定映射（类型名 -> (方法名 -> BindingInfo)）
    /// 用于方法调用时的参数重排和函数转发（RFC-004）
    type_bindings: HashMap<String, HashMap<String, BindingInfo>>,
    /// 嵌套函数列表（在函数体内定义的函数）
    nested_functions: Vec<FunctionIR>,
    /// 闭包计数器（用于生成唯一的闭包名称）
    closure_counter: usize,
    /// 全局变量表 (name, type, initial_value)
    global_vars: Vec<(String, MonoType, Option<ConstValue>)>,
}

/// 绑定信息（用于 IR 生成阶段的方法调用转发）
///
/// 按 RFC-004 设计：记录方法绑定到哪个原始函数的哪些参数位置
#[derive(Debug, Clone)]
struct BindingInfo {
    /// 原始函数名
    function: String,
    /// 绑定位置列表（调用者 obj 填充到这些位置）
    positions: Vec<usize>,
}

/// Lambda 函数体 IR 结果
struct LambdaBodyIR {
    instructions: Vec<Instruction>,
    locals: Vec<MonoType>,
    /// 闭包函数的可变局部变量索引集合
    mut_locals: std::collections::HashSet<usize>,
}

impl Default for AstToIrGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl AstToIrGenerator {
    /// 创建新的 IR 生成器
    pub fn new() -> Self {
        Self {
            symbols: vec![HashMap::new()], // 全局作用域
            type_result: None,
            next_temp: 0,
            current_mut_locals: std::collections::HashSet::new(),
            module_mut_locals: HashMap::new(),
            current_loop_binding_locals: std::collections::HashSet::new(),
            module_loop_binding_locals: HashMap::new(),
            current_local_names: Vec::new(),
            module_local_names: HashMap::new(),
            local_var_types: HashMap::new(),
            native_bindings: Vec::new(),
            struct_definitions: HashMap::new(),
            type_bindings: HashMap::new(),
            nested_functions: Vec::new(),
            closure_counter: 0,
            global_vars: Vec::new(),
        }
    }

    /// 创建新的 IR 生成器（带类型信息）
    pub fn new_with_type_result(type_result: &TypeCheckResult) -> Self {
        Self {
            symbols: vec![HashMap::new()], // 全局作用域
            type_result: Some(Box::new(type_result.clone())),
            next_temp: 0,
            current_mut_locals: std::collections::HashSet::new(),
            module_mut_locals: HashMap::new(),
            current_loop_binding_locals: std::collections::HashSet::new(),
            module_loop_binding_locals: HashMap::new(),
            current_local_names: Vec::new(),
            module_local_names: HashMap::new(),
            local_var_types: HashMap::new(),
            native_bindings: Vec::new(),
            struct_definitions: HashMap::new(),
            type_bindings: HashMap::new(),
            nested_functions: Vec::new(),
            closure_counter: 0,
            global_vars: Vec::new(),
        }
    }

    /// 进入新的作用域
    fn enter_scope(&mut self) {
        tlog!(debug, MSG::IrGenEnterScope, &self.symbols.len().to_string());
        self.symbols.push(HashMap::new());
        tlog!(debug, MSG::IrGenEnterScope, &self.symbols.len().to_string());
    }

    /// 退出当前作用域
    fn exit_scope(&mut self) {
        tlog!(debug, MSG::IrGenExitScope, &self.symbols.len().to_string());
        self.symbols.pop();
        tlog!(debug, MSG::IrGenExitScope, &self.symbols.len().to_string());
    }

    /// 阶段3修复：实例化多态类型
    fn instantiate_poly_type(
        &self,
        poly_type: &PolyType,
    ) -> MonoType {
        // 简化实现：直接返回多态类型的主体
        // 实际实现应该进行完整的类型实例化
        poly_type.body.clone()
    }

    /// 注册局部变量
    fn register_local(
        &mut self,
        name: &str,
        local_idx: usize,
    ) {
        tlog!(
            debug,
            MSG::IrGenRegisterLocal,
            &name.to_string(),
            &local_idx.to_string()
        );
        if let Some(scope) = self.symbols.last_mut() {
            scope.insert(name.to_string(), SymbolEntry { local_idx });
        }
        // 保存变量名到当前函数的局部变量名列表
        // 确保向量长度足够（可能有空洞）
        if local_idx >= self.current_local_names.len() {
            self.current_local_names
                .resize(local_idx + 1, String::new());
        }
        self.current_local_names[local_idx] = name.to_string();
    }

    /// 查找局部变量
    fn lookup_local(
        &self,
        name: &str,
    ) -> Option<usize> {
        for scope in self.symbols.iter().rev() {
            if let Some(entry) = scope.get(name) {
                tlog!(
                    debug,
                    MSG::IrGenLookupLocal,
                    &name.to_string(),
                    &entry.local_idx.to_string()
                );
                return Some(entry.local_idx);
            }
        }
        tlog!(debug, MSG::IrGenLookupLocalNotFound, &name.to_string());
        None
    }

    /// 查找全局变量
    fn lookup_global(
        &self,
        name: &str,
    ) -> Option<usize> {
        for (idx, (var_name, _, _)) in self.global_vars.iter().enumerate() {
            if var_name == name {
                return Some(idx);
            }
        }
        None
    }

    /// 查找变量的类型
    fn lookup_var_type(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        if let Some(ref type_result) = self.type_result {
            // 调试：打印所有绑定
            tracing::debug!("Looking for variable '{}' in bindings", name);
            tracing::debug!("All bindings: {:?}", type_result.bindings);

            if let Some(poly_type) = type_result.bindings.get(name) {
                // 使用 debug 日志记录类型信息
                tracing::debug!("Found type for variable {}: {:?}", name, poly_type);
                return Some(poly_type);
            }
        } else {
            tracing::debug!("type_result is None!");
        }
        tracing::debug!("Type not found for variable: {}", name);
        None
    }

    // 删除的函数：extract_type_name_from_poly
    // 原因：根据设计文档，不再需要复杂的类型名提取逻辑
    // 方法调用现在直接生成简单函数名（方法名）

    /// 解析字段索引
    ///
    /// 从类型信息和结构体定义中动态查找字段在结构体中的位置。
    /// 查找顺序：
    /// 1. 从表达式的类型推导出结构体名，再从 struct_definitions 查找字段索引
    /// 2. 遍历所有结构体定义查找匹配的字段名（兜底）
    fn resolve_field_index(
        &self,
        expr: &ast::Expr,
        field_name: &str,
    ) -> Option<usize> {
        // 1. 尝试从表达式类型推导结构体名，精确查找
        if let Some(type_name) = self.get_expr_struct_type_name(expr) {
            if let Some(fields) = self.struct_definitions.get(&type_name) {
                for (i, field) in fields.iter().enumerate() {
                    if field.name == field_name {
                        return Some(i);
                    }
                }
            }
        }

        // 2. 兜底：遍历所有结构体定义查找字段名（当类型推导不可用时）
        for (_struct_name, fields) in &self.struct_definitions {
            for (i, field) in fields.iter().enumerate() {
                if field.name == field_name {
                    return Some(i);
                }
            }
        }

        // 3. 未找到，返回 None
        None
    }

    /// 从表达式推导其结构体类型名称
    ///
    /// 用于 resolve_field_index 等需要知道表达式类型的场景
    fn get_expr_struct_type_name(
        &self,
        expr: &ast::Expr,
    ) -> Option<String> {
        match expr {
            ast::Expr::Var(name, _) => {
                // 从类型检查结果查找变量类型
                if let Some(ref type_result) = self.type_result {
                    if let Some(mono_type) = type_result.local_var_types.get(name) {
                        return Self::mono_type_to_struct_name(mono_type);
                    }
                }
                // 从 bindings 查找
                if let Some(poly_type) = self.lookup_var_type(name) {
                    let mono_type = self.instantiate_poly_type(poly_type);
                    return Self::mono_type_to_struct_name(&mono_type);
                }
                // 从 IR 生成器追踪的类型查找
                if let Some(type_name) = self.local_var_types.get(name) {
                    if self.struct_definitions.contains_key(type_name) {
                        return Some(type_name.clone());
                    }
                }
                None
            }
            ast::Expr::Call { func, .. } => {
                // 构造器调用：Point(...) -> 类型名为 "Point"
                if let ast::Expr::Var(name, _) = func.as_ref() {
                    if self.struct_definitions.contains_key(name) {
                        return Some(name.clone());
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// 从 MonoType 提取结构体类型名
    fn mono_type_to_struct_name(mono_type: &MonoType) -> Option<String> {
        match mono_type {
            MonoType::TypeRef(name) => Some(name.clone()),
            MonoType::Struct(st) => Some(st.name.clone()),
            _ => None,
        }
    }

    /// 获取下一个临时寄存器编号
    fn next_temp_reg(&mut self) -> usize {
        let reg = self.next_temp;
        self.next_temp += 1;
        reg
    }

    /// 从 AST 模块生成 IR 模块
    pub fn generate_module_ir(
        &mut self,
        module: &ast::Module,
    ) -> Result<ModuleIR, Vec<IrGenError>> {
        let mut functions = Vec::new();
        let mut errors = Vec::new();
        let mut constants = Vec::new();

        for stmt in &module.items {
            match self.generate_stmt_ir(stmt, &mut constants) {
                Ok(Some(func_ir)) => functions.push(func_ir),
                Ok(None) => {}
                Err(e) => errors.push(e),
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        // 添加嵌套函数到模块函数列表
        functions.extend(std::mem::take(&mut self.nested_functions));

        Ok(ModuleIR {
            types: Vec::new(),
            globals: Vec::new(),
            functions,
            mut_locals: std::mem::take(&mut self.module_mut_locals),
            loop_binding_locals: std::mem::take(&mut self.module_loop_binding_locals),
            local_names: std::mem::take(&mut self.module_local_names),
            native_bindings: std::mem::take(&mut self.native_bindings),
        })
    }

    /// 生成语句的 IR
    fn generate_stmt_ir(
        &mut self,
        stmt: &ast::Stmt,
        constants: &mut Vec<ConstValue>,
    ) -> Result<Option<FunctionIR>, IrGenError> {
        match &stmt.kind {
            ast::StmtKind::Fn {
                name,
                generic_params: _,
                type_annotation,
                params,
                body: (stmts, expr),
                is_pub: _,
            } => self.generate_function_ir(
                name,
                type_annotation.as_ref(),
                params,
                stmts,
                expr,
                constants,
            ),
            ast::StmtKind::Var {
                name,
                type_annotation,
                initializer,
                is_mut: _,
            } => self.generate_global_var_ir(
                name,
                type_annotation.as_ref(),
                initializer.as_ref().map(|v| &**v),
            ),
            ast::StmtKind::MethodBind {
                type_name,
                method_name,
                method_type,
                params,
                body: (stmts, expr),
            } => self.generate_method_ir(
                type_name,
                method_name,
                method_type,
                params,
                stmts,
                expr,
                constants,
            ),
            ast::StmtKind::TypeDef {
                name, definition, ..
            } => self.generate_constructor_ir(name, definition),
            _ => Ok(None),
        }
    }

    /// 生成方法 IR
    #[allow(clippy::too_many_arguments)]
    fn generate_method_ir(
        &mut self,
        _type_name: &str,
        method_name: &str,
        method_type: &ast::Type,
        params: &[ast::Param],
        stmts: &[ast::Stmt],
        expr: &Option<Box<ast::Expr>>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<Option<FunctionIR>, IrGenError> {
        // 重置当前函数的可变局部变量追踪
        self.current_mut_locals.clear();
        // 重置当前函数的局部变量名列表
        self.current_local_names.clear();

        // 命名空间机制：方法函数名就是方法名，无复杂前缀
        // 例如：Point.get_x 生成函数名 "get_x"
        // 调用时：p.get_x() -> get_x(p)
        let func_name = method_name.to_string();

        // 解析返回类型
        let return_type = if let ast::Type::Fn { return_type, .. } = method_type {
            (**return_type).clone().into()
        } else {
            // 非函数类型，报错
            return Err(IrGenError::InternalError {
                message: format!("Method {} is not a function type", method_name),
                span: Span::default(),
            });
        };

        // 进入新作用域
        self.enter_scope();

        // 注册参数
        let mut param_types = Vec::new();
        for (i, param) in params.iter().enumerate() {
            if let Some(param_type_ast) = &param.ty {
                let param_type = param_type_ast.clone().into();
                param_types.push(param_type);
            } else {
                // 参数没有类型，默认为 Int64
                param_types.push(MonoType::Int(64));
            }

            // 注册参数到符号表
            self.register_local(&param.name, i);
        }

        // 生成指令序列
        let mut instructions = Vec::new();

        // 生成语句 IR
        for stmt in stmts {
            self.generate_local_stmt_ir(stmt, &mut instructions, constants)?;
        }

        // 生成表达式 IR
        if let Some(expr) = expr {
            let result_reg = 0;
            self.generate_expr_ir(expr, result_reg, &mut instructions, constants)?;
            instructions.push(Instruction::Ret(Some(Operand::Local(result_reg))));
        } else {
            instructions.push(Instruction::Ret(None));
        }

        // 退出作用域
        self.exit_scope();

        // 分配局部变量类型（简化：与参数相同）
        let locals_types = param_types.clone();

        // 构建函数 IR
        let func_ir = FunctionIR {
            name: func_name.clone(),
            params: param_types.clone(),
            return_type,
            is_async: false,
            locals: locals_types,
            blocks: vec![BasicBlock {
                label: 0,
                instructions,
                successors: Vec::new(),
            }],
            entry: 0,
        };

        // 保存当前函数的可变局部变量信息到模块级别映射
        if !self.current_mut_locals.is_empty() {
            self.module_mut_locals
                .insert(func_name.clone(), self.current_mut_locals.clone());
        }

        // 保存当前函数的循环绑定变量信息到模块级别映射
        if !self.current_loop_binding_locals.is_empty() {
            self.module_loop_binding_locals
                .insert(func_name.clone(), self.current_loop_binding_locals.clone());
        }

        // 保存当前函数的局部变量名列表
        self.module_local_names.insert(
            func_name.clone(),
            std::mem::take(&mut self.current_local_names),
        );

        Ok(Some(func_ir))
    }

    /// 生成函数 IR
    #[allow(clippy::only_used_in_recursion)]
    fn generate_function_ir(
        &mut self,
        name: &str,
        type_annotation: Option<&ast::Type>,
        params: &[ast::Param],
        stmts: &[ast::Stmt],
        expr: &Option<Box<ast::Expr>>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<Option<FunctionIR>, IrGenError> {
        // 检测 Native("symbol") 模式：函数体为空语句 + Native("...") 表达式
        // 形如: my_add: (a: Int, b: Int) -> Int = Native("my_add")
        if stmts.is_empty() {
            if let Some(expr_box) = expr {
                if let ast::Expr::Call {
                    func,
                    args,
                    span: _,
                } = expr_box.as_ref()
                {
                    if let Some(native_symbol) = crate::std::ffi::detect_native_binding(func, args)
                    {
                        // 记录 native 绑定，跳过函数体生成
                        self.native_bindings
                            .push(crate::std::ffi::NativeBinding::new(name, &native_symbol));
                        return Ok(None);
                    }
                }
            }
        }

        // 重置当前函数的可变局部变量追踪
        self.current_mut_locals.clear();
        // 重置当前函数的局部变量名列表
        self.current_local_names.clear();
        // 阶段3修复：改进返回类型解析，更好地与类型检查集成
        let return_type = match type_annotation {
            Some(ast::Type::Fn { return_type, .. }) => (**return_type).clone().into(),
            Some(ty) => ty.clone().into(),
            None => MonoType::Void,
        };

        // 生成函数体指令
        let mut instructions = Vec::new();

        // 进入函数体作用域
        self.enter_scope();

        // 为每个参数生成 LoadArg 指令并注册
        for (i, param) in params.iter().enumerate() {
            instructions.push(Instruction::Load {
                dst: Operand::Local(i),
                src: Operand::Arg(i),
            });
            // 存储到局部变量并注册
            instructions.push(Instruction::Store {
                dst: Operand::Local(i),
                src: Operand::Local(i),
                span: Span::dummy(),
            });
            self.register_local(&param.name, i);
            // Only mut parameters are registered as mutable
            if param.is_mut {
                self.current_mut_locals.insert(i);
            }
        }

        // 记录局部变量起始位置（在参数之后）
        let local_var_start = params.len();
        self.next_temp = local_var_start;

        // 处理语句
        for stmt in stmts {
            tlog!(
                debug,
                MSG::IrGenBeforeProcessStmt,
                &self.symbols.len().to_string()
            );
            self.generate_local_stmt_ir(stmt, &mut instructions, constants)?;
            tlog!(
                debug,
                MSG::IrGenAfterProcessStmt,
                &self.symbols.len().to_string()
            );
        }

        // 阶段3修复：简化返回值处理逻辑，明确表达式vs语句语义
        // 表达式函数：直接返回表达式的值
        // 语句函数：隐式返回Void或显式return
        if let Some(e) = expr {
            let result_reg = self.next_temp_reg();
            self.generate_expr_ir(e, result_reg, &mut instructions, constants)?;
            // 注意：generate_expr_ir 会为 Return 表达式添加 Ret 指令，
            // 所以这里不需要额外添加 Ret 指令
        } else {
            // 纯语句块：隐式返回Void
            instructions.push(Instruction::Ret(None));
        }

        // 退出函数体作用域
        tlog!(
            debug,
            MSG::IrGenAboutToExitScope,
            &self.symbols.len().to_string()
        );
        self.exit_scope();
        tlog!(
            debug,
            MSG::IrGenAfterExitScope,
            &self.symbols.len().to_string()
        );

        // 计算局部变量总数（用于 VM 分配帧空间）
        // 局部变量包括参数和函数体中声明的变量
        // 参数数量 + 临时寄存器使用数量
        let total_locals = self.next_temp;
        const MAX_LOCALS: usize = 65_535;
        if total_locals > MAX_LOCALS {
            return Err(IrGenError::InternalError {
                message: format!(
                    "too many locals allocated in function '{}': {}",
                    name, total_locals
                ),
                span: Span::dummy(),
            });
        }
        let locals_types: Vec<MonoType> = (0..total_locals)
            .map(|_| MonoType::Int(64)) // 简化：所有局部变量默认为 Int64
            .collect();

        // 构建函数 IR
        let func_ir = FunctionIR {
            name: name.to_string(),
            params: params
                .iter()
                .filter_map(|p| p.ty.clone())
                .map(|t| t.into())
                .collect(),
            return_type,
            is_async: false,
            locals: locals_types,
            blocks: vec![BasicBlock {
                label: 0,
                instructions,
                successors: Vec::new(),
            }],
            entry: 0,
        };

        // 保存当前函数的可变局部变量信息到模块级别映射
        if !self.current_mut_locals.is_empty() {
            self.module_mut_locals
                .insert(name.to_string(), self.current_mut_locals.clone());
        }

        // 保存当前函数的循环绑定变量信息到模块级别映射
        if !self.current_loop_binding_locals.is_empty() {
            self.module_loop_binding_locals
                .insert(name.to_string(), self.current_loop_binding_locals.clone());
        }

        // 保存当前函数的局部变量名列表
        self.module_local_names.insert(
            name.to_string(),
            std::mem::take(&mut self.current_local_names),
        );

        Ok(Some(func_ir))
    }

    /// 尝试将表达式求值为编译时常量
    fn eval_const_expr(
        &self,
        expr: &ast::Expr,
    ) -> Option<ConstValue> {
        match expr {
            ast::Expr::Lit(literal, _) => match literal {
                ast::Literal::Int(n) => Some(ConstValue::Int(*n)),
                ast::Literal::Float(f) => Some(ConstValue::Float(*f)),
                ast::Literal::Bool(b) => Some(ConstValue::Bool(*b)),
                ast::Literal::String(s) => Some(ConstValue::String(s.clone())),
                ast::Literal::Char(c) => Some(ConstValue::Char(*c)),
            },
            // TODO: 支持更复杂的常量表达式
            _ => None,
        }
    }

    /// 生成全局变量 IR
    fn generate_global_var_ir(
        &mut self,
        name: &str,
        type_annotation: Option<&ast::Type>,
        initializer: Option<&ast::Expr>,
    ) -> Result<Option<FunctionIR>, IrGenError> {
        let var_type = type_annotation
            .map(|t| (*t).clone().into())
            .unwrap_or(MonoType::Int(64));

        // 尝试从 initializer 提取常量值
        let init_value = if let Some(expr) = initializer {
            self.eval_const_expr(expr)
        } else {
            None
        };

        // 注册到全局变量表
        self.global_vars
            .push((name.to_string(), var_type.clone(), init_value.clone()));

        // 生成返回常量值的函数
        // x: Int = 42 => fn x() -> Int { return 42; }
        let result_reg = 0;
        let src_operand = match &init_value {
            Some(val) => Operand::Const(val.clone()),
            None => Operand::Const(ConstValue::Int(0)),
        };
        let instructions = vec![
            Instruction::Load {
                dst: Operand::Local(result_reg),
                src: src_operand,
            },
            Instruction::Ret(Some(Operand::Local(result_reg))),
        ];

        // 为全局变量创建函数
        let func_ir = FunctionIR {
            name: name.to_string(),
            params: Vec::new(),
            return_type: var_type,
            is_async: false,
            locals: vec![MonoType::Int(64)], // 分配一个局部变量用于存储结果
            blocks: vec![BasicBlock {
                label: 0,
                instructions,
                successors: Vec::new(),
            }],
            entry: 0,
        };

        Ok(Some(func_ir))
    }

    /// 生成构造函数 IR
    fn generate_constructor_ir(
        &mut self,
        _name: &str,
        definition: &ast::Type,
    ) -> Result<Option<FunctionIR>, IrGenError> {
        // 只为结构体类型生成构造函数
        match definition {
            ast::Type::NamedStruct {
                name: struct_name,
                fields,
            } => {
                // 记录结构体定义（用于调用时填充默认值）
                self.struct_definitions
                    .insert(struct_name.clone(), fields.clone());
                self.generate_struct_constructor_ir(struct_name, fields)
            }
            ast::Type::Struct { fields, bindings } => {
                self.struct_definitions
                    .insert(_name.to_string(), fields.clone());
                // 记录绑定信息（用于方法调用时的参数重排，RFC-004）
                self.register_type_bindings(_name, bindings);
                self.generate_struct_constructor_ir(_name, fields)
            }
            _ => {
                // 非结构体类型，不生成构造函数
                Ok(None)
            }
        }
    }

    /// 注册类型绑定信息（RFC-004）
    ///
    /// 将类型定义体内的绑定（外部函数绑定和匿名函数绑定）记录到 type_bindings 映射中，
    /// 用于后续方法调用 IR 生成时的参数重排。
    fn register_type_bindings(
        &mut self,
        type_name: &str,
        bindings: &[ast::TypeBodyBinding],
    ) {
        use ast::BindingKind;

        let mut binding_map = HashMap::new();

        for binding in bindings {
            match &binding.kind {
                BindingKind::External {
                    function,
                    positions,
                } => {
                    binding_map.insert(
                        binding.name.clone(),
                        BindingInfo {
                            function: function.clone(),
                            positions: positions.clone(),
                        },
                    );
                }
                BindingKind::Anonymous {
                    params: _,
                    return_type: _,
                    positions,
                    body: _,
                } => {
                    // 匿名函数绑定：函数名使用 "类型名.__anon_方法名" 格式
                    // 后续生成匿名函数的 IR 时使用此名称
                    let anon_func_name = format!("{}.__anon_{}", type_name, binding.name);
                    binding_map.insert(
                        binding.name.clone(),
                        BindingInfo {
                            function: anon_func_name,
                            positions: positions.clone(),
                        },
                    );
                }
            }
        }

        if !binding_map.is_empty() {
            self.type_bindings
                .insert(type_name.to_string(), binding_map);
        }
    }

    /// 为结构体生成构造函数 IR 的辅助方法
    fn generate_struct_constructor_ir(
        &self,
        struct_name: &str,
        fields: &[ast::StructField],
    ) -> Result<Option<FunctionIR>, IrGenError> {
        // 构造函数接受所有字段作为参数
        let mut param_types = Vec::new();
        for field in fields {
            param_types.push(field.ty.clone().into());
        }

        let mut instructions = Vec::new();

        // 将所有参数加载到局部变量中（用于 CreateStruct）
        let mut field_operands = Vec::new();
        for (i, _field) in fields.iter().enumerate() {
            let local_reg = i;
            instructions.push(Instruction::Load {
                dst: Operand::Local(local_reg),
                src: Operand::Arg(i),
            });
            field_operands.push(Operand::Local(local_reg));
        }

        // 使用 CreateStruct 指令创建结构体
        let result_reg = fields.len(); // 结果寄存器放在所有字段之后
        instructions.push(Instruction::CreateStruct {
            dst: Operand::Local(result_reg),
            type_name: struct_name.to_string(),
            fields: field_operands,
        });

        // 返回创建的结构体
        instructions.push(Instruction::Ret(Some(Operand::Local(result_reg))));

        // 局部变量类型：每个字段 + 结果寄存器
        let mut locals_types: Vec<MonoType> = fields.iter().map(|f| f.ty.clone().into()).collect();
        locals_types.push(MonoType::TypeRef(struct_name.to_string()));

        let func_ir = FunctionIR {
            name: struct_name.to_string(),
            params: param_types,
            return_type: MonoType::TypeRef(struct_name.to_string()),
            is_async: false,
            locals: locals_types,
            blocks: vec![BasicBlock {
                label: 0,
                instructions,
                successors: Vec::new(),
            }],
            entry: 0,
        };

        Ok(Some(func_ir))
    }

    /// 生成局部语句 IR
    #[allow(clippy::only_used_in_recursion)]
    fn generate_local_stmt_ir(
        &mut self,
        stmt: &ast::Stmt,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), IrGenError> {
        match &stmt.kind {
            ast::StmtKind::Expr(expr) => {
                let result_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, result_reg, instructions, constants)?;
            }
            ast::StmtKind::Var {
                name,
                type_annotation,
                initializer,
                is_mut,
            } => {
                // 记录变量的类型信息（用于错误消息）
                if let Some(type_ann) = type_annotation {
                    let mono: MonoType = type_ann.clone().into();
                    let type_name = mono.type_name();
                    self.local_var_types.insert(name.clone(), type_name);
                } else if let Some(init_expr) = initializer {
                    // 优先使用 typecheck 结果推导类型名，AST 推断仅作为兜底
                    let inferred = self.get_expr_type_name(init_expr);
                    if inferred != "<unknown>" {
                        self.local_var_types.insert(name.clone(), inferred);
                    }
                }

                // 检查变量是否已经存在于当前或外层作用域
                // 如果存在，这是赋值操作而不是新声明
                let var_idx = if let Some(existing_idx) = self.lookup_local(name) {
                    // 变量已存在，复用其索引（这是赋值操作）
                    existing_idx
                } else {
                    // 新变量声明，分配新索引
                    let idx = self.next_temp_reg();
                    self.register_local(name, idx);
                    // 记录可变性信息
                    if *is_mut {
                        self.current_mut_locals.insert(idx);
                    }
                    idx
                };

                if let Some(expr) = initializer {
                    self.generate_expr_ir(expr, var_idx, instructions, constants)?;
                } else {
                    // 默认初始化为 0
                    instructions.push(Instruction::Load {
                        dst: Operand::Local(var_idx),
                        src: Operand::Const(ConstValue::Int(0)),
                    });
                }
                // 生成 Store 指令将值存储到局部变量
                instructions.push(Instruction::Store {
                    dst: Operand::Local(var_idx),
                    src: Operand::Local(var_idx),
                    span: stmt.span,
                });
            }
            ast::StmtKind::Fn {
                name,
                generic_params: _,
                type_annotation,
                params,
                body: (stmts, expr),
                is_pub: _,
            } => {
                // 生成嵌套函数的 IR
                match self.generate_function_ir(
                    name,
                    type_annotation.as_ref(),
                    params,
                    stmts,
                    expr,
                    constants,
                ) {
                    Ok(Some(func_ir)) => {
                        // 将嵌套函数添加到列表（会被提升到模块级别）
                        self.nested_functions.push(func_ir);
                    }
                    Ok(None) => {} // Native 函数或其他情况
                    Err(e) => return Err(e),
                }
            }
            ast::StmtKind::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                span: _,
            } => {
                // 生成 if 语句的 IR
                self.generate_if_stmt_ir(
                    condition,
                    then_branch,
                    elif_branches,
                    else_branch.as_deref(),
                    instructions,
                    constants,
                )?;
            }
            ast::StmtKind::For {
                var,
                var_mut,
                iterable,
                body,
                label: _,
            } => {
                self.generate_for_loop_ir(
                    var,
                    *var_mut,
                    iterable,
                    body,
                    None, // No result needed for statement
                    stmt.span,
                    instructions,
                    constants,
                )?;
            }
            // 处理其他语句类型
            _ => {}
        }
        Ok(())
    }

    /// 生成 if 语句的 IR
    fn generate_if_stmt_ir(
        &mut self,
        condition: &ast::Expr,
        then_branch: &ast::Block,
        elif_branches: &[(Box<ast::Expr>, Box<ast::Block>)],
        else_branch: Option<&ast::Block>,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), IrGenError> {
        // 进入新的作用域
        self.enter_scope();

        // 1. 评估条件
        let condition_reg = self.next_temp_reg();
        self.generate_expr_ir(condition, condition_reg, instructions, constants)?;

        // 2. 跳转到下一个分支的占位符 (JmpIfNot to next_branch)
        let jump_to_next_branch_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot(Operand::Local(condition_reg), 0)); // 占位符

        // 3. 生成 then 分支
        self.generate_block_ir(then_branch, instructions, constants)?;

        // 4. then 分支结束后，跳转到整个 if 语句的结束 (Jmp to end)
        let mut jump_to_end_indices = Vec::new();
        // 只有当有 else/elif 时才需要跳过它们，否则这里已经是 end
        if !elif_branches.is_empty() || else_branch.is_some() {
            let idx = instructions.len();
            instructions.push(Instruction::Jmp(0)); // 占位符
            jump_to_end_indices.push(idx);
        }

        // 5. 修复条件跳转 (JmpIfNot)，使其指向 elif 或 else (即当前位置)
        let len = instructions.len();
        if let Instruction::JmpIfNot(_, ref mut target) = instructions[jump_to_next_branch_idx] {
            *target = len;
        }

        // 6. 处理 elif 分支
        for (elif_condition, elif_body) in elif_branches.iter() {
            // 评估 elif 条件
            let elif_condition_reg = self.next_temp_reg();
            self.generate_expr_ir(elif_condition, elif_condition_reg, instructions, constants)?;

            // 跳转到下一个分支 (JmpIfNot)
            let jump_to_next_elif_idx = instructions.len();
            instructions.push(Instruction::JmpIfNot(Operand::Local(elif_condition_reg), 0));

            // 生成 elif 分支
            self.generate_block_ir(elif_body, instructions, constants)?;

            // elif 分支结束后跳转到结束
            let idx = instructions.len();
            instructions.push(Instruction::Jmp(0)); // 占位符
            jump_to_end_indices.push(idx);

            // 修复条件跳转
            let len = instructions.len();
            if let Instruction::JmpIfNot(_, ref mut target) = instructions[jump_to_next_elif_idx] {
                *target = len;
            }
        }

        // 7. 生成 else 分支
        if let Some(else_body) = else_branch {
            self.generate_block_ir(else_body, instructions, constants)?;
        }

        // 8. 修复所有跳转到结束的指令
        let end_pos = instructions.len();
        for idx in jump_to_end_indices {
            if let Instruction::Jmp(ref mut target) = instructions[idx] {
                *target = end_pos;
            }
        }

        // 退出作用域
        self.exit_scope();

        Ok(())
    }

    /// 生成 if 表达式的 IR
    #[allow(clippy::too_many_arguments)]
    fn generate_if_expr_ir(
        &mut self,
        condition: &ast::Expr,
        then_branch: &ast::Block,
        elif_branches: &[(Box<ast::Expr>, Box<ast::Block>)],
        else_branch: Option<&ast::Block>,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), IrGenError> {
        // 进入新的作用域
        self.enter_scope();

        // 1. 评估条件
        let condition_reg = self.next_temp_reg();
        self.generate_expr_ir(condition, condition_reg, instructions, constants)?;

        // 2. 跳转到下一个分支的占位符 (JmpIfNot to next)
        let jump_to_next_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot(Operand::Local(condition_reg), 0)); // 占位符

        // 3. then 分支
        let then_result_reg = self.next_temp_reg();
        self.generate_block_expr_ir(then_branch, then_result_reg, instructions, constants)?;
        instructions.push(Instruction::Move {
            dst: Operand::Local(result_reg),
            src: Operand::Local(then_result_reg),
        });

        // 4. 跳转到结束 (Jmp to end)
        let mut jumps_to_end = Vec::new();
        let jmp_idx = instructions.len();
        instructions.push(Instruction::Jmp(0)); // 占位符
        jumps_to_end.push(jmp_idx);

        // 5. 修复条件跳转
        let len = instructions.len();
        if let Instruction::JmpIfNot(_, ref mut target) = instructions[jump_to_next_idx] {
            *target = len;
        }

        // 6. Elif 分支
        for (elif_condition, elif_body) in elif_branches.iter() {
            let elif_cond_reg = self.next_temp_reg();
            self.generate_expr_ir(elif_condition, elif_cond_reg, instructions, constants)?;

            let jump_idx = instructions.len();
            instructions.push(Instruction::JmpIfNot(Operand::Local(elif_cond_reg), 0));

            let elif_res = self.next_temp_reg();
            self.generate_block_expr_ir(elif_body, elif_res, instructions, constants)?;
            instructions.push(Instruction::Move {
                dst: Operand::Local(result_reg),
                src: Operand::Local(elif_res),
            });

            let jmp_end_idx = instructions.len();
            instructions.push(Instruction::Jmp(0));
            jumps_to_end.push(jmp_end_idx);
            let len = instructions.len();
            if let Instruction::JmpIfNot(_, ref mut target) = instructions[jump_idx] {
                *target = len;
            }
        }

        // 7. Else 分支
        if let Some(else_body) = else_branch {
            let else_res = self.next_temp_reg();
            self.generate_block_expr_ir(else_body, else_res, instructions, constants)?;
            instructions.push(Instruction::Move {
                dst: Operand::Local(result_reg),
                src: Operand::Local(else_res),
            });
        }

        // 8. 修复所有跳转到结束的指令
        let end_len = instructions.len();
        for idx in jumps_to_end {
            if let Instruction::Jmp(ref mut target) = instructions[idx] {
                *target = end_len;
            }
        }

        self.exit_scope();
        Ok(())
    }

    /// 生成代码块的 IR（用于表达式）
    fn generate_block_ir(
        &mut self,
        block: &ast::Block,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), IrGenError> {
        // 进入新的作用域
        self.enter_scope();

        // 生成语句
        for stmt in &block.stmts {
            self.generate_local_stmt_ir(stmt, instructions, constants)?;
        }

        // 生成表达式（如果有）
        if let Some(expr) = &block.expr {
            let result_reg = self.next_temp_reg();
            self.generate_expr_ir(expr, result_reg, instructions, constants)?;
        }

        // 退出作用域
        self.exit_scope();

        Ok(())
    }

    /// 生成代码块的 IR（用于表达式）
    fn generate_block_expr_ir(
        &mut self,
        block: &ast::Block,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), IrGenError> {
        // 进入新的作用域
        self.enter_scope();

        // 生成语句
        for stmt in &block.stmts {
            self.generate_local_stmt_ir(stmt, instructions, constants)?;
        }

        // 生成表达式（如果有）
        if let Some(expr) = &block.expr {
            let temp_reg = self.next_temp_reg();
            self.generate_expr_ir(expr, temp_reg, instructions, constants)?;

            // 将表达式结果移动到目标寄存器
            instructions.push(Instruction::Move {
                dst: Operand::Local(result_reg),
                src: Operand::Local(temp_reg),
            });
        } else {
            // 如果块没有表达式，返回 0（Void）
            instructions.push(Instruction::Load {
                dst: Operand::Local(result_reg),
                src: Operand::Const(ConstValue::Int(0)),
            });
        }

        // 退出作用域
        self.exit_scope();

        Ok(())
    }

    /// Generate While expression IR
    fn generate_while_expr_ir(
        &mut self,
        condition: &ast::Expr,
        body: &ast::Block,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), IrGenError> {
        // Label: condition_check
        let loop_start_idx = instructions.len();

        // Evaluate condition
        let cond_reg = self.next_temp_reg();
        self.generate_expr_ir(condition, cond_reg, instructions, constants)?;

        // Jump to end if false
        let jump_end_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot(Operand::Local(cond_reg), 0)); // Placeholder

        // Body
        self.generate_block_ir(body, instructions, constants)?;

        // Jump back to start
        instructions.push(Instruction::Jmp(loop_start_idx));

        // Fix JmpIfNot target
        let end_idx = instructions.len();
        if let Instruction::JmpIfNot(_, ref mut target) = instructions[jump_end_idx] {
            *target = end_idx;
        }

        // While loop returns void/unit (0)
        instructions.push(Instruction::Load {
            dst: Operand::Local(result_reg),
            src: Operand::Const(ConstValue::Int(0)),
        });

        Ok(())
    }

    /// Generate For loop IR (simplified range loop)
    #[allow(clippy::too_many_arguments)]
    fn generate_for_loop_ir(
        &mut self,
        var_name: &str,
        #[allow(unused_variables)] var_mut: bool,
        iterable: &ast::Expr,
        body: &ast::Block,
        result_reg: Option<usize>,
        for_span: Span,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), IrGenError> {
        // Check for range loop: var in start..end
        if let ast::Expr::BinOp {
            op: ast::BinOp::Range,
            left,
            right,
            ..
        } = iterable
        {
            // Desugar to iterator-based loop (每次迭代从迭代器获取新值，不是递增)
            // for i in 1..5 等价于：
            // current = 1
            // end = 5
            // while current < end {
            //     // 将 current 值存储到循环变量的 slot
            //     body 中访问 i 时，从这个 slot 读取
            //     current = current + 1
            // }
            self.enter_scope();

            // 0. 创建迭代器状态结构
            let current_reg = self.next_temp_reg(); // 当前迭代值
            let end_reg = self.next_temp_reg(); // 结束值
            let var_reg = self.next_temp_reg(); // 循环变量的存储位置

            // 注册循环变量 - 让变量访问指向 var_reg
            self.register_local(var_name, var_reg);

            // for 循环变量的 Store 是"绑定"操作，不是"修改"
            // 将 var_reg 添加到循环绑定变量集合
            self.current_loop_binding_locals.insert(var_reg);

            // 如果使用 for mut，用户可以在循环体内修改变量
            if var_mut {
                self.current_mut_locals.insert(var_reg);
            }

            // 1. 初始化：current = start, end = end
            self.generate_expr_ir(left, current_reg, instructions, constants)?;
            self.generate_expr_ir(right, end_reg, instructions, constants)?;

            // 将初始值存储到循环变量的 slot
            instructions.push(Instruction::Store {
                dst: Operand::Local(var_reg),
                src: Operand::Local(current_reg),
                span: for_span,
            });

            // Loop start label
            let loop_start_idx = instructions.len();

            // 2. Condition check: current < end
            let cond_reg = self.next_temp_reg();
            instructions.push(Instruction::Lt {
                dst: Operand::Local(cond_reg),
                lhs: Operand::Local(current_reg),
                rhs: Operand::Local(end_reg),
            });

            // 3. Jump to end if current >= end
            let jump_end_idx = instructions.len();
            instructions.push(Instruction::JmpIfNot(Operand::Local(cond_reg), 0));

            // 4. 执行循环体
            // 循环体访问 i 时，会从 var_reg 读取
            // var_reg 在每次循环迭代前都会被更新为 current 的值
            self.generate_block_ir(body, instructions, constants)?;

            // 5. 递增：current = current + 1
            let one_reg = self.next_temp_reg();
            instructions.push(Instruction::Load {
                dst: Operand::Local(one_reg),
                src: Operand::Const(ConstValue::Int(1)),
            });
            instructions.push(Instruction::Add {
                dst: Operand::Local(current_reg),
                lhs: Operand::Local(current_reg),
                rhs: Operand::Local(one_reg),
            });

            // 6. 将新的 current 值存储到循环变量的 slot
            instructions.push(Instruction::Store {
                dst: Operand::Local(var_reg),
                src: Operand::Local(current_reg),
                span: for_span,
            });

            // 7. 跳转回循环开始
            instructions.push(Instruction::Jmp(loop_start_idx));

            // 8. Fix jump
            let end_idx = instructions.len();
            if let Instruction::JmpIfNot(_, ref mut target) = instructions[jump_end_idx] {
                *target = end_idx;
            }

            self.exit_scope();

            // If expression, load unit
            if let Some(reg) = result_reg {
                instructions.push(Instruction::Load {
                    dst: Operand::Local(reg),
                    src: Operand::Const(ConstValue::Int(0)),
                });
            }

            Ok(())
        } else if let Some(iter_ty) = self.get_expr_mono_type(iterable) {
            match iter_ty {
                // 使用迭代器协议的 For 循环
                MonoType::List(_) | MonoType::Tuple(_) | MonoType::Dict(_, _) => self
                    .generate_iterator_for_loop_ir(
                        var_name,
                        iterable,
                        body,
                        result_reg,
                        for_span,
                        instructions,
                        constants,
                    ),
                _ => {
                    // 不支持的迭代器类型，返回错误（使用实际类型名称）
                    let iter_type = self.get_expr_type_name(iterable);
                    let span = Self::get_expr_span(iterable);
                    Err(IrGenError::UnsupportedIterator { iter_type, span })
                }
            }
        } else {
            // 不支持的迭代器类型，返回错误（使用实际类型名称）
            let iter_type = self.get_expr_type_name(iterable);
            let span = Self::get_expr_span(iterable);
            Err(IrGenError::UnsupportedIterator { iter_type, span })
        }
    }

    /// 生成基于迭代器协议的 For 循环 IR
    /// 这是新的迭代器协议实现，调用 iter()/next()/has_next() 方法
    #[allow(clippy::too_many_arguments)]
    fn generate_iterator_for_loop_ir(
        &mut self,
        var_name: &str,
        iterable: &ast::Expr,
        body: &ast::Block,
        result_reg: Option<usize>,
        for_span: Span,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), IrGenError> {
        self.enter_scope();

        // 1. 计算可迭代对象
        let iterable_reg = self.next_temp_reg();
        self.generate_expr_ir(iterable, iterable_reg, instructions, constants)?;

        // 2. 创建迭代器: iterator = iter(iterable)
        // 使用 Call 指令调用 std.list.iter 函数
        let iterator_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(Operand::Local(iterator_reg)),
            func: Operand::Const(ConstValue::String("std.list.iter".to_string())),
            args: vec![Operand::Local(iterable_reg)],
        });

        // 3. 注册循环变量
        let var_reg = self.next_temp_reg();
        self.register_local(var_name, var_reg);

        // 4. 循环开始
        let loop_start_idx = instructions.len();

        // 5. 检查是否有更多元素: has_more = has_next(iterator)
        // 使用 Call 指令调用 std.list.has_next 函数
        let has_more_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(Operand::Local(has_more_reg)),
            func: Operand::Const(ConstValue::String("std.list.has_next".to_string())),
            args: vec![Operand::Local(iterator_reg)],
        });

        // 6. 如果没有更多元素，跳转到结束
        let jump_end_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot(Operand::Local(has_more_reg), 0));

        // 7. 获取下一个元素: var = next(iterator)
        // 使用 Call 指令调用 std.list.next 函数
        let element_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(Operand::Local(element_reg)),
            func: Operand::Const(ConstValue::String("std.list.next".to_string())),
            args: vec![Operand::Local(iterator_reg)],
        });
        instructions.push(Instruction::Store {
            dst: Operand::Local(var_reg),
            src: Operand::Local(element_reg),
            span: for_span,
        });

        // 8. 执行循环体
        self.generate_block_ir(body, instructions, constants)?;

        // 9. 跳转回循环开始
        instructions.push(Instruction::Jmp(loop_start_idx));

        // 10. 修复跳转
        let end_idx = instructions.len();
        if let Instruction::JmpIfNot(_, ref mut target) = instructions[jump_end_idx] {
            *target = end_idx;
        }

        self.exit_scope();

        if let Some(reg) = result_reg {
            instructions.push(Instruction::Load {
                dst: Operand::Local(reg),
                src: Operand::Const(ConstValue::Int(0)),
            });
        }

        Ok(())
    }

    // 迭代器协议已通过 Call 指令实现，不再需要独立的 IR 指令
    // 保留指令定义以供将来使用

    /// 获取表达式的 span
    fn get_expr_span(expr: &ast::Expr) -> Span {
        match expr {
            ast::Expr::Lit(_, span) => *span,
            ast::Expr::Var(_, span) => *span,
            ast::Expr::BinOp { span, .. } => *span,
            ast::Expr::UnOp { span, .. } => *span,
            ast::Expr::Call { span, .. } => *span,
            ast::Expr::FnDef { span, .. } => *span,
            ast::Expr::If { span, .. } => *span,
            ast::Expr::Match { span, .. } => *span,
            ast::Expr::While { span, .. } => *span,
            ast::Expr::For { span, .. } => *span,
            ast::Expr::Block(block) => block.span,
            ast::Expr::Return(_, span) => *span,
            ast::Expr::Break(_, span) => *span,
            ast::Expr::Continue(_, span) => *span,
            ast::Expr::Cast { span, .. } => *span,
            ast::Expr::Tuple(_, span) => *span,
            ast::Expr::List(_, span) => *span,
            ast::Expr::ListComp { span, .. } => *span,
            ast::Expr::Dict(_, span) => *span,
            ast::Expr::Index { span, .. } => *span,
            ast::Expr::FieldAccess { span, .. } => *span,
            ast::Expr::Try { span, .. } => *span,
            ast::Expr::Ref { span, .. } => *span,
            ast::Expr::Unsafe { span, .. } => *span,
            ast::Expr::Lambda { span, .. } => *span,
        }
    }

    /// 获取表达式的实际类型名称（用于错误消息）
    ///
    /// 通过查询类型检查结果获取表达式的真正类型，而不是仅描述 AST 节点结构。
    /// 例如对于变量 `nums`，返回 `List<int64>` 而非 `变量 \`nums\``。
    fn get_expr_type_name(
        &self,
        expr: &ast::Expr,
    ) -> String {
        // 如果表达式是变量，尝试从多个来源查找其类型
        if let ast::Expr::Var(name, _) = expr {
            // 1. 从类型检查结果中的 local_var_types 查找（最准确，包含具体类型）
            if let Some(ref type_result) = self.type_result {
                if let Some(mono_type) = type_result.local_var_types.get(name) {
                    return mono_type.type_name();
                }
            }
            // 2. 从 bindings 中查找全局绑定
            if let Some(poly_type) = self.lookup_var_type(name) {
                let mono_type = self.instantiate_poly_type(poly_type);
                return mono_type.type_name();
            }
            // 3. 从 IR 生成器本地追踪的类型中查找
            if let Some(type_name) = self.local_var_types.get(name) {
                return type_name.clone();
            }
        }

        // 对于非变量表达式，不做 AST 猜测，避免掩盖类型系统问题
        "<unknown>".to_string()
    }

    /// 获取表达式的推断类型（用于 IR 生成阶段的分支）
    fn get_expr_mono_type(
        &self,
        expr: &ast::Expr,
    ) -> Option<MonoType> {
        match expr {
            ast::Expr::Var(name, _) => {
                if let Some(ref type_result) = self.type_result {
                    if let Some(mono_type) = type_result.local_var_types.get(name) {
                        return Some(mono_type.clone());
                    }
                }

                self.lookup_var_type(name)
                    .map(|poly_type| self.instantiate_poly_type(poly_type))
            }
            ast::Expr::List(_, _) => Some(MonoType::List(Box::new(MonoType::Void))),
            ast::Expr::Tuple(items, _) => {
                let elems = vec![MonoType::Void; items.len()];
                Some(MonoType::Tuple(elems))
            }
            ast::Expr::Dict(_, _) => Some(MonoType::Dict(
                Box::new(MonoType::Void),
                Box::new(MonoType::Void),
            )),
            _ => None,
        }
    }

    /// 生成 Lambda 函数体 IR
    ///
    /// 返回闭包函数体的指令列表和局部变量信息
    fn generate_lambda_body_ir(
        &mut self,
        params: &[ast::Param],
        body: &ast::Block,
        constants: &mut Vec<ConstValue>,
    ) -> Result<LambdaBodyIR, IrGenError> {
        // 保存父函数的可变局部变量和局部变量名信息
        let saved_mut_locals = std::mem::take(&mut self.current_mut_locals);
        let saved_local_names = std::mem::take(&mut self.current_local_names);
        let saved_next_temp = self.next_temp;

        let mut instructions = Vec::new();

        // 进入闭包函数体作用域
        self.enter_scope();

        // 为每个参数生成 LoadArg 指令并注册
        for (i, param) in params.iter().enumerate() {
            instructions.push(Instruction::Load {
                dst: Operand::Local(i),
                src: Operand::Arg(i),
            });
            // 存储到局部变量并注册
            instructions.push(Instruction::Store {
                dst: Operand::Local(i),
                src: Operand::Local(i),
                span: Span::dummy(),
            });
            self.register_local(&param.name, i);
            // Only mut parameters are registered as mutable
            if param.is_mut {
                self.current_mut_locals.insert(i);
            }
        }

        // 记录局部变量起始位置
        let local_var_start = params.len();
        self.next_temp = local_var_start;

        // 处理函数体语句
        for stmt in &body.stmts {
            self.generate_local_stmt_ir(stmt, &mut instructions, constants)?;
        }

        // 处理返回值表达式
        // 使用 next_temp_reg 分配独立的返回值寄存器，避免与参数寄存器冲突
        if let Some(expr) = &body.expr {
            let result_reg = self.next_temp_reg();
            self.generate_expr_ir(expr, result_reg, &mut instructions, constants)?;
            // 添加返回指令
            instructions.push(Instruction::Ret(Some(Operand::Local(result_reg))));
        } else {
            // 隐式返回 Void
            instructions.push(Instruction::Ret(None));
        }

        // 退出作用域
        self.exit_scope();

        // 计算局部变量总数
        let total_locals = self.next_temp;
        let locals_types: Vec<MonoType> = (0..total_locals).map(|_| MonoType::Int(64)).collect();

        // 保存当前闭包函数的可变局部变量信息
        let mut_locals = std::mem::take(&mut self.current_mut_locals);

        // 恢复父函数的可变局部变量和局部变量名信息
        self.current_mut_locals = saved_mut_locals;
        self.current_local_names = saved_local_names;
        self.next_temp = saved_next_temp;

        Ok(LambdaBodyIR {
            instructions,
            locals: locals_types,
            mut_locals,
        })
    }

    /// 生成表达式 IR
    #[allow(clippy::only_used_in_recursion)]
    fn generate_expr_ir(
        &mut self,
        expr: &ast::Expr,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), IrGenError> {
        match expr {
            Expr::Lit(literal, _) => {
                // 常量加载
                let const_val = match literal {
                    Literal::Int(n) => ConstValue::Int(*n),
                    Literal::Float(f) => ConstValue::Float(*f),
                    Literal::Bool(b) => ConstValue::Bool(*b),
                    Literal::String(s) => ConstValue::String(s.clone()),
                    Literal::Char(c) => ConstValue::Char(*c),
                };
                // 添加到常量池
                constants.push(const_val.clone());
                instructions.push(Instruction::Load {
                    dst: Operand::Local(result_reg),
                    src: Operand::Const(const_val),
                });
            }
            Expr::Var(_, _) => {
                // 变量加载 - 首先查找局部变量，然后查找全局变量
                let var_name = if let Expr::Var(name, _) = expr {
                    name.clone()
                } else {
                    String::new()
                };

                if let Some(local_idx) = self.lookup_local(&var_name) {
                    // 局部变量：直接加载
                    instructions.push(Instruction::Load {
                        dst: Operand::Local(result_reg),
                        src: Operand::Local(local_idx),
                    });
                } else if self.lookup_global(&var_name).is_some() {
                    // 全局变量：生成函数调用获取值
                    let func_name = var_name.clone();
                    instructions.push(Instruction::Call {
                        dst: Some(Operand::Local(result_reg)),
                        func: Operand::Const(ConstValue::String(func_name)),
                        args: vec![],
                    });
                } else {
                    // 未找到变量，默认加载 0
                    instructions.push(Instruction::Load {
                        dst: Operand::Local(result_reg),
                        src: Operand::Const(ConstValue::Int(0)),
                    });
                }
            }
            Expr::BinOp {
                op,
                left,
                right,
                span,
            } => {
                tlog!(debug, MSG::DebugGeneratingIRBinOp, &format!("{:?}", op));
                // 二元运算
                let instr = match op {
                    ast::BinOp::Assign => {
                        if let Expr::Var(var_name, _) = left.as_ref() {
                            let local_idx = if let Some(idx) = self.lookup_local(var_name) {
                                idx
                            } else {
                                let idx = self.next_temp_reg();
                                self.register_local(var_name, idx);
                                idx
                            };
                            let val_reg = self.next_temp_reg();
                            self.generate_expr_ir(right, val_reg, instructions, constants)?;

                            // 更新变量的类型信息
                            // 优先使用 typecheck 结果推导类型名，AST 推断仅作为兜底
                            let inferred = self.get_expr_type_name(right);
                            if inferred != "<unknown>" {
                                self.local_var_types.insert(var_name.clone(), inferred);
                            }

                            instructions.push(Instruction::Store {
                                dst: Operand::Local(local_idx),
                                src: Operand::Local(val_reg),
                                span: *span,
                            });
                            instructions.push(Instruction::Load {
                                dst: Operand::Local(result_reg),
                                src: Operand::Local(local_idx),
                            });
                        }
                        return Ok(());
                    }
                    _ => {
                        let left_reg = self.next_temp_reg();
                        let right_reg = self.next_temp_reg();
                        self.generate_expr_ir(left, left_reg, instructions, constants)?;
                        self.generate_expr_ir(right, right_reg, instructions, constants)?;

                        match op {
                            ast::BinOp::Add => Instruction::Add {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Sub => Instruction::Sub {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Mul => Instruction::Mul {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Div => Instruction::Div {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Mod => Instruction::Mod {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Eq => Instruction::Eq {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Neq => Instruction::Ne {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Lt => Instruction::Lt {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Le => Instruction::Le {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Gt => Instruction::Gt {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Ge => Instruction::Ge {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            // ast::BinOp::Assign case is handled above checking left/right generation.
                            // This placeholder is just to remove the old duplicated block.
                            _ => Instruction::Move {
                                dst: Operand::Local(result_reg),
                                src: Operand::Const(ConstValue::Int(0)),
                            },
                        }
                    }
                };
                instructions.push(instr);
            }
            Expr::Call {
                func,
                args,
                span: _,
            } => {
                // 检查是否是方法调用：func 是 FieldAccess
                if let Expr::FieldAccess { expr, field, .. } = func.as_ref() {
                    // 方法调用 - 转换为普通函数调用
                    // 命名空间机制：p.method() -> method(p)

                    // 只有非命名空间调用才需要添加 self 参数
                    // 命名空间调用（如 std.io.println）不需要隐式参数
                    if is_namespace_call(expr) {
                        // 命名空间调用：不需要隐式参数
                        let mut arg_regs = Vec::new();
                        for arg in args.iter() {
                            let arg_reg = self.next_temp_reg();
                            self.generate_expr_ir(arg, arg_reg, instructions, constants)?;
                            arg_regs.push(Operand::Local(arg_reg));
                        }
                        let method_function_name = extract_namespace_path(expr, field);
                        instructions.push(Instruction::Call {
                            dst: Some(Operand::Local(result_reg)),
                            func: Operand::Const(ConstValue::String(
                                method_function_name.to_string(),
                            )),
                            args: arg_regs,
                        });
                    } else {
                        // 非命名空间调用：检查是否有绑定信息（RFC-004）
                        let binding_info =
                            self.get_expr_struct_type_name(expr).and_then(|type_name| {
                                self.type_bindings
                                    .get(&type_name)
                                    .and_then(|bindings| bindings.get(field).cloned())
                            });

                        if let Some(binding) = binding_info {
                            // 绑定方法调用：按 RFC-004 进行参数重排
                            // obj.method(arg1, arg2) + binding positions [0]
                            // → original_function(obj, arg1, arg2)
                            //
                            // obj.method(arg1) + binding positions [1]
                            // → original_function(arg1, obj)

                            // 首先生成对象表达式 IR
                            let obj_reg = self.next_temp_reg();
                            self.generate_expr_ir(expr, obj_reg, instructions, constants)?;

                            // 生成所有方法参数 IR
                            let mut method_arg_regs = Vec::new();
                            for arg in args.iter() {
                                let arg_reg = self.next_temp_reg();
                                self.generate_expr_ir(arg, arg_reg, instructions, constants)?;
                                method_arg_regs.push(Operand::Local(arg_reg));
                            }

                            // 按绑定位置重排参数
                            // 总参数数 = 绑定位置数(obj填充) + 方法参数数
                            let total_params = binding.positions.len() + method_arg_regs.len();
                            let mut final_args: Vec<Option<Operand>> = vec![None; total_params];

                            // 将 obj 放入绑定位置
                            for &pos in &binding.positions {
                                if pos < total_params {
                                    final_args[pos] = Some(Operand::Local(obj_reg));
                                }
                            }

                            // 将方法参数填充到剩余位置
                            let mut method_arg_iter = method_arg_regs.into_iter();
                            for slot in final_args.iter_mut() {
                                if slot.is_none() {
                                    if let Some(arg) = method_arg_iter.next() {
                                        *slot = Some(arg);
                                    }
                                }
                            }

                            // 收集最终参数列表
                            let final_arg_regs: Vec<Operand> =
                                final_args.into_iter().flatten().collect();

                            // 解析函数名
                            let func_name = if let Some(qualified) =
                                SHORT_TO_QUALIFIED.get(&binding.function)
                            {
                                qualified.clone()
                            } else {
                                binding.function.clone()
                            };

                            instructions.push(Instruction::Call {
                                dst: Some(Operand::Local(result_reg)),
                                func: Operand::Const(ConstValue::String(func_name)),
                                args: final_arg_regs,
                            });
                        } else {
                            // 常规方法调用（无绑定）：obj.method(args) → method(obj, args)
                            let mut arg_regs = Vec::new();

                            // 生成对象表达式 IR（作为第一个参数）
                            let obj_reg = self.next_temp_reg();
                            self.generate_expr_ir(expr, obj_reg, instructions, constants)?;
                            arg_regs.push(Operand::Local(obj_reg));

                            // 生成方法参数 IR
                            for arg in args.iter() {
                                let arg_reg = self.next_temp_reg();
                                self.generate_expr_ir(arg, arg_reg, instructions, constants)?;
                                arg_regs.push(Operand::Local(arg_reg));
                            }

                            let method_function_name = extract_namespace_path(expr, field);

                            instructions.push(Instruction::Call {
                                dst: Some(Operand::Local(result_reg)),
                                func: Operand::Const(ConstValue::String(
                                    method_function_name.to_string(),
                                )),
                                args: arg_regs,
                            });
                        }
                    }
                } else {
                    // 普通函数调用
                    let mut arg_regs = Vec::new();
                    for arg in args.iter() {
                        let arg_reg = self.next_temp_reg();
                        self.generate_expr_ir(arg, arg_reg, instructions, constants)?;
                        arg_regs.push(Operand::Local(arg_reg));
                    }

                    // 检查是否是结构体构造器调用，需要填充默认值
                    if let Expr::Var(name, _) = func.as_ref() {
                        if let Some(fields) = self.struct_definitions.get(name).cloned() {
                            // 这是一个结构体构造器调用
                            // 如果提供的参数数少于字段数，用默认值填充
                            if arg_regs.len() < fields.len() {
                                for field in fields.iter().skip(arg_regs.len()) {
                                    let default_reg = self.next_temp_reg();
                                    if let Some(default_expr) = &field.default {
                                        // 有默认值：生成默认值表达式 IR
                                        self.generate_expr_ir(
                                            default_expr,
                                            default_reg,
                                            instructions,
                                            constants,
                                        )?;
                                    } else {
                                        // 无默认值：用零值填充（语义检查阶段应已报错）
                                        instructions.push(Instruction::Load {
                                            dst: Operand::Local(default_reg),
                                            src: Operand::Const(ConstValue::Int(0)),
                                        });
                                    }
                                    arg_regs.push(Operand::Local(default_reg));
                                }
                            }
                        }
                    }

                    // 命名空间解析：将短名称解析为完整名称
                    // 例如：print -> std.io.print (当 print 是通过 use std.io.{print} 导入时)
                    // 检查是否是闭包调用（函数表达式不是简单的变量名）
                    let is_closure_call = !matches!(func.as_ref(), Expr::Var(_, _));

                    if is_closure_call {
                        // 闭包调用：先加载函数值，然后使用 CallDyn
                        let func_reg = self.next_temp_reg();
                        self.generate_expr_ir(func, func_reg, instructions, constants)?;

                        instructions.push(Instruction::CallDyn {
                            dst: Some(Operand::Local(result_reg)),
                            func: Operand::Local(func_reg),
                            args: arg_regs,
                        });
                    } else {
                        // 普通函数调用
                        let func_operand = if let Expr::Var(name, _) = func.as_ref() {
                            // 尝试将短名称解析为完整名称
                            let resolved_name = if is_native_name(name) {
                                // 已经是完整的 native 函数名
                                name.clone()
                            } else if let Some(qualified) = SHORT_TO_QUALIFIED.get(name) {
                                // 通过短名称映射获取完整名称
                                qualified.clone()
                            } else {
                                // 未知函数，保持原名
                                name.clone()
                            };
                            Operand::Const(ConstValue::String(resolved_name))
                        } else {
                            Operand::Const(ConstValue::Int(0))
                        };

                        instructions.push(Instruction::Call {
                            dst: Some(Operand::Local(result_reg)),
                            func: func_operand,
                            args: arg_regs,
                        });
                    }
                }
            }
            Expr::FieldAccess { expr, field, .. } => {
                // 首先检查是否是模块变量的字段访问（如 io.println）
                // io 是通过 use std.{io} 导入的模块变量
                if let Expr::Var(module_name, _) = expr.as_ref() {
                    if let Some(full_path) = resolve_module_access(module_name, field) {
                        // 模块变量方法调用：生成函数调用
                        // 例如：io.println -> Call("std.io.println", [args])
                        // 这里我们处理的是非调用场景的字段访问（如 io.println 作为值）
                        // 生成零参数调用
                        instructions.push(Instruction::Call {
                            dst: Some(Operand::Local(result_reg)),
                            func: Operand::Const(ConstValue::String(full_path)),
                            args: vec![],
                        });
                    } else {
                        // 普通字段访问
                        let obj_reg = self.next_temp_reg();
                        self.generate_expr_ir(expr, obj_reg, instructions, constants)?;
                        let field_index = self.resolve_field_index(expr, field).unwrap_or(0);
                        instructions.push(Instruction::LoadField {
                            dst: Operand::Local(result_reg),
                            src: Operand::Local(obj_reg),
                            field: field_index,
                        });
                    }
                } else {
                    // 提取完整的命名空间路径（如 std.math.PI）
                    let full_path = extract_namespace_path(expr, field);

                    // 检查是否是命名空间常量访问
                    let is_native_constant = is_native_name(&full_path);

                    if is_native_constant {
                        // 命名空间常量访问：生成零参数函数调用
                        instructions.push(Instruction::Call {
                            dst: Some(Operand::Local(result_reg)),
                            func: Operand::Const(ConstValue::String(full_path)),
                            args: vec![],
                        });
                    } else {
                        // 普通字段访问
                        let obj_reg = self.next_temp_reg();
                        self.generate_expr_ir(expr, obj_reg, instructions, constants)?;
                        let field_index = self.resolve_field_index(expr, field).unwrap_or(0);
                        instructions.push(Instruction::LoadField {
                            dst: Operand::Local(result_reg),
                            src: Operand::Local(obj_reg),
                            field: field_index,
                        });
                    }
                }
            }
            Expr::List(elements, span) => {
                // 列表字面量：先创建空列表，再按索引写入元素
                instructions.push(Instruction::AllocArray {
                    dst: Operand::Local(result_reg),
                    size: Operand::Const(ConstValue::Int(elements.len() as i128)),
                    elem_size: Operand::Const(ConstValue::Int(1)),
                });

                // 列表构建需要多次 StoreIndex，因此需要标记为可变
                self.current_mut_locals.insert(result_reg);

                for (idx, element) in elements.iter().enumerate() {
                    let element_reg = self.next_temp_reg();
                    self.generate_expr_ir(element, element_reg, instructions, constants)?;

                    let index_reg = self.next_temp_reg();
                    instructions.push(Instruction::Load {
                        dst: Operand::Local(index_reg),
                        src: Operand::Const(ConstValue::Int(idx as i128)),
                    });

                    instructions.push(Instruction::StoreIndex {
                        dst: Operand::Local(result_reg),
                        index: Operand::Local(index_reg),
                        src: Operand::Local(element_reg),
                        span: *span,
                    });
                }
            }
            Expr::Index { expr, index, .. } => {
                let src_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, src_reg, instructions, constants)?;

                let index_reg = self.next_temp_reg();
                self.generate_expr_ir(index, index_reg, instructions, constants)?;

                instructions.push(Instruction::LoadIndex {
                    dst: Operand::Local(result_reg),
                    src: Operand::Local(src_reg),
                    index: Operand::Local(index_reg),
                });
            }
            Expr::Return(expr, _) => {
                // 生成返回指令
                if let Some(e) = expr {
                    self.generate_expr_ir(e, result_reg, instructions, constants)?;
                    instructions.push(Instruction::Ret(Some(Operand::Local(result_reg))));
                } else {
                    instructions.push(Instruction::Ret(None));
                }
            }
            Expr::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                span: _,
            } => {
                // 重新实现 if 表达式，使用更简单的方法
                self.generate_if_expr_ir(
                    condition,
                    then_branch,
                    elif_branches,
                    else_branch.as_deref(),
                    result_reg,
                    instructions,
                    constants,
                )?;
            }
            Expr::While {
                condition,
                body,
                label: _,
                span: _,
            } => {
                self.generate_while_expr_ir(condition, body, result_reg, instructions, constants)?;
            }
            Expr::For {
                var,
                var_mut,
                iterable,
                body,
                label: _,
                span: for_span,
            } => {
                self.generate_for_loop_ir(
                    var,
                    *var_mut,
                    iterable,
                    body,
                    Some(result_reg),
                    *for_span,
                    instructions,
                    constants,
                )?;
            }
            Expr::Ref { expr, span: _ } => {
                // ref 表达式：创建 Arc
                // 生成内部表达式的 IR
                let src_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, src_reg, instructions, constants)?;

                // 生成 ArcNew 指令
                instructions.push(Instruction::ArcNew {
                    dst: Operand::Local(result_reg),
                    src: Operand::Local(src_reg),
                });
            }
            Expr::Unsafe { body, span: _ } => {
                // unsafe 块：生成 UnsafeBlockStart/End 标记
                // 生成 UnsafeBlockStart 指令
                instructions.push(Instruction::UnsafeBlockStart);

                // 生成块内语句的 IR
                self.generate_block_ir(body, instructions, constants)?;

                // 生成 UnsafeBlockEnd 指令
                instructions.push(Instruction::UnsafeBlockEnd);

                // unsafe 块作为表达式时返回 0
                instructions.push(Instruction::Load {
                    dst: Operand::Local(result_reg),
                    src: Operand::Const(ConstValue::Int(0)),
                });
            }
            Expr::UnOp { op, expr, span: _ } => {
                // 一元运算符
                match op {
                    ast::UnOp::Deref => {
                        // 解引用：*ptr
                        // 生成指针表达式的 IR
                        let src_reg = self.next_temp_reg();
                        self.generate_expr_ir(expr, src_reg, instructions, constants)?;

                        // 生成 PtrDeref 指令
                        instructions.push(Instruction::PtrDeref {
                            dst: Operand::Local(result_reg),
                            src: Operand::Local(src_reg),
                        });
                    }
                    ast::UnOp::Neg => {
                        // 负号：-x
                        let src_reg = self.next_temp_reg();
                        self.generate_expr_ir(expr, src_reg, instructions, constants)?;
                        instructions.push(Instruction::Neg {
                            dst: Operand::Local(result_reg),
                            src: Operand::Local(src_reg),
                        });
                    }
                    ast::UnOp::Pos => {
                        // 正号：+x（无操作）
                        self.generate_expr_ir(expr, result_reg, instructions, constants)?;
                    }
                    ast::UnOp::Not => {
                        // 逻辑非：!x
                        let src_reg = self.next_temp_reg();
                        self.generate_expr_ir(expr, src_reg, instructions, constants)?;
                        // 生成一个简单的取反操作
                        instructions.push(Instruction::Load {
                            dst: Operand::Local(result_reg),
                            src: Operand::Const(ConstValue::Int(0)),
                        });
                    }
                }
            }
            Expr::Lambda {
                params,
                body,
                span: _,
            } => {
                // Lambda 表达式 IR 生成
                // 例如: (x, y) => x + y

                // 1. 生成唯一的闭包函数名
                let closure_name = format!("closure_{}", self.closure_counter);
                self.closure_counter += 1;

                // 2. 获取闭包的返回类型（简化处理：使用 Void）
                // TODO: 可以通过类型检查结果获取更精确的返回类型
                let return_type = MonoType::Void;

                // 3. 为闭包参数分配寄存器索引
                let _param_regs: Vec<usize> = (0..params.len()).collect();

                // 4. 生成闭包函数体 IR
                // 类似于 generate_function_ir 的逻辑，但针对 Lambda
                let closure_body =
                    self.generate_lambda_body_ir(params, body.as_ref(), constants)?;

                // 5. 创建闭包函数 IR
                let param_types: Vec<MonoType> = params
                    .iter()
                    .filter_map(|p| p.ty.clone())
                    .map(|t| t.into())
                    .collect();

                let closure_func = FunctionIR {
                    name: closure_name.clone(),
                    params: param_types,
                    return_type,
                    is_async: false,
                    locals: closure_body.locals.clone(),
                    blocks: vec![BasicBlock {
                        label: 0,
                        instructions: closure_body.instructions,
                        successors: Vec::new(),
                    }],
                    entry: 0,
                };

                // 6. 将闭包函数添加到嵌套函数列表
                self.nested_functions.push(closure_func);

                // 7. 保存闭包函数的可变局部变量信息
                if !closure_body.mut_locals.is_empty() {
                    self.module_mut_locals
                        .insert(closure_name.clone(), closure_body.mut_locals);
                }

                // 8. 创建 MakeClosure 指令
                // env 为空，因为当前不处理捕获变量（后续可扩展）
                // 使用闭包函数名而不是索引
                instructions.push(Instruction::MakeClosure {
                    dst: Operand::Local(result_reg),
                    func: closure_name,
                    env: Vec::new(),
                });
            }
            _ => {
                // 默认返回 0
                instructions.push(Instruction::Load {
                    dst: Operand::Local(result_reg),
                    src: Operand::Const(ConstValue::Int(0)),
                });
            }
        }
        Ok(())
    }
}

/// IR 生成错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrGenError {
    /// 未实现的表达式类型
    UnimplementedExpr { expr_type: String, span: Span },

    /// 未实现的语句类型
    UnimplementedStmt { stmt_type: String, span: Span },

    /// 无效的操作数
    InvalidOperand { span: Span },

    /// 内部错误
    InternalError { message: String, span: Span },

    /// 不支持的迭代器类型
    /// for 循环目前只支持 `start..end` 语法的 range 迭代
    UnsupportedIterator { iter_type: String, span: Span },
}

impl std::fmt::Display for IrGenError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            IrGenError::UnimplementedExpr { expr_type, span: _ } => {
                write!(f, "未实现的表达式类型: {}", expr_type)
            }
            IrGenError::UnimplementedStmt { stmt_type, span: _ } => {
                write!(f, "未实现的语句类型: {}", stmt_type)
            }
            IrGenError::InvalidOperand { span: _ } => write!(f, "无效的操作数"),
            IrGenError::InternalError { message, span: _ } => write!(f, "内部错误: {}", message),
            IrGenError::UnsupportedIterator { iter_type, span: _ } => {
                write!(
                    f,
                    "不支持的迭代器类型: {}。for 循环目前只支持 `start..end` 语法，请使用如 `for i in 0..10` 的写法",
                    iter_type
                )
            }
        }
    }
}

impl std::error::Error for IrGenError {}

/// 从 AST 模块生成 IR
///
/// 这是编译器流程中的关键入口点：
/// 类型检查 → IR 生成 → 代码生成
pub fn generate_ir(
    ast: &crate::frontend::core::parser::ast::Module,
    result: &crate::frontend::typecheck::TypeCheckResult,
) -> Result<crate::middle::ModuleIR, Vec<Diagnostic>> {
    let mut generator = AstToIrGenerator::new_with_type_result(result);
    generator
        .generate_module_ir(ast)
        .map_err(|errors| errors.into_iter().map(convert_ir_gen_error).collect())
}

/// 将 IrGenError 转换为 Diagnostic
fn convert_ir_gen_error(e: IrGenError) -> Diagnostic {
    match e {
        IrGenError::UnimplementedExpr { expr_type, span } => ErrorCodeDefinition::internal_error(
            &format!("Unimplemented expression type: {}", expr_type),
        )
        .at(span)
        .build(),
        IrGenError::UnimplementedStmt { stmt_type, span } => ErrorCodeDefinition::internal_error(
            &format!("Unimplemented statement type: {}", stmt_type),
        )
        .at(span)
        .build(),
        IrGenError::InvalidOperand { span } => {
            ErrorCodeDefinition::internal_error("Invalid operand")
                .at(span)
                .build()
        }
        IrGenError::InternalError { message, span } => {
            ErrorCodeDefinition::internal_error(&message)
                .at(span)
                .build()
        }
        IrGenError::UnsupportedIterator { iter_type, span } => {
            ErrorCodeDefinition::unsupported_operation("iterate", &iter_type)
                .at(span)
                .build()
        }
    }
}
