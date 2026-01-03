//! GC 单元测试
//!
//! 测试垃圾回收器的配置、状态和行为

use crate::runtime::gc::{GC, GCConfig, GCState};
use std::time::Duration;

#[cfg(test)]
mod gc_config_tests {
    use super::*;

    #[test]
    fn test_gc_config_default() {
        let config = GCConfig::default();
        assert_eq!(config.initial_heap_size, 16 * 1024 * 1024);
        assert_eq!(config.max_heap_size, 256 * 1024 * 1024);
        assert!((config.young_ratio - 0.5).abs() < 0.001);
        assert_eq!(config.collection_threshold, 1024 * 1024);
    }

    #[test]
    fn test_gc_config_custom() {
        let config = GCConfig {
            initial_heap_size: 8 * 1024 * 1024,
            max_heap_size: 128 * 1024 * 1024,
            young_ratio: 0.3,
            collection_threshold: 512 * 1024,
            max_pause: Duration::from_millis(5),
        };
        assert_eq!(config.initial_heap_size, 8 * 1024 * 1024);
        assert_eq!(config.max_heap_size, 128 * 1024 * 1024);
    }

    #[test]
    fn test_gc_config_clone() {
        let config = GCConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.initial_heap_size, config.initial_heap_size);
    }
}

#[cfg(test)]
mod gc_state_tests {
    use super::*;

    #[test]
    fn test_gc_state_values() {
        assert_eq!(GCState::Idle as u8, 0);
        assert_eq!(GCState::Marking as u8, 1);
        assert_eq!(GCState::Sweeping as u8, 2);
        assert_eq!(GCState::Paused as u8, 3);
    }

    #[test]
    fn test_gc_state_partial_eq() {
        assert_eq!(GCState::Idle, GCState::Idle);
        assert_ne!(GCState::Idle, GCState::Marking);
    }

    #[test]
    fn test_gc_state_debug() {
        let debug = format!("{:?}", GCState::Idle);
        assert!(debug.contains("Idle"));
    }
}

#[cfg(test)]
mod gc_tests {
    use super::*;

    #[test]
    fn test_gc_new() {
        let gc = GC::new(GCConfig::default());
        // GC should be created successfully
        let _ = format!("{:?}", gc);
    }

    #[test]
    fn test_gc_default() {
        let gc = GC::default();
        let _ = format!("{:?}", gc);
    }

    #[test]
    fn test_gc_collect() {
        let mut gc = GC::default();
        gc.collect();
        // After collect, state should return to Idle
    }

    #[test]
    fn test_gc_should_collect() {
        let gc = GC::default();
        // Initially should return false (TODO implementation)
        assert!(!gc.should_collect());
    }

    #[test]
    fn test_gc_pause_tracking() {
        let gc = GC::default();
        let total = gc.total_pause();
        let last = gc.last_pause();
        // Initial pause times should be zero
        assert_eq!(total, Duration::ZERO);
        assert_eq!(last, Duration::ZERO);
    }
}
