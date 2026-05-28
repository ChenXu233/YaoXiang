# 测试体系重构计划

> 状态：计划
> 分支：refactor/test-suite
> 日期：2026-05-10

## 一、为什么重构

### 现状问题

1. **1752 个测试全部通过，但没抓住真实 bug**
   - match 表达式运行时返回 0（ir_gen 未处理 Match 节点）
   - 列表推导式返回 0（ir_gen 未处理 ListComp 节点）
   - `x: Int = 42` 类型标注变量声明解析失败

2. **集成测试仅验证编译成功，不验证运行时输出正确性**
   - `tests/integration/interpreter.rs` 只 `assert!(result.is_ok())`
   - `tests/integration/execution.rs` 完全被注释掉

3. **E2E .yx 文件无体系**
   - 新旧混放：`closure_test.yx`（旧）和 `spec_features_test.yx`（新）都在同一目录
   - 命名无规范：`closure_test.yx`、`closure_test2.yx`、`mut_param_test.yx`
   - 无覆盖规划：没有对应语言规范的章节映射

4. **内联测试碎片化**
   - `src/frontend/typecheck/tests/` 有 23 个文件，许多测同一件事
   - scope 测试分散在 4 个文件中
   - infer 测试分散在 3 个文件中
   - `typecheck_fixes.rs` 疑似历史补丁遗留

5. **Codegen 测试孤立**
   - 全部手造 IR，不经过 parser→typecheck→ir_gen 完整流水线
   - 测试的是"手写 IR 能否被翻译成字节码"，而不是"源码编译结果是否正确"

### 重构目标

1. **建立三层测试体系**，每层有明确的职责和覆盖标准
2. **E2E 测试可同时作为 benchmark** — 每个 .yx 测试文件可测量执行时间
3. **内部测试规范化** — 统一的测试约定、命名、断言模式
4. **覆盖语言规范关键路径** — 确保语言规范中定义的语法特性有对应的测试

---

## 二、三层测试体系

### 第一层：E2E .yx 测试套件（tests/yaoxiang/）

按语言规范章节组织，每个文件对应一个语法特性。

```
tests/yaoxiang/
├── 00-smoke/                 # 冒烟测试
│   └── hello.yx
│
├── 01-basics/                # 基本语法（规范第 2/4/5 章）
│   ├── variables.yx          # 变量声明 + 类型推导
│   ├── typed_vars.yx         # 类型标注变量 x: Int = 42
│   ├── operators.yx          # 所有运算符
│   ├── literals.yx           # 所有字面量
│   └── comments.yx           # 注释
│
├── 02-functions/             # 函数（规范第 6 章）
│   ├── definitions.yx        # name: (params) -> Ret = ...
│   ├── lambdas.yx            # Lambda 表达式
│   ├── closures.yx           # 高阶函数
│   └── generics.yx           # 泛型函数
│
├── 03-control-flow/          # 控制流（规范第 4/5 章）
│   ├── if_else.yx
│   ├── while.yx
│   ├── for.yx
│   ├── match.yx
│   └── list_comp.yx          # 列表推导式
│
├── 04-types/                 # 类型系统（规范第 3 章）
│   ├── structs.yx            # Point: Type = { x: Float, y: Float }
│   ├── enums.yx              # Color: Type = { red | green | blue }
│   └── generics.yx           # Option: (T: Type) -> Type = ...
│
├── 05-data-structures/       # 集合类型（规范第 2.6 节）
│   ├── lists.yx
│   ├── tuples.yx
│   └── dicts.yx
│
├── 06-modules/               # 模块系统（规范第 7 章）
│   ├── imports.yx
│   └── lib/
│
└── 07-errors/                # 错误处理（规范第 9 章，标记未实现特性）
    ├── result.yx
    └── option.yx
```

**文件规范**：

```yaoxiang
// 01-basics/variables.yx
// 覆盖: 规范 §5.2 变量声明, §6.2 类型推导
// 验证: 基本声明、类型推导、可变性
// 分支: refactor/test-suite
// 状态: ✅ 可运行

use std.io

main = {
    x = 42
    io.println(x)
    // expect: 42

    s = "hello"
    io.println(s)
    // expect: hello

    io.println("ALL TESTS PASSED")
}
```

**断言机制**：Rust 测试框架捕获 stdout，验证 `ALL TESTS PASSED` 字符串出现在每个 .yx 文件的输出中。

**Benchmark 扩展**：`.yx` 测试文件自然可作为性能基准——测量每次运行的时间。将来可用 `criterion` 包装，追踪性能回归。

### 第二层：集成测试（tests/integration/）

测试完整编译+执行流水线，验证输出值。

| 当前文件 | 操作 | 说明 |
|---------|------|------|
| `interpreter.rs` | 重写 | 改为编译源码 → 执行 → 断言输出值 |
| `execution.rs` | 重写（取消注释） | 修复 stack overflow，运行真实 .yx 文件 |
| `codegen.rs` | 保留 | 字节码序列化/反序列化 |
| `codegen_extended.rs` | 保留 | opcode/metadata 测试 |
| `fstring.rs` | 保留 | 补充执行验证 |
| `backends.rs` | 保留 | RuntimeValue 类型测试 |

**补充**：`tests/yx_runner.rs` — 自动发现并运行 `tests/yaoxiang/` 下所有 .yx 文件。

### 第三层：单元测试（src/*/tests/）

测试单个模块的内部逻辑，可访问私有 API。

#### 3.1 Lexer 测试（src/frontend/core/lexer/tests/）

11 个文件 → 删 1 个调试文件，留 10 个。

| 操作 | 文件 |
|------|------|
| 删除 | `debug_lexer.rs` — 仅调试用 |
| 保留 | `basic.rs`, `comments.rs`, `keywords.rs`, `literals.rs`, `operators.rs` |
| 保留 | `delimiters.rs`, `errors.rs`, `fstring.rs` |
| 保留 | `rfc004_lexer.rs`, `rfc010_lexer.rs` |

#### 3.2 Parser 测试（src/frontend/core/parser/tests/）

13 个文件 → 审查后微调。

| 操作 | 文件 |
|------|------|
| 保留 | `basic.rs`, `fn_def.rs`, `syntax_validation.rs`, `old_syntax_rejection.rs` |
| 保留 | `boundary.rs`, `concurrency.rs`, `fstring.rs` |
| 保留 | `ref_test.rs`, `unsafe_ptr.rs`, `state.rs` |
| 审查 | `binding_enhancements.rs` — 检查是否与 fn_def 重复 |

#### 3.3 Typecheck 测试（src/frontend/typecheck/tests/)

**最大问题区**：23 个文件 → 合并为 12 个。

| 操作 | 原文件 | 目标文件 |
|------|--------|---------|
| 合并 | `infer.rs` + `inference.rs` + `types.rs` | `type_inference.rs` |
| 合并 | `scope.rs` + `shadowing.rs` + `use_scope.rs` + `use_block_scope.rs` | `scoping.rs` |
| 合并 | `visibility.rs` + `pub_bind.rs` | `visibility.rs` |
| 审查 | `typecheck_fixes.rs` | 如果只是历史补丁测试则合并入对应文件后删除 |
| 保留 | `basic.rs`, `check.rs` | — |
| 保留 | `constraint.rs`, `concurrency.rs`, `fstring.rs` | — |
| 保留 | `gat.rs`, `ref_test.rs`, `result_try.rs` | — |
| 保留 | `semantic_tokens.rs`, `traits.rs`, `type_constructor_rules.rs` | — |

#### 3.4 Middle/Codegen 测试（src/middle/passes/tests/）

| 目录 | 操作 |
|------|------|
| `codegen/` | 保留现有，**补充集成式 codegen 测试**（从源码编译到 IR 验证结构） |
| `lifetime/` | 保留不动 |
| `mono/` | 保留不动 |
| `module/` | 保留不动 |

## 三、测试规范文档

同一目录下创建 `TEST_STANDARD.md`，内容：

### 命名规范

```
用途        模式                      示例
─────────────────────────────────────────────
测试模块     mod_<描述>_tests          mod_parser_basic_tests
测试函数    test_<特性>_<场景>        test_parse_fn_def_no_params
E2E 文件    <章节>-<特性>.yx          01-basics-variables.yx
```

### 断言规范

- E2E `.yx` 文件：末尾输出 `ALL TESTS PASSED`
- 集成测试：验证 stdout 包含预期值
- 单元测试：验证数据结构字段值，不用 `assert!(result.is_ok())` 作为唯一断言

### 注释规范

```
// E2E 文件头：
// 覆盖: 规范 §X.X
// 验证: 一句话描述
// 分支: refactor/test-suite
// 状态: ✅ 可运行 / ⚠️ 待修复 / 🔴 未实现
```

### 未实现特性的处理

- E2E `.yx` 不存在的功能：不写测试，等实现后再补充
- 单元测试中引用未实现功能：用 `#[ignore]` 标记，注释里写 "待 XXX 实现后启用"

---

## 四、执行计划

### Phase 0：准备工作

- [ ] 从 `dev` 创建分支 `refactor/test-suite`
- [ ] 审查 `typecheck_fixes.rs` 和 `binding_enhancements.rs`，确定是否删除
- [ ] 审查 `tests/integration/execution.rs` 的 stack overflow 问题

### Phase 1：E2E 测试框架

- [ ] 创建 `tests/yx_runner.rs` — 自动发现并运行 `tests/yaoxiang/**/*.yx`
- [ ] 创建 `tests/yaoxiang/` 新目录结构
- [ ] 编写 00-smoke 冒烟测试
- [ ] 编写 01-basics 层（当前可运行的语法）
- [ ] 编写 02-functions 层

### Phase 2：运行时 bug 修复 + 对应测试

- [ ] 修复 match 表达式（ir_gen 增加 Match 处理）
- [ ] 修复列表推导式（ir_gen 增加 ListComp 处理）
- [ ] 修复 `x: Int = 42` 变量类型标注
- [ ] 为以上修复补充对应的 .yx E2E 测试

### Phase 3：集成测试重写

- [ ] 重写 `tests/integration/interpreter.rs`（验证运行时输出值）
- [ ] 重写 `tests/integration/execution.rs`（修复 stack overflow）
- [ ] 补充集成式 codegen 测试（从源码到 IR）

### Phase 4：内联测试合并

- [ ] typecheck 测试 23→12 合并
- [ ] 删除 `debug_lexer.rs`
- [ ] 审查 parser 测试重复

### Phase 5：创建测试规范文档

- [ ] 在 `tests/yaoxiang/` 根目录创建 `TEST_STANDARDS.md`

---

## 五、验证方式

```bash
# 全部测试
cargo test

# E2E 测试
cargo test --test yx_runner

# 单元测试
cargo test --lib

# 手动运行 .yx 文件
cargo run -- run tests/yaoxiang/01-basics/variables.yx

# benchmark 运行
cargo bench
```

---

## 六、涉及文件清单

### 新建文件
- `tests/yx_runner.rs` — E2E 测试 runner
- `tests/yaoxiang/TEST_STANDARDS.md` — 测试规范
- `tests/yaoxiang/00-smoke/hello.yx`
- `tests/yaoxiang/01-basics/variables.yx`
- `tests/yaoxiang/01-basics/typed_vars.yx`
- `tests/yaoxiang/01-basics/operators.yx`
- `tests/yaoxiang/01-basics/literals.yx`
- `tests/yaoxiang/01-basics/comments.yx`
- `tests/yaoxiang/02-functions/definitions.yx`
- `tests/yaoxiang/02-functions/lambdas.yx`
- `tests/yaoxiang/02-functions/closures.yx`
- `tests/yaoxiang/03-control-flow/if_else.yx`
- `tests/yaoxiang/03-control-flow/while.yx`
- `tests/yaoxiang/03-control-flow/for.yx`
- `tests/yaoxiang/03-control-flow/match.yx`
- `tests/yaoxiang/05-data-structures/lists.yx`
- `tests/yaoxiang/05-data-structures/tuples.yx`
- `tests/yaoxiang/06-modules/imports.yx`
- `tests/yaoxiang/06-modules/lib/math.yx`

### 删除文件
- `tests/yaoxiang/closure_test.yx`
- `tests/yaoxiang/closure_test2.yx`
- `tests/yaoxiang/list_test.yx`
- `tests/yaoxiang/mut_param_test.yx`
- `tests/yaoxiang/mut_param_error_test.yx`
- `tests/yaoxiang/impl_status_test.yx`
- `tests/yaoxiang/spec_basics_test.yx`
- `tests/yaoxiang/spec_features_test.yx`
- `tests/yaoxiang/spec_functions_test.yx`
- `tests/yaoxiang/spec_types_test.yx`
- `src/frontend/core/lexer/tests/debug_lexer.rs`（待确认）

### 修改文件
- `tests/integration/interpreter.rs` — 重写
- `tests/integration/execution.rs` — 重写
- `src/frontend/core/ir_gen.rs` — 修复 match 和 listcomp
- `src/frontend/typecheck/` — 修复 `x: Int = 42`
- `src/frontend/typecheck/tests/` — 合并 23→12 文件
