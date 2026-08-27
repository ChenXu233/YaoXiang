# Changelog

## :bookmark: V0.7.13: 类型系统收紧与运行时硬错误化

> 发布日期: 2026-08-27

### 📦 版本信息

| 项目     | 值                  |
| -------- | ------------------- |
| 发布日期 | 2026-08-27          |
| 版本变更 | `0.7.12` → `0.7.13` |
| 提交数   | 99 个 commit        |

### 📋 本次更新概要

本次发版围绕**类型系统收紧**与**运行时硬错误化**两条主线推进。容器类型完成去特殊化重构（#299），List/Dict/Array 等不再走 MonoType 原生变体，统一走 `Generic` 路径；运行时按 RFC-013 将索引越界、整数溢出、除零、缺键等从静默归零改为显式报错。补完作用域三链模型（#295）与柯里化值参数固化（#294/#296/#297）三件套，所有权 CFG 数据流（#264）落地 NLL 语义。同时完成 #269 测试合规整改与样板收编的全量清理。

### ✨ 新功能

#### in 谓词与 Range 一等值（#300）

`elem in container` 返回 `Bool`（一等霍尔谓词），支持 List/Array/Dict(键)/Tuple/String/Range；`x in 1..10` 等价于 `1 <= x && x < 10`。`Range` 升格为一等值，可绑定到变量（`r = 1..10`）、参与比较、跨函数返回，复用 Tuple 三槽载体。`a..b..c` 双点语法表达步进；`step=0` 字面量编译期 E1002、运行时 E6001（升格 Result 待 #301）。

- AST `Expr::In` 节点；`KwIn` 注册为中缀运算符；`listcomp` 迭代变量改裸标识符解析
- precedence 调整 `BP_RANGE` 1→7：区间紧绑定，`x in 1..10` 中 range 整体作为容器
- IR 将 Range 脱糖为比较链（Ge + JmpIfNot 短路 + Lt），其余容器发送 `CONTAINS`
- Bytecode `NEW_ARRAY` 全链路打通（#299 §2.1）

#### 容器类型去特殊化（#299）

`MonoType` 容器原生变体（String/Bytes/Tuple/List/Dict/Set/Option/Result/Range/Arc/Weak 共 11 个）全部删除，统一走 `MonoType::Generic { name, args }`。核心类型系统零容器特殊化——最小化的是类型系统的特殊化，不是运行时能力。

- `signature.rs` / `mono.rs::from()` 不再特判 lower 容器名
- 全链路 match arm 改为 `Generic { name, .. } if name ==` 或 `is_x()` helper
- 构造点统一用 `make_list/make_dict/make_option/make_result/make_tuple/make_string`
- 字面量上下文决定落点：`[...]` 落 Array(T,N) 由上下文类型注解触发，`AllocFixedArray` IR + `NEW_ARRAY` 字节码（#299 §2.2）
- 运行时新增 `E6008 KeyNotFound`：Dict 缺键与索引越界语义不同类，独立成码保留诊断信息

#### RFC-011a 接口实现与动态分发（accepted）

接口从隐式 `Self` 魔法关键字改为显式类型参数 `Animal: (Self: Type) -> Type`；接口实例化从 `Animal,` 改为 `Animal(Dog)`，即类型构造器实例化。异构容器 `List(Animal)` 表达存在类型 `∃S. Animal(S)`。与 RFC-011 泛型系统统一，消除 4 个原阻塞点（Self 显式化 / 泛型接口实例化 / 跨编译单元 LTO / 字段方法命名空间）。

#### 编译期值参数收敛（#296/#297）

按 SPEC §4.3 重写泛型构造调用分派：实参逐位匹配声明参数，Type 位收类型实参，编译期值参数位收字面量；类型构造器落空候选报 E1094；泛型构造器实参类型不匹配改报编译错误 E1002（#286）。泛型构造器参数个数按字段赋值语义（#287）：缺参/超参 E1010，类型参数从字段值推断。

- 落空候选作为一等产物，由调用语境决定分流策略
- 字段默认值表达式未绑定变量 → E1001（不再落 IR 端 E3006）
- 函数体尾表达式作为返回值（curry/方法/普通函数三处位点统一）

#### Heap Arc 化与跨线程捕获（#278）

句柄从 `Rc<RefCell<HeapValue>>` 改为 `Arc<Mutex<HeapValue>>`：拷贝 O(1) 且跨线程有效，写回共享可见。`Heap` 退化为分配注册表，内存回收交给引用计数。Standard 模式 Struct/List/Dict 跨线程捕获测试新增。

### 🐛 Bug 修复

#### 运行时硬错误化（#279-282）

按 RFC-013 将边界失败从静默归零/兜底改为显式报错。

- List/Tuple/Array 越界读写（LoadElement/StoreElement）→ E6003，携带 max/index 字段（#279）
- `l[-1]` 负索引归并 E6003（与 `l[5]` 同类契约失败）（#299 §4）
- 整数运算溢出（Add/Sub/Mul/Div/Rem/Shl/Shr）全 `checked_*`，溢出 → E6007（#281）；release/debug 行为统一
- 除零错误携带触发表达式，关闭 `<unknown>` 回退（#282）
- Dict 缺键 + `std.dict.get` 缺键 → 新增 E6008 KeyNotFound（#299 §4）
- 编译期移位溢出 → `const_overflow`（不再静默归零）
- `std native` 错误路径（`len/is_empty/has` 参数错）→ Err 显式报
- 顶层绑定与变量解析 → E3006/E3007（不再静默填 0，#271）

#### 作用域与闭包（#295/#294/#254）

- 作用域三链模型：`globals / param_scopes / local_scopes` 分离，闭包不捕获落地（外层 enter_fn 时整体移出），curry 跨边界累积 = 柯里化固化
- 函数体内类型定义 → E1071（不再静默跳过 `_ => Ok(())`，match 穷尽）
- 柯里化值参数固化错绑：参数原位注册（中间层 MakeClosure env 直接取 args 原位），native 高阶回调 env 前置（#294）
- spawn 闭包捕获外层变量真实传值：IR 收集 `task.reads` + 字节码补 LoadUpvalue/StoreUpvalue（#254）
- Standard 调度在飞任务计数跨 drive 丢失修复

#### 类型检查与解析

- 位运算/移位运算符解析层补全：`&` 曾静默吞右操作数（#285）
- Array(T,N) 字面量落点补 N 校验与元素类型校验（#300）
- for 循环变量类型记录进 `function_local_vars`（块作用域 exit 即销毁，活不到统一保存时刻，#303）
- vec 类容器（Tuple/List/Array）Eq/Ne 结构相等：递归逐元素 + Handle 同一性快速路径（#304）
- Arc/Weak 注解 lower 对齐签名解析（不再降 Generic 名占位，`Option(Arc(Int))` E1002 误报修复）

#### LSP / 工具链

- `dump` 模板占位符格式说明符未替换：`{0:08x}/{0:04}/{1:14}/{0:?}` 不匹配原样输出，Magic/Flags/指令行被遮蔽（#272）
- `in_yaoxiang_project` 去掉 cfg(cli) 保护，wasm-pack `default-features=false` 时 E0425 找不到（#272 发版阻塞）
- E6008 locale 文案补齐：六个 locale（ja/ru/classical/miao 英文兜底待翻译），修复渲染成 `Internal error: missing i18n template`
- clippy `-D warnings` 失败：`mutable_key_type` crate 级 allow、`len_without_is_empty` 补 `is_empty`、`ir_gen` expect 改 if let

### ♻️ 重构优化

#### 所有权 / move 分析（#264）

move 分析改 CFG 数据流（NLL/Polonius 风格）：前向数据流 + 汇合 meet（Dropped>Moved>Alive）+ 循环不动点 + 字面量条件裁剪（不可达分支不建边）。`if false` 分支内 move 不再泄漏到汇合点；运行时条件分支 move 保守传播（汇合 meet = Moved，仍报错，Rust 同语义）；循环体 move 后循环外使用报错。块表达式 `{ stmt }` 作语句 IR 崩修复（补 `Expr::Block` 分支）。

#### 测试合规整改（#269）

完整阶段 1-3 大清理：

- 阶段 1：删 tokio 死依赖、134 个零引用 MSG 变体、29 个预留未发错误码、50 个死 Opcode 变体、CompileProgress/CompilationPhase/PipelineState 等零消费者；删 --tui 死旗标、Commands::Version、ReplConfig.history_size 等
- 阶段 2 前半：AST 解构与类型替换去重（`Expr::receiver_parts()/callable_parts()` 收编 5 处，`substitute_type_refs` 两份副本合，`validate_positions` 两份合，括号深度切分两份合）
- 阶段 2 小件二：tlog! 宏 16 臂压为 1 臂、RuntimeStats 记账 facade/engine 两处合一、SourceMap 行末注释处理、locate 1-indexed 换算、package add/rm 的 save+lock 序列
- §14.1/14.2：bytecode 解码 `op_u16/op_u32` 助手（48 处手搓 `from_le_bytes` 消重 -142 行）；Opcode 枚举替换 u8 常量词表（编码/解码共用，消除双词表漂移）
- §14.3/§15.std：FFI 三板斧（`export!` 宏 -400+ 行、`expect_list/expect_dict` 22 处收编、NativeContext 堆访问器 12 处收编）
- §14.4：签名 SPEC 规范化，解析器删旧语法
- §16.4：code_helpers! 宏收编 102 个 find+builder 样板（-477 行）
- §16.5：双翻译系统合一（`codes/i18n/*.json` 6 语树并入 `locales/*.json` 105 错误码 × 6 语）
- §18.1/18.2/18.3：RFC-027 测试 AST 样板收编（binop/refined_int 37+17 处）+ 横幅注释清理（-827 行装饰）+ 12 个 std 模块 impl Default 改 derive

#### 类型系统退役

退役类型级字符串协议：删除 evaluator 字符串前缀分发与 `eval_if/eval_match/eval_nat`，normalizer If/Match 字符串归约路径；三值语义交由 conditional 承担。

### 🔧 其他变更

- 依赖更新（dependabot）：codemirror / daisyui / mermaid / postcss / vue / crossbeam-channel / thiserror / clap
- CI：关闭 pnpm 11 新包 24h 窗口检查（CI 步骤注入 `PNPM_CONFIG_MINIMUM_RELEASE_AGE=0`，本地 `pnpm-workspace.yaml` 保持默认供应链保护）

### 📎 提交记录

```
050c3a20 :bug: fix(typecheck): #303 #304——for 循环变量类型记录 + vec 容器结构相等
f85cf0f7 :sparkles: feat(frontend): #300 全项收尾——Range 值语义 + A/B/D 修复 + F 解析修复
fe405233 :memo: docs(design): #300 文档同步——Array 落点语义契约与 Set 除名
5a4215f7 :fire: chore(typecheck): in 白名单删除 Set 死代码声明
46572fa8 :white_check_mark: test(typecheck): #300 落点校验 E2E 三例与跨函数边界回归
5297e4a8 :bug: fix(frontend): #300 Array(T,N) 字面量落点补 N 与元素类型校验
8e4c033d :globe_with_meridians: i18n: auto-translate locale files
c5d898b7 :pencil: docs: auto-translate documentation
ecfc23f6 :bug: fix(util): E6008 补 locale 文案——修复渲染成 internal error
7b3cf6de :white_check_mark: test(backends): #299 测试合规修正——NewArray 测试补 AAA 分段与文件头声明
3bc89b97 :memo: docs(design): #299 §4 Task 4.4-4.5——文档同步与 RFC-011 方向锚
66707010 :sparkles: feat(backends): #299 §4 Task 4.1-4.3——错误码归并（E6008 新增 + 负索引归并 E6003）
ed2d13df :sparkles: feat(parser): #299 §3 Task 3.x——in membership 谓词（一等霍尔谓词）
1f9bca43 :white_check_mark: test(typecheck): #299 §2 Task 2.3——Array 定长规则天然成立
0da0a4d8 :sparkles: feat(codegen): #299 §2 Task 2.2——字面量上下文决定落点（List/Array）
d45bb167 :sparkles: feat(runtime): #299 §2 Task 2.1——NEW_ARRAY 字节码全链路
046996e8 :fire: refactor(types): #299 §1 Task 1.7——删除 MonoType 全部容器原生变体
0985f212 :hammer: refactor(types): #299 §1 容器类型去特殊化——MonoType 复合变体全迁 Generic 路径
f9bc7cc6 :hammer: refactor(types): 为 MonoType 加 Generic helper 方法（#299 去特殊化前置）
8eb79abb fix(runtime): #299 阶段0——修静默 void（Dict 缺键/String 索引/std.dict.get 缺键改显式报错）
6e77b99c :recycle: refactor(typecheck): 补全 89e7c401 遗漏的位点迁移
89e7c401 :recycle: refactor(typecheck): 收敛编译期值参数判定为单一实现
ca12c4e4 :globe_with_meridians: i18n: auto-translate locale files
d6a95665 :pencil: docs: auto-translate documentation
d633a5a9 :memo: docs(design): 更新 RFC-011 落空候选处理说明
405f9d5c :white_check_mark: test(typecheck): 落空值参数与尾表达式回归测试
fd3d5c1f :bug: fix(typecheck): 检查字段默认值表达式
2ef12710 :bug: fix(typecheck): 类型构造器落空值参数报 E1094
5f89ba96 :bug: fix(codegen): 函数体尾表达式作为返回值
97826c80 :arrow_up: chore(deps): bump the npm-dependencies group across 1 directory with 4 updates
69297557 :pencil: docs: auto-translate documentation
c1ef33cb :rotating_light: fix(lint): 修复 CI clippy -D warnings 失败
92a2d7f1 rfc: RFC-032 缩小范围 — 仅 AST/IR 清理，MonoType 推迟到独立 RFC
db215128 :pencil: docs: auto-translate documentation
594680d7 rfc: accept RFC-011a 接口实现与动态分发
bbaf01de :memo: docs(docs): 修正 RFC-002 创建日期笔误
f8480723 :memo: docs(docs): 勘误 #296：修正编译期值参数定义
a800c2f4 :pencil: docs: auto-translate documentation
71a6d75e review: RFC-011a 接口实现与动态分发 — 消除 Self 魔法关键字，与 RFC-011 泛型系统统一
7ed51322 :globe_with_meridians: i18n: auto-translate locale files
d2d13d4c :bug: fix(typecheck): 函数体内类型定义报 E1071 而非静默跳过（#295 收尾）
c1790c85 :recycle: refactor(typecheck): 作用域三链模型——函数边界一等语义，闭包不捕获落地（#295）
0df814dc :white_check_mark: test(typecheck): curry_codegen 测分层结构行为，不测指令序列（原则 4）
eb109aaf :bug: fix(typecheck): 柯里化值参数固化错绑——参数原位注册 + native 回调 env 前置
1be53378 :pencil: docs: auto-translate documentation
77402c0b :memo: docs(types): 闭包语义收敛——不捕获、上下文柯里化固化，SPEC/RFC 对齐
b85781e5 :memo: docs(design): RFC-009a 勘误——区间模型、路径条件规则、SMT 定位
aee95e5e :white_check_mark: test(types): P0-7 所有权深水区测试矩阵 + 块表达式 IR 修复
8402dc4f :pencil: docs: auto-translate documentation
9c130b11 :bug: fix(typecheck): 泛型构造调用逐位匹配声明参数，两层调用值构造
3a850778 :memo: docs(types): 定义泛型构造调用语义，RFC 勘误对齐权威模式
d3a73c95 :bug: fix(typecheck): 泛型构造器参数个数检查与类型推断
4687d949 :pencil: docs: auto-translate documentation
29d664f4 :white_check_mark: test(typecheck): Result/Option 泛型矩阵补全
7e58aba2 :bug: fix(types): Arc/Weak 注解 lower 对齐签名解析
760516d4 :bug: fix(typecheck): 泛型构造器实参类型不匹配改报编译错误
68bd1ff8 :white_check_mark: test(parser): 位运算/移位 E2E 回归测试（#285）
9bda49d2 :bug: fix(parser): 位运算/移位运算符解析层补全（#285，& 曾静默吞右操作数）
0a7e9c3f :white_check_mark: test(backends): 除零错误 E2E 回归测试（#282）
daf4ea35 :bug: fix(runtime): 除零错误携带触发表达式（#282 关闭 <unknown> 回退）
9ef60741 :white_check_mark: test(backends): int_overflow 头部去具体错误码（与 index_oob_read 同风格）
ff3145b7 :bug: fix(backends): 运行时整数运算溢出改报错（#281）
fb8a60c3 :memo: docs(rfc): RFC-013 E6xxx 码表校准（#280）
620a97d6 :bug: fix(backends): E6xxx 错误码对齐接线（#280）
f094335e :white_check_mark: test(backends): index_oob_read 头部补 RFC-013 引用
bb5caa79 :bug: fix(backends): List/Tuple/Array 越界读写改硬错误（#279）
cf594ca4 :white_check_mark: test(backends): 堆测试合规整改
da1b4750 :bug: fix(runtime): 修复 Standard 调度在飞任务计数跨 drive 丢失
9570cb96 :sparkles: feat(backends): Heap 句柄 Arc 化实现跨线程捕获
9d2d9318 :recycle: refactor(types): 退役类型级字符串协议，删除 If/Match/Nat 求值机器
da9cd89a :arrow_up: chore(deps): bump the npm-dependencies group across 2 directories with 8 updates
c45ea118 :arrow_up: chore(deps): bump the production-dependencies group with 3 updates
44b43929 :globe_with_meridians: i18n: auto-translate locale files
985bf37c :bug: fix(types): 编译期移位溢出改报错
dc3955aa :bug: fix(std): std native 错误路径不再静默归零
b2a26d16 :bug: fix(middle): 顶层绑定与变量解析改硬错误,删除静默归零兜底
27e5817b :white_check_mark: test(typecheck): 签名测试文件头规范引用修正 + 变参测试去旧语法（test-compliance 规则 2.1）
11050afb :globe_with_meridians: i18n: auto-translate locale files
3cee3a45 :recycle: refactor(typecheck): 签名 SPEC 规范化，解析器删旧语法（#269 §14.4）
88012bcd :globe_with_meridians: i18n: auto-translate locale files
8aff6424 :pencil: docs: auto-translate documentation
0150e5e7 :recycle: refactor(parser): 删 6 处与变量名复读的英文行内注释（#269 §18.2 尾）
3cf1d05a :recycle: refactor(test): RFC-027 测试 AST 样板收编 + 横幅注释清理（#269 §18.1/18.2/18.3）
a7866000 :recycle: refactor(middle): bytecode 解码去重 + Opcode 枚举并 u8 常量（#269 §14.1/14.2）
3d7a4daa :recycle: refactor(std): FFI 三板斧收编（#269 §14.3/§15.std）
833df379 :recycle: refactor(util): 双翻译系统合一（#269 §16.5）
b2c8f7cc :recycle: refactor(util): code_helpers! 宏收编 102 个 find+builder 样板（#269 §16.4）
05a86092 :recycle: refactor(util): 样板小件收编（#269 阶段 2 小件二）
4d5c8132 :recycle: refactor(frontend): AST 解构与类型替换去重（#269 阶段 2 小件）
8850c386 :recycle: refactor(meta): ponytail 删除类第二波（#269 阶段 3 前半）
0bd7d8a1 :recycle: refactor(meta): ponytail 审计删除类第一波（#269 阶段 1）
b1fa6f29 :bug: fix(runtime): spawn 捕获外层变量真实传值（#254）
491dfe9e :green_heart: ci(docs): CI 关闭 pnpm 11 新包 24h 窗口检查
1fe7b3a2 :recycle: refactor(typecheck): move 分析改 CFG 数据流（NLL/Polonius，#264）
4ddec8cc :globe_with_meridians: i18n: auto-translate locale files
c6875f30 :bug: fix(util): dump 模板占位符格式说明符未替换，输出不可读（#272）
6f1f165c :bug: fix(util): in_yaoxiang_project 去掉 cfg(cli) 保护，wasm 构建 E0425（发版阻塞）
e13a1862 :arrow_up: chore(deps): bump the npm-dependencies group across 1 directory with 4 updates
3530aabc :arrow_up: chore(deps): bump clap in the production-dependencies group
```
