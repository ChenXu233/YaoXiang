//! Memory 单元测试
//!
//! 测试内存管理堆的实现和行为

use crate::runtime::memory::Heap;

#[cfg(test)]
mod heap_tests {
    use super::*;

    #[test]
    fn test_heap_new() {
        let heap = Heap::new();
        assert_eq!(heap.capacity(), 64 * 1024);
        assert_eq!(heap.used(), 0);
    }

    #[test]
    fn test_heap_default() {
        let heap = Heap::default();
        assert_eq!(heap.used(), 0);
    }

    #[test]
    fn test_heap_alloc() {
        let mut heap = Heap::new();
        let offset = heap.alloc(100);
        assert!(offset.is_some());
        assert_eq!(heap.used(), 100);
    }

    #[test]
    fn test_heap_alloc_multiple() {
        let mut heap = Heap::new();
        let offset1 = heap.alloc(100);
        let offset2 = heap.alloc(200);
        let offset3 = heap.alloc(50);

        assert!(offset1.is_some());
        assert!(offset2.is_some());
        assert!(offset3.is_some());
        assert_eq!(heap.used(), 350);

        // Offsets should be sequential
        assert_eq!(offset1.unwrap(), 0);
        assert_eq!(offset2.unwrap(), 100);
        assert_eq!(offset3.unwrap(), 300);
    }

    #[test]
    fn test_heap_capacity() {
        let heap = Heap::new();
        // Default capacity is 64KB
        assert_eq!(heap.capacity(), 64 * 1024);
    }

    #[test]
    fn test_heap_used() {
        let mut heap = Heap::new();
        assert_eq!(heap.used(), 0);

        heap.alloc(100);
        assert_eq!(heap.used(), 100);

        heap.alloc(200);
        assert_eq!(heap.used(), 300);
    }

    #[test]
    fn test_heap_debug() {
        let heap = Heap::new();
        let debug = format!("{:?}", heap);
        assert!(debug.contains("Heap"));
    }
}

#[cfg(test)]
mod heap_write_read_tests {
    use super::*;

    #[test]
    fn test_heap_write_read_i32() {
        let mut heap = Heap::new();
        let value: i32 = 42;
        let offset = heap.alloc(std::mem::size_of::<i32>());
        assert!(offset.is_some());

        heap.write::<i32>(offset.unwrap(), &value);
        let read_value: &i32 = heap.read(offset.unwrap());
        assert_eq!(*read_value, 42);
    }

    #[test]
    fn test_heap_write_read_f64() {
        let mut heap = Heap::new();
        let value: f64 = 3.14159;
        let offset = heap.alloc(std::mem::size_of::<f64>());
        assert!(offset.is_some());

        heap.write::<f64>(offset.unwrap(), &value);
        let read_value: &f64 = heap.read(offset.unwrap());
        assert!((*read_value - 3.14159).abs() < 0.0001);
    }

    #[test]
    fn test_heap_write_read_bool() {
        let mut heap = Heap::new();
        let value: bool = true;
        let offset = heap.alloc(std::mem::size_of::<bool>());
        assert!(offset.is_some());

        heap.write::<bool>(offset.unwrap(), &value);
        let read_value: &bool = heap.read(offset.unwrap());
        assert_eq!(*read_value, true);
    }
}
