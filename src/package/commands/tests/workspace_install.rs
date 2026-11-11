//! 测试工作空间 install（RFC-014c §lockfile 共享，Phase 6b）
//!
//! 覆盖:
//! - 成员目录执行 install → 整个 workspace 装到根
//! - 根 lockfile 唯一（成员目录无 lock）
//! - 共享 vendor 在根
