# 仓库维护机制与代码放置决策规程

> **规则本体已迁移（2026-10-05）**：三条禁令、D0–D4、红线、review 清单的**精确规则描述**见 [coding-rules.md](../coding-rules.md)——那是长期有效的唯一权威。本文保留为 **2026-10 诊断与决策记录**：事故证据、行号、外部惯例参照、被否方案的论证，随 RFC-039 同生共死。执行时请引用 coding-rules.md，不要引用本文作为规则来源。

> **附属设计文档**。本文是 [RFC-039 编译器架构重构](../../rfc/accepted/039-compiler-architecture.md) 的附属文档，对应 **P0 阶段**。
>
> 本文与 [09-execution-wbs.md](09-execution-wbs.md) 配套：P0 产出的规程约束 P1–P10 的每一个施工阶段；本阶段不产出任何编译器代码。

## 定位与范围

本文定义**"新代码放在哪、写成什么样、什么情况下必须停下来重构而不是打补丁"**的可执行规程。三条禁令：

1. **不得生造**——新增概念前必须证明它与既有概念不重复
2. **不得职责累积**——一个模块只承担一类职责；要加新职责就新建模块
3. **该重构却打了补丁**——每个改动都要先过决策程序

**本文覆盖**：新增/修改代码的决策流程、可机器检查的规则、代码审查清单、外部惯例参照。

**本文不覆盖**：四层模型与目录结构（见 `01-routing.md`）、阶段契约（见 `02`）、具体重构设计（见 `03`–`07`）。

## 现状：为什么需要这套机制

本仓库有**成熟的对策，也有同样的病症各出现过至少一次**。已查实的事故：

| 事故 | 规模 | 违反的禁令 |
| --- | --- | --- |
| 3 套平行类型表示（`ast::Type` / `MonoType` / `ir::Type`） + 序列化 `type_table`，靠手写同义词表 `"Int" \| "int" \| "i64" => ...` 弥合 | `ir.rs:3` 以 `pub use` 再导出；`mono.rs:618-643` 同义词表 | 生造 + 补丁掩盖 |
| 2 套平行运算符枚举（`ast::BinOp/UnOp` vs `const_data::BinOp/UnOp`），**无任何 `From`/`TryFrom`**，靠 import 别名 `AstBinOp`/`CEBinOp`/`B` 区分；bytecode 层还有第三套（`BinaryOp`/`UnaryOp`/`CompareOp`） | 5+ 文件起别名；`const_eval.rs:185-216` 手写有损转换 | 生造 |
| 5 个编译入口各自手工接线阶段 | 11 处行为不一致 | 补丁掩盖 |
| 8 棵测试子树从未参与编译（父模块漏写 `mod tests;`） | 1005 行 / 78 个测试从未运行 | 职责未分离（测试无处安放） |
| **`ir_gen.rs` 8448 行，因为它同时承担 6 类不相关职责**（符号表 / 模块级全局 / 语句 lowering / 常量折叠 / 循环迭代 / 调用分派） | — | **职责累积** |
| **`From<BytecodeFile>` 1400 行，因为把整个文件结构塞进一个 `From`** | — | **职责累积** |
| `include!` 拼接 1547 行、文件无模块身份 | 全仓唯一一处 `include!` | 职责累积 |
| **而同时**：`build.rs` 对错误码做了构建期硬门禁，不一致直接 `panic!` | 145 个码 + `--fix` 治愈 | —— 这是正面样本 |

**关键观察**：正面样本与事故在同一仓库。差别不在能力，在于有没有机制把"品味"变成机器可检查的规则。`build.rs:19-55` 已经证明了这条路径可行——**本机制要做的只是把它推广**。

## 目标设计

### 三条禁令的可操作定义

每条都给出**可测的判据**，不是口号。

#### 禁令一：不得生造

新增任何 `pub` 类型 / 枚举 / 常量表 / 概念之前，四条判据全部通过才算合规：

| 判据 | 内容 | 依据的本仓库事故 |
| --- | --- | --- |
| **A. 职责重叠检测** | 新增项与既有某项**变体名重合 ≥ 半数**且职责重叠 → 违规 | 3 套运算符枚举（ast / const_data / bytecode） |
| **B. 调用点不足** | 新增 `pub` 项的调用点 < 2（只有定义处 + 唯一调用处）→ 违规。无法证明它是"概念"而非局部实现 | — |
| **C. 需要消歧别名** | 需要 `use … as AstBinOp` / `CEBinOp` / `B` 这类别名才能区分同语义项 → 违规 | `const_eval.rs:19,1061`、`ownership.rs:19`、`termination.rs:15,226`、`ir_gen.rs:2317` |
| **D. 靠同义词表弥合** | 新增/修改项靠手写字符串匹配弥合差异 → 违规，必须改为穷举或显式转换 | `mono.rs:618-643` |

> **外部依据**：Go 官方 Code Review Comments 明确"**不要在有真实用例之前定义接口**——没有现实用例时，很难判断接口是否必要、更难判断该有哪些方法"（https://go.dev/wiki/CodeReviewComments ）。同文另一条"**好包名不该需要重命名**"正对应判据 C。

#### 禁令二：不得职责累积

> **决策（2026-10-03，ChenXu233）**：**不做行数 / 体积 / 文件大小门禁。** 本仓库的规模问题不通过数字解决，只通过**语义分离**解决——即"一个模块只承担一类职责"。本节原先的行数 ratchet 方案已废弃。

规模问题的正确处置是**重新划分职责边界**，不是给边界设置上限数字。理由：

- `ir_gen.rs` 8448 行的成因是**它同时承担了 6 类不相关职责**（符号表 / 模块级全局 / 语句 lowering / 常量折叠 / 循环迭代 / 调用分派）。拆成 6 个模块后每个自然只有几百行——**不需要任何行数规则**。
- `From<BytecodeFile>` 1400 行的成因是**一个 `From` 塞进了整个文件结构的逐字段搬运**。改成 82 个 `decode_<op>` 函数后自然变小。
- 反过来，设阈值会产生两种坏结果：要么把 8448 行合法化（取最大值），要么产生一次性整改压力（取中位数）。**两者都是在治标。**

| 判据 | 内容 | 依据的本仓库事故 |
| --- | --- | --- |
| **A. 职责类别判定** | 新增代码属于该模块**已有的职责类别**吗？属于第 2 类及以上 → 违规，应新建模块而非继续塞 | `ir_gen.rs` 单文件承担 6 类职责 |
| **B. 绕过权威源** | 新增表项但未更新唯一权威源（绕过 `code_tables` 手写副本）→ 违规 | 同义词表 + 平行枚举 |
| **C. 边界侵蚀** | 本次改动新增了任一跨层依赖（L2→L3、L4→L1/L2）、`include!`、`pub(crate)` 跨层泄漏 → 违规 | 2 处 `operator_interfaces::spec()`、唯一一处 `include!` |

**判据 A 是人工判断，不能机器化**——这正是它比行数规则更重要的原因：行数规则能被"拆成两个各 4000 行的文件"绕过，职责判定不能。

判据 C 可机器化，见下节「可机器检查的规则清单」。

> **外部依据**：Go 官方 Code Review Comments 明确反对行数规则：*"没有'函数不许超过 N 行'的规则……解决办法是改变函数边界，而不是开始数行"*（https://go.dev/wiki/CodeReviewComments#short-functions ）。**本仓库采纳这一立场**——差别只在于我们已确认"边界没画好"是真实病因，所以把边界定义写进判据 A，而不写进行数阈值。

#### 禁令三：该重构却打了补丁

| 判据 | 内容 | 依据的本仓库事故 |
| --- | --- | --- |
| **A. 同义重复** | 同一行为在 ≥2 处出现（别名 / 转换 / 阶段接线 / 错误映射）→ 违规，必须提到共享层 | import 别名、阶段接线 |
| **B. 扇出过大** | 一次修改要同步改 ≥3 处已有的同义映射 → **判定为架构变更**，停工，走设计文档流程 | 同义词表 + 3 套表示 |
| **C. 新增入口** | 新增"第 N 个入口/表项"而非登记进声明式阶段表 → 违规 | 5 个入口各自接线 |

### 决策程序

**每一步都是可判定的布尔条件。命中即止。**

```
D0  门禁    本次改动是否触碰任何一张已存在的表（错误码/opcode/类型/入口阶段）？
             ├─ 是 → 能指出该表的唯一权威实现模块吗？
             │        ├─ 能 → 继续
             │        └─ 不能 → ⛔ 停止。先建立权威源
             └─ 否 → 继续
                    ↓
D1  防生造   去掉新概念后，能否用「既有概念 + 参数」表达同样的语义？
             ├─ 能 → ⛔ 禁止新增。理由记入 PR
             └─ 不能 → 继续
                    ↓
D2  防并行   新概念是否与某既有概念同语义（变体名重合 ≥ 半数）？
             ├─ 是 → 有 From/TryFrom 转换且机检吗？
             │        ├─ 有 → 继续
             │        └─ 无 → ⛔ 合并为一份（禁止用 import 别名区分）
             └─ 否 → 继续
                    ↓
D3  补丁?    依次判定，命中即止：
             1) 同一行为需在 ≥2 处复制？        → 是 → ⛔ 必须提到共享层
             2) 需要新增"第 6 个入口"？          → 是 → ⛔ 禁止手工接线，改为登记进阶段表
             3) 一次修改要改 ≥3 处同义映射？    → 是 → ⛔ 判定为架构变更，停工走设计文档
             4) 以上皆否                        → 允许局部补丁，但必须附带回归测试
                    ↓
D4  职责?    本次新增代码属于目标模块已有的职责类别吗？
             ├─ 否（第 2 类及以上）→ ⛔ 新建模块，或把该职责的现有代码一并搬过去
             └─ 是 → 合规
```

**D0 的依据**：`build.rs:19-55` 已经实现了这个模式——`code_tables` 是错误码的解析/校验/比对的**唯一**实现位置，`validate` 不通过即 `panic!` 拒绝编译，并给出 `--fix` 治愈命令。**这不是新发明，是把既有范式从"只覆盖错误码"推广到"所有表"。**

**D1/D2 的外部依据**：Go 官方"**不要在有真实用例之前定义接口**"（https://go.dev/wiki/CodeReviewComments ）与"**好包名不该需要重命名**"（https://go.dev/wiki/CodeReviewComments#imports ）。D2 借鉴 RFC 2451 的唯一性思想（https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html ）。

**D4 是纯人工判断，没有机器替代**——这是它比行数规则更重要的原因：行数规则能被"拆成两个各 4000 行的文件"绕过，职责判定不能。`ir_gen.rs` 8448 行的成因是它**同时承担 6 类不相关职责**；拆成 6 个模块后每个自然只有几百行，不需要任何数字规则。

### 可机器检查的规则清单

| 规则 | 实现方式 | 外部参照 |
| --- | --- | --- |
| **表一致性**（错误码 / opcode / 类型表 / 测试接线） | 扩展现有 `build.rs` 门禁：`code_tables` 增加表类型，`parse` → `validate` → 不通过 `panic!`，配 `--fix` | 本仓 `build.rs:19-55` 已是范例 |
| **5 个入口阶段序列一致** | 新增**声明式阶段表**（`stage_table.rs`），5 个入口一律从它取序列；测试断言"5 个入口产生的阶段序列 == 表" | rustc 的 `rustc_queries!` 声明式查询表（https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md ） |
| **并行枚举缺转换** | CI 用 `syn` 遍历所有 `enum`：与既有 enum 变体名 Jaccard ≥ 0.5 且全库无 `From`/`TryFrom` → 失败 | RFC 2451 的唯一性思想（https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html ） |
| **重复概念检测** | 扫描全库 `pub enum/struct` 同名与职责重叠，命中报"二选一或写理由" | 同上 |
| **未编译测试树** | `walkdir` 扫：目录含 `#[cfg(test)]` 或测试模块文件，但父模块无对应 `mod` 声明 → 失败 | 同 `stage_table` 思路 |
| **消歧 import 别名** | 统计 `use … as …` 数量，超阈值要求附 `// reason:` 注释 | Go "避免重命名 import"（https://go.dev/wiki/CodeReviewComments#imports ） |
| **边界侵蚀**（禁令二判据 C） | 禁 `include!`；`pub(crate)` 跨层泄漏计数**只许减不许增**；禁 L2→L3 反向 `use` | rustc 的 provider 唯一归属 crate |

**最重要的一条是"5 个入口阶段序列一致"**——它直接把 D3-2 从"靠人记得"变成"表里没有就编译不过"。rustc 用 `rustc_queries!` 声明式查询表替代顺序 pass，是这个问题最成熟的解法（https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md ）。

> **不可照搬的部分**：rustc query 系统的核心收益是**增量编译与依赖图**。YaoXiang 若无增量编译需求，照搬整个 query 引擎是过度工程。**只借两层**：① 声明式阶段表；② 每个阶段有唯一归属模块。

### 外部惯例参照

| 惯例 | 来源 | 对本仓库的适用性 |
| --- | --- | --- |
| 编译器 = 一串按目的区分的 IR（Token/AST/HIR/THIR/MIR/LLVM-IR），每个 IR 明确"为何存在" | rustc-dev-guide https://rustc-dev-guide.rust-lang.org/overview.html | **高**。直接给出"新增第 4 套类型表示必须论证它服务于哪个新目的"这一门槛 |
| 目录名 = 流水线阶段名，跨阶段复用同一构造名 | 同上 | **高**。`src/` 顶层应按阶段命名，不按"工具/杂项"命名 |
| **不用顺序 pass 组织编译器**，用声明式查询表 + provider 唯一归属 | rustc-dev-guide https://github.com/rust-lang/rustc-dev-guide/blob/master/src/query.md | **最高**。这正是"5 个入口各自手工接线"的对症药 |
| 中央上下文对象持有全部查询与缓存 | rustc-dev-guide | 中。可作统一编译上下文参照，收益取决于是否有增量编译 |
| **接口放在使用方包，不放实现方包** | Go CodeReviewComments https://go.dev/wiki/CodeReviewComments#interfaces | 高。适合判"新抽象该下沉到哪一层" |
| **不要在有真实用例之前定义接口** | 同上 | **最高**。这是"不得生造"最贴切的外部表述 |
| **避免为消歧而重命名 import** | https://go.dev/wiki/CodeReviewComments#imports | **最高**。直击 `AstBinOp`/`CEBinOp`/`B` 别名 |
| 避免 `util` / `common` / `misc` / `api` 这类无意义名 | https://go.dev/wiki/CodeReviewComments#package-names | 中 |
| **机械问题交给工具，文档只管非机械问题** | CodeReviewComments 开头 | 高。决定"哪些规则该机器化"的边界 |
| 唯一性由语言强制：coherence 保证"对任意 trait+type 有且只有一个 impl" | RFC 2451 https://rust-lang.github.io/rfcs/2451-re-rebalancing-coherence.html | **最高**（思想层面）。把"重复定义"从审查话题变成机器不变量 |
| 一阶段一文件 | Zig `lib/std/zig/`（https://github.com/ziglang/zig/tree/master/lib/std/zig ） | **低**。见下 |
| **不设行数 / 体积规则**；"解决办法是改变函数边界，而不是开始数行" | Go CodeReviewComments https://go.dev/wiki/CodeReviewComments#short-functions | **最高**。本仓库已采纳（2026-10-03 决策） |

### 不能直接照搬的

1. **Zig 的"一阶段一文件"**。Zig 的阶段划分清晰（`Ast.zig` / `Parse.zig` / `AstGen.zig` / `Zir.zig`），但**文件规模完全失控**——`AstGen.zig` 576 KB、`Zir.zig` 210 KB、`Ast.zig` 148 KB（https://github.com/ziglang/zig/tree/master/lib/std/zig ）。**"按阶段分目录"可借，"一阶段一文件"不可借。**
2. **coherence / orphan 规则**。RFC 2451 解决的是**跨 crate** impl 冲突；本仓库的 3 套类型表示、2 套运算符枚举是**单 crate 内**的重复定义，语言级 coherence 不会报错，**必须自建 CI 检查**。

### 代码审查清单

每个 PR 的 review 按此逐条过：

- [ ] **D0** 是否触碰了任何一张表？该表的唯一权威实现模块是哪个？
- [ ] **D1** 新概念能否用既有概念 + 参数表达？不能的话理由写在哪？
- [ ] **D2** 是否与既有概念同语义？转换在哪里？是否需要 import 别名？
- [ ] **D3** 同一行为是否在 ≥2 处出现？是否新增了入口而非登记进阶段表？是否要改 ≥3 处同义映射？
- [ ] **D4** 本次新增代码属于目标模块**已有的职责类别**吗？还是第 2 类及以上？
- [ ] **边界**是否被侵蚀？新增跨层依赖 / `include!` / `pub(crate)` 跨层泄漏？
- [ ] **机械问题**是否已由工具兜住（fmt / clippy / `build.rs` 门禁）？review 是否只花在非机械问题上？

最后一条直接抄 Go 的分工原则：**机械问题交给工具，review 只看非机械问题**（https://go.dev/wiki/CodeReviewComments ）。

## 详细设计

### 产出物清单

| 文件 | 内容 | 类型 |
| --- | --- | --- |
| `CONTRIBUTING.md` 的"代码放置与变更规程"一节 | D0–D4 决策程序 + 三条禁令 | 文档 |
| `scripts/ci/check-concepts.py` | 禁令一的 A/B/C/D 判据（平行表示、消歧别名、同义词表、调用点不足） | 门禁脚本 |
| `scripts/ci/check-fanout.py` | 禁令三的 A/B/C 判据 | 门禁脚本 |
| `scripts/ci/check-boundary.py` | 禁令二的判据 C（禁 `include!`、`pub(crate)` 跨层泄漏只许减、禁 L2→L3） | 门禁脚本 |
| `tools/code-tables` 扩展 | D0 的表类型注册 | 复用既有 |

**没有 `baseline.toml`，没有 `clippy.toml` 阈值配置，没有 `check-scale.py`。** 规模问题由职责分离解决（禁令二判据 A，人工判断），不设数字门禁。

### 职责类别的定义方式

D4 判据 A 需要"这个模块已有哪些职责类别"这一事实。做法：

1. 在 `01-routing.md` 的**目标目录结构**中，每个目录已写明职责与边界不变量——**这就是职责类别的权威定义**
2. 提交 PR 时，在 PR 描述中声明"本次新增代码属于 `<目录名>` 的哪一条职责"
3. 若目标目录的职责清单里没有对应条目 → 说明这个职责是新类别，**要么扩职责清单（并说明为什么该放这），要么新建目录**

`check-boundary.py` 负责机械部分（跨层依赖 / `include!` / `pub(crate)`），职责归属本身由人判断——这正是它需要写进 `CONTRIBUTING.md` 而不是脚本的原因。

## 实施要点

| 步骤 | 内容 | 验收 |
| --- | --- | --- |
| 1 | 在 `CONTRIBUTING.md` 写入 D0–D4 与三条禁令；职责类别引用 `01-routing.md` 的目录职责表 | 人工 review 通过 |
| 2 | 实现 `check-concepts.py`（禁令一四条判据），先在**未改动**的代码上跑 | **应当报出现有的 3 套运算符枚举、2 套平行类型表示（`ir::Type` 别名一并计入）**——报出来才算门禁有效 |
| 3 | 实现 `check-fanout.py`（禁令三） | **故意新增第 6 个入口式接线必须红** |
| 4 | 实现 `check-boundary.py`（禁令二判据 C） | **故意加一处 L2→L3 反向 `use` 必须红**；`pub(crate)` 泄漏计数记录初始值 |
| 5 | 扩展 `tools/code-tables` 覆盖 opcode / 类型表 | 故意引入零构造变体必须红（依赖 P6 的 T1） |

**步骤 2 是本阶段的核心验收**：`check-concepts.py` 必须能**在当前未改动的代码上**报出已知的 2 套运算符枚举。如果它报不出来，说明判据设计错了，门禁是假门禁。

## 关键决策与理由

- **不设任何行数 / 体积门禁**（2026-10-03 决策）——采纳 Go 官方立场（https://go.dev/wiki/CodeReviewComments#short-functions ）。本仓库的规模问题成因是 `ir_gen.rs` 同时承担 6 类职责、`From<BytecodeFile>` 把整个文件结构塞进一个 `From`——**拆掉职责，数字自然变小，不需要数字规则**。原设想的 `baseline.toml` ratchet 与 `clippy.toml` 阈值均已废弃。
- **职责类别的权威定义放在 `01-routing.md` 的目录职责表**，不放在本篇——避免两处定义漂移。
- **只借 rustc query 的两层，不搬引擎**——声明式阶段表 + provider 唯一归属。query 引擎的核心收益是增量编译，本仓库无此需求。
- **coherence 思想要自建实现**——RFC 2451 管跨 crate，本仓库的问题是单 crate 内重复，语言不会报错。

### 未采纳的方向

- **行数 ratchet / `baseline.toml`**——本仓库的规模问题成因是职责未分离，设阈值只会把 8448 行合法化，或产生一次性整改压力。**已废弃。**
- **`clippy.toml` 的 `too-many-lines-threshold` / clang-tidy `readability-function-size`**——同上，**已废弃**。
- **照搬 rustc query 引擎**——过度设计，只借声明式表。

## 已知局限与风险

- **判据的假阳性**：禁令一 A 判据用"变体名重合 ≥ 半数"是启发式，两个真的无关的枚举可能撞名（如 `Field`）。缓解：允许 `// reason:` 注释豁免。
- **`syn` 分析的精度**：`syn` 解析宏展开后的形态有限，跨宏生成的定义可能漏检。缓解：门禁只报"疑似"，由人裁决。
- **职责判定不可机器化**：禁令二 A 是人工判断。这意味着**它只能靠 review 执行，无法靠 CI 强制**——这是本机制最大的软肋。缓解：把职责清单写进 `01-routing.md` 使其可核对；`check-boundary.py` 兜住机械部分（跨层依赖 / `include!` / `pub(crate)` 泄漏）。
- **门禁可能被绕过**：`// reason:` 注释是合法逃逸口。缓解：豁免数量本身进 `check-boundary.py` 的报告。

> **本节原列的开放问题已全部裁决。** 逐条决定见 [RFC-039 决议登记](../../rfc/accepted/039-compiler-architecture.md)（D1–D50）。**本文不留任何待定项。**
>
## 参见

- [01-routing.md](01-routing.md) — **职责类别的权威定义**（目录职责与边界不变量表）、依赖方向规范、目标目录结构、路由表 C
- [02-stage-contract.md](02-stage-contract.md) — `Stage` 声明式表（D0/D3-2 的落点）
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — 本规程要清理的历史事故
- [09-execution-wbs.md](09-execution-wbs.md) — P0 的三级任务拆解
- [RFC-009a 借用证明管线](../../rfc/accepted/009a-borrow-proof-pipeline.md)
- [RFC-013 错误码规范](../../rfc/accepted/013-error-code-specification.md) — 现有门禁的规范来源
- `build.rs:19-55` — 本仓已有的 `panic!` 级门禁范例，D0/D0 门禁模式的直接依据
