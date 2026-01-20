//! Virtual Machine executor
//!
//! 实现 YaoXiang VM 字节码执行器，支持寄存器架构和函数调用。

use crate::middle::codegen::bytecode::FunctionCode;
use crate::middle::ir::{ConstValue, Operand};
use crate::runtime::value::RuntimeValue;
use crate::vm::opcode::TypedOpcode;
use crate::vm::errors::{VMError, VMResult};
use crate::runtime::extfunc;
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
    regs: Vec<RuntimeValue>,
}

impl RegisterFile {
    /// 创建新的寄存器文件
    pub fn new(count: usize) -> Self {
        Self {
            regs: vec![RuntimeValue::Unit; count],
        }
    }

    /// 读取寄存器
    pub fn read(
        &self,
        idx: u8,
    ) -> &RuntimeValue {
        self.regs.get(idx as usize).unwrap_or(&RuntimeValue::Unit)
    }

    /// 写入寄存器
    pub fn write(
        &mut self,
        idx: u8,
        value: RuntimeValue,
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
    pub locals: Vec<RuntimeValue>,
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
            locals: vec![RuntimeValue::Unit; local_count],
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
    value_stack: Vec<RuntimeValue>,
    /// 调用栈
    call_stack: Vec<Frame>,
    /// 当前函数（使用预编码的字节码）
    current_func: Option<FunctionCode>,
    /// 当前字节码
    bytecode: Vec<u8>,
    /// 指令指针
    ip: usize,
    /// 常量池
    constants: Vec<ConstValue>,
    /// 全局变量
    globals: HashMap<String, RuntimeValue>,
    /// 函数表（预编码的字节码）
    functions: HashMap<String, FunctionCode>,
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

    /// 执行模块（接收预编译的 CompiledModule）
    pub fn execute_module(
        &mut self,
        compiled: &crate::middle::codegen::bytecode::CompiledModule,
    ) -> VMResult<()> {
        let lang = get_lang();
        let func_count = compiled.functions.len();
        debug!("{}", t(MSG::VmStart, lang, Some(&[&func_count])));

        // Load constants
        self.constants = compiled.constants.clone();

        // 直接使用预编译的函数代码（不再调用 BytecodeGenerator）
        self.functions.clear();
        for func in &compiled.functions {
            self.functions.insert(func.name.clone(), func.clone());
        }

        // 初始化 globals
        self.globals.clear();
        for (name, _ty, const_val) in &compiled.globals {
            if let Some(val) = const_val {
                self.globals.insert(name.clone(), self.const_to_value(val));
            }
        }

        // 查找 main 函数并执行
        self.status = VMStatus::Running;

        let main_func = self
            .functions
            .get("main")
            .or_else(|| self.functions.get("_start"))
            .or_else(|| self.functions.values().next())
            .cloned();

        if let Some(func) = main_func {
            self.execute_function(&func, &[])?;
        }

        self.status = VMStatus::Finished;
        debug!("{}", t_simple(MSG::VmComplete, lang));
        Ok(())
    }

    /// 执行函数（直接使用预编码的 FunctionCode）
    fn execute_function(
        &mut self,
        func: &FunctionCode,
        args: &[RuntimeValue],
    ) -> VMResult<RuntimeValue> {
        let lang = get_lang();
        debug!("{}", t(MSG::VmExecuteFn, lang, Some(&[&func.name])));

        // 检查调用深度
        if self.call_stack.len() >= self.config.max_call_depth {
            return Err(VMError::CallStackOverflow);
        }

        // 创建新帧
        let return_addr = self.ip;
        let caller_fp = self.call_stack.len();
        let arg_count = args.len();
        let local_count = func.local_count;

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
                // 同时写入寄存器，以便直接访问
                self.regs.write(i as u8, arg.clone());
                debug!(
                    "{}",
                    t(MSG::VmLoadArg, lang, Some(&[&i, &format!("{:?}", arg)]))
                );
            }
        }

        // 初始化局部变量
        for i in func.params.len()..local_count {
            frame.locals[i] = RuntimeValue::Unit;
        }

        // 保存当前状态
        let saved_ip = self.ip;
        let saved_func = self.current_func.take();

        // 压入新帧
        debug!("{}", t(MSG::VmPushFrame, lang, Some(&[&func.name])));
        debug!(
            "{}",
            t(
                MSG::VmCallStack,
                lang,
                Some(&[&(self.call_stack.len() + 1)])
            )
        );
        self.call_stack.push(frame);

        // 执行函数体（直接使用预编码的字节码）
        let result = self.execute_function_body(func)?;

        // 弹出帧
        debug!("{}", t(MSG::VmPopFrame, lang, None));
        self.call_stack.pop();

        // 恢复状态
        self.ip = saved_ip;
        self.current_func = saved_func;

        Ok(result)
    }

    /// 执行函数体（直接执行预编码字节码）
    fn execute_function_body(
        &mut self,
        func: &FunctionCode,
    ) -> VMResult<RuntimeValue> {
        self.current_func = Some(func.clone());
        self.bytecode = func.encode_all();

        loop {
            if self.ip >= self.bytecode.len() {
                return Ok(RuntimeValue::Unit);
            }

            let opcode = self.bytecode[self.ip];
            self.ip += 1;

            if let Ok(typed_opcode) = TypedOpcode::try_from(opcode) {
                if self.config.trace_execution {
                    debug!(
                        "{}",
                        t(
                            MSG::VmExecInstruction,
                            get_lang(),
                            Some(&[&format!("{:?}", typed_opcode)])
                        )
                    );
                }
                self.execute_instruction(typed_opcode)?;

                if self.should_return(&typed_opcode) {
                    return self.get_return_value(&typed_opcode);
                }
            } else {
                return Err(VMError::InvalidOpcode(opcode));
            }
        }
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
                debug!(
                    "{}",
                    t(
                        MSG::VmRegRead,
                        get_lang(),
                        Some(&[&src, &format!("{:?}", value)])
                    )
                );
                self.regs.write(dst, value.clone());
                debug!(
                    "{}",
                    t(
                        MSG::VmRegWrite,
                        get_lang(),
                        Some(&[&dst, &format!("{:?}", value)])
                    )
                );
            }

            // 加载常量
            LoadConst => {
                let dst = self.read_u8()?;
                let const_idx = self.read_u16()? as usize;
                if let Some(const_val) = self.constants.get(const_idx) {
                    let value = self.const_to_value(const_val);
                    self.regs.write(dst, value.clone());
                    debug!(
                        "{}",
                        t(
                            MSG::VmRegWrite,
                            get_lang(),
                            Some(&[&dst, &format!("{:?}", value)])
                        )
                    );
                }
            }

            // 直接加载整数常量（I64）
            I64Const => {
                let dst = self.read_u8()?;
                let value = self.read_i64()?;
                self.regs.write(dst, RuntimeValue::Int(value));
            }

            // 直接加载浮点常量（F64）
            F64Const => {
                let dst = self.read_u8()?;
                let value = self.read_f64()?;
                self.regs.write(dst, RuntimeValue::Float(value));
            }

            // 直接加载整数常量（I32）
            I32Const => {
                let dst = self.read_u8()?;
                let value = self.read_i32()? as i64;
                self.regs.write(dst, RuntimeValue::Int(value));
            }

            // 加载局部变量
            LoadLocal => {
                let dst = self.read_u8()?;
                let local_idx = self.read_u8()? as usize;
                if let Some(frame) = self.call_stack.last() {
                    if local_idx < frame.locals.len() {
                        let value = frame.locals[local_idx].clone();
                        self.regs.write(dst, value.clone());
                    }
                }
            }

            // 存储局部变量
            StoreLocal => {
                let local_idx = self.read_u8()? as usize;
                let src = self.read_u8()?;
                let value = self.regs.read(src).clone();
                if let Some(frame) = self.call_stack.last_mut() {
                    if local_idx < frame.locals.len() {
                        frame.locals[local_idx] = value.clone();
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
                        debug!(
                            "{}",
                            t(
                                MSG::VmLoadArg,
                                get_lang(),
                                Some(&[&arg_idx, &format!("{:?}", value)])
                            )
                        );
                        self.regs.write(dst, value.clone());
                        debug!(
                            "{}",
                            t(
                                MSG::VmRegWrite,
                                get_lang(),
                                Some(&[&dst, &format!("{:?}", value)])
                            )
                        );
                    }
                }
            }

            // I64 算术运算
            I64Add => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| Ok(a + b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I64Sub => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| Ok(a - b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I64Mul => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| Ok(a * b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
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
                self.regs.write(dst, RuntimeValue::Int(result));
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
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I64Neg => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;
                if let RuntimeValue::Int(n) = self.regs.read(src) {
                    self.regs.write(dst, RuntimeValue::Int(-*n));
                }
            }

            // I64 比较运算
            I64Eq => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_i64(lhs, rhs, |a, b| a == b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            I64Ne => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_i64(lhs, rhs, |a, b| a != b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            I64Lt => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_i64(lhs, rhs, |a, b| a < b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            I64Le => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_i64(lhs, rhs, |a, b| a <= b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            I64Gt => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_i64(lhs, rhs, |a, b| a > b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            I64Ge => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_i64(lhs, rhs, |a, b| a >= b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            // 跳转指令
            Jmp => {
                let offset = self.read_i32()?;
                self.ip = (self.ip as i32 + offset) as usize;
            }

            JmpIf => {
                let cond = self.read_u8()?;
                let offset = self.read_i16()?;
                if let RuntimeValue::Int(n) = self.regs.read(cond) {
                    if *n != 0 {
                        self.ip = (self.ip as i32 + offset as i32) as usize;
                    }
                }
            }

            JmpIfNot => {
                let cond = self.read_u8()?;
                let offset = self.read_i16()?;
                if let RuntimeValue::Int(n) = self.regs.read(cond) {
                    if *n == 0 {
                        self.ip = (self.ip as i32 + offset as i32) as usize;
                    }
                }
            }

            // 函数调用
            CallStatic => {
                let dst = self.read_u8()?;
                let func_id = self.read_u32()?;
                let base_arg_reg = self.read_u8()?;
                let arg_count = self.read_u8()?;

                // 收集参数
                let mut args = Vec::with_capacity(arg_count as usize);
                for i in 0..arg_count {
                    let arg_reg = base_arg_reg + i;
                    args.push(self.regs.read(arg_reg).clone());
                }

                // 从函数 ID 解析函数名（从常量池查找）
                let func_name =
                    if let Some(ConstValue::String(name)) = self.constants.get(func_id as usize) {
                        name.clone()
                    } else {
                        // 如果不是有效常量，默认使用 "print"
                        "print".to_string()
                    };

                // 优先检查外部函数注册表
                if let Some(ext_func) = extfunc::EXTERNAL_FUNCTIONS.get(&func_name) {
                    // 调用外部函数
                    let result = (ext_func.func)(&args);
                    // 处理返回值（写入目标寄存器）
                    self.regs.write(dst, result);
                } else if func_name == "print" {
                    // 调用 print 内部函数（向后兼容）
                    if let Some(first_arg) = args.first() {
                        self.call_print(first_arg)?;
                    }
                } else if func_name == "println" {
                    // 调用 println 内部函数（向后兼容）
                    if let Some(first_arg) = args.first() {
                        self.call_println(first_arg)?;
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
                self.regs.write(dst, RuntimeValue::Float(result));
            }

            F64Sub => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_f64(lhs, rhs, |a, b| Ok(a - b))?;
                self.regs.write(dst, RuntimeValue::Float(result));
            }

            F64Mul => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_f64(lhs, rhs, |a, b| Ok(a * b))?;
                self.regs.write(dst, RuntimeValue::Float(result));
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
                self.regs.write(dst, RuntimeValue::Float(result));
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

    /// 读取 u64 操作数
    fn read_u64(&mut self) -> Result<u64, VMError> {
        if self.ip + 7 < self.bytecode.len() {
            let val = u64::from_le_bytes([
                self.bytecode[self.ip],
                self.bytecode[self.ip + 1],
                self.bytecode[self.ip + 2],
                self.bytecode[self.ip + 3],
                self.bytecode[self.ip + 4],
                self.bytecode[self.ip + 5],
                self.bytecode[self.ip + 6],
                self.bytecode[self.ip + 7],
            ]);
            self.ip += 8;
            Ok(val)
        } else {
            Err(VMError::InvalidOperand)
        }
    }

    /// 读取 i64 操作数
    fn read_i64(&mut self) -> Result<i64, VMError> {
        self.read_u64().map(|v| v as i64)
    }

    /// 读取 f64 操作数
    fn read_f64(&mut self) -> Result<f64, VMError> {
        self.read_u64().map(f64::from_bits)
    }

    /// 二进制 I64 运算
    fn binary_op_i64<F>(
        &self,
        lhs_reg: u8,
        rhs_reg: u8,
        op: F,
    ) -> Result<i64, VMError>
    where
        F: FnOnce(i64, i64) -> Result<i64, VMError>,
    {
        let lhs_val = self.regs.read(lhs_reg);
        let rhs_val = self.regs.read(rhs_reg);
        match (lhs_val, rhs_val) {
            (RuntimeValue::Int(a), RuntimeValue::Int(b)) => op(*a, *b),
            _ => Err(VMError::TypeError(format!(
                "integer (lhs: {:?}, rhs: {:?})",
                lhs_val, rhs_val
            ))),
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
            (RuntimeValue::Float(a), RuntimeValue::Float(b)) => op(*a, *b),
            (RuntimeValue::Int(a), RuntimeValue::Int(b)) => op(*a as f64, *b as f64),
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
        F: FnOnce(i64, i64) -> bool,
    {
        match (self.regs.read(lhs_reg), self.regs.read(rhs_reg)) {
            (RuntimeValue::Int(a), RuntimeValue::Int(b)) => op(*a, *b),
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
    ) -> VMResult<RuntimeValue> {
        // ReturnValue 指令的返回值已经在执行时处理
        Ok(RuntimeValue::Unit)
    }

    /// 调用 print 内部函数
    fn call_print(
        &self,
        value: &RuntimeValue,
    ) -> VMResult<()> {
        self.print_value(value, false);
        Ok(())
    }

    /// 调用 println 内部函数
    fn call_println(
        &self,
        value: &RuntimeValue,
    ) -> VMResult<()> {
        self.print_value(value, true);
        Ok(())
    }

    /// 打印值的通用实现
    fn print_value(
        &self,
        value: &RuntimeValue,
        newline: bool,
    ) {
        match value {
            RuntimeValue::String(s) => {
                if newline {
                    println!("{}", s);
                } else {
                    print!("{}", s);
                }
            }
            RuntimeValue::Int(n) => {
                if newline {
                    println!("{}", n);
                } else {
                    print!("{}", n);
                }
            }
            RuntimeValue::Float(f) => {
                if newline {
                    println!("{}", f);
                } else {
                    print!("{}", f);
                }
            }
            RuntimeValue::Bool(b) => {
                if newline {
                    println!("{}", b);
                } else {
                    print!("{}", b);
                }
            }
            RuntimeValue::Char(c) => {
                if newline {
                    println!("{}", c);
                } else {
                    print!("{}", c);
                }
            }
            RuntimeValue::Unit => {
                if newline {
                    println!("()");
                } else {
                    print!("()");
                }
            }
            _ => {
                if newline {
                    println!("{:?}", value);
                } else {
                    print!("{:?}", value);
                }
            }
        }
    }

    /// 将 ConstValue 转换为 RuntimeValue
    fn const_to_value(
        &self,
        const_val: &ConstValue,
    ) -> RuntimeValue {
        match const_val {
            ConstValue::Void => RuntimeValue::Unit,
            ConstValue::Bool(b) => RuntimeValue::Bool(*b),
            ConstValue::Int(n) => RuntimeValue::Int(*n as i64),
            ConstValue::Float(f) => RuntimeValue::Float(*f),
            ConstValue::Char(c) => RuntimeValue::Char(*c as u32),
            ConstValue::String(s) => RuntimeValue::String(s.clone().into()),
            ConstValue::Bytes(b) => RuntimeValue::Bytes(b.clone().into()),
        }
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
