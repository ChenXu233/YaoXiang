//! 死代码消除（DCE）：post-mono ModuleIR 上的可达性 mark-sweep。
//!
//! # 位置与门控（2026-11-18 裁决 B 的登记项）
//!
//! - 输入是**单态化之后**的 ModuleIR（调用点改写与 q=0 擦除都已完成）。本模块不读
//!   实例化请求表，也不依赖 mono 的内部账。
//! - 本轮（#542-D2）只交付 pass，不接线：driver 不在任务 inScope。接线轮由 driver 的
//!   mono 臂尾部经**单一 helper** 调用本模块的唯一入口 eliminate；mono 因
//!   ConfigDisabled/NoObligations 被跳过时 DCE 同步跳过（数据依赖在 mono 尾部）。
//! - **不加 Stage 变体**：DCE 的输入就是 mono 的输出，做成独立 Stage 反而要重复实现
//!   「mono 跳过 ⇒ DCE 跳过」的门控。若未来需要独立门禁/独立开关，升格路径 = 新增
//!   Stage::DeadCodeElimination（Stage::ALL/scope/slug + SINGLE_FILE/MULTI_FILE 两张
//!   ProgramKind 阶段表 + data_dependencies + driver 测试 + 02/09 文档）；driver 侧
//!   helper 的单调用点使这一步是机械的。
//!
//! # 根集（单一事实源；t6 只引用本表，不复制）
//!
//! 根**体**集合 = {entry_function} ∪ {全部 TypeDecl 及其 t. 前缀方法} ∪
//! {init 指令里的按名/DefId 目标}；再从根体沿「名字 / DefId / MakeClosure」三面做
//! 最小不动点，最后补一次反向 MakeClosure 闭合（不变量 3）。
//!
//! | # | 根类别 | 判据 | 代码锚 | 不纳入的后果 |
//! | --- | --- | --- | --- | --- |
//! | 1 | main / 入口 | entry_function（若在表里）；名字 == main 或 endswith(.main) 的函数 | codegen/mod.rs::find_entry_point；util/diagnostic/mod.rs::script_main_never_called（#476） | 入口被删 ⇒ find_entry_point=None、header entry_point=0，运行期静默只跑 init；script 模式 #476 的「main 未被调用」提示与退出码消失 |
//! | 2 | 模块 init | init 指令里的按名目标与 DefId 目标（init 自身恒为根体） | middle/lower/module_ir.rs（ModuleIR.init 产出）；codegen/translator.rs::translate_init_sequence | Script/rootless 形态 init 就是程序主体；删调用目标 ⇒ codegen 回落 CALL_NATIVE ⇒ 运行期 E6006 |
//! | 3 | native/FFI 导出面 | **显式非根**：原生函数没有 IR 体（FfiBinding 入表后 lower 返回 Ok(None)），NativeFn 常量不是表键 | middle/lower/stmt.rs（extern-ref 臂）；frontend/module/registry.rs::is_native_fn_export；codegen/translator.rs::translate_module（ffi_func_meta） | 把原生名当根 = 找不存在的表项（静默通道）；把「表里没有」当悬空 = 误报。纯 yx 的 std 导出有 IR 体，由第 4/5 类的名字边覆盖 |
//! | 4 | 跨模块 Export / TypeDefPayload 引用 | IR 层的导入面 = 限定名字符串引用；Export/TypeDefPayload 只在 frontend，ModuleIR 无此表 | frontend/module/mod.rs（Export/TypeDefPayload）；middle/lower/module_ir.rs::import_aliases_from / qualify_names / rewrite_call_names；driver/helpers.rs::aggregate_instantiation_requests | 漏名字边 ⇒ 删被导入函数 ⇒ 调用点回落 CALL_NATIVE（E6006）。#539 未闭 ⇒ 不得用「没有实例化请求」当不可达证据（本 pass 不读请求表）。TypeDefPayload 的 IR 投影是 TypeDeclBody；类型表项不删 |
//! | 5 | 函数值 / MakeClosure 目标 | MakeClosure{func} 是函数表键 ⇒ 无条件根；另见反向闭合（不变量 3） | lower/expr/access.rs::emit_module_export_value；lower/expr/closure.rs；lower/curry.rs；mono/mod.rs::emit_curry_layer_clones；codegen/translator.rs::translate_make_closure（缺目标=内部错误） | 目标被删 ⇒ 翻译期硬错「闭包函数未在函数名映射表中注册」；#542 的共享层正是「失去唯一 MakeClosure 引用者」的死 IR，方向必须是先按边判死再删 |
//! | 6 | 名字只以字符串常量出现 | 逐形态判定是否函数表键：Call/TailCall 的 Const(String) func、MakeClosure 的 func 是键；NativeFn / LibraryRef / ExternRef / FfiBinding.func_name / 类型名字符串不是键 | middle/ir/ir.rs（Instruction 变体）；lower/expr/access.rs；lower/stmt.rs；frontend/module/registry.rs | 漏 Call/TailCall 字符串 ⇒ E6006；漏 Const(Int) 索引目标检测 ⇒ 删除后索引整体错位（静默错调） |
//! | 7 | 方法面 / vtable | 每个存活 TypeDecl 的 t. 前缀函数恒为根（与 codegen 建 vtable 的同一判据）；**不得**从 CallVirt.method_name 反推表键 | codegen/mod.rs::generate（vtable 段）；codegen/translator.rs::translate_call_virt；backends/interpreter/executor/ops/call.rs（缺方法=运行期硬错）；lower/expr/call.rs（CallVirt 生成点） | CallVirt 按裸方法名查运行期 vtable；删方法 ⇒ vtable 缺项 ⇒ 运行期硬错。见下方「禁猜名」 |
//! | 8 | DefId 边 | Call/TailCall/MakeClosure 的 def 字段解析到删除前函数表 ⇒ 根 | middle/lower/module_ir.rs::assign_defs / resolve_static_targets；codegen/translator.rs::translate_call（def_idx 优先于名字） | 站点名字陈旧但 def 存活时 codegen 仍调 def 目标；只按名字判活会删真正被调函数（随后回落 CALL_NATIVE） |
//! | 9 | 非根（显式不动） | TypeDecl / init 项 / globals / FFI 表 / const pool / 类型表一律不删 | middle/ir/ir.rs（FunctionBody::TypeDecl / TypeDeclBody / ModuleIR 字段）；codegen/mod.rs（类型表/vtable） | 类型名在运行期按字符串/索引消费；init 是程序体；删它们要另一类分析（类型流/死存储/索引重编），不在本 pass 职责内 |
//!
//! # 删除判据与粒度
//!
//! - 只删 FunctionBody::Code 函数；TypeDecl 恒留；init/globals/ffi_*/const pool/类型表不动。
//! - 删除粒度 = ModuleIR.functions 的一个元素（stable retain，保序、确定性）；同时修剪
//!   function_files 中已删名字的条目（按名键的旁表），未知键保留（保守）。
//! - **no-renumber**：IR 内没有跨函数下标引用（jump 是函数内绝对下标；静态调用用名字/DefId），
//!   所有字节码下标由 codegen 从存活表重建（function_name_to_idx / function_def_to_idx /
//!   entry_point / vtable func_idx / init_function / const pool）。因此本 pass 不改写任何指令。
//!   唯一例外 = Const(Int) 索引调用目标：生产 IR 零生成点（全仓唯一命中是 mono 测试夹具），
//!   遇到即中止删除（见不变量 5）。
//!
//! # 禁猜名（2026-11-18 裁决 A 的硬约束）
//!
//! 方法面根**只**来自 TypeDecl 的 t. 前缀（vtable 构造的同一判据）。本模块不实现
//! 「CallVirt.method_name + 接收者类型 → Type.method」的拼名规则：那是第二份名字映射，
//! 且受名字规则变化影响会静默漏根。CallVirt 的裸方法名不参与可达性判定。
//!
//! # 不变量（谁保证、谁验证）
//!
//! 1. 根集完备性：第 1–8 类全部入根（根集计算只在本模块一处）；验证 = 本模块直驱测试的
//!    N1–N6 负控矩阵 + 接线轮的语料 402/402 零 diff + 独立验证（t4）的根集探针。
//! 2. 引用闭合：存活体/init 里的名字目标，若曾是删除前表项则必须存活；存活体的 DefId
//!    目标同理（删除前可解析的必须仍可解析）。验证 = eliminate 返回前的后置自检
//!    （违反即返回 E8001 内部诊断、不改 ModuleIR）。
//! 3. 反向 MakeClosure 闭合（保守形态）：一个 MakeClosure 目标一旦存活，就把**全部**
//!    引用它的函数标活（即使这些引用者本身不可达；init 的 MakeClosure 恒为存活引用）。
//!    理由：verify 的 env 槽数只能从 MakeClosure 反推（middle/ir/verify.rs 的 env_lens），
//!    删掉唯一引用者会把存活闭包体判成 env_len=0 并报 Dominance 红。代价 = 多留死引用
//!    者（宁可多留、不可少留）；规则是单调且与遍历顺序无关的，保证删除集合可复现。
//! 4. 确定性/幂等：stable retain + 名字集合判定，不依赖 HashMap 迭代序；对输出再跑一次
//!    不删任何东西（直驱测试断言）。
//! 5. 不静默错调：遇到 Const(Int) 索引调用目标、重复函数名、重复 DefId 即中止（返回
//!    E8001 内部诊断），不发射可能悬空/错位的 IR。
//!
//! # 回滚路径
//!
//! 运营开关 DCE_ENABLED 的**唯一读取点**是 eliminate。置 false 时 eliminate 立即返回
//! 空报告且**不触碰 ModuleIR**（函数表、function_files 与指令逐字节不变；直驱测试断言）。
//! 回滚 = 翻这一个 const，无数据迁移、无配置项；回滚后 #542 的 post-mono verify 红判据会
//! 按预期复现——那是红灯信号，不得用豁免表掩盖（裁决 A）。
//!
//! # 非目标
//!
//! 不删类型表项/init/globals/FFI/const pool；不做类型流分析；不做死存储消除；不重编号。
//! 这些是后续独立优化，不在 #542 的删除面内。

use std::collections::{HashMap, HashSet, VecDeque};

use crate::frontend::core::types::identity::DefId;
use crate::middle::ir::ir::{ConstValue, Instruction, ModuleIR, Operand};
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

/// DCE 运营开关（默认开）。
///
/// **唯一读取点 = eliminate**。置 false 时 pass 完全无副作用（逐字节不变），
/// 回滚路径见模块 doc「回滚路径」。
const DCE_ENABLED: bool = true;

/// 一次 mark-sweep 的归因报告。
///
/// deleted 按模块内原序给出（确定性，可直接作为 .42 函数表差异的归因清单）；
/// kept = 删除后 ModuleIR.functions 的长度（含恒留的 TypeDecl）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DceReport {
    /// 被删除的函数名（模块内原序）。
    pub deleted: Vec<String>,
    /// 删除后函数表长度（含恒留的 TypeDecl）。
    pub kept: usize,
}

/// DCE 唯一公开入口：对 post-mono ModuleIR 就地做可达性 mark-sweep。
///
/// 成功返回删除归因；失败（重复名/重复 DefId/Const(Int) 索引目标/后置自检不成立）
/// 返回 E8001 内部诊断，且**不修改** ModuleIR。详见模块 doc 的根集表与不变量。
pub fn eliminate(module: &mut ModuleIR) -> Result<DceReport, Diagnostic> {
    eliminate_with(module, DCE_ENABLED)
}

/// 可测试的核心：显式开关（唯一生产读取点是 eliminate）。
fn eliminate_with(
    module: &mut ModuleIR,
    enabled: bool,
) -> Result<DceReport, Diagnostic> {
    if !enabled {
        // 回滚路径：不触碰 module（含 function_files），逐字节不变。
        return Ok(DceReport {
            deleted: Vec::new(),
            kept: module.functions.len(),
        });
    }
    mark_sweep(module)
}

/// 删除前函数表的只读索引 + 闭包目标事实。
struct PreIndex<'a> {
    by_name: HashMap<&'a str, usize>,
    def_to_idx: HashMap<DefId, usize>,
    /// 全部「MakeClosure 目标命中函数表」的目标名。
    closure_targets: HashSet<&'a str>,
    /// 目标名 → 作过 MakeClosure 引用者的函数名（按出现去重）。
    closure_referencers: HashMap<&'a str, Vec<&'a str>>,
    /// init 里的 MakeClosure 目标名：init 恒存活 ⇒ 这些目标恒有存活引用者。
    init_closure_targets: HashSet<&'a str>,
}

/// 静态名字目标：Call/TailCall 的字符串 func（IR 用字符串名分发）与 MakeClosure 的 func。
///
/// 这是「IR 里以名字引用函数的全部形态」在 pass 内的**唯一**取值点；不要在此之外
/// 再写一份 match（禁令三）。NativeFn 常量不是表键，刻意不在此列（见第 3 类根）。
fn static_name_target(instr: &Instruction) -> Option<&str> {
    match instr {
        Instruction::Call {
            func: Operand::Const(ConstValue::String(name)),
            ..
        }
        | Instruction::TailCall {
            func: Operand::Const(ConstValue::String(name)),
            ..
        } => Some(name),
        Instruction::MakeClosure { func, .. } => Some(func),
        _ => None,
    }
}

/// 静态调用目标的 DefId（Call/TailCall/MakeClosure 的 def 字段）。
fn static_def_target(instr: &Instruction) -> Option<DefId> {
    match instr {
        Instruction::Call { def, .. }
        | Instruction::TailCall { def, .. }
        | Instruction::MakeClosure { def, .. } => *def,
        _ => None,
    }
}

/// 索引形态的调用目标（func = Const(Int)）：DCE 无法修正，遇即中止。
///
/// 生产 IR 零生成点（lower 只用字符串名；mono 只用名字/DefId），全仓唯一命中是
/// mono 测试夹具。保守中止的理由：删除会平移函数序号，索引目标会静默指错函数。
fn index_call_target(instr: &Instruction) -> Option<i128> {
    match instr {
        Instruction::Call {
            func: Operand::Const(ConstValue::Int(idx)),
            ..
        }
        | Instruction::TailCall {
            func: Operand::Const(ConstValue::Int(idx)),
            ..
        } => Some(*idx),
        _ => None,
    }
}

fn internal_error(message: String) -> Diagnostic {
    ErrorCodeDefinition::internal_error(&message).build()
}

/// 建索引并做前置校验（重复名 / 重复 DefId / 索引调用目标）。
fn build_pre_index(module: &ModuleIR) -> Result<PreIndex<'_>, Diagnostic> {
    let mut by_name: HashMap<&str, usize> = HashMap::with_capacity(module.functions.len());
    for (idx, func) in module.functions.iter().enumerate() {
        if by_name.insert(func.name.as_str(), idx).is_some() {
            return Err(internal_error(format!(
                "DCE 拒绝删除：函数表存在重复的函数名 {}（codegen 的 name→idx 会静默丢一个，删除面不可判定）",
                func.name
            )));
        }
    }

    let mut def_to_idx: HashMap<DefId, usize> = HashMap::new();
    for (idx, func) in module.functions.iter().enumerate() {
        if let Some(def) = func.def {
            if def_to_idx.insert(def, idx).is_some() {
                return Err(internal_error(format!(
                    "DCE 拒绝删除：DefId({}) 在函数表里出现多次（函数身份不唯一，可达性不可判定）",
                    def.0
                )));
            }
        }
    }

    let mut closure_targets: HashSet<&str> = HashSet::new();
    let mut closure_referencers: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut init_closure_targets: HashSet<&str> = HashSet::new();

    for func in &module.functions {
        for instr in func.all_instructions() {
            if let Some(idx) = index_call_target(instr) {
                return Err(internal_error(format!(
                    "DCE 拒绝删除：函数 {} 含索引形态的静态调用目标 Const(Int({}))；删除会平移函数序号、索引无法修正",
                    func.name, idx
                )));
            }
            if let Instruction::MakeClosure { func: target, .. } = instr {
                if by_name.contains_key(target.as_str()) {
                    closure_targets.insert(target.as_str());
                    closure_referencers
                        .entry(target.as_str())
                        .or_default()
                        .push(func.name.as_str());
                }
            }
        }
    }
    for instr in &module.init {
        if let Some(idx) = index_call_target(instr) {
            return Err(internal_error(format!(
                "DCE 拒绝删除：模块 init 含索引形态的静态调用目标 Const(Int({}))；删除会平移函数序号、索引无法修正",
                idx
            )));
        }
        if let Instruction::MakeClosure { func: target, .. } = instr {
            if by_name.contains_key(target.as_str()) {
                closure_targets.insert(target.as_str());
                init_closure_targets.insert(target.as_str());
            }
        }
    }

    Ok(PreIndex {
        by_name,
        def_to_idx,
        closure_targets,
        closure_referencers,
        init_closure_targets,
    })
}

/// 一个名字若在删除前函数表里，则标活并入队。
fn mark_live(
    name: &str,
    index: &PreIndex<'_>,
    live: &mut HashSet<String>,
    queue: &mut VecDeque<String>,
) {
    if index.by_name.contains_key(name) && live.insert(name.to_string()) {
        queue.push_back(name.to_string());
    }
}

/// 扫描一个活体的指令，把名字/DefId/MakeClosure 目标标活。
fn scan_function(
    name: &str,
    module: &ModuleIR,
    index: &PreIndex<'_>,
    live: &mut HashSet<String>,
    queue: &mut VecDeque<String>,
) {
    let Some(&idx) = index.by_name.get(name) else {
        return;
    };
    for instr in module.functions[idx].all_instructions() {
        if let Some(target) = static_name_target(instr) {
            mark_live(target, index, live, queue);
        }
        if let Some(def) = static_def_target(instr) {
            if let Some(&target_idx) = index.def_to_idx.get(&def) {
                mark_live(&module.functions[target_idx].name, index, live, queue);
            }
        }
    }
}

/// 把队列里的活体全部扫描到不动点。
fn drain_live_queue(
    module: &ModuleIR,
    index: &PreIndex<'_>,
    live: &mut HashSet<String>,
    queue: &mut VecDeque<String>,
) {
    while let Some(name) = queue.pop_front() {
        scan_function(&name, module, index, live, queue);
    }
}

/// 入根（根体集合）：入口 / main 名 / TypeDecl 及其 t. 前缀方法 / init 目标。
fn seed_roots(
    module: &ModuleIR,
    index: &PreIndex<'_>,
    live: &mut HashSet<String>,
    queue: &mut VecDeque<String>,
) {
    // 1) 入口函数（若在表里）。
    if let Some(entry) = &module.entry_function {
        mark_live(entry, index, live, queue);
    }
    // 2) main：bin 入口与 script 的 #476 提示契约都按名找函数表，必须恒留。
    for func in &module.functions {
        if func.name == "main" || func.name.ends_with(".main") {
            mark_live(&func.name, index, live, queue);
        }
    }
    // 3) TypeDecl 及其 t. 前缀方法：与 codegen 建 vtable 的同一判据（禁猜名，不碰 CallVirt）。
    for func in &module.functions {
        if !func.is_type_decl() {
            continue;
        }
        mark_live(&func.name, index, live, queue);
        let prefix = format!("{}.", func.name);
        for candidate in &module.functions {
            if candidate.name.starts_with(&prefix) {
                mark_live(&candidate.name, index, live, queue);
            }
        }
    }
    // 4) init 里的按名/DefId 目标（init 自身是根体，它的指令即程序）。
    for instr in &module.init {
        if let Some(target) = static_name_target(instr) {
            mark_live(target, index, live, queue);
        }
        if let Some(def) = static_def_target(instr) {
            if let Some(&target_idx) = index.def_to_idx.get(&def) {
                mark_live(&module.functions[target_idx].name, index, live, queue);
            }
        }
    }
}

/// 不变量 3（保守形态）：MakeClosure 目标活 ⇒ 其全部引用者活。
///
/// 无条件规则（不看是否已有活引用者）：单调、只加不减，因此与集合遍历顺序无关，
/// 删除集合可复现。返回是否发生新的标活（调用方继续 drain 到不动点）。
fn close_make_closure_referencers(
    index: &PreIndex<'_>,
    live: &mut HashSet<String>,
    queue: &mut VecDeque<String>,
) -> bool {
    let mut changed = false;
    for target in &index.closure_targets {
        if !live.contains(*target) {
            continue;
        }
        if let Some(referencers) = index.closure_referencers.get(target) {
            for referencer in referencers {
                if !live.contains(*referencer) {
                    mark_live(referencer, index, live, queue);
                    changed = true;
                }
            }
        }
    }
    changed
}

/// 后置自检：任一条件不成立即返回 E8001，调用方不得落盘任何删除。
fn check_postconditions(
    module: &ModuleIR,
    index: &PreIndex<'_>,
    live: &HashSet<String>,
) -> Result<(), Diagnostic> {
    // 1) 入口仍在。
    if let Some(entry) = &module.entry_function {
        if index.by_name.contains_key(entry.as_str()) && !live.contains(entry) {
            return Err(internal_error(format!(
                "DCE 后置自检失败：入口函数 {} 被删",
                entry
            )));
        }
    }
    // 2) 存活体的名字目标闭合（若曾是表项则必须存活）。
    let check_body = |owner: &str, instr: &Instruction| -> Result<(), Diagnostic> {
        if let Some(target) = static_name_target(instr) {
            if index.by_name.contains_key(target) && !live.contains(target) {
                return Err(internal_error(format!(
                    "DCE 后置自检失败：存活体 {} 仍引用被删函数 {}",
                    owner, target
                )));
            }
        }
        if let Some(def) = static_def_target(instr) {
            if let Some(&target_idx) = index.def_to_idx.get(&def) {
                let target = &module.functions[target_idx].name;
                if !live.contains(target) {
                    return Err(internal_error(format!(
                        "DCE 后置自检失败：存活体 {} 的 DefId({}) 目标 {} 被删",
                        owner, def.0, target
                    )));
                }
            }
        }
        Ok(())
    };
    for func in &module.functions {
        if !live.contains(&func.name) {
            continue;
        }
        for instr in func.all_instructions() {
            check_body(&func.name, instr)?;
        }
    }
    for (i, instr) in module.init.iter().enumerate() {
        check_body(&format!("<init:#{i}>"), instr)?;
    }
    // 3) 反向 MakeClosure 闭合。
    for target in &index.closure_targets {
        if !live.contains(*target) {
            continue;
        }
        if index.init_closure_targets.contains(target) {
            continue;
        }
        let has_live_referencer = index
            .closure_referencers
            .get(target)
            .is_some_and(|names| names.iter().any(|name| live.contains(*name)));
        if !has_live_referencer {
            return Err(internal_error(format!(
                "DCE 后置自检失败：存活闭包体 {} 没有任何存活的 MakeClosure 引用者（env 槽数反推会失效）",
                target
            )));
        }
    }
    Ok(())
}

/// mark-sweep 主体。
fn mark_sweep(module: &mut ModuleIR) -> Result<DceReport, Diagnostic> {
    let (live, deleted) = {
        let index = build_pre_index(module)?;
        let mut live: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        seed_roots(module, &index, &mut live, &mut queue);
        drain_live_queue(module, &index, &mut live, &mut queue);
        // 不变量 3：反向闭合并入主不动点（单调规则，顺序无关）。
        loop {
            if !close_make_closure_referencers(&index, &mut live, &mut queue) {
                break;
            }
            drain_live_queue(module, &index, &mut live, &mut queue);
        }
        check_postconditions(module, &index, &live)?;

        let deleted: Vec<String> = module
            .functions
            .iter()
            .filter(|func| !live.contains(&func.name))
            .map(|func| func.name.clone())
            .collect();
        (live, deleted)
    };

    // 删除面：只动 functions 与 function_files（按名键的旁表），其余字段逐字不动。
    let pre_names: HashSet<String> = module
        .functions
        .iter()
        .map(|func| func.name.clone())
        .collect();
    module.functions.retain(|func| live.contains(&func.name));
    module
        .function_files
        .retain(|name, _| live.contains(name) || !pre_names.contains(name));

    Ok(DceReport {
        deleted,
        kept: module.functions.len(),
    })
}

#[cfg(test)]
mod tests;
