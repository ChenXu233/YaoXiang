---
title: 闭包捕获模型实现设计
status: draft
created: 2026-05-29
---

# 闭包捕获模型实现设计

## 目标

实现闭包捕获外部变量的分析、捕获方式选择、IR 生成。

## 核心规则

```
变量类型    闭包是否逃逸    捕获方式
─────────────────────────────────────────
Dup         任意            复制（零成本，无副作用）
非 Dup      不逃逸          自动借用（&T 或 &mut T 令牌）
非 Dup      逃逸            Move（所有权转移）
```

这套规则和函数调用的自动借用选择是**同一套逻辑**。不引入新概念。

## 实现清单

### Step 1：逃逸分析

**文件**: `src/frontend/core/typecheck/inference/expressions.rs`（或新建 `capture.rs`）

闭包"逃逸"的定义：

```rust
enum ClosureUsage {
    Inline,    // 当场调用或传给同步函数，不逃逸
    Escaping,  // spawn, return, 存堆, 存全局
}
```

逃逸判定规则：

```
lambda 作为 spawn { ... } 的参数      → Escaping
lambda 作为 return 值                 → Escaping
lambda 赋值给外部变量/字段             → Escaping
lambda 传给函数参数（非 spawn）        → Inline（保守）
lambda 当场调用                        → Inline
```

**保守原则**：无法确定时按 Escaping 处理。

### Step 2：捕获变量分析

**遍历闭包体 AST**，找出对闭包外部作用域变量的引用。

```rust
struct CaptureInfo {
    captures: Vec<CapturedVar>,
}

struct CapturedVar {
    name: String,           // 变量名
    usage: CaptureUsage,    // 使用方式
}

enum CaptureUsage {
    Read,           // 只读（只需要 &T）
    Write,          // 读写（需要 &mut T）
    Move,           // 所有权转移（非 Dup + 逃逸）
    DupCopy,        // Dup 类型直接复制
}
```

**分析过程**：

1. 遍历 lambda body 的 AST
2. 记录所有 `Expr::Var(name)` 引用
3. 过滤：只保留闭包外部作用域的变量
4. 按使用方式分类：
   - 赋值/调用 mut 方法 → Write
   - 只读取 → Read
   - 被 Move 到其他地方 → Move

### Step 3：捕获方式选择

```rust
fn determine_capture_mode(
    var: &CapturedVar,
    ty: &MonoType,
    usage: ClosureUsage,
    is_dup: bool,
) -> CaptureMode {
    match (is_dup, usage) {
        // Dup 类型：直接复制——最简路径
        (true, _) => CaptureMode::Copy,
        
        // 非 Dup + 逃逸 → Move
        (false, ClosureUsage::Escaping) => CaptureMode::Move,
        
        // 非 Dup + 不逃逸 → 自动借用
        (false, ClosureUsage::Inline) => match var.usage {
            CaptureUsage::Read => CaptureMode::Borrow,     // &T
            CaptureUsage::Write => CaptureMode::BorrowMut, // &mut T
            CaptureUsage::Move => CaptureMode::Move,
            CaptureUsage::DupCopy => unreachable!(),
        },
    }
}

enum CaptureMode {
    Copy,       // 直接复制值
    Borrow,     // &T 令牌
    BorrowMut,  // &mut T 令牌
    Move,       // 所有权转移
}
```

**关键场景**：

```yaoxiang
# 1. &T 令牌传入闭包——Dup → Copy，零成本
threshold: &Float = &some_float
items.filter(|p| p.x > threshold)
# threshold: &Float → Dup → CaptureMode::Copy
# 编译器：复制令牌（零大小，零运行时开销）

# 2. 非 Dup 值，闭包不逃逸——自动借用
buffer: Buffer = ...
process(|b| b.read())
# buffer 不 Dup，闭包不逃逸 → CaptureMode::Borrow
# 编译器：自动创建 &Buffer 令牌传入闭包

# 3. 闭包逃逸——Move
big_data: Data = ...
spawn { use(big_data) }
# big_data 不 Dup，spawn → Escaping → CaptureMode::Move
```

### Step 4：IR 生成

**文件**: `src/middle/core/ir_gen.rs`

```rust
// 当前（空实现）
Instruction::MakeClosure {
    dst: Operand::Local(result_reg),
    func: closure_name,
    env: Vec::new(),  // ← 永远是空的
}

// 改为
Instruction::MakeClosure {
    dst: Operand::Local(result_reg),
    func: closure_name,
    env: captured_vars,  // Vec<(Operand, CaptureMode)>
}
```

每个捕获变量的 IR 生成：

```rust
for captured in &captures {
    let src = self.lookup_local(&captured.name);
    match captured.mode {
        CaptureMode::Copy => {
            // Dup 类型：Mov 指令复制（零成本优化见 Step 5）
            instructions.push(Instruction::Move {
                dst: new_temp(),
                src,
            });
        }
        CaptureMode::Borrow => {
            // 自动借用：创建 ReadToken
            instructions.push(Instruction::Borrow {
                dst: new_temp(),
                src,
                mutable: false,
            });
        }
        CaptureMode::BorrowMut => {
            instructions.push(Instruction::Borrow {
                dst: new_temp(),
                src,
                mutable: true,
            });
        }
        CaptureMode::Move => {
            // Move：所有权转移
            instructions.push(Instruction::Move {
                dst: new_temp(),
                src,
            });
        }
    }
}
```

### Step 5：ZST 优化——令牌消除

`CaptureMode::Copy` 用于 `&T` 时，`&T` 是零大小类型。`Instruction::Move` 拷贝零字节数据 → **需要在 IR 优化 pass 中消除**。

两种实现方式：

**方式 A：IR 生成时跳过**
```rust
CaptureMode::Copy if is_zero_sized_type(ty) => {
    // 不生成任何 IR 指令
    // 闭包体直接引用外层变量（编译期）
}
```

**方式 B：IR 优化 pass**
```rust
// 新增 ZstElimination pass：
// 扫描所有 Move dst, src，若 src 类型为 ZST，删除该指令
// dst 替换为 src（别名）
```

**推荐方式 A**——在生成时就知道是 ZST，不需要后续优化。

### Step 6：借用令牌冲突检测

闭包捕获 `&mut T` 令牌后，原作用域不能同时使用该令牌：

```yaoxiang
tok = &mut point        # WriteToken 创建
closure = |x| {
    tok.shift(x, 0.0)   # tok 被闭包借用
}
tok.shift(1.0, 0.0)     # ❌ 编译错误：tok 的 WriteToken 已被闭包持有
```

这由已有的令牌冲突检测（RFC-009 v9 2.6 节）覆盖——borrow checker 在流敏感活性分析中处理。

## 文件改动清单

| # | 文件 | 改动 |
|---|------|------|
| 1 | `typecheck/inference/capture.rs`（新建） | 捕获分析 + 逃逸分析 + 模式选择 |
| 2 | `typecheck/inference/expressions.rs` | lambda 类型推断调用捕获分析 |
| 3 | `middle/core/ir_gen.rs` | MakeClosure env 填充，ZST 跳过 |
| 4 | `middle/core/ir.rs` | 可能需要 Borrow 指令（如果 IR 需要） |
| 5 | `middle/passes/lifetime/mod.rs` | 注册闭包相关的借用检查（如果有新检查） |

总改动量估计：~300 行。

## 实现顺序

1. **捕获分析**（capture.rs）——纯 AST 遍历，返回捕获变量列表
2. **逃逸分析**——判断闭包是否逃逸
3. **模式选择**——根据 Dup/非Dup + 逃逸/不逃逸 决定 CaptureMode
4. **IR 生成**——填充 MakeClosure env
5. **ZST 优化**——Dup + ZST 跳过 IR 指令

1-3 是纯类型检查层（前端）。4-5 是 IR 生成层（中端）。可以分开实现。

## 验证场景

```yaoxiang
# ✅ 场景 1：Dup 令牌复制（最核心场景）
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}

# ✅ 场景 2：非 Dup 自动借用
process_buffer: (buf: Buffer) -> Void = {
    transform(|b| b.read())  # buf 不逃逸 → &T 借用
}

# ✅ 场景 3：跨任务强制 Move
spawn_worker: (data: Data) -> Void = {
    spawn { use(data) }  # 逃逸 → Move
}

# ❌ 场景 4：借用 + 后续使用冲突
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf 已被闭包借用
}
```

## 参考

- [RFC-009 v9 所有权模型](../../design/rfc/accepted/009-ownership-model.md) — 借用令牌系统
- [RFC-007 函数语法统一](../../design/rfc/accepted/007-function-syntax-unification.md) — lambda 定义
- 探查报告：IR 生成缺口（MakeClosure env 空实现）
