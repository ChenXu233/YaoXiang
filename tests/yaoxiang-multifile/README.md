# 多文件语料层（tests/yaoxiang-multifile/）

RFC-039 决议 D40（无条件必做）/ D48（位置裁决）确立的**项目级契约语料**：
P4「统一 Driver」是全计划最大风险点，本层是 P4 唯一可执行的行为判据来源
（[07-equivalence-oracle.md](../../docs/src/dev/architecture/07-equivalence-oracle.md)
§三层判据 / [09-execution-wbs.md](../../docs/src/dev/architecture/09-execution-wbs.md) 2.3.2）。

## 与 tests/integration/multifile.rs 的分工（D48）

| | multifile.rs（语义单测） | 本目录（契约语料） |
| --- | --- | --- |
| 形态 | tempdir 临时项目，库 API 直调 | 入库的稳定项目夹具，CLI 端到端 |
| 回答的问题 | 「这个语义对不对」（开发期即时反馈） | 「重构前后行为变没变」（差分基线） |
| 运行器 | `cargo test --test integration` | `cargo test --test yx_multifile_runner` |

扩展 multifile.rs 会把语义测试与契约语料混在一起（D48 裁决原文），故多文件
**语料**一律放本层。

## 夹具规范

每个一级子目录是一个**项目夹具**：

```text
<fixture-name>/
├── yaoxiang.toml   # [package] 清单（项目身份标记，Bin 角色判定依据）
├── main.yx         # 入口（约定名；头部指令所在文件）
└── *.yx / */*.yx   # 其余模块文件
```

- **入口约定**：`main.yx`。运行器对每个夹具执行 `yaoxiang-rs run <dir>/main.yx`
  或 `check <dir>/main.yx`（按指令类别分流）。
- **判定契约**：与 `tests/yaoxiang/` 单文件语料**同一套头部指令**
  （`util::test_markers::TestFileSpec` 单点解析，禁令三合规）：
  - 无 `expect` 指令 = 行为测试：`run` 退出码 0 = PASS（值用 `assert.assert` 验证，
    规则同 .yx 测试规范——验证值，不验证输出）；
  - `// expect: compile-error EXXXX`：`check` 退出码非 0 且全部预期码出现；
  - `// expect: runtime-error EXXXX`：`check` 通过 + `run` 失败且预期码出现；
  - `// skip: <原因>` 跳过（须附 issue）。
- 文件头规范（`// 覆盖:` / `// 验证:` / `// 状态:`）与单文件语料相同，
  写在入口 main.yx 上；模块文件（lib.yx 等）只留简短说明注释。

## 现有夹具

| 夹具 | 覆盖 | 类别 |
| --- | --- | --- |
| use-type-and-function | 跨文件 use 导入类型 + 函数调用（RFC-029） | 行为 |
| use-constant | 跨文件常量导入与求值 | 行为 |
| cross-file-method | 跨文件方法调用（type_bindings 预注册） | 行为 |
| subdirectory-module | 子目录模块解析（`use util.math.{double}`） | 行为 |
| type-error-in-lib | 被引用模块内的错误沿 use 图传导（E1001） | compile-error |
| proof-obligation-honored | 多文件路径证明义务被执行（RFC-027 §4.2，P3 漏洞修复） | compile-error |
| for-loop-no-use-std-list | 无显式 `use std.list` 的 for 循环（#117 硬切换补齐，WBS 4.10.2） | 行为 |
| parse-error-in-collect-all | 模块语法错误降级为逐文件收集（E0012 + E5001，WBS 4.10.1） | compile-error |
| ref-pool-compile-set | 引用池来源 = 编译集：孤立文件（无人 use）的引用不豁免生产定义（RFC-029f 改进 1 / WBS 4.9.1） | 行为 |
| ref-pool-module-identity | 引用池身份 = (模块, 名字)：别的模块的同名顶层定义/局部变量不豁免本定义（RFC-029f 改进 2 / WBS 4.9.2） | 行为 |
| ref-pool-import-use | 消费判据 = 被 import 后真实使用：别的模块对同名标识符的调用不救活本定义，只看 use 项不算消费（RFC-029f 改进 4 / WBS 4.9.4） | 行为 |
| ref-pool-test-pool | 测试池分离：tests/ 的引用不豁免生产代码（src/）的定义（RFC-029f 改进 5 / WBS 4.9.5） | 行为 |
| ref-pool-cross-unit-method | 跨单元方法调用的定义单元归因：未被调用的同名同序方法如实报 W1005（WBS 4.9.7 / #464） | 行为 |
| ref-pool-cross-unit-name-owner | 跨单元方法候选注入面的名字归属过滤：每个裸类型名只由自己的提供模块供候选（WBS 4.9.7 / #464） | 行为 |
| init-order-entry-reads-dep-value | 初始化按 `use` 依赖拓扑序：入口顶层绑定读**被依赖单元**顶层值（修复前读到 void → 运行期 E6007，#467 / WBS 4.10.4） | 行为 |
| init-order-dep-fn-reads-own-value | 同上序：入口顶层绑定调用依赖单元的函数，该函数读**本单元**顶层值（修复前运行期 E6007，#467 / WBS 4.10.4） | 行为 |
| import-cycle-reads-before-init | 模块导入环：环内初始化顺序不可判定，编译期发 W1007（消息含环路径 + 被违反的边）；check/run 退出码均不变（#491 / WBS 4.10.5） | runtime-error |

## 计划扩展（登记用，落地时勾掉）

- [x] ~~P3（proof_calls 消费端修复）落地后：补 `proof-obligation-honored` 夹具~~
      （2026-10-06 已补：`SumUpTo(3, 7)` 触发源，`expect: compile-error E4018`）——
      漏洞判据 `test_multifile_proof_obligation_not_dropped`
      （tests/integration/proof_obligations.rs）的语料层对应物。
- [x] ~~P4 4.9.1/4.9.2 落地后：补引用池来源与身份的夹具~~（2026-10-16 已补：
      `ref-pool-compile-set` = 编译集外引用不进池、`ref-pool-module-identity` = 同名
      标识符按模块独立计数；两条夹具的 `check <入口>` 各报 1 条 W1001，是**判据
      本身**要钉的新行为——池的来源与身份见 RFC-029f 改进 1+2）
- [x] ~~P4 4.9.4/4.9.5 落地后：补消费判据与测试池分离的夹具~~（2026-10-16 已补：
      `ref-pool-import-use` = 名字只记到 import 解析出的模块、只 import 不使用不算消费；
      `ref-pool-test-pool` = tests/ 引用不外溢——两条夹具的 `check` 面各新增 1 条
      预期 W1001，见 RFC-029f 改进 4+5）
- [ ] P4 前按 07「C4 行为差分必覆盖清单」扩充：闭包捕获跨文件、spawn/迭代器、
      和类型 pattern、`?` 传播、方法重载、精化 Drop 序列等多文件形态。
