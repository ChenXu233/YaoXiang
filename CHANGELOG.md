# Changelog

## :bookmark: V0.8.0: 工业化分发落地与所有权门槛推进

> 发布日期: 2026-09-15

### 📦 版本信息

| 项目     | 值                  |
| -------- | ------------------- |
| 发布日期 | 2026-09-15          |
| 版本变更 | `0.7.14` → `0.8.0`  |
| 提交数   | 73 个 commit        |

### 📋 本次更新概要

**⚠️ 安装与调用方式变化（RFC-037）**：引擎二进制改名 `yaoxiang-rs`，新增工具链前门 `yx`（rustup 式多版本管理）。日常入口改为 `yx <命令>`（`yx check`/`yx run`/`yx test` 等，命令透传引擎），直接调用 `yaoxiang` 的旧方式不再存在。

本次发版落地 RFC-037 工业化分发全链：`yx` 前门 + 多版本工具链管理（`yx toolchain` / `yx-toolchain.toml` 项目级锁定）、cargo-dist 双层安装渠道（一行命令与解压即用）、Z3 全平台动态链接随包分发、发行链路三轮安全审计加固。0.8.0 门槛系列同步推进：借用检查 G3（调用点所有权解析表 + 链式接收者写令牌）、嵌套泛型单态化路径 A（G2）、RFC-029f 编译目标角色与导入面语义（#334）、RFC-027a 终止检查范围收窄（#318）。类型检查侧 `#321` 定案 B 落地——W 码死代码族转私有语义，pub 定义作为对外接口永不报；`check`/`run` 统一编译路径，模块系统诊断（E5001/E5003）真实接线。

### ✨ 新功能

#### yx 工具链前门与多版本管理（RFC-037）

`yx` 作为日常入口只做版本解析与派发：`yx toolchain install/default/list/uninstall/update` 管理多版本共存（`~/.yaoxiang/versions/<版本>/` 自包含工具链树），`yx self update` 自更新，`yx-toolchain.toml` 项目级版本锁定（对位 `rust-toolchain.toml`）。其余动词逐字透传引擎 `yaoxiang-rs`，参数无改写。

#### 双层安装渠道（RFC-037）

一行命令安装（rustup 模式，Linux/macOS/Windows PowerShell）与解压即用（Go/Zig 模式，压缩包自带引擎、Z3 共享库与标准库源码）双渠道，另有 apt 仓库与 Windows Inno Setup 向导。Z3 从静态链接改为全平台动态链接随包分发（`bin/` 内 libz3 + rpath `$ORIGIN`/`@loader_path`），用户可整体替换升级；标准库三级查找链保证源码随版本锁定。

#### 借用检查门槛 G3：调用点所有权解析

调用点所有权解析表落地（类型信息流接口），链式接收者写令牌与错绑定修复随之交付，三语料锚定（0.8.0 门槛 G3 阶段 1-3）。方法接收者的借用/消费判定从局部启发式升级为调用点查表，`&mut` 接收者遍历期间正确压制消费者注册。

#### 编译目标角色与导入面语义（RFC-029f，#334）

RFC-029f 定稿并两阶段实现：角色分类 + 导入面 + Bin pub 可报（Phase 1），Internal pub 收紧与 patterns 级 Test 判定（Phase 2）。不同编译目标（Bin/Test/Internal/patterns）下的 pub 可见性有了明确语义。

#### 嵌套泛型单态化路径 A（#335，门槛 G2）

嵌套泛型调用按所在特化求值分发，调用点映射按类型实参二元改写；补深度/规模分离断言。

#### W 码死代码族私有语义（#321 定案 B）

W1001-W1005 从"未使用导出"转为"未使用私有"语义：pub 定义 = 对外接口永不报；W1003（未使用导入）移入 typecheck use elaboration 检出并走警告独立通道（不阻断编译，`--deny-warnings` 可收紧为错误）。RFC-013 码义与 zh 文案同步修订。

### 🐛 Bug 修复

#### check/run 编译路径统一与模块诊断

`check` 项目内文件走与 `run` 相同的编排路径，修复多文件项目 check 假报 E1001 unknown variable；use 模块未命中报 E5001、导出未命中报 E5003（此前静默跳过）；vendor 依赖进导入路径；`install` 任一依赖失败即非零退出（此前打印失败仍返回成功），缺 git/path 来源的 registry 依赖不再静默跳过。

#### LSP 诊断走项目编排路径

LSP 单文件诊断接入项目编排，跨文件 use 不再假报 E1001；接线 40 个孤儿 LSP 测试。

#### 分发链安全审计修复

三轮安全审计发现全部闭环：tar 解包按构造拒绝 symlink/hardlink 条目与 `..` 穿越条目；`yx self update` 下载前写探针检测目标目录可写（Program Files 场景提前报错并给提权指引）+ .sha256 校验；publish 下载失败不再静默零资产；apt-repo Pages 分支不再含全仓库源码。网络失败报错附镜像自助指引，安装指南扩写镜像节。

#### 0.8.0 盘点应修项

- fresh-var 兜底三处硬化 + ListComp/Array 类型洞 + 证明调用静默丢参
- VC 证伪与 spawn 环诊断不再吞没
- match 未支持模式拒绝编译——stub 静默错译改报 E3008（#330 安全网）

#### 发版阻塞修复（#335）

- Z3 链接适配 dist 多目标矩阵：macOS 静态链 fat 库、Linux 放行 .so 未定义符号
- package-dist tar 归档路径错位、Inno Setup PkgDir 相对路径与 recursesubdirs 旗标

### ♻️ 重构优化

#### 单态化与死代码清理

- ModuleResolver 死代码删除（vendor 解析已归 orchestrator）
- gen-std 子命令取消，改仓库预生成接口视图 + 测试门禁强制同步
- Z3 共享库单源化为构建输出
- yx 测试迁父级 `tests/`，撤销 `#[ignore]` bless 入口改 examples 工具

### 📝 文档

- 错误码三套参考页对齐 `define_codes!` 注册表（85 → 118，补 W1080/E6006/E6007）
- 新增 `yx test` / `yx format` 命令参考页；check 页对齐实际旗标面（补 `--deny-warnings`、删失效 `--watch`）
- 命令示例全量迁移 yx；design 域文档命令名同步迁移
- RFC-027a 终止检查草案与范围收窄（#318）；concurrency.md §3.4 闭包捕获规范矛盾修正（RFC-009 决议）
- RFC-037 正文对齐实际实现

### 🔧 其他变更

- 依赖更新：zip 2 → 8、sha2 0.10 → 0.11（file_sha256 手动十六进制化适配）、npm 组两轮、pnpm overrides 修复 dependabot 高危告警
- CI：Build CLI 步骤跟随引擎改名；dist-release 支持手动指定 tag 构建
- rustls 升级 0.23.45（RUSTSEC-2026-0285）
