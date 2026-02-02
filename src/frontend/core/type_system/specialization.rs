//! 泛型特化模块
//!
//! 实现泛型函数的特化和实例化

use crate::util::span::Span;
use std::collections::HashMap;

/// 特化器
///
/// 负责将多态类型特化为具体类型
#[derive(Debug)]
pub struct Specializer {
    /// 特化缓存
    cache: HashMap<String, Vec<MonoType>>,
    /// 下一个特化 ID
    next_id: usize,
}

impl Specializer {
    /// 创建新的特化器
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            next_id: 0,
        }
    }

    /// 重置特化器
    pub fn reset(&mut self) {
        self.cache.clear();
        self.next_id = 0;
    }
}

/// 特化错误
#[derive(Debug, Clone)]
pub enum SpecializationError {
    CannotSpecialize {
        reason: String,
        span: Span,
    },
    AmbiguousInstantiation {
        candidates: Vec<String>,
        span: Span,
    },
}
