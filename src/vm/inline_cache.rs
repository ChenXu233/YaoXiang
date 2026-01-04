//! 内联缓存实现
//!
//! 用于优化动态分发和反射调用的性能。
//! 核心机制：
//! 1. 缓存调用点的 (ReceiverType, MethodAddr) 映射
//! 2. 首次调用慢（查表），后续调用快（直接跳转）
//! 3. 支持多态缓存（Polymorphic Inline Cache, PIC）

/// 内联缓存槽
///
/// 位于函数调用点附近，缓存类型-地址映射
#[derive(Debug, Clone, Copy)]
#[repr(C)]
#[derive(Default)]
pub struct InlineCacheSlot {
    /// 缓存的有效性标志
    pub valid: u8, // 0 = 无效, 1 = 单态, 2 = 多态
    /// 缓存的类型数量
    pub count: u8, // 1-4，通常为 1
    /// 缓存的插槽数据
    pub slots: [ICSlotData; 4],
}

/// 单个缓存插槽
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct ICSlotData {
    /// 接收者类型 ID
    pub receiver_type_id: u32,
    /// 方法地址偏移（从代码段起始）
    pub method_offset: u32,
    /// 方法的虚表索引（用于虚调用）
    pub vtable_index: u16,
    /// 保留对齐
    pub _reserved: u16,
}

/// 内联缓存管理器
#[derive(Debug)]
pub struct InlineCacheManager {
    /// 缓存池
    cache_pool: Vec<InlineCacheSlot>,
    /// 缓存大小配置
    config: ICConfig,
}

/// 内联缓存配置
#[derive(Debug, Clone)]
pub struct ICConfig {
    /// 是否启用内联缓存
    pub enabled: bool,
    /// 单态缓存大小
    pub monomorphic_size: usize,
    /// 多态缓存大小
    pub polymorphic_size: usize,
    /// 缓存失效策略
    pub invalidation_strategy: ICInvalidation,
}

impl Default for ICConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            monomorphic_size: 1,
            polymorphic_size: 4,
            invalidation_strategy: ICInvalidation::TypeChange,
        }
    }
}

/// 缓存失效策略
#[derive(Debug, Clone, Copy)]
pub enum ICInvalidation {
    /// 类型变化时失效
    TypeChange,
    /// 总是失效
    Always,
    /// 从不失效
    Never,
}

/// 内联缓存查找结果
pub enum ICCheckResult {
    /// 缓存命中，直接跳转
    Hit {
        /// 方法地址偏移
        method_offset: u32,
        /// 虚表索引
        vtable_index: u16,
    },
    /// 缓存未命中，需要查元数据
    Miss {
        /// 原因
        reason: ICMissReason,
    },
    /// 缓存失效
    Invalid,
}

/// 缓存未命中原因
#[derive(Debug, Clone, Copy)]
pub enum ICMissReason {
    /// 首次调用
    FirstCall,
    /// 多态缓存已满
    PolymorphicOverflow,
    /// 类型不匹配
    TypeMismatch,
}

impl InlineCacheManager {
    pub fn new(config: ICConfig) -> Self {
        Self {
            cache_pool: Vec::new(),
            config,
        }
    }

    /// 分配一个新的缓存槽
    pub fn allocate_slot(&mut self) -> usize {
        let idx = self.cache_pool.len();
        self.cache_pool.push(InlineCacheSlot::default());
        idx
    }

    /// 获取缓存槽
    pub fn get_slot(&self, index: usize) -> Option<&InlineCacheSlot> {
        self.cache_pool.get(index)
    }

    /// 获取可变缓存槽
    pub fn get_slot_mut(&mut self, index: usize) -> Option<&mut InlineCacheSlot> {
        self.cache_pool.get_mut(index)
    }

    /// 检查缓存
    pub fn check_cache(&self, slot_idx: usize, receiver_type_id: u32) -> ICCheckResult {
        let slot = match self.get_slot(slot_idx) {
            Some(s) => s,
            None => return ICCheckResult::Invalid,
        };

        if slot.valid == 0 {
            return ICCheckResult::Miss {
                reason: ICMissReason::FirstCall,
            };
        }

        // 查找匹配的插槽
        for i in 0..slot.count as usize {
            if slot.slots[i].receiver_type_id == receiver_type_id {
                return ICCheckResult::Hit {
                    method_offset: slot.slots[i].method_offset,
                    vtable_index: slot.slots[i].vtable_index,
                };
            }
        }

        // 未找到，检查是否可以扩展为多态
        if slot.valid == 1 && slot.count < self.config.polymorphic_size as u8 {
            return ICCheckResult::Miss {
                reason: ICMissReason::PolymorphicOverflow,
            };
        }

        ICCheckResult::Miss {
            reason: ICMissReason::TypeMismatch,
        }
    }

    /// 更新缓存
    pub fn update_cache(
        &mut self,
        slot_idx: usize,
        receiver_type_id: u32,
        method_offset: u32,
        vtable_index: u16,
    ) {
        let config = self.config.clone();
        let slot = match self.get_slot_mut(slot_idx) {
            Some(s) => s,
            None => return,
        };

        if slot.valid == 0 {
            // 首次初始化
            slot.valid = 1;
            slot.count = 1;
            slot.slots[0] = ICSlotData {
                receiver_type_id,
                method_offset,
                vtable_index,
                _reserved: 0,
            };
        } else if (slot.count as usize) < config.polymorphic_size {
            // 扩展多态缓存
            slot.valid = 2; // 多态
            slot.slots[slot.count as usize] = ICSlotData {
                receiver_type_id,
                method_offset,
                vtable_index,
                _reserved: 0,
            };
            slot.count += 1;
        } else {
            // 缓存已满，替换第一个（LRU 简化版）
            // 实际策略可能是随机替换或替换最旧的，这里简化为替换 slot 0
            slot.slots[0] = ICSlotData {
                receiver_type_id,
                method_offset,
                vtable_index,
                _reserved: 0,
            };
        }
    }

    /// 使缓存失效
    pub fn invalidate_cache(&mut self, slot_idx: usize) {
        if let Some(slot) = self.get_slot_mut(slot_idx) {
            slot.valid = 0;
            slot.count = 0;
        }
    }

    /// 获取缓存池大小
    pub fn pool_size(&self) -> usize {
        self.cache_pool.len()
    }

    /// 批量使缓存失效（根据类型 ID）
    pub fn invalidate_by_type(&mut self, _type_id: u32) {
        // 在多态缓存场景下，如果某类型的方法实现发生变化，
        // 需要使所有包含该类型的缓存失效
        // 当前简化实现：使所有缓存失效
        for slot in &mut self.cache_pool {
            slot.valid = 0;
            slot.count = 0;
        }
    }
}
