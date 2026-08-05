# Ponytail 全仓库审计报告（YaoXiang）

> 审计标准：ponytail（删过度工程，标准库/原生优先，最短实现）。
> 流程：按子分层逐个审计，每层完成后更新本文件对应章节；跨分层问题随手记入 §13 待终审。
> 只报告，不动代码。标签：`delete:` 删 / `stdlib:` 换标准库 / `native:` 换平台原生 / `yagni:` 单实现抽象或无人配置 / `shrink:` 同逻辑更短。

## 分层总览

| # | 分层 | 状态 |
|---|------|------|
| 1 | 全局依赖与 feature | ✅ 完成 |
| 2 | src/util | ✅ 完成 |
| 3 | src/frontend | ✅ 完成 |
| 4 | src/middle | ✅ 完成 |
| 5 | src/backends | ✅ 完成 |
| 6 | src/formatter | ✅ 完成 |
| 7 | src/lsp | ✅ 完成 |
| 8 | src/repl | ✅ 完成 |
| 9 | src/package | ✅ 完成 |
| 10 | src/std | ✅ 完成 |
| 11 | src/main.rs（CLI） | ✅ 完成 |
| 12 | 周边（scripts/wasm/vscode/benches/tools） | ✅ 完成 |
| 13 | 跨分层汇总与终审 | ✅ 完成 |
| 14 | 重构型机会（非删除） | ✅ 完成 |

## §1 全局依赖与 feature（Cargo.toml）

1. **delete:** tokio 在 src/ 全仓库零引用，但 `cli` feature 以 `features=["full"]` 拖着整个异步运行时。删依赖 + 删 feature 引用。 `[Cargo.toml:22,55]`
2. **delete:** `wasm = []` 空 feature 无人引用（wasm crate 用 `default-features = false`，不靠它）。 `[Cargo.toml:19]`
3. **shrink:** `tempfile` 只在 tests 里出现，却挂在 `[dependencies]` 的 cli feature 里。移到 `[dev-dependencies]`。 `[Cargo.toml]`
4. **shrink:** crossbeam 只用了 `crossbeam::channel`，换 `crossbeam-channel`，少拉 epoch/deque/queue。 `[src/backends/runtime/facade.rs]`
5. **yagni:** `cli` feature 实际没门住任何 wasm32 cfg 没门住的东西（lsp/repl 按 target 而非 feature 门控），feature 边界名存实亡。要么真门控，要么删掉合并进 wasm32 cfg。 `[src/lib.rs:23-30]`

不动：anyhow+thiserror 混用（惯例）、urlencoding/serde/toml/clap/rustyline/lsp-*/libloading/web-time 均有真实用户。

**net: -2 deps（tokio、tempfile→dev），1 dep 瘦身（crossbeam→crossbeam-channel），编译时间大幅改善。**

## §2 src/util

规模：6255 行 Rust + codes/i18n 5464 行 JSON + locales。本层是全仓库最大的砍点。

1. **delete:** `MSG` 枚举 213 个变体中 **134 个零引用**——Parser*/Vm*/Shell*/Debug*/TypeCheck*/Repl*/Package*/Bytecode*/Codegen* 等旧调试日志残留（ReplWelcome、ShellWelcome、VmBinaryOp…）。删枚举变体 + 6 个 locale 里对应翻译条目。方法：全仓库 `MSG::X` 字面扫描（tlog!/t_cur 均走此路径，已抽查验证）。 `[src/util/i18n/mod.rs:337-606, locales/*.json]` ≈ -1200 行
2. **delete?:** **31 个预留但未实现的错误码**：E0015/E0017/E0019-E0022、E2015/E2017/E2021-E2026/E2028、E3001-E3003/E3009/E3010-E3013/E3015/E3016/E3018/E3019、E4013/E4015-E4017。每码 = 定义 + builder helper + 6 语翻译。方法：字面量扫描初筛 44 个，再按 helper 函数名复核排除 13 个误报（如 E2019 double_drop 实际在用）。这些是所有权/IR/codegen/终止性检查的占位码——实现对应检查时再加回来。 `[src/util/diagnostic/codes/]` ≈ -1600 行
3. **delete:** `collect.rs` 的 `Warning` 枚举、`ErrorFormatter`、`add_warning/has_warnings/warning_count/format_warnings`——只被 re-export，全仓库零调用（pipeline.rs 自己统计警告）。 `[src/util/diagnostic/collect.rs]` ≈ -110 行
4. **delete:** `UserConfig` 的 `LintConfig`/`InstallConfig`/`ToolConfig` 三段定义 + Default，零读取方。 `[src/util/config/mod.rs:147-204]` ≈ -40 行
5. **yagni:** `ErrorCollector<E: SpannedError>` 唯一实例化是 `Diagnostic`；删泛型参数 + `SpannedError` trait（span.rs:273）。 `[src/util/diagnostic/collect.rs, src/util/span.rs]` ≈ -30 行

不动：cache.rs（LSP server/session 在用）、suggest.rs、emitter/json（CLI --json）、test_runner（main.rs test 命令）、span.rs 核心类型（Position/Span/DebugSpan/SourceFile/SourceMap 均有用户）、logger、time_compat（wasm 必需）、builder.rs（发射机制核心）、i18n 翻译机制本身（253 处调用，真实用户）。

**net: ≈ -2900 行（Rust ≈ -180 + JSON/locale ≈ -2700）。**

## §3 src/frontend

规模：38520 行，最大分层。编译器核心（lexer/parser/typecheck/types/eval/layers）逐模块查过调用方，全部存活：semantic_db→LSP、spawn/analysis→ir_gen、orchestrator→lib/pipeline、validate→repl/formatter、semantic_tokens→LSP+checker、layers→checker/inference。砍点集中在一处：

1. **delete:** `src/frontend/config.rs`（457 行）里零调用的部分：`FeatureFlags` 全部字段 + 六个 `has_rfc*/has_traits` 方法、`OptLevel`、`DiagLevel` 及 `should_show_*`、`ErrorRecoveryStrategy`、`IncrementalConfig`、全部 builder（`with_opt_level`…`all_features`）、`JsonConfig` + `ConfigAdapter` trait + `impl ConfigAdapter for ()`。全仓库唯一被读的只有 3 处：`dead_code.enabled`、`mono.enabled`、`mono.max_depth`（pipeline.rs）。CompileConfig 可缩成两个小配置。 `[src/frontend/config.rs]` ≈ -330 行
   ⚠️ 注意：crate 对外发布（有 docs.rs 元数据），这是公开 API——删前确认无外部嵌入方在用，或按 semver 走 major。
2. **shrink:** `TypeErrorCollector` 别名在 `typecheck/mod.rs` 与 `typecheck/environment.rs` 重复定义。 ≈ -3 行

**net: ≈ -335 行（全在 config.rs + 别名去重）。**

## §4 src/middle

规模：12593 行。ir_gen（4536 行，全仓最大文件）、bytecode IR、mono、codegen 逐模块查调用方，全部存活（pipeline/typecheck/lib 均在用）。本层干净，只有已知条目：

1. **yagni:** `FunctionMonomorphizer` trait（function.rs:15，约 8 个方法）唯一实现是 `Monomorphizer`，无 mock 无第二实现。删 trait 直接调具体类型（见 §13）。 `[src/middle/passes/mono/function.rs]`
2. 备忘（不算砍点）：`BytecodeFile`（codegen/bytecode.rs，.42 文件格式）与 `BytecodeModule`（core/bytecode.rs，运行时 IR）双表示 + From 转换——文件/运行时分层合理，不动；若日后发现转换层纯转发再合并。

**net: 0 行（trait 删除计入 §13 汇总）。**

## §5 src/backends

规模：6642 行。interpreter（executor/frames/registers/ffi）、common（heap/value）、runtime（engine+facade）逐模块查过，主体存活：ffi 有真实用户（libloading + C 集成测试），runtime 双模式（Embedded/Standard）都被 interpreter 和 CLI 接线，engine 被 facade 包装。

1. **delete:** `Opcode` 枚举 125 个变体中 **50 个零引用**（定义外全仓扫描）：整个 F32*/F64* 浮点算术族（28）、I32 算术族（13）、Custom0-3、Invalid、I64Const（被 LoadConst 取代）、Rethrow、LoopStart/LoopInc（循环被迭代器消除取代）。方法：解释器走 `BytecodeInstr` ADT，`Opcode::X` 引用只在 codegen/序列化映射，对比定义集与引用集。预留的窄数值 opcode，实现时再加。 `[src/backends/common/opcode.rs，515 行]` ≈ -250 行
2. **delete:** `ExecutorConfig` 六字段只有 `max_stack_depth` 被读；`initial_heap_size/max_heap_size/enable_checks/enable_debug/build_mode` 只写不读，连带 `BuildMode` 枚举整个删（见 §1）。 `[src/backends/mod.rs:329-364]` ≈ -35 行
3. **yagni:** `Executor`/`DebuggableExecutor` 双 trait 唯一实现都是 `Interpreter`，无第二 backend（见 §13）。 `[src/backends/mod.rs:264-289]`

**net: ≈ -285 行。**

## §6 src/formatter

规模：2553 行。Lean。`FormatOptions` 六字段（line_width/indent_width/use_tabs/single_quote/sort_imports/verify）全部被 handler/CLI 读取；source_map.rs 是格式化专用元信息（注释样式/空白行/token 位置），与 util/span 不重复；rules/sort_imports 有真实用户；options.rs 无多余公有函数。

**net: 0 行。**

## §7 src/lsp

规模：3014 行。Lean。13 个 handler（code_action/completion/definition/diagnostics/formatting/hover/initialize/inlay_hint/references/rename/semantic_tokens/text_document/workspace_symbol）逐个确认在 server.rs 分发处接线；session（95 行，生命周期）、world（189 行，语义状态）、protocol/locate/capabilities 都小而专。

**net: 0 行。**

## §8 src/repl

规模：1165 行。

1. **delete:** `--tui` 旗标：帮助文本宣传“TUI REPL（Experimental）”，实际分支只打印“TUI REPL mode is not available”然后 exit(1)。死旗标 + 误导性文档（还说自己是无命令时的默认，实际默认 tui: false）。 `[src/main.rs:226-228,522-528]` ≈ -8 行
2. **delete:** `ReplConfig.history_size` 字段零读取（ rustyline 用自己的配置）。 `[src/repl/mod.rs:49-50,66]` ≈ -3 行
3. **yagni:** `REPLBackend` trait（backend.rs，73 行）唯一实现 `Evaluator`，无 mock；删 trait 直接调具体类型（见 §13）。 `[src/repl/backend.rs]`
4. **yagni:** `Repl::with_config` 无外部调用方（仅内部 new() 用），作为发布 crate 的嵌入 API 保留与否自行定夺。

**net: ≈ -11 行 + trait 删除计入 §13。**

## §9 src/package

规模：3041 行。Lean。CLI 六个包命令（init/add/rm/update/install/list）全部接线；vendor（545 行）被 install/update/module_resolver/frontend 真实使用；template→init、conflict→install/source、lock→五个命令；`Source` trait 有两个实现（LocalSource/GitSource），非单实现。

**net: 0 行。**

## §10 src/std

规模：5345 行。Lean。语言标准库是产品面：15 个模块全部在 `register_all`/`all_module_infos` 登记（两表一致）；`StdModule` trait 多实现，正当；gen_interfaces（LSP 跳转/补全用）与 yx_sources（RFC-036 嵌入式 .yx std）都有真实用户；wasm32 用 cfg 排除 os/net/weak/concurrent，正确。yx_sources 里已有 ponytail 注释，作者自觉。

**net: 0 行。**

## §11 src/main.rs（CLI）

规模：576 行。十三个子命令全部接线，大多数旗标（check --exclude/--json/--color/--no-progress、format 六旗标、run --runtime/--workers、build --debug-info 等）确认有真实分支。

1. **delete:** `--tui` 死旗标（归属此处，详见 §8）：分支只报错退出。 `[src/main.rs:226-228,522-528]` ≈ -8 行
2. **shrink:** `Commands::Version` 子命令只用 info! 打印，与 clap 自带 `--version` 重复。可删。 `[src/main.rs:519-521]` ≈ -3 行
3. **shrink:** `"full"` runtime 别名解析 match 块在两处逐字重复（见 §13）：抽函数后 main.rs 侧也受益。

**net: ≈ -11 行。**

## §12 周边（scripts/wasm/vscode/benches/tools）

逐个查引用方（CI workflow/pre-commit/docs/package.json），绝大部分接线：

- i18n AI 翻译管线（14 个 mjs + 测试）→ auto-translate/auto-translate-i18n workflow ✓
- rfc/ai_agent.py、check_tracking.py（+ 686 行测试）→ rfc-ai-agent/rfc-check workflow + CONTRIBUTING ✓
- build-wasm.sh → nightly/release/docs-deploy ✓；setup.iss → _build-platforms ✓；check-docs-freshness.sh → docs-freshness ✓；generate-commit-list.mjs → release ✓；hooks/* → pre-commit ✓；sync-syntax.mjs → docs/package.json ✓
- tools/setup-z3 → README/文档（用户面工具）✓；wasm crate（playground）✓；vscode-extension（307 行 LSP client + language pack，非 stub）✓；benches/shootout/runner（真实基准对比 crate）✓

1. **delete?:** `scripts/rfc/rfc_sync.py`（226 行）+ `rfc_sync_agent.py`（239 行）零自动化引用（CI/pre-commit/docs/其他脚本都没有）。自述为“主/子 agent 调用”的 RFC↔GitHub issue 同步编排工具，但无任何调用痕迹。确认 RFC 同步流程还在用就留，不在用就连同相关说明一起删。低置信。 ≈ -465 行

**net: 待确认，最多 ≈ -465 行。**

## §13 跨分层问题汇总（终审）

1. **shrink:** `runtime_mode` 字符串→`RuntimeMode` 的 match 块（含 "full" 别名注释）在 `src/util/diagnostic/mod.rs` 两处逐字重复（约 L235、L322）。抽一个 `parse_runtime_mode()`。另：“full” 模式从未实现（work-stealing 欠奉，代码注释自认），CLI 仍宣传三档——删掉 full 选项或在帮助文本标明 alias。 ≈ -15 行
2. **yagni:** 单实现 trait 四连，按删除价值排序：
   - `REPLBackend`（src/repl/backend.rs:45，6 方法）唯一实现 Evaluator，无 mock → 删 trait 直接调 Evaluator
   - `FunctionMonomorphizer`（src/middle/passes/mono/function.rs:15）唯一实现 Monomorphizer → 同上
   - `Executor`/`DebuggableExecutor`（src/backends/mod.rs:264-289）唯一实现 Interpreter → 若近期无 JIT/第二 backend 排期则删；有路线图则留，这是四个里唯一有正当性的
   合计 ≈ -100 行（trait 定义 + 间接层）
3. **yagni:** `ConfigAdapter` trait + `impl for ()` 占位实现——已随 §3 config.rs 缩减一起删（JsonConfig 零调用）。
4. **shrink:** `TypeErrorCollector` 别名重复定义（typecheck/mod.rs 与 environment.rs）——已计入 §3。

## §14 重构型机会（非删除，结构性减码）

前 13 章偏“死代码删除”，本章是重构才能拿到的量。

1. **shrink:** 字节码解码巨型 impl：`impl From<BytecodeFile> for BytecodeModule`（core/bytecode.rs:813-1915，**1103 行**）= 48 个 opcode 臂 × 手工 `from_le_bytes` 拼操作数（48 处重复）。最小改：抽 `op_u32(&operands)`/`op_u64` 助手消重；彻底改：用单一指令 schema 宏同时生成编码（emitter 侧）与解码，两边永不漂移。 ≈ -600~800 行
2. **shrink:** 指令双词表：`Opcode` 枚举（515 行）与 `BytecodeInstr` ADT 同构并存，靠单向映射表（core/bytecode.rs:501+，约 75 臂）维持一致。Opcode 实际只是文件格式的 u8 编码标签，翻译器却先发射 Opcode 再转字节。让 BytecodeInstr 直接带 u8 编码、序列化直对 BytecodeInstr，可删整个 Opcode 枚举 + 映射表。死 opcode 问题（§5）正是双词表漂移的证据。工程量大，建议与 #1 一起做。 ≈ -500 行
3. **shrink:** std FFI 样板：137 个 `NativeExport::new` 五行块（≈ 685 行注册表）+ handler 里 95 处手工参数解构。小宏把每条目压到一行（registry ≈ -400 行），`expect_list/expect_int` 助手族消解构重复（≈ -300 行）。 ≈ -700 行
4. **shrink（设计级，后置）:** signature 字符串回环：137 个手写签名字符串被 `typecheck/signature.rs`（726 行解析器，issue #242 全集）再解析回类型。字符串是真接口（LSP/gen_interfaces 消费）不动，但可反向：NativeExport 用 Rust 类型 AST 定义，需要字符串时渲染——杀解析器 + 杀未校验字符串漂移类。净收益待评估，风险高。 潜在 -300~500 行
5. **shrink:** i18n 双翻译系统：locales/*.json（MSG）与 codes/i18n/*.json（错误码）两套平行 6 语树 + 两套加载/缓存逻辑（util/i18n + codes/builder）。合并为一个键命名空间 + 一个加载器。 ≈ -200~300 行
6. **shrink:** run 路径六连：lib.rs 的 run/run_file/run_project 各自 compile→generate→Interpreter::new，util/diagnostic/mod.rs 又两份（字节码分支 + 源码分支，rt_mode match 重复只是症状）。抽 `load_or_compile(path) -> BytecodeModule` + `exec(module, mode, workers)` 两函数。 ≈ -100~150 行
7. 备忘：`types/eval/evaluator.rs`（1114 行，RFC-027 统一求值）与 `const_eval.rs`（1002 行，const 泛型）互不引用、职责相邻，值得查一次算术求值重叠度，能并则并。

**重构小计: ≈ -2100~2550 行（不含 #4 后置项）。工程量和风险都显著高于删除类，建议单独排期；#1+#2 应合并为一个 bytecode 格式专项。**

## 终审总榜（按砍幅排序）

| # | 条目 | 分层 | 估计砍幅 | 置信 |
|---|------|------|---------|------|
| 1 | 134 个死 MSG 变体 + 6 locale 条目 | §2 | ≈ -1200 行 | 高 |
| 2 | 31 个预留未发错误码 + builder + 6 语翻译 | §2 | ≈ -1600 行 | 中高（实现对应检查时加回） |
| 3 | tokio 死依赖（features=["full"]） | §1 | 0 行，大编译时间收益 | 高 |
| 4 | frontend/config.rs 死配置族（FeatureFlags/OptLevel/…） | §3 | ≈ -330 行 | 中高（公开 API，确认无外部用户） |
| 5 | 50 个死 Opcode 变体（F32/F64/I32 族） | §5 | ≈ -250 行 | 高 |
| 6 | collect.rs Warning/ErrorFormatter 死机器 | §2 | ≈ -110 行 | 高 |
| 7 | ErrorCollector 去泛型 + SpannedError trait | §2 | ≈ -30 行 | 高 |
| 8 | 单实现 trait 四连 | §13 | ≈ -100 行 | 中（Executor 视 roadmap） |
| 9 | ExecutorConfig 死字段 + BuildMode | §1/§5 | ≈ -35 行 | 高 |
| 10 | LintConfig/InstallConfig/ToolConfig 死配置 | §2 | ≈ -40 行 | 高 |
| 11 | rfc_sync*.py 疑似死脚本 | §12 | ≈ -465 行 | 低（待确认） |
| 12 | runtime match 块重复 + full 别名 | §13 | ≈ -15 行 | 高 |
| 13 | --tui 死旗标 + Version 子命令 + history_size | §8/§11 | ≈ -14 行 | 高 |
| 14 | tempfile→dev-deps、wasm 空 feature、crossbeam→crossbeam-channel | §1 | 依赖清理 | 高 |

**net: 删除类 ≈ -4100 行（其中 JSON/locale ≈ -2700）+ 重构类潜在 ≈ -2100~2550 行；-2 deps（tokio、tempfile→dev），1 dep 瘦身（crossbeam→crossbeam-channel），1 死 feature。最大单项收益：删 tokio 的编译时间（删除类）与 bytecode 双表示合并（重构类）。**

不动清单：anyhow+thiserror 混用、lsp-types/serde/toml/clap/rustyline/libloading/urlencoding/web-time、i18n 翻译机制、语言 std 库、formatter/lsp/package/middle 主体、TimeCompat cfg 别名、BytecodeFile/BytecodeModule 双表示、crossbeam channel（Receiver 被 clone，std mpsc 顶不了）。

---

## §15 逐层重构审计（重复代码/样板）

方法：全仓规范化 6 行 shingle 哈希聚类（跨文件重复块）+ 逐层样板清点。一次性脚本，用完已删。结论先行：大文件内部多是独有逻辑，跨文件复制粘贴集中在 AST 解构、类型替换、样板注册表三类。

### §15.util

1. **shrink:** 95 个 DiagnosticBuilder helper 同一模板：`Self::find("EXXXX").unwrap()` + `.builder().param(...)`，每个 5-6 行。声明式宏一行一码（或从定义表生成）。 ≈ -280 行 `[src/util/diagnostic/codes/e*.rs]`
2. **shrink:** `tlog!` 宏 16 臂 = 4 级日志 × 4 元数，`$($arg:expr),*` 收编。 ≈ -40 行 `[src/util/i18n/mod.rs:188]`

### §15.frontend

shingle 扫描的主战场，但相对 38520 行的规模，重复占比不高——大文件（checker/ownership/inference）内部是独有逻辑。确认的跨文件复制簇：

1. **shrink:** “接收者解构 + lambda/block 身体解构”约 22 行组合，**5 处逐字重复**：checker.rs:1555、semantic_tokens.rs:444、dead_code.rs:110、completion.rs:152、ir_gen.rs:956/1886。给 `Expr` 加两个查询方法（`receiver_parts()`、`callable_parts()`）收编。 ≈ -80 行
2. **shrink:** `substitute_type_refs`（MonoType 递归替换）在 checker.rs:1659 与 inference/statements.rs:1113 **两份完全相同**，各约 35 行。并为 `MonoType::substitute(&HashMap)` 一个方法。 ≈ -35 行
3. **shrink:** use 语句 token 流扫描（KwUse 定位 + 段名收集）在 orchestrator.rs:384 与 pipeline.rs:585 两份，各约 25 行。抽 `scan_use_paths(tokens)`。 ≈ -25 行
4. **shrink:** `validate_positions`（负数/越界校验）在 lexer/symbols.rs:300 与 parser/statements/bindings.rs:197 重复。 ≈ -15 行
5. **shrink:** 手写“括号深度切分参数串”（逐字符 depth 扫描）在 const_eval.rs:667 与 normalizer.rs:297 重复——同时坐实 §14.7 的猜想：evaluator/normalizer/const_eval 求值家族有实质重叠，值得专项合并。 ≈ -15 行

小计 ≈ -170 行。

### §15.middle

1. **shrink:** 字节码层指令表示实为 **四种**：IR `Instruction`（ir.rs:47，IR 层——合理抽象，保留）→ `BytecodeInstruction`（codegen/bytecode.rs:206，原始 opcode+操作数字节）↔ `Opcode` 枚举（515 行，u8 编码标签）↔ `BytecodeInstr` ADT（core/bytecode.rs，解释器执行层），靠 75 臂映射表 + 1103 行解码 impl + translator 54 处编码点缝合。**字节码层 4 种 → 1 种**：BytecodeInstr 自带 u8 编码、编解码由同一 schema 宏生成。§5 的 50 个死 opcode 就是双词表漂移的实证。全仓最大单项重构，建议立专项（合并 §14.1/14.2 的估计）。 ≈ -1100~1600 行
2. **shrink:** FunctionIR 重建块（替换后 params/return_type 组装）在 mono/function.rs:458 与 mono/mod.rs:233 重复。 ≈ -8 行
3. ir_gen.rs（4536 行、59 fn）经 shingle 扫描确认几乎全是独有 lowering 逻辑（仅 2 处属 §15.frontend 第 1 条的接收者解构簇）。拆文件是可读性问题不是行数问题，不动。

### §15.backends

1. **shrink:** outcome 统计记账（outcome match → completed/failed/cancelled 计数 + avg 计算 + spawned）在 facade.rs EmbeddedRuntime（289-316）与 engine.rs LocalRuntime（348/1009）重复。抽 `RuntimeStats::record(&mut self, outcome, exec_time)`。 ≈ -30 行
2. executor/debug.rs 的 58 臂分派是解释器语义本体，每臂必须存在，不做表驱动化。Lean。

### §15.formatter

1. **shrink:** trailing-comment 处理块（截断到行尾 + 追加分支，约 7 行）在 handlers/expr.rs:694、module.rs:47、module.rs:86 三处。抽一个 context 方法。 ≈ -14 行
2. 其余（source_map/handlers/rules）无跨文件重复。Lean。

### §15.lsp

1. **shrink:** LSP 位置→内部坐标换算 `as usize + 1` 散落 5 个 handler（completion/rename/references/hover/definition），拢到 locate.rs 一个函数。总量小，低收益。 ≈ -6 行
2. 13 个 handler 语义各异，公共脚手架仅 3-4 行/个，不值得大抽象。Lean。

### §15.repl + §15.package

1. **shrink:** commands/add.rs 与 rm.rs 共享“manifest.save → LockFile::load → deps clone”序列（约 8 行 × 2），可抽 helper，低收益。 ≈ -8 行
2. repl 无跨文件重复。package 的 resolver/git/lock 各有真实职责。Lean。

### §15.std

1. **shrink:** 35 处 `match ctx.heap.get(h) { Some(HeapValue::List(..)) => .., _ => Err(type_only) }` 同构样板。给 NativeContext 加类型化访问器（`ctx.heap_list(h)?`、`ctx.heap_dict(h)?`）。 ≈ -100 行
2. **shrink:** 170 处 `ExecutorError::type_only/runtime_only` 构造，与 #1 的堆访问失败可共用统一错误，其余保守不动。
3. §14.3（137 exports 宏 + 参数解构助手）归属本层，不重复计。

### §15.main + 周边

1. main.rs 的 16 处 `.context()` 是 anyhow 惯用法，不动。
2. scripts/i18n（14 模块 + 350 行测试）与 scripts/rfc（16 函数）职责各异无大块重复。Lean。

## §16 重构总榜（合并 §14/§15，去重后）

| # | 条目 | 出处 | 估计砍幅 | 工程量/风险 |
|---|------|------|---------|------------|
| 1 | 字节码指令表示统一（4 种 → 2 种，编解码同 schema 生成） | §14.1+14.2、§15.middle | ≈ -1100~1600 行 | 大，专项 |
| 2 | std FFI 三板斧：exports 宏 + 参数解构助手 + 堆类型化访问器 | §14.3、§15.std | ≈ -800 行 | 中 |
| 3 | signature 字符串回环反转（Rust 类型 AST 定义，按需渲染） | §14.4 | ≈ -300~500 行 | 设计级，后置 |
| 4 | codes helper 宏（95 个 find+builder 样板） | §15.util | ≈ -280 行 | 低 |
| 5 | i18n 双翻译系统合一 | §14.5 | ≈ -200~300 行 | 中 |
| 6 | AST 解构 5 处收编 + substitute_type_refs 合并 + use 扫描合并 | §15.frontend | ≈ -170 行 | 低 |
| 7 | run 路径六连合一 | §14.6 | ≈ -100~150 行 | 低中 |
| 8 | 小件：tlog 宏 16 臂、统计记账、trailing-comment、位置换算、add/rm 序列等 | §15 各处 | ≈ -100 行 | 低 |

**重构小计 ≈ -3000~3900 行**（工程量集中在 #1/#2/#3，其余是低风险快赢）。

---

## §17 ponytail 深审：必要性 / 死旋钮 / 纯转发 / 测试样板

前两轮只覆盖"死代码 + 重复"，本轮补梯子前几级：**这东西需要存在吗？平台不是有吗？**

### 必要性裁定（先洗清嫌疑，不误伤）

- **并发运行时不是投机基建**：spawn 是真实语言特性——12 个 .yx 测试、ir_gen 真调 spawn/analysis（1006 行分析被真实消费）、translator 真编码 task_deps/resources、facade 有专门测试。保留。
- **dependent types / type families**：接线 checker/inference/normalizer/assert，是"一切皆类型"语言哲学本体。保留。
- **weak 模块**：仅 1 个 .yx 测试，但属产品面（覆盖率薄不是复杂度问题）。保留。
- **14 个错误枚举**（ConfigError/PackageError/PipelineError/…）：Rust 惯例，模块级错误 + 边界 anyhow。不动。

### 发现

1. **delete:** `locales/.i18n-cache.json`——71KB 翻译缓存**构建产物被 git 追踪**。去追踪 + 进 .gitignore。 `[locales/]` -1 产物
2. **delete:** `src/lsp/protocol.rs`（58 行）纯转发壳：`ok_response` ≡ `Response::new_ok`、`error_response` ≡ `Response::new_err` + cast、`notification` ≡ 结构体字面量。直接调 lsp_server，删文件；`method_not_found`/`internal_error` 内联进 server.rs。 ≈ -58 行
3. **delete:** 编译进度机器：`CompileProgress`（phase/percentage/current_line/total_lines/message）+ `CompilationPhase` 枚举 + `PipelineState` 六态 + `state()/reset()`。进度**零消费者**（无回调、无外部读取），PipelineState 唯一读者是 compiler.rs:199。编译是固定线性序列，不需要状态机编排，还原成 `compile(source) -> CompilationResult`。 ≈ -200~300 行 `[src/frontend/pipeline.rs, src/frontend/compiler.rs]`
4. **delete:** `EmitterConfig` 四个死旋钮：`show_source/show_help/show_related/show_line_numbers`——唯一构造点是 `{ use_colors, ..Default::default() }`，其余字段从未被改（ExecutorConfig 同款病）。删字段 + render 条件分支。 ≈ -40 行 `[src/util/diagnostic/emitter/text.rs]`
5. **shrink:** `InterpreterRuntimeConfig`（interpreter/runtime.rs）与 `RuntimeConfig`（facade.rs）同构（都是 mode+workers），execute.rs 逐字段搬运。合并为一个。 ≈ -20 行 + 少一个概念
6. **shrink（写码舒适度）:** 测试样板——RFC-027 e2e 两文件（411+456 行）+ predicate.rs（359 行）共 **8 处重复**手搓 `ConstExpr::BinOp{NamedVar, Lit(0)} + MonoType::Refined{Int(64)}` AST 树。两个方向：builder helper；或更 ponytail——**从 .yx 源码走现有 parser 构造**类型/约束（前端本来就是现成的）。 ≈ -200~300 行测试行，未来每个 RFC 测试都受益 `[src/frontend/core/typecheck/tests/rfc027_*]`
7. **DX 记录:** 全仓 753 处 `Expr::` match——穷举匹配是编译器安全网，不动；但这意味着 §15.frontend 第 1 条（重复解构抽 AST 方法）必须做，否则每种新表达式语法的成本是 ×5 文件起。

**§17 小计: ≈ -520~630 行 + 1 个缓存产物。**

## 全仓总账（三轮合并）

| 类别 | 砍幅 | 风险 |
|------|------|------|
| 删除类（§1-§13，全部实证） | ≈ -4100 行，-2 deps，1 dep 瘦身 | 低，先做 |
| 重构类（§16，bytecode 专项领头） | ≈ -3000~3900 行 | 中，排期 |
| 必要性类（§17，状态机/转发壳/死旋钮） | ≈ -520~630 行 | 低中 |
| **合计** | **≈ -7600~8600 行**（12 万行的 ~7%）+ 依赖清理 + 1 产物 | |

---

## §18 上限复核：12 万行到底还能挤多少

### 行数构成（120,281 行 .rs）

| 成分 | 行数 | 占比 |
|------|------|------|
| 测试代码（src 内 tests + 顶层 tests/*.rs） | 44,001 | 37% |
| 注释（其中 doc 注释 6,668） | 15,856 | 13% |
| 空行 | 11,589 | 10% |
| **生产代码（真身）** | **≈ 48,800** | **41%** |

12 万的观感里，真身只有约 4.9 万行。前三轮审计的 -7600~8600 行主要对着这 4.9 万行——**已是真身的 15-17%**。对一个语言级项目（lexer/parser/类型检查（所有权+依赖类型+终止性）/单态化/codegen/解释器+DAG 运行时/formatter/LSP/包管理器/REPL/i18n×6 语）这是很健康的挤压率；参照系：gleam 编译器约 10 万行（还没有解释器和包管理），roc 约 50 万行。

### 还没碰过的两个池子

1. **shrink（测试样板）:** 44,001 行测试，shingle 显示系统性样板：RFC-027 测试手搓 AST 树（8 处簇）、typecheck 测试目录**零共享 harness**（无 check_source 类助手）、ownership.rs 测试 95 处环境构造。两个杠杆：
   - 手搓 AST → 走现成 parser 从源码构造（前端是现成的）
   - 共享 `fn check(src) -> TypeResult` / `fn check_err(src, "E2015")` harness
   按 10-15% 压缩估。 `[src/**/tests/]` ≈ -3000~6000 行（低风险：测试自验证；工程量大）
   ⚠️ **合规约束（test-compliance）:** 压缩必须保留规则 2.1 文件头、3.1/3.2 命名、6.1/6.3/6.4 断言风格。harness 与规范同向：测试压到 ≤5 行后按规则 4.2 合法免除 AAA 分段注释，样板变短本身就是合规路径。
2. **delete（复读机注释，已按 test-compliance 规范修正）:** 初版误把受保护注释算进去了。项目硬规范（docs/src/dev/test-specification.md）强制的注释**不可删**：
   - 测试区（5,173 行注释几乎全保护）：规则 2.1 文件头 `//!` 规范声明（1,104 行）、规则 4.1 AAA 三段注释（1,619 行）、§/RFC 规范引用（616 行）
   - 生产区：规则 13.1 要求所有 pub 项带文档注释（6,022 行 ///）+ `//!` 模块文档（1,314 行）
   真可修剪的只剩：生产代码行内 `//` 复读注释（3,385 行里约 30-50% 复读，且必须避开解释性注释）+ 横幅装饰（283 行）。 ≈ **-800~1500 行**（原估 -2000~3000 过高，撤销）
   顺带发现（不属砍幅，反向缺口）：2,090 个 pub 项只有 42 个 doc-test 围栏，规则 13.1 远未落实——这是要**补**不是要删。
3. **shrink（微件）:** 65 个手工 `impl Default`（对照 69 个 derive）——挨个看能否 derive；50 个手工 Display 多为合理。 ≈ -100~200 行

### 诚实结论

- 这个仓库**相当健康**：yx_sources.rs 里已有 ponytail 注释（作者自觉过）、feature 门控完整、逐层调用图几乎全活。前三轮找到的死代码集中度（util/diagnostic + backends/common + frontend/config）就是全部重灾区，其余层是真代码。
- 加上本轮两个池子，**全仓理论上限 ≈ -11500~14500 行**（删除+重构+测试+注释全做满），其中生产 Rust ≈ -7000~9000 行、JSON ≈ -2700、测试 ≈ -3000~6000、注释 ≈ -800~1500（修正：测试区 3,339 行注释受 test-compliance 规则 2.1/4.1 保护，生产区 doc 受规则 13.1 保护且覆盖率仅 2%——是欠账不是砍点）。
- **再往上就不是砍过度工程，而是砍功能**：减 locale（6 语→2 语，约 -3000 JSON/-1600 MSG 映射的对应部分）、砍 LSP 能力面、合并 check/dump/build 子命令、删 formatter 规则。那是产品决策，不在 ponytail 审计范围——列出来只为说明上限在哪。

## 全仓总账（终版）

| 类别 | 砍幅 | 状态 |
|------|------|------|
| 删除类（§1-§13） | ≈ -4100 行 + -2 deps | 已实证，低风险，先做 |
| 重构类（§16，bytecode 专项领头） | ≈ -3000~3900 行 | 排期 |
| 必要性类（§17） | ≈ -520~630 行 | 低中风险 |
| 测试样板（§18.1） | ≈ -3000~6000 行 | 低风险高工程量 |
| 复读机注释（§18.2，合规修正后） | ≈ -800~1500 行 | 机械活，避开规范强制注释 |
| **合计** | **≈ -11500~14500 行**（12 万的 10-12%） | 生产真身挤压率 ~15-18%，近健康代码上限 |

合规注：测试区任何压缩（含 §18.1 的 harness 化）必须保留规则 2.1 文件头、3.x 命名、4.1 AAA、6.x 断言风格；harness 把测试压到 ≤5 行后按规则 4.2 反而合法免除 AAA 注释——压缩与合规同向。另：test-compliance 归 test-reviewer 技能执行，本审计只报不修。
