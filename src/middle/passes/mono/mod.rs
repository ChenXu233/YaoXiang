//! 单态化器
//!
//! 将泛型函数和泛型类型特化为具体类型的代码。
//! 核心策略：
//! 1. 按需特化：只对实际使用的类型组合生成代码
//! 2. 队列驱动：BFS 处理实例化请求，自动处理嵌套泛型调用

use std::collections::{HashMap, HashSet, VecDeque};
use crate::util::diagnostic::Diagnostic;
use crate::util::diagnostic::ErrorCodeDefinition;

pub mod function;
pub mod instance;

use function::FunctionMonomorphizer;
use instance::{GenericFunctionId, InstantiationRequest, SpecializationKey};
use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, LocalSlot, ModuleIR, Operand,
};

/// 是否含符号化类型实参（TypeRef(泛型参数名)）——#335 路径 A 的 deferred 分流依据
fn contains_type_ref(ty: &MonoType) -> bool {
    match ty {
        MonoType::TypeRef(_) => true,
        MonoType::Generic { args, .. } => args.iter().any(contains_type_ref),
        MonoType::Fn {
            params,
            return_type,
        } => params.iter().any(contains_type_ref) || contains_type_ref(return_type),
        MonoType::Ref { inner, .. } => contains_type_ref(inner),
        MonoType::Union(ts) | MonoType::Intersection(ts) => ts.iter().any(contains_type_ref),
        MonoType::Refined { base, .. } => contains_type_ref(base),
        _ => false,
    }
}

/// 单态化器
pub struct Monomorphizer {
    /// 泛型函数定义（从 IR 收集）
    generic_functions: HashMap<String, FunctionIR>,
    /// 泛型类型定义（从 IR 收集）
    generic_types: HashMap<String, FunctionIR>,
    /// 已生成的特化函数
    specialized_functions: HashMap<String, FunctionIR>,
    /// 待处理的实例化队列
    pending_queue: VecDeque<InstantiationRequest>,
    /// 占位请求桶（#335 路径 A）：键 = 所在泛型函数名。含 TypeRef(参数名)
    /// 符号化实参的请求挂此，待所在函数特化时以 name_map 求值入队
    deferred: HashMap<String, Vec<InstantiationRequest>>,
    /// 实际入队过的请求（含 deferred 求值产物）——调用点三元键映射的数据源。
    /// 原始请求切片含未求值的符号化请求，不能直接用于改写
    site_requests: Vec<InstantiationRequest>,
    /// 已处理的请求（去重）
    processed: HashSet<SpecializationKey>,
    /// 最大特化链深度（同一实例化链上嵌套泛型调用的层数；
    /// 拦截无限类型增长递归，如 f(T) → f(List(T)) → …）
    max_depth: usize,
    /// 实例化总数上限（#335：与递归深度分离——合法程序可含大量
    /// 互不相同的实例化，不应触发深度保护）
    max_total_instantiations: usize,
}

impl Monomorphizer {
    pub fn new() -> Self {
        Self {
            generic_functions: HashMap::new(),
            generic_types: HashMap::new(),
            specialized_functions: HashMap::new(),
            pending_queue: VecDeque::new(),
            deferred: HashMap::new(),
            site_requests: Vec::new(),
            processed: HashSet::new(),
            max_depth: 100,
            max_total_instantiations: 10_000,
        }
    }

    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            max_depth,
            ..Self::new()
        }
    }

    /// 设置实例化总数上限（与递归深度上限分离，#335）
    pub fn with_max_instantiations(
        mut self,
        max_total: usize,
    ) -> Self {
        self.max_total_instantiations = max_total;
        self
    }

    /// 核心入口：单态化 ModuleIR
    ///
    /// # Errors
    /// 当单态化实例化深度超过 `max_depth` 时返回 `Diagnostic` 错误，
    /// 表明可能存在无限泛型递归（例如泛型函数无限递归调用自身）。
    pub fn monomorphize(
        &mut self,
        module: &ModuleIR,
        requests: &[InstantiationRequest],
    ) -> Result<ModuleIR, Diagnostic> {
        // 1. 收集泛型定义（函数和类型）
        self.collect_generic_definitions(module);

        // 3. 初始化队列：含符号化 TypeRef 实参的请求进 deferred 桶
        //（泛型体内嵌套调用的实参在此形态——此前直接特化出 pair2(T, string)
        //  等符号名函数，靠解释器类型擦除掩盖），其余正常入队
        for req in requests {
            let symbolic = req.type_args.iter().any(contains_type_ref);
            if symbolic {
                if let Some(cf) = &req.containing_fn {
                    self.deferred
                        .entry(cf.clone())
                        .or_default()
                        .push(req.clone());
                    continue;
                }
            }
            self.site_requests.push(req.clone());
            self.pending_queue.push_back(req.clone());
        }

        // 4. 队列循环（BFS）
        self.process_queue()?;

        // 5. 构建输出
        let mut output = self.build_output(module);

        // 5b. t3 闭合闸：删除判据（按泛型名）与改写判据（按 (containing_fn, 名, span)
        //     三元键）粒度不同——未覆盖的调用点在原件被删后会悬空（运行期 E6006）。
        //     放回「仍被按名调用且调用点不在改写映射里」的原件，使两个判据闭合。
        self.restore_generics_with_uncovered_call_sites(module, &mut output);

        // 6. 替换调用点
        self.replace_call_sites(&mut output);

        Ok(output)
    }

    fn collect_generic_definitions(
        &mut self,
        module: &ModuleIR,
    ) {
        for func in &module.functions {
            if func.generic_params.is_some() {
                if func.is_type_decl() {
                    self.generic_types.insert(func.name.clone(), func.clone());
                } else {
                    self.generic_functions
                        .insert(func.name.clone(), func.clone());
                }
            }
        }
    }

    fn process_queue(&mut self) -> Result<(), Diagnostic> {
        while let Some(req) = self.pending_queue.pop_front() {
            let key = req.specialization_key();

            if self.processed.contains(&key) {
                continue;
            }

            // #335：两个上限分离——
            // 深度 = 同一特化链上的嵌套层数（拦截无限类型增长递归）
            if req.depth > self.max_depth {
                return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                    "泛型特化链深度超过最大限制 ({}，当前 {})，可能存在无限类型增长递归（如泛型函数以自嵌套类型递归调用自身）",
                    self.max_depth, req.depth
                ))
                .at(req.source_location)
                .build());
            }
            // 总数 = 不同实例化的规模上限（合法大程序可含大量互不相同的实例化）
            if self.processed.len() >= self.max_total_instantiations {
                return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                    "单态化实例化总数超过上限 ({})；如为合法的大规模泛型使用，请调高 mono.max_instantiations 配置",
                    self.max_total_instantiations
                ))
                .at(req.source_location)
                .build());
            }

            // #335：实例化失败显式报错——此前 `if let Some(spec)` 静默跳过，
            // 下游症状是 E6006「函数表缺失」，根因不可见
            let generic_name = req.generic_id().name().to_string();
            let n_type_params = if let Some(g) = self.generic_types.get(&generic_name) {
                g.generic_params.as_ref().map(|p| p.len()).unwrap_or(0)
            } else if let Some(g) = self.generic_functions.get(&generic_name) {
                g.generic_params.as_ref().map(|p| p.len()).unwrap_or(0)
            } else {
                // 请求目标不在 IR 泛型定义表中（限定名/native 等历史宽松路径）
                continue;
            };
            let type_args_len = req.type_args().len();
            if type_args_len != n_type_params {
                return Err(ErrorCodeDefinition::ir_instantiation_failed(
                    &generic_name,
                    &format!("类型实参数不匹配：期望 {n_type_params} 个，得到 {type_args_len} 个"),
                )
                .at(req.source_location)
                .build());
            }

            self.processed.insert(key);

            // 先尝试类型特化，再尝试函数特化
            let specialized = if self.generic_types.contains_key(&generic_name) {
                self.specialize_type(&req)
            } else {
                self.specialize_function(&req)
            };

            match specialized {
                Some(spec) => {
                    let child_depth = req.depth + 1;
                    // #335 路径 A：本次特化建立 泛型参数名 → 具体实参 绑定，
                    // 据此求值挂在所在泛型函数下的占位请求（每个特化实例各一次）
                    let param_names: Vec<String> =
                        if let Some(g) = self.generic_types.get(&generic_name) {
                            g.generic_params.clone().unwrap_or_default()
                        } else if let Some(g) = self.generic_functions.get(&generic_name) {
                            g.generic_params.clone().unwrap_or_default()
                        } else {
                            Vec::new()
                        };
                    let name_map: HashMap<String, MonoType> = param_names
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i < req.type_args().len())
                        .map(|(i, n)| (n.clone(), req.type_args()[i].clone()))
                        .collect();
                    self.fire_deferred(&generic_name, &spec.name, &name_map, child_depth);
                    self.scan_for_generic_types(&spec, child_depth);
                    self.specialized_functions.insert(spec.name.clone(), spec);
                }
                None => {
                    return Err(ErrorCodeDefinition::ir_instantiation_failed(
                        &generic_name,
                        "泛型定义缺少泛型参数或类型体，无法特化",
                    )
                    .at(req.source_location)
                    .build());
                }
            }
        }
        Ok(())
    }

    /// 收集一个函数体内所有「按名调用的目标名」（`Call`/`CallStatic` 的字符串操作数）。
    ///
    /// 用于判断一个未特化的泛型函数是否仍被调用——被调用就必须保留。
    fn call_target_names(func: &FunctionIR) -> Vec<String> {
        let FunctionBody::Code { blocks, .. } = &func.body else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for block in blocks {
            for instr in &block.instructions {
                if let Instruction::Call {
                    func: Operand::Const(ConstValue::String(name)),
                    ..
                } = instr
                {
                    out.push(name.clone());
                }
            }
        }
        out
    }

    fn build_output(
        &self,
        module: &ModuleIR,
    ) -> ModuleIR {
        // 保留哪些函数？
        //
        // 1. 非泛型函数——当然保留。
        // 2. 泛型 TypeDecl 构造器——typecheck 对构造器调用不生成 request，
        //    mono 启用时若删除则函数表缺 Entry → 运行时 E6006（#255）。
        // 3. 泛型函数但**未产生任何特化**且**仍被某处按名调用**——
        //    `std.list` 系列（`list.yx` 里声明为 `(A: Type) -> ...`）就是这类：
        //    它们随嵌入 std 合并进来，调用点写的是 `std.list.push`（原名叫，
        //    不是特化名）。若一并删掉，一旦别的泛型（如 `mk(Int)`）触发 mono，
        //    `std.list.push` 就凭空消失，运行时报「Native function not found」
        //    （实测：不调 `mk` 就正常，一调就崩）。
        // 本次请求涉及的泛型名（这些会被特化版替换，原泛型应删）。
        // 用 `self.site_requests` 而非全部请求：`build_output` 在队列处理完后调用，
        // 此时 `site_requests` 就是最终确定要特化的集合。
        let requested_generic_names: std::collections::HashSet<String> = self
            .site_requests
            .iter()
            .map(|r| r.generic_id().name().to_string())
            .collect();
        let names_called_by_name: std::collections::HashSet<String> = module
            .functions
            .iter()
            .flat_map(Self::call_target_names)
            // init 段（模块顶层语句）的按名调用同属保留依据：它不在
            // module.functions 里，漏扫会让「只被顶层调用的泛型」在别的泛型
            // 触发特化时从函数表消失（与 std.list.push 同型，运行时 E6006）
            .chain(
                module
                    .init
                    .iter()
                    .filter_map(|instr| match instr {
                        Instruction::Call {
                            func: Operand::Const(ConstValue::String(name)),
                            ..
                        } => Some(name.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            )
            .collect();

        let mut functions: Vec<FunctionIR> = module
            .functions
            .iter()
            .filter(|f| {
                if f.generic_params.is_none() || f.is_type_decl() {
                    return true;
                }
                // 已被特化的泛型：删原泛型，保留特化版（特化版由下方追加）。
                // 判据用「实例化请求里的泛型名」而非特化名前缀——
                // 特化名形如 `identity(int64)`，前缀匹配无法区分
                // `identity` 与 `identity2` 这类同名前缀。
                if requested_generic_names.contains(f.name.as_str()) {
                    return false;
                }
                // 未产生实例化请求，但仍有调用点按名引用（如嵌入 std 的
                // `std.list.push`）：必须保留，否则一旦别的泛型触发 mono，
                // 这些函数的定义就从函数表里消失，运行时「function not found」。
                f.name.starts_with("std.") || names_called_by_name.contains(&f.name)
            })
            .cloned()
            .collect();

        // HashMap 迭代顺序随进程随机化，直接发射会让同一份源码产出不同的
        // 函数序号（进而 .42 字节不同、dump 不可复现）。按名排序固定顺序。
        let mut specialized: Vec<&FunctionIR> = self.specialized_functions.values().collect();
        specialized.sort_by(|a, b| a.name.cmp(&b.name));
        for func in specialized {
            functions.push(func.clone());
        }

        ModuleIR {
            globals: module.globals.clone(),
            functions,
            init: module.init.clone(),
            init_file_ids: module.init_file_ids.clone(),
            init_locals: module.init_locals.clone(),
            ffi_libs: module.ffi_libs.clone(),
            ffi_bindings: module.ffi_bindings.clone(),
            entry_function: module.entry_function.clone(),
            source_files: module.source_files.clone(),
            function_files: module.function_files.clone(),
        }
    }

    /// 特化单个函数：将泛型函数按类型参数替换为具体函数
    fn specialize_function(
        &self,
        req: &InstantiationRequest,
    ) -> Option<FunctionIR> {
        let generic = self.generic_functions.get(req.generic_id().name())?;
        let type_params = generic.generic_params.as_ref()?;
        let type_args = req.type_args();

        // 验证类型参数数量匹配
        if type_args.len() != type_params.len() {
            return None;
        }

        // 创建类型替换表：TypeVar(index) -> 具体类型
        // generic_params 按顺序对应 type_args，TypeVar 的 index 就是它在 generic_params 中的位置
        let type_map: std::collections::HashMap<usize, MonoType> = (0..type_params.len())
            .map(|i| (i, type_args[i].clone()))
            .collect();

        // 替换参数类型
        let new_params: Vec<MonoType> = generic
            .params
            .iter()
            .map(|ty| self.substitute_single_type(ty, &type_map))
            .collect();

        // 替换返回类型
        let new_return_type = self.substitute_single_type(&generic.return_type, &type_map);

        // 替换局部变量类型
        let new_locals: Vec<LocalSlot> = match &generic.body {
            FunctionBody::Code { locals, .. } => locals
                .iter()
                .map(|slot| LocalSlot {
                    name: slot.name.clone(),
                    ty: self.substitute_single_type(&slot.ty, &type_map),
                    scope_depth: slot.scope_depth,
                })
                .collect(),
            _ => Vec::new(),
        };

        // 替换指令中的类型
        let new_blocks: Vec<BasicBlock> = match &generic.body {
            FunctionBody::Code { blocks, .. } => blocks
                .iter()
                .map(|block| self.substitute_block(block, &type_map))
                .collect(),
            _ => Vec::new(),
        };

        let new_entry = match &generic.body {
            FunctionBody::Code { entry, .. } => *entry,
            _ => 0,
        };

        // 生成特化后的函数名: identity → identity(Int)
        let type_args_str = type_args
            .iter()
            .map(|t| t.type_name())
            .collect::<Vec<_>>()
            .join(", ");
        let specialized_name = format!("{}({})", generic.name, type_args_str);

        // RFC-011 §5.2：约束形参的运算符派发改写——泛型定义体的
        // `a + b`（T 带约束）发射 `Call "T.add"`，此处按类型实参改写为
        // `Call "Vec3.add"`（用户方法）或 `Call "Int.add"`（FFI 原生背书）
        let mut blocks = new_blocks;
        for block in &mut blocks {
            for instr in &mut block.instructions {
                if let Instruction::Call {
                    func: Operand::Const(ConstValue::String(name)),
                    ..
                } = instr
                {
                    for (i, pn) in type_params.iter().enumerate() {
                        let prefix = format!("{pn}.");
                        if name.starts_with(&prefix) {
                            if let Some(concrete) = type_args.get(i) {
                                *name = format!(
                                    "{}.{}",
                                    crate::frontend::core::typecheck::operator_interfaces::operator_type_display(concrete),
                                    &name[prefix.len()..]
                                );
                            }
                        }
                    }
                }
            }
        }

        // 构建特化函数
        Some(FunctionIR {
            name: specialized_name,
            // 特化实例不复用原 DefId——mono 路径保持按名分发
            def: None,
            params: new_params,
            return_type: new_return_type,
            generic_params: None, // 清除泛型标记
            body: FunctionBody::Code {
                blocks,
                entry: new_entry,
                locals: new_locals,
            },
        })
    }

    /// 特化类型定义：将泛型类型按类型参数替换为具体类型定义
    fn specialize_type(
        &self,
        req: &InstantiationRequest,
    ) -> Option<FunctionIR> {
        let generic = self.generic_types.get(req.generic_id().name())?;
        let type_params = generic.generic_params.as_ref()?;
        let type_args = req.type_args();

        if type_args.len() != type_params.len() {
            return None;
        }

        // MonoType 替换表：TypeVar(index) → 具体类型
        let type_map: std::collections::HashMap<usize, MonoType> = (0..type_params.len())
            .map(|i| (i, type_args[i].clone()))
            .collect();

        // 名称替换表：参数名 → 具体类型（用于 ast::Type 中的 Type::Name）
        let name_map: std::collections::HashMap<String, MonoType> = type_params
            .iter()
            .zip(type_args.iter())
            .map(|(name, ty)| (name.clone(), ty.clone()))
            .collect();

        // 替换签名
        let new_params: Vec<MonoType> = generic
            .params
            .iter()
            .map(|ty| self.substitute_single_type(ty, &type_map))
            .collect();
        let new_return_type = self.substitute_single_type(&generic.return_type, &type_map);

        // 替换类型体里的类型参数
        let new_definition = match &generic.body {
            FunctionBody::TypeDecl { definition } => {
                self.substitute_type_in_ast(definition, &name_map)
            }
            _ => return None,
        };

        // 特化名：List → List(Int)
        let type_args_str = type_args
            .iter()
            .map(|t| t.type_name())
            .collect::<Vec<_>>()
            .join(", ");
        let specialized_name = format!("{}({})", generic.name, type_args_str);

        Some(FunctionIR {
            name: specialized_name,
            // 特化类型不复用原 DefId——mono 路径保持按名分发
            def: None,
            params: new_params,
            return_type: new_return_type,
            generic_params: None,
            body: FunctionBody::TypeDecl {
                definition: new_definition,
            },
        })
    }

    /// #335 路径 A：以本次特化建立的 泛型参数名 → 具体实参 绑定，求值挂在
    /// `container`（所在泛型函数）下的占位请求，求值后以特化名为 containing_fn
    /// 入队——其体内的嵌套调用随之关联到本特化实例（三元键容器维度）。
    /// 取代已删除的 scan_for_new_calls 单参启发式：嵌套调用的请求由 typecheck
    /// 在收集阶段直接发出（实参为符号化 TypeRef(参数名)，deferred 桶暂存）。
    fn fire_deferred(
        &mut self,
        container: &str,
        specialized_name: &str,
        name_map: &HashMap<String, MonoType>,
        depth: usize,
    ) {
        if let Some(bucket) = self.deferred.get(container).cloned() {
            for r in bucket {
                let new_args: Vec<MonoType> =
                    r.type_args.iter().map(|a| a.substitute(name_map)).collect();
                let mut nr =
                    InstantiationRequest::new(r.generic_id.clone(), new_args, r.source_location);
                nr.depth = depth;
                nr.containing_fn = Some(specialized_name.to_string());
                // 记录求值后的请求：调用点三元键映射的数据源
                self.site_requests.push(nr.clone());
                self.pending_queue.push_back(nr);
            }
        }
    }

    /// 扫描函数体中的 MonoType::Generic 引用，发现类型特化请求
    fn scan_for_generic_types(
        &mut self,
        func: &FunctionIR,
        depth: usize,
    ) {
        // 扫描 params
        for ty in &func.params {
            self.collect_generic_type_refs(ty, depth);
        }

        // 扫描 locals 和指令中的类型
        if let FunctionBody::Code { locals, blocks, .. } = &func.body {
            for slot in locals {
                self.collect_generic_type_refs(&slot.ty, depth);
            }
            for block in blocks {
                for instr in &block.instructions {
                    self.collect_generic_type_refs_from_instr(instr);
                }
            }
        }
    }

    /// 从 MonoType 中收集泛型类型引用
    fn collect_generic_type_refs(
        &mut self,
        ty: &MonoType,
        depth: usize,
    ) {
        match ty {
            MonoType::Generic { name, args } => {
                if self.generic_types.contains_key(name) {
                    let key = SpecializationKey::new(name.clone(), args.clone());
                    if !self.processed.contains(&key) {
                        let mut req = InstantiationRequest::new(
                            GenericFunctionId::new(name.clone(), Vec::new()),
                            args.clone(),
                            crate::util::span::Span::default(),
                        );
                        req.depth = depth;
                        self.pending_queue.push_back(req);
                    }
                }
                // 递归扫描参数（嵌套泛型：List(List(Int))）
                for arg in args {
                    self.collect_generic_type_refs(arg, depth);
                }
            }
            MonoType::Fn {
                params,
                return_type,
            } => {
                for t in params {
                    self.collect_generic_type_refs(t, depth);
                }
                self.collect_generic_type_refs(return_type, depth);
            }
            _ => {}
        }
    }

    /// 从指令中收集泛型类型引用
    fn collect_generic_type_refs_from_instr(
        &mut self,
        _instr: &Instruction,
    ) {
        // 指令中的类型信息主要通过操作数间接携带
        // 嵌套泛型函数调用由 typecheck 请求（containing_fn 占位机制）覆盖，
        // 未来如果指令直接携带 MonoType，可在此扩展
    }

    /// 替换非泛型函数中对泛型函数的调用为特化函数名
    pub fn replace_call_sites(
        &self,
        module: &mut ModuleIR,
    ) {
        // #335 路径 A：调用点映射三元键 (所在函数, 泛型名, span)——同一源码
        // span 会复制进多个特化实例（outer(int64)/outer(string) 各持一份），
        // 二元键下互相覆盖；三元键使每个特化实例改写到各自的嵌套特化
        let call_site_map = self.build_call_site_map(&self.site_requests);

        // 遍历所有非泛型函数，替换调用点（含特化函数体内对其他泛型的嵌套调用）
        for func in &mut module.functions {
            if func.generic_params.is_none() {
                self.replace_calls_in_function(func, &call_site_map);
            }
        }

        // init 段（模块顶层语句）：调用点同样按特化名改写。typecheck 在模块层
        // 收集的请求 containing_fn=None（anon 键）；此前只遍历 functions，
        // 顶层调用点保留原泛型名——原泛型又被特化删除，运行期按原名查表
        // 落空（E6006，「Native function not found」，#416 家族）
        for instr in &mut module.init {
            if let Instruction::Call {
                func: ref mut callee,
                span,
                ..
            } = instr
            {
                if let Operand::Const(ConstValue::String(name)) = callee {
                    if let Some(specialized_name) = call_site_map.get(&(None, name.clone(), *span))
                    {
                        *callee = Operand::Const(ConstValue::String(specialized_name.clone()));
                    }
                }
            }
        }
    }

    /// 构建 (泛型名, 调用点 span) → 特化函数名 的映射
    fn build_call_site_map(
        &self,
        requests: &[InstantiationRequest],
    ) -> HashMap<(Option<String>, String, crate::util::span::Span), String> {
        let mut map = HashMap::new();
        for req in requests {
            let generic_name = req.generic_id().name().to_string();

            // 处理泛型函数和泛型类型（fix #255：泛型类型构造器也需要替换）
            if !self.generic_functions.contains_key(&generic_name)
                && !self.generic_types.contains_key(&generic_name)
            {
                continue;
            }

            let type_args_str = req
                .type_args()
                .iter()
                .map(|t| t.type_name())
                .collect::<Vec<_>>()
                .join(", ");
            let specialized_name = format!("{}({})", generic_name, type_args_str);
            map.insert(
                (req.containing_fn.clone(), generic_name, req.source_location),
                specialized_name,
            );
        }
        map
    }

    /// t3 闭合闸：把「输出里仍被按名调用、但调用点不在改写映射里」的泛型原件放回函数表。
    ///
    /// 缺口成因：`build_output` 的删除判据按**泛型名**（`site_requests` 的名字集合，
    /// mod.rs `requested_generic_names`），而改写判据按 **(containing_fn, 名, span)**
    /// 三元键（[`Self::build_call_site_map`]）。同一泛型的多个调用点里只要有一个
    /// 没有请求键——实参符号化使请求进 `deferred` 桶、且所在函数非泛型时不排水；
    /// 或该调用点从未产出请求——删除原件就让那个调用点悬空：运行期按原名查表落空
    /// （E6006。实证：`src/std/tests/list_ops.yx:53` 的 `list.is_empty(list.empty(Int))`，
    /// 该泛型名的请求只存在于同函数内另一调用点）。
    ///
    /// 判据与改写**同一份映射**：映射里查不到即视为未覆盖。放回的原件让该调用点
    /// 退回类型擦除执行——正是单态化未介入时的既有语义；这不是「一律保留」：
    /// 全部调用点都被覆盖的泛型仍按原判据删除（[`Self::build_output`]）。
    fn restore_generics_with_uncovered_call_sites(
        &self,
        input: &ModuleIR,
        output: &mut ModuleIR,
    ) {
        let call_site_map = self.build_call_site_map(&self.site_requests);
        // 放回的原件体内可能再引用别的已删泛型 → 迭代到不动点。每轮至少放回一个，
        // 上界为输入模块函数数，必然终止。
        loop {
            // 名单**按函数名排序**后追加：与 `build_output` 的特化函数追加同一纪律
            //（`specialized.sort_by(|a, b| a.name.cmp(&b.name))`）。`HashSet` 迭代顺序
            // 随进程/实例随机化（F1 实证：同一输入 4 次构建出 2 个不同的 .42 SHA256），
            // 直接按迭代序追加会让同一份源码产出不同的函数序号——数序号即产物
            //（dump/字节码不可复现）。
            let mut pending: Vec<&FunctionIR> = self
                .uncovered_call_targets(output, &call_site_map)
                .into_iter()
                .filter(|name| !output.functions.iter().any(|f| &f.name == name))
                // 按名目标不是本模块定义（std native 等）——不归本闸管
                .filter_map(|name| input.functions.iter().find(|f| f.name == name))
                .collect();
            if pending.is_empty() {
                break;
            }
            pending.sort_by(|a, b| a.name.cmp(&b.name));
            for original in pending {
                output.functions.push(original.clone());
            }
        }
    }

    /// 输出模块里**按名调用**的目标名中，调用点不在改写映射里的那些（去重）。
    fn uncovered_call_targets(
        &self,
        output: &ModuleIR,
        call_site_map: &HashMap<(Option<String>, String, crate::util::span::Span), String>,
    ) -> HashSet<String> {
        let mut names = HashSet::new();
        for func in &output.functions {
            let FunctionBody::Code { blocks, .. } = &func.body else {
                continue;
            };
            for block in blocks {
                for instr in &block.instructions {
                    let Some((callee, span)) = Self::by_name_call_target(instr) else {
                        continue;
                    };
                    // 改写面只覆盖 `Call`（[`Self::call_form_is_rewritten`]）：
                    // `TailCall` / `MakeClosure` 的按名目标永不被替换成特化名，
                    // 故**一律计未覆盖**、不查映射键——「有键」只说明该站点在
                    // typecheck 侧产过请求，不代表改写会发生。保守方向：宁保留
                    //（该站点退回类型擦除执行，对应特化版可能成为死码）不悬空。
                    if !Self::call_form_is_rewritten(instr) {
                        names.insert(callee);
                        continue;
                    }
                    let named = (Some(func.name.clone()), callee.clone(), span);
                    let anon = (None, callee.clone(), span);
                    if !call_site_map.contains_key(&named) && !call_site_map.contains_key(&anon) {
                        names.insert(callee);
                    }
                }
            }
        }
        // init 段（模块顶层语句）的按名调用：改写侧用 (None, 名, span) 兜底键；
        // 「非改写面形态一律计未覆盖」同规则适用。
        for instr in &output.init {
            let Some((callee, span)) = Self::by_name_call_target(instr) else {
                continue;
            };
            if !Self::call_form_is_rewritten(instr)
                || !call_site_map.contains_key(&(None, callee.clone(), span))
            {
                names.insert(callee);
            }
        }
        names
    }

    /// 调用点改写面（[`Self::replace_calls_in_function`]）是否覆盖该形态的按名目标。
    ///
    /// **只覆盖 `Call`**（且 func 为字符串常量）：`TailCall` / `MakeClosure` 的按名
    /// 目标不会被替换成特化名。扫描面（[`Self::by_name_call_target`]）比改写面宽，
    /// 不对称只在此单点判定——非改写面形态一律按「未覆盖」保守处理，触发原件保留。
    fn call_form_is_rewritten(instr: &Instruction) -> bool {
        matches!(
            instr,
            Instruction::Call {
                func: Operand::Const(ConstValue::String(_)),
                ..
            }
        )
    }

    /// 指令的**按名调用目标**与调用点 span：IR 里以字符串名分发的三种形态
    ///（`Call` / `TailCall` / `MakeClosure`）。
    ///
    /// 不要据此假定「取到名字即会被改写」：改写面只覆盖 `Call`——某形态是否被
    /// 覆盖由 [`Self::call_form_is_rewritten`] 单点判定；扫描面更宽的那些形态在
    /// [`Self::uncovered_call_targets`] 里一律计未覆盖（保守保留原件，防删除后
    /// 调用点悬空 → 运行期 E6006）。
    fn by_name_call_target(instr: &Instruction) -> Option<(String, crate::util::span::Span)> {
        match instr {
            Instruction::Call {
                func: Operand::Const(ConstValue::String(name)),
                span,
                ..
            }
            | Instruction::TailCall {
                func: Operand::Const(ConstValue::String(name)),
                span,
                ..
            } => Some((name.clone(), *span)),
            Instruction::MakeClosure { func, span, .. } => Some((func.clone(), *span)),
            _ => None,
        }
    }

    /// 替换单个函数中所有 Call 指令的泛型函数名为特化函数名
    ///
    /// 按 (所在函数名, 被调名, 指令 span) 三元精确匹配：IR Call 指令与
    /// typecheck 实例化请求同源持有 AST 调用表达式的 span；containing_fn
    /// 为 None 的请求退化为 (None, 泛型名, span) 兜底键
    fn replace_calls_in_function(
        &self,
        func: &mut FunctionIR,
        call_site_map: &HashMap<(Option<String>, String, crate::util::span::Span), String>,
    ) {
        if let FunctionBody::Code { blocks, .. } = &mut func.body {
            for block in blocks {
                for instr in &mut block.instructions {
                    if let Instruction::Call {
                        func: ref mut callee,
                        span,
                        ..
                    } = instr
                    {
                        if let Operand::Const(ConstValue::String(name)) = callee {
                            let named = (Some(func.name.clone()), name.clone(), *span);
                            let anon = (None, name.clone(), *span);
                            if let Some(specialized_name) = call_site_map
                                .get(&named)
                                .or_else(|| call_site_map.get(&anon))
                            {
                                *callee =
                                    Operand::Const(ConstValue::String(specialized_name.clone()));
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Default for Monomorphizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
