# tools/type-tables

P6（类型表示单一化）的**类型表生成期门禁**，形态与 [`tools/code-tables`](../code-tables) 对称：
独立 crate（不在根 workspace 内，根 `Cargo.toml` 的 `workspace.exclude` 已登记），
`syn` 只允许出现在本 crate。

## 两个消费者

| 消费者 | 跑什么 | 失败形态 |
| --- | --- | --- |
| 根 `build.rs`（`[build-dependencies]`） | **只跑 T1** | **构建硬失败**（6.6.2，2026-10-30）：违规即 `panic!` 拒绝编译；通过时只打一行摘要（`cargo:warning` 信息行） |
| `type-tables` bin | T1 + T2 | `--report` 只报告；仅内部错误（读不到文件 / syn 解析失败）时 exit 1 |

```bash
cargo run --manifest-path tools/type-tables/Cargo.toml -- --report   # T1/T2 文本报告
cargo run --manifest-path tools/type-tables/Cargo.toml -- --json     # T1 结构化结果
cargo run --manifest-path tools/type-tables/Cargo.toml -- --report --root <仓库根>
```

## T1 构造点分列（`ast::Type`）

**构造点 = 表达式位置的 `Type::<变体>` 路径**（`Expr::Path` / `Expr::Struct`）。模式位置（match
臂左侧、`matches!` 第 2 参、`if let`、参数模式）不计；宏体不解析；`tests` 目录整棵排除，
内联测试模块**按 cfg 谓词判定**（只有「要求 `test` 成立才能编译」的模块才排除：`cfg(test)` /
`all(test, …)`；`cfg(not(test))` 与 `cfg(feature = "test")` 仍会扫描）。识别为 `ast::Type`
的限定名：基础名 `Type`，以及**本文件按 `use` 声明解析出的别名**（如 `use …::ast::Type as AstType`、
再导出 `use …::ir::Type as IrType`）；泛型形参同名时以形参为准（遮蔽，`fn f<T>()` 里的 `T::X`
不是 `ast::Type`）。

三类**不得混算**：

1. **parser 前向**——不在任何重建位置的构造点（当前全部落在 parser）；
2. **AST→AST 重建**——`middle/passes/mono/function.rs` 的 `substitute_type_ast` 与
   `substitute_type_in_ast`（把已有 AST 类型重造一份；6.3 已删除其中 5 变体 / 10 点）；
3. **反向桥**——A：`middle/core/bytecode.rs` 的 `impl From<MonoType> for IrType`；
   B：`function.rs` 的嵌套 fn `mono_to_ast_type`。

两个数字分开报：**前向零构造**（有效前向计数为 0）与**全局零构造**（三类合计为 0）。

**重建位置的单一源（6.6.2，裁决 1A）**：第二类 / 反向桥的归类由 `src/scan.rs` 的 `REBUILD_FUNCTIONS`
表驱动（文件 + 函数名 → 类别）：`substitute_type_ast`、`substitute_type_in_ast`、
`without_fn_param_names` 归第二类，`mono_to_ast_type` 归反向桥 B。登记表与源码**逐条对账**——
登记的函数在源码里找不到同名 `fn`（改名/删除未同步）时扫描直接报错，不得静默落回 parser 前向。

**兜底豁免逐条登记（F1）**：前向有效计数 = 原始前向点数 − **已登记**的通配臂兜底豁免点数。
豁免只在 `REGISTERED_ZERO_FORWARD` 里显式登记。**WBS 6.3 之后该表为空**：03 §5.2 的 13 个
死变体已从 `ast::Type` 删除，`Void` 的 `parser/statements/types.rs` 错误兜底随变体删除改为
`Type::Name{"()"}`（不再需要豁免），`DEAD_VARIANTS_03` 亦已清空；
**未登记的通配臂构造点照常计入前向**——否则真实构造点会隐身，门禁仍报「名单一致」。
T1.7 用同一口径核对四项：未登记的零构造变体、登记但已不成立、登记豁免点未命中（兜底点消失后
须同步删登记）、未登记的兜底点。机制本身保留且可注入登记表（`*_with(registry)`）供单测。
**已转硬（6.6.2）**：新增零构造变体必须删除或在此登记理由，否则 `cargo build` 直接失败。

## T2 三表一致

| 表 | 位置 | 语义 |
| --- | --- | --- |
| A `from_builtin_name` | `src/frontend/core/types/mono.rs` | 名字 → `MonoType` 的解析表（每臂首写法=规范名） |
| B `CONST_PARAM_TYPES` | `src/frontend/core/parser/ast.rs` | 可作 const 泛型参数的名字（**A 的子集**） |
| C LSP 内置类型名清单 | `src/lsp/world.rs` | hover/definition 展示名（**A 规范名的超集**） |

三条检查：C1 B ⊆ A；C2 A 规范名 ⊆ C；C3 C ⊆ A。差异逐条给处置——处置表在
`src/dispositions.rs`（**待办清单，不是豁免白名单**：未登记的差异计入「未登记漂移」）。

## T3（与 RFC-011 的比对）——已按 2026-10-30 用户裁决删除

比对对象不存在：RFC-011 是泛型系统设计，正文没有以「内置类型名」为列的表格。原先的文档侧口径
（「全部 markdown 表格行里 inline code 包裹的首字母大写标识符」）只能把「文档不承载内置名清单」
这一事实登记成 16 条「无需处置」的空壳差异，无法判定真伪。

内置类型名的代码侧载体只有三张表（`from_builtin_name` / `CONST_PARAM_TYPES` / LSP 清单），
已由 T2 逐名对账（含 C1/C2/C3 与 T2.5 哨兵）。故检查实现（原 `src/rfc011.rs`）与那 16 条登记
同批删除，`--report` / `--json` 不再输出该节；判据与理由见 03 §5.7。

## 与 6.0 勘察的关系

`src/baseline.rs` 内嵌勘察基线的逐变体数字，`--report` 的 T1.6 节逐项对账（`实测(=基线)`）。
**6.6.2 起基线对账也进判定**（用户裁决 2A）：计数与基线不一致时 `cargo build` 同样失败。

**基线数字同时钉住「源码树」与「口径」（6.6.3 复审 F1/F2）**：

| 钉子 | 常量 | 含义 |
| --- | --- | --- |
| 源码树 | `SCOUT_BASELINE_REV` | 数字在哪份源码上量的 |
| 口径 | `SCOUT_BASELINE_CALIBER_REV` + `SCOUT_BASELINE_CALIBER` | 按哪一代 `REBUILD_FUNCTIONS` 归类量出来的（文件 + 函数 → 类别键） |

口径快照**不与登记表共表**：共表则「改口径」时两处一起变，门禁永远看不见口径变更。门禁把快照
与现行登记表机器化对账（`baseline::caliber_drift`）：新增/删除重建函数、或某函数类别变了，都报
「**口径与基线不同代**」（带口径 revision 与逐条差异）并**跳过**基线漂移比对——跨口径的点数不可比，
继续比对只会把「口径变了」伪装成「数字漂了」。`--report` 的 T1.6 节正文印出这两个 revision，
任何人拿它们都能复算。

不一致只有三种成立解释：源码新增/删除了构造点（以可复现证据刷新 `SCOUT_BASELINE`——报错现在带
**构造点 `file:line`**，与其余几类违规一样可定位）、口径被改（同批刷新口径快照与基线数字）、
或扫描实现漂了（修工具）。
