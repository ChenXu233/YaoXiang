# Task 13.1: 热点分析

> **优先级**: P1
> **状态**: ⏳ 待实现

## 功能描述

分析执行日志，识别热点函数和循环。

## 热点检测

```rust
/// JIT 分析器
struct JITProfiler {
    /// 函数调用计数
    fn_counts: HashMap<FunctionId, usize>,
    /// 循环执行计数
    loop_counts: HashMap<LoopId, usize>,
    /// 热点阈值
    hot_threshold: usize,
    /// 采样间隔
    sample_interval: usize,
}

impl JITProfiler {
    /// 记录函数调用
    pub fn record_fn_call(&mut self, fn_id: FunctionId) {
        *self.fn_counts.entry(fn_id).or_insert(0) += 1;
    }

    /// 获取热点函数
    pub fn get_hot_functions(&self) -> Vec<(FunctionId, usize)> {
        let mut hot: Vec<_> = self.fn_counts.iter()
            .filter(|(_, count)| **count >= self.hot_threshold)
            .map(|(id, count)| (*id, *count))
            .collect();
        hot.sort_by(|a, b| b.1.cmp(&a.1));
        hot
    }
}
```

## 相关文件

- **profiler.rs**: JITProfiler
