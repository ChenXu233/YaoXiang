# Changelog

## :bookmark: V0.7.14: 测试体系闭环与错误传播语义完善

> 发布日期: 2026-09-08

### 📦 版本信息

| 项目     | 值                  |
| -------- | ------------------- |
| 发布日期 | 2026-09-08          |
| 版本变更 | `0.7.13` → `0.7.14` |
| 提交数   | 103 个 commit       |

### 📋 本次更新概要

**⚠️ 破坏性变更**：运行时错误默认携带栈帧与源码上下文（#327），`--debug-info` 旗标移除——诊断输出格式变化，依赖旧格式的下游工具需要适配。

本次发版完成 RFC-036 测试体系全链闭环：头部指令文法取代中央清单、值语义断言族与套件收集、`--parallel` 并行执行、CI 接入 yaoxiang test 双套件。错误码体系四棒收口（#320-#326）：权威注册表单源化、span 漏传拒绝构造、翻译自动化闭环（zh 缺翻译拒绝编译 + 回退链）。语言层 `?` 操作符真实语义（#301）与 unwrap on Err 透传、Python 风格 break/continue（#314）、接口机制静态+动态分发端到端（#307）落地；借用检查与循环体类型检查系列修复（#290/#313/#315）。

### ✨ 新功能

#### 运行时错误默认携带源码上下文（#327，破坏性变更）

运行时错误默认携带栈帧与源码上下文，报错即含出错位置与源码行，不再需要 `--debug-info` 旗标（已删除）。配合 unwrap on Err 透传原始错误码与消息，`Err` 值穿透 unwrap 不再丢失诊断信息。

- 运行时错误结构默认携带栈帧 + 源码上下文
- `--debug-info` 旗标移除
- `unwrap` on Err 透传原始错误码与消息

#### yaoxiang test 测试体系闭环（RFC-036，#95/#319）

自建测试基础设施 `yaoxiang test` 全链交付：负向标记从英文指令文法（`expect:`/`skip:`/`mode:` 头部指令）声明，取代行内 `[test:error]` 中央清单（六家编译器取证均为 fixture 注释声明模式，Rust/Go 双向穷尽判定）。

- 头部指令文法定案与按类别分流判定落地
- `std.test` 值语义断言族（R1，RFC-036 §3）
- `test.suite` 套件收集（R3，RFC-036 §7）
- Phase 2 CLI 选项；Phase 3 `--parallel` 并行执行与 `[tool.test].exclude`
- `assert_approx_eq` Float 近似断言
- 07-std 语料迁入库测试层 `src/std/tests/`
- CI 接入 yaoxiang test 双套件与 JSON 汇总

#### 错误码体系单源化与翻译自动化（#320-#326）

错误码从分散定义收敛为权威注册表单源：`define_codes!` 宏合一，RFC 码表从注册表生成。构造期强制替代事后校验——漏 span 直接拒绝构造，zh 缺翻译直接拒绝编译。

- 错误码权威注册表（#320 M1）与 `define_codes!` 宏单源化（#326）
- 警告独立发射通道与非阻断编译（#321 M2）
- 消息单轨首批收敛与诊断审计（#322 M3）
- 运行时 `Error` 值携带规范化错误码（#323 M4）
- span 自动注入与强制（#324）
- 翻译自动化闭环：请求语言→en→zh 回退链、en 进 bot（#325）

#### `?` 操作符与错误传播（#301/#316）

`?` 操作符获得真实语义：`Result` 解包、`Err` 沿调用栈传播。`Range` 动态 `step=0` 运行时 Result 化，`std.range` API 挂载 `?` 错误传播。

#### 接口机制端到端（RFC-011a，#307）

接口机制从静态分发到动态分发端到端交付：`[n]` 绑定全链路修复（阶段 0）、静态分发端到端（阶段 1-2）、存在类型强制点收集与变体包装、动态分发端到端（阶段 3）。接口接收者拼写定案：`&Self` 显式借用，按值 = Move（RFC-011a 勘误）。

#### 语言特性定案

- `break`/`continue` 定案 Python 风格，移除标签死链路（#314）
- `Range` 类型正式化：正式身份与迭代器协议（#302）

### 🐛 Bug 修复

#### 借用检查（#290 / RFC-009a）

三连修复借用检查误报与漏报：借用活性补 `created_at` 端点与语句级 CFG（F1，修回溯误报）；可变借用点不再自我播种消费者集（F2，修显式 `&mut` 误报）；字段写创建 `WriteToken`（F3，修派生读-写冲突静默放行）。RFC-009a 借用证明管道三缺口修复，路径条件真查询生效。

#### 方法调用与循环体类型检查（#313/#315/#317）

- 方法调用接收者签名解析：`&mut self` 接收者产生借用令牌（#315）
- 方法调用 arity 与实参类型编译期检查
- 循环体类型检查洞：`infer_stmt` 穷尽化接管（#313）
- 循环控制流 IR 缺口与赋值类型统一

### ♻️ 重构优化

#### solver 层 HM 多态退役（#251 P1）

删除 solver 层 Hindley-Milner 多态遗留，接口连贯性收口。类型系统单一实现路径，消除双轨漂移。

### 🔧 其他变更

- 依赖更新（dependabot）：npm-dependencies group 两轮（7 + 5 updates）
- CI：移除 pre-commit cargo-audit 钩子；markdownlint 忽略 `docs/superpowers` 本地工具产物
- 新增 RFC-029a 模块缓存与增量重编译草案（#293）；spec 清除全部 issue 行内引用
- 修复俄译 bot 产物行首井号循环破损（七次修复，根治待 bot 治理）

### 📎 提交记录

```
cd5e9401 :pencil: docs: auto-translate documentation
25262e01 :sparkles: feat(std): unwrap on Err 透传原始错误码与消息
1dd97fd3 :boom: feat(runtime): 运行时错误默认携带栈帧与源码上下文
c345624a :memo: docs(design): RFC-029a 按书写规范重构详细设计
85afa873 :pencil2: docs(docs): 修复俄译 RFC-036 #247 行首井号（bot 产物 lint 解锁）
3f8cc92c :pencil: docs: auto-translate documentation
e20abe08 :memo: docs(design): RFC-029a 模块缓存与增量重编译草案（#293）
b3202283 :pencil: docs: auto-translate documentation
c6a1c0a0 :memo: docs(design): RFC-036 回写 CI 双套件 JSON 汇总消费
f244a385 :rocket: ci(test): CI 接入 yaoxiang test 双套件与 JSON 汇总
d8b455cd :pencil: docs: auto-translate documentation
a7efafb6 :sparkles: feat(test): --parallel 并行执行与 [tool.test].exclude（Phase 3）
1b8e1321 :white_check_mark: test(std): assert_approx_eq Float 近似断言
22ff83e3 :pencil: docs: auto-translate documentation
9fd420f0 :white_check_mark: test(std): 07-std 语料迁入库测试层 src/std/tests/
8e85752b :pencil: docs: auto-translate documentation
482287d8 :memo: docs(design): 指令文法与分流判定落 RFC 与测试规范
a4ba304f :sparkles: feat(test): 头部指令文法定案与按类别分流判定落地
66599e65 :pencil2: docs(design): 修复 RFC-036 俄译列表符 MD004（bot 产物，zh 源无此问题）
a69f0137 :pencil: docs: auto-translate documentation
824f6887 :memo: docs(design): 测试体系分层与负向标记分流判定落文档
0dc3c77e :memo: docs(design): RFC-036 §7 套件收集落地（#319）
191a86a5 :sparkles: feat(std): test.suite 套件收集（R3，RFC-036 §7）
503345bd :memo: docs(design): RFC-036 §3/§8.1 值语义族落地与 M4 现实回写
4e656a7e :sparkles: feat(std): std.test 值语义断言族（R1，RFC-036 §3）
7f46817e :pencil: docs: auto-translate documentation
2dc79074 :globe_with_meridians: i18n: auto-translate locale files
941be4e4 :recycle: refactor(util): 注册表单源化——define_codes! 宏合一与 RFC 码表生成化（#326）
c7bda7e4 :arrow_up: chore(deps): bump the npm-dependencies group across 2 directories with 7 updates (#328)
b3c5fa78 :sparkles: feat(util): span 自动注入与强制——漏 span 拒绝构造（#324）
5ecf3a94 :white_check_mark: test(util): bot 产物的测试锚定从精确文本改为结构断言（#325 收尾）
53bd33c1 :globe_with_meridians: i18n: auto-translate locale files
36ebd098 :sparkles: feat(util): 翻译自动化闭环——zh 门槛、回退链、en 进 bot（#325）
e7afba8f :pencil: docs: auto-translate documentation
51c6b9a2 :memo: docs(design): RFC-036 §5/§8.2 记录双 runner 收口与标记约定
09d08621 :sparkles: feat(util): 双 runner 判定收口——共享标记解析（#319，RFC-036 §8.2）
3c961a8c :bug: fix(typecheck): let else 改 ? 运算符修复 clippy -D warnings
d56a2a39 :pencil: docs: auto-translate documentation
4da86ef9 :sparkles: feat(std): 运行时 Error 值携带规范化错误码（#323 M4）
58d9948f :pencil: docs: auto-translate documentation
c10b232e :recycle: refactor(util): 消息单轨首批收敛与诊断审计（#322 M3）
f6cfb47d :pencil: docs: auto-translate documentation
1752887c :sparkles: feat(util): 警告独立发射通道与非阻断编译（#321 M2）
9b6d92e8 :pencil: docs: auto-translate documentation
92db9d19 :hammer: chore(util): 错误码权威注册表机制落地（#320 M1）
a1bcf14f :sparkles: feat(util): [test:error] 预期错误码实际比对（#251，RFC-036 §8.2）
2f1b5881 :pencil: docs: auto-translate documentation
f0a377ed :memo: docs(design): 新增 RFC-013 运行时错误值章节与 RFC-039 骨架
336fa3ad :memo: docs(design): spec 清除全部 issue 行内引用
51404845 :memo: docs(design): type-system 连贯性段去除 #46 行内引用
f33d4f37 :memo: docs(design): RFC-036 §1 对齐 Phase 2 输出契约（#319）
785a512f :sparkles: feat(util): yaoxiang test Phase 2 CLI 选项（#95，RFC-036）
f929398c :pencil2: docs(docs): 第六次修复俄语文档行首井号（bot 循环破损）
ad5283d9 :pencil: docs: auto-translate documentation
225974f5 :memo: docs(design): RFC-036 测试模型定案融入正文（取代文末修订节）
c0c758f0 :pencil: docs: auto-translate documentation
682e6e0a :pencil2: docs(docs): 第五次修复俄语文档 #46 行首井号（bot 循环破损，根治待 bot 治理）
d9bbfd7e :memo: docs(design): RFC-036 修订节——测试模型定案（负向三层/值化多测试/Error 码）
e3044b4e :pencil: docs: auto-translate documentation
38d9512b :pencil2: docs(docs): 第四次修复俄语文档 #46 行首井号（bot 重翻循环破损）
958ad086 :pencil: docs: auto-translate documentation
b6515b70 :white_check_mark: test(typecheck): 补 #309 时代 err 文件缺失的 test:error 标记
1ca44238 :pencil2: docs(docs): 再次修复俄语文档 #46 行首井号误构成
70d91710 :pencil: docs: auto-translate documentation
2ccde221 :pencil2: docs(docs): RFC-036 frontmatter 关联实现追踪 issue #319
48f0f1cf :pencil: docs: auto-translate documentation
c2f2e567 :white_check_mark: test(typecheck): method_call_args 头部补规范锚点
871a9920 :pencil2: docs(docs): 修复俄语文档 issue 引用误构成为标题
c6e2655d :white_check_mark: test(typecheck): #317 方法调用检查 E2E 复现
e1d2dbb1 :bug: fix(typecheck): 方法调用 arity 与实参类型编译期检查
8567729b :pencil: docs: auto-translate documentation
43099119 :recycle: refactor(types): 删除 solver 层 HM 多态遗留 + 接口连贯性收口（#251 P1）
e1b3da5e :white_check_mark: test(std): 合规修正——range_result_step0 移除哨兵 print（TEST_STANDARDS 2.3）
c0e7cfc7 :pencil: docs: auto-translate documentation
bb75f59b :sparkles: feat(std): Range 动态 step=0 Result 化——std.range API 挂载 ? 错误传播（#316）
fed00d03 :pencil: docs: auto-translate documentation
0cc6d17a :bug: fix(typecheck): 接口接收者拼写定案——&Self 显式借用，按值 = Move（RFC-011a 勘误）
28a23ed1 :bug: fix(typecheck): #315 方法调用接收者签名解析——&mut self 接收者产生借用令牌
f2ddfaae :sparkles: feat(middle): ? 操作符真实语义——Result 解包与 Err 沿调用栈传播（#301）
079d72ac :bug: fix(typecheck): 字段写创建 WriteToken——修派生读-写冲突静默放行（#290 F3）
4e62f10f :bug: fix(typecheck): 可变借用点不再自我播种消费者集——修显式 &mut 误报（#290 F2）
7495f6c4 :bug: fix(typecheck): 借用活性补 created_at 端点与语句级 CFG——修回溯误报（#290 F1）
fe9f1999 :white_check_mark: test(parser): 合规修正——断言消息与规范锚点补全（#313/#314）
386d307d :bug: fix(typecheck): 循环体类型检查洞——infer_stmt 穷尽化接管（#313）
fecd891a :sparkles: feat(parser): break/continue 定案 Python 风格——移除标签死链路（#314）
af628e48 :white_check_mark: test(typecheck): 测试合规修正——use 归位与状态行三态化
f3e7c429 :bug: fix(typecheck): 修复 RFC-009a 借用证明管道三缺口——路径条件真查询生效
f71bf03d :bug: fix(typecheck): 修复循环控制流 IR 缺口与赋值类型统一
ce2d46e6 :wrench: chore(meta): 忽略本地开发目录 .zcode 和 .dsh
b7e8e48f :globe_with_meridians: i18n: auto-translate locale files
01d12eb6 :pencil: docs: auto-translate documentation
4da485a9 :sparkles: feat(middle): RFC-011a §6 变体包装与动态分发端到端（#307 阶段3 下）
21882f51 :sparkles: feat(typecheck): RFC-011a §6 存在类型强制点收集（#307 阶段3 上）
b6481ebe :sparkles: feat(typecheck): RFC-011a 接口机制静态分发端到端（#307 阶段1-2）
aba020be :bug: fix(typecheck): RFC-004 [n] 绑定全链路修复（#307 阶段0）
6a2220c1 :wrench: chore(ci): markdownlint 忽略 docs/superpowers 本地工具产物
ca908d25 :fire: docs(docs): 删除 ponytail 过度工程审计报告
3d78e892 :construction_worker: ci(ci): 移除 pre-commit cargo-audit 钩子
a47beaef :pencil: docs: auto-translate documentation
81ccf826 :arrow_up: chore(deps): bump the npm-dependencies group across 3 directories with 5 updates
32de7c6b :memo: docs(design): #302 stdlib.md §6.2 与 reference index 同步
c4bbc136 :white_check_mark: test(std): #302 Range 协议、身份与动态 step=0 三测
fe5eba97 :sparkles: feat(types): #302 Range 类型正式化——正式身份与迭代器协议
```
