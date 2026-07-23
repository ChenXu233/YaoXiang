# auto-dev-night 修复报告

**Worktree**: `E:/git/YaoXiang-auto-dev-night` (branch: `auto-dev-night`, from `dev` @ `157da0ed`)
**日期**: 2026-07-23
**状态**: 待用户验收

---

## 修复的 Issue

### Issue #166: TerminationChecker 对 `-> Never` 返回类型的函数应放行循环终止性检查

**问题**: 函数返回类型为 `Never` 时，`while true {}` 循环应被接受——`Never` 表示永不返回，循环不终止是预期行为。但 `TerminationChecker` 仍然报 `E8001` 错误。

**根因**: `TerminationChecker` 对所有函数一视同仁地执行循环终止性检查，未区分返回类型。`Never` 语义上保证发散，循环不终止是类型签名承诺的行为。

**修复**: `src/frontend/core/typecheck/layers/termination.rs`
- 新增 `in_never_function: bool` 字段
- 进入 `-> Never` 函数时置 `true`
- `check_while_loop` 检测到该标志时直接放行

**RFC 依据**:
- RFC-009: `Never` 是底部类型，表示永不返回
- 类型签名 `-> Never` 是发散保证，终止性检查不适用

**用户可见行为**:
- `forever: () -> Never = { while true {} }` → 编译通过 ✅
- `main: () -> Void = { while true {} }` → 仍报 E8001（Void 承诺返回）✅
- 非 Never 函数的循环终止检查不受影响 ✅

**未关闭源 Issue**: 按用户要求，不关闭 GitHub Issue #166，留待用户决定。

---

### Issue #182: Formatter internal error on const generic function signature

**问题**: `yaoxiang format` 对含 const 泛型函数签名的文件报 `Formatter internal error — this is a bug`。

**根因**: `format_fn_signature` 处理 `(N: Int) -> (n: N) -> Int` 这种嵌套 Fn 结构的签名时，未按 curry 层级拆分参数，且输出双层括号 `((n: N) -> Int)` 导致 parser 的 `is_named_curry` 判 false，格式化不幂等。

**修复**: `src/formatter/handlers/expr.rs`、`src/formatter/handlers/stmt.rs`
- `format_fn_signature` 按 `fn_type` 嵌套 Fn 结构递归拆分 `signature_params`
- 每层 curry 用单层括号 `(n: N) -> Int`，保证格式化幂等
- 新增 `format_return_with_names` 递归补全各层参数名
- 新增 `split_params_at` 按层级切分参数

**RFC 依据**:
- RFC-011: const 泛型函数签名语法
- RFC-014: 格式化器幂等性要求

**用户可见行为**:
- `identity_n: (N: Int) -> (n: N) -> Int = (n) => n` → 格式化成功 ✅
- 格式化两次结果一致（幂等）✅

**未关闭源 Issue**: 按用户要求，不关闭 GitHub Issue #182，留待用户决定。

---

### Issue #175: curried 泛型函数 `{ return x }` 块形式报 E1002

**问题**: `map: (T: Type) -> ((x: Int) -> Int) = (x) => { return x }` 报 E1002 类型错误，但 `(x) => x` 表达式形式正常。

**根因**: 类型检查器处理 `return` 语句时，取的是**最外层** curry 函数的参数类型（`T: Type`），而非 `return` 所在的**最内层**函数签名（参数 `x: Int`，返回 `Int`）。

**修复**: `src/frontend/core/typecheck/inference/statements.rs`
- 新增 `innermost_fn_param_types` 沿 curry 嵌套定位最内层函数参数
- 新增 `innermost_return_type` 定位最内层返回类型
- `return` 语句类型检查使用最内层签名

**用户可见行为**:
- `(x) => { return x }` 块形式 → 通过 ✅
- `(x) => x` 表达式形式 → 通过 ✅
- 多层 curry 嵌套的 return → 正确定位到所属函数层 ✅

**未关闭源 Issue**: 按用户要求，不关闭 GitHub Issue #175，留待用户决定。

---

### Issue #183: run 路径缺少 Float→Int 赋值类型检查

**问题**: `f: (x: Int) -> Int`，调用 `f(3.14)` 在 `run` 路径不报错，Float 静默传入 Int 参数。

**根因**: Call 表达式处理器 `if matches!(param_ty, MonoType::TypeRef(_)) { continue; }` 跳过了所有 TypeRef 参数，包括内置 `TypeRef("Int")`。`Int` 内部表示为 `TypeRef("Int")`，导致 Int 参数的类型检查被全部跳过。

**修复**: `src/frontend/core/typecheck/inference/expressions.rs`
- skip 检查前先解析 TypeRef 到具体内置类型
- `TypeRef("Int")` → `Int(64)`，`TypeRef("Float")` → `Float(64)`
- 只有真正未解析的 TypeRef 才跳过

**用户可见行为**:
- `f(3.14)` 传给 `f: (x: Int) -> Int` → `check` 和 `run` 路径都报 `E1002` ✅
- `f(42)` 传给 `f: (x: Float) -> Float` → 通过（Int→Float 扩展转换）✅
- `f(42)` 传给 `f: (x: Int) -> Int` → 通过 ✅
- `f("hello")` 传给 `f: (x: String) -> String` → 通过 ✅

**未关闭源 Issue**: 按用户要求，不关闭 GitHub Issue #183，留待用户决定。

---

### Issue #168: parser 不支持纯函数声明（无 `= value` 的类型标注语句）

**问题**: `do_forever: () -> Never` 形式的纯声明语句（无 `= value`）在 parser 中触发 `E0011 Unexpected token: 'Identifier("main")'`。

**根因**: `parse_assign_after_target` 的 type_annotation 后续检查中，`Identifier` 被视为无效 token。但纯声明（无 `= value`）后跟新语句（Identifier 起头）是合法语法——parser 不应在没有 `=` 的情况下把后续 Identifier 当错误。

**修复**: `src/frontend/core/parser/statements/declarations.rs`
- 仅拦截明确无效的后续 token：`LParen`/`FatArrow`/`Comma`
- 不再拦截 `Identifier`——它可能是下一个顶层语句
- 纯声明的语义由 typechecker 处理，parser 只出结构

**RFC 依据**:
- RFC-010: 统一语法模型 `name: type = value`，但规范未明确禁止省略 `= value` 的纯类型标注声明
- parser 不预判语义——结构合法就放行

**用户可见行为**:
- `do_forever: () -> Never` + 换行 + `main = { ... }` → parser 接受 ✅
- `x: Int` 纯声明 → parser 接受 ✅
- `Point = { x: Float, y: Float }` 无 `: Type` → 仍报错（Comma 拦截）✅
- `x: Int = 42` 正常赋值 → 通过 ✅

**未关闭源 Issue**: 按用户要求，不关闭 GitHub Issue #168，留待用户决定。

---

### Issue #146: 常量表达式求值仅支持FFI符号，其他返回None

**问题**: `eval_const_expr` 仅支持字面量和 FFI 符号求值（LibraryRef/ExternRef），`PI * 2.0`、`1 + 1` 等基本常量表达式返回 None，全局变量初始化为 0。

**根因**: `src/middle/core/ir_gen.rs` 的 `eval_const_expr` 对 `BinOp`/`UnOp`/`Var` 分支直接返回 None（`_ => None`），没有实现算术/逻辑/比较运算和全局常量引用。

**修复**: `src/middle/core/ir_gen.rs`
- `eval_const_expr` 新增 `BinOp` 分支：算术（+,-,*,/,%）、比较（==,!=,<,<=,>,>=）、逻辑（&&,||）、字符串拼接（+）
- 新增 `UnOp` 分支：一元负（-）、正（+）、非（!）
- 新增 `Var` 分支：查找已注册的全局变量常量值
- 新增 `eval_binop` 辅助函数：按类型分派运算
- 新增 `eval_unop` 辅助函数：按类型分派一元运算

**RFC 依据**:
- RFC-027 §编译期求值：全局常量应可在编译期求值
- `ConstValue` 已有 Int/Float/Bool/String/Char 变体，扩展求值器是正确方向

**用户可见行为**:
- `TWO: Int = 1 + 1` → 编译期求值为 2 ✅
- `HALF: Float = 3.0 / 2.0` → 编译期求值为 1.5 ✅
- `PI: Float = 3.14159` + `DOUBLE_PI: Float = PI * 2.0` → 6.28318 ✅
- `C: Int = A + B`（全局常量引用）→ 30 ✅
- `D: Bool = A < B` → true ✅
- `E: Int = -A` → -10 ✅

**未关闭源 Issue**: 按用户要求，不关闭 GitHub Issue #146，留待用户决定。

---

## 已撤出：Issue #181 死代码清理

**原因**: PR #186（`cleanup/dead-code-removal`）已于 2026-07-22 提交，删除 ~7800 行死代码，是本分支 #181 清理（~1097 行）的超集。为避免重复工作，已从 `auto-dev-night` 分支撤出 #181 相关 commit，留待 PR #186 合并。

---

## 验收命令

```bash
# #166 验证
echo 'forever: () -> Never = { while true {} }' > /tmp/test_never.yx
yaoxiang check /tmp/test_never.yx  # 应: Type check passed

# #182 验证
echo 'identity_n: (N: Int) -> (n: N) -> Int = (n) => n' > /tmp/test_fmt.yx
yaoxiang format /tmp/test_fmt.yx --dry-run  # 应: Would format 或无输出
yaoxiang format /tmp/test_fmt.yx  # 应输出格式化结果，不报 internal error

# #175 验证
echo 'map: (T: Type) -> ((x: Int) -> Int) = (x) => { return x }' > /tmp/test_return.yx
yaoxiang check /tmp/test_return.yx  # 应: Type check passed

# #183 验证
echo 'f: (x: Int) -> Int = (x) => x' > /tmp/test_float_int.yx
echo 'y = f(3.14)' >> /tmp/test_float_int.yx
yaoxiang check /tmp/test_float_int.yx  # 应: error E1002 Expected type 'Int', found type 'float64'

# #168 验证
printf 'use std.io\n\ndo_forever: () -> Never\n\nmain = {\n    io.println("OK")\n}\n' > /tmp/test_decl.yx
yaoxiang check /tmp/test_decl.yx  # 应: Type check passed
yaoxiang run /tmp/test_decl.yx    # 应: 输出 OK

# #146 验证
printf 'use std.io\n\nPI: Float = 3.14159\nDOUBLE_PI: Float = PI * 2.0\n\nmain = {\n    io.println(DOUBLE_PI)\n}\n' > /tmp/test_const.yx
yaoxiang run /tmp/test_const.yx  # 应: 输出 6.28318
```
