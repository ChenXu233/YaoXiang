# Changelog

## Unreleased

### ⚠️ 行为变更（RFC-029g：删除 `pub` 关键字与自动绑定 —— #399 / WBS P3.5）

- **`pub` 不再是关键字**：RFC-029 早已裁定「没有可见性机制」，本次把 RFC-029g 的删除执行到底——`pub` 从词法、AST、类型检查、死代码豁免、格式化器与 LSP 语义着色中整体移除。它现在只是普通标识符，旧写法 `pub x = 1` 会按未定义名报 `E1001`。
- **死代码告警口径收敛**：`W1001`–`W1005` 不再豁免 `pub` 声明（`exempt_pub` 开关与角色分派删除）。唯一豁免口是**包内引用池**（RFC-029f）：项目内任一文件引用到的名字视为活。按角色的变化——Bin/Internal 不变；Lib 从「pub 绝对豁免」收紧为引用池判定；Script（单文件）未引用的顶层绑定开始报 W1001/W1002/W1004/W1005。
- **死表清理**：typecheck 环境的 `exports` 集合（生产端灌表、全仓零消费）与 `add_export`/`is_exported`/`is_visible` 访问器、`auto_bind_to_type` 自动绑定、`env.exports` 回写全部删除——导出面的唯一权威仍是模块注册表 `ModuleInfo.exports`。`project_refs` / `cross_file_refs` 两个同义引用池收敛为一个。
- **std 适配**：`src/std/{list,json,option,result}.yx` 的 31 处 `pub` 前缀删除。

### ⚠️ 行为变更（WBS 3.4 静默丢弃点补遗 + SMT 缓存契约）

- **W1063 开始发射**：const 泛型约束的实参不可编译期求值时（如 `BadArray(Int, n)` 传运行期值），此前静默放行、约束保护失效且零信号；现产 Warning 级诊断 `W1063`（不阻断编译）。编译器内部探测路径（match 泛型展开的 fresh 推断变量）不触发。（3.4.1）
- **W1081 新增**：SMT 求解器不可用（Z3 缺失）时，终止性测度义务的判定从「静默跳过」改为产 Warning 级诊断 `W1081`（含未判定义务计数）。求解器在场但判不出（NotProved）不触发。（3.4.2）
- **SMT 查询缓存契约**：`Unknown`（超时/能力外）结果不再入缓存——它是资源依赖的非确定性结果，缓存会把「首次求解时的机器负载」固化为进程级事实（同项目不同负载下诊断可能不同）。`Sat`/`Unsat` 确定性结果照常缓存。长编译中曾命中缓存的 Unknown 查询，现在会在资源充裕时重试得到确定结论。

### ⚠️ 行为变更（P4 阶段契约：A1/C3 裁决 + 多文件 std.list 修复）

- **多文件 for 循环运行期 E6006 修复**：项目内不写 `use std.list` 的 `for` 循环此前**编译通过、运行期**报「Native function not found: std.list.iter」（#117 硬切换只改了单文件的嵌入 std 注入，多文件发现漏改）。现多文件发现阶段与单文件同源、无条件纳入嵌入 std.list 编译单元。（WBS 4.10.2）
- **多文件多重失败的首报顺序**：同一项目中「某文件类型检查失败 + 另一文件证明义务失败」并存时，首报由证明错误变为类型检查错误（ProofExecution 统一为独立阶段、在全部类型检查之后执行；单一失败场景诊断不变）。（A1 裁决 / WBS 4.2.1）
- **`yaoxiang check` 文件内诊断顺序归一**：同一文件有多条诊断时，stderr 条目顺序从「主循环装配序」归一为阶段拓扑序（E3020 入口校验 → 类型错误 → W1006/W1003/死代码 → 证明错误）；诊断集合、错误/警告计数与退出码完全不变。（顺序归一裁决 / WBS 4.2.2）
- **`yaoxiang check` 对含语法错误的项目改为收集式报告**：项目内某文件有语法错误时，不再整体中止只报首处 parse 错误——带病文件退出当次编译（不参与 registry/typecheck），其 parse 诊断与其余文件的检查诊断一次性收齐，导入方报「模块未找到 E5001」，退出码照常非零（rust 式收集语义）。（WBS 4.10.1）
- **内部重构（无行为变化）**：`check_project` 瘦身为 `Program { kind: Check }` 构造器，编译阶段 12 变体全接线（RoleClassification 落地），check 路径每文件 parse 从 2 次降为 1 次。（WBS 4.2.2）
- **LSP 项目内诊断不再被无关文件打断**：编辑器内做项目级诊断时，若项目里其他磁盘文件有语法错误，此前整体静默退回单文件模式（跨文件解析能力丧失）；现磁盘文件与 `yaoxiang check` 同策略降级（收集其 parse 诊断并跳过该单元），被编辑缓冲区的跨文件诊断不受影响；缓冲区的语法错误保留残缺 AST 继续分析（打字中间态语义功能不消失）。（WBS 4.2.3）
- **内部重构（无行为变化）**：单文件 `Pipeline::run` 与多文件 `compile_project` 统一为 Driver 驱动（声明式阶段表 + 拓扑跳过台账）；全语料 331 项诊断/退出码/输出 zero-diff，IR 快照零漂移。（WBS 4.1.3 / 4.2.1）

## :bookmark: V0.8.3: 包管理全链落地与终止性检查接线

> 发布日期: 2026-10-05

### 📦 版本信息

| 项目     | 值                |
| -------- | ----------------- |
| 发布日期 | 2026-10-05        |
| 版本变更 | `0.8.2` → `0.8.3` |
| 提交数   | 168 个 commit     |

### 📋 本次更新概要

本次发版交付 RFC-014 包管理体系全链路：工作空间（成员管理、共享 lockfile、严格可见性）、`.yxpkg` 源码包与 GitHub 适配层、`publish` 发布命令，以及 `[build]`/`[binaries]`/`build.yx` 三策略构建系统。语言侧按 RFC-027/027a 完成终止性检查接线——显式测度、谓词精化、路径守卫与 E4022 用户域错误码，同时把 while 循环取值拨正为块语义。std 补齐 `std.json`（纯 yx 实现）、`std.fs`、`std.net`（真实 HTTP 客户端）三个模块。另修复 f-string 转义协议错位、use 别名登记缺口、列表推导式过滤与多生成器等一批语言与工具链问题。

### ✨ 新功能

#### 工作空间（RFC-014c）

多包仓库成为一等公民：`[workspace.members]` 解析与发现、`workspace list` 查看，成员引用路径解析配 pnpm 式严格可见性，共享 lockfile 合并解析与 `[workspace.dependencies]` 继承，`workspace add/remove` 与 init 自动注册。

- `[workspace.members]` 解析 + 发现 + `workspace list`
- 成员引用路径解析 + pnpm 式严格可见性
- 共享 lockfile 合并解析 + `[workspace.dependencies]` 继承
- `workspace add/remove` + init 自动注册

#### 发布链路（RFC-014a/014c）

包管理从"能装"推进到"能发"：`.yxpkg` 源码包格式（打包/解包 + SHA256SUMS 校验），GitHub 适配层（退避 + ETag 缓存 + Release 资产），`publish` 命令（`--dry-run`/`--github`、发布前默认跑测试可 `--no-test` 跳过、workspace 引用发布替换）。Source 分发 enum 化并原生 async 化。

- `.yxpkg` 源码包格式 + SHA256SUMS 校验
- GitHub 适配层：退避 + ETag 缓存 + Release 资产
- `publish` 命令：`--dry-run`/`--github` + 发布前测试 + workspace 引用替换
- Source 分发 enum 化 + 原生 async 化

#### 构建系统（RFC-014b）

包内的原生代码构建落地：`[build]`/`[binaries]` 配置解析与策略枚举、cargo 策略（拼命令 + 平台覆盖 + scratch 隔离）、`[binaries]` 预编译下载 + 校验 + 安装决策树、`build.yx` 执行与信任门。

- `[build]`/`[binaries]` 解析 + 策略枚举 + 依赖预检
- cargo 策略：拼命令 + 平台覆盖 + scratch 隔离
- `[binaries]` 预编译下载 + 校验 + 安装决策树
- `build.yx` 执行 + 信任门

#### 项目模式解析与缓存（RFC-014）

- vendor/lock 一致性预检 + lock 优先解析 + E5001 install 提示
- 本地优先解析 + W1006 遮蔽诊断 + `--deny-shadowing`
- 全局缓存 + semver/sha2 接入 + `outdated`/`clean`/`cache` CLI
- path 依赖复制进 vendor，install 后 `use` 不再 E5001
- `workspace.members` 传目录报真实错误而非误判 nested
- init 模板改函数形态，值块 main 开箱即坏根除
- std 嵌入二进制转正为正式机制（RFC-014 std 不包化修订）

#### 终止性检查接线（RFC-027/027a）

递归与循环的终止证明从"策略 1 不可用"推进到可用：显式测度提取与终止检查器注入，循环回边测度义务与标志循环测度（策略 1 恢复生效），谓词定义注册使精化约束可符号推理，形参精化进前置条件、路径守卫累积使 gcd 递减义务可判成立，调用点强半场与返回点精化义务按 RFC-027 §3 验证。测度判伪与良基性成对判定，不成立时报 E4022 用户域错误码（六语言文案）。

- 显式测度提取与终止检查器注入（T2–T4）
- 循环回边测度义务 + 标志循环测度
- 谓词定义注册 + 精化约束符号推理
- 形参精化前置条件 + 路径守卫累积
- 调用点强半场 + 返回点精化义务
- E4022 测度不成立注册 + 六语言文案

#### std 三模块与底层通道

- `std.json`：纯 yx 实现启用 + 功能语料 + E6013 解析失败码
- `std.fs`：路径级文件系统模块（#104）
- `std.net`：接入真实 HTTP 客户端 ureq（#56）
- 码点原语 + 纯 yx 层 Error 构造通道

#### 语言与工具链

- 列表推导式 `if` 过滤与多生成器（Python 语义，#401）
- `use` 多别名支持 + 导入冲突显式报错（#410/#415）
- `format` 类型化格式说明符（#402）
- Bin 入口按绑定存在性判定（E3021 停用）
- TextMate grammar 三引号 + 多行 f-string

### 🐛 Bug 修复

#### 语言与类型检查

- f-string 花括号转义协议错位与三引号支持（#402）
- namespace 数据访问与无标注绑定导出（#396）
- `use` 别名命名空间登记缺口
- while 取值按块语义，循环值不再恒为 Void + codegen 运行期对齐
- match scrutinee 类型按 span 贯通 IR 层
- 分组 use 同权与和类型实参收集三处缺口
- 命名空间调用 arity 同规与 native 区间（#387）
- 所有权/签名/寄存器三线硬化（#390–#394 根除）
- RFC-009 Dup 表 struct 派生（#398）
- 谓词应用名开集化，带实参精化形参可解析
- 精化类型对统一透明，形参位回归锁定

#### 工具与构建

- script 模式 main 未调用时显式提示，不再静默成功（#413）
- git 子进程剥离 `GIT_*` 环境变量 + 钉稳定 cwd + 本地克隆/ls-remote 静默降级修复
- wasm32 构建修复：tokio net 门控与 package 依赖降级
- formatter 注释重复输出导致的非幂等
- 顶层 `-L` 对 explain 失效与 i18n 旁路
- Lib 角色非 pub 死代码误报——分析器增设跨文件引用豁免
- clippy 1.99 needless borrow 判定适配

### ♻️ 重构与测试

- std 4 个生产长函数拆分
- typecheck 未接入的 dispatch 管道删除
- 全仓测试合规修缮：命名/AAA 分段/断言消息/超长用例重构、内联测试模块外迁

### 📝 文档

- RFC-039 编译器架构重构设计文档新增
- RFC-029g（移除 pub 与自动绑定）接受落章
- RFC-012 f-string 支持矩阵与 BNF 补全 + 教程回调（#402）
- 模块与包管理指南按实现对账重写（#63）
- 三道事实对账门禁上线 + 英文侧 32 处示例失败修复
- 日/俄文档翻译树删除，翻译树收敛为 en
- RFC-014 全阶段落地回写（Phase 3–6 落章 + 子 RFC 转正）
- RFC-027/027a 循环形态与终止检查回写
- list.pop 文档矛盾修正（#404）

### 🔧 其他变更

- 依赖升级：sha2、thiserror、@types/node、npm-dependencies group
- 发版流水线单点化 dist-release.yml
- 示例门禁匹配改大小写不敏感 + 翻译流水线放开代码块注释
- gitignore 增补本地工作区与 AgentTeams 状态目录
