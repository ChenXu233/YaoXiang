# 所有权检查已知问题

> 最后更新：2026-06-16
> 实现位置：`src/frontend/core/typecheck/layers/ownership.rs`
> 测试位置：`src/frontend/core/typecheck/layers/tests/ownership.rs`
> 61 tests，0 failures

## 正确性缺陷

- [x] ### 1. ref 别名进 spawn 漏标逃逸（P0）— 已修复 (2026-06-15)

**场景**：
```yaoxiang
shared = ref x
alias = shared       // shared Move → alias
spawn { use(alias) } // alias ∉ ref_vars → 漏标逃逸 → 选 Rc（非原子，跨线程不安全）
```

**根因**：`OwnershipChecker` 只追踪 `Expr::Ref` 直接赋值的变量名（`ref_vars`）。当 ref 变量被 Move 给中间变量后，中间变量不改动 `ref_vars`。

**影响**：跨 spawn 使用的 ref 可能错误地编译为 `RcNew`，非原子引用计数在跨线程时可能数据竞争。

**修复**：`StmtKind::Var` 和 `BinOp::Assign` 处理器中，当右侧 `Expr::Var(name)` 且 `name ∈ ref_vars` 时，左侧目标加入 `ref_vars`（commit `9029d5b`）。

- [x] ### 2. spawn 捕获变量 Move 后外层仍可用（P1）— 已修复 (2026-06-16)

**场景**：
```yaoxiang
shared = ref data
spawn { a = shared }  // spawn 体 walk（save/restore）→ shared 在体内 Moved → 外层恢复
use(shared)            // 外层 shared 仍 Alive——正确，但 spawn 体内 shared 已 Move
```

**根因**：`Expr::Spawn` 使用 save/restore，spawn 体内的所有权变更不影响外层。这是正确的设计，但 spawn 体内 `a = shared` 的 Move 只在 spawn 的"临时 walk"中被检测。如果 spawn 体执行了 `shared` 的 Move，save/restore 让外层恢复，**但没有任何东西阻止外层在 spawn 后继续使用 `shared`**。

**影响**：如果 spawn 实际运行时 Move 了 `shared`（如 `a = shared`），外层代码在 spawn 之后仍然可以访问 `shared`——这在 YaoXiang 的并发模型中可能正确（spawn 获取独立副本），但语义未明确定义。

**修复方向**：需要明确语言规范：spawn 捕获的 Move 语义是否影响外层作用域。如果是"spawn 获取独立副本"，当前行为正确。如果是"spawn 消费所有权"，需要去掉 save/restore 或引入类似闭包的 Captures。

## 精度取舍

- [x] ### 3. 分支互斥性保守报冲突（P1）— 已修复 (2026-06-16)

**场景**：
```yaoxiang
if cond {
    a = &mut x   // 分支 A
} else {
    b = &mut x   // 分支 B
}
// 理论上：A 和 B 互斥，不应冲突
// 实际：两个 WriteToken 先后创建 → 报 BorrowConflict
```

**根因**：`NLL without fixpoint` 架构限制——单趟 AST walk 不建模路径条件，无法区分分支互斥还是顺序执行。

**修复方向**：需要 CFG 的 SMT 慢速通道介入（当前 `smt_cut` 已实现但仅在 `while + path_condition` 场景触发）。扩展到 if/else 分支需要 path_condition 传播到 Borrow handler。

- [x] ### 4. ref 类型不识别 Dup（P1）— 已修复 (2026-06-16)

**场景**：
```yaoxiang
shared = ref x
a = shared    // Move——但 ref 理论上是 Dup 类型，应可复制
b = shared    // use after move——实际上应允许
```

**根因**：所有权检查器不知道 `ref T` 是 Dup 类型（可复制引用计数句柄）。`StmtKind::Var` 的 Move 逻辑对所有类型一视同仁。

**影响**：ref 值的语义比预期更严格——不能像 RFC-009 设计的那样"自由复制"。

**修复方向**：需要从 `TypeEnvironment` 查询变量类型，对 Dup 类型跳过 Move 逻辑。这和要求 `clone()` 显式调用的整体设计一致——当前保守行为不比正确语义更宽松。

## 基础设施

- [x] ### 5. 错误码格式未统一（P2）— 已修复 (2026-06-16)

**说明**：`error.rs` 的 `ValueState` 枚举、`TypeId` 结构体、`ValueStateProvider` trait 均为死代码（零使用），已删除。文件重命名为 `state_utils.rs`，保留 `Checker` trait + `operand_display_name` 工具函数。

- [x] ### 6. 嵌套函数有参形式不分析（P2）— 已修复 (2026-06-16)

**说明**：`StmtKind::Binding` 移除了 `params.is_empty()` 条件。有参嵌套函数同样做 save→walk→diff→restore 捕获分析，walk 前注册参数为局部变量。

**说明**：`StmtKind::Binding` 只对 `params.is_empty() && !body.is_empty()` 的闭包做捕获分析。带参嵌套函数返回 `vec![]`（由 `check_module` 独立检查 body，但不分析捕获语义）。

**影响**：带参嵌套函数体内的所有权错误不会被检测（当前直接 skip），也不产生捕获信息。一个带参嵌套函数如果用了外层变量，所有权语义不被分析。

**修复方向**：统一处理有参/无参 Binding，对其 body 同时做 `check_function` + 捕获分析。
