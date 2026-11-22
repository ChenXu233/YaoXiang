//! WBS 2.1 / RFC-039 P2：IR 静态校验器（07-equivalence-oracle 第一层判据）
//!
//! 规范来源：
//! - docs/src/dev/architecture/07-equivalence-oracle.md §第一层（7 项不变量定义、
//!   verify_loose / verify_ssa 双模式语义）
//! - docs/src/dev/architecture/04-ssa.md（批 a 准入硬前置：verify_loose 必须先在
//!   现有非 SSA IR 上跑绿；D 方案与本校验器是前置关系）
//! - docs/src/rfc/accepted/039-compiler-architecture.md D38（verify_loose 跑不绿
//!   不开豁免——跑不绿说明 lower 存在隐式「同槽多次写」依赖，必须先修缺陷）
//!
//! 现实语义记录（实现以现实 IR 语义为准，与 07 文本的描述性差异如实登记）：
//! - 07 按「label 键控块图」描述 jump 目标不变量；现实 IR 中
//!   Jmp / JmpIf / JmpIfNot 的 target 是**函数内展平后的绝对指令下标**
//!   （lower/module_ir.rs 的 rebase_jump_targets 明文 + translator.rs 的
//!   global_ir_index 回填）。块级坐标另有一套：7b.3 起 `BasicBlock.successors`
//!   由 `ir::ir::partition_blocks` 按跳转与布局落空真实填充（块 label 是边的端点，
//!   与跳转目标的指令下标不是同一坐标）。故本模块有两套 CFG：
//!   指令级（支配性 / 类型一致 / jump 目标存在）与块级（只管 `Phi` 一致性，
//!   见 `block_predecessors`）。指令级 CFG：Jmp → {target}；JmpIf/JmpIfNot →
//!   {target, i+1}；Ret/TailCall → ∅；其余 → {i+1}。jump 目标有效性 =
//!   target < 指令总数。
//! - 不可达指令（entry 出发不可达）不参与支配性与类型一致判定——死代码检测
//!   不在 07 的 7 项不变量清单内；jump 目标有效性是纯结构性质，仍全量检查。
//! - 「类型一致」的相容判定受三套平行类型表示（P5 收敛前）与 IR 槽位类型
//!   填写精度限制：TypeVar / TypeRef 视为信息不足放行；Int / Float 各自宽度族
//!   互容；Never 是底部类型（mono.rs:169 明文）。SSA 值表（04 §262 ValueInfo）
//!   落地后收紧为逐值精确比对。当前覆盖面：Move/Load/Store、算术与位运算、
//!   比较双端、Neg/Not、Ret 与 return_type——其余指令的 dst 类型在无值表时
//!   不可静态推导，不查不等于豁免（无判定依据，非已知违规）。
//! - **槽位类型表整体不可信**（2026-10-06 语料实测登记）：lower 从不写
//!   LocalSlot.ty——register_local（lower/scope.rs）只写 name，temp 槽由
//!   Int(64) 占位填充（take_cur_locals）；globals 无注解时同样是 Int(64) 占位
//!   （register_local）。故「类型一致」判定的可信源收窄为**参数槽签名类型**（
//!   FunctionIR.params，注解转换）与**常量字面量**；其余槽位不参与比对。
//!   这不是豁免——豁免是「已知违规不修」，此处是「判定依据不存在」；
//!   完整判定面依赖 SSA 值表（04 §262 ValueInfo，P7 批 b 落地）。
//! - init 序列的 Local 越界不查：`init_locals` 是 #368 调试信息视角的表，
//!   不是分配上限（实测语料 init 用槽 4 而表长 1）。函数体 locals 表是完整
//!   分配表（lower/scope.rs 的 take_cur_locals 按高水位 resize），Local 越界判定仅在函数体生效。
//! - 「Phi 一致」不变量：`Instruction::Phi` 由 P7 批 b（7b.1a）引入（04 §273，
//!   IR-only 伪指令，translator 展开为 Move 串）。本模块的 def/use 提取是
//!   **穷尽 match、刻意无通配臂**（与 ir.rs span() 同款强制哲学）：Phi 的
//!   `dst` 计定义点、每个 `incoming` 输入计使用点（`def_operands` /
//!   `use_operands`）。`verify_ssa` 的 Phi 一致性分两批落：7b.1a 落
//!   `dst` 必为 `Operand::Value`、`incoming` 按前驱 label 严格升序两条；
//!   7b.1b 落三条残项——前驱数（`BasicBlock.successors` 反推的块级前驱集）、
//!   前驱 label 集合、逐输入类型一致（`values` 值表提供类型；类型不可得按
//!   文件头「现实语义记录」第 3 条显式不判定 / 显式报越界，不静默放行）。
//!   **前置已落地**：块级 `successors` 自 7b.3 起由 `ir::ir::partition_blocks`
//!   在每条生产构造点真实填充（`lower` 的块按跳转与布局落空切分），故前驱数 /
//!   前驱 label 集合两条判据在真实 IR 上有数据；`Phi` 的生产（lowering 产 Phi）
//!   仍为 0，届时**直接**受本判据约束（09a §P7 7b.3 契约 §二 第 1 条）。
//! - 「内层隔离」的 ModuleIR 级可操作化（WBS 2.1.4 改为**按槽身份**）：lower 的
//!   save/restore 契约（lower/mod.rs 的 `cur_locals` / `next_temp` / `cur_span` 字段）
//!   失败形态是**同一张槽位表被两个函数体生成期共用**。身份维度 = `LocalSlot::origin`
//!   （生成帧 `FrameId`，由 lower/scope.rs 的 NestedBodyGuard 发放）：一张表里出现的
//!   帧号至多一种，否则红。
//!   为什么不再用「MakeClosure 反推嵌套 + 名字交集」：`MakeClosure` 是**函数值的一等
//!   表示**（lower/expr/access.rs 的 emit_module_export_value），不是词法嵌套关系；
//!   按名交集会把「两个独立声明用同一个源码名」判成泄漏（2.1.4 的两处误报即
//!   `main` 的局部 `r` 与 std.json.parse 自己的 `r`），而真正的泄漏一旦把外层表
//!   清空又会让名交集的外侧变空（假阴性）。身份判据两头都堵上。

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fmt;

use crate::frontend::core::sema::MonoType;

use crate::ast::Quantity;

use super::ir::{
    ConstValue, FrameId, FunctionBody, FunctionIR, Instruction, LocalSlot, ModuleIR, Operand,
    ValueInfo,
};

/// 校验模式（07 §207）：非 SSA 现状用 Loose；SSA 形态（P7 批 b 后）用 Ssa。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyMode {
    /// 宽松模式：支配性 + jump 目标 + 槽位越界 + 类型一致 + 内层隔离。
    /// 允许同槽多次定义（现状临时槽位语句级回收，#393）。
    Loose,
    /// 严格模式：Loose 全部 + 值定义唯一性 + Phi 一致性。
    /// 现有 IR 必然红（多次定义是现行分配策略），P7 批 b 后转绿。
    Ssa,
}

/// 不变量类别（07 §第一层表格，顺序一致）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvariantKind {
    /// use-before-def：读的槽位必须有支配当前点的定义路径
    Dominance,
    /// 值定义唯一性（仅 Ssa）
    UniqueDefinition,
    /// Phi 一致性（仅 Ssa）：7b.1a 落「dst 必为 Value / incoming label 升序」，
    /// 7b.1b 落「前驱数 / 前驱集合 / 逐输入类型一致」（09a §P7 7b.1b 契约 §三）
    PhiConsistency,
    /// 跳转目标是合法指令下标
    JumpTarget,
    /// Local / Temp / Arg / Global 槽位下标不越界
    SlotBounds,
    /// 指令 dst 与 src 的 MonoType 相容
    TypeConsistency,
    /// 外层函数 locals 不含内层（闭包）函数的自有局部名
    InnerIsolation,
    /// 量词事实自洽：q=0（`Quantity::Zero`）只出现在参数区的**具名**槽上
    QuantityFact,
}

impl InvariantKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dominance => "use-before-def 支配性",
            Self::UniqueDefinition => "值定义唯一性",
            Self::PhiConsistency => "Phi 一致性",
            Self::JumpTarget => "jump 目标存在",
            Self::SlotBounds => "槽位越界",
            Self::TypeConsistency => "类型一致",
            Self::InnerIsolation => "内层隔离",
            Self::QuantityFact => "量词事实自洽",
        }
    }
}

/// 单条违规。
#[derive(Debug, Clone)]
pub struct Violation {
    pub invariant: InvariantKind,
    /// 所属函数（模块初始化序列的伪名为 "__module_init"）
    pub function: String,
    /// 展平指令下标（模块级违规为 None）
    pub instruction: Option<usize>,
    pub message: String,
}

/// 校验失败：收集全部违规（非首错即停，便于一次性看清缺陷面）。
#[derive(Debug)]
pub struct VerifyError {
    pub violations: Vec<Violation>,
}

impl fmt::Display for VerifyError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        writeln!(f, "IR 校验失败：{} 项违规", self.violations.len())?;
        for v in &self.violations {
            let at = v
                .instruction
                .map(|i| format!(" instr#{i}"))
                .unwrap_or_default();
            writeln!(
                f,
                "  [{}] {}{}: {}",
                v.invariant.label(),
                v.function,
                at,
                v.message
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for VerifyError {}

/// 07 §102 契约形态：verify(ir) -> Result<(), IrError>。
pub fn verify(
    ir: &ModuleIR,
    mode: VerifyMode,
) -> Result<(), VerifyError> {
    let mut violations = Vec::new();

    // 第一遍：从 MakeClosure{func, env} 恢复闭包 env 槽数（帧前段布局，
    // lower/expr/closure.rs 的 generate_lambda_expr_ir）——闭包体的参数槽区间判定依赖它。
    let mut env_lens: HashMap<&str, usize> = HashMap::new();
    for func in &ir.functions {
        for instr in func.all_instructions() {
            if let Instruction::MakeClosure { func, env, .. } = instr {
                env_lens.entry(func.as_str()).or_insert(env.len());
            }
        }
    }

    for func in &ir.functions {
        let env_len = env_lens.get(func.name.as_str()).copied().unwrap_or(0);
        let view = FnView::from_function(func, env_len);
        check_fn(&view, ir, mode, &mut violations);
    }

    // 模块初始化序列：线性指令流 + 跳转（循环回跳），槽位空间是 init_locals，
    // 无参数槽。包装为伪函数走同一套检查。
    if !ir.init.is_empty() {
        let view = FnView::from_init(ir);
        check_fn(&view, ir, mode, &mut violations);
    }

    check_inner_isolation(ir, &mut violations);

    if violations.is_empty() {
        Ok(())
    } else {
        Err(VerifyError { violations })
    }
}

/// 宽松模式便捷入口（2.1.2 硬门槛：现有非 SSA IR 上必须跑绿）。
pub fn verify_loose(ir: &ModuleIR) -> Result<(), VerifyError> {
    verify(ir, VerifyMode::Loose)
}

/// 严格模式便捷入口（P7 批 b 验收：行为等价 + verify_ssa 绿）。
pub fn verify_ssa(ir: &ModuleIR) -> Result<(), VerifyError> {
    verify(ir, VerifyMode::Ssa)
}

// =====================
// 函数视图：指令展平（translator 同款线性序）
// =====================

/// 单一指令流上下文（函数体或模块 init 序列）。
///
/// 块级信息的用途仅限 Phi 一致性判据（7b.1b）：`block_labels` 把展平后的
/// 指令下标映回所属基本块 label；`predecessors` 从 `BasicBlock.successors`
/// 反推（块级，与指令级 CFG 是两套坐标，见文件头「现实语义记录」第 1 条）。
/// `from_init` 无基本块结构，四者为空——Phi 出现在 init 时判据显式报违规。
struct FnView<'a> {
    name: String,
    params_len: usize,
    params: &'a [MonoType],
    /// 闭包 env 槽数（运行时铺入帧前段，lower/expr/closure.rs 的 generate_lambda_expr_ir）：
    /// 闭包体的参数槽位于 env_len..env_len+params_len，普通函数为 0..params_len。
    env_len: usize,
    return_type: Option<&'a MonoType>,
    locals: &'a [LocalSlot],
    /// SSA 值表（仅 Code 体有效；本批生产恒空）
    values: &'a [ValueInfo],
    instrs: Vec<&'a Instruction>,
    /// 展平指令下标 → 所属基本块 label（`from_init` 为空）
    block_labels: Vec<usize>,
    /// 块级前驱：块 label → 前驱块 label 集合（`from_init` 为空）
    predecessors: HashMap<usize, HashSet<usize>>,
}

/// 块级前驱表：对每条 `block → successors` 边，`predecessors[s] ∋ block.label`。
///
/// 只认 `BasicBlock.successors`（04 §三条构造约束的块级面）。7b.3 起该表由
/// `ir::ir::partition_blocks` 真实填充（真 `lower` 产物的循环 / 分支块都有出边，
/// 故本判据在真实 IR 上有数据）；合成夹具自带块图。
fn block_predecessors(blocks: &[super::ir::BasicBlock]) -> HashMap<usize, HashSet<usize>> {
    let mut predecessors: HashMap<usize, HashSet<usize>> = HashMap::new();
    for block in blocks {
        for &succ in &block.successors {
            predecessors.entry(succ).or_default().insert(block.label);
        }
    }
    predecessors
}

impl<'a> FnView<'a> {
    fn from_function(
        func: &'a FunctionIR,
        env_len: usize,
    ) -> Self {
        match &func.body {
            FunctionBody::Code {
                blocks,
                locals,
                values,
                ..
            } => Self {
                name: func.name.clone(),
                params_len: func.params.len(),
                params: &func.params,
                env_len,
                return_type: Some(&func.return_type),
                locals,
                values,
                instrs: blocks.iter().flat_map(|b| b.instructions.iter()).collect(),
                block_labels: blocks
                    .iter()
                    .flat_map(|b| std::iter::repeat_n(b.label, b.instructions.len()))
                    .collect(),
                predecessors: block_predecessors(blocks),
            },
            FunctionBody::TypeDecl { .. } => Self {
                name: func.name.clone(),
                params_len: func.params.len(),
                params: &func.params,
                env_len,
                return_type: Some(&func.return_type),
                locals: &[],
                values: &[],
                instrs: Vec::new(),
                block_labels: Vec::new(),
                predecessors: HashMap::new(),
            },
        }
    }

    fn from_init(ir: &'a ModuleIR) -> Self {
        Self {
            name: "__module_init".to_string(),
            params_len: 0,
            params: &[],
            env_len: 0,
            return_type: None,
            locals: &ir.init_locals,
            values: &[],
            instrs: ir.init.iter().collect(),
            block_labels: Vec::new(),
            predecessors: HashMap::new(),
        }
    }

    /// 入口已定义槽区间：运行时把 env 与参数铺进帧前段（frames.rs with_args），
    /// 即 0..env_len+params_len。
    fn entry_defined_range(&self) -> std::ops::Range<usize> {
        0..(self.env_len + self.params_len)
    }

    /// 参数槽的可信签名类型：locals 槽 i 对应参数时取 params[i - env_len]。
    fn param_slot_type(
        &self,
        slot: usize,
    ) -> Option<&'a MonoType> {
        let idx = slot.checked_sub(self.env_len)?;
        if idx < self.params_len && slot >= self.env_len {
            self.params.get(idx)
        } else {
            None
        }
    }
}

fn check_fn(
    view: &FnView<'_>,
    ir: &ModuleIR,
    mode: VerifyMode,
    out: &mut Vec<Violation>,
) {
    if view.instrs.is_empty() {
        return;
    }
    check_jump_targets(view, out);
    check_slot_bounds(view, ir, out);
    check_quantity_facts(view, out);

    let (succ, pred) = build_cfg(&view.instrs);
    let reachable = reachable_from(0, &succ);
    let (defs, defs_at, uses, entry_defined) = collect_defs_uses(view);
    let n_slots = slot_space(view, &defs, &uses);
    let def_in = must_defined(
        view.instrs.len(),
        n_slots,
        &pred,
        0,
        &reachable,
        &defs_at,
        &entry_defined,
    );

    check_dominance(view, &uses, &def_in, &reachable, out);
    check_type_consistency(view, ir, &reachable, out);

    if mode == VerifyMode::Ssa {
        check_unique_definition(view, &defs, &entry_defined, out);
        check_phi_structure(view, out);
    }
}

// =====================
// 不变量 8：量词事实自洽（D62 / issue #533）
// =====================

/// `LocalSlot::quantity` 是**声明绑定**事实的投影：只有参数区的具名槽能携带
/// `Some(Quantity::Zero)`。临时槽、非参数局部槽、模块初始化槽都没有声明载体——
/// 它们若带 q=0，说明事实在投影时被串到了别的槽（静默错配）。
///
/// 这条闸门是 A 步（量词事实先行）的**消费端**：真消费者（mono 按 q=0 擦除）在
/// 阶段 C 接入，先由校验器保证登记的事实不会落到无绑定的槽上。
fn check_quantity_facts(
    view: &FnView<'_>,
    out: &mut Vec<Violation>,
) {
    if view.instrs.is_empty() {
        return;
    }
    let param_end = view.env_len + view.params_len;
    for (slot, local) in view.locals.iter().enumerate() {
        if local.quantity != Some(Quantity::Zero) {
            continue;
        }
        if local.name.is_some() && slot < param_end {
            continue;
        }
        out.push(Violation {
            invariant: InvariantKind::QuantityFact,
            function: view.name.clone(),
            instruction: None,
            message: format!(
                "槽 {slot} 携带 q=0 事实，但它不在参数区 0..{param_end} 或没有具名绑定（name={:?}）",
                local.name
            ),
        });
    }
}

// =====================
// 不变量 4：jump 目标存在
// =====================

fn check_jump_targets(
    view: &FnView<'_>,
    out: &mut Vec<Violation>,
) {
    let total = view.instrs.len();
    for (i, instr) in view.instrs.iter().enumerate() {
        let target = match instr {
            Instruction::Jmp { target, .. }
            | Instruction::JmpIf { target, .. }
            | Instruction::JmpIfNot { target, .. } => *target,
            _ => continue,
        };
        if target >= total {
            out.push(Violation {
                invariant: InvariantKind::JumpTarget,
                function: view.name.clone(),
                instruction: Some(i),
                message: format!("跳转目标 {target} 超出指令总数 {total}"),
            });
        }
    }
}

// =====================
// 不变量 5：槽位越界（Local/Temp/Arg/Global）
// =====================

fn check_slot_bounds(
    view: &FnView<'_>,
    ir: &ModuleIR,
    out: &mut Vec<Violation>,
) {
    for (i, instr) in view.instrs.iter().enumerate() {
        for op in def_operands(instr).into_iter().chain(use_operands(instr)) {
            let (kind, idx, bound) = match op {
                Operand::Local(n) => ("local", *n, view.locals.len()),
                Operand::Arg(n) => ("arg", *n, view.params_len),
                Operand::Global(n) => ("global", *n, ir.globals.len()),
                // Const / Value 不是槽位引用
                _ => continue,
            };
            // init_locals 是 #368 调试信息视角的表而非分配上限（实测 init
            // 用槽 4 而表长 1）——init 序列的 local 越界无判定依据，跳过；
            // 函数体 locals 是完整分配表（lower/scope.rs 的 take_cur_locals 按高水位 resize），正常查。
            // Global / Arg 越界全量查。
            if kind == "local" && view.return_type.is_none() {
                continue;
            }
            if idx >= bound {
                out.push(Violation {
                    invariant: InvariantKind::SlotBounds,
                    function: view.name.clone(),
                    instruction: Some(i),
                    message: format!("{kind} 槽位 {idx} 越界（上限 {bound}）"),
                });
            }
        }
    }
}

// =====================
// CFG（指令级）与支配性
// =====================

/// 出边/入边表。终结指令：Jmp（单边）、JmpIf/JmpIfNot（双边）、Ret/TailCall（无）。
fn build_cfg(instrs: &[&Instruction]) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let n = instrs.len();
    let mut succ = vec![Vec::new(); n];
    let mut pred = vec![Vec::new(); n];
    for (i, instr) in instrs.iter().enumerate() {
        let next = if i + 1 < n { Some(i + 1) } else { None };
        let edges: Vec<usize> = match instr {
            Instruction::Jmp { target, .. } => vec![*target],
            Instruction::JmpIf { target, .. } | Instruction::JmpIfNot { target, .. } => {
                let mut e = vec![*target];
                if let Some(nx) = next {
                    e.push(nx);
                }
                e
            }
            Instruction::Ret { .. } | Instruction::TailCall { .. } => Vec::new(),
            _ => next.into_iter().collect(),
        };
        for &t in &edges {
            if t < n {
                succ[i].push(t);
                pred[t].push(i);
            }
        }
    }
    (succ, pred)
}

fn reachable_from(
    entry: usize,
    succ: &[Vec<usize>],
) -> HashSet<usize> {
    let mut seen = HashSet::new();
    let mut queue = VecDeque::from([entry]);
    while let Some(n) = queue.pop_front() {
        if seen.insert(n) {
            queue.extend(succ[n].iter().copied());
        }
    }
    seen
}

/// 位集（指令数规模小，但 322 文件语料下迭代 DOM 需要字操作提速）。
#[derive(Clone, PartialEq, Eq)]
struct BitSet {
    words: Vec<u64>,
}

impl BitSet {
    fn full(len: usize) -> Self {
        let mut words = vec![u64::MAX; len / 64 + 1];
        let rem = len % 64;
        if rem != 0 {
            let last = words.len() - 1;
            words[last] = (1u64 << rem) - 1;
        }
        Self { words }
    }

    fn empty(len: usize) -> Self {
        Self {
            words: vec![0; len / 64 + 1],
        }
    }

    fn set(
        &mut self,
        i: usize,
    ) {
        self.words[i / 64] |= 1u64 << (i % 64);
    }

    fn get(
        &self,
        i: usize,
    ) -> bool {
        self.words[i / 64] & (1u64 << (i % 64)) != 0
    }

    /// self ∩= other（支配性迭代的前驱交汇）。
    fn intersect_with(
        &mut self,
        other: &BitSet,
    ) {
        for (a, b) in self.words.iter_mut().zip(other.words.iter()) {
            *a &= b;
        }
    }
}

/// must-defined 正向数据流（07「一条支配当前点的定义路径」在非 SSA 现实下
/// 的可操作化）：DefIn(entry) = 入口定义槽（env + 参数）；DefIn(i) = ⋂ DefOut(preds)
/// （must 语义：所有入路径都定义的槽才算已定义）；DefOut(i) = DefIn(i) ∪ defs_at(i)。
/// use(n, i) 合法 ⟺ n ∈ DefIn(i)。
///
/// 为什么不是单点支配判定：非 SSA 下 if-else 双分支各写同槽、汇合点读是
/// 合法形态（任一路径都先写）——单点支配会把合法 IR 误判为红（2026-10-06
/// 语料实测：rfc010a_block_value.yx tail_match 的 match 汇合形态）。
/// SSA 形态下本分析与支配性重合（唯一定义使 must-defined ⟺ 支配）。
///
/// 位集长度为槽位空间（locals.len() 与最大引用槽取大，越界槽由
/// SlotBounds 单独报，此处不二次判定）。
fn must_defined(
    n_instrs: usize,
    n_slots: usize,
    pred: &[Vec<usize>],
    entry: usize,
    reachable: &HashSet<usize>,
    defs_at: &[Vec<usize>],
    entry_defined: &HashSet<usize>,
) -> Vec<BitSet> {
    // DefIn/DefOut 位集；DefIn 初值全集（交汇迭代的 top），entry 除外
    let mut def_in: Vec<BitSet> = (0..n_instrs).map(|_| BitSet::full(n_slots)).collect();
    let mut entry_set = BitSet::empty(n_slots);
    for &s in entry_defined {
        if s < n_slots {
            entry_set.set(s);
        }
    }
    def_in[entry] = entry_set;
    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..n_instrs {
            if !reachable.contains(&i) || i == entry {
                continue;
            }
            let mut acc: Option<BitSet> = None;
            for &p in &pred[i] {
                if !reachable.contains(&p) {
                    continue;
                }
                // DefOut(p) = DefIn(p) ∪ defs_at(p)，交汇直接在其上取
                let mut p_out = def_in[p].clone();
                for &d in &defs_at[p] {
                    if d < n_slots {
                        p_out.set(d);
                    }
                }
                match &mut acc {
                    None => acc = Some(p_out),
                    Some(a) => a.intersect_with(&p_out),
                }
            }
            if let Some(new) = acc {
                if new != def_in[i] {
                    def_in[i] = new;
                    changed = true;
                }
            }
        }
    }
    def_in
}

// =====================
// def / use 提取（穷尽 match，无通配臂——新增变体必须显式处理）
// =====================

/// 槽位语义判定哲学：def = 槽位本身被赋予新值（之后读该槽得到新值）；
/// use = 槽位当前值被读取。StoreField / StoreIndex / PtrStore 的 dst 是
/// 容器/指针（读），写入的是其内部字段而非槽位本身。
fn def_operands(instr: &Instruction) -> Vec<&Operand> {
    match instr {
        Instruction::Move { dst, .. }
        | Instruction::Load { dst, .. }
        | Instruction::Store { dst, .. }
        | Instruction::Pop { dst, .. }
        | Instruction::Neg { dst, .. }
        | Instruction::Not { dst, .. }
        | Instruction::Alloc { dst, .. }
        | Instruction::AllocArray { dst, .. }
        | Instruction::AllocFixedArray { dst, .. }
        | Instruction::LoadField { dst, .. }
        | Instruction::LoadIndex { dst, .. }
        | Instruction::Contains { dst, .. }
        | Instruction::HeapAlloc { dst, .. }
        | Instruction::CreateStruct { dst, .. }
        | Instruction::NewDict { dst, .. }
        | Instruction::NewTuple { dst, .. }
        | Instruction::NewRange { dst, .. }
        | Instruction::CreateVariant { dst, .. }
        | Instruction::VariantTag { dst, .. }
        | Instruction::VariantPayload { dst, .. }
        | Instruction::MakeClosure { dst, .. }
        | Instruction::ArcNew { dst, .. }
        | Instruction::RcNew { dst, .. }
        | Instruction::ArcClone { dst, .. }
        | Instruction::PtrFromRef { dst, .. }
        | Instruction::PtrDeref { dst, .. }
        | Instruction::PtrLoad { dst, .. }
        | Instruction::StringLength { dst, .. }
        | Instruction::StringConcat { dst, .. }
        | Instruction::StringGetChar { dst, .. }
        | Instruction::StringFromInt { dst, .. }
        | Instruction::StringFromFloat { dst, .. }
        | Instruction::LoadUpvalue { dst, .. } => vec![dst],
        Instruction::Add { dst, .. }
        | Instruction::Sub { dst, .. }
        | Instruction::Mul { dst, .. }
        | Instruction::Div { dst, .. }
        | Instruction::Mod { dst, .. }
        | Instruction::And { dst, .. }
        | Instruction::Or { dst, .. }
        | Instruction::Xor { dst, .. }
        | Instruction::Shl { dst, .. }
        | Instruction::Shr { dst, .. }
        | Instruction::Sar { dst, .. }
        | Instruction::Eq { dst, .. }
        | Instruction::Ne { dst, .. }
        | Instruction::Lt { dst, .. }
        | Instruction::Le { dst, .. }
        | Instruction::Gt { dst, .. }
        | Instruction::Ge { dst, .. } => vec![dst],
        Instruction::Call { dst, .. }
        | Instruction::CallVirt { dst, .. }
        | Instruction::CallDyn { dst, .. } => dst.iter().collect(),
        Instruction::Spawn { result, .. } | Instruction::SpawnFromList { result, .. } => {
            vec![result]
        }
        Instruction::Push { .. }
        | Instruction::Dup { .. }
        | Instruction::Swap { .. }
        | Instruction::Jmp { .. }
        | Instruction::JmpIf { .. }
        | Instruction::JmpIfNot { .. }
        | Instruction::TailCall { .. }
        | Instruction::Ret { .. }
        | Instruction::Free { .. }
        | Instruction::StoreField { .. }
        | Instruction::StoreIndex { .. }
        | Instruction::Yield { .. }
        | Instruction::Drop { .. }
        | Instruction::ArcDrop { .. }
        | Instruction::UnsafeBlockStart { .. }
        | Instruction::UnsafeBlockEnd { .. }
        | Instruction::PtrStore { .. }
        | Instruction::StoreUpvalue { .. }
        | Instruction::CloseUpvalue { .. } => Vec::new(),
        // Phi 的 dst 是 SSA 值的定义点（07 第一层「值定义唯一性」的 def 面）
        Instruction::Phi { dst, .. } => vec![dst],
    }
}

fn use_operands(instr: &Instruction) -> Vec<&Operand> {
    match instr {
        Instruction::Move { src, .. }
        | Instruction::Load { src, .. }
        | Instruction::Store { src, .. }
        | Instruction::Push { src, .. }
        | Instruction::Neg { src, .. }
        | Instruction::Not { src, .. }
        | Instruction::Free { src, .. }
        | Instruction::Drop { src, .. }
        | Instruction::ArcDrop { src, .. }
        | Instruction::PtrLoad { src, .. }
        | Instruction::StringLength { src, .. }
        | Instruction::StringFromInt { src, .. }
        | Instruction::StringFromFloat { src, .. }
        | Instruction::StoreUpvalue { src, .. }
        | Instruction::CloseUpvalue { src, .. } => vec![src],
        Instruction::Add { lhs, rhs, .. }
        | Instruction::Sub { lhs, rhs, .. }
        | Instruction::Mul { lhs, rhs, .. }
        | Instruction::Div { lhs, rhs, .. }
        | Instruction::Mod { lhs, rhs, .. }
        | Instruction::And { lhs, rhs, .. }
        | Instruction::Or { lhs, rhs, .. }
        | Instruction::Xor { lhs, rhs, .. }
        | Instruction::Shl { lhs, rhs, .. }
        | Instruction::Shr { lhs, rhs, .. }
        | Instruction::Sar { lhs, rhs, .. }
        | Instruction::Eq { lhs, rhs, .. }
        | Instruction::Ne { lhs, rhs, .. }
        | Instruction::Lt { lhs, rhs, .. }
        | Instruction::Le { lhs, rhs, .. }
        | Instruction::Gt { lhs, rhs, .. }
        | Instruction::Ge { lhs, rhs, .. }
        | Instruction::StringConcat { lhs, rhs, .. } => vec![lhs, rhs],
        Instruction::Pop { .. }
        | Instruction::Dup { .. }
        | Instruction::Swap { .. }
        | Instruction::Jmp { .. }
        | Instruction::Yield { .. }
        | Instruction::UnsafeBlockStart { .. }
        | Instruction::UnsafeBlockEnd { .. }
        | Instruction::LoadUpvalue { .. } => Vec::new(),
        Instruction::JmpIf { cond, .. } | Instruction::JmpIfNot { cond, .. } => vec![cond],
        Instruction::Call { func, args, .. } | Instruction::CallDyn { func, args, .. } => {
            let mut v = vec![func];
            v.extend(args);
            v
        }
        Instruction::CallVirt { obj, args, .. } => {
            let mut v = vec![obj];
            v.extend(args);
            v
        }
        Instruction::TailCall { func, args, .. } => {
            let mut v = vec![func];
            v.extend(args);
            v
        }
        Instruction::Ret { value, .. } => value.iter().collect(),
        Instruction::Alloc { size, .. } => vec![size],
        Instruction::AllocArray {
            size, elem_size, ..
        } => vec![size, elem_size],
        Instruction::AllocFixedArray { .. } => Vec::new(),
        Instruction::LoadField { src, .. } => vec![src],
        // StoreField/StoreIndex：dst 是容器（读），写入其内部而非槽位本身
        Instruction::StoreField { dst, src, .. } => vec![dst, src],
        Instruction::LoadIndex { src, index, .. } => vec![src, index],
        Instruction::StoreIndex {
            dst, index, src, ..
        } => vec![dst, index, src],
        Instruction::Contains {
            elem, container, ..
        } => vec![elem, container],
        Instruction::Spawn { closures, .. } => closures.iter().collect(),
        Instruction::SpawnFromList { closures_list, .. } => vec![closures_list],
        Instruction::HeapAlloc { .. } => Vec::new(),
        Instruction::CreateStruct { fields, .. } => fields.iter().collect(),
        Instruction::NewDict { keys, values, .. } => keys.iter().chain(values.iter()).collect(),
        Instruction::NewTuple { items, .. } => items.iter().collect(),
        Instruction::NewRange {
            start, end, step, ..
        } => vec![start, end, step],
        Instruction::CreateVariant { payload, .. } => vec![payload],
        Instruction::VariantTag { obj, .. } | Instruction::VariantPayload { obj, .. } => vec![obj],
        Instruction::MakeClosure { env, .. } => env.iter().collect(),
        Instruction::ArcNew { src, .. }
        | Instruction::RcNew { src, .. }
        | Instruction::ArcClone { src, .. } => {
            vec![src]
        }
        Instruction::PtrFromRef { src, .. } | Instruction::PtrDeref { src, .. } => vec![src],
        Instruction::PtrStore { dst, src, .. } => vec![dst, src],
        Instruction::StringGetChar { src, index, .. } => vec![src, index],
        // Phi 的每个 incoming 输入都是使用点（来自前驱的值）
        Instruction::Phi { incoming, .. } => incoming.iter().map(|(_, op)| op).collect(),
    }
}

/// 槽位定义/使用点收集：defs（槽→定义点列表）、defs_at（指令→定义槽列表）、
/// uses（槽, 指令）、entry_defined（env 槽与参数槽，运行时铺入帧前段）。
#[allow(clippy::type_complexity)]
fn collect_defs_uses(
    view: &FnView<'_>
) -> (
    HashMap<usize, Vec<usize>>,
    Vec<Vec<usize>>,
    Vec<(usize, usize)>,
    HashSet<usize>,
) {
    let mut defs: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut defs_at: Vec<Vec<usize>> = vec![Vec::new(); view.instrs.len()];
    let mut uses: Vec<(usize, usize)> = Vec::new();
    for (i, instr) in view.instrs.iter().enumerate() {
        for op in def_operands(instr) {
            if let Operand::Local(n) = op {
                defs.entry(*n).or_default().push(i);
                defs_at[i].push(*n);
            }
        }
        for op in use_operands(instr) {
            if let Operand::Local(n) = op {
                uses.push((*n, i));
            }
        }
    }
    let entry_defined: HashSet<usize> = view.entry_defined_range().collect();
    (defs, defs_at, uses, entry_defined)
}

/// 位集长度：locals 表长与实际引用槽取大（越界槽由 SlotBounds 报，
/// 支配性位集须容纳其位号以免二次误判）。
fn slot_space(
    view: &FnView<'_>,
    defs: &HashMap<usize, Vec<usize>>,
    uses: &[(usize, usize)],
) -> usize {
    let max_ref = defs
        .keys()
        .chain(uses.iter().map(|(s, _)| s))
        .copied()
        .max()
        .map_or(0, |m| m + 1);
    view.locals.len().max(max_ref)
}

// =====================
// 不变量 1：use-before-def 支配性
// =====================

/// use(n, i) 合法 ⟺ n ∈ DefIn(i)——所有到达 i 的路径上 n 都已被定义。
fn check_dominance(
    view: &FnView<'_>,
    uses: &[(usize, usize)],
    def_in: &[BitSet],
    reachable: &HashSet<usize>,
    out: &mut Vec<Violation>,
) {
    for &(slot, use_at) in uses {
        if !reachable.contains(&use_at) {
            continue;
        }
        // temp 槽（name=None）不做支配性判定：Frame 槽位以 Void 预初始化
        // （frames.rs:55），lower 有意依赖此语义表达 Void 值绑定——实测
        // empty_block_semantics.yx「empty: Void = {}」的 init Store 直接读
        // 未写槽；match 末臂守卫失败的容错落点同族（运行时路径不可达）。
        // temp 槽的错位缺陷形态是「读错值」而非「读未定义」，归 07 第三层
        // （语料行为差分）覆盖。具名槽承载源码变量，其 use-before-def 正是
        // restore 遗漏 / 作用域错位的缺陷形态，保留 must-defined 判定。
        if view.locals.get(slot).is_none_or(|s| s.name.is_none()) {
            continue;
        }
        // 越界槽位由 SlotBounds 报；位集长度已容纳（slot_space），直接查
        if !def_in[use_at].get(slot) {
            out.push(Violation {
                invariant: InvariantKind::Dominance,
                function: view.name.clone(),
                instruction: Some(use_at),
                message: format!("读取槽位 Local({slot}) 存在无定义路径（use-before-def）"),
            });
        }
    }
}

// =====================
// 不变量 2：值定义唯一性（仅 Ssa）
// =====================

fn check_unique_definition(
    view: &FnView<'_>,
    defs: &HashMap<usize, Vec<usize>>,
    entry_defined: &HashSet<usize>,
    out: &mut Vec<Violation>,
) {
    for (&slot, points) in defs {
        // SSA 下参数槽即值，重写参数槽 = 第二定义点
        let total = points.len() + usize::from(entry_defined.contains(&slot));
        if total > 1 {
            out.push(Violation {
                invariant: InvariantKind::UniqueDefinition,
                function: view.name.clone(),
                instruction: points.get(1).copied().or_else(|| points.first().copied()),
                message: format!("槽位 Local({slot}) 有 {total} 个定义点（SSA 要求唯一）"),
            });
        }
    }
}

// =====================
// 不变量 3：Phi 一致性（仅 Ssa；7b.1a 两条 + 7b.1b 三条残项）
// =====================

/// `Phi` 的可判定约束（04 §目标设计「`Phi` 作为新 `Instruction` 变体」）。
///
/// 7b.1a 已落：
/// 1. `dst` 必为 `Operand::Value`——Phi 定义的是 SSA 值，不是存储槽；
/// 2. `incoming` 按前驱 block label 严格升序——构造方保证、渲染与校验共用
///    同一规范形态（升序也是 `normalize` 的前驱排序规则）。
///
/// 7b.1b 追加（本函数）：
/// 3. `incoming.len() == 该块前驱数`（前驱集从 `BasicBlock.successors` 反推，块级）；
/// 4. `incoming` 的 label 集合 == 该块前驱集合（计数相等时才查，避免同一缺陷双报）；
/// 5. 逐输入类型一致（`values` 值表提供类型；见 [`check_phi_incoming_types`]）。
///
/// 块级前驱与指令级 CFG 是两套坐标（文件头「现实语义记录」第 1 条）：`Phi`
/// 的 incoming 键是块 label，故这里只认 `BasicBlock.successors`。生产侧自
/// 7b.3 起由 `ir::ir::partition_blocks` 按块尾终结指令与布局真实填充，
/// 因此前驱数与前驱集合两条判据在真实 IR 上有数据。
fn check_phi_structure(
    view: &FnView<'_>,
    out: &mut Vec<Violation>,
) {
    for (i, instr) in view.instrs.iter().enumerate() {
        let Instruction::Phi { dst, incoming, .. } = instr else {
            continue;
        };
        if !matches!(dst, Operand::Value(_)) {
            out.push(Violation {
                invariant: InvariantKind::PhiConsistency,
                function: view.name.clone(),
                instruction: Some(i),
                message: format!("Phi 的 dst 必须是 Operand::Value（SSA 值），实际 {dst:?}"),
            });
        }
        let labels: Vec<usize> = incoming.iter().map(|(label, _)| *label).collect();
        if !labels.windows(2).all(|w| w[0] < w[1]) {
            out.push(Violation {
                invariant: InvariantKind::PhiConsistency,
                function: view.name.clone(),
                instruction: Some(i),
                message: format!("Phi 的 incoming 未按前驱 label 严格升序：{labels:?}"),
            });
        }

        // 无基本块结构（模块 init 指令流）：前驱集不可判定——显式报，不静默跳过。
        let Some(&block_label) = view.block_labels.get(i) else {
            out.push(Violation {
                invariant: InvariantKind::PhiConsistency,
                function: view.name.clone(),
                instruction: Some(i),
                message: "Phi 出现在无基本块结构的指令流（模块 init），前驱集不可判定".to_string(),
            });
            continue;
        };
        let predecessors = view.predecessors.get(&block_label);
        let pred_count = predecessors.map_or(0, HashSet::len);
        if labels.len() != pred_count {
            let mut preds: Vec<usize> = predecessors
                .map(|set| set.iter().copied().collect())
                .unwrap_or_default();
            preds.sort_unstable();
            out.push(Violation {
                invariant: InvariantKind::PhiConsistency,
                function: view.name.clone(),
                instruction: Some(i),
                message: format!(
                    "Phi 的 incoming 数 {} != 块 {block_label} 的前驱数 {pred_count}（前驱={preds:?}）",
                    labels.len()
                ),
            });
        } else {
            let incoming_set: HashSet<usize> = labels.iter().copied().collect();
            let pred_set: HashSet<usize> = predecessors.cloned().unwrap_or_default();
            if incoming_set != pred_set {
                let mut incoming_sorted = labels.clone();
                incoming_sorted.sort_unstable();
                let mut preds: Vec<usize> = pred_set.iter().copied().collect();
                preds.sort_unstable();
                out.push(Violation {
                    invariant: InvariantKind::PhiConsistency,
                    function: view.name.clone(),
                    instruction: Some(i),
                    message: format!(
                        "Phi 的 incoming label 集合 {incoming_sorted:?} != 块 {block_label} 的前驱集合 {preds:?}"
                    ),
                });
            }
        }
        check_phi_incoming_types(view, i, incoming, out);
    }
}

/// `Phi` 的逐输入类型一致（7b.1b 第 5 条）：04 §三条构造约束之三
/// 「所有 incoming 值的 MonoType 相同」。
///
/// 类型来源 = `FunctionIR::values`（`Operand::Value(v) => values[v].ty`）。类型
/// 不可得的三种形态按文件头「现实语义记录」第 3 条处理，**显式登记、不静默放行**：
/// - `values` 为空表（本批生产恒空）：值表未填充 ⇒ 类型判定依据不存在，不判定；
/// - `values` 非空而 ValueId 越界：无值信息 ⇒ 报 `PhiConsistency`（ValueId 越界）；
/// - 非 `Value` 操作数（Const/Local/Arg/Global）：无 values 类型源 ⇒ 不判定。
fn check_phi_incoming_types(
    view: &FnView<'_>,
    at: usize,
    incoming: &[(usize, Operand)],
    out: &mut Vec<Violation>,
) {
    if view.values.is_empty() {
        return;
    }
    let mut typed: Vec<(&usize, &MonoType)> = Vec::new();
    for (label, operand) in incoming {
        let Operand::Value(id) = operand else {
            continue;
        };
        match view.values.get(id.0 as usize) {
            Some(info) => typed.push((label, &info.ty)),
            None => out.push(Violation {
                invariant: InvariantKind::PhiConsistency,
                function: view.name.clone(),
                instruction: Some(at),
                message: format!(
                    "Phi incoming(label {label}) 的 ValueId({}) 越界——values 表长 {}，无值信息可判定类型",
                    id.0,
                    view.values.len()
                ),
            }),
        }
    }
    let Some((first_label, first_ty)) = typed.first() else {
        return;
    };
    if let Some((label, ty)) = typed.iter().find(|(_, ty)| !types_compatible(first_ty, ty)) {
        out.push(Violation {
            invariant: InvariantKind::PhiConsistency,
            function: view.name.clone(),
            instruction: Some(at),
            message: format!(
                "Phi incoming 类型不一致：label {label} 为 {ty:?}，与 label {first_label} 的 {first_ty:?} 不相容"
            ),
        });
    }
}

// =====================
// 不变量 6：类型一致
// =====================

/// 相容判定（文件头「现实语义记录」第 3 条）：相等 / Never 底部 /
/// 数值宽度族互容 / TypeVar·TypeRef 信息不足放行。
fn types_compatible(
    a: &MonoType,
    b: &MonoType,
) -> bool {
    if a == b {
        return true;
    }
    if matches!(a, MonoType::Never) || matches!(b, MonoType::Never) {
        return true;
    }
    if matches!(a, MonoType::Int(_)) && matches!(b, MonoType::Int(_)) {
        return true;
    }
    if matches!(a, MonoType::Float(_)) && matches!(b, MonoType::Float(_)) {
        return true;
    }
    matches!(a, MonoType::TypeVar(_) | MonoType::TypeRef(_))
        || matches!(b, MonoType::TypeVar(_) | MonoType::TypeRef(_))
}

fn const_type(c: &ConstValue) -> Option<MonoType> {
    c.mono_type()
}

/// 可信类型源（2026-10-06 语料实测登记，文件头「现实语义记录」）：
/// lower **从不写 LocalSlot.ty**（lower/scope.rs 的 register_local / take_cur_locals 只写 name 或占位），globals
/// 无注解时同样是 Int(64) 占位（lower/scope.rs 的 register_local）——槽位类型表整体不可信。
/// 仅存的可信源：参数槽（函数签名的 params，经注解转换）与常量字面量。
/// 返回 owned 值：参数类型 clone，常量类型现场构造（校验器不在热路径）。
fn operand_type(
    op: &Operand,
    view: &FnView<'_>,
    _ir: &ModuleIR,
) -> Option<MonoType> {
    match op {
        Operand::Local(n) => view.param_slot_type(*n).cloned(),
        Operand::Const(c) => const_type(c),
        // Global：无注解占位不可信；Arg / Value：无类型表可查
        _ => None,
    }
}

fn check_type_consistency(
    view: &FnView<'_>,
    ir: &ModuleIR,
    reachable: &HashSet<usize>,
    out: &mut Vec<Violation>,
) {
    for (i, instr) in view.instrs.iter().enumerate() {
        if !reachable.contains(&i) {
            continue;
        }
        let ty_of = |op: &Operand| operand_type(op, view, ir);
        let report = |at: usize, msg: String, out: &mut Vec<Violation>| {
            out.push(Violation {
                invariant: InvariantKind::TypeConsistency,
                function: view.name.clone(),
                instruction: Some(at),
                message: msg,
            });
        };
        match *instr {
            Instruction::Move { dst, src, .. }
            | Instruction::Load { dst, src, .. }
            | Instruction::Store { dst, src, .. } => {
                if let (Some(d), Some(s)) = (ty_of(dst), ty_of(src)) {
                    if !types_compatible(&d, &s) {
                        report(i, format!("dst 槽类型 {d:?} 与 src 类型 {s:?} 不相容"), out);
                    }
                }
            }
            Instruction::Add { dst, lhs, rhs, .. }
            | Instruction::Sub { dst, lhs, rhs, .. }
            | Instruction::Mul { dst, lhs, rhs, .. }
            | Instruction::Div { dst, lhs, rhs, .. }
            | Instruction::Mod { dst, lhs, rhs, .. }
            | Instruction::And { dst, lhs, rhs, .. }
            | Instruction::Or { dst, lhs, rhs, .. }
            | Instruction::Xor { dst, lhs, rhs, .. }
            | Instruction::Shl { dst, lhs, rhs, .. }
            | Instruction::Shr { dst, lhs, rhs, .. }
            | Instruction::Sar { dst, lhs, rhs, .. } => {
                let lt = ty_of(lhs);
                let rt = ty_of(rhs);
                if let (Some(l), Some(r)) = (&lt, &rt) {
                    if !types_compatible(l, r) {
                        report(i, format!("双端类型不相容：lhs {l:?} vs rhs {r:?}"), out);
                        continue;
                    }
                }
                if let (Some(d), Some(l)) = (ty_of(dst), lt) {
                    if !types_compatible(&d, &l) {
                        report(i, format!("dst 槽类型 {d:?} 与运算类型 {l:?} 不相容"), out);
                    }
                }
            }
            Instruction::Eq { lhs, rhs, .. }
            | Instruction::Ne { lhs, rhs, .. }
            | Instruction::Lt { lhs, rhs, .. }
            | Instruction::Le { lhs, rhs, .. }
            | Instruction::Gt { lhs, rhs, .. }
            | Instruction::Ge { lhs, rhs, .. } => {
                if let (Some(l), Some(r)) = (ty_of(lhs), ty_of(rhs)) {
                    if !types_compatible(&l, &r) {
                        report(
                            i,
                            format!("比较双端类型不相容：lhs {l:?} vs rhs {r:?}"),
                            out,
                        );
                    }
                }
            }
            Instruction::Neg { dst, src, .. } | Instruction::Not { dst, src, .. } => {
                if let (Some(d), Some(s)) = (ty_of(dst), ty_of(src)) {
                    if !types_compatible(&d, &s) {
                        report(i, format!("dst 槽类型 {d:?} 与 src 类型 {s:?} 不相容"), out);
                    }
                }
            }
            Instruction::Ret {
                value: Some(value), ..
            } => {
                if let Some(ret_ty) = view.return_type {
                    if let Some(v) = ty_of(value) {
                        // Void 返回类型容忍任意尾值（尾表达式求值后由调用方丢弃
                        // 是现行语义；return_type 为 Never 时经 compat 底部规则放行）
                        if !types_compatible(ret_ty, &v) && !matches!(ret_ty, MonoType::Void) {
                            report(
                                i,
                                format!("Ret 值类型 {v:?} 与函数返回类型 {ret_ty:?} 不相容"),
                                out,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

// =====================
// 不变量 7：内层隔离
// =====================

/// 一张槽位表里出现的生成帧（`FrameId` → 该帧登记的槽位下标），按帧号有序。
///
/// `None`（无来源：手工 IR / mono 与 codegen 合成的槽）不入表——「未解析」不是
/// 「本帧」，把它算成一个帧只会制造假阳性。
fn slot_frames(locals: &[LocalSlot]) -> BTreeMap<FrameId, Vec<usize>> {
    let mut by_frame: BTreeMap<FrameId, Vec<usize>> = BTreeMap::new();
    for (idx, slot) in locals.iter().enumerate() {
        if let Some(frame) = slot.origin {
            by_frame.entry(frame).or_default().push(idx);
        }
    }
    by_frame
}

/// 不变量 7：内层隔离（WBS 2.1.4 起**按槽身份**判定）。
///
/// 判据：一个函数体（或模块初始化序列）的槽位表里出现的生成帧**至多一种**。
/// 两种即该表被两个生成期共用——lower 的 `cur_locals` save/restore 失败，
/// 内层函数继承了外层的槽位（或外层被内层覆盖）。
///
/// 为什么不看名字、也不看 `MakeClosure`：名字是源码巧合（同名不同绑定合法），
/// `MakeClosure` 是函数值表示（不是嵌套关系）。身份只看 `LocalSlot::origin`。
///
/// **已知边界：单一外来帧的污染不报红**。判据只问「这张表里有几种帧」，所以
/// 「内层整表继承外层帧」（表里只剩外层那一种）与「父表被子表整表替换」这两种
/// **单帧污染**形态不会被抓到——要抓它们需要「表的归属」本身，而不只是「表内是否自洽」。
/// 旧判据在这两种形态会红，但那份红只来自名字巧合（对合法程序同样误报），因此这是
/// 理论边界、不是相对旧判据的回归；两种泄漏方向用真实管线变异注入时都被本判据抓住
/// （全语料 61 个文件转红）。判别力证据见
/// `docs/src/dev/architecture/09a-wbs-history.md` §P2 2.1.4。
///
/// **`FrameId` 的作用域**：帧号**每个生成器（每编译单元）内唯一，不是全局唯一**——
/// `driver` 的每个单元各建一个 `AstToIrGenerator`，单元之间帧号会重复（同一个单元里
/// `main` 与嵌入的 `std.json.parse` 都显示帧 1）。本判据只在「同一张表内比较」的范围内
/// 使用它，不受影响；不要把它当跨单元的全局 id 用。
fn check_inner_isolation(
    ir: &ModuleIR,
    out: &mut Vec<Violation>,
) {
    let mut report = |function: &str, locals: &[LocalSlot]| {
        let by_frame = slot_frames(locals);
        if by_frame.len() <= 1 {
            return;
        }
        let detail = by_frame
            .iter()
            .map(|(frame, slots)| {
                let names: Vec<String> = slots
                    .iter()
                    .map(|&i| {
                        let name = locals[i].name.as_deref().unwrap_or("<temp>");
                        format!("{name}@{i}")
                    })
                    .collect();
                format!("帧{} 的槽 {:?}", frame.0, names)
            })
            .collect::<Vec<_>>()
            .join("；");
        out.push(Violation {
            invariant: InvariantKind::InnerIsolation,
            function: function.to_string(),
            instruction: None,
            message: format!(
                "同一张槽位表混入 {} 个生成帧（cur_locals save/restore 契约违反）：{detail}",
                by_frame.len()
            ),
        });
    };

    for func in &ir.functions {
        if func.is_type_decl() {
            continue;
        }
        report(&func.name, func.locals());
    }
    // 模块初始化序列与函数体同一判据：它是帧 0 的槽位表。
    report("__module_init", &ir.init_locals);
}
