# Task 5.3: 跨边界值传递检查

> **优先级**: P0
> **状态**: 🔄 待实现
> **模块**: `src/core/ownership/cross_boundary.rs`

## 功能描述

> **RFC-009 核心规则**：值跨边界传递时，需要检查 Send 约束。小对象自动复制，大对象移动。

检查值在以下场景中的传递行为：
- spawn 闭包捕获（值复制/移动）
- channel 消息传递（值传递）
- @block 注解边界

由于 YaoXiang 是函数式语言，类型透明，**不需要检查引用跨边界**（没有 ref T）。

## 跨边界规则

### 值传递规则

```yaoxiang
# ✅ 正确：值跨 spawn 边界（自动复制或移动）
spawn_value: () -> Void = () => {
    data = Data(42)
    spawn(() => {
        print(data.value)  # data 自动复制（<1KB）或移动（>1KB）
    })
}

# ✅ 正确：捕获变量（值传递）
capture_vars: () -> Void = () => {
    id = 42
    name = "test"
    spawn(() => {
        print(id)      # id 自动复制
        print(name)    # name 自动复制
    })
}

# ✅ 正确：跨 channel 传递值
channel_value: () -> Void = () => {
    data = Data(42)
    channel.send(data)  # data 自动复制或移动
}
```

### Send 检查

```yaoxiang
# ✅ Send 类型可以跨线程
type Point = Point(x: Int, y: Float)
spawn(() => {
    print("ok")
})

# ❌ 非 Send 类型不能跨线程
type NonSend = NonSend(rc: Rc[Int])
# spawn(() => { NonSend(Rc.new(42)) })  # ❌ Rc 不是 Send
```

### @block 注解（用于调试/阻塞IO）

```yaoxiang
# @block 注解用于标记阻塞操作
# 不改变所有权语义，只是阻塞当前线程

use std.io

# @block 用于阻塞 IO 操作
@block
read_file: (String) -> String = (path) => {
    io.read_file(path)  # 阻塞调用
}

# @block 内的值传递规则不变
# 小对象复制，大对象移动
```

## 检查算法

```rust
struct CrossBoundaryChecker {
    /// 代码块边界（spawn, channel, @block）
    boundaries: Vec<CodeBlock>,
    /// 捕获的变量
    captured_vars: HashMap<BlockId, Vec<ValueId>>,
    /// 跨边界错误
    errors: Vec<CrossBoundaryError>,
}

impl CrossBoundaryChecker {
    /// 检查 spawn 闭包捕获
    fn check_spawn_capture(&self, spawn: &SpawnExpr) -> Result<(), CrossBoundaryError> {
        for captured in &spawn.captured_vars {
            // 检查值是否 Send
            if !self.is_send(&self.get_type(captured)) {
                return Err(CrossBoundaryError::NonSendCaptured {
                    value: *captured,
                    span: captured.span,
                });
            }
        }

        Ok(())
    }

    /// 检查 channel 发送
    fn check_channel_send(&self, send: &ChannelSend) -> Result<(), CrossBoundaryError> {
        let value = &send.value;

        // 检查值是否 Send
        if !self.is_send(&self.get_type(value)) {
            return Err(CrossBoundaryError::NonSendValue {
                value: *self.get_value_id(value),
                ty: self.get_type(value),
                span: value.span,
            });
        }

        Ok(())
    }

    /// 分析闭包捕获的变量
    fn analyze_closure_capture(&mut self, closure: &ClosureExpr) {
        for param in &closure.params {
            self.captured_vars
                .entry(closure.id)
                .or_default()
                .push(param.value_id);
        }

        // 递归分析嵌套闭包
        for nested in &closure.nested_closures {
            self.analyze_closure_capture(nested);
        }
    }
}
```

## 错误类型

```rust
#[derive(Debug, Clone)]
pub enum CrossBoundaryError {
    NonSendCaptured {
        value: ValueId,
        span: Span,
    },
    NonSendValue {
        value: ValueId,
        ty: Type,
        span: Span,
    },
}
```

## 与 RFC-009 对照

| RFC-009 规则 | 实现状态 |
|-------------|---------|
| 值跨 spawn 边界（复制/移动） | ✅ 见 task-05-04 |
| 值跨 channel 边界（复制/移动） | ✅ 见 task-05-04 |
| Send 检查 | ✅ 已实现 |
| @block 仅用于调试/阻塞IO | ✅ 已实现 |

## 验收测试

```yaoxiang
# test_cross_boundary.yx

# === spawn 边界测试 ===
# 正确：值跨 spawn
good_value: () -> Void = () => {
    data = Data(42)
    spawn(() => {
        print(data.value)  # data 自动复制或移动
    })
}

# === Send 检查测试 ===
# 正确：Send 类型
type Point = Point(x: Int, y: Float)
spawn(() => {
    p = Point(1, 2.0)
    print(p.x)
})

# === channel 边界测试 ===
# 正确：值跨 channel
good_channel: () -> Void = () => {
    data = Data(42)
    channel.send(data)  # data 自动复制或移动
}

print("Cross boundary tests passed!")
```

## 相关文件

- **src/core/ownership/cross_boundary.rs**: 跨边界检查器
- **src/core/ownership/value_pass.rs**: 值传递机制
- **src/middle/escape_analysis/mod.rs**: 逃逸分析
