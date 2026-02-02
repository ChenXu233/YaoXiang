//! 特质系统模块
//!
//! 实现 RFC-011 特质系统

pub mod coherence;
pub mod object_safety;
pub mod resolution;
pub mod solver;

// 重新导出
pub use solver::{TraitSolver, TraitConstraint};
pub use coherence::{CoherenceChecker, OrphanChecker};
pub use object_safety::{ObjectSafetyChecker, ObjectSafetyError};
pub use resolution::{TraitResolver, TraitResolutionError};

pub use crate::util::diagnostic::Result;
