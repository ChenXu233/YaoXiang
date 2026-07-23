# bench/ — YaoXiang 多语言基准测试

## 目录结构

```
bench/
├── bench.yaml          # 配置清单（定义所有 benchmark 和语言）
├── src/                # 各 benchmark 源码
│   ├── fibonacci/      # 每问题一个目录
│   ├── matrix/
│   ├── list_ops/
│   └── string_concat/
├── runner/             # 独立 Rust 运行器
│   ├── Cargo.toml
│   └── src/main.rs
├── results/            # 运行结果存档
└── README.md           # 本文件
```

## 用法

```bash
# 跑所有 benchmark
cargo run --package bench-runner

# 只跑斐波那契
cargo run --package bench-runner -- --bench fibonacci

# 只跑 YaoXiang
cargo run --package bench-runner -- --lang yaoxiang
```

## 添加新 benchmark

1. 在 `bench/src/` 下创建 `<problem>/` 目录
2. 添加各语言实现文件（`<problem>.yx`, `<problem>.rs`, ...）
3. 在 `bench/bench.yaml` 中添加配置条目

## 设计

- **配置驱动**：bench.yaml 控制一切，不改 runner 代码
- **编译/运行两阶段测量**：编译时间一次性记录，运行时间多轮取 mean±stddev
- **多语言对比**：同问题不同语言实现直接对比
- **独立于测试**：不侵入 `cargo test`，`cargo run --package bench-runner` 独立运行