//! Runtime support for YaoXiang
//!
//! Per RFC-008: Three-tier runtime architecture
//! - Embedded Runtime: Immediate executor, no DAG, sync execution
//! - Standard Runtime: DAG scheduler, lazy evaluation, async/concurrent
//! - Full Runtime: + WorkStealer, parallel optimization
//!
//! Per RFC-009: Memory management uses Arc (ref keyword in YaoXiang)
//! - ❌ No GC - reference counting via Arc
//! - Task boundary is the leak boundary

pub mod task;

pub use task::{
    Task, TaskId, TaskContext, TaskPriority, TaskConfig, TaskSpawner, TaskState, Scheduler,
    SchedulerStats, RuntimeError,
};
