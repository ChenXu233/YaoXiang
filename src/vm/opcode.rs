//! 字节码操作码定义
//!
//! 类型化指令设计：每条指令都携带明确的类型信息，
//! 运行时无需类型检查，直接执行对应的 CPU 指令。
//!
//! 指令编码空间规划：
//! - 0x00-0x1F：控制流与基础操作
//! - 0x20-0x3F：I64/I32 整数运算
//! - 0x40-0x5F：F64/F32 浮点运算
//! - 0x60-0x7F：比较与逻辑运算
//! - 0x80-0x8F：内存与对象操作
//! - 0x90-0x9F：函数调用
//! - 0xA0-0xAF：字符串操作
//! - 0xB0-0xBF：异常处理
//! - 0xC0-0xCF：类型操作
//! - 0xD0-0xDF：反射操作
//! - 0xE0-0xFF：保留与自定义

use std::fmt;

/// 字节码操作码
///
/// 类型化指令设计：每条指令都携带明确的类型信息，
/// 运行时无需类型检查，直接执行对应的 CPU 指令。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TypedOpcode {
    // =====================
    // 基础控制流指令 (0x00-0x1F)
    // =====================
    /// 空操作
    Nop = 0x00,

    /// 无返回值返回
    Return = 0x01,

    /// 带返回值返回
    ReturnValue = 0x02,

    /// 无条件跳转
    /// 操作数：offset (i32，相对偏移量)
    Jmp = 0x03,

    /// 条件跳转（条件为真时跳转）
    /// 操作数：cond_reg (u8，条件寄存器), offset (i16，跳转偏移)
    JmpIf = 0x04,

    /// 条件跳转（条件为假时跳转）
    /// 操作数：cond_reg (u8，条件寄存器), offset (i16，跳转偏移)
    JmpIfNot = 0x05,

    /// 比较并条件跳转（用于 switch/case）
    /// 操作数：reg (u8，比较值寄存器), table_idx (u16，跳转表索引)
    Switch = 0x06,

    /// 循环开始（迭代器消除优化）
    /// 操作数：start_reg, end_reg, step_reg, exit_offset
    LoopStart = 0x07,

    /// 循环递增（迭代器消除优化）
    /// 操作数：current_reg, step_reg, loop_start_offset
    LoopInc = 0x08,

    /// 函数调用返回（尾调用优化）
    TailCall = 0x09,

    /// 暂停执行（异步调度）
    Yield = 0x0A,

    /// 标签定义（跳转目标）
    /// 操作数：label_id (u8，标签ID)
    Label = 0x0B,

    // =====================
    // 栈与寄存器操作 (0x10-0x1F)
    // =====================
    /// 寄存器移动
    /// 操作数：dst (u8，目标寄存器), src (u8，源寄存器)
    Mov = 0x10,

    /// 加载常量到寄存器
    /// 操作数：dst (u8，目标寄存器), const_idx (u16，常量池索引)
    LoadConst = 0x11,

    /// 加载局部变量
    /// 操作数：dst (u8，目标寄存器), local_idx (u8，局部变量索引)
    LoadLocal = 0x12,

    /// 存储到局部变量
    /// 操作数：src (u8，源寄存器), local_idx (u8，局部变量索引)
    StoreLocal = 0x13,

    /// 加载函数参数
    /// 操作数：dst (u8，目标寄存器), arg_idx (u8，参数索引)
    LoadArg = 0x14,

    // =====================
    // I64 整数运算指令 (0x20-0x2F)
    // 主要整数类型，使用最频繁
    // =====================
    /// I64 加法：dst = src1 + src2
    I64Add = 0x20,

    /// I64 减法：dst = src1 - src2
    I64Sub = 0x21,

    /// I64 乘法：dst = src1 * src2
    I64Mul = 0x22,

    /// I64 除法：dst = src1 / src2
    I64Div = 0x23,

    /// I64 取模：dst = src1 % src2
    I64Rem = 0x24,

    /// I64 按位与：dst = src1 & src2
    I64And = 0x25,

    /// I64 按位或：dst = src1 | src2
    I64Or = 0x26,

    /// I64 按位异或：dst = src1 ^ src2
    I64Xor = 0x27,

    /// I64 左移：dst = src1 << src2
    I64Shl = 0x28,

    /// I64 算术右移：dst = src1 >> src2（符号扩展）
    I64Sar = 0x29,

    /// I64 逻辑右移：dst = src1 >>> src2（零扩展）
    I64Shr = 0x2A,

    /// I64 取负：dst = -src
    I64Neg = 0x2B,

    /// I64 加载（从内存）
    /// 操作数：dst, base_reg, offset (i16)
    I64Load = 0x2C,

    /// I64 存储（到内存）
    /// 操作数：src, base_reg, offset (i16)
    I64Store = 0x2D,

    /// I64 加载立即数
    /// 操作数：dst, immediate (i64 立即数)
    I64Const = 0x2E,

    // =====================
    // I32 整数运算指令 (0x30-0x3F)
    // 用于内存优化场景（如字节操作）
    // =====================
    /// I32 加法
    I32Add = 0x30,

    /// I32 减法
    I32Sub = 0x31,

    /// I32 乘法
    I32Mul = 0x32,

    /// I32 除法
    I32Div = 0x33,

    /// I32 取模
    I32Rem = 0x34,

    /// I32 按位与
    I32And = 0x35,

    /// I32 按位或
    I32Or = 0x36,

    /// I32 按位异或
    I32Xor = 0x37,

    /// I32 左移
    I32Shl = 0x38,

    /// I32 算术右移
    I32Sar = 0x39,

    /// I32 逻辑右移
    I32Shr = 0x3A,

    /// I32 取负
    I32Neg = 0x3B,

    /// I32 加载
    I32Load = 0x3C,

    /// I32 存储
    I32Store = 0x3D,

    /// I32 加载立即数
    I32Const = 0x3E,

    // =====================
    // F64 浮点运算指令 (0x40-0x4F)
    // 主要浮点类型
    // =====================
    /// F64 加法
    F64Add = 0x40,

    /// F64 减法
    F64Sub = 0x41,

    /// F64 乘法
    F64Mul = 0x42,

    /// F64 除法
    F64Div = 0x43,

    /// F64 取模
    F64Rem = 0x44,

    /// F64 平方根
    F64Sqrt = 0x45,

    /// F64 取负
    F64Neg = 0x46,

    /// F64 加载
    F64Load = 0x47,

    /// F64 存储
    F64Store = 0x48,

    /// F64 加载立即数
    F64Const = 0x49,

    // =====================
    // F32 浮点运算指令 (0x50-0x5F)
    // 用于图形/科学计算
    // =====================
    /// F32 加法
    F32Add = 0x50,

    /// F32 减法
    F32Sub = 0x51,

    /// F32 乘法
    F32Mul = 0x52,

    /// F32 除法
    F32Div = 0x53,

    /// F32 取模
    F32Rem = 0x54,

    /// F32 平方根
    F32Sqrt = 0x55,

    /// F32 取负
    F32Neg = 0x56,

    /// F32 加载
    F32Load = 0x57,

    /// F32 存储
    F32Store = 0x58,

    /// F32 加载立即数
    F32Const = 0x59,

    // =====================
    // 比较指令 (0x60-0x6F)
    // =====================
    /// I64 相等比较：dst = (src1 == src2) ? 1 : 0
    I64Eq = 0x60,

    /// I64 不等比较
    I64Ne = 0x61,

    /// I64 小于比较
    I64Lt = 0x62,

    /// I64 小于等于比较
    I64Le = 0x63,

    /// I64 大于比较
    I64Gt = 0x64,

    /// I64 大于等于比较
    I64Ge = 0x65,

    /// F64 相等比较
    F64Eq = 0x66,

    /// F64 不等比较
    F64Ne = 0x67,

    /// F64 小于比较
    F64Lt = 0x68,

    /// F64 小于等于比较
    F64Le = 0x69,

    /// F64 大于比较
    F64Gt = 0x6A,

    /// F64 大于等于比较
    F64Ge = 0x6B,

    // =====================
    // 内存与对象操作指令 (0x70-0x7F)
    // =====================
    /// 栈上分配（值类型优化）
    /// 操作数：size (u16，分配大小)
    StackAlloc = 0x70,

    /// 堆分配（引用类型）
    /// 操作数：dst, type_id (u16，类型标识)
    HeapAlloc = 0x71,

    /// 释放所有权（Drop）
    /// 操作数：reg
    Drop = 0x72,

    /// 读取字段（静态偏移，极快）
    /// 操作数：dst, obj_reg, field_offset (u16)
    GetField = 0x73,

    /// 写入字段
    /// 操作数：obj_reg, field_offset (u16), src_reg
    SetField = 0x75,

    /// 加载元素（数组/列表）
    /// 操作数：dst, array_reg, index_reg
    LoadElement = 0x76,

    /// 存储元素
    /// 操作数：array_reg, index_reg, src_reg
    StoreElement = 0x77,

    /// 预分配容量的列表创建
    /// 操作数：dst, capacity (u16)
    NewListWithCap = 0x78,

    /// 创建 Arc（原子引用计数）
    /// 操作数：dst, src
    ArcNew = 0x79,

    /// 克隆 Arc（引用计数 +1）
    /// 操作数：dst, src
    ArcClone = 0x7A,

    /// 释放 Arc（引用计数 -1，归零时释放内存）
    /// 操作数：src
    ArcDrop = 0x7B,

    // =====================
    // 函数调用指令 (0x80-0x8F)
    // =====================
    /// 静态分发调用（最快）
    /// 操作数：dst, func_id (u32，函数ID), base_arg_reg, arg_count
    CallStatic = 0x80,

    /// 虚表分发调用（Trait 调用）
    /// 操作数：dst, obj_reg, vtable_idx (u16), base_arg_reg, arg_count
    CallVirt = 0x81,

    /// 动态分发调用（反射调用，带内联缓存）
    /// 操作数：dst, obj_reg, name_idx (u16，常量池方法名索引), base_arg_reg, arg_count
    CallDyn = 0x82,

    /// 创建闭包
    /// 操作数：dst, func_id (u32), upvalue_count
    MakeClosure = 0x83,

    /// 加载 Upvalue
    /// 操作数：dst, upvalue_idx
    LoadUpvalue = 0x84,

    /// 存储 Upvalue
    /// 操作数：src, upvalue_idx
    StoreUpvalue = 0x85,

    /// 关闭 Upvalue（搬迁栈上变量到堆）
    /// 操作数：reg
    CloseUpvalue = 0x86,

    // =====================
    // 字符串操作指令 (0x90-0x9F)
    // =====================
    /// 获取字符串长度
    /// 操作数：dst, src
    StringLength = 0x90,

    /// 字符串拼接
    /// 操作数：dst, str1, str2
    StringConcat = 0x91,

    /// 字符串相等比较
    /// 操作数：dst, str1, str2
    StringEqual = 0x92,

    /// 字符串获取字符
    /// 操作数：dst, src, index
    StringGetChar = 0x93,

    /// 整数转字符串
    /// 操作数：dst, int_reg
    StringFromInt = 0x94,

    /// 浮点数转字符串
    /// 操作数：dst, float_reg
    StringFromFloat = 0x95,

    // =====================
    // 异常处理指令 (0xA0-0xAF)
    // =====================
    /// try 块开始
    /// 操作数：catch_offset (u16)
    TryBegin = 0xA0,

    /// try 块结束
    TryEnd = 0xA1,

    /// 抛出异常
    /// 操作数：exception_reg
    Throw = 0xA2,

    /// 重新抛出异常
    Rethrow = 0xA3,

    // =====================
    // 边界检查指令 (调试模式) (0xB0-0xBF)
    // =====================
    /// 数组边界检查
    /// 操作数：array_reg, index_reg, dst（存储检查结果或直接跳转）
    BoundsCheck = 0xB0,

    // =====================
    // 类型操作指令 (0xC0-0xCF)
    // =====================
    /// 类型检查
    /// 操作数：obj_reg, type_id (u16), dst（存储检查结果）
    TypeCheck = 0xC0,

    /// 类型转换
    /// 操作数：dst, src, target_type_id (u16)
    Cast = 0xC1,

    // =====================
    // 反射操作指令 (0xD0-0xDF)
    // =====================
    /// 获取类型元数据（懒加载）
    /// 操作数：dst, type_id (u16)
    /// 如果元数据未加载，触发加载；否则直接返回缓存的指针
    TypeOf = 0xD0,

    // =====================
    // 保留指令 (0xE0-0xFF)
    // =====================
    /// 自定义指令 0
    Custom0 = 0xE0,

    /// 自定义指令 1
    Custom1 = 0xE1,

    /// 自定义指令 2
    Custom2 = 0xE2,

    /// 自定义指令 3
    Custom3 = 0xE3,

    /// 保留给未来使用
    Reserved4 = 0xE4,
    Reserved5 = 0xE5,
    Reserved6 = 0xE6,
    Reserved7 = 0xE7,
    Reserved8 = 0xE8,
    Reserved9 = 0xE9,

    /// 无效指令（用于 padding）
    Invalid = 0xFF,
}

impl TypedOpcode {
    /// 获取指令名称
    pub fn name(&self) -> &'static str {
        match self {
            TypedOpcode::Nop => "Nop",
            TypedOpcode::Return => "Return",
            TypedOpcode::ReturnValue => "ReturnValue",
            TypedOpcode::Jmp => "Jmp",
            TypedOpcode::JmpIf => "JmpIf",
            TypedOpcode::JmpIfNot => "JmpIfNot",
            TypedOpcode::Switch => "Switch",
            TypedOpcode::LoopStart => "LoopStart",
            TypedOpcode::LoopInc => "LoopInc",
            TypedOpcode::TailCall => "TailCall",
            TypedOpcode::Yield => "Yield",
            TypedOpcode::Label => "Label",
            TypedOpcode::LoadConst => "LoadConst",
            TypedOpcode::LoadLocal => "LoadLocal",
            TypedOpcode::StoreLocal => "StoreLocal",
            TypedOpcode::LoadArg => "LoadArg",
            TypedOpcode::I64Add => "I64Add",
            TypedOpcode::I64Sub => "I64Sub",
            TypedOpcode::I64Mul => "I64Mul",
            TypedOpcode::I64Div => "I64Div",
            TypedOpcode::I64Rem => "I64Rem",
            TypedOpcode::I64And => "I64And",
            TypedOpcode::I64Or => "I64Or",
            TypedOpcode::I64Xor => "I64Xor",
            TypedOpcode::I64Shl => "I64Shl",
            TypedOpcode::I64Sar => "I64Sar",
            TypedOpcode::I64Shr => "I64Shr",
            TypedOpcode::I64Neg => "I64Neg",
            TypedOpcode::I64Load => "I64Load",
            TypedOpcode::I64Store => "I64Store",
            TypedOpcode::I64Const => "I64Const",
            TypedOpcode::I32Add => "I32Add",
            TypedOpcode::I32Sub => "I32Sub",
            TypedOpcode::I32Mul => "I32Mul",
            TypedOpcode::I32Div => "I32Div",
            TypedOpcode::I32Rem => "I32Rem",
            TypedOpcode::I32And => "I32And",
            TypedOpcode::I32Or => "I32Or",
            TypedOpcode::I32Xor => "I32Xor",
            TypedOpcode::I32Shl => "I32Shl",
            TypedOpcode::I32Sar => "I32Sar",
            TypedOpcode::I32Shr => "I32Shr",
            TypedOpcode::I32Neg => "I32Neg",
            TypedOpcode::I32Load => "I32Load",
            TypedOpcode::I32Store => "I32Store",
            TypedOpcode::I32Const => "I32Const",
            TypedOpcode::F64Add => "F64Add",
            TypedOpcode::F64Sub => "F64Sub",
            TypedOpcode::F64Mul => "F64Mul",
            TypedOpcode::F64Div => "F64Div",
            TypedOpcode::F64Rem => "F64Rem",
            TypedOpcode::F64Sqrt => "F64Sqrt",
            TypedOpcode::F64Neg => "F64Neg",
            TypedOpcode::F64Load => "F64Load",
            TypedOpcode::F64Store => "F64Store",
            TypedOpcode::F64Const => "F64Const",
            TypedOpcode::F32Add => "F32Add",
            TypedOpcode::F32Sub => "F32Sub",
            TypedOpcode::F32Mul => "F32Mul",
            TypedOpcode::F32Div => "F32Div",
            TypedOpcode::F32Rem => "F32Rem",
            TypedOpcode::F32Sqrt => "F32Sqrt",
            TypedOpcode::F32Neg => "F32Neg",
            TypedOpcode::F32Load => "F32Load",
            TypedOpcode::F32Store => "F32Store",
            TypedOpcode::F32Const => "F32Const",
            TypedOpcode::I64Eq => "I64Eq",
            TypedOpcode::I64Ne => "I64Ne",
            TypedOpcode::I64Lt => "I64Lt",
            TypedOpcode::I64Le => "I64Le",
            TypedOpcode::I64Gt => "I64Gt",
            TypedOpcode::I64Ge => "I64Ge",
            TypedOpcode::F64Eq => "F64Eq",
            TypedOpcode::F64Ne => "F64Ne",
            TypedOpcode::F64Lt => "F64Lt",
            TypedOpcode::F64Le => "F64Le",
            TypedOpcode::F64Gt => "F64Gt",
            TypedOpcode::F64Ge => "F64Ge",
            TypedOpcode::StackAlloc => "StackAlloc",
            TypedOpcode::HeapAlloc => "HeapAlloc",
            TypedOpcode::Drop => "Drop",
            TypedOpcode::GetField => "GetField",
            TypedOpcode::SetField => "SetField",
            TypedOpcode::LoadElement => "LoadElement",
            TypedOpcode::StoreElement => "StoreElement",
            TypedOpcode::NewListWithCap => "NewListWithCap",
            TypedOpcode::ArcNew => "ArcNew",
            TypedOpcode::ArcClone => "ArcClone",
            TypedOpcode::ArcDrop => "ArcDrop",
            TypedOpcode::CallStatic => "CallStatic",
            TypedOpcode::CallVirt => "CallVirt",
            TypedOpcode::CallDyn => "CallDyn",
            TypedOpcode::MakeClosure => "MakeClosure",
            TypedOpcode::LoadUpvalue => "LoadUpvalue",
            TypedOpcode::StoreUpvalue => "StoreUpvalue",
            TypedOpcode::CloseUpvalue => "CloseUpvalue",
            TypedOpcode::StringLength => "StringLength",
            TypedOpcode::StringConcat => "StringConcat",
            TypedOpcode::StringEqual => "StringEqual",
            TypedOpcode::StringGetChar => "StringGetChar",
            TypedOpcode::StringFromInt => "StringFromInt",
            TypedOpcode::StringFromFloat => "StringFromFloat",
            TypedOpcode::TryBegin => "TryBegin",
            TypedOpcode::TryEnd => "TryEnd",
            TypedOpcode::Throw => "Throw",
            TypedOpcode::Rethrow => "Rethrow",
            TypedOpcode::BoundsCheck => "BoundsCheck",
            TypedOpcode::TypeCheck => "TypeCheck",
            TypedOpcode::Cast => "Cast",
            TypedOpcode::TypeOf => "TypeOf",
            TypedOpcode::Custom0 => "Custom0",
            TypedOpcode::Custom1 => "Custom1",
            TypedOpcode::Custom2 => "Custom2",
            TypedOpcode::Custom3 => "Custom3",
            TypedOpcode::Reserved4 => "Reserved4",
            TypedOpcode::Reserved5 => "Reserved5",
            TypedOpcode::Reserved6 => "Reserved6",
            TypedOpcode::Reserved7 => "Reserved7",
            TypedOpcode::Reserved8 => "Reserved8",
            TypedOpcode::Reserved9 => "Reserved9",
            TypedOpcode::Invalid => "Invalid",
            // 处理未列出的变体
            _ => "Unknown",
        }
    }

    /// 检查是否是数值运算指令
    pub fn is_numeric_op(&self) -> bool {
        matches!(
            self,
            TypedOpcode::I64Add
                | TypedOpcode::I64Sub
                | TypedOpcode::I64Mul
                | TypedOpcode::I64Div
                | TypedOpcode::I64Rem
                | TypedOpcode::I32Add
                | TypedOpcode::I32Sub
                | TypedOpcode::I32Mul
                | TypedOpcode::I32Div
                | TypedOpcode::I32Rem
                | TypedOpcode::F64Add
                | TypedOpcode::F64Sub
                | TypedOpcode::F64Mul
                | TypedOpcode::F64Div
                | TypedOpcode::F64Rem
                | TypedOpcode::F32Add
                | TypedOpcode::F32Sub
                | TypedOpcode::F32Mul
                | TypedOpcode::F32Div
                | TypedOpcode::F32Rem
        )
    }

    /// 检查是否是整数运算指令
    pub fn is_integer_op(&self) -> bool {
        matches!(
            self,
            TypedOpcode::I64Add
                | TypedOpcode::I64Sub
                | TypedOpcode::I64Mul
                | TypedOpcode::I64Div
                | TypedOpcode::I64Rem
                | TypedOpcode::I64And
                | TypedOpcode::I64Or
                | TypedOpcode::I64Xor
                | TypedOpcode::I64Shl
                | TypedOpcode::I64Sar
                | TypedOpcode::I64Shr
                | TypedOpcode::I32Add
                | TypedOpcode::I32Sub
                | TypedOpcode::I32Mul
                | TypedOpcode::I32Div
                | TypedOpcode::I32Rem
                | TypedOpcode::I32And
                | TypedOpcode::I32Or
                | TypedOpcode::I32Xor
                | TypedOpcode::I32Shl
                | TypedOpcode::I32Sar
                | TypedOpcode::I32Shr
        )
    }

    /// 检查是否是浮点运算指令
    pub fn is_float_op(&self) -> bool {
        matches!(
            self,
            TypedOpcode::F64Add
                | TypedOpcode::F64Sub
                | TypedOpcode::F64Mul
                | TypedOpcode::F64Div
                | TypedOpcode::F64Rem
                | TypedOpcode::F64Sqrt
                | TypedOpcode::F32Add
                | TypedOpcode::F32Sub
                | TypedOpcode::F32Mul
                | TypedOpcode::F32Div
                | TypedOpcode::F32Rem
                | TypedOpcode::F32Sqrt
        )
    }

    /// 检查是否是加载指令
    pub fn is_load_op(&self) -> bool {
        matches!(
            self,
            TypedOpcode::LoadConst
                | TypedOpcode::LoadLocal
                | TypedOpcode::LoadArg
                | TypedOpcode::I64Load
                | TypedOpcode::I32Load
                | TypedOpcode::F64Load
                | TypedOpcode::F32Load
                | TypedOpcode::LoadElement
                | TypedOpcode::GetField
                | TypedOpcode::LoadUpvalue
        )
    }

    /// 检查是否是存储指令
    pub fn is_store_op(&self) -> bool {
        matches!(
            self,
            TypedOpcode::StoreLocal
                | TypedOpcode::I64Store
                | TypedOpcode::I32Store
                | TypedOpcode::F64Store
                | TypedOpcode::F32Store
                | TypedOpcode::StoreElement
                | TypedOpcode::SetField
                | TypedOpcode::StoreUpvalue
        )
    }

    /// 检查是否是调用指令
    pub fn is_call_op(&self) -> bool {
        matches!(
            self,
            TypedOpcode::CallStatic | TypedOpcode::CallVirt | TypedOpcode::CallDyn
        )
    }

    /// 检查是否是返回指令
    pub fn is_return_op(&self) -> bool {
        matches!(
            self,
            TypedOpcode::Return | TypedOpcode::ReturnValue | TypedOpcode::TailCall
        )
    }

    /// 检查是否是跳转指令
    pub fn is_jump_op(&self) -> bool {
        matches!(
            self,
            TypedOpcode::Jmp
                | TypedOpcode::JmpIf
                | TypedOpcode::JmpIfNot
                | TypedOpcode::Switch
                | TypedOpcode::LoopStart
                | TypedOpcode::LoopInc
        )
    }

    /// 获取指令的操作数数量
    pub fn operand_count(&self) -> u8 {
        match self {
            // 无操作数
             TypedOpcode::Nop | TypedOpcode::Return | TypedOpcode::TryEnd |
            TypedOpcode::Yield | TypedOpcode::Invalid | TypedOpcode::Jmp => 0,
            // 1 个操作数
            TypedOpcode::ReturnValue | TypedOpcode::Drop | TypedOpcode::CloseUpvalue |
            TypedOpcode::Throw | TypedOpcode::Rethrow | TypedOpcode::BoundsCheck | TypedOpcode::TypeCheck |
            TypedOpcode::Label | TypedOpcode::StackAlloc | TypedOpcode::ArcDrop => 1,
            // 2 个操作数
            TypedOpcode::JmpIf | TypedOpcode::JmpIfNot |
            TypedOpcode::Mov | TypedOpcode::LoadConst | TypedOpcode::LoadLocal | TypedOpcode::StoreLocal |
            TypedOpcode::LoadArg | TypedOpcode::I64Const | TypedOpcode::I32Const | TypedOpcode::F64Const |
            TypedOpcode::F32Const | TypedOpcode::I64Neg | TypedOpcode::I32Neg | TypedOpcode::F64Neg | TypedOpcode::F32Neg |
            TypedOpcode::HeapAlloc | TypedOpcode::ArcNew | TypedOpcode::ArcClone |
            TypedOpcode::StringLength | TypedOpcode::StringFromInt | TypedOpcode::StringFromFloat |
            TypedOpcode::TypeOf | TypedOpcode::Cast => 2,
            // 3 个操作数
            TypedOpcode::Switch | TypedOpcode::LoopInc |
            TypedOpcode::I64Add | TypedOpcode::I64Sub | TypedOpcode::I64Mul | TypedOpcode::I64Div | TypedOpcode::I64Rem |
            TypedOpcode::I64And | TypedOpcode::I64Or | TypedOpcode::I64Xor | TypedOpcode::I64Shl | TypedOpcode::I64Sar | TypedOpcode::I64Shr |
            TypedOpcode::I32Add | TypedOpcode::I32Sub | TypedOpcode::I32Mul | TypedOpcode::I32Div | TypedOpcode::I32Rem |
            TypedOpcode::I32And | TypedOpcode::I32Or | TypedOpcode::I32Xor | TypedOpcode::I32Shl | TypedOpcode::I32Sar | TypedOpcode::I32Shr |
            TypedOpcode::F64Add | TypedOpcode::F64Sub | TypedOpcode::F64Mul | TypedOpcode::F64Div | TypedOpcode::F64Rem |
            TypedOpcode::F32Add | TypedOpcode::F32Sub | TypedOpcode::F32Mul | TypedOpcode::F32Div | TypedOpcode::F32Rem |
            TypedOpcode::I64Eq | TypedOpcode::I64Ne | TypedOpcode::I64Lt | TypedOpcode::I64Le | TypedOpcode::I64Gt | TypedOpcode::I64Ge |
            TypedOpcode::F64Eq | TypedOpcode::F64Ne | TypedOpcode::F64Lt | TypedOpcode::F64Le | TypedOpcode::F64Gt | TypedOpcode::F64Ge |
            TypedOpcode::I64Load | TypedOpcode::I64Store | TypedOpcode::I32Load | TypedOpcode::I32Store |
            TypedOpcode::F64Load | TypedOpcode::F64Store | TypedOpcode::F32Load | TypedOpcode::F32Store |
            TypedOpcode::GetField | TypedOpcode::SetField | TypedOpcode::NewListWithCap => 3,
            // 4 个操作数
            TypedOpcode::LoopStart | // 4 个操作数：start_reg, end_reg, step_reg, exit_offset
            TypedOpcode::TailCall | TypedOpcode::MakeClosure |
            TypedOpcode::CallStatic | TypedOpcode::CallVirt | TypedOpcode::CallDyn |
            TypedOpcode::LoadElement | TypedOpcode::StoreElement |
            TypedOpcode::StringConcat | TypedOpcode::StringEqual | TypedOpcode::StringGetChar => 4,
            // 5 个操作数
            TypedOpcode::TryBegin => 5,
            // 处理未列出的变体
            _ => 0,
        }
    }
}

impl fmt::Display for TypedOpcode {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// 从字节值创建 TypedOpcode
///
/// 如果值无效，返回 None
impl TryFrom<u8> for TypedOpcode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        // 手动实现以避免依赖非稳定 API
        match value {
            0x00 => Ok(TypedOpcode::Nop),
            0x01 => Ok(TypedOpcode::Return),
            0x02 => Ok(TypedOpcode::ReturnValue),
            0x03 => Ok(TypedOpcode::Jmp),
            0x04 => Ok(TypedOpcode::JmpIf),
            0x05 => Ok(TypedOpcode::JmpIfNot),
            0x06 => Ok(TypedOpcode::Switch),
            0x07 => Ok(TypedOpcode::LoopStart),
            0x08 => Ok(TypedOpcode::LoopInc),
            0x09 => Ok(TypedOpcode::TailCall),
            0x0A => Ok(TypedOpcode::Yield),
            0x0B => Ok(TypedOpcode::Label),
            0x10 => Ok(TypedOpcode::Mov),
            0x11 => Ok(TypedOpcode::LoadConst),
            0x12 => Ok(TypedOpcode::LoadLocal),
            0x13 => Ok(TypedOpcode::StoreLocal),
            0x14 => Ok(TypedOpcode::LoadArg),
            0x20 => Ok(TypedOpcode::I64Add),
            0x21 => Ok(TypedOpcode::I64Sub),
            0x22 => Ok(TypedOpcode::I64Mul),
            0x23 => Ok(TypedOpcode::I64Div),
            0x24 => Ok(TypedOpcode::I64Rem),
            0x25 => Ok(TypedOpcode::I64And),
            0x26 => Ok(TypedOpcode::I64Or),
            0x27 => Ok(TypedOpcode::I64Xor),
            0x28 => Ok(TypedOpcode::I64Shl),
            0x29 => Ok(TypedOpcode::I64Sar),
            0x2A => Ok(TypedOpcode::I64Shr),
            0x2B => Ok(TypedOpcode::I64Neg),
            0x2C => Ok(TypedOpcode::I64Load),
            0x2D => Ok(TypedOpcode::I64Store),
            0x2E => Ok(TypedOpcode::I64Const),
            0x30 => Ok(TypedOpcode::I32Add),
            0x31 => Ok(TypedOpcode::I32Sub),
            0x32 => Ok(TypedOpcode::I32Mul),
            0x33 => Ok(TypedOpcode::I32Div),
            0x34 => Ok(TypedOpcode::I32Rem),
            0x35 => Ok(TypedOpcode::I32And),
            0x36 => Ok(TypedOpcode::I32Or),
            0x37 => Ok(TypedOpcode::I32Xor),
            0x38 => Ok(TypedOpcode::I32Shl),
            0x39 => Ok(TypedOpcode::I32Sar),
            0x3A => Ok(TypedOpcode::I32Shr),
            0x3B => Ok(TypedOpcode::I32Neg),
            0x3C => Ok(TypedOpcode::I32Load),
            0x3D => Ok(TypedOpcode::I32Store),
            0x3E => Ok(TypedOpcode::I32Const),
            0x40 => Ok(TypedOpcode::F64Add),
            0x41 => Ok(TypedOpcode::F64Sub),
            0x42 => Ok(TypedOpcode::F64Mul),
            0x43 => Ok(TypedOpcode::F64Div),
            0x44 => Ok(TypedOpcode::F64Rem),
            0x45 => Ok(TypedOpcode::F64Sqrt),
            0x46 => Ok(TypedOpcode::F64Neg),
            0x47 => Ok(TypedOpcode::F64Load),
            0x48 => Ok(TypedOpcode::F64Store),
            0x49 => Ok(TypedOpcode::F64Const),
            0x50 => Ok(TypedOpcode::F32Add),
            0x51 => Ok(TypedOpcode::F32Sub),
            0x52 => Ok(TypedOpcode::F32Mul),
            0x53 => Ok(TypedOpcode::F32Div),
            0x54 => Ok(TypedOpcode::F32Rem),
            0x55 => Ok(TypedOpcode::F32Sqrt),
            0x56 => Ok(TypedOpcode::F32Neg),
            0x57 => Ok(TypedOpcode::F32Load),
            0x58 => Ok(TypedOpcode::F32Store),
            0x59 => Ok(TypedOpcode::F32Const),
            0x60 => Ok(TypedOpcode::I64Eq),
            0x61 => Ok(TypedOpcode::I64Ne),
            0x62 => Ok(TypedOpcode::I64Lt),
            0x63 => Ok(TypedOpcode::I64Le),
            0x64 => Ok(TypedOpcode::I64Gt),
            0x65 => Ok(TypedOpcode::I64Ge),
            0x66 => Ok(TypedOpcode::F64Eq),
            0x67 => Ok(TypedOpcode::F64Ne),
            0x68 => Ok(TypedOpcode::F64Lt),
            0x69 => Ok(TypedOpcode::F64Le),
            0x6A => Ok(TypedOpcode::F64Gt),
            0x6B => Ok(TypedOpcode::F64Ge),
            0x70 => Ok(TypedOpcode::StackAlloc),
            0x71 => Ok(TypedOpcode::HeapAlloc),
            0x72 => Ok(TypedOpcode::Drop),
            0x73 => Ok(TypedOpcode::GetField),
            0x75 => Ok(TypedOpcode::LoadElement),
            0x76 => Ok(TypedOpcode::StoreElement),
            0x77 => Ok(TypedOpcode::NewListWithCap),
            0x79 => Ok(TypedOpcode::ArcNew),
            0x7A => Ok(TypedOpcode::ArcClone),
            0x7B => Ok(TypedOpcode::ArcDrop),
            0x80 => Ok(TypedOpcode::CallStatic),
            0x81 => Ok(TypedOpcode::CallVirt),
            0x82 => Ok(TypedOpcode::CallDyn),
            0x83 => Ok(TypedOpcode::MakeClosure),
            0x84 => Ok(TypedOpcode::LoadUpvalue),
            0x85 => Ok(TypedOpcode::StoreUpvalue),
            0x86 => Ok(TypedOpcode::CloseUpvalue),
            0x90 => Ok(TypedOpcode::StringLength),
            0x91 => Ok(TypedOpcode::StringConcat),
            0x92 => Ok(TypedOpcode::StringEqual),
            0x93 => Ok(TypedOpcode::StringGetChar),
            0x94 => Ok(TypedOpcode::StringFromInt),
            0x95 => Ok(TypedOpcode::StringFromFloat),
            0xA0 => Ok(TypedOpcode::TryBegin),
            0xA1 => Ok(TypedOpcode::TryEnd),
            0xA2 => Ok(TypedOpcode::Throw),
            0xA3 => Ok(TypedOpcode::Rethrow),
            0xB0 => Ok(TypedOpcode::BoundsCheck),
            0xC0 => Ok(TypedOpcode::TypeCheck),
            0xC1 => Ok(TypedOpcode::Cast),
            0xD0 => Ok(TypedOpcode::TypeOf),
            0xE0 => Ok(TypedOpcode::Custom0),
            0xE1 => Ok(TypedOpcode::Custom1),
            0xE2 => Ok(TypedOpcode::Custom2),
            0xE3 => Ok(TypedOpcode::Custom3),
            0xFF => Ok(TypedOpcode::Invalid),
            _ => Err(()),
        }
    }
}
