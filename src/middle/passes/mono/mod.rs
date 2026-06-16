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
mod tests {
    use super::*;
    use crate::frontend::core::types::var::TypeVar;
    use crate::middle::core::ir::{BasicBlock, Instruction};

    /// 辅助函数：创建简单的泛型 identity 函数 IR
    /// fn identity<T>(x: T) -> T { return x; }
    fn make_identity_ir() -> FunctionIR {
        let param_type = MonoType::TypeVar(TypeVar::new(0));
        FunctionIR {
            name: "identity".to_string(),
            params: vec![param_type.clone()],
            return_type: param_type.clone(),
            locals: vec![param_type.clone()],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Load {
                        dst: Operand::Local(0),
                        src: Operand::Arg(0),
                    },
                    Instruction::Ret(Some(Operand::Local(0))),
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            generic_params: Some(vec!["T".to_string()]),
        }
    }

    /// 辅助函数：创建泛型 swap 函数 IR
    /// fn swap<T>(a: T, b: T) -> (T, T)
    fn make_swap_ir() -> FunctionIR {
        let t = MonoType::TypeVar(TypeVar::new(0));
        FunctionIR {
            name: "swap".to_string(),
            params: vec![t.clone(), t.clone()],
            return_type: MonoType::Tuple(vec![t.clone(), t.clone()]),
            locals: vec![t.clone(), t.clone()],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Load {
                        dst: Operand::Local(0),
                        src: Operand::Arg(0),
                    },
                    Instruction::Load {
                        dst: Operand::Local(1),
                        src: Operand::Arg(1),
                    },
                    Instruction::Ret(Some(Operand::Local(0))), // 简化：只返回二元组
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            generic_params: Some(vec!["T".to_string()]),
        }
    }

    #[test]
    fn test_specialize_identity_with_int() {
        let _mono = Monomorphizer::new();

        // 注册泛型函数
        let generic = make_identity_ir();

        // 创建实例化请求：identity(Int)
        let req = InstantiationRequest::new(
            GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
            vec![MonoType::Int(64)],
            crate::util::span::Span::default(),
        );

        // 由于 specialize_function 是私有方法，通过 monomorphize 来间接测试
        // 这里先验证 Map 的行为
        let mut mono_mut = Monomorphizer::new();
        mono_mut
            .generic_functions
            .insert("identity".to_string(), generic);

        let result = mono_mut.specialize_function(&req);

        assert!(result.is_some(), "特化应该成功");
        let func = result.unwrap();

        // 验证名称
        assert_eq!(func.name, "identity(int64)");

        // 验证参数类型已被替换
        assert_eq!(func.params.len(), 1);
        assert_eq!(
            func.params[0],
            MonoType::Int(64),
            "参数类型应为 Int(64)，实际为 {:?}",
            func.params[0]
        );

        // 验证返回类型已被替换
        assert_eq!(
            func.return_type,
            MonoType::Int(64),
            "返回类型应为 Int(64)，实际为 {:?}",
            func.return_type
        );

        // 验证局部变量类型已被替换
        assert_eq!(func.locals.len(), 1);
        assert_eq!(func.locals[0], MonoType::Int(64));

        // 验证泛型标记已清除
        assert!(func.generic_params.is_none());

        // 验证指令保留
        assert_eq!(func.blocks.len(), 1);
        assert_eq!(func.blocks[0].instructions.len(), 2);
    }

    #[test]
    fn test_specialize_identity_with_string() {
        let mut mono = Monomorphizer::new();
        let generic = make_identity_ir();
        mono.generic_functions
            .insert("identity".to_string(), generic);

        let req = InstantiationRequest::new(
            GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
            vec![MonoType::String],
            crate::util::span::Span::default(),
        );

        let result = mono.specialize_function(&req);
        assert!(result.is_some());

        let func = result.unwrap();
        assert_eq!(func.name, "identity(string)");
        assert_eq!(func.params[0], MonoType::String);
        assert_eq!(func.return_type, MonoType::String);
        assert!(func.generic_params.is_none());
    }

    #[test]
    fn test_specialize_swap_with_float() {
        let mut mono = Monomorphizer::new();
        let generic = make_swap_ir();
        mono.generic_functions.insert("swap".to_string(), generic);

        let req = InstantiationRequest::new(
            GenericFunctionId::new("swap".to_string(), vec!["T".to_string()]),
            vec![MonoType::Float(64)],
            crate::util::span::Span::default(),
        );

        let result = mono.specialize_function(&req);
        assert!(result.is_some());

        let func = result.unwrap();
        assert_eq!(func.name, "swap(float64)");
        assert_eq!(func.params.len(), 2);
        assert_eq!(func.params[0], MonoType::Float(64));
        assert_eq!(func.params[1], MonoType::Float(64));
        assert_eq!(
            func.return_type,
            MonoType::Tuple(vec![MonoType::Float(64), MonoType::Float(64)])
        );
        assert!(func.generic_params.is_none());
    }

    #[test]
    fn test_specialize_missing_generic_function() {
        let mono = Monomorphizer::new();

        let req = InstantiationRequest::new(
            GenericFunctionId::new("nonexistent".to_string(), vec!["T".to_string()]),
            vec![MonoType::Int(64)],
            crate::util::span::Span::default(),
        );

        let result = mono.specialize_function(&req);
        assert!(result.is_none(), "不存在的泛型函数应返回 None");
    }

    #[test]
    fn test_specialize_type_args_mismatch() {
        let mut mono = Monomorphizer::new();
        let generic = make_identity_ir();
        mono.generic_functions
            .insert("identity".to_string(), generic);

        // 泛型函数有 1 个参数，但提供 2 个 type_args
        let req = InstantiationRequest::new(
            GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
            vec![MonoType::Int(64), MonoType::String],
            crate::util::span::Span::default(),
        );

        let result = mono.specialize_function(&req);
        assert!(result.is_none(), "类型参数数量不匹配应返回 None");
    }

    #[test]
    fn test_specialize_non_generic_function() {
        let mut mono = Monomorphizer::new();
        // 一个没有 generic_params 的普通函数
        let func = FunctionIR {
            name: "add".to_string(),
            params: vec![MonoType::Int(64), MonoType::Int(64)],
            return_type: MonoType::Int(64),
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![],
                successors: Vec::new(),
            }],
            entry: 0,
            generic_params: None,
        };
        mono.generic_functions.insert("add".to_string(), func);

        let req = InstantiationRequest::new(
            GenericFunctionId::new("add".to_string(), vec!["T".to_string()]),
            vec![MonoType::Int(64)],
            crate::util::span::Span::default(),
        );

        let result = mono.specialize_function(&req);
        assert!(
            result.is_none(),
            "非泛型函数特化应返回 None（generic_params 为 None）"
        );
    }

    #[test]
    fn test_specialize_with_generic_type_args() {
        // 测试泛型函数中使用泛型类型参数的特化
        let t = MonoType::TypeVar(TypeVar::new(0));
        let list_t = MonoType::List(Box::new(t.clone()));

        let generic = FunctionIR {
            name: "first".to_string(),
            params: vec![list_t],
            return_type: t,
            locals: vec![MonoType::List(Box::new(MonoType::TypeVar(TypeVar::new(0))))],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(None)],
                successors: Vec::new(),
            }],
            entry: 0,
            generic_params: Some(vec!["T".to_string()]),
        };

        let mut mono = Monomorphizer::new();
        mono.generic_functions.insert("first".to_string(), generic);

        let req = InstantiationRequest::new(
            GenericFunctionId::new("first".to_string(), vec!["T".to_string()]),
            vec![MonoType::String],
            crate::util::span::Span::default(),
        );

        let result = mono.specialize_function(&req);
        assert!(result.is_some());

        let func = result.unwrap();
        // 参数类型应为 List(String)
        assert_eq!(func.params[0], MonoType::List(Box::new(MonoType::String)));
        // 返回类型应为 String
        assert_eq!(func.return_type, MonoType::String);
        // 局部变量类型应为 List(String)
        assert_eq!(func.locals[0], MonoType::List(Box::new(MonoType::String)));
    }

    #[test]
    fn test_scan_for_new_calls_no_generic_calls() {
        let mut mono = Monomorphizer::new();

        // 一个没有泛型调用的特化函数
        let func = FunctionIR {
            name: "simple".to_string(),
            params: vec![],
            return_type: MonoType::Void,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(None)],
                successors: Vec::new(),
            }],
            entry: 0,
            generic_params: None,
        };

        // 不应 panic
        mono.scan_for_new_calls(&func);
        assert!(mono.pending_queue.is_empty());
    }

    #[test]
    fn test_scan_for_new_calls_with_generic_call() {
        let mut mono = Monomorphizer::new();

        // 注册一个泛型函数到 generic_functions
        let callee_ir = make_identity_ir();
        mono.generic_functions
            .insert("identity".to_string(), callee_ir);

        // 创建一个特化函数，其中调用 identity
        let func = FunctionIR {
            name: "wrapper(Int)".to_string(),
            params: vec![MonoType::Int(64)],
            return_type: MonoType::Int(64),
            locals: vec![MonoType::Int(64)],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Call {
                        dst: Some(Operand::Local(0)),
                        func: Operand::Const(ConstValue::String("identity".to_string())),
                        args: vec![Operand::Arg(0)],
                        span: crate::util::span::Span::default(),
                    },
                    Instruction::Ret(Some(Operand::Local(0))),
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            generic_params: None,
        };

        mono.scan_for_new_calls(&func);

        // 应该产生一个新的实例化请求: identity(Int)
        assert_eq!(mono.pending_queue.len(), 1, "应该有一个新的实例化请求");

        let pending = &mono.pending_queue[0];
        assert_eq!(pending.generic_id().name(), "identity");
        assert_eq!(pending.type_args().len(), 1);
        assert_eq!(pending.type_args()[0], MonoType::Int(64));
    }

    #[test]
    fn test_scan_for_new_calls_duplicate_prevented() {
        let mut mono = Monomorphizer::new();

        // 注册泛型函数
        mono.generic_functions
            .insert("identity".to_string(), make_identity_ir());

        // 标记 identity(Int) 已处理
        mono.processed.insert(SpecializationKey::new(
            "identity".to_string(),
            vec![MonoType::Int(64)],
        ));

        // 创建调用 identity 的特化函数
        let func = FunctionIR {
            name: "dup_check".to_string(),
            params: vec![MonoType::Int(64)],
            return_type: MonoType::Int(64),
            locals: vec![MonoType::Int(64)],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Call {
                        dst: Some(Operand::Local(0)),
                        func: Operand::Const(ConstValue::String("identity".to_string())),
                        args: vec![Operand::Arg(0)],
                        span: crate::util::span::Span::default(),
                    },
                    Instruction::Ret(Some(Operand::Local(0))),
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            generic_params: None,
        };

        mono.scan_for_new_calls(&func);

        // 不应该产生重复请求（已经 processed）
        assert!(
            mono.pending_queue.is_empty(),
            "已处理的请求不应重复加入队列"
        );
    }

    #[test]
    fn test_operand_to_type_hint() {
        let mono = Monomorphizer::new();

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(64), MonoType::String],
            return_type: MonoType::Void,
            locals: vec![MonoType::Bool, MonoType::Float(64)],
            blocks: vec![],
            entry: 0,
            generic_params: None,
        };

        // Arg(0) -> Int(64)
        let arg_ty = mono.operand_to_type_hint(&Operand::Arg(0), &func);
        assert_eq!(arg_ty, Some(MonoType::Int(64)));

        // Arg(1) -> String
        let arg_ty = mono.operand_to_type_hint(&Operand::Arg(1), &func);
        assert_eq!(arg_ty, Some(MonoType::String));

        // Arg(99) -> None (越界)
        let arg_ty = mono.operand_to_type_hint(&Operand::Arg(99), &func);
        assert_eq!(arg_ty, None);

        // Local(0) -> Bool
        let local_ty = mono.operand_to_type_hint(&Operand::Local(0), &func);
        assert_eq!(local_ty, Some(MonoType::Bool));

        // Const(Int) -> Int(64)
        let const_ty = mono.operand_to_type_hint(&Operand::Const(ConstValue::Int(42)), &func);
        assert_eq!(const_ty, Some(MonoType::Int(64)));

        // Const(String) -> String
        let const_ty = mono.operand_to_type_hint(
            &Operand::Const(ConstValue::String("hello".to_string())),
            &func,
        );
        assert_eq!(const_ty, Some(MonoType::String));
    }

    #[test]
    fn test_replace_call_sites_basic() {
        let mut mono = Monomorphizer::new();

        // 注册泛型函数 identity
        mono.generic_functions
            .insert("identity".to_string(), make_identity_ir());

        // 创建一个非泛型函数 main，其中调用泛型函数 identity
        let main_func = FunctionIR {
            name: "main".to_string(),
            params: vec![],
            return_type: MonoType::Void,
            locals: vec![MonoType::Int(64)],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Call {
                        dst: Some(Operand::Local(0)),
                        func: Operand::Const(ConstValue::String("identity".to_string())),
                        args: vec![Operand::Const(ConstValue::Int(42))],
                        span: crate::util::span::Span::default(),
                    },
                    Instruction::Ret(Some(Operand::Local(0))),
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            generic_params: None,
        };

        let mut module = ModuleIR {
            functions: vec![main_func],
            ..Default::default()
        };

        let requests = vec![InstantiationRequest::new(
            GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
            vec![MonoType::Int(64)],
            crate::util::span::Span::default(),
        )];

        mono.replace_call_sites(&mut module, &requests);

        // 验证调用点已替换
        let main_func = &module.functions[0];
        if let Instruction::Call { func: callee, .. } = &main_func.blocks[0].instructions[0] {
            assert_eq!(
                *callee,
                Operand::Const(ConstValue::String("identity(int64)".to_string())),
                "Call 指令的 func 应该被替换为特化函数名"
            );
        } else {
            panic!("第一条指令应该是 Call");
        }
    }

    #[test]
    fn test_replace_call_sites_skips_generic_functions() {
        let mono = Monomorphizer::new();

        // 创建一个泛型函数 wrapper，其中调用 identity
        let wrapper_func = FunctionIR {
            name: "wrapper".to_string(),
            params: vec![MonoType::TypeVar(TypeVar::new(0))],
            return_type: MonoType::TypeVar(TypeVar::new(0)),
            locals: vec![MonoType::TypeVar(TypeVar::new(0))],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Call {
                        dst: Some(Operand::Local(0)),
                        func: Operand::Const(ConstValue::String("identity".to_string())),
                        args: vec![Operand::Arg(0)],
                        span: crate::util::span::Span::default(),
                    },
                    Instruction::Ret(Some(Operand::Local(0))),
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            generic_params: Some(vec!["T".to_string()]),
        };

        let mut module = ModuleIR {
            functions: vec![wrapper_func],
            ..Default::default()
        };

        let requests = vec![InstantiationRequest::new(
            GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
            vec![MonoType::Int(64)],
            crate::util::span::Span::default(),
        )];

        mono.replace_call_sites(&mut module, &requests);

        // 泛型函数内的调用不应被替换
        let wrapper = &module.functions[0];
        if let Instruction::Call { func: callee, .. } = &wrapper.blocks[0].instructions[0] {
            assert_eq!(
                *callee,
                Operand::Const(ConstValue::String("identity".to_string())),
                "泛型函数内的调用不应被替换"
            );
        } else {
            panic!("第一条指令应该是 Call");
        }
    }

    #[test]
    fn test_replace_call_sites_multiple_functions() {
        let mut mono = Monomorphizer::new();

        // 注册泛型函数 identity
        mono.generic_functions
            .insert("identity".to_string(), make_identity_ir());

        // 两个非泛型函数都调用 identity
        let func_a = FunctionIR {
            name: "a".to_string(),
            params: vec![],
            return_type: MonoType::Void,
            locals: vec![MonoType::Int(64)],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Call {
                    dst: Some(Operand::Local(0)),
                    func: Operand::Const(ConstValue::String("identity".to_string())),
                    args: vec![Operand::Const(ConstValue::Int(1))],
                    span: crate::util::span::Span::default(),
                }],
                successors: Vec::new(),
            }],
            entry: 0,
            generic_params: None,
        };

        let func_b = FunctionIR {
            name: "b".to_string(),
            params: vec![],
            return_type: MonoType::Void,
            locals: vec![MonoType::String],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Call {
                    dst: Some(Operand::Local(0)),
                    func: Operand::Const(ConstValue::String("identity".to_string())),
                    args: vec![Operand::Const(ConstValue::String("hi".to_string()))],
                    span: crate::util::span::Span::default(),
                }],
                successors: Vec::new(),
            }],
            entry: 0,
            generic_params: None,
        };

        let mut module = ModuleIR {
            functions: vec![func_a, func_b],
            ..Default::default()
        };

        let requests = vec![InstantiationRequest::new(
            GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
            vec![MonoType::Int(64)],
            crate::util::span::Span::default(),
        )];

        mono.replace_call_sites(&mut module, &requests);

        // 两个函数中的调用都应被替换
        for func in &module.functions {
            if let Instruction::Call { func: callee, .. } = &func.blocks[0].instructions[0] {
                assert_eq!(
                    *callee,
                    Operand::Const(ConstValue::String("identity(int64)".to_string())),
                    "函数 {} 的 Call 应该被替换",
                    func.name
                );
            } else {
                panic!("第一条指令应该是 Call");
            }
        }
    }

    #[test]
    fn test_replace_call_sites_no_matching_request() {
        let mono = Monomorphizer::new();

        let main_func = FunctionIR {
            name: "main".to_string(),
            params: vec![],
            return_type: MonoType::Void,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Call {
                    dst: None,
                    func: Operand::Const(ConstValue::String("foo".to_string())),
                    args: vec![],
                    span: crate::util::span::Span::default(),
                }],
                successors: Vec::new(),
            }],
            entry: 0,
            generic_params: None,
        };

        let mut module = ModuleIR {
            functions: vec![main_func],
            ..Default::default()
        };

        // 没有 foo 的请求，不应替换
        let requests = vec![InstantiationRequest::new(
            GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
            vec![MonoType::Int(64)],
            crate::util::span::Span::default(),
        )];

        mono.replace_call_sites(&mut module, &requests);

        let main_func = &module.functions[0];
        if let Instruction::Call { func: callee, .. } = &main_func.blocks[0].instructions[0] {
            assert_eq!(
                *callee,
                Operand::Const(ConstValue::String("foo".to_string())),
                "不匹配的调用不应被替换"
            );
        }
    }

    #[test]
    fn test_monomorphize_end_to_end_with_call_replacement() {
        // 端到端测试：泛型 identity 被特化，main 中的调用被替换
        let identity = make_identity_ir();

        let main_func = FunctionIR {
            name: "main".to_string(),
            params: vec![],
            return_type: MonoType::Int(64),
            locals: vec![MonoType::Int(64)],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Call {
                        dst: Some(Operand::Local(0)),
                        func: Operand::Const(ConstValue::String("identity".to_string())),
                        args: vec![Operand::Const(ConstValue::Int(42))],
                        span: crate::util::span::Span::default(),
                    },
                    Instruction::Ret(Some(Operand::Local(0))),
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            generic_params: None,
        };

        let module = ModuleIR {
            functions: vec![identity, main_func],
            ..Default::default()
        };

        let mut mono = Monomorphizer::new();
        let requests = vec![InstantiationRequest::new(
            GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
            vec![MonoType::Int(64)],
            crate::util::span::Span::default(),
        )];

        let result = mono.monomorphize(&module, &requests);

        // 应有 2 个函数：main（调用已替换）+ identity(int64)
        assert_eq!(result.functions.len(), 2);

        // 找到 main 函数
        let main_out = result.functions.iter().find(|f| f.name == "main").unwrap();
        if let Instruction::Call { func: callee, .. } = &main_out.blocks[0].instructions[0] {
            assert_eq!(
                *callee,
                Operand::Const(ConstValue::String("identity(int64)".to_string())),
                "main 中的调用应被替换为 identity(int64)"
            );
        } else {
            panic!("第一条指令应该是 Call");
        }

        // 找到特化函数
        let specialized = result
            .functions
            .iter()
            .find(|f| f.name == "identity(int64)")
            .expect("应该存在 identity(int64) 特化函数");
        assert!(specialized.generic_params.is_none());
    }
}
