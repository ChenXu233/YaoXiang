#![allow(clippy::result_large_err)]

//! 一致性检查
//!
//! 实现特质一致性检查（语言规范 §3.5.1）
//! - 唯一性规则：同一类型对同一接口只能实现一次
//! - 孤儿规则：预留（需要模块归属信息，见 #73）

use crate::frontend::core::types::TraitTable;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

/// 一致性检查器
///
/// 检查 trait 实现的唯一性规则
pub struct CoherenceChecker<'a> {
    trait_table: &'a TraitTable,
}

impl<'a> CoherenceChecker<'a> {
    /// 创建新的检查器
    pub fn new(trait_table: &'a TraitTable) -> Self {
        Self { trait_table }
    }

    /// 执行所有一致性检查，返回发现的错误
    pub fn check(&self) -> Vec<Diagnostic> {
        let mut errors = Vec::new();
        errors.extend(self.check_conflicting_implementations());
        errors
    }

    /// 检查冲突的实现
    ///
    /// TraitTable 的 HashMap 结构已阻止真正的重复 key，
    /// 这里检查的是语义层面的冲突：
    /// - 同一类型的同一 trait 有多个实现（已被 add_impl 拦截）
    /// - 未来可扩展为泛型 overlap 检测
    fn check_conflicting_implementations(&self) -> Vec<Diagnostic> {
        // 当前 TraitTable::add_impl 已在插入时检测冲突并拒绝覆盖。
        // 此处遍历所有已注册实现，做最终验证。
        // 未来当 #73 添加 span 后，可在此报告冲突位置。
        let mut errors = Vec::new();

        for ((trait_name, for_type), _) in self.trait_table.all_implementations() {
            // 验证 trait 定义存在
            if !self.trait_table.has_trait(trait_name) {
                errors.push(ErrorCodeDefinition::conflicting_trait_impls(trait_name).build());
            }
            // 验证类型不为空
            if for_type.is_empty() {
                errors.push(ErrorCodeDefinition::conflicting_trait_impls(trait_name).build());
            }
        }

        errors
    }
}

/// 孤儿检查器
///
/// 确保 trait 实现遵循孤儿规则。
/// 当前为空实现——需要模块归属信息（#73）才能判断 impl 属于哪个模块。
pub struct OrphanChecker;

impl Default for OrphanChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl OrphanChecker {
    /// 创建新的检查器
    pub fn new() -> Self {
        Self
    }

    /// 检查孤儿实现
    pub fn check(&self) -> Vec<Diagnostic> {
        // 孤儿规则需要模块归属信息（TraitImplementation.module），
        // 当前数据结构不支持，等 #73 完成后实现。
        Vec::new()
    }
}
