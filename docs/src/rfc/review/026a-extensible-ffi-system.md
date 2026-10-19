---
title: 'RFC-026a: 可扩展 FFI 机制体系'
status: '审核中'
issue: '#135'
author: '晨煦'
created: '2026-06-05'
updated: '2026-10-14'
group: 'rfc-026'
---

# RFC-026a: 可扩展 FFI 机制体系

> **父 RFC**: [RFC-026: FFI 核心机制](../accepted/026-ffi-core-mechanism.md)
>
> 本 RFC 定义 RFC-026 的可扩展性部分——如何把 C
> ABI 之外的 FFI 机制（Wasm、Python、自定义 ABI）作为插件接入，以及动态加载模式。

## 摘要

RFC-026 定义了 FFI 核心机制，`Native.c("lib")` 走内置 C ABI。本 RFC 把 ABI 机制抽象为可插拔的
`FfiMechanism`，让核心不硬编码任何具体 ABI：

1. **`FfiMechanism` 抽象**：定义机制必须实现的四个操作（加载库、解析符号、编组、调用）
2. **机制标签即机制选择**：`Native.c` / `Native.wasm` / `Native.python` 分别选择注册的机制
3. **编译期机制注册表**：机制标签在编译期校验，未注册的标签产生编译错误
4. **静态 vs 动态加载**：两种模式都保持 RFC-026 的安全边界

## 动机

RFC-026 只内置 C ABI（`Native.c`）。但 YaoXiang 未来可能需要：

- 调用 Wasm 模块（`Native.wasm`）
- 嵌入 Python 扩展（`Native.python`）
- 用户自定义 ABI（专有硬件、RPC 桥接）

与其在编译器里硬编码这些 ABI，不如把"如何加载库、如何解析符号、如何编组、如何调用"抽象成一个 trait，每种机制作为插件实现。核心只认识
`FfiMechanism`，不认识任何具体 ABI。

### 设计约束

1. **机制标签编译期校验**：`Native.xxx(...)` 中的 `xxx` 必须是已注册的机制，否则编译错误
2. **不硬编码机制**：编译器不内置机制列表（除 `.c` 作为参考实现），机制由插件注册。
   **`.rs` 的地位（2026-10-14 澄清）**：现状代码已存在两个机制标签——`.c`（C ABI，RFC-026）
   与 `.rs`（YaoXiang std 内部 Rust 函数分发，`interpreter/ffi.rs` 模块文档、`typecheck/mod.rs`
   `Native.rs` 注册）。`.rs` 是 std 实现细节而**非对外机制**——它不走插件注册、不对外开放第三
   方实现，本 RFC 的机制注册表只管理对外可插拔机制（`.c` 起步），`.rs` 保持硬编码旁路。
3. **保持 RFC-026 安全边界**：任何机制都必须遵守类型两分、编组临时区隔离、Move + RAII
4. **自举兼容**：机制注册表退化为 YaoXiang 的 `Dict`/`Set`

---

## 提案

### 1. `FfiMechanism` 抽象

每种 FFI 机制实现四个操作。这是核心不硬编码 ABI 的关键——编译器只调用这个接口，不知道背后是 C、Wasm 还是别的：

```rust
trait FfiMechanism {
    /// 机制标签，如 "c" / "wasm" / "python"
    fn tag(&self) -> &str;

    /// 加载库。C: dlopen/静态 link；Wasm: 实例化模块；Python: import。
    /// 返回一个机制内部的库句柄。
    fn load_library(&self, id: &str) -> Result<LibraryHandle>;

    /// 解析符号。编译期可调用以验证符号存在。
    /// C: dlsym/符号表查找；Wasm: 导出表查找。
    fn resolve(&self, lib: &LibraryHandle, symbol: &str) -> Result<SymbolHandle>;

    /// 调用。按 YaoXiang 签名编组参数、执行、编组返回值。
    /// 必须遵守 RFC-026 §3 的编组规则（临时区隔离）。
    fn invoke(
        &self,
        sym: &SymbolHandle,
        args: &[RuntimeValue],
        sig: &Signature,
    ) -> Result<RuntimeValue>;
}
```

**关键**：`invoke` 的实现必须遵守 RFC-026
§3——入参复制到临时区、返回 memcpy、借用限定单次调用。机制可以选择自己的 ABI 细节，但**不能违反安全边界**。这是插件的义务。

### 2. 机制标签即机制选择

```yaoxiang
// .c → C ABI 机制（RFC-026 内置参考实现）
sqlite3 = Native.c("libsqlite3")
SqliteDb.open: (f: String) -> ?SqliteDb = sqlite3("sqlite3_open")

// .wasm → Wasm 机制（yx_wasm_ffi 插件注册）
wasm_mod = Native.wasm("mymodule.wasm")
process: (input: String) -> String = wasm_mod("process")

// .python → Python 机制（yx_python_ffi 插件注册）
np = Native.python("numpy")
```

`Native.c` / `Native.wasm` 中的 `.c` / `.wasm` 是**机制标签**，选择用哪个注册的
`FfiMechanism`。核心内置 `.c` 作为参考实现；其他由插件提供。

### 3. 机制注册与编译期校验

插件通过 `.so` 在编译期向机制注册表声明它提供的机制标签：

```text
use yx_wasm_ffi
  → 加载 libyx_wasm_ffi.so
  → 调 yx_register_mechanism()
  → 注册 FfiMechanism { tag: "wasm", ... }
  → 机制注册表新增 "wasm"

// 之后：
Native.wasm("mod.wasm")    // ✅ 编译通过，"wasm" 已注册
Native.foo("x")            // ❌ 编译错误: Unknown FFI mechanism 'foo'
                           //    Try: `use yx_foo_ffi`
```

编译期机制注册表**只存机制标签**（字符串）+ 对应的 `FfiMechanism` 实例指针。编译 `Native.xxx(...)`
时查表，标签不存在则编译错误。

#### 3.1 插件信任模型（2026-10-14 补）

编译期加载 `.so` 插件意味着**编译过程执行任意本地代码**——插件与编译器同进程、同权限。这不是
本 RFC 独有的问题（proc-macro、编译器插件体系皆有此性质），但必须在接受前明确立场，不能默认：

| 层级 | 措施 | 状态 |
| ---- | ---- | ---- |
| 来源约束 | 机制插件只能来自包管理器（RFC-014）可信源安装路径，**不搜索 `PATH`、不加载当前目录 `.so` | 本 RFC 定死 |
| 透明性 | 实际加载的插件路径在编译诊断中输出（`--verbose` 默认可见），构建可复现 | 本 RFC 定死 |
| 签名 | 注册中心（RFC-014a）对插件包签名，编译器校验签名后加载 | 推迟到 RFC-014a 联合设计 |
| ABI 版本化 | `yx_register_mechanism` 入口带版本协商参数（`ABI_VERSION` 常量），不匹配即拒绝加载并报编译错误 | 本 RFC 定死；具体数值实现期定 |

**威胁定性**：机制插件 = 编译期任意代码执行。与 Rust proc-macro 同级信任，靠来源约束 + 版本化
控制面，不承诺沙箱。这点与 RFC-026 §6（unsafe 逃生舱）的哲学一致：显式信任，不做隐藏隔离。

### 4. 静态 vs 动态加载

`load_library` 的实现决定加载时机，两种模式都保持 RFC-026 的安全边界：

| 模式                    | `load_library` 行为            | 符号验证                       | 类型               |
| ----------------------- | ------------------------------ | ------------------------------ | ------------------ |
| **静态**（默认，C ABI） | 编译期 `-llib`，库进符号表     | 编译期读符号表                 | 完全实             |
| **动态**                | 运行期首次调用时 dlopen/实例化 | 首次加载时验证，缺失 fail-fast | 声明可信，加载即验 |

```yaoxiang
// 静态：C 库编译期链入
sqlite3 = Native.c("libsqlite3")           // 编译期 -lsqlite3

// 动态：运行期发现的插件（语法承载方式待定，见开放问题 Q2）
plugin = <动态加载入口>("./plugins/foo.so")   // 运行期 dlopen
```

> **⚠️ 语法待定（2026-10-14）**：原文档此处写 `Native.c.dynamic(...)`。现状前端把 `Native.c`
> 注册为点分全名变量，`FieldAccess` 经 `extract_namespace_path` 从 `Var("Native")` 拼路径解析
> （`typecheck/mod.rs:218-247` 注释明示"Native 本身必须是 Struct 且有字段"）。`.dynamic` 是
> 第三级路径，该机制目前不承载。候选方案与取舍见「开放问题 Q2」，定案前不以 `Native.c.dynamic`
> 为承诺语法。

无论静态还是动态，编组都走 RFC-026
§3 的临时区隔离。动态模式下符号缺失是**干净的运行期错误**（fail-fast），不是崩溃。

### 5. 完整信息流

```
use yx_wasm_ffi                     ← 注册 "wasm" 机制
       │
       ▼
wasm_mod = Native.wasm("mod.wasm")
  编译期：查机制注册表 "wasm" 存在 ✅
         → 调 wasm 机制的 load_library("mod.wasm")
         → 实例化 Wasm 模块，返回库句柄
       │
       ▼
process: (input: String) -> String = wasm_mod("process")
  编译期：调 wasm 机制的 resolve(lib, "process") 验证导出存在 ✅
         → 生成 CallNative { mechanism: "wasm", lib, symbol: "process", sig }
       │
       ▼  运行期
  CallNative 执行
  → 机制的 invoke(sym, args, sig)
  → 按 sig 编组（临时区隔离）→ 执行 Wasm → 编组返回
```

### 6. 自举后的退化

Rust 托管期的 `FfiMechanism` trait + 机制注册表，自举后退化为 YaoXiang 的普通结构：

```yaoxiang
// 自举后，机制注册表是 Dict
let mechanisms: Dict(String, FfiMechanism) = {}
mechanisms["c"] = c_mechanism
mechanisms["wasm"] = wasm_mechanism

// FfiMechanism 是 YaoXiang 里的一个接口（RFC-011a 动态分发）
// Native.c("lib") → mechanisms["c"].load_library("lib")
```

Rust 期用 trait
object（`Box<dyn FfiMechanism>`），自举后用 YaoXiang 接口（RFC-011a）。接口一致：加载、解析、编组、调用。

---

## 权衡

### 优点

1. **零硬编码 ABI**：核心只认识 `FfiMechanism`，新 ABI = 新插件
2. **安全边界统一**：所有机制强制遵守 RFC-026 §3 编组规则
3. **编译期机制校验**：机制标签不存在编译期报错，不会运行期才发现
4. **静态动态统一抽象**：`load_library` 的实现细节隐藏在机制内

### 缺点

1. **插件编写门槛**：实现 `FfiMechanism` 需要理解目标 ABI + 编组契约
2. **机制义务靠约定，但约定要有牙齿（2026-10-14 补）**：编组临时区隔离靠插件遵守，核心无法在
   编译期完全静态验证任意插件实现。分层执行条款：
   - **接口义务（强制）**：`FfiMechanism` trait 方法签名即契约——`invoke` 必须按 `sig` 编组，
     违反签名属实现缺陷，运行时 panic 兜底；
   - **官方插件义务（强制）**：`yx_wasm_ffi` 等官方插件必须随编译器测试套件跑**机制契约测试**
     （一套标准编组用例：跨界 String/List/结构体、越界写、悬垂指针），契约测试红 = 发布阻断；
   - **第三方插件义务（约定 + 文档化）**：注册表记录插件自报的义务符合声明，文档明示
     "非官方机制的编组安全由插件作者负责"。核心提供契约测试框架供第三方自测。

---

## 开放问题（2026-10-14 评审补充）

**Q1：插件签名校验的归属**。本 RFC 定死来源约束与 ABI 版本化，签名校验推迟到 RFC-014a（注册
中心）联合设计。若 RFC-014a 落地晚于阶段 1b，中间窗口期机制插件只靠来源路径约束——是否可接受？
（倾向：可接受，与 proc-macro 生态早期状态一致；RFC-014a 定案后补校验。）

**Q2：动态加载入口的语法承载**。`Native.c.dynamic(...)` 在现状 FieldAccess 解析设计下不可达
（见 §4 警告框）。候选：

| 方案 | 优点 | 缺点 |
| ---- | ---- | ---- |
| a. `Native.c_dynamic("path")`（扁平标签） | 零解析改动，标签表直接承载 | 标签命名空间被 `_` 复合标签污染 |
| b. `Native.dynamic("c", "path")`（参数化） | 语法正交，机制标签仍是单级 | `dynamic` 成为保留特殊名，自身即特例 |
| c. 库层函数 `std.ffi.dynamic_load(mech, path)` | 编译器零改动，纯 std 实现 | 绕过编译期机制校验（标签是运行期字符串），违背约束 1 |

（倾向：b——`dynamic` 是加载模式的正交维度而非新机制，参数化表达最诚实；代价是 FieldAccess
需支持单级特殊方法分发，实现量可控。定案前阶段 1b 不含动态加载。）

**Q3：`.rs` 是否长期保留硬编码旁路**。本 RFC 将其定性为 std 实现细节（见设计约束 2）。若未来
std 全部走 YaoXiang 自举重写，`.rs` 是否随之退场或转为普通机制——留待自举路线（RFC-039）定案。

**Q4：Wasm 机制的 GC 边界**。Wasm 线性内存对象的生命周期与 YaoXiang 堆的 Move + RAII（RFC-026
§4）如何互操作——`LibraryHandle` drop 时 Wasm 实例是否强制释放？参考插件 `yx_wasm_ffi`
设计时必须回答，本 RFC 不预答。

---

## 实现策略

### 阶段 1a：机制抽象 (v0.8)

- [ ] 定义 `FfiMechanism` trait（load_library / resolve / invoke）
- [ ] 把 RFC-026 的 C ABI 实现重构为 `CMechanism: FfiMechanism`
- [ ] 实现编译期机制注册表（标签 → 机制实例）
- [ ] `Native.xxx` 编译期查机制注册表校验

### 阶段 1b：动态加载 + 插件 (v0.9)

- [ ] 实现 `.so` 插件加载（`yx_register_mechanism`，含 ABI 版本协商）
- [ ] 实现动态库加载模式（语法待 Q2 定案）
- [ ] 参考插件：`yx_wasm_ffi`（Wasm 机制，须过机制契约测试）

---

## 与其他 RFC 的关系

- **RFC-026**（父）：FFI 核心机制——`FfiMechanism` 必须遵守其编组规则和安全边界
- **RFC-011a**：接口与动态分发——自举后 `FfiMechanism` 退化为 YaoXiang 接口
- **RFC-014**：包管理系统——`.so` 插件的发现和加载依赖包管理器
- **RFC-021**（已废弃）：库驱动 FFI 扩展——本 RFC 将其 `ffi.load_library` API 下沉到机制插件层

---

## 设计决策记录

| 决策         | 决定                              | 原因                         | 日期       |
| ------------ | --------------------------------- | ---------------------------- | ---------- |
| 机制抽象     | `FfiMechanism` trait，四操作      | 核心不硬编码 ABI，只认接口   | 2026-07-03 |
| 机制义务     | 插件必须遵守 RFC-026 编组规则     | 安全边界不因机制不同而破坏   | 2026-07-03 |
| 机制标签校验 | 编译期查注册表                    | 未注册机制编译期报错         | 2026-07-03 |
| 静态/动态    | `load_library` 实现决定           | 时机是机制细节，安全边界不变 | 2026-07-03 |
| 自举退化     | trait → YaoXiang 接口（RFC-011a） | 不做宿主语言过度抽象         | 2026-07-03 |
| 插件信任模型 | 来源约束 + ABI 版本化，不承诺沙箱 | 与 proc-macro 同级信任，显式化 | 2026-10-14 |
| `.rs` 地位  | std 实现细节，不走注册表          | 与"除 `.c` 外皆插件"的表述对齐 | 2026-10-14 |
| `.dynamic` 语法 | 退回开放问题 Q2，原示例不作承诺 | 现状 FieldAccess 不承载三级路径 | 2026-10-14 |

---

## 生命周期与归宿

| 状态       | 位置                        | 说明         |
| ---------- | --------------------------- | ------------ |
| **审核中** | `docs/design/rfc/review/`   | 开放社区讨论 |
| **已接受** | `docs/design/rfc/accepted/` | 正式设计文档 |
