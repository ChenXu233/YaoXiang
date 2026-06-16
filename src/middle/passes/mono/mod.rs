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

use instance::{InstantiationRequest, SpecializationKey};
use crate::middle::core::ir::{FunctionIR, ModuleIR};

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
        self.build_output(module)
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

    /// 特化单个函数（placeholder，Task 6 实现）
    fn specialize_function(
        &self,
        _req: &InstantiationRequest,
    ) -> Option<FunctionIR> {
        // TODO: Task 6 实现
        None
    }

    /// 扫描特化函数中的新泛型调用（placeholder，Task 6 实现）
    fn scan_for_new_calls(
        &mut self,
        _func: &FunctionIR,
    ) {
        // TODO: Task 6 实现
    }

    /// 替换调用点（placeholder，Task 7 实现）
    fn replace_call_sites(
        &self,
        _module: &mut ModuleIR,
        _requests: &[InstantiationRequest],
    ) {
        // TODO: Task 7 实现
    }
}

impl Default for Monomorphizer {
    fn default() -> Self {
        Self::new()
    }
}
