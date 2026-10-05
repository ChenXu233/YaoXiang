# Type Representation Unification

> **Supplementary design document.** This document is supplementary to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/draft/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiering, and execution-stage order are in the RFC-039 main
> text; the positioning of each supplementary document is found in the
> [index of this directory](index.md).

## Position and Scope

### Coverage

Converge the currently parallel multiple type representations into a single set, and eliminate the
four categories of burden derived therefrom:

- **13 production-zero-construction `ast::Type` variants** (out of the 26 variants in
  `ast.rs:427-541`, see 2.2)
- **A hand-written type-name synonym table** (`mono.rs:618-643`, see 2.3)
- **The parser's real reverse-dependency on the typecheck layer** (2 locations, see 2.4)
- **One production-zero-construction expression variant** `Expr::FnDef` (`ast.rs:37-43`, see 2.6)

The concrete deliverables are four: a per-variant disposition table for the 26 variants (5.2), the
semantic-defining `NameKind` for `Type::Name` and synonym-table elimination (5.3), the
`TypeEnvProbe` injection data flow (5.4), and the type-table generation-time gates T1–T3 (5.7).

### Directory Renaming and This Document's Paths

RFC-039 decision D1 determines that the directory renaming (`typecheck/`→`sema/`,
`middle/core/`→`middle/ir/`) is to be executed **together with P5/P6 completion, not split into
stages**. The 50-item change list and all acceptance greps in this document are written **using the
pre-rename current paths**—they are the construction baseline for each P6 batch **before** the
rename batch; the directory rename at the end of each batch is a **pure move batch** (C1 zero-diff),
in a separate commit. It is expected that the baseline becomes invalid after the rename commit; use
git history as the bisection reference.

### Out of Scope

- **The construction method for directory renaming.** `Type` is not moved out of `parser/ast.rs` in
  this stage; only `lower.rs` is added under `types/` as the sole conversion point. Directory
  renaming (`typecheck/`→`sema/`, etc.) is executed at the end of P5/P6 per RFC-039 decision D1,
  with the form shown in the target directory structure in `01-routing.md`.
- **Four-layer model and dependency-direction specification.** See [01-routing.md](01-routing.md).
- **Equivalence-criteria tiering and validator implementation.** The definitions of C1–C6, the
  three-tier criteria (IR validator / normalized snapshot / corpus diff) are in
  [07-equivalence-oracle.md](07-equivalence-oracle.md). This document only references **C3
  (type-representation convergence)** as its own acceptance criterion.
- **SSA-ification.** See [04-ssa.md](04-ssa.md). This document is its prerequisite; see reasoning
  below.
- **P1–P10 global execution sequence.** See the RFC-039 main text and
  [02-stage-contract.md](02-stage-contract.md). This document only gives the stage partition for its
  own line (see "Implementation Notes").

### Division of Labor with RFC-039

RFC-039 is the upper-level master document: the four-layer model, criteria tiering, cross-document
stage order, and "why do this refactoring now." This document is the **construction blueprint for
the "type-representation convergence" line in RFC-039**: which variants to delete, which files to
change, in what order, and what gates prevent regression.

All line numbers, variant lists, and construction-point statistics in this document come from static
verification of `src/`, with `file:line` evidence given item by item. **This document does not
record execution process; it records only code facts and the dispositions derived from them.**

For ease of cross-referencing with the same batch of documents, the "Target Design" and "Detailed
Design" chapters follow the `5.x` / `6.x` subsection numbering; the remaining chapters follow the
unified style of this directory.

### Why This Must Precede SSA-ification

`ir::Type` is not a third independent type, but a `pub use` alias of `ast::Type` (`ir.rs:3`).
[04-ssa.md](04-ssa.md) needs to introduce register-type annotations and new instructions to IR; if
the representation is not converged before that, the SSA type annotations will grow directly on the
26-variant syntax type via the `ir::Type` alias, effectively solidifying a third representation.

The capability boundary of the current `ir::Type` is fixed by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) embeds a complete expression tree inside a type.
  An `Expr` appearing in an IR instruction's type annotation is impossible.
- `ast::Type::Literal { name, base_type }` (`ast.rs:476-482`) carries the source-code name and
  `Span`; it is the syntax form of a compile-time literal type, not a runtime type.

Doing SSA type annotations on top of the 26 variants is equivalent to permanently solidifying these
13 dead variants and the two syntax-only variants into the IR contract.

## Current State

The following are all statements of code fact, with no proposals. Each item gives `file:line`
evidence.

### 2.1 Three Parallel Representations and Lossy Bridging

The multiplicity of type representations is the common upstream of four independent defects:

| Downstream symptom                                                            | Causal relationship with type representation                                                                                                                                                                                                                                                                                                         |
| ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The same type text has multiple legal AST forms                               | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but type-distinct**, the parser picks the latter, and typecheck must supplement a table to translate the former back                                                                               |
| Bytecode-layer signatures are inconsistent with IR-layer signatures           | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only obtain the alias, so `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`) while `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) declares `params` as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding a built-in type name requires changes in multiple places with no check | Because the name→type mapping is scattered in `from_builtin_name` (`src/frontend/core/types/mono.rs:618-643`) and the string match in `bytecode.rs:2371-2376`, each is written separately with no cross-validation                                                                                                                                   |
| `"Terminates"` is hard-coded in the parser                                    | Because the parser cannot obtain the judgment "is this a type" or "is this an operator interface", it can only embed type-layer knowledge as a string literal into the syntax layer (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                      |

**The first three are representation problems; the fourth is a downstream side effect of the
representation problem.** Doing only item 1 (deleting dead variants) leaves the synonym table and
the reverse dependency; doing only item 4 (injecting callbacks into the parser) builds on
still-inconsistent representations.

One of the three representations is an alias illusion:

| Name        | Definition location                       | Lines       | Role                                                    |
| ----------- | ----------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541` | 26 variants | Syntax-layer type                                       |
| `MonoType`  | `src/frontend/core/types/mono.rs`         | 1077 lines  | Semantic-layer type (after monomorphization)            |
| `ir::Type`  | `src/middle/core/ir.rs:3`                 | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

`ir.rs:3` is a single `pub use` line, and `ir.rs:6` immediately does
`use crate::frontend::core::typecheck::MonoType;`. The IR module depends on two "type" symbols at
the same time, but one of them is merely an alias.

There is a fourth type carrier on the serialization side:

| Carrier                         | Element type    | Location                                   |
| ------------------------------- | --------------- | ------------------------------------------ |
| `BytecodeModule.type_table`     | `Vec<ir::Type>` | `src/middle/core/bytecode.rs:854`          |
| `FunctionCode.type_table`       | `Vec<MonoType>` | `src/middle/passes/codegen/bytecode.rs:32` |
| Interpreter module `type_table` | `Vec<ir::Type>` | `src/backends/interpreter/image.rs:44`     |

**Lossy bridging.** `From<MonoType> for IrType` (`src/middle/core/bytecode.rs:2353-2390`) downgrades
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
(`bytecode.rs:2386-2387`, whose comment self-admits to being a "transitional branch before deleting
the enum variant").

`IrType::Void` here is **a legal type value rather than an error signal**. Therefore downstream
cannot distinguish between "this function actually returns `Void`" and "this function's return type
was lost during bridging"—this is one of the reasons the first-tier "type consistency" invariant of
07's first-tier validator cannot actually pass green on the current code.

Meanwhile `IrType::String` / `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`) and
`ast::Type::Bytes` (`ast.rs:436`), which are exactly the dead variants to be deleted in 2.2—**both
ends of the lossy bridge point to dead code**.

### 2.2 Production-Zero-Construction Variants

**Criterion scope**: The construction-point scan range is all of `src/`, excluding `tests/` and
`*/tests/*`. Of the 26 variants of `ast::Type`, the following 13 have **only match arms in
production code and no construction points**:

| Variant                              | Definition       | All production hits                                                                                                                                        |
| ------------------------------------ | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Int(usize)`                         | `ast.rs:432`     | `mono.rs:699` (arm)                                                                                                                                        |
| `Float(usize)`                       | `ast.rs:433`     | `mono.rs:700` (arm)                                                                                                                                        |
| `Char`                               | `ast.rs:434`     | `mono.rs:701` (arm)                                                                                                                                        |
| `String`                             | `ast.rs:435`     | `mono.rs:702` (arm)                                                                                                                                        |
| `Bytes`                              | `ast.rs:436`     | `mono.rs:703` (arm)                                                                                                                                        |
| `Bool`                               | `ast.rs:437`     | `mono.rs:707` (arm)                                                                                                                                        |
| `Void`                               | `ast.rs:438`     | `mono.rs:708` (arm), `types.rs:836` (**only construction point**)                                                                                          |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | `mono.rs:747` (arm)                                                                                                                                        |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | `mono.rs:743` (arm)                                                                                                                                        |
| `Option(Box<Type>)`                  | `ast.rs:455`     | `mono.rs:770` (arm)                                                                                                                                        |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | `mono.rs:771` (arm)                                                                                                                                        |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | `mono.rs:814` (arm)                                                                                                                                        |
| `AssocType { .. }`                   | `ast.rs:463-471` | `mono.rs:781-786` (arm), `formatter/handlers/types.rs:84` (arm), `semantic_tokens.rs:228` (arm)—**no production construction point found in current code** |

The root-cause chain can be fully verified:

1. **The parser represents all primitive types as `Type::Name { name }`**—the fallback arm at
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered into dedicated nodes**—the comment at `types.rs:392-396`
   explicitly states: "RFC-010: Result/Option are not lowered into dedicated AST nodes…they take the
   same `Type::Generic` path as user-defined generic types. The type representation is still
   `Generic{"Result"/"Option", args}`."
3. **The only construction point of `Type::Void` is an error fallback**—`types.rs:836`'s
   `_ => (Vec::new(), Type::Void)`, at the end of a match returning `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction points: `types.rs:364`, `types.rs:571`, `types.rs:100`, and
`declarations.rs:573`; they are not dead variants.

**These 13 variants have construction points in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
Hence this document's statement is "**production zero construction**" rather than "global zero
reference." These tests are themselves the solidification of the tested behavior—they assert exactly
"what these syntax forms can be lowered to," and these forms are never produced by the parser. When
deleting variants, these tests must be deleted or rewritten.

#### Scope Boundary: Reverse Bridges Reconstruct 7 of Them

The "only construction point / no construction point" in the above table only holds in the **parser
→ AST forward-construction** direction. There are two production-level `MonoType → ast::Type`
reconstructions in the reverse direction, which **actually construct 7 of the variants in the above
table**:

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
    // #299: container types such as String/Bytes are now Generic, reverse-solved via type_name back to AstType
    MonoType::Generic { name, .. } if name == "String" => AstType::String,
    MonoType::Generic { name, .. } if name == "Bytes" => AstType::Bytes,
    _ => AstType::Name { name: mono.type_name(), span },
}
```

**This scope boundary has a direct effect on the disposition**:

- The `bytecode.rs` location disappears together with #28 of 6.3 (deletion of the entire
  `From<MonoType> for IrType` impl), incurring no additional cost.
- `function.rs:507-529` **is not within the coverage of any existing change item**. It is a
  `MonoType → AstType` type-name reverse-solver (with the comment `#299`), which reverse-solves
  `String` / `Bytes` from `Generic` back to dedicated variants by name. After deleting
  `ast::Type::String` / `Bytes`, these two guard branches must be rewritten; after deleting `Int` /
  `Float` / `Bool` / `Char` / `Void`, the behavior of the `_` arm changes (types originally falling
  into `_` will silently switch to `AstType::Name`). This is item 47 added by 6.3.
- **Direct consequence for the gate**: A T1 that scans only forward construction points would judge
  the above 7 variants as "having construction points", thereby removing them from the deletable
  list. T1 must explicitly include reverse bridges in the statistical scope; otherwise the gate's
  conclusion will contradict this document's disposition table (see 5.7).

### 2.3 Hand-Written Synonym Table

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:618-643`) uses a string table to map
type names to `MonoType`:

```
"Int" | "int" | "Int64" | "int64" | "i64"  => Some(MonoType::Int(64)),   // :620
"DateTime" | "datetime"                     => Some(MonoType::Int(64)),   // :627
"Int32" | "int32" | "i32"                   => Some(MonoType::Int(32)),   // :628
...
"Void" | "void" | "()"                      => Some(MonoType::Void),      // :640
```

**The only reason this table exists is to bridge inconsistency at the AST layer**: because
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent but type-distinct at
the AST layer (and the parser only produces the latter), `from_builtin_name` must simultaneously
accept different cases, abbreviations, signed names, and the `DateTime` alias. `"()"` appearing in
the `Void` line is especially telling—it cooperates with the `Type::Void` fallback at
`types.rs:836`.

The same "which names are built-in type names" knowledge has a total of four copies in the
repository, with no consistency check between them:

| #   | Location                              | Form                                                                                                                               |
| --- | ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `mono.rs:618-643` `from_builtin_name` | String match, name → `MonoType`                                                                                                    |
| 2   | `bytecode.rs:2371-2376`               | String match, copies `"String"` / `"Bytes"` / `"Tuple"`                                                                            |
| 3   | `ast.rs:838-843` `CONST_PARAM_TYPES`  | 15 const generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`) in a `&[&str]` constant |
| 4   | `src/lsp/world.rs:176`                | LSP-side built-in type name list                                                                                                   |

Item 3 `CONST_PARAM_TYPES` is used by `ast.rs:912`'s `extract_generic_param_names` to judge const
generic parameters—it enumerates "type names usable as const parameters", which has a
**partial-overlap but non-identical** relationship with the full set of `from_builtin_name`, and no
mechanism guarantees they stay synchronized. Item 4 is the fourth copy used by the LSP for syntax
highlighting/hovers, likewise without validation (the T2 comparison scope includes it).

### 2.4 Parser's Reverse Dependency on the Type Layer

RFC-039 routing table C lists 3 entries (`declarations.rs:27-80` calls `is_type_param_annotation` /
`name_used_as_type`, which are functions belonging to the type layer). **The actual measurement
differs from that statement**: these two functions are defined inside the parser itself, not in the
type layer.

- `is_type_param_annotation` is defined in
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the only `fn` definition in the whole
  repository)
- `name_used_as_type` is defined in `declarations.rs:42-80` (the only `fn` definition in the whole
  repository)
- `declare_predicate` / `is_predicate_name` are defined in
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

The production code with real cross-layer references to `typecheck` totals **2 locations**, and they
call the same function `crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Containing function                | Purpose                                                                                        |
| --- | --------------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature-parameter filter closure | Judging constraint-position formal parameters (`T: Add`), not occupying runtime parameters     |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`      | Judging constraint-position formal parameters, producing `GenericParamName` with `constraints` |

Location 2 is more inner than location 1—**`parser/ast.rs` itself** (the entire block
`ast.rs:847-948` with `StmtKind`/`Expr` in the same file) directly references a typecheck symbol.
Routing table C only records the one in `declarations.rs`.

It combines with the hard-coding at `declarations.rs:498-499` to form the same judgment:

```rust
// declarations.rs:498-499
// Extended as built-in predicates are added in the future (future open-set solution:
// pass predicate names down to the parser, or change to a syntax-layer-judgeable form).
// The parser does not hold a type environment, so it checks two places:
// hard-coded built-in predicates, plus the declarations accumulated in the current pass (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment self-admits to being a temporary form of a "future open-set solution".
`n == "Terminates"` is **embedding type-layer knowledge as a string literal in the syntax layer**.

**Why this is downstream of the type-representation problem**: the reason the parser needs to ask
"is this an operator-interface name" is that it must distinguish whether `Type::Name` refers to a
type or to an interface constraint. **`Type::Name { name: String }` carries at least 4 semantics**
(concrete type / type variable / constraint name / predicate name) with a single undifferentiated
string, the syntax layer cannot self-judge, and the judgment power is forced to escalate to the type
layer.

#### Duplicate Implementation: Same Name, Same Meaning, Behavior Already Diverged

`name_used_as_type` has two implementations. The comment at `ast.rs:843-846` self-admits this:

```rust
/// Whether the parameter name is used as a type reference in the given type
/// (`(N: Int) -> (n: N)`'s `N`).
///
/// Synonymous with the `name_used_as_type` in `declarations.rs`; this is implemented
/// independently here to avoid cross-module dependencies inside the parser.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations have **already diverged**:

| Dimension               | `declarations.rs:42-80` `name_used_as_type`                                     | `ast.rs:847-885` `name_used_as_type_in`              |
| ----------------------- | ------------------------------------------------------------------------------- | ---------------------------------------------------- |
| Signature               | With `is_predicate: &dyn Fn(&str) -> bool` callback                             | No callback                                          |
| `Type::Generic` branch  | First short-circuits with `is_predicate(app_name)` returning `false` (`:54-56`) | No such short-circuit, recurses into `args` directly |
| `Type::Struct { body }` | **None** (falls into `_ => false`)                                              | **Has** (`:871-879`, traverses `Field` / `Expr`)     |
| `Type::NamedStruct`     | **None**                                                                        | **Has** (`:880-882`, traverses `fields`)             |

The comment at `ast.rs:868-870` records the origin of the latter two branches: "Previously these two
branches were missing, and const generic parameters, when referenced inside their definition body,
were not recognized as 'used as a type'; the annotation check would falsely report legal const
parameters as unknown names."

That is: **the function copied to avoid cross-module dependencies has fixes that have not flowed
back to the original**. The `ast.rs` version has field recursion inside the definition body; the
`declarations.rs` version does not—the same source code yields different conclusions at the two
judgment points. This must be merged along with the introduction of `NameKind` in 5.3.

### 2.5 Parallel Operator Enums

There are two copies of the operator enum, not referring to each other, and **the members have
already diverged**:

| Concept         | Syntax layer                    | Constant-evaluation layer                                             | Difference                                                                                                                      |
| --------------- | ------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Binary operator | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | Syntax layer has unique `Neq` (`:197`), `Range` (`:205`), `Assign` (`:206`); constant layer uses `Ne` (`:243`) instead of `Neq` |
| Unary operator  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | Syntax layer has unique `Deref` (`:222`); constant layer has unique `BitNot` (`:329`)                                           |

The two also each implement a complete `Display` (`const_data.rs:291-317`) and category predicates
(`const_data.rs:260-289`'s `is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise`).

**Why this is part of the type-representation problem**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) **are not types**—when they appear in a type-position expression,
they fall into `Type::ConstExpr` (`ast.rs:506`), i.e. "compile-time-only variant." This is directly
related to the IR-layer type discipline of 6.1: after convergence, `Type::ConstExpr` disappears
after `Type → MonoType`; IR never sees it, while `Assign` / `Deref` continue to exist via paths like
`ir_gen.rs`'s `BinOp::Assign => Ok(MonoType::Void)` (`inference/expressions.rs:875`). The divergence
of the two enums makes "which operators can appear in type position" impossible to determine from
either side alone.

This stage does not merge these two enums (their assignment is in
[05-frontend-paradigm.md](05-frontend-paradigm.md)'s "operator change surface"), but T1's scan
objects should include `ast::BinOp` / `ast::UnOp`, so that "newly added zero-construction operators"
are also visible.

### 2.6 `Expr::FnDef`: A Production-Zero-Construction Expression Variant

`Expr::FnDef` (`ast.rs:37-43`) **being production zero-construction** is true—the whole repository
has only two construction locations: `src/frontend/core/parser/tests/ast.rs:837` and
`src/frontend/core/typecheck/tests/checker.rs:111`, both are tests.

But **the consumption points are far more than 4**. The actual production consumption points are 12:

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

In addition, there are 2 exhaustive match arms forced to exist for it: `ast.rs:1026`
(`Expr::span()`) and `pratt/mod.rs:47` (`expr_end_line`).

**The path that function definitions actually take** is: `nud.rs:425-429` constructs
`Expr::Lambda` + `declarations.rs:462-479` constructs `StmtKind::Assign` (putting the `Lambda` into
`value`). That is to say, the `FnDef` branch is **a parallel path that is never executed but
maintained by 14 locations**.

### 2.7 Syntax Nodes Save Semantics on Behalf of the Type Layer

**First, `StmtKind::Assign` carries typechecker-specific fields.** The `Assign` variant at
`ast.rs:241-249` contains `signature_params: Vec<Param>` (`ast.rs:244-245`), with the comment
stating outright:

> `/// The first group of signature parameters as-is (including parameter names), for the typechecker's classify_generic_params`

The statement AST reserves a dedicated slot for the type checker. Construction point:
`declarations.rs:472`.

**Second, expression position is treated as a parameter list.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a **Lambda with an
empty body**. Then the `Expr::Lambda { params, .. } => Some(params.clone())` arm at
`src/frontend/core/parser/pratt/led.rs:466` "fishes it back" as a parameter list.

That is: a normal variant of the AST is used as a **temporary parameter-list carrier**, and then its
semantics are restored by a match arm in another module.

**Third, `Type::NamedParen` carries a return-position binder name.** The comment at `ast.rs:519-540`
states outright:

> RFC-027 §3's return-position refinement relies on it to declare **return formal-parameter names**
> (called binders here)…**drop it, and the type checker can only guess** "free variables in
> constraints that are not in scope are return-value formal parameters", so `(r: P(m))` would
> silently substitute the undeclared `m` as a binder (verified defect).

The syntax node is saving binder identity on behalf of the type layer.

## Target Design

### 5.1 Form of the Target Representation

**Position: a single representation = `Type` (originally `ast::Type`) as the sole type structure;
`MonoType` demoted to a working form internal to the type checker; the `ir::Type` alias deleted.**

Three representations converge into **two roles**, but only one is a "type":

```
Sole type structure  Type  ─────────────────────────────┐
   (26 → 13 variants, lives in types/ rather than parser/ast.rs)  │
                                                  │  From<Type> for MonoType  (total, no semantic loss)
                                                  │  ← sole conversion point types/lower.rs
Type-checker working form  MonoType  ──────────────────────┘
   (carries TypeVar / substitution state, only flows within typecheck)
   ×
ir::Type alias deleted; BytecodeFunction.params / type_table use MonoType
```

**Why not "merge into `MonoType`"** (this is the most easily proposed option, and is rejected here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.),
   `MonoType` does not. The C3 criterion of 07 is "diagnostic codes and messages are identical"—the
   diagnostic's location information must be traceable back to the source. Adding `Span` to
   `MonoType` would cause post-monomorphization types to carry a bunch of meaningless sentinel
   spans.
2. `MonoType` contains `TypeVar` and substitution state (`types/substitute.rs:112`,
   `types/solver.rs:411`'s `pub fn unify`). These are **intermediate states of the type checker's
   solving process**, not types themselves. Putting them into the sole type structure is equivalent
   to making the IR construction layer need to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027), lowered away in `MonoType`. Unifying into `MonoType` means
   these syntax forms must be added back to it.
4. **The most critical reverse argument**: elevating `MonoType` to the sole representation is
   equivalent to making the L2 syntax layer depend on the L3 semantic layer's types. RFC-039
   explicitly forbids L2 from depending on L3. One of the goals of this document is to eliminate
   reverse dependencies; choosing a solution that would create reverse dependencies is
   self-contradictory.

**Why `ir::Type` must be directly deleted rather than "preserved"**:

The alias at `ir.rs:3` causes `BytecodeFunction.params` (`bytecode.rs:812`) and `return_type`
(`bytecode.rs:814`) to use `ir::Type`, while on the serialization side `codegen/bytecode.rs:277-278`
uses `MonoType`. The two sets of field types are connected via `bytecode.rs:2339`'s
`file.type_table.into_iter().map(|t| t.into())` and the lossy `From` bridge at
`bytecode.rs:2353-2390`.

**The only reason the `From` bridge exists is "the two sides have different field types."** Unifying
`bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, `image.rs:44` to use `MonoType` removes the
`From<MonoType> for IrType` from having an existence reason and it can be deleted in its entirety
(together with the reverse-reconstruction recorded in 2.2).

**Explicitly not done**: `Type` is not moved from `parser/ast.rs` to `sema/types/`. In this stage,
only `lower.rs` is added under `types/` as the sole conversion point; the definition location of
`Type` is not moved; directory renaming is executed at the end of P6 as a separate move batch per
D1—at that point `Type`'s home is the top-level `ast/type_.rs` (AST top-level domain), and
`sema/types/` is the home of the `MonoType` working form.

### 5.2 Per-Variant Disposition Table for the 26 Variants

> Disposition categories: **Delete** (production zero-construction), **Retain** (production has
> construction points), **Migrate** (semantics relocated to other nodes).

| #   | Variant                                                            | Definition line  | Disposition                                | Reason                                                                                                                                                                                                                                                                                       |
| --- | ------------------------------------------------------------------ | ---------------- | ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Retain** (strengthened)                  | The parser's **sole** primitive-type exit (`types.rs:162-165`). After deleting the 12 primitive-type variants, all primitive types go through it. A "semantic kind of name" annotation needs to be added (see 5.3)                                                                           |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                                 | Production zero-construction (`mono.rs:699` is the only arm). Bit-width is carried by the `Name` path in `from_builtin_name`                                                                                                                                                                 |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                                 | Production zero-construction (`mono.rs:700`)                                                                                                                                                                                                                                                 |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                                 | Production zero-construction (`mono.rs:701`)                                                                                                                                                                                                                                                 |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                                 | Production zero-construction (`mono.rs:702`). **Note**: `bytecode.rs:2372`'s `IrType::String` consumes it; that arm must be rewritten after deletion                                                                                                                                         |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                                 | Production zero-construction (`mono.rs:703-706`). Same as above: `bytecode.rs:2373` consumes it                                                                                                                                                                                              |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                                 | Production zero-construction (`mono.rs:707`)                                                                                                                                                                                                                                                 |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                                 | The only production construction is the error fallback `types.rs:836`. This fallback is eliminated together with the error-path refactoring in 5.6                                                                                                                                           |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Retain**                                 | Has production consumption (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                                          |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Retain**                                 | Has production construction point `types.rs:364`                                                                                                                                                                                                                                             |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                                 | Production zero-construction (`mono.rs:743-746`)                                                                                                                                                                                                                                             |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                                 | Production zero-construction (`mono.rs:747`). Enum goes via `Struct` + `TypeBodyItem`, unrelated to `mono.rs:2378`'s `MonoType::Enum(_) => IrType::Void`                                                                                                                                     |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Retain**                                 | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                               |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:451-454` | **Retain**                                 | Consumed by `bytecode.rs:2366`                                                                                                                                                                                                                                                               |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                                 | Production zero-construction (`mono.rs:770`). `types.rs:392-396` already states it takes the `Generic` path                                                                                                                                                                                  |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                                 | Production zero-construction (`mono.rs:771`). Same as above                                                                                                                                                                                                                                  |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Retain**                                 | The actual representation of `Result` / `Option` / `String` / `Bytes` all goes through it (`types.rs:397-399`)                                                                                                                                                                               |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                                 | Production zero-construction (the only construction in the whole repository is in test `types/tests/mono.rs:178`; the syntax layer has no `::` path that produces it). If associated-type syntax is enabled in the future, it will be redefined by a new proposal at that time (decision D8) |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                                 | Production zero-construction (`mono.rs:814`)                                                                                                                                                                                                                                                 |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Retain**                                 | Has production construction point `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                                     |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Retain**                                 | Raw pointer types inside `unsafe` blocks                                                                                                                                                                                                                                                     |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Retain**                                 | Borrow notation                                                                                                                                                                                                                                                                              |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Retain**                                 | Has production construction points `types.rs:100`, `declarations.rs:573`; RFC-010                                                                                                                                                                                                            |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Retain (and mark as compile-time-only)** | Has production construction point `types.rs:160`. The only variant in `Type` that embeds `Expr`; forbidden from being carried at the IR layer (see 6.1)                                                                                                                                      |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Retain**                                 | RFC-004 currying's layer terminator; `split_curry` depends on its existence                                                                                                                                                                                                                  |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                                | Semantics (binder name) relocated to the type checker; the AST retains only the syntax. See 5.6 for details                                                                                                                                                                                  |

**Net effect: 26 → 14 variants** (delete 12, migrate 1, retain 13).

> **Reverse bridges must be handled before deleting variants**: the `function.rs:507-529` recorded
> in 2.2 reconstructs 7 of items #2–#8. Deletion-wise it must be processed in the same batch as
> #1–#8, otherwise the `_` arm of that function will silently change behavior (see #47 of 6.3).

> **`AssocType` handling**: per D8, directly delete (table item #18 of 5.2). T1's gate (5.7) **must
> independently reproduce this conclusion** in its first report on unmodified code—if the report
> shows `AssocType` has a production construction point, it means this section's verification is
> wrong, and the **correct action is to go back to the RFC decision table to change D8 and explain
> why**, not to leave the variant in the whitelist.

### 5.3 `NameKind` and Synonym-Table Elimination

**Problem**: `Type::Name { name: String }` carries at least 4 semantics—concrete type / type
variable / constraint name (operator interface) / predicate name—with a single undifferentiated
string. `declarations.rs:498-499` needs to judge "is it a predicate", `declarations.rs:508-511`
needs to judge "is it an operator interface", `ast.rs:930` needs to judge "is it an operator
interface", and the two `name_used_as_type*` in `declarations.rs:42-80` and `ast.rs:847-885` need to
judge "is it a type reference"—all relying on string comparison.

**Proposal: add a `kind` field to `Type::Name`, determined once by the parser.**

```rust
// target form (schematic; line numbers are after change)
Name {
    name: String,
    kind: NameKind,   // new
    span: Span,
}

enum NameKind {
    Builtin,      // built-in type name: Int / Float / Bool / ... → handed to from_builtin_name
    UserType,     // user-declared type name (including std.result / std.option for Generic)
    TypeVar,      // type variable: T / K / V
    Constraint,   // operator interface / constraint name: Add / Ord / ...
    Predicate,    // compile-time predicate: Terminates / Sorted / ...
}
```

**Key design point: the information required to determine `NameKind` is exactly the information the
parser cannot currently obtain** (type environment, declared operator-interface table, declared
predicate table). Therefore this field **cannot be determined independently inside the parser**—it
must be determined by the injected environment. This connects directly to 5.4.

**Disposition of the synonym table**:

`from_builtin_name` (`mono.rs:618-643`) is **retained, but semantically downgraded**. Before the
refactor, it is a disambiguator "bridging two equivalent representations at the AST layer"; after
the refactor, `Type::Int(64)` is deleted, "Int" has only one representation path, and it degenerates
into **the sole name→type resolution table**.

Specific reasons to retain it (cannot delete):

- The comment for `"DateTime" | "datetime" => Some(MonoType::Int(64)` (`mono.rs:627`) records a real
  fix (#338 related issue: `now()`'s return value cannot be passed to `format_time`). This is an
  **alias** semantic, not the same as the case aliases of `Int`/`int`.
- It is the **sole cut-in point** between standard-library type names and language built-in names.

But **the redundancy within the table can be cut**. After the refactor, the parser will only produce
canonical case (`Int` / `Int32` / `Float` / `Bool` / ...), so:

| Category                                           | Current state (`mono.rs:620-642`) | Target                                                                                                                                                                                   |
| -------------------------------------------------- | --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Case aliases (`"int"` / `"i64"` / `"int64"`)       | Retained                          | **Delete**—the parser will not produce these after normalization                                                                                                                         |
| Abbreviation aliases (`"i64"` / `"i32"` / `"f64"`) | Retained                          | **Move to the lexer layer** (registered in the keyword table in `src/frontend/core/lexer/state.rs` as equivalent writings of `Int`); `from_builtin_name` only recognizes canonical names |
| `DateTime` alias                                   | Retained (`mono.rs:627`)          | **Retain**—semantic alias, with comment                                                                                                                                                  |
| `"()"` (`mono.rs:640`)                             | Retained                          | **Delete**—nowhere to produce it after the `Type::Void` fallback (`types.rs:836`) is deleted                                                                                             |

**Net effect**: `from_builtin_name` converges from 12 match arms to about 6 canonical names,
semantics shifts from "disambiguation table" to "resolution table", and **it becomes the single
source of truth for type names** (the gate at 5.7 validates it).

**Merging the two `name_used_as_type*`**: the two duplicate implementations recorded in 2.4 must be
merged into one; otherwise the kind-defining information provided by `NameKind` will be bypassed by
the two independent string-comparison logics. The merged implementation is placed in `parser/ast.rs`
(data source near `Type`), and `declarations.rs` is changed to call it; the `is_predicate`
short-circuit is changed to read `NameKind::Predicate`, and the `Struct` / `NamedStruct` field
recursion takes the version of `ast.rs:871-882` (newer, more complete fix).

**Assignment of `CONST_PARAM_TYPES`**: the const generic parameter name table at `ast.rs:838-841`
partially overlaps with the full set of `from_builtin_name` (see 2.3). After the introduction of
`NameKind::Builtin`, the judgment at `ast.rs:912` can be changed to "`kind == Builtin` and the name
is within the const-parameter-acceptable set", explicitly connecting the two pieces of knowledge;
the subset definition of const parameters is validated by the T2 gate (see 5.7).

> **Design judgment**: if there are type annotations such as `i64` / `int` in the `.yx` corpus,
> normalizing them to `Int` will change the **diagnostic location** of these files (but not the
> diagnostic code). Per the C3 criterion this is allowed (messages and order can be normalized), but
> the baseline needs to be recorded in stage 1.

### 5.4 Data Flow of Predicates / Type Names

**Current data flow** (parser-internal closed loop + 2 boundary-crossing points):

```
ast.rs (extract_generic_param_names, :930)  ─┐
declarations.rs:509                          ─┴─→ typecheck::operator_interfaces::spec()   ✗
declarations.rs
  ├─ declare_predicate()      ← parser_state.rs:46 (held by parser, only declarations resolved in this pass)
  ├─ n == "Terminates"        ← hard-coded literal (declarations.rs:499)
  └─ (the boundary-crossing call above)
```

**Target data flow** (injection, not boundary-crossing):

```
L1 orchestration layer (constructed once at entry)
   │  ① Compile built-in operator-interface table
   │  ② Compile std's .yx declarations → predicate-name set
   ▼
TypeEnvProbe (trait, defined in L2, src/frontend/core/parser/probe.rs)
   │  fn is_predicate(&self, name: &str) -> bool
   │  fn is_constraint(&self, name: &str) -> bool
   ▼
ParserState (parser_state.rs:46/54 extended, changed to delegate to probe)
   ▼
declarations.rs:498-511 and ast.rs:930 changed to
   let is_predicate_app = |n: &str| state.probe().is_predicate(n);
   if state.probe().is_constraint(n) { ... }
```

**Per-file changes**:

| File                                                                                 | Change                                                                                                                                                                                                                                                                                         |
| ------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/probe.rs` (new)                                            | Define `trait TypeEnvProbe`, providing four pure query methods: `is_predicate` / `is_constraint` / `is_type_var` / `is_builtin_type`. **The trait is defined in L2, and the implementation is provided by L3**—this is legitimate dependency inversion                                         |
| `src/frontend/core/parser/parser_state.rs:46-58`                                     | Retain `declare_predicate` / `is_predicate_name` (for accumulation in this pass); add `probe: Box<dyn TypeEnvProbe>` field and `probe()` accessor                                                                                                                                              |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                        | Delete `n == "Terminates"`; change to `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                                                                          |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                        | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; change to `state.probe().is_constraint(n)`                                                                                                                                                                   |
| `src/frontend/core/parser/ast.rs:930`                                                | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` is the definition location of `Type` / `Expr` / `StmtKind`, and should not hold environment dependencies—this function needs to be changed to accept a `&dyn TypeEnvProbe` parameter (see below) |
| All 5 parser construction points (`Parser::new` / `ParserState::new` in `parser.rs`) | Accept the `probe: Box<dyn TypeEnvProbe>` parameter. **The default implementation `NullProbe` (all return false)** ensures parser unit tests are unaffected                                                                                                                                    |

**Signature problem of `extract_generic_param_names`**: the
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` at `ast.rs:891` is a
free function that accepts no context. After deleting the boundary-crossing call at `ast.rs:930`, it
needs the `is_constraint` judgment, so the signature must add `probe: &dyn TypeEnvProbe`. Callers
(including `parser/tests/ast.rs`) synchronously pass `&NullProbe`.

**Timing of `NameKind` determination**: when `types.rs:162-165` constructs `Type::Name`, use
`state.probe()` to determine `kind`, set it once, and all subsequent match arms no longer perform
string comparison.

**This eliminates all 3 entries of RFC-039's routing table C**—per the 2.4 scope, 2 of them only
involve parser-internal functions; the real cross-layer references are `declarations.rs:509` and
`ast.rs:930`, both covered by this section.

### 5.5 `Expr::FnDef` Merging

**Position: delete `Expr::FnDef` (`ast.rs:37-43`), and function definitions uniformly take the
`Expr::Lambda` + `StmtKind::Assign` path.**

**This is the already-running path**: `nud.rs:425-429` + `declarations.rs:462-479` (see 2.6). The
`FnDef` branch is a never-executed parallel implementation.

**Change list** (12 consumption + 2 exhaustive arms, listed one by one):

| Location                             | Current arm                                          | Target                                                                                                                                                 |
| ------------------------------------ | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `spawn/placement.rs:122`             | `Expr::FnDef { body, .. } => self.check_block(body)` | Merge into the existing `Expr::Lambda` arm (`body` is also `Box<Block>`, **directly delete this arm**)                                                 |
| `spawn/analysis.rs:912`              | `Expr::FnDef { body, .. } => { ... }`                | Same as above, merge into Lambda arm                                                                                                                   |
| `formatter/handlers/expr.rs:43`      | `Expr::FnDef { ... }`                                | Change to formatting the Lambda in `Assign`; delete this arm                                                                                           |
| `orchestrator.rs:1492`               | `if let Expr::FnDef { name, .. }`                    | Change to take the name from `StmtKind::Assign { target, .. }`                                                                                         |
| `ir_gen.rs:4325`                     | `ast::Expr::FnDef { span, .. } => *span`             | Merge into Lambda arm (Lambda also carries `span`)                                                                                                     |
| `ir_gen.rs:5097`                     | `\| ast::Expr::FnDef { span, .. }`                   | Delete this or-pattern                                                                                                                                 |
| `checker.rs:1642`                    | `if let ...::Expr::FnDef {`                          | Change to judge `Expr::Lambda`                                                                                                                         |
| `checker/semantic_tokens.rs:1416`    | `Expr::FnDef {`                                      | Delete this arm                                                                                                                                        |
| `inference/expressions.rs:3374`      | `...::Expr::FnDef {`                                 | Change to judge `Expr::Lambda`                                                                                                                         |
| `inference/existential.rs:55`        | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                                 |
| `passes/dead_code.rs:305`            | `Expr::FnDef {`                                      | Change to judge `Expr::Lambda`                                                                                                                         |
| `layers/ownership.rs:1207`           | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                                 |
| `layers/termination.rs:752`          | `Expr::FnDef {`                                      | Change to judge `Expr::Lambda`; the comment at `termination.rs:2303` "`fn(): Never` — FnDef.return_type is the bare return type" updated synchronously |
| `ast.rs:1026`                        | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                                 |
| `pratt/mod.rs:47`                    | `Expr::FnDef { span, .. } => *span`                  | Delete this arm                                                                                                                                        |
| `typecheck/tests/checker.rs:109-111` | Test constructs `Expr::FnDef`                        | Rewrite to `Expr::Lambda` + `Assign`                                                                                                                   |
| `parser/tests/ast.rs:837-847`        | Test constructs `Expr::FnDef`                        | Delete this test                                                                                                                                       |

**Behavior unchanged**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, `span`,
structurally identical to the first three of `FnDef`; the `name` field unique to `FnDef` has no
place to be used at the `Expr` level (the name is on `StmtKind::Assign.target`).

**Risk and mitigation**: non-exhaustive `if let` matches such as `checker.rs:1642`,
`inference/expressions.rs:3374` will become never-matching after the refactor—no error, no warning.
Mitigation: the type-table gate (5.7) adds a "zero-construction variant" check; after `Expr::FnDef`
is deleted, anyone who rewrites `if let Expr::FnDef` will directly fail to compile (the variant does
not exist)—**this is a compile-time guarantee, no gate needed**.

### 5.6 Disposition of `signature_params` and `NamedParen`

**`StmtKind::Assign.signature_params` (`ast.rs:244-245`) → delete.**

Reason: its reason for existence (the comment at `ast.rs:244`) is "for the typechecker's
`classify_generic_params`"—**a dedicated slot reserved by the statement AST for the type checker**.
But `declarations.rs:500-511`'s `extracted_params` has already computed `value_params` (having
completed the type-position/constraint-position filtering) within the same function; passing it down
with the statement is duplicate carrying.

Change: `ast.rs:244-245` delete the field → `declarations.rs:472` delete the argument → the data
source of `classify_generic_params` changes to recomputation in place (it can obtain the
`Lambda.params` from `value: Option<Box<Expr>>`). **Net: one field declaration deleted, one
construction site deleted, one carrying path deleted.**

> **Risk**: `signature_params` is the **unfiltered** raw list, while `Lambda.params` is **filtered**
> (the filter at `declarations.rs:500-511` removes the type and constraint positions). If
> `classify_generic_params` depends on the unfiltered version, the refactor will change behavior.
> **It must be verified in stage 1 which one `classify_generic_params` actually uses**—equivalence
> cannot be assumed.

**`Type::NamedParen` (`ast.rs:533-540`) → retain the node, remove binder semantics.**

The comment at `ast.rs:519-532` records its **sufficient reason for existence** (see 2.7): in
`(r: P(m))`, if `r` is dropped, the type checker can only guess "free variables in constraints not
in scope are return-value formal parameters", and would **silently substitute the undeclared `m` as
a binder** (the comment notes this as a "verified defect").

Therefore **it cannot be simply deleted**—that would reintroduce a previously fixed correctness
defect. The correct disposition is:

1. Retain `NamedParen { param, param_span, inner }` as a **pure syntax node**; `param` /
   `param_span` only carry source-code facts and do not bear type-layer binder semantics.
2. The interpretation right of the binder is transferred to `sema/`: the type checker, when
   consuming `NamedParen`, **explicitly** extracts `param` as the binder. This is "reading a field
   of a syntax node", of the same nature as the `Expr` embedded in `ConstExpr`—the syntax node
   provides the facts, and the semantic layer does the interpretation.
3. The comment is changed from "the type checker can only guess" to "this node provides the binder
   name; the semantic interpretation is in `sema/`".

**Why this is "migration" rather than "deletion"**: the syntax tree must be able to represent source
code losslessly. `r` does exist in the source, and the AST cannot pretend it does not. The problem
is not the existence of the node, but **the responsibility for semantic interpretation not being
explicitly defined**.

### 5.7 Type-Table Generation-Time Gate

**The reference object has been validated in this repository**: `build.rs:19-37` calls
`tools/code-tables`'s `parse_registry` / `validate`, performing uniqueness, segment, and
zh-completeness validation on 145 error codes, and `is_ok()` being false directly `panic!` refuses
to compile; `build.rs:39-54` further compares the RFC-013 markdown code table item by item, and
inconsistency likewise `panic!`. **RFC-013 documents and code are therefore always
consistent**—`tools/code-tables/Cargo.toml:15-16` has only a `serde_json` dependency, and the
validation logic is text-level parsing without depending on `syn`.

The type table currently has **no** similar gate. The form of the mechanism to be added in this
document can be directly modeled on it.

**New crate**: `tools/type-tables` (with `Cargo.toml` depending only on `serde_json`, symmetric with
`code-tables`). Four checks:

| #                                                               | Check                                                                                                                                                                                                                                                                                           | Input                    | Failure condition                                                                                           | Treatment                                                         |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | ----------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| **T1 Zero-construction variants**                               | Scan the `pub enum Type` (and `Expr` / `StmtKind` / `BinOp` / `UnOp`) variant list in `src/frontend/core/parser/ast.rs`; under `src/` count the **construction points** for each variant (excluding `tests/` and `*/tests/*`), **and separately mark reconstruction points in reverse bridges** | Variant list + hit table | A certain variant has **zero construction points** and is not in the explicit whitelist                     | Delete the variant, or move it to the whitelist and explain why   |
| **T2 Synonym table consistent with whole-repository name list** | `from_builtin_name` match table in `types/mono.rs` + `ast.rs:838-843` `CONST_PARAM_TYPES` + LSP built-in type name list in `src/lsp/world.rs:176`                                                                                                                                               | Three tables             | A type name exists in one table but is not recognized in another (compared by each's semantic mapping rule) | `type-tables --fix` bi-directional completion                     |
| **T3 Built-in type names consistent with RFC-011 document**     | Canonical-name set in `from_builtin_name` + type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                                                    | Two tables               | Code-table range / names inconsistent with the registry                                                     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --fix` |
| **T4 `Type → MonoType` exhaustiveness**                         | `Type` variant list vs match arms in `types/lower.rs`                                                                                                                                                                                                                                           | Two tables               | A variant has no corresponding arm, or an arm has no corresponding variant                                  | Add arm / delete arm                                              |

**Implementation notes for T4**: Rust's exhaustive match already guarantees the variant→arm
direction at compile time. T4's value is in the **reverse** direction—detecting **stale arms
pointing to deleted variants** in `lower.rs` (such code already fails to compile when the variant is
deleted, so T4 is effectively redundant). **T4 therefore degrades to a CI assertion (`cargo build`
success passes), and does not enter the `panic!` path of `build.rs`.** T1–T3 are the three that
truly need text parsing.

**Two implementation difficulties of T1 and mitigations**:

1. **Distinguishing construction points from match arms** requires syntactic analysis. The
   repository's root `Cargo.toml` **has no `syn` dependency**, and `tools/code-tables` also only has
   `serde_json`. Mitigation:
   - **Preferred**: introduce `syn` to `tools/type-tables` (only that crate; the root crate is not
     affected). `syn` is the standard approach, and the gate crate is independent of the compiler.
   - **Fallback**: reuse the existing `tools/extract_arm.py` (a Python tool already in the
     repository) line-level heuristic—construction points are on the right side of `= ` or in
     function argument positions, match arms are on the left side of `=>`. Use the `=>` occurrence
     count as an approximation.
   - **Conservative starting point**: when T1 first goes online, **only report, do not fail**
     (`cargo:warning`), take a full baseline once to confirm the heuristic is correct, then change
     to `panic!`. This is consistent with the handling of "the CI gate will be red at the beginning
     and a baseline must be established first".
2. **Reverse-bridge construction points must be categorized separately**. 2.2 records the two
   `MonoType → ast::Type` reconstructions at `bytecode.rs:2353-2390` and `function.rs:507-529`. If
   T1 mixes their reconstruction points with the parser's forward constructions, the 7 variants
   `Int` / `Float` / `Bool` / `Char` / `Void` / `String` / `Bytes` will be judged as "having
   construction points" and exempted from deletion, directly contradicting the disposition table of
   5.2. T1's report must list the two types of construction points separately, and require separate
   explanation of reverse-bridge reconstruction in the whitelist mechanism.

**Gate coverage**: in `build.rs`, add a `type_tables::validate(root, &entries)` call, located right
after the existing block at `build.rs:19-37`, following the
`panic!("Type-table validation failed ({} errors ...)", ...)` form.

## Detailed Design

### 6.1 Type-System Impact

| Affected area                                                | Judgment                                                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact**. Inference consumes `MonoType` throughout, does not touch `ast::Type` variants                                                                                                                                                                                                                                              |
| Solver (`types/solver.rs`)                                   | **No impact**. `pub fn unify` (`solver.rs:411`), `substitute` at `types/substitute.rs:112`, and `substitute` at `types/mono.rs:649` all operate on `MonoType`                                                                                                                                                                             |
| Constant evaluation (`types/eval/`)                          | **No impact**. `eval/const_eval.rs` (1128 lines), `eval/dependent_types.rs` (`check_structural_termination` at `478`), `unify` at `eval/reducer.rs:572` all operate on `MonoType`                                                                                                                                                         |
| Monomorphization (`middle/passes/mono/`)                     | **Needs change**. `mono/function.rs:358`, `362`, `451`, `456`, `554`, `558`, `618`, `623` 8 locations match `AstType::NamedStruct` / `AstType::AssocType`—if `AssocType`'s disposition is decided as deletion, these 8 locations need to be updated. **Also need to change `mono_to_ast_type` at `function.rs:507-529`** (see #47 of 6.3) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Needs change**. `ast::Type::NamedStruct` matches at `ir_gen.rs:1266`, `1399`; `let params: Vec<MonoType> = signature_params` at `ir_gen.rs:1380` (this is another consumption point of `signature_params`; 5.6 must be verified together)                                                                                               |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Needs change**. `type_id_to_monotype` at `codegen/bytecode.rs:492-495`, the encoding loop at `351-352`, the assembly at `632`                                                                                                                                                                                                           |
| Interpreter (`backends/interpreter/`)                        | **Needs change**. `type_table: Vec<ir::Type>` at `image.rs:44` changed to `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, the element-type change will affect the `{:?}` output format                                                                                                                  |
| RFC-027 compile-time types                                   | **Needs change**. `Type::ConstExpr` is retained but marked as **compile-time-only**; it disappears after `Type → MonoType` (`types/lower.rs`) and is never visible at the IR layer                                                                                                                                                        |

**IR-layer type discipline (for 04-ssa to inherit)**: after convergence, IR type annotations can
only take the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only** variants, and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. This discipline should be
enforced by the "type consistency" invariant of 07's first-tier validator. `ast::BinOp::Assign` /
`ast::UnOp::Deref` (2.5) are expression-side forms of the same kind of problem—they can appear
inside `ConstExpr`, so they likewise cannot sink down into IR type annotations.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** Reasons:

1. The 11 deleted variants have no **forward** construction points in production code → no source
   can produce them (for reverse-bridge disposition, see #47/#28 of 6.3).
2. `ir::Type` is an alias of `ast::Type` → changing it back to `MonoType` does not change the type
   set, only the **carrier**.
3. The **loss** of `From<MonoType> for IrType` (8 variants → `Void`) no longer happens after the
   change—this **may** change the behavior of certain programs (types previously lost are now
   preserved). **This is the only direction of behavior change, and it is a fix rather than a
   regression**, but it must be confirmed by 07's third-tier corpus diff that there is no
   regression.

**Semantic change points requiring focused attention**:

| Location                | Current state                                                                                 | After convergence                           |
| ----------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------- |
| `bytecode.rs:2371-2376` | `Generic` that is not `String`/`Bytes`/`Tuple` → `IrType::Void`                               | Retain the real `MonoType::Generic`         |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Retain the real `MonoType`                  |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` fallback                                                                  | **Delete** (exhaustive match is sufficient) |

> **Design judgment**: these 8 variants currently all collapsing to `Void` means **the code paths
> that depend on the signature bytecode (interpreter's arity/type check, serialization format)
> currently see `Void` uniformly**. After convergence these locations will see the real types.
> **This is the only source of possible behavior change in this document, and the direction is
> "restoring correct information"**. If the corpus diff shows differences, it should be considered
> **exposing an existing defect** rather than a regression introduced by this document—verification
> method: separately construct a `.yx` program containing `Ref` parameters, and compare
> `dump_bytecode` before and after the change.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                   | Change                                                                                                                                                                                                                           | Category                 |
| --- | ---------------------------------------------------------------------- | -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472` | Delete `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants                                                                                                                              | Pure deletion            |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                              | `Name` adds `kind: NameKind` field                                                                                                                                                                                               | Data flow                |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43`                                | Delete `Expr::FnDef`                                                                                                                                                                                                             | Pure deletion            |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                              | Delete `Assign.signature_params`                                                                                                                                                                                                 | Pure deletion            |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026`                                 | Delete `Expr::FnDef` or-pattern                                                                                                                                                                                                  | Follows #3               |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                              | `NamedParen` comment changed to "syntax provides binder name, semantic interpretation in `sema/`"                                                                                                                                | Documentation            |
| 7   | `src/frontend/core/parser/ast.rs`                                      | after `795-814`                        | Add `pub enum NameKind`                                                                                                                                                                                                          | New addition             |
| 8   | `src/frontend/core/parser/probe.rs`                                    | new                                    | `trait TypeEnvProbe` + `NullProbe`                                                                                                                                                                                               | New addition             |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                | Add `probe` field and accessor                                                                                                                                                                                                   | Data flow                |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                              | Use `state.probe()` to determine `NameKind`                                                                                                                                                                                      | Data flow                |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                              | Comment update (`Result`/`Option` take `Generic`, already consistent with fact; only supplement variant-deletion note)                                                                                                           | Documentation            |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                  | Delete the `_ => (Vec::new(), Type::Void)` fallback, change to explicit error return                                                                                                                                             | Behavior                 |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                              | Delete `n == "Terminates"`, change to `state.probe().is_predicate(n)`                                                                                                                                                            | Reverse dependency       |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                              | Delete `typecheck::operator_interfaces::spec()` call                                                                                                                                                                             | Reverse dependency       |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                  | Delete `signature_params` argument                                                                                                                                                                                               | Follows #4               |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                   | Delete `Expr::FnDef` arm                                                                                                                                                                                                         | Follows #3               |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                  | Retain (is the parameter-list fishback point; with 5.5 changed to directly read Lambda)                                                                                                                                          | Follows #3               |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814` | Delete 11 match arms                                                                                                                                                                                                             | Follows #1               |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                              | Cut synonyms (case aliases, `i64`-type abbreviations, `"()"`)                                                                                                                                                                    | Convergence              |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                  | `DateTime` alias retained                                                                                                                                                                                                        | Retain                   |
| 21  | `src/frontend/core/types/lower.rs`                                     | new                                    | Migrate `From<Type> for MonoType` (currently in `mono.rs:692-830`) as the sole conversion point                                                                                                                                  | New addition             |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                          | Register `i64`/`int` etc. as equivalent writings of `Int`                                                                                                                                                                        | Convergence              |
| 23  | `src/middle/core/ir.rs`                                                | `3`                                    | Delete `pub use ... ast::Type`                                                                                                                                                                                                   | Pure deletion            |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                  | `FunctionCode.params` retain `Vec<MonoType>` (already is)                                                                                                                                                                        | —                        |
| 25  | `src/middle/core/bytecode.rs`                                          | `812`, `814`                           | `BytecodeFunction.params` / `return_type` change to `MonoType`                                                                                                                                                                   | Convergence              |
| 26  | `src/middle/core/bytecode.rs`                                          | `854`                                  | `BytecodeModule.type_table` change to `Vec<MonoType>`                                                                                                                                                                            | Convergence              |
| 27  | `src/middle/core/bytecode.rs`                                          | `2339`                                 | `type_table` conversion changed to `.collect()` (elements already same type)                                                                                                                                                     | Follows #26              |
| 28  | `src/middle/core/bytecode.rs`                                          | `2352-2390`                            | **Delete** the entire `From<MonoType> for IrType` impl                                                                                                                                                                           | Pure deletion            |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                   | `type_table` change to `Vec<MonoType>`                                                                                                                                                                                           | Convergence              |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 locations)              | `AstType::AssocType` arm handled per gate conclusion                                                                                                                                                                             | TBD                      |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | new / after `build.rs:19-37`           | Type-table gates T1–T3                                                                                                                                                                                                           | Gate                     |
| 32  | `spawn/placement.rs`                                                   | `122`                                  | Delete `Expr::FnDef` arm                                                                                                                                                                                                         | Follows #3               |
| 33  | `spawn/analysis.rs`                                                    | `912`                                  | Same as above                                                                                                                                                                                                                    | Follows #3               |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                   | Same as above                                                                                                                                                                                                                    | Follows #3               |
| 35  | `orchestrator.rs`                                                      | `1492`                                 | Same as above, change to take name from `Assign.target`                                                                                                                                                                          | Follows #3               |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                         | Same as above                                                                                                                                                                                                                    | Follows #3               |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                 | Same as above                                                                                                                                                                                                                    | Follows #3               |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                 | Same as above                                                                                                                                                                                                                    | Follows #3               |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                 | Same as above                                                                                                                                                                                                                    | Follows #3               |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                   | Same as above                                                                                                                                                                                                                    | Follows #3               |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                  | Same as above                                                                                                                                                                                                                    | Follows #3               |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                 | Same as above                                                                                                                                                                                                                    | Follows #3               |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                          | Same as above + comment update                                                                                                                                                                                                   | Follows #3               |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                              | Delete `Expr::FnDef` test                                                                                                                                                                                                        | Test                     |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                              | Rewrite to Lambda + Assign                                                                                                                                                                                                       | Test                     |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`        | Delete 11 dead variants' lower assertions                                                                                                                                                                                        | Test                     |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                              | `mono_to_ast_type` delete `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches, uniformly take `AstType::Name` + `mono.type_name()`. **Must be processed in the same batch as #1**                              | Convergence              |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                           | `extract_generic_param_names` signature add `probe: &dyn TypeEnvProbe`; delete `typecheck::operator_interfaces::spec()` call, change to `probe.is_constraint(name)`. Callers (including `parser/tests/ast.rs`) pass `&NullProbe` | Reverse dependency       |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                | Delete the `name_used_as_type` in this file, change to call the merged version in `ast.rs`; the `is_predicate` short-circuit changed to read `NameKind::Predicate`                                                               | Duplicate implementation |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                  | `CONST_PARAM_TYPES` judgment changed to wire via `NameKind::Builtin` (see 5.3)                                                                                                                                                   | Convergence              |

> **The gate itself must also be gated**: `tools/type-tables` and `code-tables` likewise need `mod`
> declaration wiring (this repository has the lesson of 8 orphan test trees). When creating a new
> crate, register it synchronously in the root `Cargo.toml`.

### 6.4 Backward Compatibility Strategy

**Constraint**: the 293 `.yx` corpus in `tests/yaoxiang/` (single-file) + the multi-file corpus
layer built in P2 (`tests/yaoxiang-multifile/`) + `src/std/tests/*.yx` + the standard library **must
all have unchanged behavior** (diagnostic codes and messages identical, order can be normalized).

| Risk                                                             | Judgment                                                                                                                                                                                                                                                                                                                      | Mitigation                                                                                                                                                                                                                                                                  |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source code writes type annotations like `i64` / `int` / `f64`   | After normalization to `Int`, `from_builtin_name` only recognizes canonical names                                                                                                                                                                                                                                             | **First thing in stage 1**: scan the full corpus to count non-canonical type-name occurrences. If 0, synonyms can be deleted directly; if non-zero, per 5.3 move to the lexer alias table (**syntax layer accepts, lower layer normalizes**, no impact on diagnostic codes) |
| Source code writes `DateTime`                                    | Alias retained (`mono.rs:627`)                                                                                                                                                                                                                                                                                                | No impact                                                                                                                                                                                                                                                                   |
| `("()")` appearing in type position                              | The `LParen` branch from `types.rs:167` handles tuples/parameter groups, producing `Tuple` rather than `Name{"()"}`                                                                                                                                                                                                           | Before deleting `"()"`, first count the occurrences of `"()"` as a type name in the full corpus                                                                                                                                                                             |
| `dump_bytecode` output change                                    | The element type of `type_table` changes from `ir::Type` to `MonoType`, the `{:?}` format differs                                                                                                                                                                                                                             | **C3 criterion allows** (07 specifies `dump_bytecode` is only compared in the C1/C2 stage). But the output diff needs to be recorded in stage 2 for manual review                                                                                                           |
| Behavior change after lossy-bridge fix                           | See 6.2                                                                                                                                                                                                                                                                                                                       | Corpus diff + targeted `dump_bytecode` comparison                                                                                                                                                                                                                           |
| Type-substitution result change after `mono_to_ast_type` rewrite | `function.rs:507-529` is the exit of monomorphization type substitution (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back to `AstType`). After deleting variants, if the `_` arm is taken, the substitution result changes from `AstType::Int(64)` to something like `AstType::Name { name: "i64" }` | Stage 3 targeted comparison: select corpus with generic substitution, compare the `type_name()` of monomorphization products before and after the change. **Must not assume `{:?}` output equivalence**                                                                     |
| `.yx` source itself                                              | **Zero changes**                                                                                                                                                                                                                                                                                                              | This document does not require changing any corpus or std source files. This is a hard acceptance item                                                                                                                                                                      |

**Hard acceptance condition**: `git diff --stat tests/ src/std/` must be empty (except for new
additions in the multi-file corpus layer and test baseline files).

## Implementation Notes

The criterion uniformly adopts 07's **C3: diagnostic codes and messages identical (order can be
normalized)**. Below is the stage partition for this document's line; the cross-document global
sequence relationship is in RFC-039.

| Stage                                         | Content                                                                                                                                                                                                                                         | Acceptance criterion                                                                                                                                                                                                                       | Rollback point                                    |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------- |
| **0. Gate first**                             | New `tools/type-tables`, implement T1/T2/T3, T1's first version **only reports, does not fail**. Take a full baseline on **unmodified** code                                                                                                    | T1 report accurately lists 13 zero-construction variants (including the qualitative conclusion for `AssocType`), and **lists separately the reverse-bridge reconstruction points recorded in 2.2**; T2/T3 all green or give a fixable list | No code change, no rollback needed                |
| **1. Name normalization**                     | Count non-canonical type names in the corpus; `from_builtin_name` (`mono.rs:620-642`) cuts synonyms, moves them to the lexer alias table. **Synchronously verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`** | C3: full-corpus diagnostic set **identical item-by-item** to the stage-0 baseline                                                                                                                                                          | Single commit, revertible                         |
| **2. Bytecode type unification**              | Change `bytecode.rs:812`/`814`/`854`, `image.rs:44` to `MonoType`; delete `From<MonoType> for IrType` (`bytecode.rs:2352-2390`)                                                                                                                 | C3 + targeted `dump_bytecode` comparison (6.2 table item by item)                                                                                                                                                                          | Single commit, revertible                         |
| **3. Dead variant deletion**                  | Delete 11 variants per the 5.2 table; clear `mono.rs:699-708` etc. match arms, the `types.rs:836` fallback; **process `function.rs:507-529` (#47) in the same batch**; synchronize `types/tests/mono.rs`                                        | C3 + `cargo build` all green (exhaustive match forces all arms to be completed) + generic-substitution targeted comparison (last line of 6.4)                                                                                              | Single commit, revertible                         |
| **4. Parser data flow**                       | Add `probe.rs`; change `parser_state.rs:46-58`; change `declarations.rs:498-511` and `ast.rs:930` (#48); `types.rs:162-165` determines `NameKind`; merge the two `name_used_as_type*` (#49)                                                     | C3 + **new assertion**: `grep 'typecheck' src/frontend/core/parser/` should be 0                                                                                                                                                           | Single commit, revertible                         |
| **5. AST dead variants and dedicated fields** | Delete `Expr::FnDef` (16 locations) + `Assign.signature_params`; `NamedParen` semantic migration                                                                                                                                                | C3 + `tests/integration/` 18 modules all green                                                                                                                                                                                             | Split into two commits (FnDef / signature_params) |
| **6. Gate hardens**                           | T1 changes from warning to `panic!`; T4 degrades to CI assertion                                                                                                                                                                                | `cargo build` **must fail** when a zero-construction variant is intentionally introduced (red test first)                                                                                                                                  | Single commit, revertible                         |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Stages 3 and 5 must be after stage
2**—`bytecode.rs:2372-2373` consumes `IrType::String` / `IrType::Bytes`; deleting the variants first
would break compilation. Stage 3's #47 must be in the same batch as #1, otherwise `mono_to_ast_type`
silently changes behavior.

**Dependency on downstream**: after all stages of this document are complete, [04-ssa.md](04-ssa.md)
can begin. At that point the element types of `BytecodeFunction.params`, `FunctionCode.params`, and
`type_table` are consistent, and the SSA type annotations have a unique drop-point.

### Mandatory Verification per Stage

| Stage | Must execute                                                                                                                                                     |
| ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`, manually verify T1's report (**including reverse-bridge reconstruction point separation**) |
| 1-5   | `cargo test`; full corpus diff on `tests/yaoxiang/` (07 third tier); C3 criterion item-by-item comparison                                                        |
| 2     | Additionally: construct a `.yx` sample for each variant in the 6.2 table, `dump_bytecode` before and after for comparison                                        |
| 3     | Additionally: select corpus with generic substitution, compare the `type_name()` of monomorphization products before and after the change                        |
| 4     | Additionally: `rg 'typecheck' src/frontend/core/parser/` returns 0                                                                                               |
| 6     | Additionally: intentionally write a zero-construction variant, confirm `cargo build` fails (red→green)                                                           |

## Key Decisions and Rationale

### Decisions

| Decision                          | Decision                                                                                                                                        | Rationale                                                                                                                                                                                                                                                         |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Form of the target representation | `Type` as the sole type structure + `MonoType` as the working form; `ir::Type` alias deleted                                                    | The alias at `ir.rs:3` is the direct cause of the field-type inconsistency between `BytecodeFunction.params` and `FunctionCode.params` (2.1). After the alias is deleted, the two sets of field types unify, and the `From` bridge loses its reason for existence |
| Reject "unify into `MonoType`"    | Not adopted                                                                                                                                     | Would cause L2→L3 reverse dependency (violating RFC-039 layering), and would require adding `Span` / `ConstExpr` back to `MonoType`. Full argument in 5.1                                                                                                         |
| `NamedParen` not deleted          | Retain as a pure syntax node, migrate binder semantics to `sema/`                                                                               | `ast.rs:519-532` records that deletion would reintroduce a previously fixed correctness defect: `(r: P(m))` would silently substitute the undeclared `m` as a binder                                                                                              |
| Directory renaming                | **Do it** (D1), execute as a pure move batch at the end of P5/P6; the body of this document is written using the pre-rename current state paths | Decision on 2026-10-03 (ChenXu233); construction method supplemented on 2026-10-04                                                                                                                                                                                |

### Rejected Alternatives

Four categories of alternatives were not adopted: **retaining three representations and only adding
a conversion layer** solves none of the root causes, and `ir::Type` is still not an independent
type; the dead variants, synonym table, and reverse dependency would not disappear; **using macros
to generate the synonym table** treats the symptom, not the cause—after `Type::Int(usize)` is
deleted the macros have no reason to exist, and the gate T2 takes over consistency guarantees (more
verifiable than the macro's expansion correctness); **only deleting dead variants without unifying**
has the lowest cost but leaves the problem in place, with no gate to prevent the same dead variants
from re-growing when someone adds `Type::Int128(usize)` (if only one commit can be made, it should
be clearly noted as a subset of that plan and T1 should be delivered along with it); **unify into
`MonoType`** see 5.1.

### Benefits

- **Eliminates the maintenance burden of 14 locations**. The 12 consumption points + 2 exhaustive
  arms of `Expr::FnDef` are a never-executed parallel path; after deletion, `ir_gen.rs` /
  `ownership.rs` / `termination.rs` all lose one branch.
- **Eliminates one already-diverged duplicate implementation**. The two copies of
  `name_used_as_type` (2.4) have already diverged on field recursion inside the definition body;
  after merging, only one remains.
- **After the lossy bridge disappears, `IrType::Void` is no longer a "legal type value"**. This is a
  prerequisite for 07's first-tier validator's "type consistency" invariant to actually pass green.
- **Gates make regression impossible**. T1 turns "adding a zero-construction variant" from currently
  passing silently to a build failure—this repository's existing `build.rs` mechanism proves this
  path is viable.
- **Zero source-code changes**. 293 `.yx` corpus entries and the std library do not need any
  modification; this is the fundamental reason the C3 criterion can hold.

## Known Limitations and Risks

### Risks

- **Stage 2 may have behavior changes**. After the lossy bridge is fixed, 8 `MonoType` variants will
  now see the real type rather than `Void`. The direction is a fix, but it will change the paths
  that depend on the signature bytecode. This is the document's greatest uncertainty.
- **The qualitative determination of `AssocType` depends on the accuracy of the gate**. T1, if it
  uses a line-level heuristic (`=>` count) rather than `syn`, may misjudge. This is why stage 0 must
  be manually verified.
- **T1's omission of reverse bridges would make the disposition table and the gate contradict each
  other**. 2.2 has proven that 7 "dead variants" have production reconstruction points in reverse
  bridges. If T1 does not separate the construction-point direction, these 7 would be judged as
  "having construction points" and exempted from deletion—the gate would become an obstacle to the
  deletion work rather than a guarantee.
- **`signature_params` deletion has semantic risk**. It carries the **unfiltered** parameter list,
  while `Lambda.params` is **filtered**. If `classify_generic_params` depends on the former, the
  refactor will change the generic-classification result. **It must be verified first, equivalence
  cannot be assumed** (the `signature_params` consumption point on the ir_gen side at
  `ir_gen.rs:1380` must be verified together).
- **Stage 3 touches `middle/passes/mono/function.rs`**. Besides the 8 `AssocType` matches, the
  `mono_to_ast_type` at `507-529` is the exit of monomorphization type substitution; after
  rewriting, the `type_name()` of the substitution result may change (last line of 6.4).
- **The non-canonical type names in the `.yx` corpus may be non-zero**. If the corpus uses `i64` /
  `int` extensively, stage 1's "directly delete synonyms" is not feasible and degenerates to "move
  to the lexer alias table", reducing the convergence magnitude of `from_builtin_name`.
- **The gate will be red at first launch**. If T1's first launch directly `panic!`, it will
  immediately block compilation. It must first only report to take a baseline.

### Undecided Issues

> **All open questions originally listed in this section have been decided.** See the
> [RFC-039 Decision Registry](../../rfc/draft/039-compiler-architecture.md) (D1–D50) for
> item-by-item decisions. **This document leaves no pending items.**

## See Also

### Upper-Level and Same-Batch Documents

- [RFC-039 Compiler Functionality Routing Catalog Design (Master Document)](../../rfc/draft/039-compiler-architecture.md)
  — upper-level master document; four-layer model, criteria tiering, routing table C (3
  type-representation-related entries assigned to this document)
- [01-routing.md](01-routing.md) — Functionality routing tables A/B/C, dependency-direction
  specification, target directory structure
- [04-ssa.md](04-ssa.md) — SSA-ification; this document's prerequisite and downstream consumer
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — Frontend paradigm; operator change-surface
  convergence
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C3 category definition, diagnostic-set
  comparison specification, the type-consistency invariant of the first-tier IR validator

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — Basis for the
  unified path of `Type::Generic`
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — Built-in type table
  (comparison object of gate T3)
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Example
  of the generation-time gate in `build.rs`
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Source of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

- `src/frontend/core/parser/ast.rs:427-541` — Definition of `ast::Type`'s 26 variants
- `src/frontend/core/parser/ast.rs:519-540` — `Type::NamedParen`'s binder-semantics description
  (evidence that deletion would reintroduce a defect)
- `src/frontend/core/parser/ast.rs:241-249` — `StmtKind::Assign.signature_params`
  typechecker-specific field
- `src/frontend/core/parser/ast.rs:838-841` — `CONST_PARAM_TYPES`: the third copy of const generic
  parameter names
- `src/frontend/core/parser/ast.rs:843-885` — `name_used_as_type_in`: duplicate implementation of
  `declarations.rs:42` (already diverged)
- `src/frontend/core/parser/ast.rs:891`, `930` — `extract_generic_param_names` and its
  boundary-crossing call
- `src/frontend/core/parser/ast.rs:191-213`, `217-223` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — Parser represents all primitive types as
  `Type::Name`
- `src/frontend/core/parser/statements/types.rs:392-396` — `Result`/`Option` not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:836` — The only forward construction point of
  `Type::Void` (error fallback)
- `src/frontend/core/parser/statements/declarations.rs:42-80` — Original `name_used_as_type`
- `src/frontend/core/parser/statements/declarations.rs:494-511` — Hard-coded `"Terminates"` and
  `typecheck::operator_interfaces::spec()` call
- `src/frontend/core/parser/pratt/nud.rs:505-514` — Expression position represented as empty-body
  `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:466` — `expr_to_params` arm fishes the parameter list back
- `src/frontend/core/types/mono.rs:618-643` — `from_builtin_name` synonym table
- `src/frontend/core/types/mono.rs:692-830` — `From<Type> for MonoType` implementation (12 of 13
  match arms are here)
- `src/frontend/core/types/mono.rs` — `MonoType` definition (about 22 variants, from `mono.rs:167`;
  1077-line file)
- `src/frontend/core/types/const_data.rs:234-258`, `321-330` — `const_data::BinOp` /
  `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — Compile-time constant evaluation (1128 lines)
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- `src/middle/core/ir.rs:3` — Evidence of `ir::Type` as an alias of `ast::Type`
- `src/middle/core/bytecode.rs:808-814` — `BytecodeFunction`'s `Vec<ir::Type>` fields
- `src/middle/core/bytecode.rs:2352-2390` — Lossy `From<MonoType> for IrType` bridge
- `src/middle/passes/codegen/bytecode.rs:32`, `275-278` — `FunctionCode`'s `Vec<MonoType>` fields
- `src/middle/passes/mono/function.rs:507-529` — `mono_to_ast_type`: reverse bridge that
  reconstructs 7 "dead variants"
- `src/backends/interpreter/image.rs:44` — Interpreter module's `type_table`
- `build.rs:19-37` — Error-code registry generation-time gate (form reference for the type-table
  gate)
- `build.rs:39-54` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — Gate crate depends only on `serde_json` (root crate has no
  `syn`; `tools/type-tables` introducing `syn` is the first exception)
