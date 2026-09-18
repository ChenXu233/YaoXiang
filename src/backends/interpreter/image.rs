//! 解释器只读镜像（Image）
//!
//! 把「加载期建好、执行期不变」的状态从 `Interpreter` 中分离出来，
//! 与可变的执行状态（堆、全局槽位、运行时）解耦。
//!
//! ## 为什么需要单独的结构
//!
//! 执行器当前是一个大 struct，调用栈与函数表同处一身——这导致热路径上
//! 执行一条指令必须先把 frame 从 `call_stack` 弹出才能拿到 `&mut self`
//! （见 `executor/debug.rs` 的 `step_one`），带来逐指令的深拷贝与
//! `pop`/`push` 搬运。
//!
//! 分离出只读的 `Image` 后：
//! - 指令可从 `&Image` 读取（借用不落在执行状态上）
//! - 调用栈可作为独立参数贯穿
//! - 跨线程任务可直接共享 `Arc<Image>`（只读，天然 `Send + Sync`），
//!   替代原先靠裸指针 + `unsafe impl Send` 的共享方式
//!
//! ## 字段归属判据
//!
//! 判据是**「执行期是否被改写」**，不是「是否只读一次」：
//! - `functions_by_id`：加载期由 `execute_module` 填充，之后只读
//! - `constants`：同上
//! - `type_table`：同上
//! - `vtable_cache`：加载期从字节码 vtables 段直建，之后只读
//!
//! 相对地，`heap` / `global_slots` / `rt` 等在执行期持续改写，留在可变侧。

use std::collections::HashMap;

use crate::backends::common::value::FunctionValue;
use crate::middle::bytecode::{BytecodeFunction, ConstValue};

/// 解释器的只读镜像：加载期建好，执行期不变。
///
/// 生命周期 = 一次模块装载。`execute_module` 填充后不再改写。
#[derive(Debug, Clone, Default)]
pub struct Image {
    /// 常量池（跨模块共享）
    pub constants: Vec<ConstValue>,
    /// 函数表（按索引分发：CallStatic / MakeClosure / CallDyn 全部走这里）
    pub functions_by_id: Vec<BytecodeFunction>,
    /// 类型表
    pub type_table: Vec<crate::middle::core::ir::Type>,
    /// 类型 vtable 缓存（type_name → 方法表）
    ///
    /// 每类型的 vtable 只构建一次，消除「每次 CreateStruct 都 O(n) 扫函数表 +
    /// 向 functions_by_id 重复追加」的浪费与泄漏。
    pub vtable_cache: HashMap<String, Vec<(String, FunctionValue)>>,
}

impl Image {
    /// 创建空镜像
    pub fn new() -> Self {
        Self::default()
    }

    /// 按函数表索引取函数；越界返回 `None`
    pub fn function(
        &self,
        idx: usize,
    ) -> Option<&BytecodeFunction> {
        self.functions_by_id.get(idx)
    }

    /// 按函数表索引取函数名；越界返回 `None`
    pub fn function_name(
        &self,
        idx: usize,
    ) -> Option<&str> {
        self.function(idx).map(|f| f.name.as_str())
    }

    /// 按索引取常量；越界返回 `None`
    pub fn constant(
        &self,
        idx: usize,
    ) -> Option<&ConstValue> {
        self.constants.get(idx)
    }

    /// 取类型 vtable；未注册的类型返回空表
    pub fn vtable(
        &self,
        type_name: &str,
    ) -> Vec<(String, FunctionValue)> {
        self.vtable_cache
            .get(type_name)
            .cloned()
            .unwrap_or_default()
    }
}
