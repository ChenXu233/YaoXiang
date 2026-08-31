//! 所有权证明层（Layer 1）
//!
//! RFC-009a: 令牌生命期分析——基于霍尔证明管道。
//!
//! 品牌树追踪令牌派生关系。冲突判断依赖两条规则：
//! 同源 + 至少一方为写。前缀关系仅用于级联释放和错误信息。
//!
//! § 品牌树: 令牌派生关系与冲突检测
//! § 系统谓词清单: 5 种命题（borrow_conflict / use_after_move / use_after_drop / double_drop / mut_violation）
//! § 快速通道: 反向 BFS 活性分析
//! § 慢速通道: SMT 逻辑切断

use std::collections::{HashMap, HashSet};
use crate::util::span::Span;
use super::super::proof::context::ProofContext;
use super::super::proof::verdict::ProofResult;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, UnOp};

use crate::frontend::core::typecheck::proof::smt::ast::{SMTSort, SMTResult};
use crate::frontend::core::typecheck::proof::smt::translate::translate_constraint;
#[cfg(not(target_arch = "wasm32"))]
use crate::frontend::core::typecheck::proof::smt::z3_backend::Z3Backend;

// ── ReleasePlan ───────────────────────────────────────────

/// NLL 精确释放计划
///
/// key = 最后使用位置的 Span，value = 在该位置需要 Drop 的变量名列表（LIFO 顺序）
#[derive(Debug, Clone, Default)]
pub struct ReleasePlan {
    pub drops: HashMap<Span, Vec<String>>,
}

/// 参数的所有权语义（从函数签名推导）
#[derive(Clone, Copy)]
#[allow(dead_code)]
enum ParamOwnership {
    /// T → 所有权转移
    Move,
    /// &T → 创建 ReadToken
    ReadBorrow,
    /// &mut T → 创建 WriteToken + 待定冲突检查
    WriteBorrow,
}

// ── BrandId ───────────────────────────────────────────────

/// 编译期唯一的令牌品牌标识。
///
/// `#0`、`#1` 为独立根令牌。`#0.x` 从 `#0` 派生（字段访问）。
/// 前缀比较用于级联释放和错误信息，不用于冲突判断。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BrandId(String);

impl BrandId {
    pub fn root(n: u64) -> Self {
        Self(format!("#{}", n))
    }

    pub fn derive_field(
        &self,
        field: &str,
    ) -> Self {
        Self(format!("{}.{}", self.0, field))
    }

    /// `self` 是否是 `other` 的前缀（`other` 从 `self` 派生）。
    pub fn is_prefix_of(
        &self,
        other: &BrandId,
    ) -> bool {
        other.0.starts_with(&self.0)
            && (other.0.len() == self.0.len() || other.0.as_bytes()[self.0.len()] == b'.')
    }

    /// 返回数字根 ID（`#0.x` → `#0`）。
    pub fn root_id(&self) -> &str {
        self.0.split('.').next().unwrap_or(&self.0)
    }
}

impl std::fmt::Display for BrandId {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── TokenKind ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    ReadToken,
    WriteToken,
}

impl TokenKind {
    pub fn is_read(self) -> bool {
        matches!(self, Self::ReadToken)
    }
    pub fn is_write(self) -> bool {
        matches!(self, Self::WriteToken)
    }
}

// ── BrandNode ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BrandNode {
    pub id: BrandId,
    pub kind: TokenKind,
    /// 源码级变量名（非 Operand）
    pub source_var: String,
    pub parent: Option<BrandId>,
    pub children: HashSet<BrandId>,
    /// 消费该令牌的 CFG 节点索引。
    pub consumers: HashSet<usize>,
    /// ReadToken 冻结期间的活跃副本数。
    pub ref_count: usize,
    /// 活性区间 [created_at, last_use] 的 created_at 端点（RFC-009a）：
    /// 令牌出生的 CFG 节点。fast_path_check 反向 BFS 以此为屏障——出生之前
    /// 令牌不存在，活性不得向更早蔓延（#290 F1：此前缺失，读令牌活性向过去
    /// 无限延伸 → 后文取读令牌误伤前文已释放的 &mut，E2018 回溯误报）。
    pub birth_node: usize,
    /// #315：瞬态令牌——调用实参/方法接收者自动借用产生（§12.4"令牌随调用
    /// 结束释放"）。瞬态令牌的消费者集只含其调用节点，不被后文同名变量的
    /// 使用延长（add_consumer_for_var / 写类竞争声明均跳过）——否则后置读
    /// 会把它区间拉长穿越中间的写点，伪造 E2018（§12.4 链 q.sum→q.shift→q.sum）。
    /// var 绑定借用（v = &p）为 false，活到作用域结束（D5 裁决）。
    pub transient: bool,
}

impl BrandNode {
    fn new(
        id: BrandId,
        kind: TokenKind,
        source_var: String,
        birth_node: usize,
        transient: bool,
    ) -> Self {
        Self {
            id,
            kind,
            source_var,
            parent: None,
            children: HashSet::new(),
            consumers: HashSet::new(),
            ref_count: 1,
            birth_node,
            transient,
        }
    }
}

// ── BrandTree ─────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct BrandTree {
    nodes: HashMap<BrandId, BrandNode>,
    next_id: u64,
}

impl BrandTree {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            next_id: 0,
        }
    }

    // ── 创建 ──────────────────────────────────────────

    pub fn create_read_token(
        &mut self,
        source: String,
        birth_node: usize,
        transient: bool,
    ) -> BrandId {
        let id = BrandId::root(self.next_id);
        self.next_id += 1;
        self.nodes.insert(
            id.clone(),
            BrandNode::new(
                id.clone(),
                TokenKind::ReadToken,
                source,
                birth_node,
                transient,
            ),
        );
        id
    }

    pub fn create_write_token(
        &mut self,
        source: String,
        birth_node: usize,
        transient: bool,
    ) -> BrandId {
        let id = BrandId::root(self.next_id);
        self.next_id += 1;
        self.nodes.insert(
            id.clone(),
            BrandNode::new(
                id.clone(),
                TokenKind::WriteToken,
                source,
                birth_node,
                transient,
            ),
        );
        id
    }

    // ── 派生 ──────────────────────────────────────────

    /// 从父令牌派生字段访问令牌。返回子令牌 ID。
    pub fn derive_field(
        &mut self,
        parent_id: &BrandId,
        field: &str,
        birth_node: usize,
    ) -> Option<BrandId> {
        let parent = self.nodes.get(parent_id)?;
        let child_id = parent_id.derive_field(field);
        let source_var = parent.source_var.clone();
        let kind = parent.kind;
        let transient = parent.transient;

        let mut child = BrandNode::new(child_id.clone(), kind, source_var, birth_node, transient);
        child.parent = Some(parent_id.clone());

        self.nodes.insert(child_id.clone(), child);
        self.nodes
            .get_mut(parent_id)
            .unwrap()
            .children
            .insert(child_id.clone());

        Some(child_id)
    }

    // ── 消费者 ────────────────────────────────────────

    pub fn add_consumer(
        &mut self,
        id: &BrandId,
        node_idx: usize,
    ) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.consumers.insert(node_idx);
        }
    }

    /// #315：切换令牌的瞬态标记。`v = &p` 绑定到引用变量时由瞬态（借用表达式
    /// 默认）转为 var 绑定（活到作用域结束，D5 裁决）。
    pub fn set_transient(
        &mut self,
        id: &BrandId,
        transient: bool,
    ) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.transient = transient;
        }
    }

    pub fn consumers(
        &self,
        id: &BrandId,
    ) -> HashSet<usize> {
        self.nodes
            .get(id)
            .map(|n| n.consumers.clone())
            .unwrap_or_default()
    }

    // ── 引用计数 ──────────────────────────────────────

    pub fn inc_ref(
        &mut self,
        id: &BrandId,
    ) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.ref_count += 1;
        }
    }

    pub fn dec_ref(
        &mut self,
        id: &BrandId,
    ) {
        if let Some(node) = self.nodes.get_mut(id) {
            if node.ref_count > 0 {
                node.ref_count -= 1;
            }
        }
    }

    // ── 查询 ──────────────────────────────────────────

    pub fn get(
        &self,
        id: &BrandId,
    ) -> Option<&BrandNode> {
        self.nodes.get(id)
    }

    pub fn root_tokens(&self) -> Vec<&BrandId> {
        self.nodes
            .values()
            .filter(|n| n.parent.is_none())
            .map(|n| &n.id)
            .collect()
    }

    // ── 冲突判断 ──────────────────────────────────────

    /// 判断两个令牌是否冲突。
    ///
    /// 两条规则（RFC-009a 修正）：
    /// 1. 同源（同一个 source_var）
    /// 2. 至少一方为 WriteToken
    pub fn conflicts(
        &self,
        a: &BrandId,
        b: &BrandId,
    ) -> bool {
        let node_a = match self.nodes.get(a) {
            Some(n) => n,
            None => return false,
        };
        let node_b = match self.nodes.get(b) {
            Some(n) => n,
            None => return false,
        };

        if node_a.source_var != node_b.source_var {
            return false;
        }

        node_a.kind.is_write() || node_b.kind.is_write()
    }

    /// 获取所有与给定令牌冲突的活跃令牌。
    pub fn conflicting_with(
        &self,
        id: &BrandId,
    ) -> Vec<&BrandId> {
        self.nodes
            .keys()
            .filter(|other| *other != id && self.conflicts(id, other))
            .collect()
    }

    /// 移除令牌及其所有派生子令牌（级联删除）。
    pub fn remove(
        &mut self,
        id: &BrandId,
    ) {
        if let Some(node) = self.nodes.remove(id) {
            for child in node.children.clone() {
                self.remove(&child);
            }
            if let Some(parent_id) = &node.parent {
                if let Some(parent) = self.nodes.get_mut(parent_id) {
                    parent.children.remove(id);
                }
            }
        }
    }
}

// ── 控制流图（CFG）—— RFC-009a §快速通道 ──────────────

/// CFG 边类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// 普通前向边
    Normal,
    /// break 边（结构切断——反向 BFS 不穿越）
    Break,
    /// 回边（循环——需要路径条件或 SMT 逻辑切断）
    BackEdge,
}

/// 控制流图中的节点
#[derive(Debug, Clone)]
pub struct CfgNode {
    pub id: usize,
    /// 后继节点及边类型
    pub successors: Vec<(usize, EdgeKind)>,
    /// 前驱节点（用于反向 BFS）
    pub predecessors: Vec<usize>,
    /// 该节点的路径条件（if guard / while cond / match pattern）。
    /// #312：存真实 ConstExpr（SMT 可消费）——此前存 AST Debug 文本，
    /// 只能人读不能进 Z3，回边切断查询被迫用占位符恒切断。
    pub path_condition: Option<ConstExpr>,
    /// 本节点的变量操作序列（#264：walk 阶段记录，数据流分析阶段消费）
    pub ops: Vec<VarOp>,
}

/// 变量操作（#264 数据流分析的最小指令集）
///
/// walk 阶段把变量状态的变更/读取记录到 CFG 节点，walk 结束后
/// `analyze_var_flow` 在 CFG 上做前向数据流（汇合 meet + 循环不动点），
/// 对每个 Read 判定 Move/Drop 违例。
#[derive(Debug, Clone)]
pub enum VarOp {
    /// 声明/重绑定 → Alive（覆盖旧状态，Python 风格重声明）
    Declare { var: String },
    /// move 转移 → Moved
    Move { var: String },
    /// 读取检查点（分析时判定 E2014/E2018）
    Read { var: String, span: Span },
    /// 作用域退出：Alive → Dropped（#264：Dropped 语义 = 作用域外不可见）
    Drop { var: String, span: Span },
}

/// 变量状态格：Dropped(2) > Moved(1) > Alive(0)，汇合取 max（保守）
/// 保守序：越不可用越保守；单方存在的变量保留原值（分支内新变量已被 Drop）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VarState {
    Alive,
    Moved,
    Dropped,
}

/// 函数体的控制流图
///
/// 线性代码：节点 0→1→2→...→N
/// if/else：split 到各分支，分支末尾汇合
/// loop：回边从循环尾回到循环头
#[derive(Debug, Default)]
pub struct ControlFlowGraph {
    pub nodes: Vec<CfgNode>,
    /// 入口节点索引
    pub entry: usize,
    /// 出口节点索引
    pub exit: usize,
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            entry: 0,
            exit: 0,
        }
    }

    /// 添加节点，返回节点索引
    pub fn add_node(
        &mut self,
        path_condition: Option<ConstExpr>,
    ) -> usize {
        let id = self.nodes.len();
        self.nodes.push(CfgNode {
            id,
            successors: Vec::new(),
            predecessors: Vec::new(),
            path_condition,
            ops: Vec::new(),
        });
        id
    }

    /// 添加边 from → to
    pub fn add_edge(
        &mut self,
        from: usize,
        to: usize,
        kind: EdgeKind,
    ) {
        self.nodes[from].successors.push((to, kind));
        if kind != EdgeKind::Break {
            // break 边不作为反向 BFS 的前驱（结构切断）
            self.nodes[to].predecessors.push(from);
        }
    }
}

// ── 快速通道：反向 BFS（RFC-009a §快速通道） ─────────────

/// 快速通道结果
#[derive(Debug)]
pub enum FastPathResult {
    Safe,
    Unsafe { live_tokens: Vec<BrandId> },
}

/// 反向 BFS 活性分析（覆盖 95%+ 场景）。
///
/// 算法（RFC-009a §反向 BFS 活性分析）：
/// 1. 收集所有与 write_token 冲突的令牌
/// 2. 从每个冲突令牌的消费者出发，反向 BFS
/// 3. break 边切断（不穿越——add_edge 时已排除出 predecessors）
/// 4. 回边 → SMT 逻辑切断
/// 5. write_node ∈ unsafe → Unsafe
pub fn fast_path_check(
    tree: &BrandTree,
    cfg: &ControlFlowGraph,
    write_token: &BrandId,
    write_node: usize,
) -> FastPathResult {
    let conflicting = tree.conflicting_with(write_token);
    if conflicting.is_empty() {
        return FastPathResult::Safe;
    }

    let mut unsafe_nodes: HashSet<usize> = HashSet::new();

    // #290 F1：逐冲突令牌独立反向 BFS。活性区间 [created_at, last_use] 的
    // created_at 端点 = birth_node 屏障——标记出生节点但不再向其前驱蔓延
    //（出生之前令牌不存在）。此前无屏障，读令牌活性向过去无限延伸：
    // 后文取读令牌会误伤前文已释放的 &mut（回溯误报 E2018）。
    // 早期退出用局部 visited——跨令牌共享会因另一令牌先标记过某节点而
    // 截断本令牌的蔓延，可能漏掉写点的真实冲突（假阴性）。
    for conflict_id in &conflicting {
        let birth = tree
            .get(conflict_id)
            .map(|n| n.birth_node)
            .unwrap_or(usize::MAX);
        let mut visited: HashSet<usize> = HashSet::new();
        let mut queue: Vec<usize> = tree
            .consumers(conflict_id)
            .into_iter()
            .filter(|c| *c < cfg.nodes.len())
            .collect();

        while let Some(cur) = queue.pop() {
            if visited.contains(&cur) {
                continue;
            }
            visited.insert(cur);
            if cur >= cfg.nodes.len() {
                continue;
            }

            // 出生屏障：该节点之后（更早方向）令牌尚未出生
            if cur == birth {
                continue;
            }

            for &pred in &cfg.nodes[cur].predecessors {
                // 结构切断：break 边不会出现在 predecessors 中（add_edge 已过滤）

                let is_back_edge = cfg.nodes[pred]
                    .successors
                    .iter()
                    .any(|(succ, kind)| *succ == cur && *kind == EdgeKind::BackEdge);

                if is_back_edge {
                    // #312：路径条件取写节点自身条件（RFC-009a 勘误 2026-08-17：
                    // 判定目标为写节点），与循环头条件做蕴含判定。
                    // 任一缺失（无守卫的循环体 / for 循环 / loop）或 SMT 无法证明
                    // 蕴含 → 回边穿越 → 保守拒绝（SMT 是精度层，不是 soundness 依赖）
                    let write_cond = cfg.nodes[write_node].path_condition.as_ref();
                    let loop_cond = cfg.nodes[cur].path_condition.as_ref();
                    if let (Some(pc), Some(lc)) = (write_cond, loop_cond) {
                        if smt_cut(pc, lc) {
                            continue; // 逻辑切断
                        }
                    }
                }

                if !visited.contains(&pred) {
                    queue.push(pred);
                }
            }
        }
        unsafe_nodes.extend(visited);
    }

    if unsafe_nodes.contains(&write_node) {
        let live_tokens: Vec<BrandId> = conflicting
            .into_iter()
            .filter(|id| tree.consumers(id).iter().any(|c| unsafe_nodes.contains(c)))
            .cloned()
            .collect();
        FastPathResult::Unsafe { live_tokens }
    } else {
        FastPathResult::Safe
    }
}

// ── 慢速通道：SMT 逻辑切断（RFC-009a §慢速通道） ─────────

/// SMT 逻辑切断：判定 `write_path_cond ⇒ !loop_cond`
///
/// 仅在回边 + 双侧路径条件（写节点自身条件 + 循环头条件）齐备时调用。
/// RFC-009a 勘误（2026-08-17）：SMT 是精度层而非 soundness 依赖——
/// 蕴含无法证明（Sat/Unknown/Z3 不可用）一律返回 false = 回边穿越 = 保守拒绝。
///
/// 构造：目标 `!(path ∧ loop)`，Unsat = 蕴含成立 → 切断。
/// #312：此前用 `NamedVar("path_cond")` 占位符并假设其为真 → 恒 Unsat →
/// 恒切断 → 回边永不穿越 → 循环内借用写静默放行。现用真实 ConstExpr
/// 条件构造查询，assumptions 为空。
pub(crate) fn smt_cut(
    path_cond: &ConstExpr,
    loop_cond: &ConstExpr,
) -> bool {
    // wasm 模式下 Z3 不可用，保守不切断（回边穿越）
    #[cfg(target_arch = "wasm32")]
    return false;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let conj = ConstExpr::BinOp {
            op: BinOp::And,
            left: Box::new(path_cond.clone()),
            right: Box::new(loop_cond.clone()),
        };
        // 目标：!(path ∧ loop)。Unsat = 在空假设下该蕴含恒成立 → 切断；
        // Sat = 存在「写路径与循环重入共存」的模型 → 不切断。
        let constraint = ConstExpr::UnOp {
            op: UnOp::Not,
            expr: Box::new(conj.clone()),
        };

        let mut var_sorts: HashMap<String, SMTSort> = HashMap::new();
        collect_var_sorts(&conj, &mut var_sorts);

        let commands = translate_constraint(&constraint, &[], &var_sorts);

        let backend = match Z3Backend::new() {
            Ok(b) => b,
            Err(_) => return false, // Z3 不可用 → 保守不切断（回边穿越）
        };

        matches!(backend.solve(&commands, 100), SMTResult::Unsat)
    } // cfg(not(target_arch = "wasm32"))
}

/// 收集 ConstExpr 中出现的 NamedVar 及其 SMT 排序。
///
/// 排序推断：算术/比较运算的操作数为数值（Int）；其余上下文默认 Bool。
/// 判据保守——排序错误的查询会求解失败 → Unknown → 回边穿越（sound 方向）。
fn collect_var_sorts(
    expr: &ConstExpr,
    sorts: &mut HashMap<String, SMTSort>,
) {
    const NUMERIC_OPS: &[BinOp] = &[
        BinOp::Add,
        BinOp::Sub,
        BinOp::Mul,
        BinOp::Div,
        BinOp::Mod,
        BinOp::Lt,
        BinOp::Le,
        BinOp::Gt,
        BinOp::Ge,
    ];

    fn collect_numeric(
        expr: &ConstExpr,
        sorts: &mut HashMap<String, SMTSort>,
    ) {
        match expr {
            ConstExpr::NamedVar(name) => {
                sorts.insert(name.clone(), SMTSort::Int);
            }
            ConstExpr::BinOp { left, right, .. } => {
                collect_numeric(left, sorts);
                collect_numeric(right, sorts);
            }
            ConstExpr::UnOp { expr, .. } => collect_numeric(expr, sorts),
            ConstExpr::Call { args, .. } => {
                for arg in args {
                    collect_numeric(arg, sorts);
                }
            }
            ConstExpr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                collect_numeric(condition, sorts);
                collect_numeric(then_branch, sorts);
                collect_numeric(else_branch, sorts);
            }
            ConstExpr::Range { start, end } => {
                collect_numeric(start, sorts);
                collect_numeric(end, sorts);
            }
            _ => {}
        }
    }

    match expr {
        ConstExpr::NamedVar(_) => {}
        ConstExpr::Lit(_) | ConstExpr::Var(_) => {}
        ConstExpr::BinOp { op, left, right } => {
            if NUMERIC_OPS.contains(op) {
                collect_numeric(left, sorts);
                collect_numeric(right, sorts);
            } else {
                collect_var_sorts(left, sorts);
                collect_var_sorts(right, sorts);
            }
        }
        ConstExpr::UnOp { expr, .. } => collect_var_sorts(expr, sorts),
        ConstExpr::Call { args, .. } => {
            for arg in args {
                collect_var_sorts(arg, sorts);
            }
        }
        ConstExpr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            // 条件与两分支递归：两分支可能是数值或布尔，交给各自子树判据
            collect_var_sorts(condition, sorts);
            collect_var_sorts(then_branch, sorts);
            collect_var_sorts(else_branch, sorts);
        }
        ConstExpr::Range { start, end } => {
            collect_numeric(start, sorts);
            collect_numeric(end, sorts);
        }
    }
}

// ── 系统谓词生成器（RFC-009a §系统谓词清单） ────────────

/// 借用冲突谓词：`forall t ∈ conflicting(v): dead_at(t, node)`
pub fn emit_borrow_predicate(
    tree: &BrandTree,
    cfg: &ControlFlowGraph,
    token: &BrandId,
    node_idx: usize,
    span: Span,
) -> ProofResult {
    match fast_path_check(tree, cfg, token, node_idx) {
        FastPathResult::Safe => ProofResult::Proved,
        FastPathResult::Unsafe { live_tokens } => {
            ProofResult::Disproved(super::super::proof::verdict::DisproofModel {
                kind: super::super::proof::verdict::DisproofKind::BorrowConflict,
                assignments: vec![
                    ("token".into(), format!("{}", token)),
                    (
                        "live_tokens".into(),
                        live_tokens
                            .iter()
                            .map(|t| format!("{}", t))
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                ],
                constraint: format!("{} 的冲突令牌仍存活", token),
                span: Some(span),
                predicate_span: None,
            })
        }
    }
}

/// Move 后使用谓词：`¬moved(v)`
pub fn emit_move_predicate(
    var_name: &str,
    is_moved: bool,
    span: Span,
) -> ProofResult {
    if is_moved {
        ProofResult::Disproved(super::super::proof::verdict::DisproofModel {
            kind: super::super::proof::verdict::DisproofKind::UseAfterMove,
            assignments: vec![("variable".into(), var_name.into())],
            constraint: format!("{} 已被移动，不可再使用", var_name),
            span: Some(span),
            predicate_span: None,
        })
    } else {
        ProofResult::Proved
    }
}

/// Drop 后使用谓词：`¬dropped(v)`
pub fn emit_drop_predicate(
    var_name: &str,
    is_dropped: bool,
    span: Span,
) -> ProofResult {
    if is_dropped {
        ProofResult::Disproved(super::super::proof::verdict::DisproofModel {
            kind: super::super::proof::verdict::DisproofKind::UseAfterDrop,
            assignments: vec![("variable".into(), var_name.into())],
            constraint: format!("{} 已被释放，不可再使用", var_name),
            span: Some(span),
            predicate_span: None,
        })
    } else {
        ProofResult::Proved
    }
}

/// 双重 Drop 谓词
pub fn emit_double_drop_predicate(
    var_name: &str,
    is_dropped: bool,
    span: Span,
) -> ProofResult {
    if is_dropped {
        ProofResult::Disproved(super::super::proof::verdict::DisproofModel {
            kind: super::super::proof::verdict::DisproofKind::DoubleDrop,
            assignments: vec![("variable".into(), var_name.into())],
            constraint: format!("{} 已被释放，不可重复释放", var_name),
            span: Some(span),
            predicate_span: None,
        })
    } else {
        ProofResult::Proved
    }
}

/// 可变性违规谓词：`is_mut(v)`
pub fn emit_mut_predicate(
    var_name: &str,
    is_mutable: bool,
    span: Span,
) -> ProofResult {
    if !is_mutable {
        ProofResult::Disproved(super::super::proof::verdict::DisproofModel {
            kind: super::super::proof::verdict::DisproofKind::MutViolation,
            assignments: vec![("variable".into(), var_name.into())],
            constraint: format!("{} 不可变，不能赋值", var_name),
            span: Some(span),
            predicate_span: None,
        })
    } else {
        ProofResult::Proved
    }
}

// ── 入口：ProofContext → ProofResult ──────────────────────

/// 检查所有权无冲突（Layer 1 证明管线入口，RFC-009a）
///
/// #265：从无条件 Proved 的桩变为真入口——构造 OwnershipChecker 遍历函数体，
/// 分支守卫在 walk_if/walk_while 中注入 `ctx.assumptions`（FlowSensitiveGamma
/// 的首个消费者），后续系统谓词可携带路径条件送入验证。
///
/// 由 checker.rs 在遍历函数体时调用。
pub fn check_ownership(
    ctx: &mut ProofContext<'_>,
    module: &Module,
    env: &crate::frontend::core::typecheck::environment::TypeEnvironment,
    type_ledger: &HashMap<(usize, String), crate::frontend::core::types::PolyType>,
) -> (Vec<ProofResult>, ReleasePlan, HashSet<String>) {
    let mut checker = OwnershipChecker::new();
    // #265：共享假设栈——walk_if/walk_while 的分支守卫注入 ctx.assumptions
    std::mem::swap(&mut checker.gamma, &mut ctx.assumptions);
    let result = checker.check_module(module, env, type_ledger);
    std::mem::swap(&mut checker.gamma, &mut ctx.assumptions);
    result
}

// ── OwnershipChecker：AST 遍历 ───────────────────────────

use crate::frontend::core::parser::ast::{Expr, Module, Stmt, StmtKind};

/// 赋值/传参/返回的复制语义（SPEC §11.2，#256）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopySemantics {
    /// Dup：&T / ref T——复制令牌，源存活
    Dup,
    /// Linear：&mut T——独占，复制被拒绝（#257）
    Linear,
    /// 原语值类型（Int/Float/Bool/Char）：编译器内置值复制
    ValueCopy,
    /// 默认：所有权转移
    Move,
}

/// 待验证的写操作（遍历完成后排空）
struct PendingWrite {
    token: BrandId,
    node_idx: usize,
    span: Span,
}

/// 所有权检查器——遍历 AST 构建 BrandTree + CFG，执行所有权验证
pub struct OwnershipChecker {
    brand_tree: BrandTree,
    cfg: ControlFlowGraph,
    /// 变量是否声明为 mut（用于可变性违规检测）
    var_mutability: HashMap<String, bool>,
    pending_writes: Vec<PendingWrite>,
    /// 当前 CFG 节点索引（walk 过程中推进）
    current_node: usize,
    /// 当前继承的路径条件（#290 F1b：逐语句节点继承分支/循环守卫，
    /// 守卫写（if 内字段写）的 SMT 切断依赖写节点携带条件）
    current_condition: Option<ConstExpr>,
    /// 当前 AST 片段的源码位置（walk 过程中更新）
    current_span: Span,
    /// CFG 节点 → 源码 Span（build_release_plan 用）
    node_spans: HashMap<usize, Span>,
    /// 作用域栈：每个元素是当前作用域内声明的变量名列表
    /// walk_stmts 进入时 push，退出时 pop 并标记 Alive→Dropped
    scope_vars: Vec<Vec<String>>,
    /// 作用域退出时收集的 Drop 记录（Span, 变量名）
    /// build_release_plan 会将这些与 BrandTree 消费者分析合并
    scope_drops: Vec<(Span, String)>,
    /// 类型环境引用（用裸指针避免生命周期重写；env 生命周期 > checker）
    env: Option<*const crate::frontend::core::typecheck::environment::TypeEnvironment>,
    /// ref 创建的变量（Expr::Ref 的赋值目标）
    ref_vars: HashSet<String>,
    /// 变量类型账本（推断层移交）：(定义语句 span offset, 变量名) → 类型（#256）
    type_ledger: HashMap<(usize, String), crate::frontend::core::types::PolyType>,
    /// 变量名 → 定义语句键（镜像 scope_vars，随作用域 push/pop）
    scope_keys: Vec<HashMap<String, usize>>,
    /// 当前 walk 的语句键（span 起始 offset）
    cur_stmt_key: usize,
    /// spawn 体内使用的 ref 变量（逃逸 → 选 Arc）
    escaped_refs: HashSet<String>,
    /// 当前是否在 spawn 体内
    inside_spawn: bool,
    /// 当前是否在 unsafe 块内
    inside_unsafe: bool,
    /// spawn 块内 ref 变量的依赖图（ref_a → ref_b 表示 a 持有 b 的引用）
    spawn_ref_graph: HashMap<String, HashSet<String>>,
    /// 当前 spawn 块内使用的 ref 变量集合
    current_spawn_refs: HashSet<String>,
    /// 字段赋值记录：(变量名, 字段名, 被赋值的变量名)
    field_assignments: Vec<(String, String, String)>,
    /// #265：流敏感假设栈 Γ——分支守卫在 walk_if/walk_while 中 inject/exit_scope
    gamma: crate::frontend::core::typecheck::proof::assumptions::FlowSensitiveGamma,
    /// #312：WriteBorrow 实参遍历深度——>0 时 add_consumer_for_var 压制。
    /// &mut 实参是对变量的独占写请求而非读取消费，把写节点注册成现有读令牌的
    /// 消费者会让反向 BFS 以写节点为种子自我标记恒 unsafe，路径条件切断全部失效。
    write_borrow_arg_depth: usize,
    /// #312：引用变量 → 其持有的令牌（`view = &p` 把 view 绑到 p 的读令牌）。
    /// 此前透过引用的使用（view.x）按 source_var 匹配不到令牌，消费者从未注册，
    /// 借用活性对「通过引用使用」完全失明——反向 BFS 缺少最关键的种子。
    ref_bindings: HashMap<String, BrandId>,
    /// #312：最近一次 Borrow 表达式创建的令牌（Assign 臂捕获后绑到目标变量）
    last_created_token: Option<BrandId>,
}

impl Default for OwnershipChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl OwnershipChecker {
    pub fn new() -> Self {
        Self {
            brand_tree: BrandTree::new(),
            cfg: ControlFlowGraph::new(),
            var_mutability: HashMap::new(),
            pending_writes: Vec::new(),
            current_node: 0,
            current_condition: None,
            current_span: Span::dummy(),
            write_borrow_arg_depth: 0,
            ref_bindings: HashMap::new(),
            last_created_token: None,
            node_spans: HashMap::new(),
            scope_vars: Vec::new(),
            scope_drops: Vec::new(),
            env: None,
            ref_vars: HashSet::new(),
            type_ledger: HashMap::new(),
            scope_keys: vec![HashMap::new()],
            cur_stmt_key: 0,
            escaped_refs: HashSet::new(),
            inside_spawn: false,
            inside_unsafe: false,
            spawn_ref_graph: HashMap::new(),
            current_spawn_refs: HashSet::new(),
            field_assignments: Vec::new(),
            gamma: crate::frontend::core::typecheck::proof::assumptions::FlowSensitiveGamma::new(),
        }
    }

    /// #265：假设栈只读访问器（测试观测分支守卫进出）
    pub fn gamma(
        &self
    ) -> &crate::frontend::core::typecheck::proof::assumptions::FlowSensitiveGamma {
        &self.gamma
    }

    /// 重置函数级状态
    fn reset(&mut self) {
        self.brand_tree = BrandTree::new();
        self.cfg = ControlFlowGraph::new();
        self.var_mutability.clear();
        self.pending_writes.clear();
        self.node_spans.clear();
        self.scope_vars.clear();
        self.scope_drops.clear();
        self.ref_vars.clear();
        // 注：type_ledger 是模块级数据（check_module 注入），不随函数重置
        self.scope_keys.clear();
        self.scope_keys.push(HashMap::new());
        self.cur_stmt_key = 0;
        self.escaped_refs.clear();
        self.inside_spawn = false;
        self.inside_unsafe = false;
        self.spawn_ref_graph.clear();
        self.current_spawn_refs.clear();
        self.field_assignments.clear();
        self.write_borrow_arg_depth = 0;
        self.ref_bindings.clear();
        self.last_created_token = None;
        self.current_node = self.cfg.add_node(None); // 入口节点
        self.current_span = Span::dummy();
        // #265：假设栈随函数重置（路径条件不跨函数）
        self.gamma =
            crate::frontend::core::typecheck::proof::assumptions::FlowSensitiveGamma::new();
    }

    /// #265：把分支/循环守卫表达式转为 ConstExpr（注入假设栈用）
    ///
    /// 复用 const_eval 的转换器；非常量表达式返回 None——守卫只是增强信息，
    /// 无法转换时跳过 inject 而非中断检查（ponytail：不为此中断所有权检查）。
    fn condition_as_const(
        expr: &Expr
    ) -> Option<crate::frontend::core::types::const_data::ConstExpr> {
        crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(expr)
    }

    /// #265：ConstExpr 取反（else 分支路径条件 = !condition）
    fn negate_const(
        expr: crate::frontend::core::types::const_data::ConstExpr
    ) -> crate::frontend::core::types::const_data::ConstExpr {
        crate::frontend::core::types::const_data::ConstExpr::UnOp {
            op: crate::frontend::core::types::const_data::UnOp::Not,
            expr: Box::new(expr),
        }
    }

    /// 从表达式提取变量名（用于 Borrow/FieldAccess/Move 识别）
    fn extract_var_name(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Var(name, _) => Some(name.clone()),
            Expr::FieldAccess { expr: inner, .. } => Self::extract_var_name(inner),
            // #257 审计：`&mut arr[i]` / `&*ptr` 的借用目标此前返回 None，
            // 导致整个借用令牌块（可变性/冲突检测）被跳过——粗粒度归属到
            // 底层变量，保守但 sound（宁可误报冲突，不可漏检别名）
            Expr::Index { expr: inner, .. } => Self::extract_var_name(inner),
            Expr::UnOp {
                op: crate::frontend::core::parser::ast::UnOp::Deref,
                expr: inner,
                ..
            } => Self::extract_var_name(inner),
            _ => None,
        }
    }

    /// 提取完整调用路径（`list.len` / `len`），用于查调用目标签名。
    /// 与 extract_var_name（最内层变量名）不同——后者用于借用令牌归属。
    fn extract_call_path(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Var(name, _) => Some(name.clone()),
            Expr::FieldAccess {
                expr: inner, field, ..
            } => Self::extract_call_path(inner).map(|p| format!("{p}.{field}")),
            _ => None,
        }
    }

    /// 从 TypeEnvironment 查询函数参数的所有权语义
    fn lookup_param_types(
        &self,
        func_name: &str,
        arg_count: usize,
        env: &crate::frontend::core::typecheck::environment::TypeEnvironment,
    ) -> Vec<ParamOwnership> {
        // ponytail: std 限定名回退（list.len → std.list.len）+ 短名兜底；
        // 用户模块经 use 绑定为 Struct 时仍退化 Move，需 Resolver 才精确（罕见，暂不处理）
        let fn_ty = env
            .get_var(func_name)
            .map(|p| p.body.clone())
            .or_else(|| {
                if func_name.contains('.') && !func_name.starts_with("std.") {
                    env.native_signatures
                        .get(&format!("std.{func_name}"))
                        .cloned()
                } else {
                    None
                }
            })
            .or_else(|| env.native_signatures.get(func_name).cloned());
        match fn_ty {
            Some(crate::frontend::core::types::MonoType::Fn { params, .. }) => params
                .iter()
                .take(arg_count)
                .map(|p| match p {
                    crate::frontend::core::types::MonoType::Ref { mutable: true, .. } => {
                        ParamOwnership::WriteBorrow
                    }
                    crate::frontend::core::types::MonoType::Ref { mutable: false, .. } => {
                        ParamOwnership::ReadBorrow
                    }
                    _ => ParamOwnership::Move,
                })
                .collect(),
            _ => vec![ParamOwnership::Move; arg_count],
        }
    }

    /// 对单个变量参数执行所有权操作（按 ParamOwnership 语义）
    fn apply_param_ownership(
        &mut self,
        var_name: &str,
        ownership: &ParamOwnership,
    ) {
        match ownership {
            ParamOwnership::Move => {
                // #256：类型驱动——Dup/ValueCopy 传参是复制，不 move
                match self.classify_var(var_name) {
                    CopySemantics::Dup | CopySemantics::ValueCopy => {}
                    _ => {
                        self.push_var_op(VarOp::Move {
                            var: var_name.to_string(),
                        });
                    }
                }
            }
            ParamOwnership::ReadBorrow => {
                // transient=true：调用实参/方法接收者的读令牌随调用结束释放（§12.4），
                // 不被后文同名变量使用延长（#315）
                let token = self.brand_tree.create_read_token(
                    var_name.to_string(),
                    self.current_node,
                    true,
                );
                self.brand_tree.add_consumer(&token, self.current_node);
                if !self.brand_tree.conflicting_with(&token).is_empty() {
                    self.pending_writes.push(PendingWrite {
                        token: token.clone(),
                        node_idx: self.current_node,
                        span: self.current_span,
                    });
                }
            }
            ParamOwnership::WriteBorrow => {
                // transient=true：同 ReadBorrow——写令牌也只活在其调用节点（§12.5
                // 顺序复用的合法性正来自此：下一次 &mut 与已释放的上一次不冲突）
                let token = self.brand_tree.create_write_token(
                    var_name.to_string(),
                    self.current_node,
                    true,
                );
                self.brand_tree.add_consumer(&token, self.current_node);
                // WriteBorrow 总是进 pending_writes 检查冲突
                self.pending_writes.push(PendingWrite {
                    token: token.clone(),
                    node_idx: self.current_node,
                    span: self.current_span,
                });
            }
        }
    }

    /// 从表达式提取源码 span
    fn expr_span(expr: &Expr) -> Span {
        match expr {
            Expr::Lit(_, s)
            | Expr::Var(_, s)
            | Expr::Return(_, s)
            | Expr::Break(s)
            | Expr::Continue(s) => *s,
            Expr::BinOp { span, .. }
            | Expr::UnOp { span, .. }
            | Expr::Call { span, .. }
            | Expr::FnDef { span, .. }
            | Expr::If { span, .. }
            | Expr::Match { span, .. }
            | Expr::While { span, .. }
            | Expr::For { span, .. }
            | Expr::SpawnFor { span, .. }
            | Expr::Borrow { span, .. }
            | Expr::FieldAccess { span, .. }
            | Expr::Index { span, .. }
            | Expr::Tuple(_, span)
            | Expr::List(_, span)
            | Expr::Cast { span, .. }
            | Expr::Try { span, .. } => *span,
            Expr::Block(block) => block.span,
            _ => Span::dummy(),
        }
    }

    /// 为变量名对应的所有活跃令牌添加消费者
    fn add_consumer_for_var(
        &mut self,
        var_name: &str,
    ) {
        // #312：WriteBorrow 实参遍历期间压制消费者注册（见字段文档）
        if self.write_borrow_arg_depth > 0 {
            return;
        }
        let token_ids: Vec<BrandId> = self
            .brand_tree
            .root_tokens()
            .into_iter()
            .filter(|id| {
                self.brand_tree.get(id).is_some_and(|n| {
                    // #290 F1：普通读不消费 WriteToken——写令牌的活性区间只覆盖
                    // 其写操作本身（[birth, 写点]），读不延长它；把读节点挂上去
                    // 会让反向 BFS 把写点之后的节点标成写令牌活跃区（回溯误报）。
                    // 经 ref_bindings 的使用（w.x 透过 &mut）不受此限——那是
                    // 引用本身的用法，属写令牌活性。
                    // #315：瞬态令牌（调用实参/接收者自动借用）已随调用释放，
                    // 后文同名变量的使用不得延长其区间（§12.4 链 q.sum→q.shift→q.sum）
                    n.source_var == var_name && n.kind.is_read() && !n.transient
                })
            })
            .cloned()
            .collect();
        for id in &token_ids {
            self.brand_tree.add_consumer(id, self.current_node);
        }
        // #312：引用变量的使用 = 其持有令牌的消费（view.x 消费 view 绑定的读令牌）。
        // 令牌 source_var 是被借变量名，按名匹配不到引用变量——此前透过引用的
        // 使用从未注册消费者，借用活性失明。
        if let Some(tok) = self.ref_bindings.get(var_name) {
            self.brand_tree.add_consumer(tok, self.current_node);
        }
    }

    /// #257：Linear 令牌（&mut T）赋值复制的拒绝诊断（E2003）
    fn linear_copy_error(
        src_name: &str,
        span: Span,
    ) -> ProofResult {
        ProofResult::Disproved(super::super::proof::verdict::DisproofModel {
            kind: super::super::proof::verdict::DisproofKind::LinearTokenCopy,
            assignments: vec![("variable".into(), src_name.into())],
            constraint: format!(
                "`{}` 持有 `&mut` 写入令牌：SPEC §11.2 规定 &mut T 是 Linear（独占、非 Dup），不能复制。请在需要的地方重新创建 `&mut` 借用",
                src_name
            ),
            span: Some(span),
            predicate_span: None,
        })
    }

    /// 记录绑定：变量名 → 当前语句键（镜像作用域栈，#256）
    fn record_binding(
        &mut self,
        name: &str,
    ) {
        if let Some(scope) = self.scope_keys.last_mut() {
            scope.insert(name.to_string(), self.cur_stmt_key);
        }
    }

    /// #264：把变量操作记录到当前 CFG 节点（数据流分析阶段消费）
    fn push_var_op(
        &mut self,
        op: VarOp,
    ) {
        self.cfg.nodes[self.current_node].ops.push(op);
    }

    /// #264：检查点改记录 Read op——真正的 Move/Drop 判定延迟到
    /// `analyze_var_flow`（per-node 状态 + 汇合 meet + 循环不动点）。
    /// 返回 Proved 占位：walk 期不再即时判定。
    fn check_var_read(
        &mut self,
        name: &str,
        span: Span,
    ) -> ProofResult {
        self.push_var_op(VarOp::Read {
            var: name.to_string(),
            span,
        });
        ProofResult::Proved
    }

    /// #256：类型驱动的复制语义分类（SPEC §11.2）
    fn classify_mono(ty: &crate::frontend::core::types::MonoType) -> CopySemantics {
        use crate::frontend::core::types::MonoType;
        match ty {
            MonoType::Ref { mutable: false, .. } => CopySemantics::Dup,
            MonoType::Ref { mutable: true, .. } => CopySemantics::Linear,
            m if m.is_arc() => CopySemantics::Dup,
            MonoType::Int(_) | MonoType::Float(_) | MonoType::Bool | MonoType::Char => {
                CopySemantics::ValueCopy
            }
            // #302：Range 是不可变三标量记录（运行时内联值），值语义
            m if m.is_range() => CopySemantics::ValueCopy,
            _ => CopySemantics::Move,
        }
    }

    /// #256：变量语义分类——先查账本（作用域内层优先），再查 env（顶层/函数绑定），
    /// 都查不到保守按 Move
    fn classify_var(
        &self,
        name: &str,
    ) -> CopySemantics {
        for scope in self.scope_keys.iter().rev() {
            if let Some(key) = scope.get(name) {
                if let Some(poly) = self.type_ledger.get(&(*key, name.to_string())) {
                    return Self::classify_mono(&poly.body);
                }
            }
        }
        if let Some(env_ptr) = self.env {
            let env = unsafe { &*env_ptr };
            if let Some(poly) = env.get_var(name) {
                return Self::classify_mono(&poly.body);
            }
        }
        // ref T（Arc/Rc）语法回退：账本/env 都查不到时保留 #251 的追踪结果
        if self.ref_vars.contains(name) {
            return CopySemantics::Dup;
        }
        CopySemantics::Move
    }

    /// #315：查变量的推断类型——账本（作用域内层优先）优先，再查 env；
    /// 供方法接收者签名解析定位 "Type.method" 的 Type 名
    fn lookup_var_type(
        &self,
        name: &str,
    ) -> Option<crate::frontend::core::types::MonoType> {
        for scope in self.scope_keys.iter().rev() {
            if let Some(key) = scope.get(name) {
                if let Some(poly) = self.type_ledger.get(&(*key, name.to_string())) {
                    return Some(poly.body.clone());
                }
            }
        }
        let env_ptr = self.env?;
        let env = unsafe { &*env_ptr };
        env.get_var(name).map(|p| p.body.clone())
    }

    /// #315：方法调用接收者签名解析（#290 F4 路线 A 最小管道）
    ///
    /// `p.shift(...)` 的接收者藏在 func（FieldAccess）里，lookup_param_types 按
    /// "p.shift" 查 env 落空 → 全 Move 回退，&mut self 从不产生写令牌（漏报）。
    /// 这里借账本把接收者变量解析到类型名，拼 "Type.method" 查 method_bindings，
    /// 返回（接收者变量名，接收者所有权，显式实参所有权——对齐 params[1..]）。
    /// 仅处理接收者为裸变量、类型可解析为具名 Struct/TypeRef 的形态；
    /// 链式/泛型接收者/std native 方法回退原路径（#315 非目标）。
    fn method_receiver_ownership(
        &self,
        func: &Expr,
        arg_count: usize,
        env: &crate::frontend::core::typecheck::environment::TypeEnvironment,
    ) -> Option<(String, ParamOwnership, Vec<ParamOwnership>)> {
        let crate::frontend::core::parser::ast::Expr::FieldAccess {
            expr: obj, field, ..
        } = func
        else {
            return None;
        };
        let recv_name = Self::extract_var_name(obj)?;
        let recv_ty = self.lookup_var_type(&recv_name)?;
        let type_name = match &recv_ty {
            crate::frontend::core::types::MonoType::Struct(st) => st.name.clone(),
            crate::frontend::core::types::MonoType::TypeRef(n) => n.clone(),
            _ => return None,
        };
        let method = env.get_method_binding(&type_name, field)?;
        let crate::frontend::core::types::MonoType::Fn { params, .. } = method else {
            return None;
        };
        // 接收者占 params[0]（RFC-004：Type.method 首参即接收者）。
        // 仅显式 &self/&mut self（Ref 形）进令牌管线；Self 泛型（RFC-011a 接口
        // 分发）与按值接收者的所有权语义属 #315 非目标，回退原路径（不建令牌）。
        if !matches!(
            params.first(),
            Some(crate::frontend::core::types::MonoType::Ref { .. })
        ) {
            return None;
        }
        let own_of = |ty: &crate::frontend::core::types::MonoType| match ty {
            crate::frontend::core::types::MonoType::Ref { mutable: true, .. } => {
                ParamOwnership::WriteBorrow
            }
            crate::frontend::core::types::MonoType::Ref { mutable: false, .. } => {
                ParamOwnership::ReadBorrow
            }
            _ => ParamOwnership::Move,
        };
        let recv_own = own_of(&params[0]);
        let arg_owns = params.iter().skip(1).take(arg_count).map(own_of).collect();
        Some((recv_name, recv_own, arg_owns))
    }

    // ── 控制流方法（walk_expr 和 walk_stmt 共用） ──────────

    /// walk_if：If 表达式/语句的控制流构建
    ///
    /// 负责 CFG 分叉（split → then/elif/else → merge）、
    /// 路径条件收集、各分支子图遍历。
    fn walk_if(
        &mut self,
        condition: &Expr,
        then_body: &[Stmt],
        else_ifs: &[(&Expr, &[Stmt])],
        else_body: Option<&[Stmt]>,
    ) -> Vec<ProofResult> {
        let split_node = self.current_node;
        let mut results = self.walk_expr(condition);
        // #290 F1b：整个 if 结构结束后恢复进入前的路径条件
        let saved_condition = self.current_condition.clone();
        // #264：字面量常量条件 → 不可达分支不建边不遍历（消除 move 泄漏误报）。
        // 仅认字面量 Bool：不做 const_eval 传播（#262/#263 路径条件 soundness 未解决，
        // 保守起见只裁剪编译期显然不可达的分支）。
        let cond_value = Self::literal_bool(condition);

        let merge_node = self.cfg.add_node(None);

        // then 分支 —— 路径条件 = condition（#265：守卫注入假设栈）
        // #312：CFG 节点存真实 ConstExpr（与 gamma 注入同一转换），供回边 SMT 查询消费
        let then_reachable = cond_value != Some(false);
        let then_start = self.cfg.add_node(Self::condition_as_const(condition));
        if then_reachable {
            self.cfg.add_edge(split_node, then_start, EdgeKind::Normal);
        }
        self.current_node = then_start;
        // #290 F1b：分支内逐语句节点继承分支守卫
        self.current_condition = Self::condition_as_const(condition);
        self.gamma.enter_scope();
        if let Some(cond) = Self::condition_as_const(condition) {
            self.gamma.inject(cond);
        }
        if then_reachable {
            results.extend(self.walk_stmts(then_body));
        }
        self.gamma.exit_scope();
        if then_reachable {
            self.cfg
                .add_edge(self.current_node, merge_node, EdgeKind::Normal);
        }

        // elif 分支 —— 路径条件 = else_if_cond
        let mut remaining_reachable = cond_value != Some(true);
        for (else_if_cond, else_if_body) in else_ifs {
            results.extend(self.walk_expr(else_if_cond));
            let else_if_start = self.cfg.add_node(Self::condition_as_const(else_if_cond));
            let elif_reachable =
                remaining_reachable && Self::literal_bool(else_if_cond) != Some(false);
            if elif_reachable {
                self.cfg
                    .add_edge(split_node, else_if_start, EdgeKind::Normal);
            }
            self.current_node = else_if_start;
            // #290 F1b：分支内逐语句节点继承分支守卫
            self.current_condition = Self::condition_as_const(else_if_cond);
            self.gamma.enter_scope();
            if let Some(cond) = Self::condition_as_const(else_if_cond) {
                self.gamma.inject(cond);
            }
            if elif_reachable {
                results.extend(self.walk_stmts(else_if_body));
            }
            self.gamma.exit_scope();
            if elif_reachable {
                self.cfg
                    .add_edge(self.current_node, merge_node, EdgeKind::Normal);
            }
            remaining_reachable =
                remaining_reachable && Self::literal_bool(else_if_cond) != Some(true);
        }

        // else 分支 —— 路径条件 = !condition
        let else_reachable = remaining_reachable;
        if let Some(else_body) = else_body {
            let else_start = self
                .cfg
                .add_node(Self::condition_as_const(condition).map(Self::negate_const));
            if else_reachable {
                self.cfg.add_edge(split_node, else_start, EdgeKind::Normal);
            }
            self.current_node = else_start;
            // #290 F1b：分支内逐语句节点继承分支守卫
            self.current_condition = Self::condition_as_const(condition).map(Self::negate_const);
            self.gamma.enter_scope();
            if let Some(cond) = Self::condition_as_const(condition) {
                self.gamma.inject(Self::negate_const(cond));
            }
            if else_reachable {
                results.extend(self.walk_stmts(else_body));
            }
            self.gamma.exit_scope();
            if else_reachable {
                self.cfg
                    .add_edge(self.current_node, merge_node, EdgeKind::Normal);
            }
        } else if remaining_reachable {
            self.cfg.add_edge(split_node, merge_node, EdgeKind::Normal);
        }

        self.current_node = merge_node;
        // #290 F1b：恢复 if 之前的路径条件
        self.current_condition = saved_condition;
        results
    }

    /// #264：字面量 Bool 条件提取（仅认 `true`/`false` 字面量，
    /// 不做 const_eval——路径条件 soundness 未解决前的保守裁剪）
    fn literal_bool(expr: &Expr) -> Option<bool> {
        match expr {
            Expr::Lit(crate::frontend::core::parser::ast::Literal::Bool(b), _) => Some(*b),
            _ => None,
        }
    }

    /// walk_while：While 循环的控制流构建
    ///
    /// 负责 CFG 循环结构（head → body → back_edge → after_loop）、
    /// 路径条件收集。
    fn walk_while(
        &mut self,
        condition: &Expr,
        body: &[Stmt],
    ) -> Vec<ProofResult> {
        // #312：循环头路径条件存真实 ConstExpr（loop_cond，回边 SMT 查询的右元）
        let head_node = self.cfg.add_node(Self::condition_as_const(condition));
        self.cfg
            .add_edge(self.current_node, head_node, EdgeKind::Normal);

        let mut results = self.walk_expr(condition);

        let body_start = self.cfg.add_node(None);
        self.cfg.add_edge(head_node, body_start, EdgeKind::Normal);
        self.current_node = body_start;
        // #290 F1b：循环体语句不继承入边条件（与 body_start 无条件一致）；
        // 守卫条件由体内 if 分支自行设置
        let saved_condition = self.current_condition.take();
        // #265：循环守卫注入假设栈（循环体路径条件 = condition）
        self.gamma.enter_scope();
        if let Some(cond) = Self::condition_as_const(condition) {
            self.gamma.inject(cond);
        }
        results.extend(self.walk_stmts(body));
        self.gamma.exit_scope();
        self.current_condition = saved_condition;

        // 回边：body_end → head
        self.cfg
            .add_edge(self.current_node, head_node, EdgeKind::BackEdge);

        let after_loop = self.cfg.add_node(None);
        self.cfg.add_edge(head_node, after_loop, EdgeKind::Normal);
        self.current_node = after_loop;
        results
    }

    /// walk_for：For 循环的控制流构建
    ///
    /// 迭代变量每次迭代新绑定（语言设计保证），
    /// CFG 循环结构（head → body → back_edge → after_loop）。
    fn walk_for(
        &mut self,
        var: &str,
        var_mut: bool,
        iterable: &Expr,
        body: &[Stmt],
    ) -> Vec<ProofResult> {
        let mut results = self.walk_expr(iterable);
        self.push_var_op(VarOp::Declare {
            var: var.to_string(),
        });
        self.var_mutability.insert(var.to_string(), var_mut);
        self.record_binding(var);

        let head_node = self.cfg.add_node(None);
        self.cfg
            .add_edge(self.current_node, head_node, EdgeKind::Normal);

        let body_start = self.cfg.add_node(None);
        self.cfg.add_edge(head_node, body_start, EdgeKind::Normal);
        self.current_node = body_start;
        // #290 F1b：循环体语句不继承入边条件（与 body_start 无条件一致）
        let saved_condition = self.current_condition.take();
        results.extend(self.walk_stmts(body));
        self.current_condition = saved_condition;

        self.cfg
            .add_edge(self.current_node, head_node, EdgeKind::BackEdge);

        let after_loop = self.cfg.add_node(None);
        self.cfg.add_edge(head_node, after_loop, EdgeKind::Normal);
        self.current_node = after_loop;
        results
    }

    fn walk_expr(
        &mut self,
        expr: &Expr,
    ) -> Vec<ProofResult> {
        self.current_span = Self::expr_span(expr);
        let result = match expr {
            Expr::Var(name, _) => {
                let mut results = Vec::new();
                let check = self.check_var_read(name, self.current_span);
                if !check.is_proved() {
                    results.push(check);
                }
                // spawn 体内使用 ref 变量 → 标记逃逸
                if self.inside_spawn && self.ref_vars.contains(name) {
                    self.escaped_refs.insert(name.clone());
                    self.current_spawn_refs.insert(name.clone());
                }
                self.add_consumer_for_var(name);
                results
            }

            Expr::Borrow { mutable, expr, .. } => {
                // #290 F2：&mut 的取址走遍不注册消费者——写节点被挂到既有读令牌
                // 的消费者集会让反向 BFS 自我播种（读令牌"消费"于写点 → 恒 unsafe，
                // R5c 形态）。与 Call 臂 WriteBorrow 实参的压制同机制。
                if *mutable {
                    self.write_borrow_arg_depth += 1;
                }
                let mut results = self.walk_expr(expr);
                if *mutable {
                    self.write_borrow_arg_depth -= 1;
                }
                if let Some(var_name) = Self::extract_var_name(expr) {
                    // 变量本身被"使用"——检查 Move/Drop 状态
                    let check = self.check_var_read(&var_name, self.current_span);
                    if !check.is_proved() {
                        results.push(check);
                    }
                    // #290 F2：可变借用点不在既有令牌上登记消费者——读令牌的活性
                    // 由自身区间判定（后置使用经反向 BFS 抵达写点才冲突），此处
                    // 登记等于把写点伪造成读令牌的使用点，回溯误报复活。
                    if !*mutable {
                        self.add_consumer_for_var(&var_name);
                    }

                    // 可变性检查：&mut 要求变量声明为 mut
                    if *mutable {
                        let is_mut = self.var_mutability.get(&var_name).copied().unwrap_or(true);
                        if !is_mut {
                            results.push(emit_mut_predicate(&var_name, false, self.current_span));
                            return results; // 不创建 WriteToken，避免级联误报
                        }
                    }

                    // #315：借用表达式默认瞬态（调用实参等未绑定形态随语句释放，
                    // §12.4）；Assign 绑定到引用变量时经 set_transient(false)
                    // 转 var 绑定（活到作用域结束，D5 裁决）
                    let token = if *mutable {
                        self.brand_tree.create_write_token(
                            var_name.clone(),
                            self.current_node,
                            true,
                        )
                    } else {
                        self.brand_tree
                            .create_read_token(var_name.clone(), self.current_node, true)
                    };
                    self.brand_tree.add_consumer(&token, self.current_node);
                    // #290 F1：可变借用的创建是对同源**写类**既有令牌的竞争声明
                    //（var 绑定的写令牌活到作用域结束），把创建节点登记进它们的
                    // 消费者，写-写冲突才有活性可判。读类令牌不登记——读不延长
                    // 写令牌活性，也不被新写反向延长（读靠自身区间判定，R2 修复点）。
                    if *mutable {
                        let write_conflicts: Vec<BrandId> = self
                            .brand_tree
                            .conflicting_with(&token)
                            .into_iter()
                            .filter(|id| {
                                self.brand_tree.get(id).is_some_and(|n| {
                                    // #315：瞬态写令牌（调用实参）已随调用释放，
                                    // 不参与竞争声明，登记只会伪造其活性
                                    n.kind.is_write() && !n.transient
                                })
                            })
                            .cloned()
                            .collect();
                        for c in &write_conflicts {
                            self.brand_tree.add_consumer(c, self.current_node);
                        }
                    }
                    // #312：记录本 Borrow 创建的令牌，Assign 臂将其绑到目标变量
                    //（view = &p → ref_bindings[view] = token），供 add_consumer_for_var 解析
                    self.last_created_token = Some(token.clone());

                    // 检查品牌树中是否已有冲突令牌，有则送入反向 BFS 验证
                    if !self.brand_tree.conflicting_with(&token).is_empty() {
                        self.pending_writes.push(PendingWrite {
                            token: token.clone(),
                            node_idx: self.current_node,
                            span: self.current_span,
                        });
                    }
                }
                results
            }

            Expr::FieldAccess {
                expr: inner, field, ..
            } => {
                let results = self.walk_expr(inner);
                if let Some(var_name) = Self::extract_var_name(inner) {
                    self.add_consumer_for_var(&var_name);
                    let parent_ids: Vec<BrandId> = self
                        .brand_tree
                        .root_tokens()
                        .iter()
                        .filter(|id| {
                            self.brand_tree
                                .get(id)
                                .is_some_and(|n| n.source_var == var_name)
                        })
                        .map(|id| (*id).clone())
                        .collect();
                    for parent_id in &parent_ids {
                        self.brand_tree
                            .derive_field(parent_id, field, self.current_node);
                    }
                }
                results
            }

            // Assign 需检查目标变量可变性（仅对已声明变量的重赋值）
            // 注：保持 walk left→right 顺序以兼容现有 var_state 时序
            // Assign 需检查目标变量可变性（仅对已声明变量的重赋值）
            // 注：保持 walk left→right 顺序以兼容现有 var_state 时序
            Expr::BinOp {
                op, left, right, ..
            } => {
                if *op == crate::frontend::core::parser::ast::BinOp::Assign {
                    // #257：表达式层赋值同样拒绝 Linear（&mut）令牌复制
                    if let Expr::Var(src_name, _) = right.as_ref() {
                        if self.classify_var(src_name) == CopySemantics::Linear {
                            let mut r = vec![Self::linear_copy_error(src_name, self.current_span)];
                            r.extend(self.walk_expr(right));
                            return r;
                        }
                    }
                    if let Expr::Var(name, _) = left.as_ref() {
                        // 仅在变量已存在且已记录可变性时检查（重赋值场景）
                        if let Some(&is_mut) = self.var_mutability.get(name) {
                            let mut r = self.walk_expr(left);
                            self.last_created_token = None;
                            r.extend(self.walk_expr(right));
                            // #312：x = &y 重赋值 → 重新绑定引用令牌（&x 解析为 Borrow）
                            if matches!(right.as_ref(), Expr::Borrow { .. }) {
                                if let Some(tok) = self.last_created_token.take() {
                                    // var 绑定：令牌活到作用域结束（D5），撤销瞬态
                                    self.brand_tree.set_transient(&tok, false);
                                    self.ref_bindings.insert(name.clone(), tok);
                                }
                            }
                            if !is_mut {
                                r.push(emit_mut_predicate(name, false, self.current_span));
                            }
                            self.add_consumer_for_var(name);
                            // ref 属性传播：x = ref_var → x 也是 ref 变量（spawn/Arc 机制）
                            if let Expr::Var(src_name, _) = right.as_ref() {
                                if self.ref_vars.contains(src_name) {
                                    self.ref_vars.insert(name.clone());
                                }
                            }
                            r
                        } else {
                            // 变量未在 var_mutability 中 → 首次声明（非 StmtKind::Var 路径）
                            let mut r = self.walk_expr(right);
                            // #312：view = &p 形式（Expr 赋值首声明）——捕获新建令牌
                            // 绑定到目标变量，使透过引用的使用（view.x）能注册消费者
                            //（&x 解析为 Expr::Borrow；Expr::Ref 是 `ref` 关键字，不建令牌）
                            if matches!(right.as_ref(), Expr::Borrow { .. }) {
                                if let Some(tok) = self.last_created_token.take() {
                                    // var 绑定：令牌活到作用域结束（D5），撤销瞬态
                                    self.brand_tree.set_transient(&tok, false);
                                    self.ref_bindings.insert(name.clone(), tok);
                                }
                            }
                            r.extend(self.walk_expr(left));
                            self.push_var_op(VarOp::Declare { var: name.clone() });
                            self.var_mutability.insert(name.clone(), false);
                            if let Some(scope) = self.scope_vars.last_mut() {
                                scope.push(name.clone());
                            }
                            // ref 属性传播（spawn/Arc 机制）
                            if let Expr::Var(src_name, _) = right.as_ref() {
                                if self.ref_vars.contains(src_name) {
                                    self.ref_vars.insert(name.clone());
                                }
                            }
                            r
                        }
                    } else {
                        let mut r = self.walk_expr(left);
                        r.extend(self.walk_expr(right));
                        r
                    }
                } else {
                    let mut r = self.walk_expr(left);
                    r.extend(self.walk_expr(right));
                    r
                }
            }
            Expr::UnOp {
                op: crate::frontend::core::parser::ast::UnOp::Deref,
                expr,
                span,
            } => {
                let mut results = Vec::new();
                if !self.inside_unsafe {
                    results.push(ProofResult::Disproved(
                        super::super::proof::verdict::DisproofModel {
                            kind: super::super::proof::verdict::DisproofKind::UnsafeViolation,
                            assignments: vec![],
                            constraint: "deref outside unsafe block".to_string(),
                            span: Some(*span),
                            predicate_span: None,
                        },
                    ));
                }
                results.extend(self.walk_expr(expr));
                results
            }
            Expr::UnOp { expr: inner, .. } => self.walk_expr(inner),
            Expr::Cast { expr: inner, .. } => self.walk_expr(inner),
            Expr::Index { expr, index, .. } => {
                let mut r = self.walk_expr(expr);
                r.extend(self.walk_expr(index));
                r
            }
            Expr::Tuple(elements, _) | Expr::List(elements, _) => {
                elements.iter().flat_map(|e| self.walk_expr(e)).collect()
            }
            Expr::Try { expr: inner, .. } => self.walk_expr(inner),
            Expr::Call { func, args, .. } => {
                let mut results = Vec::new();
                // 确定调用目标名（用于查签名和捕获）
                let func_name = Self::extract_call_path(func);
                // 查询函数的参数签名（未知函数回退为全 Move）
                let env: &crate::frontend::core::typecheck::environment::TypeEnvironment =
                    unsafe { &*self.env.unwrap() };
                // #315：方法调用（func = FieldAccess）接收者签名解析。命中时接收者
                // 走与自由函数 &mut 实参同款管线（活性检查 + 借用令牌 + 冲突登记），
                // 显式实参对齐方法签名 params[1..]；未命中（链式/泛型/std native）
                // 回退原路径。
                let method_sig = self.method_receiver_ownership(func, args.len(), env);
                if let Some((recv_name, recv_own, _)) = &method_sig {
                    let is_write = matches!(recv_own, ParamOwnership::WriteBorrow);
                    // 同 #312：&mut 接收者遍历期间压制消费者注册，避免写节点
                    // 被挂成既有读令牌的消费者后反向 BFS 自我标记恒 unsafe
                    if is_write {
                        self.write_borrow_arg_depth += 1;
                    }
                    results.extend(self.walk_expr(func));
                    if is_write {
                        self.write_borrow_arg_depth -= 1;
                    }
                    let check = self.check_var_read(recv_name, self.current_span);
                    if !check.is_proved() {
                        results.push(check);
                    }
                    if !is_write {
                        self.add_consumer_for_var(recv_name);
                    }
                    self.apply_param_ownership(recv_name, recv_own);
                } else {
                    results.extend(self.walk_expr(func));
                }
                let param_types = match &method_sig {
                    Some((_, _, arg_owns)) => arg_owns.clone(),
                    None => func_name
                        .as_ref()
                        .map(|n| self.lookup_param_types(n, args.len(), env))
                        .unwrap_or_else(|| vec![ParamOwnership::Move; args.len()]),
                };
                // 处理显式参数
                for (i, arg) in args.iter().enumerate() {
                    let ownership = param_types.get(i).unwrap_or(&ParamOwnership::Move);
                    let is_write_borrow = matches!(ownership, ParamOwnership::WriteBorrow);
                    // #312：WriteBorrow 实参遍历期间压制消费者注册——Var 臂的
                    // add_consumer_for_var 会把写节点注册成现有读令牌的消费者，
                    // 反向 BFS 以写节点为种子自我标记恒 unsafe，路径条件切断全部失效
                    if is_write_borrow {
                        self.write_borrow_arg_depth += 1;
                    }
                    results.extend(self.walk_expr(arg));
                    if is_write_borrow {
                        self.write_borrow_arg_depth -= 1;
                    }
                    if let Expr::Var(name, _) = arg {
                        let check = self.check_var_read(name, self.current_span);
                        if !check.is_proved() {
                            results.push(check);
                        }
                        if !is_write_borrow {
                            self.add_consumer_for_var(name);
                        }
                        self.apply_param_ownership(name, ownership);
                    }
                }
                results
            }
            Expr::Return(Some(inner), _) => {
                let results = self.walk_expr(inner);
                if let Expr::Var(name, _) = inner.as_ref() {
                    // #256：Dup/ValueCopy 返回是复制；Move/Linear 转移所有权
                    match self.classify_var(name) {
                        CopySemantics::Dup | CopySemantics::ValueCopy => {}
                        _ => {
                            self.push_var_op(VarOp::Move { var: name.clone() });
                        }
                    }
                }
                results
            }
            Expr::Return(None, _) => vec![],

            // Block：直接遍历内部语句
            Expr::Block(block) => self.walk_stmts(&block.stmts),

            Expr::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                let else_ifs: Vec<(&Expr, &[Stmt])> = else_if_branches
                    .iter()
                    .map(|(cond, body)| (cond.as_ref(), body.stmts.as_slice()))
                    .collect();
                let else_body = else_branch.as_ref().map(|b| b.stmts.as_slice());
                self.walk_if(condition, &then_branch.stmts, &else_ifs, else_body)
            }

            Expr::While {
                condition, body, ..
            } => self.walk_while(condition, &body.stmts),

            Expr::For {
                var,
                var_mut,
                iterable,
                body,
                ..
            } => self.walk_for(var, *var_mut, iterable, &body.stmts),

            Expr::Spawn { body, .. } => {
                let was_spawn = self.inside_spawn;
                self.inside_spawn = true;

                let prev_spawn_refs = std::mem::take(&mut self.current_spawn_refs);

                let results = self.walk_stmts(&body.stmts);

                // 构建 ref 依赖图
                let spawn_refs = std::mem::take(&mut self.current_spawn_refs);
                for ref_a in &spawn_refs {
                    for ref_b in &spawn_refs {
                        if ref_a != ref_b && self.ref_holds_ref(ref_a, ref_b) {
                            self.spawn_ref_graph
                                .entry(ref_a.clone())
                                .or_default()
                                .insert(ref_b.clone());
                        }
                    }
                }

                self.current_spawn_refs = prev_spawn_refs;
                self.inside_spawn = was_spawn;
                results
            }

            Expr::SpawnFor { body, .. } => {
                let was_spawn = self.inside_spawn;
                self.inside_spawn = true;
                let results = self.walk_stmts(&body.stmts);
                self.inside_spawn = was_spawn;
                results
            }

            Expr::Unsafe { body, .. } => {
                let was_unsafe = self.inside_unsafe;
                self.inside_unsafe = true;
                let results = self.walk_stmts(&body.stmts);
                self.inside_unsafe = was_unsafe;
                results
            }

            Expr::Ref { expr, .. } => {
                if self.inside_spawn {
                    if let Some(name) = Self::extract_var_name(expr) {
                        self.current_spawn_refs.insert(name);
                    }
                }
                // 继续处理子表达式
                self.walk_expr(expr)
            }

            // FnDef / Lambda / Lit / FString 等跳过
            _ => vec![],
        };
        self.node_spans.insert(self.current_node, self.current_span);
        result
    }

    fn walk_stmt(
        &mut self,
        stmt: &Stmt,
    ) -> Vec<ProofResult> {
        self.current_span = stmt.span;
        // 账本键与推断层对齐：当前语句的 span offset（#256）
        self.cur_stmt_key = stmt.span.start.offset;
        let result = match &stmt.kind {
            StmtKind::Expr(expr) => self.walk_expr(expr),

            StmtKind::Assign {
                target,
                value,
                is_mut,
                ..
            } => {
                use crate::frontend::core::parser::ast::Expr;
                let name = match target.as_ref() {
                    Expr::Var(n, _) => n.clone(),
                    // 非变量目标（字段写入 `m.x = v`、解构等）：仍须遍历 target 与
                    // value，让内部变量的 Move/Drop 状态被检查（#257：此前直接
                    // 返回导致已 move 的 `&mut` 令牌可经字段写入继续使用）。
                    _ => {
                        let mut results = self.walk_expr(target);
                        if let Some(v) = value.as_deref() {
                            results.extend(self.walk_expr(v));
                        }
                        // #290 F3：字段写创建 WriteToken（RFC-009 §2.7 同源
                        // WriteToken 与派生 ReadToken 不能同时活跃）。此前字段写
                        // 从不建令牌，派生读-写冲突静默放行（R1）。
                        // 经 ref 绑定的目标（w.x = v，w = &mut p）不建新令牌——
                        // 写经由 w 绑定的既有令牌，登记消费者延长其活性。
                        if let Expr::FieldAccess { expr: inner, .. } = target.as_ref() {
                            if let Some(root) = Self::extract_var_name(inner) {
                                if let Some(bound) = self.ref_bindings.get(&root).cloned() {
                                    self.brand_tree.add_consumer(&bound, self.current_node);
                                } else {
                                    let token = self.brand_tree.create_write_token(
                                        root.clone(),
                                        self.current_node,
                                        true,
                                    );
                                    self.brand_tree.add_consumer(&token, self.current_node);
                                    // 字段写令牌是瞬态的（不绑定变量，活在本节点）：
                                    // 不做写类竞争声明（那是 var 绑定令牌的语义），
                                    // 顺序字段写 q.x = 5; q.y = 7 不构成冲突。
                                    if !self.brand_tree.conflicting_with(&token).is_empty() {
                                        self.pending_writes.push(PendingWrite {
                                            token,
                                            node_idx: self.current_node,
                                            span: self.current_span,
                                        });
                                    }
                                }
                            }
                        }
                        return results;
                    }
                };
                // #264：is_new 判断用作用域账本——必须在 record_binding 之前查
                //（record_binding 会把 name 插入当前作用域，先查才能区分首声明/重绑定）
                let is_new = !self.scope_keys.iter().any(|m| m.contains_key(&name));
                self.record_binding(&name);
                let initializer = value.as_deref();
                let mut results = Vec::new();
                self.push_var_op(VarOp::Declare { var: name.clone() });
                self.var_mutability.insert(name.clone(), *is_mut);
                if is_new {
                    if let Some(scope) = self.scope_vars.last_mut() {
                        scope.push(name.clone());
                    }
                }
                if let Some(Expr::BinOp {
                    op: crate::frontend::core::parser::ast::BinOp::Assign,
                    left,
                    right,
                    ..
                }) = initializer
                {
                    if let Expr::FieldAccess {
                        expr: inner, field, ..
                    } = left.as_ref()
                    {
                        if let Some(var_name) = Self::extract_var_name(inner) {
                            if let Expr::Var(assigned_name, _) = right.as_ref() {
                                self.field_assignments.push((
                                    var_name,
                                    field.clone(),
                                    assigned_name.clone(),
                                ));
                            }
                        }
                    }
                }
                if let Some(init) = initializer {
                    if matches!(init, Expr::Ref { .. }) {
                        self.ref_vars.insert(name.clone());
                    }
                }
                if let Some(init) = initializer {
                    if let Expr::BinOp {
                        op: crate::frontend::core::parser::ast::BinOp::Assign,
                        left,
                        right,
                        ..
                    } = init
                    {
                        if let Expr::Var(assigned_name, _) = left.as_ref() {
                            if assigned_name == &name {
                                results.extend(self.walk_expr(right));
                                return results;
                            }
                        }
                    }
                    // #312：清空上次残留，walk 后若初值是 `&x`（Borrow）则捕获新建令牌
                    self.last_created_token = None;
                    results.extend(self.walk_expr(init));
                    if matches!(init, Expr::Borrow { .. }) {
                        if let Some(tok) = self.last_created_token.take() {
                            // var 绑定：令牌活到作用域结束（D5），撤销瞬态
                            self.brand_tree.set_transient(&tok, false);
                            self.ref_bindings.insert(name.clone(), tok);
                        }
                    }
                    if let Expr::Var(src_name, _) = init {
                        // #256/#257：类型驱动的复制语义（SPEC §11.2）
                        match self.classify_var(src_name) {
                            CopySemantics::Linear => {
                                results.push(Self::linear_copy_error(src_name, self.current_span));
                                return results;
                            }
                            CopySemantics::Dup | CopySemantics::ValueCopy => {}
                            CopySemantics::Move => {
                                self.push_var_op(VarOp::Move {
                                    var: src_name.clone(),
                                });
                            }
                        }
                        // ref 属性传播（spawn/Arc 机制）
                        if self.ref_vars.contains(src_name) {
                            self.ref_vars.insert(name.clone());
                        }
                    }
                }
                // 递归处理 Lambda/Block 函数体
                if let Some(init) = initializer {
                    if let Expr::Lambda { params, body, .. } = init {
                        if !body.stmts.is_empty() {
                            for param in params {
                                self.push_var_op(VarOp::Declare {
                                    var: param.name.clone(),
                                });
                                self.var_mutability.insert(param.name.clone(), param.is_mut);
                                self.record_binding(&param.name);
                            }
                            results.extend(self.walk_stmts(&body.stmts));
                        }
                    } else if let Expr::Block(block) = init {
                        if !block.stmts.is_empty() {
                            results.extend(self.walk_stmts(&block.stmts));
                        }
                    }
                }
                results
            }

            StmtKind::Return(Some(expr)) => {
                let results = self.walk_expr(expr);
                if let Expr::Var(name, _) = expr.as_ref() {
                    // #256：Dup/ValueCopy 返回是复制；Move/Linear 转移所有权
                    match self.classify_var(name) {
                        CopySemantics::Dup | CopySemantics::ValueCopy => {}
                        _ => {
                            self.push_var_op(VarOp::Move { var: name.clone() });
                        }
                    }
                }
                results
            }
            StmtKind::Return(None) => vec![],

            StmtKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                let else_ifs: Vec<(&Expr, &[Stmt])> = else_if_branches
                    .iter()
                    .map(|(cond, body)| (cond.as_ref(), body.stmts.as_slice()))
                    .collect();
                let else_body = else_branch.as_ref().map(|b| b.stmts.as_slice());
                self.walk_if(condition, &then_branch.stmts, &else_ifs, else_body)
            }

            StmtKind::For {
                var,
                var_mut,
                iterable,
                body,
                ..
            } => self.walk_for(var, *var_mut, iterable, &body.stmts),

            _ => vec![],
        };
        self.node_spans.insert(self.current_node, self.current_span);
        result
    }

    fn walk_stmts(
        &mut self,
        stmts: &[Stmt],
    ) -> Vec<ProofResult> {
        self.scope_vars.push(Vec::new());
        self.scope_keys.push(HashMap::new());
        let mut results = Vec::new();
        for stmt in stmts {
            // #290 F1b：语句级 CFG——每条语句独占节点并链接前驱。此前直线语句
            // 全部挤在分支骨架的同一节点上，令牌出生/消费的语句顺序对活性不可见：
            // 同节点内"先写后读"与"先读后写"无法区分（R2 直线回溯误报的根源）。
            // 节点继承 current_condition（守卫写 #312 的 SMT 切断依赖它）。
            let stmt_node = self.cfg.add_node(self.current_condition.clone());
            self.cfg
                .add_edge(self.current_node, stmt_node, EdgeKind::Normal);
            self.current_node = stmt_node;
            results.extend(self.walk_stmt(stmt));
        }
        // 作用域退出：将本作用域内声明且仍 Alive 的变量标记为 Dropped
        // #264：判定延迟到数据流分析（Drop op 在分析时检查状态）；
        // scope_drops（release plan 用）也由分析阶段收集。
        if let Some(scope) = self.scope_vars.pop() {
            let span = self.current_span;
            for var in &scope {
                self.push_var_op(VarOp::Drop {
                    var: var.clone(),
                    span,
                });
            }
        }
        self.scope_keys.pop();
        results
    }

    /// 检查 ref_a 是否持有 ref_b 的引用（通过检查字段赋值）
    fn ref_holds_ref(
        &self,
        ref_a: &str,
        ref_b: &str,
    ) -> bool {
        // 检查是否有 ref_a.field = ref_b 的赋值
        self.field_assignments
            .iter()
            .any(|(var, _, assigned)| var == ref_a && assigned == ref_b)
    }

    /// 检测 spawn ref 循环（DFS）
    fn detect_spawn_cycle(&self) -> Option<String> {
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();
        let mut path = Vec::new();

        for node in self.spawn_ref_graph.keys() {
            if !visited.contains(node)
                && self.detect_cycle_dfs(node, &mut visited, &mut recursion_stack, &mut path)
            {
                return Some(path.join(" -> "));
            }
        }
        None
    }

    /// DFS 检测循环
    fn detect_cycle_dfs(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        visited.insert(node.to_string());
        recursion_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = self.spawn_ref_graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.detect_cycle_dfs(neighbor, visited, recursion_stack, path) {
                        return true;
                    }
                } else if recursion_stack.contains(neighbor) {
                    // 找到循环
                    return true;
                }
            }
        }

        path.pop();
        recursion_stack.remove(node);
        false
    }

    /// 生成 NLL 精确释放计划
    ///
    /// 两源合并：
    /// 1. BrandTree 令牌消费者分析（借用变量的最后使用点）
    /// 2. 作用域退出收集的 Drop 记录（非借用变量的作用域结束点）
    ///
    /// 结果按 Span 分组，组内 LIFO 排序（子先父后）。
    fn build_release_plan(
        &self,
        params: &[crate::frontend::core::parser::ast::Param],
    ) -> ReleasePlan {
        let param_names: HashSet<&str> = params.iter().map(|p| p.name.as_str()).collect();
        let mut span_groups: HashMap<Span, Vec<&str>> = HashMap::new();

        // 源 1：BrandTree 令牌消费者
        for node in self.brand_tree.nodes.values() {
            if param_names.contains(node.source_var.as_str()) {
                continue; // 参数由调用方负责释放
            }
            if let Some(&max_consumer) = node.consumers.iter().max() {
                if let Some(&span) = self.node_spans.get(&max_consumer) {
                    span_groups.entry(span).or_default().push(&node.source_var);
                }
            }
        }

        // 源 2：作用域退出 Drop 记录（覆盖非借用变量）
        for (span, var) in &self.scope_drops {
            if !param_names.contains(var.as_str()) {
                span_groups.entry(*span).or_default().push(var);
            }
        }

        // 每组内去重 + LIFO 排序（子先父后）
        let mut drops: HashMap<Span, Vec<String>> = HashMap::new();
        for (span, vars) in span_groups {
            let mut unique: Vec<String> = vars
                .iter()
                .collect::<HashSet<_>>()
                .into_iter()
                .map(|s| s.to_string())
                .collect();
            // 按前缀关系排序：持有子令牌的变量先释放
            unique.sort_by(|a, b| {
                let a_is_child_of_b = self.brand_tree.nodes.values().any(|n| {
                    n.source_var == *a
                        && n.parent.as_ref().is_some_and(|p| {
                            self.brand_tree
                                .nodes
                                .get(p)
                                .is_some_and(|pn| pn.source_var == *b)
                        })
                });
                if a_is_child_of_b {
                    std::cmp::Ordering::Greater // a 是子 → 先释放（后排序）
                } else {
                    std::cmp::Ordering::Less
                }
            });
            drops.insert(span, unique);
        }

        ReleasePlan { drops }
    }

    /// #264：前向数据流分析（NLL/Polonius 风格 move 分析）
    ///
    /// walk 阶段已把变量操作记录到各 CFG 节点。这里：
    /// 1. 每节点 in/out 状态（HashMap<String, VarState>）
    /// 2. 汇合 meet：所有前驱 out 取保守 max（Dropped > Moved > Alive）；
    ///    单方存在的变量保留原值（分支内新变量已被 Drop op 标 Dropped，
    ///    汇合后保持 Dropped = 作用域外不可见）
    /// 3. 循环回边 → 迭代到不动点（状态格有限单调，必收敛）
    /// 4. 检查：每节点按序执行 ops，Read 时按当前状态判 E2014/E2018；
    ///    Drop 时 Alive → Dropped 并收集 scope_drops（release plan）
    fn analyze_var_flow(&mut self) -> Vec<ProofResult> {
        let n = self.cfg.nodes.len();
        // in/out 状态（Vec 下标 = CFG 节点 id）
        let mut ins: Vec<HashMap<String, VarState>> = vec![HashMap::new(); n];
        let mut outs: Vec<HashMap<String, VarState>> = vec![HashMap::new(); n];

        // 不动点迭代：直到所有节点 in/out 不再变化
        loop {
            let mut changed = false;
            for i in 0..n {
                // in = meet(所有前驱 out)
                let mut new_in: HashMap<String, VarState> = HashMap::new();
                for &pred in &self.cfg.nodes[i].predecessors {
                    Self::meet_into(&mut new_in, &outs[pred]);
                }
                // transfer：顺序执行 ops
                let mut new_out = new_in.clone();
                for op in &self.cfg.nodes[i].ops {
                    Self::apply_op_state(&mut new_out, op);
                }
                if new_in != ins[i] || new_out != outs[i] {
                    ins[i] = new_in;
                    outs[i] = new_out;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // 检查阶段：每节点从 in 出发顺序执行，Read 判定 + Drop 收集
        let mut results = Vec::new();
        for (i, in_state) in ins.iter().enumerate() {
            let mut state = in_state.clone();
            for op in &self.cfg.nodes[i].ops {
                match op {
                    VarOp::Read { var, span } => match state.get(var) {
                        Some(VarState::Moved) => {
                            results.push(emit_move_predicate(var, true, *span));
                        }
                        Some(VarState::Dropped) => {
                            results.push(emit_drop_predicate(var, true, *span));
                        }
                        _ => {}
                    },
                    VarOp::Drop { var, span } => {
                        if state.get(var) == Some(&VarState::Alive) {
                            state.insert(var.clone(), VarState::Dropped);
                            self.scope_drops.push((*span, var.clone()));
                        }
                    }
                    other => Self::apply_op_state(&mut state, other),
                }
            }
        }
        results
    }

    /// meet：把 other 合并进 acc（逐变量保守 max，单方存在保留原值）
    fn meet_into(
        acc: &mut HashMap<String, VarState>,
        other: &HashMap<String, VarState>,
    ) {
        for (k, v) in other {
            match acc.get(k) {
                Some(cur) if v > cur => {
                    acc.insert(k.clone(), *v);
                }
                None => {
                    acc.insert(k.clone(), *v);
                }
                _ => {}
            }
        }
    }

    /// transfer：单条 op 对状态的更新（Declare/Move/Drop；Read 不影响）
    fn apply_op_state(
        state: &mut HashMap<String, VarState>,
        op: &VarOp,
    ) {
        match op {
            VarOp::Declare { var } => {
                state.insert(var.clone(), VarState::Alive);
            }
            VarOp::Move { var } => {
                state.insert(var.clone(), VarState::Moved);
            }
            // #264：transfer 与检查阶段一致——Drop 把 Alive 降为 Dropped
            //（scope_drops 收集只在检查阶段做一次，transfer 多轮迭代不重复）
            VarOp::Drop { var, .. } => {
                if state.get(var) == Some(&VarState::Alive) {
                    state.insert(var.clone(), VarState::Dropped);
                }
            }
            VarOp::Read { .. } => {}
        }
    }

    /// 检查单个函数体：重置状态 → 一趟遍历 → 排空待定写操作 → ReleasePlan
    fn check_function(
        &mut self,
        _name: &str,
        params: &[crate::frontend::core::parser::ast::Param],
        body: &[Stmt],
        env: &crate::frontend::core::typecheck::environment::TypeEnvironment,
        stmt_key: usize,
    ) -> (Vec<ProofResult>, ReleasePlan, HashSet<String>) {
        self.reset();

        // 设置类型环境引用（供 walk_expr 使用）
        self.env = Some(env as *const _);

        // 标记参数为 Alive，记录可变性；参数键与推断层对齐（#256）
        // #264：参数作为 entry 节点的 Declare op（数据流分析的初值）
        self.cur_stmt_key = stmt_key;
        for param in params {
            self.push_var_op(VarOp::Declare {
                var: param.name.clone(),
            });
            self.var_mutability.insert(param.name.clone(), param.is_mut);
            self.record_binding(&param.name);
        }

        // 一趟遍历：构建 CFG + 记录变量操作 + 收集待定写操作
        let mut results = self.walk_stmts(body);
        self.cfg.exit = self.current_node;

        // #264：数据流分析——per-node 变量状态（汇合 meet + 循环不动点），
        // 对每个 Read op 判定 Move/Drop 违例，并收集 scope_drops。
        results.extend(self.analyze_var_flow());

        // 排空待定写操作：反向 BFS（CFG + BrandTree + 消费者此时全部完整）
        for pending in self.pending_writes.drain(..) {
            results.push(emit_borrow_predicate(
                &self.brand_tree,
                &self.cfg,
                &pending.token,
                pending.node_idx,
                pending.span,
            ));
        }

        let release_plan = self.build_release_plan(params);
        let escaped = std::mem::take(&mut self.escaped_refs);
        (results, release_plan, escaped)
    }

    /// 遍历模块中的所有函数体，执行所有权检查
    pub fn check_module(
        &mut self,
        module: &Module,
        _env: &crate::frontend::core::typecheck::environment::TypeEnvironment,
        type_ledger: &HashMap<(usize, String), crate::frontend::core::types::PolyType>,
    ) -> (Vec<ProofResult>, ReleasePlan, HashSet<String>) {
        self.type_ledger = type_ledger.clone();
        let mut results = Vec::new();
        let mut merged_drops: HashMap<Span, Vec<String>> = HashMap::new();
        let mut merged_escaped: HashSet<String> = HashSet::new();
        for stmt in &module.items {
            if let StmtKind::Assign { target, value, .. } = &stmt.kind {
                use crate::frontend::core::parser::ast::Expr;
                let name = match target.as_ref() {
                    Expr::Var(n, _) => n.clone(),
                    _ => continue,
                };
                let (params, body) = match value {
                    Some(v) => {
                        if let Expr::Lambda { params, body, .. } = v.as_ref() {
                            (params.clone(), body.stmts.clone())
                        } else if let Expr::Block(block) = v.as_ref() {
                            (Vec::new(), block.stmts.clone())
                        } else {
                            (Vec::new(), Vec::new())
                        }
                    }
                    None => (Vec::new(), Vec::new()),
                };
                if params.is_empty() && body.is_empty() {
                    continue;
                }
                let (func_results, func_plan, escaped) =
                    self.check_function(&name, &params, &body, _env, stmt.span.start.offset);
                results.extend(func_results);
                merged_drops.extend(func_plan.drops);
                merged_escaped.extend(escaped);
            }
        }

        // 检测 spawn ref 循环
        if let Some(cycle) = self.detect_spawn_cycle() {
            results.push(ProofResult::Disproved(
                super::super::proof::verdict::DisproofModel {
                    kind: super::super::proof::verdict::DisproofKind::SpawnCycleViolation,
                    assignments: vec![],
                    constraint: format!("spawn ref cycle: {}", cycle),
                    span: None,
                    predicate_span: None,
                },
            ));
        }

        (
            results,
            ReleasePlan {
                drops: merged_drops,
            },
            merged_escaped,
        )
    }
}
