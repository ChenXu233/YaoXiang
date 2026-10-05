# Intermediate Representation SSA-ization

> **Subordinate design document**. This document is subordinate to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/draft/039-compiler-architecture.md). The
> four-layer model, acceptance criteria grading, and execution phase ordering are in the main body
> of RFC-039; the positioning of each subordinate document is in [this directory index](index.md).

## Positioning and Scope

This document handles the IR construction discipline of the L3 layer, proposing a "each value has a
unique definition" construction approach for `src/middle/core/ir_gen.rs` (8448 lines). **The goal is
not to introduce a new instruction set, but to eliminate three categories of discipline defects that
produce no errors on failure**:

1. 6 manual register save/restore pairs scattered across dozens of lines apart;
2. The `arg_regs` semantic rearrangement shared by 6 branches inside `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent failure of cross-layer contracts keyed by `Span`.

This document is responsible for: IR form (`Operand` / `Instruction`) changes, temporary value
allocation discipline, cross-layer contract explicitness, call expression dispatch decomposition,
and how the IR static verifier is wired up.

This document is NOT responsible for: type representation convergence
([03](03-type-unification.md)), stage contracts and obligations ledger ([02](02-stage-contract.md)),
equivalence criterion definition and gating ([07](07-equivalence-oracle.md)), frontend paradigm
([05](05-frontend-paradigm.md)), dead code and empty design cleanup ([06](06-cleanup-inventory.md)).
This document references the conclusions of these documents many times and does not repeat the
arguments.

**The criteria category** is executed per **C4 (IR form change)** of
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariant**, not full IR snapshot equality. SSA-ization necessarily changes the IR form; forcibly
using snapshots would induce the team to relax criteria (this is precisely the trap recorded in 07).

**Position in the RFC-039 phase sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, acceptance is behavioral equivalence + `verify_ssa` green). The internal
implementation batch numbering in this document (batches a ~ d) and the ordering within P7 are two
different things: the former is this document's batch strategy, the latter is RFC-039's global
phases.

## Feasibility Prerequisites

**Verified facts.** The IR already possesses **all structural prerequisites** required for SSA
construction:

| SSA prerequisite             | Current state in this repository                                                                              | Location              |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------- | --------------------- |
| Explicit CFG                 | `BasicBlock { label, instructions, successors }`                                                              | `ir.rs:624-628`       |
| Explicit entry block         | `FunctionBody::Code { blocks, entry, locals }`                                                                | `ir.rs:636-642`       |
| Scope nesting information    | `LocalSlot::scope_depth` ("0 = function parameter layer, for nested scope disambiguation of duplicate names") | `ir.rs:658`           |
| Three-address form           | 76 variants, 54 with `dst: Operand`, 3 with `dst: Option<Operand>`                                            | `ir.rs:47-533`        |
| Per-instruction span         | Each variant carries a `span: Span` field                                                                     | `ir.rs:47-533`        |
| Value type annotatable       | `LocalSlot::ty: MonoType`                                                                                     | `ir.rs:656`           |
| Slot count upper bound check | E3014 check inside `generate_function_ir` (`MAX_REGISTERS = 255`)                                             | `ir_gen.rs:1987-2000` |

**If these prerequisites are not in place, the cost estimate of this document would be entirely
different.**

**Two decisive points:**

**First, SSA-ization is not adding new capability, it is removing a capability dependency.** In the
current state, temporary slots are **actively designed to be defined multiple times** — three
locations `1954-1959` / `3632-3638` / `4737-4742` explicitly roll back `next_temp`. The first thing
in SSA-ization is to **delete these three rollback points**, letting `next_temp` increase
monotonically. After deletion, **each temporary slot naturally has only one definition point** —
unique definition changes from "a rule that needs checking" to "a mathematical consequence of the
construction approach".

**Second, the slot count upper bound is already a hard check.** The `temp_high_water` at
`ir_gen.rs:889` is already tracking the true high water mark, and `1992-2000` is already reporting
errors against the 255 upper limit. **Removing the rollback will not cause slot count to run out of
control** — `temp_high_water` records the historical maximum occupation, which is unrelated to
whether rollback occurs (the comments at `887-888` are for this very purpose).

**The combined meaning of these two points**: `ir_gen.rs` is already maintaining all the bookkeeping
information required for SSA, it just **recovers** that information. The main work in SSA-ization
lies in the decomposition of `generate_call_expr_ir` and the explicitness of cross-layer contracts,
not in the construction of IR expression capability.

### Prerequisite: Unification of Type Representation

**Verified fact.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` additionally has `use crate::frontend::core::typecheck::MonoType;`, and `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. The serialization side has a third one, `ir::Type`, bridged by
`impl From<MonoType> for IrType` at `bytecode.rs:2353`.

That is: **two sets of type representations coexist in the IR, and one of them directly `pub use`s
the AST type.**

SSA-ization needs to add a `Phi` variant, record types for each SSA value (to supply the "type
consistent" invariant for the 07 validator), and judge whether the types of two predecessors at a
CFG merge point are compatible. **If the type representation has not yet converged, SSA will write a
type compatibility judgment for each representation** — a third representation will grow out of it.

This is not a question of "which to do first is better", it is a question that **doing SSA first
will produce three sets of type compatibility logic**.
[03 Type Representation Unification](03-type-unification.md) must first complete the convergence of
`pub use ast::Type` at `ir.rs:3` and the removal of the bridging at `bytecode.rs:2353`. Item 6 of
the change list in this document explicitly records that `ir.rs:3` **is not touched by this
document**.

## Current State: Four Categories of Defects

`ir_gen.rs` is not a "ugly but works" file. It is a **file that bets correctness on line-by-line
human review**:

- **0 `#[cfg(test)]`, 0 `mod tests`** in the file (verified).
- `src/middle/core/tests/mod.rs` declares three modules: `bytecode` / `def_assign` / `local_slots`,
  with 29 tests in total — among them, `def_assign.rs` and `local_slots.rs` **positively** cover the
  DefId allocation and local slot naming of ir_gen (sentinel level), while `bytecode.rs`'s 4
  references to ir_gen (`421` / `423` / `1028` / `1173`) are **all negative assertions** — "ir_gen
  does not produce", "ir_gen also does not construct", "ir_gen frontend does not construct
  corresponding IR".

That is: ir_gen has 29 sentinel-level tests pinning down local behavior, but **there is no IR
structural invariant validation at all** (dominance, unique definition, jump targets, type
consistency) — none of the three modules declared in `middle/core/tests/mod.rs` makes positive
assertions about the overall IR form.

Yet this file happens to contain several kinds of **defects where compilation succeeds, all checks
pass green, the program runs, but the values are wrong**. Their common property: **failure produces
no error**.

This is where the highest risk lies. Not "the function is too long", not "the naming is chaotic",
but **the core invariants of this file have no machine-executable enforcement**.

`ir_gen.rs:248-249` writes this as an explicit contract in plain text:

> Save before generating a nested function body, restore after — **must be physically adjacent to
> the save/restore point of `next_temp`**, otherwise the inner function's names will leak into the
> outer layer.

This is a **rule that uses comments in place of assertions**. It holds today only because those 6
pairs of save/restore are all coincidentally written correctly.

**Why "just add a verifier" is not enough**: a verifier can **catch** these defects, but it cannot
catch their **cause**. The cause is not "missed during review", but:

> **The current IR form itself allows a slot to be defined multiple times, and `ir_gen`'s temporary
> register allocator actively depends on this.**

Specific evidence is the three statement-level register recoveries (`1954-1959` / `3632-3638` /
`4737-4742`) — at the end of each statement, they **roll back** `next_temp` to "the larger of the
last named slot and `temp_floor`" (`1959`:
`self.next_temp = named_watermark.max(self.temp_floor);`). This means the same `Operand::Local(n)`
slot is **written repeatedly within the same function body**, with the number of writes, the order
of writes, and the live range completely determined by the single manual decision of "when to roll
back".

Under this form, "unique definition" is not a rule that can be incrementally imposed, it **directly
conflicts with the register allocation strategy**. So SSA-ization is not adding a check to existing
code, it is **replacing the allocation strategy that depends on multiple definitions**. The good
news is that the replacement cost is very low — see the previous section "Feasibility
Prerequisites".

### Defect Category 1: 6 Manual save/restore Pairs, Silent on Failure

**Verified fact.** 6 save/restore pairs, with the maximum distance between save and restore reaching
**161 lines**:

| Save point            | Restore point | Span      | Function                        | Entry line |
| --------------------- | ------------- | --------- | ------------------------------- | ---------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | 63 lines  | `generate_method_ir`            | `1677`     |
| `1858-1866`           | `2026-2029`   | 161 lines | `generate_function_ir`          | `1830`     |
| `2116`                | `2180`        | 64 lines  | `generate_curry_innermost_func` | `2105`     |
| `2224-2229`           | `2284-2288`   | 59 lines  | `generate_curry_function_ir`    | `2209`     |
| `2768-2777`           | `2820-2851`   | 74 lines  | `generate_anon_binding_ir`      | `2759`     |
| `4686-4695`           | `4767-4774`   | 72 lines  | `generate_lambda_body_ir`       | `4678`     |

> **Ownership attribution note**: The correspondence of the two curry functions is that
> `2116`/`2180` belong to `generate_curry_innermost_func` (entry `2105`), and
> `2224-2229`/`2284-2288` belong to `generate_curry_function_ir` (entry `2209`) — that is, the
> "innermost" goes to the former, do not intuitively infer backward by line number ordering.

Fields involved: `next_temp` / `temp_high_water` / `temp_floor` / `cur_locals` / `cur_span` /
`loop_stack`, uniformly driven by `next_temp_reg` (`ir_gen.rs:884-891`). This function maintains the
true high water mark of `temp_high_water` at `889` (comment: statement-level recovery rolls back
`next_temp`, total slot count is based on historical maximum occupation). A similar comment
convention also applies to `cur_span` (`:238-242` self-stated "save before generating nested
function body, restore after (same handling as next_temp)").

**Two inconsistent conventions** (this is a more hidden problem than "missed write"):

| Location         | Convention                                                                                                                |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `ir_gen.rs:1798` | `take_cur_locals(param_types.len())` — truncated by **number of parameters**                                              |
| `ir_gen.rs:2001` | `take_cur_locals(total_locals)`, where `total_locals = self.temp_high_water` (`:1992`) — truncated by **high water mark** |

`generate_method_ir` uses the former, **and does not have the E3014 overflow check at `1992-2000`**.
The two functions give two different answers for "how many slots should a function body have", and
neither reports an error.

**Three repeated statement-level register recoveries, with the same criterion but different
guards**:

| Location    | Function                           | Guard                     |
| ----------- | ---------------------------------- | ------------------------- |
| `1954-1959` | `generate_function_ir`             | None                      |
| `3632-3638` | `generate_block_ir` (entry `3602`) | `if result_reg.is_none()` |
| `4737-4742` | `generate_lambda_body_ir`          | None                      |

The guard at `3632` has a clear reason (comments at `3626-3631`: a block of expression operand bits,
the outer layer may hold sibling argument temporaries that survive across blocks). The other two
have no equivalent guard. **The same criterion is hand-copied in three places, with one having a
guard and two not** — such differences are exactly what SSA must eliminate.

**The restore point of `generate_anon_binding_ir` is separated by IR construction** (`2768-2777`
save → `2820-2824` restore 5 fields → `2829-2848` construct `func_ir` → `2851` finally restore
`loop_stack`). The two halves of the same state are physically separated by 22 lines of construction
code, and the order dependency is **reversed** (restore the slot table first, then construct IR,
then restore the loop stack).

**Consequence**: Miss one restore → `next_temp` leaks, temporary values skip numbers, `cur_locals`
names leak to the outer layer. **The IR is still self-consistent, still passes all checks, still
compiles, but the values are wrong.**

### Defect Category 2: `arg_regs` Semantic Rearrangement in `generate_call_expr_ir`

**Verified fact.** `generate_call_expr_ir` is located at `ir_gen.rs:7269-8014`, **746 lines**, a
single function. The function signature (`7269-7279`) accepts `func: &Expr` / `args: &[Expr]` /
`named_args: &[(String, Expr)]` / `span: &Span` (additionally with `_expr` / `result_reg` /
`instructions` / `constants`), and the 6 internal branches share the same `Vec<Operand>` and perform
**semantic rewriting**:

| Lines       | Semantics                                                                                                                                                      |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `7395`      | Namespace call: flatten by `slots` then re-`collect` — named argument rearrangement                                                                            |
| `7724`      | Struct construction: `unwrap` by `final_args` slot-by-slot then re-`collect` — field rearrangement                                                             |
| `7835-7848` | Function call: named argument rearrangement; `7838-7846` **supplements a new register with value 0 for missing slots** (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();` — clone then emit instructions                                                                              |

The original form of the four rearrangement locations in source code:

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

The comment at `7835-7848` self-states: "Uncovered slots: remain absent, the type checking phase
reports E1010. At runtime, supplement 0 for safety (normal paths will not reach here)."

**This design carries two layers of risk**:

1. **Misalignment reports no error, only wrong values.** Each of the 6 branches decides the final
   order of `arg_regs`; any order assumption error will not panic.
2. **Canonicalization snapshots are immune to this.** 07 has recorded this limitation: the snapshot
   tool renames temporary values by "order of first appearance", so "the 3rd argument should have
   used the 5th register" is completely identical in the snapshot. **This category of defects does
   not even have a snapshot-level criterion in the C4 phase.**

**Supplementary verification: the struct construction path's rearrangement has no duplicate slot
guard.** The `final_args` at `7724` is filled by "positional arguments enter positions first, named
arguments overwrite by field name", **with no `slots[idx].is_some()`-like duplicate slot check
before overwrite** (contrast with the namespace path at `7381` which has one) — when named arguments
silently override positional arguments, there is no panic, no diagnostic; and the argument
**evaluation order** (source code order) diverges from the final `arg_regs` order (field declaration
order) at this point. The assertion-ification of `fill_missing` in batch d must also cover the
"duplicate slot" check, otherwise the silent override on the struct construction path will survive
intact.

### Defect Category 3: Silent Failure of Span-Keyed Cross-Layer Contracts

**Verified fact.** There are exactly **two** span-keyed cross-layer contracts in the entire
repository:

| Contract               | Declaration                                  | Production                                                                               | Transfer                                                               | Consumption              |
| ---------------------- | -------------------------------------------- | ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------ |
| `release_plan`         | `ir_gen.rs:191` `HashMap<Span, Vec<String>>` | `ownership.rs:31` (`ReleasePlan::drops` field), `2574` `build_release_plan`, `2786` call | `typecheck/types.rs:31` ← `checker.rs:1282` / `1440` ← `ir_gen.rs:341` | `ir_gen.rs:1942`         |
| `overload_resolutions` | `ir_gen.rs:222` `HashMap<Span, String>`      | `inference/expressions.rs:4366`, `inference/statements.rs:2758` / `2857`                 | `typecheck/types.rs:56` ← `checker.rs:1447-1448` ← `ir_gen.rs:364-366` | `ir_gen.rs:7420`, `7493` |

The form at `1942`:

```rust
// NLL Release: insert Drop instruction at statement boundary
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

`if let Some` — **not found means not found, no else branch, no assertion, no count**. Any
inconsistency in how the two sides compute spans → **Drop instructions silently disappear, zero
errors**. The Drop sequence of refinement types is the core carrier of the ownership semantics in
this project (`ownership.rs:26-32` self-states "NLL precise release plan", key is the Span of the
last use position), and its landing is silent.

`7420` / `7493` are the same form (`if let Some(mangled) = self.overload_resolutions.get(span)`),
with the consequence that overload resolution falls back to the default binding.

> **`method_def_ordinals` is not span-keyed.** It is `HashMap<String, usize>` (`ir_gen.rs:224`),
> initialized at `:369`, with the sole write point at `:1692-1702` (assign `#N` suffix in definition
> order by `base_name`). Its risk is **another kind**: definition order dependency — the `#N` suffix
> is determined by the AST definition order; if the generation order is inconsistent with the
> typecheck registration order, the same function name resolves to different targets. This is not
> span mismatch, but it is also "wrong with no error", handled in "Additional Mechanisms for
> Span-Keyed" mechanism three.

### Defect Category 4: Implicit Handshake in spawn/lambda

**Verified fact.** `ir_gen.rs` has **two** locations that construct `ast::Expr::Lambda` themselves
and callback into `generate_expr_ir`, relying on a set of mutable fields for handshake:

**Handshake A — `generate_spawn_for_ir` (entry `4143`)**:

```
4245    self.pending_env_vars = vec![Operand::Local(element_reg)];
4246    let lambda = ast::Expr::Lambda { params: ..., body: ..., span };
4256    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
```

`generate_expr_ir` → `generate_lambda_expr_ir` (entry `6543`) → `6571-6572`
`std::mem::take(&mut self.pending_env_vars)` consumes.

**Handshake B — `generate_spawn_expr_ir` (entry `6641`)**:

```
6698    self.pending_env_vars = env_ops;
6699    self.pending_env_names = env_names;
6701    let lambda = ast::Expr::Lambda { params: Vec::new(), body: Box::new(ast::Block { stmts: vec![ast::Stmt { ... }] }), span };
6712    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
6713    self.pending_env_vars.clear();
6714    self.pending_env_names.clear();
```

**Handshake C — `closure_captures`**: written at `6589-6593`, **in the same function** `6598` calls
`generate_lambda_body_ir` (entry `4678`), `6600` clears. There are two read points: `:6284`
(`LoadUpvalue` inside the closure body) and **`:458`** (`closure_captures.contains_key(head)` on the
`flatten_namespace` path).

The read point at `:458` is the most dangerous: it is on the **namespace resolution** path, meaning
the survival state of a `HashMap<String, usize>` can change the variable name resolution result.
Between `6589` write and `6600` clear lies a complete closure body generation (`6598`); if any path
inside `generate_lambda_body_ir` returns early without clearing the table (it has multiple return
points when the `?` operator propagates errors), **`closure_captures` will leak to the unrelated
code at `:458`**.

Similar implicit state includes `pending_env_names` (B handshake writes at `6699`, clears at
`6714`).

### What SSA Will NOT Solve (Honestly Listed)

| Problem                                           | Location                                                                                                                | Why SSA does not solve it                                                                                                                                                                                                               |
| ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `compile_pattern` complexity                      | `ir_gen.rs:5390-5687`, **298 lines**, 4 self-recursions (`5473` / `5504` / `5619` / `5640`), external call point `5270` | The dispatch complexity of pattern matching (union payload / tuple / struct / literal / nested) is **inherent to the algorithm**. SSA only turns the `JmpIfNot` generated by guards into `Phi`, it does not reduce the number of guards |
| `eval_const_expr` complexity                      | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming self-recursion) | The expression coverage of constant folding is unrelated to SSA                                                                                                                                                                         |
| Span-keyed failure                                | `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`)                                                       | **SSA does not touch this layer at all.** Requires independent mechanism, see "Additional Mechanisms for Span-Keyed"                                                                                                                    |
| `Instruction` variant count                       | 76 (`ir.rs:47-533`)                                                                                                     | SSA adds `Phi`, **net change +1**. The 76 variants correspond to 76 bytecode opcodes or degrade to NOP, will not be reduced by SSA                                                                                                      |
| `method_def_ordinals` definition order dependency | `ir_gen.rs:224` / `369` / `1692-1702`                                                                                   | Same category as span-keyed but different cause, see "Additional Mechanisms for Span-Keyed" mechanism three                                                                                                                             |

## Target Design

### SSA Form Definition

**Design judgment.** The target form is as follows.

#### How to Change `Operand` Variants

The current `ir.rs:12-20` has 7 variants in total. **Full repository construction point
verification**:

| Variant             | Production construction points                                                                                                                                                    | Disposition                                                                              |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 8 files                                                                                                                                                                           | **Keep**, unchanged                                                                      |
| `Local(usize)`      | 7 files                                                                                                                                                                           | **Keep but split semantics** (see below)                                                 |
| `Arg(usize)`        | 4 files                                                                                                                                                                           | **Keep**, as implicit definition of function entry (parameters are block 0's definition) |
| `Global(usize)`     | 5 files                                                                                                                                                                           | **Keep**, unchanged                                                                      |
| `Temp(usize)`       | **0 construction points** (only `codegen/operand.rs:39-45` and `:75` have handling arms)                                                                                          | **Delete** (dead variant)                                                                |
| `Label(usize)`      | **0 construction points**                                                                                                                                                         | **Delete** (dead variant)                                                                |
| `Register(u8)`      | **0 construction points** (`ir.rs:19` comment says "Added for codegen", but codegen goes through `codegen/operand.rs`'s `OperandResolver`, which does not construct this variant) | **Delete** (dead variant)                                                                |

`Local(usize)` is split into two variants, because it currently **simultaneously carries two
mutually exclusive semantics** — "storage location for named variables" (writable multiple times)
and "compiler temporary register" (should be written only once):

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA value: densely numbered within the function, single definition
    Value(ValueId),
    /// Storage slot for named local variable: addressable, allows multiple definitions (non-SSA part)
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` is a new type `u32` (`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`),
densely numbered within the function, **monotonically increasing, never rolled back**.

#### `Phi` as New `Instruction` Variant

```rust
/// Merge point: each predecessor basic block provides one definition of this value
/// incoming sorted in ascending order of predecessor block label (guaranteed at construction, for normalization)
Phi {
    dst: Operand,                                  // must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

Three construction constraints (all enforced by `verify_ssa`, see [07](07-equivalence-oracle.md)
first layer):

1. `incoming.len() == predecessor count of this block`.
2. The label set of `incoming` == the set of labels of those blocks whose `successors` include this
   block's label in `blocks`.
3. All `incoming` values have the same `MonoType` (**depends on the type representation in 03 having
   converged**).

**Disposing of the guard issue at `3632-3638`**: The current recovery in `generate_block_ir` has an
`if result_reg.is_none()` guard (comment at `3626-3631`: a block of expression operand bits, the
outer layer may hold sibling argument temporaries that survive across blocks). After SSA-ization,
this guard **is no longer needed** — because temporary values are no longer rolled back, sibling
argument temporaries that survive across blocks hold their own `ValueId` and will not be affected by
rollback. **This is where SSA-ization can truly simplify code, not just rename things.**

#### Value Identification Approach

**Design judgment**: Do not do mem2reg, do not do virtual register numbering, **only do explicit
renaming**. Reasons:

- YaoXiang's `&` / reference / ownership semantics make "which slots can be mem2reg-ed" a **type
  system problem**, not a CFG problem. Verified: `ir.rs:656` `LocalSlot::ty: MonoType` carries
  complete type information, but judging "whether this `MonoType` can be mem2reg-ed" requires the
  cooperation of the converged type representation and refinement type rules from 03. **Doing
  mem2reg before 03 is guessing.**
- Explicit renaming only requires changing the return value semantics of `next_temp_reg()` at one
  place (`ir_gen.rs:884-891`) and deleting the three rollbacks, **without touching the control flow
  of any lowering algorithm**.

Value table form: `FunctionBody::Code` adds `values: Vec<ValueInfo>`, where

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot), None means pure temporary value
    pub debug_name: Option<String>,
    pub defining_block: usize,   // label of the basic block where the definition resides
}
```

`values` and `locals` coexist: `locals` carries addressable named variables (possibly defined
multiple times), `values` carries single-definition SSA values. **The existence of `locals` does not
violate SSA** — it is the "storage" part outside SSA (LLVM's alloca model).

#### Change Magnitude Estimate

| Item                                                 | Estimate               | Basis                                                                                                                       |
| ---------------------------------------------------- | ---------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `next_temp_reg` refactor                             | ~15 lines              | Single function at `ir_gen.rs:884-891`                                                                                      |
| Delete three statement-level recoveries              | −15 lines              | `1954-1959` / `3632-3638` / `4737-4742`                                                                                     |
| Converge 6 save/restore into RAII                    | Net ±0 lines           | See "Item-by-item Disposition Table"                                                                                        |
| `ir.rs` structural changes                           | +120 ~ 180 lines       | `ValueId` / `Phi` / `ValueInfo` / value table                                                                               |
| Add `Phi` arm to `translator.rs`                     | +25 ~ 40 lines         | One of the existing 50 `translate_*`                                                                                        |
| Add `PHI` instruction or runtime fill to `executor/` | +60 ~ 120 lines        | Depends on `Phi`'s bytecode form selection                                                                                  |
| Rename `Operand::Local` → `Value` call sites         | Mechanical replacement | **Verified**: `Operand::Local(` appears **323 times** in `ir_gen.rs`, **355 times** in 6 files across the entire repository |

### Item-by-item Disposition Table

| Defect category                                     | Whether SSA eliminates it  | Disposition                                                                                                                                                                                                                                                                                                                                                                 | Criterion                                                                |
| --------------------------------------------------- | -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| **1. 6 manual save/restore**                        | **Yes (root cause level)** | `next_temp` / `temp_high_water` / `temp_floor` change to `&mut` monotonic counters, no longer requiring cross-function-body saving. `cur_locals` / `loop_stack` / `cur_span` change to use RAII guard (`struct NestedBodyGuard<'a>`, `Drop` restores), **making "forgetting to restore" a compile error**. The comment contract at `248-249` is enforced by the type system | 07 first layer "inner isolation" invariant + `verify_loose`/`verify_ssa` |
| **1b. Inconsistent conventions (`1798` vs `2001`)** | **Yes**                    | Unify to `take_cur_locals(self.temp_high_water)`, and the E3014 check (`1992-2000`) extracted into the shared function `check_register_budget()`, called at both locations                                                                                                                                                                                                  | Corpus diff + snapshot (zero-diff possible in pure move phase)           |
| **1c. Three statement-level recovery criteria**     | **Yes**                    | Delete all three (`1954-1959` / `3632-3638` / `4737-4742`), including the `result_reg.is_none()` guard at `3632`                                                                                                                                                                                                                                                            | C4: behavioral equivalence + invariants                                  |
| **2. `arg_regs` semantic rearrangement**            | **Partial**                | SSA turns "using the wrong register" into "using the wrong value ID" and can be caught by dominance checks, but **the rearrangement logic itself remains**. Real elimination depends on function decomposition — each of the 6 branches `push`es, and `arg_regs` ownership stays at the top of the function                                                                 | First-layer dominance invariant (snapshot is immune to this)             |
| **3. Span-keyed failure**                           | **No**                     | SSA does not touch this layer at all. See "Additional Mechanisms for Span-Keyed"                                                                                                                                                                                                                                                                                            | 07's `test_release_plan_spans_consumed`                                  |
| **4. Implicit order dependency**                    | **No**                     | See "`synth.rs` Boundary"                                                                                                                                                                                                                                                                                                                                                   | Requires new explicit criteria                                           |
| `compile_pattern` 298 lines                         | No                         | See "What SSA Will NOT Solve"                                                                                                                                                                                                                                                                                                                                               | —                                                                        |
| `eval_const_expr` 191 lines                         | No                         | See "What SSA Will NOT Solve"                                                                                                                                                                                                                                                                                                                                               | —                                                                        |
| `Instruction` 76 variants                           | No (+1)                    | Add `Phi` → 77                                                                                                                                                                                                                                                                                                                                                              | —                                                                        |

### Register Allocator: Self-Developed or Use Existing Crate

**Verified facts (decisive)**:

1. `Operand::Register(u8)` (`ir.rs:19`) has **0 construction points in the entire repository**.
2. The backend is not a register machine — `src/middle/passes/codegen/operand.rs`'s
   `OperandResolver` resolves `Operand::Local/Temp/Arg` to `u8`, with an upper limit of 255 (the
   `Temp` arm overflow at `39-45` is E3014).
3. The output of `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1577 lines, 50
   `translate_*`, function bodies distributed at `101-1558`) is
   `BytecodeInstruction::new(opcode, operands)`, with the opcode coming from
   `src/backends/common/opcode.rs` (83 constants).
4. `src/backends/interpreter/executor/ops/` is the interpreter's opcode dispatch family.

**Conclusion**: The so-called "register" is **an operand stack slot index, not a physical
register**. The target machine **has no finite register file, no register pressure, no spill/split
requirements**.

**Evaluate three approaches**:

| Approach                                                   | Evaluation                                                                                                                                                                                                                                                                                                                                                                                             |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `regalloc` / `regalloc2` (rustc lineage)                   | **Not adopted.** Both take rustc's `Function` trait + MIR's `Place`/`Operand` abstraction as input. Writing an adapter layer for YaoXiang IR requires implementing the complete trait (CFG, liveness, spill slot, scratch), **500+ lines of adapter code** for zero benefit — because there is no real register file to color. And rustc itself only does linear scan here (when `enable-llvm` is off) |
| `graph-alloc` series                                       | **Not adopted.** Same as above, and the API is even less stable                                                                                                                                                                                                                                                                                                                                        |
| **Self-developed linear scan (Chaitin-Briggs simplified)** | **Adopted.** What YaoXiang actually needs is: map each `ValueId` to a non-conflicting `u8` stack slot in `values` order (`OperandResolver` already has the `u8` contract and the E3014 upper limit). This is essentially **maximum clique coloring of non-overlapping live ranges**, which under no register file constraints degenerates to "assign the first available slot in definition order"     |

**Design judgment**: Self-developed, target 500-1000 lines (including `values → Reg` mapping, live
range calculation, interface with `OperandResolver`). **At the same time, it must be admitted: if
the `u8`/255 slot upper limit is not enough under real load, the self-developed allocator has to
handle overflow, while the existing code already treats it as an error at `1992-2000`.** This means
this document **does not solve** the "not enough registers in large functions" problem — **that is a
downstream consequence of the recursion depth of `compile_pattern`, a separate issue.**

`Cargo.toml:37-39`'s dev-dependencies are only `criterion` / `proptest`; the `[dependencies]`
section has no regalloc / petgraph / graphlib — introducing any crate would add to the dependency
surface.

### Decomposition Strategy for the 746-line `generate_call_expr_ir`

**Must be done last**, and **only after the SSA form and criteria are in place**. Reason: the
decomposition itself is C1 (pure move, criterion is snapshot zero-diff), but only after the
`arg_regs` semantics have been reduced by SSA to "each argument has an independent `ValueId`", the
decomposition will not move the old order assumptions intact into the new file.

**Core design constraint: `arg_regs` ownership stays at the top of the function, the 6 branches only
`push`, never rearrange.**

```
generate_call_expr_ir (746 lines)
├── Argument evaluation: evaluate in source order, push to arg_regs — shared by 6 branches, the only place where append is allowed
├── CallArgs (value object)
│   └── arg_regs: Vec<Operand>  — only created at the top, 6 branches only push, never re-collect/reorder
├── Branch 1 → emit_namespace_call(args: CallArgs)         [7395 branch extracted]
├── Branch 2 → emit_bound_call(args: CallArgs)              [7406+ branch extracted]
├── Branch 3 → emit_struct_ctor(args: CallArgs)             [7724 branch extracted]
├── Branch 4 → emit_plain_call(args: CallArgs)              [7835-7848 branch extracted]
├── Branch 5 → emit_curried_call(args: CallArgs)
└── Branch 6 → emit_method_call(args: CallArgs)             [7420 / 7493 branch extracted]
```

The "supplement 0" logic at `7835-7848` (`7838-7846`) descends into
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, **and replaces the
self-stated comment "normal paths will not reach here" with an assertion**: if any slot is missing
and typecheck has not reported E1010, directly return a diagnostic. **This turns a silent fallback
into a hard error, and is the most direct improvement of this document on defect category 2.**

**Criterion**: The pure move part (SSA form unchanged) uses snapshot zero-diff; the
assertion-ification of `fill_missing` belongs to C4, using corpus diff + first-layer validator.

### `synth.rs` Boundary: Turning Implicit Callbacks into Explicit Synthesized AST Boundaries

**Verified fact**: The locations in `ir_gen.rs` that construct `ast::Expr` are **exactly 2** —
`:4246` and `:6701` (additionally `:6704` constructs `ast::Stmt`, `:510` / `:1266` / `:1399`
construct `ast::Type`).

**Proposal**: Create a new `src/middle/lower/synth.rs`, as the **only** module in the entire
repository that is allowed to construct `ast::Expr` / `ast::Stmt` / `ast::Type` in the L3 layer. The
reason is not "tidiness", but:

> Constructing AST means **re-entering the frontend lowering entry**. And `generate_expr_ir`'s
> expected `pending_env_vars` / `closure_captures` state is a **precondition that the caller is
> responsible for preparing**, and this precondition is currently not expressed by any type or
> signature — it holds based on the runtime fact of "who calls me".

Change to explicit signature:

```rust
// src/middle/lower/synth.rs
pub struct SynthEnv {
    pub env_vars: Vec<Operand>,
    pub env_names: Vec<String>,
    pub captures: HashMap<String, usize>,
}

/// Synthesize a closure AST capturing env; the capture set is given explicitly by parameter,
/// not depending on the caller's prior writing to self.pending_env_*
pub fn synth_closure(
    params: Vec<ast::Param>,
    body: ast::Block,
    env: SynthEnv,
    span: Span,
) -> ast::Expr
```

Specific changes:

| Location                       | Current state                                                                                  | Change to                                                                                                                                                                                           |
| ------------------------------ | ---------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ir_gen.rs:4245-4256`          | Write `pending_env_vars` → build Lambda → callback                                             | Call `synth::synth_closure(...)` to get AST, explicitly pass `SynthEnv`; `generate_lambda_expr_ir` (`6543`) adds an internal entry carrying `SynthEnv`, **no longer reads `self.pending_env_vars`** |
| `ir_gen.rs:6698-6714`          | Write → build → callback → clear                                                               | Same as above; the `clear()` at `6713-6714` disappears (no shared state to clear)                                                                                                                   |
| `ir_gen.rs:6571-6572`          | `mem::take(&mut self.pending_env_vars)`                                                        | Change to read from parameter                                                                                                                                                                       |
| `ir_gen.rs:6589-6593` / `6600` | Write / clear `closure_captures`                                                               | Same as above; `closure_captures` downgrades to **local variable during closure body generation**, the `clear()` at `6600` disappears                                                               |
| `ir_gen.rs:458`                | `self.closure_captures.contains_key(head)` reads shared field on the namespace resolution path | **This is the most dangerous one**: change to explicit parameter, otherwise any `?` early return of `generate_lambda_body_ir` will leak the capture table to unrelated code                         |

**Criterion**: New sentinel test `test_no_ast_construction_outside_synth`, scanning `src/middle/**`
except `synth.rs` must not contain `ast::Expr::` / `ast::Stmt {` construction syntax. Form same as
RFC-039's `check-module-boundary.py`.

### Disposition of Span-Keyed Issues (What SSA Cannot Solve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism One: Consumption Coverage Assertion (mandatory, lowest cost).** 07 has defined
`test_release_plan_spans_consumed` (the Span set produced by ownership ⊆ the Span set consumed by
IR). The supplement in this document is to make it a **runtime count** rather than just a test:

```rust
// around ir_gen.rs:341
// Record how many plans typecheck delivered
self.release_plan_total = type_result.release_plan.drops.len();
// Each time :1942 hits, self.release_plan_hit += 1;
// At the tail of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* new diagnostic code: E3xxx NLL release plan not consumed by IR */);
//   }
```

**Why this works**: `drop()` instructions on non-`ref` locals are runtime-observable (destructor
side effects), but **"a drop that should have happened didn't happen" does not necessarily crash
immediately under YaoXiang's current value semantics + Arc/Rc model** — this is exactly why it is
silent. Counting turns "silent" into "detectable".

**Mechanism Two: Span Key Changed to `DefId` or Explicit Plan ID (mid-term).** The fundamental fix
is to **not use Span as a key**. `Span` is a source code location, which drifts due to macro
expansion, `include`, multi-file merging. `FunctionIR` already has the `def: Option<DefId>` field
(`ir.rs:677`), showing that DefId is available at the IR layer. **Design judgment**: Introduce
`PlanId(u32)` for `ReleasePlan` and `overload_resolutions`, allocated by ownership / overload
resolution at production time, and matched by `PlanId` when consumed by IR. **This belongs to the
obligations ledger of 02 (`Obligations`) and should be implemented together with
[02](02-stage-contract.md), not in this document.**

**Mechanism Three: `method_def_ordinals` Definition Order Dependency (independent small item).**
This field is `HashMap<String, usize>` (`ir_gen.rs:224`), with key being
`"{type_name}.{method_name}"` (**no module qualifier**, written at `:1691-1700`) — if two modules
define a same-named `Type.method`, they share the same counter, and the `#N` mangled name drifts
accordingly. **Its failure mode is not span mismatch but order/name mismatch** — if typecheck's
registration order is inconsistent with ir_gen's definition order, the same-named method resolves to
different targets. Disposition: Change to typecheck side **one-shot production** of
`HashMap<DefId, String>` (bare name or `#N` mangled name), ir_gen only queries, not modifies. **This
eliminates a mutable cross-function state inside `ir_gen`.**

### Tracking the Three Hardcoded Discards

**Verified fact.** `impl From<BytecodeFile> for BytecodeModule` (`bytecode.rs:943-2350`, 1408 lines)
has three hardcoded discards:

| Location           | Field                | Value                                          | Consequence                                                                                                                    |
| ------------------ | -------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `bytecode.rs:2312` | `upvalue_count`      | `0` // Not stored in BytecodeFile              | Closure upvalue count is zeroed after `.42` roundtrip                                                                          |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet            | **`.42` output loses the entire exception table**                                                                              |
| `bytecode.rs:2341` | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **`.42` output loses global variable information** (note that `2347`'s `global_names` **is** filled, the two are inconsistent) |

**Design judgment (revised 2026-10-05, decision D52): All three are brought into the scope of this
document, as P7 batch e (7e), no longer filed as "independent issue".** The original judgment (the
latter two filed as independent issues) and the reasons for it are recorded below, along with why
they were overturned:

1. ~~They are **format layer gaps** (`BytecodeFile` does not serialize these fields), unrelated to
   SSA.~~ True, but not a reason for exclusion — P7 already touches `bytecode.rs`, and fixing all
   three at once only needs one `VERSION` bump (4→5), splitting into two version bumps is actually
   more expensive.
2. ~~Fixing them requires changing the `.42` disk format → version bump → scope expands from "IR
   form" to "artifact format".~~ The version bump mechanism already exists (`MAGIC` + `VERSION`
   header, the read side rejects by version), and `.42` is a build artifact, cross-version read
   compatibility is not the goal — what expands is the work, not the risk.
3. ~~`2315` / `2341` actually have a low risk level, `.42` direct execution is a secondary path.~~
   **Overturned**: Loss of the exception table makes throw/try behave incorrectly on `.42` direct
   execution — throw/try is core language semantics; secondary path does not mean silent errors are
   acceptable. The principle of this refactoring is **no leftover core functionality unimplemented**
   (`Vec::new() // Not implemented yet` is exactly this kind of leftover).

**Also kept**: All three must still be registered in the `Obligations` ledger as "produced but not
serialized" until 7e is completed, to prevent it from becoming a fourth "thing no one knows about".
`2312` lands with the version bump (D17), and the assigned-but-unused opcode values (D32/D34) are
reclaimed in the same batch.

## Detailed Design

### Cascade Effects of IR Structure Changes

#### `src/middle/core/ir.rs` (905 lines)

| Change                                                               | Location            | Nature                                            |
| -------------------------------------------------------------------- | ------------------- | ------------------------------------------------- |
| Add `ValueId(u32)` type                                              | Near file head `10` | New                                               |
| `Operand` delete `Temp` / `Label` / `Register`, add `Value(ValueId)` | `12-20`             | **Variant add/delete**                            |
| `Instruction` add `Phi` variant                                      | End of `47-533`     | **Variant +1 → 77**                               |
| `FunctionBody::Code` add `values: Vec<ValueInfo>`                    | `636-642`           | Field +1                                          |
| Add `ValueInfo` struct                                               | Around `660`        | New                                               |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`              | `689-719`           | Need to add `values()` / `values_mut()` accessors |

The `pub use ...ast::Type` at `ir.rs:3` **is not touched by this document** — it is the scope of
[03](03-type-unification.md). But `ValueInfo::ty` must use the **converged** single type.

#### `src/middle/core/bytecode.rs` (2422 lines)

| Change                                                         | Location                              | Nature                                                                                        |
| -------------------------------------------------------------- | ------------------------------------- | --------------------------------------------------------------------------------------------- |
| `BytecodeInstr` add `Phi` or "runtime expand to Move" strategy | `127-554` (currently **66** variants) | Variant +1 or +0 (see below)                                                                  |
| `opcode()` add `Phi` arm                                       | `558`                                 | Required (`BytecodeInstr` exhaustive match)                                                   |
| `size()` add `Phi` arm                                         | `649`                                 | Required                                                                                      |
| `From<BytecodeFile>` decoding match add `Phi` arm              | `943-2350`                            | Required                                                                                      |
| `upvalue_count: 0` fix                                         | `2312`                                | This document fixes along the way (in-memory format; disk requires `VERSION` bump, see below) |

**The bytecode form of `Phi` requires a design decision.** Two options (**design judgment, recommend
A**, reasons in "Key Decisions and Reasons"):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded in `translator.rs` into a
  series of `Move { dst: vreg, src: incoming[i] }` at the start of that block, selected by the
  predecessor block index. **No new opcode**, `.42` format zero changes. Cost: increased bytecode
  volume (one per predecessor).
- B: Add new `PHI` opcode + predecessor index encoding, interpreter maintains register versions on
  jump. **Requires changing `executor/` state model + `.42` format + version number**. Benefit is
  only volume.

Decisive reason for choosing A: Changing the `.42` format would trigger version number issues and
full compatibility discussions, **and A's volume cost is irrelevant in the interpreted execution
scenario** (what executes is Move, not Phi).

#### `src/middle/passes/codegen/translator.rs` (1577 lines, 50 `translate_*`, distributed at `101-1558`)

| Change                                             | Location                                                                                                                         | Nature                                                                      |
| -------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `Operand::Value(v)` dispatch                       | `OperandResolver` (`codegen/operand.rs`)                                                                                         | Add arm                                                                     |
| `Instruction::Phi` translation arm                 | Inside dispatch match, before `Free`(533)                                                                                        | New branch, expand to Move series (option A)                                |
| Existing 9 NOP degradation **positions unchanged** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **Verified as 9**: `655-657` covers 2 variants, `658-660` covers 4 variants |

**SSA-ization will not reduce the NOP count.** These 9 degradations reflect the expression
capability gap of "IR has, bytecode does not", which SSA-ization does not touch.

#### `src/backends/interpreter/executor/`

If option A is adopted, `executor/` **needs no changes at all** — `Phi` disappears from what the
interpreter can see at the `translator.rs` stage. **This is the second decisive reason for choosing
A.**

`executor/ops/control.rs:90`'s `BytecodeInstr::Switch` has an active implementation, but **no**
`Switch` / `BrTable` / `JumpTable` variants exist in `ir.rs` (verified: zero hits for these three
names in `ir.rs`) — **this is a reverse gap**: the bytecode layer has instructions the IR layer
cannot produce. It does not affect this document, but is recorded here because it shows that "IR →
bytecode" is not surjective.

The dispatch match at `executor/debug.rs:194` likewise needs no changes (no new opcode under option
A).

### Compiler Change List

**File by file, function by function.** All line numbers are the **starting point** of the change
(verified):

| No. | File                                      | Location                                                  | Change                                                                                                             |
| --- | ----------------------------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| 1   | `src/middle/core/ir.rs`                   | `12-20`                                                   | `Operand` variant add/delete                                                                                       |
| 2   | `src/middle/core/ir.rs`                   | End of `47-533`                                           | Add `Instruction::Phi`                                                                                             |
| 3   | `src/middle/core/ir.rs`                   | `636-642`                                                 | `FunctionBody::Code` add `values`                                                                                  |
| 4   | `src/middle/core/ir.rs`                   | Around `660`                                              | Add `ValueInfo`                                                                                                    |
| 5   | `src/middle/core/ir.rs`                   | `689-719`                                                 | Add `values()` / `values_mut()` accessors                                                                          |
| 6   | `src/middle/core/ir.rs`                   | `3`                                                       | **`pub use ast::Type` not touched** ([03](03-type-unification.md) scope, recorded only)                            |
| 7   | `src/middle/core/ir_gen.rs`               | `884-891` `next_temp_reg`                                 | Return `Operand::Value`, monotonically increasing, no rollback                                                     |
| 8   | `src/middle/core/ir_gen.rs`               | `1954-1959`                                               | **Delete** statement-level recovery                                                                                |
| 9   | `src/middle/core/ir_gen.rs`               | `3632-3638`                                               | **Delete** statement-level recovery (including `result_reg.is_none()` guard)                                       |
| 10  | `src/middle/core/ir_gen.rs`               | `4737-4742`                                               | **Delete** statement-level recovery                                                                                |
| 11  | `src/middle/core/ir_gen.rs`               | `1745-1759` / `1819-1822`                                 | Switch to RAII guard (`generate_method_ir`, entry `1677`)                                                          |
| 12  | `src/middle/core/ir_gen.rs`               | `1858-1866` / `2026-2029`                                 | Switch to RAII guard (`generate_function_ir`, entry `1830`)                                                        |
| 13  | `src/middle/core/ir_gen.rs`               | `2116` / `2180`                                           | Switch to RAII guard (`generate_curry_innermost_func`, entry `2105`)                                               |
| 14  | `src/middle/core/ir_gen.rs`               | `2224-2229` / `2284-2288`                                 | Switch to RAII guard (`generate_curry_function_ir`, entry `2209`)                                                  |
| 15  | `src/middle/core/ir_gen.rs`               | `2768-2777` / `2820-2851`                                 | Switch to RAII guard (`generate_anon_binding_ir`, entry `2759`)                                                    |
| 16  | `src/middle/core/ir_gen.rs`               | `4686-4695` / `4767-4774`                                 | Switch to RAII guard (`generate_lambda_body_ir`, entry `4678`)                                                     |
| 17  | `src/middle/core/ir_gen.rs`               | `1987-2000`                                               | E3014 check extracted into `check_register_budget()`, shared by `1798`/`2001` after convention unification         |
| 18  | `src/middle/core/ir_gen.rs`               | `1798`                                                    | `take_cur_locals(param_types.len())` → `take_cur_locals(self.temp_high_water)`, eliminate convention inconsistency |
| 19  | `src/middle/core/ir_gen.rs`               | `248-249`                                                 | Comment contract deleted (enforced by type system), replaced with a note pointing to `verify`                      |
| 20  | `src/middle/core/ir_gen.rs`               | `4245-4256`                                               | Change to call `synth::synth_closure`                                                                              |
| 21  | `src/middle/core/ir_gen.rs`               | `6698-6714`                                               | Change to call `synth::synth_closure`, delete `6713-6714`                                                          |
| 22  | `src/middle/core/ir_gen.rs`               | `6571-6572`                                               | Change to read `SynthEnv` from parameter, no longer `mem::take` shared field                                       |
| 23  | `src/middle/core/ir_gen.rs`               | `6589-6593` / `6600`                                      | `closure_captures` downgrades to local, delete `clear()`                                                           |
| 24  | `src/middle/core/ir_gen.rs`               | `458`                                                     | `closure_captures.contains_key` changes to explicit parameter (**highest priority implicit dependency**)           |
| 25  | `src/middle/core/ir_gen.rs`               | `341` + `1942`                                            | Add span-keyed consumption count                                                                                   |
| 26  | `src/middle/core/ir_gen.rs`               | `224` / `369` / `1692-1702`                               | `method_def_ordinals` changed to read-only                                                                         |
| 27  | `src/middle/core/ir_gen.rs`               | `7269-8014`                                               | **Do last**: split into top-level `CallArgs` + 6 `emit_*`                                                          |
| 28  | `src/middle/core/ir_gen.rs`               | `7838-7846`                                               | "Supplement 0" fallback changed to return diagnostic                                                               |
| 29  | `src/middle/lower/synth.rs`               | **New file**                                              | Synthesized AST boundary                                                                                           |
| 30  | `src/middle/ir/verify.rs`                 | **New file** ([07](07-equivalence-oracle.md) first layer) | `verify_loose` / `verify_ssa`                                                                                      |
| 31  | `src/middle/passes/codegen/translator.rs` | dispatch match                                            | Add `Instruction::Phi` arm (option A: expand to Move series)                                                       |
| 32  | `src/middle/passes/codegen/operand.rs`    | `36-45` / `72-77`                                         | `Operand::Value` resolution arm; delete `Temp` arm (`39-45`)                                                       |
| 33  | `src/middle/core/bytecode.rs`             | `127-554`                                                 | **Unchanged under option A** (`Phi` does not enter bytecode)                                                       |
| 34  | `src/middle/core/bytecode.rs`             | `558` / `649` / `943-2350`                                | **Unchanged under option A**                                                                                       |
| 35  | `src/middle/core/bytecode.rs`             | `2312`                                                    | `upvalue_count: 0` fix                                                                                             |
| 36  | `src/backends/interpreter/executor/**`    | —                                                         | **Zero changes under option A**                                                                                    |
| 37  | `src/middle/passes/regalloc.rs`           | **New file**                                              | Linear scan allocator                                                                                              |
| 38  | `scripts/ci/check-synth-boundary.py`      | **New file**                                              | `synth.rs` boundary sentinel check                                                                                 |

### Backward Compatibility: Version Number of `.42` Format

**Verified fact**: The `.42` disk format **already has a version number mechanism** —
`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` ("YXBC") and
`VERSION: u32 = 4`, written to the header (`:342-343`), validated on read (`:454-467`, mismatch
reports "unsupported bytecode version").

**Design judgment (direct corollary of option A)**:

- `Phi` is expanded into a `Move` series at the `translator.rs` stage, **no new opcode is
  produced**.
- The 83 constants in `opcode.rs` are **zero changes**, `opcode_name()` (`120`) is unchanged, the
  decoding match (`bytecode.rs:943-2350`) is unchanged.
- The field layout of the `.42` format is **zero changes**, `VERSION` stays at 4. Old `.42` files
  can still be read by new binaries, new `.42` files can still be read by old binaries.
  **SSA-ization itself does not require a version bump.**

**The only scenario requiring a version bump is the fix of `2312 upvalue_count`** (decision D17:
fix, and handle along with the version bump): add the `upvalue_count` field to `BytecodeFunction`
and have the encoder write it, which is a **new field in the format**, bumping `VERSION` from 4
to 5. The existing version validation on the read side rejects old files per existing behavior —
`.42` is a build artifact (`main.rs:671`), cross-version read compatibility is not the goal, the
version bump requires no additional migration mechanism. Assigned-but-unused opcode values
(D32/D34's `Switch` / `TailCall`) are reclaimed along with the version bump.

## Implementation Key Points

### Batch Strategy

Four batches. **Each item within a batch is an independent commit, independent criterion,
independent revert.** The "change item" column in the table below points to the number in the
"Compiler Change List" above.

| Batch       | Content                                                                                                                                               | Change items                 | Criterion                                                                                                                                                      | Rollback point                                                                                                                     |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Cut off multiple definitions: delete 3 statement-level recoveries → 6 save/restore switch to RAII guard → unify conventions                           | 8-10 (first) → 11-16 → 17-18 | Delete recovery: **C4** (behavioral equivalence + first-layer dominance invariant). RAII and convention unification are pure move: **C1** snapshot zero-diff   | Per-item single commit, `git revert` rolls back                                                                                    |
| **Batch b** | SSA form switch: `ir.rs` structural change + `Operand` mechanical replacement (323 places) → add `Phi` arm to `translator.rs` → linear scan allocator | 1-5, 7 → 31-32 → 37          | **C4**: behavioral equivalence + `verify_ssa` all green + `.42` roundtrip test                                                                                 | `ir.rs` change **occupies a single commit** (type change affects full repository compilation, cannot be incrementally rolled back) |
| **Batch c** | Implicit contract explicitness: `synth.rs` boundary + sentinel script → span consumption count → `method_def_ordinals` read-only                      | 20-24, 29, 38 → 25 → 26      | **C4**: behavioral equivalence + new script incorporated into CI; consumption count zero triggers on new diagnostic (otherwise indicates real mismatch exists) | Independent commit                                                                                                                 |
| **Batch d** | `generate_call_expr_ir` decomposition + "supplement 0" fallback assertion-ification                                                                   | 27-28                        | Pure move part: **C1** snapshot zero-diff. Assertion-ification part: **C4** corpus diff + first-layer validator                                                | Independent commit                                                                                                                 |

**Three dependencies within batches whose order cannot be swapped**:

1. Batch a's "delete recovery" must precede batch b's `ir.rs` change — first let `next_temp`
   increase monotonically, then `ValueId` has the semantic foundation of "single definition".
2. Batch b's `Phi` arm must precede batch d's decomposition — otherwise the decomposition will move
   the old `arg_regs` order assumptions intact into the new file.
3. Batch d must be last — it depends on the previous two batches having already converged `arg_regs`
   semantics to "each argument has an independent `ValueId`".

**Position relative to RFC-039**: All four batches above fall within RFC-039's **P7**, and the
criteria grading (`verify_loose` / `verify_ssa`) is established by P2. Between batches is
**serial**, but within batch a, between 8-10 / 11-16 / 17-18 there are only order constraints, no
coupling.

### Prerequisites (Hard)

**Prerequisite One: Criteria Baseline.** [07](07-equivalence-oracle.md)'s first-layer validator's
`verify_loose` mode **must run green on existing (non-SSA) IR first** before batch a is allowed to
proceed.

Reason: The criterion for SSA-ization is C4 (behavioral equivalence + invariants). If the invariant
validator itself cannot run green on existing IR before the refactor, then every acceptance of batch
a ~ batch d lacks an executable criterion, and can only rely on corpus diff — and corpus diff is
**precisely insensitive** to defect categories 1/2/3 (demonstrated in 07 "Alternative Plan A").
**This is a real ordering constraint.**

**Prerequisite Two: Type Representation Convergence.** [03](03-type-unification.md) must complete
the convergence of `pub use ast::Type` at `ir.rs:3` and the removal of the
`From<MonoType> for IrType` bridging at `bytecode.rs:2353`. **Otherwise `ValueInfo::ty` will
introduce a third set of type compatibility judgments.**

### Rollback Strategy

- Each change item is an independent commit, `git revert` granularity = change item granularity.
- Batch b's `ir.rs` structural change is the **only part that cannot be incrementally rolled back**
  (type change will affect full repository compilation). Therefore it must **occupy a single
  commit**, and before it, batch a must have run green with a revert-able baseline.
- **No feature flags.** Reason: `Operand` variant add/delete cannot be isolated with runtime
  switches; feature flags would cause CI to not test the other path for a long time — this is
  exactly the recurrence of "test wiring relies on human memory, corruption silently happens"
  recorded in RFC-039.

### Expectation Management: Net Line Increase of 900-1600 Lines

**Must be stated up front, otherwise it will be questioned by "lines did not decrease" midway
through implementation.**

| Item                                  | Line change                                                                                                                                                             |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                        | `ir_gen.rs` **8448**                                                                                                                                                    |
| Delete 3 statement-level recoveries   | **−15**                                                                                                                                                                 |
| 6 save/restore switch to RAII         | Net ±0 (RAII guard definition ~+20, use sites from 22 lines down to ~12 lines)                                                                                          |
| `next_temp_reg` refactor              | +5                                                                                                                                                                      |
| `generate_call_expr_ir` decomposition | **+40 ~ 80** (overhead of signatures and parameter passing for 6 `emit_*`; this is the typical cost of pure move decomposition, **decomposition increases line count**) |
| Span-keyed consumption count          | +15                                                                                                                                                                     |
| `method_def_ordinals` read-only       | +20                                                                                                                                                                     |
| `synth.rs` new file                   | +80 ~ 120                                                                                                                                                               |
| `verify.rs` new file                  | +400 ~ 600 (07 first layer, in this document's scope but counted independently)                                                                                         |
| `regalloc.rs` new file                | **+500 ~ 1000**                                                                                                                                                         |
| **Total**                             | **`ir_gen.rs` ~8448 → 8600 ~ 8800; including new files, net increase 900 ~ 1600 lines**                                                                                 |

**The early expectation of "8448 → 5000~6000" was wrong**, the actual measured derivation result is
the opposite. Need to explain why:

1. **The core benefit of this document is not reducing line count, it is eliminating an entire
   category of "wrong with no error" defects.** The 6 manual save/restore (22 lines) become ~12
   lines + a guard type after SSA + RAII — **the line count is almost unchanged, but "forgetting to
   restore" changes from a possibility to a compile error**.
2. **Pure move decomposition necessarily increases line count.** Splitting `generate_call_expr_ir`
   (746 lines) into 7 functions, each must repeat the signature (this function already has
   `#[allow(clippy::too_many_arguments)]`, `7268`), `&mut self` binding, error propagation.
   **"Splitting files reduces line count" is a misconception — it reduces cognitive load, not
   physical line count.**
3. **The new verifier and allocator are net line additions.** These two (~900-1600 lines) are
   precisely **the carrier of the benefit**.

**Honest statement of net benefit**:

> Line count-wise, this document is most likely a **net increase**. The benefit is in
> **verifiability** — turning "wrong with no error" defects into "can't pass without reporting"
> invariants. If line count is used as the acceptance criterion, this document will fail; the
> correct acceptance criterion is the "Batch Strategy" criteria.

**This stage will cause the line count of `ir_gen.rs` to net increase, and it is intentional.** The
2026-10-03 decision has cancelled all line/volume gates (see [08](08-maintenance-mechanism.md)
prohibition two), therefore **no line waiver list is needed** — the legitimacy of the increase comes
from the responsibility separation argument: SSA-ization separates "register allocation + definition
discipline" from lowering into independent modules, and the line count increase is in exchange for
eliminating 6 manual save/restore and the `arg_regs` semantic rearrangement, two categories of
manual discipline where "misalignment reports no error, only wrong values".

## Key Decisions and Reasons

### Three Core Decisions

**Decision One: Temporary Value Strategy — Delete the Three Statement-Level Rollbacks, Let Single
Definition Become a Mathematical Consequence of Construction.** The current rollbacks (`1954-1959` /
`3632-3638` / `4737-4742`) exist to reuse temporary slots. Delete them, and `next_temp` increases
monotonically, with each temporary slot naturally having only one definition point. **Hand-copied
differences in criterion consistency but different guards** (`3632` has `result_reg.is_none()`
guard, the other two do not) disappear accordingly. The cost is increased slot occupation —
`temp_high_water` already has the 255 upper limit check (`1992-2000`) as a backstop.

**Decision Two: Do Not Do mem2reg, Only Do Explicit Renaming.** Reasons already given in "Value
Identification Approach": YaoXiang's `&` / reference / ownership semantics make "which slots can be
mem2reg-ed" a type system problem, requiring the converged type representation from
[03](03-type-unification.md) to judge. **Doing mem2reg before 03 is guessing.** Explicit renaming
only requires changing the return value semantics of `next_temp_reg` at one place + deleting the
three rollbacks, without touching the control flow of any lowering algorithm.

**Decision Three: `Phi` Adopts Option A (IR-only Pseudo-Instruction, Expanded to Move Series).**
`executor/` and `.42` format have **zero changes**, the regression surface is limited to L3.
Decisive reason: Changing the `.42` format would trigger version number issues and full
compatibility discussions, while A's volume cost is irrelevant in interpreted execution (what
executes is Move, not Phi). The cost is that `Phi` is invisible in `dump_bytecode` (need to be
careful when entering the C1/C2 phase that 07's third layer compares against it).

### Rejected Alternatives

**A. All-at-once Decomposition (Do SSA + Decomposition + `synth` Boundary + Allocator
Simultaneously). Rejected.** Violates 07's core principle: criteria must be graded by category.
Mixing four batches in one PR, mixing pure move changes with C4 changes together, snapshot drift
cannot be attributed — once the snapshot changes, you don't know whether the decomposition moved
things wrong or SSA changed things wrong. **Rollback granularity degrades from "change item" to
"everything".**

**B. Split Files First Then Optimize Order. Rejected.** Decomposition does not eliminate any root
cause: 6 manual save/restore are still scattered across 10 files, `arg_regs` semantic rearrangement
still requires manual reasoning. Worse, **decomposing first loses the opportunity to "build criteria
before changing"** — decomposition produces a large number of line-level diffs, mixed into the diff
of subsequent SSA changes, making review ineffective. This is the same source as the 8 orphan test
trees (1005 lines / 78 tests never run) recorded in 06-cleanup-inventory.md: **order is wrong, every
subsequent step accelerates on the wrong basis.**

**C. Introduce an Intermediate SSA Layer Without Changing the Existing Lowering. Rejected, but
recorded as "the third path once considered".** The form is to keep `ir_gen.rs` unchanged and add a
new pass that converts its non-SSA IR output to SSA IR. **The advantage** is that lowering is
completely untouched, and rollback is easy. **Reason for rejection**: (1) It does not eliminate the
root cause of defect category 1 — the three statement-level recoveries are still in `ir_gen.rs`, and
the 6 save/restore are still 59-161 lines apart; the conversion pass can merge multiple definitions
into `Phi`, but **it cannot fix "inner function names leaking to the outer layer"** — that is a
`cur_locals` save/restore error, which occurs before the IR form, and the conversion pass cannot see
it. (2) It turns SSA into a second representation — the repository already has the precedent of 3
parallel type representations, and the core lesson of this project is "**boundaries only exist in
human awareness**". (3) `verify_loose` becomes a permanent burden.

**D. Keep the Status Quo, Only Add a Verifier. Rejected as the only plan, but it is a component of
this document.** RFC-039 Alternative Plan B has given the same judgment: gating can prevent
regression, it cannot fix the status quo. **But we must clearly state what D can do**: D can
**catch** **instances** of defect categories 1/2/3 (provided the verifier covers dominance, type
consistency, inner isolation), but cannot catch defect category 4 (`closure_captures` leak) — that
requires a cross-function liveness assertion, which goes beyond the function boundary of
`verify(&ModuleIR)`. **Therefore D and this document are not in a substitution relationship, but in
a prerequisite relationship.**

## Known Limitations and Risks

### Risks

- **Net line increase of 900–1600 lines** (new `verify.rs` / `regalloc.rs`). **This is not a risk
  point** — the 2026-10-03 decision has cancelled line/volume gates, and the scale issue is solved
  by responsibility separation (see [08](08-maintenance-mechanism.md) prohibition two). The
  legitimacy of the increase lies in: it replaces 6 manual save/restore and `arg_regs` semantic
  rearrangement, two categories of manual discipline where "misalignment reports no error, only
  wrong values".
- **`verify.rs`'s "unique definition" check is meaningless under non-SSA form** (`Operand::Local`
  allows multiple definitions) — 07 has recorded this limitation, and `verify_loose` is its degraded
  mode before this batch.
- **Deleting statement-level recovery will change the temporary slot allocation pattern** (batch a).
  Even if `temp_high_water` guarantees no overflow, **the allocation result will change** — this is
  exactly why the C4 phase "behavioral equivalence" criterion must really run the full corpus,
  rather than spot checks.
- **Whether the `u8` / 255 slot upper limit** (`codegen/operand.rs:39-45`) is still sufficient after
  deleting rollbacks, **is unverified**. The E3014 check in `generate_function_ir` (`1992-2000`)
  will catch it, but catching means compilation failure — which may expose a batch of oversized
  functions previously masked by "rollback".
- **The advantage must be recorded alongside**: turning "untestable errors" into "testable
  invariants"; not changing the backend (`executor/` and `.42` have zero changes); not changing the
  semantics of `Instruction` (76 → 77 variants, all preserved, all 50 `translate_*` preserved);
  incremental (each item independently revert-able).

> **The open questions originally listed in this section have all been adjudicated.** See
> [RFC-039 Decisions Registry](../../rfc/draft/039-compiler-architecture.md) for item-by-item
> decisions (D1–D50). **This document leaves no open items.**

## See Also

### Documents

- [RFC-039 Compiler Architecture Refactoring](../../rfc/draft/039-compiler-architecture.md) —
  Upper-level general outline; four-layer model, criteria grading, P1-P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **Hard prerequisite**; C4 category
  criterion definition, `verify_loose` / `verify_ssa`, three-layer criteria, snapshot immunity
  limitation to `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **Hard prerequisite**; convergence of 3
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; `PlanId` refactor attribution
  for `ReleasePlan`
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — 8 orphan test trees; the negative example of
  "create directories first, then wiring fails"

### Code Locations

**IR Definition**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` ([03](03-type-unification.md) scope)
- `src/middle/core/ir.rs:12-20` — `Operand` 7 variants
- `src/middle/core/ir.rs:47-533` — `Instruction` 76 variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (including `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (including `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (including `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (feasibility basis for `PlanId`
  plan)
- `src/middle/core/ir.rs:689-719` — `all_instructions` and other accessors

**Defect Category 1 (save/restore)**

- `src/middle/core/ir_gen.rs:248-249` — Plain text contract "must be physically adjacent to the
  save/restore point of next_temp"
- `src/middle/core/ir_gen.rs:238-242` — `cur_span`'s same-style comment convention
- `src/middle/core/ir_gen.rs:884-891` — `next_temp_reg`, including true high water mark of
  `temp_high_water`
- `src/middle/core/ir_gen.rs:1954-1959` / `3632-3638` / `4737-4742` — Three statement-level register
  recoveries
- `src/middle/core/ir_gen.rs:1798` vs `2001` — `take_cur_locals` convention inconsistency
- `src/middle/core/ir_gen.rs:1987-2000` — E3014 register overflow check (`MAX_REGISTERS = 255`)
- `src/middle/core/ir_gen.rs:1337` — `rebase_jump_targets` (where 07 "jump target exists" invariant
  points)
- 6 save/restore function entries: `1677` / `1830` / `2105` / `2209` / `2759` / `4678`

**Defect Category 2 (arg_regs)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — Three semantic rearrangements
  and clone
- `src/middle/core/ir_gen.rs:7838-7846` — "Supplement 0" silent fallback

**Defect Category 3 (span-keyed)**

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — Three cross-layer field declarations
- `src/middle/core/ir_gen.rs:341` / `1942` — `release_plan`'s write and consumption
- `src/middle/core/ir_gen.rs:7420` / `7493` — `overload_resolutions` consumption
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — `method_def_ordinals` initialization and sole
  write point (key is unqualified `"Type.method"`)
- `src/frontend/core/typecheck/types.rs:31` / `56` — Two fields on the transfer structure
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — `ReleasePlan`
  definition and production
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — Cross-layer transfer
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  `overload_resolutions` production

**Defect Category 4 (Implicit Order Dependency)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — `generate_spawn_for_ir`'s AST synthesis and
  callback
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — `generate_spawn_expr_ir`'s same-style handshake
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — Two read points of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — Another 3 `ast::Type` constructions

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — `BytecodeInstr` 66 variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1408 lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — Three hardcoded discards (compare `2344` /
  `2347`)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (bridge to be removed by
  [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — 9 NOP
  degradations
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, `u8` upper limit 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; file has 83 constants in total
- `src/backends/interpreter/executor/ops/control.rs:90` — Active implementation of
  `BytecodeInstr::Switch` (no corresponding variant on IR side)
- `src/backends/interpreter/executor/debug.rs:194` — Dispatch match

**Test Status**

- `src/middle/core/ir_gen.rs` — 8448 lines, 0 `#[cfg(test)]`, 0 `mod tests`
- `src/middle/core/tests/mod.rs` — Declares `bytecode` / `def_assign` / `local_slots`, 29 tests in
  total; `def_assign` / `local_slots` positively pin down DefId and slot naming, no IR structural
  invariant validation
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — 4 references to ir_gen, all
  negative assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop`
  (per-opcode roundtrip criterion example, last test in the file; file has 1209 lines / 23 `#[test]`
  in total)

**Engineering Configuration**

- `Cargo.toml:37-39` — dev-dependencies are only `criterion` / `proptest`; `[dependencies]` has no
  regalloc / petgraph / graphlib
- `docs/src/.vitepress/config.js:226-231` — Sidebar auto-scans the `/rfc/draft` directory; this
  document is not in that directory, referenced by RFC-039 and this directory index
