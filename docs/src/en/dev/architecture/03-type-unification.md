---
title: Type Representation Unification
---

> **Subsidiary design document**. This document is a subsidiary of
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiering, and execution phase ordering are in the body of
> RFC-039; the positioning of each subsidiary document is in [this directory's index](index.md).

## Positioning and Scope

### Coverage

Collapse the currently parallel multiple type representations into one, and eliminate the four
classes of burden derived therefrom:

- **13 production-zero-construction `ast::Type` variants** (out of the 26 variants at
  `ast.rs:427-541`, see 2.2)
- **A hand-written type-name synonym table** (`mono.rs:618-643`, see 2.3)
- **The parser's real reverse dependency on the typecheck layer** (2 sites, see 2.4)
- **A production-zero-construction expression variant** `Expr::FnDef` (`ast.rs:37-43`, see 2.6)

The concrete deliverables are four: a 26-variant per-variant disposition table (5.2), the semantic
kind `NameKind` for `Type::Name` and the synonym-table elimination (5.3), the `TypeEnvProbe`
injected data flow (5.4), and the type-table generation-time gates T1–T3 (5.7).

### Directory Rename and This Document's Path

RFC-039 decision D1 determines that the directory rename (`typecheck/`→`sema/`,
`middle/core/`→`middle/ir/`) is to be executed, **completed together with P5/P6, not phased**. The
50-item change list and all acceptance greps in this document are written against the **pre-rename**
current paths — they are the construction baseline for each P6 batch **before the rename batch**;
the directory rename at the close of each batch is a **pure-move batch** (C1 zero-diff), a separate
commit. The baseline becoming invalid after the rename commit is expected behavior; use git history
as the bisection basis.

### Out of Scope

- **The construction approach for the directory rename**. `Type` is not moved out of `parser/ast.rs`
  in this phase; only `lower.rs` is added under `types/` as the sole conversion point. The directory
  rename (`typecheck/`→`sema/`, etc.) is executed at the close of P5/P6 per RFC-039 decision D1; the
  form is the target directory structure in `01-routing.md`.
- **The four-layer model and dependency-direction specification**. See
  [01-routing.md](01-routing.md).
- **The tiering of equivalence criteria and validator implementation**. The definitions of C1–C6,
  the three-tier criteria (IR validator / normalized snapshot / corpus diff) are in
  [07-equivalence-oracle.md](07-equivalence-oracle.md). This document references only **C3 (type
  representation convergence)** as its own acceptance criterion.
- **SSA-ization**. See [04-ssa.md](04-ssa.md). This document is its prerequisite; the reason is
  below.
- **P1–P10 global execution sequence**. See the body of RFC-039 and
  [02-stage-contract.md](02-stage-contract.md). This document gives only its own track's phase
  division (see "Implementation Points").

### Division of Labor with RFC-039

RFC-039 is the upper-level master plan: the four-layer model, criterion tiering, cross-document
phase ordering, and "why do this refactor now". This document is the **construction drawing for the
type-representation-convergence thread** in RFC-039: which variants to delete concretely, which
files to change, in what order, and what gates to prevent regression.

All line numbers, variant lists, and construction-point statistics in this document come from static
verification against `src/`; each item gives `file:line` evidence. **This document records only code
facts and the disposition derived from them — it does not record the execution process.**

To facilitate cross-referencing with sibling documents, the "Target Design" and "Detailed Design"
chapters follow the `5.x` / `6.x` sub-section numbering; the other chapters follow this directory's
unified convention.

### Why It Must Precede SSA-ization

`ir::Type` is not a third independent type, but a `pub use` alias of `ast::Type` (`ir.rs:3`).
[04-ssa.md](04-ssa.md) needs to introduce register-type annotations and new instructions to the IR;
if the representation is not converged before then, the SSA type annotations will ride the
`ir::Type` alias directly onto the 26-variant syntax type — that is, locking in a third
representation.

The capability boundary of the current `ir::Type` is pinned by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) embeds a complete expression tree inside the
  type. Having `Expr` appear in the type annotation of an IR instruction is impossible.
- `ast::Type::Literal { name, base_type }` (`ast.rs:476-482`) carries the source name and `Span`; it
  is the syntactic form of a compile-time literal type, not a runtime type.

Doing SSA type annotation on top of the 26 variants is equivalent to permanently cementing these 13
dead variants and two syntax-only variants into the IR contract.

## Current State

All of the following is a statement of code facts, not proposals. Each item gives `file:line`
evidence.

### 2.1 Three Parallel Representations and the Lossy Bridge

The multiplicity of type representation is the common upstream of four independent defects:

| Downstream symptom                                                              | Causal relationship with type representation                                                                                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The same type text has multiple legal AST forms                                 | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but type-distinct**, the parser chooses the latter, and typecheck must supplement a table to translate the former back                                                                              |
| Bytecode-layer signatures inconsistent with IR-layer signatures                 | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only get the alias, so `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`) while `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) has `params` declared as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding a builtin type name requires changes in multiple places with no checking | Because the name→type mapping is scattered across `from_builtin_name` (`src/frontend/core/types/mono.rs:618-643`) and the string match in `bytecode.rs:2371-2376`, each written twice and unverified against the other                                                                                                                                |
| `"Terminates"` hard-coded in the parser                                         | Because the parser cannot get a judgment of "is this a type" or "is this an operator interface", it can only embed the type-layer's knowledge into the syntax layer as a string literal (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                   |

**The first three are representation issues; the fourth is a downstream side-effect of a
representation issue.** Doing only item 1 (deleting dead variants) leaves the synonym table and the
reverse dependency; doing only item 4 (injecting a callback into the parser) builds on a
still-inconsistent representation.

Of the three representations, one is an alias illusion:

| Name        | Definition location                       | Lines       | Role                                                    |
| ----------- | ----------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541` | 26 variants | Syntax-layer type                                       |
| `MonoType`  | `src/frontend/core/types/mono.rs`         | 1077 lines  | Semantic-layer type (after monomorphization)            |
| `ir::Type`  | `src/middle/core/ir.rs:3`                 | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

`ir.rs:3` is a single `pub use`; `ir.rs:6` then `use crate::frontend::core::typecheck::MonoType;`.
The IR module depends on two "type" symbols simultaneously, but one of them is merely an alias.

There is a fourth type carrier on the serialization side:

| Carrier                         | Element type    | Location                                   |
| ------------------------------- | --------------- | ------------------------------------------ |
| `BytecodeModule.type_table`     | `Vec<ir::Type>` | `src/middle/core/bytecode.rs:854`          |
| `FunctionCode.type_table`       | `Vec<MonoType>` | `src/middle/passes/codegen/bytecode.rs:32` |
| Interpreter module `type_table` | `Vec<ir::Type>` | `src/backends/interpreter/image.rs:44`     |

**Lossy bridge.** `From<MonoType> for IrType` (`src/middle/core/bytecode.rs:2353-2390`) downgrades
`MonoType` to `ast::Type`, and its **information loss is silent**:

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

8 `MonoType` variants all collapse to `IrType::Void`, plus a `_ => IrType::Void` fallback
(`bytecode.rs:2386-2387`, whose comment self-admits it is a "transitional branch before deleting
enum variants").

`IrType::Void` here is **a legal type value rather than an error signal**. Therefore the downstream
cannot distinguish "this function really does return `Void`" from "this function's return type was
lost during the bridge" — this is one of the reasons the type-consistency invariant of the layer-1
validator in 07 cannot actually pass green on the current code.

Meanwhile `IrType::String` / `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`) and
`ast::Type::Bytes` (`ast.rs:436`) respectively, which are exactly the dead variants to be deleted in
2.2 — **both ends of the lossy bridge point at dead code**.

### 2.2 Production-Zero-Construction Variants

**Criterion**: The construction-point scan covers all of `src/`, excluding `tests/` and `*/tests/*`.
Of the 26 variants of `ast::Type`, the following 13 have **only match arms in production code, with
no construction points**:

| Variant                              | Definition       | All production hits                                                                                                                                          |
| ------------------------------------ | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Int(usize)`                         | `ast.rs:432`     | `mono.rs:699` (arm)                                                                                                                                          |
| `Float(usize)`                       | `ast.rs:433`     | `mono.rs:700` (arm)                                                                                                                                          |
| `Char`                               | `ast.rs:434`     | `mono.rs:701` (arm)                                                                                                                                          |
| `String`                             | `ast.rs:435`     | `mono.rs:702` (arm)                                                                                                                                          |
| `Bytes`                              | `ast.rs:436`     | `mono.rs:703` (arm)                                                                                                                                          |
| `Bool`                               | `ast.rs:437`     | `mono.rs:707` (arm)                                                                                                                                          |
| `Void`                               | `ast.rs:438`     | `mono.rs:708` (arm), `types.rs:836` (**sole construction point**)                                                                                            |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | `mono.rs:747` (arm)                                                                                                                                          |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | `mono.rs:743` (arm)                                                                                                                                          |
| `Option(Box<Type>)`                  | `ast.rs:455`     | `mono.rs:770` (arm)                                                                                                                                          |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | `mono.rs:771` (arm)                                                                                                                                          |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | `mono.rs:814` (arm)                                                                                                                                          |
| `AssocType { .. }`                   | `ast.rs:463-471` | `mono.rs:781-786` (arm), `formatter/handlers/types.rs:84` (arm), `semantic_tokens.rs:228` (arm) — **no production construction point found in current code** |

The root-cause chain can be fully verified:

1. **The parser represents all primitive types as `Type::Name { name }`** — the fallback arm at
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered into dedicated nodes** — the comment at `types.rs:392-396`
   states explicitly: "RFC-010: Result/Option are not lowered into dedicated AST nodes … they go
   through the same `Type::Generic` path as user-defined generic types. The type representation is
   still `Generic{"Result"/"Option", args}`".
3. **`Type::Void`'s sole construction point is an error fallback** — `_ => (Vec::new(), Type::Void)`
   at `types.rs:836`, located at the end of a match returning `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction points: `types.rs:364`, `types.rs:571`, `types.rs:100` and
`declarations.rs:573` — they are not dead variants.

**These 13 variants have construction points in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
Hence the wording in this document is "**production zero-construction**" rather than "globally
zero-referenced". These tests themselves are the solidification of the behavior under test — they
assert exactly "what these syntactic forms can be lowered into", and these forms are never produced
by the parser. When deleting the variants, these tests must be deleted or rewritten along with them.

#### Criterion boundary: the reverse bridge will reconstruct 7 of them

The "sole construction point / no construction point" in the table above holds only in the **parser
→ AST forward-construction** direction. There exist two production-grade `MonoType → ast::Type`
reconstructions in the reverse direction, which **actually construct the 7 variants in the table
above**:

| Reverse bridge                                                 | Location                                 | Reconstructed 13-variant members                                                                                         |
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
    // #299: String/Bytes and other container types are now Generic, decoded back to AstType via type_name
    MonoType::Generic { name, .. } if name == "String" => AstType::String,
    MonoType::Generic { name, .. } if name == "Bytes" => AstType::Bytes,
    _ => AstType::Name { name: mono.type_name(), span },
}
```

**This criterion boundary directly affects the disposition**:

- The `bytecode.rs` site disappears along with #28 (delete the entire `From<MonoType> for IrType`
  impl) in 6.3, imposing no additional cost.
- `function.rs:507-529` **is not within the coverage of any existing change item**. It is a
  `MonoType → AstType` type-name decoder (comment-tagged `#299`) that, by name, decodes `String` /
  `Bytes` from `Generic` back to dedicated variants. After deleting `ast::Type::String` / `Bytes`,
  these two guard branches must be rewritten; after deleting `Int` / `Float` / `Bool` / `Char` /
  `Void`, the behavior of the `_` arm changes (what previously fell into `_` will silently go to
  `AstType::Name`). This is item 47, added in 6.3.
- **Direct consequence for the gate**: A T1 that only scans forward construction points will judge
  the 7 variants above as "having construction points", thereby excluding them from the deletable
  list. T1 must explicitly include reverse bridges in the statistical scope, otherwise the gate's
  conclusion will contradict this document's disposition table (see 5.7).

### 2.3 The Hand-Written Synonym Table

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:618-643`) uses a string table to map
type names to `MonoType`:

```
"Int" | "int" | "Int64" | "int64" | "i64"  => Some(MonoType::Int(64)),   // :620
"DateTime" | "datetime"                     => Some(MonoType::Int(64)),   // :627
"Int32" | "int32" | "i32"                   => Some(MonoType::Int(32)),   // :628
...
"Void" | "void" | "()"                      => Some(MonoType::Void),      // :640
```

**The sole reason this table exists is to bridge the AST layer's inconsistency**: because
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent but type-distinct in
the AST layer (and the parser only produces the latter), `from_builtin_name` must accept case,
abbreviations, signed names, and the `DateTime` alias simultaneously. The appearance of `"()"` on
the `Void` line is especially telling — it cooperates with the `Type::Void` fallback at
`types.rs:836`.

The same piece of knowledge — "which names are builtin type names" — has **seven** copies in the
repo, with no mutual consistency check:

| #   | Location                                                      | Form                                                                                                                                                                      |
| --- | ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `mono.rs:618-643` `from_builtin_name`                         | String match, name → `MonoType`                                                                                                                                           |
| 2   | `bytecode.rs:2371-2376`                                       | String match, copies three entries: `"String"` / `"Bytes"` / `"Tuple"`                                                                                                    |
| 3   | `ast.rs:838-843` `CONST_PARAM_TYPES`                          | A `&[&str]` constant of 15 const-generic-parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`)                                        |
| 4   | `src/lsp/world.rs:176`                                        | The LSP-side builtin type-name list                                                                                                                                       |
| 5   | `const_data.rs:401-408` `ConstKind::from_ast_type_name`       | String match, name → `ConstKind`. **Coverage inconsistent with item 1**: does not accept lowercase or the `i64` / `i32` aliases (registered in the P0 gate on 2026-10-06) |
| 6   | `eval/const_eval.rs:498-511` the size table of `eval_builtin` | String match, name → byte size. Contains a `"Uint"` arm, while item 1 has no `Uint` mapping at all — **dead arm, living evidence of table drift**                         |
| 7   | `middle/passes/codegen/bytecode.rs:744-756` `to_type_id`      | String match, builtin name → serialized type ID. `"Set" => 25` contradicts the typecheck side's "Set is removed" (`inference/expressions.rs:2917-2918`, #300 decision 4)  |

Item 3, `CONST_PARAM_TYPES`, is used by `extract_generic_param_names` at `ast.rs:912` to determine
const-generic parameters — it enumerates "type names eligible as const parameters", which is a
**partial overlap but non-identical** relation with the full set of `from_builtin_name`, with no
mechanism ensuring the two are kept in sync. Item 4 is the fourth copy for LSP syntax
highlighting/hover, equally unverified (T2's comparison scope includes it).

Items 5–7 were first-run registered by `check-concepts.py` (P0, [09](09-execution-wbs.md) §0.2.1) on
2026-10-06: item 5 is the second type-name synonym table; items 6 / 7 share the same root as items
1–4 (builtin-name → semantic / ID string-keyed tables), with different uses but the same drift risk.
The same gate also hit 11 sites of string-dispatched builtin container names (e.g.
`"List" | "Vec" | "Array"` ×4, `"String" | "Bytes"` ×4) on the typecheck / eval / middle sides —
**not within this section nor within 5.4's parser-dataflow scope**; the list and disposition are
registered in [06-cleanup-inventory.md](06-cleanup-inventory.md) §G3, under the P6 extension
verification.

### 2.4 The Parser's Reverse Dependency on the Type Layer

RFC-039's routing table C lists 3 items (`declarations.rs:27-80` calls `is_type_param_annotation` /
`name_used_as_type`; these two functions belong to the type layer). **The actual measurement differs
from that wording**: these two functions are defined inside the parser itself, not in the type
layer.

- `is_type_param_annotation` is defined at
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the only `fn` definition in the whole
  repo)
- `name_used_as_type` is defined at `declarations.rs:42-80` (the only `fn` definition in the whole
  repo)
- `declare_predicate` / `is_predicate_name` are defined at
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

The truly cross-layer references to `typecheck` in production code total **2 sites**, and both call
the same function `crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Containing function                | Purpose                                                                               |
| --- | --------------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature-parameter filter closure | Determines constraint-position formals (`T: Add`); does not occupy runtime parameters |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`      | Determines constraint-position formals, yields `GenericParamName` with `constraints`  |

Item 2 is more inner — **`parser/ast.rs` itself** (the entire block at `ast.rs:847-948` and
`StmtKind`/`Expr` are in the same file) directly references a typecheck symbol. Routing table C only
recorded one site in `declarations.rs`.

It forms the same judgment together with the hard-coding at `declarations.rs:498-499`:

```rust
// declarations.rs:498-499
// Extended as built-in predicates are added (future open-set scheme:
// pass the predicate name set down to the parser, or change to a
// syntax-layer-determinable form).
// The parser does not hold the type environment, so look in two places:
// hard-coded built-in predicates, plus the accumulated declarations
// already parsed in this pass (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment self-admits this is a temporary form of a "future open-set scheme". `n == "Terminates"`
is **embedding the type layer's knowledge as a string literal into the syntax layer**.

**Why this is downstream of the type-representation problem**: The reason the parser needs to ask
"is this an operator-interface name" is that it must distinguish whether `Type::Name` refers to a
type or to an interface constraint. **`Type::Name { name: String }` carries at least 4 semantics in
an undifferentiated string** (concrete type / type variable / constraint name / predicate name); the
syntax layer cannot decide on its own, and the decision is forced up to the type layer.

#### Duplicated implementation: same name and meaning, behavior already diverged

`name_used_as_type` has two implementations. The comment at `ast.rs:843-846` self-admits this:

```rust
/// Whether a parameter name is used as a type reference within a given type
/// (`(N: Int) -> (n: N)`'s `N`).
///
/// Synonymous with `declarations.rs`'s `name_used_as_type`; this is independently
/// implemented to avoid cross-module dependencies inside the parser.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations have **already diverged**:

| Dimension               | `declarations.rs:42-80` `name_used_as_type`                         | `ast.rs:847-885` `name_used_as_type_in`              |
| ----------------------- | ------------------------------------------------------------------- | ---------------------------------------------------- |
| Signature               | Takes an `is_predicate: &dyn Fn(&str) -> bool` callback             | No callback                                          |
| `Type::Generic` branch  | Short-circuits to `false` after `is_predicate(app_name)` (`:54-56`) | No such short-circuit, recurses directly into `args` |
| `Type::Struct { body }` | **None** (falls through to `_ => false`)                            | **Has** (`:871-879`, traverses `Field` / `Expr`)     |
| `Type::NamedStruct`     | **None**                                                            | **Has** (`:880-882`, traverses `fields`)             |

The comment at `ast.rs:868-870` records the origin of the latter two branches: "Previously missing
these two branches caused a const-generic parameter, when referenced inside a definition body, to
fail the 'used as a type' check, making the annotation check misreport legal const parameters as
unknown names."

That is: **the function copied to avoid cross-module dependencies has had its fix not flow back to
the original**. The `ast.rs` version has field recursion inside definition bodies, the
`declarations.rs` version does not — the same source text yields different conclusions at the two
decision points. This must be merged together with the `NameKind` introduced in 5.3.

### 2.5 Parallel Operator Enums

There are two operator enums, mutually non-referencing, and **members have already diverged**:

| Concept         | Syntax layer                    | Constant-evaluation layer                                             | Difference                                                                                                                                |
| --------------- | ------------------------------- | --------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Binary operator | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | The syntax layer uniquely has `Neq` (`:197`), `Range` (`:205`), `Assign` (`:206`); the constant layer uses `Ne` (`:243`) instead of `Neq` |
| Unary operator  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | The syntax layer uniquely has `Deref` (`:222`); the constant layer uniquely has `BitNot` (`:329`)                                         |

Both have also each implemented a full `Display` (`const_data.rs:291-317`) and classification
predicates (`is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise` at
`const_data.rs:260-289`).

**Why this is part of the type-representation problem**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) are **not types** — when they appear in type position they fall
into `Type::ConstExpr` (`ast.rs:506`), i.e. "compile-time-only variant". This is directly related to
the IR-layer type discipline in 6.1: after convergence, `Type::ConstExpr` disappears after
`Type → MonoType`, never visible to the IR, while `Assign` / `Deref` continue to exist via paths
like `ir_gen.rs`'s `BinOp::Assign => Ok(MonoType::Void)` (`inference/expressions.rs:875`). The
divergence of the two enums means "which operators can appear in type position" cannot be determined
from either side alone.

This phase does not merge these two enums (their ownership is in
[05-frontend-paradigm.md](05-frontend-paradigm.md)'s "operator-change surface"), but T1's gate scan
target should include `ast::BinOp` / `ast::UnOp`, so that "newly-added zero-construction operators"
are equally visible.

### 2.6 `Expr::FnDef`: The Production-Zero-Construction Expression Variant

`Expr::FnDef` (`ast.rs:37-43`) being **production zero-construction** is true — the whole repo has
only two construction sites: `src/frontend/core/parser/tests/ast.rs:837` and
`src/frontend/core/typecheck/tests/checker.rs:111`, both tests.

But **consumption sites are far more than 4**. Measured production consumption sites: 12.

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

There are 2 additional exhaustive match arms forced to exist for it: `ast.rs:1026` (`Expr::span()`)
and `pratt/mod.rs:47` (`expr_end_line`).

**The path actually taken by function definitions is**: `nud.rs:425-429` constructs `Expr::Lambda`
and `declarations.rs:462-479` constructs `StmtKind::Assign` (putting the `Lambda` into `value`). In
other words, the `FnDef` branch is **a parallel path that is never executed but is maintained by 14
locations**.

### 2.7 Syntax Nodes Holding Semantics on Behalf of the Type Layer

**First, `StmtKind::Assign` carries typechecker-specific fields.** The `Assign` variant at
`ast.rs:241-249` contains `signature_params: Vec<Param>` (`ast.rs:244-245`), and the comment says:

> `/// The first group of signature parameters as-is (including parameter names), for the typechecker's classify_generic_params to use`

The statement AST reserves a dedicated slot for the type checker. Construction point:
`declarations.rs:472`.

**Second, the expression position is treated as a parameter list.** `(a: Int, b: Int)` at
`src/frontend/core/parser/pratt/nud.rs:505-514` is parsed as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`** — **a Lambda with an
empty body**. Then the arm `Expr::Lambda { params, .. } => Some(params.clone())` at
`src/frontend/core/parser/pratt/led.rs:466` "fishes" it back out and uses it as a parameter list.

That is: a normal variant of the AST is used as **a temporary parameter-list carrier**, and then its
semantics is restored by a match arm in another module.

**Third, `Type::NamedParen` carries the binder name of the return position.** The comment at
`ast.rs:519-540` states:

> The return-position refinement of RFC-027 §3 relies on it to declare the **return formal-parameter
> name** (called binder here) … **dropping it, the type checker can only guess** that "free
> variables in constraints not in scope are return-value formals", and then `(r: P(m))` would also
> silently substitute the undeclared `m` as a binder (verified defect).

The syntax node is holding binder identity on behalf of the type layer.

## Target Design

### 5.1 Form of the Target Representation

**Claim: Single representation = `Type` (originally `ast::Type`) as the sole type structure;
`MonoType` demoted to a working form inside the type checker; the `ir::Type` alias deleted.**

The three representations converge into **two roles**, but only one is "type":

```
Sole type structure  Type  ─────────────────────────────┐
   (26 → 13 variants, lives in types/ not parser/ast.rs)  │
                                                        │  From<Type> for MonoType  (total, no semantic loss)
                                                        │  ← sole conversion point types/lower.rs
Type-check working form  MonoType  ──────────────────────┘
   (carries TypeVar / substitution state, only flows within typecheck)
   ×
ir::Type alias deleted, BytecodeFunction.params / type_table changed to use MonoType
```

**Why it is not "merge into `MonoType`"** (this is the most easily proposed scheme, and is rejected
here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.);
   `MonoType` does not. C3 in 07 says "diagnostic codes and messages are the same" — the location
   information of the diagnostic must be traceable back to the source. Adding `Span` to `MonoType`
   would cause monomorphized types to carry a bunch of meaningless sentinel spans.
2. `MonoType` contains `TypeVar` and substitution state (`types/substitute.rs:112`, `pub fn unify`
   at `types/solver.rs:411`). These are **intermediate states of the type checker's solving
   process**, not types themselves. Putting them into the sole type structure would mean the IR
   construction layer needs to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027), and are lowered away in `MonoType`. Unifying into `MonoType`
   means adding these syntactic forms back to it.
4. **The most critical counter-argument**: Promoting `MonoType` to the sole representation means
   making the L2 syntax layer depend on the L3 semantic layer's types. RFC-039 explicitly forbids L2
   from depending on L3. One of this document's goals is to eliminate reverse dependencies; choosing
   a scheme that would cause a reverse dependency is self-contradictory.

**Why `ir::Type` must be deleted directly rather than "preserved"**:

The alias at `ir.rs:3` causes `BytecodeFunction.params` (`bytecode.rs:812`) and `return_type`
(`bytecode.rs:814`) to use `ir::Type`, while the serialization side at `codegen/bytecode.rs:277-278`
uses `MonoType`. The two sets of field types are connected by the lossy `From` bridge at
`bytecode.rs:2339`'s `file.type_table.into_iter().map(|t| t.into())` and `bytecode.rs:2353-2390`.

**The sole reason the `From` bridge exists is "the two sides' field types differ"**. Unifying
`bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, `image.rs:44` to `MonoType` removes the
reason for `From<MonoType> for IrType` to exist, and the whole impl can be deleted (along with
eliminating the reverse reconstruction recorded in 2.2).

**What is explicitly NOT done**: `Type` is not moved from `parser/ast.rs` to `sema/types/`. This
phase only adds `lower.rs` under `types/` as the sole conversion point; `Type`'s definition location
stays. The directory rename is executed at the close of P6 as an independent move batch per D1 — at
that point `Type`'s home is the top-level `ast/type_.rs` (top-level AST domain), and `sema/types/`
is `MonoType`'s working-form home.

### 5.2 Per-Variant Disposition Table for the 26 Variants

> Disposition categories: **Delete** (production zero-construction), **Keep** (production has
> construction points), **Migrate** (semantics relocated to another node).

| #   | Variant                                                            | Definition line  | Disposition                              | Reason                                                                                                                                                                                                                                                                              |
| --- | ------------------------------------------------------------------ | ---------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Keep (strengthened)**                  | The parser's **only** primitive-type exit (`types.rs:162-165`). After deleting 12 primitive-type variants, all primitive types go through it. A "semantic kind of the name" annotation needs to be added (see 5.3)                                                                  |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                               | Production zero-construction (`mono.rs:699` is the only arm). The bit width is carried by the `Name` path in `from_builtin_name`                                                                                                                                                    |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                               | Production zero-construction (`mono.rs:700`)                                                                                                                                                                                                                                        |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                               | Production zero-construction (`mono.rs:701`)                                                                                                                                                                                                                                        |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                               | Production zero-construction (`mono.rs:702`). **Note**: `bytecode.rs:2372`'s `IrType::String` consumes it; after deletion that arm must be rewritten                                                                                                                                |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                               | Production zero-construction (`mono.rs:703-706`). Same as above with `bytecode.rs:2373` consumption                                                                                                                                                                                 |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                               | Production zero-construction (`mono.rs:707`)                                                                                                                                                                                                                                        |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                               | The production's sole construction is the error fallback at `types.rs:836`. That fallback is eliminated together with the error-path rework in 5.6                                                                                                                                  |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Keep**                                 | Has production consumption (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                                 |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Keep**                                 | Has production construction point `types.rs:364`                                                                                                                                                                                                                                    |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                               | Production zero-construction (`mono.rs:743-746`)                                                                                                                                                                                                                                    |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                               | Production zero-construction (`mono.rs:747`). Enums go through `Struct` + `TypeBodyItem`, unrelated to `MonoType::Enum(_) => IrType::Void` at `mono.rs:2378`                                                                                                                        |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Keep**                                 | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                      |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:451-454` | **Keep**                                 | Consumed by `bytecode.rs:2366`; **field reshape `params: Vec<Type>` → `Vec<Param>` (name and type co-resident) along with 6.8.1 (D54)** — the name from `Assign.signature_params` is taken up here (see 6.5.2)                                                                      |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                               | Production zero-construction (`mono.rs:770`). `types.rs:392-396` already says they go through `Generic`                                                                                                                                                                             |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                               | Production zero-construction (`mono.rs:771`). Same as above                                                                                                                                                                                                                         |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Keep**                                 | The actual representation of `Result` / `Option` / `String` / `Bytes` all go through it (`types.rs:397-399`)                                                                                                                                                                        |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                               | Production zero-construction (the only construction in the whole repo is in test `types/tests/mono.rs:178`; the syntax layer has no `::` path to produce it). If associated-type syntax is enabled in the future, it will be redefined by a new proposal at that time (Decision D8) |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                               | Production zero-construction (`mono.rs:814`)                                                                                                                                                                                                                                        |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Keep**                                 | Has production construction point `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                            |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Keep**                                 | Bare-pointer types inside unsafe blocks                                                                                                                                                                                                                                             |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Keep**                                 | Borrow notation                                                                                                                                                                                                                                                                     |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Keep**                                 | Has production construction points `types.rs:100`, `declarations.rs:573`; RFC-010                                                                                                                                                                                                   |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Keep (and mark as compile-time-only)** | Has production construction point `types.rs:160`. The only variant in `Type` that embeds `Expr`; forbidden in the IR layer (see 6.1)                                                                                                                                                |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Keep**                                 | RFC-004 currying's layer terminator; `split_curry` depends on its existence                                                                                                                                                                                                         |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                              | Semantics (binder name) relocated to the type checker; AST keeps only syntax. See 5.6 for details                                                                                                                                                                                   |

**Net effect: 26 → 14 variants** (delete 12, migrate 1, keep 13).

> **The reverse bridge must be disposed of before deleting variants**: `function.rs:507-529`
> recorded in 2.2 will reconstruct 7 of items #2–#8 in this table. It must be processed in the same
> batch as #1–#8 in the deletion order, otherwise the `_` arm of that function will silently change
> behavior (see #47 in 6.3).

> **`AssocType`'s handling**: Per D8, delete it directly (item 18 in the 5.2 table). T1's first
> report on unchanged code (5.7) **must independently reproduce this conclusion** — if the report
> shows `AssocType` has production construction points, it means this section's verification is
> wrong, and **the correct action is to go back to the RFC decision table to amend D8 and explain
> why**, not to keep the variant on the whitelist.

### 5.3 `NameKind` and Synonym-Table Elimination

**Problem**: `Type::Name { name: String }` carries at least 4 semantics in an undifferentiated
string — concrete type / type variable / constraint name (operator interface) / predicate name.
`declarations.rs:498-499` needs to determine "is it a predicate", `declarations.rs:508-511` needs to
determine "is it an operator interface", `ast.rs:930` needs to determine "is it an operator
interface", `declarations.rs:42-80` and `ast.rs:847-885` two copies of `name_used_as_type*` need to
determine "is it a type reference" — all by string comparison.

**Proposal: Add a `kind` field to `Type::Name`, determined once by the parser.**

```rust
// Target form (illustrative; line numbers are after the change)
Name {
    name: String,
    kind: NameKind,   // newly added
    span: Span,
}

enum NameKind {
    Builtin,      // Builtin type name: Int / Float / Bool / ... → handed to from_builtin_name
    UserType,     // User-declared type name (including std.result / std.option of Generic)
    TypeVar,      // Type variable: T / K / V
    Constraint,   // Operator interface / constraint name: Add / Ord / ...
    Predicate,    // Compile-time predicate: Terminates / Sorted / ...
}
```

**Key design point: determining `NameKind` requires exactly the information the parser cannot
currently obtain** (type environment, declared operator-interface table, declared predicate table).
Therefore this field **cannot be determined independently within the parser** — it must be
determined by the injected environment. This connects directly to 5.4.

**Synonym-table disposition**:

`from_builtin_name` (`mono.rs:618-643`) is **kept, but semantically downgraded**. Before the change
it is a disambiguator that "bridges two equivalent representations at the AST layer"; after the
change, `Type::Int(64)` is deleted and `"Int"` has only one representation path, so it degrades to
the **sole name→type resolution table**.

Concrete reasons to keep it (cannot be deleted):

- The comment for `"DateTime" | "datetime" => Some(MonoType::Int(64)` (`mono.rs:627`) records a real
  fix (a side issue of #338: `now()`'s return value cannot be passed into `format_time`). This is
  **alias** semantics, not the same as the `Int`/`int` case aliases.
- It is the **sole intersection** between the standard library's type names and the language's
  builtin names.

But **redundancy inside the table can be cut**. After the change the parser will only produce
canonical case (`Int` / `Int32` / `Float` / `Bool` / ...), so:

| Category                                           | Current (`mono.rs:620-642`) | Target                                                                                                                                                                                |
| -------------------------------------------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Case aliases (`"int"` / `"i64"` / `"int64"`)       | Kept                        | **Delete** — the parser will not produce them after normalization                                                                                                                     |
| Abbreviation aliases (`"i64"` / `"i32"` / `"f64"`) | Kept                        | **Move to lexer layer** (registered as equivalent spellings of `Int` in the keyword table at `src/frontend/core/lexer/state.rs`); `from_builtin_name` only recognizes canonical names |
| `DateTime` alias                                   | Kept (`mono.rs:627`)        | **Keep** — semantic alias, with comment                                                                                                                                               |
| `"()"` (`mono.rs:640`)                             | Kept                        | **Delete** — after the `Type::Void` fallback (`types.rs:836`) is deleted, no place produces it                                                                                        |

**Net effect**: `from_builtin_name` converges from 12 match arms to about 6 canonical names,
semantics change from "disambiguation table" to "resolution table", and **becomes the single source
of truth for type names** (verified by the gate in 5.7).

**Merging the two copies of `name_used_as_type*`**: the two duplicate implementations recorded in
2.4 must be merged into one, otherwise the kind information provided by `NameKind` will be bypassed
by the two independent string-comparison logics. The merged implementation goes in `parser/ast.rs`
(the data source is near `Type`); `declarations.rs` is changed to call it; the `is_predicate`
short-circuit is changed to read `NameKind::Predicate`; the field recursion in `Struct` /
`NamedStruct` takes the version from `ast.rs:871-882` (newer, with more complete fixes).

**Ownership of `CONST_PARAM_TYPES`**: The const-generic-parameter-name table at `ast.rs:838-841`
partially overlaps with the full set of `from_builtin_name` (see 2.3). With `NameKind::Builtin`
introduced, the judgment at `ast.rs:912` can be changed to "`kind == Builtin` and the name is within
the const-parameter-eligible set", explicitly connecting the two pieces of knowledge; the subset
definition of const parameters is verified by the T2 gate (see 5.7).

> **Design judgment**: If non-canonical spellings like `i64` / `int` exist in the `.yx` corpus,
> normalizing them to `Int` will change the **diagnostic position** of those files (not the
> diagnostic code). By the C3 criterion this is allowed (messages and order can be normalized), but
> the baseline must be recorded in phase 1.

### 5.4 Predicate / Type-Name Data Flow

**Current data flow** (parser-internal closed loop + 2 boundary sites):

```
ast.rs (extract_generic_param_names, :930)  ─┐
declarations.rs:509                          ─┴─→ typecheck::operator_interfaces::spec()   ✗
declarations.rs
  ├─ declare_predicate()      ← parser_state.rs:46 (parser self-held, only this pass's already-parsed declarations)
  ├─ n == "Terminates"        ← hard-coded literal (declarations.rs:499)
  └─ (the above boundary calls)
```

**Target data flow** (injection, no boundary crossing):

```
L1 orchestration layer (constructed once at entry)
   │  ① Compile the builtin operator-interface table
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

| File                                                                                     | Change                                                                                                                                                                                                                                                                            |
| ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/probe.rs` (new)                                                | Define `trait TypeEnvProbe`, providing four pure-query methods: `is_predicate` / `is_constraint` / `is_type_var` / `is_builtin_type`. **The trait is defined in L2, the implementation is provided by L3** — this is a legal dependency inversion                                 |
| `src/frontend/core/parser/parser_state.rs:46-58`                                         | Keep `declare_predicate` / `is_predicate_name` (for this-pass accumulation), add a `probe: Box<dyn TypeEnvProbe>` field and a `probe()` accessor                                                                                                                                  |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                            | Delete `n == "Terminates"`; change to `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                                                             |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                            | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; change to `state.probe().is_constraint(n)`                                                                                                                                                      |
| `src/frontend/core/parser/ast.rs:930`                                                    | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` is where `Type` / `Expr` / `StmtKind` are defined, and should not hold environment dependencies — this function needs to change to take a `&dyn TypeEnvProbe` parameter (see below) |
| All 5 parser construction points (`Parser::new` / `ParserState::new` inside `parser.rs`) | Accept a `probe: Box<dyn TypeEnvProbe>` parameter. **Default implementation `NullProbe` (all return false)** ensures parser unit tests are unaffected                                                                                                                             |

**The signature issue of `extract_generic_param_names`**:
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` at `ast.rs:891` is a
free function that takes no context. After deleting the boundary call at `ast.rs:930` it needs the
`is_constraint` judgment, so the signature must add `probe: &dyn TypeEnvProbe`. Callers (including
`parser/tests/ast.rs`) change synchronously to pass `&NullProbe`.

**`NameKind` determination timing**: When `types.rs:162-165` constructs `Type::Name`, use
`state.probe()` to determine `kind`, determined once, and no subsequent match arm does any string
comparison.

**This eliminates all 3 items in RFC-039's routing table C** — 2 of which, per the 2.4 criterion,
only involve functions inside the parser anyway; the truly cross-layer references are the 2 sites
`declarations.rs:509` and `ast.rs:930`, both covered by this section.

### 5.5 Merging `Expr::FnDef`

**Claim: Delete `Expr::FnDef` (`ast.rs:37-43`); function definitions uniformly go through
`Expr::Lambda` + `StmtKind::Assign`.**

**This is the path that is already running**: `nud.rs:425-429` + `declarations.rs:462-479` (see
2.6). The `FnDef` branch is a never-executed parallel implementation.

**Change list** (12 consumption sites + 2 exhaustive arms, listed one by one):

| Location                             | Existing arm                                         | Target                                                                                                                                         |
| ------------------------------------ | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `spawn/placement.rs:122`             | `Expr::FnDef { body, .. } => self.check_block(body)` | Merge into the existing `Expr::Lambda` arm (`body` is also `Box<Block>`, **delete this arm directly**)                                         |
| `spawn/analysis.rs:912`              | `Expr::FnDef { body, .. } => { ... }`                | Same as above, merge into the Lambda arm                                                                                                       |
| `formatter/handlers/expr.rs:43`      | `Expr::FnDef { ... }`                                | Change to format the Lambda inside `Assign`; delete this arm                                                                                   |
| `orchestrator.rs:1492`               | `if let Expr::FnDef { name, .. }`                    | Change to take the name from `StmtKind::Assign { target, .. }`                                                                                 |
| `ir_gen.rs:4325`                     | `ast::Expr::FnDef { span, .. } => *span`             | Merge into the Lambda arm (`Lambda` also carries `span`)                                                                                       |
| `ir_gen.rs:5097`                     | `\| ast::Expr::FnDef { span, .. }`                   | Delete this or-pattern                                                                                                                         |
| `checker.rs:1642`                    | `if let ...::Expr::FnDef {`                          | Change to `Expr::Lambda`                                                                                                                       |
| `checker/semantic_tokens.rs:1416`    | `Expr::FnDef {`                                      | Delete this arm                                                                                                                                |
| `inference/expressions.rs:3374`      | `...::Expr::FnDef {`                                 | Change to `Expr::Lambda`                                                                                                                       |
| `inference/existential.rs:55`        | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                         |
| `passes/dead_code.rs:305`            | `Expr::FnDef {`                                      | Change to `Expr::Lambda`                                                                                                                       |
| `layers/ownership.rs:1207`           | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                         |
| `layers/termination.rs:752`          | `Expr::FnDef {`                                      | Change to `Expr::Lambda`; the comment at `termination.rs:2303` "fn(): Never — FnDef.return_type is the bare return type" updated synchronously |
| `ast.rs:1026`                        | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                         |
| `pratt/mod.rs:47`                    | `Expr::FnDef { span, .. } => *span`                  | Delete this arm                                                                                                                                |
| `typecheck/tests/checker.rs:109-111` | Test constructs `Expr::FnDef`                        | Rewrite as `Expr::Lambda` + `Assign`                                                                                                           |
| `parser/tests/ast.rs:837-847`        | Test constructs `Expr::FnDef`                        | Delete this test                                                                                                                               |

**No behavior change**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, `span`,
structurally identical to `FnDef`'s first three; `FnDef`'s unique `name` field has nowhere to be
used at the `Expr` level (the name is on `StmtKind::Assign.target`).

**Risk and mitigation**: `checker.rs:1642`, `inference/expressions.rs:3374` and similar
**non-exhaustive** `if let` matches become never-matching after the change — no error, no warning.
Mitigation: the type-table gate (5.7) adds a "zero-construction variant" check; after `Expr::FnDef`
is deleted, if anyone re-writes `if let Expr::FnDef` it will fail to compile directly (the variant
does not exist), **this is a compile-time guarantee, no gate needed**.

### 5.6 Disposition of `signature_params` and `NamedParen`

**`StmtKind::Assign.signature_params` (`ast.rs:244-245`) → Delete.**

Reason: Its reason for existing (the comment at `ast.rs:244`) is "for the typechecker
`classify_generic_params` to use" — **a dedicated slot reserved by the statement AST for the type
checker**. But `extracted_params` at `declarations.rs:500-511` already computes `value_params`
within the same function (after the type-position/constraint-position filter); passing it down with
the statement is duplicate transport.

Change: `ast.rs:244-245` delete the field → `declarations.rs:472` delete the actual argument → the
data source for `classify_generic_params` is changed to recompute in place (it can get the
`Lambda.params` from inside `value: Option<Box<Expr>>`). **Net deletion: one field declaration, one
construction site, one transport path.**

> **Risk**: `signature_params` is the **unfiltered** raw list, while `Lambda.params` is the
> **filtered** list (the filter at `declarations.rs:500-511` removes type positions and constraint
> positions). If `classify_generic_params` depends on the unfiltered version, the change will alter
> behavior. **The actual version used by `classify_generic_params` must be verified in phase 1**;
> equivalence cannot be assumed.

**`Type::NamedParen` (`ast.rs:533-540`) → Keep the node, remove the binder semantics.**

The comment at `ast.rs:519-532` records its **sufficient reason** for existing (see 2.7): in
`(r: P(m))`, if `r` is dropped, the type checker can only guess that "free variables in constraints
not in scope are return-value formals", and will **also silently substitute the undeclared `m` as a
binder** (the comment tags it as a "verified defect").

Therefore it **cannot simply be deleted** — that would reintroduce a correctness defect that has
already been fixed. The correct disposition is:

1. `NamedParen { param, param_span, inner }` is kept as a **pure syntax node**; `param` /
   `param_span` only carry source-code facts, and do not bear type-layer binder semantics.
2. The interpretation authority of the binder is handed to `sema/`: the type checker, when consuming
   `NamedParen`, **explicitly** extracts `param` as a binder. This is "reading the fields of a
   syntax node", of the same nature as `ConstExpr` embedding `Expr` — the syntax node provides the
   facts, the semantic layer does the interpretation.
3. The comment changes from "the type checker can only guess" to "this node provides the binder
   name, and the semantic interpretation is in `sema/`".

**Why this is "migrate" rather than "delete"**: the syntax tree must be able to represent the source
code without loss. `r` truly exists in the source; the AST cannot pretend it does not. The problem
is not that the node exists, but that **the responsibility of semantic interpretation has not been
clearly defined**.

### 5.7 Type-Table Generation-Time Gates

**Reference object verified in this repo**: `build.rs:19-37` calls `tools/code-tables`'s
`parse_registry` / `validate`, performing uniqueness, segment-position, zh-completeness checks on
145 error codes, and `is_ok()` being false directly `panic!`s to refuse compilation;
`build.rs:39-54` further compares each entry against RFC-013's markdown code table line by line,
with inconsistency also `panic!`ing. **The RFC-013 document and code are therefore always
consistent** — `tools/code-tables/Cargo.toml:15-16` has only a `serde_json` dependency, and the
check logic is text-level parsing, not relying on `syn`.

The type table currently has **no** similar gate. The form of the mechanism to be added in this
document can be directly reused.

**New crate**: `tools/type-tables` (`Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). Four checks:

| #                                                             | Check                                                                                                                                                                                                                                                                                                 | Input                    | Failure condition                                                                                                | Treatment                                                            |
| ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | ---------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **T1 Zero-Construction Variants**                             | Scan the variant lists of `pub enum Type` (and `Expr` / `StmtKind` / `BinOp` / `UnOp`) in `src/frontend/core/parser/ast.rs`; count the **construction points** of each variant under `src/` (excluding `tests/` and `*/tests/*`), **and separately mark reconstruction points in the reverse bridge** | Variant list + hit table | A variant has **zero construction points** and is not in the explicit whitelist                                  | Delete the variant, or move it to the whitelist with a stated reason |
| **T2 Synonym Table Consistent with the Whole-Repo Name List** | The `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` at `ast.rs:838-843` + the LSP builtin type-name list at `src/lsp/world.rs:176`                                                                                                                                           | Three tables             | A type name exists in one table but is not recognized in another (compared by each's own semantic mapping rules) | `type-tables --fix` bidirectional filling                            |
| **T3 Builtin Type Names Consistent with RFC-011 Document**    | The canonical-name set of `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                                                  | Two tables               | Code-table ranges / names inconsistent with the registry                                                         | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --fix`    |
| **T4 `Type → MonoType` Exhaustiveness**                       | The `Type` variant list vs. the match arms in `types/lower.rs`                                                                                                                                                                                                                                        | Two tables               | A variant without a corresponding arm, or an arm without a corresponding variant                                 | Add arms / delete arms                                               |

**Implementation note on T4**: Rust's exhaustive match already guarantees the "variant → arm"
direction at compile time. T4's value lies in the **reverse** direction — detecting **stale arms
pointing to deleted variants** in `lower.rs` (such code already fails to compile when the variant is
deleted, so T4 is effectively redundant). **T4 therefore degrades to a CI assertion (`cargo build`
succeeding means passing), not entering the `panic!` path of `build.rs`.** T1–T3 are the three items
that truly need text parsing.

**Two implementation difficulties of T1 and their mitigations**:

1. **Distinguishing construction points from match arms** requires syntactic analysis. The root
   `Cargo.toml` has **no `syn` dependency**, and `tools/code-tables` also has only `serde_json`.
   Mitigation:
   - **Preferred**: introduce `syn` to `tools/type-tables` (only that crate, the root crate is
     untouched). `syn` is the standard approach, and the gate crate is independent of the compiler.
   - **Fallback**: reuse the existing `tools/extract_arm.py` (an in-repo existing Python tool)'s
     line-level heuristics — construction points lie to the right of `=` or in function-argument
     position, match arms lie to the left of `=>`. Use the count of `=>` occurrences as a proxy.
   - **Conservative starting point**: T1's first version **only reports and does not fail**
     (`cargo:warning`), takes one full baseline to confirm the heuristics, then changes to `panic!`.
     This is consistent with "the CI gate will be red at first and needs a baseline first".
2. **The reverse bridge's construction points must be categorized separately**. 2.2 records two
   `MonoType → ast::Type` reconstructions at `bytecode.rs:2353-2390` and `function.rs:507-529`. If
   T1 conflates their reconstruction points with the parser's forward construction, the 7 variants
   `Int` / `Float` / `Bool` / `Char` / `Void` / `String` / `Bytes` will be judged "has construction
   points" and exempted from deletion, directly contradicting the disposition table in 5.2. T1's
   report must list the two types of construction points separately, and require a separate
   explanation in the whitelist mechanism for reverse-bridge reconstruction.

**Gate coverage**: A new `type_tables::validate(root, &entries)` call is added to `build.rs`,
immediately after the existing block at `build.rs:19-37`, following the
`panic!("type-table validation failed ({} errors ...)", ...)` form.

## Detailed Design

### 6.1 Type-System Impact

| Surface                                                      | Judgment                                                                                                                                                                                                                                                                                                                                           |
| ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact**. Inference consumes `MonoType` throughout, never touches `ast::Type` variants                                                                                                                                                                                                                                                        |
| Solver (`types/solver.rs`)                                   | **No impact**. `pub fn unify` (`solver.rs:411`), `substitute` at `types/substitute.rs:112`, `substitute` at `types/mono.rs:649` all operate on `MonoType`                                                                                                                                                                                          |
| Constant evaluation (`types/eval/`)                          | **No impact**. `eval/const_eval.rs` (1128 lines), `eval/dependent_types.rs` (`check_structural_termination` at `478`), `unify` at `eval/reducer.rs:572` all operate on `MonoType`                                                                                                                                                                  |
| Monomorphization (`middle/passes/mono/`)                     | **Needs change**. 8 places in `mono/function.rs:358`, `362`, `451`, `456`, `554`, `558`, `618`, `623` match `AstType::NamedStruct` / `AstType::AssocType` — if the disposition of `AssocType` is to delete, these 8 places need to be updated synchronously. **Also needs to change `mono_to_ast_type` at `function.rs:507-529`** (see #47 in 6.3) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Needs change**. `ir_gen.rs:1266`, `1399`'s `ast::Type::NamedStruct` match; `ir_gen.rs:1380`'s `let params: Vec<MonoType> = signature_params` (this is another consumption site of `signature_params`, which 5.6 also needs to verify)                                                                                                            |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Needs change**. `codegen/bytecode.rs:492-495`'s `type_id_to_monotype`, the encoding loop at `351-352`, the assembly at `632`                                                                                                                                                                                                                     |
| Interpreter (`backends/interpreter/`)                        | **Needs change**. `image.rs:44`'s `type_table: Vec<ir::Type>` to `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, the element-type change will affect `{:?}` output format                                                                                                                                        |
| RFC-027 compile-time types                                   | **Needs change**. `Type::ConstExpr` is kept but marked **compile-time-only**; it disappears after `Type → MonoType` (`types/lower.rs`) and is never visible at the IR layer                                                                                                                                                                        |

**IR-layer type discipline (for 04-ssa to inherit)**: after convergence, IR type annotations can
only take the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only** variants, and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. This discipline should be
enforced by the "type-consistency" invariant of 07's layer-1 validator. `ast::BinOp::Assign` /
`ast::UnOp::Deref` (2.5) are expression-side forms of the same problem — they can appear inside
`ConstExpr`, so they likewise must not sink into IR type annotations.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** Basis:

1. The 11 deleted variants have no **forward** construction points in production code → no source
   code can produce them (see #47/#28 in 6.3 for the reverse-bridge disposition).
2. `ir::Type` is an alias of `ast::Type` → changing it back to `MonoType` does not change the type
   set, only the **carrier**.
3. The **loss** of `From<MonoType> for IrType` (8 variants → `Void`) no longer occurs after the
   change — this **may** change some programs' behavior (types previously lost are now preserved).
   **This is the only direction of behavior change, and is a fix rather than a regression**, but the
   07 layer-3 corpus diff must confirm no regression.

**Semantic change points that need focused attention**:

| Location                | Current                                                                                       | After convergence                      |
| ----------------------- | --------------------------------------------------------------------------------------------- | -------------------------------------- |
| `bytecode.rs:2371-2376` | Non-`String`/`Bytes`/`Tuple` `Generic` → `IrType::Void`                                       | Preserve the real `MonoType::Generic`  |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Preserve the real `MonoType`           |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` fallback                                                                  | **Delete** (exhaustive match suffices) |

> **Design judgment**: these 8 variants currently all collapse to `Void`, meaning **code paths that
> depend on signature bytecode (the interpreter's arity/type check, serialization format) currently
> see `Void` uniformly**. After convergence these locations will see real types. **This is the only
> possible source of behavior change in this document, and the direction is "restoring correct
> information"**. If corpus diffs differ, it should be treated as **exposing an existing defect**
> rather than a regression introduced by this document — the judgment method: construct a single
> `.yx` program containing a `Ref` parameter, and compare `dump_bytecode` before and after the
> change.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                   | Change                                                                                                                                                                                                                                   | Category                 |
| --- | ---------------------------------------------------------------------- | -------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472` | Delete the `String` / `Bytes` / `Bool` / `Void` / `Union` / `Enum` / `Option` / `Result` / `Sum` / `Int` / `Float` variants                                                                                                              | Pure deletion            |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                              | Add a `kind: NameKind` field to `Name`                                                                                                                                                                                                   | Data flow                |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43`                                | Delete `Expr::FnDef`                                                                                                                                                                                                                     | Pure deletion            |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                              | Delete `Assign.signature_params`                                                                                                                                                                                                         | Pure deletion            |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026`                                 | Delete the `Expr::FnDef` or-pattern                                                                                                                                                                                                      | With #3                  |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                              | Change the `NamedParen` comment to "syntax provides the binder name, semantic interpretation is in `sema/`"                                                                                                                              | Doc                      |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                        | Add `pub enum NameKind`                                                                                                                                                                                                                  | New                      |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                    | `trait TypeEnvProbe` + `NullProbe`                                                                                                                                                                                                       | New                      |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                | Add a `probe` field and accessor                                                                                                                                                                                                         | Data flow                |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                              | Use `state.probe()` to determine `NameKind`                                                                                                                                                                                              | Data flow                |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                              | Comment update (`Result`/`Option` go through `Generic`, already consistent with fact, just adds a note about variant deletion)                                                                                                           | Doc                      |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                  | Delete the `_ => (Vec::new(), Type::Void)` fallback; change to explicit error return                                                                                                                                                     | Behavior                 |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                              | Delete `n == "Terminates"`, change to `state.probe().is_predicate(n)`                                                                                                                                                                    | Reverse dependency       |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                              | Delete the `typecheck::operator_interfaces::spec()` call                                                                                                                                                                                 | Reverse dependency       |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                  | Delete the `signature_params` actual argument                                                                                                                                                                                            | With #4                  |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                   | Delete the `Expr::FnDef` arm                                                                                                                                                                                                             | With #3                  |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                  | Keep (it is the parameter-list recovery point, along with 5.5 changes to read Lambda directly)                                                                                                                                           | With #3                  |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814` | Delete 11 match arms                                                                                                                                                                                                                     | With #1                  |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                              | Cut synonyms (case aliases, `i64`-class abbreviations, `"()"`)                                                                                                                                                                           | Convergence              |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                  | Keep the `DateTime` alias                                                                                                                                                                                                                | Keep                     |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                    | Migrate `From<Type> for MonoType` (currently at `mono.rs:692-830`) as the sole conversion point                                                                                                                                          | New                      |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                          | Register `i64`/`int` etc. as equivalent spellings of `Int`                                                                                                                                                                               | Convergence              |
| 23  | `src/middle/core/ir.rs`                                                | `3`                                    | Delete `pub use ... ast::Type`                                                                                                                                                                                                           | Pure deletion            |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                  | `FunctionCode.params` stays `Vec<MonoType>` (already is)                                                                                                                                                                                 | —                        |
| 25  | `src/middle/core/bytecode.rs`                                          | `812`, `814`                           | `BytecodeFunction.params` / `return_type` change to `MonoType`                                                                                                                                                                           | Convergence              |
| 26  | `src/middle/core/bytecode.rs`                                          | `854`                                  | `BytecodeModule.type_table` change to `Vec<MonoType>`                                                                                                                                                                                    | Convergence              |
| 27  | `src/middle/core/bytecode.rs`                                          | `2339`                                 | `type_table` conversion changes to `.collect()` (elements already same-typed)                                                                                                                                                            | With #26                 |
| 28  | `src/middle/core/bytecode.rs`                                          | `2352-2390`                            | **Delete** the entire `From<MonoType> for IrType` impl                                                                                                                                                                                   | Pure deletion            |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                   | `type_table` change to `Vec<MonoType>`                                                                                                                                                                                                   | Convergence              |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 places)                 | `AstType::AssocType` arms handled per the gate conclusion                                                                                                                                                                                | TBD                      |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`           | Type-table gates T1–T3                                                                                                                                                                                                                   | Gate                     |
| 32  | `spawn/placement.rs`                                                   | `122`                                  | Delete the `Expr::FnDef` arm                                                                                                                                                                                                             | With #3                  |
| 33  | `spawn/analysis.rs`                                                    | `912`                                  | Same as above                                                                                                                                                                                                                            | With #3                  |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                   | Same as above                                                                                                                                                                                                                            | With #3                  |
| 35  | `orchestrator.rs`                                                      | `1492`                                 | Same as above, change to take the name from `Assign.target`                                                                                                                                                                              | With #3                  |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                         | Same as above                                                                                                                                                                                                                            | With #3                  |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                 | Same as above                                                                                                                                                                                                                            | With #3                  |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                 | Same as above                                                                                                                                                                                                                            | With #3                  |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                 | Same as above                                                                                                                                                                                                                            | With #3                  |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                   | Same as above                                                                                                                                                                                                                            | With #3                  |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                  | Same as above                                                                                                                                                                                                                            | With #3                  |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                 | Same as above                                                                                                                                                                                                                            | With #3                  |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                          | Same as above + comment update                                                                                                                                                                                                           | With #3                  |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                              | Delete the `Expr::FnDef` test                                                                                                                                                                                                            | Test                     |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                              | Rewrite as Lambda + Assign                                                                                                                                                                                                               | Test                     |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`        | Delete the lower assertions of the 11 dead variants                                                                                                                                                                                      | Test                     |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                              | `mono_to_ast_type` deletes the `AstType::Int` / `Float` / `Bool` / `Char` / `Void` / `String` / `Bytes` branches, uniformly going through `AstType::Name` + `mono.type_name()`. **Must be processed in the same batch as #1**            | Convergence              |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                           | `extract_generic_param_names` signature adds `probe: &dyn TypeEnvProbe`; delete the `typecheck::operator_interfaces::spec()` call and change to `probe.is_constraint(name)`. Callers (including `parser/tests/ast.rs`) pass `&NullProbe` | Reverse dependency       |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                | Delete the `name_used_as_type` in this file, change to call the merged version in `ast.rs`; the `is_predicate` short-circuit is changed to read `NameKind::Predicate`                                                                    | Duplicate implementation |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                  | The `CONST_PARAM_TYPES` judgment is wired through `NameKind::Builtin` (see 5.3)                                                                                                                                                          | Convergence              |

> **The gate itself must also be gated**: `tools/type-tables`, like `code-tables`, also needs `mod`
> declaration wiring (this repo has the lesson of 8 orphan test trees). When creating a new crate,
> register it synchronously in the root `Cargo.toml`.

### 6.4 Backward-Compatibility Strategy

**Constraint**: the 293 `.yx` corpus files in `tests/yaoxiang/` (single file) + the multi-file
corpus layer established in P2 (`tests/yaoxiang-multifile/`) + `src/std/tests/*.yx` + the standard
library, **must all have unchanged behavior** (diagnostic codes and messages the same, order can be
normalized).

| Risk                                                            | Judgment                                                                                                                                                                                                                                                                                                                       | Mitigation                                                                                                                                                                                                                                                                                      |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source code uses `i64` / `int` / `f64` etc. as type annotations | After normalization to `Int`, `from_builtin_name` only recognizes canonical names                                                                                                                                                                                                                                              | **First thing in phase 1**: scan the whole corpus to count the occurrences of non-canonical type names. If 0, the synonyms can be deleted directly; if non-zero, per 5.3 move them to the lexer's alias table (**syntax layer accepts, lower layer normalizes**, no impact on diagnostic codes) |
| Source code uses `DateTime`                                     | Keep the alias (`mono.rs:627`)                                                                                                                                                                                                                                                                                                 | No impact                                                                                                                                                                                                                                                                                       |
| `("()")` appears in type position                               | `types.rs:167`-onward's `LParen` branch handles tuples / parameter groups, producing `Tuple` rather than `Name{"()"}`                                                                                                                                                                                                          | Before deleting `"()"`, count the occurrences of `"()"` as a type name in the whole corpus                                                                                                                                                                                                      |
| `dump_bytecode` output change                                   | `type_table` element type changes from `ir::Type` to `MonoType`, `{:?}` format differs                                                                                                                                                                                                                                         | **Allowed by C3 criterion** (07 specifies that `dump_bytecode` is only compared in C1/C2 phase). But the output diff needs to be recorded in phase 2 for manual review                                                                                                                          |
| Behavior change after the lossy bridge is fixed                 | See 6.2                                                                                                                                                                                                                                                                                                                        | Corpus diff + targeted `dump_bytecode` comparison                                                                                                                                                                                                                                               |
| `mono_to_ast_type` rewrite changes the type-substitution result | `function.rs:507-529` is the exit of monomorphization type substitution (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back to `AstType`). After deleting variants, going through the `_` arm will change the substitution result from `AstType::Int(64)` to `AstType::Name { name: "i64" }` or similar | Phase 3 targeted comparison: select corpus with generic substitution, compare the monomorphized product's `type_name()` before and after the change. **Must not assume `{:?}` output equivalence**                                                                                              |
| `.yx` source code itself                                        | **Zero modification**                                                                                                                                                                                                                                                                                                          | This document does not require any corpus or std source-file modification. This is a hard acceptance item                                                                                                                                                                                       |

**Hard acceptance condition**: `git diff --stat tests/ src/std/` must be empty (except for the
addition of multi-file corpus layer and test-baseline files).

## Implementation Points

The criterion uniformly adopts 07's **C3: diagnostic codes and messages the same (order can be
normalized)**. The following is this document's own track's phase division; cross-document global
ordering is in RFC-039.

| Phase                                         | Content                                                                                                                                                                                                                                              | Acceptance Criterion                                                                                                                                                                                                                                 | Rollback Point                                  |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| **0. Gate First**                             | Create `tools/type-tables`, implement T1/T2/T3; T1's first version **only reports and does not fail**. Take a full baseline on **unchanged** code                                                                                                    | T1's report accurately lists the 13 zero-construction variants (including the qualitative conclusion for `AssocType`), and **separately lists the reverse-bridge reconstruction points recorded in 2.2**; T2/T3 are all green or give a fixable list | No code changes, no rollback needed             |
| **1. Name Normalization**                     | Count non-canonical type names in the corpus; `from_builtin_name` (`mono.rs:620-642`) cuts synonyms and moves them to the lexer's alias table. **Synchronously verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`** | C3: the full corpus's diagnostic set is **identically item-by-item** to the phase 0 baseline                                                                                                                                                         | Independent commit, revertible                  |
| **2. Bytecode Type Unification**              | Change `bytecode.rs:812`/`814`/`854`, `image.rs:44` to `MonoType`; delete `From<MonoType> for IrType` (`bytecode.rs:2352-2390`)                                                                                                                      | C3 + targeted `dump_bytecode` comparison (each item in the 6.2 table)                                                                                                                                                                                | Independent commit, revertible                  |
| **3. Dead-Variant Deletion**                  | Delete 11 variants per the 5.2 table; clean the match arms at `mono.rs:699-708` etc., the `types.rs:836` fallback; **process `function.rs:507-529` in the same batch (#47)**; synchronize `types/tests/mono.rs`                                      | C3 + `cargo build` all green (exhaustive match forces all arms to be filled) + targeted generic-substitution comparison (last row of 6.4)                                                                                                            | Independent commit, revertible                  |
| **4. Parser Data Flow**                       | Add `probe.rs`; change `parser_state.rs:46-58`; change `declarations.rs:498-511` and `ast.rs:930` (#48); `types.rs:162-165` determines `NameKind`; merge the two copies of `name_used_as_type*` (#49)                                                | C3 + **newly added assertion**: `grep 'typecheck' src/frontend/core/parser/` should be 0                                                                                                                                                             | Independent commit, revertible                  |
| **5. AST Dead Variants and Dedicated Fields** | Delete `Expr::FnDef` (16 places) + `Assign.signature_params`; `NamedParen` semantic migration                                                                                                                                                        | C3 + all 18 modules of `tests/integration/` are green                                                                                                                                                                                                | Two separate commits (FnDef / signature_params) |
| **6. Gate Hardening**                         | T1 changes from warning to `panic!`; T4 degrades to a CI assertion                                                                                                                                                                                   | `cargo build` **must fail** when a zero-construction variant is deliberately introduced (red test first)                                                                                                                                             | Independent commit, revertible                  |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Phases 3 and 5 must be after phase 2** —
`bytecode.rs:2372-2373` consume `IrType::String` / `IrType::Bytes`, and deleting the variants first
will break compilation. Phase 3's #47 must be in the same batch as #1, otherwise `mono_to_ast_type`
silently changes behavior.

**Dependency on downstream**: only after all phases of this document are complete can
[04-ssa.md](04-ssa.md) start. At that point, the element types of `BytecodeFunction.params`,
`FunctionCode.params`, and `type_table` are consistent, and the SSA's type annotation has a unique
landing point.

### Mandatory Verification at Each Phase

| Phase | Must Execute                                                                                                                                                                   |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`, manually verify T1's report (**including the separate listing of reverse-bridge reconstruction points**) |
| 1–5   | `cargo test`; full-corpus diff of `tests/yaoxiang/` (07 layer 3); C3 criterion compared item by item                                                                           |
| 2     | Extra: for each variant in the 6.2 table, construct one `.yx` sample, and compare `dump_bytecode` before and after the change                                                  |
| 3     | Extra: corpus with generic substitution, compare the monomorphized product's `type_name()` before and after the change                                                         |
| 4     | Extra: `rg 'typecheck' src/frontend/core/parser/` returns 0                                                                                                                    |
| 6     | Extra: deliberately write a zero-construction variant, confirm `cargo build` fails (red → green)                                                                               |

## Key Decisions and Reasons

### Decisions

| Decision                       | Determination                                                                                                                                  | Reason                                                                                                                                                                                                                                                        |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Target representation form     | `Type` as the sole type structure + `MonoType` as the working form; the `ir::Type` alias deleted                                               | The alias at `ir.rs:3` is the direct cause of the field-type inconsistency between `BytecodeFunction.params` and `FunctionCode.params` (2.1). After the alias is deleted, the two field types are unified and the `From` bridge loses its reason for existing |
| Reject "unify into `MonoType`" | Not adopted                                                                                                                                    | Would cause an L2→L3 reverse dependency (violating RFC-039 layering), and would require adding `Span` / `ConstExpr` back to `MonoType`. Full reasoning in 5.1                                                                                                 |
| `NamedParen` not deleted       | Kept as a pure syntax node, binder semantics migrated to `sema/`                                                                               | The comment at `ast.rs:519-532` records that deletion would reintroduce a previously-fixed correctness defect: `(r: P(m))` would silently substitute the undeclared `m` as a binder                                                                           |
| Directory rename               | **Do it** (D1), executed at the close of P5/P6 as a pure-move batch; the body of this document is written against the pre-rename current paths | 2026-10-03 decision (ChenXu233); 2026-10-04 supplementary construction method                                                                                                                                                                                 |

### Rejected Alternative Plans

All four alternative plans were not adopted: **keeping three representations and only adding a
conversion layer** does not solve any root cause, and `ir::Type` is still not an independent type,
with dead variants, synonym table, and reverse dependency all remaining; **using macros to generate
the synonym table** treats the symptom, not the cause — after `Type::Int(usize)` is deleted the
macro has no reason to exist, and the consistency guarantee is taken over by gate T2 (more
verifiable than macro-expansion correctness); **only deleting dead variants without unification**
has the lowest cost but leaves the problem in place, and with no gate to prevent the next person
from adding `Type::Int128(usize)` causing the same dead variant to grow back (if only one commit can
be made, it should be explicitly recorded as a subset of this plan and T1 should be delivered
together); **unifying into `MonoType`** see 5.1.

### Benefits

- **Eliminates the maintenance burden at 14 locations**. The 12 consumption sites + 2 exhaustive
  arms of `Expr::FnDef` are a never-executed parallel path; after deletion, `ir_gen.rs` /
  `ownership.rs` / `termination.rs` all lose one branch.
- **Eliminates a duplicated, diverged implementation**. The two copies of `name_used_as_type` (2.4)
  have already diverged in field recursion inside definition bodies; after merging, only one
  remains.
- **After the lossy bridge disappears, `IrType::Void` is no longer a "legal type value"**. This is
  the prerequisite for 07's layer-1 validator "type-consistency" invariant to actually pass green.
- **The gate makes regression impossible**. T1 turns "newly-added zero-construction variants" from
  silently passing currently into build failure — the existing `build.rs` mechanism in this repo
  proves this path works.
- **Zero source-code changes**. The 293 `.yx` corpus and the std library do not need any
  modification; this is the root reason C3 can hold.

## Known Limitations and Risks

### Risks

- **Phase 2 may have behavior changes**. After the lossy bridge is fixed, the 8 `MonoType` variants
  now see real types instead of `Void`. The direction is a fix, but it will change paths that depend
  on signature bytecode. This is the document's biggest uncertainty.
- **The qualitative conclusion on `AssocType` depends on the gate's accuracy**. If T1 uses
  line-level heuristics (`=>` count) instead of `syn`, it may misjudge. This is the reason phase 0
  must be manually verified.
- **T1 missing the reverse bridge will make the disposition table and the gate contradict each
  other**. 2.2 has proven that the 7 "dead variants" have production reconstruction points in the
  reverse bridge. If T1 does not separate the construction-point direction, these 7 will be judged
  "has construction points" and exempted from deletion — the gate will become an obstacle to the
  deletion work rather than a guarantee.
- **Deleting `signature_params` has semantic risk**. It carries the **unfiltered** parameter list,
  while `Lambda.params` is the **filtered** list. If `classify_generic_params` depends on the
  former, the change will alter the generic-classification result. **It must be verified first,
  equivalence cannot be assumed** (the `signature_params` consumption site on the ir_gen side at
  `ir_gen.rs:1380` is also verified together).
- **Phase 3 touches `middle/passes/mono/function.rs`**. Beyond the 8 `AssocType` matches,
  `mono_to_ast_type` at `507-529` is the exit of monomorphization type substitution; after the
  rewrite, the substituted `type_name()` may change (last row of 6.4).
- **Non-canonical type names in the `.yx` corpus may be non-zero**. If the corpus uses `i64` / `int`
  extensively, phase 1's "delete synonyms directly" is not feasible and degrades to "move to the
  lexer's alias table", reducing the convergence magnitude of `from_builtin_name`.
- **The gate will be red at first**. If T1's first version `panic!`s directly, it will immediately
  block compilation. It must first only report to take a baseline.

### Unresolved Questions

> **All open questions originally listed in this section have been decided.** The
> decision-by-decision results are in the
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document does not leave any to-be-determined items.**

## See Also

### Upper-Level and Sibling Documents

- [RFC-039 Compiler Function Routing Directory Design (Master Plan)](../../rfc/accepted/039-compiler-architecture.md)
  — the upper-level master plan; the four-layer model, criterion tiering, routing table C (3
  type-representation-related items belong to this document)
- [01-routing.md](01-routing.md) — Function Routing Tables A/B/C, dependency-direction
  specification, target directory structure
- [04-ssa.md](04-ssa.md) — SSA-ization; this document's prerequisite and downstream consumer
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — Frontend paradigm; operator-change-surface
  convergence
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C3 category definition, diagnostic-set
  comparison specification, the layer-1 IR validator's type-consistency invariant

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — basis of the
  `Type::Generic` unified path
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — builtin type table
  (T3 gate's comparison object)
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — example
  of `build.rs` generation-time gate
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — origin of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

- `src/frontend/core/parser/ast.rs:427-541` — `ast::Type` 26-variant definition
- `src/frontend/core/parser/ast.rs:519-540` — `Type::NamedParen`'s binder-semantics explanation
  (evidence that deletion would reintroduce the defect)
- `src/frontend/core/parser/ast.rs:241-249` — `StmtKind::Assign.signature_params`'
  typechecker-specific field
- `src/frontend/core/parser/ast.rs:838-841` — `CONST_PARAM_TYPES`: the third copy of
  const-generic-parameter names
- `src/frontend/core/parser/ast.rs:843-885` — `name_used_as_type_in`: the duplicate implementation
  of `declarations.rs:42` (already diverged)
- `src/frontend/core/parser/ast.rs:891`, `930` — `extract_generic_param_names` and its boundary call
  inside
- `src/frontend/core/parser/ast.rs:191-213`, `217-223` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — the parser represents all primitive types
  as `Type::Name`
- `src/frontend/core/parser/statements/types.rs:392-396` — `Result`/`Option` not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:836` — `Type::Void`'s sole forward construction
  point (error fallback)
- `src/frontend/core/parser/statements/declarations.rs:42-80` — `name_used_as_type` original
- `src/frontend/core/parser/statements/declarations.rs:494-511` — hard-coded `"Terminates"` and
  `typecheck::operator_interfaces::spec()` call
- `src/frontend/core/parser/pratt/nud.rs:505-514` — expression position represented as empty-body
  `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:466` — the `expr_to_params` arm fishes the parameter list
  back out
- `src/frontend/core/types/mono.rs:618-643` — `from_builtin_name` synonym table
- `src/frontend/core/types/mono.rs:692-830` — `From<Type> for MonoType` implementation (12 of the 13
  match arms are here)
- `src/frontend/core/types/mono.rs` — `MonoType` definition (about 22 variants,
  `mono.rs:167`-onward; 1077-line file)
- `src/frontend/core/types/const_data.rs:234-258`, `321-330` — `const_data::BinOp` /
  `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — compile-time constant evaluation (1128 lines)
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- `src/middle/core/ir.rs:3` — evidence of `ir::Type` as the `ast::Type` alias
- `src/middle/core/bytecode.rs:808-814` — `BytecodeFunction`'s `Vec<ir::Type>` fields
- `src/middle/core/bytecode.rs:2352-2390` — lossy `From<MonoType> for IrType` bridge
- `src/middle/passes/codegen/bytecode.rs:32`, `275-278` — `FunctionCode`'s `Vec<MonoType>` fields
- `src/middle/passes/mono/function.rs:507-529` — `mono_to_ast_type`: reverse bridge that
  reconstructs 7 "dead variants"
- `src/backends/interpreter/image.rs:44` — interpreter module's `type_table`
- `build.rs:19-37` — error-code registry construction-time gate (form reference for the type-table
  gate)
- `build.rs:39-54` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — gate crate depends only on `serde_json` (root crate has no
  `syn`; `tools/type-tables` introducing `syn` is the first exception)
