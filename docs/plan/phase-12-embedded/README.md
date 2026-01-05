# Phase 12: Embedded Runtime（嵌入式运行时）

> **模块路径**: `src/runtime/core/embedded/`
> **状态**: ⏳ 待实现
> **依赖**: P8（Core Runtime）

## 概述

Embedded Runtime 提供轻量级运行时环境，适用于资源受限环境或脚本嵌入场景。

## 与 Core Runtime 的关系

```
runtime/core/
├── embedded/              # P8 + P12（立即执行器）
│   ├── executor.rs        # 立即执行器核心
│   ├── mod.rs
│   └── api.rs             # 嵌入式 API（P12）
└── ...
```

## 文件结构

```
phase-12-embedded/
├── README.md              # 本文档
├── task-12-01-api.md      # 嵌入式 API
├── task-12-02-script.md   # 脚本引擎集成
└── task-12-03-embed.md    # 嵌入示例
```

## 完成状态

| Task | 名称 | 状态 | 依赖 |
|------|------|------|------|
| task-12-01 | 嵌入式 API | ⏳ 待实现 | task-08-05 |
| task-12-02 | 脚本引擎集成 | ⏳ 待实现 | task-12-01 |
| task-12-03 | 嵌入示例 | ⏳ 待实现 | task-12-02 |

## 嵌入式 API 设计

```rust
/// YaoXiang 嵌入式引擎
struct Engine {
    runtime: Runtime,
    gc: Gc,
    modules: ModuleCache,
}

impl Engine {
    /// 创建新引擎
    fn new() -> Self;

    /// 加载并执行脚本
    fn eval(&mut self, code: &str) -> Result<Value, Error>;

    /// 注册 native 函数
    fn register_fn<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&[Value]) -> Value + 'static;

    /// 设置内存限制
    fn set_memory_limit(&mut self, limit: usize);
}
```

## 脚本引擎集成

```rust
/// 作为游戏脚本引擎
fn game_script_example() {
    let mut engine = Engine::new();

    // 注册游戏 API
    engine.register_fn("log", |args| {
        println!("[Game] {}", args[0]);
        Value::Void
    });

    // 执行游戏脚本
    engine.eval(r#"
        log("Player joined: " + player_name)
        health = 100
        while health > 0 {
            update_player()
        }
    "#).unwrap();
}
```

## 使用场景

1. **嵌入式系统**：资源受限的运行环境
2. **游戏脚本**：作为游戏的脚本引擎
3. **应用扩展**：为应用提供脚本能力
4. **IoT 设备**：轻量级脚本执行

## 相关文档

- [Core Runtime - Embedded](../phase-08-core-runtime/embedded/README.md)
