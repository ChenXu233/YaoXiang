# Type Representation Unification

> **Subsidiary design document**. This document is subsidiary to
> [RFC-039 Compiler Architecture Refactor](../../rfc/draft/039-compiler-architecture.md). The
> four-layer model, acceptance criterion grading, and execution stage ordering are covered in the
> RFC-039 main text; the positioning of each subsidiary document is given in
> [this directory's index](index.md).

## Positioning and Scope

### Coverage

Converge the currently parallel multiple type representations into one set, and eliminate the four
classes of burden arising therefrom:

- **13 production-zero-construction `ast::Type` variants** (of the 26 variants in `ast.rs:427-541`,
  see 2.2)
- **A handwritten synonym table for type names** (`mono.rs:618-643`, see 2.3)
- **parser's real reverse dependency on the typecheck layer** (2 locations, see 2.4)
- **A production-zero-construction expression variant** `Expr::FnDef` (`ast.rs:37-43`, see 2.6)

The concrete deliverables are four items: a per-variant disposition table for the 26 variants (5.2),
`NameKind` for semantic typing of `Type::Name` and elimination of the synonym table (5.3), the
`TypeEnvProbe` injection-style data flow (5.4), and the type-table build-time gate T1-T3 (5.7).

### Directory Rename and This Document's Path

RFC-039 decision D1 determines that the directory rename (`typecheck/`→`sema/`,
`middle/core/`→`middle/ir/`) is to be performed, **completed together with P5/P6, not split into
stages**. This document's 50-item change list and all acceptance greps are **written using
pre-rename current paths**—they are the construction baseline for P6 batches **before the rename
batch**; the directory rename at the end of each batch is a **pure relocation batch** (C1
zero-diff), committed separately. The baseline becoming invalid after the rename commit is expected
behavior; use git history as the bisection basis.

### Out of Scope

- **The construction method for the directory rename**. `Type` is not moved out of `parser/ast.rs`
  at this stage; only `lower.rs` is newly added under `types/` as the sole conversion point. The
  directory rename (`typecheck/`→`sema/`, etc.) is executed together with the P5/P6 wrap-up per
  RFC-039 decision D1; the form is given by the target directory structure in `01-routing.md`.
- **The four-layer model and dependency direction specification**. See
  [01-routing.md](01-routing.md).
- **The grading of equivalence criteria and validator implementation**. The definitions of C1-C6,
  the three layers of criteria (IR validator / canonicalization snapshot / corpus diffing) are in
  [07-equivalence-oracle.md](07-equivalence-oracle.md). This document only references **C3 (type
  representation convergence)** as its own acceptance criterion.
- **SSA-ization**. See [04-ssa.md](04-ssa.md). This document is its prerequisite, for the reason
  given below.
- **The P1-P10 global execution sequence**. See the RFC-039 main text and
  [02-stage-contract.md](02-stage-contract.md). This document only gives the stage breakdown for its
  own line (see "Implementation Points").

### Division of Labor with RFC-039

RFC-039 is the upper-level master plan: the four-layer model, criterion grading, cross-document
stage ordering, and "why do this refactor now". This document is the **construction blueprint for
the type-representation-convergence line** in RFC-039: which variants to delete, which files to
change, in what order, and what gates prevent regression.

All line numbers, variant lists, and construction-point statistics in this document are derived from
static verification of `src/`, with each item citing `file:line`. **This document records only code
facts and the dispositions derived from them, not the execution process.**

For ease of cross-reference with sibling documents, the "Target Design" and "Detailed Design"
chapters use the `5.x` / `6.x` subsection numbering; the other chapters follow the unified format of
this directory.

### Why This Must Precede SSA-ization

`ir::Type` is not a third independent type, but a `pub use` alias of `ast::Type` (`ir.rs:3`).
[04-ssa.md](04-ssa.md) introduces register type annotations and new instructions to the IR; without
prior convergence of the representation, the SSA type annotations will, via the `ir::Type` alias,
grow directly on the 26-variant syntactic types, effectively solidifying a third representation.

The capability boundary of the current `ir::Type` is pinned by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) embeds a complete expression tree into the type.
  An `Expr` appearing in an IR instruction's type annotation is impossible.
- `ast::Type::Literal { name, base_type }` (`ast.rs:476-482`) carries the source-level name and
  `Span`, the syntactic form of a compile-time literal type, not a runtime type.

Doing SSA type annotation on top of the 26 variants amounts to permanently solidifying these 13 dead
variants and two syntax-specific variants into the IR contract.

## Current State

All of the following are factual code statements, without proposals. Each item cites `file:line` as
evidence.

### 2.1 Three Parallel Representations and the Lossy Bridge

The multiplicity of type representations is the common upstream of four independent defects:

| Downstream symptom                                                             | Causal relationship to the type representation                                                                                                                                                                                                                                                                                                               |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| The same type text has multiple legal AST forms                                | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent yet type-different**, the parser chose the latter, and typecheck must supplement a table to translate the former back                                                                                      |
| The bytecode-layer signature and IR-layer signature are inconsistent           | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only obtain the alias, so `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`) while `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) has its `params` declared as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding a built-in type name requires changes in multiple places, with no check | Because the name→type mapping is scattered between `from_builtin_name` (`src/frontend/core/types/mono.rs:618-643`) and the string match in `bytecode.rs:2371-2376`, each written twice and mutually unverified                                                                                                                                               |
| The parser hard-codes `"Terminates"`                                           | Because the parser cannot obtain the judgment "is this a type" or "is this an operator interface", it is forced to embed type-layer knowledge as string literals into the syntax layer (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                           |

**The first three are representation problems; the fourth is a downstream side-effect of the
representation problem.** Doing only item 1 (deleting dead variants) leaves the synonym table and
the reverse dependency; doing only item 4 (injecting callbacks into the parser) builds on a
still-inconsistent representation.

One of the three representations is an alias illusion:

| Name        | Definition location                       | Lines       | Role                                                    |
| ----------- | ----------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541` | 26 variants | Syntax-layer type                                       |
| `MonoType`  | `src/frontend/core/types/mono.rs`         | 1077 lines  | Semantic-layer type (after monomorphization)            |
| `ir::Type`  | `src/middle/core/ir.rs:3`                 | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

`ir.rs:3` is a single `pub use` line; `ir.rs:6` then has
`use crate::frontend::core::typecheck::MonoType;`. The IR module depends on two "type" symbols
simultaneously, but one of them is only an alias.

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

All 8 `MonoType` variants collapse to `IrType::Void`, plus an `_ => IrType::Void` catch-all
(`bytecode.rs:2386-2387`, whose comment admits it is "a transitional branch before deleting enum
variants").

`IrType::Void` here is **a legal type value, not an error signal**. Therefore downstream cannot
distinguish "this function actually returns `Void`" from "this function's return type was lost
during bridging"—this is one reason why the "type consistency" invariant of the first-layer
validator in 07 cannot truly pass green on the current code.

Meanwhile, the `IrType::String` / `IrType::Bytes` references point to `ast::Type::String`
(`ast.rs:435`) and `ast::Type::Bytes` (`ast.rs:436`), which are exactly the dead variants to be
deleted in 2.2—**both ends of the lossy bridge point to dead code**.

### 2.2 Production-Zero-Construction Variants

**Criterion**: The construction-point scan range is the entirety of `src/`, excluding `tests/` and
`*/tests/*`. Among the 26 variants of `ast::Type`, the following 13 have **only match arms, no
construction points, in production code**:

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

The root-cause chain is fully auditable:

1. **The parser represents all primitive types as `Type::Name { name }`**—the fallback arm at
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered to dedicated nodes**—the comment at `types.rs:392-396`
   explicitly states: "RFC-010: Result/Option are not lowered to dedicated AST nodes… they take the
   same `Type::Generic` path as user-defined generic types. The type representation remains
   `Generic{"Result"/"Option", args}`."
3. **`Type::Void`'s sole construction point is an error catch-all**—`_ => (Vec::new(), Type::Void)`
   at `types.rs:836`, located at the end of a `match` returning `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction points: `types.rs:364`, `types.rs:571`, `types.rs:100`, and
`declarations.rs:573`; they are not dead variants.

**These 13 variants have construction points in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
Therefore this document's wording is "**production-zero-construction**" rather than
"global-zero-reference". These tests themselves are the solidification of the behavior under
test—they assert exactly "what these syntactic forms can be lowered to as a `MonoType`", and these
forms are never produced by the parser. When deleting variants, these tests must be deleted or
rewritten together.

#### Criterion Boundary: The Reverse Bridge Reconstructs 7 of Them

The "sole construction point / no construction point" in the table above only holds in the **parser
→ AST forward-construction** direction. There are two production-level `MonoType → ast::Type`
reconstructions in the reverse direction, which **actually construct 7 of the variants in the
table**:

| Reverse bridge                                                 | Location                                 | 13-variant members reconstructed                                                                                         |
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
    // #299: Container types like String/Bytes are now Generic, type_name is reverse-resolved to AstType
    MonoType::Generic { name, .. } if name == "String" => AstType::String,
    MonoType::Generic { name, .. } if name == "Bytes" => AstType::Bytes,
    _ => AstType::Name { name: mono.type_name(), span },
}
```

**This criterion boundary has a direct effect on the disposition**:

- The `bytecode.rs` one disappears along with item #28 in 6.3 (deleting the entire
  `From<MonoType> for IrType` impl), incurring no extra cost.
- `function.rs:507-529` is **not within the coverage of any existing change item**. It is a
  `MonoType → AstType` type-name reverse resolver (comment labeled `#299`), reverse-resolving
  `String` / `Bytes` from `Generic` back to dedicated variants by name. After deleting
  `ast::Type::String` / `Bytes`, these two guard branches must be rewritten; after deleting `Int` /
  `Float` / `Bool` / `Char` / `Void`, the behavior of the `_` arm will change (some types that
  originally fell into `_` will silently switch to `AstType::Name`). This is the supplementary item
  #47 in 6.3.
- **Direct consequence for the gate**: a T1 that only scans forward construction points will judge
  the above 7 variants as "having construction points" and thereby exclude them from the deletable
  list. T1 must explicitly include the reverse bridge in its counting scope, otherwise the gate's
  conclusion will contradict this document's disposition table (see 5.7).

### 2.3 The Handwritten Synonym Table

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:618-643`) uses a string table to map
type names to `MonoType`:

```
"Int" | "int" | "Int64" | "int64" | "i64"  => Some(MonoType::Int(64)),   // :620
"DateTime" | "datetime"                     => Some(MonoType::Int(64)),   // :627
"Int32" | "int32" | "i32"                   => Some(MonoType::Int(32)),   // :628
...
"Void" | "void" | "()"                      => Some(MonoType::Void),      // :640
```

**The sole reason this table exists is to bridge the AST-layer inconsistency**: because
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent yet type-different at
the AST layer (and the parser only produces the latter), `from_builtin_name` must accept case
variants, abbreviations, signed names, and the `DateTime` alias simultaneously. The fact that `"()"`
appears in the `Void` row is especially telling—it cooperates with the `Type::Void` catch-all at
`types.rs:836`.

The same "which names are built-in type names" knowledge exists in four copies in the repository,
with no mutual consistency check:

| #   | Location                              | Form                                                                                                                              |
| --- | ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `mono.rs:618-643` `from_builtin_name` | String match, name → `MonoType`                                                                                                   |
| 2   | `bytecode.rs:2371-2376`               | String match, copying three: `"String"` / `"Bytes"` / `"Tuple"`                                                                   |
| 3   | `ast.rs:838-843` `CONST_PARAM_TYPES`  | 15 const generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`) as `&[&str]` constants |
| 4   | `src/lsp/world.rs:176`                | The LSP-side list of built-in type names                                                                                          |

Item 3, `CONST_PARAM_TYPES`, is used by `extract_generic_param_names` at `ast.rs:912` to determine
const generic parameters—it enumerates "type names usable as const parameters", which is **partially
overlapping but non-identical** with the full set of `from_builtin_name`, and there is no mechanism
ensuring the two stay in sync. Item 4 is the fourth copy for LSP syntax highlighting/hover, likewise
unchecked (T2's comparison scope includes it).

### 2.4 parser's Reverse Dependency on the Type Layer

RFC-039 routing table C records 3 entries (`declarations.rs:27-80` calls `is_type_param_annotation`
/ `name_used_as_type`, and these two functions belong to the type layer). **The actual measurement
differs from that wording**: these two functions are defined inside the parser itself, not in the
type layer.

- `is_type_param_annotation` is defined in
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the sole `fn` definition in the repo)
- `name_used_as_type` is defined in `declarations.rs:42-80` (the sole `fn` definition in the repo)
- `declare_predicate` / `is_predicate_name` are defined in
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also within the parser

The production code that actually cross-layer-references `typecheck` totals **2 locations**, both
calling the same function `crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Containing function                        | Role                                                                                   |
| --- | --------------------------------------------------------- | ------------------------------------------ | -------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Closure for filtering signature parameters | Determines constraint-position formals (`T: Add`), not occupying runtime parameters    |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`              | Determines constraint-position formals, produces `GenericParamName` with `constraints` |

Item 2 is further inwards—**`parser/ast.rs` itself** (the entire block at `ast.rs:847-948`, same
file as `StmtKind`/`Expr`) directly references the typecheck symbol. Routing table C only recorded
the one in `declarations.rs`.

It constitutes, together with the hard-coding at `declarations.rs:498-499`, the same judgment:

```rust
// declarations.rs:498-499
// Extended as subsequent built-in predicates are added (future open-set plan: pass predicate names down to
// the parser, or change to a syntax-layer-determinable form).
// The parser does not hold a type environment, so it looks at two places: hard-coded built-in predicates,
// plus the accumulated declarations already parsed in this pass (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment admits this is a temporary form for the "future open-set plan". `n == "Terminates"` is
**type-layer knowledge embedded as a string literal in the syntax layer**.

**Why this is a downstream effect of the type-representation problem**: the parser needs to ask "is
this an operator interface name" because it must distinguish whether `Type::Name` refers to a type
or to an interface constraint. **`Type::Name { name: String }` carries at least 4 semantics in a
single undifferentiated string** (concrete type / type variable / constraint name / predicate name),
the syntax layer cannot determine on its own, and the determination authority is forced to be
uplifted to the type layer.

#### Duplicated Implementation: Same Name, Same Meaning, Behavior Already Forked

`name_used_as_type` has two implementations. The comment at `ast.rs:843-846` admits this:

```rust
/// Whether the parameter name is treated as a type reference in the given type (`(N: Int) -> (n: N)`'s `N`).
///
/// Synonymous with `name_used_as_type` in `declarations.rs`; independently implemented here to avoid
/// cross-module dependencies within the parser.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations **have already forked**:

| Dimension               | `declarations.rs:42-80` `name_used_as_type`                                    | `ast.rs:847-885` `name_used_as_type_in`              |
| ----------------------- | ------------------------------------------------------------------------------ | ---------------------------------------------------- |
| Signature               | Takes `is_predicate: &dyn Fn(&str) -> bool` callback                           | No callback                                          |
| `Type::Generic` branch  | Short-circuits returning `false` via `is_predicate(app_name)` first (`:54-56`) | No such short-circuit; recurses into `args` directly |
| `Type::Struct { body }` | **None** (falls into `_ => false`)                                             | **Has it** (`:871-879`, traverses `Field` / `Expr`)  |
| `Type::NamedStruct`     | **None**                                                                       | **Has it** (`:880-882`, traverses `fields`)          |

The comment at `ast.rs:868-870` records the origin of the latter two branches: "Previously missing
these two branches, when a const generic parameter was referenced inside the definition body, it
could not be detected as 'used as a type', and annotation validation would mistakenly report a legal
const parameter as unknown name."

In other words: **the function copied to avoid cross-module dependencies had its fix not flow back
to the original.** The `ast.rs` version has field recursion inside the definition body; the
`declarations.rs` version does not—the same source code produces different conclusions at the two
determination points. This is the duplication that must be merged when introducing `NameKind` in
5.3.

### 2.5 Parallel Operator Enums

There are two copies of the operator enum, with no mutual reference and **the members have already
forked**:

| Concept          | Syntax layer                    | Constant-evaluation layer                                             | Difference                                                                                                                      |
| ---------------- | ------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Binary operators | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | Syntax layer has unique `Neq` (`:197`), `Range` (`:205`), `Assign` (`:206`); constant layer uses `Ne` (`:243`) instead of `Neq` |
| Unary operators  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | Syntax layer has unique `Deref` (`:222`); constant layer has unique `BitNot` (`:329`)                                           |

The two copies also each implement complete `Display` (`const_data.rs:291-317`) and classification
predicates (`is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise` at
`const_data.rs:260-289`).

**Why this is part of the type-representation problem**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) are **not types**—when they appear in a type position expression
they fall into `Type::ConstExpr` (`ast.rs:506`), i.e., the "compile-time-only variant". This relates
directly to the IR-layer type discipline in 6.1: after convergence, `Type::ConstExpr` disappears
after `Type → MonoType` and is never visible to IR, while `Assign` / `Deref` continue to exist via
paths like `ir_gen.rs`'s `BinOp::Assign => Ok(MonoType::Void)` at `inference/expressions.rs:875`.
The fork in the two enums makes "which operators can appear in a type position" undecidable from
either side alone.

This stage does not merge these two enums (their ownership is in
[05-frontend-paradigm.md](05-frontend-paradigm.md)'s "operator change surface"), but T1's gate scan
target should include `ast::BinOp` / `ast::UnOp`, making "newly-added zero-construction operators"
equally visible.

### 2.6 `Expr::FnDef`: A Production-Zero-Construction Expression Variant

`Expr::FnDef` (`ast.rs:37-43`) being **production-zero-construction** is true—there are only two
constructions in the entire repo: `src/frontend/core/parser/tests/ast.rs:837` and
`src/frontend/core/typecheck/tests/checker.rs:111`, both in tests.

But **the consumption points far exceed 4**. The actual production consumption points are 12:

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

There are additionally 2 forced exhaustive match arms: `ast.rs:1026` (`Expr::span()`) and
`pratt/mod.rs:47` (`expr_end_line`).

**The actual path function definitions take is**: `nud.rs:425-429` constructs `Expr::Lambda` +
`declarations.rs:462-479` constructs `StmtKind::Assign` (putting `Lambda` into `value`). In other
words, the `FnDef` branch is **a parallel path that never executes but is maintained by 14
locations**.

### 2.7 Syntax Nodes Storing Semantics on Behalf of the Type Layer

**First, `StmtKind::Assign` carries a typechecker-specific field.** The `Assign` variant at
`ast.rs:241-249` contains `signature_params: Vec<Param>` (`ast.rs:244-245`), whose comment says
outright:

> `/// The first group of signature parameters as-is (including parameter names), for the typechecker's classify_generic_params`

The statement AST reserves a dedicated slot for the typechecker. Construction point:
`declarations.rs:472`.

**Second, an expression position is used as a parameter list.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a Lambda **with an
empty body**. Subsequently, the `Expr::Lambda { params, .. } => Some(params.clone())` arm at
`src/frontend/core/parser/pratt/led.rs:466` "fishes it back" as a parameter list.

That is: a normal variant of the AST is used as a **temporary parameter-list carrier**, then
restored to semantics by a match arm in another module.

**Third, `Type::NamedParen` carries the return-position binder name.** The comment at
`ast.rs:519-540` explicitly says:

> The return-position refinement of RFC-027 §3 depends on it declaring **return formal parameter
> names** (called binders here)… **without it, the typechecker can only guess** that "free variables
> in the constraint not in scope are return-value formals", so `(r: P(m))` would also silently
> substitute undeclared `m` as a binder (verified defect).

The syntax node is preserving binder identity on behalf of the type layer.

## Target Design

### 5.1 Form of the Target Representation

**Position: a single representation = `Type` (originally `ast::Type`) as the only type structure;
`MonoType` demoted to the typechecker's internal working form; the `ir::Type` alias deleted.**

Three representations converge into **two roles**, but only one of them is "the type":

```
Sole type structure  Type  ──────────────────────┐
   (26 → 13 variants, living in types/ rather than parser/ast.rs) │
                                                       │  From<Type> for MonoType  (total, no semantic loss)
                                                       │  ← sole conversion point types/lower.rs
Typechecker working form  MonoType  ──────────────┘
   (with TypeVar / substitution state, flowing only within typecheck)
   ×
ir::Type alias deleted, BytecodeFunction.params / type_table changed to use MonoType
```

**Why not "merge into `MonoType`"** (this is the most easily proposed solution, and is rejected
here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.), while
   `MonoType` does not. C3's criterion in 07 is "same diagnostic codes and messages"—the
   diagnostic's location information must be traceable back to the source. Adding `Span` to
   `MonoType` would make post-monomorphization types carry a pile of meaningless sentinel spans.
2. `MonoType` contains `TypeVar` and substitution state (`types/substitute.rs:112`, `pub fn unify`
   at `types/solver.rs:411`). These are **intermediate states of the typechecker's solving
   process**, not types per se. Putting them in the sole type structure would require the IR
   construction layer to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027), and are lowered away in `MonoType`. Unifying into `MonoType`
   means having to add these syntactic forms back to it.
4. **The most critical reverse argument**: elevating `MonoType` to the sole representation would
   make the L2 syntax layer depend on the L3 semantic layer's type. RFC-039 explicitly prohibits L2
   from depending on L3. One of this document's goals is to eliminate reverse dependencies; choosing
   a solution that would cause a reverse dependency is self-contradictory.

**Why `ir::Type` must be deleted directly rather than "preserved"**:

The alias at `ir.rs:3` makes `BytecodeFunction.params` (`bytecode.rs:812`) and `return_type`
(`bytecode.rs:814`) use `ir::Type`, while on the serialization side `codegen/bytecode.rs:277-278`
uses `MonoType`. The two field types are connected by the lossy `From` bridge at
`bytecode.rs:2339`'s `file.type_table.into_iter().map(|t| t.into())` and `bytecode.rs:2353-2390`.

**The sole reason the `From` bridge exists is "the two field types differ"**. Changing
`bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, `image.rs:44` to uniformly use `MonoType`
removes the bridge's reason for existence, and the entire impl can be deleted (also eliminating the
reverse reconstructions recorded in 2.2).

**Explicitly not done**: `Type` is not moved from `parser/ast.rs` to `sema/types/`. This stage only
adds `lower.rs` under `types/` as the sole conversion point, and the definition location of `Type`
is not touched; the directory rename is executed as an independent relocation batch at the P6
wrap-up per D1—at that time, `Type`'s home is the top-level `ast/type_.rs` (AST top-level domain),
and `sema/types/` is the home of the `MonoType` working form.

### 5.2 Per-Variant Disposition Table for the 26 Variants

> Disposition categories: **Delete** (production-zero-construction), **Retain** (has production
> construction point), **Migrate** (semantics moved to another node).

| #   | Variant                                                            | Definition line  | Disposition                                | Reason                                                                                                                                                                                                                                                                           |
| --- | ------------------------------------------------------------------ | ---------------- | ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Retain** (strengthen)                    | The parser's **sole** primitive type exit (`types.rs:162-165`). After deleting the 12 primitive type variants, all primitive types go through it. A "semantic kind of the name" annotation needs to be added (see 5.3)                                                           |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                                 | Production-zero-construction (sole arm at `mono.rs:699`). Bit width is carried by the `Name` path from `from_builtin_name`                                                                                                                                                       |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                                 | Production-zero-construction (`mono.rs:700`)                                                                                                                                                                                                                                     |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                                 | Production-zero-construction (`mono.rs:701`)                                                                                                                                                                                                                                     |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                                 | Production-zero-construction (`mono.rs:702`). **Note**: `bytecode.rs:2372`'s `IrType::String` consumes it; after deletion, that arm must be rewritten                                                                                                                            |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                                 | Production-zero-construction (`mono.rs:703-706`). Same as above with `bytecode.rs:2373` consuming                                                                                                                                                                                |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                                 | Production-zero-construction (`mono.rs:707`)                                                                                                                                                                                                                                     |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                                 | Sole production construction is the error catch-all `types.rs:836`. That catch-all is eliminated together with the error-path rework in 5.6                                                                                                                                      |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Retain**                                 | Has production consumption (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                              |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Retain**                                 | Has production construction point `types.rs:364`                                                                                                                                                                                                                                 |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                                 | Production-zero-construction (`mono.rs:743-746`)                                                                                                                                                                                                                                 |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                                 | Production-zero-construction (`mono.rs:747`). Enums go through `Struct` + `TypeBodyItem`, unrelated to `mono.rs:2378`'s `MonoType::Enum(_) => IrType::Void`                                                                                                                      |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Retain**                                 | Consumed at `bytecode.rs:2374`                                                                                                                                                                                                                                                   |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:451-454` | **Retain**                                 | Consumed at `bytecode.rs:2366`                                                                                                                                                                                                                                                   |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                                 | Production-zero-construction (`mono.rs:770`). `types.rs:392-396` already indicates it takes the `Generic` path                                                                                                                                                                   |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                                 | Production-zero-construction (`mono.rs:771`). Same as above                                                                                                                                                                                                                      |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Retain**                                 | The actual representation of `Result` / `Option` / `String` / `Bytes` all go through it (`types.rs:397-399`)                                                                                                                                                                     |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                                 | Production-zero-construction (the sole construction in the repo is in test `types/tests/mono.rs:178`; the syntax layer has no `::` path producing it). If the associated-type syntax is enabled in the future, it will be redefined by a new proposal at that time (decision D8) |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                                 | Production-zero-construction (`mono.rs:814`)                                                                                                                                                                                                                                     |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Retain**                                 | Has production construction point `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                         |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Retain**                                 | Raw pointer type within unsafe blocks                                                                                                                                                                                                                                            |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Retain**                                 | Borrowing marker                                                                                                                                                                                                                                                                 |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Retain**                                 | Has production construction points `types.rs:100`, `declarations.rs:573`; RFC-010                                                                                                                                                                                                |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Retain (and mark as compile-time-only)** | Has production construction point `types.rs:160`. The only variant in `Type` that embeds `Expr`; prohibited in the IR layer (see 6.1)                                                                                                                                            |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Retain**                                 | RFC-004 currying layer terminator, `split_curry` depends on its existence                                                                                                                                                                                                        |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                                | Semantics (binder name) returned to the typechecker, AST retains only syntax. See 5.6 for details                                                                                                                                                                                |

**Net effect: 26 → 14 variants** (delete 12, migrate 1, retain 13).

> **The reverse bridge must be handled before deleting variants**: the `function.rs:507-529`
> recorded in 2.2 will reconstruct 7 of items #2-#8 in this table. In terms of deletion order, it
> must be processed in the same batch as #1-#8, otherwise that function's `_` arm will silently
> change behavior (see #47 in 6.3).

> **`AssocType` handling**: per D8, directly delete (item #18 in the 5.2 table). The T1 gate (5.7)'s
> first report on **unchanged code must independently reproduce this conclusion**—if the report
> shows `AssocType` as having a production construction point, it means this section's verification
> is wrong; **the correct action is to return to the RFC decision table to modify D8 with an
> explanation**, not to keep the variant in the whitelist.

### 5.3 `NameKind` and Synonym-Table Elimination

**Problem**: `Type::Name { name: String }` carries at least 4 semantics in a single undifferentiated
string—concrete type / type variable / constraint name (operator interface) / predicate name.
`declarations.rs:498-499` needs to determine "is it a predicate", `declarations.rs:508-511` needs to
determine "is it an operator interface", `ast.rs:930` needs to determine "is it an operator
interface", and the two `name_used_as_type*` at `declarations.rs:42-80` and `ast.rs:847-885` need to
determine "is it a type reference"—all relying on string comparison.

**Proposal: add a `kind` field to `Type::Name`, typed once by the parser**.

```rust
// Target form (illustrative; line numbers are post-change)
Name {
    name: String,
    kind: NameKind,   // newly added
    span: Span,
}

enum NameKind {
    Builtin,      // Built-in type name: Int / Float / Bool / ... → handed to from_builtin_name
    UserType,     // User-declared type name (including std.result / std.option for Generic)
    TypeVar,      // Type variable: T / K / V
    Constraint,   // Operator interface / constraint name: Add / Ord / ...
    Predicate,    // Compile-time predicate: Terminates / Sorted / ...
}
```

**Key design point: the information needed to determine `NameKind` is exactly what the parser cannot
currently obtain** (type environment, declared operator-interface table, declared predicate table).
Therefore this field **cannot be determined within the parser independently**—it must be determined
by the injected environment. This connects directly to 5.4.

**Synonym table disposition**:

`from_builtin_name` (`mono.rs:618-643`) **is retained, but semantically downgraded**. Before the
change, it is a disambiguator that "bridges two equivalent representations at the AST layer"; after
the change, `Type::Int(64)` has been deleted, "Int" has only one representation path, and it demotes
to **the sole name→type resolution table**.

Concrete reasons to retain it (cannot be deleted):

- The comment on `"DateTime" | "datetime" => Some(MonoType::Int(64)` (`mono.rs:627`) records a real
  fix (issue #338-related: the return value of `now()` could not be passed into `format_time`). This
  is an **alias** semantic, not the same thing as `Int`/`int` case aliases.
- It is the **sole intersection point** between standard library type names and language built-in
  names.

But the **redundancy within the table can be cut**. After the change, the parser will only produce
canonical case (`Int` / `Int32` / `Float` / `Bool` / ...), so:

| Category                                           | Current state (`mono.rs:620-642`) | Target                                                                                                                                                                           |
| -------------------------------------------------- | --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Case aliases (`"int"` / `"i64"` / `"int64"`)       | Retain                            | **Delete**—not produced after parser normalization                                                                                                                               |
| Abbreviation aliases (`"i64"` / `"i32"` / `"f64"`) | Retain                            | **Move to the lexer layer** (registered in `src/frontend/core/lexer/state.rs`'s keyword table as equivalent writings of `Int`); `from_builtin_name` only accepts canonical names |
| `DateTime` alias                                   | Retain (`mono.rs:627`)            | **Retain**—semantic alias, with comment                                                                                                                                          |
| `"()"` (`mono.rs:640`)                             | Retain                            | **Delete**—after the `Type::Void` catch-all (`types.rs:836`) is deleted, there is nowhere to produce it                                                                          |

**Net effect**: `from_builtin_name` converges from 12 match arms to about 6 canonical names; its
semantics shifts from "disambiguator" to "resolver", and it **becomes the single source of truth for
type names** (gate-verified by 5.7).

**Merging the two `name_used_as_type*`**: the two duplicated implementations recorded in 2.4 must be
merged into one; otherwise the kind-typing information provided by `NameKind` will be bypassed by
two independent string-comparison logics. The merged implementation is placed in `parser/ast.rs`
(data source near `Type`); `declarations.rs` is changed to call it; the `is_predicate` short-circuit
is changed to read `NameKind::Predicate`; and the `Struct` / `NamedStruct` field recursion is taken
from the `ast.rs:871-882` version (newer, more complete fix).

**Ownership of `CONST_PARAM_TYPES`**: the const generic parameter name table at `ast.rs:838-841`
partially overlaps with the full set of `from_builtin_name` (see 2.3). After `NameKind::Builtin` is
introduced, the determination at `ast.rs:912` can be changed to "kind == Builtin and the name is in
the const-parameter-acceptable set", explicitly wiring the two pieces of knowledge together; the
const-parameter subset definition is verified by the T2 gate (see 5.7).

> **Design judgment**: if the `.yx` corpus contains type annotations written as `i64` / `int`,
> normalizing them to `Int` will change the **diagnostic location** of these files (not the
> diagnostic code). Per the C3 criterion, this is allowed (messages and order can be normalized),
> but the baseline must be recorded at stage 1.

### 5.4 Data Flow for Predicates / Type Names

**Current data flow** (closed loop within the parser + 2 cross-boundary calls):

```
ast.rs (extract_generic_param_names, :930)  ─┐
declarations.rs:509                          ─┴─→ typecheck::operator_interfaces::spec()   ✗
declarations.rs
  ├─ declare_predicate()      ← parser_state.rs:46 (parser-owned, only declarations parsed in this pass)
  ├─ n == "Terminates"        ← hard-coded string literal (declarations.rs:499)
  └─ (the above cross-boundary calls)
```

**Target data flow** (injection, no cross-boundary):

```
L1 orchestration layer (construct once at the entry)
   │  ① Compile the built-in operator-interface table
   │  ② Compile the std .yx declarations → predicate name set
   ▼
TypeEnvProbe (trait, defined in L2, src/frontend/core/parser/probe.rs)
   │  fn is_predicate(&self, name: &str) -> bool
   │  fn is_constraint(&self, name: &str) -> bool
   ▼
ParserState (extended at parser_state.rs:46/58, delegating to probe)
   ▼
declarations.rs:498-511 and ast.rs:930 changed to
   let is_predicate_app = |n: &str| state.probe().is_predicate(n);
   if state.probe().is_constraint(n) { ... }
```

**Per-file changes**:

| File                                                                                 | Change                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/probe.rs` (new)                                            | Define `trait TypeEnvProbe`, providing four pure-query methods: `is_predicate` / `is_constraint` / `is_type_var` / `is_builtin_type`. **The trait is defined in L2, the implementation is provided by L3**—this is a legitimate dependency inversion                                    |
| `src/frontend/core/parser/parser_state.rs:46-58`                                     | Retain `declare_predicate` / `is_predicate_name` (for in-pass accumulation); add `probe: Box<dyn TypeEnvProbe>` field and `probe()` accessor                                                                                                                                            |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                        | Delete `n == "Terminates"`; change to `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                                                                   |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                        | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; change to `state.probe().is_constraint(n)`                                                                                                                                                            |
| `src/frontend/core/parser/ast.rs:930`                                                | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` is the definition location of `Type` / `Expr` / `StmtKind` and should not hold environment dependencies—this function needs to change to take a `&dyn TypeEnvProbe` parameter (see below) |
| All 5 parser construction points (`Parser::new` / `ParserState::new` in `parser.rs`) | Accept a `probe: Box<dyn TypeEnvProbe>` parameter. **Default implementation `NullProbe` (returns false for all)** ensures parser unit tests are unaffected                                                                                                                              |

**Signature issue with `extract_generic_param_names`**:
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` at `ast.rs:891` is a
free function that takes no context. After deleting the cross-boundary call at `ast.rs:930`, it
needs the `is_constraint` determination, so its signature must add `probe: &dyn TypeEnvProbe`.
Callers (including `parser/tests/ast.rs`) are correspondingly changed to pass `&NullProbe`.

**`NameKind` determination timing**: when `types.rs:162-165` constructs `Type::Name`, use
`state.probe()` to determine `kind`, pin it once, and no subsequent match arm does string
comparison.

**This eliminates all 3 entries of RFC-039 routing table C**—of which, per the 2.4 criterion, 2
entries only involve parser-internal functions, and the actual cross-layer reference is the two
locations `declarations.rs:509` and `ast.rs:930`, both covered by this section.

### 5.5 Merging `Expr::FnDef`

**Position: delete `Expr::FnDef` (`ast.rs:37-43`), and unify function definitions under
`Expr::Lambda` + `StmtKind::Assign`.**

**This is the already-running path**: `nud.rs:425-429` + `declarations.rs:462-479` (see 2.6). The
`FnDef` branch is a parallel implementation that never executes.

**Change list** (12 consumption + 2 exhaustive arms, listed individually):

| Location                             | Existing arm                                         | Target                                                                                                                                              |
| ------------------------------------ | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `spawn/placement.rs:122`             | `Expr::FnDef { body, .. } => self.check_block(body)` | Merge into the existing `Expr::Lambda` arm (`body` is also `Box<Block>`, **directly delete this arm**)                                              |
| `spawn/analysis.rs:912`              | `Expr::FnDef { body, .. } => { ... }`                | Same as above, merge into the Lambda arm                                                                                                            |
| `formatter/handlers/expr.rs:43`      | `Expr::FnDef { ... }`                                | Change to formatting the Lambda within `Assign`; that arm is deleted                                                                                |
| `orchestrator.rs:1492`               | `if let Expr::FnDef { name, .. }`                    | Change to take the name from `StmtKind::Assign { target, .. }`                                                                                      |
| `ir_gen.rs:4325`                     | `ast::Expr::FnDef { span, .. } => *span`             | Merge into the Lambda arm (Lambda also carries `span`)                                                                                              |
| `ir_gen.rs:5097`                     | `\| ast::Expr::FnDef { span, .. }`                   | Delete this or-pattern                                                                                                                              |
| `checker.rs:1642`                    | `if let ...::Expr::FnDef {`                          | Change to `Expr::Lambda`                                                                                                                            |
| `checker/semantic_tokens.rs:1416`    | `Expr::FnDef {`                                      | Delete this arm                                                                                                                                     |
| `inference/expressions.rs:3374`      | `...::Expr::FnDef {`                                 | Change to `Expr::Lambda`                                                                                                                            |
| `inference/existential.rs:55`        | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                              |
| `passes/dead_code.rs:305`            | `Expr::FnDef {`                                      | Change to `Expr::Lambda`                                                                                                                            |
| `layers/ownership.rs:1207`           | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                              |
| `layers/termination.rs:752`          | `Expr::FnDef {`                                      | Change to `Expr::Lambda`; the comment at `termination.rs:2303` "fn(): Never — FnDef.return_type is the bare return type" is updated correspondingly |
| `ast.rs:1026`                        | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                              |
| `pratt/mod.rs:47`                    | `Expr::FnDef { span, .. } => *span`                  | Delete this arm                                                                                                                                     |
| `typecheck/tests/checker.rs:109-111` | Test constructs `Expr::FnDef`                        | Rewrite as `Expr::Lambda` + `Assign`                                                                                                                |
| `parser/tests/ast.rs:837-847`        | Test constructs `Expr::FnDef`                        | Delete this test                                                                                                                                    |

**No behavior change**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, `span`,
isomorphic to the first three of `FnDef`; `FnDef`'s unique `name` field has no use at the `Expr`
layer (the name is on `StmtKind::Assign.target`).

**Risk and mitigation**: `checker.rs:1642`, `inference/expressions.rs:3374`, and similar
**non-exhaustive** `if let` matches become never-matching after the change—no error, no warning.
Mitigation: the type-table gate (5.7) adds a "zero-construction variant" check; after `Expr::FnDef`
is deleted, if someone rewrites `if let Expr::FnDef`, it will fail to compile directly (variant does
not exist), **this is a compile-time guarantee, no gate required**.

### 5.6 Disposition of `signature_params` and `NamedParen`

**`StmtKind::Assign.signature_params` (`ast.rs:244-245`) → Delete.**

Reason: its reason for existence (the comment at `ast.rs:244`) is "for the typechecker's
`classify_generic_params`"—**a dedicated slot reserved by the statement AST for the typechecker**.
But `declarations.rs:500-511`'s `extracted_params` has already computed `value_params` (having
completed the type-position/constraint-position filter) within the same function; passing it down
with the statement is duplicated transport.

Change: `ast.rs:244-245` delete field → `declarations.rs:472` delete the actual argument →
`classify_generic_params`'s data source is changed to recompute in place (it can obtain
`Lambda.params` within `value: Option<Box<Expr>>`). **Net delete: one field declaration, one
construction, one transport path.**

> **Risk**: `signature_params` is the **unfiltered, raw** list, while `Lambda.params` is **already
> filtered** (the filter at `declarations.rs:500-511` removes the type position and constraint
> position). If `classify_generic_params` depends on the unfiltered version, the change will alter
> behavior. **It must be verified in stage 1 which copy `classify_generic_params` actually uses**;
> equivalence cannot be assumed.

**`Type::NamedParen` (`ast.rs:533-540`) → Retain the node, move the binder semantics out.**

The comment at `ast.rs:519-532` records the **sufficient reason** for its existence (see 2.7): if
`r` is dropped in `(r: P(m))`, the typechecker can only guess "free variables in the constraint not
in scope are return-value formals", and will **silently substitute the undeclared `m` as a binder**
(labeled as a "verified defect" in the comment).

Therefore it **cannot be simply deleted**—that would reintroduce a fixed correctness defect. The
correct disposition is:

1. `NamedParen { param, param_span, inner }` is retained as a **pure syntax node**;
   `param`/`param_span` only carry source-level facts, and do not take on the type-layer binder
   semantics.
2. The binder's interpretation authority is handed to `sema/`: the typechecker, when consuming
   `NamedParen`, **explicitly** extracts `param` as the binder. This is "reading a syntax node's
   field", same nature as `ConstExpr`'s embedded `Expr`—the syntax node provides the fact, the
   semantic layer does the interpretation.
3. The comment is changed from "the typechecker can only guess" to "this node provides the binder
   name, semantic interpretation is in `sema/`".

**Why this is a "migration" rather than a "deletion"**: the syntax tree must be able to losslessly
represent the source code. `r` does exist in the source code, and the AST cannot pretend it does
not. The problem is not that the node exists, but that **the responsibility for semantic
interpretation has not been clearly defined**.

### 5.7 Type-Table Build-Time Gate

**The reference object has been verified in this repo**: `build.rs:19-37` calls
`tools/code-tables`'s `parse_registry` / `validate` to perform uniqueness, segment, and
zh-completeness checks on 145 error codes; when `is_ok()` is false it directly `panic!`s and refuses
to compile; `build.rs:39-54` further compares each entry against RFC-013's markdown code table, and
likewise `panic!`s on inconsistency. **The RFC-013 document and code are therefore always
consistent**—`tools/code-tables/Cargo.toml:15-16` has only a `serde_json` dependency, and the
validation logic is text-level parsing, not depending on `syn`.

The type table currently has **no** such gate. The form of the mechanism to be added in this
document can be reused directly.

**New crate**: `tools/type-tables` (`Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). Four checks:

| #                                                           | Check                                                                                                                                                                                                                                                                                               | Input                    | Failure condition                                                                                            | Treatment                                                         |
| ----------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------- |
| **T1 Zero-construction variants**                           | Scan the variant list of `pub enum Type` (and `Expr` / `StmtKind` / `BinOp` / `UnOp`) in `src/frontend/core/parser/ast.rs`; count each variant's **construction points** under `src/` (excluding `tests/` and `*/tests/*`), **and separately flag the reconstruction points in the reverse bridge** | Variant list + hit table | Some variant is **zero-construction** and not in the explicit whitelist                                      | Delete the variant, or move it to the whitelist with a reason     |
| **T2 Synonym table consistent with whole-repo name list**   | `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` in `ast.rs:838-843` + the LSP built-in type name list in `src/lsp/world.rs:176`                                                                                                                                            | Three tables             | Some type name exists in one table but not recognized by another (compared by each semantic's mapping rules) | `type-tables --fix` to fill in both directions                    |
| **T3 Built-in type names consistent with RFC-011 document** | The canonical name set of `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                                         | Two tables               | Code-table intervals/names inconsistent with the registry                                                    | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --fix` |
| **T4 Exhaustive `Type → MonoType`**                         | `Type` variant list vs. match arms in `types/lower.rs`                                                                                                                                                                                                                                              | Two tables               | Some variant has no corresponding arm, or some arm has no corresponding variant                              | Add arm / delete arm                                              |

**T4 implementation note**: Rust's exhaustive `match` already guarantees the "variant → arm"
direction at compile time. T4's value is in the **reverse** direction—detecting **stale arms
pointing to deleted variants** in `lower.rs` (such code fails to compile when the variant is
deleted, so T4 is in fact redundant). **T4 is therefore downgraded to a CI assertion (passed when
`cargo build` succeeds), not entering the `panic!` path of `build.rs`.** T1-T3 are the three that
truly need text parsing.

**Two implementation difficulties and mitigations for T1**:

1. **Distinguishing construction points from match arms** requires syntactic analysis. The root
   `Cargo.toml` has **no `syn` dependency**, and `tools/code-tables` likewise only has `serde_json`.
   Mitigation:
   - **Preferred**: add `syn` to `tools/type-tables` (only this crate, the root crate is untouched).
     `syn` is the standard practice, and the gate crate is independent of the compiler.
   - **Fallback**: reuse the existing `tools/extract_arm.py` (a pre-existing Python tool in the
     repo)'s line-level heuristics—construction points are on the right side of `= ` or in the
     function-argument position, match arms are on the left side of `=>`. Use the count of `=>`
     occurrences as an approximation.
   - **Conservative starting point**: when T1 first goes online, it **only reports, does not fail**
     (`cargo:warning`), takes one full baseline to confirm the heuristic is correct, then changes to
     `panic!`. This is consistent with "the CI gate will go red at the beginning and the baseline
     must be established first".
2. **The reverse bridge's construction points must be categorized separately**. 2.2 records two
   `MonoType → ast::Type` reconstructions at `bytecode.rs:2353-2390` and `function.rs:507-529`. If
   T1 mixes their reconstruction points with the parser's forward construction, the 7 variants `Int`
   / `Float` / `Bool` / `Char` / `Void` / `String` / `Bytes` will be judged as "having construction
   points" and exempted from deletion, directly contradicting the 5.2 disposition table. T1's report
   must list the two classes of construction points separately, and require the whitelist mechanism
   to separately explain reverse-bridge reconstructions.

**Gate coverage**: add a `type_tables::validate(root, &entries)` call in `build.rs`, placed right
after the existing block at `build.rs:19-37`, reusing the
`panic!("Type-table validation failed ({} items error ...)", ...)` form.

## Detailed Design

### 6.1 Type-System Impact

| Impact surface                                               | Judgment                                                                                                                                                                                                                                                                                                                                                      |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact**. inference consumes `MonoType` throughout, does not touch `ast::Type` variants                                                                                                                                                                                                                                                                  |
| Solver (`types/solver.rs`)                                   | **No impact**. `pub fn unify` (`solver.rs:411`), `substitute` at `types/substitute.rs:112`, `substitute` at `types/mono.rs:649` all operate on `MonoType`                                                                                                                                                                                                     |
| Constant evaluation (`types/eval/`)                          | **No impact**. `eval/const_eval.rs` (1128 lines), `eval/dependent_types.rs` (`check_structural_termination` at `478`), `unify` at `eval/reducer.rs:572` all on `MonoType`                                                                                                                                                                                     |
| Monomorphization (`middle/passes/mono/`)                     | **Needs changes**. `mono/function.rs:358`, `362`, `451`, `456`, `554`, `558`, `618`, `623`—8 locations match `AstType::NamedStruct` / `AstType::AssocType`—if the disposition of `AssocType` is determined to be deletion, these 8 locations need to be updated in sync. **Also need to change `mono_to_ast_type` at `function.rs:507-529`** (see #47 in 6.3) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Needs changes**. `ir_gen.rs:1266`, `1399`'s `ast::Type::NamedStruct` match; `ir_gen.rs:1380`'s `let params: Vec<MonoType> = signature_params` (this is another consumption point of `signature_params`, and needs to be verified together in 5.6)                                                                                                           |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Needs changes**. `codegen/bytecode.rs:492-495`'s `type_id_to_monotype`, the encoding loop at `351-352`, the assembly at `632`                                                                                                                                                                                                                               |
| Interpreter (`backends/interpreter/`)                        | **Needs changes**. `image.rs:44`'s `type_table: Vec<ir::Type>` changed to `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, the element type change will affect `{:?}`'s output format                                                                                                                                        |
| RFC-027 compile-time types                                   | **Needs changes**. `Type::ConstExpr` is retained but marked as **compile-time only**; after `Type → MonoType` (`types/lower.rs`), it disappears and is never visible at the IR layer                                                                                                                                                                          |

**IR-layer type discipline (to be inherited by 04-ssa)**: after convergence, the IR's type
annotations can only take the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal`
/ `Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only** variants and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. This discipline should be
enforced by the "type consistency" invariant of the first-layer validator in 07.
`ast::BinOp::Assign` / `ast::UnOp::Deref` (2.5) are the same kind of issue on the expression
side—they can appear within `ConstExpr`, and therefore likewise must not be sunk into the IR's type
annotations.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** Basis:

1. The 11 deleted variants have no **forward** construction points in production code → no source
   code can produce them (reverse-bridge disposition: see #47/#28 in 6.3).
2. `ir::Type` is an alias of `ast::Type` → changing it back to `MonoType` does not change the type
   set, only the **carrier**.
3. The **loss** of `From<MonoType> for IrType` (8 variants → `Void`) no longer occurs after the
   change—**this may** change the behavior of certain programs (the previously lost types are now
   retained). **This is the only direction of behavior change, and it is a fix rather than a
   regression**, but it must be confirmed by the 07 third-layer corpus diffing as having no
   regression.

**Semantic change points requiring focused attention**:

| Location                | Current                                                                                       | After convergence                      |
| ----------------------- | --------------------------------------------------------------------------------------------- | -------------------------------------- |
| `bytecode.rs:2371-2376` | `Generic` other than `String`/`Bytes`/`Tuple` → `IrType::Void`                                | Retain the real `MonoType::Generic`    |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Retain the real `MonoType`             |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` catch-all                                                                 | **Delete** (exhaustive match suffices) |

> **Design judgment**: the fact that all 8 variants currently collapse to `Void` means that **the
> code paths depending on signature bytecode (the interpreter's arity/type check, serialization
> format) currently all see `Void`**. After convergence, these locations will see the real type.
> **This is the only potential source of behavior change in this document, and the direction is
> "restoring correct information"**. If corpus diffing produces differences, it should be treated as
> **exposing a pre-existing defect** rather than a regression introduced by this document—the
> verification method: separately construct a `.yx` program with `Ref` parameters, and
> `dump_bytecode` before and after the change for comparison.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                   | Change                                                                                                                                                                                                                                | Category                  |
| --- | ---------------------------------------------------------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472` | Delete `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants                                                                                                                                   | Pure deletion             |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                              | `Name` adds `kind: NameKind` field                                                                                                                                                                                                    | Data flow                 |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43`                                | Delete `Expr::FnDef`                                                                                                                                                                                                                  | Pure deletion             |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                              | Delete `Assign.signature_params`                                                                                                                                                                                                      | Pure deletion             |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026`                                 | Delete `Expr::FnDef` or-pattern                                                                                                                                                                                                       | With #3                   |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                              | `NamedParen` comment changed to "syntax provides the binder name, semantic interpretation is in `sema/`"                                                                                                                              | Documentation             |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                        | Add `pub enum NameKind`                                                                                                                                                                                                               | New addition              |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                    | `trait TypeEnvProbe` + `NullProbe`                                                                                                                                                                                                    | New addition              |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                | Add `probe` field and accessor                                                                                                                                                                                                        | Data flow                 |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                              | Use `state.probe()` to determine `NameKind`                                                                                                                                                                                           | Data flow                 |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                              | Comment update (`Result`/`Option` go through `Generic`, already consistent with fact, only add a variant-deletion note)                                                                                                               | Documentation             |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                  | Delete the `_ => (Vec::new(), Type::Void)` catch-all, change to explicit error return                                                                                                                                                 | Behavior                  |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                              | Delete `n == "Terminates"`, change to `state.probe().is_predicate(n)`                                                                                                                                                                 | Reverse dependency        |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                              | Delete the `typecheck::operator_interfaces::spec()` call                                                                                                                                                                              | Reverse dependency        |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                  | Delete `signature_params` actual argument                                                                                                                                                                                             | With #4                   |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                   | Delete `Expr::FnDef` arm                                                                                                                                                                                                              | With #3                   |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                  | Retain (this is the parameter-list-fishing-back point; with 5.5 changed to read Lambda directly)                                                                                                                                      | With #3                   |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814` | Delete 11 match arms                                                                                                                                                                                                                  | With #1                   |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                              | Cut synonyms (case aliases, `i64`-type abbreviations, `"()"`)                                                                                                                                                                         | Convergence               |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                  | `DateTime` alias retained                                                                                                                                                                                                             | Retain                    |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                    | Migrate `From<Type> for MonoType` (currently at `mono.rs:692-830`) to be the sole conversion point                                                                                                                                    | New addition              |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                          | Register `i64`/`int` etc. as equivalent writings of `Int`                                                                                                                                                                             | Convergence               |
| 23  | `src/middle/core/ir.rs`                                                | `3`                                    | Delete `pub use ... ast::Type`                                                                                                                                                                                                        | Pure deletion             |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                  | `FunctionCode.params` kept as `Vec<MonoType>` (already so)                                                                                                                                                                            | —                         |
| 25  | `src/middle/core/bytecode.rs`                                          | `812`, `814`                           | `BytecodeFunction.params` / `return_type` changed to `MonoType`                                                                                                                                                                       | Convergence               |
| 26  | `src/middle/core/bytecode.rs`                                          | `854`                                  | `BytecodeModule.type_table` changed to `Vec<MonoType>`                                                                                                                                                                                | Convergence               |
| 27  | `src/middle/core/bytecode.rs`                                          | `2339`                                 | `type_table` conversion changed to `.collect()` (elements now same type)                                                                                                                                                              | With #26                  |
| 28  | `src/middle/core/bytecode.rs`                                          | `2352-2390`                            | **Delete** the entire `From<MonoType> for IrType` impl                                                                                                                                                                                | Pure deletion             |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                   | `type_table` changed to `Vec<MonoType>`                                                                                                                                                                                               | Convergence               |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 locations)              | `AstType::AssocType` arms handled per gate conclusion                                                                                                                                                                                 | To be determined          |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`           | Type-table gate T1-T3                                                                                                                                                                                                                 | Gate                      |
| 32  | `spawn/placement.rs`                                                   | `122`                                  | Delete `Expr::FnDef` arm                                                                                                                                                                                                              | With #3                   |
| 33  | `spawn/analysis.rs`                                                    | `912`                                  | Same as above                                                                                                                                                                                                                         | With #3                   |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                   | Same as above                                                                                                                                                                                                                         | With #3                   |
| 35  | `orchestrator.rs`                                                      | `1492`                                 | Same as above, change to take the name from `Assign.target`                                                                                                                                                                           | With #3                   |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                         | Same as above                                                                                                                                                                                                                         | With #3                   |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                 | Same as above                                                                                                                                                                                                                         | With #3                   |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                 | Same as above                                                                                                                                                                                                                         | With #3                   |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                 | Same as above                                                                                                                                                                                                                         | With #3                   |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                   | Same as above                                                                                                                                                                                                                         | With #3                   |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                  | Same as above                                                                                                                                                                                                                         | With #3                   |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                 | Same as above                                                                                                                                                                                                                         | With #3                   |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                          | Same as above + comment update                                                                                                                                                                                                        | With #3                   |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                              | Delete `Expr::FnDef` test                                                                                                                                                                                                             | Test                      |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                              | Rewrite as Lambda + Assign                                                                                                                                                                                                            | Test                      |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`        | Delete the 11 dead variants' lower assertions                                                                                                                                                                                         | Test                      |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                              | `mono_to_ast_type` deletes `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches, unified to `AstType::Name` + `mono.type_name()`. **Must be processed in the same batch as #1**                                      | Convergence               |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                           | `extract_generic_param_names` signature adds `probe: &dyn TypeEnvProbe`; delete the `typecheck::operator_interfaces::spec()` call, change to `probe.is_constraint(name)`. Callers (including `parser/tests/ast.rs`) pass `&NullProbe` | Reverse dependency        |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                | Delete the local `name_used_as_type` in this file, change to calling the merged version in `ast.rs`; the `is_predicate` short-circuit is changed to read `NameKind::Predicate`                                                        | Duplicated implementation |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                  | `CONST_PARAM_TYPES` determination changed to wire through `NameKind::Builtin` (see 5.3)                                                                                                                                               | Convergence               |

> **The gate itself must also be gated**: `tools/type-tables`, like `code-tables`, also needs `mod`
> declaration wiring (the lesson of 8 orphan test trees already in this repo). When creating a new
> crate, register it in the root `Cargo.toml` simultaneously.

### 6.4 Backward-Compatibility Strategy

**Constraint**: the 293 `.yx` corpus in `tests/yaoxiang/` (single file) + the multi-file corpus
layer established by P2 (`tests/yaoxiang-multifile/`) + `src/std/tests/*.yx` + the standard library,
**must all have unchanged behavior** (same diagnostic codes and messages, order can be normalized).

| Risk                                                                      | Judgment                                                                                                                                                                                                                                                                                                                        | Mitigation                                                                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source code written with `i64` / `int` / `f64` and other type annotations | After normalization to `Int`, `from_builtin_name` only accepts canonical names                                                                                                                                                                                                                                                  | **First thing in stage 1**: scan the entire corpus to count occurrences of non-canonical type names. If 0, the synonyms can be deleted directly; if not 0, per 5.3 move into the lexer alias table (**accepted at the syntax layer, normalized at the lower layer**, no impact on diagnostic codes) |
| Source code written with `DateTime`                                       | Alias retained (`mono.rs:627`)                                                                                                                                                                                                                                                                                                  | No impact                                                                                                                                                                                                                                                                                           |
| `("()")` appearing in a type position                                     | The `LParen` branch starting at `types.rs:167` handles tuples/parameter groups, producing `Tuple` rather than `Name{"()"}`                                                                                                                                                                                                      | Before deleting `"()"`, first count the occurrences of `"()"` as a type name in the entire corpus                                                                                                                                                                                                   |
| `dump_bytecode` output change                                             | `type_table` element type changes from `ir::Type` to `MonoType`, `{:?}` format differs                                                                                                                                                                                                                                          | **Allowed by C3 criterion** (07 specifies `dump_bytecode` is only compared at C1/C2 stage). But the output diff needs to be recorded at stage 2 for manual review                                                                                                                                   |
| Behavior change after lossy-bridge fix                                    | See 6.2                                                                                                                                                                                                                                                                                                                         | Corpus diff + targeted `dump_bytecode` comparison                                                                                                                                                                                                                                                   |
| `mono_to_ast_type` rewrite changes type-substitution results              | `function.rs:507-529` is the exit of monomorphization type substitution (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back to `AstType`). After deleting variants, if it goes to the `_` arm, the substitution result changes from `AstType::Int(64)` to something like `AstType::Name { name: "i64" }` | Targeted comparison at stage 3: select corpora containing generic substitution, compare the `type_name()` of the monomorphization product before and after the change. **Do not assume `{:?}` output equivalence**                                                                                  |
| The `.yx` source code itself                                              | **Zero modifications**                                                                                                                                                                                                                                                                                                          | This document does not require modifying any corpus or std source files. This is a hard acceptance item                                                                                                                                                                                             |

**Hard acceptance condition**: `git diff --stat tests/ src/std/` must be empty (except for the
multi-file corpus layer and the test baseline file additions).

## Implementation Points

The unified criterion is 07's **C3: same diagnostic codes and messages (order can be normalized)**.
Below is the stage breakdown for this document's line; the global ordering across documents is in
RFC-039.

| Stage                                         | Content                                                                                                                                                                                                                                            | Acceptance criterion                                                                                                                                                                                                                                  | Rollback point                         |
| --------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| **0. Gate first**                             | Create `tools/type-tables`, implement T1/T2/T3, T1's first version **only reports, does not fail**. Take a full baseline on **unchanged** code                                                                                                     | T1's report accurately lists the 13 zero-construction variants (including the qualitative conclusion for `AssocType`), and **separately lists the reverse-bridge reconstruction points recorded in 2.2**; T2/T3 fully green or produce a fixable list | No code changes, no rollback needed    |
| **1. Name normalization**                     | Count non-canonical type names in the corpus; `from_builtin_name` (`mono.rs:620-642`) cuts synonyms, moves them into the lexer alias table. **Simultaneously verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`** | C3: the full-corpus diagnostic set is **identically the same as the stage-0 baseline**                                                                                                                                                                | Single commit, revertible              |
| **2. Bytecode type unification**              | Change `bytecode.rs:812`/`814`/`854`, `image.rs:44` to `MonoType`; delete `From<MonoType> for IrType` (`bytecode.rs:2352-2390`)                                                                                                                    | C3 + targeted `dump_bytecode` comparison (per-item in the 6.2 table)                                                                                                                                                                                  | Single commit, revertible              |
| **3. Dead-variant deletion**                  | Per the 5.2 table, delete the 11 variants; clean up `mono.rs:699-708` and other match arms, the `types.rs:836` catch-all; **process `function.rs:507-529` (#47) in the same batch**; sync `types/tests/mono.rs`                                    | C3 + `cargo build` fully green (exhaustive match forces all arms to be filled) + generic-substitution targeted comparison (last line of 6.4)                                                                                                          | Single commit, revertible              |
| **4. parser data flow**                       | Add `probe.rs`; change `parser_state.rs:46-58`; change `declarations.rs:498-511` and `ast.rs:930` (#48); `types.rs:162-165` determines `NameKind`; merge the two `name_used_as_type*` (#49)                                                        | C3 + **newly-added assertion**: `grep 'typecheck' src/frontend/core/parser/` should be 0                                                                                                                                                              | Single commit, revertible              |
| **5. AST dead variants and dedicated fields** | Delete `Expr::FnDef` (16 locations) + `Assign.signature_params`; migrate `NamedParen` semantics                                                                                                                                                    | C3 + `tests/integration/` 18 modules fully green                                                                                                                                                                                                      | Two commits (FnDef / signature_params) |
| **6. Gate becomes hard**                      | T1 changes from warning to `panic!`; T4 downgraded to CI assertion                                                                                                                                                                                 | `cargo build` **must fail** when a zero-construction variant is deliberately introduced (red test first)                                                                                                                                              | Single commit, revertible              |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Stages 3 and 5 must come after stage
2**—`bytecode.rs:2372-2373` consumes `IrType::String` / `IrType::Bytes`; deleting the variant first
would break compilation. Stage 3's #47 must be in the same batch as #1, otherwise `mono_to_ast_type`
will silently change behavior.

**Downstream dependency**: after all stages of this document are complete, [04-ssa.md](04-ssa.md)
can start work. At that point, `BytecodeFunction.params`, `FunctionCode.params`, and `type_table`
will have a unified element type, and SSA's type annotations have a unique landing point.

### Mandatory Verification for Each Stage

| Stage | Must execute                                                                                                                                                                     |
| ----- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`, manually verify the T1 report (**including the separate listing of reverse-bridge reconstruction points**) |
| 1-5   | `cargo test`; full-corpus diff for `tests/yaoxiang/` (07 third layer); C3 criterion item-by-item comparison                                                                      |
| 2     | Additionally: for each variant in the 6.2 table, construct a `.yx` sample and `dump_bytecode` before and after the change for comparison                                         |
| 3     | Additionally: for corpora containing generic substitution, compare the `type_name()` of the monomorphization product before and after the change                                 |
| 4     | Additionally: `rg 'typecheck' src/frontend/core/parser/` returns 0                                                                                                               |
| 6     | Additionally: deliberately write a zero-construction variant and confirm `cargo build` fails (red→green)                                                                         |

## Key Decisions and Reasons

### Decisions

| Decision                       | Determination                                                                                                                                    | Reason                                                                                                                                                                                                                                                          |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Form of target representation  | `Type` as the sole type structure + `MonoType` as working form; `ir::Type` alias deleted                                                         | The alias at `ir.rs:3` is the direct cause of the field-type inconsistency between `BytecodeFunction.params` and `FunctionCode.params` (2.1). After the alias is deleted, the two field types are unified, and the `From` bridge loses its reason for existence |
| Reject "unify into `MonoType`" | Not adopted                                                                                                                                      | Would cause L2→L3 reverse dependency (violating RFC-039 layering), and would require adding `Span` / `ConstExpr` back to `MonoType`. Full argument in 5.1                                                                                                       |
| Do not delete `NamedParen`     | Retain as a pure syntax node, migrate binder semantics to `sema/`                                                                                | `ast.rs:519-532` records that deletion would reintroduce a fixed correctness defect: `(r: P(m))` would silently substitute undeclared `m` as a binder                                                                                                           |
| Directory rename               | **Do it** (D1), executed as a pure relocation batch at the P5/P6 wrap-up; this document's body paths are written in the pre-rename current state | 2026-10-03 decision (ChenXu233); 2026-10-04 construction-method addition                                                                                                                                                                                        |

### Rejected Alternative Plans

Four alternative plans are not adopted: **keeping three representations and only adding a conversion
layer** does not solve any of the root causes, and `ir::Type` still is not an independent type; the
dead variants and the synonym table and the reverse dependency all still exist. **Using macros to
generate the synonym table** treats the symptom rather than the cause—after `Type::Int(usize)` is
deleted, the macro has no reason to exist, and consistency is guaranteed by the T2 gate (more
verifiable than the macro's expansion correctness). **Only deleting the dead variants without
unification** has the lowest cost but leaves the problem in place, and there is no gate to prevent
the next person from adding `Type::Int128(usize)` and the same dead variant growing back (if only
one commit can be done, it should be clearly noted as a subset of that plan and the T1 gate should
be delivered together). **Unifying into `MonoType`** is in 5.1.

### Benefits

- **Eliminates the maintenance burden of 14 locations**. The 12 consumption + 2 exhaustive arms of
  `Expr::FnDef` are a parallel path that never executes; deleting it removes one branch each in
  `ir_gen.rs` / `ownership.rs` / `termination.rs`.
- **Eliminates a duplicated implementation that has already forked**. The two copies of
  `name_used_as_type` (2.4) have already diverged in their definition-body field recursion; after
  merging, only one remains.
- **After the lossy bridge disappears, `IrType::Void` is no longer "a legal type value"**. This is a
  prerequisite for the "type consistency" invariant of the 07 first-layer validator to truly pass
  green.
- **The gate makes regression impossible**. T1 turns "adding a zero-construction variant" from
  current silent pass-through to build failure—the existing `build.rs` mechanism in this repo proves
  this path works.
- **Zero source-code modifications**. The 293 `.yx` corpus and std library do not need any
  modification; this is the fundamental reason the C3 criterion can hold.

## Known Limitations and Risks

### Risks

- **Stage 2 may have behavior change**. After the lossy-bridge fix, the 8 `MonoType` variants now
  see the real type rather than `Void`. The direction is a fix, but it will change the paths
  depending on signature bytecode. This is the largest uncertainty in this document.
- **The qualitative determination of `AssocType` depends on gate accuracy**. If T1 uses line-level
  heuristics (`=>` count) instead of `syn`, it may misjudge. This is why stage 0 requires manual
  verification.
- **T1's undercount of the reverse bridge will cause the disposition table and the gate to
  contradict each other**. 2.2 has proven that the 7 "dead variants" have production reconstruction
  points in the reverse bridge. If T1 does not separately list the construction-point direction,
  these 7 will be judged as "having construction points" and exempted from deletion—the gate will
  become a hindrance to the deletion work rather than a guarantee.
- **Deleting `signature_params` carries semantic risk**. It carries the **unfiltered** parameter
  list, while `Lambda.params` is **filtered**. If `classify_generic_params` depends on the former,
  the change will alter the generic-classification result. **It must be verified first; equivalence
  cannot be assumed** (the `signature_params` consumption point at `ir_gen.rs:1380` on the ir_gen
  side must be verified together).
- **Stage 3 touches `middle/passes/mono/function.rs`**. In addition to the 8 matches for
  `AssocType`, `mono_to_ast_type` at `507-529` is the exit of monomorphization type substitution,
  and after the rewrite the `type_name()` of the substitution result may change (last line of 6.4).
- **Non-canonical type names in the `.yx` corpus may be non-zero**. If the corpus uses `i64` / `int`
  extensively, the stage-1 "directly delete synonyms" is infeasible and degrades to "move into the
  lexer alias table", reducing the convergence extent of `from_builtin_name`.
- **The gate will go red at the beginning**. If T1 goes online with `panic!` directly, it will
  immediately block compilation. It must first only report and take the baseline.

### Open Questions

> **The open questions originally listed in this section have all been decided.** See item-by-item
> decisions in [RFC-039 Decision Register](../../rfc/draft/039-compiler-architecture.md) (D1–D50).
> **This document leaves no pending items.**

## See Also

### Upper-Level and Sibling Documents

- [RFC-039 Compiler Feature Routing Catalog Design (Master Plan)](../../rfc/draft/039-compiler-architecture.md)
  — upper-level master plan; the four-layer model, criterion grading, routing table C (the 3
  type-representation-related entries belong to this document)
- [01-routing.md](01-routing.md) — feature routing tables A/B/C, dependency direction specification,
  target directory structure
- [04-ssa.md](04-ssa.md) — SSA-ization; this document's prerequisite and downstream consumer
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — frontend paradigm; operator change surface
  convergence
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C3 category definition, diagnostic-set
  comparison specification, type-consistency invariant of the first-layer IR validator

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — basis for the
  `Type::Generic` unified path
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — built-in type table
  (comparison target for gate T3)
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — example of
  the `build.rs` build-time gate
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — source of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

- `src/frontend/core/parser/ast.rs:427-541` — definition of the 26 `ast::Type` variants
- `src/frontend/core/parser/ast.rs:519-540` — the binder-semantics note for `Type::NamedParen`
  (evidence that deletion would reintroduce the defect)
- `src/frontend/core/parser/ast.rs:241-249` — `StmtKind::Assign.signature_params`, the
  typechecker-dedicated field
- `src/frontend/core/parser/ast.rs:838-841` — `CONST_PARAM_TYPES`: the third copy of const generic
  parameter names
- `src/frontend/core/parser/ast.rs:843-885` — `name_used_as_type_in`: the duplicate of
  `declarations.rs:42` (already forked)
- `src/frontend/core/parser/ast.rs:891`, `930` — `extract_generic_param_names` and the
  cross-boundary call within it
- `src/frontend/core/parser/ast.rs:191-213`, `217-223` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — the parser represents all primitive types
  as `Type::Name`
- `src/frontend/core/parser/statements/types.rs:392-396` — `Result`/`Option` are not lowered to
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:836` — `Type::Void`'s sole forward construction
  point (error catch-all)
- `src/frontend/core/parser/statements/declarations.rs:42-80` — the original `name_used_as_type`
- `src/frontend/core/parser/statements/declarations.rs:494-511` — hard-coded `"Terminates"` and
  `typecheck::operator_interfaces::spec()` call
- `src/frontend/core/parser/pratt/nud.rs:505-514` — expression position represented as empty-body
  `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:466` — `expr_to_params` arm fishes the parameter list back
- `src/frontend/core/types/mono.rs:618-643` — `from_builtin_name` synonym table
- `src/frontend/core/types/mono.rs:692-830` — `From<Type> for MonoType` implementation (12 of the 13
  match arms are here)
- `src/frontend/core/types/mono.rs` — `MonoType` definition (about 22 variants, from `mono.rs:167`;
  1077-line file)
- `src/frontend/core/types/const_data.rs:234-258`, `321-330` — `const_data::BinOp` /
  `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — compile-time constant evaluation (1128 lines)
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- `src/middle/core/ir.rs:3` — evidence that `ir::Type` is an alias of `ast::Type`
- `src/middle/core/bytecode.rs:808-814` — the `Vec<ir::Type>` field of `BytecodeFunction`
- `src/middle/core/bytecode.rs:2352-2390` — the lossy `From<MonoType> for IrType` bridge
- `src/middle/passes/codegen/bytecode.rs:32`, `275-278` — the `Vec<MonoType>` field of
  `FunctionCode`
- `src/middle/passes/mono/function.rs:507-529` — `mono_to_ast_type`: the reverse bridge that
  reconstructs the 7 "dead variants"
- `src/backends/interpreter/image.rs:44` — the interpreter module's `type_table`
- `build.rs:19-37` — error-code registry build-time gate (form reference for the type-table gate)
- `build.rs:39-54` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — the gate crate depends only on `serde_json` (root crate has
  no `syn`; `tools/type-tables` introducing `syn` is the first exception)
