# YaoXiang 测试规范

> 版本: 1.0
> 适用分支: refactor/test-suite

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

按语言规范章节组织：

```
tests/yaoxiang/
├── 00-smoke/             # 冒烟测试
├── 01-basics/            # 基本语法（规范第2/4/5章）
├── 02-functions/         # 函数（规范第6章）
├── 03-control-flow/      # 控制流（规范第4/5章）
├── 04-types/             # 类型系统（规范第3章）
├── 05-data-structures/   # 数据结构（规范第2.6节）
├── 06-modules/           # 模块系统（规范第7章）
├── 07-errors/            # 错误处理（规范第9章）
├── 08-concurrency/       # 并发（RFC-001）
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

每个 `.yx` 测试文件**必须**在末尾输出 `ALL TESTS PASSED`：

```yaoxiang
io.println("ALL TESTS PASSED")
```

测试框架捕获 stdout 验证此行存在。

### 2.4 已知 Bug 的处理

对于已知有 bug 的语法特性，测试文件可以包含被注释的代码，并标记 TODO：

```yaoxiang
// TODO: 修复 match bug 后取消注释
// desc = match 1 {
//     1 => "one",
//     _ => "other"
// }
// io.println(desc)
```

### 2.5 未实现特性的处理

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
cargo run -- run tests/yaoxiang/01-basics/variables.yx
```

---

## 六、提交检查清单

提交前确认：

- [ ] `cargo test` 全部通过
- [ ] E2E 测试文件有正确的文件头
- [ ] 新测试输出 `ALL TESTS PASSED`
- [ ] 禁用的测试注明原因
