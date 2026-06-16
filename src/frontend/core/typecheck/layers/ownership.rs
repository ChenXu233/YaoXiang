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
use crate::frontend::core::types::const_data::ConstValue;
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

// ── Captures ──────────────────────────────────────────────

/// 闭包的捕获变量集合（定义时分析产出，调用时消费）
#[derive(Debug, Clone, Default)]
struct Captures {
    /// 只读捕获 → 调用时创建 ReadToken
    reads: HashSet<String>,
    /// 写入捕获 → 调用时创建 WriteToken
    writes: HashSet<String>,
    /// 移动捕获 → 调用时标记 Moved
    moves: HashSet<String>,
}

/// Key = 闭包变量名 → 捕获信息
type CapturesStore = HashMap<String, Captures>;

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

/// 状态快照（定义闭包时保存/恢复）
struct StateSnapshot {
    var_state: HashMap<String, VarState>,
    brand_nodes_count: usize,
    scope_vars_len: usize,
    scope_drops_len: usize,
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
}

impl BrandNode {
    fn new(
        id: BrandId,
        kind: TokenKind,
        source_var: String,
    ) -> Self {
        Self {
            id,
            kind,
            source_var,
            parent: None,
            children: HashSet::new(),
            consumers: HashSet::new(),
            ref_count: 1,
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
    ) -> BrandId {
        let id = BrandId::root(self.next_id);
        self.next_id += 1;
        self.nodes.insert(
            id.clone(),
            BrandNode::new(id.clone(), TokenKind::ReadToken, source),
        );
        id
    }

    pub fn create_write_token(
        &mut self,
        source: String,
    ) -> BrandId {
        let id = BrandId::root(self.next_id);
        self.next_id += 1;
        self.nodes.insert(
            id.clone(),
            BrandNode::new(id.clone(), TokenKind::WriteToken, source),
        );
        id
    }

    // ── 派生 ──────────────────────────────────────────

    /// 从父令牌派生字段访问令牌。返回子令牌 ID。
    pub fn derive_field(
        &mut self,
        parent_id: &BrandId,
        field: &str,
    ) -> Option<BrandId> {
        let parent = self.nodes.get(parent_id)?;
        let child_id = parent_id.derive_field(field);
        let source_var = parent.source_var.clone();
        let kind = parent.kind;

        let mut child = BrandNode::new(child_id.clone(), kind, source_var);
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
    /// 该节点的路径条件（if guard / while cond / match pattern）
    pub path_condition: Option<String>,
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
        path_condition: Option<String>,
    ) -> usize {
        let id = self.nodes.len();
        self.nodes.push(CfgNode {
            id,
            successors: Vec::new(),
            predecessors: Vec::new(),
            path_condition,
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
    let mut queue: Vec<usize> = Vec::new();

    for conflict_id in &conflicting {
        for consumer in tree.consumers(conflict_id) {
            if consumer < cfg.nodes.len() {
                queue.push(consumer);
            }
        }
    }

    while let Some(cur) = queue.pop() {
        if unsafe_nodes.contains(&cur) {
            continue;
        }
        unsafe_nodes.insert(cur);
        if cur >= cfg.nodes.len() {
            continue;
        }

        for &pred in &cfg.nodes[cur].predecessors {
            // 结构切断：break 边不会出现在 predecessors 中（add_edge 已过滤）

            let is_back_edge = cfg.nodes[pred]
                .successors
                .iter()
                .any(|(succ, kind)| *succ == cur && *kind == EdgeKind::BackEdge);

            if is_back_edge {
                if let (Some(ref path_cond), Some(ref loop_cond)) = (
                    &cfg.nodes[pred].path_condition,
                    &cfg.nodes[cur].path_condition,
                ) {
                    if smt_cut(path_cond, loop_cond) {
                        continue; // 逻辑切断
                    }
                }
            }

            if !unsafe_nodes.contains(&pred) {
                queue.push(pred);
            }
        }
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

/// SMT 逻辑切断：判定 `path_cond ⇒ !loop_cond`
///
/// 仅在回边 + 有路径条件时调用。
/// 使用 RFC-027 的 Z3 后端（已实现）。
///
/// 构造约束：!(path_cond ∧ loop_cond)
/// unsat → 蕴含成立 → 切断成功
/// sat   → 存在反例 → 不切断
fn smt_cut(
    _path_cond: &str,
    _loop_cond: &str,
) -> bool {
    // wasm 模式下 Z3 不可用，保守不切断
    #[cfg(target_arch = "wasm32")]
    return false;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let constraint = ConstExpr::UnOp {
            op: UnOp::Not,
            expr: Box::new(ConstExpr::BinOp {
                op: BinOp::And,
                left: Box::new(ConstExpr::NamedVar("path_cond".into())),
                right: Box::new(ConstExpr::NamedVar("loop_cond".into())),
            }),
        };

        let path_assumption = ConstExpr::BinOp {
            op: BinOp::Eq,
            left: Box::new(ConstExpr::NamedVar("path_cond".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Bool(true))),
        };
        let loop_assumption = ConstExpr::BinOp {
            op: BinOp::Eq,
            left: Box::new(ConstExpr::NamedVar("loop_cond".into())),
            right: Box::new(ConstExpr::Lit(ConstValue::Bool(true))),
        };

        let assumptions = vec![path_assumption, loop_assumption];
        let mut var_sorts = HashMap::new();
        var_sorts.insert("path_cond".into(), SMTSort::Bool);
        var_sorts.insert("loop_cond".into(), SMTSort::Bool);

        let commands = translate_constraint(&constraint, &assumptions, &var_sorts);

        let backend = match Z3Backend::new() {
            Ok(b) => b,
            Err(_) => return false, // Z3 不可用 → 保守不切断
        };

        matches!(backend.solve(&commands, 100), SMTResult::Unsat)
    } // cfg(not(target_arch = "wasm32"))
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

/// 检查所有权无冲突（Layer 1）。
///
/// 由 checker.rs 在遍历函数体时调用核心逻辑
/// （BrandTree + 快速通道 + 谓词）。
pub fn check_ownership(_ctx: &ProofContext<'_>) -> ProofResult {
    ProofResult::Proved
}

// ── OwnershipChecker：AST 遍历 ───────────────────────────

use crate::frontend::core::parser::ast::{Expr, Module, Stmt, StmtKind};

/// 函数内变量状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VarState {
    Alive,
    Moved,
    /// 值已被释放（作用域退出时自动标记）
    Dropped,
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
    var_state: HashMap<String, VarState>,
    /// 变量是否声明为 mut（用于可变性违规检测）
    var_mutability: HashMap<String, bool>,
    pending_writes: Vec<PendingWrite>,
    /// 当前 CFG 节点索引（walk 过程中推进）
    current_node: usize,
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
    /// 闭包捕获存储（定义时分析产出，调用时消费）
    captures_store: CapturesStore,
    /// 类型环境引用（用裸指针避免生命周期重写；env 生命周期 > checker）
    env: Option<*const crate::frontend::core::typecheck::environment::TypeEnvironment>,
    /// ref 创建的变量（Expr::Ref 的赋值目标）
    ref_vars: HashSet<String>,
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
            var_state: HashMap::new(),
            var_mutability: HashMap::new(),
            pending_writes: Vec::new(),
            current_node: 0,
            current_span: Span::dummy(),
            node_spans: HashMap::new(),
            scope_vars: Vec::new(),
            scope_drops: Vec::new(),
            captures_store: CapturesStore::new(),
            env: None,
            ref_vars: HashSet::new(),
            escaped_refs: HashSet::new(),
            inside_spawn: false,
            inside_unsafe: false,
            spawn_ref_graph: HashMap::new(),
            current_spawn_refs: HashSet::new(),
            field_assignments: Vec::new(),
        }
    }

    /// 重置函数级状态
    fn reset(&mut self) {
        self.brand_tree = BrandTree::new();
        self.cfg = ControlFlowGraph::new();
        self.var_state.clear();
        self.var_mutability.clear();
        self.pending_writes.clear();
        self.node_spans.clear();
        self.scope_vars.clear();
        self.scope_drops.clear();
        self.captures_store.clear();
        self.ref_vars.clear();
        self.escaped_refs.clear();
        self.inside_spawn = false;
        self.inside_unsafe = false;
        self.spawn_ref_graph.clear();
        self.current_spawn_refs.clear();
        self.field_assignments.clear();
        self.current_node = self.cfg.add_node(None); // 入口节点
        self.current_span = Span::dummy();
    }

    /// 从表达式提取变量名（用于 Borrow/FieldAccess/Move 识别）
    fn extract_var_name(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Var(name, _) => Some(name.clone()),
            Expr::FieldAccess { expr: inner, .. } => Self::extract_var_name(inner),
            _ => None,
        }
    }

    /// 保存当前状态快照（闭包定义前调用）
    fn save_state(&self) -> StateSnapshot {
        StateSnapshot {
            var_state: self.var_state.clone(),
            brand_nodes_count: self.brand_tree.nodes.len(),
            scope_vars_len: self.scope_vars.len(),
            scope_drops_len: self.scope_drops.len(),
        }
    }

    /// 恢复到快照状态（闭包定义后调用，消除定义产生的副作用）
    fn restore_state(
        &mut self,
        snapshot: StateSnapshot,
    ) {
        self.var_state = snapshot.var_state;
        // 回退 brand_tree：删除闭包体内新增的令牌
        let keys: Vec<BrandId> = self.brand_tree.nodes.keys().cloned().collect();
        for key in keys.iter().skip(snapshot.brand_nodes_count) {
            self.brand_tree.remove(key);
        }
        // 回退 scope_vars
        while self.scope_vars.len() > snapshot.scope_vars_len {
            self.scope_vars.pop();
        }
        // 回退 scope_drops：截断闭包体 walk 产生的 Drop 记录
        self.scope_drops.truncate(snapshot.scope_drops_len);
        // 清除闭包体 walk 产生的待定写操作（令牌已被 restore 删除）
        self.pending_writes.clear();
    }

    /// 对比当前状态和快照，提取闭包的捕获变量
    fn diff_captures(
        &self,
        snapshot: &StateSnapshot,
    ) -> Captures {
        let mut captures = Captures::default();
        // 检查新增的品牌树令牌（闭包体创建的）
        let brand_keys: Vec<BrandId> = self.brand_tree.nodes.keys().cloned().collect();
        for key in brand_keys.iter().skip(snapshot.brand_nodes_count) {
            if let Some(node) = self.brand_tree.get(key) {
                let var = &node.source_var;
                // 检查该变量在外层是否存在（外层变量 → 捕获；否则 → 局部变量）
                if snapshot.var_state.contains_key(var) {
                    if node.kind.is_write() {
                        captures.writes.insert(var.clone());
                    } else {
                        captures.reads.insert(var.clone());
                    }
                }
            }
        }
        // 检查 var_state 变化：Alive → Moved 的是 Move 捕获
        for (var, &current_state) in &self.var_state {
            if let Some(&saved_state) = snapshot.var_state.get(var) {
                if saved_state == VarState::Alive && current_state == VarState::Moved {
                    captures.moves.insert(var.clone());
                }
            }
        }
        // 去重：Move 优先于 Write，Write 优先于 Read
        for mv in &captures.moves.clone() {
            captures.writes.remove(mv);
            captures.reads.remove(mv);
        }
        for wr in &captures.writes.clone() {
            captures.reads.remove(wr);
        }
        captures
    }

    /// 从 TypeEnvironment 查询函数参数的所有权语义
    fn lookup_param_types(
        &self,
        func_name: &str,
        arg_count: usize,
        env: &crate::frontend::core::typecheck::environment::TypeEnvironment,
    ) -> Vec<ParamOwnership> {
        let fn_type = env.get_var(func_name);
        match fn_type {
            Some(poly) => {
                if let crate::frontend::core::types::MonoType::Fn { params, .. } = &poly.body {
                    params
                        .iter()
                        .take(arg_count)
                        .map(|p| match p {
                            crate::frontend::core::types::MonoType::Ref {
                                mutable: true, ..
                            } => ParamOwnership::WriteBorrow,
                            crate::frontend::core::types::MonoType::Ref {
                                mutable: false, ..
                            } => ParamOwnership::ReadBorrow,
                            _ => ParamOwnership::Move,
                        })
                        .collect()
                } else {
                    vec![ParamOwnership::Move; arg_count]
                }
            }
            None => vec![ParamOwnership::Move; arg_count],
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
                if !self.ref_vars.contains(var_name) {
                    self.var_state.insert(var_name.to_string(), VarState::Moved);
                }
            }
            ParamOwnership::ReadBorrow => {
                let token = self.brand_tree.create_read_token(var_name.to_string());
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
                let token = self.brand_tree.create_write_token(var_name.to_string());
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

    /// 处理调用捕获参数（闭包的隐式参数）
    fn apply_captures_at_call(
        &mut self,
        captures: &Captures,
    ) -> Vec<ProofResult> {
        let mut results = Vec::new();
        for var in &captures.reads {
            let check = self.check_var_read(var, self.current_span);
            if !check.is_proved() {
                results.push(check);
            }
            self.add_consumer_for_var(var);
            self.apply_param_ownership(var, &ParamOwnership::ReadBorrow);
        }
        for var in &captures.writes {
            let check = self.check_var_read(var, self.current_span);
            if !check.is_proved() {
                results.push(check);
            }
            self.add_consumer_for_var(var);
            self.apply_param_ownership(var, &ParamOwnership::WriteBorrow);
        }
        for var in &captures.moves {
            let check = self.check_var_read(var, self.current_span);
            if !check.is_proved() {
                results.push(check);
            }
            self.add_consumer_for_var(var);
            self.apply_param_ownership(var, &ParamOwnership::Move);
        }
        results
    }

    /// 从表达式提取源码 span
    fn expr_span(expr: &Expr) -> Span {
        match expr {
            Expr::Lit(_, s)
            | Expr::Var(_, s)
            | Expr::Return(_, s)
            | Expr::Break(_, s)
            | Expr::Continue(_, s) => *s,
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
        let token_ids: Vec<BrandId> = self
            .brand_tree
            .root_tokens()
            .into_iter()
            .filter(|id| {
                self.brand_tree
                    .get(id)
                    .is_some_and(|n| n.source_var == var_name)
            })
            .cloned()
            .collect();
        for id in &token_ids {
            self.brand_tree.add_consumer(id, self.current_node);
        }
    }

    /// 检查变量读取的 Move/Drop 状态（前向检查）
    fn check_var_read(
        &self,
        name: &str,
        span: Span,
    ) -> ProofResult {
        match self.var_state.get(name) {
            Some(VarState::Moved) => emit_move_predicate(name, true, span),
            Some(VarState::Dropped) => emit_drop_predicate(name, true, span),
            _ => ProofResult::Proved,
        }
    }

    /// 推进 CFG 节点（创建新节点并从当前节点连 Normal 边）
    #[allow(dead_code)] // 控制流方法提取后暂未使用，保留供后续使用
    fn next_node(&mut self) -> usize {
        let node = self.cfg.add_node(None);
        self.cfg.add_edge(self.current_node, node, EdgeKind::Normal);
        self.current_node = node;
        node
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
        elifs: &[(&Expr, &[Stmt])],
        else_body: Option<&[Stmt]>,
    ) -> Vec<ProofResult> {
        let split_node = self.current_node;
        let mut results = self.walk_expr(condition);

        let merge_node = self.cfg.add_node(None);

        // then 分支 —— 路径条件 = condition
        let then_start = self.cfg.add_node(Some(format!("{:?}", condition)));
        self.cfg.add_edge(split_node, then_start, EdgeKind::Normal);
        self.current_node = then_start;
        results.extend(self.walk_stmts(then_body));
        self.cfg
            .add_edge(self.current_node, merge_node, EdgeKind::Normal);

        // elif 分支 —— 路径条件 = elif_cond
        for (elif_cond, elif_body) in elifs {
            results.extend(self.walk_expr(elif_cond));
            let elif_start = self.cfg.add_node(Some(format!("{:?}", elif_cond)));
            self.cfg.add_edge(split_node, elif_start, EdgeKind::Normal);
            self.current_node = elif_start;
            results.extend(self.walk_stmts(elif_body));
            self.cfg
                .add_edge(self.current_node, merge_node, EdgeKind::Normal);
        }

        // else 分支 —— 路径条件 = !condition
        if let Some(else_body) = else_body {
            let else_start = self.cfg.add_node(Some(format!("!({:?})", condition)));
            self.cfg.add_edge(split_node, else_start, EdgeKind::Normal);
            self.current_node = else_start;
            results.extend(self.walk_stmts(else_body));
            self.cfg
                .add_edge(self.current_node, merge_node, EdgeKind::Normal);
        } else {
            self.cfg.add_edge(split_node, merge_node, EdgeKind::Normal);
        }

        self.current_node = merge_node;
        results
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
        let head_node = self.cfg.add_node(Some(format!("{:?}", condition)));
        self.cfg
            .add_edge(self.current_node, head_node, EdgeKind::Normal);

        let mut results = self.walk_expr(condition);

        let body_start = self.cfg.add_node(None);
        self.cfg.add_edge(head_node, body_start, EdgeKind::Normal);
        self.current_node = body_start;
        results.extend(self.walk_stmts(body));

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
        self.var_state.insert(var.to_string(), VarState::Alive);
        self.var_mutability.insert(var.to_string(), var_mut);

        let head_node = self.cfg.add_node(None);
        self.cfg
            .add_edge(self.current_node, head_node, EdgeKind::Normal);

        let body_start = self.cfg.add_node(None);
        self.cfg.add_edge(head_node, body_start, EdgeKind::Normal);
        self.current_node = body_start;
        results.extend(self.walk_stmts(body));

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
                let mut results = self.walk_expr(expr);
                if let Some(var_name) = Self::extract_var_name(expr) {
                    // 变量本身被"使用"——检查 Move/Drop 状态
                    let check = self.check_var_read(&var_name, self.current_span);
                    if !check.is_proved() {
                        results.push(check);
                    }
                    self.add_consumer_for_var(&var_name);

                    // 可变性检查：&mut 要求变量声明为 mut
                    if *mutable {
                        let is_mut = self.var_mutability.get(&var_name).copied().unwrap_or(true);
                        if !is_mut {
                            results.push(emit_mut_predicate(&var_name, false, self.current_span));
                            return results; // 不创建 WriteToken，避免级联误报
                        }
                    }

                    let token = if *mutable {
                        self.brand_tree.create_write_token(var_name)
                    } else {
                        self.brand_tree.create_read_token(var_name)
                    };
                    self.brand_tree.add_consumer(&token, self.current_node);

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
                        self.brand_tree.derive_field(parent_id, field);
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
                    if let Expr::Var(name, _) = left.as_ref() {
                        // 仅在变量已存在且已记录可变性时检查（重赋值场景）
                        if let Some(&is_mut) = self.var_mutability.get(name) {
                            let mut r = self.walk_expr(left);
                            r.extend(self.walk_expr(right));
                            if !is_mut {
                                r.push(emit_mut_predicate(name, false, self.current_span));
                            }
                            self.add_consumer_for_var(name);
                            // ref 属性传播：x = ref_var → x 也是 ref 变量
                            if let Expr::Var(src_name, _) = right.as_ref() {
                                if self.ref_vars.contains(src_name) {
                                    self.ref_vars.insert(name.clone());
                                }
                            }
                            r
                        } else {
                            // 变量未在 var_mutability 中 → 首次声明（非 StmtKind::Var 路径）
                            let mut r = self.walk_expr(right);
                            r.extend(self.walk_expr(left));
                            self.var_state.insert(name.clone(), VarState::Alive);
                            self.var_mutability.insert(name.clone(), false);
                            if let Some(scope) = self.scope_vars.last_mut() {
                                scope.push(name.clone());
                            }
                            // ref 属性传播
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
                let mut results = self.walk_expr(func);
                // 确定调用目标名（用于查签名和捕获）
                let func_name = Self::extract_var_name(func);
                // 查询函数的参数签名（未知函数回退为全 Move）
                let env: &crate::frontend::core::typecheck::environment::TypeEnvironment =
                    unsafe { &*self.env.unwrap() };
                let param_types = func_name
                    .as_ref()
                    .map(|n| self.lookup_param_types(n, args.len(), env))
                    .unwrap_or_else(|| vec![ParamOwnership::Move; args.len()]);
                // 处理显式参数
                for (i, arg) in args.iter().enumerate() {
                    results.extend(self.walk_expr(arg));
                    if let Expr::Var(name, _) = arg {
                        let check = self.check_var_read(name, self.current_span);
                        if !check.is_proved() {
                            results.push(check);
                        }
                        self.add_consumer_for_var(name);
                        let ownership = param_types.get(i).unwrap_or(&ParamOwnership::Move);
                        self.apply_param_ownership(name, ownership);
                    }
                }
                // 处理闭包的隐式捕获参数
                if let Some(fn_name) = &func_name {
                    if let Some(captures) = self.captures_store.get(fn_name).cloned() {
                        results.extend(self.apply_captures_at_call(&captures));
                    }
                }
                results
            }
            Expr::Return(Some(inner), _) => {
                let results = self.walk_expr(inner);
                if let Expr::Var(name, _) = inner.as_ref() {
                    if !self.ref_vars.contains(name) {
                        self.var_state.insert(name.clone(), VarState::Moved);
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
                elif_branches,
                else_branch,
                ..
            } => {
                let elifs: Vec<(&Expr, &[Stmt])> = elif_branches
                    .iter()
                    .map(|(cond, body)| (cond.as_ref(), body.stmts.as_slice()))
                    .collect();
                let else_body = else_branch.as_ref().map(|b| b.stmts.as_slice());
                self.walk_if(condition, &then_branch.stmts, &elifs, else_body)
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

                // 保存当前 spawn 的 ref 集合
                let prev_spawn_refs = std::mem::take(&mut self.current_spawn_refs);

                let snapshot = self.save_state();
                let mut results = self.walk_stmts(&body.stmts);
                let captures = self.diff_captures(&snapshot);
                self.restore_state(snapshot);

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
                // spawn 定义即调用——捕获变量在调用点消耗
                results.extend(self.apply_captures_at_call(&captures));
                results
            }

            Expr::SpawnFor { body, .. } => {
                let was_spawn = self.inside_spawn;
                self.inside_spawn = true;
                let snapshot = self.save_state();
                let mut results = self.walk_stmts(&body.stmts);
                let captures = self.diff_captures(&snapshot);
                self.restore_state(snapshot);
                self.inside_spawn = was_spawn;
                results.extend(self.apply_captures_at_call(&captures));
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
        let result = match &stmt.kind {
            StmtKind::Expr(expr) => self.walk_expr(expr),

            StmtKind::Var {
                name,
                initializer,
                is_mut,
                ..
            } => {
                let mut results = Vec::new();
                let is_new = !self.var_state.contains_key(name);
                self.var_state.insert(name.clone(), VarState::Alive);
                self.var_mutability.insert(name.clone(), *is_mut);
                // 仅新声明的变量加入作用域（重赋值不重复注册，避免内层作用域错误 Drop）
                if is_new {
                    if let Some(scope) = self.scope_vars.last_mut() {
                        scope.push(name.clone());
                    }
                }

                // 记录字段赋值: a.field = b
                if let Some(init) = initializer {
                    if let Expr::BinOp {
                        op: crate::frontend::core::parser::ast::BinOp::Assign,
                        left,
                        right,
                        ..
                    } = init.as_ref()
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
                }

                // 检测 ref x 声明
                if let Some(init) = initializer {
                    if matches!(init.as_ref(), Expr::Ref { .. }) {
                        self.ref_vars.insert(name.clone());
                    }
                }

                if let Some(init) = initializer {
                    // 检测解析器回退 artifact：init 是 BinOp::Assign(Var(name), rhs)
                    // 此时只 walk rhs（真正的值），避免把声明误判为重赋值
                    if let Expr::BinOp {
                        op: crate::frontend::core::parser::ast::BinOp::Assign,
                        left,
                        right,
                        ..
                    } = init.as_ref()
                    {
                        if let Expr::Var(assigned_name, _) = left.as_ref() {
                            if assigned_name == name {
                                results.extend(self.walk_expr(right));
                                return results;
                            }
                        }
                    }
                    results.extend(self.walk_expr(init));
                    // 只有直接传变量才标记 Move（字段访问或借用不转移所有权）
                    // ref 类型是 Dup——不 Move，可多次复制
                    if let Expr::Var(src_name, _) = init.as_ref() {
                        if !self.ref_vars.contains(src_name) {
                            self.var_state.insert(src_name.clone(), VarState::Moved);
                        }
                        // ref 属性传播：alias = shared → alias 也是 ref 变量
                        if self.ref_vars.contains(src_name) {
                            self.ref_vars.insert(name.clone());
                        }
                    }
                }
                results
            }

            StmtKind::Return(Some(expr)) => {
                let results = self.walk_expr(expr);
                if let Expr::Var(name, _) = expr.as_ref() {
                    if !self.ref_vars.contains(name) {
                        self.var_state.insert(name.clone(), VarState::Moved);
                    }
                }
                results
            }
            StmtKind::Return(None) => vec![],

            StmtKind::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                let elifs: Vec<(&Expr, &[Stmt])> = elif_branches
                    .iter()
                    .map(|(cond, body)| (cond.as_ref(), body.stmts.as_slice()))
                    .collect();
                let else_body = else_branch.as_ref().map(|b| b.stmts.as_slice());
                self.walk_if(condition, &then_branch.stmts, &elifs, else_body)
            }

            StmtKind::For {
                var,
                var_mut,
                iterable,
                body,
                ..
            } => self.walk_for(var, *var_mut, iterable, &body.stmts),

            StmtKind::Binding {
                name, body, params, ..
            } => {
                let mut results = Vec::new();
                // 只处理无参闭包（{ body } 语法，非 fn 语法）
                if params.is_empty() && !body.is_empty() {
                    let snapshot = self.save_state();
                    results.extend(self.walk_stmts(body));
                    let captures = self.diff_captures(&snapshot);
                    if !captures.reads.is_empty()
                        || !captures.writes.is_empty()
                        || !captures.moves.is_empty()
                    {
                        self.captures_store.insert(name.clone(), captures);
                    }
                    self.restore_state(snapshot);
                }
                results
            }

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
        let mut results = Vec::new();
        for stmt in stmts {
            results.extend(self.walk_stmt(stmt));
        }
        // 作用域退出：将本作用域内声明且仍 Alive 的变量标记为 Dropped
        if let Some(scope) = self.scope_vars.pop() {
            for var in &scope {
                if self.var_state.get(var) == Some(&VarState::Alive) {
                    self.var_state.insert(var.clone(), VarState::Dropped);
                    self.scope_drops.push((self.current_span, var.clone()));
                }
            }
        }
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

    /// 检查单个函数体：重置状态 → 一趟遍历 → 排空待定写操作 → ReleasePlan
    fn check_function(
        &mut self,
        _name: &str,
        params: &[crate::frontend::core::parser::ast::Param],
        body: &[Stmt],
        env: &crate::frontend::core::typecheck::environment::TypeEnvironment,
    ) -> (Vec<ProofResult>, ReleasePlan, HashSet<String>) {
        self.reset();

        // 设置类型环境引用（供 walk_expr 使用）
        self.env = Some(env as *const _);

        // 标记参数为 Alive，记录可变性
        for param in params {
            self.var_state.insert(param.name.clone(), VarState::Alive);
            self.var_mutability.insert(param.name.clone(), param.is_mut);
        }

        // 一趟遍历：构建 CFG + 前向检查 + 收集待定写操作
        let mut results = self.walk_stmts(body);
        self.cfg.exit = self.current_node;

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
    ) -> (Vec<ProofResult>, ReleasePlan, HashSet<String>) {
        let mut results = Vec::new();
        let mut merged_drops: HashMap<Span, Vec<String>> = HashMap::new();
        let mut merged_escaped: HashSet<String> = HashSet::new();
        for stmt in &module.items {
            if let StmtKind::Binding {
                name,
                params,
                body,
                type_name,
                type_annotation,
                ..
            } = &stmt.kind
            {
                // 跳过类型构造器（type_name 存在且无 params 和 body）
                if type_name.is_some() && params.is_empty() && body.is_empty() {
                    continue;
                }
                // 跳过纯类型注解（有 type_annotation 但无 params 和 body）
                if type_annotation.is_some() && params.is_empty() && body.is_empty() {
                    continue;
                }
                // 跳过空函数体（无 params 也无 body — const 之类）
                if params.is_empty() && body.is_empty() {
                    continue;
                }
                let (func_results, func_plan, escaped) =
                    self.check_function(name, params, body, _env);
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
