# Intermediate Representation SSA Conversion

> **Ancillary design document.** This document is an ancillary to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criterion classification, and execution stage ordering are in the
> body of RFC-039; the role of each ancillary document is given in
> [this directory's index](index.md).

## Positioning and Scope

This document addresses the IR construction discipline at the L3 layer, and proposes a "each value
has a single definition" construction method for `src/middle/core/ir_gen.rs` (8448 lines). **The
goal is not to introduce a new instruction set, but to eliminate three classes of discipline defects
that produce no errors on failure**:

1. 6 manual register save/restore pairs scattered dozens of lines apart;
2. The `arg_regs` semantic reshuffling shared by 6 branches inside `generate_call_expr_ir`
   (`ir_gen.rs:7269-8014`, 746 lines);
3. Silent invalidation of cross-layer contracts keyed on `Span`.

This document is responsible for: IR shape (`Operand` / `Instruction`) changes, temporary value
allocation discipline, making cross-layer contracts explicit, splitting call expression dispatch,
and the wiring of IR static verifiers.

This document is not responsible for: type representation convergence
([03](03-type-unification.md)), stage contracts and obligation ledgers ([02](02-stage-contract.md)),
the definition and gating of equivalence criteria ([07](07-equivalence-oracle.md)), frontend
paradigms ([05](05-frontend-paradigm.md)), or dead code and hollow-design cleanup
([06](06-cleanup-inventory.md)). This document references conclusions from these documents in
several places, and does not re-argue them.

**Criterion category** is executed under **C4 (IR shape change)** per
[07 Equivalence Criteria](07-equivalence-oracle.md): **behavioral equivalence + IR structural
invariants**, not full IR snapshot equality. SSA conversion necessarily changes IR shape, and
forcing snapshots here would induce the team to relax criteria (this is precisely the trap recorded
in 07).

**Position in the RFC-039 stage sequence is P7** (`ir.rs` / `ir_gen.rs` / `bytecode.rs` /
`translator.rs`, accepted with behavioral equivalence + `verify_ssa` green). The internal
implementation batch numbering in this document (batches a–d) is not the same as P7's internal
ordering: the former is this document's batching strategy, the latter is RFC-039's global stages.

## Feasibility Prerequisites

**Verified facts.** The IR already possesses **all structural prerequisites** for SSA construction:

| SSA prerequisite             | Current state in this repo                                                                                         | Location              |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------ | --------------------- |
| Explicit CFG                 | `BasicBlock { label, instructions, successors }`                                                                   | `ir.rs:624-628`       |
| Explicit entry block         | `FunctionBody::Code { blocks, entry, locals }`                                                                     | `ir.rs:636-642`       |
| Scope nesting information    | `LocalSlot::scope_depth` ("0 = function parameter layer, for disambiguating same-named entities in nested scopes") | `ir.rs:658`           |
| Three-address form           | 76 variants, 54 with `dst: Operand`, 3 with `dst: Option<Operand>`                                                 | `ir.rs:47-533`        |
| Per-instruction span         | Every variant carries a `span: Span` field                                                                         | `ir.rs:47-533`        |
| Annotatable value type       | `LocalSlot::ty: MonoType`                                                                                          | `ir.rs:656`           |
| Slot count upper-bound check | E3014 check inside `generate_function_ir` (`MAX_REGISTERS = 255`)                                                  | `ir_gen.rs:1987-2000` |

**If these prerequisites did not exist, the cost estimates in this document would be entirely
different.**

**The two decisive points:**

**First, SSA conversion is not adding capability — it is removing a capability dependency.** In the
current state, temporary slots are **actively designed to be multiply defined** — explicit rollbacks
of `next_temp` at `1954-1959` / `3632-3638` / `4737-4742`. The first step of SSA conversion is to
**delete these three rollbacks**, letting `next_temp` increase monotonically. After deletion,
**every temporary slot naturally has only one definition point** — single definition shifts from "a
rule to be checked" to "a mathematical consequence of the construction method".

**Second, the slot count upper bound is already a hard check.** `temp_high_water` at `ir_gen.rs:889`
is already tracking the true high-water mark, and `1992-2000` is already reporting errors against
the 255 cap. **Removing the rollbacks will not let slot count run out of control** —
`temp_high_water` records historical peak occupancy, which is independent of whether rollbacks
happen (the comment at `887-888` states exactly this).

**What these two points together imply**: `ir_gen.rs` is already maintaining all the bookkeeping
information that SSA needs — it just **recycles** that information. The bulk of the SSA-conversion
work lies in splitting `generate_call_expr_ir` and making cross-layer contracts explicit, not in
building IR expressive power.

### Prerequisite Dependency: Type Representation Unification

**Verified fact.** `src/middle/core/ir.rs:3`:

```rust
pub use crate::frontend::core::parser::ast::Type;
```

`ir.rs:6` additionally has `use crate::frontend::core::typecheck::MonoType;`, and `LocalSlot::ty`
(`ir.rs:656`) uses `MonoType`. On the serialization side, the third representation is `ir::Type`,
bridged by `impl From<MonoType> for IrType` at `bytecode.rs:2353`.

That is: **two parallel type representations exist inside the IR, and one of them directly
`pub use`s the AST's type.**

SSA conversion must add a `Phi` variant, must record a type for every SSA value (to feed the 07
verifier's "type-consistent" invariant), and must judge whether the two predecessors' types at a CFG
merge point are compatible. **If the type representation has not been converged, SSA will end up
writing one type-compatibility check per representation** — and the third representation will grow
in.

This is not a "which to do first" question; it is a question of **"doing it first produces three
copies of type-compatibility logic"**. [03 Type Representation Unification](03-type-unification.md)
must first complete the `pub use ast::Type` convergence at `ir.rs:3` and tear out the bridge at
`bytecode.rs:2353`. Item 6 in this document's change list explicitly states that `ir.rs:3` is **not
touched by this document**.

## Current State: Four Classes of Defects

`ir_gen.rs` is not a "ugly but running" file. It is a file that **stakes correctness on human
line-by-line reading**:

- The file has **0 `#[cfg(test)]`, 0 `mod tests`** (verified).
- `src/middle/core/tests/mod.rs` declares the three modules `bytecode` / `def_assign` /
  `local_slots`, totalling 29 tests — of which `def_assign.rs` and `local_slots.rs` are **forward**
  coverage of ir_gen's DefId allocation and local-slot naming (sentinel-level), while
  `bytecode.rs`'s 4 references to ir_gen (`421` / `423` / `1028` / `1173`) are **all reverse
  assertions** — "ir_gen does not produce", "ir_gen does not construct", "ir_gen's frontend does not
  construct the corresponding IR".

That is: ir_gen has 29 sentinel-level tests pinning down local behavior, but **there is no IR
structural invariant check whatsoever** (dominance, single definition, jump targets, type
consistency) — none of the three modules declared in `middle/core/tests/mod.rs` makes any forward
assertion about IR overall shape.

Yet this file contains several **defects that compile successfully, pass all checks, run correctly,
and only get values wrong**. Their common property is: **failure produces no error**.

This is where the highest risk lies. It is not "the function is too long", nor "naming is chaotic",
but that **the core invariants of this file have no machine-enforceable enforcement**.

`ir_gen.rs:248-249` writes this out as an explicit contract:

> Save before generating a nested function body, restore afterward — **must be physically adjacent
> to the save/restore points of `next_temp`**, otherwise the inner function's names will leak into
> the outer scope.

This is a **rule written as a comment instead of an assertion**. It holds today only because those 6
save/restore pairs all happen to be written correctly.

**Why "just add a verifier" is not enough**: a verifier can **catch** these defects, but it cannot
catch **what produces them**. The cause is not "missed during review"; it is:

> **The current IR shape itself allows a slot to be multiply defined, and `ir_gen`'s temporary
> register allocator actively depends on this.**

Concrete evidence: three statement-level register reclamations (`1954-1959` / `3632-3638` /
`4737-4742`) — at the end of each statement they roll `next_temp` **back** to the larger of "last
named slot" and `temp_floor` (`1959`: `self.next_temp = named_watermark.max(self.temp_floor);`).
This means the same `Operand::Local(n)` slot is **written repeatedly** within a single function
body, with the number, order, and live range of writes entirely decided by one manual decision:
"when to roll back".

In this shape, "single definition" is not a rule that can be incrementally imposed; it is in
**direct conflict with the register-allocation strategy**. So SSA conversion is not adding a check
to existing code; it is **replacing the strategy that depends on multiple definitions**. The good
news is that replacing this strategy is cheap — see "Feasibility Prerequisites" above.

### Defect Class 1: 6 manual save/restore pairs, no error on failure

**Verified facts.** 6 save/restore pairs; the maximum distance between a save and its restore is
**161 lines**:

| Save point            | Restore point | Span      | Function                        | Entry line |
| --------------------- | ------------- | --------- | ------------------------------- | ---------- |
| `ir_gen.rs:1745-1759` | `1819-1822`   | 63 lines  | `generate_method_ir`            | `1677`     |
| `1858-1866`           | `2026-2029`   | 161 lines | `generate_function_ir`          | `1830`     |
| `2116`                | `2180`        | 64 lines  | `generate_curry_innermost_func` | `2105`     |
| `2224-2229`           | `2284-2288`   | 59 lines  | `generate_curry_function_ir`    | `2209`     |
| `2768-2777`           | `2820-2851`   | 74 lines  | `generate_anon_binding_ir`      | `2759`     |
| `4686-4695`           | `4767-4774`   | 72 lines  | `generate_lambda_body_ir`       | `4678`     |

> **Attribution note**: the correspondence for the two curry functions is `2116`/`2180` belongs to
> `generate_curry_innermost_func` (entry `2105`), `2224-2229`/`2284-2288` belongs to
> `generate_curry_function_ir` (entry `2209`) — i.e. the "innermost" goes to the former; do not
> infer by intuition from line numbers.

Fields involved: `next_temp` / `temp_high_water` / `temp_floor` / `cur_locals` / `cur_span` /
`loop_stack`, all driven by `next_temp_reg` (`ir_gen.rs:884-891`). That function maintains the true
high-water mark `temp_high_water` at `889` (its own comment states: statement-level reclamation
rolls back `next_temp`, so the total slot count is the historical peak). The same comment convention
applies to `cur_span` (`238-242` self-states "save before nested function body generation, restore
afterward (same handling as next_temp)").

**Two inconsistent gauges** (this is a more insidious problem than "missed writing"):

| Location         | Gauge                                                                                                                     |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------- |
| `ir_gen.rs:1798` | `take_cur_locals(param_types.len())` — truncated by **number of parameters**                                              |
| `ir_gen.rs:2001` | `take_cur_locals(total_locals)`, where `total_locals = self.temp_high_water` (`:1992`) — truncated by **high-water mark** |

`generate_method_ir` uses the former and **does not have the E3014 overflow check from
`1992-2000`**. The two functions give two different answers to "how many slots should the function
body have", and neither reports an error.

**Three duplicated statement-level register reclamations with consistent criteria but differing
guards**:

| Location    | Function                           | Guard                     |
| ----------- | ---------------------------------- | ------------------------- |
| `1954-1959` | `generate_function_ir`             | None                      |
| `3632-3638` | `generate_block_ir` (entry `3602`) | `if result_reg.is_none()` |
| `4737-4742` | `generate_lambda_body_ir`          | None                      |

The guard at `3632` has a clear reason (comment at `3626-3631`: the block for the expression-operand
position, the outer layer may hold sibling-argument temporaries that live across blocks). The other
two have no equivalent guard. **The same criterion is hand-copied in three places, with one having a
guard and two not** — this kind of difference is exactly what SSA is meant to eliminate.

**The restore point in `generate_anon_binding_ir` is separated by IR construction** (`2768-2777`
save → `2820-2824` restore 5 fields → `2829-2848` construct `func_ir` → `2851` finally restore
`loop_stack`). The two halves of the same state group are physically split by 22 lines of
construction code, and the ordering dependency is **reversed** (restore the slot table first, then
construct IR, then restore the loop stack).

**Consequence**: miss one restore → `next_temp` leaks, temporary values jump numbers, `cur_locals`
names leak into the outer scope. **The IR is still self-consistent, still passes every check, still
compiles, only the values are wrong.**

### Defect Class 2: `arg_regs` semantic reshuffling in `generate_call_expr_ir`

**Verified facts.** `generate_call_expr_ir` is at `ir_gen.rs:7269-8014`, **746 lines**, a single
function. Its signature (`7269-7279`) takes `func: &Expr` / `args: &[Expr]` /
`named_args: &[(String, Expr)]` / `span: &Span` (plus `_expr` / `result_reg` / `instructions` /
`constants`); internally 6 branches share the same `Vec<Operand>` and perform **semantic rewrites**:

| Line        | Semantics                                                                                                                                            |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `7395`      | Namespace call: flatten by `slots` then `collect` again — named-argument reshuffle                                                                   |
| `7724`      | Struct construction: `unwrap` each slot of `final_args` then `collect` again — field reshuffle                                                       |
| `7835-7848` | Function call: named-argument reshuffle; `7838-7846` **fills a new register with value 0 for missing slots** (`next_temp_reg` + `Instruction::Load`) |
| `8000`      | `let final_args: Vec<Operand> = arg_regs.clone();` — clone before emitting instructions                                                              |

Source as-is for the four reshuffles:

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

The comment at `7835-7848` self-states: "Uncovered slots: remain absent; the typecheck phase reports
E1010. At runtime, fill 0 as a safety measure (the normal path will not reach here)."

**This design has two layers of risk**:

1. **Misalignment does not error, it only gets values wrong.** Each of the 6 branches independently
   decides the final order of `arg_regs`; any order assumption error does not panic.
2. **Normalized snapshots are immune to this.** 07 already documents this limitation: snapshot tools
   rename temporaries by "order of first appearance", so "the 3rd argument should have used the 5th
   register" looks identical in the snapshot. **At the C4 stage, this class of defects has no
   snapshot-level criterion to even exist.**

**Supplementary verification: the struct-construction path's reshuffle has no duplicate-slot
guard.** The `final_args` at `7724` is filled as "positional arguments occupy slots first, named
arguments overwrite by field name", with **no `slots[idx].is_some()`-style duplicate-slot check
before overwrite** (contrast: the namespace path at `7381` has one) — when a named argument silently
overwrites a positional one, it does not panic and does not diagnose; and the **evaluation order**
of arguments (source order) and the final `arg_regs` order (field-declaration order) diverge here.
Batch d's `fill_missing` assertionification must additionally cover the "duplicate slot" check,
otherwise the struct-construction path's silent overwrite survives intact.

### Defect Class 3: Silent failure of span-keyed cross-layer contracts (**resolved**, 2026-10-13 / RFC-039 D20, WBS 4.5)

**Verified facts (audit time-point).** There are **two** span-keyed cross-layer contracts repo-wide:

| Contract               | Declaration                                                                                                                     | Production                                                                                                            | Transport                                                                              | Consumption                                                                                                              |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `release_plan`         | ~~`ir_gen.rs:191` `HashMap<Span, Vec<String>>`~~ → now `HashMap<PlanId, Vec<String>>` (`ownership.rs:39`)                       | `ownership.rs:31` (`ReleasePlan::drops` field), `build_release_plan` (dispatches `walk_stmt`'s statement identity)    | `typecheck/types.rs:38` ← `checker.rs` transports ← `ir_gen.rs:360` (clone read point) | `ir_gen.rs:3103` (`ReleasePlan::consume`, sole entry; settlement-complete flag `:1095`)                                  |
| `overload_resolutions` | ~~`ir_gen.rs:222` `HashMap<Span, String>`~~ → now `OverloadResolutions` (table + consumption ledger in one body, `types.rs:62`) | `inference/expressions.rs:4550` (written by call-site `PlanId`; (`inference/statements.rs` only transparently merges) | `typecheck/types.rs:62` ← `checker.rs` assembles ← `ir_gen.rs` field holder            | `ir_gen.rs:7697` (with binding), `7772` (without binding) (`OverloadResolutions::get`; settlement-complete flag `:1097`) |

Form at audit time (`ir_gen.rs:1942`, **no longer exists**):

```rust
// NLL Release: insert Drop instructions at statement boundaries
if let Some(vars) = self.release_plan.get(&stmt.span) {
```

`if let Some` — **not found is just not found, no else branch, no assertion, no count**. Any
inconsistency in how spans are computed on either side → **Drop instructions silently disappear,
zero errors**. The Drop sequence of refined types (refinement type) is the core carrier of this
project's ownership semantics (`ownership.rs:26-32` self-states "NLL precise release plan"), yet its
landing was once silent.

`7420` / `7493` are the same shape (`if let Some(mangled) = self.overload_resolutions.get(span)`),
with the consequence that overload resolution falls back to the default binding.

**Disposition (4.5 landed)**: both keys are now `PlanId`, and consumption **accounts on hit** (read
and account in the same call); a non-empty difference set at the sole settlement point raises
**E-level `W1090`** — "not found" is no longer silent. The landing shape and criteria are in
[02](02-stage-contract.md) §2 revision notes (4.5); these line numbers (`1942` / `7420` / `7493`)
are retained as audit-time evidence, not as pointers to the current state.

> **`method_def_ordinals` is not span-keyed.** It is `HashMap<String, usize>` (`ir_gen.rs:224`),
> initialized at `:369`, with a sole write point at `:1692-1702` (assigns `#N` suffix by
> `base_name`'s definition order). Its risk is **different**: definition-order dependency — the `#N`
> suffix is decided by AST definition order; if the generation order and the typecheck registration
> order disagree, the same function name resolves to different targets. This is not span mismatch,
> but it has the same "wrong without error" character; the disposition is in "Additional mechanisms
> for span-keyed issues", mechanism three.

### Defect Class 4: spawn/lambda implicit handshake

**Verified facts.** `ir_gen.rs` has **two** places that construct `ast::Expr::Lambda` themselves and
call back into `generate_expr_ir`, relying on a group of mutable fields for handshaking:

**Handshake A — `generate_spawn_for_ir` (entry `4143`):**

```
4245    self.pending_env_vars = vec![Operand::Local(element_reg)];
4246    let lambda = ast::Expr::Lambda { params: ..., body: ..., span };
4256    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
```

`generate_expr_ir` → `generate_lambda_expr_ir` (entry `6543`) → `6571-6572` consumes via
`std::mem::take(&mut self.pending_env_vars)`.

**Handshake B — `generate_spawn_expr_ir` (entry `6641`):**

```
6698    self.pending_env_vars = env_ops;
6699    self.pending_env_names = env_names;
6701    let lambda = ast::Expr::Lambda { params: Vec::new(), body: Box::new(ast::Block { stmts: vec![ast::Stmt { ... }] }), span };
6712    self.generate_expr_ir(&lambda, closure_reg, instructions, constants)?;
6713    self.pending_env_vars.clear();
6714    self.pending_env_names.clear();
```

**Handshake C — `closure_captures`**: written at `6589-6593`, **within the same function** at `6598`
calls `generate_lambda_body_ir` (entry `4678`); `6600` clears. There are two read points: `:6284`
(`LoadUpvalue` inside the closure body) and **`:458`** (`closure_captures.contains_key(head)` on the
`flatten_namespace` path).

The `:458` read point is the most dangerous: it is on the **namespace-resolution** path, meaning the
live state of a `HashMap<String, usize>` can change variable-name resolution results. Between
`6589`'s write and `6600`'s clear lies a full closure-body generation (`6598`); if any path inside
`generate_lambda_body_ir` returns early without clearing the table (it has multiple return points
when `?` propagates errors), **`closure_captures` will leak into later unrelated code at `:458`**.

Other implicit state of the same kind includes `pending_env_names` (handshake B writes at `6699`,
clears at `6714`).

### What SSA will not solve (honestly listed)

| Issue                                             | Location                                                                                                                             | Why SSA does not solve it                                                                                                                                                                                           |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `compile_pattern` complexity                      | `ir_gen.rs:5390-5687`, **298 lines**, self-recurses 4 times (`5473` / `5504` / `5619` / `5640`), external call site `5270`           | The dispatch complexity of pattern matching (union payloads / tuples / structs / literals / nesting) is **inherent to the algorithm**. SSA only turns guards' `JmpIfNot` into `Phi`; it does not reduce guard count |
| `eval_const_expr` complexity                      | `ir_gen.rs:2298-2488`, **191 lines** (`2297` has `#[allow(clippy::only_used_in_recursion)]`, confirming self-recursion)              | Constant-folding expression coverage is unrelated to SSA                                                                                                                                                            |
| Span-keyed invalidation (**resolved**)            | Audit time-point: `release_plan` (`1942`), `overload_resolutions` (`7420` / `7493`) — after 4.5, `PlanId` key + mandatory settlement | **SSA does not touch this layer at all** (conclusion unchanged). Already independently solved by the obligation ledger in [02](02-stage-contract.md); see "Disposition of span-keyed issues", mechanism two         |
| `Instruction` variant count                       | 76 (`ir.rs:47-533`)                                                                                                                  | SSA adds `Phi`, **net +1**. 76 variants correspond to 76 bytecode opcodes (or are downgraded to NOP); they do not decrease because of SSA                                                                           |
| `method_def_ordinals` definition-order dependency | `ir_gen.rs:224` / `369` / `1692-1702`                                                                                                | Same kind as span-keyed but different cause; see "Additional mechanisms for span-keyed issues", mechanism three                                                                                                     |

## Target Design

### SSA Shape Definition

**Design judgment.** The target shape is as follows.

#### How to change `Operand` variants

The current `ir.rs:12-20` has 7 variants. **Verified repo-wide construction sites**:

| Variant             | Production sites                                                                                                                                                                         | Disposition                                                                              |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `Const(ConstValue)` | 8 files                                                                                                                                                                                  | **Keep**, unchanged                                                                      |
| `Local(usize)`      | 7 files                                                                                                                                                                                  | **Keep but split semantics** (see below)                                                 |
| `Arg(usize)`        | 4 files                                                                                                                                                                                  | **Keep**, as implicit definitions of function entry (parameters are block-0 definitions) |
| `Global(usize)`     | 5 files                                                                                                                                                                                  | **Keep**, unchanged                                                                      |
| `Temp(usize)`       | **0 construction sites** (only handling arms in `codegen/operand.rs:39-45` and `:75`)                                                                                                    | **Delete** (dead variant)                                                                |
| `Label(usize)`      | **0 construction sites**                                                                                                                                                                 | **Delete** (dead variant)                                                                |
| `Register(u8)`      | **0 construction sites** (the comment at `ir.rs:19` says "Added for codegen", but codegen goes through `OperandResolver` in `codegen/operand.rs`, which does not construct this variant) | **Delete** (dead variant)                                                                |

`Local(usize)` is split into two variants because it currently **carries two mutually exclusive
semantics simultaneously** — "the storage location of a named variable" (writable multiple times)
and "the compiler's temporary register" (should be written once):

```rust
pub enum Operand {
    Const(ConstValue),
    /// SSA value: dense numbering within the function, single definition
    Value(ValueId),
    /// Storage slot of a named local variable: addressable, multiple definitions allowed (non-SSA part)
    Local(usize),
    Arg(usize),
    Global(usize),
}
```

`ValueId` is a `u32` newtype (`#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]`),
densely numbered within the function, **monotonically increasing, never rolled back**.

#### `Phi` as a new `Instruction` variant

```rust
/// Merge point: each predecessor basic block supplies one definition of this value
/// incoming is sorted by predecessor block label (ensured at construction time, for normalization)
Phi {
    dst: Operand,                                  // must be Operand::Value
    incoming: Vec<(usize, Operand)>,               // (predecessor label, value from that predecessor)
    span: Span,
},
```

Three construction constraints (all enforced by `verify_ssa`, see [07](07-equivalence-oracle.md)
layer one):

1. `incoming.len() == number of predecessors of this block`.
2. The label set of `incoming` equals the set of labels whose `successors` contain this block's
   label in `blocks`.
3. All `incoming` values have the same `MonoType` (**depends on 03's type representation being
   converged**).

**Disposition of the `3632-3638` guard issue**: the current `generate_block_ir` reclamation has the
`if result_reg.is_none()` guard (comment at `3626-3631`: the block for the expression-operand
position; the outer layer may hold sibling-argument temporaries that live across blocks). After SSA
conversion, this guard **is no longer needed** — because temporaries are no longer rolled back,
sibling-argument temporaries living across blocks hold their own `ValueId` and are not affected by
rollbacks. **This is the place where SSA conversion actually simplifies code, not just renames
things.**

#### Value identification method

**Design judgment**: do not do mem2reg, do not do virtual register numbering, **only do explicit
renaming**. Reasons:

- YaoXiang's `&` / reference / ownership semantics make "which slots can be mem2reg'd" a
  **type-system problem**, not a CFG problem. Verified: `ir.rs:656` `LocalSlot::ty: MonoType`
  carries full type information, but judging "whether this `MonoType` is mem2reg-able" requires the
  converged type representation from 03 and the cooperation of refined-type rules. **Doing mem2reg
  before 03 is guessing.**
- Explicit renaming only needs to change the return-value semantics of `next_temp_reg()`
  (`ir_gen.rs:884-891`) plus delete the three rollbacks, **without touching the control flow of any
  lowering algorithm**.

Value-table shape: `FunctionBody::Code` gains `values: Vec<ValueInfo>`, where

```rust
pub struct ValueInfo {
    pub ty: MonoType,
    /// Debug name (from the eliminated named slot), None means pure temporary
    pub debug_name: Option<String>,
    pub defining_block: usize,   // label of the basic block containing the definition
}
```

`values` and `locals` coexist: `locals` carry addressable named variables (possibly multiply
defined), `values` carry single-definition SSA values. **The existence of `locals` does not violate
SSA** — it is the "memory" part outside SSA (LLVM's alloca model).

#### Change-volume estimate

| Item                                                    | Estimate               | Basis                                                                                                            |
| ------------------------------------------------------- | ---------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `next_temp_reg` rework                                  | ~15 lines              | Single function at `ir_gen.rs:884-891`                                                                           |
| Delete three statement-level reclamations               | −15 lines              | `1954-1959` / `3632-3638` / `4737-4742`                                                                          |
| Converge 6 save/restore pairs into RAII                 | net ±0 lines           | See "Per-item disposition table"                                                                                 |
| `ir.rs` structural changes                              | +120 ~ 180 lines       | `ValueId` / `Phi` / `ValueInfo` / value table                                                                    |
| Add `Phi` arm in `translator.rs`                        | +25 ~ 40 lines         | One of the existing 50 `translate_*`                                                                             |
| Add `PHI` instruction or runtime filling in `executor/` | +60 ~ 120 lines        | Depends on the bytecode-shape choice for `Phi`                                                                   |
| Rename `Operand::Local` → `Value` call sites            | Mechanical replacement | **Verified**: `Operand::Local(` appears **323 times** inside `ir_gen.rs`, **355 times** across 6 files repo-wide |

### Per-item Disposition Table

| Defect class                                       | Eliminated by SSA?                | Disposition                                                                                                                                                                                                                                                                                                                                                         | Criterion                                                                    |
| -------------------------------------------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| **1. 6 manual save/restore pairs**                 | **Yes (at the root-cause level)** | `next_temp` / `temp_high_water` / `temp_floor` become `&mut` monotonic counters, no longer requiring cross-function-body save. `cur_locals` / `loop_stack` / `cur_span` switched to RAII guard (`struct NestedBodyGuard<'a>`, restores in `Drop`), **making "forgot to restore" a compile error**. The comment contract at `248-249` is enforced by the type system | 07 layer-one "inner-layer isolation" invariant + `verify_loose`/`verify_ssa` |
| **1b. Gauge inconsistency (`1798` vs `2001`)**     | **Yes**                           | Unify to `take_cur_locals(self.temp_high_water)`, and E3014 check (`1992-2000`) extracted into shared function `check_register_budget()`, called from both sites                                                                                                                                                                                                    | Corpus diff + snapshot (zero-diff possible in pure-move phase)               |
| **1c. Three statement-level reclamation criteria** | **Yes**                           | All three deleted (`1954-1959` / `3632-3638` / `4737-4742`), including the `result_reg.is_none()` guard at `3632`                                                                                                                                                                                                                                                   | C4: behavioral equivalence + invariants                                      |
| **2. `arg_regs` semantic reshuffle**               | **Partial**                       | SSA turns "wrong register used" into "wrong value ID used" and is caught by dominance checks, but **the reshuffle logic itself remains**. True elimination relies on function splitting — each of 6 branches only `push`es, ownership of `arg_regs` stays at the function top                                                                                       | Layer-one dominance invariant (snapshots are immune)                         |
| **3. Span-keyed invalidation**                     | **No**                            | SSA does not touch this layer at all. See "Additional mechanisms for span-keyed issues"                                                                                                                                                                                                                                                                             | 07's `test_release_plan_spans_consumed`                                      |
| **4. Implicit ordering dependency**                | **No**                            | See "`synth.rs` boundary"                                                                                                                                                                                                                                                                                                                                           | Requires new explicit criterion                                              |
| `compile_pattern` 298 lines                        | No                                | See "What SSA will not solve"                                                                                                                                                                                                                                                                                                                                       | —                                                                            |
| `eval_const_expr` 191 lines                        | No                                | See "What SSA will not solve"                                                                                                                                                                                                                                                                                                                                       | —                                                                            |
| `Instruction` 76 variants                          | No (+1)                           | Add `Phi` → 77                                                                                                                                                                                                                                                                                                                                                      | —                                                                            |

### Register Allocator: In-house or Existing Crate

**Verified facts (decisive)**:

1. `Operand::Register(u8)` (`ir.rs:19`) has **0 construction sites repo-wide**.
2. The backend is not a register machine — `OperandResolver` in
   `src/middle/passes/codegen/operand.rs` resolves `Operand::Local/Temp/Arg` to `u8`, capped at 255
   (the `Temp` arm at `39-45` overflows as E3014).
3. `translator.rs` (`src/middle/passes/codegen/translator.rs`, 1577 lines, 50 `translate_*`,
   function bodies distributed across `101-1558`) outputs
   `BytecodeInstruction::new(opcode, operands)`, opcodes from `src/backends/common/opcode.rs` (83
   constants).
4. `src/backends/interpreter/executor/ops/` is the interpreter's opcode dispatch family.

**Conclusion**: the so-called "registers" are **operand stack slot indices, not physical
registers**. The target machine has **no finite register file, no register pressure, no spill/split
requirement**.

**Evaluation of three options**:

| Option                                               | Evaluation                                                                                                                                                                                                                                                                                                                                                                                 |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `regalloc` / `regalloc2` (rustc lineage)             | **Not adopted.** Both take rustc's `Function` trait + MIR's `Place`/`Operand` abstractions as input. Writing an adapter for YaoXiang IR requires implementing the full trait (CFG, liveness, spill slot, scratch), **500+ lines of adapter code** for zero benefit — because there is no real register file to color. And rustc itself only does linear scan here (with `enable-llvm` off) |
| `graph-alloc` series                                 | **Not adopted.** Same as above, with even less stable API                                                                                                                                                                                                                                                                                                                                  |
| **In-house linear scan (Chaitin-Briggs simplified)** | **Adopted.** What YaoXiang actually needs is: map each `ValueId` to a non-conflicting `u8` stack slot in `values` order (`OperandResolver` already has the `u8` contract and E3014 cap). This is essentially **max-clique coloring over non-overlapping live ranges**, which — under no-register-file constraint — degenerates to "assign the first available slot in definition order"    |

**Design judgment**: in-house, targeting 500-1000 lines (including `values → Reg` mapping,
live-range computation, interface with `OperandResolver`). **At the same time, we must acknowledge:
if the `u8`/255 slot cap is insufficient under real load, the in-house allocator must handle
overflow, and the existing code already treats it as an error at `1992-2000`.** This means this
document **does not solve** the "large function lacks registers" problem — **that is a downstream
consequence of `compile_pattern`'s recursion depth, a separate topic.**

`Cargo.toml:37-39`'s dev-dependencies are only `criterion` / `proptest`; `[dependencies]` has no
regalloc / petgraph / graphlib — introducing any crate adds a new dependency surface.

### Splitting Strategy for `generate_call_expr_ir` (746 lines)

**Must be done last**, and **must follow SSA shape and criteria**. Reason: the split itself is C1
(pure move, criterion is snapshot zero-diff), but only after `arg_regs`'s semantics is reduced by
SSA to "each argument gets its own `ValueId`" can the split move the old order assumptions verbatim
into the new file.

**Core design constraint: `arg_regs` ownership stays at the function top; the 6 branches only
`push`, never reshuffle.**

```
generate_call_expr_ir (746 lines)
├── Argument evaluation: evaluate in source order, push to arg_regs — shared by 6 branches, the only place appending is allowed
├── CallArgs (value object)
│   └── arg_regs: Vec<Operand>  — created only at the top; 6 branches only push, never collect/reorder
├── Branch 1 → emit_namespace_call(args: CallArgs)         [extract 7395 branch]
├── Branch 2 → emit_bound_call(args: CallArgs)              [extract 7406+ branch]
├── Branch 3 → emit_struct_ctor(args: CallArgs)             [extract 7724 branch]
├── Branch 4 → emit_plain_call(args: CallArgs)              [extract 7835-7848 branch]
├── Branch 5 → emit_curried_call(args: CallArgs)
└── Branch 6 → emit_method_call(args: CallArgs)             [extract 7420 / 7493 branch]
```

The "fill 0" logic at `7835-7848` (`7838-7846`) sinks into
`CallArgs::fill_missing(&mut self, instructions) -> Result<(), Diagnostic>`, **and replaces the
self-stated "normal path will not reach here" comment with an assertion**: if any slot is absent and
typecheck did not report E1010, return a diagnostic directly. **This turns a silent fallback into a
hard error — the most direct improvement on defect class 2 in this document.**

**Criterion**: the pure-move part (SSA shape unchanged) uses snapshot zero-diff; the `fill_missing`
assertionification belongs to C4, using corpus diff + layer-one verifier.

### `synth.rs` Boundary: Turning Implicit Callbacks into an Explicit Synthesized-AST Boundary

**Verified fact**: there are **exactly 2** places in `ir_gen.rs` that construct `ast::Expr` —
`:4246` and `:6701` (plus `:6704` constructing `ast::Stmt`, and `:510` / `:1266` / `:1399`
constructing `ast::Type`).

**Proposal**: create `src/middle/lower/synth.rs`, as the **only** module in the whole repo allowed
to construct `ast::Expr` / `ast::Stmt` / `ast::Type` at the L3 layer. The reason is not "tidiness",
but:

> Constructing AST means **re-entering the frontend lowering entry**. And the `pending_env_vars` /
> `closure_captures` state that `generate_expr_ir` expects is a **precondition that the caller is
> responsible for preparing**, and this precondition currently has no type or signature to express —
> it holds only by the runtime fact of "who called me".

Change to explicit signature:

```rust
// src/middle/lower/synth.rs
pub struct SynthEnv {
    pub env_vars: Vec<Operand>,
    pub env_names: Vec<String>,
    pub captures: HashMap<String, usize>,
}

/// Synthesize a closure AST that captures env; the capture set is given explicitly by parameters,
/// and does not depend on the caller's prior writes to self.pending_env_*
pub fn synth_closure(
    params: Vec<ast::Param>,
    body: ast::Block,
    env: SynthEnv,
    span: Span,
) -> ast::Expr
```

Concrete changes:

| Location                       | Current                                                                                          | Change to                                                                                                                                                                                               |
| ------------------------------ | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ir_gen.rs:4245-4256`          | Write `pending_env_vars` → construct Lambda → call back                                          | Call `synth::synth_closure(...)` to get AST, pass `SynthEnv` explicitly; `generate_lambda_expr_ir` (`6543`) gains an inner entry that carries `SynthEnv`, **no longer reading `self.pending_env_vars`** |
| `ir_gen.rs:6698-6714`          | Write → construct → call back → clear                                                            | Same as above; the `clear()` at `6713-6714` disappears (no shared state to clear)                                                                                                                       |
| `ir_gen.rs:6571-6572`          | `mem::take(&mut self.pending_env_vars)`                                                          | Read from parameter instead                                                                                                                                                                             |
| `ir_gen.rs:6589-6593` / `6600` | Write / clear `closure_captures`                                                                 | Same as above; `closure_captures` becomes a **local variable during closure-body generation**, the `clear()` at `6600` disappears                                                                       |
| `ir_gen.rs:458`                | `self.closure_captures.contains_key(head)` reads a shared field on the namespace-resolution path | **This is the most dangerous one**: change to an explicit parameter, otherwise any `?`-early-return in `generate_lambda_body_ir` will leak the capture table into unrelated code                        |

**Criterion**: add sentinel test `test_no_ast_construction_outside_synth`, scanning `src/middle/**`
so that `ast::Expr::` / `ast::Stmt {` construction syntaxes cannot appear outside `synth.rs`. Shape
is the same as RFC-039's `check-module-boundary.py`.

### Disposition of Span-keyed Issues (What SSA Cannot Solve)

**Design judgment.** Three independent mechanisms, ordered by cost:

**Mechanism one: consumption-coverage assertion (mandatory, lowest cost; superseded on 2026-10-13 by
a stronger form).** 07 already defines `test_release_plan_spans_consumed` (now settled by `PlanId`).
This document's original addition was to make it a **runtime count** rather than just a test; the
landed form is stronger than that conception — **consumption ledger (`ConsumptionLedger<K>`) + sole
settlement point (E-level `W1090`)**, no new diagnostic code, and does not rely on counters inside
ir_gen to self-verify:

```rust
// near ir_gen.rs:341
// record how many plans typecheck delivered
self.release_plan_total = type_result.release_plan.drops.len();
// each time :1942 hits, self.release_plan_hit += 1;
// at the tail of generate_module_ir (after assign_defs):
//   if self.release_plan_hit < self.release_plan_total {
//       return Err(/* new diagnostic code: E3xxx NLL release plan not consumed by IR */);
//   }
```

**Why this works**: `drop()` instructions are runtime-observable for non-`ref` locals (destructor
side-effects), but **"should have dropped but did not" may not crash immediately under YaoXiang's
current value semantics + Arc/Rc model** — this is exactly why it was silent. Counting turns
"silent" into "detectable".

**Mechanism two: Span key → `DefId` or explicit plan ID (**landed 2026-10-13**: RFC-039 D20 / WBS
4.5 implemented per this mechanism as a `PlanId(u32)` structural-identity key +
mandatory-consumption settlement; landed form is in [02](02-stage-contract.md) §2 revision notes).**
The root fix is **don't use Span as a key**. `Span` is a source location, subject to drift from
macro expansion, `include`, and multi-file merging. `FunctionIR` already has the
`def: Option<DefId>` field (`ir.rs:677`), showing DefId is available at the IR layer. **Design
judgment**: introduce `PlanId(u32)` for `ReleasePlan` and `overload_resolutions`, allocated by
ownership / overload resolution at production time, matched by IR consumption by `PlanId`. **This
belongs to 02's obligation ledger (`Obligations`); it should be implemented together with
[02](02-stage-contract.md), not in this document.**

**Mechanism three: `method_def_ordinals`'s definition-order dependency (independent small item).**
The field is `HashMap<String, usize>` (`ir_gen.rs:224`), with keys `"{type_name}.{method_name}"`
(**without module qualification**, written at `:1691-1700`) — if two modules define same-named
`Type.method`, they share the same counter, and `#N` mangled names drift with it. **Its failure mode
is not span mismatch but order/duplicate-name mismatch** — if typecheck's registration order and
ir_gen's definition order disagree, same-named methods resolve to different targets. Disposition:
change so that typecheck **produces once** a `HashMap<DefId, String>` (bare name or `#N` mangled
name), and ir_gen only reads, never writes. **This removes one mutable cross-function state from
`ir_gen`.**

### Tracking Three Hard-coded Drops

**Verified facts.** `impl From<BytecodeFile> for BytecodeModule` (`bytecode.rs:943-2350`, 1408
lines) has three hard-coded drops:

| Location           | Field                | Value                                          | Consequence                                                                                                                     |
| ------------------ | -------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `bytecode.rs:2312` | `upvalue_count`      | `0` // Not stored in BytecodeFile              | Closure upvalue count zeroes out after `.42` round-trip                                                                         |
| `bytecode.rs:2315` | `exception_handlers` | `Vec::new()` // Not implemented yet            | **`.42` artifact loses the entire exception table**                                                                             |
| `bytecode.rs:2341` | `globals`            | `Vec::new()` // Not stored in BytecodeFile yet | **`.42` artifact loses global-variable information** (note `2347`'s `global_names` **is** populated — the two are inconsistent) |

**Design judgment (revised 2026-10-05, resolution D52): all three are folded into this document's
scope, as P7 batch e (7e), and no longer "filed as independent issues"**. The original judgment (the
latter two as independent issues) and the reasons are recorded below, along with why they were
overturned:

1. ~~They are **format-layer gaps** (`BytecodeFile` does not serialize these fields), unrelated to
   SSA.~~ True, but not a reason for exclusion — P7 already touches `bytecode.rs`, and fixing all
   three at once only needs one `VERSION` bump (4→5); splitting into two bumps is actually more
   expensive.
2. ~~Fixing them requires changing the `.42` on-disk format → bump version → scope inflates from "IR
   shape" to "artifact format".~~ The bump mechanism already exists (`MAGIC` + `VERSION` header; the
   read side rejects by version), and `.42` is a build artifact — cross-version read compatibility
   is not a goal; the inflation is in workload, not in risk.
3. ~~`2315` / `2341` are actually low-risk, and running `.42` directly is a minor path.~~
   **Overturned**: losing the exception table causes throw/try to behave incorrectly when running
   `.42` directly — throw/try is core language semantics; minor path does not mean silently wrong.
   The principle of this refactor is **to leave no unimplemented residue for core functionality**
   (`Vec::new() // Not implemented yet` is exactly that kind of residue).

**Also retained**: the three must still be registered in the `Obligations` ledger as "produced but
not serialized" until 7e completes, to prevent it from becoming a fourth "thing nobody knows about".
`2312` lands together with the version bump (D17); already-allocated but unused opcode values
(D32/D34) are reclaimed in the same batch.

## Detailed Design

### Cascading Impacts of IR Structural Changes

#### `src/middle/core/ir.rs` (905 lines)

| Change                                                               | Location             | Nature                                    |
| -------------------------------------------------------------------- | -------------------- | ----------------------------------------- |
| Add `ValueId(u32)` type                                              | near file head, `10` | New                                       |
| `Operand` remove `Temp` / `Label` / `Register`, add `Value(ValueId)` | `12-20`              | **Variant add/remove**                    |
| `Instruction` add `Phi` variant                                      | end of `47-533`      | **Variant +1 → 77**                       |
| `FunctionBody::Code` add `values: Vec<ValueInfo>`                    | `636-642`            | Field +1                                  |
| Add `ValueInfo` struct                                               | near `660`           | New                                       |
| `all_instructions` / `blocks` / `blocks_mut` / `locals`              | `689-719`            | Add `values()` / `values_mut()` accessors |

The `pub use ...ast::Type` at `ir.rs:3` is **not touched by this document** — it is
[03](03-type-unification.md)'s scope. But `ValueInfo::ty` must use the **converged** single type.

#### `src/middle/core/bytecode.rs` (2422 lines)

| Change                                                            | Location                              | Nature                                                                             |
| ----------------------------------------------------------------- | ------------------------------------- | ---------------------------------------------------------------------------------- |
| `BytecodeInstr` adds `Phi` or "runtime-expanded to Move" strategy | `127-554` (currently **66** variants) | Variant +1 or +0 (see below)                                                       |
| `opcode()` add `Phi` arm                                          | `558`                                 | Required (exhaustive match on `BytecodeInstr`)                                     |
| `size()` add `Phi` arm                                            | `649`                                 | Required                                                                           |
| `From<BytecodeFile>` decode match add `Phi` arm                   | `943-2350`                            | Required                                                                           |
| `upvalue_count: 0` fix                                            | `2312`                                | Fixed in this document (in-memory format; on-disk needs `VERSION` bump, see below) |

**The bytecode shape of `Phi` needs a design decision**. Two options (**design judgment: A is
recommended**, see "Key Decisions and Reasons" for the reasoning):

- **A (recommended): `Phi` is an IR-only pseudo-instruction**, expanded in `translator.rs` into a
  series of `Move { dst: vreg, src: incoming[i] }` at the start of the block, selected by the
  predecessor-block index. **No new opcode**, zero changes to `.42` format. Cost: increased bytecode
  volume (one copy per predecessor).
- B: Add `PHI` opcode + predecessor-index encoding; the interpreter maintains register versions on
  jump. **Requires changes to `executor/` state model + `.42` format + version number**. The benefit
  is only volume.

The decisive reason for choosing A: changing the `.42` format triggers version-number issues and
full backward-compatibility discussions, **while A's volume cost is irrelevant in interpreted
execution** (what executes is `Move`, not `Phi`).

#### `src/middle/passes/codegen/translator.rs` (1577 lines, 50 `translate_*`, distributed across `101-1558`)

| Change                                      | Location                                                                                                                         | Nature                                                                      |
| ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `Operand::Value(v)` dispatch                | `OperandResolver` (`codegen/operand.rs`)                                                                                         | Add arm                                                                     |
| `Instruction::Phi` translation arm          | inside dispatch match, before `Free` (`533`)                                                                                     | New branch, expanded to a `Move` sequence (option A)                        |
| Existing 9 NOP downgrades **stay in place** | `533` Free / `631` Dup / `632` Swap / `655-657` UnsafeBlockStart+UnsafeBlockEnd / `658-660` PtrFromRef+PtrDeref+PtrStore+PtrLoad | **Verified as 9**: `655-657` covers 2 variants, `658-660` covers 4 variants |

**SSA conversion will not reduce the NOP count.** These 9 downgrades reflect the
expressive-capability gap "IR has, bytecode does not"; SSA conversion does not touch them.

#### `src/backends/interpreter/executor/`

Under option A, `executor/` requires **no changes at all** — `Phi` disappears from the interpreter's
view at the `translator.rs` stage. **This is the second decisive reason for choosing A.**

`executor/ops/control.rs:90`'s `BytecodeInstr::Switch` has an active implementation, but in `ir.rs`
**no `Switch` / `BrTable` / `JumpTable` variant exists at all** (verified: zero hits for these three
names in `ir.rs`) — **this is a reverse gap**: an instruction exists at the bytecode layer that the
IR layer cannot produce. It does not affect this document, but it is noted here because it shows "IR
→ bytecode" is not surjective.

`executor/debug.rs:194`'s dispatch match also requires no change (under option A, no new opcode).

### Compiler Change List

**File by file, function by function.** All line numbers are the **starting** points of the change
(verified):

| #   | File                                      | Location                                                | Change                                                                                                              |
| --- | ----------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| 1   | `src/middle/core/ir.rs`                   | `12-20`                                                 | `Operand` variant add/remove                                                                                        |
| 2   | `src/middle/core/ir.rs`                   | end of `47-533`                                         | Add `Instruction::Phi`                                                                                              |
| 3   | `src/middle/core/ir.rs`                   | `636-642`                                               | `FunctionBody::Code` add `values`                                                                                   |
| 4   | `src/middle/core/ir.rs`                   | near `660`                                              | Add `ValueInfo`                                                                                                     |
| 5   | `src/middle/core/ir.rs`                   | `689-719`                                               | Add `values()` / `values_mut()` accessors                                                                           |
| 6   | `src/middle/core/ir.rs`                   | `3`                                                     | **`pub use ast::Type` not touched** ([03](03-type-unification.md) scope, recorded only)                             |
| 7   | `src/middle/core/ir_gen.rs`               | `884-891` `next_temp_reg`                               | Returns `Operand::Value`, monotonically increasing, no rollback                                                     |
| 8   | `src/middle/core/ir_gen.rs`               | `1954-1959`                                             | **Delete** statement-level reclamation                                                                              |
| 9   | `src/middle/core/ir_gen.rs`               | `3632-3638`                                             | **Delete** statement-level reclamation (including the `result_reg.is_none()` guard)                                 |
| 10  | `src/middle/core/ir_gen.rs`               | `4737-4742`                                             | **Delete** statement-level reclamation                                                                              |
| 11  | `src/middle/core/ir_gen.rs`               | `1745-1759` / `1819-1822`                               | Switch to RAII guard (`generate_method_ir`, entry `1677`)                                                           |
| 12  | `src/middle/core/ir_gen.rs`               | `1858-1866` / `2026-2029`                               | Switch to RAII guard (`generate_function_ir`, entry `1830`)                                                         |
| 13  | `src/middle/core/ir_gen.rs`               | `2116` / `2180`                                         | Switch to RAII guard (`generate_curry_innermost_func`, entry `2105`)                                                |
| 14  | `src/middle/core/ir_gen.rs`               | `2224-2229` / `2284-2288`                               | Switch to RAII guard (`generate_curry_function_ir`, entry `2209`)                                                   |
| 15  | `src/middle/core/ir_gen.rs`               | `2768-2777` / `2820-2851`                               | Switch to RAII guard (`generate_anon_binding_ir`, entry `2759`)                                                     |
| 16  | `src/middle/core/ir_gen.rs`               | `4686-4695` / `4767-4774`                               | Switch to RAII guard (`generate_lambda_body_ir`, entry `4678`)                                                      |
| 17  | `src/middle/core/ir_gen.rs`               | `1987-2000`                                             | Extract E3014 check into `check_register_budget()`, shared by `1798`/`2001` after gauge unification                 |
| 18  | `src/middle/core/ir_gen.rs`               | `1798`                                                  | `take_cur_locals(param_types.len())` → `take_cur_locals(self.temp_high_water)`, eliminating the gauge inconsistency |
| 19  | `src/middle/core/ir_gen.rs`               | `248-249`                                               | Delete the comment contract (enforced by the type system), replace with a pointer to `verify`                       |
| 20  | `src/middle/core/ir_gen.rs`               | `4245-4256`                                             | Switch to call `synth::synth_closure`                                                                               |
| 21  | `src/middle/core/ir_gen.rs`               | `6698-6714`                                             | Switch to call `synth::synth_closure`, delete `6713-6714`                                                           |
| 22  | `src/middle/core/ir_gen.rs`               | `6571-6572`                                             | Read `SynthEnv` from parameter, no longer `mem::take` from shared field                                             |
| 23  | `src/middle/core/ir_gen.rs`               | `6589-6593` / `6600`                                    | `closure_captures` demoted to local, delete `clear()`                                                               |
| 24  | `src/middle/core/ir_gen.rs`               | `458`                                                   | `closure_captures.contains_key` switched to explicit parameter (**highest-priority implicit dependency**)           |
| 25  | `src/middle/core/ir_gen.rs`               | `341` + `1942`                                          | Add span-keyed consumption count                                                                                    |
| 26  | `src/middle/core/ir_gen.rs`               | `224` / `369` / `1692-1702`                             | `method_def_ordinals` switched to read-only                                                                         |
| 27  | `src/middle/core/ir_gen.rs`               | `7269-8014`                                             | **Done last**: split into top-level `CallArgs` + 6 `emit_*`                                                         |
| 28  | `src/middle/core/ir_gen.rs`               | `7838-7846`                                             | "Fill 0" fallback changed to return a diagnostic                                                                    |
| 29  | `src/middle/lower/synth.rs`               | **New file**                                            | Synthesized-AST boundary                                                                                            |
| 30  | `src/middle/ir/verify.rs`                 | **New file** ([07](07-equivalence-oracle.md) layer one) | `verify_loose` / `verify_ssa`                                                                                       |
| 31  | `src/middle/passes/codegen/translator.rs` | dispatch match                                          | Add `Instruction::Phi` arm (option A: expanded to `Move` sequence)                                                  |
| 32  | `src/middle/passes/codegen/operand.rs`    | `36-45` / `72-77`                                       | `Operand::Value` resolution arm; remove `Temp` arm (`39-45`)                                                        |
| 33  | `src/middle/core/bytecode.rs`             | `127-554`                                               | **Unchanged under option A** (`Phi` does not enter bytecode)                                                        |
| 34  | `src/middle/core/bytecode.rs`             | `558` / `649` / `943-2350`                              | **Unchanged under option A**                                                                                        |
| 35  | `src/middle/core/bytecode.rs`             | `2312`                                                  | `upvalue_count: 0` fix                                                                                              |
| 36  | `src/backends/interpreter/executor/**`    | —                                                       | **Zero changes under option A**                                                                                     |
| 37  | `src/middle/passes/regalloc.rs`           | **New file**                                            | Linear-scan allocator                                                                                               |
| 38  | `scripts/ci/check-synth-boundary.py`      | **New file**                                            | `synth.rs` boundary sentinel check                                                                                  |

### Backward Compatibility: `.42` Format Version Number

**Verified facts**: the `.42` on-disk format **already has a version-number mechanism** —
`src/middle/passes/codegen/bytecode.rs:14-16` defines `MAGIC = 0x59584243` ("YXBC") and
`VERSION: u32 = 4`, written to the header (`:342-343`), and the read side validates (`:454-467`,
mismatches report "unsupported bytecode version").

**Design judgment (direct consequence of option A)**:

- `Phi` is expanded into a `Move` sequence at the `translator.rs` stage, **producing no new
  opcode**.
- `opcode.rs`'s 83 constants are **zero-change**, `opcode_name()` (`120`) is unchanged, and the
  decode match (`bytecode.rs:943-2350`) is unchanged.
- The `.42` format's field layout is **zero-change**, and `VERSION` stays at 4. Old `.42` files can
  still be read by new binaries, and new `.42` files can still be read by old binaries. **SSA
  conversion itself does not require a version bump.**

**The only scenario requiring a version bump is the `2312 upvalue_count` fix** (resolution D17: fix,
and handle with the version bump): add an `upvalue_count` field to `BytecodeFunction` and have the
encoder write it, which is a **format field addition**, bumping `VERSION` from 4 to 5. The read
side's existing version check rejects old files per its existing behavior — `.42` is a build
artifact (`main.rs:671`), cross-version read compatibility is not a goal, and the bump does not
require an extra migration mechanism. Already-allocated but unused opcode values (D32/D34's `Switch`
/ `TailCall`) are reclaimed with the bump.

## Implementation Notes

### Batching Strategy

Four batches. **Each item in a batch is an independent commit, independent criterion, independent
revert.** The "change item" column points to the number in the "Compiler Change List" above.

| Batch       | Content                                                                                                                                             | Change items                 | Criterion                                                                                                                                                  | Revert point                                                                                                          |
| ----------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| **Batch a** | Cut multiple definitions: delete 3 statement-level reclamations → 6 save/restore pairs switched to RAII guard → gauge unification                   | 8-10 (first) → 11-16 → 17-18 | Delete reclamations: **C4** (behavioral equivalence + layer-one dominance invariant). RAII and gauge unification are pure moves: **C1** snapshot zero-diff | Per-item single commit, `git revert` reverts                                                                          |
| **Batch b** | SSA shape switch: `ir.rs` structural change + `Operand` mechanical replacement (323 sites) → `translator.rs` adds `Phi` arm → linear-scan allocator | 1-5, 7 → 31-32 → 37          | **C4**: behavioral equivalence + `verify_ssa` fully green + `.42` round-trip test                                                                          | `ir.rs` change **occupies one commit** (type change affects whole-repo compilation, cannot be incrementally reverted) |
| **Batch c** | Make implicit contracts explicit: `synth.rs` boundary + sentinel script → span consumption count → `method_def_ordinals` read-only                  | 20-24, 29, 38 → 25 → 26      | **C4**: behavioral equivalence + new script into CI; consumption count zero-fires on new diagnostic (otherwise a real mismatch exists)                     | Independent commits                                                                                                   |
| **Batch d** | Split `generate_call_expr_ir` + "fill 0" fallback assertionification                                                                                | 27-28                        | Pure-move part: **C1** snapshot zero-diff. Assertionification part: **C4** corpus diff + layer-one verifier                                                | Independent commit                                                                                                    |

**Three in-batch orderings that cannot be swapped**:

1. Batch a's "delete reclamations" must precede batch b's `ir.rs` change — first make `next_temp`
   monotonically increasing, only then does `ValueId` have the semantic basis of "single
   definition".
2. Batch b's `Phi` arm must precede batch d's split — otherwise the split will move the old
   `arg_regs` order assumption verbatim into the new file.
3. Batch d must be last — it depends on the first two batches having converged `arg_regs` semantics
   to "each argument gets its own `ValueId`".

**Position relative to RFC-039**: all four batches above fall within RFC-039's **P7**; criterion
classification (`verify_loose` / `verify_ssa`) is built by P2. Between batches is **serial**, but
within batch a the 8-10 / 11-16 / 17-18 have only ordering constraints, no coupling.

### Prerequisites (Hard)

**Prerequisite one: criterion baseline.** The [07](07-equivalence-oracle.md) layer-one verifier's
`verify_loose` mode **must run green on the current (non-SSA) IR** before batch a is allowed.

Reason: the criterion for SSA conversion is C4 (behavioral equivalence + invariants). If the
invariant verifier itself cannot run green on the current IR before the rework, then every
acceptance of batches a–d lacks an executable criterion and can only rely on corpus diff — and
corpus diff is **precisely insensitive** to defect classes 1/2/3 (07 "Alternative A" has argued
this). **This is a real ordering constraint.**

**Prerequisite two: type representation convergence.** [03](03-type-unification.md) must complete
the `pub use ast::Type` convergence at `ir.rs:3` and tear out the `From<MonoType> for IrType` bridge
at `bytecode.rs:2353`. **Otherwise `ValueInfo::ty` introduces a third type-compatibility check.**

### Revert Strategy

- Each change is one independent commit, `git revert` granularity = change-item granularity.
- Batch b's `ir.rs` structural change is the **only non-incrementally-revertable** part (the type
  change affects whole-repo compilation). So it **must occupy one commit**, and batch a must have
  run green and have a revertable baseline before it.
- **No feature flags.** Reason: `Operand` variant add/remove cannot be isolated by a runtime switch;
  feature flags would make CI not test the other path long-term — this is exactly the "test wiring
  relies on human memory, corruption happens silently" recurrence recorded in RFC-039.

### Expectation Management: Net 900-1600 Line Increase

**This must be said up front, otherwise "line count did not go down" will be raised
mid-implementation.**

| Item                                  | Line change                                                                                                                                             |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting point                        | `ir_gen.rs` **8448**                                                                                                                                    |
| Delete 3 statement-level reclamations | **−15**                                                                                                                                                 |
| 6 save/restore pairs switched to RAII | net ±0 (RAII guard definition ~+20, use sites drop from 22 to ~12 lines)                                                                                |
| `next_temp_reg` rework                | +5                                                                                                                                                      |
| Split `generate_call_expr_ir`         | **+40 ~ 80** (6 `emit_*` signatures and parameter-passing overhead; this is the typical cost of a pure-move split, **splitting makes line count rise**) |
| Span-keyed consumption count          | +15                                                                                                                                                     |
| `method_def_ordinals` read-only       | +20                                                                                                                                                     |
| New file `synth.rs`                   | +80 ~ 120                                                                                                                                               |
| New file `verify.rs`                  | +400 ~ 600 (07 layer one, scoped to this document but counted independently)                                                                            |
| New file `regalloc.rs`                | **+500 ~ 1000**                                                                                                                                         |
| **Total**                             | **`ir_gen.rs` ~8448 → 8600 ~ 8800; including new files, net +900 ~ 1600 lines**                                                                         |

**The early expectation of "8448 → 5000~6000" is wrong**; the actual measured deduction is the
opposite. Reasons:

1. **This document's core benefit is not reducing line count, but eliminating an entire class of
   "wrong without error" defects.** The 6 manual save/restore pairs (22 lines) become ~12 lines +
   one guard type after SSA + RAII — **the line count barely changes, but "forgot to restore" shifts
   from a possibility to a compile error**.
2. **Pure-move splits necessarily increase line count.** `generate_call_expr_ir`'s 746 lines split
   into 7 functions, each repeating the signature (this function already has
   `#[allow(clippy::too_many_arguments)]`, `7268`), `&mut self` binding, and error propagation.
   **"Split files to reduce line count" is a misconception — it reduces cognitive load, not physical
   line count.**
3. **The new verifier and allocator are net additions.** These two (~900-1600 lines) are exactly
   **the carriers of the benefit**.

**Honest statement of net benefit**:

> In line count, this document is most likely a **net addition**. The benefit is in
> **verifiability** — turning "wrong without error" defects into "fail to run means report"
> invariants. If line count is the acceptance criterion, this document fails; the correct acceptance
> criterion is the one in "Batching Strategy".

**This phase will net-increase `ir_gen.rs`'s code volume, intentionally.** The 2026-10-03 decision
has cancelled all line-count / volume gates (see [08](08-maintenance-mechanism.md) ban two), so **no
line-count waiver list is needed** — the increment's justification lies in separation-of-concerns:
SSA conversion peels "register allocation + definition discipline" out of lowering into independent
modules, and the line-count increase buys elimination of the two classes of "misalignment silently
gives wrong value" hand-discipline: 6 manual save/restore pairs and `arg_regs` semantic reshuffling.

## Key Decisions and Reasons

### Three Core Decisions

**Decision one: temporary-value strategy — delete the three statement-level rollbacks, making single
definition a mathematical consequence of the construction.** The current rollbacks (`1954-1959` /
`3632-3638` / `4737-4742`) exist to reuse temporary slots. Deleting them makes `next_temp`
monotonically increase, and every temporary slot naturally has only one definition point. The
"consistent criterion but differing guards" class of hand-copied differences (the
`result_reg.is_none()` guard at `3632` versus the absence at the other two) disappears. The cost is
increased slot usage — the 255 cap check at `temp_high_water` (`1992-2000`) acts as a backstop.

**Decision two: do not do mem2reg, only do explicit renaming.** Reasons given in "Value
identification method": YaoXiang's `&` / reference / ownership semantics make "which slots can be
mem2reg'd" a type-system problem, requiring the converged type representation from
[03](03-type-unification.md) to judge. **Doing mem2reg before 03 is guessing.** Explicit renaming
only needs to change the return-value semantics of `next_temp_reg` plus delete the three rollbacks,
without touching the control flow of any lowering algorithm.

**Decision three: `Phi` adopts option A (IR-only pseudo-instruction, expanded to `Move` sequence).**
`executor/` and the `.42` format get **zero changes**, and the regression surface is confined to L3.
Decisive reason: changing the `.42` format triggers version-number issues and full
backward-compatibility discussions, while A's volume cost is irrelevant in interpreted execution
(what executes is `Move`, not `Phi`). Cost: `Phi` is invisible in `dump_bytecode` (need to be aware
of this in C1/C2 when comparing 07 layer three).

### Rejected Alternatives

**A. All-at-once split (SSA + split + `synth` boundary + allocator together). Rejected.** Violates
07's core principle: criteria must be classified by category. Mixing four batches in one PR mixes
pure-move changes with C4 changes, and snapshot drift cannot be attributed — once the snapshot
changes, you cannot tell whether the split moved wrong or the SSA changed wrong. **Revert
granularity degrades from "change item" to "everything".**

**B. Split files first, then optimize the order. Rejected.** Splitting does not eliminate any root
cause: the 6 manual save/restore pairs are still scattered across 10 files, and the `arg_regs`
semantic reshuffle still requires manual reasoning. Worse, **splitting first loses the opportunity
to "establish criteria before changing"** — splitting produces massive line-level diff that mixes
with subsequent SSA-change diff, making review ineffective. This is the same kind of issue as the 8
orphan test trees (1005 lines / 78 tests never run) recorded in 06-cleanup-inventory.md: **the wrong
order accelerates every subsequent step on a wrong foundation.**

**C. Introduce an intermediate SSA layer without changing the existing lowering. Rejected, but
recorded as "a third path once considered".** The shape is to leave `ir_gen.rs` unchanged and add a
new pass that converts its non-SSA IR output into SSA IR. **Advantage**: lowering is completely
untouched, easy to revert. **Reasons for rejection**: (1) It does not eliminate the root cause of
defect class 1 — the three statement-level reclamations remain in `ir_gen.rs`, and the 6
save/restore pairs are still 59-161 lines apart; a conversion pass can fold multiple definitions
into `Phi`, but **cannot fix "inner function's name leaks into outer scope"** — that is a
`cur_locals` save/restore error, happening before the IR shape, invisible to the conversion pass.
(2) It makes SSA a second representation — the repo has the prior art of 3 parallel type
representations, and this project's core lesson is "**boundaries only exist in human awareness**".
(3) `verify_loose` becomes a permanent burden.

**D. Keep the status quo and only add a verifier. Rejected as the sole solution, but it is part of
this document.** RFC-039 alternative B has already given the same judgment: gates can prevent
regression but cannot fix the status quo. **But it must be clear what D can achieve**: D can **catch
instances** of defect classes 1/2/3 (provided the verifier covers dominance, type consistency,
inner-layer isolation), but cannot catch defect class 4 (`closure_captures` leakage) — that requires
a cross-function liveness assertion, beyond `verify(&ModuleIR)`'s function boundary. **So D and this
document are not substitutes — D is a prerequisite of this document.**

## Known Limitations and Risks

### Risks

- **Net line increase of 900–1600 lines** (new `verify.rs` / `regalloc.rs`). **This is not a risk
  point** — the 2026-10-03 decision has cancelled line-count / volume gates, and scale issues are
  resolved by separation of concerns (see [08](08-maintenance-mechanism.md) ban two). The
  increment's justification is that it replaces the two classes of "misalignment silently gives
  wrong value" hand-discipline: 6 manual save/restore pairs and `arg_regs` semantic reshuffle.
- **`verify.rs`'s "single definition" check is meaningless in non-SSA shape** (`Operand::Local`
  allows multiple definitions) — 07 already records this limitation; `verify_loose` is its degraded
  mode before this batch.
- **Deleting statement-level reclamations will change the temporary-slot allocation pattern** (batch
  a). Even though `temp_high_water` guarantees no overflow, **the allocation result will change** —
  this is exactly why the C4 phase's "behavioral equivalence" criterion must actually run the full
  corpus, not a sample.
- **Whether the `u8` / 255 slot cap** (`codegen/operand.rs:39-45`) is still sufficient after
  deleting rollbacks is **unverified**. The E3014 check in `generate_function_ir` (`1992-2000`) will
  catch it, but catch means compile failure — possibly exposing a batch of huge functions previously
  masked by "rollback".
- **Advantages to be noted alongside**: turning "errors that cannot be measured" into "invariants
  that can be measured"; no backend changes (`executor/` and `.42` zero change); no `Instruction`
  semantic changes (76 → 77 variants all retained, 50 `translate_*` all retained); incremental (each
  item independently revertable).

> **The open issues originally listed in this section are all settled.** Per-item decisions are in
> [RFC-039 Decision Register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document leaves no pending items.**

## See Also

### Documents

- [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md) —
  overarching charter; four-layer model, criterion classification, P1-P10 execution order
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — **hard prerequisite**; C4 category
  criterion definition, `verify_loose` / `verify_ssa`, three-layer criteria, the limitation of
  snapshot immunity to `arg_regs`
- [03-type-unification.md](03-type-unification.md) — **hard prerequisite**; convergence of the 3
  parallel type representations
- [02-stage-contract.md](02-stage-contract.md) — `Obligations` ledger; `ReleasePlan`'s `PlanId`
  refactor's home
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — 8 orphan test trees; the negative case of
  "build directory first, fail to wire later"

### Code Locations

**IR definition**

- `src/middle/core/ir.rs:3` — `pub use ...ast::Type` ([03](03-type-unification.md) scope)
- `src/middle/core/ir.rs:12-20` — `Operand` 7 variants
- `src/middle/core/ir.rs:47-533` — `Instruction` 76 variants (54 with `dst: Operand`, 3 with
  `dst: Option<Operand>`)
- `src/middle/core/ir.rs:624-628` — `BasicBlock` (with `successors`)
- `src/middle/core/ir.rs:636-642` — `FunctionBody::Code` (with `entry` / `locals`)
- `src/middle/core/ir.rs:653-659` — `LocalSlot` (with `scope_depth` / `ty`)
- `src/middle/core/ir.rs:677` — `FunctionIR::def: Option<DefId>` (feasibility basis for the `PlanId`
  scheme)
- `src/middle/core/ir.rs:689-719` — `all_instructions` and other accessors

**Defect Class 1 (save/restore)**

- `src/middle/core/ir_gen.rs:248-249` — explicit contract "must be physically adjacent to the
  save/restore points of next_temp"
- `src/middle/core/ir_gen.rs:238-242` — same comment convention for `cur_span`
- `src/middle/core/ir_gen.rs:884-891` — `next_temp_reg`, includes `temp_high_water` true high-water
  mark
- `src/middle/core/ir_gen.rs:1954-1959` / `3632-3638` / `4737-4742` — three statement-level register
  reclamations
- `src/middle/core/ir_gen.rs:1798` vs `2001` — `take_cur_locals` gauge inconsistency
- `src/middle/core/ir_gen.rs:1987-2000` — E3014 register-overflow check (`MAX_REGISTERS = 255`)
- `src/middle/core/ir_gen.rs:1337` — `rebase_jump_targets` (referred to by 07's "jump targets exist"
  invariant)
- 6 save/restore function entries: `1677` / `1830` / `2105` / `2209` / `2759` / `4678`

**Defect Class 2 (arg_regs)**

- `src/middle/core/ir_gen.rs:7268` — `#[allow(clippy::too_many_arguments)]`
- `src/middle/core/ir_gen.rs:7269-8014` — `generate_call_expr_ir` (746 lines)
- `src/middle/core/ir_gen.rs:7395` / `7724` / `7835-7848` / `8000` — three semantic reshuffles and
  the clone
- `src/middle/core/ir_gen.rs:7838-7846` — silent "fill 0" fallback

**Defect Class 3 (span-keyed)** — the following are **audit-time** forensic pointers (after 4.5 key
migration: `release_plan` / `overload_resolutions` have been `PlanId`-ified; current-state pointers
are in [02](02-stage-contract.md) §2 revision notes; `method_def_ordinals` is still mechanism
three's todo)

- `src/middle/core/ir_gen.rs:191` / `222` / `224` — declarations of three cross-layer fields
- `src/middle/core/ir_gen.rs:341` / `1942` — `release_plan` write and consumption
- `src/middle/core/ir_gen.rs:7420` / `7493` — `overload_resolutions` consumption
- `src/middle/core/ir_gen.rs:369` / `1691-1700` — `method_def_ordinals` initialization and sole
  write point (key is module-unqualified `"Type.method"`)
- `src/frontend/core/typecheck/types.rs:31` / `56` — two fields on the transport struct
- `src/frontend/core/typecheck/layers/ownership.rs:26-32` / `2574` / `2786` — `ReleasePlan`
  definition and production
- `src/frontend/core/typecheck/checker.rs:1282` / `1440` / `1447-1448` — cross-layer transport
- `src/frontend/core/typecheck/inference/expressions.rs:4366` / `statements.rs:2758` / `2857` —
  `overload_resolutions` production

**Defect Class 4 (implicit ordering dependency)**

- `src/middle/core/ir_gen.rs:4143` / `4245-4256` — `generate_spawn_for_ir`'s AST synthesis and
  callback
- `src/middle/core/ir_gen.rs:6641` / `6698-6714` — `generate_spawn_expr_ir`'s same handshake
- `src/middle/core/ir_gen.rs:6543` / `6571-6572` / `6589-6593` / `6600` — `pending_env_*` and
  `closure_captures`
- `src/middle/core/ir_gen.rs:6284` / **`458`** — two read points of `closure_captures`
- `src/middle/core/ir_gen.rs:510` / `1266` / `1399` — 3 other `ast::Type` constructions

**Downstream**

- `src/middle/core/bytecode.rs:127-554` — `BytecodeInstr` 66 variants
- `src/middle/core/bytecode.rs:558` / `649` — `opcode()` / `size()`
- `src/middle/core/bytecode.rs:943-2350` — `impl From<BytecodeFile> for BytecodeModule` (1408 lines)
- `src/middle/core/bytecode.rs:2312` / `2315` / `2341` — three hard-coded drops (contrast `2344` /
  `2347`)
- `src/middle/core/bytecode.rs:2353` — `impl From<MonoType> for IrType` (bridge to be torn out by
  [03](03-type-unification.md))
- `src/middle/passes/codegen/translator.rs:533` / `631` / `632` / `655-657` / `658-660` — 9 NOP
  downgrades
- `src/middle/passes/codegen/operand.rs:36-45` / `72-77` — `OperandResolver`, `u8` cap 255
- `src/backends/common/opcode.rs:120` — `opcode_name()`; file has 83 constants total
- `src/backends/interpreter/executor/ops/control.rs:90` — active implementation of
  `BytecodeInstr::Switch` (no corresponding variant on the IR side)
- `src/backends/interpreter/executor/debug.rs:194` — dispatch match

**Test Status**

- `src/middle/core/ir_gen.rs` — 8448 lines, 0 `#[cfg(test)]`, 0 `mod tests`
- `src/middle/core/tests/mod.rs` — declares `bytecode` / `def_assign` / `local_slots`, total 29
  tests; `def_assign` / `local_slots` forward-pin DefId and slot naming, no IR structural invariant
  checks
- `src/middle/core/tests/bytecode.rs:421` / `423` / `1028` / `1173` — 4 references to ir_gen, all
  reverse assertions
- `src/middle/core/tests/bytecode.rs:958-1209` — `test_every_opcode_roundtrips_not_silently_nop`
  (per-opcode round-trip criterion example, the last test in the file; file has 1209 lines / 23
  `#[test]`)

**Engineering Configuration**

- `Cargo.toml:37-39` — dev-dependencies are only `criterion` / `proptest`; `[dependencies]` has no
  regalloc / petgraph / graphlib
- `docs/src/.vitepress/config.js:226-231` — sidebar auto-scans `/rfc/draft` directory; this document
  is not under that directory and is referenced by RFC-039 and this directory's index
