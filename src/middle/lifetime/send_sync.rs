//! Send/Sync 约束检查
//!
//! 检查类型是否满足 Send/Sync 约束，确保并发安全：
//! - Send: 类型可以安全地跨线程传输（值传递）
//! - Sync: 类型可以安全地跨线程共享引用
//!
//! 设计原则：
//! 1. YaoXiang 优先使用值传递，Sync 很少需要
//! 2. 基本类型自动满足 Send + Sync
//! 3. Arc 自动满足 Send + Sync
//! 4. Rc 既不是 Send 也不是 Sync

use super::error::OwnershipError;
use crate::frontend::typecheck::MonoType;
use crate::middle::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashMap;

/// Send/Sync 检查器
///
/// 检测以下错误：
/// - NotSend: 非 Send 类型用于跨线程操作
/// - NotSync: 非 Sync 类型用于跨线程共享
#[derive(Debug)]
pub struct SendSyncChecker {
    /// 收集的错误
    errors: Vec<OwnershipError>,
    /// 当前位置 (block_idx, instr_idx)
    location: (usize, usize),
    /// 闭包定义映射: closure_operand -> (func, env)
    closures: HashMap<Operand, (usize, Vec<Operand>)>,
}

impl SendSyncChecker {
    /// 创建新的 Send/Sync 检查器
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            location: (0, 0),
            closures: HashMap::new(),
        }
    }

    /// 检查函数的所有权语义
    pub fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[OwnershipError] {
        self.clear();
        self.build_closure_map(func);

        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                self.location = (block_idx, instr_idx);
                self.check_instruction(instr, func);
            }
        }

        &self.errors
    }

    /// 获取收集的错误
    pub fn errors(&self) -> &[OwnershipError] {
        &self.errors
    }

    /// 清除状态
    pub fn clear(&mut self) {
        self.errors.clear();
        self.closures.clear();
    }

    /// 构建闭包映射
    fn build_closure_map(
        &mut self,
        func: &FunctionIR,
    ) {
        self.closures.clear();
        for block in func.blocks.iter() {
            for instr in block.instructions.iter() {
                if let Instruction::MakeClosure {
                    dst,
                    func: func_idx,
                    env,
                } = instr
                {
                    self.closures.insert(dst.clone(), (*func_idx, env.clone()));
                }
            }
        }
    }

    /// 检查指令
    fn check_instruction(
        &mut self,
        instr: &Instruction,
        func: &FunctionIR,
    ) {
        match instr {
            // Spawn 检查：闭包捕获的变量必须是 Send
            Instruction::Spawn {
                func: closure_op, ..
            } => {
                self.check_spawn(closure_op, func);
            }
            // ArcNew: Arc 总是 Send + Sync
            Instruction::ArcNew { .. } => {
                // Arc 本身是 Send+Sync，不检查底层类型
                // 但 src 必须在这个上下文中有效（所有权检查已覆盖）
            }
            // ArcClone: 克隆 Arc，不改变 Send/Sync 属性
            Instruction::ArcClone { .. } => {
                // Arc 总是 Send+Sync
            }
            // 跨线程函数调用检查（如果将来实现）
            _ => {}
        }
    }

    /// 检查 spawn 操作的 Send 约束
    fn check_spawn(
        &mut self,
        closure_op: &Operand,
        func: &FunctionIR,
    ) {
        // 如果闭包是 Local，检查其环境变量
        // 注意：需要先 clone env 避免借用冲突
        if let Some((_, env)) = self.closures.get(closure_op) {
            let env: Vec<Operand> = env.clone();
            for captured in env {
                if let Some(ty) = self.get_operand_type(&captured, func) {
                    if !self.is_send(&ty) {
                        self.report_not_send(&captured, &ty, "closure captures non-Send type");
                    }
                }
            }
        }
    }

    /// 获取操作数的类型
    fn get_operand_type(
        &self,
        operand: &Operand,
        func: &FunctionIR,
    ) -> Option<MonoType> {
        match operand {
            Operand::Const(c) => Some(self.const_type(c)),
            Operand::Arg(idx) => func.params.get(*idx).cloned(),
            Operand::Local(idx) => func.locals.get(*idx).cloned(),
            Operand::Temp(_) => None,   // 临时变量类型需要额外追踪
            Operand::Global(_) => None, // 全局变量类型需要额外信息
            Operand::Label(_) => None,
            Operand::Register(_) => None,
        }
    }

    /// 常量类型
    fn const_type(
        &self,
        c: &crate::middle::ir::ConstValue,
    ) -> MonoType {
        use crate::middle::ir::ConstValue;
        match c {
            ConstValue::Void => MonoType::Void,
            ConstValue::Bool(_) => MonoType::Bool,
            ConstValue::Int(_) => MonoType::Int(64),
            ConstValue::Float(_) => MonoType::Float(64),
            ConstValue::Char(_) => MonoType::Char,
            ConstValue::String(_) => MonoType::String,
            ConstValue::Bytes(_) => MonoType::Bytes,
        }
    }

    /// 检查类型是否 Send
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn is_send(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            // 基本类型总是 Send
            MonoType::Void => true,
            MonoType::Bool => true,
            MonoType::Int(_) => true,
            MonoType::Float(_) => true,
            MonoType::Char => true,
            MonoType::String => true,
            MonoType::Bytes => true,

            // 列表、字典、集合：元素类型必须 Send
            MonoType::List(elem) => self.is_send(elem),
            MonoType::Dict(key, value) => self.is_send(key) && self.is_send(value),
            MonoType::Set(elem) => self.is_send(elem),

            // 元组：所有元素必须 Send
            MonoType::Tuple(types) => types.iter().all(|t| self.is_send(t)),

            // 函数类型：参数和返回类型必须 Send
            MonoType::Fn {
                params,
                return_type,
                ..
            } => params.iter().all(|p| self.is_send(p)) && self.is_send(return_type),

            // Arc: 总是 Send（原子引用计数）
            MonoType::Arc(inner) => self.is_send(inner),

            // Range: 元素类型必须 Send
            MonoType::Range { elem_type } => self.is_send(elem_type),

            // 联合/交集类型：所有成员必须 Send
            MonoType::Union(types) => types.iter().all(|t| self.is_send(t)),
            MonoType::Intersection(types) => types.iter().all(|t| self.is_send(t)),

            // 结构体：所有字段必须 Send
            MonoType::Struct(s) => s.fields.iter().all(|(_, f)| self.is_send(f)),

            // 枚举：所有变体必须 Send
            MonoType::Enum(_) => true, // 枚举只是标签，视为 Send

            // 类型变量和类型引用：保守假设为 Send（类型检查已验证）
            MonoType::TypeVar(_) => true,
            MonoType::TypeRef(_) => true,
        }
    }

    /// 检查类型是否 Sync
    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn is_sync(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            // 基本类型总是 Sync
            MonoType::Void => true,
            MonoType::Bool => true,
            MonoType::Int(_) => true,
            MonoType::Float(_) => true,
            MonoType::Char => true,
            MonoType::String => true,
            MonoType::Bytes => true,

            // 列表：必须是 RefCell[T] 或类似包装才有意义，保守返回 false
            MonoType::List(_) => false,

            // 字典：保守返回 false
            MonoType::Dict(_, _) => false,

            // 集合：保守返回 false
            MonoType::Set(_) => false,

            // 元组：如果是 (T, &T) 形式可能 Sync，保守返回 false
            MonoType::Tuple(types) => types.iter().all(|t| self.is_sync(t)),

            // 函数类型：通常不用于共享
            MonoType::Fn { .. } => false,

            // Arc: 总是 Sync（安全共享）
            MonoType::Arc(inner) => self.is_sync(inner),

            // Range: 元素类型必须 Sync
            MonoType::Range { elem_type } => self.is_sync(elem_type),

            // 联合/交集：所有成员必须 Sync
            MonoType::Union(types) => types.iter().all(|t| self.is_sync(t)),
            MonoType::Intersection(types) => types.iter().all(|t| self.is_sync(t)),

            // 结构体：所有字段必须 Sync
            MonoType::Struct(s) => s.fields.iter().all(|(_, f)| self.is_sync(f)),

            // 枚举：只是标签
            MonoType::Enum(_) => true,

            // 类型变量和类型引用：保守假设为 Sync
            MonoType::TypeVar(_) => true,
            MonoType::TypeRef(_) => true,
        }
    }

    /// 报告 NotSend 错误
    fn report_not_send(
        &mut self,
        operand: &Operand,
        ty: &MonoType,
        reason: &str,
    ) {
        self.errors.push(OwnershipError::NotSend {
            value: self.operand_to_string(operand),
            reason: format!("{} (type: {})", reason, self.type_to_string(ty)),
            location: self.location,
        });
    }

    /// 报告 NotSync 错误
    fn report_not_sync(
        &mut self,
        operand: &Operand,
        ty: &MonoType,
        reason: &str,
    ) {
        self.errors.push(OwnershipError::NotSync {
            value: self.operand_to_string(operand),
            reason: format!("{} (type: {})", reason, self.type_to_string(ty)),
            location: self.location,
        });
    }

    /// 操作数转字符串
    fn operand_to_string(
        &self,
        operand: &Operand,
    ) -> String {
        match operand {
            Operand::Const(c) => format!("const_{:?}", c),
            Operand::Local(idx) => format!("local_{}", idx),
            Operand::Arg(idx) => format!("arg_{}", idx),
            Operand::Temp(idx) => format!("temp_{}", idx),
            Operand::Global(idx) => format!("global_{}", idx),
            Operand::Label(idx) => format!("label_{}", idx),
            Operand::Register(idx) => format!("reg_{}", idx),
        }
    }

    /// 类型转字符串
    fn type_to_string(
        &self,
        ty: &MonoType,
    ) -> String {
        ty.type_name()
    }
}

impl Default for SendSyncChecker {
    fn default() -> Self {
        Self::new()
    }
}
