//! Scheduler 单元测试
//!
//! 测试任务调度器的配置、任务状态和调度行为

use crate::runtime::scheduler::{Scheduler, SchedulerConfig, Task, TaskId, TaskPriority, TaskState};

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
        assert_eq!(TaskState::Cancelled as u8, 4);
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
mod task_tests {
    use super::*;

    #[test]
    fn test_task_new() {
        let task = Task::new(TaskId(1), TaskPriority::Normal, 1024 * 1024);
        assert_eq!(task.id(), TaskId(1));
        assert_eq!(task.priority(), TaskPriority::Normal);
        assert_eq!(task.stack_size(), 1024 * 1024);
    }

    #[test]
    fn test_task_state() {
        let task = Task::new(TaskId(1), TaskPriority::Normal, 1024 * 1024);
        assert_eq!(task.state(), TaskState::Ready);
    }

    #[test]
    fn test_task_set_state() {
        let task = Task::new(TaskId(1), TaskPriority::Normal, 1024 * 1024);
        assert_eq!(task.state(), TaskState::Ready);
        
        task.set_state(TaskState::Running);
        assert_eq!(task.state(), TaskState::Running);
        
        task.set_state(TaskState::Finished);
        assert_eq!(task.state(), TaskState::Finished);
    }

    #[test]
    fn test_task_debug() {
        let task = Task::new(TaskId(1), TaskPriority::Normal, 1024 * 1024);
        let debug = format!("{:?}", task);
        assert!(debug.contains("Task"));
    }
}

#[cfg(test)]
mod scheduler_config_tests {
    use super::*;

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        // num_workers should be at least 1
        assert!(config.num_workers >= 1);
        assert_eq!(config.default_stack_size, 2 * 1024 * 1024);
        assert_eq!(config.steal_batch, 4);
        assert_eq!(config.max_queue_size, 1024);
    }

    #[test]
    fn test_scheduler_config_custom() {
        let config = SchedulerConfig {
            num_workers: 8,
            default_stack_size: 4 * 1024 * 1024,
            steal_batch: 8,
            max_queue_size: 2048,
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
        let scheduler = Scheduler::new();
        let _ = format!("{:?}", scheduler);
    }

    #[test]
    fn test_scheduler_default() {
        let scheduler = Scheduler::default();
        let _ = format!("{:?}", scheduler);
    }

    #[test]
    fn test_scheduler_spawn() {
        let scheduler = Scheduler::new();
        let task = Arc::new(Task::new(TaskId(1), TaskPriority::Normal, 1024 * 1024));
        scheduler.spawn(task);
        // Spawn should not panic
    }

    #[test]
    fn test_scheduler_with_config() {
        let config = SchedulerConfig {
            num_workers: 2,
            default_stack_size: 1024 * 1024,
            steal_batch: 2,
            max_queue_size: 512,
        };
        let scheduler = Scheduler::with_config(config);
        let _ = format!("{:?}", scheduler);
    }
}
