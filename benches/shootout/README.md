# shootout/ — YaoXiang 多语言基准对比

## 目录结构

```
benches/shootout/
├── bench.toml          # 配置清单（定义所有 benchmark 和语言）
├── src/                # 各 benchmark 源码
│   ├── fibonacci/      # 每问题一个目录
│   ├── matrix/
│   ├── list_ops/
│   └── string_concat/
├── runner/             # Rust 运行器（workspace member）
│   ├── Cargo.toml
│   └── src/main.rs
├── ../history/         # 性能历史档案（nightly 写入；见 ../history/README.md）
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
- **`--out-json`**：结构化结果 `{ meta, rows, failures }`，
  字段名是**跨语言契约**（`scripts/ci/bench-report.sh` 按名取数），
  改动会被 `cargo test -p shootout` 的门禁拦住

每条 `rows` 记录 bench / lang / input / `median_ms` / `mean_ms` /
`stddev_ms` / `relative_pct` / `compile_ms` / `output`。头条数字取
**中位数**：墙钟样本右偏，一次调度抖动就能把均值拉高。

`meta` 记录**环境归因**：各语言工具链版本、CPU、核数、OS。读数变了却不知道
编译器和机器也变了，就无从判断是语言退步还是环境换版。

## 正确性判据

`output` 是被测程序的实际输出。同输入下它必须**跨语言一致**——这条不再靠人肉看
表格，runner 判两级：

1. **金标**：`[benchmarks.X.expected]` 里声明的输入 → 期望输出，逐语言比对；
2. **跨语言一致**：无金标时，同一 (bench, input) 下各语言输出必须相同。

任一条不符 → 产出 `failures` 条目并以**非零码退出**：算错的程序再快也不是读数，
不该混进报告与历史。既无金标又只有一门语言时无法判定，runner 会显式打印
「⚠ 未校验 N 组」，而不是默认通过——「没检查」与「检查通过」在报告里必须分开。
（`string_concat` 目前就是这一类：输出是 12×输入长度的字符串，写金标等于往配置里
塞 19000 字符，且它只有 yaoxiang 一门实现。）

## 读数口径

- 每门语言在配置里写 `version` 命令，runner 每语言只取一次版本号，随读数入库；
- 报告里的「相对最快」是该输入下最快语言的倍数；「geomean 比值」是各语言跨全部
  测点的几何平均——加/减一个 benchmark 不会让它跳动，适合跨夜追踪；
- 单语言测点会在报告里显式列出（没有横比对象，只有自身读数）；
- 跨夜趋势见 `benches/history/`（nightly 自动入库）与文档站 `/perf/` 图表。

`failures` 记录未能产出结果的语言。必须显式保留而不是静默省略：
`matrix` 的 yaoxiang 实现曾因 `List`→`Vec` 统一后类型报错而编译失败，
报告里只是「少了一行」，读起来像「这个语言没配这项基准」。此时 runner
以非零码退出，CI 该步会报红。

## 添加新 benchmark

1. 在 `benches/shootout/src/` 下创建 `<problem>/` 目录
2. 添加各语言实现文件（`<problem>.yx`, `<problem>.rs`, ...）
3. 在 `benches/shootout/bench.toml` 中添加配置条目（字段名拼错会被
   `deny_unknown_fields` 直接报错，`src` 路径写错会被 `cargo test -p shootout`
   拦住——配置只在 nightly CI 里读取，而那个 job 不阻断发布）

## 设计

- **配置驱动**：bench.toml 控制一切，不改 runner 代码
- **配置格式是 TOML**（2026-10-15 由 YAML 迁移）：`serde_yaml` / `serde_yaml_ng`
  都依赖 `unsafe-libyaml`（RUSTSEC-2024-0449，unmaintained，`cargo audit` 的一条
  allowed warning）；`toml` 本来就在本仓库依赖树内（根 `Cargo.toml`、
  `tools/yx/Cargo.toml`），故迁移不引入新包，只净减那条链、audit 归零。
  不用 JSON：JSON 没有注释，而本文件的注释记着「读数可比性」的判定依据
- **编译/运行两阶段测量**：编译时间一次性记录，运行时间多轮取 mean±stddev
- **多语言对比**：同问题不同语言实现直接对比
- **独立于测试**：基准不挂在根 `cargo test` 上，`cargo run -p shootout` 独立运行；
  `cargo test -p shootout` 只跑清单校验（配置可解析、`src` 存在、JSON 字段名未改）
- **子进程输出被捕获**：不继承 stdout（否则每次测量都会把被测程序的输出
  混进结果流），且各轮输出必须一致——不一致说明结果不可复现，那样的读数
  没有比较意义
