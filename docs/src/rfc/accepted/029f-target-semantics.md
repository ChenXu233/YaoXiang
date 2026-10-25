---
title: 'RFC-029f: 编译目标角色与导入面语义'
status: '已接受'
author: '晨煦'
created: '2026-09-12'
updated: '2026-09-13'
accepted: '2026-09-13'
issue: '#334'
---

# RFC-029f: 编译目标角色与导入面语义

## 摘要

> **落地修正（2026-10-08，RFC-029g）**：`pub` 关键字已由 RFC-029g 整体删除。本文正文中
> 所有「`pub` 豁免 / `pub` 文件 / 未用 pub 报警」的表述，请按「**顶层绑定 + 包内引用池**」
> 读取：角色分类（Script / Bin / Lib / Internal / Test）与导入面语义不变，变化只在死代码
> 豁免的判定条件（见 [029g](029g-remove-pub-and-auto-bind.md) 与
> `docs/src/reference/warning-code/warning-codes.md` 的判定口径）。

兑现 RFC-029「子 RFC 规划」的延伸槽位（029b–029e 已占，顺延 029f）：为源文件定义
**编译目标角色模型**（Script / Bin / Lib / Test / Internal 五类）及其**导入面语义**——
无 manifest 时按入口可达性推断（零配置），有 manifest 时按 `[lib]` / `[[bin]]` /
`[exports]`（RFC-015 已定义的字段）显式声明。填补四项悬空语义：跨包导入面按什么算、
死代码警告的 pub 豁免适用域（#321 定案 B）、`[exports]` 与 Registry 可达性的关系、
以及 RFC-014b `[binaries]`（预编译分发产物）与 bin 角色源文件的名词消歧。

## 动机

### 宿主与边界依据

RFC-029（已接受）的入口选择与可见性决策是本 RFC 的直接上游：

> **入口文件选择**优先级：1. `[run].main` 2. `[[bin]]` 第一项的 `path`
> 3. `src/main.yx`（约定默认）。—— RFC-029 §包间循环：再说

> **可见性**：不存在。作用域 + 分发边界覆盖所有场景，不需要新关键字。
> —— RFC-029 §设计决策记录（2026-07-30）

第二条是本 RFC 的设计红线：导入面语义**不得引入新的可见性关键字**，
只能由"分发边界"（文件角色 + 声明导出面）承担。

四个消费方各自撞到了同一堵墙：

| 消费方 | 撞墙点 |
| ------ | ------ |
| #321 死代码警告 | 定案 B「pub = 对外接口永不报」是因为没有角色模型区分"真对外"与"看着像对外"；bin 内未用 pub 报不报（方案 A）悬置至今 |
| RFC-014c 工作空间（审核中，#113） | 成员包互相 `use` 时导入面按什么算——`[exports]`、`[lib]` 单文件、还是全部 pub 文件——**全文无一条语义条文** |
| RFC-015 配置系统（已接受） | `[lib]` / `[[bin]]` / `[exports]` 字段已定义，除 RFC-029 消费了入口优先级外，其余字段的语义（尤其 `[exports]` 与 Registry 可达性的关系）无人对齐 |
| RFC-037 打包 / RFC-029a 缓存（草案，#293） | 打包产物选择、增量重编译的缓存单元边界，都需要角色模型作前置 |

### 当前的问题

**四项语义空白**（2026-09-12 盘点，翻 014/014a/014b/014c/015/029 全文确认）：

1. **跨包导入面无定义**。014c 的成员包互引示例只有目录结构（`src/lib.yx`），
   `use utils.helper` 能拿到什么没有任何条文。当前隐式规则是"该包所有顶层
   pub 文件"——内部实现文件也被拖进命名空间，依赖边界形同虚设。
2. **死代码 pub 豁免没有适用域**。#321 定案 B 的"pub 永不报"在单文件脚本里
   过保守（脚本没有外部消费者，pub 无意义），在多文件项目的入口文件里又
   过宽松（无人 import 根文件的 pub 就是死代码）。没有角色模型，A/B 两案的
   切分线画不出来。
3. **三个导出概念无人对齐**。`[exports]`（015，路径映射）、`[lib]`（015，
   单文件路径）、Registry 可达性（029，从入口沿 use 可达）——三者是包含
   关系、相等关系还是各管各的，无条文。
4. **名词撞车**。RFC-014b 的 `[binaries]` 是**预编译分发产物**（.so/exe 下载
   优先策略），与"bin 目标（源码角色）"完全两个层面。014c 落地后两者必然
   同屏出现，不趁早消歧会持续产生误解。

**实证：B 定案的保守性已经是实际痛点**。#321 M2 复盘记录：默认配置下
W1001–W1005 全族静默的结构性原因正是"所有 pub 都被当对外接口"——这是
模型缺失的补偿行为，不是终态。

## 提案

### 核心设计：文件角色模型

**角色是文件级属性**（与 015 的字段粒度一致：`[lib]` 是单文件、`[[bin]]`
是文件列表、`[exports]` 是文件映射），不是包级开关。

| 角色 | 判定 | 语义 |
| ---- | ---- | ---- |
| **Bin** | `[run].main` / `[[bin]].path` 指向的文件；或（无 manifest 时）含 `main` 且未被任何文件 `use` 的文件 | 程序入口。要求存在名为 `main` 的**绑定**，值或函数皆可：函数 main 入口期零参调用，值 main 初始化期求值即执行（求值结果即使是函数值也不再调用）。缺 `main` 编译错误（E3020）；顶层不得有可执行语句。其 `pub` **可报死代码**（无包外消费者）；`main` 为可达性根 |
| **Lib** | `[exports]` 映射命中的文件；或 `[lib].path`；或（无 manifest 时）被其他文件的 `use` 引用到的文件 | 分发边界。其 `pub` **豁免死代码**（包外消费者不可见，宁漏报）；经它导出的符号跨包可见 |
| **Test** | RFC-036 测试文件规则命中的文件（`tests/` 目录、`*_test.yx` 等既有约定）；**不设** `[[test]]` 显式声明 | 测试代码。不参与死代码判定；其引用计入被测代码的可达性根 |
| **Internal** | 包内有 manifest 时，既不在导出面也不含 main 的文件 | 包内实现。`pub` 豁免至二阶段（按包内 use 图可达性收紧，以 Bin 角色语料验证为前置）；包外 `use` **不可达** |
| **Script** | 单文件直跑（`run foo.yx`）的文件 | **无入口概念**。顶层语句（含绑定初始化）按书写顺序即程序主体；`main` 是普通绑定，**不自动调用**——想跑写 `main()`。Registry 只有 std |

**判定优先级**：manifest 显式声明 > 入口可达性推断 > 036 测试约定。
显式声明永远赢——推断只是无声明时的默认。

### 入口判定

角色模型不仅用于死代码分析，也**驱动入口判定**。各角色的入口规则：

| 角色 | 入口规则 |
| ---- | -------- |
| **Script**（无 manifest） | **无入口概念**。顶层语句（含绑定初始化）按书写顺序即程序主体；`main` 是普通绑定，**不自动调用**——想跑写 `main()` |
| **Bin**（有 manifest） | 要求存在名为 `main` 的**绑定**（值/函数皆可：值 main 初始化期求值、函数 main 入口期零参调用）；缺 `main` 编译错误（E3020）。顶层不得有可执行语句 |
| **Lib / Internal / Test** | 不要求 `main`（它们不是入口） |

**Script 与 Bin 的入口必须互斥**：「Script 下 `main` 不特殊」与「Bin 下 `main` 是入口」
不能同时成立——若 Script 既执行顶层语句又隐式调 `main`，显式写了 `main()` 的脚本会双跑
（#356）。故 Script 只有一个执行入口：顶层语句。

**角色判定与入口判定的区别**（实现注记）：入口判定用「有无 manifest」
（`find_project_root().is_some()`）而非 `roles::classify()`。原因：classify
回答「这个文件被谁消费」（服务于死代码分析）；有 manifest 却没 `main` 的文件
在 classify 里是 Internal（它不被别的文件 use，合理），但用户把它当入口跑了，
此时必须有入口。两个问题不同，判据自也不同。

**值 `main` 的执行形态**（#388 定案）：函数已是值（类型层统一），入口按
**绑定存在性**判定而非可调用性——值 main 与函数 main 只是求值策略不同。
值 main 不设入口调用（#356 防双跑：初始化与入口调用绝不叠加），与 Script
同款执行形态；其求值就是初始化序列的一部分：按全局绑定拓扑序执行，
独立绑定按源序。因此把 Script 顶层语句包进 `main = { ... }` 迁入 Bin
**不是保序迁移**——散语句按书写序执行，绑定初始化按拓扑序；需要先行
的初始化应让 `main` 显式依赖它。`main: Int = 5` 类不可调用值 main 是
合法但无观察效果的程序（对标 Rust 空 `fn main() {}`）。

详见 `docs/src/reference/language-spec/syntax.md` §3.11。

### 示例

```toml
# yaoxiang.toml（字段全部是 RFC-015 已定义的，本 RFC 不新增配置面）
[lib]
path = "src/lib.yx"

[[bin]]
name = "my-cli"
path = "src/cli.yx"

[exports]
"." = "src/lib.yx"
"./internal-helper" = "src/helper.yx"   # ← 决定跨包可见性的是它，不是 pub
```

```yaoxiang
# src/cli.yx（Bin 角色）
pub unused_fn = (x: Int) => x    # ← W1001 可报：bin 无包外消费者
main: () -> Void = { ... }

# src/lib.yx（Lib 角色，在导出面上）
pub api_fn = ...                 # ← 永不报：包外消费者不可见

# src/other.yx（Internal 角色，不在导出面）
pub semi_api = ...               # ← 豁免至二阶段（宁漏报），届时按 use 图收紧
```

### 导入面语义

跨包 `use pkg.x` 的解析序（第一条命中即生效）：

1. `[exports]` 映射存在 → 导入面 = 映射列出的文件集合，`x` 须在这些文件的
   顶层绑定中；
2. 否则 `[lib].path` 存在 → 导入面 = 该单文件；
3. 都没有（无 manifest 依赖、路径依赖裸目录）→ **维持现状**：全部顶层文件
   可导入（兼容既有行为，收紧另议）。

**工作空间成员**：导出面只由成员包自己的 manifest 定义，workspace 根不得
覆盖或扩展成员导出面——成员自包含是 014c 的核心设计，根覆盖会破坏封装。

与 Registry 可达性的关系（对齐 029）：`[exports]`/`[lib]` 定义的是**跨包
解析的合法起点集合**；起点确定后，Registry 仍按 029 的"从入口沿 `use`
可达"扩展——导出文件自身 `use` 进来的内部文件随链接进入，但不产生新的
跨包起点。

### 与既有规划的划界

| 邻居 | 边界 |
| ---- | ---- |
| RFC-029d「CLI `--entry` 覆盖入口」（规划中） | 029f 定义角色模型；029d 消费它——`--entry` 是 Bin 角色的 CLI 级覆盖手段 |
| RFC-014b `[binaries]` | **名词消歧**：`[binaries]` 是预编译分发产物（下载优先策略），`[[bin]]` 是源码角色声明。前者存在于依赖包的 manifest，后者存在于本包 manifest，不互换 |
| RFC-014c 工作空间 | 成员包互引的导入面 = 本 RFC 解析序；workspace 协调机制（members、共享 lockfile）仍归 014c |
| RFC-036 测试 | Test 角色判定直接采用 036 的既有文件规则，不新增约定 |
| #321 死代码 | 定案 B（pub 一律豁免）显式降级为"无 target 时的过渡语义"；Bin 角色的 pub 可报是本 RFC 兑现方案 A 的形态 |

## 详细设计

### 角色判定算法（编排器内）

```
fn classify(files, manifest, entry_reach) -> Map<File, Role>:
    # 1. 显式层：manifest 声明优先
    for f in manifest.exports.values():  role[f] = Lib
    if manifest.lib_path:                role[lib_path] = Lib
    for b in manifest.binaries:          role[b.path] = Bin
    if manifest.run_main:                role[run_main] = Bin

    # 2. 推断层：无声明处按入口可达性
    for f in files where role[f] 未定:
        if f 被 ≥1 个非自身文件 use:      role[f] = Lib
        elif f 含 main 且无其他文件 use 它: role[f] = Bin
        else:                             role[f] = Internal

    # 3. 测试层：036 规则命中 → Test（覆盖 Lib/Internal，不覆盖显式 Bin）
    apply_rfc036_test_rules(files, &role)

    # 单文件直跑：整个模型旁路，行为不变
    if no manifest and single_file:      all Script
```

### 编译器改动

- `frontend/config.rs`：manifest 解析补 `[exports]` → 角色表的映射
  （字段解析 015 已有）。
- `frontend/module/orchestrator.rs`：Registry 构建前插入角色分类阶段；
  跨包解析按导入面序校验 `use` 目标，越界报既有 `module_not_found` 族
  （不新增错误码，消息补"不在导出面"提示）。
- `typecheck/passes/dead_code.rs`：入口点集合从"main + 全部 pub"改为
  "main + Lib 角色文件的 pub"（Bin 立即可报；Internal 豁免至二阶段，
  Phase 2 见实现策略）。
- LSP（RFC-017）：补全/悬停的"可导入项"按导入面过滤；`main` 缺失诊断
  仅对 Bin 角色文件报。

### 运行时行为

无变化。角色模型是纯编译期/解析期语义，不影响 IR 与执行。

### 向后兼容性

- 无 manifest 项目：行为完全不变（推断层还原现状——被 use 的是 Lib、
  含 main 的是 Bin）。死代码警告的增量仅在 Bin 文件：无人 import 根文件的
  unused pub 从静默变为 W1001——这是 #321 方案 A 的预期行为，非破坏。
- 有 manifest 项目：`[exports]` 已声明的，导入面从"全部 pub 文件"收窄为
  声明面。**这是唯一的行为收窄点**：依赖他人包内部文件的代码会开始报
  `module_not_found`。鉴于包生态尚未建立（029：「当前无第三方包生态」），
  破坏面为零；仍设一个次版本过渡期（越界先警告后报错）。

## 权衡

### 优点

- 零新配置面：字段全是 015 已定义的，本 RFC 只补语义。
- 零新关键字：导入面由分发边界承担，与 029「可见性不存在」决策同构。
- 四个消费方（#321 / 014c / 029a / 037）一次建模，不再各撞各的墙。
- 无 manifest 路径零成本：脚本语言的零配置基因完整保留。

### 缺点

- 角色推断（被 use 即 Lib）在"入口文件 use 工具文件"的常见形态下会把
  工具文件判成 Lib，其 pub 豁免——比理想粒度宽。这是宁漏报方向的有意
  选择，收紧需要 use 图上的调用方向分析，第一版不做。
- `[exports]` 收窄导入面是有破坏性的语义收紧（虽有过渡期）。

## 替代方案

| 方案 | 描述 | 未采纳原因 |
| ---- | ---- | ---------- |
| 顶层 RFC-040 | 新开顶层编号 | 模块图语义（导入面/可达性/入口）全在 029 领地，拆顶层会造成同一对象两份顶层文档；且 029b–029e 槽位模式已确立 |
| 挂入 014c | 作为 workspace 的一节 | 依赖方向反了：语言层（029x）定义语义、包层（014x）消费语义。014c 还在审核中，中途扩范围拖累其落地 |
| 方案 A 原样（#321 菜单） | manifest 强制 `[[bin]]`/`[[lib]]` target 才有死代码语义 | 为窄切片（带 main 根文件的 unused pub）引入强制配置；本 RFC 的推断层以零配置获得同等收益 |
| 纯入口推断（无 manifest 层） | 不消费 015 字段 | `[exports]` 已是 015 已接受字段，跨包导入面绕不开它；不定义等于留空白给 014c |

## 实现策略

### 分阶段

- **Phase 1**：角色分类 + 导入面解析 + Bin pub 可报（Internal/Test 维持豁免）
- **Phase 2**：Internal pub 收紧为"包内 use 图不可达即可报"——前置条件为
  Phase 1 落地后 Bin 角色警告语料验证无误报回调；触发即做，不设时间表

### 依赖关系

- 前置：RFC-029 编排器（已落地）、RFC-015 manifest 字段（已定义）。
- 被依赖：#321（Bin pub 可报）、RFC-014c（导入面）、RFC-029a（角色作为
  缓存单元边界）、RFC-037（产物选择）、RFC-029d（`--entry` 覆盖）。
- 与 RFC-014c 的落地顺序：014c 审核通过前，029f 的导入面序应先定稿，
  014c 以引用方式消费，避免审核中途扩范围。

### 风险

- 角色推断与用户直觉的偏差（上述权衡第一条）→ 以警告（不阻断）形式
  呈现全部增量行为，可随时 disable。
- `[exports]` 收窄的过渡期管理 → 次版本内先警告。

## 开放问题

草案阶段三项均已定案（2026-09-13，决议入附录B 与正文），无未决项：

- [x] Internal pub 是否收紧 → **定案**：二阶段按包内 use 图可达性收紧，
  以 Bin 角色语料验证为前置（见实现策略 Phase 2）
- [x] `[[test]]` 显式声明 → **定案**：不引入，Test 角色仅由 036 规则
  判定；036 将来有显式声明需求时由其自身扩展
- [x] workspace 根覆盖成员导出面 → **定案**：不允许，成员自包含是 014c
  核心设计，根覆盖破坏封装

---

## 附录B：设计决策记录

| 决策 | 决定 | 日期 | 记录人 | 依据 |
| ---- | ---- | ---- | ------ | ---- |
| 编号 | 029f（029b 已预留热重载、029c 已删除、029d/029e 已占位） | 2026-09-12 | 晨煦 | RFC-029 §子 RFC 规划 |
| 归属 029 家族而非顶层 RFC | 是 | 2026-09-12 | 晨煦 | 导入面/可达性/入口同属模块图语义；029 已消费 `[[bin]]` 入口优先级 |
| 可见性关键字 | 不引入 | 2026-09-12 | 晨煦 | 与 RFC-029「可见性不存在」决策同构，导入面由分发边界承担 |
| #321 定案 B 的地位 | 降级为"无 target 时的过渡语义" | 2026-09-12 | 晨煦 | Bin 角色 pub 可报即 #321 方案 A 的兑现形态 |
| 角色粒度 | 文件级 | 2026-09-12 | 晨煦 | 015 字段粒度（`[lib]` 单文件、`[[bin]]` 列表、`[exports]` 映射） |
| `[binaries]` 消歧 | 014b 预编译分发产物 ≠ `[[bin]]` 源码角色 | 2026-09-12 | 晨煦 | 两层概念（下载策略 vs 编译角色），014c 落地前钉死 |
| Internal pub 收紧 | 二阶段按包内 use 图可达性收紧，前置 = Bin 角色语料验证 | 2026-09-13 | 晨煦 | 宁漏报渐进收紧，语料驱动避免过早误报 |
| `[[test]]` 显式声明 | 不引入；Test 仅由 036 规则判定 | 2026-09-13 | 晨煦 | 036 未有需求前不加配置面 |
| workspace 根覆盖成员导出面 | 不允许 | 2026-09-13 | 晨煦 | 成员自包含是 014c 核心设计，根覆盖破坏封装 |
| Script 下 `main` 是否自动调用 | **不自动**——顶层语句即程序，`main()` 需显式 | 2026-09-17 | 晨煦 | 两规则共存会让显式 `main()` 双跑（#356）；Script 只能有一个执行入口 |
| 角色模型是否用于入口判定 | **是**（此前仅用于死代码警告） | 2026-09-17 | 晨煦 | 本 RFC 定义 Bin 的「`main` 为可达性根」，不接入口则该语义空转 |

## 附录C：术语表

| 术语 | 定义 |
| ---- | ---- |
| 角色（Role） | 源文件的编译目标身份：Script / Bin / Lib / Test / Internal |
| 导入面（Import Surface） | 跨包 `use` 时合法解析起点的文件集合，由 `[exports]`/`[lib]` 声明或推断 |
| 分发边界 | 029 术语：包的对外暴露范围；本 RFC 将其从隐式（全部 pub 文件）精确化为导入面 |
| Script 态 | 无 manifest 单文件直跑的旁路形态，一切行为与现状一致 |

## 参考文献

- RFC-029 模块语义（父 RFC：入口选择、可见性决策、子 RFC 规划）
- RFC-015 配置系统（`[lib]`/`[[bin]]`/`[exports]` 字段定义）
- RFC-014b 构建系统与二进制分发（`[binaries]`，名词消歧对象）
- RFC-014c 工作空间支持（导入面的首个消费方，审核中）
- RFC-036 测试框架（Test 角色规则来源）
- #321 M2 警告码独立发射通道（定案 B 与方案 A 的出处）

## 已知缺陷：已落地（2026-10-16，WBS 4.9 轮 1）

> 上一版（2026-10-09）将下列三项列为「引用池的三处设计缺陷」。三项已由 WBS 4.9 轮 1（提交 `3863a9ff0`）逐条落地；`collect_project_refs` 与 `orchestrator.rs` 已随 P4 4.2.9 拆分移入 `src/driver/{state,arms,helpers}.rs`，下表所有 file:line 均在被引用 revision 上重取。

### 引用池的三处设计缺陷 —— 修正状态

| # | 原缺陷 | 现状（2026-10-16） | 落地代码（rev `3863a9ff0`） |
|---|--------|-------------------|----------------------------|
| 1 | 扫描面是目录而非编译面 | **池的根 = 包目标**：`collect_package_refs`（`helpers.rs:481`）按 `roles::ExplicitSurfaces` 的 `[[bin]]`/`[lib]`/`[exports]`/`[run].main` 根调 `discover_with_used` 取编译集；**不扫目录找入口**。全部根都跑不通时退化为被检文件自身的编译集（`collect_compilation_refs` / `helpers.rs:454`） | `src/driver/helpers.rs:454-512`（退路路径）；`src/driver/helpers.rs:481-512`（manifest 声明面路径） |
| 2 | 粒度是项目级裸名，无模块维度 | **池类型 = `HashMap<String, HashSet<String>>`**：键 = 模块键、值 = 该模块的引用名集合；消费端在 `arms.rs:603-609` 把「除本单元外」的所有模块条目并集后送给 `DeadCodeAnalyzer`，模块自有名（顶层定义与局部绑定，`use` 导入名例外）不再豁免别的模块同名定义 | `src/driver/arms.rs:603-609`（消费端 `other_module_refs`）；装配唯一入口 `src/driver/helpers.rs:429-449`（`collect_ref_pool`） |
| 3 | Script 复用 Lib 的池（违反单文件语义） | **Script 不接池**：`arms.rs:611-623` 的 `match us.role` 只对 `Bin`/`Internal`（`set_exempt_pub`）与 `Lib`（`set_cross_file_refs`）装配，Script/Test 走 `_ => {}` 默认臂；后者只按文件内定义-使用图判定 | `src/driver/arms.rs:611-623` |

**后续改进 4 / 5 —— 修正状态（2026-10-16，WBS 4.9 轮 2）**

> 本节两项原列「待 P4 4.9 落地」。两项已由 WBS 4.9 轮 2 落地（提交 `e6cf1aeaa`，2026-10-17 推送）；下表 file:line 均在 revision `e9675c9dd` 上重取（2026-10-17 轮 3 收口补注——轮 2 落地后的修复轮与收口轮改动了同批锚点文件，行号以重取值为准；轮 1 教训：push 前的本地哈希会被 `pull --rebase` 重写）。

| # | 原改进 | 现状（2026-10-16） | 落地代码 |
|---|--------|-------------------|----------|
| 4 | 消费定义细化：区分「被 import」和「被 import 后真实使用」（仅后者算消费） | **消费判据 = 「被 import 后真实使用」**：一条消费记录 = 某模块用 `use` 把名字 n 从模块 M 带进作用域**且在使用点真的引用了 n**，只记到 M（`RefPool::consumed_from`）；只出现在 use 项里（无使用点）、别的模块对**同名**标识符的引用都不算消费。import 的目标模块按发现期的模块键集合判定（`resolve_import_module`，**不另起一套路径解析**——复用 4.6.1 已交付的发现期解析产物，见 `39546020`）；命名空间形态（`use lib` + `lib.foo()`）同样归因 | `src/driver/helpers.rs:442-447`（`RefPool`）；`src/driver/helpers.rs:449-506` + `:511-524`（`collect_ref_pool` + `resolve_import_module`）；`src/frontend/core/typecheck/passes/dead_code.rs:186-200`（`ModuleNames`）、`:384`（`collect_module_names`）、`:132-184`（`ImportFace` / `collect_import_face`） |
| 5 | 测试池分离：tests/ 的引用不豁免生产代码（src/） | **`FileRole::Test` 不贡献消费面**：Test 模块的引用单独成池（Test 自身不参与死代码判定，故其引用不外溢），生产代码的定义只被生产代码的消费救活。角色由 `roles::classify` 单点判定（测试发现规则 RFC-036） | `src/driver/helpers.rs:463-466`（`collect_ref_pool` 的 Test 短路）；`src/driver/helpers.rs:707-708`（`roles::classify` 单点判定）；`src/driver/arms.rs:633-635`（退化与声明面两条路径共用的 Test 跳过；角色喂池见 `:661-672`） |

**残留面（登记，不阻塞）**：① 字段/方法位引用（`obj.render()` 只有短名）在 AST 层无法归因到接收者类型所属模块——基名命中命名空间导入时按 import 归因、模块限定构造 `ns.T` 按**类型名面**归入方法归因（见下节 4.9.7），其余按名字维度保守计入 `RefPool::field_names`（`src/driver/helpers.rs` 的 `collect_ref_pool`）；② 方法可达性：按 D20 决议产物**逐候选 + 按定义单元**收窄（`resolved_method_marks`）——**同单元内**未被命中的兄弟重载如实报 W1005；**跨单元同名同序的未调用定义在可达形态下**同样如实报（WBS 4.9.7 / issue #464，已收口，机制与实测见下节；引用产出侧提交 `b8b3cc3f6`）。**残余边界（登记，不得宣称全域；全部按实测）**：(a) 单独 `check <定义文件>`（该文件无调用点、调用方不在单元集）⇒ 没有决议条目 ⇒ 回落池的**名字级豁免**（`RefPool::field_names` 全局平铺）⇒ **不报**；(b) 同一个裸类型名被多个模块带进作用域（源程序在名字层已歧义，入口另报 E5006）⇒ 候选面整名跳过 ⇒ 无决议条目 ⇒ 同名同序组不报；(c) 调用方在本单元对另一单元类型名做过**任意方法调用**时（即便不命中该另侧定义），该另侧定义按**可能归属**并入（`typed_owners` 规则②）⇒ 不报；双侧都只构造、无任何方法调用时 `typed_owners` 不生效，两侧如实报 W1005（t20 实测夹具：双侧只构造 ⇒ 2 条 W1005）；(d) 整体导入 `use M` 不让裸类型名进作用域、std 模块的候选面未接入——两形态都不产条目，回落名字级豁免。回落边界：某名字组在本单元无任何决议记录时（如调用点歧义 E1107、不经决议的调用形态）回落短名共判（宁漏报方向）——因此不宣称「未被调用的方法一律如实报」。

### 引用计数失真（WBS 4.9.6 / issue #450）—— 修正状态（2026-10-16）

> 原登记：重载方法**被调用且决议生效，仍被误报 `W1005 Unused method`**（引用计数对重载调用失真）。2026-10-16 用户裁决 A 将其登记为独立三级任务并入 WBS 4.9 轮 2；本项属 D 类（删掉误报即改变诊断对外面），已授权落地。

| 环节 | 原缺陷 | 现状 | 落地代码 |
|------|--------|------|----------|
| 可达性探针 | 扫描按**限定名**查引用（`Point.render`），而引用点收集到的是**短名**（`obj.render()` 只有 `render`）⇒ 被调用的方法永远不可达，无项目根路径下误报 W1005 | 方法按**短名探针**判定调用面，并以 **D20 决议产物**做逐候选收窄（`resolved_methods` + `resolved_groups`；未被决议命中的兄弟重载仍报 W1005）。作用域边界：**同单元内**逐候选成立；**跨单元（registry 解析）调用**在产出侧（`b8b3cc3f6`）与驱动侧归因（本批）落地后，**在可达形态下**同样逐候选 + 逐定义单元成立（见残留面②与下节 4.9.7） | `src/frontend/core/typecheck/passes/dead_code.rs:782-792`（探针）；`:815-877`（逐定义判定与豁免闸） |
| 定义表键控 | `all_defs` 按限定名键控 ⇒ **同名重载互相覆盖**（两个重载只剩一条，只报 1 条） | 定义表**逐定义一条**：方法追加、同名非方法绑定按名字去重保留最后一次（重载是各自独立的定义，顶层重赋值是同一个绑定） | `src/frontend/core/typecheck/passes/dead_code.rs:304-322`（`record_definition`）、`:28-29`（字段） |

**不在 P3.5 改的理由（与上一版一致）**：模块级解析需要 `use` 项的路径解析完成（`use a::foo` → foo 来自模块 a），属 P4 阶段契约与 Driver 统一的范畴。

### 跨单元方法可达性归因（WBS 4.9.7 / issue #464）—— 修正状态（2026-10-17）

> 原登记（残留面②）：跨单元（registry 解析）的方法调用不产生 D20 决议条目 ⇒ 该名字组回落池的**名字级**
> 豁免 ⇒ **跨单元同名同序的未调用方法不被报**。本条分两笔落地：**产出侧**（跨单元方法调用产生决议条目，
> 提交 `b8b3cc3f6`；其注入面在修复轮按名字归属逐对过滤，见下表）与**驱动侧归因**（按定义单元给出可达性标记并停用名字级豁免）。下表 file:line 取于
> **驱动侧归因工作树**——提交 `d5c00235a`；产出侧提交 `b8b3cc3f6`（见上）。本节全部 file:line 统一取于 revision `e9675c9dd`（2026-10-17 轮 3 收口时重取，覆盖 `d5c00235a` 落地后被修复轮 `d5dd1a37b` 平移的注入面区间）；提交号一律 push 后真值（push 前的本地哈希会被 `pull --rebase` 重写，见 09a）。

| 环节 | 原缺陷 | 现状 | 落地代码 |
|------|--------|------|----------|
| 决议条目产出 | 跨单元调用点的候选表三张全缺（`method_overloads` / `method_overload_ir_names` / `method_overload_param_names`）⇒ 决议段整段跳过 ⇒ 条目 **0 条** | 候选组「候选 / 混编 IR 名 / 声明期形参名」按同序打包随 `ModuleInfo.method_overloads` 跨单元传递，注册表按模块键升序确定性合并；Typecheck 臂按本单元 import face **逐对**取候选——只注入「键的提供模块 == 该**类型名前缀**在本单元 scope 里的提供模块」的候选组（名字 ← 模块配对，不跨对组合；自有键位不外并、同名冲突整名跳过）⇒ 同点 `cands=1`、条目 1 条（`Temp.to`）。**修复轮实测（F1）**：此前「名字集合 × 模块集合」的组合会把另一模块定义的同名类型候选并进同一键位 ⇒ `use a.{Temp}` + `use b.{Other}`（a 亦定义 Other、b 亦定义 Temp、方法同名同序）下 `check` 报 2×E1107、退出 1（修前成功的程序编译失败）；逐对过滤后 `check`/`run` 均退出 0，入库夹具 `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/` | `src/frontend/core/typecheck/environment.rs:145`（`merge_from`：候选组级同序追加）、`:336`（快照）、`:384`（注入）、`:399`（`append_overload_group`：三张平行表**唯一**落表点，落表前校验等长）；`src/frontend/module/registry.rs:150`（`merge_method_overloads`）；`src/frontend/module/mod.rs:155-157`（`ModuleInfo` 字段）；`src/driver/arms.rs:344-358`（注入面）、`:1521-1579`（`visible_method_overloads`，注入单点：名字 ← 提供模块逐对、同名冲突整名跳过、自有键位不外并）。**注入面修复轮（F1，`d5dd1a37b`）**：修复前的「名字集合 × 模块集合」组合把另一模块定义的同名类型候选并进同一键位 ⇒ `use a.{Temp}` + `use b.{Other}`（a 亦定义 Other、b 亦定义 Temp、方法同名同序）下 `check` 报 2×E1107、退出 1（修前成功的程序编译失败）；逐对过滤后 `check`/`run` 均退出 0——入库夹具 `tests/yaoxiang-multifile/ref-pool-cross-unit-name-owner/`、钉测试 `src/driver/tests/overload_injection.rs`（归位提交 `c2f887843`） |
| 驱动侧归因 | 决议条目按**名字**全局标记（`(基名, 声明序)` 一个集合）⇒ 另一单元的同名同序定义被同名兄弟的调用豁免（跨单元同名同序此前**不报**） | 标记按**定义单元**归因（`MethodMarks { by_unit, groups }`）：条目名 `{Type}.{method}#k` 的 `{Type}` 是**定义侧**裸名且不带单元 ⇒ 用记录方单元的**类型名面**算「该单元能拼出这个名字的模块集合」，条目归入集合内各单元的 `by_unit`；`groups` = 出现过条目的名字组（全局） | `src/driver/helpers.rs:569-573`（`MethodMarks`）、`:591-619`（`typed_owners`：花括号项 + 内联别名目标 + 模块限定访问 `ns.T`，复用 `collect_module_names` + `resolve_import_module` 单点）、`:621-654`（`resolved_method_marks`，经 `OverloadResolutions::iter` 纯读）、`src/driver/arms.rs:650-658`（逐单元取 `by_unit[unit_key]`）、`:649-660`（注入分析器） |
| 判定面 | 有决议记录的名字组仍走池的**名字级**豁免（短名全局平铺） | 有决议记录的名字组**不走名字级豁免**：命中本单元声明的活、未命中的如实报 W1005（`!marked_group` 为豁免闸的前置条件） | `src/frontend/core/typecheck/passes/dead_code.rs:44`（字段）、`:252-257`（`set_resolved_groups`）、`:262-267`（`method_group_has_marks`）、`:782-795`（可达性逐候选）、`:815-877`（逐定义判定 / 豁免闸 / W1005 发射） |

**对外可观察变化（A 类：旧行为非成功 ⇒ 顺路修，同提交登记）**：

- **跨单元重载决议此前不生效**：候选表三张全缺 ⇒ 退回跨单元单值绑定表（`method_bindings`，同名定义
  last-wins）⇒ **多候选同名方法的调用报 `error[E1002]`**（实测 `Expected type 'string', found type 'int64'`）；
  修复后按实参逐候选决议，同夹具实测 `Type check passed`。判据落在 integration 用例
  `test_multifile_cross_unit_method_overload_resolves_by_argument_type`（`tests/integration/multifile.rs`，
  同名同序 Int/String 两候选、两条调用各自命中）——把注入面关掉时该用例翻红于同一条 E1002。
- **跨单元同名同序的未调用方法在可达形态下开始如实报 W1005**：三文件夹具（`main.yx` 用
  `use a.{Temp}` 命名类型、`use b` 只把 b 带进编译集）实测——`a.Temp.to` 有调用点不报、`a.Temp.other`
  报、**`b.Temp.to` 报**；修复前同一夹具只报 `a.Temp.other`（b 的定义靠池的名字级豁免蒙混过关）。
  入库夹具 `tests/yaoxiang-multifile/ref-pool-cross-unit-method/`（CLI 可复跑）；驱动级钉
  `src/driver/tests/method_unused.rs` 的 `test_cross_unit_uncalled_same_named_method_is_reported`。

**残余边界（登记，不得宣称全域；逐条按实测）**：

1. **单独 `check <定义文件>`**（该文件无调用点、调用方不在单元集）⇒ 没有决议条目 ⇒ 回落名字级豁免
   ⇒ **不报**（宁漏报方向）。实测：同一三文件夹具 `check b.yx` 只报未使用类型 W1002，不报 `b.Temp.to`。
2. **名字层已歧义**：同一个裸类型名被多个模块带进作用域（`use a.{Temp}` + `use b.{Temp}`）⇒ 候选面整名
   跳过 ⇒ 无决议条目 ⇒ 同名同序组回落名字级豁免、不报；该入口另报 `E5006 Duplicate import`。实测对照
   （只把 b 按命名空间导入）同类兄弟候选如实报，证明判据不是空转。
3. **调用方在本单元对另一单元类型名做过任意方法调用**（即便不命中该另侧定义）⇒ 该另侧定义按
   **可能归属**并入（`typed_owners` 规则②）⇒ 不报；双侧都只构造、无任何方法调用时 `typed_owners`
   不生效，两侧如实报 W1005（实测夹具 `t20-b3-no-method-call`：双侧只构造 ⇒ 2 条 W1005）——宁漏报方向。
4. **内联别名 `use M.{T as X}` 与模块限定构造 `ns.T` 不注入候选面**（产出侧收窄规则②）：调用点键仍是
   定义侧 `T`（实测两形态的 `X(..).to()` / `ns.T(..).to()` 都落键 `T.to`）⇒ 同名类型的 `#k` 精确性以
   「记录方能拼出的模块集合」为界，**不区分「哪个单元的定义真被选中」**；也正因如此，这些形态下
   **不能**把另一单元的同名同序定义判死（否则真实被调用的方法被误报）。
5. **整体导入 `use M` 不让裸类型名进作用域**（实测写 `Temp(...)` 报 E1001）、**std 模块的候选面未接入**
   （其 `ModuleInfo` 由 std 注册路径构造）——两形态都不产条目，回落名字级豁免。

**驱动侧读取是纯读**：`resolved_method_marks` 经 `OverloadResolutions::iter`（不触达 `get`、不计入消费
台账）⇒ 不掩盖 W1090「消费必达」结算；驱动级钉
`test_resolved_method_marks_read_leaves_consumption_ledger_untouched`（读后条目仍在 `unconsumed()` 差集里）。

**收口实测补记（2026-10-17 轮 3；注入面修复轮 `d5dd1a37b` + 钉测试归位 `c2f887843` 之后）**：
注入面过滤为「名字 ← 提供模块」**逐对**取候选（只并入「键的提供模块 == 该类型名前缀在本单元
scope 里的提供模块」的候选组）——单点在 `visible_method_overloads`（`src/driver/arms.rs:1521-1579`，
生产调用点 `src/driver/arms.rs:354`）；下游落表只有一处（`environment.rs:399` `append_overload_group`：
三张平行表同序追加 + 等长 debug 断言；组间追加唯一走 `merge_from`，`environment.rs:145`）。
变异核验：还原跨对过滤时驱动级钉翻红于 2×E1107/2×E1053，主验收夹具三条断言逐条不变（证明修复未回退主验收）。
独立验证 / 复审以冻结 revision 对（修复前 `d5c00235a` / 修复后 `e9675c9dd`）双二进制对拍复核：主验收夹具
`tests/yaoxiang-multifile/ref-pool-cross-unit-method/` 的诊断集两侧一致；负对照（无同名类型 / 同名类型
但方法名不同）两侧全绿；同名裸类型名被多模块带进作用域（构造器 / 值维度）的既有歧义行为两侧一致；
反向漏报面由同名同序**未被调用**的定义如实报 W1005 覆盖（修复不再吞掉它们）。

**判据与漂移（本批实测，含收口轮）**：驱动级钉测试——归因轮 5 条（主验收 / 内联别名误报防线 / 模块限定名
误报防线 / 键作用域双向 / 纯读，`src/driver/tests/method_unused.rs`）+ 注入面轮 3 条（名字归属逐对过滤 /
方法名不同不误伤 / 三表等长配对，`src/driver/tests/overload_injection.rs`，归位提交 `c2f887843`），
另 4 条 #450 旧判据不回归；IR 快照 204 条零漂移；语料基线 346 → 347 行（驱动侧归因轮，新夹具
`ref-pool-cross-unit-method/`）→ 348 行（注入面修复轮，新夹具 `ref-pool-cross-unit-name-owner/`），
每笔只 +1 条 behavior 记录、既有行逐字节不变（基线 `tests/baselines/corpus-parity.jsonl` 在
`e9675c9dd` 共 348 行）。

### 跨单元同名类型的方法派发归因（WBS 4.9.8 / issue #465）—— 修正状态（2026-10-25）

> 现象：两个编译单元各自定义**同名类型** `Temp` 与同名方法 `Temp.tag`（实现可区分：a = `v + 1.0`、
> b = `v * 10.0`），入口用裸名引入 a（`use a.{Temp}`）、再用**内联别名**（`use b.{Temp as TempB}`）或
> **模块限定构造**（`b.Temp(..)`）引入 b。两侧**构造都正确**（字段值对），但 `u.tag()` 派发到 a 的实现；
> `run` 退出 1（断言失败）——静默错值（D59 C 类）。`check` 的警告面取决于**清单是否可读**：issue 给出的
> 清单形态（`[package]` 只有 `name`、缺 `version`）实为 **2 条 `W1002 unused type`**（`a.yx:1:1` /
> `b.yx:1:1`），补上 `version` 才是 0 warning；清单解析失败 / 缺必填字段时**静默退化**（残余边界 4），
> 与死代码判据本身无关。

| 环节 | 原缺陷 | 现状 | 落地代码 |
|------|--------|------|----------|
| 派发名来源 | 派发名 = **定义侧裸类型名** + 方法名（`Temp.tag`）。typecheck 的 `MonoType` 对导入类型只留定义侧裸名（`Temp`），两单元的同名类型在类型层不可区分 | 派发名先按**接收者写出的类型名**归因到定义单元：`Temp.tag` → `b.Temp.tag`（写出名 → 导入面 → 定义单元限定名）。**方法调用与算子接口（Index / Equal）同一判据** | `src/middle/core/ir_gen.rs:1265`（`method_dispatch_name`，**唯一判据**）、`:1232`（`imported_type_owner`：`use_aliases` + `Resolver::resolve_namespace` + 注册表 `ExportKind::Type` 单一来源；**权威限定名精确命中即返回、不再解析**）、`:1188`（`constructor_written_type_name`，纯语法提取）、`:1169`（`written_type_name_of`，注解写出名） |
| 写出名的记录 | 无此概念：`ir_gen` 只按 `MonoType` 的裸名派发 | 局部变量记在符号表条目（`SymbolEntry.written_type`，作用域自随符号表）；顶层绑定记在逐单元表（`global_written_types`，Script 形态 `t = Temp(1.0)` 是全局槽位）；形参注解同链记录 | `src/middle/core/ir_gen.rs:1115`（`note_local_written_type`）、`:1129`（`note_param_written_types`）、`:1159`（`binding_written_type`：先局部后顶层）、`:109`（`SymbolEntry.written_type` 字段）、`:3255`（顶层登记点） |
| 五处生产路径 | 三条方法命名路径（RFC-004 绑定 / D20 重载决议 / 裸名推导）各产各的名字；两个算子接口臂（Index → `Type.index`、Equal → `Type.equal`）另拼短类型名，完全绕过归因（修复轮 R1 实测） | 五条路径都经 `method_dispatch_name` 产出最终调用名（判据只此一份，D3） | `src/middle/core/ir_gen.rs:8318`（绑定臂）、`:8379`（决议臂）、`:8532`（裸名臂）、`:6444`（Index 臂）、`:7858`（**二元接口共享臂**：Equal + Add/Subtract/Multiply/Divide/Modulo） |

**机制判别（判别实验，先于修复）**：issue #465 列了两条候选机制——(a) `type_bindings` 以**短类型名**键控，
同名单元互相覆盖；(b) D20 决议覆盖 `binding.function`。在本形态上只有 (b) 成立：入口单元的 `type_bindings`
为空（类型体里没有内联外部绑定），两处方法调用都命中 `overload_resolutions`，决议名都是**定义侧裸名**
`Temp.tag`；随后 `qualify_names` 按别名前缀把裸名前缀 `Temp` 解析到**裸名提供者** a（`use a.{Temp}`），
于是两个调用点同落 `a.Temp.tag`——与 issue 的 8 组对照一致（赢家 = 裸名属主；与 import 顺序、与是否有 a 侧
调用点无关；两单元类型名不同或同文件两类型时正确）。机制 (a) 在本形态不活跃（`seed_cross_file_types` 只登记
类型体**内联**绑定的短名，与方法语句形态无关）。

**对外可观察变化（C 类：旧行为非成功 ⇒ 顺路修）**：修复前静默错值的程序现在走各自定义单元的实现；
无同名类型的跨单元方法调用与两组阴性对照（类型名不同 / 同文件两类型）行为不变。判据全部为**值级**：
两个单元的方法体改成可区分（`+1.0` / `*10.0`）后断言 `2.0` / `20.0`——修复前同一夹具得 `2.0` / `3.0`
（`CROSS_UNIT_A/B` 此前两单元实现完全相同，对该缺陷是盲的）。

**判据与漂移（本批实测，含修复轮）**：驱动级钉测试 **10 条**（内联别名 / 模块限定构造 / 算子两臂 / 本地类型
对照 / 8 组对照中的 5 组 / 阴性对照，`src/driver/tests/method_dispatch.rs`，结构层读**合并 IR 的调用名**、
行为层读解释器全局槽位值）+ 集成值级断言 **5 条**（`tests/integration/multifile.rs`，含 R1 算子形态与 R2
`p.q` + 别名形态）；
变异核验：把 `method_dispatch_name` 的归因关掉（原样返回定义侧名字）⇒ 新增的两条同名类型用例翻红
（调用名回到 `a.Temp.tag` ×2），阴性对照与既有 159 条驱动级用例不回归；补丁机械可逆（`git apply -R` 后
与被变异文件逐字节一致）。语料基线 `tests/baselines/corpus-parity.jsonl` **356 行零漂移**、IR 快照 216 条零漂移
（本改动只改调用名归属，不改 IR 形状，也不新增诊断码 / warning）。

**修复轮（R1–R4，2026-10-25；来源 = 异家族复审对冻结基线的 needs_revision）**：

- **R1 算子接口两臂**：Index / Equal 派发此前用 typecheck 过来的**短类型名**拼调用名（`Temp.index` /
  `Temp.equal`），不经归因 ⇒ 两单元同名类型的算子实现互相落错单元。实测（冻结基线）：`u[0]` 得 `3.0`、
  `u == t` 得 `true`（应 `20.0` / `false`），`check` 退出 0。现在两臂同经 `method_dispatch_name`
  （`:6444` / `:7858`），修复后 `20.0` / `false`；**本地类型两形态逐字节不变**（钉
  `test_local_type_operator_arms_keep_own_dispatch` + 语料 `operator_index.yx` / `struct_equality.yx` 全绿）。
  复核轮实测：`:7858` 是**全部「接口驱动二元运算」的共享臂**（Equal + Add/Subtract/Multiply/Divide/
  Modulo）——两单元同名类型上的 `u + u` 修复前 `3.0`、修复后 `20.0`，算术五族随同一处判据同修；
  本节原表把它记作「Equal 臂」低估了覆盖面，已按实修订。
- **R2 归属判定单点 + 不二次解析**：上一版 `constructor_written_type_name` 产出**权威限定名**
  （`p.q.Temp`），`imported_type_owner` 又把它过一遍 `resolve_namespace`，首段被命名空间别名表改写
  （`use other as p` ⇒ `other.q.Temp`）⇒ 类型导出判定失败 ⇒ 回落裸名（实测 `y = 3.0`）。现在构造器只做
  **纯语法提取**（写出名原样），`imported_type_owner` 是唯一判定点，且**权威限定名精确命中即返回、
  不再解析**（`:1232`）。钉：`test_multifile_qualified_module_key_not_rewritten_by_namespace_alias`
  （模块键 `p.q` + 未使用的 `use other as p`，先红后绿）。
- **R3 文档更正**：本节原先「`check` 0 error / 0 warning」的表述与实测不符——issue 清单缺 `version` 时
  `check` 报 2 条 `W1002`。已按实情改写并登记为残余边界 4；清单解析失败 / 缺字段的**静默退化**本身
  是另一处缺陷（不在本批修）。
- **R4 8 组对照补钉**：驱动级新增 5 组值级用例——`use` 顺序对调、两单元方法体对调（值跟单元走：
  `10.0` / `3.0`）、**裸名属主改为字典序靠后的 `z.yx`**（证明赢家来自 `use` 面而非排序）、只调别名侧、
  同文件两类型阴性对照；连同原有 3 条，8 组对照现在全部入库可复跑。
- **变异核验**（机械可逆，`git apply -R` 后逐字节一致）：M1 关闭 `method_dispatch_name` 归因 ⇒ 7 条跨单元
  用例翻红（含算子两臂与 4 组对照）、3 条本地/阴性对照仍绿；M3 恢复「旧构造器 + 去精确命中」⇒ R2 用例翻红。
**残余边界（登记，宁缺不猜；逐条为实测或代码面可读出的事实）**：

1. **写出名不可得时回落裸名**（与修复前一致）：类型经函数返回值、字段、容器元素流入的接收者没有写出名可依
   ——例如 `u = make_b()`（返回 b 单元类型的跨单元函数）之后的 `u.tag()` 仍按裸名前缀解析。
2. **内联别名 × 同名重载**：typecheck 的候选注入面仍按 4.9.7 收窄规则（规则②：内联别名 / 模块限定形态不注入），
   派发只替换类型前缀、保留决议的 `#k` 混编序——`#k` 的跨单元精确性不变（同 4.9.7 残留面④）。
3. **跨单元值绑定转手**：`use M.{v}` 引入的实例（类型由定义单元记录）没有本地写出名，同样回落裸名。
4. **清单解析失败 / 缺必填字段时静默退化**（实测）：`[package]` 缺 `version`（issue #465 给出的清单形态）时，
   `check` 报 2 条 `W1002 unused type`（`a.yx:1:1` / `b.yx:1:1`）——两个类型都被入口引用，属**误报**；
   补上 `version` 即 0 条，清单是**非法 TOML** 时同样静默退化。根因在清单解析失败的静默退化路径，
   不在死代码判据本身（登记；另开 WBS 任务 / issue 由队长办，本批不修）。
