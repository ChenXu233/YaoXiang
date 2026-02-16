//! 事件订阅机制

use super::{Event, EventEmitter, EventMetadata, EventType};
use std::sync::{Arc, Mutex};
use std::any::Any;

/// 事件订阅者 trait
pub trait EventSubscriber: Send + Sync {
    /// 订阅的事件类型
    fn event_types(&self) -> Vec<EventType>;

    /// 处理事件
    fn on_event(
        &self,
        event: &dyn Any,
        metadata: &EventMetadata,
    );
}

/// 包装订阅者以匹配特定事件类型
struct WrappedSubscriber<S> {
    subscriber: Arc<S>,
    event_types: Vec<EventType>,
}

impl<S: EventSubscriber> WrappedSubscriber<S> {
    fn new(subscriber: Arc<S>) -> Self {
        Self {
            event_types: subscriber.event_types(),
            subscriber,
        }
    }
}

impl<S: EventSubscriber> EventSubscriber for WrappedSubscriber<S> {
    fn event_types(&self) -> Vec<EventType> {
        self.event_types.clone()
    }

    fn on_event(
        &self,
        event: &dyn Any,
        metadata: &EventMetadata,
    ) {
        self.subscriber.on_event(event, metadata);
    }
}

/// 事件过滤器
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    include_types: Vec<EventType>,
    exclude_types: Vec<EventType>,
    include_names: Vec<&'static str>,
    exclude_names: Vec<&'static str>,
}

impl EventFilter {
    /// 创建包含所有类型的过滤器
    pub fn all() -> Self {
        Self {
            include_types: vec![EventType::All],
            ..Default::default()
        }
    }

    /// 创建只包含特定类型的过滤器
    pub fn include_only(types: impl Into<Vec<EventType>>) -> Self {
        Self {
            include_types: types.into(),
            ..Default::default()
        }
    }

    /// 排除特定类型
    pub fn exclude(
        mut self,
        types: impl Into<Vec<EventType>>,
    ) -> Self {
        self.exclude_types.extend(types.into());
        self
    }

    /// 只包含特定名称的事件
    pub fn include_names(
        mut self,
        names: impl Into<Vec<&'static str>>,
    ) -> Self {
        self.include_names = names.into();
        self
    }

    /// 排除特定名称的事件
    pub fn exclude_names(
        mut self,
        names: impl Into<Vec<&'static str>>,
    ) -> Self {
        self.exclude_names = names.into();
        self
    }

    /// 检查事件是否通过过滤器
    pub fn matches(
        &self,
        event_type: EventType,
        event_name: &'static str,
    ) -> bool {
        // 检查排除列表
        if self.exclude_types.contains(&event_type) {
            return false;
        }
        if self.exclude_names.contains(&event_name) {
            return false;
        }

        // 检查包含列表
        if self.include_types.is_empty() && self.include_names.is_empty() {
            return true;
        }
        if self.include_types.contains(&EventType::All) {
            return true;
        }
        if self.include_types.contains(&event_type) {
            return true;
        }
        if self.include_names.contains(&event_name) {
            return true;
        }

        false
    }
}

/// 事件订阅
#[derive(Debug, Clone)]
pub struct Subscription {
    id: u64,
    filter: EventFilter,
}

impl Subscription {
    pub fn new(
        id: u64,
        filter: EventFilter,
    ) -> Self {
        Self { id, filter }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn filter(&self) -> &EventFilter {
        &self.filter
    }
}

/// 订阅句柄（用于取消订阅）
#[derive(Clone)]
pub struct SubscriptionHandle {
    subscription_id: u64,
    event_bus: Arc<EventBus>,
}

impl SubscriptionHandle {
    pub fn unsubscribe(self) {
        self.event_bus.remove_subscription(self.subscription_id);
    }
}

/// 事件总线
pub struct EventBus {
    subscribers: Arc<Mutex<Vec<Box<dyn EventSubscriber>>>>,
    subscriptions: Arc<Mutex<Vec<Subscription>>>,
    next_subscription_id: Arc<Mutex<u64>>,
    sequence_counter: Arc<Mutex<u64>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
            subscriptions: Arc::new(Mutex::new(Vec::new())),
            next_subscription_id: Arc::new(Mutex::new(0)),
            sequence_counter: Arc::new(Mutex::new(0)),
        }
    }
}

impl EventBus {
    /// 创建新的事件总线
    pub fn new() -> Self {
        Self::default()
    }

    /// 订阅所有事件
    pub fn subscribe<S: EventSubscriber + 'static>(
        &self,
        subscriber: S,
    ) -> SubscriptionHandle {
        self.subscribe_with_filter(subscriber, EventFilter::all())
    }

    /// 使用过滤器订阅事件
    pub fn subscribe_with_filter<S: EventSubscriber + 'static>(
        &self,
        subscriber: S,
        filter: EventFilter,
    ) -> SubscriptionHandle {
        let id = {
            let mut counter = self.next_subscription_id.lock().unwrap();
            let id = *counter;
            *counter += 1;
            id
        };

        let subscription = Subscription::new(id, filter);
        self.subscriptions.lock().unwrap().push(subscription);

        let wrapped = WrappedSubscriber::new(Arc::new(subscriber));
        self.subscribers.lock().unwrap().push(Box::new(wrapped));

        SubscriptionHandle {
            subscription_id: id,
            event_bus: Arc::new(self.clone()),
        }
    }

    /// 移除订阅
    pub fn remove_subscription(
        &self,
        subscription_id: u64,
    ) {
        let mut subscriptions = self.subscriptions.lock().unwrap();
        subscriptions.retain(|s| s.id != subscription_id);

        // 注意：这里不移除 subscriber，因为 subscriber 可能在其他地方被引用
        // 在实际应用中可能需要引用计数或更复杂的清理逻辑
    }

    /// 清除所有订阅
    pub fn clear(&self) {
        self.subscribers.lock().unwrap().clear();
        self.subscriptions.lock().unwrap().clear();
    }

    /// 获取订阅数量
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.lock().unwrap().len()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            subscribers: Arc::clone(&self.subscribers),
            subscriptions: Arc::clone(&self.subscriptions),
            next_subscription_id: Arc::clone(&self.next_subscription_id),
            sequence_counter: Arc::clone(&self.sequence_counter),
        }
    }
}

impl EventEmitter for EventBus {
    fn emit<E: Event>(
        &mut self,
        event: E,
    ) {
        let metadata = EventMetadata {
            sequence: self.next_sequence(),
            ..Default::default()
        };
        self.emit_with(event, metadata);
    }

    fn emit_with<E: Event>(
        &mut self,
        event: E,
        metadata: EventMetadata,
    ) {
        // 将事件装箱为 dyn Any
        let event_any = Box::new(event) as Box<dyn Any>;

        // 获取事件类型信息（用于过滤）
        let event_type = if let Some(e) = event_any.as_ref().downcast_ref::<E>() {
            e.event_type()
        } else {
            EventType::All
        };

        let event_name = if let Some(e) = event_any.as_ref().downcast_ref::<E>() {
            e.name()
        } else {
            "Unknown"
        };

        let subscribers = self.subscribers.lock().unwrap();
        let subscriptions = self.subscriptions.lock().unwrap();

        for subscription in subscriptions.iter() {
            if !subscription.filter().matches(event_type, event_name) {
                continue;
            }

            for subscriber in subscribers.iter() {
                let sub_types = subscriber.event_types();
                if sub_types.contains(&EventType::All) || sub_types.contains(&event_type) {
                    subscriber.on_event(event_any.as_ref(), &metadata);
                }
            }
        }
    }

    fn next_sequence(&mut self) -> u64 {
        let mut counter = self.sequence_counter.lock().unwrap();
        let seq = *counter;
        *counter += 1;
        seq
    }
}

/// 简单的控制台日志订阅者（用于调试）
#[derive(Debug, Default)]
pub struct ConsoleLogger;

impl EventSubscriber for ConsoleLogger {
    fn event_types(&self) -> Vec<EventType> {
        vec![EventType::All]
    }

    fn on_event(
        &self,
        event: &dyn Any,
        metadata: &EventMetadata,
    ) {
        let type_name = std::any::type_name_of_val(event);
        let seq = metadata.sequence;
        eprintln!("[Event {}] {}", seq, type_name);
    }
}

/// LSP 通知订阅者（用于集成）
#[derive(Debug)]
pub struct LspNotifier {}

impl LspNotifier {
    pub fn new(_sender: std::sync::mpsc::Sender<(String, serde_json::Value)>) -> Self {
        Self {}
    }
}

impl EventSubscriber for LspNotifier {
    fn event_types(&self) -> Vec<EventType> {
        vec![EventType::Progress, EventType::Diagnostic]
    }

    fn on_event(
        &self,
        _event: &dyn Any,
        _metadata: &EventMetadata,
    ) {
        // 这里可以添加 LSP 通知逻辑
        // 例如将进度事件转换为 $/progress 通知
    }
}

/// 收集器订阅者（用于测试）
#[derive(Debug, Default)]
pub struct EventCollector {
    events: Arc<Mutex<Vec<(String, EventMetadata)>>>,
}

impl EventCollector {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn events(&self) -> Vec<(String, EventMetadata)> {
        self.events.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

impl EventSubscriber for EventCollector {
    fn event_types(&self) -> Vec<EventType> {
        vec![EventType::All]
    }

    fn on_event(
        &self,
        event: &dyn Any,
        metadata: &EventMetadata,
    ) {
        let type_name = std::any::type_name_of_val(event);
        let mut events = self.events.lock().unwrap();
        events.push((type_name.to_string(), metadata.clone()));
    }
}
