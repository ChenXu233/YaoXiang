//! Trait 对象支持
//!
//! 提供特质对象的创建和使用

/// 特质对象
pub type TraitObject = Box<dyn std::any::Any>;

/// 创建特质对象
pub fn make_trait_object<T: 'static>(value: T) -> TraitObject {
    Box::new(value)
}

/// 从特质对象中获取值
pub fn downcast_trait_object<T: 'static>(obj: &TraitObject) -> Option<&T> {
    obj.downcast_ref::<T>()
}
