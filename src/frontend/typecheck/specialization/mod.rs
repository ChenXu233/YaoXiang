//! 泛型特化模块
//!
//! 实现泛型函数的特化和实例化

pub mod algorithm;
pub mod instantiate;
pub mod substitution;

// 重新导出
pub use algorithm::{Specializer, SpecializationAlgorithm};
pub use substitution::{Substituter, SubstitutionResult};
pub use instantiate::{Instantiator, InstanceResult};

pub use crate::frontend::shared::error::Result;
pub use crate::frontend::core::type_system::{MonoType, PolyType, TypeVar};
