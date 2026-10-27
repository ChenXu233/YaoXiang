# tools/type-tables

P6（类型表示单一化）的**类型表生成期门禁**，形态与 [`tools/code-tables`](../code-tables) 对称：
独立 crate（不在根 workspace 内，根 `Cargo.toml` 的 `workspace.exclude` 已登记），
`syn` 只允许出现在本 crate。

## 两个消费者

| 消费者 | 跑什么 | 失败形态 |
| --- | --- | --- |
| 根 `build.rs`（`[build-dependencies]`） | **只跑 T1** | 首版**只发 `cargo:warning`**，不 panic、不使构建失败（6.6「门禁转硬」才改 panic） |
| `type-tables` bin | T1 + T2 + T3 | `--report` 只报告；仅内部错误（读不到文件 / syn 解析失败）时 exit 1 |

```bash
cargo run --manifest-path tools/type-tables/Cargo.toml -- --report   # T1/T2/T3 文本报告
cargo run --manifest-path tools/type-tables/Cargo.toml -- --json     # T1 结构化结果
cargo run --manifest-path tools/type-tables/Cargo.toml -- --report --root <仓库根>
```

## T1 构造点分列（`ast::Type`）

**构造点 = 表达式位置的 `Type::<变体>` 路径**（`Expr::Path` / `Expr::Struct`）。模式位置（match
臂左侧、`matches!` 第 2 参、`if let`、参数模式）不计；宏体不解析；`tests` 目录与
`#[cfg(test)] mod tests` 整棵排除。识别为 `ast::Type` 的限定名：`Type` / `AstType` / `IrType`，
以及文件内 import 了 `Type as T` 时的 `T`。

三类**不得混算**：

1. **parser 前向**——不在任何重建位置的构造点（当前全部落在 parser）；
2. **AST→AST 重建**——`middle/passes/mono/function.rs` 的 `substitute_type_ast` 与
   `substitute_type_in_ast`（把已有 AST 类型重造一份；03 与 09 都没登记这一类）；
3. **反向桥**——A：`middle/core/bytecode.rs` 的 `impl From<MonoType> for IrType`；
   B：`function.rs` 的嵌套 fn `mono_to_ast_type`。

两个数字分开报：**前向零构造**（第 1 类为 0）与**全局零构造**（三类合计为 0）。
`Void` 的唯一前向点是 `_ => (…, Type::Void)` 错误兜底（03 §5.2 #8 / 6.4 #12 消除），
按「通配 match 臂内的兜底点」结构性识别并单列，故它仍计入前向零构造但保留原始计数。

已登记的前向零构造变体清单在 `src/t1.rs` 的 `REGISTERED_ZERO_FORWARD`：6.6 转硬后，
**新增前向零构造变体必须删除或在此登记理由**（03 §5.7 的白名单机制）。

## T2 三表一致

| 表 | 位置 | 语义 |
| --- | --- | --- |
| A `from_builtin_name` | `src/frontend/core/types/mono.rs` | 名字 → `MonoType` 的解析表（每臂首写法=规范名） |
| B `CONST_PARAM_TYPES` | `src/frontend/core/parser/ast.rs` | 可作 const 泛型参数的名字（**A 的子集**） |
| C LSP 内置类型名清单 | `src/lsp/world.rs` | hover/definition 展示名（**A 规范名的超集**） |

三条检查：C1 B ⊆ A；C2 A 规范名 ⊆ C；C3 C ⊆ A。差异逐条给处置——处置表在
`src/dispositions.rs`（**待办清单，不是豁免白名单**：未登记的差异计入「未登记漂移」）。

## T3 与 RFC-011 一致

RFC-011 **没有**以「内置类型名」为列的表格（03 §5.7 假定它对标 RFC-013 的码表，但 RFC-013 是
码表规范、RFC-011 是泛型系统设计）。故文档侧口径 = **RFC-011 全部 markdown 表格行里、inline code
包裹且整体是首字母大写标识符的 token**；另附「全文是否提及」信息列，并用 `src/dispositions.rs`
逐条登记差异与处置。T3 的结论里如实写明这条前提。

## 与 6.0 勘察的关系

`src/baseline.rs` 内嵌 6.0 勘察（冻结 `bfe2f6ba`）的逐变体数字，`--report` 的 T1.6 节会逐项对账
（`实测(=基线)`）；不一致即扫描口径漂移，需以可复现证据裁决。基线不参与 build.rs 的判定。
