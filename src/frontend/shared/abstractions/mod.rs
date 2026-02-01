//! 抽象接口层
//!
//! 定义抽象接口和特质

pub mod parser;
pub mod trait_objects;
pub mod type_checker;

// 重新导出
pub use parser::ParserTrait;
pub use type_checker::TypeCheckerTrait;
pub use trait_objects::TraitObject;
