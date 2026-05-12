//! 特质系统模块
//!
//! 实现 RFC-011 特质系统。合并自 type_level 的 trait 相关功能：
//! - bounds: Trait bounds (原 trait_bounds.rs)
//! - auto_derive: 自动派生
//! - inheritance: 特质继承
//! - std_traits: 标准库特质
//! - impl_check: 方法绑定检查

pub mod auto_derive;
pub mod bounds;
pub mod coherence;
pub mod impl_check;
pub mod inheritance;
pub mod object_safety;
pub mod resolution;
pub mod solver;
pub mod std_traits;

// 重新导出
pub use solver::{TraitSolver, TraitConstraint};
pub use coherence::{CoherenceChecker, OrphanChecker};
pub use object_safety::{ObjectSafetyChecker, ObjectSafetyError};
pub use resolution::{TraitResolver, TraitResolutionError};

pub use crate::util::diagnostic::Result;
