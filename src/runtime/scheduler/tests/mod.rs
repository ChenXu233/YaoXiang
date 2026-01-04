//! Scheduler 单元测试
//!
//! 测试任务调度器的配置、任务状态和调度行为

// 声明子模块
mod task;
mod queue;
mod work_stealer;
mod flow_scheduler;

use crate::runtime::scheduler::{FlowScheduler, SchedulerConfig, Task, TaskId, TaskPriority, TaskState};

#[cfg(test)]
mod task_id_tests {
    use super::*;

    #[test]
    fn test_task_id_new() {
        let id = TaskId(1);
        assert_eq!(id.0, 1);
    }

    #[test]
    fn test_task_id_clone() {
        let id = TaskId(42);
        let cloned = id.clone();
        assert_eq!(id.0, cloned.0);
    }

    #[test]
    fn test_task_id_partial_eq() {
        assert_eq!(TaskId(1), TaskId(1));
        assert_ne!(TaskId(1), TaskId(2));
    }

    #[test]
    fn test_task_id_debug() {
        let id = TaskId(5);
        let debug = format!("{:?}", id);
        assert!(debug.contains("5"));
    }
}

#[cfg(test)]
mod task_state_tests {
    use super::*;

    #[test]
    fn test_task_state_values() {
        assert_eq!(TaskState::Ready as u8, 0);
        assert_eq!(TaskState::Running as u8, 1);
        assert_eq!(TaskState::Waiting as u8, 2);
        assert_eq!(TaskState::Finished as u8, 3);
        assert_eq!(TaskState::Failed as u8, 4);
        assert_eq!(TaskState::Cancelled as u8, 5);
    }

    #[test]
    fn test_task_state_partial_eq() {
        assert_eq!(TaskState::Ready, TaskState::Ready);
        assert_ne!(TaskState::Ready, TaskState::Running);
    }

    #[test]
    fn test_task_state_debug() {
        let debug = format!("{:?}", TaskState::Ready);
        assert!(debug.contains("Ready"));
    }
}

#[cfg(test)]
mod task_priority_tests {
    use super::*;

    #[test]
    fn test_task_priority_values() {
        assert_eq!(TaskPriority::Low as u8, 0);
        assert_eq!(TaskPriority::Normal as u8, 1);
        assert_eq!(TaskPriority::High as u8, 2);
        assert_eq!(TaskPriority::Critical as u8, 3);
    }

    #[test]
    fn test_task_priority_ord() {
        assert!(TaskPriority::Low < TaskPriority::Normal);
        assert!(TaskPriority::Normal < TaskPriority::High);
        assert!(TaskPriority::High < TaskPriority::Critical);
    }

    #[test]
    fn test_task_priority_partial_eq() {
        assert_eq!(TaskPriority::Low, TaskPriority::Low);
        assert_ne!(TaskPriority::Low, TaskPriority::High);
    }

    #[test]
    fn test_task_priority_debug() {
        let debug = format!("{:?}", TaskPriority::High);
        assert!(debug.contains("High"));
    }
}

#[cfg(test)]
mod scheduler_config_tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        // num_workers should be at least 1
        assert!(config.num_workers >= 1);
        assert_eq!(config.default_stack_size, 2 * 1024 * 1024);
        assert_eq!(config.steal_batch, 4);
        assert_eq!(config.max_queue_size, 1024);
        assert!(config.use_work_stealing);
        assert_eq!(config.idle_timeout, Duration::from_millis(1));
        assert!(!config.enable_stats);
    }

    #[test]
    fn test_scheduler_config_custom() {
        let config = SchedulerConfig {
            num_workers: 8,
            default_stack_size: 4 * 1024 * 1024,
            steal_batch: 8,
            max_queue_size: 2048,
            use_work_stealing: true,
            idle_timeout: Duration::from_millis(5),
            enable_stats: true,
        };
        assert_eq!(config.num_workers, 8);
        assert_eq!(config.default_stack_size, 4 * 1024 * 1024);
    }

    #[test]
    fn test_scheduler_config_clone() {
        let config = SchedulerConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.num_workers, config.num_workers);
    }
}

#[cfg(test)]
mod scheduler_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_scheduler_new() {
        let scheduler = FlowScheduler::new();
        let _ = format!("{:?}", scheduler);
    }

    #[test]
    fn test_scheduler_default() {
        let scheduler = FlowScheduler::default();
        let _ = format!("{:?}", scheduler);
    }

    #[test]
    fn test_scheduler_spawn() {
        let scheduler = FlowScheduler::new();
        let task = Arc::new(Task::new(TaskId(1), TaskPriority::Normal, 1024 * 1024, || {}));
        scheduler.spawn(task);
        // Spawn should not panic
    }

    #[test]
    fn test_scheduler_with_config() {
        use std::time::Duration;

        let config = SchedulerConfig {
            num_workers: 2,
            default_stack_size: 1024 * 1024,
            steal_batch: 2,
            max_queue_size: 512,
            use_work_stealing: true,
            idle_timeout: Duration::from_millis(2),
            enable_stats: false,
        };
        let scheduler = FlowScheduler::with_config(config);
        let _ = format!("{:?}", scheduler);
    }
}
