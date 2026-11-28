//! IR 到字节码翻译器
//!
//! 将中间表示（IR）翻译为字节码指令。

use crate::frontend::core::sema::MonoType;
use crate::middle::bytecode::file::BytecodeInstruction;
use crate::middle::bytecode::opcode;
use crate::middle::bytecode::Reg;
use crate::middle::ir::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, LocalSlot, ModuleIR, Operand,
};
use crate::middle::passes::codegen::emitter::Emitter;
use crate::middle::passes::codegen::operand::OperandResolver;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::{DebugSpan, FileId, Span};
use std::collections::HashMap;

/// 模块初始化函数的保留名（T2）。
///
/// 顶层绑定编译为全局槽位 + 初始化指令；这串指令装在一个合成函数里，
/// 由运行时在执行 `main` 前先调用。`__yx_` 前缀是保留段，不会与用户标识符碰撞。
pub const MODULE_INIT_FUNCTION: &str = "__yx_module_init";

/// FFI 函数元数据 — 机制/库/符号
#[derive(Debug, Clone)]
struct FfiFuncMeta {
    mechanism: String,
    lib: String,
    symbol: String,
}

/// 一条待回填的跳转（`translate_function` 与 `translate_init_sequence` 共用）。
///
/// 跳转目标在发射时还不知道（前向跳转），故先记占位、等全部指令落位后按
/// `ir_to_bytecode_map` 回填相对偏移。
struct PendingJump {
    /// 该跳转指令自身的字节码下标
    bytecode_idx: usize,
    /// 目标 IR 指令的展平下标（`ir_to_bytecode_map` 的键）
    target_ir_idx: usize,
    /// `opcode::JMP` / `opcode::JMP_IF` / `opcode::JMP_IF_NOT`
    opcode: u8,
    /// 跳转所在块 label；模块 init 序列没有基本块结构，为 `None`。
    ///
    /// 只有带源块的跳转才可能被重定向到「该前驱的 `Phi` 组起点」（方案 A，
    /// 见 [`Translator::backfill_jumps_impl`] 的 `edge_entry`）。
    source_block: Option<usize>,
}

/// 汇合块入口的 `Phi` 展开计划（方案 A）：一个前驱一组 `Move` 串。
#[derive(Debug, Default)]
struct PhiPlan {
    /// 块 label → 组序（落空前驱领首，其余按前驱 label 升序）
    groups: HashMap<usize, Vec<PhiGroup>>,
    /// 块 label → 该块首条指令的展平 IR 下标（入边跳转的识别键）
    first_ir_index: HashMap<usize, usize>,
    /// 块 label → 块内第一条非 `Phi` 指令的展平 IR 下标（组间 `JMP` 的目标）
    body_ir_index: HashMap<usize, usize>,
}

/// 一个前驱对应的 `Move` 串：`Phi` 的 `incoming` 按该前驱选出 `src`。
#[derive(Debug)]
struct PhiGroup {
    /// 该组对应的前驱块 label（落空前驱或跳转前驱）
    predecessor: usize,
    /// `(dst, src, span)`，按本块 `Phi` 的指令序
    moves: Vec<(Operand, Operand, Span)>,
}

/// 块首连续 `Phi` 的借用视图（[`Translator::plan_phi_edges`] 的中间形态）。
struct BlockPhi<'a> {
    dst: &'a Operand,
    incoming: &'a [(usize, Operand)],
    span: Span,
}

/// `Phi` 形态违规（IR 不变量被破坏）：`E8001` 内部错误的单点构造。
///
/// 这些形态的**判据**归 `verify_ssa`（`PhiConsistency`）；translator 只保证自己
/// **不产出错代码**，故以内部错误响亮失败——不静默 NOP、不静默错值。
fn phi_layout_violation(message: impl std::fmt::Display) -> Diagnostic {
    ErrorCodeDefinition::internal_error(&message.to_string()).build()
}

/// IR 到字节码翻译器
///
/// 职责：
/// - 将 Instruction 翻译为 BytecodeInstruction
/// - 管理翻译过程中的状态
/// - 处理跳转偏移回填
#[derive(Debug)]
pub struct Translator {
    /// 字节码发射器
    emitter: Emitter,
    /// 操作数解析器
    operand_resolver: OperandResolver,
    /// 当前函数
    current_function: Option<FunctionIR>,
    /// 闭包函数的索引偏移量（用于计算闭包函数在模块中的正确索引）
    closure_function_offset: Option<usize>,
    /// 函数名到索引的映射
    function_name_to_idx: Option<HashMap<String, usize>>,
    /// DefId 到函数表索引的映射（Stage 3：按 DefId 分发的主路径）
    function_def_to_idx: HashMap<crate::frontend::core::types::identity::DefId, usize>,
    /// FFI 函数元数据缓存: func_name → mechanism/lib/symbol
    ffi_func_meta: HashMap<String, FfiFuncMeta>,

    /// 是否生成运行时调试信息（IP -> Span）
    generate_debug_info: bool,

    /// Debug info 关联的源文件 id（用于多文件/模块定位）
    source_file_id: FileId,
}

impl Translator {
    /// 创建新的翻译器
    pub fn new() -> Self {
        Translator {
            emitter: Emitter::new(),
            operand_resolver: OperandResolver::new(),
            current_function: None,
            ffi_func_meta: HashMap::new(),
            function_def_to_idx: HashMap::new(),
            closure_function_offset: None,
            function_name_to_idx: None,
            generate_debug_info: false,
            source_file_id: 0,
        }
    }

    pub fn set_generate_debug_info(
        &mut self,
        enable: bool,
    ) {
        self.generate_debug_info = enable;
    }

    pub fn set_source_file_id(
        &mut self,
        file_id: FileId,
    ) {
        self.source_file_id = file_id;
    }

    /// 添加常量（用于测试）
    pub fn test_add_constant(
        &mut self,
        value: ConstValue,
    ) -> usize {
        self.emitter.add_constant(value)
    }

    /// 翻译模块
    pub fn translate_module(
        &mut self,
        module: &ModuleIR,
    ) -> Result<TranslatorOutput, Diagnostic> {
        // 消费 FFI 函数绑定 — 存储 mechanism/lib/symbol 元数据
        for binding in &module.ffi_bindings {
            if let crate::middle::ir::ir::FfiBinding::FuncBinding {
                func_name,
                lib_id,
                symbol,
            } = binding
            {
                if let Some(lib) = module.ffi_libs.get(*lib_id) {
                    self.ffi_func_meta.insert(
                        func_name.clone(),
                        FfiFuncMeta {
                            mechanism: lib.mechanism.clone(),
                            lib: lib.lib_name.clone(),
                            symbol: symbol.clone(),
                        },
                    );
                }
            }
        }

        // 建立函数名到索引的映射
        let mut function_name_to_idx: HashMap<String, usize> = HashMap::new();
        for (idx, func) in module.functions.iter().enumerate() {
            function_name_to_idx.insert(func.name.clone(), idx);
        }
        // 建立 DefId 到索引的映射（Stage 3 主路径；FunctionIR.def 由 lower 的 `assign_defs` 填充）
        self.function_def_to_idx = module
            .functions
            .iter()
            .enumerate()
            .filter_map(|(idx, f)| f.def.map(|d| (d, idx)))
            .collect();

        // 注册闭包函数的索引偏移量（闭包函数从 module.functions.len() 开始）
        // 这样 translate_make_closure 就可以正确计算闭包函数的索引
        let closure_offset = module.functions.len();
        self.closure_function_offset = Some(closure_offset);
        self.function_name_to_idx = Some(function_name_to_idx);

        let mut code_section = crate::middle::bytecode::file::CodeSection {
            functions: Vec::new(),
        };

        for func in &module.functions {
            // 多文件模式（#252）：按函数来源文件设置 debug span 的 file_id。
            // mono 特化/克隆名自 #443 起随建件继承定义方归属（mono
            // `build_output` 补 function_files 条目）；回退 0 只剩单文件
            // 管线（function_files 恒空，唯一文件即入口）兜底。
            self.source_file_id =
                module.function_files.get(&func.name).copied().unwrap_or(0) as FileId;
            match &func.body {
                FunctionBody::TypeDecl { body, .. } => {
                    // 类型定义：从 IR 原生收窄载体机械合成构造函数（6.6.12）
                    let ctor = self.synthesize_constructor(func, body)?;
                    code_section.functions.push(ctor);
                }
                FunctionBody::Code { .. } => {
                    // 普通函数：翻译指令
                    let func_code = self.translate_function(func)?;
                    code_section.functions.push(func_code);
                }
            }
        }

        // T2：模块初始化序列编译为一个合成函数 `__yx_module_init`，
        // 追加到函数表末尾并作为入口前置（运行时先跑它再跑 main）。
        // 顶层绑定的求值对顶层代码可见，写成函数是让它可被 call/ret 包裹的
        // 最小手段；不污染用户函数表（名字带 `__yx_` 前缀保留段）。
        //
        // 必须在 take_constant_pool() 之前翻译：init 里的字面量要进常量池，
        // 否则 LoadConst 会指向其他函数的常量（错位）。
        if !module.init.is_empty() {
            // #368：单文件恒 file_id 0；多文件按 `init_file_ids` 逐条定文件，
            // 两路都挂源码位置。
            let init_func = self.translate_init_sequence(
                &module.init,
                &module.init_locals,
                &module.init_file_ids,
            )?;
            code_section.functions.push(init_func);
        }

        let const_pool = self.emitter.take_constant_pool();

        Ok(TranslatorOutput {
            code_section,
            const_pool,
        })
    }

    /// T2：把模块初始化指令序列编译成一个零参函数。
    ///
    /// 与 `translate_function` 的差别只在尾部：初始化无返回值，故补 `Return`。
    /// **跳转回填与函数体同路** —— `for`/`while` 在顶层（Script 模式）
    /// 会生成回跳，漏回填会让 target 停在占位 0（实测：顶层 for 只跑一次）。
    ///
    /// #368：调试元数据（ip→span、局部名）与函数体同路生成。此前这里
    /// 硬编码空表，导致顶层语句的运行期错误既无源码位置也无变量名。
    ///
    /// `file_ids` 逐条给出该指令的源文件序号（单文件为空 → 恒 file_id 0）。
    fn translate_init_sequence(
        &mut self,
        init: &[Instruction],
        locals: &[LocalSlot],
        file_ids: &[usize],
    ) -> Result<crate::middle::bytecode::file::FunctionCode, Diagnostic> {
        let mut instructions = Vec::new();
        let mut debug_map = HashMap::new();
        let mut pending_jumps: Vec<PendingJump> = Vec::new();

        // 模块 init 没有值表：清空值映射（其临时操作数仍是 Local）。
        self.operand_resolver.clear_value_slots();

        // 只保留具名槽位；临时寄存器不进调试信息（与 translate_function 同款）
        let local_names: HashMap<usize, String> = locals
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.name.as_ref().map(|n| (i, n.clone())))
            .collect();

        for (init_idx, instr) in init.iter().enumerate() {
            let bytecode_idx = instructions.len();
            if self.generate_debug_info {
                // 位置直接来自指令自身：生成期就地捕获，无跨调用配对状态
                let span = instr.span();
                if !span.is_dummy() {
                    let file_id = file_ids.get(init_idx).copied().unwrap_or(0) as FileId;
                    debug_map.insert(bytecode_idx, DebugSpan::new(file_id, span));
                }
            }
            if let Some((target, opcode)) = Self::get_jump_target(instr) {
                pending_jumps.push(PendingJump {
                    bytecode_idx,
                    target_ir_idx: target,
                    opcode,
                    // 模块 init 序列没有基本块结构，不存在 Phi 组展开。
                    source_block: None,
                });
            }
            // E3017 的发射点必须带 span（#324）：逐指令把位置交给解析器。
            self.operand_resolver.set_span(instr.span());
            let bc = self.translate_instruction(instr)?;
            instructions.push(bc);
        }

        // IR 下标 → 字节码下标（此处一条 IR 指令→一条字节码指令，恒等映射；
        // 用 map 而非直等是为了与 translate_function 同形，未来指令展开时不错位）
        let mut ir_to_bytecode_map: HashMap<usize, usize> =
            (0..init.len()).map(|i| (i, i)).collect();
        // 段尾哨兵：IR 长度 → 字节码长度。循环出口这类「跳到段尾之后」的目标
        // 落在 init.len()，不在表里就查不到 → 偏移留在占位 0 → 运行时 E6007。
        // `translate_function` 尾部有同款哨兵（见其 global_ir_index 收尾），
        // 此处此前漏了：顶层 `for` 是最后一个语句时必崩。
        // 必须在补 Return **之前**取长度——那里正是段尾。
        ir_to_bytecode_map.insert(init.len(), instructions.len());

        // 末尾补 Return（初始化无返回值）
        instructions.push(crate::middle::bytecode::file::BytecodeInstruction::new(
            opcode::RETURN,
            vec![],
        ));

        Self::backfill_jumps_impl(
            &mut instructions,
            &ir_to_bytecode_map,
            &pending_jumps,
            &HashMap::new(),
        );

        Ok(crate::middle::bytecode::file::FunctionCode {
            name: MODULE_INIT_FUNCTION.to_string(),
            params: Vec::new(),
            return_type: MonoType::Void,
            instructions,
            // #368：不再是硬编码 0。槽位表长度覆盖具名局部；纯临时寄存器
            // 由 `Frame::set_slot` 按需扩容（顶层语句的临时量是即用即弃的）。
            local_count: locals.len(),
            // 顶层序列没有闭包体（捕获在各自的闭包函数里），上值槽恒为 0；
            // 仍走同一个计数器，避免「两处各说一套」。
            upvalue_count: Self::upvalue_slot_count(init.iter()),
            local_names,
            debug_map,
        })
    }

    /// 翻译单个函数
    fn translate_function(
        &mut self,
        func: &FunctionIR,
    ) -> Result<crate::middle::bytecode::file::FunctionCode, Diagnostic> {
        self.current_function = Some(func.clone());

        let mut instructions = Vec::new();
        let mut debug_map = HashMap::new();
        let mut ir_to_bytecode_map = HashMap::new();
        let mut pending_jumps: Vec<PendingJump> = Vec::new();
        let mut global_ir_index = 0;

        let (blocks_ref, locals_len, local_names) = match &func.body {
            FunctionBody::Code { blocks, locals, .. } => {
                // 只保留具名槽位；临时寄存器不进调试信息
                let names: HashMap<usize, String> = locals
                    .iter()
                    .enumerate()
                    .filter_map(|(i, slot)| slot.name.as_ref().map(|n| (i, n.clone())))
                    .collect();
                (blocks.iter().collect::<Vec<_>>(), locals.len(), names)
            }
            FunctionBody::TypeDecl { .. } => {
                // #322 M3：E_INTERNAL 伪码收敛为注册码 E8001
                return Err(ErrorCodeDefinition::internal_error(
                    "translate_function called on TypeDecl — should be handled by synthesize_constructor",
                )
                .build());
            }
        };

        // 7b.2：本函数的值 → 槽映射（index = ValueId；7b.4 分配器接管后重算）。
        self.operand_resolver.set_value_slots(func.value_slots());

        // 7b.3 方案 A：`Phi` 是 IR-only 伪指令，按前驱展开成块入口的 Move 组；
        // 入边跳转由 `edge_entry` 重定向到各自前驱的组起点（见 `backfill_jumps_impl`）。
        let phi_plan = Self::plan_phi_edges(&blocks_ref)?;
        let mut edge_entry: HashMap<(usize, usize), usize> = HashMap::new();

        for block in &blocks_ref {
            if let Some(groups) = phi_plan.groups.get(&block.label) {
                if let (Some(&first_ir_index), Some(&body_ir_index)) = (
                    phi_plan.first_ir_index.get(&block.label),
                    phi_plan.body_ir_index.get(&block.label),
                ) {
                    let edges = self.emit_phi_prologue(
                        block.label,
                        groups,
                        body_ir_index,
                        &mut instructions,
                        &mut pending_jumps,
                    )?;
                    for (predecessor, entry) in edges {
                        edge_entry.insert((predecessor, first_ir_index), entry);
                    }
                }
            }
            for instr in &block.instructions {
                ir_to_bytecode_map.insert(global_ir_index, instructions.len());

                // 方案 A：`Phi` 不产出字节码——Move 串已在块入口按前驱展开。
                // 这也使「入边跳转指向汇合块首条指令」自然解析到块体起点。
                if let Instruction::Phi { .. } = instr {
                    global_ir_index += 1;
                    continue;
                }

                let current_bytecode_idx = instructions.len();

                if self.generate_debug_info {
                    // 位置直接来自指令自身：生成期就地捕获，无跨调用配对状态
                    let span = instr.span();
                    if !span.is_dummy() {
                        debug_map.insert(
                            current_bytecode_idx,
                            DebugSpan::new(self.source_file_id, span),
                        );
                    }
                }

                // 检查是否是跳转指令，记录待回填信息
                if let Some((target, opcode)) = Self::get_jump_target(instr) {
                    pending_jumps.push(PendingJump {
                        bytecode_idx: current_bytecode_idx,
                        target_ir_idx: target,
                        opcode,
                        source_block: Some(block.label),
                    });
                }

                global_ir_index += 1;

                // E3017 的发射点必须带 span（#324）：逐指令把位置交给解析器。
                self.operand_resolver.set_span(instr.span());
                let bytecode_instr = self.translate_instruction(instr)?;
                instructions.push(bytecode_instr);
            }
        }

        ir_to_bytecode_map.insert(global_ir_index, instructions.len());

        // 回填跳转偏移
        Self::backfill_jumps_impl(
            &mut instructions,
            &ir_to_bytecode_map,
            &pending_jumps,
            &edge_entry,
        );

        Ok(crate::middle::bytecode::file::FunctionCode {
            name: func.name.clone(),
            params: func.params.clone(),
            return_type: func.return_type.clone(),
            instructions,
            local_count: locals_len,
            // RFC-039 D17：上值槽数由函数体的上值读写推导后落盘（v5），
            // 载入侧不再硬编 0。
            upvalue_count: Self::upvalue_slot_count(
                blocks_ref
                    .iter()
                    .flat_map(|block| block.instructions.iter()),
            ),
            local_names,
            debug_map,
        })
    }

    /// 函数体的上值槽数 = 体内经 `LoadUpvalue` / `StoreUpvalue` 触达的最大槽下标 + 1。
    ///
    /// 为什么以 IR 而不是字节码为准：上值下标在 IR 里是 `usize`，编码成字节时
    /// 截成 u8；以 IR 计数才是「函数需要多少上值槽」的语义事实，编码器截断
    /// 另有自己的 `to_reg` 越界检查。生产端只此一处（`translate_function`
    /// 与 `translate_init_sequence` 共用），消费端是 `file::FunctionCode.upvalue_count`
    /// 的落盘/载入与 `BytecodeModule` 解码。
    fn upvalue_slot_count<'a>(instructions: impl Iterator<Item = &'a Instruction>) -> usize {
        instructions
            .filter_map(|instr| match instr {
                Instruction::LoadUpvalue { upvalue_idx, .. }
                | Instruction::StoreUpvalue { upvalue_idx, .. } => Some(*upvalue_idx + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }

    /// 方案 A 的展开计划：汇合块入口的 `Phi` 按**前驱**分组为 Move 串
    /// （04-ssa.md §「`Phi` 的字节码形态」方案 A：IR-only 伪指令，不新增 opcode）。
    ///
    /// 组序：**落空前驱**领首——它是布局上紧邻本块、且块尾不是
    /// `Jmp` / `Ret` 的前驱；落空路径直接流进本块入口，必须命中第一组。
    /// 其余前驱按 label 升序，靠「入边跳转重定向到自己的组起点」选择。
    ///
    /// 形态自保（这些形态的判据归 `verify_ssa`，本函数只保证**不产出错代码**）：
    /// `Phi` 必须是块首连续指令、`dst` 必须是 `Operand::Value`、入口块不得有
    /// `Phi`、同块多个 `Phi` 的 incoming 前驱集必须一致、落空前驱必须在 incoming 里。
    /// 违者返回 E8001 内部错误——不静默跳过、不静默错值。
    fn plan_phi_edges(blocks: &[&BasicBlock]) -> Result<PhiPlan, Diagnostic> {
        let mut plan = PhiPlan::default();
        let mut flat_index = 0usize;
        for block in blocks {
            plan.first_ir_index.insert(block.label, flat_index);
            // 块首连续 Phi 的条数：块体起点 = 首条非 Phi 指令
            let leading_phis = block
                .instructions
                .iter()
                .take_while(|instr| matches!(instr, Instruction::Phi { .. }))
                .count();
            plan.body_ir_index
                .insert(block.label, flat_index + leading_phis);
            flat_index += block.instructions.len();
        }

        for (block_index, block) in blocks.iter().enumerate() {
            let mut phis: Vec<BlockPhi<'_>> = Vec::new();
            let mut seen_non_phi = false;
            for instr in &block.instructions {
                if let Instruction::Phi {
                    dst,
                    incoming,
                    span,
                } = instr
                {
                    if seen_non_phi {
                        return Err(phi_layout_violation(format!(
                            "Instruction::Phi 必须是块 {} 的块首指令（方案 A 只在块入口展开）",
                            block.label
                        )));
                    }
                    if !matches!(dst, Operand::Value(_)) {
                        return Err(phi_layout_violation(format!(
                            "块 {} 的 Phi 的 dst 不是 Operand::Value，无法解析为目标槽",
                            block.label
                        )));
                    }
                    phis.push(BlockPhi {
                        dst,
                        incoming: incoming.as_slice(),
                        span: *span,
                    });
                } else {
                    seen_non_phi = true;
                }
            }
            if phis.is_empty() {
                continue;
            }
            if block_index == 0 {
                return Err(phi_layout_violation(
                    "入口块不得有 Instruction::Phi（无前驱可选出 Move 源）",
                ));
            }

            // 各 Phi 的 incoming 前驱集必须一致，否则无法共享同一组序。
            let mut pred_labels: Vec<usize> =
                phis[0].incoming.iter().map(|(label, _)| *label).collect();
            pred_labels.sort_unstable();
            if pred_labels.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(phi_layout_violation(format!(
                    "块 {} 的 Phi 的 incoming 出现重复前驱 label：{pred_labels:?}",
                    block.label
                )));
            }
            for phi in phis.iter().skip(1) {
                let mut labels: Vec<usize> = phi.incoming.iter().map(|(label, _)| *label).collect();
                labels.sort_unstable();
                if labels != pred_labels {
                    return Err(phi_layout_violation(format!(
                        "块 {} 的同块多个 Phi 的 incoming 前驱集不一致：{pred_labels:?} vs {labels:?}",
                        block.label
                    )));
                }
            }

            // 落空前驱：布局上紧邻本块、且块尾不是无条件跳转 / 返回。
            let fallthrough_predecessor = match blocks[block_index - 1].instructions.last() {
                Some(Instruction::Jmp { .. } | Instruction::Ret { .. }) => None,
                _ => Some(blocks[block_index - 1].label),
            };
            let mut order: Vec<usize> = Vec::with_capacity(pred_labels.len());
            if let Some(predecessor) = fallthrough_predecessor {
                if !pred_labels.contains(&predecessor) {
                    return Err(phi_layout_violation(format!(
                        "块 {} 的落空前驱 {predecessor} 不在 Phi 的 incoming 里（前驱集 {pred_labels:?}）",
                        block.label
                    )));
                }
                order.push(predecessor);
            }
            for label in &pred_labels {
                if !order.contains(label) {
                    order.push(*label);
                }
            }

            let mut groups: Vec<PhiGroup> = Vec::with_capacity(order.len());
            for predecessor in order {
                let mut moves: Vec<(Operand, Operand, Span)> = Vec::with_capacity(phis.len());
                for phi in &phis {
                    let source = phi
                        .incoming
                        .iter()
                        .find(|(label, _)| *label == predecessor)
                        .map(|(_, src)| src);
                    let Some(source) = source else {
                        return Err(phi_layout_violation(format!(
                            "块 {} 的 Phi 缺少前驱 {predecessor} 的 incoming 源",
                            block.label
                        )));
                    };
                    moves.push(((*phi.dst).clone(), source.clone(), phi.span));
                }
                groups.push(PhiGroup { predecessor, moves });
            }
            plan.groups.insert(block.label, groups);
        }

        Ok(plan)
    }

    /// 方案 A：把当前块的 `Phi` 组落到块入口，返回「(前驱 label, 组起点字节码下标)」。
    ///
    /// 组间用既有 `JMP` 跳过（`pending_jumps` 回填到块体起点），故临界边无需新块、
    /// 无需新 opcode；单前驱时组内只有 Move、没有 `JMP`。
    fn emit_phi_prologue(
        &mut self,
        block_label: usize,
        groups: &[PhiGroup],
        body_ir_index: usize,
        instructions: &mut Vec<BytecodeInstruction>,
        pending_jumps: &mut Vec<PendingJump>,
    ) -> Result<Vec<(usize, usize)>, Diagnostic> {
        let mut edges: Vec<(usize, usize)> = Vec::with_capacity(groups.len());
        for (group_index, group) in groups.iter().enumerate() {
            edges.push((group.predecessor, instructions.len()));

            // 并行语义自保：一组内的 Move 同时发生。顺序执行前先证明不会互相
            // 覆盖（后序 Move 读到的槽不能被前序 Move 写过）；真出现拷贝环时
            // 需要临时槽，而临时槽归 7b.4 的分配器——本批响亮失败，不静默错值。
            let mut slots: Vec<(u8, u8)> = Vec::with_capacity(group.moves.len());
            for (dst, src, span) in &group.moves {
                self.operand_resolver.set_span(*span);
                slots.push((
                    self.operand_resolver.to_reg(dst)?,
                    self.operand_resolver.to_reg(src)?,
                ));
            }
            for (index, (dst_slot, _)) in slots.iter().enumerate() {
                if slots[index + 1..]
                    .iter()
                    .any(|(_, src_slot)| src_slot == dst_slot)
                {
                    return Err(phi_layout_violation(format!(
                        "块 {block_label} 的 Phi 组（前驱 {}）存在并行拷贝环：槽 {dst_slot} 先被写、随后被读，需要临时槽（见任务 7b.3 的提交信息）",
                        group.predecessor
                    )));
                }
            }

            for (dst, src, span) in &group.moves {
                self.operand_resolver.set_span(*span);
                let instruction = self.translate_move(dst, src)?;
                instructions.push(instruction);
            }
            if group_index + 1 < groups.len() {
                pending_jumps.push(PendingJump {
                    bytecode_idx: instructions.len(),
                    target_ir_idx: body_ir_index,
                    opcode: opcode::JMP,
                    source_block: Some(block_label),
                });
                instructions.push(BytecodeInstruction::new(opcode::JMP, vec![0, 0, 0, 0]));
            }
        }
        Ok(edges)
    }

    /// 从类型定义机械合成构造函数
    ///
    /// 等价于旧 IR 生成器的 `generate_struct_constructor_ir`，但在 codegen 阶段执行。
    /// 数据源自 6.6.12 起是 IR 原生的 [`TypeDeclBody`]（收窄载体），不再读语法层 AST。
    ///
    /// **本轮不改行为**：仍按历史行为把**全部**字段（含函数类型字段/方法）算进构造
    /// 参数。SPEC `type-system.md:443` 要求方法不进构造参数，该不一致登记在 提交信息
    /// §P6 6.6.12（载体上的 `is_method` 已能表达区分，留给后续轮次接线）。
    fn synthesize_constructor(
        &mut self,
        type_func: &FunctionIR,
        body: &crate::middle::ir::ir::TypeDeclBody,
    ) -> Result<crate::middle::bytecode::file::FunctionCode, Diagnostic> {
        let fields = &body.fields;
        let struct_name = &type_func.name;

        if fields.is_empty() {
            // 无字段类型（如枚举）——返回空构造函数
            return Ok(crate::middle::bytecode::file::FunctionCode {
                name: struct_name.clone(),
                params: Vec::new(),
                return_type: type_func.return_type.clone(),
                instructions: Vec::new(),
                local_count: 0,
                // 构造器没有闭包体，不读上值槽
                upvalue_count: 0,
                local_names: HashMap::new(),
                debug_map: HashMap::new(),
            });
        }

        // 6.9.5-B.1b / H2：构造参数与运行期槽**只含数据字段**。
        //
        // witness 字段是编译期证据、运行期无数据槽，故不占构造参数位
        //（`Box(v0)`，不是 `Box(v0, w0)`）；声明序 → 运行期序的映射只走
        // `runtime_field_index`（唯一映射点，见 `types/mono.rs`）。
        // B.1a 保证生产路径不产出 `Some(Zero)`，故对现存程序是恒等变换。
        let quantities: Vec<_> = fields.iter().map(|field| field.quantity).collect();
        let data_field_count =
            crate::frontend::core::types::mono::runtime_field_count(quantities.iter().copied());

        // 构造函数 IR 指令序列：Load(每个数据字段) + CreateStruct + Ret
        let mut ir_instructions = Vec::new();
        let mut field_operands = Vec::new();
        let mut param_types: Vec<MonoType> = Vec::new();

        for (declared_index, field) in fields.iter().enumerate() {
            let Some(runtime_index) = crate::frontend::core::types::mono::runtime_field_index(
                quantities.iter().copied(),
                declared_index,
            ) else {
                continue;
            };
            ir_instructions.push(Instruction::Load {
                dst: Operand::Local(runtime_index),
                src: Operand::Arg(runtime_index),
                // 合成构造函数：无源码对应语句，位置诚实留空
                span: Span::dummy(),
            });
            field_operands.push(Operand::Local(runtime_index));
            param_types.push(field.ty.clone());
        }

        let result_reg = data_field_count;
        ir_instructions.push(Instruction::CreateStruct {
            dst: Operand::Local(result_reg),
            type_name: struct_name.clone(),
            fields: field_operands,
            span: Span::dummy(),
        });
        ir_instructions.push(Instruction::Ret {
            value: Some(Operand::Local(result_reg)),
            span: Span::dummy(),
        });

        // 创建临时 FunctionIR 用于 translate_instruction
        let mut locals: Vec<LocalSlot> = param_types.iter().cloned().map(LocalSlot::temp).collect();
        // 临时槽类型 = 声明类型的身份（lower 随 TypeDeclBody 携带）。
        // 不按 IR 函数名的点号反解身份：名字是投影的像，反解会引入第二套编码。
        locals.push(LocalSlot::temp(MonoType::TypeRef(body.identity.clone())));

        let temp_func = FunctionIR {
            name: struct_name.clone(),
            // 构造器与类型同一绑定身份：调用 Point(...) 解析到类型的 DefId
            def: type_func.def,
            params: param_types,
            return_type: type_func.return_type.clone(),
            generic_params: None,
            body: FunctionBody::Code {
                // 合成构造函数体没有跳转：`partition_blocks` 给出单块 + 空出边，
                // 与手工构造等价（块构造走唯一入口，见 `ir::ir::partition_blocks`）。
                blocks: crate::middle::ir::ir::partition_blocks(ir_instructions.clone()),
                entry: 0,
                locals: locals.clone(),
                values: Vec::new(),
                value_slots: Vec::new(),
            },
        };

        // 设置临时当前函数，调用 translate_function 完成翻译
        let saved_func = self.current_function.clone();
        let func_code = self.translate_function(&temp_func)?;
        self.current_function = saved_func;

        Ok(func_code)
    }

    /// 从指令中提取跳转目标（如果是跳转指令）
    fn get_jump_target(instr: &Instruction) -> Option<(usize, u8)> {
        match instr {
            Instruction::Jmp { target, .. } => Some((*target, opcode::JMP)),
            Instruction::JmpIf { target, .. } => Some((*target, opcode::JMP_IF)),
            Instruction::JmpIfNot { target, .. } => Some((*target, opcode::JMP_IF_NOT)),
            _ => None,
        }
    }

    /// 回填跳转偏移（实际实现）
    ///
    /// `edge_entry` 是方案 A 的「前驱 → `Phi` 组起点」表（键 = (前驱块 label,
    /// 汇合块首条指令的展平下标)）：入边跳转优先按它重定向到该前驱的组，其余
    /// 跳转按 `ir_to_bytecode_map` 解析。模块 init 序列没有块结构，表为空。
    fn backfill_jumps_impl(
        instructions: &mut [BytecodeInstruction],
        ir_to_bytecode_map: &HashMap<usize, usize>,
        pending_jumps: &[PendingJump],
        edge_entry: &HashMap<(usize, usize), usize>,
    ) {
        for pending in pending_jumps {
            let target_bytecode_idx = pending
                .source_block
                .and_then(|source| edge_entry.get(&(source, pending.target_ir_idx)).copied())
                .or_else(|| ir_to_bytecode_map.get(&pending.target_ir_idx).copied());
            let Some(target_bytecode_idx) = target_bytecode_idx else {
                continue;
            };
            // 计算相对偏移: target - current
            let offset = (target_bytecode_idx as i32) - (pending.bytecode_idx as i32);
            let bytes = offset.to_le_bytes();

            let instr = &mut instructions[pending.bytecode_idx];
            match pending.opcode {
                opcode::JMP => {
                    // Jmp 操作数: [offset: i32]
                    instr.operands[0] = bytes[0];
                    instr.operands[1] = bytes[1];
                    instr.operands[2] = bytes[2];
                    instr.operands[3] = bytes[3];
                }
                opcode::JMP_IF | opcode::JMP_IF_NOT => {
                    // JmpIf/JmpIfNot 操作数: [cond_reg: u8, offset: i32]
                    instr.operands[1] = bytes[0];
                    instr.operands[2] = bytes[1];
                    instr.operands[3] = bytes[2];
                    instr.operands[4] = bytes[3];
                }
                _ => {}
            }
        }
    }

    /// 翻译单条 IR 指令
    fn translate_instruction(
        &mut self,
        instr: &Instruction,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        use Instruction::*;

        match instr {
            Move { dst, src, .. } => self.translate_move(dst, src),
            Load { dst, src, .. } => self.translate_load(dst, src),
            Store { dst, src, .. } => self.translate_store(dst, src),

            Add { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_ADD, dst, lhs, rhs),
            Sub { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_SUB, dst, lhs, rhs),
            Mul { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_MUL, dst, lhs, rhs),
            Div { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_DIV, dst, lhs, rhs),
            Mod { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_REM, dst, lhs, rhs),

            And { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_AND, dst, lhs, rhs),
            Or { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_OR, dst, lhs, rhs),
            Xor { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_XOR, dst, lhs, rhs),
            Shl { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_SHL, dst, lhs, rhs),
            Shr { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_SHR, dst, lhs, rhs),
            Sar { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_SAR, dst, lhs, rhs),
            Neg { dst, src, .. } => self.translate_unary_op(opcode::I64_NEG, dst, src),
            // RFC-039 D33 / issue #562：Not 与 Neg 分码（I64_NOT），否则编码/
            // 解码往返把逻辑非改写为取负
            Not { dst, src, .. } => self.translate_unary_op(opcode::I64_NOT, dst, src),

            Eq { dst, lhs, rhs, .. } => {
                self.translate_compare(opcode::I64_EQ, opcode::I64_NE, dst, lhs, rhs)
            }
            Ne { dst, lhs, rhs, .. } => {
                self.translate_compare(opcode::I64_NE, opcode::I64_EQ, dst, lhs, rhs)
            }
            Lt { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_LT, dst, lhs, rhs),
            Le { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_LE, dst, lhs, rhs),
            Gt { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_GT, dst, lhs, rhs),
            Ge { dst, lhs, rhs, .. } => self.translate_binary_op(opcode::I64_GE, dst, lhs, rhs),

            Jmp { target, .. } => self.translate_jmp(*target),
            JmpIf { cond, target, .. } => self.translate_jmp_if(cond, *target),
            JmpIfNot { cond, target, .. } => self.translate_jmp_if_not(cond, *target),
            Ret { value, .. } => self.translate_ret(value),

            Call {
                dst,
                func,
                args,
                def,
                ..
            } => self.translate_call(dst, func, args, *def),
            CallVirt {
                dst,
                obj,
                method_name,
                args,
                ..
            } => self.translate_call_virt(dst, obj, method_name.as_str(), args),
            CallDyn {
                dst, func, args, ..
            } => self.translate_call_dyn(dst, func, args),
            Alloc { dst, .. } => self.translate_alloc(dst),
            Free { .. } => Ok(BytecodeInstruction::new(opcode::NOP, vec![])),
            AllocArray { dst, .. } => self.translate_alloc_array(dst),
            AllocFixedArray { dst, count, .. } => {
                // #299 §2：定长数组构造——NEW_ARRAY(dst, count)
                let dst_reg = self.operand_resolver.to_reg(dst)?;
                Ok(BytecodeInstruction::new(
                    opcode::NEW_ARRAY,
                    vec![
                        dst_reg,
                        *count as u8,
                        (*count >> 8) as u8,
                        (*count >> 16) as u8,
                        (*count >> 24) as u8,
                    ],
                ))
            }

            LoadField {
                dst, src, field, ..
            } => self.translate_load_field(dst, src, *field),
            StoreField {
                dst, field, src, ..
            } => self.translate_store_field(dst, *field, src),
            LoadIndex {
                dst, src, index, ..
            } => self.translate_load_index(dst, src, index),
            StoreIndex {
                dst, index, src, ..
            } => self.translate_store_index(dst, index, src),
            Contains {
                dst,
                elem,
                container,
                ..
            } => {
                // #299 §3: membership 谓词 → CONTAINS
                let dst_reg = self.operand_resolver.to_reg(dst)?;
                let elem_reg = self.operand_resolver.to_reg(elem)?;
                let container_reg = self.operand_resolver.to_reg(container)?;
                Ok(BytecodeInstruction::new(
                    opcode::CONTAINS,
                    vec![dst_reg, elem_reg, container_reg],
                ))
            }

            Spawn {
                span: _,
                closures,
                plan,
                result,
            } => self.translate_spawn_multi(closures, plan, result),
            Yield { .. } => Ok(BytecodeInstruction::new(opcode::YIELD, vec![])),

            HeapAlloc { dst, .. } => self.translate_heap_alloc(dst),
            CreateStruct {
                span: _,
                dst,
                type_name,
                fields,
            } => self.translate_create_struct(dst, type_name, fields),
            NewDict {
                dst, keys, values, ..
            } => self.translate_new_dict(dst, keys, values),
            NewTuple { dst, items, .. } => self.translate_new_tuple(dst, items),
            NewRange {
                span: _,
                dst,
                start,
                end,
                step,
            } => self.translate_new_range(dst, start, end, step),
            CreateVariant {
                dst,
                group,
                variant,
                payload,
                ..
            } => self.translate_create_variant(dst, group, *variant, payload),
            VariantTag {
                dst, obj, group, ..
            } => self.translate_variant_access(opcode::VARIANT_TAG, dst, obj, group),
            VariantPayload {
                dst, obj, group, ..
            } => self.translate_variant_access(opcode::VARIANT_PAYLOAD, dst, obj, group),
            MakeClosure {
                span: _,
                dst,
                func,
                def,
                env,
            } => self.translate_make_closure(dst, func, *def, env),
            Drop { src: operand, .. } => self.translate_drop(operand),

            Push { src: operand, .. } => self.translate_push(operand),
            Pop { dst: operand, .. } => self.translate_pop(operand),
            Dup { .. } => Ok(BytecodeInstruction::new(opcode::NOP, vec![])),
            Swap { .. } => Ok(BytecodeInstruction::new(opcode::NOP, vec![])),

            ArcNew { dst, src, .. } => self.translate_arc_new(dst, src),
            RcNew { dst, src, .. } => self.translate_rc_new(dst, src),
            ArcClone { dst, src, .. } => self.translate_arc_clone(dst, src),
            ArcDrop { src: operand, .. } => self.translate_arc_drop(operand),

            StringLength { dst, src, .. } => self.translate_string_length(dst, src),
            StringConcat { dst, lhs, rhs, .. } => self.translate_string_concat(dst, lhs, rhs),
            StringGetChar {
                dst, src, index, ..
            } => self.translate_string_get_char(dst, src, index),
            StringFromInt { dst, src, .. } => self.translate_string_from_int(dst, src),
            StringFromFloat { dst, src, .. } => self.translate_string_from_float(dst, src),

            LoadUpvalue {
                dst, upvalue_idx, ..
            } => self.translate_load_upvalue(dst, *upvalue_idx),
            StoreUpvalue {
                src, upvalue_idx, ..
            } => self.translate_store_upvalue(src, *upvalue_idx),

            // unsafe 块和指针操作（暂不支持，跳过）
            UnsafeBlockStart { .. } | UnsafeBlockEnd { .. } => {
                Ok(BytecodeInstruction::new(opcode::NOP, vec![]))
            }
            PtrFromRef { .. } | PtrDeref { .. } | PtrStore { .. } | PtrLoad { .. } => {
                Ok(BytecodeInstruction::new(opcode::NOP, vec![]))
            }

            CloseUpvalue { src: operand, .. } => self.translate_close_upvalue(operand),

            // spawn for: 从 List 寄存器动态读取闭包并 spawn
            Instruction::SpawnFromList {
                span: _,
                closures_list,
                plan,
                result,
            } => self.translate_spawn_from_list(closures_list, plan, result),

            // 方案 A（04 §「`Phi` 的字节码形态」）：`Phi` 是 IR-only 伪指令，
            // 其 Move 串由 `translate_function` 在**汇合块入口**按前驱展开
            // （`plan_phi_edges` / `emit_phi_prologue`，入边跳转重定向到组起点）。
            // 单条 IR 指令的返回位容不下「按前驱分布的多条 Move」，故本臂不可达：
            // 走到这里说明块首 Phi 的收集漏了（内部缺陷）——响亮失败，
            // 不静默 NOP、也不静默错值（与「executor 永远看不到 Phi」同性质）。
            Phi { .. } => Err(phi_layout_violation(
                "Instruction::Phi 未在块入口展开（translate_function 的 Phi 收集漏块）",
            )),
        }
    }

    // ===== 翻译辅助方法 =====

    fn translate_move(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        // 防御性：如果 dst 是 Local，走 StoreLocal 语义
        // （IR 层统一走 Store 后正常路径不会触发，但 if/match 表达式结果聚合的 Move{dst:Local} 仍可能存在）
        if let Operand::Local(local_idx) = dst {
            let src_reg = self.operand_resolver.to_reg(src)?;
            return Ok(BytecodeInstruction::new(
                opcode::STORE_LOCAL,
                vec![*local_idx as u8, src_reg],
            ));
        }
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::MOV,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_load(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        match src {
            Operand::Const(const_val) => {
                let const_idx = self.emitter.add_constant(const_val.clone());
                Ok(BytecodeInstruction::new(
                    opcode::LOAD_CONST,
                    vec![dst_reg, (const_idx as u16) as u8, (const_idx >> 8) as u8],
                ))
            }
            Operand::Local(local_idx) => Ok(BytecodeInstruction::new(
                opcode::LOAD_LOCAL,
                vec![dst_reg, *local_idx as u8],
            )),
            Operand::Arg(arg_idx) => Ok(BytecodeInstruction::new(
                opcode::LOAD_ARG,
                vec![dst_reg, *arg_idx as u8],
            )),
            Operand::Global(idx) => {
                // LoadGlobal: dst(1) + global_idx(2, 小端)
                let idx = *idx as u16;
                Ok(BytecodeInstruction::new(
                    opcode::LOAD_GLOBAL,
                    vec![dst_reg, idx as u8, (idx >> 8) as u8],
                ))
            }
            _ => {
                let src_reg = self.operand_resolver.to_reg(src)?;
                Ok(BytecodeInstruction::new(
                    opcode::MOV,
                    vec![dst_reg, src_reg],
                ))
            }
        }
    }

    fn translate_store(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        if let Operand::Local(local_idx) = dst {
            let src_reg = self.operand_resolver.to_reg(src)?;
            Ok(BytecodeInstruction::new(
                opcode::STORE_LOCAL,
                vec![*local_idx as u8, src_reg],
            ))
        } else if let Operand::Global(idx) = dst {
            // StoreGlobal: global_idx(2, 小端) + src(1)
            let idx = *idx as u16;
            let src_reg = self.operand_resolver.to_reg(src)?;
            Ok(BytecodeInstruction::new(
                opcode::STORE_GLOBAL,
                vec![idx as u8, (idx >> 8) as u8, src_reg],
            ))
        } else {
            Err(ErrorCodeDefinition::codegen_invalid_operand("invalid operand").build())
        }
    }

    fn translate_binary_op(
        &mut self,
        opcode: u8,
        dst: &Operand,
        lhs: &Operand,
        rhs: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let lhs_reg = self.operand_resolver.to_reg(lhs)?;
        let rhs_reg = self.operand_resolver.to_reg(rhs)?;

        Ok(BytecodeInstruction::new(
            opcode,
            vec![dst_reg, lhs_reg, rhs_reg],
        ))
    }

    /// 翻译比较操作，统一使用整数比较指令
    /// 注意：实际类型检查在运行时通过 executor.rs 的 exec_compare 完成
    fn translate_compare(
        &mut self,
        eq_opcode: u8,
        _ne_opcode: u8,
        dst: &Operand,
        lhs: &Operand,
        rhs: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        // 统一使用整数比较指令，运行时通过 exec_compare 根据实际类型执行正确比较
        self.translate_binary_op(eq_opcode, dst, lhs, rhs)
    }

    fn translate_unary_op(
        &mut self,
        opcode: u8,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(opcode, vec![dst_reg, src_reg]))
    }

    fn translate_jmp(
        &mut self,
        _target: usize,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        Ok(BytecodeInstruction::new(opcode::JMP, vec![0, 0, 0, 0]))
    }

    fn translate_jmp_if(
        &mut self,
        cond: &Operand,
        _target: usize,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let cond_reg = self.operand_resolver.to_reg(cond)?;
        Ok(BytecodeInstruction::new(
            opcode::JMP_IF,
            vec![cond_reg, 0, 0, 0, 0],
        ))
    }

    fn translate_jmp_if_not(
        &mut self,
        cond: &Operand,
        _target: usize,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let cond_reg = self.operand_resolver.to_reg(cond)?;
        Ok(BytecodeInstruction::new(
            opcode::JMP_IF_NOT,
            vec![cond_reg, 0, 0, 0, 0],
        ))
    }

    fn translate_ret(
        &mut self,
        value: &Option<Operand>,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        match value {
            Some(v) => {
                let reg = self.operand_resolver.to_reg(v)?;
                Ok(BytecodeInstruction::new(opcode::RETURN_VALUE, vec![reg]))
            }
            None => Ok(BytecodeInstruction::new(opcode::RETURN, vec![])),
        }
    }

    fn translate_call(
        &mut self,
        dst: &Option<Operand>,
        func: &Operand,
        args: &[Operand],
        def: Option<crate::frontend::core::types::identity::DefId>,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = if let Some(d) = dst {
            self.operand_resolver.to_reg(d)?
        } else {
            0
        };

        let func_name = match func {
            Operand::Const(ConstValue::String(name)) => Some(name.clone()),
            _ => None,
        };

        // 四类调用目标（按优先级）：
        // 0. DefId 命中（Stage 3 主路径：字节码函数含构造器）→ CallStatic 函数表索引
        // 1. ExternRef FFI（ffi_func_meta 携带 mechanism/lib/symbol）→ CallNative 带元数据
        // 2. 名字命中（测试手工构造 IR 无 def 的回退路径）→ CallStatic 函数表索引
        // 3. std 原生/外部名（def 未命中函数表）→ CallNative 无元数据按名 FFI
        let ffi_meta = func_name
            .as_ref()
            .and_then(|n| self.ffi_func_meta.get(n).cloned());
        let def_idx = def.and_then(|d| self.function_def_to_idx.get(&d).copied());
        let bytecode_idx = func_name.as_ref().and_then(|n| {
            self.function_name_to_idx
                .as_ref()
                .and_then(|m| m.get(n).copied())
        });

        let (opcode, func_id) = if let Some(idx) = def_idx {
            (opcode::CALL_STATIC, idx as u32)
        } else if ffi_meta.is_some() {
            let const_idx = self
                .emitter
                .add_constant(ConstValue::String(func_name.clone().unwrap()));
            (opcode::CALL_NATIVE, const_idx as u32)
        } else if let Some(idx) = bytecode_idx {
            (opcode::CALL_STATIC, idx as u32)
        } else {
            match func {
                Operand::Const(ConstValue::Int(i)) => (opcode::CALL_STATIC, *i as u32),
                Operand::Const(ConstValue::String(name)) => {
                    let const_idx = self.emitter.add_constant(ConstValue::String(name.clone()));
                    (opcode::CALL_NATIVE, const_idx as u32)
                }
                // 禁止静默回退到 0 号函数（#251 同类：兑底曾静默吞掉元组字面量）
                other => {
                    return Err(ErrorCodeDefinition::codegen_invalid_operand(&format!(
                        "unresolvable call target in translate_call: {:?}",
                        other
                    ))
                    .build());
                }
            }
        };
        let base_arg_reg = if let Some(first_arg) = args.first() {
            self.operand_resolver.to_reg(first_arg)?
        } else {
            0
        };
        let mut operands = vec![dst_reg];
        operands.extend_from_slice(&func_id.to_le_bytes());
        operands.push(base_arg_reg);
        // 对 FFI 函数，在 func_name_idx 后追加 mechanism/lib/symbol 的常量池索引
        if let Some(meta) = &ffi_meta {
            let mech_idx = self
                .emitter
                .add_constant(ConstValue::String(meta.mechanism.clone()));
            let lib_idx = self
                .emitter
                .add_constant(ConstValue::String(meta.lib.clone()));
            let sym_idx = self
                .emitter
                .add_constant(ConstValue::String(meta.symbol.clone()));
            operands.extend_from_slice(&mech_idx.to_le_bytes()); // 4 bytes
            operands.extend_from_slice(&lib_idx.to_le_bytes()); // 4 bytes
            operands.extend_from_slice(&sym_idx.to_le_bytes()); // 4 bytes
        }
        operands.push(args.len() as u8);
        for arg in args {
            let arg_reg = self.operand_resolver.to_reg(arg)?;
            operands.extend_from_slice(&(arg_reg as u16).to_le_bytes());
        }

        Ok(BytecodeInstruction::new(opcode, operands))
    }

    fn translate_spawn_multi(
        &mut self,
        closures: &[Operand],
        plan: &crate::middle::ir::ir::ExecutionPlan,
        result: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(result)?;

        // 编码 closures
        let closure_regs: Vec<u8> = closures
            .iter()
            .map(|c| self.operand_resolver.to_reg(c))
            .collect::<Result<Vec<u8>, _>>()?;

        let mut operands = Vec::new();

        // dst (2 bytes LE)
        operands.extend_from_slice(&(dst_reg as u16).to_le_bytes());

        // closures.len() as u32 (4 bytes LE)
        operands.extend_from_slice(&(closure_regs.len() as u32).to_le_bytes());

        // each closure register (2 bytes LE each)
        for reg in &closure_regs {
            operands.extend_from_slice(&(*reg as u16).to_le_bytes());
        }

        // task_deps.len() as u32 (4 bytes LE)
        operands.extend_from_slice(&(plan.task_deps.len() as u32).to_le_bytes());

        // for each task: deps.len(4) + deps(4*each)
        for deps in &plan.task_deps {
            operands.extend_from_slice(&(deps.len() as u32).to_le_bytes());
            for &dep in deps {
                operands.extend_from_slice(&(dep as u32).to_le_bytes());
            }
        }

        // task_resources.len() as u32 (4 bytes LE)
        operands.extend_from_slice(&(plan.task_resources.len() as u32).to_le_bytes());

        // for each task: res_count(4) + for each res: str_len(4) + str_bytes
        for res in &plan.task_resources {
            operands.extend_from_slice(&(res.len() as u32).to_le_bytes());
            for s in res {
                let bytes = s.as_bytes();
                operands.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                operands.extend_from_slice(bytes);
            }
        }

        Ok(BytecodeInstruction::new(opcode::SPAWN, operands))
    }

    fn translate_spawn_from_list(
        &mut self,
        closures_list: &Operand,
        plan: &crate::middle::ir::ir::ExecutionPlan,
        result: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(result)?;
        let list_reg = self.operand_resolver.to_reg(closures_list)?;

        let mut operands = Vec::new();

        // dst (2 bytes LE)
        operands.extend_from_slice(&(dst_reg as u16).to_le_bytes());

        // closures_list (2 bytes LE)
        operands.extend_from_slice(&(list_reg as u16).to_le_bytes());

        // task_deps.len() as u32 (4 bytes LE)
        operands.extend_from_slice(&(plan.task_deps.len() as u32).to_le_bytes());

        // for each task: deps.len(4) + deps(4*each)
        for deps in &plan.task_deps {
            operands.extend_from_slice(&(deps.len() as u32).to_le_bytes());
            for &dep in deps {
                operands.extend_from_slice(&(dep as u32).to_le_bytes());
            }
        }

        // task_resources.len() as u32 (4 bytes LE)
        operands.extend_from_slice(&(plan.task_resources.len() as u32).to_le_bytes());

        // for each task: res_count(4) + for each res: str_len(4) + str_bytes
        for res in &plan.task_resources {
            operands.extend_from_slice(&(res.len() as u32).to_le_bytes());
            for s in res {
                let bytes = s.as_bytes();
                operands.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                operands.extend_from_slice(bytes);
            }
        }

        Ok(BytecodeInstruction::new(opcode::SPAWN_FROM_LIST, operands))
    }

    fn translate_call_virt(
        &mut self,
        dst: &Option<Operand>,
        obj: &Operand,
        method_name: &str,
        args: &[Operand],
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = if let Some(d) = dst {
            self.operand_resolver.to_reg(d)?
        } else {
            0
        };
        let obj_reg = self.operand_resolver.to_reg(obj)?;
        let name_idx = self
            .emitter
            .add_constant(ConstValue::String(method_name.to_owned())) as u16;
        let base_arg_reg = if let Some(first_arg) = args.first() {
            self.operand_resolver.to_reg(first_arg)?
        } else {
            0
        };
        let mut operands = vec![dst_reg, obj_reg];
        operands.extend_from_slice(&name_idx.to_le_bytes());
        operands.push(base_arg_reg);
        operands.push(args.len() as u8);
        Ok(BytecodeInstruction::new(opcode::CALL_VIRT, operands))
    }

    fn translate_call_dyn(
        &mut self,
        dst: &Option<Operand>,
        func: &Operand,
        args: &[Operand],
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = if let Some(d) = dst {
            self.operand_resolver.to_reg(d)?
        } else {
            0
        };
        let obj_reg = self.operand_resolver.to_reg(func)?;
        let mut arg_regs: Vec<Reg> = Vec::new();
        for arg in args {
            let reg = self.operand_resolver.to_reg(arg)? as u16;
            arg_regs.push(Reg(reg));
        }
        let mut operands = vec![dst_reg, obj_reg];
        operands.extend_from_slice(&0u16.to_le_bytes());
        for reg in &arg_regs {
            operands.push(reg.0 as u8);
        }
        operands.push(arg_regs.len() as u8);
        Ok(BytecodeInstruction::new(opcode::CALL_DYN, operands))
    }

    fn translate_alloc(
        &mut self,
        dst: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        Ok(BytecodeInstruction::new(opcode::STACK_ALLOC, vec![dst_reg]))
    }

    fn translate_alloc_array(
        &mut self,
        dst: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        Ok(BytecodeInstruction::new(
            opcode::NEW_LIST_WITH_CAP,
            vec![dst_reg, 0, 0],
        ))
    }

    fn translate_load_field(
        &mut self,
        dst: &Operand,
        src: &Operand,
        field: usize,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        let field_offset = field as u16;
        Ok(BytecodeInstruction::new(
            opcode::GET_FIELD,
            vec![
                dst_reg,
                src_reg,
                (field_offset & 0xFF) as u8,
                (field_offset >> 8) as u8,
            ],
        ))
    }

    fn translate_store_field(
        &mut self,
        dst: &Operand,
        field: usize,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        let field_offset = field as u16;
        Ok(BytecodeInstruction::new(
            opcode::SET_FIELD,
            vec![
                dst_reg,
                (field_offset & 0xFF) as u8,
                (field_offset >> 8) as u8,
                src_reg,
            ],
        ))
    }

    fn translate_load_index(
        &mut self,
        dst: &Operand,
        src: &Operand,
        index: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        let index_reg = self.operand_resolver.to_reg(index)?;
        Ok(BytecodeInstruction::new(
            opcode::LOAD_ELEMENT,
            vec![dst_reg, src_reg, index_reg],
        ))
    }

    fn translate_store_index(
        &mut self,
        dst: &Operand,
        index: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let index_reg = self.operand_resolver.to_reg(index)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::STORE_ELEMENT,
            vec![dst_reg, index_reg, src_reg],
        ))
    }

    fn translate_heap_alloc(
        &mut self,
        dst: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        Ok(BytecodeInstruction::new(
            opcode::HEAP_ALLOC,
            vec![dst_reg, 0, 0],
        ))
    }

    /// 翻译 CreateStruct 指令
    /// 格式: dst(1) + type_name_idx(4) + field_count(1) + fields(2*count)
    fn translate_create_struct(
        &mut self,
        dst: &Operand,
        type_name: &str,
        fields: &[Operand],
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let name_idx = self
            .emitter
            .add_constant(ConstValue::String(type_name.to_string())) as u32;
        let mut operands = vec![dst_reg];
        operands.extend_from_slice(&name_idx.to_le_bytes());
        operands.push(fields.len() as u8);
        for field in fields {
            let field_reg = self.operand_resolver.to_reg(field)?;
            operands.extend_from_slice(&(field_reg as u16).to_le_bytes());
        }
        Ok(BytecodeInstruction::new(opcode::CREATE_STRUCT, operands))
    }

    /// 翻译 NewDict 指令
    /// 格式: dst(2) + pair_count(4) + keys(2*count) + values(2*count)
    fn translate_new_dict(
        &mut self,
        dst: &Operand,
        keys: &[Operand],
        values: &[Operand],
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let pair_count = keys.len() as u32;
        let mut operands = Vec::new();
        // dst (2 bytes LE)
        operands.extend_from_slice(&(dst_reg as u16).to_le_bytes());
        // pair_count (4 bytes LE)
        operands.extend_from_slice(&pair_count.to_le_bytes());
        // key registers (2 bytes LE each)
        for key in keys {
            let key_reg = self.operand_resolver.to_reg(key)?;
            operands.extend_from_slice(&(key_reg as u16).to_le_bytes());
        }
        // value registers (2 bytes LE each)
        for val in values {
            let val_reg = self.operand_resolver.to_reg(val)?;
            operands.extend_from_slice(&(val_reg as u16).to_le_bytes());
        }
        Ok(BytecodeInstruction::new(opcode::NEW_DICT, operands))
    }

    /// 翻译 NewTuple 指令
    /// 格式: dst(2) + item_count(4) + items(2*count)
    fn translate_new_tuple(
        &mut self,
        dst: &Operand,
        items: &[Operand],
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let item_count = items.len() as u32;
        let mut operands = Vec::new();
        // dst (2 bytes LE)
        operands.extend_from_slice(&(dst_reg as u16).to_le_bytes());
        // item_count (4 bytes LE)
        operands.extend_from_slice(&item_count.to_le_bytes());
        // item registers (2 bytes LE each)
        for item in items {
            let item_reg = self.operand_resolver.to_reg(item)?;
            operands.extend_from_slice(&(item_reg as u16).to_le_bytes());
        }
        Ok(BytecodeInstruction::new(opcode::NEW_TUPLE, operands))
    }

    /// 翻译 NewRange 指令（#302）
    /// 格式: dst(2) + start(2) + end(2) + step(2)
    fn translate_new_range(
        &mut self,
        dst: &Operand,
        start: &Operand,
        end: &Operand,
        step: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let start_reg = self.operand_resolver.to_reg(start)?;
        let end_reg = self.operand_resolver.to_reg(end)?;
        let step_reg = self.operand_resolver.to_reg(step)?;
        let mut operands = Vec::new();
        for r in [dst_reg, start_reg, end_reg, step_reg] {
            operands.extend_from_slice(&(r as u16).to_le_bytes());
        }
        Ok(BytecodeInstruction::new(opcode::NEW_RANGE, operands))
    }

    /// RFC-011a §6: 包装具体值为存在类型变体
    fn translate_create_variant(
        &mut self,
        dst: &Operand,
        group: &str,
        variant: u32,
        payload: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let payload_reg = self.operand_resolver.to_reg(payload)?;
        let group_idx = self
            .emitter
            .add_constant(ConstValue::String(group.to_owned())) as u16;
        let mut operands = Vec::new();
        operands.extend_from_slice(&(dst_reg as u16).to_le_bytes());
        operands.extend_from_slice(&group_idx.to_le_bytes());
        operands.extend_from_slice(&variant.to_le_bytes());
        operands.extend_from_slice(&(payload_reg as u16).to_le_bytes());
        Ok(BytecodeInstruction::new(opcode::CREATE_VARIANT, operands))
    }

    /// RFC-011a §6: 变体号/负载提取（守卫在解释器执行臂）
    fn translate_variant_access(
        &mut self,
        op: u8,
        dst: &Operand,
        obj: &Operand,
        group: &str,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let obj_reg = self.operand_resolver.to_reg(obj)?;
        let group_idx = self
            .emitter
            .add_constant(ConstValue::String(group.to_owned())) as u16;
        let mut operands = Vec::new();
        operands.extend_from_slice(&(dst_reg as u16).to_le_bytes());
        operands.extend_from_slice(&(obj_reg as u16).to_le_bytes());
        operands.extend_from_slice(&group_idx.to_le_bytes());
        Ok(BytecodeInstruction::new(op, operands))
    }

    fn translate_make_closure(
        &mut self,
        dst: &Operand,
        func_name: &str,
        def: Option<crate::frontend::core::types::identity::DefId>,
        env: &[Operand],
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        // DefId 主路径（Stage 3）；名字路径为测试手工构造 IR 的回退
        let func_id = if let Some(idx) = def.and_then(|d| self.function_def_to_idx.get(&d)) {
            *idx as u32
        } else {
            let name_to_idx = self.function_name_to_idx.as_ref().ok_or_else(|| {
                ErrorCodeDefinition::internal_error(
                    "translate_make_closure 调用时 function_name_to_idx 未初始化",
                )
                .build()
            })?;
            name_to_idx.get(func_name).copied().ok_or_else(|| {
                ErrorCodeDefinition::internal_error(&format!(
                    "闭包函数 '{}' 未在函数名映射表中注册",
                    func_name
                ))
                .build()
            })? as u32
        };
        let mut operands = vec![dst_reg];
        operands.extend_from_slice(&func_id.to_le_bytes());
        operands.push(env.len() as u8);
        for op in env {
            let reg = self.operand_resolver.to_reg(op)?;
            operands.extend_from_slice(&(reg as u16).to_le_bytes());
        }
        Ok(BytecodeInstruction::new(opcode::MAKE_CLOSURE, operands))
    }

    fn translate_drop(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(opcode::DROP, vec![reg]))
    }

    fn translate_push(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(opcode::MOV, vec![reg]))
    }

    fn translate_pop(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(opcode::MOV, vec![reg]))
    }

    fn translate_arc_new(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::ARC_NEW,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_rc_new(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::RC_NEW,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_arc_clone(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::ARC_CLONE,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_arc_drop(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(opcode::ARC_DROP, vec![reg]))
    }

    fn translate_string_length(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::STRING_LENGTH,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_string_concat(
        &mut self,
        dst: &Operand,
        lhs: &Operand,
        rhs: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let lhs_reg = self.operand_resolver.to_reg(lhs)?;
        let rhs_reg = self.operand_resolver.to_reg(rhs)?;
        Ok(BytecodeInstruction::new(
            opcode::STRING_CONCAT,
            vec![dst_reg, lhs_reg, rhs_reg],
        ))
    }

    fn translate_string_get_char(
        &mut self,
        dst: &Operand,
        src: &Operand,
        index: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        let index_reg = self.operand_resolver.to_reg(index)?;
        Ok(BytecodeInstruction::new(
            opcode::STRING_GET_CHAR,
            vec![dst_reg, src_reg, index_reg],
        ))
    }

    fn translate_string_from_int(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::STRING_FROM_INT,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_string_from_float(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::STRING_FROM_FLOAT,
            vec![dst_reg, src_reg],
        ))
    }

    fn translate_load_upvalue(
        &mut self,
        dst: &Operand,
        upvalue_idx: usize,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let dst_reg = self.operand_resolver.to_reg(dst)?;
        Ok(BytecodeInstruction::new(
            opcode::LOAD_UPVALUE,
            vec![dst_reg, upvalue_idx as u8],
        ))
    }

    fn translate_store_upvalue(
        &mut self,
        src: &Operand,
        upvalue_idx: usize,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let src_reg = self.operand_resolver.to_reg(src)?;
        Ok(BytecodeInstruction::new(
            opcode::STORE_UPVALUE,
            vec![src_reg, upvalue_idx as u8],
        ))
    }

    fn translate_close_upvalue(
        &mut self,
        operand: &Operand,
    ) -> Result<BytecodeInstruction, Diagnostic> {
        let reg = self.operand_resolver.to_reg(operand)?;
        Ok(BytecodeInstruction::new(opcode::CLOSE_UPVALUE, vec![reg]))
    }
}

impl Default for Translator {
    fn default() -> Self {
        Translator::new()
    }
}

/// 翻译器输出
pub struct TranslatorOutput {
    pub code_section: crate::middle::bytecode::file::CodeSection,
    pub const_pool: Vec<ConstValue>,
}
