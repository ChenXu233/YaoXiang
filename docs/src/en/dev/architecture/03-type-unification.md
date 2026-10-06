# Type Representation Unification

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criterion tiering, and execution stage sequence are in the body of
> RFC-039; the positioning of each subsidiary document is in [this directory's index](index.md).

## Positioning and Scope

### Coverage

Converge the currently parallel multiple type representations into a single one, and eliminate the
four categories of burden derived from it:

- **13 `ast::Type` variants with zero production constructions** (out of 26 variants in
  `ast.rs:427-541`, see 2.2)
- **A handwritten type-name synonym table** (`mono.rs:618-643`, see 2.3)
- **The parser's real reverse dependency on the typecheck layer** (2 locations, see 2.4)
- **An expression variant with zero production constructions** `Expr::FnDef` (`ast.rs:37-43`, see
  2.6)

The specific deliverables are four items: the 26-variant per-variant disposition table (5.2), the
semantic kind-setting `NameKind` for `Type::Name` and elimination of the synonym table (5.3),
`TypeEnvProbe` injection-style data flow (5.4), and type-table generation-time gates T1-T3 (5.7).

### Directory Renaming and This Document's Path

RFC-039 decision D1 resolves that directory renaming (`typecheck/`→`sema/`,
`middle/core/`→`middle/ir/`) shall be executed, **completed together with P5/P6, not in stages**.
This document's 50-item change list and all acceptance greps are written **using the pre-rename
current paths**—they are the construction baseline for P6 batches **before the rename batch**; the
directory rename at the end of each batch is a **pure move batch** (C1 zero-diff), in its own
commit. It is expected behavior that the baseline becomes invalid after the rename commit; use git
history as the bisection basis.

### Non-Coverage

- **The construction method for directory renaming**. `Type` is not moved out of `parser/ast.rs` at
  this stage; only `lower.rs` is added under `types/` as the sole conversion point. Directory
  renaming (`typecheck/`→`sema/`, etc.) is executed at the end of P5/P6 per RFC-039 decision D1, the
  form is in the target directory structure of `01-routing.md`.
- **The four-layer model and dependency direction specification**. See
  [01-routing.md](01-routing.md).
- **The tiering of equivalence criteria and validator implementation**. The definitions of C1-C6,
  the three-tier criteria (IR validator / normalization snapshots / corpus diffing) are in
  [07-equivalence-oracle.md](07-equivalence-oracle.md). This document only references **C3 (type
  representation convergence)** as its own acceptance criterion.
- **SSA-ization**. See [04-ssa.md](04-ssa.md). This document is its predecessor; the reason is
  below.
- **The P1-P10 global execution sequence**. See the body of RFC-039 and
  [02-stage-contract.md](02-stage-contract.md). This document only provides its own line's stage
  division (see "Implementation Points").

### Division of Labor with RFC-039

RFC-039 is the upper-level master: the four-layer model, criterion tiering, cross-document stage
ordering, and "why do this refactoring now". This document is the **construction drawing for the
type representation convergence line** in RFC-039: which variants to delete, which files to change,
in what order, and what gates prevent regression.

All line numbers, variant lists, and construction-point statistics in this document come from static
verification against `src/`, and `file:line` is given for each item. **This document does not record
the execution process, only code facts and the dispositions derived from them.**

To facilitate cross-referencing with same-batch documents, the "Target Design" and "Detailed Design"
chapters reuse the `5.x` / `6.x` subsection numbering; other chapters follow this directory's
unified convention.

### Why It Must Precede SSA-ization

`ir::Type` is not a third independent type, but a `pub use` alias of `ast::Type` (`ir.rs:3`).
[04-ssa.md](04-ssa.md) needs to introduce register type annotations and new instructions to IR; if
representation is not converged before that, SSA's type annotations will grow directly on the
26-variant syntactic type via the `ir::Type` alias, effectively cementing a third representation.

The capability boundary of the current `ir::Type` is locked by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) embeds a full expression tree inside the type. An
  `Expr` appearing in an IR instruction's type annotation is impossible.
- `ast::Type::Literal { name, base_type }` (`ast.rs:476-482`) carries the source name and `Span`; it
  is the syntactic form of a compile-time literal type, not a runtime type.

Doing SSA type annotations on top of the 26 variants is equivalent to permanently solidifying these
13 dead variants and two syntax-specific variants into the IR contract.

## Current State

All of the following are code-fact statements, not proposals. Each item provides `file:line`
evidence.

### 2.1 Three Parallel Representations and Lossy Bridging

The multiplicity of type representation is the common upstream of four independent defects:

| Downstream symptom                                                    | Causal relationship with type representation                                                                                                                                                                                                                                                                                              |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The same type text has multiple valid AST forms                       | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but type-different**, the parser chooses the latter, so typecheck must supply a table to translate the former back                                                                      |
| Bytecode layer signatures inconsistent with IR layer signatures       | Because `ir::Type` is an alias not an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only get the alias, so `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`) while `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) declares `params` as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding a builtin type name requires multiple changes with no checking | Because the name→type mapping is scattered in `from_builtin_name` (`src/frontend/core/types/mono.rs:618-643`) and the string match in `bytecode.rs:2371-2376`; each is written once with no cross-checking                                                                                                                                |
| `"Terminates"` hardcoded in parser                                    | Because the parser cannot obtain "is this a type" or "is this an operator interface" judgments, so it can only embed type-layer knowledge as string literals into the syntax layer (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                            |

**The first three are representation problems; the fourth is a downstream side-effect of the
representation problem.** Doing only item 1 (deleting dead variants) leaves the synonym table and
reverse dependency; doing only item 4 (injecting a callback into the parser) builds on a
still-inconsistent representation.

One of the three representations is an alias illusion:

| Name        | Definition location                       | Lines       | Role                                                    |
| ----------- | ----------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541` | 26 variants | Syntax-layer type                                       |
| `MonoType`  | `src/frontend/core/types/mono.rs`         | 1077 lines  | Semantics-layer type (after monomorphization)           |
| `ir::Type`  | `src/middle/core/ir.rs:3`                 | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

`ir.rs:3` is a single `pub use` line; `ir.rs:6` immediately
`use crate::frontend::core::typecheck::MonoType;`. The IR module depends on two "type" symbols
simultaneously, but one is merely an alias.

There is a fourth type carrier on the serialization side:

| Carrier                         | Element type    | Location                                   |
| ------------------------------- | --------------- | ------------------------------------------ |
| `BytecodeModule.type_table`     | `Vec<ir::Type>` | `src/middle/core/bytecode.rs:854`          |
| `FunctionCode.type_table`       | `Vec<MonoType>` | `src/middle/passes/codegen/bytecode.rs:32` |
| Interpreter module `type_table` | `Vec<ir::Type>` | `src/backends/interpreter/image.rs:44`     |

**Lossy bridging.** `From<MonoType> for IrType` (`src/middle/core/bytecode.rs:2353-2390`) downgrades
`MonoType` to `ast::Type`; its **information loss is silent**:

```rust
// bytecode.rs:2371-2376
MonoType::Generic { name, args } => match name.as_str() {
    "String" => IrType::String,
    "Bytes" => IrType::Bytes,
    "Tuple" => IrType::Tuple(...),
    _ => IrType::Void,          // ← any other generic type silently becomes Void
},
// bytecode.rs:2378-2385
MonoType::Struct(_) | MonoType::Enum(_) | MonoType::Ref { .. }
    | MonoType::TypeVar(_) | MonoType::TypeRef(_) | MonoType::Union(_)
    | MonoType::Intersection(_) | MonoType::AssocType { .. } => IrType::Void,
```

8 `MonoType` variants all collapse to `IrType::Void`, plus a `_ => IrType::Void` catch-all
(`bytecode.rs:2386-2387`, the comment self-admits it's "a transitional branch before deleting the
enum variants").

`IrType::Void` here is **a legal type value, not an error signal**. Therefore downstream cannot
distinguish "this function really returns `Void`" from "this function's return type was lost during
bridging"—this is one of the reasons the "type consistency" invariant of 07's first-layer validator
cannot truly pass on the current code.

Meanwhile `IrType::String` / `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`) and
`ast::Type::Bytes` (`ast.rs:436`), which are exactly the dead variants to be deleted in 2.2—**both
ends of the lossy bridge point to dead code**.

### 2.2 Zero-Production-Construction Variants

**Criterion scope**: the construction-point scan covers the full `src/`, excluding `tests/` and
`*/tests/*`. Out of 26 `ast::Type` variants, the following 13 have **only match arms, no
construction points** in production code:

| Variant                              | Definition       | All production hits                                                                                                                                        |
| ------------------------------------ | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Int(usize)`                         | `ast.rs:432`     | `mono.rs:699` (arm)                                                                                                                                        |
| `Float(usize)`                       | `ast.rs:433`     | `mono.rs:700` (arm)                                                                                                                                        |
| `Char`                               | `ast.rs:434`     | `mono.rs:701` (arm)                                                                                                                                        |
| `String`                             | `ast.rs:435`     | `mono.rs:702` (arm)                                                                                                                                        |
| `Bytes`                              | `ast.rs:436`     | `mono.rs:703` (arm)                                                                                                                                        |
| `Bool`                               | `ast.rs:437`     | `mono.rs:707` (arm)                                                                                                                                        |
| `Void`                               | `ast.rs:438`     | `mono.rs:708` (arm), `types.rs:836` (**sole construction point**)                                                                                          |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | `mono.rs:747` (arm)                                                                                                                                        |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | `mono.rs:743` (arm)                                                                                                                                        |
| `Option(Box<Type>)`                  | `ast.rs:455`     | `mono.rs:770` (arm)                                                                                                                                        |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | `mono.rs:771` (arm)                                                                                                                                        |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | `mono.rs:814` (arm)                                                                                                                                        |
| `AssocType { .. }`                   | `ast.rs:463-471` | `mono.rs:781-786` (arm), `formatter/handlers/types.rs:84` (arm), `semantic_tokens.rs:228` (arm)—**no production construction point found in current code** |

The root-cause chain is fully verifiable:

1. **The parser represents all primitive types as `Type::Name { name }`**—the fallback arm at
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered into dedicated nodes**—the comment at `types.rs:392-396`
   explicitly states: "RFC-010: Result/Option are not lowered into dedicated AST nodes...they take
   the same `Type::Generic` path as user-defined generic types. The type representation remains
   `Generic{"Result"/"Option", args}`".
3. **`Type::Void`'s sole construction point is an error catch-all**—the
   `_ => (Vec::new(), Type::Void)` at `types.rs:836`, located at the end of a match returning
   `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction points: `types.rs:364`, `types.rs:571`, `types.rs:100` and
`declarations.rs:573`—they are not dead variants.

**These 13 variants have construction points in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
Therefore this document's statement is "**zero production construction**" rather than "zero global
reference". These tests are themselves the solidification of the tested behavior—they assert exactly
"what these syntactic forms can be lowered to", and the parser never produces these forms. When
deleting variants these tests must be deleted or rewritten together.

#### Criterion Boundary: Reverse Bridges Will Rebuild 7 of Them

The "sole construction point / no construction point" in the table above only holds for the **parser
→ AST forward construction** direction. In the reverse direction there are two production-level
`MonoType → ast::Type` reconstructions, which **actually construct 7 of the variants in the table**:

| Reverse bridge                                                 | Location                                 | Rebuilt 13-variant members                                                                                               |
| -------------------------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `From<MonoType> for IrType`                                    | `bytecode.rs:2353-2390`                  | `Int` (`2357`), `Float` (`2358`), `Bool` (`2359`), `Char` (`2360`), `Void` (`2361`), `String` (`2372`), `Bytes` (`2373`) |
| `mono_to_ast_type` (nested fn inside `substitute_type_in_ast`) | `middle/passes/mono/function.rs:507-529` | `Int` (`512`), `Float` (`513`), `Bool` (`514`), `Char` (`515`), `Void` (`516`), `String` (`522`), `Bytes` (`523`)        |

```
// middle/passes/mono/function.rs:511-527
match mono {
    MonoType::Int(n) => AstType::Int(*n),
    MonoType::Float(n) => AstType::Float(*n),
    MonoType::Bool => AstType::Bool,
    MonoType::Char => AstType::Char,
    MonoType::Void => AstType::Void,
    MonoType::TypeRef(name) => AstType::Name { name: name.clone(), span },
    // #299: String/Bytes and other container types are now Generic, reverse-resolved to AstType via type_name
    MonoType::Generic { name, .. } if name == "String" => AstType::String,
    MonoType::Generic { name, .. } if name == "Bytes" => AstType::Bytes,
    _ => AstType::Name { name: mono.type_name(), span },
}
```

**This criterion boundary has a direct effect on disposition**:

- The `bytecode.rs` site disappears together with #28 of 6.3 (delete the entire
  `From<MonoType> for IrType` impl), incurring no extra cost.
- `function.rs:507-529` is **not within the coverage of any existing change item**. It is a
  `MonoType → AstType` type-name reverse-resolver (comment tagged `#299`), which reverse-resolves
  `String` / `Bytes` from `Generic` back to dedicated variants by name. After deleting
  `ast::Type::String` / `Bytes` these two guard branches must be rewritten; after deleting `Int` /
  `Float` / `Bool` / `Char` / `Void` the `_` arm's behavior will change (the types that originally
  fell into `_` will silently switch to `AstType::Name`). This is supplementary item #47 of 6.3.
- **Direct consequence for the gates**: a T1 that only scans forward construction points will judge
  the above 7 variants as "has construction points", thereby excluding them from the deletable list.
  T1 must explicitly include reverse bridges in its statistical scope, otherwise the gate's
  conclusion contradicts this document's disposition table (see 5.7).

### 2.3 Handwritten Synonym Table

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:618-643`) uses a string table to map
type names to `MonoType`:

```
"Int" | "int" | "Int64" | "int64" | "i64"  => Some(MonoType::Int(64)),   // :620
"DateTime" | "datetime"                     => Some(MonoType::Int(64)),   // :627
"Int32" | "int32" | "i32"                   => Some(MonoType::Int(32)),   // :628
...
"Void" | "void" | "()"                      => Some(MonoType::Void),      // :640
```

**The sole reason this table exists is to bridge inconsistency at the AST layer**: because
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent but type-different at
the AST layer (and the parser only produces the latter), `from_builtin_name` must simultaneously
accept case variants, abbreviations, signed names, and the `DateTime` alias. The appearance of
`"()"` in the `Void` line is especially telling—it cooperates with the `Type::Void` catch-all at
`types.rs:836`.

The same knowledge of "which names are builtin type names" has four copies in the repo, with no
consistency check between them:

| #   | Location                              | Form                                                                                                                                 |
| --- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | `mono.rs:618-643` `from_builtin_name` | String match, name → `MonoType`                                                                                                      |
| 2   | `bytecode.rs:2371-2376`               | String match, copies `"String"` / `"Bytes"` / `"Tuple"` three items                                                                  |
| 3   | `ast.rs:838-843` `CONST_PARAM_TYPES`  | 15 const generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`...`"F64"` / `"Char"` / `"String"`) as a `&[&str]` constant |
| 4   | `src/lsp/world.rs:176`                | LSP-side builtin type-name list                                                                                                      |

Item 3 `CONST_PARAM_TYPES` is used by `extract_generic_param_names` at `ast.rs:912` to determine
const generic parameters—it enumerates "type names usable as const parameters", which is a
**partially overlapping but not identical** relationship to `from_builtin_name`'s full set, with no
mechanism to keep them in sync. Item 4 is the fourth copy for LSP syntax highlighting/hover, also
unchecked (T2's comparison scope includes it).

### 2.4 Parser's Reverse Dependency on the Type Layer

RFC-039 routing table C records 3 entries (`declarations.rs:27-80` calls `is_type_param_annotation`
/ `name_used_as_type`, both of which belong to the type layer). **The actual measurement differs
from that statement**: these two functions are defined inside the parser itself, not in the type
layer.

- `is_type_param_annotation` is defined at
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the sole `fn` definition in the repo)
- `name_used_as_type` is defined at `declarations.rs:42-80` (the sole `fn` definition in the repo)
- `declare_predicate` / `is_predicate_name` are defined at
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

The actual production code that cross-references `typecheck` is **2 locations**, both calling the
same function `crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Containing function                | Purpose                                                                                |
| --- | --------------------------------------------------------- | ---------------------------------- | -------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature parameter filter closure | Determines constraint-position formals (`T: Add`), not occupying runtime parameters    |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`      | Determines constraint-position formals, produces `GenericParamName` with `constraints` |

Location 2 is more inward than location 1—**`parser/ast.rs` itself** (the entire block at
`ast.rs:847-948`, in the same file as `StmtKind`/`Expr`) directly references typecheck symbols. The
routing table C only records one location in `declarations.rs`.

It forms the same judgment together with the hardcoding at `declarations.rs:498-499`:

```rust
// declarations.rs:498-499
// (Extended as built-in predicates are added later; future open-set approach:
// pass predicate names down to the parser, or change to a form determinable
// at the syntax layer.)
// The parser doesn't hold the type environment, so look at two places:
// built-in predicates hardcoded, plus the declarations accumulated in this
// pass (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment self-admits this is a temporary form of a "future open-set approach".
`n == "Terminates"` is **embedding type-layer knowledge as a string literal inside the syntax
layer**.

**Why this is a downstream of the type representation problem**: the reason the parser needs to ask
"is this an operator interface name" is that it must distinguish whether `Type::Name` refers to a
type or to an interface constraint. **`Type::Name { name: String }` carries at least 4 semantics
with an undifferentiated string** (concrete type / type variable / constraint name / predicate
name); the syntax layer cannot determine it on its own, so the determination is forced to be moved
up to the type layer.

#### Duplicate Implementation: Same Name and Meaning, Behavior Already Diverged

`name_used_as_type` has two implementations. The comment at `ast.rs:843-846` self-admits this:

```rust
/// Whether the parameter name is used as a type reference within the given type
/// (the `N` in `(N: Int) -> (n: N)`).
///
/// Synonymous with `name_used_as_type` in `declarations.rs`; this independent
/// implementation avoids cross-module dependencies within the parser.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations **have already diverged**:

| Dimension               | `declarations.rs:42-80` `name_used_as_type`                                     | `ast.rs:847-885` `name_used_as_type_in`              |
| ----------------------- | ------------------------------------------------------------------------------- | ---------------------------------------------------- |
| Signature               | Takes `is_predicate: &dyn Fn(&str) -> bool` callback                            | No callback                                          |
| `Type::Generic` branch  | Short-circuits with `is_predicate(app_name)` returning `false` first (`:54-56`) | No such short-circuit; directly recurses into `args` |
| `Type::Struct { body }` | **None** (falls to `_ => false`)                                                | **Yes** (`:871-879`, traverses `Field` / `Expr`)     |
| `Type::NamedStruct`     | **None**                                                                        | **Yes** (`:880-882`, traverses `fields`)             |

The comment at `ast.rs:868-870` records the origin of the latter two branches: "Previously missing
these two branches, when a const generic parameter is referenced in a definition body the 'used as
type' detection failed, and annotation validation would misreport a valid const parameter as
unknown."

That is: **the function duplicated to avoid cross-module dependencies, its fixes did not flow back
to the original**. The `ast.rs` version has field recursion inside the definition body, the
`declarations.rs` version does not—the same source code yields different conclusions at the two
judgment points. This must be merged together when `NameKind` is introduced in 5.3.

### 2.5 Parallel Operator Enums

There are two copies of the operator enum, mutually non-referencing and **with already-diverged
members**:

| Concept          | Syntax layer                    | Const evaluation layer                                                | Difference                                                                                                                     |
| ---------------- | ------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Binary operators | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | Syntax layer uniquely has `Neq` (`:197`), `Range` (`:205`), `Assign` (`:206`); const layer uses `Ne` (`:243`) instead of `Neq` |
| Unary operators  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | Syntax layer uniquely has `Deref` (`:222`); const layer uniquely has `BitNot` (`:329`)                                         |

The two also each implement a complete `Display` (`const_data.rs:291-317`) and classification
predicates (`const_data.rs:260-289`'s `is_arithmetic` / `is_comparison` / `is_logical` /
`is_bitwise`).

**Why this is part of the type representation problem**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) **are not types**—when they appear at type positions in
expressions, they fall into `Type::ConstExpr` (`ast.rs:506`), i.e., "compile-time-only variant".
This is directly related to the IR-layer type discipline in 6.1: after convergence,
`Type::ConstExpr` disappears after `Type → MonoType`, never visible to IR, while `Assign` / `Deref`
will continue to exist via `ir_gen.rs`'s `BinOp::Assign => Ok(MonoType::Void)`-style paths
(`inference/expressions.rs:875`). The divergence of the two enums makes it impossible to determine
"which operators can appear at type positions" from either side alone.

This stage does not merge the two enums (their assignment is in
[05-frontend-paradigm.md](05-frontend-paradigm.md)'s "operator change surface"), but T1's scan
target should include `ast::BinOp` / `ast::UnOp`, so that "newly added zero-construction operators"
are also visible.

### 2.6 `Expr::FnDef`: Expression Variant with Zero Production Construction

`Expr::FnDef` (`ast.rs:37-43`) **is** zero-construction in production—the entire repo has only two
construction sites: `src/frontend/core/parser/tests/ast.rs:837` and
`src/frontend/core/typecheck/tests/checker.rs:111`, both tests.

But **consumption points are far more than 4**. The actual production consumption points number 12:

| File                                                     | Line           |
| -------------------------------------------------------- | -------------- |
| `src/frontend/core/spawn/placement.rs`                   | `122`          |
| `src/frontend/core/spawn/analysis.rs`                    | `912`          |
| `src/formatter/handlers/expr.rs`                         | `43`           |
| `src/frontend/module/orchestrator.rs`                    | `1492`         |
| `src/middle/core/ir_gen.rs`                              | `4325`, `5097` |
| `src/frontend/core/typecheck/checker.rs`                 | `1642`         |
| `src/frontend/core/typecheck/checker/semantic_tokens.rs` | `1416`         |
| `src/frontend/core/typecheck/inference/expressions.rs`   | `3374`         |
| `src/frontend/core/typecheck/inference/existential.rs`   | `55`           |
| `src/frontend/core/typecheck/passes/dead_code.rs`        | `305`          |
| `src/frontend/core/typecheck/layers/ownership.rs`        | `1207`         |
| `src/frontend/core/typecheck/layers/termination.rs`      | `752`          |

There are also 2 exhaustive match arms forced to exist for it: `ast.rs:1026` (`Expr::span()`) and
`pratt/mod.rs:47` (`expr_end_line`).

**The actual path function definitions take** is: `nud.rs:425-429` constructs `Expr::Lambda` +
`declarations.rs:462-479` constructs `StmtKind::Assign` (putting the `Lambda` into `value`). In
other words, the `FnDef` branch is **a never-executed parallel path maintained by 14 locations**.

### 2.7 Syntax Nodes Preserving Semantics for the Type Layer

**First, `StmtKind::Assign` carries a typechecker-specific field.** The `Assign` variant at
`ast.rs:241-249` contains `signature_params: Vec<Param>` (`ast.rs:244-245`), with the comment
stating directly:

> `/// The first group of signature parameters as-is (including parameter names), for the typechecker's classify_generic_params`

The statement AST reserves a dedicated slot for the type checker. Construction site:
`declarations.rs:472`.

**Second, expression positions are treated as parameter lists.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a **Lambda with empty
body**. Subsequently the `Expr::Lambda { params, .. } => Some(params.clone())` arm at
`src/frontend/core/parser/pratt/led.rs:466` "fishes" it back out as a parameter list.

That is: a normal variant of AST is used as a **temporary parameter-list carrier**, with another
module's match arm restoring the semantics.

**Third, `Type::NamedParen` carries return-position binder names.** The comment at `ast.rs:519-540`
states explicitly:

> RFC-027 §3's return-position refinement relies on it to declare **return formal parameter names**
> (called binders here)...**dropping it, the type checker can only guess** "free variables in
> constraints not in scope are return value formals", so `(r: P(m))` would silently substitute
> undeclared `m` as a binder too (verified defect).

Syntax nodes are preserving binder identity for the type layer.

## Target Design

### 5.1 Shape of the Target Representation

**Proposition: single representation = `Type` (originally `ast::Type`) as the sole type structure;
`MonoType` demoted to a working form inside the type checker; the `ir::Type` alias deleted.**

Three representations converge to **two roles**, but only one is "type":

```
Sole type structure  Type  ─────────────────────┐
   (26 → 13 variants, living under types/ not parser/ast.rs)  │
                                                  │  From<Type> for MonoType  (total, no semantic loss)
                                                  │  ← sole conversion point types/lower.rs
Typecheck working form  MonoType  ──────────────┘
   (carries TypeVar / substitution state, only flowing inside typecheck)
   ×
ir::Type alias deleted, BytecodeFunction.params / type_table switched to MonoType
```

**Why not "merge into `MonoType`"** (this is the easiest proposed scheme, rejected here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.),
   `MonoType` does not. 07's C3 criterion is "diagnostic codes and messages are the same"—the
   diagnostic's position information must trace back to the source. Adding `Span` to `MonoType`
   would make post-monomorphization types carry a pile of meaningless sentinel spans.
2. `MonoType` contains `TypeVar` and substitution state (`types/substitute.rs:112`,
   `types/solver.rs:411`'s `pub fn unify`). These are **intermediate states of the type checker's
   solving process**, not types themselves. Putting them into the sole type structure means the IR
   construction layer needs to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027), lowered away in `MonoType`. Unifying to `MonoType` means we
   need to add these syntactic forms back to it.
4. **The most critical reverse argument**: elevating `MonoType` to the sole representation means
   making the L2 syntax layer depend on the L3 semantics layer's type. RFC-039 explicitly states
   that L2 must not depend on L3. One of this document's goals is to eliminate reverse dependencies;
   choosing a solution that creates reverse dependencies is self-contradictory.

**Why `ir::Type` must be deleted directly, not "preserved"**:

The alias at `ir.rs:3` makes `BytecodeFunction.params` (`bytecode.rs:812`) and `return_type`
(`bytecode.rs:814`) use `ir::Type`, while the serialization side at `codegen/bytecode.rs:277-278`
uses `MonoType`. The two field types are connected by the lossy `From` bridge at
`bytecode.rs:2339`'s `file.type_table.into_iter().map(|t| t.into())` and `bytecode.rs:2353-2390`.

**The sole reason the `From` bridge exists is "the two sides' field types are different"**. Unifying
`bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, `image.rs:44` to `MonoType` makes
`From<MonoType> for IrType` lose its reason to exist, and can be deleted in its entirety (also
eliminating the reverse reconstruction recorded in 2.2).

**Explicitly not done**: not moving `Type` out of `parser/ast.rs` into `sema/types/`. This stage
only adds `lower.rs` under `types/` as the sole conversion point; `Type`'s definition location
stays. Directory renaming, per D1, executes at the end of P6 as an independent move batch—at that
time `Type`'s home is the top-level `ast/type_.rs` (AST top-level domain), and `sema/types/` is
`MonoType`'s working-form home.

### 5.2 26-Variant Per-Variant Disposition Table

> Disposition classification: **Delete** (zero production construction), **Preserve** (has
> production construction points), **Migrate** (semantics assigned to another node).

| #   | Variant                                                            | Definition line  | Disposition                                  | Reason                                                                                                                                                                                                                                                                          |
| --- | ------------------------------------------------------------------ | ---------------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Preserve** (strengthen)                    | The parser's **sole** primitive type exit (`types.rs:162-165`). After deleting the 12 primitive type variants, all primitive types go through it. Need to add a "semantic kind of the name" annotation (see 5.3)                                                                |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                                   | Zero production construction (`mono.rs:699` is the sole arm). Bit width carried by `from_builtin_name`'s `Name` path                                                                                                                                                            |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                                   | Zero production construction (`mono.rs:700`)                                                                                                                                                                                                                                    |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                                   | Zero production construction (`mono.rs:701`)                                                                                                                                                                                                                                    |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                                   | Zero production construction (`mono.rs:702`). **Note**: `bytecode.rs:2372`'s `IrType::String` consumes it; the arm must be rewritten after deletion                                                                                                                             |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                                   | Zero production construction (`mono.rs:703-706`). Same as above with `bytecode.rs:2373` consumption                                                                                                                                                                             |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                                   | Zero production construction (`mono.rs:707`)                                                                                                                                                                                                                                    |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                                   | Sole production construction is the error catch-all `types.rs:836`. That catch-all is eliminated together with the error-path refactoring in 5.6                                                                                                                                |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Preserve**                                 | Has production consumption (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                             |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Preserve**                                 | Has production construction point `types.rs:364`                                                                                                                                                                                                                                |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                                   | Zero production construction (`mono.rs:743-746`)                                                                                                                                                                                                                                |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                                   | Zero production construction (`mono.rs:747`). Enums go through `Struct` + `TypeBodyItem`, unrelated to `mono.rs:2378`'s `MonoType::Enum(_) => IrType::Void`                                                                                                                     |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Preserve**                                 | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                  |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:451-454` | **Preserve**                                 | Consumed by `bytecode.rs:2366`; **field reshape `params: Vec<Type>` → `Vec<Param>` (name and type co-located) follows 6.8.1 (D54)**—`Assign.signature_params`'s name is taken over here (see 6.5.2)                                                                             |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                                   | Zero production construction (`mono.rs:770`). `types.rs:392-396` already explains taking the `Generic` path                                                                                                                                                                     |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                                   | Zero production construction (`mono.rs:771`). Same as above                                                                                                                                                                                                                     |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Preserve**                                 | `Result` / `Option` / `String` / `Bytes`'s actual representation all go through it (`types.rs:397-399`)                                                                                                                                                                         |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                                   | Zero production construction (sole construction in the entire repo is in test `types/tests/mono.rs:178`; the syntax layer has no `::` path producing it). If associated-type syntax is enabled in the future, it will be redefined by a new proposal at that time (decision D8) |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                                   | Zero production construction (`mono.rs:814`)                                                                                                                                                                                                                                    |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Preserve**                                 | Has production construction point `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                        |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Preserve**                                 | Raw pointer type inside unsafe blocks                                                                                                                                                                                                                                           |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Preserve**                                 | Borrow notation                                                                                                                                                                                                                                                                 |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Preserve**                                 | Has production construction points `types.rs:100`, `declarations.rs:573`; RFC-010                                                                                                                                                                                               |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Preserve (and mark as compile-time only)** | Has production construction point `types.rs:160`. The only variant in `Type` that embeds `Expr`; forbidden in the IR layer (see 6.1)                                                                                                                                            |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Preserve**                                 | RFC-004 currying terminator, `split_curry` depends on its existence                                                                                                                                                                                                             |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                                  | Semantics (binder name) moves to the type checker; AST only preserves syntax. See 5.6 for details                                                                                                                                                                               |

**Net effect: 26 → 14 variants** (delete 12, migrate 1, preserve 13).

> **Reverse bridges must be addressed before deleting variants**: the `function.rs:507-529` recorded
> in 2.2 will reconstruct 7 of items #2-#8 in this table. In deletion order it must be handled in
> the same batch as #1-#8, otherwise the function's `_` arm will silently change behavior (see #47
> of 6.3).

> **`AssocType`'s handling**: per D8, delete directly (item #18 of table 5.2). T1's gate (5.7) on
> un-modified code's first report **must independently reproduce this conclusion**—if the report
> shows `AssocType` has a production construction point, it means this section's verification is
> wrong; **the correct action is to go back to the RFC decision table to change D8 and explain the
> reason**, not to keep the variant in the whitelist.

### 5.3 `NameKind` and Synonym Table Elimination

**Problem**: `Type::Name { name: String }` carries at least 4 semantics with an undifferentiated
string—concrete type / type variable / constraint name (operator interface) / predicate name.
`declarations.rs:498-499` needs to determine "is it a predicate", `declarations.rs:508-511` needs to
determine "is it an operator interface", `ast.rs:930` needs to determine "is it an operator
interface", `declarations.rs:42-80` and `ast.rs:847-885`'s two `name_used_as_type*` need to
determine "is it a type reference"—all relying on string comparison.

**Proposal: add a `kind` field to `Type::Name`, determined once by the parser**.

```rust
// Target form (illustrative; line numbers are after changes)
Name {
    name: String,
    kind: NameKind,   // newly added
    span: Span,
}

enum NameKind {
    Builtin,      // Builtin type name: Int / Float / Bool / ... → handed to from_builtin_name
    UserType,     // User-declared type name (including Generic's std.result / std.option)
    TypeVar,      // Type variable: T / K / V
    Constraint,   // Operator interface / constraint name: Add / Ord / ...
    Predicate,    // Compile-time predicate: Terminates / Sorted / ...
}
```

**Key design point: what `NameKind` determination needs is exactly the information the parser cannot
currently obtain** (type environment, declared operator interface table, declared predicate table).
Therefore this field **cannot be determined independently inside the parser**—it must be determined
by an injected environment. This directly connects to 5.4.

**Synonym table disposition**:

`from_builtin_name` (`mono.rs:618-643`) is **preserved, but with demoted semantics**. Before
refactoring it was a "disambiguator bridging two equivalent representations at the AST layer"; after
refactoring `Type::Int(64)` is deleted, "Int" has only one representation path, so it degrades to a
**sole name→type resolution table**.

Specific reasons to preserve it (cannot delete):

- The comment at `"DateTime" | "datetime" => Some(MonoType::Int(64)` (`mono.rs:627`) records a real
  fix (related to issue #338: `now()`'s return value can't be passed to `format_time`). This is
  **alias** semantics, not the same as `Int`/`int`'s case aliasing.
- It is the **sole cut-in point** between standard library type names and language builtin names.

But the **redundancy inside the table can be cut**. After refactoring the parser will only produce
normalized case (`Int` / `Int32` / `Float` / `Bool` / ...), so:

| Category                                           | Current (`mono.rs:620-642`) | Target                                                                                                                                                                             |
| -------------------------------------------------- | --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Case aliases (`"int"` / `"i64"` / `"int64"`)       | Preserved                   | **Delete**—parser normalization won't produce them                                                                                                                                 |
| Abbreviation aliases (`"i64"` / `"i32"` / `"f64"`) | Preserved                   | **Move to lexer layer** (registered as equivalent writings of `Int` in the keyword table at `src/frontend/core/lexer/state.rs`); `from_builtin_name` only accepts normalized names |
| `DateTime` alias                                   | Preserved (`mono.rs:627`)   | **Preserve**—semantic alias, with comment                                                                                                                                          |
| `"()"` (`mono.rs:640`)                             | Preserved                   | **Delete**—after deleting the `Type::Void` catch-all (`types.rs:836`), there's no source producing it                                                                              |

**Net effect**: `from_builtin_name` converges from 12 match arms to about 6 normalized names; its
semantics shifts from "disambiguation table" to "resolution table", and **becomes the single source
of truth for type names** (5.7's gate verifies it).

**Merging the two `name_used_as_type*`**: the two duplicate implementations recorded in 2.4 must be
merged into one, otherwise the kind-setting information provided by `NameKind` will be bypassed by
two independently maintained string-comparison logics. The merged implementation goes into
`parser/ast.rs` (data source is near `Type`); `declarations.rs` is changed to call it; the
`is_predicate` short-circuit is changed to read `NameKind::Predicate`; the field recursion for
`Struct` / `NamedStruct` is taken from the `ast.rs:871-882` version (newer, with more complete
fixes).

**`CONST_PARAM_TYPES`'s assignment**: the const generic parameter name table at `ast.rs:838-841`
partially overlaps with `from_builtin_name`'s full set (see 2.3). After `NameKind::Builtin` is
introduced, the judgment at `ast.rs:912` can be changed to "`kind == Builtin` and the name is in the
const-parameter-acceptable set", explicitly bridging the two pieces of knowledge; the
const-parameter subset definition is verified by the T2 gate (see 5.7).

> **Design judgment**: if `.yx` corpus contains type annotations written as `i64` / `int`,
> normalizing them to `Int` will change these files' **diagnostic positions** (not the diagnostic
> codes). Per the C3 criterion this is allowed (messages and ordering can be normalized), but the
> baseline must be recorded in stage 1.

### 5.4 Data Flow for Predicates/Type Names

**Current data flow** (closed loop inside parser + 2 boundary crossings):

```
ast.rs (extract_generic_param_names, :930)  ─┐
declarations.rs:509                          ─┴─→ typecheck::operator_interfaces::spec()   ✗
declarations.rs
  ├─ declare_predicate()      ← parser_state.rs:46 (parser self-holds, only declarations parsed in this pass)
  ├─ n == "Terminates"        ← hardcoded literal (declarations.rs:499)
  └─ (the boundary calls above)
```

**Target data flow** (injection, no boundary crossing):

```
L1 Orchestration layer (constructed once at entry)
   │  ① Compile built-in operator interface table
   │  ② Compile std's .yx declarations → predicate name set
   ▼
TypeEnvProbe (trait, defined in L2, src/frontend/core/parser/probe.rs)
   │  fn is_predicate(&self, name: &str) -> bool
   │  fn is_constraint(&self, name: &str) -> bool
   ▼
ParserState (extended at parser_state.rs:46/54, delegates to probe)
   ▼
declarations.rs:498-511 and ast.rs:930 changed to
   let is_predicate_app = |n: &str| state.probe().is_predicate(n);
   if state.probe().is_constraint(n) { ... }
```

**Per-file changes**:

| File                                                                                    | Change                                                                                                                                                                                                                                                                                  |
| --------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/probe.rs` (new)                                               | Define `trait TypeEnvProbe`, providing four pure-query methods: `is_predicate` / `is_constraint` / `is_type_var` / `is_builtin_type`. **Trait defined in L2, implementation provided by L3**—this is legitimate dependency inversion                                                    |
| `src/frontend/core/parser/parser_state.rs:46-58`                                        | Keep `declare_predicate` / `is_predicate_name` (for this-pass accumulation); add `probe: Box<dyn TypeEnvProbe>` field and `probe()` accessor                                                                                                                                            |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                           | Delete `n == "Terminates"`; change to `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                                                                   |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                           | Delete `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; change to `state.probe().is_constraint(n)`                                                                                                                                                                |
| `src/frontend/core/parser/ast.rs:930`                                                   | Delete `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` is the definition site for `Type` / `Expr` / `StmtKind`, and should not hold environment dependencies—this function needs to be changed to accept a `&dyn TypeEnvProbe` parameter (see below) |
| All 5 parser construction sites (`Parser::new` / `ParserState::new` inside `parser.rs`) | Accept `probe: Box<dyn TypeEnvProbe>` parameter. **Default implementation `NullProbe` (returns false for everything)** ensures parser unit tests are not affected                                                                                                                       |

**Signature issue with `extract_generic_param_names`**: the
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` at `ast.rs:891` is a
free function that doesn't accept any context. After deleting the `ast.rs:930` boundary call it
needs `is_constraint` judgment, so the signature must add `probe: &dyn TypeEnvProbe`. Callers
(including `parser/tests/ast.rs`) synchronously pass `&NullProbe`.

**`NameKind` determination timing**: when `types.rs:162-165` constructs `Type::Name`, use
`state.probe()` to determine `kind`, set it once, and no subsequent match arm does string
comparison.

**This eliminates all 3 entries of RFC-039 routing table C**—2 of them per the 2.4 scope only
involved parser-internal functions; the real cross-layer references are the 2 locations
`declarations.rs:509` and `ast.rs:930`, both covered by this section.

### 5.5 `Expr::FnDef` Merge

**Proposition: delete `Expr::FnDef` (`ast.rs:37-43`); function definitions uniformly go through
`Expr::Lambda` + `StmtKind::Assign`.**

**This is the already-running path**: `nud.rs:425-429` + `declarations.rs:462-479` (see 2.6). The
`FnDef` branch is a never-executed parallel implementation.

**Change list** (12 consumption + 2 exhaustive arms, listed one by one):

| Location                             | Existing arm                                         | Target                                                                                                                                                  |
| ------------------------------------ | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `spawn/placement.rs:122`             | `Expr::FnDef { body, .. } => self.check_block(body)` | Merge into the existing `Expr::Lambda` arm (`body` is also `Box<Block>`, **delete this arm directly**)                                                  |
| `spawn/analysis.rs:912`              | `Expr::FnDef { body, .. } => { ... }`                | Same as above, merge into the Lambda arm                                                                                                                |
| `formatter/handlers/expr.rs:43`      | `Expr::FnDef { ... }`                                | Change to format the Lambda in `Assign`; delete this arm                                                                                                |
| `orchestrator.rs:1492`               | `if let Expr::FnDef { name, .. }`                    | Change to get the name from `StmtKind::Assign { target, .. }`                                                                                           |
| `ir_gen.rs:4325`                     | `ast::Expr::FnDef { span, .. } => *span`             | Merge into the Lambda arm (`Lambda` also carries `span`)                                                                                                |
| `ir_gen.rs:5097`                     | `\| ast::Expr::FnDef { span, .. }`                   | Delete this or-pattern                                                                                                                                  |
| `checker.rs:1642`                    | `if let ...::Expr::FnDef {`                          | Change to judge `Expr::Lambda`                                                                                                                          |
| `checker/semantic_tokens.rs:1416`    | `Expr::FnDef {`                                      | Delete this arm                                                                                                                                         |
| `inference/expressions.rs:3374`      | `...::Expr::FnDef {`                                 | Change to judge `Expr::Lambda`                                                                                                                          |
| `inference/existential.rs:55`        | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                                  |
| `passes/dead_code.rs:305`            | `Expr::FnDef {`                                      | Change to judge `Expr::Lambda`                                                                                                                          |
| `layers/ownership.rs:1207`           | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                                  |
| `layers/termination.rs:752`          | `Expr::FnDef {`                                      | Change to judge `Expr::Lambda`; the comment at `termination.rs:2303` "`fn(): Never` — FnDef.return_type is a bare return type" is updated synchronously |
| `ast.rs:1026`                        | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                                  |
| `pratt/mod.rs:47`                    | `Expr::FnDef { span, .. } => *span`                  | Delete this arm                                                                                                                                         |
| `typecheck/tests/checker.rs:109-111` | Test constructs `Expr::FnDef`                        | Rewrite as `Expr::Lambda` + `Assign`                                                                                                                    |
| `parser/tests/ast.rs:837-847`        | Test constructs `Expr::FnDef`                        | Delete this test                                                                                                                                        |

**No behavior change**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, `span`,
isomorphic to `FnDef`'s first three; `FnDef`'s unique `name` field is not usable at the `Expr` layer
(the name is on `StmtKind::Assign.target`).

**Risk and mitigation**: non-exhaustive `if let` matches like `checker.rs:1642`,
`inference/expressions.rs:3374` become never-matching after refactoring—no error, no warning.
Mitigation: the type-table gate (5.7) adds a "zero-construction variant" check; after `Expr::FnDef`
is deleted, if someone rewrites `if let Expr::FnDef` it will fail to compile directly (variant
doesn't exist), **this is a compile-time guarantee, no gate needed**.

### 5.6 `signature_params` and `NamedParen` Disposition

**`StmtKind::Assign.signature_params` (`ast.rs:244-245`) → Delete.**

Reason: its reason for existence (the comment at `ast.rs:244`) is "for the typechecker's
`classify_generic_params`"—**a dedicated slot reserved by the statement AST for the type checker**.
But the `extracted_params` at `declarations.rs:500-511` has already computed `value_params`
(filtering of type-position and constraint-position completed) within the same function, so passing
it down with the statement is duplicated transport.

Change: delete the field at `ast.rs:244-245` → delete the argument at `declarations.rs:472` → change
the data source of `classify_generic_params` to recompute on-site (it can get `Lambda.params` from
`value: Option<Box<Expr>>`). **Net deletion: one field declaration, one construction site, one
transport path.**

> **Risk**: `signature_params` carries the **unfiltered** parameter list, while `Lambda.params` is
> the **filtered** one (the filter at `declarations.rs:500-511` removes type-position and
> constraint-position). If `classify_generic_params` depends on the unfiltered version, behavior
> will change after refactoring. **Must verify in stage 1 which one `classify_generic_params`
> actually uses**; cannot assume equivalence.

**`Type::NamedParen` (`ast.rs:533-540`) → Preserve the node, move out binder semantics.**

The comment at `ast.rs:519-532` records its **sufficient reason for existence** (see 2.7): if `r` in
`(r: P(m))` is dropped, the type checker can only guess "free variables in constraints not in scope
are return value formals", which would **silently substitute undeclared `m` as a binder too** (the
comment marks this as a "verified defect").

Therefore **it cannot be simply deleted**—that would reintroduce a fixed correctness defect. The
correct disposition is:

1. `NamedParen { param, param_span, inner }` is preserved as a **pure syntax node**;
   `param`/`param_span` only carry source-code facts and don't bear the type-layer binder semantics.
2. Binder interpretation rights are transferred to `sema/`: when the type checker consumes
   `NamedParen`, it **explicitly** extracts `param` as a binder. This is "reading syntax node
   fields", same nature as `ConstExpr` embedding `Expr`—syntax nodes provide facts, the semantics
   layer does interpretation.
3. The comment changes from "the type checker can only guess" to "this node provides the binder
   name, semantic interpretation is in `sema/`".

**Why this is "migrate" rather than "delete"**: the syntax tree must be able to represent the source
code losslessly. `r` really exists in the source code, and the AST cannot pretend it doesn't. The
problem is not the node's existence, but that **the responsibility for semantic interpretation isn't
clearly defined**.

### 5.7 Type-Table Generation-Time Gates

**Reference object already verified in this repo**: `build.rs:19-37` calls `tools/code-tables`'s
`parse_registry` / `validate` to perform uniqueness, segment, and zh-completeness checks on 145
error codes; on `is_ok()` being false it directly `panic!` and refuses to compile; `build.rs:39-54`
further compares the RFC-013 markdown code table item-by-item, and on inconsistency also `panic!`.
**RFC-013 documents and code are therefore always consistent**—`tools/code-tables/Cargo.toml:15-16`
has only a `serde_json` dependency; the verification logic is text-level parsing, not depending on
`syn`.

The type table currently has **no** similar gate. The mechanism form to be added in this document
can directly follow this pattern.

**New crate**: `tools/type-tables` (`Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). Four checks:

| #                                                          | Check                                                                                                                                                                                                                                                                                         | Input                    | Failure condition                                                                                               | Remedy                                                              |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| **T1 Zero-construction variant**                           | Scan the `pub enum Type` (and `Expr` / `StmtKind` / `BinOp` / `UnOp`) variant list in `src/frontend/core/parser/ast.rs`; count each variant's **construction points** under `src/` (excluding `tests/` and `*/tests/*`), **and separately mark the reconstruction points in reverse bridges** | Variant list + hit table | Some variant has **zero construction points** and isn't in the explicit whitelist                               | Delete the variant, or move it to the whitelist and note the reason |
| **T2 Synonym table consistent with whole-repo name list**  | The `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` at `ast.rs:838-843` + the LSP builtin type-name list at `src/lsp/world.rs:176`                                                                                                                                   | Three tables             | Some type name exists in one table but is not recognized in another (compared by each's semantic mapping rules) | `type-tables --fix` bidirectional completion                        |
| **T3 Builtin type names consistent with RFC-011 document** | The normalized name set in `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                                         | Two tables               | Code-table intervals/names inconsistent with the registry                                                       | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --fix`   |
| **T4 `Type → MonoType` exhaustiveness**                    | The `Type` variant list vs the match arms in `types/lower.rs`                                                                                                                                                                                                                                 | Two tables               | Some variant has no corresponding arm, or some arm has no corresponding variant                                 | Add arm / delete arm                                                |

**Implementation note for T4**: Rust's exhaustive match already guarantees the "variant → arm"
direction at compile time. T4's value is in the **reverse** direction—detecting **stale arms
pointing to deleted variants** in `lower.rs` (such code fails to compile at the moment the variant
is deleted, so T4 is effectively redundant). **T4 is therefore downgraded to a CI assertion (passing
if `cargo build` succeeds) and does not enter `build.rs`'s `panic!` path.** T1-T3 are the three that
truly need text parsing.

**Two implementation difficulties for T1 and mitigations**:

1. **Distinguishing construction points from match arms** requires syntax analysis. The root
   `Cargo.toml` has **no `syn` dependency**, and `tools/code-tables` also only has `serde_json`.
   Mitigation:
   - **Preferred**: introduce `syn` to `tools/type-tables` (only this crate; the root crate is
     untouched). `syn` is the standard practice, and the gate crate is independent of the compiler.
   - **Fallback**: reuse the existing `tools/extract_arm.py` (an existing Python tool in the repo)'s
     line-level heuristics—construction points are on the right side of `= ` or in the function
     argument position, match arms are on the left side of `=>`. Use the `=>` occurrence count as an
     approximation.
   - **Conservative starting point**: on T1's first launch, **only report, don't fail**
     (`cargo:warning`); take one full baseline to confirm the heuristic is correct before changing
     to `panic!`. This aligns with "the CI gate will be red initially and needs a baseline first."
2. **The construction points in reverse bridges must be categorized separately**. 2.2 records two
   `MonoType → ast::Type` reconstructions in `bytecode.rs:2353-2390` and `function.rs:507-529`. If
   T1 mixes their reconstruction points with the parser's forward construction, the 7 variants `Int`
   / `Float` / `Bool` / `Char` / `Void` / `String` / `Bytes` will be judged as "has construction
   points" and exempted from deletion, directly contradicting the 5.2 disposition table. T1's report
   must separate the two types of construction points, and require a separate explanation for
   reverse-bridge reconstructions in the whitelist mechanism.

**Gate coverage**: add a `type_tables::validate(root, &entries)` call in `build.rs`, located
immediately after the existing block at `build.rs:19-37`, following the
`panic!("Type table validation failed ({} items of error ...)", ...)` form.

## Detailed Design

### 6.1 Type System Impact

| Impact surface                                               | Judgment                                                                                                                                                                                                                                                                                                                                             |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact**. Inference consumes `MonoType` throughout and does not touch `ast::Type` variants                                                                                                                                                                                                                                                      |
| Solver (`types/solver.rs`)                                   | **No impact**. `pub fn unify` (`solver.rs:411`), the `substitute` at `types/substitute.rs:112`, the `substitute` at `types/mono.rs:649` all operate on `MonoType`                                                                                                                                                                                    |
| Constant evaluation (`types/eval/`)                          | **No impact**. `eval/const_eval.rs` (1128 lines), `eval/dependent_types.rs` (`check_structural_termination` at `478`), the `unify` at `eval/reducer.rs:572` all operate on `MonoType`                                                                                                                                                                |
| Monomorphization (`middle/passes/mono/`)                     | **Needs change**. `mono/function.rs:358`, `362`, `451`, `456`, `554`, `558`, `618`, `623`—8 places match `AstType::NamedStruct` / `AstType::AssocType`—if `AssocType`'s disposition is determined as deletion, these 8 places need to be updated synchronously. **Also need to change `mono_to_ast_type` at `function.rs:507-529`** (see #47 of 6.3) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Needs change**. The `ast::Type::NamedStruct` match at `ir_gen.rs:1266`, `1399`; the `let params: Vec<MonoType> = signature_params` at `ir_gen.rs:1380` (this is another consumption point of `signature_params`; 5.6 needs to be verified together)                                                                                                |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Needs change**. `type_id_to_monotype` at `codegen/bytecode.rs:492-495`, the encoding loop at `351-352`, the assembly at `632`                                                                                                                                                                                                                      |
| Interpreter (`backends/interpreter/`)                        | **Needs change**. `type_table: Vec<ir::Type>` at `image.rs:44` changed to `Vec<MonoType>`; the type-name formatting by `type_table` index at `repl/eval.rs:369-370` will be affected by the element type change in `{:?}` output format                                                                                                              |
| RFC-027 compile-time types                                   | **Needs change**. `Type::ConstExpr` is preserved but marked as **compile-time only**; after `Type → MonoType` (`types/lower.rs`) it disappears, and is never visible in the IR layer                                                                                                                                                                 |

**IR-layer type discipline (to be inherited by 04-ssa)**: after convergence, IR's type annotations
can only take the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only** variants, and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. This discipline should be
enforced by 07's first-layer validator's "type consistency" invariant. `ast::BinOp::Assign` /
`ast::UnOp::Deref` (2.5) are the expression-side forms of the same problem—they can appear inside
`ConstExpr`, and therefore likewise must not sink down to IR type annotations.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** Basis:

1. The 11 deleted variants have no **forward** construction points in production code → no source
   code can produce them (reverse-bridge disposition see #47/#28 of 6.3).
2. `ir::Type` is an alias of `ast::Type` → changing it back to `MonoType` does not change the type
   set, only the **carrier**.
3. The **loss** of `From<MonoType> for IrType` (8 variants → `Void`) will no longer happen after the
   change—this **may** change some programs' behavior (previously lost types are now preserved).
   **This is the only direction of behavior change, and it is a fix not a regression**, but it must
   be confirmed by 07's third-layer corpus diffing to be regression-free.

**Semantic change points that need focus**:

| Location                | Current                                                                                       | After convergence                       |
| ----------------------- | --------------------------------------------------------------------------------------------- | --------------------------------------- |
| `bytecode.rs:2371-2376` | `Generic` other than `String`/`Bytes`/`Tuple` → `IrType::Void`                                | Preserves the real `MonoType::Generic`  |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Preserves the real `MonoType`           |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` catch-all                                                                 | **Delete** (exhaustive match is enough) |

> **Design judgment**: the fact that all these 8 variants currently collapse to `Void` means **the
> code paths depending on the signature bytecode (interpreter's arity/type check, serialization
> format) currently see `Void` uniformly**. After convergence these positions will see real types.
> **This is the only potential source of behavior change in this document, and the direction is
> "restore correct information"**. If corpus diffing shows differences, it should be treated as
> **exposing an existing defect**, not a regression introduced by this document—the judgment method:
> separately construct a `.yx` program containing `Ref` parameters, and compare `dump_bytecode`
> before and after the change.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                   | Change                                                                                                                                                                                                                               | Category                 |
| --- | ---------------------------------------------------------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------ |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472` | Delete `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants                                                                                                                                  | Pure deletion            |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                              | `Name` adds `kind: NameKind` field                                                                                                                                                                                                   | Data flow                |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43`                                | Delete `Expr::FnDef`                                                                                                                                                                                                                 | Pure deletion            |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                              | Delete `Assign.signature_params`                                                                                                                                                                                                     | Pure deletion            |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026`                                 | Delete `Expr::FnDef` or-pattern                                                                                                                                                                                                      | Follows #3               |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                              | `NamedParen` comment changed to "syntax provides binder name, semantic interpretation in `sema/`"                                                                                                                                    | Doc                      |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                        | Add `pub enum NameKind`                                                                                                                                                                                                              | New                      |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                    | `trait TypeEnvProbe` + `NullProbe`                                                                                                                                                                                                   | New                      |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                | Add `probe` field and accessor                                                                                                                                                                                                       | Data flow                |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                              | Use `state.probe()` to set `NameKind`                                                                                                                                                                                                | Data flow                |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                              | Comment update (`Result`/`Option` take `Generic`, already consistent with fact; only add variant-deletion note)                                                                                                                      | Doc                      |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                  | Delete `_ => (Vec::new(), Type::Void)` catch-all; change to explicit error return                                                                                                                                                    | Behavior                 |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                              | Delete `n == "Terminates"`; change to `state.probe().is_predicate(n)`                                                                                                                                                                | Reverse dependency       |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                              | Delete `typecheck::operator_interfaces::spec()` call                                                                                                                                                                                 | Reverse dependency       |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                  | Delete `signature_params` argument                                                                                                                                                                                                   | Follows #4               |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                   | Delete `Expr::FnDef` arm                                                                                                                                                                                                             | Follows #3               |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                  | Preserve (the parameter-list fish-back point; with 5.5 changed to read Lambda directly)                                                                                                                                              | Follows #3               |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814` | Delete 11 match arms                                                                                                                                                                                                                 | Follows #1               |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                              | Cut synonyms (case aliases, `i64`-style abbreviations, `"()"`)                                                                                                                                                                       | Convergence              |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                  | `DateTime` alias preserved                                                                                                                                                                                                           | Preserve                 |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                    | Migrate `From<Type> for MonoType` (currently at `mono.rs:692-830`) as the sole conversion point                                                                                                                                      | New                      |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                          | Register `i64`/`int` etc. as equivalent writings of `Int`                                                                                                                                                                            | Convergence              |
| 23  | `src/middle/core/ir.rs`                                                | `3`                                    | Delete `pub use ... ast::Type`                                                                                                                                                                                                       | Pure deletion            |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                  | `FunctionCode.params` stays `Vec<MonoType>` (already so)                                                                                                                                                                             | —                        |
| 25  | `src/middle/core/bytecode.rs`                                          | `812`, `814`                           | `BytecodeFunction.params` / `return_type` changed to `MonoType`                                                                                                                                                                      | Convergence              |
| 26  | `src/middle/core/bytecode.rs`                                          | `854`                                  | `BytecodeModule.type_table` changed to `Vec<MonoType>`                                                                                                                                                                               | Convergence              |
| 27  | `src/middle/core/bytecode.rs`                                          | `2339`                                 | `type_table` conversion changed to `.collect()` (elements already of the same type)                                                                                                                                                  | Follows #26              |
| 28  | `src/middle/core/bytecode.rs`                                          | `2352-2390`                            | **Delete** the entire `From<MonoType> for IrType` impl                                                                                                                                                                               | Pure deletion            |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                   | `type_table` changed to `Vec<MonoType>`                                                                                                                                                                                              | Convergence              |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 places)                 | `AstType::AssocType` arm handled per gate conclusion                                                                                                                                                                                 | TBD                      |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`           | Type-table gates T1-T3                                                                                                                                                                                                               | Gate                     |
| 32  | `spawn/placement.rs`                                                   | `122`                                  | Delete `Expr::FnDef` arm                                                                                                                                                                                                             | Follows #3               |
| 33  | `spawn/analysis.rs`                                                    | `912`                                  | Same as above                                                                                                                                                                                                                        | Follows #3               |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                   | Same as above                                                                                                                                                                                                                        | Follows #3               |
| 35  | `orchestrator.rs`                                                      | `1492`                                 | Same as above; change to get name from `Assign.target`                                                                                                                                                                               | Follows #3               |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                         | Same as above                                                                                                                                                                                                                        | Follows #3               |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                 | Same as above                                                                                                                                                                                                                        | Follows #3               |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                 | Same as above                                                                                                                                                                                                                        | Follows #3               |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                 | Same as above                                                                                                                                                                                                                        | Follows #3               |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                   | Same as above                                                                                                                                                                                                                        | Follows #3               |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                  | Same as above                                                                                                                                                                                                                        | Follows #3               |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                 | Same as above                                                                                                                                                                                                                        | Follows #3               |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                          | Same as above + comment update                                                                                                                                                                                                       | Follows #3               |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                              | Delete `Expr::FnDef` test                                                                                                                                                                                                            | Test                     |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                              | Rewrite as Lambda + Assign                                                                                                                                                                                                           | Test                     |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`        | Delete lower assertions for the 11 dead variants                                                                                                                                                                                     | Test                     |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                              | `mono_to_ast_type` deletes `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches, uniformly going through `AstType::Name` + `mono.type_name()`. **Must be handled in the same batch as #1**                          | Convergence              |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                           | `extract_generic_param_names` signature adds `probe: &dyn TypeEnvProbe`; delete `typecheck::operator_interfaces::spec()` call and change to `probe.is_constraint(name)`. Callers (including `parser/tests/ast.rs`) pass `&NullProbe` | Reverse dependency       |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                | Delete the `name_used_as_type` inside this file; change to call the merged version in `ast.rs`; `is_predicate` short-circuit changed to read `NameKind::Predicate`                                                                   | Duplicate implementation |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                  | `CONST_PARAM_TYPES` judgment changed to wire through `NameKind::Builtin` (see 5.3)                                                                                                                                                   | Convergence              |

> **The gate itself must also be gated**: `tools/type-tables`, like `code-tables`, needs `mod`
> declaration wiring (the lesson of the 8 orphan test trees already in this repo). Register the new
> crate synchronously in the root `Cargo.toml`.

### 6.4 Backward Compatibility Strategy

**Constraint**: the 293 `.yx` corpora in `tests/yaoxiang/` (single-file) + the multi-file corpus
layer established by P2 (`tests/yaoxiang-multifile/`) + `src/std/tests/*.yx` + the standard library,
**must all have unchanged behavior** (diagnostic codes and messages the same, order normalizable).

| Risk                                                             | Judgment                                                                                                                                                                                                                                                                                                                             | Mitigation                                                                                                                                                                                                                                                                              |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source code contains `i64` / `int` / `f64` etc. type annotations | After normalization to `Int`, `from_builtin_name` only accepts normalized names                                                                                                                                                                                                                                                      | **First thing in stage 1**: scan the full corpus and count occurrences of non-normalized type names. If 0, synonyms can be deleted directly; if not 0, per 5.3 move them into the lexer's alias table (**syntax layer accepts, lower layer normalizes**, no impact on diagnostic codes) |
| Source code contains `DateTime`                                  | Preserve alias (`mono.rs:627`)                                                                                                                                                                                                                                                                                                       | No impact                                                                                                                                                                                                                                                                               |
| `("()")` appears at type position                                | The `LParen` branch at `types.rs:167` and following handles tuples/parameter groups, producing `Tuple` not `Name{"()"}`                                                                                                                                                                                                              | Before deleting `"()"`, count the occurrences of `"()"` as a type name in the full corpus first                                                                                                                                                                                         |
| `dump_bytecode` output change                                    | `type_table` element type changes from `ir::Type` to `MonoType`; `{:?}` format differs                                                                                                                                                                                                                                               | **Allowed by the C3 criterion** (07 specifies `dump_bytecode` is only compared at the C1/C2 stage). But the output diff needs to be recorded in stage 2 for manual review                                                                                                               |
| Behavior change after lossy-bridge fix                           | See 6.2                                                                                                                                                                                                                                                                                                                              | Corpus diff + targeted `dump_bytecode` comparison                                                                                                                                                                                                                                       |
| `mono_to_ast_type` rewrite changes type-substitution result      | `function.rs:507-529` is the exit of monomorphization type substitution (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back to `AstType`). After deleting variants, if it goes through the `_` arm, the substitution result changes from `AstType::Int(64)` to something like `AstType::Name { name: "i64" }` | Stage 3 targeted comparison: select corpora containing generic substitution, compare the `type_name()` of monomorphization products before and after the change. **Must not assume `{:?}` output is equivalent**                                                                        |
| `.yx` source code itself                                         | **Zero modification**                                                                                                                                                                                                                                                                                                                | This document does not require modifying any corpus or std source files. This is a hard acceptance item                                                                                                                                                                                 |

**Hard acceptance condition**: `git diff --stat tests/ src/std/` must be empty (except for additions
of multi-file corpus layer and test baseline files).

## Implementation Points

The criterion uniformly uses 07's **C3: diagnostic codes and messages the same (order
normalizable)**. The following is this line's stage division; the cross-document global sequencing
is in RFC-039.

| Stage                                         | Content                                                                                                                                                                                                                                          | Acceptance criterion                                                                                                                                                                                                                         | Rollback point                         |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| **0. Gate first**                             | Create `tools/type-tables`, implement T1/T2/T3; T1's first version **only reports, doesn't fail**. Take the full baseline on **un-modified** code                                                                                                | T1's report accurately lists 13 zero-construction variants (including the qualitative conclusion on `AssocType`), and **separately lists the reverse-bridge reconstruction points recorded in 2.2**; T2/T3 all green or gives a fixable list | No code changes, no rollback needed    |
| **1. Name normalization**                     | Count non-normalized type names in corpus; `from_builtin_name` (`mono.rs:620-642`) cuts synonyms, moves them into the lexer's alias table. **Synchronously verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`** | C3: full corpus diagnostic set is **item-by-item identical** to stage 0 baseline                                                                                                                                                             | Single commit, revertable              |
| **2. Bytecode type unification**              | Change `bytecode.rs:812`/`814`/`854`, `image.rs:44` to `MonoType`; delete `From<MonoType> for IrType` (`bytecode.rs:2352-2390`)                                                                                                                  | C3 + targeted `dump_bytecode` comparison (each item in 6.2's table)                                                                                                                                                                          | Single commit, revertable              |
| **3. Dead-variant deletion**                  | Delete 11 variants per the 5.2 table; clean `mono.rs:699-708` etc. match arms, the `types.rs:836` catch-all; **handle `function.rs:507-529` (#47) in the same batch**; synchronize `types/tests/mono.rs`                                         | C3 + `cargo build` all green (exhaustive match forces all arms to be completed) + targeted comparison of generic substitution (last line of 6.4)                                                                                             | Single commit, revertable              |
| **4. Parser data flow**                       | Add `probe.rs`; change `parser_state.rs:46-58`; change `declarations.rs:498-511` and `ast.rs:930` (#48); `types.rs:162-165` sets `NameKind`; merge the two `name_used_as_type*` (#49)                                                            | C3 + **new assertion**: `grep 'typecheck' src/frontend/core/parser/` should be 0                                                                                                                                                             | Single commit, revertable              |
| **5. AST dead variants and dedicated fields** | Delete `Expr::FnDef` (16 locations) + `Assign.signature_params`; `NamedParen` semantic migration                                                                                                                                                 | C3 + `tests/integration/` 18 modules all green                                                                                                                                                                                               | Two commits (FnDef / signature_params) |
| **6. Gate becomes hard**                      | T1 changes from warning to `panic!`; T4 downgraded to CI assertion                                                                                                                                                                               | `cargo build` **must fail** when intentionally introducing a zero-construction variant (red test first)                                                                                                                                      | Single commit, revertable              |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Stages 3 and 5 must follow stage
2**—`bytecode.rs:2372-2373` consumes `IrType::String` / `IrType::Bytes`; deleting the variants first
will break compilation. Stage 3's #47 must be in the same batch as #1, otherwise `mono_to_ast_type`
silently changes behavior.

**Dependencies on downstream**: after all stages of this document are complete,
[04-ssa.md](04-ssa.md) can begin. At this time `BytecodeFunction.params`, `FunctionCode.params`,
`type_table` have a unified element type, and SSA's type annotations have a unique landing point.

### Mandatory Verification per Stage

| Stage | Must execute                                                                                                                                                              |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`, manually check T1's report (**including separate listing of reverse-bridge reconstruction points**) |
| 1-5   | `cargo test`; full corpus diff on `tests/yaoxiang/` (07's third layer); C3 criterion compared item-by-item                                                                |
| 2     | Additional: construct a `.yx` sample for each variant in 6.2's table, `dump_bytecode` comparison before and after the change                                              |
| 3     | Additional: corpora containing generic substitution, compare the `type_name()` of monomorphization products before and after the change                                   |
| 4     | Additional: `rg 'typecheck' src/frontend/core/parser/` returns 0                                                                                                          |
| 6     | Additional: intentionally write a zero-construction variant, confirm `cargo build` fails (red→green)                                                                      |

## Key Decisions and Reasons

### Decisions

| Decision                       | Determination                                                                                                                                | Reason                                                                                                                                                                                                                                         |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Target representation shape    | `Type` as sole type structure + `MonoType` as working form; `ir::Type` alias deleted                                                         | The alias at `ir.rs:3` is the direct cause of `BytecodeFunction.params` and `FunctionCode.params` field-type inconsistency (2.1). After the alias is deleted, the two field types are unified, and the `From` bridge loses its reason to exist |
| Reject "unify into `MonoType`" | Not adopted                                                                                                                                  | Would cause L2→L3 reverse dependency (violating RFC-039 layering), and would need to add `Span` / `ConstExpr` back to `MonoType`. Full argument in 5.1                                                                                         |
| `NamedParen` not deleted       | Preserved as pure syntax node, binder semantics migrated to `sema/`                                                                          | The record at `ast.rs:519-532` shows that deletion would reintroduce a fixed correctness defect: `(r: P(m))` would silently substitute undeclared `m` as a binder                                                                              |
| Directory renaming             | **Do it** (D1), executed as a pure move batch at the end of P5/P6; this document's body paths are written using the pre-rename current state | 2026-10-03 decision (ChenXu233); 2026-10-04 supplementary construction method                                                                                                                                                                  |

### Rejected Alternatives

Four categories of alternatives were not adopted: **keeping the three representations and only
adding a conversion layer** doesn't address any root cause, and `ir::Type` is still not an
independent type; dead variants, synonym table, and reverse dependency—none of them disappear;
**using macros to generate the synonym table** treats symptoms, not cause—after deleting
`Type::Int(usize)` the macro has no reason to exist, replaced by gate T2 for consistency guarantee
(more verifiable than macro expansion correctness); **only deleting dead variants without
unification** has the lowest cost but leaves the problem in place, and with no gate to prevent the
next person from adding `Type::Int128(usize)` and the same dead variants growing back (if only one
commit can be made, it should be clearly noted as a subset of this scheme with T1 delivered
together); **unify into `MonoType`** see 5.1.

### Benefits

- **Eliminate maintenance burden at 14 locations**. The 12 consumption + 2 exhaustive arms of
  `Expr::FnDef` is a never-executed parallel path; after deletion, `ir_gen.rs` / `ownership.rs` /
  `termination.rs` each have one fewer branch.
- **Eliminate a duplicate implementation that has already diverged**. The two copies of
  `name_used_as_type` (2.4) have already diverged on definition-body field recursion; after merging
  there is only one.
- **After the lossy bridge disappears, `IrType::Void` is no longer a "legal type value"**. This is
  the precondition for 07's first-layer validator's "type consistency" invariant to truly pass.
- **The gate makes regression impossible**. T1 turns "newly added zero-construction variant" from
  silent pass to build failure—the existing `build.rs` mechanism in this repo proves this path
  works.
- **Zero source code change**. The 293 `.yx` corpora and the std library need no modification—this
  is the fundamental reason the C3 criterion can hold.

## Known Limitations and Risks

### Risks

- **Stage 2 may have behavior change**. After the lossy-bridge fix, 8 `MonoType` variants can now
  see real types instead of `Void`. The direction is a fix, but it will change the paths that depend
  on signature bytecode. This is the document's largest uncertainty.
- **`AssocType`'s qualitative conclusion depends on gate accuracy**. If T1 uses line-level
  heuristics (`=>` count) instead of `syn`, it may misjudge. This is why stage 0 requires manual
  verification.
- **T1 missing reverse bridges will make the disposition table and gate contradict each other**. 2.2
  has proven that 7 "dead variants" have production reconstruction points in reverse bridges. If T1
  doesn't separate the construction-point direction, these 7 will be judged as "has construction
  points" and exempted from deletion—the gate will become a resistance to deletion work rather than
  a guarantee. | **2. Deleting `signature_params` has semantic risk**. It carries the **unfiltered**
  parameter list, while `Lambda.params` is the **filtered** one. If `classify_generic_params`
  depends on the former, refactoring will change the generic classification result. **Must verify
  first; cannot assume equivalence** (the `signature_params` consumption point at `ir_gen.rs:1380`
  on the ir_gen side is also verified). | **3. Stage 3 touches `middle/passes/mono/function.rs`**.
  Besides the 8 matches for `AssocType`, the `mono_to_ast_type` at `507-529` is the exit of
  monomorphization type substitution; after rewriting, the `type_name()` of the substitution result
  may change (last line of 6.4).
- **Non-normalized type names in `.yx` corpus may be non-zero**. If the corpus heavily uses `i64` /
  `int`, stage 1's "directly delete synonyms" is not feasible, and degrades to "move into lexer's
  alias table"; the convergence extent of `from_builtin_name` decreases.
- **The gate will be red initially**. If T1's first launch directly does `panic!` it will
  immediately block compilation. Must first only report to take a baseline.

### Unresolved Questions

> **The open questions originally listed in this section have all been adjudicated.** Item-by-item
> decisions are in the [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md)
> (D1–D50). **This document leaves no pending items.**

## See Also

### Upper-Level and Same-Batch Documents

- [RFC-039 Compiler Function Routing Directory Design (Master)](../../rfc/accepted/039-compiler-architecture.md)
  — Upper-level master; four-layer model, criterion tiering, routing table C (3
  type-representation-related entries belong to this document)
- [01-routing.md](01-routing.md) — Function routing tables A/B/C, dependency-direction
  specification, target directory structure
- [04-ssa.md](04-ssa.md) — SSA-ization; this document's predecessor and downstream consumer
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — Frontend paradigm; operator change-surface
  convergence
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C3 category definition, diagnostic-set
  comparison specification, type-consistency invariant of the first-layer IR validator

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — Basis for
  `Type::Generic` unified path
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — Builtin type table
  (T3 gate's comparison object)
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Example
  of `build.rs` generation-time gate
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Source of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

- `src/frontend/core/parser/ast.rs:427-541` — `ast::Type` 26-variant definition
- `src/frontend/core/parser/ast.rs:519-540` — `Type::NamedParen` binder-semantics explanation
  (evidence that deletion reintroduces a defect)
- `src/frontend/core/parser/ast.rs:241-249` — `StmtKind::Assign.signature_params`
  typechecker-dedicated field
- `src/frontend/core/parser/ast.rs:838-841` — `CONST_PARAM_TYPES`: third copy of const generic
  parameter names
- `src/frontend/core/parser/ast.rs:843-885` — `name_used_as_type_in`: duplicate implementation of
  `declarations.rs:42` (already diverged)
- `src/frontend/core/parser/ast.rs:891`, `930` — `extract_generic_param_names` and the boundary call
  inside
- `src/frontend/core/parser/ast.rs:191-213`, `217-223` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — Parser represents all primitive types as
  `Type::Name`
- `src/frontend/core/parser/statements/types.rs:392-396` — `Result`/`Option` are not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:836` — `Type::Void`'s sole forward construction
  point (error catch-all)
- `src/frontend/core/parser/statements/declarations.rs:42-80` — `name_used_as_type` original
- `src/frontend/core/parser/statements/declarations.rs:494-511` — Hardcoded `"Terminates"` and
  `typecheck::operator_interfaces::spec()` call
- `src/frontend/core/parser/pratt/nud.rs:505-514` — Expression position represented as empty-body
  `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:466` — `expr_to_params` arm that fishes the parameter list
  back
- `src/frontend/core/types/mono.rs:618-643` — `from_builtin_name` synonym table
- `src/frontend/core/types/mono.rs:692-830` — `From<Type> for MonoType` implementation (12 of the 13
  match arms are here)
- `src/frontend/core/types/mono.rs` — `MonoType` definition (about 22 variants, from `mono.rs:167`;
  1077-line file)
- `src/frontend/core/types/const_data.rs:234-258`, `321-330` — `const_data::BinOp` /
  `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — Compile-time constant evaluation (1128 lines)
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- `src/middle/core/ir.rs:3` — Evidence of `ir::Type` as `ast::Type` alias
- `src/middle/core/bytecode.rs:808-814` — `BytecodeFunction`'s `Vec<ir::Type>` field
- `src/middle/core/bytecode.rs:2352-2390` — Lossy `From<MonoType> for IrType` bridge
- `src/middle/passes/codegen/bytecode.rs:32`, `275-278` — `FunctionCode`'s `Vec<MonoType>` field
- `src/middle/passes/mono/function.rs:507-529` — `mono_to_ast_type`: reverse bridge that
  reconstructs 7 "dead variants"
- `src/backends/interpreter/image.rs:44` — Interpreter module's `type_table`
- `build.rs:19-37` — Error code registry build-time gate (form reference for type-table gate)
- `build.rs:39-54` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — Gate crate depends only on `serde_json` (root crate has no
  `syn`; `tools/type-tables` introducing `syn` is the first exception)
