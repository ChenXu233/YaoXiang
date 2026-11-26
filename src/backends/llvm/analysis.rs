//! AOT-3.1 的值证据与能力边界（批次 3.5 起含 SSA 汇合映射）。
//!
//! 只消费经 `verify_ssa` 校验的 `ModuleIR`：`Operand::Value` 的类型取
//! `ValueInfo.ty`，`Local` 保持存储语义；本模块不重建 SSA 或类型，也不把
//! 类型证明交给 LLVM 猜测。显式未知与不支持类型按能力边界返回带值 id 的诊断。
//!
//! 汇合映射（批次 3.5）：`Phi` 的 dst 与 incoming 的类型证据同样取
//! `ValueInfo.ty`；显式未知输入按生产端同一契约跳过（`is_explicit_unknown_type`），
//! 由发射层按 Phi 的 dst 类型物化为 `undef`。`Local` 的多前驱写—汇合读由发射层
//! 把 alloca 提升到入口块实现，本模块只保留「读点在某条路径上没有写」这一真正
//! 不支持的形态。

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, bail, Result};

use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{
    is_explicit_unknown_type, BasicBlock, ConstValue, FunctionBody, Instruction, ModuleIR, Operand,
    ValueId,
};
use crate::middle::ir::verify::verify_ssa;
use crate::util::span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScalarType {
    Void,
    Int,
    Bool,
    String,
}

impl ScalarType {
    pub(super) fn from_mono(ty: &MonoType) -> Result<Self> {
        match ty {
            MonoType::Never | MonoType::Void => Ok(Self::Void),
            MonoType::Bool => Ok(Self::Bool),
            MonoType::Int(64) => Ok(Self::Int),
            MonoType::Generic { name, .. } if name == "String" => Ok(Self::String),
            MonoType::Refined { base, .. }
            | MonoType::Literal {
                base_type: base, ..
            } => Self::from_mono(base),
            other => bail!("native AOT supports only Int, Bool, String and Void; got {other:?}"),
        }
    }
}

#[derive(Clone)]
pub(super) struct OwnedSignature {
    pub(super) params: Vec<ScalarType>,
    pub(super) return_type: ScalarType,
}

#[derive(Clone, Default)]
pub(super) struct FunctionAnalysis {
    pub(super) locals: HashMap<usize, ScalarType>,
    pub(super) values: Vec<MonoType>,
}

impl FunctionAnalysis {
    /// 值表里的原始类型（不做支持集判定）：Phi 的显式未知输入判定要用它。
    pub(super) fn value_mono(
        &self,
        id: ValueId,
    ) -> Result<&MonoType> {
        self.values
            .get(id.0 as usize)
            .ok_or_else(|| anyhow!("ValueId({}) has no ValueInfo", id.0))
    }

    /// 生产端的显式未知标记：`ssa::materialize_undef` 的 `Load Void`（全函数
    /// 0 定义与部分前驱未定义两类）与 `resolve_phi_type` 的未知收敛共用。
    pub(super) fn is_explicit_unknown_value(
        &self,
        id: ValueId,
    ) -> Result<bool> {
        Ok(is_explicit_unknown_type(self.value_mono(id)?))
    }

    /// Operand::Value 的类型证据：只取函数值表，显式未知/不支持类型在这里
    /// 转成带 ValueId 与原始类型的诊断；不在 AOT 侧猜类型。
    pub(super) fn value_type(
        &self,
        id: ValueId,
    ) -> Result<ScalarType> {
        let ty = self.value_mono(id)?;
        ScalarType::from_mono(ty).map_err(|e| {
            anyhow!(
                "ValueId({}) has unsupported type {ty:?} for native AOT: {e}",
                id.0
            )
        })
    }
}

pub(super) struct ModuleAnalysis {
    pub(super) globals: HashMap<usize, ScalarType>,
    pub(super) functions: HashMap<String, FunctionAnalysis>,
    pub(super) signatures: HashMap<String, OwnedSignature>,
    pub(super) definitions: HashMap<u32, String>,
    pub(super) reachable: HashSet<String>,
}

pub(super) fn analyze_module(
    module: &ModuleIR,
    source_name: &str,
) -> Result<ModuleAnalysis> {
    verify_ssa(module).map_err(|e| anyhow!("native AOT IR verification failed: {e}"))?;
    if !module.ffi_libs.is_empty() || !module.ffi_bindings.is_empty() {
        bail!("native AOT external FFI is outside the AOT-1 capability boundary");
    }

    let mut definitions = HashMap::new();
    for function in &module.functions {
        if let Some(def) = function.def {
            definitions.insert(def.0, function.name.clone());
        }
    }
    let reachable = reachable_functions(module, &definitions);
    reject_unused_script_main(module, &reachable, source_name)?;

    let mut signatures = HashMap::new();
    for function in module
        .functions
        .iter()
        .filter(|function| reachable.contains(&function.name))
    {
        if function.generic_params.is_some()
            || matches!(function.body, FunctionBody::TypeDecl { .. })
        {
            bail!(
                "native AOT does not support generic or type-declaration function {:?}",
                function.name
            );
        }
        let signature = OwnedSignature {
            params: function
                .params
                .iter()
                .map(ScalarType::from_mono)
                .collect::<Result<Vec<_>>>()?,
            return_type: ScalarType::from_mono(&function.return_type)?,
        };
        if signature.params.contains(&ScalarType::Void) {
            bail!(
                "native AOT does not support Void parameters in {:?}",
                function.name
            );
        }
        if signatures
            .insert(function.name.clone(), signature)
            .is_some()
        {
            bail!(
                "native AOT found duplicate function name {:?}",
                function.name
            );
        }
    }

    let mut analysis = ModuleAnalysis {
        globals: HashMap::new(),
        functions: HashMap::new(),
        signatures,
        definitions,
        reachable,
    };
    let init_instructions = module.init.iter().collect::<Vec<_>>();
    let init = analyze_stream(
        module,
        &analysis.signatures,
        &analysis.definitions,
        &mut analysis.globals,
        "__yx_module_init",
        &[],
        ScalarType::Void,
        &init_instructions,
        &[],
        &[],
        source_name,
        &module.source_files,
        &module.init_file_ids,
    )?;
    analysis
        .functions
        .insert("__yx_module_init".to_string(), init);

    for function in module
        .functions
        .iter()
        .filter(|function| analysis.reachable.contains(&function.name))
    {
        let FunctionBody::Code { blocks, values, .. } = &function.body else {
            unreachable!("type declarations were rejected above")
        };
        let instructions = blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .collect::<Vec<_>>();
        let value_types = values
            .iter()
            .map(|value| value.ty.clone())
            .collect::<Vec<_>>();
        let block_starts = block_starts(blocks);
        let function_source = module
            .function_files
            .get(&function.name)
            .and_then(|file_id| module.source_files.get(*file_id))
            .map(String::as_str)
            .unwrap_or(source_name);
        let signature = &analysis.signatures[&function.name];
        check_local_read_definitions(
            blocks,
            &function.name,
            function_source,
            signature.params.len(),
        )?;
        let body = analyze_stream(
            module,
            &analysis.signatures,
            &analysis.definitions,
            &mut analysis.globals,
            &function.name,
            &signature.params,
            signature.return_type,
            &instructions,
            &value_types,
            &block_starts,
            function_source,
            &module.source_files,
            &[],
        )?;
        analysis.functions.insert(function.name.clone(), body);
    }
    for global in &module.globals {
        if let std::collections::hash_map::Entry::Vacant(entry) =
            analysis.globals.entry(global.index)
        {
            entry.insert(ScalarType::from_mono(&global.ty)?);
        }
    }
    Ok(analysis)
}

/// Script 形态（无 entry_function）定义了 main 却没有任何可达调用时，
/// 原生路径必须构建期拒绝：AOT 不隐式调用 main，静默 exit 0 会掩盖程序未执行。
fn reject_unused_script_main(
    module: &ModuleIR,
    reachable: &HashSet<String>,
    source_name: &str,
) -> Result<()> {
    if module.entry_function.is_some() || reachable.contains("main") {
        return Ok(());
    }
    let Some(main) = module
        .functions
        .iter()
        .find(|function| function.name == "main")
    else {
        return Ok(());
    };
    let span = match &main.body {
        FunctionBody::Code { blocks, .. } => blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .next()
            .map(|instruction| instruction.span()),
        FunctionBody::TypeDecl { .. } => None,
    };
    let file = module
        .function_files
        .get("main")
        .and_then(|file_id| module.source_files.get(*file_id))
        .map(String::as_str)
        .unwrap_or(source_name);
    let (line, column) = span
        .map(|span| (span.start.line, span.start.column))
        .unwrap_or((0, 0));
    bail!(
        "native AOT unsupported input at {file}:{line}:{column} in main: Script defines main but never calls it; AOT does not call main implicitly. Use a Bin project (yaoxiang.toml) or remove the unused main"
    );
}

/// 汇合读的 L4 自有判据（批次 3.5 收窄后的形态边界）。
///
/// 发射层把 `Local` 的 alloca 统一提升到入口块，因此「同一 `Local` 在多个前驱块
/// 写、汇合后读」不再需要每个 alloca 各占一个支配点，本判据不再以批次 3.5 拒绝
/// 这类程序。仍不支持的是**读点在某条可达路径上根本没有写**：VM 的帧槽默认值是
/// Void，带类型的 alloca 无法物化这个默认值，发射 `load` 会读到未初始化内存。
///
/// 判据保持 L4 自有（自建可达块 CFG、前驱表与 must-def 不动点），不提升到 L3、
/// 不重建 SSA——它只判支持集并给出带位置的拒绝。`Phi` 的 incoming 在前驱出口处
/// 读取，故按前驱的 must-out 逐条判。
fn check_local_read_definitions(
    blocks: &[BasicBlock],
    function_name: &str,
    source_name: &str,
    params: usize,
) -> Result<()> {
    if blocks.len() < 2 {
        return Ok(());
    }
    let index_by_label: HashMap<usize, usize> = blocks
        .iter()
        .enumerate()
        .map(|(index, block)| (block.label, index))
        .collect();
    let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); blocks.len()];
    let mut reachable = vec![false; blocks.len()];
    let mut queue = vec![0usize];
    reachable[0] = true;
    while let Some(index) = queue.pop() {
        for successor in &blocks[index].successors {
            let Some(&next) = index_by_label.get(successor) else {
                continue;
            };
            if !reachable[next] {
                reachable[next] = true;
                queue.push(next);
            }
            if !predecessors[next].contains(&index) {
                predecessors[next].push(index);
            }
        }
    }

    let written_at: Vec<HashSet<usize>> = blocks
        .iter()
        .map(|block| {
            block
                .instructions
                .iter()
                .flat_map(|instruction| instruction.def_operands())
                .filter_map(|operand| match operand {
                    Operand::Local(slot) => Some(*slot),
                    _ => None,
                })
                .collect()
        })
        .collect();

    // must-in：进入块时在所有可达路径上都已经写过的 Local。参数槽是入口块的
    // 隐式定义，必须计入块 0，否则参数读会被误判成“某条路径没有写”。
    //
    // 这是「必须定义」分析（前驱出口取交集），按标准方向从全集出发迭代收缩到
    // **最大**不动点。不能从空集出发做并集迭代：循环回边会把入口路径上已写过的
    // 槽位从循环头丢掉（入口写 v、循环体只写 i，循环后的 v 读就会被误判为
    // “某条路径没写”）。全集只取可达块里出现过的槽位，不可达块的写不参与。
    let all_slots: HashSet<usize> = written_at
        .iter()
        .enumerate()
        .filter(|(index, _)| reachable[*index])
        .flat_map(|(_, slots)| slots.iter().copied())
        .chain(0..params)
        .collect();
    let mut must_in: Vec<HashSet<usize>> = vec![HashSet::new(); blocks.len()];
    for index in 1..blocks.len() {
        if reachable[index] {
            must_in[index] = all_slots.clone();
        }
    }
    must_in[0] = (0..params).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for index in 1..blocks.len() {
            if !reachable[index] {
                continue;
            }
            let reachable_preds: Vec<usize> = predecessors[index]
                .iter()
                .copied()
                .filter(|pred| reachable[*pred])
                .collect();
            let mut entry: HashSet<usize> = match reachable_preds.first() {
                Some(first) => must_in[*first]
                    .union(&written_at[*first])
                    .copied()
                    .collect(),
                None => HashSet::new(),
            };
            for pred in &reachable_preds[1..] {
                let out: HashSet<usize> =
                    must_in[*pred].union(&written_at[*pred]).copied().collect();
                entry.retain(|slot| out.contains(slot));
            }
            if entry != must_in[index] {
                must_in[index] = entry;
                changed = true;
            }
        }
    }
    let must_out: Vec<HashSet<usize>> = (0..blocks.len())
        .map(|index| must_in[index].union(&written_at[index]).copied().collect())
        .collect();

    let reject = |slot: usize, span: Span| -> Result<()> {
        Err(anyhow!(
            "native AOT unsupported input at {source_name}:{}:{} in {function_name}: Local({slot}) is read on a path where it is never written; AOT does not materialize the VM frame-slot default (Void)",
            span.start.line,
            span.start.column
        ))
    };

    for (index, block) in blocks.iter().enumerate() {
        if !reachable[index] {
            continue;
        }
        let mut defined = must_in[index].clone();
        for instruction in &block.instructions {
            if let Instruction::Phi { incoming, .. } = instruction {
                for (label, operand) in incoming {
                    let Operand::Local(slot) = operand else {
                        continue;
                    };
                    let Some(&pred) = index_by_label.get(label) else {
                        continue;
                    };
                    if !reachable[pred] || !must_out[pred].contains(slot) {
                        return reject(*slot, instruction.span());
                    }
                }
                continue;
            }
            for operand in instruction.use_operands() {
                let Operand::Local(slot) = operand else {
                    continue;
                };
                if !defined.contains(slot) {
                    return reject(*slot, instruction.span());
                }
            }
            for operand in instruction.def_operands() {
                if let Operand::Local(slot) = operand {
                    defined.insert(*slot);
                }
            }
        }
    }
    Ok(())
}

fn block_starts(blocks: &[BasicBlock]) -> Vec<usize> {
    let mut starts = Vec::with_capacity(blocks.len());
    let mut flat = 0;
    for block in blocks {
        starts.push(flat);
        flat += block.instructions.len();
    }
    starts
}

fn reachable_functions(
    module: &ModuleIR,
    definitions: &HashMap<u32, String>,
) -> HashSet<String> {
    fn direct_target(
        func: &Operand,
        def: Option<u32>,
        definitions: &HashMap<u32, String>,
    ) -> Option<String> {
        let Operand::Const(ConstValue::String(name)) = func else {
            return None;
        };
        Some(
            def.and_then(|id| definitions.get(&id).cloned())
                .unwrap_or_else(|| name.clone()),
        )
    }

    let mut pending = module
        .init
        .iter()
        .filter_map(|instruction| match instruction {
            Instruction::Call { func, def, .. } | Instruction::TailCall { func, def, .. } => {
                direct_target(func, def.map(|id| id.0), definitions)
            }
            Instruction::MakeClosure { func, .. } => Some(func.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if let Some(entry) = &module.entry_function {
        pending.push(entry.clone());
    }
    let mut reachable = HashSet::new();
    while let Some(name) = pending.pop() {
        if !reachable.insert(name.clone()) {
            continue;
        }
        let Some(function) = module
            .functions
            .iter()
            .find(|function| function.name == name)
        else {
            continue;
        };
        for instruction in function.all_instructions() {
            match instruction {
                Instruction::Call { func, def, .. } | Instruction::TailCall { func, def, .. } => {
                    if let Some(target) = direct_target(func, def.map(|id| id.0), definitions) {
                        pending.push(target);
                    }
                }
                Instruction::MakeClosure { func, .. } => pending.push(func.clone()),
                _ => {}
            }
        }
    }
    reachable
}

#[allow(clippy::too_many_arguments)]
fn analyze_stream(
    module: &ModuleIR,
    signatures: &HashMap<String, OwnedSignature>,
    definitions: &HashMap<u32, String>,
    globals: &mut HashMap<usize, ScalarType>,
    function_name: &str,
    params: &[ScalarType],
    return_type: ScalarType,
    instructions: &[&Instruction],
    value_types: &[MonoType],
    block_starts: &[usize],
    source_name: &str,
    source_files: &[String],
    file_ids: &[usize],
) -> Result<FunctionAnalysis> {
    let mut result = FunctionAnalysis {
        values: value_types.to_vec(),
        ..FunctionAnalysis::default()
    };
    for (index, ty) in params.iter().enumerate() {
        result.locals.insert(index, *ty);
    }

    let mut terminated = false;
    let mut seen_non_phi = false;
    for (index, instruction) in instructions.iter().enumerate() {
        // 每块独立检查终止性：前一 Ret/Jmp 不吞掉下一块。
        if block_starts.contains(&index) {
            terminated = false;
            seen_non_phi = false;
        }
        // lower 会在显式返回后追加一个仅含 Ret(None) 的无前驱落空块；
        // 标量函数里它是不可达填充，不作为真实返回参与类型分析。
        if index > 0
            && index + 1 == instructions.len()
            && matches!(instruction, Instruction::Ret { value: None, .. })
            && return_type != ScalarType::Void
        {
            continue;
        }
        let span = instruction.span();
        let instruction_source = file_ids
            .get(index)
            .and_then(|file_id| source_files.get(*file_id))
            .map(String::as_str)
            .unwrap_or(source_name);
        let unsupported = |detail: String| {
            anyhow!(
                "native AOT unsupported input at {instruction_source}:{}:{} in {function_name}: {detail}",
                span.start.line,
                span.start.column
            )
        };
        if terminated {
            if index + 1 == instructions.len()
                && matches!(instruction, Instruction::Ret { value: None, .. })
            {
                continue;
            }
            return Err(unsupported(
                "only one trailing Ret(None) may follow a function's first return".to_string(),
            ));
        }
        let read = |op: &Operand, result: &FunctionAnalysis| {
            operand_type(op, result, globals, params, module)
                .map_err(|e| unsupported(e.to_string()))
        };
        let write = |dst: &Operand,
                     ty: ScalarType,
                     result: &mut FunctionAnalysis,
                     globals: &mut HashMap<usize, ScalarType>|
         -> Result<()> {
            match dst {
                Operand::Local(slot) => {
                    if let Some(old) = result.locals.insert(*slot, ty) {
                        if old != ty {
                            return Err(unsupported(format!(
                                "slot {slot} changes type from {old:?} to {ty:?}"
                            )));
                        }
                    }
                    Ok(())
                }
                Operand::Value(id) => {
                    // 生产端把「部分前驱未定义」物化为入口块的一条 `Load Void`
                    // （`ssa::materialize_undef`，类型记显式未知）。该形态只允许
                    // 由 Void 源定义；其他写入仍要求值表类型与指令结果一致，显式
                    // 未知的读取照旧拒绝。
                    if ty == ScalarType::Void && result.is_explicit_unknown_value(*id)? {
                        return Ok(());
                    }
                    let declared = result.value_type(*id)?;
                    if declared != ty {
                        return Err(unsupported(format!(
                            "ValueId({}) type {declared:?} disagrees with instruction result {ty:?}",
                            id.0
                        )));
                    }
                    Ok(())
                }
                Operand::Global(slot) => {
                    if let Some(old) = globals.insert(*slot, ty) {
                        if old != ty {
                            return Err(unsupported(format!(
                                "global {slot} changes type from {old:?} to {ty:?}"
                            )));
                        }
                    }
                    Ok(())
                }
                other => Err(unsupported(format!("unsupported destination {other:?}"))),
            }
        };

        match instruction {
            Instruction::Move { dst, src, .. }
            | Instruction::Load { dst, src, .. }
            | Instruction::Store { dst, src, .. } => {
                let ty = read(src, &result)?;
                write(dst, ty, &mut result, globals)?;
            }
            Instruction::Add { dst, lhs, rhs, .. }
            | Instruction::Sub { dst, lhs, rhs, .. }
            | Instruction::Mul { dst, lhs, rhs, .. }
            | Instruction::Div { dst, lhs, rhs, .. }
            | Instruction::Mod { dst, lhs, rhs, .. } => {
                require_type(read(lhs, &result)?, ScalarType::Int, "integer lhs")
                    .map_err(|e| unsupported(e.to_string()))?;
                require_type(read(rhs, &result)?, ScalarType::Int, "integer rhs")
                    .map_err(|e| unsupported(e.to_string()))?;
                write(dst, ScalarType::Int, &mut result, globals)?;
            }
            Instruction::Neg { dst, src, .. } => {
                require_type(read(src, &result)?, ScalarType::Int, "integer operand")
                    .map_err(|e| unsupported(e.to_string()))?;
                write(dst, ScalarType::Int, &mut result, globals)?;
            }
            Instruction::Not { dst, src, .. } => {
                require_type(read(src, &result)?, ScalarType::Bool, "boolean operand")
                    .map_err(|e| unsupported(e.to_string()))?;
                write(dst, ScalarType::Bool, &mut result, globals)?;
            }
            Instruction::Eq { dst, lhs, rhs, .. } | Instruction::Ne { dst, lhs, rhs, .. } => {
                let left = read(lhs, &result)?;
                let right = read(rhs, &result)?;
                if left != right || !matches!(left, ScalarType::Int | ScalarType::Bool) {
                    return Err(unsupported(format!(
                        "equality requires two Int or two Bool operands, got {left:?} and {right:?}"
                    )));
                }
                write(dst, ScalarType::Bool, &mut result, globals)?;
            }
            Instruction::Lt { dst, lhs, rhs, .. }
            | Instruction::Le { dst, lhs, rhs, .. }
            | Instruction::Gt { dst, lhs, rhs, .. }
            | Instruction::Ge { dst, lhs, rhs, .. } => {
                require_type(read(lhs, &result)?, ScalarType::Int, "comparison lhs")
                    .map_err(|e| unsupported(e.to_string()))?;
                require_type(read(rhs, &result)?, ScalarType::Int, "comparison rhs")
                    .map_err(|e| unsupported(e.to_string()))?;
                write(dst, ScalarType::Bool, &mut result, globals)?;
            }
            Instruction::Call {
                dst,
                func,
                args,
                def,
                ..
            } => {
                let Operand::Const(ConstValue::String(name)) = func else {
                    return Err(unsupported(format!("indirect call target {func:?}")));
                };
                let arg_types = args
                    .iter()
                    .map(|arg| read(arg, &result))
                    .collect::<Result<Vec<_>>>()?;
                let returned = if name == "std.assert.assert" {
                    if !(arg_types.len() == 1 && arg_types[0] == ScalarType::Bool
                        || arg_types.len() == 2
                            && arg_types[0] == ScalarType::Bool
                            && arg_types[1] == ScalarType::String)
                    {
                        return Err(unsupported(format!(
                            "std.assert.assert expects Bool or (Bool, String), got {arg_types:?}"
                        )));
                    }
                    // lower 对函数体内 std 调用结果没有值表类型证据；该内建
                    // 的返回类型由分析器直接拥有（Void），不是对未知类型的
                    // 静默猜测。其他 std 调用仍走显式未知拒绝。
                    if let Some(Operand::Value(id)) = dst {
                        if matches!(
                            result.values.get(id.0 as usize),
                            Some(MonoType::TypeRef(name)) if name == "?"
                        ) {
                            result.values[id.0 as usize] = MonoType::Void;
                        }
                    }
                    ScalarType::Void
                } else {
                    let resolved = def.and_then(|id| definitions.get(&id.0)).unwrap_or(name);
                    let signature = signatures.get(resolved).ok_or_else(|| {
                        unsupported(format!("unknown direct call target {resolved:?}"))
                    })?;
                    if signature.params != arg_types {
                        return Err(unsupported(format!(
                            "call {resolved:?} expects {:?}, got {arg_types:?}",
                            signature.params
                        )));
                    }
                    signature.return_type
                };
                if let Some(dst) = dst {
                    write(dst, returned, &mut result, globals)?;
                }
            }
            Instruction::Ret { value, .. } => {
                if return_type == ScalarType::Void {
                    if let Some(value) = value {
                        read(value, &result)?;
                    }
                } else {
                    let value = value
                        .as_ref()
                        .ok_or_else(|| unsupported("missing scalar return value".to_string()))?;
                    let actual = read(value, &result)?;
                    if actual != return_type {
                        return Err(unsupported(format!(
                            "return type {actual:?} does not match {return_type:?}"
                        )));
                    }
                }
            }
            Instruction::Drop { src, .. } => {
                read(src, &result)?;
            }
            Instruction::Phi { dst, incoming, .. } => {
                if seen_non_phi {
                    return Err(unsupported(
                        "Phi must be grouped at the start of its basic block".to_string(),
                    ));
                }
                let Operand::Value(id) = dst else {
                    return Err(unsupported(format!(
                        "Phi destination must be an SSA value, got {dst:?}"
                    )));
                };
                if incoming.is_empty() {
                    return Err(unsupported(format!(
                        "Phi destination ValueId({}) has no incoming edge; a block without predecessors has no LLVM phi form",
                        id.0
                    )));
                }
                // 类型证据取 `ValueInfo.ty`：显式未知/不支持类型在这里带 id 拒绝。
                let phi_type = result
                    .value_type(*id)
                    .map_err(|e| unsupported(e.to_string()))?;
                for (label, operand) in incoming {
                    // 生产端的 Phi 输入只有 SSA 值与常量（`materialize_undef` 的
                    // `Load Void` 也产出 Value）。`Local` 输入要在前驱出口插入
                    // load 才能物化，不在批次 3.5 的映射面内；显式拒绝，不发射
                    // 终结指令之后的 load 让 LLVM verifier 兜底。
                    if !matches!(operand, Operand::Value(_) | Operand::Const(_)) {
                        return Err(unsupported(format!(
                            "Phi incoming from predecessor {label} must be an SSA value or constant, got {operand:?}"
                        )));
                    }
                    // 与生产端 `resolve_phi_type` 同一契约：跳过显式未知输入
                    // （发射层按 dst 类型物化 undef）；其余已知输入必须同型。
                    if matches!(operand, Operand::Value(source) if result.is_explicit_unknown_value(*source)?)
                    {
                        continue;
                    }
                    let incoming_type = read(operand, &result)?;
                    if incoming_type != phi_type {
                        return Err(unsupported(format!(
                            "Phi destination ValueId({}) is {phi_type:?} but the incoming value from predecessor {label} is {incoming_type:?}",
                            id.0
                        )));
                    }
                }
            }
            Instruction::Jmp { .. } => {}
            Instruction::JmpIf { cond, .. } | Instruction::JmpIfNot { cond, .. } => {
                require_type(read(cond, &result)?, ScalarType::Bool, "branch condition")
                    .map_err(|e| unsupported(e.to_string()))?;
            }
            other => {
                return Err(unsupported(format!(
                    "instruction outside the straight-line AOT-1 subset: {other:?}"
                )))
            }
        }
        if matches!(
            instruction,
            Instruction::Ret { .. }
                | Instruction::Jmp { .. }
                | Instruction::JmpIf { .. }
                | Instruction::JmpIfNot { .. }
        ) {
            terminated = true;
        }
        if !matches!(instruction, Instruction::Phi { .. }) {
            seen_non_phi = true;
        }
    }
    Ok(result)
}

fn operand_type(
    operand: &Operand,
    function: &FunctionAnalysis,
    globals: &HashMap<usize, ScalarType>,
    params: &[ScalarType],
    module: &ModuleIR,
) -> Result<ScalarType> {
    match operand {
        Operand::Const(ConstValue::Void) => Ok(ScalarType::Void),
        Operand::Const(ConstValue::Bool(_)) => Ok(ScalarType::Bool),
        Operand::Const(ConstValue::Int(_)) => Ok(ScalarType::Int),
        Operand::Const(ConstValue::String(_)) => Ok(ScalarType::String),
        Operand::Local(slot) => function
            .locals
            .get(slot)
            .copied()
            .ok_or_else(|| anyhow!("local slot {slot} has no established type")),
        Operand::Value(id) => function.value_type(*id),
        Operand::Arg(index) => params
            .get(*index)
            .copied()
            .ok_or_else(|| anyhow!("argument {index} is out of bounds")),
        Operand::Global(slot) => globals.get(slot).copied().map_or_else(
            || {
                module
                    .globals
                    .iter()
                    .find(|global| global.index == *slot)
                    .ok_or_else(|| anyhow!("global slot {slot} is not declared"))
                    .and_then(|global| ScalarType::from_mono(&global.ty))
            },
            Ok,
        ),
        other => anyhow::bail!("unsupported operand {other:?}"),
    }
}

fn require_type(
    actual: ScalarType,
    expected: ScalarType,
    role: &str,
) -> Result<()> {
    if actual != expected {
        bail!("{role} must be {expected:?}, got {actual:?}");
    }
    Ok(())
}
