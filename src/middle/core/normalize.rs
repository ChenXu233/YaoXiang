//! WBS 2.2.1 / RFC-039 P2：规范化 IR 快照的规范化器（07-equivalence-oracle 第二层判据）
//!
//! 规范来源：docs/src/dev/architecture/07-equivalence-oracle.md §第二层
//! 「规范化 IR 快照」。快照必须规范化，否则任何寄存器编号变化都导致全量
//! diff 失败。规范化规则与 07 表格逐项对应：
//!
//! | 07 规则 | 本模块实现 |
//! | 剥离 Span | 不打印 span 字段 |
//! | 临时值重命名 | Local/Temp/Arg(n) 按函数内首次出现顺序重写为 %0 %1 ...（保留定义-使用拓扑顺序） |
//! | 全局槽位相对化 | Global(n) 按模块内首次出现顺序重写为 @0 @1 ... |
//! | 前驱排序 | successors 打印前按序排列（现状恒为空，规则预留）；Phi 输入随 P7 批 b 变体引入处理 |
//! | 结构体字段排序 | **不排序**——构造顺序是语义（07 原文） |
//!
//! 07 已知局限（原文照搬）：规范化会抹掉「第 N 个实参用了哪个寄存器」——
//! 本层单独不足以覆盖缺陷类 2（arg_regs 重排），其安全性由第一层支配性
//! 检查（verify.rs）+ 第三层语料行为差分共同保证。
//!
//! 打印格式为确定性纯文本（指令按展平线性序，与 translator 的
//! global_ir_index 语义一致）；指令字段分派是穷尽 match 无通配臂——
//! 新增 Instruction 变体编译失败，强制在此登记打印形态（ir.rs span()
//! 同款强制哲学）。字符串字段（类型名/函数名/方法名）原样保留——
//! 它们是语义；DefId 原样打印（绑定序变化是真实变化，C1 下应 diff）。

use std::collections::HashMap;
use std::fmt::Write as _;

use super::ir::{ConstValue, FunctionBody, Instruction, ModuleIR, Operand};

/// 槽位重命名器：首次出现顺序分配规范名。
struct Renamer {
    map: HashMap<usize, usize>,
    next: usize,
}

impl Renamer {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            next: 0,
        }
    }

    fn rename(
        &mut self,
        raw: usize,
    ) -> usize {
        *self.map.entry(raw).or_insert_with(|| {
            let n = self.next;
            self.next += 1;
            n
        })
    }
}

/// 格式化上下文（分层）：FnCtx 函数级（locals/args 每函数重置——跨函数共享
/// 会让「函数 A 加一个槽引用」引发「函数 B 全部重编号」的漂移爆炸）；
/// ModCtx 模块级（globals/defs 全模块共享，使用序相对化）。
///
/// defs 相对化的理由：DefId 是 intern 序编号，跨进程不稳定（2026-10-07 实测
/// 同文件同进程两次生成分得 141/143）——数值无语义，但「同一目标 vs 不同
/// 目标」的等价类是语义，按首次出现序重编号为 d0 d1 ... 保留之。
struct FnCtx {
    locals: Renamer,
    args: Renamer,
}

struct ModCtx {
    globals: Renamer,
    defs: Renamer,
}

/// 操作数规范化打印。
fn fmt_operand(
    op: &Operand,
    f: &mut FnCtx,
    m: &mut ModCtx,
) -> String {
    match op {
        Operand::Local(n) | Operand::Temp(n) => format!("%{}", f.locals.rename(*n)),
        Operand::Arg(n) => format!("arg%{}", f.args.rename(*n)),
        Operand::Global(n) => format!("@{}", m.globals.rename(*n)),
        Operand::Label(n) => format!("label({n})"),
        Operand::Register(n) => format!("reg({n})"),
        Operand::Const(c) => fmt_const(c),
    }
}

/// 常量打印：值内嵌（字符串转义保证单行确定输出）。
fn fmt_const(c: &ConstValue) -> String {
    match c {
        ConstValue::Void => "void".to_string(),
        ConstValue::Bool(b) => format!("bool({b})"),
        ConstValue::Int(i) => format!("int({i})"),
        ConstValue::Float(f) => format!("float({})", f.to_bits()),
        ConstValue::Char(ch) => format!("char({})", ch.escape_debug()),
        ConstValue::String(s) => format!("string({:?})", s),
        ConstValue::Bytes(b) => format!("bytes({b:?})"),
        ConstValue::LibraryRef { mechanism, lib } => format!("libref({mechanism},{lib})"),
        ConstValue::ExternRef {
            mechanism,
            lib,
            symbol,
        } => format!("externref({mechanism},{lib},{symbol})"),
    }
}

/// 操作数列表打印（args / fields / env / items 等）。
fn fmt_ops(
    ops: &[Operand],
    f: &mut FnCtx,
    m: &mut ModCtx,
) -> String {
    let inner: Vec<String> = ops.iter().map(|o| fmt_operand(o, f, m)).collect();
    format!("[{}]", inner.join(", "))
}

/// 指令规范化打印（穷尽 match，无通配臂）。span 不打印（剥离规则）。
fn fmt_instruction(
    instr: &Instruction,
    f: &mut FnCtx,
    m: &mut ModCtx,
) -> String {
    let o = |op: &Operand, f: &mut FnCtx, m: &mut ModCtx| fmt_operand(op, f, m);
    match instr {
        // ---- {dst, src} 族 ----
        Instruction::Move { dst, src, .. } => format!("Move {} <- {}", o(dst, f, m), o(src, f, m)),
        Instruction::Load { dst, src, .. } => format!("Load {} <- {}", o(dst, f, m), o(src, f, m)),
        Instruction::Store { dst, src, .. } => {
            format!("Store {} <- {}", o(dst, f, m), o(src, f, m))
        }
        Instruction::Neg { dst, src, .. } => format!("Neg {} <- {}", o(dst, f, m), o(src, f, m)),
        Instruction::Not { dst, src, .. } => format!("Not {} <- {}", o(dst, f, m), o(src, f, m)),
        Instruction::Cast {
            dst,
            src,
            target_type,
            ..
        } => format!(
            "Cast {} <- {} as {:?}",
            o(dst, f, m),
            o(src, f, m),
            target_type
        ),
        Instruction::LoadField {
            dst, src, field, ..
        } => format!("LoadField {} <- {}.{field}", o(dst, f, m), o(src, f, m)),
        Instruction::ArcNew { dst, src, .. } => {
            format!("ArcNew {} <- {}", o(dst, f, m), o(src, f, m))
        }
        Instruction::RcNew { dst, src, .. } => {
            format!("RcNew {} <- {}", o(dst, f, m), o(src, f, m))
        }
        Instruction::ArcClone { dst, src, .. } => {
            format!("ArcClone {} <- {}", o(dst, f, m), o(src, f, m))
        }
        Instruction::PtrFromRef { dst, src, .. } => {
            format!("PtrFromRef {} <- {}", o(dst, f, m), o(src, f, m))
        }
        Instruction::PtrDeref { dst, src, .. } => {
            format!("PtrDeref {} <- {}", o(dst, f, m), o(src, f, m))
        }
        Instruction::PtrLoad { dst, src, .. } => {
            format!("PtrLoad {} <- {}", o(dst, f, m), o(src, f, m))
        }
        Instruction::PtrStore { dst, src, .. } => {
            format!("PtrStore {} <- {}", o(dst, f, m), o(src, f, m))
        }
        Instruction::StringLength { dst, src, .. } => {
            format!("StringLength {} <- {}", o(dst, f, m), o(src, f, m))
        }
        Instruction::StringFromInt { dst, src, .. } => {
            format!("StringFromInt {} <- {}", o(dst, f, m), o(src, f, m))
        }
        Instruction::StringFromFloat { dst, src, .. } => {
            format!("StringFromFloat {} <- {}", o(dst, f, m), o(src, f, m))
        }
        Instruction::LoadUpvalue {
            dst, upvalue_idx, ..
        } => format!("LoadUpvalue {} <- upvalue({upvalue_idx})", o(dst, f, m)),
        // ---- {dst, lhs, rhs} 族 ----
        Instruction::Add { dst, lhs, rhs, .. } => {
            format!("Add {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Sub { dst, lhs, rhs, .. } => {
            format!("Sub {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Mul { dst, lhs, rhs, .. } => {
            format!("Mul {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Div { dst, lhs, rhs, .. } => {
            format!("Div {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Mod { dst, lhs, rhs, .. } => {
            format!("Mod {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::And { dst, lhs, rhs, .. } => {
            format!("And {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Or { dst, lhs, rhs, .. } => {
            format!("Or {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Xor { dst, lhs, rhs, .. } => {
            format!("Xor {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Shl { dst, lhs, rhs, .. } => {
            format!("Shl {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Shr { dst, lhs, rhs, .. } => {
            format!("Shr {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Sar { dst, lhs, rhs, .. } => {
            format!("Sar {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Eq { dst, lhs, rhs, .. } => {
            format!("Eq {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Ne { dst, lhs, rhs, .. } => {
            format!("Ne {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Lt { dst, lhs, rhs, .. } => {
            format!("Lt {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Le { dst, lhs, rhs, .. } => {
            format!("Le {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Gt { dst, lhs, rhs, .. } => {
            format!("Gt {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::Ge { dst, lhs, rhs, .. } => {
            format!("Ge {} <- {}, {}", o(dst, f, m), o(lhs, f, m), o(rhs, f, m))
        }
        Instruction::StringConcat { dst, lhs, rhs, .. } => format!(
            "StringConcat {} <- {}, {}",
            o(dst, f, m),
            o(lhs, f, m),
            o(rhs, f, m)
        ),
        // ---- {dst} 族 ----
        Instruction::Pop { dst, .. } => format!("Pop {}", o(dst, f, m)),
        Instruction::AllocFixedArray { dst, count, .. } => {
            format!("AllocFixedArray {} count={count}", o(dst, f, m))
        }
        Instruction::HeapAlloc { dst, type_id, .. } => {
            format!("HeapAlloc {} type_id={type_id}", o(dst, f, m))
        }
        // ---- {src} 族 ----
        Instruction::Push { src, .. } => format!("Push {}", o(src, f, m)),
        Instruction::Free { src, .. } => format!("Free {}", o(src, f, m)),
        Instruction::Drop { src, .. } => format!("Drop {}", o(src, f, m)),
        Instruction::ArcDrop { src, .. } => format!("ArcDrop {}", o(src, f, m)),
        Instruction::CloseUpvalue { src, .. } => format!("CloseUpvalue {}", o(src, f, m)),
        Instruction::StoreUpvalue {
            src, upvalue_idx, ..
        } => format!("StoreUpvalue upvalue({upvalue_idx}) <- {}", o(src, f, m)),
        Instruction::TypeTest { src, ty, .. } => format!("TypeTest {} is {:?}", o(src, f, m), ty),
        // ---- 无操作数 / 纯跳转 ----
        Instruction::Dup { .. } => "Dup".to_string(),
        Instruction::Swap { .. } => "Swap".to_string(),
        Instruction::Yield { .. } => "Yield".to_string(),
        Instruction::UnsafeBlockStart { .. } => "UnsafeBlockStart".to_string(),
        Instruction::UnsafeBlockEnd { .. } => "UnsafeBlockEnd".to_string(),
        Instruction::Jmp { target, .. } => format!("Jmp -> instr({target})"),
        Instruction::JmpIf { cond, target, .. } => {
            format!("JmpIf {} -> instr({target})", o(cond, f, m))
        }
        Instruction::JmpIfNot { cond, target, .. } => {
            format!("JmpIfNot {} -> instr({target})", o(cond, f, m))
        }
        // ---- 索引 / 复合字段族 ----
        Instruction::LoadIndex {
            dst, src, index, ..
        } => format!(
            "LoadIndex {} <- {}[{}]",
            o(dst, f, m),
            o(src, f, m),
            o(index, f, m)
        ),
        Instruction::StringGetChar {
            dst, src, index, ..
        } => format!(
            "StringGetChar {} <- {}[{}]",
            o(dst, f, m),
            o(src, f, m),
            o(index, f, m)
        ),
        Instruction::StoreIndex {
            dst, index, src, ..
        } => format!(
            "StoreIndex {}[{}] <- {}",
            o(dst, f, m),
            o(index, f, m),
            o(src, f, m)
        ),
        Instruction::StoreField {
            dst,
            field,
            src,
            type_name,
            field_name,
            ..
        } => format!(
            "StoreField {}.{field} <- {} (type={type_name:?}, name={field_name:?})",
            o(dst, f, m),
            o(src, f, m)
        ),
        Instruction::Contains {
            dst,
            elem,
            container,
            ..
        } => format!(
            "Contains {} <- {} in {}",
            o(dst, f, m),
            o(elem, f, m),
            o(container, f, m)
        ),
        Instruction::Alloc { dst, size, .. } => {
            format!("Alloc {} size={}", o(dst, f, m), o(size, f, m))
        }
        Instruction::AllocArray {
            dst,
            size,
            elem_size,
            ..
        } => format!(
            "AllocArray {} size={}, elem={}",
            o(dst, f, m),
            o(size, f, m),
            o(elem_size, f, m)
        ),
        Instruction::CreateStruct {
            dst,
            type_name,
            fields,
            ..
        } => format!(
            "CreateStruct {} <- {type_name}{}",
            o(dst, f, m),
            fmt_ops(fields, f, m)
        ),
        Instruction::NewDict {
            dst, keys, values, ..
        } => format!(
            "NewDict {} <- keys={} values={}",
            o(dst, f, m),
            fmt_ops(keys, f, m),
            fmt_ops(values, f, m)
        ),
        Instruction::NewTuple { dst, items, .. } => {
            format!("NewTuple {} <- {}", o(dst, f, m), fmt_ops(items, f, m))
        }
        Instruction::NewRange {
            dst,
            start,
            end,
            step,
            ..
        } => format!(
            "NewRange {} <- {}, {}, {}",
            o(dst, f, m),
            o(start, f, m),
            o(end, f, m),
            o(step, f, m)
        ),
        Instruction::CreateVariant {
            dst,
            group,
            variant,
            payload,
            ..
        } => format!(
            "CreateVariant {} <- {group}::{variant} {}",
            o(dst, f, m),
            o(payload, f, m)
        ),
        Instruction::VariantTag {
            dst, obj, group, ..
        } => format!("VariantTag {} <- {} : {group}", o(dst, f, m), o(obj, f, m)),
        Instruction::VariantPayload {
            dst, obj, group, ..
        } => format!(
            "VariantPayload {} <- {} : {group}",
            o(dst, f, m),
            o(obj, f, m)
        ),
        Instruction::MakeClosure {
            dst,
            func,
            def,
            env,
            ..
        } => format!(
            "MakeClosure {} <- {func} env={} (def={})",
            o(dst, f, m),
            fmt_ops(env, f, m),
            fmt_def(def, m)
        ),
        // ---- 调用族 ----
        Instruction::Call {
            dst,
            func,
            args,
            def,
            ..
        } => format!(
            "Call {} <- {} {} (def={})",
            fmt_opt_dst(dst, f, m),
            o(func, f, m),
            fmt_ops(args, f, m),
            fmt_def(def, m)
        ),
        Instruction::CallVirt {
            dst,
            obj,
            method_name,
            args,
            ..
        } => format!(
            "CallVirt {} <- {}.{method_name} {}",
            fmt_opt_dst(dst, f, m),
            o(obj, f, m),
            fmt_ops(args, f, m)
        ),
        Instruction::CallDyn {
            dst, func, args, ..
        } => format!(
            "CallDyn {} <- {} {}",
            fmt_opt_dst(dst, f, m),
            o(func, f, m),
            fmt_ops(args, f, m)
        ),
        Instruction::TailCall {
            func, args, def, ..
        } => format!(
            "TailCall {} {} (def={})",
            o(func, f, m),
            fmt_ops(args, f, m),
            fmt_def(def, m)
        ),
        Instruction::Ret { value, .. } => match value {
            Some(v) => format!("Ret {}", o(v, f, m)),
            None => "Ret".to_string(),
        },
        // ---- spawn 族（ExecutionPlan 结构化打印）----
        Instruction::Spawn {
            closures,
            plan,
            result,
            ..
        } => format!(
            "Spawn {} <- {} {}",
            o(result, f, m),
            fmt_ops(closures, f, m),
            fmt_plan(plan)
        ),
        Instruction::SpawnFromList {
            closures_list,
            plan,
            result,
            ..
        } => format!(
            "SpawnFromList {} <- {} {}",
            o(result, f, m),
            o(closures_list, f, m),
            fmt_plan(plan)
        ),
    }
}

fn fmt_opt_dst(
    dst: &Option<Operand>,
    f: &mut FnCtx,
    m: &mut ModCtx,
) -> String {
    match dst {
        Some(d) => fmt_operand(d, f, m),
        None => "_".to_string(),
    }
}

fn fmt_def(
    def: &Option<crate::frontend::module::symbol::DefId>,
    m: &mut ModCtx,
) -> String {
    match def {
        Some(d) => format!("d{}", m.defs.rename(d.0 as usize)),
        None => "_".to_string(),
    }
}

/// ExecutionPlan 打印：组序（拓扑序=语义）保留；组内任务索引升序;
/// task_deps/task_resources 按任务序原样（序即语义）。
fn fmt_plan(plan: &super::ir::ExecutionPlan) -> String {
    let groups: Vec<String> = plan
        .groups
        .iter()
        .map(|g| format!("{:?}", g.task_indices))
        .collect();
    format!(
        "plan(groups=[{}], deps={:?}, resources={:?})",
        groups.join(", "),
        plan.task_deps,
        plan.task_resources
    )
}

// =====================
// 模块级规范化
// =====================

/// 规范化打印整个 ModuleIR 为确定性文本。
///
/// 确定性约束：所有 HashMap 迭代必须先排序；指令序 = 展平线性序
/// （translator 的 global_ir_index 语义）；函数序 = Vec 生成序（C1 下不变）。
pub fn normalize_module(ir: &ModuleIR) -> String {
    let mut out = String::new();
    let mut mctx = ModCtx {
        globals: Renamer::new(),
        defs: Renamer::new(),
    };

    writeln!(out, "module").unwrap();

    // 全局槽位表：声明序（T3 拓扑序，语义相关）+ 名字 + 类型；
    // 指令引用按使用序相对化（fmt_operand 的 @k）
    for (i, g) in ir.globals.iter().enumerate() {
        writeln!(out, "  global[{i}] {} : {:?}", g.name, g.ty).unwrap();
    }
    if let Some(entry) = &ir.entry_function {
        writeln!(out, "  entry_function: {entry}").unwrap();
    }
    // source_files / function_files：多文件元数据（#252）；HashMap 必排序
    for (i, f) in ir.source_files.iter().enumerate() {
        writeln!(out, "  source_file[{i}]: {f}").unwrap();
    }
    let mut ff: Vec<(&String, &usize)> = ir.function_files.iter().collect();
    ff.sort_by_key(|(name, _)| (*name).clone());
    for (name, idx) in ff {
        writeln!(out, "  function_file {name} -> {idx}").unwrap();
    }
    for lib in &ir.ffi_libs {
        writeln!(
            out,
            "  ffi_lib[{}] {} {}",
            lib.id, lib.mechanism, lib.lib_name
        )
        .unwrap();
    }
    for b in &ir.ffi_bindings {
        match b {
            super::ir::FfiBinding::TypeBinding {
                type_name,
                lib_id,
                symbol,
            } => {
                writeln!(out, "  ffi_type {type_name} lib={lib_id} sym={symbol}").unwrap();
            }
            super::ir::FfiBinding::FuncBinding {
                func_name,
                lib_id,
                symbol,
            } => {
                writeln!(out, "  ffi_func {func_name} lib={lib_id} sym={symbol}").unwrap();
            }
        }
    }

    // 函数体
    for func in &ir.functions {
        write!(out, "  fn {}(", func.name).unwrap();
        let ps: Vec<String> = func.params.iter().map(|p| format!("{p:?}")).collect();
        write!(out, "{}", ps.join(", ")).unwrap();
        writeln!(out, ") -> {:?}", func.return_type).unwrap();
        if let Some(gp) = &func.generic_params {
            writeln!(out, "    generic_params: {gp:?}").unwrap();
        }
        match &func.body {
            FunctionBody::TypeDecl { definition } => {
                writeln!(out, "    type_decl {definition:?}").unwrap();
            }
            FunctionBody::Code { blocks, locals, .. } => {
                // 具名槽表（源码变量身份；temp 占位槽无信息不打印）
                for (i, s) in locals.iter().enumerate() {
                    if let Some(name) = &s.name {
                        writeln!(
                            out,
                            "    slot[{i}] {name} : {:?} @depth{}",
                            s.ty, s.scope_depth
                        )
                        .unwrap();
                    }
                }
                writeln!(out, "    locals_total={}", locals.len()).unwrap();
                let mut fctx = FnCtx {
                    locals: Renamer::new(),
                    args: Renamer::new(),
                };
                for block in blocks {
                    // successors 恒空（现状）；排序规则预留（07 前驱排序）
                    let mut succ = block.successors.clone();
                    succ.sort_unstable();
                    if !succ.is_empty() {
                        writeln!(out, "    block[{}] succ={succ:?}", block.label).unwrap();
                    }
                    for instr in &block.instructions {
                        writeln!(out, "    {}", fmt_instruction(instr, &mut fctx, &mut mctx))
                            .unwrap();
                    }
                }
            }
        }
    }

    // 模块初始化序列
    if !ir.init.is_empty() {
        writeln!(out, "  init (locals={})", ir.init_locals.len()).unwrap();
        for (i, s) in ir.init_locals.iter().enumerate() {
            if let Some(name) = &s.name {
                writeln!(
                    out,
                    "    islot[{i}] {name} : {:?} @depth{}",
                    s.ty, s.scope_depth
                )
                .unwrap();
            }
        }
        let mut fctx = FnCtx {
            locals: Renamer::new(),
            args: Renamer::new(),
        };
        for instr in &ir.init {
            writeln!(out, "    {}", fmt_instruction(instr, &mut fctx, &mut mctx)).unwrap();
        }
    }
    out
}
