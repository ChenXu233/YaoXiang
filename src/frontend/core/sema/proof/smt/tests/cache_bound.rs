//! SMT 查询缓存上界与命中率计数测试 — #375 / WBS 4.4.2
//!
//! #375: 缓存无上限随编译过程无界增长——触顶整体清空（evict_if_full）。
//! WBS 4.4.2: 「缓存命中率可观测」——cache_hits/cache_misses 计数。

#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use crate::frontend::core::sema::proof::smt::ast::{SMTResult};
#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(target_arch = "wasm32"))]
use crate::frontend::core::sema::proof::smt::z3_backend::{evict_if_full, CACHE_CAP};

/// 触顶清空边界：恰满 CAP 时下一次写入路径先清空，缓存永不越过上限
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_cache_eviction_clears_at_cap() {
    let mut cache: HashMap<u64, SMTResult> = HashMap::new();
    // 填到 CAP-1：未触顶，不清
    for i in 0..CACHE_CAP - 1 {
        cache.insert(i as u64, SMTResult::Unsat);
    }
    evict_if_full(&mut cache);
    assert_eq!(cache.len(), CACHE_CAP - 1, "未触顶不得清空");
    // 第 CAP 条进入后再次触发：必须整体清空（#375 的无界增长在此截断）
    cache.insert(u64::MAX, SMTResult::Unsat);
    assert_eq!(cache.len(), CACHE_CAP);
    evict_if_full(&mut cache);
    assert!(
        cache.is_empty(),
        "达到 CACHE_CAP 必须整体清空，实际 {} 条",
        cache.len()
    );
}

/// Unknown 不入缓存：资源依赖的非确定性结果不得固化（#375 方向4 补遗，
/// D58 单例化硬前置）。此前 solve 对任何结果 insert——一次超时会把
/// 「首次求解时的机器负载」固化为进程级事实。
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn test_unknown_result_is_never_cached() {
    use crate::frontend::core::sema::proof::smt::z3_backend::should_cache;

    // Arrange: 三种结果的典型形态
    let unknown = SMTResult::Unknown {
        reason: "timeout".to_string(),
    };
    let sat = SMTResult::Sat {
        model: crate::frontend::core::sema::proof::smt::ast::SMTModel {
            assignments: vec![("x".to_string(), "1".to_string())],
        },
    };
    let unsat = SMTResult::Unsat;

    // Act
    let unknown_cacheable = should_cache(&unknown);
    let sat_cacheable = should_cache(&sat);
    let unsat_cacheable = should_cache(&unsat);

    // Assert
    assert!(!unknown_cacheable, "Unknown（超时/能力外）不得入缓存");
    assert!(sat_cacheable, "Sat 是确定性结果，应可缓存");
    assert!(unsat_cacheable, "Unsat 是确定性结果，应可缓存");
}
