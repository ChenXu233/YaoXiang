# 中间表示 SSA 化

> **附属设计文档**。本文是 [RFC-039 编译器架构重构](../../rfc/accepted/039-compiler-architecture.md) 的附属文档。四层模型、验收判据分级与执行阶段顺序见 RFC-039 正文;各附属文档的定位见 [本目录索引](index.md)。

## 定位与范围

本文处理 L3 层的 IR 构造纪律，针对 `src/middle/core/ir_gen.rs`（8448 行）提出「每个值唯一定义」的构造方式。**目标不是引入新指令集，而是消除三类失败时不产生任何错误的纪律缺陷**：

1. 6 处分离在数十行之外的手工寄存器 save/restore；
2. `generate_call_expr_ir`（`ir_gen.rs:7269-8014`，746 行）内 6 个分支共享的 `arg_regs` 语义重排；
3. 以 `Span` 为键的跨层契约静默失效。

本文负责：IR 形态（`Operand` / `Instruction`）变更、临时值分配纪律、跨层契约显式化、调用表达式分发拆分、IR 静态校验器的接线方式。

本文不负责：类型表示收敛（[03](03-type-unification.md)）、阶段契约与义务账本（[02](02-stage-contract.md)）、等价性判据的定义与门禁（[07](07-equivalence-oracle.md)）、前端范式（[05](05-frontend-paradigm.md)）、死代码与空头设计清理（[06](06-cleanup-inventory.md)）。本文多处引用这些文档的结论，不重复论证。

**判据类别**按 [07 等价性判据](07-equivalence-oracle.md) 的 **C4（IR 形态变更）** 执行：**行为等价 + IR 结构不变量**，不用 IR 快照全等。SSA 化必然改变 IR 形态，强行用快照会诱导团队放宽判据（07 记录的正是这个陷阱）。

**在 RFC-039 阶段序列中的位置是 P7**（`ir.rs` / `ir_gen.rs` / `bytecode.rs` / `translator.rs`，验收为行为等价 + `verify_ssa` 绿）。本文内部的实施批次编号（批 a ~ d）与 P7 内部的先后顺序是两回事：前者是本文的分批策略，后者是 RFC-039 的全局阶段。

## 可行性前提

**已核实事实。** IR 已经具备 SSA 构造所需的**全部结构性前提**：

| SSA 前提 | 本仓现状 | 位置 |
| --- | --- | --- |
| 显式 CFG | `BasicBlock { label, instructions, successors }` | `src/middle/ir/ir.rs` 的 `BasicBlock` |
| 显式入口块 | `FunctionBody::Code { blocks, entry, locals }` | `src/middle/ir/ir.rs` 的 `FunctionBody` |
| 作用域嵌套信息 | `LocalSlot::scope_depth`（"0 = 函数参数层，供嵌套作用域重名消歧"） | `src/middle/ir/ir.rs` 的 `LocalSlot` |
| 三地址形态 | 76 个变体，54 个带 `dst: Operand`、3 个带 `dst: Option<Operand>` | `src/middle/ir/ir.rs` 的 `Instruction` |
| 每指令 span | 每个变体带 `span: Span` 字段 | 同上 |
| 值类型可标注 | `LocalSlot::ty: MonoType` | `src/middle/ir/ir.rs` 的 `LocalSlot` |
| 槽位计数上界检查 | 取表与 E3014 预算闸单点（`MAX_REGISTERS = 255`） | `src/middle/lower/scope.rs` 的 `take_body_locals` |

**若不具备这些前提，本文的成本估计会完全不同。**

**决定性的两点：**

**其一，SSA 化不是新增能力，是删除一个能力依赖。** 现状下临时槽位**被主动设计为多次定义**——`src/middle/lower/stmt.rs` 的 `generate_function_ir` 与 `generate_block_ir`、`src/middle/lower/expr/closure.rs` 的 `generate_lambda_body_ir` 三处显式回滚 `next_temp`。SSA 化的第一件事是**删掉这三处回滚**，让 `next_temp` 单调递增。删掉之后，**每个临时槽位天然只有一个定义点**——唯一定义从"需要检查的规则"变成"构造方式的数学后果"。批 a 只收敛了保存/恢复面（见「缺陷类 1」），删回滚本身随批 b 的分配器一起做。

**其二，槽位计数上界是硬检查，删回滚会让槽位数失控（实测）。** `src/middle/lower/scope.rs` 的 `take_body_locals` 按 `temp_high_water` 取表并在超过 255 时报 E3014。实测删掉三处回收后跑全语料：2 条语料越过上限触发 E3014——`src/std/tests/string_ops.yx` 354 槽、`tests/yaoxiang/01-syntax/control-flow/list_comp.yx` 323 槽；其余语料的槽位峰值由 51 升到 211。`temp_high_water` 记录历史最高占用，本身不随回滚变化；但回滚消失后分配器不再复用槽位，峰值因此上抬。

**这两点合起来的含义**：lower 已经在维护 SSA 所需的全部记账信息，只是把这些信息**回收掉**了；删回滚要等分配器（7b.4）承担槽位复用。SSA 化的工作量主要在 `generate_call_expr_ir` 的拆分与跨层契约的显式化，不在 IR 表达能力的建设。

### 前置依赖：类型表示单一化

**已核实事实。** `src/middle/core/ir.rs:3`：

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` 另有 `use crate::frontend::core::typecheck::MonoType;`，`LocalSlot::ty`（`ir.rs:656`）用的是 `MonoType`。序列化侧第三套是 `ir::Type`，靠 `bytecode.rs:2353` 的 `impl From<MonoType> for IrType` 搭桥。

即：**IR 里同时存在两套类型表示，且其中一套直接 `pub use` 了 AST 的类型。**

SSA 化要新增 `Phi` 变体，要给每个 SSA 值记录类型（供 07 校验器的"类型一致"不变量），要判断 CFG 汇合处两个前驱的类型是否相容。**如果类型表示尚未收敛，SSA 会为每一套表示各写一份类型兼容判断**——第三套表示就此长成。

这不是"先做哪个更好"的问题，是**先做会产出三套类型兼容逻辑**的问题。[03 类型表示单一化](03-type-unification.md) 必须先完成 `ir.rs:3` 的 `pub use ast::Type` 收敛与 `bytecode.rs:2353` 的桥接拆除。本文改动清单第 6 项明确记录 `ir.rs:3` **本文不动**。

## 现状：四类缺陷

`ir_gen.rs` 不是一个「写得丑但能跑」的文件。它是一个**把正确性押在人的逐行审读上的文件**：

- 文件内 **0 个 `#[cfg(test)]`、0 个 `mod tests`**（实测）。
- `src/middle/core/tests/mod.rs` 声明 `bytecode` / `def_assign` / `local_slots` 三个模块，共 29 个测试——其中 `def_assign.rs` 与 `local_slots.rs` **正向**覆盖 ir_gen 的 DefId 分配与局部槽位命名（哨兵级），`bytecode.rs` 4 处提到 ir_gen（`421` / `423` / `1028` / `1173`）**全部是反向断言**——"ir_gen 不产出"、"ir_gen 也不构造"、"ir_gen 前端不构造对应 IR"。

即：ir_gen 有 29 个哨兵级测试钉住局部行为，但**没有任何 IR 结构不变量校验**（支配性、唯一定义、jump 目标、类型一致）——`middle/core/tests/mod.rs` 声明的三个模块没有一个对 IR 整体形态做正向断言。

而这个文件里恰好有几类**编译成功、检查全绿、程序照跑，只是值错了**的缺陷。它们共同的性质是：**失败不产生错误**。

这就是最高风险所在。不是「函数太长」，不是「命名混乱」，而是**这个文件的核心不变量没有任何机器可执行的强制手段**。

「嵌套函数体生成前保存、之后恢复，否则内层函数的名字会串到外层」这条契约由类型强制：`src/middle/lower/scope.rs` 的 `NestedBodyGuard` 独占 `&mut` 生成器，摘帧与放回各自只有一处实现（见「缺陷类 1」）。

**为什么不只是"加个 verifier 就够了"**：verifier 能**抓住**这些缺陷，但抓不住它们**产生的原因**。原因不是"审读时看漏了"，而是：

> **当前 IR 形态本身允许一个槽位被多次定义，而 `ir_gen` 的临时寄存器分配器主动依赖这一点。**

具体证据是三处语句级寄存器回收（`src/middle/lower/stmt.rs` 的 `generate_function_ir` / `generate_block_ir`、`src/middle/lower/expr/closure.rs` 的 `generate_lambda_body_ir`）——它们在每条语句末尾把 `next_temp` **回滚**到「最后一个具名槽 ⋈ `temp_floor`」的较大者（`self.next_temp = named_watermark.max(self.temp_floor);`）。这意味着同一个 `Operand::Local(n)` 槽位**在同一函数体内被反复写入**，且写入次数、写入顺序、存活区间完全由「回滚时机」这一个手工决策决定。

在这个形态下，「唯一定义」不是一条可以增量施加的规则，而是**与寄存器分配策略直接冲突的**。所以 SSA 化不是给现有代码加个检查，是**换掉那个依赖多次定义的分配策略**。好消息是这个策略的替换成本很低——见上节「可行性前提」。

### 缺陷类 1：多次定义与手工 save/restore（批 a 已收敛 restore 面）

**已核实事实。** 进入嵌套函数体需要保存/恢复的入口一共 7 个。批 a（7a.2）把它们收敛成一个 RAII guard：`src/middle/lower/scope.rs` 的 `NestedBodyGuard<'a>`（`Deref`/`DerefMut` 到生成器，`Drop` 无条件恢复，`#[must_use]`）。

| 入口函数 | 文件 | guard 形态 | 回滚范围 |
| --- | --- | --- | --- |
| `generate_method_ir` | `src/middle/lower/stmt.rs` | `NestedBodyGuard::enter_decl` | 槽位表 + span（三项计数不回滚） |
| `generate_function_ir` | `src/middle/lower/stmt.rs` | `NestedBodyGuard::enter_decl` | 同上 |
| 嵌套函数定义调用点 | `src/middle/lower/stmt.rs` | `NestedBodyGuard::enter` | 槽位表 + span + 三项计数 |
| `generate_lambda_body_ir` | `src/middle/lower/expr/closure.rs` | `NestedBodyGuard::enter` | 同上 |
| `generate_anon_binding_ir` | `src/middle/lower/globals.rs` | `NestedBodyGuard::enter` | 同上 |
| `generate_curry_innermost_func` | `src/middle/lower/curry.rs` | `NestedBodyGuard::enter` | 同上 |
| `generate_curry_function_ir` | `src/middle/lower/curry.rs` | `NestedBodyGuard::enter` | 同上 |

- 摘帧只发生在两个构造函数（`enter` / `enter_decl`），放回只发生在 `Drop`；guard 在生命周期内独占 `&mut` 生成器，作用域里没有第二份 `&mut` 可用，`?`/早退一律经 `Drop` 恢复。
- 涉及的字段：`next_temp` / `temp_high_water` / `temp_floor` / `cur_locals` / `cur_span` / `loop_stack`，由 `src/middle/lower/scope.rs` 的 `SavedFrame` 承载；`next_temp_reg` 继续维护 `temp_high_water` 真高水位（语句级回收会回滚 `next_temp`，槽位总数以历史最高占用为准）。
- `enter_decl` 不回滚三项计数：函数体/方法体回到模块初始化流后，后续定义接着子体的终值编号。这是 C1 快照零漂移的约束，也是「声明面只回滚表」的口径。

**口径统一与预算单点（7a.3 已落地）**：`src/middle/lower/scope.rs` 的 `take_body_locals` 是唯一的取表 + 预算点——按 `temp_high_water` 截取，越界带函数体 span 报 E3014，`MAX_REGISTERS = 255` 全仓一处定义。六个函数体形态（函数 / 方法 / lambda / 匿名绑定 / curry 中间层 / curry 最内层）都经它取表，不存在第二套「函数体该有多少槽」的口径。

**三处重复的语句级寄存器回收仍在（7a.1 拆出批 a，移至批 b，与 7b.4 同批）**：

| 位置 | 所在函数 | 守卫 |
| --- | --- | --- |
| `src/middle/lower/stmt.rs` | `generate_function_ir` | 无 |
| `src/middle/lower/stmt.rs` | `generate_block_ir` | `if result_reg.is_none()` |
| `src/middle/lower/expr/closure.rs` | `generate_lambda_body_ir` | 无 |

`generate_block_ir` 的守卫有明确理由（表达式操作数位的块，外层可能持有跨块存活的兄弟实参临时）。另两处没有等价守卫。**同一段判据在三处手抄，其中一处有守卫两处没有**——这类差异正是 SSA 要消灭的对象。

**为什么拆出**：删回滚会让槽位数失控（见「可行性前提」其二与「已知局限与风险」），需先由线性扫描分配器（7b.4）承担槽位复用。

**后果（今日形态）**：guard 独占 `&mut`，恢复只有 `Drop` 一处实现，漏一次 restore 没有可写的落点；剩下的是「一个槽位多次定义」本身——分配器依赖它，I 层不变量因此无法断言唯一定义，直到批 b 删除回收。

### 缺陷类 2：`generate_call_expr_ir` 的 `arg_regs` 语义重排

**已核实事实。** `generate_call_expr_ir` 位于 `ir_gen.rs:7269-8014`，**746 行**，单函数。函数签名（`7269-7279`）接受 `func: &Expr` / `args: &[Expr]` / `named_args: &[(String, Expr)]` / `span: &Span`（另带 `_expr` / `result_reg` / `instructions` / `constants`），内部 6 个分支共享同一个 `Vec<Operand>` 并做**语义重写**：

| 行 | 语义 |
| --- | --- |
| `7395` | 命名空间调用：按 `slots` 展平后重新 `collect` —— 命名实参重排 |
| `7724` | 结构体构造：按 `final_args` 逐槽 `unwrap` 后重新 `collect` —— 字段重排 |
| `7835-7848` | 函数调用：命名实参重排；`7838-7846` 对缺席槽位**补一个值为 0 的新寄存器**（`next_temp_reg` + `Instruction::Load`） |
| `8000` | `let final_args: Vec<Operand> = arg_regs.clone();` —— 克隆后再发指令 |

四处重排的源码原貌：

```rust
// 7395
arg_regs = slots.into_iter().flatten().collect();
// 7724
arg_regs = final_args.into_iter().map(|s| s.unwrap()).collect();
// 7835-7848
arg_regs = slots
    .into_iter()
    .map(|s| {
        s.unwrap_or_else(|| {
            let r = self.next_temp_reg();
            instructions.push(Instruction::Load {
                dst: Operand::Local(r),
                src: Operand::Const(ConstValue::Int(0)),
                span: self.cur_span,
            });
            Operand::Local(r)
        })
    })
    .collect();
// 8000
let final_args: Vec<Operand> = arg_regs.clone();
```

`7835-7848` 的注释自述：「未覆盖的槽位：保持缺席，由类型检查阶段报 E1010。运行期为安全起见补 0（正常路径不会到这里）。」

**这个设计有两重风险**：

1. **错位不报错只错值。** 6 个分支各自决定 `arg_regs` 的最终顺序，任何一处顺序假设错误都不会 panic。
2. **规范化快照对此免疫。** 07 已记录该局限：快照工具按"首次出现顺序"重命名临时值后，「第 3 个实参本该用第 5 个寄存器」在快照里完全相同。**这一类缺陷在 C4 阶段连快照这一层判据都不存在。**

**补充核实：结构体构造路径的重排无重复槽守卫。** `7724` 的 `final_args` 按"位置实参先入位、命名实参按字段名覆写"填充，**覆写前没有 `slots[idx].is_some()` 之类的重复槽检查**（对比命名空间路径 `7381` 有）——命名实参静默顶掉位置实参时不 panic、不诊断；且实参**求值顺序**（源码序）与最终 `arg_regs` 顺序（字段声明序）在此分叉。批 d 的 `fill_missing` 断言化必须一并覆盖"重复槽"检查，否则结构体构造路径的静默顶替会原样存活。

### 缺陷类 3：span 键控的跨层契约静默失效（**已消除**，2026-10-13 / RFC-039 D20、WBS 4.5）

**已核实事实（审计时点）。** 全仓有**两个** span 键控的跨层契约：

| 契约 | 声明 | 产出 | 传递 | 消费 |
| --- | --- | --- | --- | --- |
| `release_plan` | ~~`ir_gen.rs:191` `HashMap<Span, Vec<String>>`~~ → 现 `HashMap<PlanId, Vec<String>>`（`ownership.rs:39`） | `ownership.rs:31`（`ReleasePlan::drops` 字段）、`build_release_plan`（分派 `walk_stmt` 的语句身份） | `typecheck/types.rs:38` ← `checker.rs` 传递 ← `ir_gen.rs:360`（克隆读点） | `ir_gen.rs:3103`（`ReleasePlan::consume`，唯一入口；结算完成标记 `:1095`） |
| `overload_resolutions` | ~~`ir_gen.rs:222` `HashMap<Span, String>`~~ → 现 `OverloadResolutions`（表 + 消费台账同体，`types.rs:62`） | `inference/expressions.rs:4550`（按调用点 `PlanId` 写；（`inference/statements.rs` 仅透传合并） | `typecheck/types.rs:62` ← `checker.rs` 组装 ← `ir_gen.rs` 字段持有 | `ir_gen.rs:7697`（有绑定）、`7772`（无绑定）（`OverloadResolutions::get`；结算完成标记 `:1097`） |

审计时点的形态（`ir_gen.rs:1942`，**现已不存在**）：

```rust
// NLL Release: 在语句边界插入 Drop 指令
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

`if let Some` —— **查不到就是查不到，没有 else 分支、没有断言、没有计数**。两侧 span 计算方式任何一处不一致 → **Drop 指令静默消失，零错误**。精化类型（refinement type）的 Drop 序列是本项目所有权语义的核心载体（`ownership.rs:26-32` 自述「NLL 精确释放计划」），而它的落地曾是静默的。

`7420` / `7493` 同款形态（`if let Some(mangled) = self.overload_resolutions.get(span)`），后果是重载决议回退到默认绑定。

**处置（4.5 落地）**：两条键均改为 `PlanId` 结构身份，消费**命中即记账**（读与记账同一次调用），差集非空在唯一结算点报 **E 级 `W1090`**——「查不到」不再是静默。落地形态与判据见 [02](02-stage-contract.md) §2 修订注记（4.5）；本行号（`1942` / `7420` / `7493`）保留为审计时点证据，不作为现状指针。

> **`method_def_ordinals` 不是 span 键控。** 它是 `HashMap<String, usize>`（`ir_gen.rs:224`），在 `:369` 初始化，唯一写入点 `:1692-1702`（按 `base_name` 的定义序分配 `#N` 后缀）。它的风险是**另一种**：定义序依赖——`#N` 后缀由 AST 定义顺序决定，若生成顺序与 typecheck 注册序不一致，同一函数名会解析到不同目标。这一条不是 span 失配，但同样"错了不报错"，处置见「span 键控的额外机制」的机制三。

### 缺陷类 4：spawn/lambda 隐式握手

**已核实事实。** `ir_gen.rs` 有**两处**自己构造 `ast::Expr::Lambda` 并回调 `generate_expr_ir`，靠一组可变字段握手：

**握手 A —— `generate_spawn_for_ir`（入口 `4143`）：**

```
4245    self.pending_env_vars = vec![Operand::Local(element_reg)];
4246    let lambda = ast::Expr::Lambda { params: ..., body: ..., span };
4256    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
```

`generate_expr_ir` → `generate_lambda_expr_ir`（入口 `6543`）→ `6571-6572` `std::mem::take(&mut self.pending_env_vars)` 消费。

**握手 B —— `generate_spawn_expr_ir`（入口 `6641`）：**

```
6698    self.pending_env_vars = env_ops;
6699    self.pending_env_names = env_names;
6701    let lambda = ast::Expr::Lambda { params: Vec::new(), body: Box::new(ast::Block { stmts: vec![ast::Stmt { ... }] }), span };
6712    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
6713    self.pending_env_vars.clear();
6714    self.pending_env_names.clear();
```

**握手 C —— `closure_captures`**：写于 `6589-6593`，**在同一个函数里** `6598` 调用 `generate_lambda_body_ir`（入口 `4678`），`6600` 清除。读点有两处：`:6284`（闭包体内 `LoadUpvalue`）与 **`:458`**（`flatten_namespace` 路径上的 `closure_captures.contains_key(head)`）。

`:458` 这个读点最危险：它在**命名空间解析**路径上，意味着一个 `HashMap<String, usize>` 的存活状态会改变变量名解析结果。`6589` 写、`6600` 清之间隔着一次完整的闭包体生成（`6598`）；若 `generate_lambda_body_ir` 内部任何一条路径提前返回而不清表（它在 `?` 运算符传播错误时有多个返回点），**`closure_captures` 就会残留到后续无关代码的 `:458`**。

同类隐式状态还有 `pending_env_names`（B 握手写于 `6699`，`6714` 清）。

### SSA 不会解决的（诚实列出）

| 问题 | 位置 | 为什么 SSA 不解决 |
| --- | --- | --- |
| `compile_pattern` 复杂度 | `ir_gen.rs:5390-5687`，**298 行**，自递归 4 次（`5473` / `5504` / `5619` / `5640`），外部调用点 `5270` | 模式匹配（union 载荷 / 元组 / 结构体 / 字面量 / 嵌套）的分派复杂度是**算法固有**的。SSA 只把守卫产生的 `JmpIfNot` 变成 `Phi`，不减少守卫数量 |
| `eval_const_expr` 复杂度 | `ir_gen.rs:2298-2488`，**191 行**（`2297` 有 `#[allow(clippy::only_used_in_recursion)]`，确认自递归） | 常量折叠的表达式覆盖度与 SSA 无关 |
| span 键控失效（**已消除**） | 审计时点：`release_plan`（`1942`）、`overload_resolutions`（`7420` / `7493`）——4.5 后为 `PlanId` 键 + 结算必达 | **SSA 完全没有触及这一层**（结论不变）。已由[02](02-stage-contract.md) 的义务账本独立机制解决，见「span 键控问题的处置」机制二 |
| `Instruction` 变体数 | 75 个（`ir.rs` 的 `Instruction` 枚举；含 7b.1a 引入的 `Phi`） | SSA 增加 `Phi`，**净变化 +1**（已落地）。变体对应字节码 opcode 或降级为 NOP，不会因 SSA 减少；`Phi` 按方案 A 是 IR-only 伪指令，不新增 opcode |
| `method_def_ordinals` 定义序依赖 | `ir_gen.rs:224` / `369` / `1692-1702` | 与 span 键控同类但不同因，见「span 键控的额外机制」机制三 |

## 目标设计

### SSA 形态定义

**设计判断。** 目标形态如下。

#### `Operand` 变体如何改

`ir.rs` 的 `Operand` 枚举共 5 个变体。**全仓实测**（命中数为 `git grep -c` 按文件求和，`src/` 全量含测试；构造点数为值位置构造）：

| 变体 | 全仓实测 | 处置 |
| --- | --- | --- |
| `Const(ConstValue)` | 35 个文件 / 148 处命中 | **保留**，不变 |
| `Value(ValueId)` | 7 个文件 / 26 处命中；7b.1a 引入时**生产构造点 0**，7b.2 起 `next_temp_reg` 为唯一生产入口（`src/middle/lower/scope.rs`） | **保留**（7b.1a 引入） |
| `Local(usize)` | 29 个文件 / 417 处命中 | **保留但拆分语义**（见下） |
| `Arg(usize)` | 8 个文件 / 11 处命中 | **保留**，作为函数入口的隐式定义（参数即块 0 的定义） |
| `Global(usize)` | 16 个文件 / 26 处命中 | **保留**，不变 |
| `Temp(usize)` | **0 个构造点**（处理臂只在 `codegen/operand.rs` 的 `OperandResolver::to_reg` 与 `is_register`，随变体一并删除） | **已删除**（死变体，7b.1a） |
| `Label(usize)` | **0 个构造点** | **已删除**（死变体，7b.1a） |
| `Register(u8)` | **0 个构造点**（原变体声明处注释称 "Added for codegen"，但 codegen 走的是 `codegen/operand.rs` 的 `OperandResolver`，不构造该变体） | **已删除**（死变体，7b.1a） |

`Local(usize)` 拆分为两个变体，因为当前它**同时承担两种互斥语义**——「具名变量的存储位置」（可多次写）与「编译器临时寄存器」（应只写一次）：

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA 值：函数内稠密编号，唯一定义
    Value(ValueId),
    /// 具名局部变量的存储槽：可寻址、允许多次定义（非 SSA 部分）
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` 为 `u32` 新类型（`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`），函数内稠密编号，**单调递增、永不回滚**。拆分与 `ValueId` 已由 7b.1a 落地（`src/middle/ir/ir.rs` 的 `Operand` 枚举）；生产端（`next_temp_reg` 改产 `Operand::Value`）已由 7b.2 落地（见下「值标识方式」的生产形态）。

#### `Phi` 作为新 `Instruction` 变体

```rust
/// 汇合点：每个前驱基本块提供该值的一个定义
/// incoming 按前驱 block label 升序排列（构造时保证，便于规范化）
Phi {
    dst: Operand,                                  // 必为 Operand::Value
    incoming: Vec<(usize, Operand)>,               // (前驱 label, 来自该前驱的值)
    span: Span,
},
```

三条构造约束（见 [07](07-equivalence-oracle.md) 第一层）：

1. `incoming.len() == 该块前驱数`。
2. `incoming` 的 label 集合 == `blocks` 中所有 `successors` 含本块 label 的那些块的 label 集合。
3. 所有 `incoming` 值的 `MonoType` 相同（**依赖 03 的类型表示已收敛**）。

`verify_ssa` 的落地状态（7b.1b 已补齐前三条）：约束 1 与 2 需要值表与块级 CFG，已于 **7b.1b** 落地为「块级前驱数」与「前驱 label 集合」两条判据——前驱集从 `BasicBlock.successors` 反推（`src/middle/ir/verify.rs` 的 `block_predecessors`），计数不等只报计数、集合不等才报集合（同一缺陷不双报）；约束 3 的逐输入类型一致同期落地，取值来自值表 `values[v].ty`，**复用既有的 `types_compatible`**（未引入第三套类型兼容判断；03 的类型表示收敛仍是「前置条件（硬性）·前置二」，届时 `ValueInfo::ty` 仍是 `MonoType`）。三条判据的违规类别均为 `InvariantKind::PhiConsistency`。

**类型不可得的三种形态（显式登记，不静默放行）**：值表为空 ⇒ 判定依据不存在（模块 init / 顶层语句的恒常形态，`value_mode=false` 路径）；值表非空而 `ValueId` 越界 ⇒ 报违规；非 `Value` 操作数（`Const` / `Local` / `Arg` / `Global`）⇒ 无值表类型源，不判定。

**实测前提（已于 7b.3 兑现）**：7b.3 之前全仓 `BasicBlock.successors` 的生产构造全部是 `Vec::new()`（块级前驱恒空，前驱数与前驱集合两条判据当时由合成夹具承担）。**7b.3 起块级 `successors` 由 `src/middle/ir/ir.rs` 的 `partition_blocks` 在每条生产构造点真实填充**（`lower` 6 处 + codegen 合成构造函数体 1 处全部走该唯一入口；`lower` 产物的循环 / 分支块有多块、双出边与回边），故前驱数与前驱集合两条判据在真实 IR 上有数据——真 lowering 产物上的直驱测试钉住该登记-消费闭环。`Phi` 的生产构造点仍为 0（生产端归后续批次），落地时直接受本判据约束。

7b.1a 只引入变体：lowering 生产构造点为 0（至今仍为 0，生产端归后续批次）。`translator.rs` 的消费面已于 **7b.3 落地**：方案 A 在块入口按前驱展开为 `Move` 串（组落在汇合块入口、组间既有 `JMP` 跳过、入边跳转按前驱重定向到组起点），`translate_instruction` 的 `Phi` 臂是不可达 E8001 防御臂（`executor` 永远看不到 `Phi`）。

**处置 `3632-3638` 的守卫问题**：现状 `generate_block_ir` 的回收有 `if result_reg.is_none()` 守卫（`3626-3631` 注释：表达式操作数位的块，外层可能持有跨块存活的兄弟实参临时）。SSA 化后该守卫**不再需要**——因为临时值不再回滚，跨块存活的兄弟实参持有的是自己的 `ValueId`，不会被回滚波及。**这一条是 SSA 化能真正简化代码的地方，而不只是改名。**

#### 值标识方式

**设计判断**：不做内存化（mem2reg），不做虚拟寄存器编号，**只做显式重命名**。理由：

- YaoXiang 的 `&` / 引用 / 所有权语义使"哪些槽位可内存化"是一个**类型系统问题**，不是 CFG 问题。已核实 `ir.rs` 的 `LocalSlot::ty: MonoType` 携带完整类型信息，但判定"该 `MonoType` 是否可被内存化"需要 03 收敛后的类型表示与精化类型规则的配合。**在 03 之前做 mem2reg 就是在猜。**
- 显式重命名只需改 `next_temp_reg()` 一处（`src/middle/lower/scope.rs` 的 `next_temp_reg`）的返回值语义 + 三处回滚的删除，**不触碰任何 lowering 算法的控制流**。

值表形态（**7b.1b 已落地**）：`FunctionBody::Code` 增加 `values: Vec<ValueInfo>`，其中

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// 调试名（来自被消除的具名槽位），None 表示纯临时值
    pub debug_name: Option<String>,
    pub defining_block: usize,   // 定义所在基本块的 label
}
```

访问器与既有访问器同形（`src/middle/ir/ir.rs` 的 `impl FunctionIR`）：`values()` 只读、TypeDecl 上返回空表（镜像 `locals()`）；`values_mut()` 可变、TypeDecl 上 `panic!`（镜像 `blocks_mut()`）。

**生产形态（7b.2 已落地）**：`next_temp_reg()` 返回 `Operand::Value(ValueId)` 并在当前函数体的值表登记一条 `ValueInfo`（`src/middle/lower/scope.rs` 的 `next_temp_reg`）。逐项落地形态：

- **计数器隔离**：`ValueId` 由独立计数器 `next_value: u32` 发放，每函数体从 0 起算、单调递增、永不回滚；`NestedBodyGuard` 保存/还原它与值表（每个函数体一套）；三处语句级回收只回滚 `next_temp`（槽），不触碰 `next_value`；柯里层循环逐层重置（每层一个 `FunctionIR`）。模块 init / 顶层语句没有值表——`value_mode=false` 时 `next_temp_reg` 退化为旧的 `Local` 槽分配，init 序列零变化。
- **ty 来源**（只取 lowering 已有信息，不新造类型判定器）：字面量经 `ConstValue::mono_type()`（与 verify 的常量类型映射收敛为单一来源）；变量/调用经 `generate_expr_ir` 入口查类型检查结果与绑定函数返回类型；比较/逻辑/字符串族/前向传播在函数体收尾按指令语义补齐。不可得落**显式未知 `MonoType::TypeRef("?")`**（不静默 `Int(64)` 占位，`LocalSlot.ty` 一律不读）。232 份快照实测：9425 条值中真实类型 7080、显式未知 2345（24.9%），未知集中在调用/字段/索引/容器构造形态。
- **defining_block**：取生成期当前块 label；**7b.3 起块切分已落地**（真 lowering 的 while 体为多块），但本字段的取值仍是生成期 `cur_block_label`（当前恒 0）——与真实定义块暂不一致；更正须与 232 份快照 `values` 段再生同批，登记给 `Phi` 生产批次 / 7b.4。
- **debug_name**：全 `None`（具名变量仍是 `Local`，不迁值表）。
- **索引**：`value_slots: Vec<usize>`（index = ValueId）与 `values` 同索引，记录 lowering 既有临时槽分配——值表不含槽号而字节码必须落 `u8` 槽，过渡映射使 codegen 还原同一字节码；线性扫描分配器（7b.4）落地后重算本表。

`values` 与 `locals` 并存：`locals` 承载可寻址具名变量（可能多次定义），`values` 承载单定义 SSA 值。**`locals` 的存在不违反 SSA**——它就是 SSA 之外的"存储"部分（LLVM 的 alloca 模型）。

**与 `LocalSlot::origin` 的归口**：两者是**两个维度**，不得混用——`LocalSlot::origin`（origin 侧 2.1.4 落的**槽位身份**字段，记录该槽的生成帧 `FrameId`）回答"这个槽属于哪一帧"，`ValueInfo.defining_block` 回答"这个 SSA 值定义在哪个基本块"。值身份归 `ValueInfo`，槽身份归 `LocalSlot::origin`；7b.1b 只加值表，**不改 `LocalSlot::origin`**（实测零改动）。

#### 改动量估算

| 项 | 估算 | 依据 |
| --- | --- | --- |
| `next_temp_reg` 改造 | ~15 行 | `src/middle/lower/scope.rs` 单函数 |
| 删除三处语句级回收 | −15 行 | `1954-1959` / `3632-3638` / `4737-4742` |
| 6 处 save/restore 收敛为 RAII | 净 ±0 行 | 见「逐项处置表」 |
| `ir.rs` 结构变更 | +120 ~ 180 行 | `ValueId` / `Phi` / `ValueInfo` / 值表；**7b.1a 落前两项、7b.1b 落后两项**（实测：`src/middle/ir/ir.rs` 7b.1a +37/−3、7b.1b +36/−0 ⇒ 累计 +73/−3；`normalize.rs` +22/−5；`verify.rs` 7b.1a +63/−14、7b.1b +162/−13 ⇒ 累计 +225/−27） |
| `translator.rs` 加 `Phi` 臂 | +25 ~ 40 行（待 7b.3） | 现有 50 个 `translate_*` 之一 |
| `executor/` | **零改动** | 方案 A：`Phi` 是 IR-only 伪指令，在 `translator.rs` 展开为 Move 串，不进字节码（决策三） |
| 临时值改产 `Operand::Value` 的调用点 | 机械替换（待 7b.2） | **已实测**（`git grep -c` 按文件求和，`src/` 全量含测试）：`Operand::Local(` 共 415 处 / 28 文件，其中 `src/middle/lower/` 332 处 / 14 文件；临时值的改产点是单点 `next_temp_reg`。**7b.2 实测（基线 revision 复测与落地净量）**：`next_temp_reg()` 调用点 140 处 / 14 文件、`result_reg: usize` 参数链 35 个函数 + 3 个 `Option<usize>` 结果位、`src/middle/lower/` 内 `Operand::Local(` 342 处；代码笔对 `src/` 合计 +23319/−12495，其中 232 份 IR 快照再生占 +21866/−11950，源码与测试净 +1453/−545（机械替换与 normalize 渲染、mono 投影、六条直驱测试；另含 5 份错误码生成物随笔入库） |

### 逐项处置表

| 缺陷类 | SSA 是否消除 | 处置方式 | 判据 |
| --- | --- | --- | --- |
| **1. 多次定义与手工 save/restore** | **部分（restore 面已收敛；唯一定义待 SSA）** | 7 个嵌套函数体入口已收成 `src/middle/lower/scope.rs` 的 `NestedBodyGuard<'a>`（`Drop` 恢复 `cur_locals` / `loop_stack` / `cur_span` 与三项计数），"忘记恢复"变成编译错误；三处语句级回收保留到批 b（7a.1 与 7b.4 同批），`next_temp` 仍可回滚 | 07 第一层「内层隔离」不变量 + `verify_loose`/`verify_ssa` |
| **1b. 口径不一致** | **是（已落地）** | 统一为 `src/middle/lower/scope.rs` 的 `take_body_locals`：按 `temp_high_water` 截取，E3014 预算检查与取表同点、带函数体 span，六个函数体形态全覆盖；`MAX_REGISTERS` 全仓一处定义 | 语料差分 + 快照 zero-diff（已实证） |
| **1c. 三处语句级回收判据** | **是（待批 b）** | 三处回收仍在，删除随批 b 与 7b.4 同批执行（先有分配器承担槽位复用）；`generate_block_ir` 的 `result_reg.is_none()` 守卫同删 | C4：行为等价 + 不变量 |
| **2. `arg_regs` 语义重排** | **部分** | SSA 让"用错寄存器"变成"用错值 ID"且可被支配性检查抓住，但**重排逻辑本身仍在**。真正的消除靠函数拆分——6 个分支各自 `push`，`arg_regs` 所有权留在函数顶部 | 第一层支配性不变量（快照对此免疫） |
| **3. span 键控失效** | **否** | SSA 完全不触及。见「span 键控的额外机制」 | 07 的 `test_release_plan_spans_consumed` |
| **4. 隐式顺序依赖** | **否** | 见「`synth.rs` 边界」 | 需新增显式判据 |
| `compile_pattern` 298 行 | 否 | 见「SSA 不会解决的」 | — |
| `eval_const_expr` 191 行 | 否 | 见「SSA 不会解决的」 | — |
| `Instruction` 75 变体 | 否（+1，已落地） | `Phi` 已由 7b.1a 引入；方案 A 下它是 IR-only 伪指令，不新增 opcode | — |

### 寄存器分配器：自研还是用现成 crate

**已核实事实（决定性）**：

1. `Operand::Register(u8)` **全仓 0 个构造点**——该变体已不在 `ir.rs` 的 `Operand` 枚举中（7b.1a 删除）。
2. 后端不是寄存器机——`src/middle/passes/codegen/operand.rs` 的 `OperandResolver` 把 `Operand::Local` 与 `Operand::Arg` 解析为 `u8`，上限 255（超出即 E3014）；`Operand::Value` 由 7b.2 的过渡映射 `value_slots` 解析为既有临时槽（`>255` 报 E3014、无映射报 E3017 且带指令 span），线性扫描分配器（7b.4）接管后重算该映射。
3. `translator.rs`（`src/middle/passes/codegen/translator.rs`，1577 行，50 个 `translate_*`，函数体分布在 `101-1558`）的输出是 `BytecodeInstruction::new(opcode, operands)`，opcode 来自 `src/backends/common/opcode.rs`（83 个常量）。
4. `src/backends/interpreter/executor/ops/` 是解释器的 opcode 分派族。

**结论**：所谓"寄存器"是**操作数栈槽位索引，不是物理寄存器**。目标机**没有有限的寄存器文件、没有寄存器压力、没有溢出/分裂需求**。

**评估三种方案**：

| 方案 | 评估 |
| --- | --- |
| `regalloc` / `regalloc2`（rustc 血统） | **不采用。** 两者都以 rustc 的 `Function` trait + MIR 的 `Place`/`Operand` 抽象为输入。为 YaoXiang IR 写适配层需实现完整 trait（CFG、liveness、spill slot、scratch），**500+ 行适配代码**换零收益——因为没有真实寄存器文件可着色。且 rustc 自身在此处也只做线性扫描（`enable-llvm` 关闭时） |
| `graph-alloc` 系列 | **不采用。** 同上，且 API 更不稳定 |
| **自研线性扫描（Chaitin-Briggs 简化）** | **采用。** YaoXiang 实际需要的是：按 `values` 顺序把每个 `ValueId` 映射到一个不冲突的 `u8` 栈槽（`OperandResolver` 已有 `u8` 契约与 E3014 上限）。这本质是**活跃区间不重叠的最大团着色**，在无寄存器文件约束下退化为"按定义序首次可用槽位分配" |

**设计判断**：自研，目标 500-1000 行（含 `values → Reg` 映射、活跃区间计算、与 `OperandResolver` 的接口）。**同时必须承认：如果 `u8`/255 的槽位上限在真实负载下不够，自研分配器要处理溢出，而现有代码已在 `1992-2000` 把它当错误处理。** 这意味着本文**不解决**"大函数寄存器不够"的问题——**那是 `compile_pattern` 递归深度的下游后果，属另一个议题。**

`Cargo.toml:37-39` 的 dev-dependencies 仅 `criterion` / `proptest`；`[dependencies]` 中无 regalloc / petgraph / graphlib——引入任一 crate 都是新增依赖面。

### `generate_call_expr_ir` 746 行的拆分策略

**必须最后做**，且**必须先有 SSA 形态与判据**。理由：拆分本身是 C1（纯搬移，判据为快照 zero-diff），但只有在 `arg_regs` 的语义被 SSA 消除到"每个实参一个独立 `ValueId`"之后，拆分才不会把旧的顺序假设原样搬进新文件。

**核心设计约束：`arg_regs` 所有权留在函数顶部，6 个分支只 `push` 不重排。**

```
generate_call_expr_ir (746 行)
├── 参数求值：按源码顺序求值，push 到 arg_regs —— 6 个分支共用，唯一允许 append 的地方
├── CallArgs（值对象）
│   └── arg_regs: Vec<Operand>  —— 只在顶部创建，6 个分支只 push，绝不重新 collect/reorder
├── 分支 1 → emit_namespace_call(args: CallArgs)         [7395 分支抽出]
├── 分支 2 → emit_bound_call(args: CallArgs)              [7406+ 分支抽出]
├── 分支 3 → emit_struct_ctor(args: CallArgs)             [7724 分支抽出]
├── 分支 4 → emit_plain_call(args: CallArgs)              [7835-7848 分支抽出]
├── 分支 5 → emit_curried_call(args: CallArgs)
└── 分支 6 → emit_method_call(args: CallArgs)             [7420 / 7493 分支抽出]
```

`7835-7848` 的"补 0"逻辑（`7838-7846`）下沉为 `CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`，**并把"正常路径不会到这里"的自述注释换成一条断言**：若任何槽位缺席且 typecheck 未报 E1010，直接返回诊断。**这一条把一个静默兜底变成硬错误，是本文对缺陷类 2 最直接的改善。**

**判据**：纯搬移部分（SSA 形态未变）用快照 zero-diff；`fill_missing` 的断言化属于 C4，用语料差分 + 第一层校验器。

### `synth.rs` 边界：把隐式回调变成显式的合成 AST 边界

**已核实事实**：`ir_gen.rs` 中构造 `ast::Expr` 的位置**恰为 2 处**——`:4246` 与 `:6701`（另有 `:6704` 构造 `ast::Stmt`、`:510` / `:1266` / `:1399` 构造 `ast::Type`）。

**提案**：新建 `src/middle/lower/synth.rs`，作为**全仓唯一**允许在 L3 层构造 `ast::Expr` / `ast::Stmt` / `ast::Type` 的模块。理由不是"整洁"，而是：

> 构造 AST 意味着**重新进入前端 lowering 入口**。而 `generate_expr_ir` 期望的 `pending_env_vars` / `closure_captures` 状态是**调用方负责准备的前置条件**，而这个前置条件目前没有任何类型或签名表达——它靠"谁调用我"这个运行时事实成立。

改成显式签名：

```rust
// src/middle/lower/synth.rs
pub struct SynthEnv {
    pub env_vars: Vec<Operand>,
    pub env_names: Vec<String>,
    pub captures: HashMap<String, usize>,
}

/// 合成一个捕获 env 的闭包 AST；捕获集由参数显式给出，
/// 不依赖调用方对 self.pending_env_* 的事先写入
pub fn synth_closure(
    params: Vec<ast::Param>,
    body: ast::Block,
    env: SynthEnv,
    span: Span,
) -> ast::Expr
```

具体改动：

| 位置 | 现状 | 改为 |
| --- | --- | --- |
| `ir_gen.rs:4245-4256` | 写 `pending_env_vars` → 造 Lambda → 回调 | 调 `synth::synth_closure(...)` 得 AST，显式传 `SynthEnv`；`generate_lambda_expr_ir`（`6543`）增加一个携带 `SynthEnv` 的内部入口，**不再读 `self.pending_env_vars`** |
| `ir_gen.rs:6698-6714` | 写 → 造 → 回调 → 清 | 同上；`6713-6714` 的 `clear()` 消失（无共享状态可清） |
| `ir_gen.rs:6571-6572` | `mem::take(&mut self.pending_env_vars)` | 改为从入参读取 |
| `ir_gen.rs:6589-6593` / `6600` | 写 / 清 `closure_captures` | 同上；`closure_captures` 降为**闭包体生成期间的局部变量**，`6600` 的 `clear()` 消失 |
| `ir_gen.rs:458` | `self.closure_captures.contains_key(head)` 在命名空间解析路径上读共享字段 | **这是最危险的一处**：改为显式参数，否则 `generate_lambda_body_ir` 的任一 `?` 提前返回都会让捕获表泄漏到无关代码 |

**判据**：新增哨兵测试 `test_no_ast_construction_outside_synth`，扫 `src/middle/**` 除 `synth.rs` 外不得出现 `ast::Expr::` / `ast::Stmt {` 构造语法。形态同 RFC-039 的 `check-module-boundary.py`。

### span 键控问题的处置（SSA 解决不了的部分）

**设计判断。** 三个独立机制，按成本排序：

**机制一：消费覆盖率断言（必做，成本最低；2026-10-13 已由更强形态取代）。**
07 已定义 `test_release_plan_spans_consumed`（现按 `PlanId` 结算）。本文的补充原是把它做成**运行期计数**而非仅测试；落地形态比该设想更强——**消费台账（`ConsumptionLedger<K>`）+ 唯一结算点（E 级 `W1090`）**，不新增诊断码，也不靠 ir_gen 内部计数器自证：

```rust
// ir_gen.rs:341 附近
// 记下 typecheck 交了几条计划
self.release_plan_total = type_result.release_plan.drops.len();
// 每次 :1942 命中时 self.release_plan_hit += 1;
// generate_module_ir 尾部（assign_defs 之后）：
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* 新诊断码：E3xxx NLL 释放计划未被 IR 消费 */);
//   }
```

**为什么这能工作**：`drop()` 指令对非 `ref` 局部是运行时可观察的（析构副作用），但**"该 drop 的没 drop"在 YaoXiang 当前值语义 + Arc/Rc 模型下未必立刻崩溃**——这正是它静默的原因。计数把"静默"变成"可检测"。

**机制二：Span 键改为 `DefId` 或显式计划 ID（**2026-10-13 已落地**：RFC-039 D20 / WBS 4.5 按本机制实现为 `PlanId(u32)` 结构身份键 + 消费必达结算，落地形态见 [02](02-stage-contract.md) §2 修订注记）。**
根本修法是**不要用 Span 当键**。`Span` 是源码位置，会因宏展开、`include`、多文件合并而漂移。`FunctionIR` 已有 `def: Option<DefId>` 字段（`ir.rs:677`），说明 DefId 在 IR 层可用。**设计判断**：为 `ReleasePlan` 与 `overload_resolutions` 引入 `PlanId(u32)`，由 ownership / overload 决议在产出时分配，IR 消费时按 `PlanId` 匹配。**这属于 02 的义务账本（`Obligations`）范畴，应与 [02](02-stage-contract.md) 一并实施，不放在本文。**

**机制三：`method_def_ordinals` 的定义序依赖（独立小项）。**
该字段是 `HashMap<String, usize>`（`ir_gen.rs:224`），key 为 `"{type_name}.{method_name}"`（**无模块限定**，`:1691-1700` 写入）——两个模块若定义同名 `Type.method`，共享同一计数器，`#N` 混编名随之漂移。**它的失效模式不是 span 失配而是顺序/重名失配**——若 typecheck 的注册序与 ir_gen 的定义序不一致，同名方法会解析到不同目标。处置：改为由 typecheck 侧**一次性产出** `HashMap<DefId, String>`（裸名或 `#N` 混编名），ir_gen 只查不改。**这消除 `ir_gen` 内的一个可变跨函数状态。**

### 三条硬编码丢弃的跟踪

**已核实事实。** `impl From<BytecodeFile> for BytecodeModule`（`bytecode.rs:943-2350`，1408 行）中有三处硬编码丢弃：

| 位置 | 字段 | 值 | 后果 |
| --- | --- | --- | --- |
| `bytecode.rs:2312` | `upvalue_count` | `0` // Not stored in BytecodeFile | 闭包 upvalue 数量在 `.42` 往返后归零 |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet | **`.42` 产物丢失整个异常表** |
| `bytecode.rs:2341` | `globals` | `Vec::new()` // Not stored in BytecodeFile yet | **`.42` 产物丢失全局变量信息**（注意 `2347` 的 `global_names` **有**被填充，两者不一致） |

**设计判断（2026-10-05 修订，决议 D52）：三处全部收进本文范围，作为 P7 批 e（7e），不再立"独立 issue"。** 原判断（后两条立独立 issue）的理由记录如下，以及为什么被推翻：

1. ~~它们是**格式层缺口**（`BytecodeFile` 没序列化这些字段），与 SSA 无关。~~ 成立，但不构成排除理由——P7 本来就动 `bytecode.rs`，且三处同修只需一次 `VERSION` 升版（4→5），拆成两次升版反而更贵。
2. ~~修它们需要改 `.42` 落盘格式 → 升版 → 范围从"IR 形态"膨胀到"产物格式"。~~ 升版机制已存在（`MAGIC` + `VERSION` 头，读取侧按版本拒绝），且 `.42` 是构建产物，跨版本兼容不是目标——膨胀的是工作量，不是风险。
3. ~~`2315` / `2341` 实际风险等级低，`.42` 直跑是次要路径。~~ **被推翻**：异常表丢失使 throw/try 在 `.42` 直跑时行为错误——throw/try 是语言核心语义；次要路径不等于可以静默错。本次重构的原则是**不留核心功能的待实现遗留**（`Vec::new() // Not implemented yet` 正是这类遗留）。

**同时保留**：三处仍必须在 `Obligations` 账本里登记为"产出但未序列化"直至 7e 完成，避免它变成第四种"没人知道的事"。`2312` 随升版一并落地（D17），已分配未使用的 opcode 值（D32/D34）同批回收。

## 详细设计

### IR 结构变更的连锁影响

#### `src/middle/ir/ir.rs`

| 改动 | 位置 | 性质 |
| --- | --- | --- |
| 新增 `ValueId(u32)` 类型 | 文件头（`ValueId` 定义） | 新增（**7b.1a 已落地**） |
| `Operand` 删 `Temp` / `Label` / `Register`，增 `Value(ValueId)` | `Operand` 枚举定义 | **变体增删**（**7b.1a 已落地**） |
| `Instruction` 新增 `Phi` 变体 | `Instruction` 枚举末尾（`Phi` 变体） | **变体 +1**（**7b.1a 已落地**；枚举现为 75 个变体） |
| `FunctionBody::Code` 增 `values: Vec<ValueInfo>` | `FunctionBody::Code` | 字段 +1（**7b.1b 已落地**） |
| 新增 `ValueInfo` 结构 | `ValueInfo` | 新增（**7b.1b 已落地**） |
| `all_instructions` / `blocks` / `blocks_mut` / `locals` | 同名访问器 | 需加 `values()` / `values_mut()` 访问器（**7b.1b 已落地**，与既有访问器同形） |

文件头的 `pub use ...ast::Type` **本文不动**——它是 [03](03-type-unification.md) 的范围。但 `ValueInfo::ty` 必须用**收敛后**的单一类型。

#### `src/middle/core/bytecode.rs`（2422 行）

| 改动 | 位置 | 性质 |
| --- | --- | --- |
| `BytecodeInstr` 新增 `Phi` 或"运行期展开为 Move"策略 | `127-554`（现 **66** 个变体） | 变体 +1 或 +0（见下） |
| `opcode()` 加 `Phi` 臂 | `558` | 必须（`BytecodeInstr` 穷尽 match） |
| `size()` 加 `Phi` 臂 | `649` | 必须 |
| `From<BytecodeFile>` 解码 match 加 `Phi` 臂 | `943-2350` | 必须 |
| `upvalue_count: 0` 修正 | `2312` | 本文顺带修（内存格式；落盘需升 `VERSION`，见下） |

**`Phi` 的字节码形态需要设计决策**。两个选项（**设计判断，推荐 A**，理由见「关键决策与理由」）：

- **A（推荐）：`Phi` 是 IR-only 伪指令**，在 `translator.rs` 展开为该块开头的一串 `Move { dst: vreg, src: incoming[i] }`，按前驱 block 索引选择。**不新增 opcode**，`.42` 格式零变更。代价：字节码体积增加（每个前驱一份）。
- B：新增 `PHI` opcode + 前驱索引编码，解释器在跳转时维护寄存器版本。**需要改 `executor/` 状态模型 + `.42` 格式 + 版本号**。收益仅是体积。

选 A 的决定性理由：`.42` 格式变更会触发版本号问题与全量兼容性讨论，**而 A 的体积代价在解释执行场景下无关紧要**（执行的是 Move，不是 Phi）。

#### `src/middle/passes/codegen/translator.rs`（1577 行，50 个 `translate_*`，分布于 `101-1558`）

| 改动 | 位置 | 性质 |
| --- | --- | --- |
| `Operand::Value(v)` 分派 | `OperandResolver`（`codegen/operand.rs`） | 加臂 |
| `Instruction::Phi` 翻译臂 | dispatch match 内，`Free`(533) 之前 | 新增分支，展开为 Move 串（方案 A） |
| 现有 9 条 NOP 降级**位置不变** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **已核实为 9 条**：`655-657` 覆盖 2 个变体，`658-660` 覆盖 4 个变体 |

**SSA 化不会减少 NOP 数。** 这 9 条降级反映的是「IR 有、字节码没有」的表达能力缺口，SSA 化不触碰。

#### `src/backends/interpreter/executor/`

若采用方案 A，`executor/` **无需任何改动**——`Phi` 在 `translator.rs` 阶段就消失了解释器能见。**这是选 A 的第二个决定性理由。**

`executor/ops/control.rs:90` 的 `BytecodeInstr::Switch` 有活跃实现，但 `ir.rs` 中**不存在** `Switch` / `BrTable` / `JumpTable` 任何变体（实测 `ir.rs` 中这三个名字零命中）——**这是一个反向缺口**：字节码层有 IR 层产不出的指令。它不影响本文，但记录在此，因为它说明「IR → 字节码」不是满射。

`executor/debug.rs:194` 的分派 match 同样无需改动（方案 A 下无新 opcode）。

### 编译器改动清单

**逐文件、逐函数。** 所有行号为改动**起点**（实测核实）：

| 序 | 文件 | 位置 | 改动 |
| --- | --- | --- | --- |
| 1 | `src/middle/ir/ir.rs` | `Operand` 枚举定义 | `Operand` 变体增删（**7b.1a 已落地**：删 `Temp` / `Label` / `Register`，增 `Value(ValueId)`） |
| 2 | `src/middle/ir/ir.rs` | `Instruction` 枚举末尾（`Phi` 变体） | 新增 `Instruction::Phi`（**7b.1a 已落地**） |
| 3 | `src/middle/ir/ir.rs` | `FunctionBody::Code` | 增 `values`（**7b.1b 已落地**） |
| 4 | `src/middle/ir/ir.rs` | `ValueInfo` | 新增 `ValueInfo`（**7b.1b 已落地**） |
| 5 | `src/middle/ir/ir.rs` | `values()` / `values_mut()` 访问器 | 加访问器（**7b.1b 已落地**） |
| 6 | `src/middle/ir/ir.rs` | `pub use ast::Type` | **`pub use ast::Type` 不动**（[03](03-type-unification.md) 范围，仅记录） |
| 7 | `src/middle/core/ir_gen.rs` | `884-891` `next_temp_reg` | 返回 `Operand::Value`，单调递增不回滚（**7b.2 已落地**：`src/middle/lower/scope.rs` 的 `next_temp_reg` 产 `Operand::Value(ValueId)`；新增独立 `next_value` 计数器，每函数体从 0 起算，三处语句级回收只回滚 `next_temp` 不触碰它） |
| 8 | `src/middle/core/ir_gen.rs` | `1954-1959` | **删除**语句级回收 |
| 9 | `src/middle/core/ir_gen.rs` | `3632-3638` | **删除**语句级回收（含 `result_reg.is_none()` 守卫） |
| 10 | `src/middle/core/ir_gen.rs` | `4737-4742` | **删除**语句级回收 |
| 11 | `src/middle/core/ir_gen.rs` | `1745-1759` / `1819-1822` | 换 RAII guard（`generate_method_ir`，入口 `1677`） |
| 12 | `src/middle/core/ir_gen.rs` | `1858-1866` / `2026-2029` | 换 RAII guard（`generate_function_ir`，入口 `1830`） |
| 13 | `src/middle/core/ir_gen.rs` | `2116` / `2180` | 换 RAII guard（`generate_curry_innermost_func`，入口 `2105`） |
| 14 | `src/middle/core/ir_gen.rs` | `2224-2229` / `2284-2288` | 换 RAII guard（`generate_curry_function_ir`，入口 `2209`） |
| 15 | `src/middle/core/ir_gen.rs` | `2768-2777` / `2820-2851` | 换 RAII guard（`generate_anon_binding_ir`，入口 `2759`） |
| 16 | `src/middle/core/ir_gen.rs` | `4686-4695` / `4767-4774` | 换 RAII guard（`generate_lambda_body_ir`，入口 `4678`） |
| 17 | `src/middle/core/ir_gen.rs` | `1987-2000` | E3014 检查提取为 `check_register_budget()`，供 `1798`/`2001` 口径统一后共用 |
| 18 | `src/middle/core/ir_gen.rs` | `1798` | `take_cur_locals(param_types.len())` → `take_cur_locals(self.temp_high_water)`，消除口径不一致 |
| 19 | `src/middle/core/ir_gen.rs` | `248-249` | 注释契约删除（由类型系统强制），替换为指向 `verify` 的说明 |
| 20 | `src/middle/core/ir_gen.rs` | `4245-4256` | 改调 `synth::synth_closure` |
| 21 | `src/middle/core/ir_gen.rs` | `6698-6714` | 改调 `synth::synth_closure`，删 `6713-6714` |
| 22 | `src/middle/core/ir_gen.rs` | `6571-6572` | 改从入参读 `SynthEnv`，不再 `mem::take` 共享字段 |
| 23 | `src/middle/core/ir_gen.rs` | `6589-6593` / `6600` | `closure_captures` 降为局部，删 `clear()` |
| 24 | `src/middle/core/ir_gen.rs` | `458` | `closure_captures.contains_key` 改显式参数（**最高优先级的隐式依赖**） |
| 25 | `src/middle/core/ir_gen.rs` | `341` + `1942` | 加 span 键控的消费计数 |
| 26 | `src/middle/core/ir_gen.rs` | `224` / `369` / `1692-1702` | `method_def_ordinals` 改为只读 |
| 27 | `src/middle/core/ir_gen.rs` | `7269-8014` | **最后做**：拆为顶部 `CallArgs` + 6 个 `emit_*` |
| 28 | `src/middle/core/ir_gen.rs` | `7838-7846` | "补 0" 兜底改为返回诊断 |
| 29 | `src/middle/lower/synth.rs` | **新建** | 合成 AST 边界 |
| 30 | `src/middle/ir/verify.rs` | **新建**（[07](07-equivalence-oracle.md) 第一层） | `verify_loose` / `verify_ssa` |
| 31 | `src/middle/passes/codegen/translator.rs` | dispatch match | `Instruction::Phi` 的翻译（**7b.3 已落地**：方案 A 展开在**块入口**——`plan_phi_edges` / `emit_phi_prologue` 按前驱发 `Move` 组、组间既有 `JMP` 跳过、入边跳转按 `(前驱, 汇合块首指令)` 重定向到组起点；`translate_instruction` 的 `Phi` 臂收敛为不可达 E8001 防御臂——单条返回位放不下按前驱分布的多条 Move，到达即响亮失败） |
| 32 | `src/middle/passes/codegen/operand.rs` | `36-45` / `72-77` | `Operand::Value` 解析臂；删 `Temp` 臂（`39-45`）（**7b.2 已落地**：`to_reg(Value(v))` 查 `FunctionBody::Code.value_slots[v]` → `u8`（`>255` 报 E3014；无映射报 E3017 且带指令 span）；删 `Temp` 臂由 7b.1a 先行落地） |
| 33 | `src/middle/core/bytecode.rs` | `127-554` | **方案 A 下不变**（`Phi` 不进字节码） |
| 34 | `src/middle/core/bytecode.rs` | `558` / `649` / `943-2350` | **方案 A 下不变** |
| 35 | `src/middle/core/bytecode.rs` | `2312` | `upvalue_count: 0` 修正 |
| 36 | `src/backends/interpreter/executor/**` | — | **方案 A 下零改动** |
| 37 | `src/middle/passes/regalloc.rs` | **新建** | 线性扫描分配器 |
| 38 | `scripts/ci/check-synth-boundary.py` | **新建** | `synth.rs` 边界哨兵检查 |

### 向后兼容性：`.42` 格式的版本号

**已核实事实**：`.42` 落盘格式**已经有版本号机制**——`src/middle/passes/codegen/bytecode.rs:14-16` 定义 `MAGIC = 0x59584243`（"YXBC"）与 `VERSION: u32 = 4`，写入头部（`:342-343`），读取时校验（`:454-467`，不符即报 "unsupported bytecode version"）。

**设计判断（方案 A 的直接推论）**：

- `Phi` 在 `translator.rs` 阶段展开为 `Move` 串，**不产生新 opcode**。
- `opcode.rs` 的 83 个常量**零变化**，`opcode_name()`（`120`）不变，解码 match（`bytecode.rs:943-2350`）不变。
- `.42` 格式的字段布局**零变化**，`VERSION` 维持 4。旧 `.42` 文件仍可被新二进制读取，新 `.42` 文件仍可被旧二进制读取。**SSA 化本身不需要升版。**

**唯一需要升版的场景是 `2312 upvalue_count` 的修正**（决议 D17：修，且随升版一并处理）：给 `BytecodeFunction` 增加 `upvalue_count` 字段并让编码器写入，属于**格式新增字段**，把 `VERSION` 从 4 升到 5。读取侧已有的版本校验按既有行为拒绝旧文件——`.42` 是构建产物（`main.rs:671`），跨版本读取兼容不是目标，升版无需额外迁移机制。已分配未使用的 opcode 值（D32/D34 的 `Switch` / `TailCall`）随升版一并回收。

## 实施要点

### 分批策略

分四批。**批内每项独立 commit、独立判据、独立 revert**。下表的"改动项"指向上文「编译器改动清单」的序号。

| 批 | 内容 | 改动项 | 判据 | 回滚点 |
| --- | --- | --- | --- | --- |
| **批 a** | 切断多次定义：删 3 处语句级回收 → 6 处 save/restore 换 RAII guard → 口径统一 | 8-10（先）→ 11-16 → 17-18 | 删除回收：**C4**（行为等价 + 第一层支配性不变量）。RAII 与口径统一属纯搬移：**C1** 快照 zero-diff | 逐项单 commit，`git revert` 即回滚 |
| **批 b** | SSA 形态切换：`ir.rs` 结构变更 + 临时值改产 `Operand::Value`（机械替换）→ `translator.rs` 的 `Phi` 臂（方案 A）→ 线性扫描分配器。批内按 WBS 三级任务串行：7b.1a（`Operand` 变体收敛 + `Instruction::Phi` 引入，**已落地**）→ 7b.1b（`FunctionIR` / `BasicBlock` / 类型标注字段，**已落地**）→ 7b.2（`next_temp_reg` 改产 `Operand::Value`，**已落地**）→ 7b.3（`translator.rs` 的 `Phi` 臂 + `BasicBlock.successors` 填充，**已落地**）→ 7b.4（线性扫描分配器 + 删三处语句级回收，与 7a.1 同批） | 1-5、7 → 31-32 → 37 | **C4**：行为等价 + `verify_ssa` 全绿 + `.42` 往返测试 | `ir.rs` 变更**独占一个 commit**（类型变更波及全仓编译，不可增量回滚） |
| **批 c** | 隐式契约显式化：`synth.rs` 边界 + 哨兵脚本 → span 消费计数 → `method_def_ordinals` 只读化 | 20-24、29、38 → 25 → 26 | **C4**：行为等价 + 新脚本纳入 CI；消费计数在新诊断上零触发（否则说明真存在失配） | 独立 commit |
| **批 d** | `generate_call_expr_ir` 拆分 + "补 0" 兜底断言化 | 27-28 | 纯搬移部分：**C1** 快照 zero-diff。断言化部分：**C4** 语料差分 + 第一层校验器 | 独立 commit |

**批内顺序不可交换的三处依赖：**

1. 批 a 的"删回收"排在批 b 的 `ir.rs` 变更之后（删三处语句级回收随 7b.4 与线性扫描分配器同批落地，7a.1 一并执行）——先让 `next_temp` 单调递增，`ValueId` 才有"唯一定义"的语义基础（该语义在 7b.2 的 `next_temp_reg` 改产 `Value` 与 7b.3 的 `Phi` 臂落地后才成立）。
2. 批 b 的 `Phi` 臂必须先于批 d 的拆分——否则拆分会把旧的 `arg_regs` 顺序假设原样搬进新文件。
3. 批 d 必须最后——它依赖前两批已经把 `arg_regs` 语义收敛到"每个实参一个独立 `ValueId`"。

**相对 RFC-039 的位置**：以上四批全部落在 RFC-039 的 **P7** 之内，判据分级（`verify_loose` / `verify_ssa`）由 P2 建立。批与批之间是**串行**的，但批 a 内部的 8-10 / 11-16 / 17-18 之间只有顺序约束，无耦合。

### 前置条件（硬性）

**前置一：判据基线。** [07](07-equivalence-oracle.md) 第一层校验器的 `verify_loose` 模式**必须在现有（非 SSA）IR 上先跑绿**，才允许进入批 a。

理由：SSA 化的判据是 C4（行为等价 + 不变量）。如果不变量校验器本身在改造前就无法在现有 IR 上跑绿，那么批 a ~ 批 d 的每一次验收都缺少可执行判据，只能靠语料差分——而语料差分对缺陷类 1/2/3 **恰好不敏感**（07「替代方案 A」已论证）。**这是一个真实的排序约束。**

**前置二：类型表示收敛。** [03](03-type-unification.md) 必须完成 `ir.rs:3` 的 `pub use ast::Type` 收敛与 `bytecode.rs:2353` 的 `From<MonoType> for IrType` 桥接拆除。**否则 `ValueInfo::ty` 会引入第三套类型兼容判断。**

### 回滚策略

- 每项改动一个独立 commit，`git revert` 粒度 = 改动项粒度。
- 批 b 的 `ir.rs` 结构变更是**唯一不可增量回滚**的部分（类型变更会波及全仓编译）。因此它必须**独占一个 commit**，且在其之前批 a 必须已跑绿并有可 revert 的基线。
- **不使用 feature flag。** 理由：`Operand` 变体增删无法用运行时开关隔离；feature flag 会让 CI 长期不测另一条路径——这正是 RFC-039 记录的"测试接线靠人记，腐化静默发生"的重演。

### 预期管理：行数净增 900-1600 行

**必须先说清楚，否则会在实施中途被"行数没降"质疑。**

| 项 | 行数变化 |
| --- | --- |
| 起点 | `ir_gen.rs` **8448** |
| 删 3 处语句级回收 | **−15** |
| 6 处 save/restore 换 RAII | 净 ±0（RAII guard 定义约 +20，使用点从 22 行降到约 12 行） |
| `next_temp_reg` 改造 | +5 |
| `generate_call_expr_ir` 拆分 | **+40 ~ 80**（6 个 `emit_*` 的签名与参数传递开销；这是纯搬移拆分的典型代价，**拆分使行数上升**） |
| span 键控消费计数 | +15 |
| `method_def_ordinals` 只读化 | +20 |
| `synth.rs` 新文件 | +80 ~ 120 |
| `verify.rs` 新文件 | +400 ~ 600（07 第一层，虽在本文范围但独立计） |
| `regalloc.rs` 新文件 | **+500 ~ 1000** |
| **合计** | **`ir_gen.rs` 约 8448 → 8600 ~ 8800；含新文件则净增 900 ~ 1600 行** |

**早期"8448 → 5000~6000"的预期方向是错的**，实测推演的结果相反。需要说明为什么：

1. **本文的核心收益不是减行数，是消灭"错了不报错"的一整类缺陷。** 6 处手工 save/restore（22 行）在 SSA + RAII 后变成约 12 行 + 一个 guard 类型——**代码行数几乎不变，但"忘记恢复"从一种可能变成一种编译错误**。
2. **纯搬移拆分必然使行数上升。** `generate_call_expr_ir` 746 行拆成 7 个函数，每个都要重复签名（该函数已有 `#[allow(clippy::too_many_arguments)]`，`7268`）、`&mut self` 绑定、错误传播。**"拆文件降行数"是误解——它降的是认知负荷，不是物理行数。**
3. **新增的 verifier 与分配器是净增行数。** 这两样（约 900-1600 行）恰恰是**收益的载体**。

**净收益的诚实陈述**：

> 行数上，本文大概率是**净增**的。收益在**可验证性**——把"错了不报错"的缺陷变成"跑不过就报错"的不变量。如果以行数作为验收标准，本文会失败；正确的验收标准是「分批策略」里的判据。

**本阶段会使 `ir_gen.rs` 的代码量净增，且是有意的。** 2026-10-03 决策已取消一切行数 / 体积门禁（见 [08](08-maintenance-mechanism.md) 禁令二），因此**不需要行数豁免清单**——增量的正当性由职责分离论证：SSA 化把"寄存器分配 + 定义纪律"从 lowering 中剥离成独立模块，代码量增加换来的是消除了 6 处手工 save/restore 与 `arg_regs` 语义重排这两类"错位不报错只错值"的手工纪律。

## 关键决策与理由

### 三条核心决策

**决策一：临时值策略——删除三处语句级回滚，让唯一定义成为构造的数学后果。**
现状的回滚（`1954-1959` / `3632-3638` / `4737-4742`）是为了复用临时槽位。删掉它们，`next_temp` 单调递增，每个临时槽位天然只有一个定义点。**判据一致但守卫不同**（`3632` 有 `result_reg.is_none()` 守卫、另两处没有）这类手抄差异随之消失。代价是槽位占用上升——`temp_high_water` 已有 255 上限检查（`1992-2000`）兜底。

**决策二：不做 mem2reg，只做显式重命名。**
理由已在「值标识方式」给出：YaoXiang 的 `&` / 引用 / 所有权语义让"哪些槽位可内存化"成为类型系统问题，需要 [03](03-type-unification.md) 收敛后的类型表示才能判定。**在 03 之前做 mem2reg 就是在猜。** 显式重命名只需改 `next_temp_reg` 一处的返回值语义 + 删三处回滚，不触碰任何 lowering 算法的控制流。

**决策三：`Phi` 采用方案 A（IR-only 伪指令，展开为 Move 串）。**
`executor/` 与 `.42` 格式**零改动**，回归面被限制在 L3。决定性理由：`.42` 格式变更会触发版本号问题与全量兼容性讨论，而 A 的体积代价在解释执行下无关紧要（执行的是 Move，不是 Phi）。代价是 `dump_bytecode` 中 `Phi` 不可见（进入 C1/C2 阶段时需注意 07 第三层要比对它）。

### 被否决的替代方案

**A. 一次性全拆（同时做 SSA + 拆分 + `synth` 边界 + 分配器）。否决。** 违反 07 的核心原则：判据必须按类别分级。四个批混在一个 PR 里，纯搬移改动与 C4 改动混在一起，快照 drift 无法归因——快照一变就不知道是拆分搬错了还是 SSA 改错了。**回滚粒度从"改动项"退化为"全部"。**

**B. 先拆文件再优化顺序。否决。** 拆分不消除任何根因：6 处手工 save/restore 仍分散在 10 个文件里，`arg_regs` 语义重排仍需人工推理。更糟的是**先拆分会失去"改之前先建判据"的机会**——拆分产生大量行级 diff，混进后续 SSA 改动的 diff 里，让 review 失效。这与 06-cleanup-inventory.md 记录的 8 棵孤儿测试树（1005 行 / 78 个测试从未运行）同源：**顺序错了，后面每一步都在错误的基础上加速。**

**C. 引入中间 SSA 层而不改现有 lowering。否决，但记录为"曾经考虑过的第三条路"。** 形态是保留 `ir_gen.rs` 不变，新增一个 pass 把它产出的非 SSA IR 转成 SSA IR。**优点**是 lowering 完全不动、回滚容易。**否决理由**：(1) 不消除缺陷类 1 的根因——三处语句级回收仍在 `ir_gen.rs` 里，6 处 save/restore 仍分离 59-161 行；转换 pass 能把多次定义合并成 `Phi`，但**无法修复"内层函数的名字串到外层"**——那是 `cur_locals` 的 save/restore 错误，发生在 IR 形态之前，转换 pass 看不到。(2) 把 SSA 变成第二套表示——仓库已有 3 套平行类型表示的前车之鉴，而本项目的核心教训是"**边界只存在于人的自觉里**"。(3) `verify_loose` 会成为永久负担。

**D. 保持现状，只加 verifier。否决为唯一方案，但它是本文的组成部分。** RFC-039 替代方案 B 已给出同一判断：门禁能防退化，不能修现状。**但必须说清 D 能做到什么**：D 能**抓住**缺陷类 1/2/3 的**实例**（前提是 verifier 覆盖支配性、类型一致、内层隔离），但抓不住缺陷类 4（`closure_captures` 泄漏）——那需要一个跨函数的活性断言，超出 `verify(&ModuleIR)` 的函数边界。**因此 D 与本文不是替代关系，是前置关系。**

## 已知局限与风险

### 风险

- **代码量净增 900–1600 行**（新增 `verify.rs` / `regalloc.rs`）。**这不是风险点**——2026-10-03 决策已取消行数 / 体积门禁，规模问题由职责分离解决（见 [08](08-maintenance-mechanism.md) 禁令二）。增量的正当性在于：它换掉了 6 处手工 save/restore 与 `arg_regs` 语义重排这两类"错位不报错只错值"的手工纪律。
- **`verify.rs` 的"唯一定义"检查在非 SSA 形态下无意义**（`Operand::Local` 允许多次定义）——07 已记录该局限，`verify_loose` 是它在本批之前的降级模式。
- **删除语句级回收会改变临时槽位分配模式**（批 b 执行，7a.1 与 7b.4 同批）。即使 `temp_high_water` 保证不溢出，**分配结果会变**——这正是 C4 阶段"行为等价"判据必须真正跑全语料的原因，而不是抽查。
- **`u8` / 255 的槽位上限在删除回滚后不够用（已核实）**：实测删掉三处回收后跑全语料，2 条语料越过 255 触发 E3014——`src/std/tests/string_ops.yx` 354 槽、`tests/yaoxiang/01-syntax/control-flow/list_comp.yx` 323 槽；其余语料的槽位峰值由 51 升到 211。`src/middle/lower/scope.rs` 的 `take_body_locals` 会捕获越界，但捕获即编译失败。所以删回滚排在分配器（7b.4）之后：先让分配器承担槽位复用，再删回滚。
- **优点须一并记录**：把"不能测的错误"变成"能测的不变量"；不改后端（`executor/` 与 `.42` 零改动）；不动 `Instruction` 的语义（76 → 77 变体全部保留，50 个 `translate_*` 全部保留）；可增量（每项独立可 revert）。

> **本节原列的开放问题已全部裁决。** 逐条决定见 [RFC-039 决议登记](../../rfc/accepted/039-compiler-architecture.md)（D1–D50）。**本文不留任何待定项。**
>
## 参见

### 文档

- [RFC-039 编译器架构重构](../../rfc/accepted/039-compiler-architecture.md) — 上位总纲；四层模型、判据分级、P1-P10 执行顺序
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **硬前置**；C4 类别的判据定义、`verify_loose` / `verify_ssa`、三层判据、快照对 `arg_regs` 免疫的局限
- [03-type-unification.md](03-type-unification.md) — **硬前置**；3 套平行类型表示的收敛
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` 账本；`ReleasePlan` 的 `PlanId` 改造归属
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — 8 棵孤儿测试树；「先建目录后接线失败」的反面教材

### 代码位置

**IR 定义**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type`（[03](03-type-unification.md) 范围）
- `src/middle/core/ir.rs:12-20` — `Operand` 7 变体
- `src/middle/core/ir.rs:47-533` — `Instruction` 76 变体（54 带 `dst: Operand`、3 带 `dst: Option<Operand>`）
- `src/middle/core/ir.rs:624-628` — `BasicBlock`（含 `successors`）
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code`（含 `entry` / `locals`）
- `src/middle/core/ir.rs:653-659` — `LocalSlot`（含 `scope_depth` / `ty`）
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>`（`PlanId` 方案的可行性依据）
- `src/middle/core/ir.rs:689-719` — `all_instructions` 等访问器

**缺陷类 1（多次定义与 save/restore）**

- `src/middle/lower/scope.rs` 的 `NestedBodyGuard` — 7 个嵌套函数体入口的保存/恢复（`Deref`/`DerefMut` + `Drop`）
- `src/middle/lower/scope.rs` 的 `take_body_locals` — 唯一取表 + E3014 预算点（`MAX_REGISTERS = 255`）
- `src/middle/lower/scope.rs` 的 `take_cur_locals` — 只在 `take_body_locals` 内部调用
- `src/middle/lower/scope.rs` 的 `next_temp_reg` — 维护 `temp_high_water` 真高水位
- `src/middle/lower/stmt.rs` 的 `generate_function_ir` / `generate_block_ir` 与 `src/middle/lower/expr/closure.rs` 的 `generate_lambda_body_ir` — 三处语句级寄存器回收
- `src/middle/lower/stmt.rs` 的 `rebase_jump_targets` — 07「jump 目标存在」不变量所指

**缺陷类 2（arg_regs）**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir`（746 行）
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — 三处语义重排与克隆
- `src/middle/core/ir_gen.rs:7838-7846` — 「补 0」静默兜底

**缺陷类 3（span 键控）** —— 下列为**审计时点**的取证指针（4.5 键迁移后：`release_plan` / `overload_resolutions` 两键已 `PlanId` 化，现状指针见 [02](02-stage-contract.md) §2 修订注记；`method_def_ordinals` 仍为机制三的待办）

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — 三个跨层字段的声明
- `src/middle/core/ir_gen.rs:341` / `1942` — `release_plan` 的写入与消费
- `src/middle/core/ir_gen.rs:7420` / `7493` — `overload_resolutions` 的消费
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — `method_def_ordinals` 的初始化与唯一写入点（key 为无模块限定的 `"Type.method"`）
- `src/frontend/core/typecheck/types.rs:31` / `56` — 传递结构上的两个字段
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — `ReleasePlan` 定义与产出
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — 跨层传递
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` — `overload_resolutions` 产出

**缺陷类 4（隐式顺序依赖）**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — `generate_spawn_for_ir` 的 AST 合成与回调
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — `generate_spawn_expr_ir` 的同款握手
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` 与 `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — `closure_captures` 的两个读点
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — 另 3 处 `ast::Type` 构造

**下游**

- `src/middle/core/bytecode.rs:127-554` — `BytecodeInstr` 66 变体
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule`（1408 行）
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — 三处硬编码丢弃（对照 `2344` / `2347`）
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType`（[03](03-type-unification.md) 待拆的桥接）
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — 9 条 NOP 降级
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`，`u8` 上限 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`；文件共 83 个常量
- `src/backends/interpreter/executor/ops/control.rs:90` — `BytecodeInstr::Switch` 的活跃实现（IR 侧无对应变体）
- `src/backends/interpreter/executor/debug.rs:194` — 分派 match

**测试现状**

- `src/middle/core/ir_gen.rs` — 8448 行，0 个 `#[cfg(test)]`，0 个 `mod tests`
- `src/middle/core/tests/mod.rs` — 声明 `bytecode` / `def_assign` / `local_slots`，共 29 个测试；`def_assign` / `local_slots` 正向钉住 DefId 与槽位命名，无 IR 结构不变量校验
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — 4 处提到 ir_gen，全为反向断言
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop`（逐 opcode 往返判据范例，文件内最后一个 test；文件共 1209 行 / 23 个 `#[test]`）

**工程配置**

- `Cargo.toml:37-39` — dev-dependencies 仅 `criterion` / `proptest`；`[dependencies]` 中无 regalloc / petgraph / graphlib
- `docs/src/.vitepress/config.js:226-231` — 侧边栏对 `/rfc/draft` 目录做自动扫描；本文不在该目录下，由 RFC-039 与本目录索引引用
