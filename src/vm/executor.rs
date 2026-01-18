//! Virtual Machine executor
//!
//! 实现 YaoXiang VM 字节码执行器，支持寄存器架构和函数调用。

use crate::middle::ir::{ConstValue, FunctionIR, Instruction, ModuleIR, Operand};
use crate::vm::opcode::TypedOpcode;
use crate::vm::errors::{VMError, VMResult};
use crate::util::i18n::{t, t_simple, MSG};
use crate::util::logger::get_lang;
use std::collections::HashMap;
use tracing::debug;

/// 通用寄存器数量
const GENERAL_PURPOSE_REGS: usize = 64;

/// VM 配置
#[derive(Debug, Clone)]
pub struct VMConfig {
    /// 初始栈大小
    pub stack_size: usize,
    /// 最大调用深度
    pub max_call_depth: usize,
    /// 是否启用跟踪
    pub trace_execution: bool,
}

impl Default for VMConfig {
    fn default() -> Self {
        Self {
            stack_size: 64 * 1024,
            max_call_depth: 1024,
            trace_execution: false,
        }
    }
}

/// 虚拟寄存器文件
#[derive(Debug, Clone)]
pub struct RegisterFile {
    /// 通用寄存器
    regs: Vec<Value>,
}

impl RegisterFile {
    /// 创建新的寄存器文件
    pub fn new(count: usize) -> Self {
        Self {
            regs: vec![Value::Void; count],
        }
    }

    /// 读取寄存器
    pub fn read(
        &self,
        idx: u8,
    ) -> &Value {
        self.regs.get(idx as usize).unwrap_or(&Value::Void)
    }

    /// 写入寄存器
    pub fn write(
        &mut self,
        idx: u8,
        value: Value,
    ) {
        let idx = idx as usize;
        if idx < self.regs.len() {
            self.regs[idx] = value;
        }
    }

    /// 获取寄存器数量
    pub fn len(&self) -> usize {
        self.regs.len()
    }

    /// 检查寄存器文件是否为空
    pub fn is_empty(&self) -> bool {
        self.regs.is_empty()
    }
}

/// 调用帧
#[derive(Debug, Clone)]
pub struct Frame {
    /// 函数名
    pub name: String,
    /// 返回地址（字节码偏移）
    pub return_addr: usize,
    /// 调用者的帧指针
    pub caller_fp: usize,
    /// 参数数量
    pub arg_count: usize,
    /// 局部变量
    pub locals: Vec<Value>,
    /// 指令指针（恢复执行的位置）
    pub ip: usize,
}

impl Frame {
    /// 创建新的调用帧
    pub fn new(
        name: String,
        return_addr: usize,
        caller_fp: usize,
        arg_count: usize,
        local_count: usize,
    ) -> Self {
        Self {
            name,
            return_addr,
            caller_fp,
            arg_count,
            locals: vec![Value::Void; local_count],
            ip: 0,
        }
    }

    /// 获取参数数量
    pub fn arg_count(&self) -> usize {
        self.arg_count
    }
}

/// 虚拟机
#[derive(Debug)]
pub struct VM {
    /// 配置
    config: VMConfig,
    /// 状态
    status: VMStatus,
    /// 错误
    error: Option<VMError>,
    /// 寄存器文件
    regs: RegisterFile,
    /// 值栈（用于传递参数和临时存储）
    value_stack: Vec<Value>,
    /// 调用栈
    call_stack: Vec<Frame>,
    /// 当前函数
    current_func: Option<FunctionIR>,
    /// 当前字节码
    bytecode: Vec<u8>,
    /// 指令指针
    ip: usize,
    /// 常量池
    constants: Vec<ConstValue>,
    /// 全局变量
    globals: HashMap<String, Value>,
    /// 函数表
    functions: HashMap<String, FunctionIR>,
}

impl Default for VM {
    fn default() -> Self {
        Self::new_with_config(VMConfig::default())
    }
}

impl VM {
    /// 使用默认配置创建 VM
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用配置创建 VM
    pub fn new_with_config(config: VMConfig) -> Self {
        Self {
            config,
            status: VMStatus::Ready,
            error: None,
            regs: RegisterFile::new(GENERAL_PURPOSE_REGS),
            value_stack: Vec::with_capacity(64 * 1024),
            call_stack: Vec::with_capacity(1024),
            current_func: None,
            bytecode: Vec::new(),
            ip: 0,
            constants: Vec::new(),
            globals: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    /// 获取 VM 状态
    pub fn status(&self) -> VMStatus {
        self.status
    }

    /// 获取 VM 错误
    pub fn error(&self) -> Option<&VMError> {
        self.error.as_ref()
    }

    /// 执行模块
    pub fn execute_module(
        &mut self,
        module: &ModuleIR,
    ) -> VMResult<()> {
        let lang = get_lang();
        let func_count = module.functions.len();
        debug!("{}", t(MSG::VmStart, lang, Some(&[&func_count])));

        // 初始化常量池
        self.constants = module.constants.clone();

        // 初始化函数表
        self.functions.clear();
        for func in &module.functions {
            self.functions.insert(func.name.clone(), func.clone());
        }

        // 初始化 globals
        self.globals.clear();
        for (name, _ty, const_val) in &module.globals {
            if let Some(val) = const_val {
                self.globals.insert(name.clone(), self.const_to_value(val));
            }
        }

        // 查找 main 函数并执行
        self.status = VMStatus::Running;

        // 使用引用避免移动问题
        let main_func = self
            .functions
            .get("main")
            .or_else(|| self.functions.get("_start"))
            .or_else(|| self.functions.values().next());

        if let Some(func) = main_func {
            // 克隆函数以避免借用冲突
            let func_clone = func.clone();
            self.execute_function(&func_clone, &[])?;
        }

        self.status = VMStatus::Finished;
        debug!("{}", t_simple(MSG::VmComplete, lang));
        Ok(())
    }

    /// 执行函数
    fn execute_function(
        &mut self,
        func: &FunctionIR,
        args: &[Value],
    ) -> VMResult<Value> {
        // 检查调用深度
        if self.call_stack.len() >= self.config.max_call_depth {
            return Err(VMError::CallStackOverflow);
        }

        // 创建新帧
        let return_addr = self.ip;
        let caller_fp = self.call_stack.len();
        let arg_count = args.len();
        let local_count = func.locals.len();

        let mut frame = Frame::new(
            func.name.clone(),
            return_addr,
            caller_fp,
            arg_count,
            local_count,
        );

        // 初始化参数（参数存储在局部变量的前面）
        for (i, arg) in args.iter().enumerate().take(func.params.len()) {
            if i < local_count {
                frame.locals[i] = arg.clone();
            }
        }

        // 初始化局部变量
        for i in func.params.len()..local_count {
            frame.locals[i] = Value::Void;
        }

        // 保存当前状态
        let saved_ip = self.ip;
        let saved_func = self.current_func.take();

        // 压入新帧
        self.call_stack.push(frame);

        // 执行函数体
        let result = self.execute_function_body(func)?;

        // 弹出帧
        self.call_stack.pop();

        // 恢复状态
        self.ip = saved_ip;
        self.current_func = saved_func;

        Ok(result)
    }

    /// 执行函数体
    fn execute_function_body(
        &mut self,
        func: &FunctionIR,
    ) -> VMResult<Value> {
        self.current_func = Some(func.clone());

        // 生成字节码（直接从 IR 生成字节码）
        let bytecode = self.generate_bytecode(func)?;
        self.bytecode = bytecode;

        loop {
            // 检查是否超出字节码范围
            if self.ip >= self.bytecode.len() {
                // 函数没有明确的返回，返回 Void
                return Ok(Value::Void);
            }

            // 获取并执行指令
            let opcode = self.bytecode[self.ip];
            self.ip += 1;

            // 解码并执行指令
            if let Ok(typed_opcode) = TypedOpcode::try_from(opcode) {
                self.execute_instruction(typed_opcode)?;

                // 检查是否是返回指令
                if self.should_return(&typed_opcode) {
                    return self.get_return_value(&typed_opcode);
                }
            } else {
                return Err(VMError::InvalidOpcode(opcode));
            }
        }
    }

    /// 生成函数的字节码
    fn generate_bytecode(
        &mut self,
        func: &FunctionIR,
    ) -> Result<Vec<u8>, VMError> {
        let mut bytecode = Vec::new();

        for block in &func.blocks {
            for instr in &block.instructions {
                self.encode_instruction(instr, &mut bytecode)?;
            }
        }

        Ok(bytecode)
    }

    /// 编码指令为字节码
    fn encode_instruction(
        &mut self,
        instr: &Instruction,
        bytecode: &mut Vec<u8>,
    ) -> Result<(), VMError> {
        use crate::middle::ir::Instruction as Instr;
        use crate::vm::opcode::TypedOpcode as TOp;

        match instr {
            // 移动指令
            Instr::Move { dst, src } => {
                let dst_reg = self.operand_to_reg(dst);
                let src_reg = self.operand_to_reg(src);
                bytecode.push(TOp::Mov as u8);
                bytecode.push(dst_reg);
                bytecode.push(src_reg);
            }

            // 加载常量
            Instr::Load { dst, src } => {
                if let Operand::Const(const_val) = src {
                    let dst_reg = self.operand_to_reg(dst);
                    // 查找常量索引
                    let mut const_idx = None;
                    for (i, existing) in self.constants.iter().enumerate() {
                        if *existing == *const_val {
                            const_idx = Some(i);
                            break;
                        }
                    }
                    let idx = const_idx.unwrap_or_else(|| {
                        self.constants.push(const_val.clone());
                        self.constants.len() - 1
                    });
                    bytecode.push(TOp::LoadConst as u8);
                    bytecode.push(dst_reg);
                    bytecode.push((idx & 0xFF) as u8);
                    bytecode.push(((idx >> 8) & 0xFF) as u8);
                } else {
                    let dst_reg = self.operand_to_reg(dst);
                    let src_reg = self.operand_to_reg(src);
                    bytecode.push(TOp::Mov as u8);
                    bytecode.push(dst_reg);
                    bytecode.push(src_reg);
                }
            }

            // 存储
            Instr::Store { dst: _, src } => {
                // Store 指令简化处理
                let _src_reg = self.operand_to_reg(src);
                bytecode.push(TOp::Nop as u8);
            }

            // 算术运算
            Instr::Add { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst);
                let lhs_reg = self.operand_to_reg(lhs);
                let rhs_reg = self.operand_to_reg(rhs);
                bytecode.push(TOp::I64Add as u8);
                bytecode.push(dst_reg);
                bytecode.push(lhs_reg);
                bytecode.push(rhs_reg);
            }

            Instr::Sub { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst);
                let lhs_reg = self.operand_to_reg(lhs);
                let rhs_reg = self.operand_to_reg(rhs);
                bytecode.push(TOp::I64Sub as u8);
                bytecode.push(dst_reg);
                bytecode.push(lhs_reg);
                bytecode.push(rhs_reg);
            }

            Instr::Mul { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst);
                let lhs_reg = self.operand_to_reg(lhs);
                let rhs_reg = self.operand_to_reg(rhs);
                bytecode.push(TOp::I64Mul as u8);
                bytecode.push(dst_reg);
                bytecode.push(lhs_reg);
                bytecode.push(rhs_reg);
            }

            Instr::Div { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst);
                let lhs_reg = self.operand_to_reg(lhs);
                let rhs_reg = self.operand_to_reg(rhs);
                bytecode.push(TOp::I64Div as u8);
                bytecode.push(dst_reg);
                bytecode.push(lhs_reg);
                bytecode.push(rhs_reg);
            }

            // 比较运算
            Instr::Eq { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst);
                let lhs_reg = self.operand_to_reg(lhs);
                let rhs_reg = self.operand_to_reg(rhs);
                bytecode.push(TOp::I64Eq as u8);
                bytecode.push(dst_reg);
                bytecode.push(lhs_reg);
                bytecode.push(rhs_reg);
            }

            Instr::Ne { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst);
                let lhs_reg = self.operand_to_reg(lhs);
                let rhs_reg = self.operand_to_reg(rhs);
                bytecode.push(TOp::I64Ne as u8);
                bytecode.push(dst_reg);
                bytecode.push(lhs_reg);
                bytecode.push(rhs_reg);
            }

            Instr::Lt { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst);
                let lhs_reg = self.operand_to_reg(lhs);
                let rhs_reg = self.operand_to_reg(rhs);
                bytecode.push(TOp::I64Lt as u8);
                bytecode.push(dst_reg);
                bytecode.push(lhs_reg);
                bytecode.push(rhs_reg);
            }

            Instr::Le { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst);
                let lhs_reg = self.operand_to_reg(lhs);
                let rhs_reg = self.operand_to_reg(rhs);
                bytecode.push(TOp::I64Le as u8);
                bytecode.push(dst_reg);
                bytecode.push(lhs_reg);
                bytecode.push(rhs_reg);
            }

            Instr::Gt { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst);
                let lhs_reg = self.operand_to_reg(lhs);
                let rhs_reg = self.operand_to_reg(rhs);
                bytecode.push(TOp::I64Gt as u8);
                bytecode.push(dst_reg);
                bytecode.push(lhs_reg);
                bytecode.push(rhs_reg);
            }

            Instr::Ge { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst);
                let lhs_reg = self.operand_to_reg(lhs);
                let rhs_reg = self.operand_to_reg(rhs);
                bytecode.push(TOp::I64Ge as u8);
                bytecode.push(dst_reg);
                bytecode.push(lhs_reg);
                bytecode.push(rhs_reg);
            }

            // 跳转
            Instr::Jmp(target) => {
                let offset = *target as i32;
                let bytes = offset.to_le_bytes();
                bytecode.push(TOp::Jmp as u8);
                bytecode.extend_from_slice(&bytes);
            }

            Instr::JmpIf(cond, target) => {
                let cond_reg = self.operand_to_reg(cond);
                let offset = *target as i16;
                let bytes = offset.to_le_bytes();
                bytecode.push(TOp::JmpIf as u8);
                bytecode.push(cond_reg);
                bytecode.extend_from_slice(&bytes);
            }

            Instr::JmpIfNot(cond, target) => {
                let cond_reg = self.operand_to_reg(cond);
                let offset = *target as i16;
                let bytes = offset.to_le_bytes();
                bytecode.push(TOp::JmpIfNot as u8);
                bytecode.push(cond_reg);
                bytecode.extend_from_slice(&bytes);
            }

            // 返回
            Instr::Ret(val) => {
                if let Some(v) = val {
                    let reg = self.operand_to_reg(v);
                    bytecode.push(TOp::ReturnValue as u8);
                    bytecode.push(reg);
                } else {
                    bytecode.push(TOp::Return as u8);
                }
            }

            // 函数调用
            Instr::Call { dst, func, args } => {
                let dst_reg = dst.as_ref().map(|d| self.operand_to_reg(d)).unwrap_or(0);

                // 解析函数名
                let func_name = match func {
                    Operand::Const(ConstValue::String(s)) => s.clone(),
                    _ => "print".to_string(),
                };

                // 简单的字符串哈希作为函数 ID
                let func_id = self.hash_string(&func_name);

                bytecode.push(TOp::CallStatic as u8);
                bytecode.push(dst_reg);
                bytecode.extend_from_slice(&func_id.to_le_bytes());
                bytecode.push(0); // base_arg_reg
                bytecode.push(args.len() as u8);
            }

            // 栈操作
            Instr::Push(operand) => {
                let reg = self.operand_to_reg(operand);
                bytecode.push(TOp::Mov as u8);
                bytecode.push(reg);
            }

            Instr::Pop(_) => {
                bytecode.push(TOp::Nop as u8);
            }

            Instr::Dup => {
                bytecode.push(TOp::Nop as u8);
            }

            Instr::Swap => {
                bytecode.push(TOp::Nop as u8);
            }

            // 类型操作
            Instr::Neg { dst, src } => {
                let dst_reg = self.operand_to_reg(dst);
                let src_reg = self.operand_to_reg(src);
                bytecode.push(TOp::I64Neg as u8);
                bytecode.push(dst_reg);
                bytecode.push(src_reg);
            }

            Instr::Mod { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst);
                let lhs_reg = self.operand_to_reg(lhs);
                let rhs_reg = self.operand_to_reg(rhs);
                bytecode.push(TOp::I64Rem as u8);
                bytecode.push(dst_reg);
                bytecode.push(lhs_reg);
                bytecode.push(rhs_reg);
            }

            // 其他指令使用 Nop
            _ => {
                bytecode.push(TOp::Nop as u8);
            }
        }

        Ok(())
    }

    /// 将操作数转换为寄存器编号
    fn operand_to_reg(
        &self,
        operand: &Operand,
    ) -> u8 {
        match operand {
            Operand::Register(r) => *r,
            Operand::Local(idx) => *idx as u8,
            Operand::Temp(idx) => *idx as u8,
            Operand::Arg(idx) => *idx as u8,
            _ => 0,
        }
    }

    /// 简单的字符串哈希
    fn hash_string(
        &self,
        s: &str,
    ) -> u32 {
        let mut hash = 0u32;
        for c in s.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(c as u32);
        }
        hash
    }

    /// 添加常量到常量池
    fn add_constant(
        &mut self,
        const_val: ConstValue,
    ) -> Result<usize, VMError> {
        // 检查是否已存在
        for (i, existing) in self.constants.iter().enumerate() {
            if *existing == const_val {
                return Ok(i);
            }
        }
        // 添加新常量
        self.constants.push(const_val.clone());
        Ok(self.constants.len() - 1)
    }

    /// 执行指令
    fn execute_instruction(
        &mut self,
        opcode: TypedOpcode,
    ) -> VMResult<()> {
        use TypedOpcode::*;

        match opcode {
            // 无操作
            Nop => {}

            // 返回指令
            Return => {
                return Ok(());
            }

            ReturnValue => {
                return Ok(());
            }

            // 移动指令
            Mov => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;
                let value = self.regs.read(src).clone();
                self.regs.write(dst, value);
            }

            // 加载常量
            LoadConst => {
                let dst = self.read_u8()?;
                let const_idx = self.read_u16()? as usize;
                if let Some(const_val) = self.constants.get(const_idx) {
                    let value = self.const_to_value(const_val);
                    self.regs.write(dst, value);
                }
            }

            // 加载局部变量
            LoadLocal => {
                let dst = self.read_u8()?;
                let local_idx = self.read_u8()? as usize;
                if let Some(frame) = self.call_stack.last() {
                    if local_idx < frame.locals.len() {
                        let value = frame.locals[local_idx].clone();
                        self.regs.write(dst, value);
                    }
                }
            }

            // 存储局部变量
            StoreLocal => {
                let local_idx = self.read_u8()? as usize;
                let src = self.read_u8()?;
                if let Some(frame) = self.call_stack.last_mut() {
                    if local_idx < frame.locals.len() {
                        frame.locals[local_idx] = self.regs.read(src).clone();
                    }
                }
            }

            // 加载参数
            LoadArg => {
                let dst = self.read_u8()?;
                let arg_idx = self.read_u8()? as usize;
                if let Some(frame) = self.call_stack.last() {
                    if arg_idx < frame.arg_count && arg_idx < frame.locals.len() {
                        let value = frame.locals[arg_idx].clone();
                        self.regs.write(dst, value);
                    }
                }
            }

            // I64 算术运算
            I64Add => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| Ok(a + b))?;
                self.regs.write(dst, Value::Int(result));
            }

            I64Sub => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| Ok(a - b))?;
                self.regs.write(dst, Value::Int(result));
            }

            I64Mul => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| Ok(a * b))?;
                self.regs.write(dst, Value::Int(result));
            }

            I64Div => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| {
                    if b == 0 {
                        Err(VMError::DivisionByZero)
                    } else {
                        Ok(a / b)
                    }
                })?;
                self.regs.write(dst, Value::Int(result));
            }

            I64Rem => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| {
                    if b == 0 {
                        Err(VMError::DivisionByZero)
                    } else {
                        Ok(a % b)
                    }
                })?;
                self.regs.write(dst, Value::Int(result));
            }

            I64Neg => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;
                if let Value::Int(n) = self.regs.read(src) {
                    self.regs.write(dst, Value::Int(-*n));
                }
            }

            // I64 比较运算
            I64Eq => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_i64(lhs, rhs, |a, b| a == b);
                self.regs.write(dst, Value::Int(result as i128));
            }

            I64Ne => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_i64(lhs, rhs, |a, b| a != b);
                self.regs.write(dst, Value::Int(result as i128));
            }

            I64Lt => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_i64(lhs, rhs, |a, b| a < b);
                self.regs.write(dst, Value::Int(result as i128));
            }

            I64Le => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_i64(lhs, rhs, |a, b| a <= b);
                self.regs.write(dst, Value::Int(result as i128));
            }

            I64Gt => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_i64(lhs, rhs, |a, b| a > b);
                self.regs.write(dst, Value::Int(result as i128));
            }

            I64Ge => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_i64(lhs, rhs, |a, b| a >= b);
                self.regs.write(dst, Value::Int(result as i128));
            }

            // 跳转指令
            Jmp => {
                let offset = self.read_i32()?;
                self.ip = (self.ip as i32 + offset) as usize;
            }

            JmpIf => {
                let cond = self.read_u8()?;
                let offset = self.read_i16()?;
                if let Value::Int(n) = self.regs.read(cond) {
                    if *n != 0 {
                        self.ip = (self.ip as i32 + offset as i32) as usize;
                    }
                }
            }

            JmpIfNot => {
                let cond = self.read_u8()?;
                let offset = self.read_i16()?;
                if let Value::Int(n) = self.regs.read(cond) {
                    if *n == 0 {
                        self.ip = (self.ip as i32 + offset as i32) as usize;
                    }
                }
            }

            // 函数调用
            CallStatic => {
                let _dst = self.read_u8()?;
                let func_id = self.read_u32()?;
                let _base_arg_reg = self.read_u8()?;
                let arg_count = self.read_u8()?;

                // 收集参数
                let mut args = Vec::with_capacity(arg_count as usize);
                for i in 0..arg_count {
                    let arg_reg = i;
                    args.push(self.regs.read(arg_reg).clone());
                }

                // 从函数 ID 解析函数名
                let func_name = self
                    .find_function_by_id(func_id)
                    .unwrap_or_else(|| "print".to_string());

                // 检查是否是内部函数
                if func_name == "print" {
                    // 调用 print 内部函数
                    if let Some(first_arg) = args.first() {
                        self.call_print(first_arg)?;
                    }
                } else if let Some(target_func) = self.functions.get(&func_name) {
                    // 调用用户定义函数 - 克隆函数以避免借用冲突
                    let func_clone = target_func.clone();
                    self.execute_function(&func_clone, &args)?;
                }
            }

            // F64 算术运算
            F64Add => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_f64(lhs, rhs, |a, b| Ok(a + b))?;
                self.regs.write(dst, Value::Float(result));
            }

            F64Sub => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_f64(lhs, rhs, |a, b| Ok(a - b))?;
                self.regs.write(dst, Value::Float(result));
            }

            F64Mul => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_f64(lhs, rhs, |a, b| Ok(a * b))?;
                self.regs.write(dst, Value::Float(result));
            }

            F64Div => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_f64(lhs, rhs, |a, b| {
                    if b == 0.0 {
                        Err(VMError::DivisionByZero)
                    } else {
                        Ok(a / b)
                    }
                })?;
                self.regs.write(dst, Value::Float(result));
            }

            // 其他指令
            Drop => {
                let _reg = self.read_u8()?;
                // 简化：忽略 Drop
            }

            _ => {
                // 未实现的指令
            }
        }

        Ok(())
    }

    /// 根据函数 ID 查找函数名
    fn find_function_by_id(
        &self,
        func_id: u32,
    ) -> Option<String> {
        // 简化实现：遍历函数表
        for name in self.functions.keys() {
            let id = self.hash_string(name);
            if id == func_id {
                return Some(name.clone());
            }
        }
        None
    }

    /// 读取 u8 操作数
    fn read_u8(&mut self) -> Result<u8, VMError> {
        if self.ip < self.bytecode.len() {
            let val = self.bytecode[self.ip];
            self.ip += 1;
            Ok(val)
        } else {
            Err(VMError::InvalidOperand)
        }
    }

    /// 读取 u16 操作数
    fn read_u16(&mut self) -> Result<u16, VMError> {
        if self.ip + 1 < self.bytecode.len() {
            let val = u16::from_le_bytes([self.bytecode[self.ip], self.bytecode[self.ip + 1]]);
            self.ip += 2;
            Ok(val)
        } else {
            Err(VMError::InvalidOperand)
        }
    }

    /// 读取 u32 操作数
    fn read_u32(&mut self) -> Result<u32, VMError> {
        if self.ip + 3 < self.bytecode.len() {
            let val = u32::from_le_bytes([
                self.bytecode[self.ip],
                self.bytecode[self.ip + 1],
                self.bytecode[self.ip + 2],
                self.bytecode[self.ip + 3],
            ]);
            self.ip += 4;
            Ok(val)
        } else {
            Err(VMError::InvalidOperand)
        }
    }

    /// 读取 i32 操作数
    fn read_i32(&mut self) -> Result<i32, VMError> {
        self.read_u32().map(|v| v as i32)
    }

    /// 读取 i16 操作数
    fn read_i16(&mut self) -> Result<i16, VMError> {
        self.read_u16().map(|v| v as i16)
    }

    /// 二进制 I64 运算
    fn binary_op_i64<F>(
        &self,
        lhs_reg: u8,
        rhs_reg: u8,
        op: F,
    ) -> Result<i128, VMError>
    where
        F: FnOnce(i128, i128) -> Result<i128, VMError>,
    {
        match (self.regs.read(lhs_reg), self.regs.read(rhs_reg)) {
            (Value::Int(a), Value::Int(b)) => op(*a, *b),
            _ => Err(VMError::TypeError("integer".to_string())),
        }
    }

    /// 二进制 F64 运算
    fn binary_op_f64<F>(
        &self,
        lhs_reg: u8,
        rhs_reg: u8,
        op: F,
    ) -> Result<f64, VMError>
    where
        F: FnOnce(f64, f64) -> Result<f64, VMError>,
    {
        match (self.regs.read(lhs_reg), self.regs.read(rhs_reg)) {
            (Value::Float(a), Value::Float(b)) => op(*a, *b),
            (Value::Int(a), Value::Int(b)) => op(*a as f64, *b as f64),
            _ => Err(VMError::TypeError("float".to_string())),
        }
    }

    /// I64 比较
    fn compare_i64<F>(
        &self,
        lhs_reg: u8,
        rhs_reg: u8,
        op: F,
    ) -> bool
    where
        F: FnOnce(i128, i128) -> bool,
    {
        match (self.regs.read(lhs_reg), self.regs.read(rhs_reg)) {
            (Value::Int(a), Value::Int(b)) => op(*a, *b),
            _ => false,
        }
    }

    /// 检查是否应该返回
    fn should_return(
        &self,
        opcode: &TypedOpcode,
    ) -> bool {
        matches!(opcode, TypedOpcode::Return | TypedOpcode::ReturnValue)
    }

    /// 获取返回值
    fn get_return_value(
        &self,
        _opcode: &TypedOpcode,
    ) -> VMResult<Value> {
        // ReturnValue 指令的返回值已经在执行时处理
        Ok(Value::Void)
    }

    /// 调用 print 内部函数
    fn call_print(
        &self,
        value: &Value,
    ) -> VMResult<()> {
        match value {
            Value::String(s) => {
                print!("{}", s);
            }
            Value::Int(n) => {
                print!("{}", n);
            }
            Value::Float(f) => {
                print!("{}", f);
            }
            Value::Bool(b) => {
                print!("{}", b);
            }
            Value::Char(c) => {
                print!("{}", c);
            }
            Value::Void => {
                print!("()");
            }
            _ => {
                print!("{:?}", value);
            }
        }
        Ok(())
    }

    /// 将 ConstValue 转换为 Value
    fn const_to_value(
        &self,
        const_val: &ConstValue,
    ) -> Value {
        match const_val {
            ConstValue::Void => Value::Void,
            ConstValue::Bool(b) => Value::Bool(*b),
            ConstValue::Int(n) => Value::Int(*n),
            ConstValue::Float(f) => Value::Float(*f),
            ConstValue::Char(c) => Value::Char(*c),
            ConstValue::String(s) => Value::String(s.clone()),
            ConstValue::Bytes(b) => Value::Bytes(b.clone()),
        }
    }
}

/// 运行时值
#[derive(Debug, Clone)]
pub enum Value {
    /// 无值 / 单元类型
    Void,
    /// 布尔值
    Bool(bool),
    /// 整数（128位）
    Int(i128),
    /// 浮点数（64位）
    Float(f64),
    /// 字符
    Char(char),
    /// 字符串
    String(String),
    /// 字节数组
    Bytes(Vec<u8>),
    /// 列表
    List(Vec<Value>),
    /// 字典
    Dict(HashMap<Value, Value>),
}

impl Value {
    /// 检查是否需要 drop
    pub fn needs_drop(&self) -> bool {
        matches!(
            self,
            Value::String(_) | Value::Bytes(_) | Value::List(_) | Value::Dict(_)
        )
    }
}

/// VM 执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VMStatus {
    /// 准备好执行
    Ready,
    /// 正在执行
    Running,
    /// 执行完成
    Finished,
    /// 发生错误
    Error,
}
