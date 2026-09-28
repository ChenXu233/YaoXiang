# shootout/ — YaoXiang 多语言基准对比

## 目录结构

```
benches/shootout/
├── bench.yaml          # 配置清单（定义所有 benchmark 和语言）
├── src/                # 各 benchmark 源码
│   ├── fibonacci/      # 每问题一个目录
│   ├── matrix/
│   ├── list_ops/
│   └── string_concat/
├── runner/             # Rust 运行器（workspace member）
│   ├── Cargo.toml
│   └── src/main.rs
├── results/            # 运行结果存档
└── README.md           # 本文件
```

## 用法

```bash
# 跑所有 benchmark
cargo run -p shootout

# 只跑斐波那契
cargo run -p shootout -- --bench fibonacci

# 只跑 YaoXiang
cargo run -p shootout -- --lang yaoxiang

# 另存机器可读结果（CI 走这个，供 bench-report.sh 渲染表格）
cargo run -p shootout -- --out-json target/shootout-results.json
```

### 输出

- **stdout**：人读的框线报告（本地看用）
- **`--out-json`**：结构化结果 `{ rows: [...], failures: [...] }`，
  字段名是**跨语言契约**（`scripts/ci/bench-report.sh` 按名取数），
  改动会被 `cargo test -p shootout` 的门禁拦住

每条 `rows` 记录 bench / lang / input / `mean_ms` / `stddev_ms` /
`relative_pct` / `compile_ms` / `output`。其中 `output` 是被测程序的
实际输出——同输入下应**跨语言一致**，报告里可据此发现算错。

`failures` 记录未能产出结果的语言。必须显式保留而不是静默省略：
`matrix` 的 yaoxiang 实现曾因 `List`→`Vec` 统一后类型报错而编译失败，
报告里只是「少了一行」，读起来像「这个语言没配这项基准」。此时 runner
以非零码退出，CI 该步会报红。

## 添加新 benchmark

1. 在 `benches/shootout/src/` 下创建 `<problem>/` 目录
2. 添加各语言实现文件（`<problem>.yx`, `<problem>.rs`, ...）
3. 在 `benches/shootout/bench.yaml` 中添加配置条目

## 设计

- **配置驱动**：bench.yaml 控制一切，不改 runner 代码
- **编译/运行两阶段测量**：编译时间一次性记录，运行时间多轮取 mean±stddev
- **多语言对比**：同问题不同语言实现直接对比
- **独立于测试**：不侵入 `cargo test`，`cargo run -p shootout` 独立运行
- **子进程输出被捕获**：不继承 stdout（否则每次测量都会把被测程序的输出
  混进结果流），且各轮输出必须一致——不一致说明结果不可复现，那样的读数
  没有比较意义
