//! 单态化器
//!
//! 将泛型函数和泛型类型特化为具体类型的代码。
//! 核心策略：
//! 1. 按需特化：只对实际使用的类型组合生成代码
//! 2. 队列驱动：BFS 处理实例化请求，自动处理嵌套泛型调用

use std::collections::{HashMap, HashSet, VecDeque};

pub mod function;
pub mod instance;
pub mod type_mono;

use function::FunctionMonomorphizer;
use instance::{GenericFunctionId, InstantiationRequest, SpecializationKey};
use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, ModuleIR, Operand};

/// 单态化器
pub struct Monomorphizer {
    /// 泛型函数定义（从 IR 收集）
    generic_functions: HashMap<String, FunctionIR>,
    /// 已生成的特化函数
    specialized_functions: HashMap<String, FunctionIR>,
    /// 待处理的实例化队列
    pending_queue: VecDeque<InstantiationRequest>,
    /// 已处理的请求（去重）
    processed: HashSet<SpecializationKey>,
    /// 最大递归深度
    max_depth: usize,
}

impl Monomorphizer {
    pub fn new() -> Self {
        Self {
            generic_functions: HashMap::new(),
            specialized_functions: HashMap::new(),
            pending_queue: VecDeque::new(),
            processed: HashSet::new(),
            max_depth: 100,
        }
    }

    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            max_depth,
            ..Self::new()
        }
    }

    /// 核心入口：单态化 ModuleIR
    pub fn monomorphize(
        &mut self,
        module: &ModuleIR,
        requests: &[InstantiationRequest],
    ) -> ModuleIR {
        // 1. 收集泛型函数定义
        self.collect_generic_functions(module);

        // 2. 初始化队列
        for req in requests {
            self.pending_queue.push_back(req.clone());
        }

        // 3. 队列循环（BFS）
        self.process_queue();

        // 4. 构建输出
        let mut output = self.build_output(module);

        // 5. 替换调用点
        self.replace_call_sites(&mut output, requests);

        output
    }

    fn collect_generic_functions(
        &mut self,
        module: &ModuleIR,
    ) {
        for func in &module.functions {
            if func.generic_params.is_some() {
                self.generic_functions
                    .insert(func.name.clone(), func.clone());
            }
        }
    }

    fn process_queue(&mut self) {
        while let Some(req) = self.pending_queue.pop_front() {
            let key = req.specialization_key();

            if self.processed.contains(&key) {
                continue;
            }
            self.processed.insert(key);

            if let Some(specialized) = self.specialize_function(&req) {
                self.scan_for_new_calls(&specialized);
                self.specialized_functions
                    .insert(specialized.name.clone(), specialized);
            }
        }
    }

    fn build_output(
        &self,
        module: &ModuleIR,
    ) -> ModuleIR {
        let mut functions: Vec<FunctionIR> = module
            .functions
            .iter()
            .filter(|f| f.generic_params.is_none())
            .cloned()
            .collect();

        for func in self.specialized_functions.values() {
            functions.push(func.clone());
        }

        ModuleIR {
            functions,
            ..module.clone()
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
        let new_locals: Vec<MonoType> = generic
            .locals
            .iter()
            .map(|ty| self.substitute_single_type(ty, &type_map))
            .collect();

        // 替换指令中的类型
        let new_blocks: Vec<BasicBlock> = generic
            .blocks
            .iter()
            .map(|block| self.substitute_block(block, &type_map))
            .collect();

        // 生成特化后的函数名: identity → identity(Int)
        let type_args_str = type_args
            .iter()
            .map(|t| t.type_name())
            .collect::<Vec<_>>()
            .join(", ");
        let specialized_name = format!("{}({})", generic.name, type_args_str);

        // 构建特化函数
        Some(FunctionIR {
            name: specialized_name,
            params: new_params,
            return_type: new_return_type,
            locals: new_locals,
            blocks: new_blocks,
            entry: generic.entry,
            generic_params: None, // 清除泛型标记
        })
    }

    /// 扫描特化函数体中的泛型调用，将新发现的实例化请求加入队列
    fn scan_for_new_calls(
        &mut self,
        func: &FunctionIR,
    ) {
        for instr in func.all_instructions() {
            if let crate::middle::core::ir::Instruction::Call {
                func: callee, args, ..
            } = instr
            {
                // 从调用操作数提取被调用函数名
                let callee_name = match callee {
                    Operand::Const(ConstValue::String(name)) => name.clone(),
                    _ => continue,
                };

                // 检查被调用函数是否是已知的泛型函数
                if !self.generic_functions.contains_key(&callee_name) {
                    continue;
                }

                let generic_func = match self.generic_functions.get(&callee_name) {
                    Some(f) => f,
                    None => continue,
                };

                let type_params = match &generic_func.generic_params {
                    Some(p) => p,
                    None => continue,
                };

                // 从 args 中尝试推断类型参数
                let arg_types: Vec<MonoType> = args
                    .iter()
                    .filter_map(|op| self.operand_to_type_hint(op, func))
                    .collect();

                // 如果无法推断任何参数类型，跳过
                if arg_types.is_empty() {
                    continue;
                }

                // 使用推断的参数类型创建实例化请求
                // 简单启发式：使用第一个参数的类型作为泛型参数
                if type_params.len() == 1 {
                    let type_arg = arg_types[0].clone();
                    let key = SpecializationKey::new(callee_name.clone(), vec![type_arg.clone()]);

                    if !self.processed.contains(&key) {
                        let req = InstantiationRequest::new(
                            GenericFunctionId::new(callee_name.clone(), type_params.clone()),
                            vec![type_arg],
                            crate::util::span::Span::default(),
                        );
                        self.pending_queue.push_back(req);
                    }
                }
            }
        }
    }

    /// 从特化函数中获取操作数对应的类型提示
    fn operand_to_type_hint(
        &self,
        op: &Operand,
        func: &FunctionIR,
    ) -> Option<MonoType> {
        match op {
            Operand::Local(idx) => func.locals.get(*idx).cloned(),
            Operand::Arg(idx) => {
                if *idx < func.params.len() {
                    Some(func.params[*idx].clone())
                } else {
                    None
                }
            }
            Operand::Const(cv) => match cv {
                ConstValue::Int(_) => Some(MonoType::Int(64)),
                ConstValue::Float(_) => Some(MonoType::Float(64)),
                ConstValue::Bool(_) => Some(MonoType::Bool),
                ConstValue::String(_) => Some(MonoType::String),
                ConstValue::Char(_) => Some(MonoType::Char),
                ConstValue::Void => Some(MonoType::Void),
                _ => None,
            },
            Operand::Temp(idx) => func.locals.get(*idx).cloned(),
            _ => None,
        }
    }

    /// 替换非泛型函数中对泛型函数的调用为特化函数名
    pub fn replace_call_sites(
        &self,
        module: &mut ModuleIR,
        requests: &[InstantiationRequest],
    ) {
        // 构建调用点映射：generic_name -> specialized_name
        let call_site_map = self.build_call_site_map(requests);

        // 遍历所有非泛型函数，替换调用点
        for func in &mut module.functions {
            if func.generic_params.is_none() {
                self.replace_calls_in_function(func, &call_site_map);
            }
        }
    }

    /// 构建泛型函数名到特化函数名的映射
    fn build_call_site_map(
        &self,
        requests: &[InstantiationRequest],
    ) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for req in requests {
            let generic_name = req.generic_id().name().to_string();

            // 只处理已知的泛型函数
            if !self.generic_functions.contains_key(&generic_name) {
                continue;
            }

            let type_args_str = req
                .type_args()
                .iter()
                .map(|t| t.type_name())
                .collect::<Vec<_>>()
                .join(", ");
            let specialized_name = format!("{}({})", generic_name, type_args_str);
            map.insert(generic_name, specialized_name);
        }
        map
    }

    /// 替换单个函数中所有 Call 指令的泛型函数名为特化函数名
    fn replace_calls_in_function(
        &self,
        func: &mut FunctionIR,
        call_site_map: &HashMap<String, String>,
    ) {
        for block in &mut func.blocks {
            for instr in &mut block.instructions {
                if let Instruction::Call {
                    func: ref mut callee,
                    ..
                } = instr
                {
                    if let Operand::Const(ConstValue::String(name)) = callee {
                        if let Some(specialized_name) = call_site_map.get(name) {
                            *callee = Operand::Const(ConstValue::String(specialized_name.clone()));
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
