//! AOT-3.1 的值证据与能力边界。
//!
//! 只消费经 `verify_ssa` 校验的 `ModuleIR`：`Operand::Value` 的类型取
//! `ValueInfo.ty`，`Local` 保持存储语义；本模块不重建 SSA 或类型，也不把
//! 类型证明交给 LLVM 猜测。显式未知与不支持类型按能力边界返回带值 id 的诊断。

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, bail, Result};

use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{
    BasicBlock, ConstValue, FunctionBody, Instruction, ModuleIR, Operand, ValueId,
};
use crate::middle::ir::verify::verify_ssa;

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
    /// Operand::Value 的类型证据：只取函数值表，显式未知/不支持类型在这里
    /// 转成带 ValueId 与原始类型的诊断；不在 AOT 侧猜类型。
    pub(super) fn value_type(
        &self,
        id: ValueId,
    ) -> Result<ScalarType> {
        let ty = self
            .values
            .get(id.0 as usize)
            .ok_or_else(|| anyhow!("ValueId({}) has no ValueInfo", id.0))?;
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
    if !module.source_files.is_empty() || module.entry_function.is_some() {
        bail!("native AOT currently supports a single script file, not a multi-file project");
    }
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
        let signature = &analysis.signatures[&function.name];
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
            source_name,
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
) -> Result<FunctionAnalysis> {
    let mut result = FunctionAnalysis {
        values: value_types.to_vec(),
        ..FunctionAnalysis::default()
    };
    for (index, ty) in params.iter().enumerate() {
        result.locals.insert(index, *ty);
    }

    let mut terminated = false;
    for (index, instruction) in instructions.iter().enumerate() {
        // 每块独立检查终止性：前一 Ret/Jmp 不吞掉下一块。
        if block_starts.contains(&index) {
            terminated = false;
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
        let unsupported = |detail: String| {
            anyhow!(
                "native AOT unsupported input at {source_name}:{}:{} in {function_name}: {detail}",
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
            Instruction::Phi { .. } => {
                return Err(unsupported(
                    "Phi mapping is deferred to batch 3.5 (AOT-3.5 boundary)".to_string(),
                ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::ir::ir::{BasicBlock, FunctionIR, ValueInfo};
    use crate::util::span::Span;

    fn base_function(
        return_type: MonoType,
        values: Vec<ValueInfo>,
        instructions: Vec<Instruction>,
    ) -> FunctionIR {
        FunctionIR {
            name: "f".to_string(),
            def: None,
            params: vec![],
            return_type,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions,
                    successors: vec![],
                }],
                entry: 0,
                locals: vec![],
                values,
                value_slots: vec![],
            },
        }
    }

    fn module_with(function: FunctionIR) -> ModuleIR {
        let mut module = ModuleIR::default();
        module.functions.push(function);
        module.init.push(Instruction::Call {
            dst: None,
            func: Operand::Const(ConstValue::String("f".to_string())),
            args: vec![],
            def: None,
            span: Span::dummy(),
        });
        module
    }

    fn load_value(
        id: u32,
        src: Operand,
    ) -> Instruction {
        Instruction::Load {
            dst: Operand::Value(ValueId(id)),
            src,
            span: Span::dummy(),
        }
    }

    fn ret_value(id: u32) -> Instruction {
        Instruction::Ret {
            value: Some(Operand::Value(ValueId(id))),
            span: Span::dummy(),
        }
    }

    #[test]
    fn test_native_value_type_known_scalar_passes() {
        // Arrange
        let function = base_function(
            MonoType::Int(64),
            vec![ValueInfo {
                ty: MonoType::Int(64),
                debug_name: None,
                defining_block: 0,
            }],
            vec![
                load_value(0, Operand::Const(ConstValue::Int(1))),
                ret_value(0),
            ],
        );
        let module = module_with(function);

        // Act
        let analysis = analyze_module(&module, "value_known.yx")
            .expect("known scalar value should be accepted");

        // Assert
        assert_eq!(
            analysis.functions["f"].values[0],
            MonoType::Int(64),
            "known value type must be carried from ValueInfo"
        );
    }

    #[test]
    fn test_native_explicit_unknown_value_is_rejected() {
        // Arrange
        let function = base_function(
            MonoType::Int(64),
            vec![ValueInfo {
                ty: MonoType::TypeRef("?".to_string()),
                debug_name: None,
                defining_block: 0,
            }],
            vec![
                load_value(0, Operand::Const(ConstValue::Int(1))),
                ret_value(0),
            ],
        );
        let module = module_with(function);

        // Act
        let error = analyze_module(&module, "value_unknown.yx")
            .err()
            .expect("explicit unknown value type must be rejected");

        // Assert
        let detail = error.to_string();
        assert!(
            detail.contains("ValueId(0)") && detail.contains("TypeRef(\"?\")"),
            "diagnostic must include value id and original type: {detail}"
        );
    }

    #[test]
    fn test_native_unsupported_value_type_is_rejected() {
        // Arrange
        let function = base_function(
            MonoType::Int(64),
            vec![ValueInfo {
                ty: MonoType::Float(64),
                debug_name: None,
                defining_block: 0,
            }],
            vec![
                load_value(0, Operand::Const(ConstValue::Int(1))),
                ret_value(0),
            ],
        );
        let module = module_with(function);

        // Act
        let error = analyze_module(&module, "value_unsupported.yx")
            .err()
            .expect("unsupported value type must be rejected");

        // Assert
        let detail = error.to_string();
        assert!(
            detail.contains("ValueId(0)") && detail.contains("Float(64)"),
            "diagnostic must include value id and unsupported type: {detail}"
        );
    }

    #[test]
    fn test_native_multi_definition_value_is_rejected_by_verify_ssa() {
        // Arrange
        let function = base_function(
            MonoType::Int(64),
            vec![ValueInfo {
                ty: MonoType::Int(64),
                debug_name: None,
                defining_block: 0,
            }],
            vec![
                load_value(0, Operand::Const(ConstValue::Int(1))),
                load_value(0, Operand::Const(ConstValue::Int(2))),
                ret_value(0),
            ],
        );
        let module = module_with(function);

        // Act
        let error = analyze_module(&module, "value_double.yx")
            .err()
            .expect("multi-definition value must fail SSA entry verification");

        // Assert
        assert!(
            error.to_string().contains("verification"),
            "diagnostic must identify SSA verification: {error}"
        );
    }

    #[test]
    fn test_native_phi_is_rejected_with_batch_35_boundary() {
        // Arrange: block 0 jumps to block 1, which contains a valid Phi.
        let function = FunctionIR {
            name: "f".to_string(),
            def: None,
            params: vec![],
            return_type: MonoType::Int(64),
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![
                    BasicBlock {
                        label: 0,
                        instructions: vec![Instruction::Jmp {
                            target: 1,
                            span: Span::dummy(),
                        }],
                        successors: vec![1],
                    },
                    BasicBlock {
                        label: 1,
                        instructions: vec![
                            Instruction::Phi {
                                dst: Operand::Value(ValueId(0)),
                                incoming: vec![(0, Operand::Const(ConstValue::Int(1)))],
                                span: Span::dummy(),
                            },
                            ret_value(0),
                        ],
                        successors: vec![],
                    },
                ],
                entry: 0,
                locals: vec![],
                values: vec![ValueInfo {
                    ty: MonoType::Int(64),
                    debug_name: None,
                    defining_block: 1,
                }],
                value_slots: vec![],
            },
        };
        let module = module_with(function);

        // Act
        let error = analyze_module(&module, "phi.yx")
            .err()
            .expect("Phi must remain outside AOT-3.1");

        // Assert
        assert!(
            error.to_string().contains("3.5"),
            "diagnostic must name the AOT-3.5 boundary: {error}"
        );
    }
}
