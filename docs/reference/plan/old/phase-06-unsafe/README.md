# Phase 6: unsafe 代码块与裸指针

> **模块路径**: `src/core/unsafe/`
> **状态**: ⏳ 后期（v0.5+）
> **设计依据**: [RFC-009 所有权模型 v7](../../design/rfc/009-ownership-model.md)

> **注意**：此 Phase 属于后期扩展，MVP 不需要。FFI/裸指针用于系统编程，初期场景不需要。

## 概述

`unsafe` 代码块允许在受控环境中使用裸指针，用于系统级编程（FFI、内存操作等）。

> **核心设计决策**：`unsafe` 是用户保证安全的"逃生舱"
> - 裸指针 `*T` 只能在 `unsafe` 块中使用
> - 裸指针不满足 Send/Sync（单线程使用）
> - 可绕过循环引用检测（用户负责）

## unsafe 规则

### 裸指针语法

```yaoxiang
# 裸指针类型
PtrType ::= '*' TypeExpr

# 获取裸指针
p: Point = Point(1.0, 2.0)
ptr: *Point = &p     # 获取地址

# 解引用（用户保证有效）
(*ptr).x = 0.0

# 指针运算
ptr2 = ptr + 1
```

### unsafe 代码块

```yaoxiang
# unsafe 代码块语法
UnsafeBlock ::= 'unsafe' '{' Stmt* '}'

# 示例
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p     # 裸指针
    (*ptr).x = 0.0       # 解引用
    # 指针运算等...
}

# outside unsafe, raw pointers cannot exist
```

### 绕过循环检测

```yaoxiang
# unsafe 逃生舱：允许跨任务循环引用（用户负责）
unsafe {
    a.child = ref b
    b.child = ref a    # 允许，但用户保证不会导致问题
}
```

## 任务列表

| 顺序 | 任务 | 名称 | 依赖 | 优先级 |
|------|------|------|------|--------|
| 1 | task-06-01 | unsafe 关键字解析 | 无 | P0 |
| 2 | task-06-02 | 裸指针 `*T` 类型 | task-06-01 | P0 |
| 3 | task-06-03 | unsafe 块作用域规则 | task-06-01 | P0 |
| 4 | task-06-04 | 裸指针 Send/Sync 约束 | task-06-02 | P1 |

### 任务说明

#### Task 6.1: unsafe 关键字解析

- `unsafe` 关键字识别
- `unsafe` 块语法解析
- 嵌套 unsafe 块处理
- 实现：`src/core/unsafe/parser.rs`

> **依赖**：无

#### Task 6.2: 裸指针 `*T` 类型

- 裸指针类型语法 `*T`
- 裸指针与其他类型的交互
- 裸指针不能离开 unsafe 块
- 实现：`src/core/unsafe/types.rs`

> **依赖**：task-06-01

#### Task 6.3: unsafe 块作用域规则

- 裸指针的作用域限制
- unsafe 块内语句类型检查
- 嵌套 unsafe 处理
- 实现：`src/core/unsafe/checker.rs`

> **依赖**：task-06-01

#### Task 6.4: 裸指针 Send/Sync 约束

- `*T` 不满足 Send
- `*T` 不满足 Sync
- 单线程使用限制
- 实现：`src/core/unsafe/send_sync.rs`

> **依赖**：task-06-02，task-05-05（Send/Sync 约束）

## 限制

| 限制 | 说明 |
|------|------|
| 作用域 | 裸指针只能在 unsafe 块内使用 |
| Send/Sync | 裸指针不满足 Send/Sync |
| 生命周期 | 用户保证不悬空、不释放后使用 |
| 循环检测 | unsafe 内可绕过循环引用检测 |

## 相关文件

- **src/core/unsafe/mod.rs**: unsafe 检查器主实现
- **src/core/unsafe/parser.rs**: unsafe 解析
- **src/core/unsafe/types.rs**: 裸指针类型
- **src/core/unsafe/checker.rs**: 作用域规则检查
- **src/core/unsafe/send_sync.rs**: Send/Sync 约束

## 参考文档

- [RFC-009 所有权模型 v7](../../design/accepted/009-ownership-model.md)
- [YaoXiang 语言规范](../../design/language-spec.md)
