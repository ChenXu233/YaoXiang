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
use crate::frontend::module::resolver::Resolver;
use crate::frontend::module::symbol::SymbolTable;
use crate::frontend::module::ExportKind;
use crate::frontend::core::typecheck::{MonoType, PolyType, TypeCheckResult};
use crate::middle::core::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, ModuleIR, Operand,
};
use crate::tlog;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::i18n::MSG;
use crate::util::span::Span;
use std::collections::HashMap;

/// 把命名空间表达式拆成「头 + 字段段列表」，供 [`Resolver`] 解析。
///
/// `std.io.println`（FieldAccess 链）→ `("std", ["io", "println"])`；
/// 非 Var 结尾的表达式（如方法调用结果再取字段）返回 None。
/// 纯 AST 机械操作；「头是不是命名空间」「拼成什么限定名」的语义归 [`Resolver`]。
fn flatten_namespace(expr: &ast::Expr) -> Option<(&str, Vec<&str>)> {
    let mut segs: Vec<&str> = Vec::new();
    let mut cur = expr;
    loop {
        match cur {
            ast::Expr::Var(name, _) => {
                segs.push(name.as_str());
                segs.reverse();
                let head = segs[0];
                return Some((head, segs[1..].to_vec()));
            }
            ast::Expr::FieldAccess { expr, field, .. } => {
                segs.push(field.as_str());
                cur = expr;
            }
            _ => return None,
        }
    }
}

/// 检查类型是否实现了 Stringable 接口（即有 to_string 方法）
/// 用于 print/println 的零开销分发
fn type_implements_stringable(mono_type: &MonoType) -> bool {
    match mono_type {
        // 具体类型：检查方法表中是否有 to_string
        MonoType::Struct(struct_type) => struct_type.methods.contains_key("to_string"),
        // 基础类型默认都有字符串表示
        MonoType::String
        | MonoType::Int(_)
        | MonoType::Float(_)
        | MonoType::Bool
        | MonoType::Char => true,
        // 其他类型使用兜底实现
        _ => false,
    }
}

/// 获取类型的字符串表示（用于兜底实现）
fn get_type_fallback_string(mono_type: &MonoType) -> String {
    match mono_type {
        MonoType::Void => "void".to_string(),
        MonoType::Bool => "bool".to_string(),
        MonoType::Int(_) => "int".to_string(),
        MonoType::Float(_) => "float".to_string(),
        MonoType::Char => "char".to_string(),
        MonoType::String => "string".to_string(),
        MonoType::Bytes => "bytes".to_string(),
        MonoType::Struct(s) => s.name.clone(),
        MonoType::Enum(e) => e.name.clone(),
        MonoType::Tuple(_) => "tuple".to_string(),
        MonoType::List(_) => "list".to_string(),
        MonoType::Dict(_, _) => "dict".to_string(),
        MonoType::Set(_) => "set".to_string(),
        MonoType::Fn { .. } => "function".to_string(),
        MonoType::TypeRef(name) => name.clone(),
        // 其他类型使用默认名称
        _ => "unknown".to_string(),
    }
}

/// 符号表条目
#[derive(Debug, Clone)]
struct SymbolEntry {
    local_idx: usize,
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
    /// 局部变量类型追踪（用于错误消息中显示实际类型）
    local_var_types: HashMap<String, String>,
    /// FFI 库绑定
    ffi_libs: Vec<crate::middle::core::ir::FfiLibBinding>,
    /// FFI 绑定 — 不透明类型或外部函数
    ffi_bindings: Vec<crate::middle::core::ir::FfiBinding>,
    /// 下一个库 ID
    next_lib_id: usize,
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
    /// 约束变量的具体类型映射（接口直接赋值优化）
    /// 当 `d: Drawable = Circle(1)` 时，记录 d -> "Circle"（具体类型名）
    /// 用于方法调用时选择直接调用而非 vtable 查找
    constraint_var_concrete_types: HashMap<String, String>,
    /// RFC-004: 匿名函数绑定生成的独立 FunctionIR 列表
    anon_function_irs: Vec<FunctionIR>,
    /// NLL 精确释放计划（所有权检查器产出）
    release_plan: HashMap<Span, Vec<String>>,
    /// 待捕获的环境变量（由 spawn for 等设置，供下一个 Expr::Lambda 使用）
    /// 在生成闭包函数体时，这些变量的当前寄存器值会被捕获到闭包环境中。
    pending_env_vars: Vec<Operand>,
    /// #254：待捕获变量名（与 pending_env_vars 一一对应，供闭包体 Var 解析）
    pending_env_names: Vec<String>,
    /// #254：当前闭包体的捕获表（变量名 → env 槽位索引）。
    /// 生成闭包体期间设置，闭包体内 Var 命中则生成 LoadUpvalue。
    closure_captures: HashMap<String, usize>,
    /// RFC-029 #243：用户模块命名空间变量（别名 → 模块限定键）。
    /// 由 `use lib` / `use lib as l` 等整体导入填充，使 `lib.helper()` 在 IR 生成
    /// 阶段被识别为命名空间调用并解析为限定名 `lib.helper`（与 `qualify_module_ir`
    /// 产出的函数定义名天然契合）。std 命名空间走原有 `is_std_submodule` 分支，不在此表。
    user_namespaces: HashMap<String, String>,
    /// 本文件 use 导入的本地名 → 限定名（含 #245 条目别名）。
    /// 生成期解析调用名时优先于 registry 的全局短名表（文件级精确）。
    use_aliases: HashMap<String, String>,
    /// 本文件持有的模块注册表（std + 用户模块），取代生成期多处 `with_std()` 重建。
    registry: ModuleRegistry,
    /// 本文件的模块限定键（多文件模式 Some → 启用限定；单文件 None → 不限定）。
    module_key: Option<String>,
}

/// 绑定信息（用于 IR 生成阶段的方法调用转发）
///
/// 按 RFC-004 设计：记录方法绑定到哪个原始函数的哪些参数位置
#[derive(Debug, Clone)]
struct BindingInfo {
    /// 原始函数名
    function: String,
    /// 绑定位置列表（调用者 obj 填充到这些位置）
    positions: Vec<i64>,
}

/// Lambda 函数体 IR 结果
struct LambdaBodyIR {
    instructions: Vec<Instruction>,
    locals: Vec<MonoType>,
}

/// 一层 curry 签名
///
/// 由 `split_curry` 从嵌套的 `Type::Fn` 拆出。
/// `params` 是这层的参数（带名字），从拍平的 `signature_params` 切出。
/// `return_type` 是这层的返回类型：还有下一层时是 `Type::Fn`，最后一层是值类型。
/// 一层 curry 签名
struct CurryLayer {
    params: Vec<ast::Param>,
    return_type: ast::Type,
}

impl AstToIrGenerator {
    /// 创建新的 IR 生成器（带类型信息）
    pub fn new_with_type_result(
        type_result: &TypeCheckResult,
        registry: ModuleRegistry,
        module_key: Option<String>,
    ) -> Self {
        Self {
            symbols: vec![HashMap::new()],
            type_result: Some(Box::new(type_result.clone())),
            next_temp: 0,
            local_var_types: HashMap::new(),
            ffi_libs: Vec::new(),
            ffi_bindings: Vec::new(),
            next_lib_id: 0,
            struct_definitions: HashMap::new(),
            type_bindings: HashMap::new(),
            nested_functions: Vec::new(),
            closure_counter: 0,
            global_vars: Vec::new(),
            constraint_var_concrete_types: HashMap::new(),
            anon_function_irs: Vec::new(),
            release_plan: type_result.release_plan.drops.clone(),
            pending_env_vars: Vec::new(),
            pending_env_names: Vec::new(),
            closure_captures: HashMap::new(),
            user_namespaces: HashMap::new(),
            use_aliases: HashMap::new(),
            registry,
            module_key,
        }
    }

    /// 当前文件的模块限定键（单文件模式默认 `main`，与 TypeChecker 一致）。
    fn module_key_or_main(&self) -> &str {
        self.module_key.as_deref().unwrap_or("main")
    }

    /// 构造当前文件上下文的名字解析器（名字 → 限定名 → DefId 的唯一所有者）。
    fn resolver(&self) -> Resolver<'_> {
        Resolver::new(
            self.registry.symbols(),
            &self.registry,
            self.module_key_or_main(),
            &self.user_namespaces,
        )
    }

    /// `recv` 是否是命名空间调用的接收者（头为 std / std 子模块 / 用户模块别名）。
    /// 机械扁平化后委托 [`Resolver::is_namespace`]——语义归 Resolver。
    fn is_namespace_receiver(
        &self,
        recv: &ast::Expr,
    ) -> bool {
        flatten_namespace(recv)
            .map(|(head, _)| self.resolver().is_namespace(head))
            .unwrap_or(false)
    }

    /// 解析 `recv.field` 的限定名（如 `std.io.println`），经 [`Resolver`] 统一解析。
    /// 非 Var 结尾的接收者退回 `field` 本身（与既有行为一致）。
    fn resolve_field_path(
        &self,
        recv: &ast::Expr,
        field: &str,
    ) -> String {
        match flatten_namespace(recv) {
            Some((head, mut fields)) => {
                fields.push(field);
                self.resolver().resolve_namespace(head, &fields)
            }
            None => field.to_string(),
        }
    }

    /// 拆分 curry 签名为多层
    ///
    /// 输入：`type_annotation` 是 `Type::Fn` 的嵌套结构（如 `(N: Int) -> (n: N) -> Int`）
    /// 输入：`signature_params` 是拍平的所有 curry 参数（如 `[N: Int, n: N]`）
    /// 输出：按层切分的 `CurryLayer` 列表
    ///
    /// 非 curry 函数（如 `(a: Int, b: Int) -> Int`）返回 1 个 layer，
    /// 其 `return_type` 不是 `Type::Fn`。
    #[allow(clippy::while_let_loop)]
    fn split_curry(
        type_ann: &ast::Type,
        signature_params: &[ast::Param],
    ) -> Vec<CurryLayer> {
        let mut layers = Vec::new();
        let mut params_iter = signature_params.iter().cloned().peekable();
        let mut current_type = type_ann.clone();

        loop {
            match current_type {
                ast::Type::Fn {
                    params: type_params,
                    return_type,
                } => {
                    let layer_params: Vec<ast::Param> =
                        (&mut params_iter).take(type_params.len()).collect();
                    let ret_type = *return_type;
                    current_type = ret_type.clone();
                    // 类型参数层（如 `(T: Type)`）是编译期参数：不占运行时参数位，
                    // 该层擦除（不生成运行时函数层）。调用点 T 由类型推断填充（RFC-011）。
                    // ponytail: 仅处理纯类型参数层；类型/值参数混合同层视为值层（罕见，暂不拆）
                    let is_type_layer =
                        !type_params.is_empty() && type_params.iter().all(Self::is_type_param_ann);
                    if !is_type_layer {
                        layers.push(CurryLayer {
                            params: layer_params,
                            return_type: ret_type,
                        });
                    }
                }
                _ => break,
            }
        }
        layers
    }

    /// 该类型注解是否为 `Type`（元类型）——即类型参数，编译期擦除。
    fn is_type_param_ann(ty: &ast::Type) -> bool {
        match ty {
            ast::Type::MetaType { .. } => true,
            ast::Type::Name { name, .. } => name == "Type",
            _ => false,
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

    /// 获取约束变量的具体类型名（如果编译期可确定）
    ///
    /// 用于方法调用时选择直接调用（零开销）而非 vtable 查找
    fn get_constraint_var_concrete_type(
        &self,
        var_name: &str,
    ) -> Option<&String> {
        self.constraint_var_concrete_types.get(var_name)
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
        for fields in self.struct_definitions.values() {
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
                    let mono_type = poly_type.body.clone();
                    return Self::mono_type_to_struct_name(&mono_type);
                }
                // 从 IR 生成器追踪的类型查找
                if let Some(type_name) = self.local_var_types.get(name) {
                    // #266: &mut 令牌穿透——"&mut Point" 取底层 "Point"
                    let base = Self::strip_ref_prefix(type_name);
                    if self.struct_definitions.contains_key(base) {
                        return Some(base.to_string());
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
            // #266: &mut 令牌穿透——Ref 递归取 inner（m = &mut q 的类型是
            // Ref { mutable, inner: Point }，方法派发应基于 Point）
            MonoType::Ref { inner, .. } => Self::mono_type_to_struct_name(inner),
            _ => None,
        }
    }

    /// #266: 穿透 `&` / `&mut` 引用前缀取底层类型名
    /// `"&mut Point"` → `"Point"`；`"&Point"` → `"Point"`；无前缀原样返回
    fn strip_ref_prefix(type_name: &str) -> &str {
        let t = type_name.trim();
        if let Some(rest) = t.strip_prefix("&mut ") {
            rest
        } else if let Some(rest) = t.strip_prefix('&') {
            rest.trim_start()
        } else {
            t
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
    ) -> Result<ModuleIR, Vec<Diagnostic>> {
        // RFC-029：用户模块命名空间别名（`use lib` / `use lib as l`）由 typecheck 登记
        // （模块解析归 typecheck 所有），IR 生成直接消费，不再自行从 AST 重新推导。
        if let Some(ref tr) = self.type_result {
            self.user_namespaces = tr.module_namespaces.clone();
        }
        // use 导入别名表（含 #245 条目别名）：生成期解析调用名用。
        self.use_aliases = self.build_import_aliases(module);

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

        // RFC-004: 添加匿名函数绑定生成的 IR 到模块函数列表
        functions.extend(std::mem::take(&mut self.anon_function_irs));

        let mut ir = ModuleIR {
            globals: Vec::new(),
            functions,
            ffi_libs: std::mem::take(&mut self.ffi_libs),
            ffi_bindings: std::mem::take(&mut self.ffi_bindings),
            entry_function: None,
            source_files: Vec::new(),
            function_files: HashMap::new(),
        };
        // RFC-029: 多文件模式下限定本文件的顶层函数名与调用引用。
        // 单文件（module_key 为 None）不限定，保持原有行为。
        if let Some(key) = &self.module_key {
            let aliases = self.build_import_aliases(module);
            Self::qualify_names(&mut ir, key, &aliases);
        }
        self.assign_defs(&mut ir);
        Ok(ir)
    }

    /// Stage 3 尾部 pass：为所有函数分配 DefId 并解析静态引用。
    ///
    /// 名字此时已是最终形态（多文件已限定、单文件保持裸名）：
    /// - intern 规则：多文件直接用限定名；单文件用 `qualify("main", bare)`——与
    ///   `module_key_or_main` 一致，保证 `FunctionIR.def` 与调用侧解析结果同源。
    /// - 方法在单文件首次 intern 时类别退化为 `Function`（多文件已在 register 期以
    ///   `Method` intern，幂等保留）；下游只查 def 不查 kind，无行为差异。
    /// - 泛型函数经 mono 特化后重命名，def 随之失效——mono 路径保持按名分发（现状），
    ///   codegen 对 def 未命中回退按名，两条路径都正确。
    fn assign_defs(
        &self,
        ir: &mut ModuleIR,
    ) {
        let single_file = self.module_key.is_none();
        let mut symbols = self.registry.symbols_mut();
        for func in &mut ir.functions {
            let intern_name = if single_file {
                SymbolTable::qualify("main", &func.name)
            } else {
                func.name.clone()
            };
            let kind = if func.is_type_decl() {
                crate::frontend::module::symbol::DefKind::Type
            } else {
                crate::frontend::module::symbol::DefKind::Function
            };
            func.def = Some(symbols.intern_full(&intern_name, kind));
        }
        // 解析静态引用：先直查（多文件限定名 / std 原生），单文件裸名回退 qualify("main", ·)。
        // 所有函数已 intern 完毕，前向引用在此必然可解。
        let resolve = |name: &str| {
            symbols.def(name).or_else(|| {
                if single_file {
                    symbols.def(&SymbolTable::qualify("main", name))
                } else {
                    None
                }
            })
        };
        for func in &mut ir.functions {
            if func.is_type_decl() {
                continue; // TypeDecl 无指令体
            }
            for block in func.blocks_mut() {
                for instr in &mut block.instructions {
                    match instr {
                        Instruction::Call { func, def, .. }
                        | Instruction::TailCall { func, def, .. } => {
                            if let Operand::Const(ConstValue::String(name)) = func {
                                *def = resolve(name);
                            }
                        }
                        Instruction::MakeClosure { func, def, .. } => {
                            *def = resolve(func);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// RFC-029 限定名：从一个文件的 `use` 语句构建导入别名表（本地短名 → 源模块限定名）。
    ///
    /// 供限定名重写解析跨文件调用目标。std 调用已由 IR 生成限定为 `std.*`，不在此表。
    fn build_import_aliases(
        &self,
        module: &ast::Module,
    ) -> HashMap<String, String> {
        let mut aliases = HashMap::new();
        for stmt in &module.items {
            if let ast::StmtKind::Use {
                path,
                items,
                item_aliases,
                ..
            } = &stmt.kind
            {
                let Some(exports) = self.registry.get_exports(path) else {
                    continue;
                };
                if let Some(names) = items {
                    // 本地名：内联别名（#245）> 原名
                    for (i, name) in names.iter().enumerate() {
                        if let Some(export) = exports.get(name) {
                            // #244：类型也限定——构造调用 `Point(...)` 与方法调用 `Point.get_x`
                            // 都需把本地短名别名到源模块限定名才能解析。
                            if matches!(
                                export.kind,
                                ExportKind::Function | ExportKind::Constant | ExportKind::Type
                            ) {
                                let local_name = item_aliases
                                    .as_ref()
                                    .and_then(|v| v.get(i))
                                    .and_then(|a| a.as_ref())
                                    .unwrap_or(name);
                                aliases.insert(local_name.clone(), export.full_path.clone());
                            }
                        }
                    }
                }
            }
        }
        aliases
    }

    /// RFC-029 限定名：把本文件 IR 里的顶层函数名重写为 `{module_key}.{name}`，
    /// 并把函数体里对这些函数与导入函数的调用/闭包引用一并重写。
    ///
    /// module=record 语义：`a.helper` 与 `b.helper` 是两个 record 的不同字段，本就不冲突；
    /// 解释器函数表是扁平 name→func，故用限定名作键使其共存。限定名统一经
    /// [`SymbolTable::qualify`] 生成——全仓库限定名语法的唯一所有者。
    fn qualify_names(
        ir: &mut ModuleIR,
        module_key: &str,
        aliases: &HashMap<String, String>,
    ) {
        // 先收集本文件的类型名（TypeDecl），供方法派生函数前缀匹配。
        let mut type_names: std::collections::HashSet<String> = std::collections::HashSet::new();
        for func in &ir.functions {
            if func.is_type_decl() {
                type_names.insert(func.name.clone());
            }
        }

        // 本文件要限定的函数：TypeDecl、方法派生（前缀是本地类型）、普通函数。
        let mut rename: HashMap<String, String> = HashMap::new();
        for func in &ir.functions {
            let bare = &func.name;
            if func.is_type_decl() {
                rename.insert(bare.clone(), SymbolTable::qualify(module_key, bare));
            } else if let Some(dot_pos) = bare.find('.') {
                // 带点名字：只有限定点是本地类型时才限定，避免误伤 std 等已限定名。
                let prefix = &bare[..dot_pos];
                if type_names.contains(prefix) {
                    rename.insert(bare.clone(), SymbolTable::qualify(module_key, bare));
                }
            } else {
                rename.insert(bare.clone(), SymbolTable::qualify(module_key, bare));
            }
        }

        // 重写函数定义名。
        for func in &mut ir.functions {
            if let Some(q) = rename.get(&func.name) {
                func.name = q.clone();
            }
        }

        // 重写调用/闭包引用：本文件函数（rename）+ 导入函数（aliases）。
        for func in &mut ir.functions {
            if func.is_type_decl() {
                continue;
            }
            for block in func.blocks_mut() {
                for instr in &mut block.instructions {
                    Self::rewrite_call_names(instr, &rename, aliases);
                }
            }
        }
    }

    /// 重写单条指令里的函数名引用（Call/TailCall 的字符串 func、MakeClosure 的 func）。
    ///
    /// 解析顺序：完全匹配 rename（本文件函数）→ 完全匹配 aliases（导入函数）→
    /// 前缀匹配（方法调用 `Point.get_x` 经 `Point` 的 rename/alias 限定为 `lib.Point.get_x`）。
    fn rewrite_call_names(
        instr: &mut Instruction,
        rename: &HashMap<String, String>,
        aliases: &HashMap<String, String>,
    ) {
        let resolve = |name: &str| -> Option<String> {
            rename
                .get(name)
                .cloned()
                .or_else(|| aliases.get(name).cloned())
                .or_else(|| {
                    let dot_pos = name.find('.')?;
                    let prefix = &name[..dot_pos];
                    let suffix = &name[dot_pos..]; // 含点
                    rename
                        .get(prefix)
                        .or_else(|| aliases.get(prefix))
                        .map(|q| format!("{}{}", q, suffix))
                })
        };
        match instr {
            Instruction::Call { func, .. } | Instruction::TailCall { func, .. } => {
                if let Operand::Const(ConstValue::String(name)) = func {
                    if let Some(q) = resolve(name) {
                        *name = q;
                    }
                }
            }
            Instruction::MakeClosure { func, .. } => {
                if let Some(q) = resolve(func) {
                    *func = q;
                }
            }
            _ => {}
        }
    }

    /// RFC-029 多文件编排：预注册其他文件的类型上下文（结构体字段布局 + 方法绑定），
    /// 使本文件 IR 生成能解析跨文件的字段索引与方法调用重排。
    ///
    /// 仅注册类型上下文，**不生成函数 IR**——各文件的函数与匿名绑定 IR 由
    /// 各自的 `generate_module_ir` 生成，链接后按名字解析。镜像 `generate_stmt_ir`
    /// 中 `TypeDefinition` 的结构体上下文注册逻辑。
    pub fn seed_cross_file_types(
        &mut self,
        modules: &[&ast::Module],
    ) {
        for module in modules {
            for stmt in &module.items {
                if let ast::StmtKind::TypeDefinition {
                    name, definition, ..
                } = &stmt.kind
                {
                    match definition {
                        ast::Type::NamedStruct {
                            name: struct_name,
                            fields,
                            ..
                        } => {
                            self.struct_definitions
                                .insert(struct_name.clone(), fields.clone());
                        }
                        ast::Type::Struct { body } => {
                            let fields: Vec<ast::StructField> = body
                                .iter()
                                .filter_map(|it| {
                                    if let ast::TypeBodyItem::Field(f) = it {
                                        Some(f.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            let bindings: Vec<ast::TypeBodyBinding> = body
                                .iter()
                                .filter_map(|it| {
                                    if let ast::TypeBodyItem::Binding(b) = it {
                                        Some(b.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            self.struct_definitions.insert(name.clone(), fields);
                            self.register_type_bindings(name, &bindings);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// RFC-029 多文件编排：登记其他文件定义的全局变量名，使本文件中对跨文件全局的
    /// 裸名引用解析为 `Call(访问器函数)`，而非误判为未定义（`Load 0`）。
    ///
    /// 不生成访问器函数 IR——那由定义该全局的文件各自的 `generate_module_ir` 生成。
    /// `lookup_global` 只按名字匹配，故 `init_value` 留 `None`。
    pub fn seed_cross_file_globals(
        &mut self,
        globals: &[(String, MonoType)],
    ) {
        for (name, ty) in globals {
            self.global_vars.push((name.clone(), ty.clone(), None));
        }
    }

    /// 生成语句的 IR
    fn generate_stmt_ir(
        &mut self,
        stmt: &ast::Stmt,
        constants: &mut Vec<ConstValue>,
    ) -> Result<Option<FunctionIR>, Diagnostic> {
        match &stmt.kind {
            ast::StmtKind::TypeDefinition {
                name,
                signature_params,
                definition,
                is_pub: _,
            } => {
                use crate::frontend::core::parser::ast::extract_generic_param_names;
                use crate::frontend::core::types::mono::UniverseLevel;

                // 提取泛型参数名
                let generic_params = extract_generic_param_names(signature_params);
                let generic_param_names = if generic_params.is_empty() {
                    None
                } else {
                    Some(generic_params.iter().map(|p| p.name.clone()).collect())
                };

                // 签名：(T: Type) -> Type → params = [MetaType], return_type = MetaType
                let params: Vec<MonoType> = signature_params
                    .iter()
                    .map(|p| {
                        p.ty.as_ref()
                            .map(|t| t.clone().into())
                            .unwrap_or(MonoType::MetaType {
                                universe_level: UniverseLevel::type1(),
                                type_params: Vec::new(),
                            })
                    })
                    .collect();
                let return_type = MonoType::MetaType {
                    universe_level: UniverseLevel::type1(),
                    type_params: Vec::new(),
                };

                // 记录 struct_definitions（字段索引解析仍需要）
                // 同时处理类型绑定和匿名函数 IR 生成
                match definition {
                    ast::Type::NamedStruct {
                        name: struct_name,
                        fields,
                        ..
                    } => {
                        self.struct_definitions
                            .insert(struct_name.clone(), fields.clone());
                    }
                    ast::Type::Struct { body } => {
                        let fields: Vec<ast::StructField> = body
                            .iter()
                            .filter_map(|it| {
                                if let ast::TypeBodyItem::Field(f) = it {
                                    Some(f.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        let bindings: Vec<ast::TypeBodyBinding> = body
                            .iter()
                            .filter_map(|it| {
                                if let ast::TypeBodyItem::Binding(b) = it {
                                    Some(b.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        self.struct_definitions.insert(name.clone(), fields);
                        // 记录绑定信息（用于方法调用时的参数重排，RFC-004）
                        self.register_type_bindings(name, &bindings);
                        // RFC-004: 为匿名函数绑定生成独立的 FunctionIR
                        for binding in &bindings {
                            if let ast::BindingKind::Anonymous {
                                params: anon_params,
                                return_type: anon_ret,
                                positions: _,
                                body,
                            } = &binding.kind
                            {
                                let anon_func_name = format!("{}.__anon_{}", name, binding.name);
                                match self.generate_anon_binding_ir(
                                    &anon_func_name,
                                    anon_params,
                                    anon_ret,
                                    body,
                                    constants,
                                ) {
                                    Ok(Some(func_ir)) => {
                                        self.anon_function_irs.push(func_ir);
                                    }
                                    Ok(None) => {}
                                    Err(e) => return Err(e),
                                }
                            }
                        }
                    }
                    _ => {}
                }

                Ok(Some(FunctionIR {
                    def: None, // 由 generate_module_ir 尾部 assign_defs 填充
                    name: name.clone(),
                    params,
                    return_type,
                    generic_params: generic_param_names,
                    body: FunctionBody::TypeDecl {
                        definition: definition.clone(),
                    },
                }))
            }
            ast::StmtKind::Assign {
                target,
                type_annotation,
                signature_params,
                value,
                ..
            } => {
                use crate::frontend::core::parser::ast::Expr;
                let (name, type_name) = match target.as_ref() {
                    Expr::Var(n, _) => (n.clone(), None),
                    Expr::FieldAccess { expr, field, .. } => {
                        if let Expr::Var(tn, _) = expr.as_ref() {
                            (field.clone(), Some(tn.clone()))
                        } else {
                            (field.clone(), None)
                        }
                    }
                    _ => return Ok(None),
                };
                let (params, body): (Vec<_>, Vec<_>) = match value {
                    Some(v) => v.callable_parts(),
                    None => (Vec::new(), Vec::new()),
                };
                let generic_params =
                    crate::frontend::core::parser::ast::extract_generic_param_names(
                        signature_params,
                    );
                if let Some(type_name) = type_name {
                    // MethodBind
                    self.generate_method_ir(
                        &type_name,
                        &name,
                        type_annotation.as_ref().unwrap(),
                        &params,
                        &body,
                        constants,
                    )
                } else if !params.is_empty() || !body.is_empty() {
                    // Fn: 普通函数
                    let generic_param_names = if generic_params.is_empty() {
                        None
                    } else {
                        Some(generic_params.iter().map(|p| p.name.clone()).collect())
                    };
                    // curry 分流：如果 return_type 是 Type::Fn，走 curry 生成路径
                    // 注意：必须传 signature_params（所有层参数），而非 params（仅 Lambda 参数）
                    if let Some(ast::Type::Fn { return_type, .. }) = type_annotation.as_ref() {
                        if matches!(return_type.as_ref(), ast::Type::Fn { .. }) {
                            self.generate_curry_function_ir(
                                &name,
                                type_annotation.as_ref().unwrap(),
                                signature_params,
                                &body,
                                constants,
                                generic_param_names,
                            )
                        } else {
                            self.generate_function_ir(
                                &name,
                                type_annotation.as_ref(),
                                &params,
                                &body,
                                constants,
                                generic_param_names,
                            )
                        }
                    } else {
                        self.generate_function_ir(
                            &name,
                            type_annotation.as_ref(),
                            &params,
                            &body,
                            constants,
                            generic_param_names,
                        )
                    }
                } else if name == "main" {
                    // 空 main（`main = {}`）：入口点绑定，生成空函数而非全局变量。
                    // #271 #2：空块不是"折叠不到的初始化"，硬错误会误伤合法空入口。
                    self.generate_function_ir(
                        &name,
                        type_annotation.as_ref(),
                        &params,
                        &body,
                        constants,
                        None,
                    )
                } else {
                    // 全局变量
                    self.generate_global_var_ir(
                        &name,
                        type_annotation.as_ref(),
                        value.as_ref().map(|v| v.as_ref()),
                    )
                }
            }
            // 导入语句：解析在独立 pass 完成（build_import_aliases），不生成运行时代码
            ast::StmtKind::Use { .. } => Ok(None),
            _ => {
                // 顶层只允许定义（绑定/类型/导入）；可执行语句必须在函数体内。
                // 禁止静默丢弃（#251 同类：表达式级兑底曾静默归零）
                Err(ErrorCodeDefinition::ir_internal_error(&format!(
                    "unhandled top-level statement in IR generation: {:?}",
                    std::mem::discriminant(&stmt.kind)
                ))
                .at(stmt.span)
                .build())
            }
        }
    }

    /// 生成方法 IR
    #[allow(clippy::too_many_arguments)]
    fn generate_method_ir(
        &mut self,
        type_name: &str,
        method_name: &str,
        method_type: &ast::Type,
        params: &[ast::Param],
        body: &[ast::Stmt],
        constants: &mut Vec<ConstValue>,
    ) -> Result<Option<FunctionIR>, Diagnostic> {
        // 命名空间机制：方法函数名 = Type.method
        // 例如：Point.get_x 生成函数名 "Point.get_x"
        // 调用时：p.get_x() -> Point.get_x(p)
        let func_name = format!("{}.{}", type_name, method_name);

        // 注册方法到 type_bindings，使方法调用脱糖能找到绑定
        let binding_entry = ast::TypeBodyBinding {
            name: method_name.to_string(),
            kind: ast::BindingKind::DefaultExternal {
                function: func_name.clone(),
            },
        };
        self.register_type_bindings(type_name, &[binding_entry]);

        // 解析返回类型
        let return_type = if let ast::Type::Fn { return_type, .. } = method_type {
            (**return_type).clone().into()
        } else {
            // 非函数类型，报错
            return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                "Method {} is not a function type",
                method_name
            ))
            .build());
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

        // 记录局部变量起始位置（在参数之后）
        let local_var_start = params.len();
        self.next_temp = local_var_start;

        // 生成指令序列
        let mut instructions = Vec::new();

        // 生成语句 IR
        for stmt in body {
            self.generate_local_stmt_ir(stmt, &mut instructions, constants)?;
        }
        instructions.push(Instruction::Ret(None));

        // 退出作用域
        self.exit_scope();

        // 分配局部变量类型（简化：与参数相同）
        let locals_types = param_types.clone();

        // 构建函数 IR
        let func_ir = FunctionIR {
            def: None, // 由 generate_module_ir 尾部 assign_defs 填充
            name: func_name.clone(),
            params: param_types.clone(),
            return_type,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions,
                    successors: Vec::new(),
                }],
                entry: 0,
                locals: locals_types,
            },
        };

        Ok(Some(func_ir))
    }

    /// 生成函数 IR
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn generate_function_ir(
        &mut self,
        name: &str,
        type_annotation: Option<&ast::Type>,
        params: &[ast::Param],
        body: &[ast::Stmt],
        constants: &mut Vec<ConstValue>,
        generic_params: Option<Vec<String>>,
    ) -> Result<Option<FunctionIR>, Diagnostic> {
        // 检测 native("symbol") 模式：函数体为空语句 + Native("...") 表达式
        // 检测 native("symbol") 模式：函数体为空语句 + Native("...") 表达式
        // 形如: my_add: (a: Int, b: Int) -> Int = Native("my_add")
        //
        // 通过 name resolution 检测，不再硬编码 Var("Native") 字符串匹配。
        // Native 是 std.ffi 模块中真实存在的函数，名称通过 ModuleRegistry 解析。
        if body.is_empty() {
            // 空函数体，无法检测 native 模式
        }

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

            self.register_local(&param.name, i);
        }

        // 记录局部变量起始位置（在参数之后）
        let local_var_start = params.len();
        self.next_temp = local_var_start;

        // 检查函数体是否为 FFI ExternRef 绑定
        if let Some(ConstValue::ExternRef {
            mechanism,
            lib,
            symbol,
        }) = self.try_eval_body_as_extern_ref(body)
        {
            let lib_id = self.get_or_create_lib_id(&mechanism, &lib);
            self.ffi_bindings
                .push(crate::middle::core::ir::FfiBinding::FuncBinding {
                    func_name: name.to_string(),
                    lib_id,
                    symbol: symbol.clone(),
                });
            return Ok(None);
        }

        // 生成语句 IR
        for stmt in body {
            tlog!(
                debug,
                MSG::IrGenBeforeProcessStmt,
                &self.symbols.len().to_string()
            );
            self.generate_local_stmt_ir(stmt, &mut instructions, constants)?;
            // NLL Release: 在语句边界插入 Drop 指令
            if let Some(vars) = self.release_plan.get(&stmt.span) {
                for var in vars {
                    if let Some(local_idx) = self.lookup_local(var) {
                        instructions.push(Instruction::Drop(Operand::Local(local_idx)));
                    }
                }
            }
            tlog!(
                debug,
                MSG::IrGenAfterProcessStmt,
                &self.symbols.len().to_string()
            );
        }
        instructions.push(Instruction::Ret(None));

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
            return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                "too many locals allocated in function '{}': {}",
                name, total_locals
            ))
            .build());
        }
        let locals_types: Vec<MonoType> = (0..total_locals)
            .map(|_| MonoType::Int(64)) // 简化：所有局部变量默认为 Int64
            .collect();

        // 构建函数 IR
        let func_ir = FunctionIR {
            def: None, // 由 generate_module_ir 尾部 assign_defs 填充
            name: name.to_string(),
            params: params
                .iter()
                .filter_map(|p| p.ty.clone())
                .map(|t| t.into())
                .collect(),
            return_type,
            generic_params,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions,
                    successors: Vec::new(),
                }],
                entry: 0,
                locals: locals_types,
            },
        };

        Ok(Some(func_ir))
    }

    /// 生成 curry 中间层的 FunctionIR
    ///
    /// 中间层职责：LoadArg 本层参数 → MakeClosure 包装下一层 → Ret(闭包)
    #[allow(clippy::too_many_arguments)]
    fn generate_curry_intermediate_func(
        &mut self,
        func_name: &str,
        layer: &CurryLayer,
        env_count: usize,
        next_func_name: &str,
        constants: &mut Vec<ConstValue>,
        generic_params: Option<Vec<String>>,
    ) -> Result<FunctionIR, Diagnostic> {
        let _ = constants; // 中间层不生成常量
        let mut instructions = Vec::new();

        // 1. 本层参数先加载（用 Arg(env_count + i)）
        for (i, param) in layer.params.iter().enumerate() {
            instructions.push(Instruction::Load {
                dst: Operand::Local(i),
                src: Operand::Arg(env_count + i),
            });
            self.register_local(&param.name, i);
        }

        // 2. 外层参数后加载（用 Arg(i)）
        let env_start = layer.params.len();
        for i in 0..env_count {
            let dst = env_start + i;
            instructions.push(Instruction::Load {
                dst: Operand::Local(dst),
                src: Operand::Arg(i),
            });
        }
        self.next_temp = env_start + env_count;

        // 3. MakeClosure：env = 外层 env + 本层参数
        let mut full_env: Vec<Operand> = (env_start..env_start + env_count)
            .map(Operand::Local)
            .collect();
        for i in 0..layer.params.len() {
            full_env.push(Operand::Local(i));
        }
        let closure_dst = self.next_temp_reg();
        instructions.push(Instruction::MakeClosure {
            dst: Operand::Local(closure_dst),
            func: next_func_name.to_string(),
            env: full_env,
            def: None,
        });

        // 4. Ret(closure)
        instructions.push(Instruction::Ret(Some(Operand::Local(closure_dst))));

        // 5. 构建 FunctionIR
        let param_types: Vec<MonoType> = layer
            .params
            .iter()
            .filter_map(|p| p.ty.clone())
            .map(MonoType::from)
            .collect();
        let return_type: MonoType = layer.return_type.clone().into();
        let total_locals = self.next_temp;
        let locals_types: Vec<MonoType> = vec![MonoType::Int(64); total_locals];

        Ok(FunctionIR {
            def: None, // 由 generate_module_ir 尾部 assign_defs 填充
            name: func_name.to_string(),
            params: param_types,
            return_type,
            generic_params,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions,
                    successors: Vec::new(),
                }],
                entry: 0,
                locals: locals_types,
            },
        })
    }

    /// 生成 curry 最内层的 FunctionIR
    ///
    /// 最内层职责：LoadUpvalue 读所有外层参数 → LoadArg 本层参数 → 执行原 body → Ret
    /// env_param_names：所有外层参数名（按累积顺序）
    #[allow(clippy::too_many_arguments)]
    fn generate_curry_innermost_func(
        &mut self,
        func_name: &str,
        layer: &CurryLayer,
        env_param_names: &[String],
        body: &[ast::Stmt],
        constants: &mut Vec<ConstValue>,
        generic_params: Option<Vec<String>>,
    ) -> Result<FunctionIR, Diagnostic> {
        let mut instructions = Vec::new();
        let env_count = env_param_names.len();

        // 1. 本层参数先加载（用 Arg(env_count + i)，此时 slots[env_count + i] 还未被覆盖）
        for (i, param) in layer.params.iter().enumerate() {
            instructions.push(Instruction::Load {
                dst: Operand::Local(i),
                src: Operand::Arg(env_count + i),
            });
            self.register_local(&param.name, i);
        }

        // 2. 外层参数后加载（用 Arg(i) 读 slots[i]）
        //    注意：必须在参数之后加载，因为 env 写入的 slots 位置可能与 arg 重叠
        let env_start = layer.params.len();
        for (i, name) in env_param_names.iter().enumerate() {
            let dst = env_start + i;
            instructions.push(Instruction::Load {
                dst: Operand::Local(dst),
                src: Operand::Arg(i),
            });
            self.register_local(name, dst);
        }

        // 3. next_temp 起始 = 参数数 + env 数
        self.next_temp = env_start + env_count;

        // 4. 执行原 body（复用 generate_local_stmt_ir）
        for stmt in body {
            self.generate_local_stmt_ir(stmt, &mut instructions, constants)?;
        }
        instructions.push(Instruction::Ret(None));

        let param_types: Vec<MonoType> = layer
            .params
            .iter()
            .filter_map(|p| p.ty.clone())
            .map(MonoType::from)
            .collect();
        let return_type: MonoType = layer.return_type.clone().into();
        let total_locals = self.next_temp;
        let locals_types: Vec<MonoType> = vec![MonoType::Int(64); total_locals];

        Ok(FunctionIR {
            def: None, // 由 generate_module_ir 尾部 assign_defs 填充
            name: func_name.to_string(),
            params: param_types,
            return_type,
            generic_params,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions,
                    successors: Vec::new(),
                }],
                entry: 0,
                locals: locals_types,
            },
        })
    }

    /// 生成 curry 函数的完整 IR 链
    ///
    /// 从外（layer 0）往内（layer N-1）遍历 layers，每层生成一个 FunctionIR。
    /// 最外层用原名 `name`，内层用 `__{name}_l{index}`。
    ///
    /// env_param_names 在循环中累积：进入第 k 层时，它包含 layer 0..k-1 的所有参数名。
    /// 中间层函数从 Arg(0..env_count) 加载外层参数作为 env，从 Arg(env_count..) 加载本层参数。
    /// 最内层函数同理加载后执行原 body。
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn generate_curry_function_ir(
        &mut self,
        name: &str,
        type_annotation: &ast::Type,
        all_params: &[ast::Param],
        body: &[ast::Stmt],
        constants: &mut Vec<ConstValue>,
        generic_params: Option<Vec<String>>,
    ) -> Result<Option<FunctionIR>, Diagnostic> {
        let layers = Self::split_curry(type_annotation, all_params);

        let mut layer_funcs: Vec<FunctionIR> = Vec::new();
        let mut env_param_names: Vec<String> = Vec::new();

        // 保存外层状态，避免污染
        let saved_next_temp = self.next_temp;

        for (i, layer) in layers.iter().enumerate() {
            let is_innermost = i == layers.len() - 1;
            let is_outermost = i == 0;
            let func_name = if i == 0 {
                name.to_string()
            } else {
                format!("__{}_l{}", name, i - 1)
            };

            // 只有最外层函数保留 generic_params（用于单态化）
            let layer_generic_params = if is_outermost {
                generic_params.clone()
            } else {
                None
            };

            // 进入新作用域
            self.enter_scope();
            // 重置本层的临时寄存器（每层独立计数）
            self.next_temp = 0;

            let func = if is_innermost {
                self.generate_curry_innermost_func(
                    &func_name,
                    layer,
                    &env_param_names,
                    body,
                    constants,
                    layer_generic_params,
                )?
            } else {
                let next_name = format!("__{}_l{}", name, i);
                self.generate_curry_intermediate_func(
                    &func_name,
                    layer,
                    env_param_names.len(),
                    &next_name,
                    constants,
                    layer_generic_params,
                )?
            };
            self.exit_scope();

            // 累积本层参数到 env 追踪（给 innermost_func 用）
            for p in &layer.params {
                env_param_names.push(p.name.clone());
            }
            layer_funcs.push(func);
        }

        // 恢复外层状态
        self.next_temp = saved_next_temp;

        // 内层函数加入 nested_functions，最外层返回给调用者
        // 第 0 个是最外层，其余是内层
        let outer = layer_funcs.remove(0);
        self.nested_functions.extend(layer_funcs);
        Ok(Some(outer))
    }
    /// 尝试将表达式求值为编译时常量
    #[allow(clippy::only_used_in_recursion)]
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
                ast::Literal::Void => None,
            },
            // RFC-027: 常量表达式折叠（#261）。
            // 仅折叠纯数值/字符串/布尔的算术与比较，避开除法（除 0）与 Range/Assign。
            // 任一子表达式不能折时整条不折，与 RFC-027 §362「失败回退 Unproven」一致。
            ast::Expr::BinOp {
                op, left, right, ..
            } => {
                use ast::BinOp as B;
                let l = self.eval_const_expr(left)?;
                let r = self.eval_const_expr(right)?;
                // ponytail: 二元折叠的穷举不含 Range/Assign，后者语义不允许初始化于常量
                match (l, r) {
                    (ConstValue::Int(a), ConstValue::Int(b)) => match op {
                        B::Add => Some(ConstValue::Int(a.wrapping_add(b))),
                        B::Sub => Some(ConstValue::Int(a.wrapping_sub(b))),
                        B::Mul => Some(ConstValue::Int(a.wrapping_mul(b))),
                        B::Mod => (b != 0).then(|| ConstValue::Int(a % b)),
                        B::Eq => Some(ConstValue::Bool(a == b)),
                        B::Neq => Some(ConstValue::Bool(a != b)),
                        B::Lt => Some(ConstValue::Bool(a < b)),
                        B::Le => Some(ConstValue::Bool(a <= b)),
                        B::Gt => Some(ConstValue::Bool(a > b)),
                        B::Ge => Some(ConstValue::Bool(a >= b)),
                        B::And => Some(ConstValue::Bool(a != 0 && b != 0)),
                        B::Or => Some(ConstValue::Bool(a != 0 || b != 0)),
                        // #285: 位运算/移位常量折叠（与 const_eval 一致）
                        B::BitAnd => Some(ConstValue::Int(a & b)),
                        B::BitOr => Some(ConstValue::Int(a | b)),
                        B::BitXor => Some(ConstValue::Int(a ^ b)),
                        B::Shl => (0..=63)
                            .contains(&b)
                            .then(|| ConstValue::Int(a.wrapping_shl(b as u32))),
                        B::Shr => (0..=63)
                            .contains(&b)
                            .then(|| ConstValue::Int(a.wrapping_shr(b as u32))),
                        B::Div | B::Range | B::Assign => None,
                    },
                    (ConstValue::Float(a), ConstValue::Float(b)) => match op {
                        B::Add => Some(ConstValue::Float(a + b)),
                        B::Sub => Some(ConstValue::Float(a - b)),
                        B::Mul => Some(ConstValue::Float(a * b)),
                        B::Mod => Some(ConstValue::Float(a % b)),
                        B::Eq => Some(ConstValue::Bool(a == b)),
                        B::Neq => Some(ConstValue::Bool(a != b)),
                        B::Lt => Some(ConstValue::Bool(a < b)),
                        B::Le => Some(ConstValue::Bool(a <= b)),
                        B::Gt => Some(ConstValue::Bool(a > b)),
                        B::Ge => Some(ConstValue::Bool(a >= b)),
                        // 位运算/移位仅限 Int：Float 折叠为 None（运行时同样受限）
                        B::BitAnd | B::BitOr | B::BitXor | B::Shl | B::Shr => None,
                        B::Div | B::Range | B::Assign => None,
                        B::And | B::Or => None,
                    },
                    (ConstValue::String(a), ConstValue::String(b)) => match op {
                        B::Add => Some(ConstValue::String(format!("{a}{b}"))),
                        B::Eq => Some(ConstValue::Bool(a == b)),
                        B::Neq => Some(ConstValue::Bool(a != b)),
                        _ => None,
                    },
                    (ConstValue::Bool(a), ConstValue::Bool(b)) => match op {
                        B::And => Some(ConstValue::Bool(a && b)),
                        B::Or => Some(ConstValue::Bool(a || b)),
                        B::Eq => Some(ConstValue::Bool(a == b)),
                        B::Neq => Some(ConstValue::Bool(a != b)),
                        _ => None,
                    },
                    _ => None,
                }
            }
            ast::Expr::UnOp { op, expr, .. } => {
                use ast::UnOp as U;
                let v = self.eval_const_expr(expr)?;
                match (op, v) {
                    (U::Neg, ConstValue::Int(n)) => Some(ConstValue::Int(n.wrapping_neg())),
                    (U::Neg, ConstValue::Float(f)) => Some(ConstValue::Float(-f)),
                    (U::Not, ConstValue::Bool(b)) => Some(ConstValue::Bool(!b)),
                    _ => None,
                }
            }
            // RFC-012: F-string 常量求值
            ast::Expr::FString { segments, .. } => {
                let mut result = String::new();
                for seg in segments {
                    match seg {
                        ast::FStringSegment::Text(s) => result.push_str(s),
                        ast::FStringSegment::Interpolation { expr, format_spec } => {
                            let val = self.eval_const_expr(expr)?;
                            let val_str = match &val {
                                ConstValue::Int(n) => n.to_string(),
                                ConstValue::Float(f) => f.to_string(),
                                ConstValue::Bool(b) => b.to_string(),
                                ConstValue::String(s) => s.clone(),
                                ConstValue::Char(c) => c.to_string(),
                                ConstValue::Void => String::new(),
                                ConstValue::Bytes(b) => format!("{:?}", b),
                                ConstValue::LibraryRef { .. } | ConstValue::ExternRef { .. } => {
                                    // FFI types cannot be formatted at compile time
                                    return None;
                                }
                            };
                            // 格式化说明符在常量求值中不处理，遇到则退回运行时
                            if format_spec.is_some() {
                                return None;
                            }
                            result.push_str(&val_str);
                        }
                    }
                }
                Some(ConstValue::String(result))
            }
            // 编译期 FFI 求值：Native.c("lib") → ConstValue::LibraryRef
            // lib("sym") → ConstValue::ExternRef
            ast::Expr::Call { func: _, args, .. } => {
                // 通过 type_result 检查调用表达式的推断类型
                if let Some(expr_type) = self.get_expr_mono_type(expr) {
                    match expr_type {
                        MonoType::LibraryRef { mechanism, .. } => {
                            // Native.c("lib") — 需要提取 lib 名字符串
                            if args.len() == 1 {
                                if let Some(lib_str) = Self::extract_string_arg(args) {
                                    return Some(ConstValue::LibraryRef {
                                        mechanism,
                                        lib: lib_str,
                                    });
                                }
                            }
                        }
                        MonoType::ExternRef { mechanism, lib, .. }
                            // lib("sym") — 需要提取 symbol 名字符串
                            if args.len() == 1 => {
                                if let Some(sym_str) = Self::extract_string_arg(args) {
                                    return Some(ConstValue::ExternRef {
                                        mechanism,
                                        lib,
                                        symbol: sym_str,
                                    });
                                }
                            }
                        _ => {}
                    }
                }
                None
            }
            // TODO: 支持更复杂的常量表达式
            _ => None,
        }
    }

    /// 从函数调用的参数列表中提取第一个字符串字面量
    fn extract_string_arg(args: &[ast::Expr]) -> Option<String> {
        args.first().and_then(|arg| match arg {
            ast::Expr::Lit(ast::Literal::String(s), _) => Some(s.clone()),
            _ => None,
        })
    }

    /// 获取或创建 FFI 库绑定 ID
    fn get_or_create_lib_id(
        &mut self,
        mechanism: &str,
        lib_name: &str,
    ) -> usize {
        if let Some(existing) = self
            .ffi_libs
            .iter()
            .find(|l| l.mechanism == mechanism && l.lib_name == lib_name)
        {
            return existing.id;
        }
        let id = self.next_lib_id;
        self.next_lib_id += 1;
        self.ffi_libs.push(crate::middle::core::ir::FfiLibBinding {
            id,
            mechanism: mechanism.to_string(),
            lib_name: lib_name.to_string(),
        });
        id
    }

    /// 检查函数体（Stmt 列表）是否是对 ExternRef 的求值
    fn try_eval_body_as_extern_ref(
        &self,
        body: &[ast::Stmt],
    ) -> Option<ConstValue> {
        if body.len() == 1 {
            match &body[0].kind {
                ast::StmtKind::Expr(expr) | ast::StmtKind::Return(Some(expr)) => {
                    // 直接 eval 表达式（内部会走 eval_const_expr 的 Call 分支检查类型）
                    return self.eval_const_expr(expr);
                }
                _ => {}
            }
        }
        None
    }

    /// 生成全局变量 IR
    fn generate_global_var_ir(
        &mut self,
        name: &str,
        type_annotation: Option<&ast::Type>,
        initializer: Option<&ast::Expr>,
    ) -> Result<Option<FunctionIR>, Diagnostic> {
        let var_type = type_annotation
            .map(|t| (*t).clone().into())
            .unwrap_or(MonoType::Int(64));

        // 尝试从 initializer 提取常量值
        // 这里返回 None 是正常的——表示初始值不是编译期常量表达式
        // （如 main = {} 这样的块表达式），需要运行时求值
        let init_value = initializer.and_then(|expr| self.eval_const_expr(expr));

        // #271 #2：有 init 表达式但常量折叠不到 → 硬错误（不再静默填 0）。
        // 无 init 的绑定保留零初始化语义（C 风格默认值，非静默兜底）。
        if initializer.is_some() && init_value.is_none() {
            return Err(ErrorCodeDefinition::top_level_init_not_const(name)
                .at(Self::get_expr_span(initializer.expect("checked above")))
                .build());
        }

        // 注册到全局变量表（init_value 由 init 表达式的常量折叠得；折叠不到的留 None）
        // ponytail: 该字段由 #261+B PR 删除，本 PR 不动数据结构
        self.global_vars
            .push((name.to_string(), var_type.clone(), init_value.clone()));

        // 零参访问器函数：函数体返回 init_value。
        // 注：原实现曾硬编码 `LoadConst(Int(0)) + Ret`，对常量可折叠的表达式（`1+2+3`）静默丢值（#261）
        // #271 #2：折叠不到的 init 已在上面报 E3007；此处 init_value 非 None，
        // 无 init 的绑定走零初始化（C 语义默认值，非静默兜底）。
        let result_reg = 0;
        let src_operand = Operand::Const(init_value.unwrap_or(ConstValue::Int(0)));
        let instructions = vec![
            Instruction::Load {
                dst: Operand::Local(result_reg),
                src: src_operand,
            },
            Instruction::Ret(Some(Operand::Local(result_reg))),
        ];

        // 为全局变量创建函数
        let func_ir = FunctionIR {
            def: None, // 由 generate_module_ir 尾部 assign_defs 填充
            name: name.to_string(),
            params: Vec::new(),
            return_type: var_type,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions,
                    successors: Vec::new(),
                }],
                entry: 0,
                locals: vec![MonoType::Int(64)], // 分配一个局部变量用于存储结果
            },
        };

        Ok(Some(func_ir))
    }

    /// RFC-004: 为匿名函数绑定生成独立的 FunctionIR
    ///
    /// 匿名函数绑定在类型定义体内以 lambda 形式定义，需要生成独立的函数 IR。
    /// 函数命名格式为 `TypeName.__anon_method_name`。
    fn generate_anon_binding_ir(
        &mut self,
        name: &str,
        params: &[ast::Param],
        return_type: &ast::Type,
        body: &ast::Expr,
        constants: &mut Vec<ConstValue>,
    ) -> Result<Option<FunctionIR>, Diagnostic> {
        // 保存父函数状态
        let saved_next_temp = self.next_temp;

        let mut instructions = Vec::new();

        // 进入匿名函数作用域
        self.enter_scope();

        // 为每个参数生成 LoadArg 指令并注册
        for (i, param) in params.iter().enumerate() {
            instructions.push(Instruction::Load {
                dst: Operand::Local(i),
                src: Operand::Arg(i),
            });

            self.register_local(&param.name, i);
        }

        // 记录局部变量起始位置
        let local_var_start = params.len();
        self.next_temp = local_var_start;

        // 生成表达式体的 IR，并返回结果
        let result_reg = self.next_temp_reg();
        self.generate_expr_ir(body, result_reg, &mut instructions, constants)?;
        instructions.push(Instruction::Ret(Some(Operand::Local(result_reg))));

        // 退出作用域
        self.exit_scope();

        // 计算局部变量总数
        let total_locals = self.next_temp;
        let locals_types: Vec<MonoType> = (0..total_locals).map(|_| MonoType::Int(64)).collect();

        // 恢复父函数状态
        self.next_temp = saved_next_temp;

        // 解析返回类型
        let ret_type: MonoType = return_type.clone().into();

        let func_ir = FunctionIR {
            def: None, // 由 generate_module_ir 尾部 assign_defs 填充
            name: name.to_string(),
            params: params
                .iter()
                .filter_map(|p| p.ty.clone())
                .map(|t| t.into())
                .collect(),
            return_type: ret_type,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions,
                    successors: Vec::new(),
                }],
                entry: 0,
                locals: locals_types,
            },
        };

        Ok(Some(func_ir))
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

        // 合并到已有的绑定映射中，而非覆盖
        let mut binding_map = self.type_bindings.remove(type_name).unwrap_or_default();

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
                BindingKind::DefaultExternal { function } => {
                    // RFC-004: 默认绑定，位置由类型检查器自动推导，此处使用位置 0
                    binding_map.insert(
                        binding.name.clone(),
                        BindingInfo {
                            function: function.clone(),
                            positions: vec![0],
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

    /// 生成局部语句 IR
    #[allow(clippy::only_used_in_recursion)]
    fn generate_local_stmt_ir(
        &mut self,
        stmt: &ast::Stmt,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        match &stmt.kind {
            ast::StmtKind::Expr(expr) => {
                let result_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, result_reg, instructions, constants)?;
            }
            ast::StmtKind::Assign {
                target,
                type_annotation,
                signature_params,
                value,
                span,
                ..
            } => {
                use crate::frontend::core::parser::ast::Expr;
                // 字段赋值：q.x = v / m.x = v（m 是 &mut 令牌）。
                // Struct 运行时值是堆句柄，StoreField 原地写共享对象——
                // 令牌与自动借用天然写回，无需 copy-back（#266）。
                if let Expr::FieldAccess {
                    expr: obj_expr,
                    field,
                    ..
                } = target.as_ref()
                {
                    let field_index =
                        self.resolve_field_index(obj_expr, field).ok_or_else(|| {
                            ErrorCodeDefinition::ir_internal_error(&format!(
                                "无法解析字段索引: '{}'",
                                field
                            ))
                            .at(Self::get_expr_span(obj_expr))
                            .build()
                        })?;
                    let obj_reg = self.next_temp_reg();
                    self.generate_expr_ir(obj_expr, obj_reg, instructions, constants)?;
                    let val_reg = self.next_temp_reg();
                    let value_expr = value.as_ref().ok_or_else(|| {
                        ErrorCodeDefinition::ir_internal_error("字段赋值缺少右侧值")
                            .at(*span)
                            .build()
                    })?;
                    self.generate_expr_ir(value_expr, val_reg, instructions, constants)?;
                    instructions.push(Instruction::StoreField {
                        dst: Operand::Local(obj_reg),
                        field: field_index,
                        src: Operand::Local(val_reg),
                        type_name: None,
                        field_name: Some(field.clone()),
                        span: *span,
                    });
                    return Ok(());
                }
                let name = match target.as_ref() {
                    Expr::Var(n, _) => n.clone(),
                    // 不认识的赋值目标：显式报错，不静默吞掉（#266）
                    _ => {
                        return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                            "不支持的赋值目标: {:?}",
                            target
                        ))
                        .at(Self::get_expr_span(target))
                        .build())
                    }
                };
                let (params, body): (Vec<_>, Vec<_>) = match value {
                    Some(v) => v.callable_parts(),
                    None => (Vec::new(), Vec::new()),
                };
                // 如果有 params/body，是嵌套函数
                if !params.is_empty() || !body.is_empty() {
                    let generic_params =
                        crate::frontend::core::parser::ast::extract_generic_param_names(
                            signature_params,
                        );
                    let generic_param_names = if generic_params.is_empty() {
                        None
                    } else {
                        Some(generic_params.iter().map(|p| p.name.clone()).collect())
                    };
                    // curry 分流：如果 return_type 是 Type::Fn，走 curry 生成路径
                    // 注意：必须传 signature_params（所有层参数），而非 params（仅 Lambda 参数）
                    let func_result =
                        if let Some(ast::Type::Fn { return_type, .. }) = type_annotation.as_ref() {
                            if matches!(return_type.as_ref(), ast::Type::Fn { .. }) {
                                self.generate_curry_function_ir(
                                    &name,
                                    type_annotation.as_ref().unwrap(),
                                    signature_params,
                                    &body,
                                    constants,
                                    generic_param_names,
                                )
                            } else {
                                self.generate_function_ir(
                                    &name,
                                    type_annotation.as_ref(),
                                    &params,
                                    &body,
                                    constants,
                                    generic_param_names,
                                )
                            }
                        } else {
                            self.generate_function_ir(
                                &name,
                                type_annotation.as_ref(),
                                &params,
                                &body,
                                constants,
                                generic_param_names,
                            )
                        };
                    match func_result {
                        Ok(Some(func_ir)) => {
                            self.nested_functions.push(func_ir);
                        }
                        Ok(None) => {}
                        Err(e) => return Err(e),
                    }
                    return Ok(());
                }
                // 普通变量
                let initializer = value.as_ref().map(|v| v.as_ref());
                if let Some(type_ann) = type_annotation {
                    let mono: MonoType = type_ann.clone().into();
                    let type_name = mono.type_name();
                    self.local_var_types.insert(name.clone(), type_name.clone());
                    if mono.is_constraint() {
                        if let Some(init_expr) = initializer {
                            if let Some(concrete_type_name) =
                                self.get_expr_struct_type_name(init_expr)
                            {
                                self.constraint_var_concrete_types
                                    .insert(name.clone(), concrete_type_name);
                            }
                        }
                    }
                } else if let Some(init_expr) = initializer {
                    let inferred = self.get_expr_type_name(init_expr);
                    if inferred != "<unknown>" {
                        self.local_var_types.insert(name.clone(), inferred);
                    }
                }
                let var_idx = if let Some(existing_idx) = self.lookup_local(&name) {
                    existing_idx
                } else {
                    let idx = self.next_temp_reg();
                    self.register_local(&name, idx);
                    idx
                };
                if let Some(expr) = initializer {
                    // 统一走 generate_expr_ir — 把 RHS 值直接放入 var_idx 槽
                    self.generate_expr_ir(expr, var_idx, instructions, constants)?;
                } else {
                    instructions.push(Instruction::Load {
                        dst: Operand::Local(var_idx),
                        src: Operand::Const(ConstValue::Int(0)),
                    });
                }
            }
            ast::StmtKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                span: _,
            } => {
                // 生成 if 语句的 IR
                self.generate_if_stmt_ir(
                    condition,
                    then_branch,
                    else_if_branches,
                    else_branch.as_deref(),
                    instructions,
                    constants,
                )?;
            }
            ast::StmtKind::For {
                var,
                var_span: _,
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
            ast::StmtKind::DestructureAssign { names, rhs, span } => {
                // 优化：如果 RHS 是元组字面量 (1, 2)，直接提取每个元素赋值
                // 否则通过 LoadIndex 索引
                if let ast::Expr::Tuple(elems, _) = rhs.as_ref() {
                    for (i, name) in names.iter().enumerate() {
                        let var_idx = self.next_temp_reg();
                        self.register_local(&name.name, var_idx);

                        if let Some(elem) = elems.get(i) {
                            self.generate_expr_ir(elem, var_idx, instructions, constants)?;
                        }
                    }
                } else {
                    // 非字面量元组：生成 RHS，然后通过 LoadIndex 提取
                    let rhs_reg = self.next_temp_reg();
                    self.generate_expr_ir(rhs, rhs_reg, instructions, constants)?;

                    for (i, name) in names.iter().enumerate() {
                        let var_idx = self.next_temp_reg();
                        self.register_local(&name.name, var_idx);

                        let index_reg = self.next_temp_reg();
                        instructions.push(Instruction::Load {
                            dst: Operand::Local(index_reg),
                            src: Operand::Const(ConstValue::Int(i as i128)),
                        });

                        instructions.push(Instruction::LoadIndex {
                            dst: Operand::Local(var_idx),
                            src: Operand::Local(rhs_reg),
                            index: Operand::Local(index_reg),
                            span: *span,
                        });
                    }
                }
            }
            ast::StmtKind::Return(expr) => match expr {
                Some(e) => {
                    let result_reg = self.next_temp_reg();
                    self.generate_expr_ir(e, result_reg, instructions, constants)?;
                    instructions.push(Instruction::Ret(Some(Operand::Local(result_reg))));
                }
                None => {
                    instructions.push(Instruction::Ret(None));
                }
            },
            // 合法不产生代码的语句：
            // - Use：导入解析在独立 pass 完成，无运行时代码
            // - Error：解析器恢复占位符；存在解析错误时编译已在此前终止
            ast::StmtKind::Use { .. } | ast::StmtKind::Error(_) => {}
            // 块内类型定义不受支持：此前被静默忽略（定义即无效，使用时才在远处报 E1001）
            ast::StmtKind::TypeDefinition { name, .. } => {
                return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                    "type definition `{}` inside a block is not supported; type definitions belong at the top level",
                    name
                ))
                .at(stmt.span)
                .build());
            }
        }
        Ok(())
    }

    /// 生成 if 语句的 IR
    fn generate_if_stmt_ir(
        &mut self,
        condition: &ast::Expr,
        then_branch: &ast::Block,
        else_if_branches: &[(Box<ast::Expr>, Box<ast::Block>)],
        else_branch: Option<&ast::Block>,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 进入新的作用域
        self.enter_scope();

        // 1. 评估条件
        let condition_reg = self.next_temp_reg();
        self.generate_expr_ir(condition, condition_reg, instructions, constants)?;

        // 2. 跳转到下一个分支的占位符 (JmpIfNot to next_branch)
        let jump_to_next_branch_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot(Operand::Local(condition_reg), 0)); // 占位符

        // 3. 生成 then 分支
        self.generate_block_ir(then_branch, None, instructions, constants)?;

        // 4. then 分支结束后，跳转到整个 if 语句的结束 (Jmp to end)
        let mut jump_to_end_indices = Vec::new();
        // 只有当有 else/else if 时才需要跳过它们，否则这里已经是 end
        if !else_if_branches.is_empty() || else_branch.is_some() {
            let idx = instructions.len();
            instructions.push(Instruction::Jmp(0)); // 占位符
            jump_to_end_indices.push(idx);
        }

        // 5. 修复条件跳转 (JmpIfNot)，使其指向 else if 或 else (即当前位置)
        let len = instructions.len();
        if let Instruction::JmpIfNot(_, ref mut target) = instructions[jump_to_next_branch_idx] {
            *target = len;
        }

        // 6. 处理 else if 分支
        for (else_if_condition, else_if_body) in else_if_branches.iter() {
            // 评估 else if 条件
            let else_if_condition_reg = self.next_temp_reg();
            self.generate_expr_ir(
                else_if_condition,
                else_if_condition_reg,
                instructions,
                constants,
            )?;

            // 跳转到下一个分支 (JmpIfNot)
            let jump_to_next_else_if_idx = instructions.len();
            instructions.push(Instruction::JmpIfNot(
                Operand::Local(else_if_condition_reg),
                0,
            ));

            // 生成 else if 分支
            self.generate_block_ir(else_if_body, None, instructions, constants)?;

            // else if 分支结束后跳转到结束
            let idx = instructions.len();
            instructions.push(Instruction::Jmp(0)); // 占位符
            jump_to_end_indices.push(idx);

            // 修复条件跳转
            let len = instructions.len();
            if let Instruction::JmpIfNot(_, ref mut target) = instructions[jump_to_next_else_if_idx]
            {
                *target = len;
            }
        }

        // 7. 生成 else 分支
        if let Some(else_body) = else_branch {
            self.generate_block_ir(else_body, None, instructions, constants)?;
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
        else_if_branches: &[(Box<ast::Expr>, Box<ast::Block>)],
        else_branch: Option<&ast::Block>,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
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
        self.generate_block_ir(then_branch, Some(then_result_reg), instructions, constants)?;
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

        // 6. else if 分支
        for (else_if_condition, else_if_body) in else_if_branches.iter() {
            let else_if_cond_reg = self.next_temp_reg();
            self.generate_expr_ir(else_if_condition, else_if_cond_reg, instructions, constants)?;

            let jump_idx = instructions.len();
            instructions.push(Instruction::JmpIfNot(Operand::Local(else_if_cond_reg), 0));

            let else_if_res = self.next_temp_reg();
            self.generate_block_ir(else_if_body, Some(else_if_res), instructions, constants)?;
            instructions.push(Instruction::Move {
                dst: Operand::Local(result_reg),
                src: Operand::Local(else_if_res),
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
            self.generate_block_ir(else_body, Some(else_res), instructions, constants)?;
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

    /// 生成代码块的 IR
    ///
    /// 当 `result_reg` 为 `Some(reg)` 时，表示块作为表达式使用：
    /// 块中最后一条语句如果是表达式（`StmtKind::Expr`），
    /// 其值直接写入 `reg` 作为块的返回值。
    /// 块中没有 return 也没有尾部表达式时，`reg` 保持默认（Void）。
    ///
    /// 当 `result_reg` 为 `None` 时，块作为语句序列执行，不关心返回值。
    fn generate_block_ir(
        &mut self,
        block: &ast::Block,
        result_reg: Option<usize>,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        // 进入新的作用域
        self.enter_scope();

        let last_idx = block.stmts.len().checked_sub(1);
        for (i, stmt) in block.stmts.iter().enumerate() {
            let is_last = Some(i) == last_idx;
            // 块作为表达式 + 最后一条语句是表达式 → 表达式的值写入 result_reg
            if let (Some(reg), true) = (result_reg, is_last) {
                match &stmt.kind {
                    ast::StmtKind::Expr(expr) => {
                        self.generate_expr_ir(expr, reg, instructions, constants)?;
                        continue;
                    }
                    ast::StmtKind::If {
                        condition,
                        then_branch,
                        else_if_branches,
                        else_branch,
                        ..
                    } => {
                        // 块里的 if 在表达式位置：按 if 表达式生成（值写入 reg）
                        self.generate_if_expr_ir(
                            condition,
                            then_branch,
                            else_if_branches,
                            else_branch.as_deref(),
                            reg,
                            instructions,
                            constants,
                        )?;
                        continue;
                    }
                    _ => {}
                }
            }
            // 其他情况正常生成语句
            self.generate_local_stmt_ir(stmt, instructions, constants)?;
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
    ) -> Result<(), Diagnostic> {
        // Label: condition_check
        let loop_start_idx = instructions.len();

        // Evaluate condition
        let cond_reg = self.next_temp_reg();
        self.generate_expr_ir(condition, cond_reg, instructions, constants)?;

        // Jump to end if false
        let jump_end_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot(Operand::Local(cond_reg), 0)); // Placeholder

        // Body
        self.generate_block_ir(body, None, instructions, constants)?;

        // Jump back to start
        instructions.push(Instruction::Jmp(loop_start_idx));

        // Fix JmpIfNot target
        let end_idx = instructions.len();
        if let Instruction::JmpIfNot(_, ref mut target) = instructions[jump_end_idx] {
            *target = end_idx;
        }

        // While loop returns void
        instructions.push(Instruction::Load {
            dst: Operand::Local(result_reg),
            src: Operand::Const(ConstValue::Void),
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
    ) -> Result<(), Diagnostic> {
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
            self.generate_block_ir(body, None, instructions, constants)?;

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

            // If expression, load void
            if let Some(reg) = result_reg {
                instructions.push(Instruction::Load {
                    dst: Operand::Local(reg),
                    src: Operand::Const(ConstValue::Void),
                });
            }

            Ok(())
        } else if let Some(
            _iter_ty @ (MonoType::List(_)
            | MonoType::Tuple(_)
            | MonoType::Dict(_, _)
            | MonoType::Range { .. }),
        ) = self.get_expr_mono_type(iterable)
        {
            // 使用迭代器协议的 For 循环
            self.generate_iterator_for_loop_ir(
                var_name,
                iterable,
                body,
                result_reg,
                for_span,
                instructions,
                constants,
            )
        } else if let Some(_iter_ty) = self.get_expr_mono_type(iterable) {
            // 不支持的迭代器类型，返回错误（使用实际类型名称）
            let iter_type = self.get_expr_type_name(iterable);
            let span = Self::get_expr_span(iterable);
            Err(ErrorCodeDefinition::ir_unsupported_iterator(&iter_type)
                .at(span)
                .build())
        } else {
            // 不支持的迭代器类型，返回错误（使用实际类型名称）
            let iter_type = self.get_expr_type_name(iterable);
            let span = Self::get_expr_span(iterable);
            Err(ErrorCodeDefinition::ir_unsupported_iterator(&iter_type)
                .at(span)
                .build())
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
    ) -> Result<(), Diagnostic> {
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
            span: for_span,
            def: None,
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
            span: for_span,
            def: None,
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
            span: for_span,
            def: None,
        });
        instructions.push(Instruction::Store {
            dst: Operand::Local(var_reg),
            src: Operand::Local(element_reg),
            span: for_span,
        });

        // 8. 执行循环体
        self.generate_block_ir(body, None, instructions, constants)?;

        // 9. 跳转回循环开始
        instructions.push(Instruction::Jmp(loop_start_idx));

        // 10. 修复跳转
        let end_idx = instructions.len();
        if let Instruction::JmpIfNot(_, ref mut target) = instructions[jump_end_idx] {
            *target = end_idx;
        }

        self.exit_scope();

        if let Some(reg) = result_reg {
            // For loop returns void
            instructions.push(Instruction::Load {
                dst: Operand::Local(reg),
                src: Operand::Const(ConstValue::Void),
            });
        }

        Ok(())
    }

    /// 生成 spawn for 数据并行循环的 IR
    ///
    /// 将 `spawn for item in items { process(item) }` 展开为：
    /// ```text
    /// closures = []          // AllocArray
    /// iterator = iter(items) // Call
    /// while has_next(iterator) {
    ///     item = next(iterator)
    ///     closure = () => { body }  // Lambda
    ///     push(closures, closure)   // Call std.list.push
    /// }
    /// Spawn { closures, plan, result }
    /// ```
    #[allow(clippy::too_many_arguments)]
    fn generate_spawn_for_ir(
        &mut self,
        var_name: &str,
        // ponytail: for-mut 的可变性由 typecheck 校验，ir_gen 无需追踪
        _var_mut: bool,
        iterable: &ast::Expr,
        body: &ast::Block,
        result_reg: usize,
        span: Span,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        use crate::frontend::core::spawn::analysis::analyze_spawn_for;

        // 1. 分析循环体读写集
        let (trait_table, local_var_types) = if let Some(ref type_result) = self.type_result {
            (&type_result.trait_table, &type_result.local_var_types)
        } else {
            static EMPTY_TRAIT_TABLE: std::sync::LazyLock<
                crate::frontend::core::types::TraitTable,
            > = std::sync::LazyLock::new(crate::frontend::core::types::TraitTable::default);
            static EMPTY_VAR_TYPES: std::sync::LazyLock<
                std::collections::HashMap<String, crate::frontend::core::types::MonoType>,
            > = std::sync::LazyLock::new(std::collections::HashMap::new);
            (&*EMPTY_TRAIT_TABLE, &*EMPTY_VAR_TYPES)
        };

        let spawn_for_analysis =
            analyze_spawn_for(var_name, iterable, body, trait_table, local_var_types);

        // 2. 进入 spawn 作用域
        self.enter_scope();

        // 3. 创建空列表寄存器，用于收集闭包
        let closures_list_reg = self.next_temp_reg();
        instructions.push(Instruction::AllocArray {
            dst: Operand::Local(closures_list_reg),
            size: Operand::Const(ConstValue::Int(0)),
            elem_size: Operand::Const(ConstValue::Int(1)),
        });

        // 4. 计算可迭代对象
        let iterable_reg = self.next_temp_reg();
        self.generate_expr_ir(iterable, iterable_reg, instructions, constants)?;

        // 5. 创建迭代器: iterator = iter(iterable)
        let iterator_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(Operand::Local(iterator_reg)),
            func: Operand::Const(ConstValue::String("std.list.iter".to_string())),
            args: vec![Operand::Local(iterable_reg)],
            span,
            def: None,
        });

        // 6. 注册循环变量
        let var_reg = self.next_temp_reg();
        self.register_local(var_name, var_reg);

        // 7. 循环开始
        let loop_start_idx = instructions.len();

        // 8. has_next?
        let has_more_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(Operand::Local(has_more_reg)),
            func: Operand::Const(ConstValue::String("std.list.has_next".to_string())),
            args: vec![Operand::Local(iterator_reg)],
            span,
            def: None,
        });

        // 9. 如果没有更多元素，跳转到结束
        let jump_end_idx = instructions.len();
        instructions.push(Instruction::JmpIfNot(Operand::Local(has_more_reg), 0));

        // 10. 获取下一个元素: var = next(iterator)
        let element_reg = self.next_temp_reg();
        instructions.push(Instruction::Call {
            dst: Some(Operand::Local(element_reg)),
            func: Operand::Const(ConstValue::String("std.list.next".to_string())),
            args: vec![Operand::Local(iterator_reg)],
            span,
            def: None,
        });
        instructions.push(Instruction::Store {
            dst: Operand::Local(var_reg),
            src: Operand::Local(element_reg),
            span,
        });

        // 11. 在循环体内创建闭包：(item) => { body }
        //     迭代变量作为参数传入，当前值通过 env 捕获
        let closure_reg = self.next_temp_reg();
        // 捕获迭代变量的当前值（element_reg 在 next() 返回后被设置）
        self.pending_env_vars = vec![Operand::Local(element_reg)];
        let lambda = ast::Expr::Lambda {
            params: vec![ast::Param {
                name: var_name.to_string(),
                ty: None,
                is_mut: false,
                span,
            }],
            body: Box::new(body.clone()),
            span,
        };
        self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;

        // 12. 将闭包推入列表: push(closures, closure)
        instructions.push(Instruction::Call {
            dst: Some(Operand::Local(closures_list_reg)),
            func: Operand::Const(ConstValue::String("std.list.push".to_string())),
            args: vec![
                Operand::Local(closures_list_reg),
                Operand::Local(closure_reg),
            ],
            span,
            def: None,
        });

        // 13. 跳转回循环开始
        instructions.push(Instruction::Jmp(loop_start_idx));

        // 14. 修复跳出循环的跳转目标
        let end_idx = instructions.len();
        if let Instruction::JmpIfNot(_, ref mut target) = instructions[jump_end_idx] {
            *target = end_idx;
        }

        // 15. 构建执行计划
        //     所有迭代共享同一个读写集（编译期分析的循环体特征）
        let read_write_sets = vec![(
            spawn_for_analysis.reads.clone(),
            spawn_for_analysis.writes.clone(),
        )];
        let resource_var_sets = vec![spawn_for_analysis.resource_vars.clone()];
        let plan = crate::frontend::core::spawn::analysis::build_execution_plan(
            &read_write_sets,
            &resource_var_sets,
        );

        // 16. 生成 SpawnFromList 指令
        //     closures_list 参数是闭包列表寄存器（运行时动态收集）
        instructions.push(Instruction::SpawnFromList {
            closures_list: Operand::Local(closures_list_reg),
            plan,
            result: Operand::Local(result_reg),
        });

        // 17. 退出 spawn 作用域
        self.exit_scope();

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
            ast::Expr::SpawnFor { span, .. } => *span,
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
            ast::Expr::Spawn { span, .. } => *span,
            ast::Expr::Lambda { span, .. } => *span,
            ast::Expr::FString { span, .. } => *span,
            ast::Expr::Error(span) => *span,
            ast::Expr::Borrow { span, .. } => *span,
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
                let mono_type = poly_type.body.clone();
                return mono_type.type_name();
            }
            // 3. 从 IR 生成器本地追踪的类型中查找
            if let Some(type_name) = self.local_var_types.get(name) {
                return type_name.clone();
            }
        }

        // 构造器调用：Point(1.0, 2.0) → 类型名为 "Point"
        if let ast::Expr::Call { func, .. } = expr {
            if let ast::Expr::Var(name, _) = func.as_ref() {
                if self.struct_definitions.contains_key(name) {
                    return name.clone();
                }
            }
        }

        // 对于其他表达式，不做 AST 猜测
        "<unknown>".to_string()
    }

    /// 解析函数表达式为函数名 Operand（用于普通函数调用）
    fn resolve_function_name(
        &self,
        func: &ast::Expr,
    ) -> Result<Operand, Diagnostic> {
        if let Expr::Var(name, _) = func {
            let resolved_name = if let Some(qualified) = self.use_aliases.get(name) {
                qualified.clone()
            } else if self.registry.is_native_name(name) {
                name.clone()
            } else if let Some(qualified) = self.registry.short_to_qualified_map().get(name) {
                qualified.clone()
            } else {
                name.clone()
            };
            Ok(Operand::Const(ConstValue::String(resolved_name)))
        } else if let Expr::Call { func: inner, .. } = func {
            // 两层调用 Container(Int)(42, 43)：内层类型实参运行期擦除，
            // 按结构体构造器名生成（字段填充由调用点实参决定）。
            if let Expr::Var(name, _) = inner.as_ref() {
                if self.struct_definitions.contains_key(name) {
                    return Ok(Operand::Const(ConstValue::String(name.clone())));
                }
            }
            Err(ErrorCodeDefinition::ir_internal_error(&format!(
                "无法解析函数名：非结构体构造器调用 {:?}",
                func
            ))
            .at(Self::get_expr_span(func))
            .build())
        } else {
            Err(ErrorCodeDefinition::ir_internal_error(&format!(
                "无法解析函数名：非变量表达式 {:?}",
                func
            ))
            .at(Self::get_expr_span(func))
            .build())
        }
    }

    /// 判断函数表达式是否可在编译期解析为静态函数名。
    ///
    /// 只有全局函数声明（通过 let/fn 定义的具名函数）才是静态的。
    /// 局部变量、闭包表达式、函数调用返回值等全部走动态分发。
    fn is_static_fn_name(
        &self,
        func: &ast::Expr,
    ) -> bool {
        match func {
            ast::Expr::Var(name, _) => {
                if self.lookup_local(name).is_some() {
                    return false;
                }
                self.lookup_var_type(name).is_some()
            }
            ast::Expr::Call { func: inner, .. } => match inner.as_ref() {
                ast::Expr::Var(name, _) => self.struct_definitions.contains_key(name),
                _ => false,
            },
            _ => false,
        }
    }

    /// 获取表达式的推断类型（用于 IR 生成阶段的分支）
    fn get_expr_mono_type(
        &self,
        expr: &ast::Expr,
    ) -> Option<MonoType> {
        match expr {
            ast::Expr::BinOp {
                op: ast::BinOp::Range,
                left,
                right,
                ..
            } => {
                let left_ty = self.get_expr_mono_type(left).unwrap_or(MonoType::Int(64));
                let right_ty = self.get_expr_mono_type(right).unwrap_or(MonoType::Int(64));
                let elem_type = if left_ty == right_ty {
                    left_ty
                } else {
                    MonoType::Int(64)
                };
                Some(MonoType::Range {
                    elem_type: Box::new(elem_type),
                })
            }
            ast::Expr::Var(name, _) => {
                if let Some(ref type_result) = self.type_result {
                    if let Some(mono_type) = type_result.local_var_types.get(name) {
                        return Some(mono_type.clone());
                    }
                }

                self.lookup_var_type(name)
                    .map(|poly_type| poly_type.body.clone())
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
    ) -> Result<LambdaBodyIR, Diagnostic> {
        // 保存父函数的临时寄存器计数
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
        }

        // 记录局部变量起始位置
        let local_var_start = params.len();
        self.next_temp = local_var_start;

        // 处理函数体语句
        for stmt in &body.stmts {
            self.generate_local_stmt_ir(stmt, &mut instructions, constants)?;
        }

        // 如果没有遇到 Ret 指令，追加 Ret(None)
        let has_ret = instructions
            .iter()
            .any(|inst| matches!(inst, Instruction::Ret(_)));
        if !has_ret {
            instructions.push(Instruction::Ret(None));
        }

        // 退出作用域
        self.exit_scope();

        // 计算局部变量总数
        let total_locals = self.next_temp;
        let locals_types: Vec<MonoType> = (0..total_locals).map(|_| MonoType::Int(64)).collect();

        // 恢复父函数的临时寄存器计数
        self.next_temp = saved_next_temp;

        Ok(LambdaBodyIR {
            instructions,
            locals: locals_types,
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
    ) -> Result<(), Diagnostic> {
        match expr {
            Expr::Lit(literal, _) => {
                // 常量加载
                let const_val = match literal {
                    Literal::Int(n) => ConstValue::Int(*n),
                    Literal::Float(f) => ConstValue::Float(*f),
                    Literal::Bool(b) => ConstValue::Bool(*b),
                    Literal::String(s) => ConstValue::String(s.clone()),
                    Literal::Char(c) => ConstValue::Char(*c),
                    Literal::Void => ConstValue::Void,
                };
                // 添加到常量池
                constants.push(const_val.clone());
                instructions.push(Instruction::Load {
                    dst: Operand::Local(result_reg),
                    src: Operand::Const(const_val),
                });
            }
            Expr::Var(var_name, var_span) => {
                // #254：闭包体内——先查捕获表（外层变量经 env 捕获，LoadUpvalue 读）
                if let Some(&env_idx) = self.closure_captures.get(var_name) {
                    instructions.push(Instruction::LoadUpvalue {
                        dst: Operand::Local(result_reg),
                        upvalue_idx: env_idx,
                    });
                } else if let Some(local_idx) = self.lookup_local(var_name) {
                    // 局部变量：直接加载
                    instructions.push(Instruction::Load {
                        dst: Operand::Local(result_reg),
                        src: Operand::Local(local_idx),
                    });
                } else if self.lookup_global(var_name).is_some() {
                    // 全局变量：生成函数调用获取值
                    let func_name = var_name.clone();
                    instructions.push(Instruction::Call {
                        dst: Some(Operand::Local(result_reg)),
                        func: Operand::Const(ConstValue::String(func_name)),
                        args: vec![],
                        span: *var_span,
                        def: None,
                    });
                } else if let Some(qualified) = self.use_aliases.get(var_name) {
                    // 选择性导入的绑定（常量/函数）：按限定名调用 native handler 取值。
                    // 例：use std.math.{PI} → PI 展开为 std.math.PI，调用 native_pi 返回真实常量。
                    // 修复：此前常量引用掉进“静默 Load Int(0)”兜底，PI 运行时恒为 0（#251）。
                    instructions.push(Instruction::Call {
                        dst: Some(Operand::Local(result_reg)),
                        func: Operand::Const(ConstValue::String(qualified.clone())),
                        args: vec![],
                        span: *var_span,
                        def: None,
                    });
                } else if matches!(
                    var_name.as_str(),
                    "Int"
                        | "Float"
                        | "Bool"
                        | "String"
                        | "Char"
                        | "Bytes"
                        | "Type"
                        | "Void"
                        | "Never"
                ) {
                    // 内置类型名作为类型实参（如 SafeArray(Int, 3)）：类型宇宙的值，
                    // 运行时无表示，加载 Void 占位（类型参数在编译期已被消费）
                    instructions.push(Instruction::Load {
                        dst: Operand::Local(result_reg),
                        src: Operand::Const(ConstValue::Void),
                    });
                } else {
                    // #271 #3：未解析变量 → 硬错误（#254 spawn 捕获已落地，不再需要静默 Load 0 兜底）。
                    // 走到这里说明 typecheck 漏网，属编译器内部一致性问题。
                    return Err(ErrorCodeDefinition::unresolved_variable(var_name)
                        .at(*var_span)
                        .build());
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

                            // 统一走 Store — 消除 Var→Var 走 Move 的特殊情况
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
                    ast::BinOp::And | ast::BinOp::Or => {
                        // SPEC §2.2 / RFC-010 权威语义：and/or 短路求值
                        // a and b ≡ if a { b } else { false }；a or b ≡ if a { true } else { b }
                        let lhs_reg = self.next_temp_reg();
                        self.generate_expr_ir(left, lhs_reg, instructions, constants)?;
                        let is_and = matches!(op, ast::BinOp::And);
                        let short_idx = instructions.len();
                        if is_and {
                            instructions.push(Instruction::JmpIfNot(Operand::Local(lhs_reg), 0));
                        } else {
                            instructions.push(Instruction::JmpIf(Operand::Local(lhs_reg), 0));
                        }
                        self.generate_expr_ir(right, result_reg, instructions, constants)?;
                        let end_idx = instructions.len();
                        instructions.push(Instruction::Jmp(0));
                        // 短路值：and → false，or → true
                        let sc_target = instructions.len();
                        if let Instruction::JmpIf(_, ref mut t) = instructions[short_idx] {
                            *t = sc_target;
                        }
                        if let Instruction::JmpIfNot(_, ref mut t) = instructions[short_idx] {
                            *t = sc_target;
                        }
                        instructions.push(Instruction::Load {
                            dst: Operand::Local(result_reg),
                            src: Operand::Const(if is_and {
                                ConstValue::Bool(false)
                            } else {
                                ConstValue::Bool(true)
                            }),
                        });
                        let end_target = instructions.len();
                        if let Instruction::Jmp(ref mut t) = instructions[end_idx] {
                            *t = end_target;
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
                                span: *span,
                            },
                            ast::BinOp::Mod => Instruction::Mod {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                                span: *span,
                            },
                            // #285: 位运算/移位（SPEC §2.2 级 7/8）
                            ast::BinOp::BitAnd => Instruction::And {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::BitOr => Instruction::Or {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::BitXor => Instruction::Xor {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Shl => Instruction::Shl {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Shr => Instruction::Shr {
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
                            // Assign 在上方分支处理；And/Or 走短路求值；Range 仅限 for/切片上下文。
                            // 剩余运算符到达此处即内部错误——禁止静默兜底（教训：&&/|| 曾静默编译为常量 0，#251）
                            _ => {
                                return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                                    "unhandled binary operator: {:?}",
                                    op
                                ))
                                .build());
                            }
                        }
                    }
                };
                instructions.push(instr);
            }
            Expr::Call {
                func,
                args,
                named_args,
                span,
            } => {
                // 检查是否是方法调用：func 是 FieldAccess
                if let Expr::FieldAccess { expr, field, .. } = func.as_ref() {
                    // 方法调用 - 转换为普通函数调用
                    // 命名空间机制：p.method() -> method(p)

                    // 只有非命名空间调用才需要添加 self 参数
                    // 命名空间调用（如 std.io.println）不需要隐式参数
                    if self.is_namespace_receiver(expr) {
                        // 命名空间调用：不需要隐式参数
                        let mut arg_regs = Vec::new();
                        for arg in args.iter() {
                            let arg_reg = self.next_temp_reg();
                            self.generate_expr_ir(arg, arg_reg, instructions, constants)?;
                            arg_regs.push(Operand::Local(arg_reg));
                        }
                        let method_function_name = self.resolve_field_path(expr, field);
                        instructions.push(Instruction::Call {
                            dst: Some(Operand::Local(result_reg)),
                            func: Operand::Const(ConstValue::String(
                                method_function_name.to_string(),
                            )),
                            args: arg_regs,
                            span: *span,
                            def: None,
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
                            let total_params = binding.positions.len() + method_arg_regs.len();
                            let mut final_args: Vec<Operand> = Vec::with_capacity(total_params);
                            let mut method_arg_iter = method_arg_regs.into_iter();

                            for pos in 0..total_params {
                                if binding.positions.contains(&(pos as i64)) {
                                    final_args.push(Operand::Local(obj_reg));
                                } else if let Some(arg_reg) = method_arg_iter.next() {
                                    final_args.push(arg_reg);
                                }
                            }

                            // 解析函数名
                            let func_name = if let Some(qualified) = self
                                .registry
                                .short_to_qualified_map()
                                .get(&binding.function)
                            {
                                qualified.clone()
                            } else {
                                binding.function.clone()
                            };

                            instructions.push(Instruction::Call {
                                dst: Some(Operand::Local(result_reg)),
                                func: Operand::Const(ConstValue::String(func_name)),
                                args: final_args,
                                span: *span,
                                def: None,
                            });
                        } else {
                            // 常规方法调用（无绑定）：obj.method(args) → method(obj, args)
                            // 接口直接赋值优化：检查对象是否是约束变量
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

                            // 检查对象是否是约束变量（接口直接赋值优化）
                            let var_name = if let Expr::Var(name, _) = expr.as_ref() {
                                Some(name.clone())
                            } else {
                                None
                            };

                            let concrete_type = var_name.as_ref().and_then(|name| {
                                self.get_constraint_var_concrete_type(name).cloned()
                            });

                            if let Some(concrete_type_name) = concrete_type {
                                // 编译期可确定具体类型 → 直接调用（零开销）
                                // d.draw(screen) → ConcreteType.draw(d, screen)
                                let qualified_name = format!("{}.{}", concrete_type_name, field);

                                let final_args: Vec<Operand> = arg_regs.clone();

                                instructions.push(Instruction::Call {
                                    dst: Some(Operand::Local(result_reg)),
                                    func: Operand::Const(ConstValue::String(qualified_name)),
                                    args: final_args,
                                    span: *span,
                                    def: None,
                                });
                            } else if var_name.as_ref().is_some_and(|name| {
                                // 检查变量的类型标注是否是约束类型（但具体类型未知）
                                self.local_var_types
                                    .get(name)
                                    .and_then(|type_name| {
                                        // 如果变量类型是约束类型且不在 constraint_var_concrete_types 中
                                        // 说明具体类型无法在编译期确定，需要 vtable 调用
                                        if !self.struct_definitions.contains_key(type_name)
                                            && !self
                                                .constraint_var_concrete_types
                                                .contains_key(name)
                                        {
                                            // 简单启发式：如果变量类型不是已知结构体，可能是约束类型
                                            Some(true)
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or(false)
                            }) {
                                // 编译期无法确定具体类型 → CallVirt（vtable 调用）
                                instructions.push(Instruction::CallVirt {
                                    dst: Some(Operand::Local(result_reg)),
                                    obj: Operand::Local(obj_reg),
                                    method_name: field.to_string(),
                                    args: arg_regs,
                                    span: *span,
                                });
                            } else {
                                // 普通方法调用
                                // 优先使用类型名（而非变量名）构建函数名
                                // 例如：a.is_greater(b) 中 a 的类型是 Node
                                // → 函数名应为 "Node.is_greater" 而非 "a.is_greater"
                                let func_name = if let Expr::Var(name, _) = expr.as_ref() {
                                    if let Some(type_name) = self.local_var_types.get(name) {
                                        // #266: &mut 令牌穿透——变量类型可能是
                                        // "&mut Point"，方法名应基于底层结构体 "Point"
                                        let base = Self::strip_ref_prefix(type_name);
                                        format!("{}.{}", base, field)
                                    } else {
                                        self.resolve_field_path(expr, field)
                                    }
                                } else {
                                    self.resolve_field_path(expr, field)
                                };

                                let final_args: Vec<Operand> = arg_regs.clone();

                                instructions.push(Instruction::Call {
                                    dst: Some(Operand::Local(result_reg)),
                                    func: Operand::Const(ConstValue::String(func_name)),
                                    args: final_args,
                                    span: *span,
                                    def: None,
                                });
                            }
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

                    // RFC-010: 处理命名参数构造 `Point(x=1, y=2)`
                    if !named_args.is_empty() {
                        if let Expr::Var(name, _) = func.as_ref() {
                            if let Some(fields) = self.struct_definitions.get(name).cloned() {
                                // 生成命名参数的 IR
                                let mut named_regs: Vec<(String, Operand)> = Vec::new();
                                for (arg_name, arg_expr) in named_args.iter() {
                                    let arg_reg = self.next_temp_reg();
                                    self.generate_expr_ir(
                                        arg_expr,
                                        arg_reg,
                                        instructions,
                                        constants,
                                    )?;
                                    named_regs.push((arg_name.clone(), Operand::Local(arg_reg)));
                                }

                                // 按字段顺序重排参数
                                let mut final_args: Vec<Option<Operand>> = vec![None; fields.len()];

                                // 先放置位置参数
                                for (i, reg) in arg_regs.iter().enumerate() {
                                    if i < fields.len() {
                                        final_args[i] = Some(reg.clone());
                                    }
                                }

                                // 再放置命名参数（按字段名匹配）
                                for (name, reg) in &named_regs {
                                    if let Some(idx) = fields.iter().position(|f| &f.name == name) {
                                        final_args[idx] = Some(reg.clone());
                                    }
                                }

                                // 填充默认值
                                for (i, slot) in final_args.iter_mut().enumerate() {
                                    if slot.is_none() {
                                        let default_reg = self.next_temp_reg();
                                        if let Some(default_expr) = &fields[i].default {
                                            self.generate_expr_ir(
                                                default_expr,
                                                default_reg,
                                                instructions,
                                                constants,
                                            )?;
                                        } else {
                                            instructions.push(Instruction::Load {
                                                dst: Operand::Local(default_reg),
                                                src: Operand::Const(ConstValue::Int(0)),
                                            });
                                        }
                                        *slot = Some(Operand::Local(default_reg));
                                    }
                                }

                                arg_regs = final_args.into_iter().map(|s| s.unwrap()).collect();
                            }
                        }
                    }

                    // 检查是否是结构体构造器调用，需要填充默认值
                    // 两层调用 X(类型参数)(构造参数)：func 是 Call{func: Var(name)}，
                    // 内层类型实参运行期擦除，外层实参按字段位置填充。
                    let struct_ctor_name: Option<String> = match func.as_ref() {
                        Expr::Var(name, _) => Some(name.clone()),
                        Expr::Call { func: inner, .. } => match inner.as_ref() {
                            Expr::Var(name, _) => Some(name.clone()),
                            _ => None,
                        },
                        _ => None,
                    };
                    if let Some(name) = struct_ctor_name {
                        if let Some(fields) = self.struct_definitions.get(&name).cloned() {
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

                    // 检查是否是动态函数调用（非静态函数名的 Var）
                    if !self.is_static_fn_name(func) {
                        // 闭包调用：先加载函数值，然后使用 CallDyn
                        let func_reg = self.next_temp_reg();
                        self.generate_expr_ir(func, func_reg, instructions, constants)?;

                        instructions.push(Instruction::CallDyn {
                            dst: Some(Operand::Local(result_reg)),
                            func: Operand::Local(func_reg),
                            args: arg_regs,
                            span: *span,
                        });
                    } else {
                        // ========== print/println 零开销分发处理 ==========
                        // 检查是否是 print 或 println 调用
                        let is_print_call = if let Expr::Var(name, _) = func.as_ref() {
                            matches!(
                                name.as_str(),
                                "print" | "println" | "std.io.print" | "std.io.println"
                            )
                        } else {
                            false
                        };

                        // 如果是 print/println 且有参数，尝试零开销分发
                        if is_print_call && !args.is_empty() {
                            let arg_expr = &args[0];
                            // 获取参数的类型信息
                            let arg_type = self.get_expr_mono_type(arg_expr);

                            if let Some(mono_type) = arg_type {
                                // 检查类型是否实现了 Stringable（to_string 方法）
                                if type_implements_stringable(&mono_type) {
                                    // 零开销路径：直接调用 to_string 方法
                                    // 生成: arg.to_string()
                                    let func_name = format!(
                                        "{}.to_string",
                                        get_type_fallback_string(&mono_type)
                                    );
                                    let mut arg_regs_for_method = Vec::new();

                                    // 先计算参数值
                                    let arg_reg = self.next_temp_reg();
                                    self.generate_expr_ir(
                                        arg_expr,
                                        arg_reg,
                                        instructions,
                                        constants,
                                    )?;
                                    arg_regs_for_method.push(Operand::Local(arg_reg));

                                    // 调用 to_string 方法
                                    let to_string_reg = self.next_temp_reg();
                                    instructions.push(Instruction::Call {
                                        dst: Some(Operand::Local(to_string_reg)),
                                        func: Operand::Const(ConstValue::String(func_name)),
                                        args: arg_regs_for_method,
                                        span: *span,
                                        def: None,
                                    });

                                    // 然后调用 std.io.print 输出字符串
                                    // 使用 resolved name
                                    let print_func_name = if let Expr::Var(name, _) = func.as_ref()
                                    {
                                        if name == "print" || name == "println" {
                                            if let Some(qualified) =
                                                self.registry.short_to_qualified_map().get(name)
                                            {
                                                qualified.clone()
                                            } else {
                                                format!("std.io.{}", name)
                                            }
                                        } else {
                                            name.clone()
                                        }
                                    } else {
                                        "std.io.print".to_string()
                                    };

                                    instructions.push(Instruction::Call {
                                        dst: Some(Operand::Local(result_reg)),
                                        func: Operand::Const(ConstValue::String(print_func_name)),
                                        args: vec![Operand::Local(to_string_reg)],
                                        span: *span,
                                        def: None,
                                    });
                                } else {
                                    // 兜底路径：类型未实现 Stringable，调用 std.io.print 输出类型信息
                                    // 生成: std.io.format_fallback(arg, type_name)
                                    let type_name = get_type_fallback_string(&mono_type);

                                    // 先计算参数值
                                    let arg_reg = self.next_temp_reg();
                                    self.generate_expr_ir(
                                        arg_expr,
                                        arg_reg,
                                        instructions,
                                        constants,
                                    )?;

                                    // 调用 format_fallback 获取类型信息字符串
                                    let fallback_reg = self.next_temp_reg();
                                    instructions.push(Instruction::Call {
                                        dst: Some(Operand::Local(fallback_reg)),
                                        func: Operand::Const(ConstValue::String(
                                            "std.io.format_fallback".to_string(),
                                        )),
                                        args: vec![
                                            Operand::Local(arg_reg),
                                            Operand::Const(ConstValue::String(type_name)),
                                        ],
                                        span: *span,
                                        def: None,
                                    });

                                    // 然后调用 std.io.print 输出
                                    let print_func_name = if let Expr::Var(name, _) = func.as_ref()
                                    {
                                        if name == "print" || name == "println" {
                                            if let Some(qualified) =
                                                self.registry.short_to_qualified_map().get(name)
                                            {
                                                qualified.clone()
                                            } else {
                                                format!("std.io.{}", name)
                                            }
                                        } else {
                                            name.clone()
                                        }
                                    } else {
                                        "std.io.print".to_string()
                                    };

                                    instructions.push(Instruction::Call {
                                        dst: Some(Operand::Local(result_reg)),
                                        func: Operand::Const(ConstValue::String(print_func_name)),
                                        args: vec![Operand::Local(fallback_reg)],
                                        span: *span,
                                        def: None,
                                    });
                                }
                            } else {
                                // 无法获取类型，使用默认处理
                                let func_operand = self.resolve_function_name(func)?;
                                instructions.push(Instruction::Call {
                                    dst: Some(Operand::Local(result_reg)),
                                    func: func_operand,
                                    args: arg_regs,
                                    span: *span,
                                    def: None,
                                });
                            }
                        } else {
                            // 非 print 调用或无参数，使用默认处理
                            // ========== 默认函数调用处理 ==========
                            let final_args: Vec<Operand> = arg_regs.clone();

                            let func_operand = self.resolve_function_name(func)?;
                            instructions.push(Instruction::Call {
                                dst: Some(Operand::Local(result_reg)),
                                func: func_operand,
                                args: final_args,
                                span: *span,
                                def: None,
                            });
                        }
                    }
                }
            }
            Expr::FieldAccess { expr, field, span } => {
                // 首先检查是否是模块变量的字段访问（如 io.println）
                // io 是通过 use std.{io} 导入的模块变量
                if let Expr::Var(module_name, _) = expr.as_ref() {
                    if let Some(full_path) = {
                        let reg = &self.registry;
                        if reg.is_std_submodule(module_name) {
                            let path = format!("std.{}", field);
                            if reg.is_native_name(&path) {
                                Some(path)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } {
                        // 模块变量方法调用：生成函数调用
                        // 例如：io.println -> Call("std.io.println", [args])
                        // 这里我们处理的是非调用场景的字段访问（如 io.println 作为值）
                        // 生成零参数调用
                        instructions.push(Instruction::Call {
                            dst: Some(Operand::Local(result_reg)),
                            func: Operand::Const(ConstValue::String(full_path)),
                            args: vec![],
                            span: *span,
                            def: None,
                        });
                    } else {
                        // 普通字段访问
                        let obj_reg = self.next_temp_reg();
                        self.generate_expr_ir(expr, obj_reg, instructions, constants)?;
                        let field_index =
                            self.resolve_field_index(expr, field).ok_or_else(|| {
                                ErrorCodeDefinition::ir_internal_error(&format!(
                                    "无法解析字段索引: '{}'",
                                    field
                                ))
                                .at(Self::get_expr_span(expr))
                                .build()
                            })?;
                        instructions.push(Instruction::LoadField {
                            dst: Operand::Local(result_reg),
                            src: Operand::Local(obj_reg),
                            field: field_index,
                            span: *span,
                        });
                    }
                } else {
                    // 提取完整的命名空间路径（如 std.math.PI）
                    let full_path = self.resolve_field_path(expr, field);

                    // 检查是否是命名空间常量访问
                    if self.registry.is_native_name(&full_path) {
                        // 命名空间常量访问：生成零参数函数调用
                        instructions.push(Instruction::Call {
                            dst: Some(Operand::Local(result_reg)),
                            func: Operand::Const(ConstValue::String(full_path)),
                            args: vec![],
                            span: *span,
                            def: None,
                        });
                    } else {
                        // 普通字段访问
                        let obj_reg = self.next_temp_reg();
                        self.generate_expr_ir(expr, obj_reg, instructions, constants)?;
                        let field_index =
                            self.resolve_field_index(expr, field).ok_or_else(|| {
                                ErrorCodeDefinition::ir_internal_error(&format!(
                                    "无法解析字段索引: '{}'",
                                    field
                                ))
                                .at(Self::get_expr_span(expr))
                                .build()
                            })?;
                        instructions.push(Instruction::LoadField {
                            dst: Operand::Local(result_reg),
                            src: Operand::Local(obj_reg),
                            field: field_index,
                            span: *span,
                        });
                    }
                }
            }
            Expr::ListComp {
                element,
                var,
                iterable,
                condition,
                span,
            } => {
                // 列表推导式 IR 生成
                // [x * x for x in items] 等价于:
                //   1. 创建空结果列表
                //   2. 通过迭代器遍历 iterable
                //   3. 对每个元素: 绑定到 var, 检查 condition(可选), 计算 element, push 到结果列表
                //   4. 返回结果列表

                // 1. 创建空结果列表
                instructions.push(Instruction::AllocArray {
                    dst: Operand::Local(result_reg),
                    size: Operand::Const(ConstValue::Int(0)),
                    elem_size: Operand::Const(ConstValue::Int(1)),
                });

                // 2. 计算可迭代对象
                let iterable_reg = self.next_temp_reg();
                self.generate_expr_ir(iterable, iterable_reg, instructions, constants)?;

                // 3. 创建迭代器
                let iterator_reg = self.next_temp_reg();
                instructions.push(Instruction::Call {
                    dst: Some(Operand::Local(iterator_reg)),
                    func: Operand::Const(ConstValue::String("std.list.iter".to_string())),
                    args: vec![Operand::Local(iterable_reg)],
                    span: *span,
                    def: None,
                });

                // 4. 注册循环变量
                let var_reg = self.next_temp_reg();
                self.register_local(var, var_reg);

                // 5. 循环开始
                let loop_start_idx = instructions.len();

                // 6. has_next?
                let has_next_reg = self.next_temp_reg();
                instructions.push(Instruction::Call {
                    dst: Some(Operand::Local(has_next_reg)),
                    func: Operand::Const(ConstValue::String("std.list.has_next".to_string())),
                    args: vec![Operand::Local(iterator_reg)],
                    span: *span,
                    def: None,
                });

                let jump_end_idx = instructions.len();
                instructions.push(Instruction::JmpIfNot(
                    Operand::Local(has_next_reg),
                    0, // 占位符
                ));

                // 7. next element
                let element_reg = self.next_temp_reg();
                instructions.push(Instruction::Call {
                    dst: Some(Operand::Local(element_reg)),
                    func: Operand::Const(ConstValue::String("std.list.next".to_string())),
                    args: vec![Operand::Local(iterator_reg)],
                    span: *span,
                    def: None,
                });

                // 8. 存储到循环变量
                instructions.push(Instruction::Store {
                    dst: Operand::Local(var_reg),
                    src: Operand::Local(element_reg),
                    span: *span,
                });

                // 9. 如果有条件，检查条件
                if let Some(cond_expr) = condition {
                    let cond_reg = self.next_temp_reg();
                    self.generate_expr_ir(cond_expr, cond_reg, instructions, constants)?;

                    let skip_push_idx = instructions.len();
                    instructions.push(Instruction::JmpIfNot(
                        Operand::Local(cond_reg),
                        0, // 占位符
                    ));

                    // 10. 计算元素表达式
                    let comp_reg = self.next_temp_reg();
                    self.generate_expr_ir(element, comp_reg, instructions, constants)?;

                    // 11. push 到结果列表
                    instructions.push(Instruction::Call {
                        dst: Some(Operand::Local(result_reg)),
                        func: Operand::Const(ConstValue::String("std.list.push".to_string())),
                        args: vec![Operand::Local(result_reg), Operand::Local(comp_reg)],
                        span: *span,
                        def: None,
                    });

                    // 修复条件跳转
                    let after_push = instructions.len();
                    if let Instruction::JmpIfNot(_, ref mut target) = instructions[skip_push_idx] {
                        *target = after_push;
                    }
                } else {
                    // 10. 计算元素表达式
                    let comp_reg = self.next_temp_reg();
                    self.generate_expr_ir(element, comp_reg, instructions, constants)?;

                    // 11. push 到结果列表
                    instructions.push(Instruction::Call {
                        dst: Some(Operand::Local(result_reg)),
                        func: Operand::Const(ConstValue::String("std.list.push".to_string())),
                        args: vec![Operand::Local(result_reg), Operand::Local(comp_reg)],
                        span: *span,
                        def: None,
                    });
                }

                // 12. 跳回循环开始
                instructions.push(Instruction::Jmp(loop_start_idx));

                // 13. 修复跳出循环的跳转目标
                let end_pos = instructions.len();
                if let Instruction::JmpIfNot(_, ref mut target) = instructions[jump_end_idx] {
                    *target = end_pos;
                }
            }
            Expr::List(elements, span) => {
                // 列表字面量：先创建空列表，再按索引写入元素
                instructions.push(Instruction::AllocArray {
                    dst: Operand::Local(result_reg),
                    size: Operand::Const(ConstValue::Int(elements.len() as i128)),
                    elem_size: Operand::Const(ConstValue::Int(1)),
                });

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
            Expr::Dict(pairs, _span) => {
                // 字典字面量：使用 NewDict 指令一次性创建
                let mut keys = Vec::new();
                let mut values = Vec::new();
                for (key_expr, val_expr) in pairs {
                    let key_reg = self.next_temp_reg();
                    self.generate_expr_ir(key_expr, key_reg, instructions, constants)?;
                    keys.push(Operand::Local(key_reg));
                    let val_reg = self.next_temp_reg();
                    self.generate_expr_ir(val_expr, val_reg, instructions, constants)?;
                    values.push(Operand::Local(val_reg));
                }
                instructions.push(Instruction::NewDict {
                    dst: Operand::Local(result_reg),
                    keys,
                    values,
                });
            }
            Expr::Index { expr, index, span } => {
                let src_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, src_reg, instructions, constants)?;

                let index_reg = self.next_temp_reg();
                self.generate_expr_ir(index, index_reg, instructions, constants)?;

                instructions.push(Instruction::LoadIndex {
                    dst: Operand::Local(result_reg),
                    src: Operand::Local(src_reg),
                    index: Operand::Local(index_reg),
                    span: *span,
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
            Expr::Try { expr, span: _ } => {
                // `expr?`：当前阶段仅作为错误传播标记，运行时等价于 `expr`。
                // 错误的传播由解释器/Runtime 的错误通道处理（RFC-001）。
                self.generate_expr_ir(expr, result_reg, instructions, constants)?;
            }
            Expr::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                span: _,
            } => {
                // 重新实现 if 表达式，使用更简单的方法
                self.generate_if_expr_ir(
                    condition,
                    then_branch,
                    else_if_branches,
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
                // 生成内部表达式的 IR
                let src_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, src_reg, instructions, constants)?;

                // 逃逸分析：跨 spawn 使用 → Arc，否则 → Rc
                let var_name = match expr.as_ref() {
                    ast::Expr::Var(name, _) => Some(name.clone()),
                    _ => None,
                };
                let use_arc = var_name.as_ref().is_some_and(|n| {
                    self.type_result
                        .as_ref()
                        .is_some_and(|tr| tr.escaped_refs.contains(n))
                });

                if use_arc {
                    instructions.push(Instruction::ArcNew {
                        dst: Operand::Local(result_reg),
                        src: Operand::Local(src_reg),
                    });
                } else {
                    instructions.push(Instruction::RcNew {
                        dst: Operand::Local(result_reg),
                        src: Operand::Local(src_reg),
                    });
                }
            }
            Expr::Unsafe { body, span: _ } => {
                // unsafe 块：生成 UnsafeBlockStart/End 标记
                // 生成 UnsafeBlockStart 指令
                instructions.push(Instruction::UnsafeBlockStart);

                // 生成块内语句的 IR
                self.generate_block_ir(body, None, instructions, constants)?;

                // 生成 UnsafeBlockEnd 指令
                instructions.push(Instruction::UnsafeBlockEnd);

                // unsafe 块作为表达式时返回 void
                instructions.push(Instruction::Load {
                    dst: Operand::Local(result_reg),
                    src: Operand::Const(ConstValue::Void),
                });
            }
            // spawn for 数据并行循环（RFC-024 §2.4）
            Expr::SpawnFor {
                var,
                var_mut,
                iterable,
                body,
                span,
            } => {
                self.generate_spawn_for_ir(
                    var,
                    *var_mut,
                    iterable,
                    body,
                    result_reg,
                    *span,
                    instructions,
                    constants,
                )?;
            }
            Expr::Spawn { body, span } => {
                // Spawn block: spawn { ... }
                // RFC-024: DAG 分析识别直接子表达式，为每个生成独立闭包

                // 1. DAG 分析：识别直接子表达式，生成执行计划
                let (trait_table, local_var_types) = if let Some(ref type_result) = self.type_result
                {
                    (&type_result.trait_table, &type_result.local_var_types)
                } else {
                    // 无类型信息时使用空表（向后兼容）
                    static EMPTY_TRAIT_TABLE: std::sync::LazyLock<
                        crate::frontend::core::types::TraitTable,
                    > = std::sync::LazyLock::new(crate::frontend::core::types::TraitTable::default);
                    static EMPTY_VAR_TYPES: std::sync::LazyLock<
                        std::collections::HashMap<String, crate::frontend::core::types::MonoType>,
                    > = std::sync::LazyLock::new(std::collections::HashMap::new);
                    (&*EMPTY_TRAIT_TABLE, &*EMPTY_VAR_TYPES)
                };
                let analysis = crate::frontend::core::spawn::analysis::analyze_spawn_body(
                    body,
                    trait_table,
                    local_var_types,
                );

                // 2. 进入 spawn 作用域
                self.enter_scope();

                // 3. 为每个直接子表达式生成闭包
                let mut closure_regs = Vec::new();
                for task in &analysis.tasks {
                    // 如果是赋值，注册目标变量到 spawn 作用域
                    if let Some(target) = &task.target {
                        if self.lookup_local(target).is_none() {
                            let reg = self.next_temp_reg();
                            self.register_local(target, reg);
                        }
                    }

                    // 将 RHS 包装为无参闭包：() => { rhs }
                    // #254：捕获 task 读取的外层变量（RFC-024 §2.3 Move 值捕获）
                    // 过滤：仅捕获能在当前作用域链解析的变量（spawn 内声明的由
                    // 任务间依赖传递，不在此捕获）；task.reads 由 spawn analysis 提供。
                    let mut env_ops = Vec::new();
                    let mut env_names = Vec::new();
                    for var in &task.reads {
                        if let Some(local_idx) = self.lookup_local(var) {
                            env_ops.push(Operand::Local(local_idx));
                            env_names.push(var.clone());
                        }
                    }
                    self.pending_env_vars = env_ops;
                    self.pending_env_names = env_names;
                    let closure_reg = self.next_temp_reg();
                    let lambda = ast::Expr::Lambda {
                        params: Vec::new(),
                        body: Box::new(ast::Block {
                            stmts: vec![ast::Stmt {
                                kind: ast::StmtKind::Expr(Box::new(task.expr.clone())),
                                span: *span,
                            }],
                            span: *span,
                        }),
                        span: *span,
                    };
                    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
                    self.pending_env_vars.clear();
                    self.pending_env_names.clear();
                    closure_regs.push(Operand::Local(closure_reg));
                }

                // 4. 生成 spawn 块剩余语句（非直接子表达式，如 var 声明等）
                for stmt in &body.stmts {
                    if !crate::frontend::core::spawn::analysis::is_direct_child(stmt) {
                        self.generate_local_stmt_ir(stmt, instructions, constants)?;
                    }
                }

                // 5. 生成 Spawn 指令（多闭包 + 执行计划）
                // Spawn 指令会等待所有闭包完成，之后 t1/t2 等变量才可用
                instructions.push(Instruction::Spawn {
                    closures: closure_regs,
                    plan: analysis.plan,
                    result: Operand::Local(result_reg),
                });

                // 6. 块的结果值：从 return 语句获取（RFC-010 语义）
                // 必须在 Spawn 之后生成，因为 return 表达式可能引用闭包的结果变量
                let mut has_return = false;
                for stmt in &body.stmts {
                    if let ast::StmtKind::Expr(ref expr_stmt) = stmt.kind {
                        if let ast::Expr::Return(Some(ret_expr), _) = expr_stmt.as_ref() {
                            let ret_reg = self.next_temp_reg();
                            self.generate_expr_ir(ret_expr, ret_reg, instructions, constants)?;
                            instructions.push(Instruction::Move {
                                dst: Operand::Local(result_reg),
                                src: Operand::Local(ret_reg),
                            });
                            has_return = true;
                            break;
                        }
                    }
                }
                if !has_return {
                    // 无 return 语句，块值为 Void（result_reg 保持默认 0）
                }

                // 7. 退出 spawn 作用域
                self.exit_scope();
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
                        instructions.push(Instruction::Not {
                            dst: Operand::Local(result_reg),
                            src: Operand::Local(src_reg),
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

                let env_vars = std::mem::take(&mut self.pending_env_vars);
                let env_names = std::mem::take(&mut self.pending_env_names);
                // #254：捕获表（变量名 → env 槽位），供闭包体内 Var 解析 → LoadUpvalue
                self.closure_captures = env_names
                    .iter()
                    .enumerate()
                    .map(|(i, n)| (n.clone(), i))
                    .collect();

                // 5. 生成闭包函数体 IR
                // 类似于 generate_function_ir 的逻辑，但针对 Lambda
                let closure_body =
                    self.generate_lambda_body_ir(params, body.as_ref(), constants)?;
                // #254：闭包体生成完毕，清除捕获表
                self.closure_captures.clear();

                // 6. 创建闭包函数 IR
                let param_types: Vec<MonoType> = params
                    .iter()
                    .filter_map(|p| p.ty.clone())
                    .map(|t| t.into())
                    .collect();

                let closure_func = FunctionIR {
                    def: None, // 由 generate_module_ir 尾部 assign_defs 填充
                    name: closure_name.clone(),
                    params: param_types,
                    return_type,
                    generic_params: None,
                    body: FunctionBody::Code {
                        blocks: vec![BasicBlock {
                            label: 0,
                            instructions: closure_body.instructions,
                            successors: Vec::new(),
                        }],
                        entry: 0,
                        locals: closure_body.locals.clone(),
                    },
                };

                // 7. 将闭包函数添加到嵌套函数列表
                self.nested_functions.push(closure_func);

                // 9. 创建 MakeClosure 指令
                // env 包含被捕获的外部变量的 Operand
                instructions.push(Instruction::MakeClosure {
                    dst: Operand::Local(result_reg),
                    func: closure_name,
                    env: env_vars,
                    def: None,
                });
            }
            Expr::Borrow {
                mutable: _,
                expr,
                span: _,
            } => {
                // 1. 生成内部表达式的 IR
                let inner_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, inner_reg, instructions, constants)?;

                // 借用令牌（& / &mut）是编译期品牌，运行时零大小：
                // Struct 值是堆句柄，传参/赋值复制的是句柄，共享同一对象——
                // 因此字段写（StoreField）经令牌自然写回底层，无需额外指令（#266）。
                // 所有权与借用合法性已在 typecheck 层验证（RFC-009a）。
                instructions.push(Instruction::Move {
                    dst: Operand::Local(result_reg),
                    src: Operand::Local(inner_reg),
                });
            }
            Expr::Match {
                expr: match_expr,
                arms,
                span: _,
            } => {
                // Match 表达式 IR 生成
                // 模式: match scrutinee { pat1 => body1, pat2 => body2, _ => bodyN }
                //
                // IR 结构:
                //   1. 评估 scrutinee
                //   2. 对每个 arm:
                //      a. 如果模式是 Literal: 比较 scrutinee == literal, JmpIfNot 到下一个 arm
                //      b. 如果模式是 Wildcard: 始终匹配
                //      c. 生成 arm body, Move 结果到 result_reg, Jmp 到 end
                //   3. 修复所有跳转目标

                // 1. 评估 scrutinee
                let scrutinee_reg = self.next_temp_reg();
                self.generate_expr_ir(match_expr, scrutinee_reg, instructions, constants)?;

                let mut jumps_to_end: Vec<usize> = Vec::new();

                for arm in arms {
                    // 检查模式是否匹配
                    let needs_condition = matches!(arm.pattern, ast::Pattern::Wildcard);

                    let jump_to_next_idx = if needs_condition {
                        // Wildcard: 始终匹配，不需条件跳转
                        None
                    } else {
                        // 生成条件: 比较 scrutinee 和模式值
                        let cmp_reg = self.next_temp_reg();

                        match &arm.pattern {
                            ast::Pattern::Literal(lit) => {
                                let const_val = match lit {
                                    ast::Literal::Int(n) => ConstValue::Int(*n),
                                    ast::Literal::Float(f) => ConstValue::Float(*f),
                                    ast::Literal::Bool(b) => ConstValue::Bool(*b),
                                    ast::Literal::String(s) => ConstValue::String(s.clone()),
                                    ast::Literal::Char(c) => ConstValue::Char(*c),
                                    ast::Literal::Void => ConstValue::Void,
                                };
                                constants.push(const_val.clone());
                                instructions.push(Instruction::Load {
                                    dst: Operand::Local(cmp_reg),
                                    src: Operand::Const(const_val),
                                });
                            }
                            _ => {
                                // 不支持的 pattern: 加载 0，总会跳到下一个 arm
                                instructions.push(Instruction::Load {
                                    dst: Operand::Local(cmp_reg),
                                    src: Operand::Const(ConstValue::Int(0)),
                                });
                            }
                        }

                        // 比较: scrutinee == pattern_value
                        let eq_reg = self.next_temp_reg();
                        instructions.push(Instruction::Eq {
                            dst: Operand::Local(eq_reg),
                            lhs: Operand::Local(scrutinee_reg),
                            rhs: Operand::Local(cmp_reg),
                        });

                        // 如果不相等，跳到下一个 arm
                        let jmp_idx = instructions.len();
                        instructions.push(Instruction::JmpIfNot(
                            Operand::Local(eq_reg),
                            0, // 占位符
                        ));
                        Some(jmp_idx)
                    };

                    // 生成 arm body，结果放入 result_reg
                    let arm_result_reg = self.next_temp_reg();
                    self.generate_block_ir(
                        &arm.body,
                        Some(arm_result_reg),
                        instructions,
                        constants,
                    )?;
                    instructions.push(Instruction::Move {
                        dst: Operand::Local(result_reg),
                        src: Operand::Local(arm_result_reg),
                    });

                    // 跳转到 match 结束
                    let jmp_end_idx = instructions.len();
                    instructions.push(Instruction::Jmp(0)); // 占位符
                    jumps_to_end.push(jmp_end_idx);

                    // 修复条件跳转目标（指向当前 arm 之后的代码）
                    if let Some(jmp_idx) = jump_to_next_idx {
                        let current_pos = instructions.len();
                        if let Instruction::JmpIfNot(_, ref mut target) = instructions[jmp_idx] {
                            *target = current_pos;
                        }
                    }
                }

                // 修复所有跳转到结束的指令
                let end_pos = instructions.len();
                for idx in jumps_to_end {
                    if let Instruction::Jmp(ref mut target) = instructions[idx] {
                        *target = end_pos;
                    }
                }
            }
            // RFC-012: F-string 代码生成
            Expr::FString { segments, span } => {
                // 1. 尝试常量求值
                if let Some(const_val) = self.eval_const_expr(expr) {
                    constants.push(const_val.clone());
                    instructions.push(Instruction::Load {
                        dst: Operand::Local(result_reg),
                        src: Operand::Const(const_val),
                    });
                    return Ok(());
                }

                // 2. 转换为 format() 调用
                // 构建 format_str: "Hello {} is {} years old"
                // 构建 args: [name, age]
                let mut format_str = String::new();
                let mut arg_regs = Vec::new();
                let mut arg_index = 0usize;

                for segment in segments {
                    match segment {
                        ast::FStringSegment::Text(text) => {
                            format_str.push_str(text);
                        }
                        ast::FStringSegment::Interpolation {
                            expr: interp_expr,
                            format_spec,
                        } => {
                            // Build format placeholder: {0}, {1}, or {0:.2f}
                            if let Some(spec) = format_spec {
                                format_str.push_str(&format!("{{{0}:{1}}}", arg_index, spec));
                            } else {
                                format_str.push_str(&format!("{{{}}}", arg_index));
                            }
                            arg_index += 1;

                            // Generate IR for the interpolation expression
                            let arg_reg = self.next_temp_reg();
                            self.generate_expr_ir(interp_expr, arg_reg, instructions, constants)?;
                            arg_regs.push(Operand::Local(arg_reg));
                        }
                    }
                }

                // Load format string constant
                let fmt_reg = self.next_temp_reg();
                let fmt_const = ConstValue::String(format_str);
                constants.push(fmt_const.clone());
                instructions.push(Instruction::Load {
                    dst: Operand::Local(fmt_reg),
                    src: Operand::Const(fmt_const),
                });

                // Build args: [format_str, arg0, arg1, ...]
                let mut call_args = vec![Operand::Local(fmt_reg)];
                call_args.extend(arg_regs);

                // Generate Call to std.string.format
                instructions.push(Instruction::Call {
                    dst: Some(Operand::Local(result_reg)),
                    func: Operand::Const(ConstValue::String("std.string.format".to_string())),
                    args: call_args,
                    span: *span,
                    def: None,
                });
            }
            Expr::Tuple(items, _span) => {
                // SPEC §3.6 元组字面量：逐个求值元素，用 NewTuple 一次性构造
                let mut item_regs = Vec::with_capacity(items.len());
                for item_expr in items {
                    let item_reg = self.next_temp_reg();
                    self.generate_expr_ir(item_expr, item_reg, instructions, constants)?;
                    item_regs.push(Operand::Local(item_reg));
                }
                instructions.push(Instruction::NewTuple {
                    dst: Operand::Local(result_reg),
                    items: item_regs,
                });
            }
            Expr::Block(block) => {
                // 语句位置块表达式（SPEC §12.5 good_seq）：逐语句生成，
                // 最后表达式值写入 result_reg（复用块 IR 生成）。
                self.generate_block_ir(block, Some(result_reg), instructions, constants)?;
            }
            other => {
                // 未实现的表达式变体：硬错误，禁止静默归零（#251：&&/|| 曾被同类兜底吞掉）
                return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                    "unhandled expression in IR generation: {:?}",
                    std::mem::discriminant(other)
                ))
                .at(Self::get_expr_span(other))
                .build());
            }
        }
        Ok(())
    }
}

/// 这是编译器流程中的关键入口点：
/// 类型检查 → IR 生成 → 代码生成
pub fn generate_ir(
    ast: &crate::frontend::core::parser::ast::Module,
    result: &crate::frontend::core::typecheck::TypeCheckResult,
) -> Result<crate::middle::ModuleIR, Vec<Diagnostic>> {
    // 单文件模式：仅 std 注册表，module_key 为 None → 不启用模块限定。
    let mut generator =
        AstToIrGenerator::new_with_type_result(result, ModuleRegistry::with_std(), None);
    generator.generate_module_ir(ast)
}

/// RFC-029 多文件编排：在跨文件上下文下为单个文件生成 IR。
///
/// 预注册其他文件的类型布局（`cross_file_types`）与全局变量名（`cross_file_globals`），
/// 使本文件的跨文件字段访问、方法调用与全局引用能正确解析；随后仅对 `ast`
/// 这一个文件生成 IR。各文件产出的 `ModuleIR` 由编排器在 IR 层链接。
pub fn generate_ir_with_context(
    ast: &crate::frontend::core::parser::ast::Module,
    result: &crate::frontend::core::typecheck::TypeCheckResult,
    cross_file_types: &[&crate::frontend::core::parser::ast::Module],
    cross_file_globals: &[(String, MonoType)],
    registry: &ModuleRegistry,
    module_key: &str,
) -> Result<crate::middle::ModuleIR, Vec<Diagnostic>> {
    let mut generator = AstToIrGenerator::new_with_type_result(
        result,
        registry.clone(),
        Some(module_key.to_string()),
    );
    generator.seed_cross_file_types(cross_file_types);
    generator.seed_cross_file_globals(cross_file_globals);
    generator.generate_module_ir(ast)
}
