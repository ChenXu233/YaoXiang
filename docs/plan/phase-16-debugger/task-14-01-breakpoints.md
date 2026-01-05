# Task 14.1: 断点管理

> **优先级": P1
> **状态**: ⏳ 待实现

## 功能描述

支持软件断点和硬件断点的设置、触发和管理。

## 断点结构

```rust
/// 断点
struct Breakpoint {
    /// 断点 ID
    id: BreakpointId,
    /// 断点位置
    location: SourceLocation,
    /// 断点状态
    state: BreakpointState,
    /// 命中计数
    hit_count: usize,
    /// 条件表达式
    condition: Option<Expr>,
    /// 命中时执行的命令
    commands: Vec<DebugCommand>,
}

enum BreakpointState {
    Enabled,
    Disabled,
    Pending,  // 尚未加载
}
```

## 相关文件

- **breakpoint.rs**: BreakpointManager
