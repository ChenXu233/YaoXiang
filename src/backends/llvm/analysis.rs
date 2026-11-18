//! AOT-1 的类型证据与能力边界。
//!
//! 当前共享 IR 的槽位可重复写入，且临时槽的 `Int(64)` 可能只是占位。
//! 本模块只从函数签名、常量、指令结果和直接调用签名传播标量类型；
//! 不能把类型证明交给 LLVM 猜测，也不能把本分析扩张成第二套通用 SSA。

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, bail, Result};

use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{ConstValue, FunctionBody, Instruction, ModuleIR, Operand};
use crate::middle::ir::verify::verify_loose;

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
    verify_loose(module).map_err(|e| anyhow!("native AOT IR verification failed: {e}"))?;
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
        let FunctionBody::Code { blocks, .. } = &function.body else {
            unreachable!("type declarations were rejected above")
        };
        let instructions = blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .collect::<Vec<_>>();
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
    source_name: &str,
) -> Result<FunctionAnalysis> {
    let mut result = FunctionAnalysis::default();
    for (index, ty) in params.iter().enumerate() {
        result.locals.insert(index, *ty);
    }

    let mut terminated = false;
    for (index, instruction) in instructions.iter().enumerate() {
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
                Operand::Local(slot) | Operand::Temp(slot) => {
                    if let Some(old) = result.locals.insert(*slot, ty) {
                        if old != ty {
                            return Err(unsupported(format!(
                                "slot {slot} changes type from {old:?} to {ty:?}"
                            )));
                        }
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
            other => {
                return Err(unsupported(format!(
                    "instruction outside the straight-line AOT-1 subset: {other:?}"
                )))
            }
        }
        if matches!(instruction, Instruction::Ret { .. }) {
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
        Operand::Local(slot) | Operand::Temp(slot) => function
            .locals
            .get(slot)
            .copied()
            .ok_or_else(|| anyhow!("local slot {slot} has no established type")),
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
