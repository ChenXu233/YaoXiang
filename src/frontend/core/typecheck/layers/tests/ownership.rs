//! 所有权检查测试 — 基于 RFC-009 + RFC-009a
//!
//! RFC-009  §2.7: 品牌机制
//! RFC-009a §品牌树: 令牌派生关系与冲突检测
//! RFC-009a §系统谓词清单: 5 种命题
//! RFC-009a §用例分析: 线性代码 / if-else / 循环

use crate::frontend::core::typecheck::layers::ownership::{BrandId, BrandTree, TokenKind};

// ── BrandId 前缀匹配 ──────────────────────────────────

#[test]
fn test_prefix_matching() {
    // Arrange
    let root = BrandId::root(0);
    let field = root.derive_field("x");
    let deep = field.derive_field("y");

    // Act & Assert
    assert!(root.is_prefix_of(&field));
    assert!(root.is_prefix_of(&deep));
    assert!(field.is_prefix_of(&deep));
    assert!(!field.is_prefix_of(&root));
}

#[test]
fn test_different_roots_no_prefix_relation() {
    let a = BrandId::root(0);
    let b = BrandId::root(1);
    assert!(!a.is_prefix_of(&b));
    assert!(!b.is_prefix_of(&a));
}

#[test]
fn test_root_id_extraction() {
    assert_eq!(BrandId::root(42).root_id(), "#42");
    assert_eq!(BrandId::root(42).derive_field("x").root_id(), "#42");
    assert_eq!(
        BrandId::root(42)
            .derive_field("x")
            .derive_field("y")
            .root_id(),
        "#42"
    );
}

// ── 冲突判断（RFC-009a §品牌树） ──────────────────────

#[test]
fn test_read_vs_read_no_conflict() {
    let mut tree = BrandTree::new();
    let r1 = tree.create_read_token("x".into());
    let r2 = tree.create_read_token("x".into());
    assert!(!tree.conflicts(&r1, &r2));
}

#[test]
fn test_read_vs_write_conflict() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let w = tree.create_write_token("x".into());
    assert!(tree.conflicts(&r, &w));
}

#[test]
fn test_write_vs_write_conflict() {
    let mut tree = BrandTree::new();
    let w1 = tree.create_write_token("x".into());
    let w2 = tree.create_write_token("x".into());
    assert!(tree.conflicts(&w1, &w2));
}

#[test]
fn test_different_source_no_conflict() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let w = tree.create_write_token("y".into());
    assert!(!tree.conflicts(&r, &w));
}

#[test]
fn test_derived_read_vs_write_root_conflict() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let r_field = tree.derive_field(&r, "field").unwrap();
    let w = tree.create_write_token("x".into());
    // 同源 + 有写 = 冲突，与派生关系无关
    assert!(tree.conflicts(&r_field, &w));
}

#[test]
fn test_derived_read_vs_derived_read_no_conflict() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let rx = tree.derive_field(&r, "a").unwrap();
    let ry = tree.derive_field(&r, "b").unwrap();
    // 同源但都读 → 不冲突
    assert!(!tree.conflicts(&rx, &ry));
}

// ── 级联删除 ──────────────────────────────────────────

#[test]
fn test_remove_cascades_to_children() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let r_field = tree.derive_field(&r, "field").unwrap();
    assert!(tree.get(&r_field).is_some());

    tree.remove(&r);
    assert!(tree.get(&r).is_none());
    assert!(tree.get(&r_field).is_none());
}

#[test]
fn test_remove_cleans_up_parent_children_set() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let child = tree.derive_field(&r, "field").unwrap();
    assert!(tree.get(&r).unwrap().children.contains(&child));

    tree.remove(&child);
    assert!(!tree.get(&r).unwrap().children.contains(&child));
}

// ── 消费者追踪 ────────────────────────────────────────

#[test]
fn test_consumer_tracking() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    tree.add_consumer(&r, 3);
    tree.add_consumer(&r, 5);

    let c = tree.consumers(&r);
    assert!(c.contains(&3));
    assert!(c.contains(&5));
    assert_eq!(c.len(), 2);
}

#[test]
fn test_consumer_unknown_token_returns_empty() {
    let tree = BrandTree::new();
    assert!(tree.consumers(&BrandId::root(999)).is_empty());
}

// ── conflicting_with ──────────────────────────────────

#[test]
fn test_conflicting_with_returns_all_conflicts() {
    let mut tree = BrandTree::new();
    let r = tree.create_read_token("x".into());
    let w1 = tree.create_write_token("x".into());
    let w2 = tree.create_write_token("x".into());

    let conflicts = tree.conflicting_with(&r);
    assert_eq!(conflicts.len(), 2);
}
