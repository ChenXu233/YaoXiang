//! Frontend Event System
//!
//!细粒度事件系统，为 LSP 和 IDE 集成提供支持。

mod base;
mod diagnostic;
mod phase;
mod progress;
mod subscribe;

pub use base::*;
pub use phase::*;
pub use progress::*;
pub use diagnostic::*;
pub use subscribe::*;

/// 事件元数据
#[derive(Debug, Clone)]
pub struct EventMetadata {
    /// 事件发生的时间戳
    pub timestamp: std::time::Instant,
    /// 源文件信息
    pub source_file: Option<String>,
    /// 事件序号（用于排序）
    pub sequence: u64,
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self {
            timestamp: std::time::Instant::now(),
            source_file: None,
            sequence: 0,
        }
    }
}

impl EventMetadata {
    /// 创建带有源文件的事件元数据
    pub fn with_source(source: impl Into<String>) -> Self {
        Self {
            source_file: Some(source.into()),
            ..Default::default()
        }
    }

    /// 设置事件序号
    pub fn with_sequence(
        mut self,
        seq: u64,
    ) -> Self {
        self.sequence = seq;
        self
    }
}

/// 事件类型枚举（用于类型过滤）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// 基础事件
    Base,
    /// 阶段事件
    Phase,
    /// 进度事件
    Progress,
    /// 诊断事件
    Diagnostic,
    /// 所有事件
    All,
}

impl From<&dyn Event> for EventType {
    fn from(_: &dyn Event) -> Self {
        // 默认返回 All，具体事件类型可以覆盖
        EventType::All
    }
}

use std::any::Any;

/// 事件特征
pub trait Event: Any + Send + Sync {
    /// 获取事件类型
    fn event_type(&self) -> EventType;

    /// 获取事件名称（用于日志）
    fn name(&self) -> &'static str;

    /// 获取事件元数据
    fn metadata(&self) -> &EventMetadata;

    /// 设置事件元数据
    fn set_metadata(
        &mut self,
        metadata: EventMetadata,
    );

    /// 获取关联的 span（如果有）
    fn span(&self) -> Option<crate::util::span::Span> {
        None
    }
}

/// 事件发射器 trait
pub trait EventEmitter {
    /// 发射一个事件
    fn emit<E: Event>(
        &mut self,
        event: E,
    );

    /// 发射一个带元数据的事件
    fn emit_with<E: Event>(
        &mut self,
        event: E,
        metadata: EventMetadata,
    );

    /// 获取当前事件序号
    fn next_sequence(&mut self) -> u64;
}

/// 空事件发射器（不执行任何操作）
#[derive(Debug, Default)]
pub struct NullEmitter;

impl EventEmitter for NullEmitter {
    fn emit<E: Event>(
        &mut self,
        _event: E,
    ) {
    }

    fn emit_with<E: Event>(
        &mut self,
        _event: E,
        _metadata: EventMetadata,
    ) {
    }

    fn next_sequence(&mut self) -> u64 {
        0
    }
}
