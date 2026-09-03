# YaoXiang 测试规范

> 版本: 2.0
> 适用分支: all (assert.assert pattern)

---

## 一、测试层级

YaoXiang 采用三层测试体系：

```
┌─────────────────────────────────────┐
│  第一层: E2E .yx 测试               │
│  tests/yaoxiang/                    │
│  验证: 完整编译+执行流水线          │
├─────────────────────────────────────┤
│  第二层: 集成测试                    │
│  tests/integration/                 │
│  验证: 模块间交互、公开 API         │
├─────────────────────────────────────┤
│  第三层: 单元测试                    │
│  src/*/tests/                       │
│  验证: 单个模块内部逻辑             │
└─────────────────────────────────────┘
```

---

## 二、E2E .yx 测试规范

### 2.1 目录结构

按语言规范章节组织（对齐 `docs/src/reference/language-spec/`）：

```
tests/yaoxiang/
├── 00-smoke/             # 冒烟测试
├── 01-syntax/            # 语法规范（对应 syntax.md）
│   ├── basics/           #   基本语法
│   ├── functions/        #   函数定义与调用
│   └── control-flow/     #   控制流
├── 02-type-system/       # 类型系统（对应 type-system.md）
├── 03-modules/           # 模块系统（对应 modules.md）
├── 04-concurrency/       # 并发模型（对应 concurrency.md）
├── 05-ownership/         # 所有权（独立章节）
├── 06-compile-errors/    # 编译期错误检测
├── 99-demos/             # 论文演示（非规范测试）
└── TEST_STANDARDS.md     # 本文件
```

### 2.2 文件头格式

每个 `.yx` 文件必须以以下格式开头：

```yaoxiang
// <章节>/<文件名>.yx
// 覆盖: 规范 §X.X
// 验证: 一句话描述此文件验证的内容
// 状态: ✅ 可运行 | ⚠️ 待修复 | 🔴 未实现
```

状态说明：

- `✅ 可运行` — 当前实现完全支持
- `⚠️ 待修复` — 已知有 bug，测试已注释或跳过
- `🔴 未实现` — 编译器暂未实现该特性

### 2.3 断言约定

使用 `assert.assert` 验证值，不再使用 `io.println("ALL TESTS PASSED")` 哨兵字符串：

```yaoxiang
// 🟢 新模式（强制）
use std.assert

main = {
    x = 42
    assert.assert(x == 42, "x should be 42")
}

// 🔴 旧模式（已废弃）
use std.io
main = {
    x = 42
    io.println(x)
    io.println("ALL TESTS PASSED")  // ❌ 不再使用
}
```

测试框架通过 exit code 判断（0=通过，非0=断言失败），不再依赖 stdout 字符串匹配。

### 2.4 文件头标记

文件头注释可声明测试性质，`yaoxiang test` 与 cargo test 语料 runner（yx_runner）
据此判定——两者共用同一解析实现（`src/util/test_markers.rs`，RFC-036 §8.2），
目录约定（按 06-compile-errors 目录反向判定）已废弃。标记扫描窗口为**前 16 行**：

```yaoxiang
// [test:error]: <应失败的原因>
// 预期: 编译错误 E2018
// [test:ignore]: <原因>
// [test:runtime]: standard
```

- `[test:error]` — 本文件**应失败**（编译期或运行期；验证正确报错，06-compile-errors
  目录用）。runner 反向判定：`yaoxiang run` 退出码非 0 = PASS；退出码 0 = FAIL（该报错没报）。
  标记后的 `预期: 编译错误 E2018` / `预期: 运行时错误 E6008` 行声明期望错误码，
  runner **实际比对**输出中的 `[EXXXX]`，码不符 = FAIL 并指明实际出现的码；
  无码行回退 exit≠0 判定。
- `[test:ignore]: <原因>` — 本文件被跳过，计入报告的 skipped（追踪 issue 编号，
  见提交检查清单）。
- `[test:runtime]: <模式>` — 子进程运行时模式（`standard` / `embedded` / `full`），
  runner 透传 `--runtime`。

### 2.5 已知 Bug 的处理

对于已知有 bug 的语法特性，测试文件可以包含被注释的代码，并标记 TODO：

```yaoxiang
// TODO: 修复 match bug 后取消注释
// desc = match 1 {
//     1 => "one",
//     _ => "other"
// }
// io.println(desc)
```

### 2.6 未实现特性的处理

不存在的功能不写测试。等实现后再补充 `.yx` 文件。

---

## 三、集成测试规范

### 3.1 测试位置

`tests/integration/` 和 `tests/yx_runner.rs`

### 3.2 断言要求

- 必须验证实际输出值，不能只 `assert!(result.is_ok())`
- 对 `yaoxiang::run()`，通过返回值判断编译执行成功
- 对 E2E 测试，通过 `std::process::Command` 捕获 stdout 验证输出

### 3.3 禁用测试处理

禁用的测试必须注明原因和跟踪 issue：

```rust
// TODO: 修复 stack overflow (#XXX)
// #[test]
// fn test_disabled() { ... }
```

---

## 四、单元测试规范

### 4.1 测试位置

每个模块下的 `tests/` 子目录，保持与源码相同的层级结构。

### 4.2 命名规范

```
测试模块：mod_<描述>_tests
测试函数：test_<特性>_<场景>
```

示例：

```rust
mod mod_parser_fn_def_tests {
    fn test_parse_fn_def_with_params() { ... }
    fn test_parse_fn_def_block_body() { ... }
}
```

### 4.3 断言要求

- 验证具体的返回值或数据结构字段，不使用 `assert!(result.is_ok())` 作为唯一断言
- 对解析测试：验证 AST 节点字段值
- 对类型检查测试：验证推断出的 MonoType

### 4.4 文件大小上限

单个测试文件建议不超过 500 行。超过时应拆分到多个文件或合并重复用例。

---

## 五、运行方式

```bash
# 全部测试
cargo test

# E2E .yx 测试
cargo test --test yx_runner

# 集成测试
cargo test --test integration

# 单元测试（lib）
cargo test --lib

# 单个模块测试
cargo test -p yaoxiang --lib -- <module>::tests::

# 手动运行单个 .yx 文件
cargo run -- run tests/yaoxiang/01-syntax/basics/variables.yx
```

---

## 六、提交检查清单

提交前确认：

- [ ] `cargo test` 全部通过
- [ ] E2E 测试文件有正确的文件头（// 覆盖: + // 验证: + // 状态:）
- [ ] 使用 `assert.assert` 而非 `io.println("ALL TESTS PASSED")`
- [ ] 每个 `assert.assert` 有自定义错误消息
- [ ] `[test:ignore]` 文件有追踪 issue 编号
