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
use super::super::proof::context::ProofContext;
use super::super::proof::verdict::ProofResult;

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
    /// 消费该令牌的 DAG 节点索引。
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
