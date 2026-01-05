# Phase 16: 调试器

> **模块路径**: `src/vm/debugger/`
> **状态**: ⏳ 待实现
> **依赖**: P11（VM）

## 概述

调试器支持断点、单步执行、变量查看等调试功能，提供完整的调试体验。

## 文件结构

```
phase-16-debugger/
├── README.md                    # 本文档
├── task-16-01-breakpoints.md    # 断点管理
├── task-16-02-stepping.md       # 单步执行
├── task-16-03-inspector.md      # 变量检查
└── task-16-04-breakpoints.md    # 调试协议
```

## 完成状态

| Task | 名称 | 状态 | 依赖 |
|------|------|------|------|
| task-16-01 | 断点管理 | ⏳ 待实现 | task-11-01 |
| task-16-02 | 单步执行 | ⏳ 待实现 | task-16-01 |
| task-16-03 | 变量检查 | ⏳ 待实现 | task-16-02 |
| task-16-04 | 调试协议 | ⏳ 待实现 | task-16-03 |

## 架构

```
┌─────────────────────────────────────────────────────────┐
│                     Debugger                             │
├─────────────────────────────────────────────────────────┤
│  Breakpoint Manager                                      │
│  ├── 行断点                                              │
│  ├── 条件断点                                            │
│  └── 断点命中计数                                        │
├─────────────────────────────────────────────────────────┤
│  Stepping Engine                                        │
│  ├── Step Over                                          │
│  ├── Step Into                                          │
│  └── Step Out                                           │
├─────────────────────────────────────────────────────────┤
│  Variable Inspector                                     │
│  ├── 局部变量                                            │
│  ├── 闭包变量                                            │
│  └── 表达式求值                                          │
├─────────────────────────────────────────────────────────┤
│  Debug Protocol                                         │
│  ├── DAP 适配器                                          │
│  └── CLI 接口                                            │
└─────────────────────────────────────────────────────────┘
```

## 断点管理

```rust
/// 断点类型
enum Breakpoint {
    Line {
        file: PathBuf,
        line: usize,
        condition: Option<Expr>,
        hit_count: usize,
    },
    Address {
        addr: usize,
    },
    Function {
        name: String,
    },
}

/// 断点管理器
struct BreakpointManager {
    breakpoints: HashMap<BreakpointId, Breakpoint>,
    active_breakpoints: BTreeSet<BreakpointId>,
}
```

## 单步执行

```rust
/// 单步模式
enum StepMode {
    Over,           // 不进入函数
    Into,           // 进入函数
    Out,            // 跳出函数
}

/// 单步执行器
struct SteppingEngine {
    step_mode: StepMode,
    target_frame: FrameId,    // 目标帧
    target_line: Option<usize>,
}
```

## 变量检查

```rust
/// 变量检查器
struct VariableInspector {
    current_frame: Frame,
    locals: HashMap<String, Value>,
    watch_expressions: Vec<Expr>,
}

impl VariableInspector {
    /// 获取所有局部变量
    fn get_locals(&self) -> Vec<Variable> {
        self.current_frame.locals()
            .iter()
            .map(|(name, value)| Variable { name, value })
            .collect()
    }

    /// 求值监视表达式
    fn eval_watch(&self, expr: &Expr) -> Result<Value, Error> {
        self.interpreter.eval(expr, &self.locals)
    }
}
```

## 调试协议

支持 DAP（Debug Adapter Protocol）：

```json
// 断点设置请求
{
    "command": "setBreakpoints",
    "arguments": {
        "source": {
            "path": "example.yx"
        },
        "breakpoints": [
            { "line": 10 },
            { "line": 20, "condition": "x > 0" }
        ]
    }
}
```

## 相关文件

- **mod.rs**: 调试器主模块
- **breakpoint.rs**: 断点管理
- **inspector.rs**: 变量检查
- **protocol.rs**: 调试协议

## 相关文档

- [Phase 11: VM](../phase-11-vm/README.md)
- [Phase 15: JIT](../phase-15-jit/README.md)
