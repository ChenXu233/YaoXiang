//! Virtual Machine executor
//!
//! 实现 YaoXiang VM 字节码执行器，支持寄存器架构和函数调用。

use crate::middle::codegen::bytecode::FunctionCode;
use crate::middle::ir::{ConstValue, Operand};
use crate::runtime::value::{RuntimeValue, Heap, HeapValue, Handle, FunctionValue, FunctionId};
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
    /// 内存空间（用于 Load/Store 指令）
    memory: Vec<u8>,
    /// 跳转表（用于 Switch 指令）
    jump_tables: HashMap<u16, HashMap<i64, i32>>,
    /// 标签位置（用于 Label 指令）
    labels: HashMap<u16, usize>,
    /// 堆存储（用于 List/Tuple/Struct/Dict）
    heap: Heap,
    /// 当前闭包环境（upvalue 存储）
    closure_env: Vec<RuntimeValue>,
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
            memory: Vec::with_capacity(64 * 1024),
            jump_tables: HashMap::new(),
            labels: HashMap::new(),
            heap: Heap::new(),
            closure_env: Vec::new(),
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
                // 无返回值，压入 Unit 到值栈
                self.value_stack.push(RuntimeValue::Unit);
                return Ok(());
            }

            ReturnValue => {
                // 从返回值寄存器读取返回值
                let ret_reg = self.read_u8()?;
                let ret_value = self.regs.read(ret_reg).clone();
                // 压入返回值到值栈
                self.value_stack.push(ret_value.clone());
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

            // I64 位运算
            I64And => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| Ok(a & b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I64Or => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| Ok(a | b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I64Xor => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| Ok(a ^ b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I64Shl => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| Ok(a << b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I64Sar => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| Ok(a >> b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I64Shr => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i64(lhs, rhs, |a, b| Ok(a >> b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
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

            // =====================
            // I32 算术运算指令
            // =====================
            I32Add => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i32(lhs, rhs, |a, b| Ok(a + b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I32Sub => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i32(lhs, rhs, |a, b| Ok(a - b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I32Mul => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i32(lhs, rhs, |a, b| Ok(a * b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I32Div => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i32(lhs, rhs, |a, b| {
                    if b == 0 {
                        Err(VMError::DivisionByZero)
                    } else {
                        Ok(a / b)
                    }
                })?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I32Rem => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i32(lhs, rhs, |a, b| {
                    if b == 0 {
                        Err(VMError::DivisionByZero)
                    } else {
                        Ok(a % b)
                    }
                })?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I32Neg => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;
                if let RuntimeValue::Int(n) = self.regs.read(src) {
                    self.regs.write(dst, RuntimeValue::Int(-*n));
                }
            }

            // I32 位运算
            I32And => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i32(lhs, rhs, |a, b| Ok(a & b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I32Or => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i32(lhs, rhs, |a, b| Ok(a | b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I32Xor => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i32(lhs, rhs, |a, b| Ok(a ^ b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I32Shl => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i32(lhs, rhs, |a, b| Ok(a << b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I32Sar => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i32(lhs, rhs, |a, b| Ok(a >> b))?;
                self.regs.write(dst, RuntimeValue::Int(result));
            }

            I32Shr => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_i32(lhs, rhs, |a, b| Ok((a as i32 >> b) as i64))?;
                self.regs.write(dst, RuntimeValue::Int(result));
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

            // =====================
            // 控制流指令
            // =====================

            // Switch: 多分支跳转
            // 操作数: reg (u8), table_idx (u16), default_offset (i16)
            Switch => {
                let reg = self.read_u8()?;
                let table_idx = self.read_u16()?;
                let default_offset = self.read_i16()?;

                let value = match self.regs.read(reg) {
                    RuntimeValue::Int(n) => *n,
                    _ => {
                        // 非整数值使用默认分支
                        self.ip = (self.ip as i32 + default_offset as i32) as usize;
                        return Ok(());
                    }
                };

                // 查找跳转表
                if let Some(jump_table) = self.jump_tables.get(&table_idx) {
                    if let Some(&case_offset) = jump_table.get(&value) {
                        // 找到匹配 case，跳转到目标地址
                        self.ip = (self.ip as i32 + case_offset) as usize;
                        return Ok(());
                    }
                }

                // 默认跳转
                self.ip = (self.ip as i32 + default_offset as i32) as usize;
            }

            // Label: 跳转标签（记录位置）
            // 操作数: label_id (u16)
            Label => {
                let label_id = self.read_u16()?;
                self.labels.insert(label_id, self.ip);
            }

            // LoopStart: 循环开始标记（无操作）
            LoopStart => {
                // 循环开始标记，不执行任何操作，仅用于调试/分析
            }

            // LoopInc: 循环递增
            // 操作数: counter_reg (u8), step_reg (u8), iterations_reg (u8)
            LoopInc => {
                let counter_reg = self.read_u8()?;
                let step_reg = self.read_u8()?;
                let iterations_reg = self.read_u8()?;

                let counter = match self.regs.read(counter_reg) {
                    RuntimeValue::Int(n) => *n,
                    _ => return Ok(()),
                };
                let step = match self.regs.read(step_reg) {
                    RuntimeValue::Int(n) => *n,
                    _ => return Ok(()),
                };
                let iterations = match self.regs.read(iterations_reg) {
                    RuntimeValue::Int(n) => *n,
                    _ => return Ok(()),
                };

                let new_counter = counter + step;
                // 检查是否达到迭代次数
                if step > 0 && new_counter >= iterations {
                    // 循环结束，不执行循环体
                    // 这里需要外部设置 loop_end_ip，暂时跳过
                } else if step < 0 && new_counter <= iterations {
                    // 倒序循环结束
                } else {
                    // 更新计数器，继续循环
                    self.regs.write(counter_reg, RuntimeValue::Int(new_counter));
                }
            }

            // Yield: 协程让出
            Yield => {
                // 协程让出，当前实现为无操作
                // 完整实现需要保存 VM 状态并支持恢复
            }

            // =====================
            // 函数调用指令
            // =====================

            // TailCall: 尾调用优化
            // 操作数: func_id (u32), base_arg (u8), arg_count (u8), _pad (u8)
            TailCall => {
                let func_id = self.read_u32()?;
                let base_arg = self.read_u8()?;
                let arg_count = self.read_u8()?;
                let _pad = self.read_u8()?; // 对齐填充

                // 从函数 ID 解析函数名
                let func_name =
                    if let Some(ConstValue::String(name)) = self.constants.get(func_id as usize) {
                        name.clone()
                    } else {
                        return Err(VMError::InvalidState(format!(
                            "Invalid function ID: {}",
                            func_id
                        )));
                    };

                // 收集参数
                let mut args = Vec::with_capacity(arg_count as usize);
                for i in 0..arg_count {
                    let arg_reg = base_arg + i;
                    let arg_value = self.regs.read(arg_reg).clone();
                    args.push(arg_value);
                }

                // 查找目标函数并 clone
                let target_func = if let Some(func) = self.functions.get(&func_name) {
                    func.clone()
                } else {
                    return Err(VMError::RuntimeError(format!(
                        "Function not found: {}",
                        func_name
                    )));
                };

                // 验证参数数量
                if target_func.params.len() != args.len() {
                    return Err(VMError::RuntimeError(format!(
                        "Argument count mismatch for tail call: expected {}, got {}",
                        target_func.params.len(),
                        args.len()
                    )));
                }

                // 尾调用：复用当前栈帧
                // 保存当前状态
                let saved_bytecode = self.bytecode.clone();
                let saved_ip = self.ip;
                let saved_func = self.current_func.clone();

                // 设置新函数
                self.current_func = Some(target_func.clone());
                self.bytecode = target_func.encode_all();
                self.ip = 0;

                // 初始化参数
                for (i, arg) in args.iter().enumerate() {
                    self.regs.write(i as u8, arg.clone());
                }

                // 初始化局部变量
                let local_count = target_func.local_count;
                for i in target_func.params.len()..local_count {
                    self.regs.write(i as u8, RuntimeValue::Unit);
                }

                // 执行新函数体
                let result = self.execute_function_body(&target_func)?;

                // 恢复字节码和 IP
                self.bytecode = saved_bytecode;
                self.ip = saved_ip;
                self.current_func = saved_func;

                // 将返回值写入寄存器 0
                self.regs.write(0, result);
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
                    let arg_value = self.regs.read(arg_reg).clone();
                    args.push(arg_value.clone());
                    // 将参数压入值栈
                    self.value_stack.push(arg_value);
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
                    // 弹出参数
                    for _ in 0..arg_count {
                        self.value_stack.pop();
                    }
                    // 处理返回值（写入目标寄存器并压入值栈）
                    self.regs.write(dst, result.clone());
                    self.value_stack.push(result);
                } else if func_name == "print" {
                    // 调用 print 内部函数（向后兼容）
                    if let Some(first_arg) = args.first() {
                        self.call_print(first_arg)?;
                    }
                    // 弹出参数
                    for _ in 0..arg_count {
                        self.value_stack.pop();
                    }
                } else if func_name == "println" {
                    // 调用 println 内部函数（向后兼容）
                    if let Some(first_arg) = args.first() {
                        self.call_println(first_arg)?;
                    }
                    // 弹出参数
                    for _ in 0..arg_count {
                        self.value_stack.pop();
                    }
                } else if let Some(target_func) = self.functions.get(&func_name) {
                    // 调用用户定义函数 - 克隆函数以避免借用冲突
                    let func_clone = target_func.clone();
                    let result = self.execute_function(&func_clone, &args)?;
                    // 函数返回值已压入值栈（在 ReturnValue 处理中）
                    self.regs.write(dst, result.clone());
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

            // F64 取模
            F64Rem => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_f64(lhs, rhs, |a, b| {
                    if b == 0.0 {
                        Err(VMError::DivisionByZero)
                    } else {
                        Ok(a % b)
                    }
                })?;
                self.regs.write(dst, RuntimeValue::Float(result));
            }

            // F64 平方根
            F64Sqrt => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;
                if let RuntimeValue::Float(f) = self.regs.read(src) {
                    self.regs.write(dst, RuntimeValue::Float(f.sqrt()));
                }
            }

            // F64 取负
            F64Neg => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;
                if let RuntimeValue::Float(f) = self.regs.read(src) {
                    self.regs.write(dst, RuntimeValue::Float(-f));
                }
            }

            // =====================
            // 内存读写指令
            // =====================
            // I64Load: dst = *(base + offset)
            I64Load => {
                let dst = self.read_u8()?;
                let base = self.read_u8()?;
                let offset = self.read_i16()?;
                if let RuntimeValue::Int(addr) = self.regs.read(base) {
                    let addr = (*addr + offset as i64) as usize;
                    if let Some(bytes) = self.memory_read_i64(addr) {
                        self.regs.write(dst, RuntimeValue::Int(bytes));
                    }
                }
            }

            // I64Store: *(base + offset) = src
            I64Store => {
                let base = self.read_u8()?;
                let offset = self.read_i16()?;
                let src = self.read_u8()?;
                if let (RuntimeValue::Int(addr), RuntimeValue::Int(value)) =
                    (self.regs.read(base), self.regs.read(src))
                {
                    let addr = (*addr + offset as i64) as usize;
                    self.memory_write_i64(addr, *value);
                }
            }

            // I32Load: dst = *(base + offset) as i32
            I32Load => {
                let dst = self.read_u8()?;
                let base = self.read_u8()?;
                let offset = self.read_i16()?;
                if let RuntimeValue::Int(addr) = self.regs.read(base) {
                    let addr = (*addr + offset as i64) as usize;
                    if let Some(bytes) = self.memory_read_i32(addr) {
                        self.regs.write(dst, RuntimeValue::Int(bytes as i64));
                    }
                }
            }

            // I32Store: *(base + offset) = src as i32
            I32Store => {
                let base = self.read_u8()?;
                let offset = self.read_i16()?;
                let src = self.read_u8()?;
                if let (RuntimeValue::Int(addr), RuntimeValue::Int(value)) =
                    (self.regs.read(base), self.regs.read(src))
                {
                    let addr = (*addr + offset as i64) as usize;
                    self.memory_write_i32(addr, *value as i32);
                }
            }

            // F64Load: dst = *(base + offset) as f64
            F64Load => {
                let dst = self.read_u8()?;
                let base = self.read_u8()?;
                let offset = self.read_i16()?;
                if let RuntimeValue::Int(addr) = self.regs.read(base) {
                    let addr = (*addr + offset as i64) as usize;
                    if let Some(bytes) = self.memory_read_f64(addr) {
                        self.regs.write(dst, RuntimeValue::Float(bytes));
                    }
                }
            }

            // F64Store: *(base + offset) = src as f64
            F64Store => {
                let base = self.read_u8()?;
                let offset = self.read_i16()?;
                let src = self.read_u8()?;
                if let (RuntimeValue::Int(addr), RuntimeValue::Float(value)) =
                    (self.regs.read(base), self.regs.read(src))
                {
                    let addr = (*addr + offset as i64) as usize;
                    self.memory_write_f64(addr, *value);
                }
            }

            // F32Load: dst = *(base + offset) as f32
            F32Load => {
                let dst = self.read_u8()?;
                let base = self.read_u8()?;
                let offset = self.read_i16()?;
                if let RuntimeValue::Int(addr) = self.regs.read(base) {
                    let addr = (*addr + offset as i64) as usize;
                    if let Some(bytes) = self.memory_read_f32(addr) {
                        self.regs.write(dst, RuntimeValue::Float(bytes as f64));
                    }
                }
            }

            // F32Store: *(base + offset) = src as f32
            F32Store => {
                let base = self.read_u8()?;
                let offset = self.read_i16()?;
                let src = self.read_u8()?;
                if let (RuntimeValue::Int(addr), RuntimeValue::Float(value)) =
                    (self.regs.read(base), self.regs.read(src))
                {
                    let addr = (*addr + offset as i64) as usize;
                    self.memory_write_f32(addr, *value as f32);
                }
            }

            // F64 比较运算
            F64Eq => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_f64(lhs, rhs, |a, b| a == b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            F64Ne => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_f64(lhs, rhs, |a, b| a != b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            F64Lt => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_f64(lhs, rhs, |a, b| a < b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            F64Le => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_f64(lhs, rhs, |a, b| a <= b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            F64Gt => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_f64(lhs, rhs, |a, b| a > b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            F64Ge => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_f64(lhs, rhs, |a, b| a >= b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            // =====================
            // F32 算术运算指令
            // =====================
            F32Add => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_f32(lhs, rhs, |a, b| Ok(a + b))?;
                self.regs.write(dst, RuntimeValue::Float(result));
            }

            F32Sub => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_f32(lhs, rhs, |a, b| Ok(a - b))?;
                self.regs.write(dst, RuntimeValue::Float(result));
            }

            F32Mul => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_f32(lhs, rhs, |a, b| Ok(a * b))?;
                self.regs.write(dst, RuntimeValue::Float(result));
            }

            F32Div => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_f32(lhs, rhs, |a, b| {
                    if b == 0.0 {
                        Err(VMError::DivisionByZero)
                    } else {
                        Ok(a / b)
                    }
                })?;
                self.regs.write(dst, RuntimeValue::Float(result));
            }

            F32Rem => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.binary_op_f32(lhs, rhs, |a, b| {
                    if b == 0.0 {
                        Err(VMError::DivisionByZero)
                    } else {
                        Ok(a % b)
                    }
                })?;
                self.regs.write(dst, RuntimeValue::Float(result));
            }

            F32Sqrt => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;
                if let RuntimeValue::Float(f) = self.regs.read(src) {
                    self.regs.write(dst, RuntimeValue::Float(f.sqrt()));
                }
            }

            F32Neg => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;
                if let RuntimeValue::Float(f) = self.regs.read(src) {
                    self.regs.write(dst, RuntimeValue::Float(-f));
                }
            }

            F32Const => {
                let dst = self.read_u8()?;
                let value = self.read_f32()?;
                self.regs.write(dst, RuntimeValue::Float(value));
            }

            // F32 比较运算
            F32Eq => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_f32(lhs, rhs, |a, b| a == b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            F32Ne => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_f32(lhs, rhs, |a, b| a != b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            F32Lt => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_f32(lhs, rhs, |a, b| a < b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            F32Le => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_f32(lhs, rhs, |a, b| a <= b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            F32Gt => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_f32(lhs, rhs, |a, b| a > b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            F32Ge => {
                let dst = self.read_u8()?;
                let lhs = self.read_u8()?;
                let rhs = self.read_u8()?;
                let result = self.compare_f32(lhs, rhs, |a, b| a >= b);
                self.regs.write(dst, RuntimeValue::Bool(result));
            }

            // =====================
            // 字符串操作指令
            // =====================
            StringLength => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;
                if let RuntimeValue::String(s) = self.regs.read(src) {
                    self.regs.write(dst, RuntimeValue::Int(s.len() as i64));
                }
            }

            StringConcat => {
                let dst = self.read_u8()?;
                let str1 = self.read_u8()?;
                let str2 = self.read_u8()?;
                match (self.regs.read(str1), self.regs.read(str2)) {
                    (RuntimeValue::String(s1), RuntimeValue::String(s2)) => {
                        let mut result = String::new();
                        result.push_str(s1.as_ref());
                        result.push_str(s2.as_ref());
                        self.regs.write(dst, RuntimeValue::String(result.into()));
                    }
                    _ => return Err(VMError::TypeError("string".to_string())),
                }
            }

            StringEqual => {
                let dst = self.read_u8()?;
                let str1 = self.read_u8()?;
                let str2 = self.read_u8()?;
                match (self.regs.read(str1), self.regs.read(str2)) {
                    (RuntimeValue::String(s1), RuntimeValue::String(s2)) => {
                        self.regs
                            .write(dst, RuntimeValue::Bool(s1.as_ref() == s2.as_ref()));
                    }
                    _ => return Err(VMError::TypeError("string".to_string())),
                }
            }

            StringGetChar => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;
                let idx = self.read_u8()?;
                if let RuntimeValue::String(s) = self.regs.read(src) {
                    if let Some(ch) = s.chars().nth(idx as usize) {
                        self.regs.write(dst, RuntimeValue::Char(ch as u32));
                    }
                }
            }

            StringFromInt => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;
                if let RuntimeValue::Int(n) = self.regs.read(src) {
                    self.regs
                        .write(dst, RuntimeValue::String(n.to_string().into()));
                }
            }

            StringFromFloat => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;
                if let RuntimeValue::Float(f) = self.regs.read(src) {
                    self.regs
                        .write(dst, RuntimeValue::String(f.to_string().into()));
                }
            }

            // =====================
            // 闭包操作指令
            // =====================
            MakeClosure => {
                let dst = self.read_u8()?;
                let func_id = self.read_u32()?;
                let upvalue_count = self.read_u8()?;

                // 从常量池获取函数名（用于错误信息）
                let _func_name = match self.constants.get(func_id as usize) {
                    Some(ConstValue::String(name)) => name.clone(),
                    _ => {
                        return Err(VMError::RuntimeError(format!(
                            "Invalid function ID for closure: {}",
                            func_id
                        )));
                    }
                };

                // 收集 upvalue（从 closure_env 中获取前 upvalue_count 个）
                let mut env = Vec::with_capacity(upvalue_count as usize);
                for i in 0..upvalue_count {
                    if let Some(value) = self.closure_env.get(i as usize) {
                        env.push(value.clone());
                    } else {
                        env.push(RuntimeValue::Unit);
                    }
                }

                // 创建闭包
                let closure = RuntimeValue::Function(FunctionValue {
                    func_id: FunctionId(func_id),
                    env,
                });
                self.regs.write(dst, closure);
            }

            LoadUpvalue => {
                let dst = self.read_u8()?;
                let upvalue_idx = self.read_u8()?;

                // 从闭包环境加载
                if let Some(value) = self.closure_env.get(upvalue_idx as usize) {
                    self.regs.write(dst, value.clone());
                } else {
                    self.regs.write(dst, RuntimeValue::Unit);
                }
            }

            StoreUpvalue => {
                let src = self.read_u8()?;
                let upvalue_idx = self.read_u8()?;

                let value = self.regs.read(src).clone();

                // 确保 closure_env 有足够的空间
                if upvalue_idx as usize >= self.closure_env.len() {
                    self.closure_env.resize(upvalue_idx as usize + 1, RuntimeValue::Unit);
                }
                self.closure_env[upvalue_idx as usize] = value;
            }

            CloseUpvalue => {
                let reg = self.read_u8()?;

                // 将栈上的变量移动到闭包环境（堆分配）
                let value = self.regs.read(reg).clone();

                // 找到一个空的 upvalue 位置或添加到末尾
                let mut empty_idx = None;
                for (i, v) in self.closure_env.iter().enumerate() {
                    if matches!(v, RuntimeValue::Unit) {
                        empty_idx = Some(i);
                        break;
                    }
                }

                match empty_idx {
                    Some(idx) => {
                        self.closure_env[idx] = value;
                    }
                    None => {
                        self.closure_env.push(value);
                    }
                }
            }

            // =====================
            // 异常处理指令
            // =====================
            TryBegin => {
                let _catch_offset = self.read_u16()?;
                // Try 块开始（简化处理）
            }

            TryEnd => {
                // Try 块结束（简化处理）
            }

            Throw => {
                let _exception_reg = self.read_u8()?;
                // 抛出异常（简化处理）
                return Err(VMError::RuntimeError("exception thrown".to_string()));
            }

            Rethrow => {
                // 重新抛出异常（简化处理）
                return Err(VMError::RuntimeError("exception rethrown".to_string()));
            }

            // =====================
            // 内存与对象操作指令
            // =====================
            StackAlloc => {
                let _size = self.read_u16()?;
                // 栈分配（简化处理）
            }

            HeapAlloc => {
                let _dst = self.read_u8()?;
                let _type_id = self.read_u16()?;
                // 堆分配（简化处理）
            }

            GetField => {
                let dst = self.read_u8()?;
                let obj_reg = self.read_u8()?;
                let field_offset = self.read_u16()? as usize;

                let value = match self.regs.read(obj_reg).clone() {
                    RuntimeValue::List(handle) | RuntimeValue::Tuple(handle) => {
                        let h = handle.0;
                        if let Some(HeapValue::List(items)) = self.heap.get(Handle(h)) {
                            items
                                .get(field_offset)
                                .cloned()
                                .unwrap_or(RuntimeValue::Unit)
                        } else if let Some(HeapValue::Tuple(items)) = self.heap.get(Handle(h)) {
                            items
                                .get(field_offset)
                                .cloned()
                                .unwrap_or(RuntimeValue::Unit)
                        } else {
                            RuntimeValue::Unit
                        }
                    }
                    RuntimeValue::Struct { fields, .. } => {
                        let h = fields.0;
                        if let Some(HeapValue::Tuple(items)) = self.heap.get(Handle(h)) {
                            items
                                .get(field_offset)
                                .cloned()
                                .unwrap_or(RuntimeValue::Unit)
                        } else {
                            RuntimeValue::Unit
                        }
                    }
                    _ => return Err(VMError::TypeError("list/tuple/struct".into())),
                };
                self.regs.write(dst, value);
            }

            SetField => {
                let obj_reg = self.read_u8()?;
                let field_offset = self.read_u16()? as usize;
                let src_reg = self.read_u8()?;

                let value = self.regs.read(src_reg).clone();

                match self.regs.read(obj_reg).clone() {
                    RuntimeValue::List(handle) => {
                        let h = handle.0;
                        if let Some(HeapValue::List(items)) = self.heap.get_mut(Handle(h)) {
                            if field_offset >= items.len() {
                                return Err(VMError::IndexOutOfBounds {
                                    index: field_offset,
                                    size: items.len(),
                                });
                            }
                            items[field_offset] = value;
                        } else {
                            return Err(VMError::TypeError("list".into()));
                        }
                    }
                    RuntimeValue::Struct {
                        type_id: _,
                        fields: field_handle,
                    } => {
                        let h = field_handle.0;
                        if let Some(HeapValue::Tuple(items)) = self.heap.get_mut(Handle(h)) {
                            if field_offset >= items.len() {
                                return Err(VMError::FieldNotFound(field_offset));
                            }
                            items[field_offset] = value;
                        } else {
                            return Err(VMError::TypeError("struct fields".into()));
                        }
                    }
                    _ => return Err(VMError::TypeError("list/struct".into())),
                }
            }

            LoadElement => {
                let dst = self.read_u8()?;
                let array_reg = self.read_u8()?;
                let index_reg = self.read_u8()?;
                match (self.regs.read(array_reg), self.regs.read(index_reg)) {
                    (RuntimeValue::List(handle), RuntimeValue::Int(idx)) => {
                        let h = handle.0;
                        if let Some(HeapValue::List(items)) = self.heap.get(Handle(h)) {
                            if let Some(val) = items.get(*idx as usize) {
                                self.regs.write(dst, val.clone());
                            }
                        }
                    }
                    _ => return Err(VMError::TypeError("list or index".to_string())),
                }
            }

            StoreElement => {
                let array_reg = self.read_u8()?;
                let index_reg = self.read_u8()?;
                let src_reg = self.read_u8()?;

                let value = self.regs.read(src_reg).clone();
                let idx = match self.regs.read(index_reg) {
                    RuntimeValue::Int(n) => *n as usize,
                    _ => return Err(VMError::TypeError("int".into())),
                };

                match self.regs.read(array_reg).clone() {
                    RuntimeValue::List(handle) => {
                        let h = handle.0;
                        if let Some(HeapValue::List(items)) = self.heap.get_mut(Handle(h)) {
                            if idx >= items.len() {
                                return Err(VMError::IndexOutOfBounds {
                                    index: idx,
                                    size: items.len(),
                                });
                            }
                            items[idx] = value;
                        } else {
                            return Err(VMError::TypeError("list".into()));
                        }
                    }
                    _ => return Err(VMError::TypeError("list".into())),
                }
            }

            NewListWithCap => {
                let dst = self.read_u8()?;
                let capacity = self.read_u16()? as usize;
                let handle = self
                    .heap
                    .allocate(HeapValue::List(Vec::with_capacity(capacity)));
                self.regs.write(dst, RuntimeValue::List(handle));
            }

            ArcNew => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;

                // Arc 创建：将值包装成 Arc
                // 对应 ref 关键字的运行时行为
                let value = self.regs.read(src).clone();
                let arc_value = value.into_arc();
                self.regs.write(dst, arc_value);
            }

            ArcClone => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;

                // Arc 克隆：增加引用计数
                // 由于 RuntimeValue::Arc 内部使用 Arc，自动处理引用计数
                if let RuntimeValue::Arc(arc_ref) = self.regs.read(src).clone() {
                    // Arc::clone 会增加引用计数
                    self.regs.write(dst, RuntimeValue::Arc(arc_ref));
                } else {
                    // 非 Arc 类型，clone 后创建新的 Arc
                    let value = self.regs.read(src).clone();
                    self.regs.write(dst, value.into_arc());
                }
            }

            ArcDrop => {
                let src = self.read_u8()?;

                // Arc 释放：减少引用计数
                // 由于使用 Rust 的 Arc，引用计数在 Arc 被 drop 时自动减少
                // 这里只需要清除寄存器中的引用即可
                self.regs.write(src, RuntimeValue::Unit);
            }

            // =====================
            // 类型操作指令
            // =====================
            TypeCheck => {
                let _obj_reg = self.read_u8()?;
                let _type_id = self.read_u16()?;
                let _dst = self.read_u8()?;
                // 类型检查（简化处理）
            }

            Cast => {
                let dst = self.read_u8()?;
                let src = self.read_u8()?;
                let target_type_id = self.read_u16()?;

                // 类型转换
                // target_type_id 定义：
                // 0: I64 → F64
                // 1: F64 → I64
                // 2: I64 → I32
                // 3: I32 → I64
                // 4: F64 → F32
                // 5: F32 → F64

                match target_type_id {
                    0 => {
                        // I64 → F64
                        if let RuntimeValue::Int(n) = self.regs.read(src) {
                            self.regs.write(dst, RuntimeValue::Float(*n as f64));
                        } else {
                            return Err(VMError::TypeError("int".into()));
                        }
                    }
                    1 => {
                        // F64 → I64
                        if let RuntimeValue::Float(f) = self.regs.read(src) {
                            self.regs.write(dst, RuntimeValue::Int(*f as i64));
                        } else {
                            return Err(VMError::TypeError("float".into()));
                        }
                    }
                    2 => {
                        // I64 → I32
                        if let RuntimeValue::Int(n) = self.regs.read(src) {
                            self.regs.write(dst, RuntimeValue::Int(*n as i32 as i64));
                        } else {
                            return Err(VMError::TypeError("int".into()));
                        }
                    }
                    3 => {
                        // I32 → I64
                        if let RuntimeValue::Int(n) = self.regs.read(src) {
                            self.regs.write(dst, RuntimeValue::Int(*n as i64));
                        } else {
                            return Err(VMError::TypeError("int".into()));
                        }
                    }
                    4 => {
                        // F64 → F32
                        if let RuntimeValue::Float(f) = self.regs.read(src) {
                            self.regs.write(dst, RuntimeValue::Float(*f as f32 as f64));
                        } else {
                            return Err(VMError::TypeError("float".into()));
                        }
                    }
                    5 => {
                        // F32 → F64
                        if let RuntimeValue::Float(f) = self.regs.read(src) {
                            self.regs.write(dst, RuntimeValue::Float(*f as f64));
                        } else {
                            return Err(VMError::TypeError("float".into()));
                        }
                    }
                    _ => {
                        return Err(VMError::RuntimeError(format!(
                            "Unsupported cast type: {}",
                            target_type_id
                        )));
                    }
                }
            }

            TypeOf => {
                let dst = self.read_u8()?;
                let _type_id = self.read_u16()?;
                // 类型获取（简化处理）
                self.regs.write(dst, RuntimeValue::Unit);
            }

            // =====================
            // 边界检查指令
            // =====================
            BoundsCheck => {
                let _array_reg = self.read_u8()?;
                let _index_reg = self.read_u8()?;
                let _dst = self.read_u8()?;
                // 边界检查（简化处理）
            }

            // 其他指令
            Drop => {
                let _reg = self.read_u8()?;
                // 简化：忽略 Drop
            }

            // 未实现的指令 - 返回错误而不是静默忽略
            _ => return Err(VMError::UnimplementedOpcode(opcode)),
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

    /// 读取 f32 操作数
    fn read_f32(&mut self) -> Result<f64, VMError> {
        self.read_u32().map(|v| f32::from_bits(v) as f64)
    }

    // =====================
    // 内存读写辅助方法
    // =====================

    /// 从内存读取 i64
    fn memory_read_i64(
        &self,
        addr: usize,
    ) -> Option<i64> {
        if addr + 8 <= self.memory.len() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.memory[addr..addr + 8]);
            Some(i64::from_le_bytes(bytes))
        } else {
            None
        }
    }

    /// 向内存写入 i64
    fn memory_write_i64(
        &mut self,
        addr: usize,
        value: i64,
    ) {
        let bytes = value.to_le_bytes();
        if addr + 8 <= self.memory.len() {
            self.memory[addr..addr + 8].copy_from_slice(&bytes);
        } else if addr <= self.memory.len() {
            // 扩展内存
            self.memory.resize(addr + 8, 0);
            self.memory[addr..addr + 8].copy_from_slice(&bytes);
        }
    }

    /// 从内存读取 i32
    fn memory_read_i32(
        &self,
        addr: usize,
    ) -> Option<i32> {
        if addr + 4 <= self.memory.len() {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&self.memory[addr..addr + 4]);
            Some(i32::from_le_bytes(bytes))
        } else {
            None
        }
    }

    /// 向内存写入 i32
    fn memory_write_i32(
        &mut self,
        addr: usize,
        value: i32,
    ) {
        let bytes = value.to_le_bytes();
        if addr + 4 <= self.memory.len() {
            self.memory[addr..addr + 4].copy_from_slice(&bytes);
        } else if addr <= self.memory.len() {
            self.memory.resize(addr + 4, 0);
            self.memory[addr..addr + 4].copy_from_slice(&bytes);
        }
    }

    /// 从内存读取 f64
    fn memory_read_f64(
        &self,
        addr: usize,
    ) -> Option<f64> {
        if addr + 8 <= self.memory.len() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.memory[addr..addr + 8]);
            Some(f64::from_le_bytes(bytes))
        } else {
            None
        }
    }

    /// 向内存写入 f64
    fn memory_write_f64(
        &mut self,
        addr: usize,
        value: f64,
    ) {
        let bytes = value.to_le_bytes();
        if addr + 8 <= self.memory.len() {
            self.memory[addr..addr + 8].copy_from_slice(&bytes);
        } else if addr <= self.memory.len() {
            self.memory.resize(addr + 8, 0);
            self.memory[addr..addr + 8].copy_from_slice(&bytes);
        }
    }

    /// 从内存读取 f32
    fn memory_read_f32(
        &self,
        addr: usize,
    ) -> Option<f32> {
        if addr + 4 <= self.memory.len() {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&self.memory[addr..addr + 4]);
            Some(f32::from_le_bytes(bytes))
        } else {
            None
        }
    }

    /// 向内存写入 f32
    fn memory_write_f32(
        &mut self,
        addr: usize,
        value: f32,
    ) {
        let bytes = value.to_le_bytes();
        if addr + 4 <= self.memory.len() {
            self.memory[addr..addr + 4].copy_from_slice(&bytes);
        } else if addr <= self.memory.len() {
            self.memory.resize(addr + 4, 0);
            self.memory[addr..addr + 4].copy_from_slice(&bytes);
        }
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

    /// 二进制 I32 运算
    fn binary_op_i32<F>(
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

    /// 二进制 F32 运算
    fn binary_op_f32<F>(
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

    /// F32 比较
    fn compare_f32<F>(
        &self,
        lhs_reg: u8,
        rhs_reg: u8,
        op: F,
    ) -> bool
    where
        F: FnOnce(f64, f64) -> bool,
    {
        match (self.regs.read(lhs_reg), self.regs.read(rhs_reg)) {
            (RuntimeValue::Float(a), RuntimeValue::Float(b)) => op(*a, *b),
            _ => false,
        }
    }

    /// F64 比较
    fn compare_f64<F>(
        &self,
        lhs_reg: u8,
        rhs_reg: u8,
        op: F,
    ) -> bool
    where
        F: FnOnce(f64, f64) -> bool,
    {
        match (self.regs.read(lhs_reg), self.regs.read(rhs_reg)) {
            (RuntimeValue::Float(a), RuntimeValue::Float(b)) => op(*a, *b),
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
