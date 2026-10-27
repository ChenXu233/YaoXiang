# Type Representation Unification

> **Companion design document.** This is a companion document to
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). See
> the main text of RFC-039 for the four-layer model, acceptance-criterion tiers, and execution-phase
> order; see the [index in this directory](index.md) for the positioning of each companion document.

## Positioning and Scope

### Coverage

Consolidate the several parallel type representations currently in use into a single representation,
eliminating the four categories of burden derived from them:

- **13 `ast::Type` variants with zero forward construction by the parser** (13 of the 26 variants in
  `ast.rs:427-541`; see 2.2; the definition of “forward” excludes reverse bridges and AST→AST
  reconstruction points)
- **A handwritten table of type-name synonyms** (`mono.rs:618-643`; see 2.3)
- **The parser’s genuine reverse dependencies on the typecheck layer** (2 locations; see 2.4)
- **One expression variant with zero production construction**, `Expr::FnDef` (`ast.rs:37-43`; see
  2.6)

The concrete deliverables are fourfold: a per-variant disposition table for all 26 variants (5.2), a
semantic `NameKind` for `Type::Name` and elimination of the synonym table (5.3), an injected
`TypeEnvProbe` data flow (5.4), and build-time type-table gates T1–T3 (5.7).

### Directory Renaming and This Document’s Paths

RFC-039 decision D1 mandates directory renaming (`typecheck/`→`sema/`, `middle/core/`→`middle/ir/`),
to be completed **together with P5/P6, without phasing**. This document’s 50-item change list and
all acceptance greps **are written against the pre-rename paths**—they are the construction baseline
for the individual P6 batches **before the rename batch**; the directory rename at the end of each
batch is a **pure move batch** (C1 zero-diff), committed separately. The baseline becoming invalid
after the rename commit is expected; use git history as the bisection reference.

### Not Covered

- **How the directory rename is performed**. `Type` does not move out of `parser/ast.rs` in this
  phase; only a new `lower.rs` is added under `types/` as the sole conversion point. The directory
  rename (`typecheck/`→`sema/`, etc.) is performed at the end of P5/P6 according to RFC-039 decision
  D1, in the form shown in the target directory structure in `01-routing.md`.
- **The four-layer model and dependency-direction rules**. See [01-routing.md](01-routing.md).
- **The tiering of equivalence criteria and validator implementation**. The definitions of C1–C6 and
  the three tiers of criteria (IR validator / normalized snapshots / corpus differential) are in
  [07-equivalence-oracle.md](07-equivalence-oracle.md). This document references only **C3
  (type-representation convergence)** as its acceptance criterion.
- **SSA conversion**. See [04-ssa.md](04-ssa.md). This document is its prerequisite, for the reason
  given below.
- **The global P1–P10 execution sequence**. See the main text of RFC-039 and
  [02-stage-contract.md](02-stage-contract.md). This document gives only the phase breakdown for its
  own workstream (see “Implementation Essentials”).

### Division of Labor with RFC-039

RFC-039 is the overarching framework: the four-layer model, criterion tiers, cross-document phase
ordering, and the rationale for performing this refactoring now. This document is the construction
plan for **type-representation convergence**, one of the workstreams in RFC-039: exactly which
variants to delete, which files to change, in what order, and which gates will prevent regression.

All line numbers, variant lists, and construction-site counts in this document were obtained through
static verification of `src/`; each item cites `file:line`. **This document does not record the
execution process; it records code facts and the resulting dispositions.**

To facilitate cross-references with documents from the same batch, the “Target Design” and “Detailed
Design” chapters retain the `5.x` / `6.x` subsection numbering; the remaining chapters follow this
directory’s unified conventions.

### Why This Must Precede SSA Conversion

`ir::Type` is not a third independent type; it is a `pub use` alias for `ast::Type` (`ir.rs:3`).
[04-ssa.md](04-ssa.md) will introduce register type annotations and new instructions into the IR. If
the representation is not consolidated first, SSA type annotations will flow through the `ir::Type`
alias directly onto the syntactic type’s 26 variants, effectively institutionalizing a third
representation.

The current capability boundary of `ir::Type` is defined by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) embeds a complete expression tree in a type. It
  is impossible for an `Expr` to appear in an IR instruction’s type annotation.
- `ast::Type::Literal { name, base_type }` (`ast.rs:476-482`) carries a source name and `Span`. It
  is the syntactic form of a compile-time literal type, not a runtime type.

Applying SSA type annotations over the 26 variants would permanently codify the 13 dead variants and
the two syntax-only variants in the IR contract.

## Current State

Everything below is a statement of code fact, not a proposal. Each item provides `file:line`
evidence.

### 2.1 Three Parallel Representations and a Lossy Bridge

The multiplicity of type representations is the common upstream cause of four independent defects:

| Downstream symptom                                                              | Causal relationship to type representation                                                                                                                                                                                                                                                                                                                            |
| ------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The same type text has multiple valid AST forms                                 | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but have different types**, the parser selects the latter, and typecheck must supply a table to translate the former back                                                                                           |
| Bytecode-layer signatures differ from IR-layer signatures                       | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only obtain the alias. Consequently, `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`), while `FunctionCode`’s `params` (`src/middle/passes/codegen/bytecode.rs:275`) is declared as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding a built-in type name requires changes in multiple places, with no checks | Because the name→type mapping is scattered across `from_builtin_name` (`src/frontend/core/types/mono.rs:618-643`) and the string match in `bytecode.rs:2371-2376`, with the same mapping written twice and no cross-validation                                                                                                                                        |
| `"Terminates"` is hard-coded in the parser                                      | Because the parser cannot determine “is this a type?” or “is this an operator interface?”, it can only embed type-layer knowledge in the syntax layer as a string literal (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                                                 |

**The first three are representation issues; the fourth is a downstream side effect of a
representation issue.** Doing only item 1 (deleting dead variants) would leave the synonym table and
reverse dependencies; doing only item 4 (injecting a callback into the parser) would build on an
inconsistent representation.

One of the three representations is an alias illusion:

| Name        | Definition location                       | Size        | Role                                                    |
| ----------- | ----------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541` | 26 variants | Syntax-layer type                                       |
| `MonoType`  | `src/frontend/core/types/mono.rs`         | 1,077 lines | Semantic-layer type (after monomorphization)            |
| `ir::Type`  | `src/middle/core/ir.rs:3`                 | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

`ir.rs:3` is a one-line `pub use`, followed immediately by
`use crate::frontend::core::typecheck::MonoType;` at `ir.rs:6`. The IR module depends on two “type”
symbols simultaneously, but one of them is only an alias.

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
    _ => IrType::Void,          // ← Any other generic type silently becomes Void
},
// bytecode.rs:2378-2385
MonoType::Struct(_) | MonoType::Enum(_) | MonoType::Ref { .. }
    | MonoType::TypeVar(_) | MonoType::TypeRef(_) | MonoType::Union(_)
    | MonoType::Intersection(_) | MonoType::AssocType { .. } => IrType::Void,
```

All eight `MonoType` variants collapse into `IrType::Void`, in addition to the `_ => IrType::Void`
fallback (`bytecode.rs:2386-2387`, whose comment acknowledges that it is a “transitional branch from
before enum variants were removed”).

`IrType::Void` is here a **legal type value rather than an error signal**. Downstream code therefore
cannot distinguish “this function really returns `Void`” from “this function’s return type was lost
during bridging”—one reason why 07’s first-tier validator cannot currently make its “type
consistency” invariant truly pass.

At the same time, `IrType::String` / `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`) and
`ast::Type::Bytes` (`ast.rs:436`), which are precisely the dead variants to be removed in 2.2—**both
ends of the lossy bridge point to dead code**.

### 2.2 Parser-Forward Variants with Zero Construction

**Definition of the measurement**: the construction-site scan covers all of `src/`, excluding
`tests/` and `*/tests/*`. A **construction site = an expression-position `Type::<variant>` path**
(pattern positions do not count), and results must be **listed separately in three categories and
never conflated**: ① **parser-forward** (parser → AST); ② **AST→AST reconstruction** (the two
replacement functions in `middle/passes/mono/function.rs`); ③ **reverse bridge**
(`From<MonoType> for IrType` and `mono_to_ast_type`).

Of the 26 variants of `ast::Type`, the following 13 have zero construction in the **parser-forward**
direction (reconstruction points in the latter two categories are covered in 2.2.1 / 2.2.2):

| Variant                              | Definition       | Parser-forward construction sites                                                                                                                                                                                                                                                                                     |
| ------------------------------------ | ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Int(usize)`                         | `ast.rs:432`     | None (`mono.rs:699` is a match arm)                                                                                                                                                                                                                                                                                   |
| `Float(usize)`                       | `ast.rs:433`     | None (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                   |
| `Char`                               | `ast.rs:434`     | None (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                   |
| `String`                             | `ast.rs:435`     | None (`mono.rs:702` is a match arm)                                                                                                                                                                                                                                                                                   |
| `Bytes`                              | `ast.rs:436`     | None (`mono.rs:703` is a match arm)                                                                                                                                                                                                                                                                                   |
| `Bool`                               | `ast.rs:437`     | None (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                   |
| `Void`                               | `ast.rs:438`     | `types.rs:836` (**the only forward site = error fallback**) `_ => (Vec::new(), Type::Void)`, registered as an exemption item by item; removed with item 12 in 6.4                                                                                                                                                     |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | None (`mono.rs:747` is a match arm)                                                                                                                                                                                                                                                                                   |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | None (`mono.rs:743` is a match arm); 2 AST→AST reconstruction points, see 2.2.2                                                                                                                                                                                                                                       |
| `Option(Box<Type>)`                  | `ast.rs:455`     | None (`mono.rs:770` is a match arm); 2 AST→AST reconstruction points, see 2.2.2                                                                                                                                                                                                                                       |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | None (`mono.rs:771` is a match arm); 2 AST→AST reconstruction points, see 2.2.2                                                                                                                                                                                                                                       |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | None (`mono.rs:814` is a match arm); 2 AST→AST reconstruction points, see 2.2.2                                                                                                                                                                                                                                       |
| `AssocType { .. }`                   | `ast.rs:463-471` | None (`mono.rs:781-786`, `formatter/handlers/types.rs:84`, and `semantic_tokens.rs:230` are all match arms); **2 production AST→AST reconstruction points**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3; **D8 remains unchanged** |

> **Line-anchor drift registration (P6 6.0 measurement, 2026-10-27):** the “Definition” columns in
> this table and in 5.2 were written on 2026-10-05. Seventeen lines were subsequently inserted
> before the `ast::Type` enum (`pub enum Type` moved from 427 to 444), so **the definition lines of
> all 26 variants uniformly shifted by +17** (for example, `Int` moved from 432 → 449, and
> `NamedParen` from 533-540 → 550-557). This round only **registers** the drift and does not alter
> the table body; subsequent references use the `ast::Type` symbol anchor.

The root-cause chain can be fully verified:

1. **The parser represents every primitive type as `Type::Name { name }`**—the fallback arm at
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered into dedicated nodes**—the comment at `types.rs:392-396`
   explicitly states: “RFC-010: Result/Option are not lowered into dedicated AST nodes…they follow
   the same `Type::Generic` path as user-defined generic types. The type representation remains
   `Generic{"Result"/"Option", args}`.”
3. **The only forward construction site of `Type::Void` is an error
   fallback**—`_ => (Vec::new(), Type::Void)` at `types.rs:836`, at the end of a match returning
   `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction sites: `types.rs:364`, `types.rs:571`, `types.rs:100`, and
`declarations.rs:573`. They are not dead variants.

**These 13 variants have construction sites in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
Therefore, this document’s wording is “**zero parser-forward construction**”: it means neither “zero
references anywhere” nor “zero production construction”—11 of them do have production construction
points in the other two categories (reverse bridge / AST→AST reconstruction). These tests themselves
codify the behavior under test: they assert precisely what these syntactic forms lower to as a
`MonoType`, even though the parser never produces those forms. When the variants are removed, these
tests must also be removed or rewritten.

#### 2.2.1 Measurement Boundary (1): The Reverse Bridge Reconstructs 7 of Them

The table’s “zero forward construction” status holds only in the **parser → AST
forward-construction** direction. In the reverse direction, there are two production-grade
`MonoType → ast::Type` reconstructions that **actually construct 7 of the variants in the table
above**:

| Reverse bridge                                                 | Location                                 | Members of the 13-variant list reconstructed                                                                             |
| -------------------------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `From<MonoType> for IrType`                                    | `bytecode.rs:2353-2390`                  | `Int` (`2357`), `Float` (`2358`), `Bool` (`2359`), `Char` (`2360`), `Void` (`2361`), `String` (`2372`), `Bytes` (`2373`) |
| `mono_to_ast_type` (nested fn inside `substitute_type_in_ast`) | `middle/passes/mono/function.rs:507-529` | `Int` (`512`), `Float` (`513`), `Bool` (`514`), `Char` (`515`), `Void` (`516`), `String` (`522`), `Bytes` (`523`)        |

```rust
// middle/passes/mono/function.rs:511-527
match mono {
    MonoType::Int(n) => AstType::Int(*n),
    MonoType::Float(n) => AstType::Float(*n),
    MonoType::Bool => AstType::Bool,
    MonoType::Char => AstType::Char,
    MonoType::Void => AstType::Void,
    MonoType::TypeRef(name) => AstType::Name { name: name.clone(), span },
    // #299: Container types such as String/Bytes are now Generic and are converted back to AstType via type_name
    MonoType::Generic { name, .. } if name == "String" => AstType::String,
    MonoType::Generic { name, .. } if name == "Bytes" => AstType::Bytes,
    _ => AstType::Name { name: mono.type_name(), span },
}
```

**This measurement boundary directly affects disposition**:

- The `bytecode.rs` location disappears together with item 28 in 6.3 (removal of the entire
  `From<MonoType> for IrType` impl), so it adds no extra cost.
- `function.rs:507-529` **is not covered by any existing change item**. It is a `MonoType → AstType`
  type-name resolver (annotated `#299` in the comment), converting `String` / `Bytes` from `Generic`
  back into dedicated variants by name. After removing `ast::Type::String` / `Bytes`, these two
  guard branches must be rewritten; after removing `Int` / `Float` / `Bool` / `Char` / `Void`, the
  behavior of the `_` arm changes (some types that previously fell into `_` will silently switch to
  `AstType::Name`). This is item 47 added to 6.3.
- **Direct consequence for the gate**: a T1 gate that scans only forward construction sites would
  judge the seven variants above to have construction sites and exclude them from the removable
  list. T1 must explicitly include reverse bridges in its measurement scope; otherwise, the gate’s
  conclusions conflict with the disposition table in this document (see 5.7).

#### 2.2.2 Measurement Boundary (2): AST→AST Reconstruction (the Third Category Added in This Revision)

`substitute_type_ast` (320-497) and `substitute_type_in_ast` (499-655) in
`middle/passes/mono/function.rs` are **AST→AST** type-variable substitutions (recreating an existing
AST type, **not** `MonoType → AST`). The P6 T1 gate measurement (6.0.2 baseline, 2026-10-27) found
**15 variants / 28 points**, of which **5 variants / 10 points** belong to the 13-variant list
above:

| Variant     | AST→AST reconstruction points |
| ----------- | ----------------------------- |
| `Union`     | `377`, `606`                  |
| `Option`    | `427`, `600`                  |
| `Result`    | `431`, `602`                  |
| `Sum`       | `467`, `632`                  |
| `AssocType` | `456`, `623`                  |

**When 6.3 removes these 5 variants, it must rewrite these 10 points in the same batch** (already
registered in 09’s 6.3.2); otherwise, those arms will fail to compile after the variants are
removed.

### 2.3 Handwritten Synonym Table

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`, measured at 841-867 / **13 arms**)
uses a string table to map type names to `MonoType`:

```
"Int" | "int" | "Int64" | "int64" | "i64"  => Some(MonoType::Int(64)),   // :843
"DateTime" | "datetime"                     => Some(MonoType::Int(64)),   // :850
"Int32" | "int32" | "i32"                   => Some(MonoType::Int(32)),   // :851
...
"Void" | "void" | "()"                      => Some(MonoType::Void),      // :863
```

**The sole reason this table exists is to bridge the AST layer’s inconsistency**: because
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent at the AST layer but
have different types (and the parser produces only the latter), `from_builtin_name` must accept case
variants, abbreviations, signed names, and the `DateTime` alias. The appearance of `"()"` on the
`Void` line is especially revealing: it works in conjunction with the `Type::Void` fallback at
`types.rs:836`.

The same knowledge—which names are built-in type names—exists in seven copies throughout the
repository, with no consistency validation whatsoever:

| #   | Location (**measured 2026-10-27**; this section was written on 2026-10-05, so the original line numbers have drifted)                                                  | Form                                                                                                                                                                                         |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`, measured at 841-867 / **13 arms**)                                                                   | String match, name → `MonoType`                                                                                                                                                              |
| 2   | `src/middle/core/bytecode.rs:2371-2376`                                                                                                                                | String match, copying `"String"` / `"Bytes"` / `"Tuple"`                                                                                                                                     |
| 3   | `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`, measured at 855-858)                                                                                           | A `&[&str]` constant containing 15 const generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`)                                                   |
| 4   | `src/lsp/world.rs:178-181`                                                                                                                                             | The LSP-side list of built-in type names (15 entries)                                                                                                                                        |
| 5   | `ConstKind::from_ast_type_name` (`src/frontend/core/types/const_data.rs`, measured at 407)                                                                             | String match, name → `ConstKind`. **Its coverage differs from item 1**: it does not accept lowercase or the `i64` / `i32` family of aliases (registered by the P0 gate on 2026-10-06)        |
| 6   | `eval_builtin`’s primitive-type size table (`src/frontend/core/types/eval/const_eval.rs`, measured at 497-512; `"Int"` / `"Uint"` / `"Float"` / `"String"` → 8 at 503) | String match, name → byte size. It contains a `"Uint"` arm, whereas item 1 has no `Uint` mapping at all—**a dead arm and living evidence of table drift**                                    |
| 7   | `to_type_id` (`src/middle/passes/codegen/bytecode.rs`, measured at 735-775; `"Set" => 25` at 762)                                                                      | String match, built-in name → serialized type ID. `"Set" => 25` contradicts the typecheck-side “Set is excluded by name” decision (`inference/expressions.rs:2917-2918`, decision 4 in #300) |

> **Line-anchor drift registration (P6 6.0 measurement, 2026-10-27):** the line numbers of all seven
> copies in this section **have drifted** (item 1 by +223; the others by 2–9 lines), but **the
> number and content of the entries have not changed** (13 arms in item 1; 15 entries each in items
> 3 and 4). Subsequent references use **symbol anchors + repository paths**; the line numbers in
> this table are for location only. The arm count in item 1 and the living evidence of drift in
> items 5–7 (the dead `Uint` arm and the contradiction between `Set` and the typecheck side) **all
> remain**, and this section made no code changes.

Item 3, `CONST_PARAM_TYPES`, is used by `extract_generic_param_names`
(`src/frontend/core/parser/ast.rs`, measured at 932) to identify const generic parameters. It
enumerates “type names that may be used as const parameters,” and its relationship to the full set
handled by `from_builtin_name` is **partially overlapping but not identical**, with no mechanism
ensuring that the two remain synchronized. Item 4 is a fourth copy used for LSP syntax highlighting
and hovers, likewise without validation (T2’s comparison scope includes it).

Items 5–7 were first registered by `check-concepts.py` (P0, [09](09-execution-wbs.md) §0.2.1) on
2026-10-06: item 5 is a second type-name synonym table; items 6 and 7 share the same root as items
1–4 (string-keyed tables mapping built-in names to semantics / IDs), though their uses differ and
they carry the same drift risk. The same gate also found 11 string-based dispatches of built-in
container names on the typecheck / eval / middle sides (`"List" | "Vec" | "Array"` ×4,
`"String" | "Bytes"` ×4, etc.). These are **not covered by this section or by the parser data flow
in 5.4**; the inventory and disposition are registered in
[06-cleanup-inventory.md](06-cleanup-inventory.md) §G3 under the P6 extended-verification items.

### 2.4 The Parser’s Reverse Dependency on the Type Layer

RFC-039 routing table C records 3 entries (`declarations.rs:27-80` calls `is_type_param_annotation`
/ `name_used_as_type`, both of which belong to the type layer). **Measurement differs from that
description**: these two functions are defined inside the parser itself, not in the type layer.

- `is_type_param_annotation` is defined at
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the only `fn` definition in the
  repository)
- `name_used_as_type` is defined at `declarations.rs:42-80` (the only `fn` definition in the
  repository)
- `declare_predicate` / `is_predicate_name` are defined at
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

There are only **2 production-code cross-layer references** to `typecheck`, and both call the same
function, `crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Enclosing function                 | Purpose                                                                                             |
| --- | --------------------------------------------------------- | ---------------------------------- | --------------------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature-parameter filter closure | Identifies constraint-position parameters (`T: Add`), which do not occupy runtime parameter slots   |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`      | Identifies constraint-position parameters and produces `GenericParamName` values with `constraints` |

The second reference is deeper: **`parser/ast.rs` itself**—the same file containing
`StmtKind`/`Expr` (`ast.rs:847-948` as a whole)—directly references a typecheck symbol. Routing
table C records only the `declarations.rs` location.

Together with the hard-coding at `declarations.rs:498-499`, it forms the same decision:

```rust
// declarations.rs:498-499
// Extended as built-in predicates are added (the future open-set approach: pass predicate
// names down to the parser, or change to a form decidable by the syntax layer).
// The parser does not hold the type environment, so it looks in two places: hard-coded
// built-in predicates, plus declarations parsed so far (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment itself acknowledges that this is the temporary form of a “future open-set approach.”
`n == "Terminates"` **embeds type-layer knowledge in the syntax layer as a string literal**.

**Why this is downstream of the type-representation problem**: the parser needs to ask “is this an
operator-interface name?” because it must distinguish whether `Type::Name` refers to a type or an
interface constraint. **`Type::Name { name: String }` uses an undifferentiated string to carry at
least 4 meanings** (concrete type / type variable / constraint name / predicate name). The syntax
layer cannot determine this on its own, so the decision authority is pushed up into the type layer.

#### Duplicate Implementation: Same Name and Meaning, Behavior Already Diverged

There are two implementations of `name_used_as_type`. The comment at `ast.rs:843-846` acknowledges
this:

```rust
/// Whether a parameter name is treated as a type reference in a given type (`(N: Int) -> (n: N)`).
///
/// Semantically equivalent to `name_used_as_type` in `declarations.rs`; independently
/// implemented here to avoid a cross-module dependency within the parser.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations **have already diverged**:

| Dimension               | `name_used_as_type` in `declarations.rs:42-80`                                  | `name_used_as_type_in` in `ast.rs:847-885`           |
| ----------------------- | ------------------------------------------------------------------------------- | ---------------------------------------------------- |
| Signature               | Takes an `is_predicate: &dyn Fn(&str) -> bool` callback                         | No callback                                          |
| `Type::Generic` branch  | First short-circuits on `is_predicate(app_name)` and returns `false` (`:54-56`) | No such short-circuit; directly recurses into `args` |
| `Type::Struct { body }` | **Absent** (falls into `_ => false`)                                            | **Present** (`:871-879`; traverses `Field` / `Expr`) |
| `Type::NamedStruct`     | **Absent**                                                                      | **Present** (`:880-882`; traverses `fields`)         |

The comment at `ast.rs:868-870` records the origin of the latter two branches: “Previously, because
these two branches were missing, a const generic parameter referenced in a definition body could not
be identified as ‘used as a type,’ and annotation validation falsely reported a legal const
parameter as an unknown name.”

In other words: **the function was duplicated to avoid a cross-module dependency, but fixes did not
flow back to the original.** The `ast.rs` version recursively traverses fields in a definition body,
whereas the `declarations.rs` version does not—the same source code therefore yields different
conclusions at two decision points. This duplicate must be consolidated when 5.3 introduces
`NameKind`.

### 2.5 Parallel Operator Enums

There are two operator enums that do not reference each other and **whose members have already
diverged**:

| Concept          | Syntax layer                    | Constant-evaluation layer                                             | Difference                                                                                                                                  |
| ---------------- | ------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Binary operators | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | The syntax layer alone has `Neq` (`:197`), `Range` (`:205`), and `Assign` (`:206`); the constant layer uses `Ne` (`:243`) rather than `Neq` |
| Unary operators  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | The syntax layer alone has `Deref` (`:222`); the constant layer alone has `BitNot` (`:329`)                                                 |

Each also implements a complete `Display` (`const_data.rs:291-317`) and classification predicates
(`is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise` at `const_data.rs:260-289`).

**Why this is part of the type-representation problem**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) **are not types**. When they appear in type position, they fall
into `Type::ConstExpr` (`ast.rs:506`), the “compile-time-only variant.” This directly relates to the
IR-layer type discipline in 6.1: after convergence, `Type::ConstExpr` disappears following
`Type → MonoType`, and the IR can never see it, while `Assign` / `Deref` continue to exist through
paths such as `ir_gen.rs`’s `BinOp::Assign => Ok(MonoType::Void)` (`inference/expressions.rs:875`).
The divergence between the two enums makes it impossible to determine “which operators may appear in
type position?” from either side alone.

This phase does not merge the two enums (their ownership is covered in
[05-frontend-paradigm.md](05-frontend-paradigm.md) under “operator-change impact”), but T1’s scan
scope should include `ast::BinOp` / `ast::UnOp` so that “new operators with zero construction” is
equally visible.

### 2.6 `Expr::FnDef`: An Expression Variant with Zero Production Construction

It is true that `Expr::FnDef` (`ast.rs:37-43`) has **zero production construction**—there are only
two construction sites in the entire repository: `src/frontend/core/parser/tests/ast.rs:837` and
`src/frontend/core/typecheck/tests/checker.rs:111`, both in tests.

However, **there are far more than 4 consumption points**. The measured production consumption count
is 12:

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

There are also 2 exhaustive match arms that exist solely for it: `ast.rs:1026` (`Expr::span()`) and
`pratt/mod.rs:47` (`expr_end_line`).

**The actual path taken by function definitions** is: `nud.rs:425-429` constructs `Expr::Lambda`,
and `declarations.rs:462-479` constructs `StmtKind::Assign` (placing the `Lambda` in `value`). In
other words, the `FnDef` branch is **a parallel path that never executes but is maintained in 14
locations**.

### 2.7 Syntax Nodes Store Semantics on Behalf of the Type Layer

**First, `StmtKind::Assign` carries a typechecker-specific field.** The `Assign` variant at
`ast.rs:241-249` contains `signature_params: Vec<Param>` (`ast.rs:244-245`), and the comment states
directly:

> `/// The first parameter group of the signature is preserved as-is (including parameter names), for typechecker classify_generic_params use`

The statement AST reserves a dedicated slot for the type checker. Construction point:
`declarations.rs:472`.

**Second, expression position is treated as a parameter list.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a **Lambda with an
empty body**. It is then “retrieved” as a parameter list by the
`Expr::Lambda { params, .. } => Some(params.clone())` arm at
`src/frontend/core/parser/pratt/led.rs:466`.

In other words, a normal AST variant is used as a **temporary parameter-list carrier**, and another
module’s match arm restores its meaning.

**Third, `Type::NamedParen` carries the return-position binder name.** The comment at
`ast.rs:519-540` states explicitly:

> RFC-027 §3’s return-position refinement relies on it to declare the **return formal parameter
> name** (called a binder here)…**Without it, the type checker can only guess** that “a free
> variable not in scope in the constraint is the return formal parameter,” so `(r: P(m))` would also
> silently substitute the undeclared `m` as a binder (measured defect).

A syntax node is storing binder identity on behalf of the type layer.

## Target Design

### 5.1 Shape of the Target Representation

**Proposition: a single representation = `Type` (formerly `ast::Type`) as the sole type structure;
`MonoType` is demoted to the type checker’s internal working representation; the `ir::Type` alias is
removed.**

The three representations converge into **two roles**, but only one is a “type”:

```
唯一类型结构  Type  ─────────────────────────────┐
   (26 → 13 变体, 住在 types/ 而非 parser/ast.rs)  │
                                                  │  From<Type> for MonoType  (total, 无语义损失)
                                                  │  ← 唯一转换点 types/lower.rs
类型检查工作表示  MonoType  ──────────────────────┘
   (带 TypeVar / 替换态, 只在 typecheck 内部流转)
   ×
ir::Type 别名删除, BytecodeFunction.params / type_table 改用 MonoType
```

**Why not “merge into `MonoType`”** (this is the proposal most likely to be raised, and it is
rejected here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.);
   `MonoType` does not. 07’s C3 criterion is “identical diagnostic codes and messages”—diagnostic
   position information must be traceable to source. Adding `Span` to `MonoType` would burden
   monomorphized types with a collection of meaningless sentinel spans.
2. `MonoType` contains `TypeVar` and substitution states (`types/substitute.rs:112`; `pub fn unify`
   at `types/solver.rs:411`). These are **intermediate states of the type checker’s solving
   process**, not types themselves. Putting them into the sole type structure would require the IR
   construction layer to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027) and are lowered away in `MonoType`. Unifying on `MonoType`
   would require adding these syntactic forms back to it.
4. **The decisive counterargument**: promoting `MonoType` to the sole representation would make the
   L2 syntax layer depend on the L3 semantic layer’s type. RFC-039 explicitly forbids L2 from
   depending on L3. One of this document’s goals is to eliminate reverse dependencies, so choosing a
   design that creates one is self-contradictory.

**Why `ir::Type` must be deleted outright rather than “retained”**:

The alias at `ir.rs:3` causes `BytecodeFunction.params` (`bytecode.rs:812`) and `return_type`
(`bytecode.rs:814`) to use `ir::Type`, while the serialization side uses `MonoType` at
`codegen/bytecode.rs:277-278`. The two field types are connected by
`file.type_table.into_iter().map(|t| t.into())` at `bytecode.rs:2339` and the lossy `From` bridge at
`bytecode.rs:2353-2390`.

**The sole reason the `From` bridge exists is that “the field types on the two sides differ.”**
After changing `bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, and `image.rs:44` to use
`MonoType`, `From<MonoType> for IrType` loses its reason to exist and can be removed in full
(eliminating the reverse reconstruction recorded in 2.2 as a side effect).

**Explicitly out of scope**: `Type` is not moved from `parser/ast.rs` to `sema/types/` in this
phase. This phase only adds `lower.rs` under `types/` as the sole conversion point, leaving the
definition of `Type` in place. The directory rename is performed as a separate move batch at the end
of P6 according to D1—at that point, `Type` belongs in the top-level `ast/type_.rs` (the AST’s
top-level domain), while `sema/types/` is the home of the `MonoType` working representation.

### 5.2 Per-Variant Disposition Table for All 26 Variants

> Disposition categories: **delete** (zero parser-forward construction), **retain** (has forward
> construction sites), and **migrate** (semantics moved to another node).

| #   | Variant                                                            | Definition line  | Disposition                                | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| --- | ------------------------------------------------------------------ | ---------------- | ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Retain** (strengthened)                  | The parser’s **sole primitive-type exit point** (`types.rs:162-165`). After the 12 primitive-type variants are removed, every primitive type takes this path. It needs a new annotation for the semantic kind of a name (see 5.3)                                                                                                                                                                                                                              |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:699` is a match arm). Bit width is carried by the `Name` path through `from_builtin_name`                                                                                                                                                                                                                                                                                                                           |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:702` is a match arm). **Note**: `bytecode.rs:2372`’s `IrType::String` consumes it, so the arm must be rewritten after removal                                                                                                                                                                                                                                                                                       |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:703-706` are match arms). As above, `bytecode.rs:2373` consumes it                                                                                                                                                                                                                                                                                                                                                  |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                                 | The only parser-forward construction point is the error fallback `types.rs:836` (`_ => (Vec::new(), Type::Void)`, registered as an itemized exemption). That fallback is removed together with the error-path refactoring in 6.4                                                                                                                                                                                                                               |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Retain**                                 | Has production consumers (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                                                                                                                                                                                                              |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Retain**                                 | Has production construction point `types.rs:364`                                                                                                                                                                                                                                                                                                                                                                                                               |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:743-746` are match arms); 2 AST→AST reconstruction points (`function.rs:377`/`606`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                        |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:747` is a match arm); zero across all three categories combined (the only globally zero-construction variant). Enums use `Struct` + `TypeBodyItem`; this is unrelated to `mono.rs:2378`’s `MonoType::Enum(_) => IrType::Void`                                                                                                                                                                                       |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Retain**                                 | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:451-454` | **Retain**                                 | Consumed by `bytecode.rs:2366`; **field reshaping `params: Vec<Type>` → `Vec<Param>` (name and type colocated) occurs with 6.8.1 (D54)**—the names are thereby inherited from `Assign.signature_params` (see 6.5.2)                                                                                                                                                                                                                                            |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:770` is a match arm); 2 AST→AST reconstruction points (`function.rs:427`/`600`) are rewritten in the same batch as 6.3.2. `types.rs:392-396` already explains that the `Generic` path is used                                                                                                                                                                                                                       |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:771` is a match arm); 2 AST→AST reconstruction points (`function.rs:431`/`602`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                            |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Retain**                                 | The actual representation of `Result` / `Option` / `String` / `Bytes` (`types.rs:397-399`)                                                                                                                                                                                                                                                                                                                                                                     |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                                 | Zero parser-forward construction (the syntax layer has no `::` path that produces it); **2 production AST→AST reconstruction points**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3 (registered in 6.3.2). **D8 remains unchanged**: the measurement is forward-oriented, and if associated-type syntax is enabled in the future, a new proposal at that time will define it |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:814` is a match arm); 2 AST→AST reconstruction points (`function.rs:467`/`632`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                            |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Retain**                                 | Has production construction point `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                                                                                                                                                                                                       |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Retain**                                 | Raw pointer type inside unsafe blocks                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Retain**                                 | Borrow notation                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Retain**                                 | Has production construction points `types.rs:100`, `declarations.rs:573`; RFC-010                                                                                                                                                                                                                                                                                                                                                                              |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Retain (and mark as compile-time only)** | Has production construction point `types.rs:160`. It is the only variant in `Type` that embeds an `Expr`, and the IR layer is forbidden from carrying it (see 6.1)                                                                                                                                                                                                                                                                                             |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Retain**                                 | RFC-004 currying’s layer terminator; `split_curry` depends on its existence                                                                                                                                                                                                                                                                                                                                                                                    |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                                | The semantics (binder name) move to the type checker, while the AST retains only syntax. See 5.6                                                                                                                                                                                                                                                                                                                                                               |

**Net effect: 26 → 13 variants** (13 deleted, 1 migrated, 12 retained).

> **The reverse bridge and AST→AST reconstruction must be handled before variants are deleted**:
> `function.rs:507-529` (`mono_to_ast_type`) recorded in 2.2 reconstructs 7 of items 2–8 in this
> table; in addition, the AST→AST reconstruction recorded in 2.2.2 covers 5 variants / 10 points in
> the 13-variant list (2 each for Union/Option/Result/Sum/AssocType). Both categories of
> reconstruction points must be handled in the same batch as 6.3.1; otherwise, behavior either
> changes silently (see 6.3 #47) or the code no longer compiles.

> **How `AssocType` is handled**: **D8 remains unchanged**, and it is deleted directly according to
> D8 (item 18 in the 5.2 table). The **measurement is forward-oriented**: zero forward construction
> holds because the syntax layer has no path that produces a `::` associated type. Therefore, D8
> need not be changed, and the variant does not enter the whitelist. Its **2 production AST→AST
> reconstruction points** (`function.rs:456`’s `substitute_type_ast` and `function.rs:623`’s
> `substitute_type_in_ast`) are registered in 6.3.2 and rewritten in the same batch as 6.3. T1’s
> three-category breakdown (5.7) has reproduced this conclusion (per-variant consistency of 26/26
> measured in commit 6.0.1).

### 5.3 `NameKind` and Elimination of the Synonym Table

**Problem**: `Type::Name { name: String }` uses an undifferentiated string to carry at least 4
meanings—concrete type / type variable / constraint name (operator interface) / predicate name.
`src/frontend/core/parser/statements/declarations.rs:500-501` must determine “is this a predicate?”,
lines 510-512 in the same file must determine “is this an operator interface?”,
`src/frontend/core/parser/ast.rs:971` must determine “is this an operator interface?”, and the two
`name_used_as_type*` implementations—`name_used_as_type_in` in the same file (measured at 864) and
`name_used_as_type` at `declarations.rs:42`—must determine “is this a type reference?”, all through
string comparison.

**Proposal: add a `kind` field to `Type::Name`, classified once by the parser.**

```rust
// Target form (illustration; line numbers are post-change)
Name {
    name: String,
    kind: NameKind,   // New
    span: Span,
}

enum NameKind {
    Builtin,      // Built-in type name: Int / Float / Bool / ... → passed to from_builtin_name
    UserType,     // User-declared type name (including Generic's std.result / std.option)
    TypeVar,      // Type variable: T / K / V
    Constraint,   // Operator interface / constraint name: Add / Ord / ...
    Predicate,    // Compile-time predicate: Terminates / Sorted / ...
}
```

**Key design point: classifying `NameKind` requires exactly the information the parser does not
currently have** (the type environment, the declared operator-interface table, and the declared
predicate table). Therefore, this field **cannot be classified independently inside the parser**—it
must be classified using an injected environment. This leads directly to 5.4.

**Disposition of the synonym table**:

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`, measured at 841-867 / **13 arms**)
is **retained, but its semantics are downgraded**. Before the change, it was a disambiguator
bridging two equivalent AST representations; after `Type::Int(64)` is removed, “Int” has only one
representation path, reducing it to the **sole name→type resolution table**.

The specific reasons it cannot be deleted are:

- The comment on `"DateTime" | "datetime" => Some(MonoType::Int(64))`
  (`src/frontend/core/types/mono.rs:850`) records a real fix (related issue #338: `now()`’s return
  value could not be passed to `format_time`). This is **alias semantics**, not the same thing as
  the `Int`/`int` capitalization alias.
- It is the **sole cut point** between standard-library type names and language built-in names.

The table’s **redundancy can be removed**, however. After the change, the parser produces only
canonical capitalization (`Int` / `Int32` / `Float` / `Bool` / ...), so:

| Category                                               | Current state (`src/frontend/core/types/mono.rs:843-864`) | Target                                                                                                                                                                                       |
| ------------------------------------------------------ | --------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Capitalization aliases (`"int"` / `"i64"` / `"int64"`) | Retained                                                  | **Delete**—the parser will not produce them after normalization                                                                                                                              |
| Abbreviation aliases (`"i64"` / `"i32"` / `"f64"`)     | Retained                                                  | **Move to the lexer layer** (register them as equivalent spellings of `Int` in the keyword table at `src/frontend/core/lexer/state.rs`); `from_builtin_name` recognizes only canonical names |
| `DateTime` alias                                       | Retained (`mono.rs:850`)                                  | **Retain**—semantic alias, with a comment                                                                                                                                                    |
| `"()"` (`mono.rs:863`)                                 | Retained                                                  | **Delete**—after the `Type::Void` fallback (`src/frontend/core/parser/statements/types.rs:836`) is removed, there is no source that produces it                                              |

**Net effect**: `from_builtin_name` converges from **13 match arms** to **13 canonical names**—**the
number of arms does not decrease** (each canonical name still needs its own arm); what is removed
are **25 alias spellings**. Its semantics change from “disambiguation table” to “resolution table,”
and it becomes the **single source of truth** for type names (validated by the 5.7 gates).

**Alias-counting definition**: **25** is the authoritative mechanical count (the in-arm `Int64` /
`Float64` entries and all other capitalization and abbreviation aliases are included). If only
spellings differing from the canonical name in capitalization are counted, the result is 23; the
difference is exactly the in-arm `Int64` / `Float64` entries. Both definitions refer to the same
table, and **the alias-removal scope is unaffected**.

**Consolidating the two `name_used_as_type*` implementations**: the duplicated implementations
recorded in 2.4 must be merged into one; otherwise, the classification information provided by
`NameKind` will be bypassed by two independent string-comparison implementations. The merged
implementation belongs in `parser/ast.rs` (the data source is near `Type`), and `declarations.rs`
calls it instead. The `is_predicate` short-circuit reads `NameKind::Predicate`, while `Struct` /
`NamedStruct` field recursion uses the newer and more complete version at `ast.rs:894-905`.

**Ownership of `CONST_PARAM_TYPES`**: the const generic parameter-name table `CONST_PARAM_TYPES`
(`src/frontend/core/parser/ast.rs`, measured at 855-858) partially overlaps the full set handled by
`from_builtin_name` (see 2.3). After `NameKind::Builtin` is introduced, the decision in
`extract_generic_param_names` (same file, measured at 932) can be changed to “`kind == Builtin` and
the name is in the acceptable const-parameter set,” explicitly connecting the two sources of
knowledge. The definition of the const-parameter subset is validated by the T2 gate (see 5.7).

> **Design judgment**: if `.yx` corpus files contain type annotations such as `i64` / `int`,
> normalizing them to `Int` changes those files’ **diagnostic positions** (not their diagnostic
> codes). This is permitted under the C3 criterion (messages and ordering may be normalized), but
> the baseline must be recorded in phase 1. **Measurement (P6 6.0, 2026-10-27)**: a full scan of
> `tests/yaoxiang` (316 `.yx` files), `tests/yaoxiang-multifile`, and `src/std` found **0
> non-canonical names in actual type positions**; 16 raw token matches were adjudicated individually
> as 12 strings and 4 comments (both instances of `i64` / `f64` were in comments). Therefore, name
> normalization in 6.1 **does not change the diagnostic position of any corpus file**, and **there
> is no need to regenerate the baseline**.

### 5.4 Data Flow for Predicates and Type Names

**Current data flow** (an internal parser loop plus 2 out-of-bounds references):

```
ast.rs (extract_generic_param_names, :930)  ─┐
declarations.rs:509                          ─┴─→ typecheck::operator_interfaces::spec()   ✗
declarations.rs
  ├─ declare_predicate()      ← parser_state.rs:46（parser 自持，仅本遍已解析的声明）
  ├─ n == "Terminates"        ← 硬编码字面量（declarations.rs:499）
  └─ （上述越界调用）
```

**Target data flow** (injection, no boundary violation):

```
L1 编排层（入口处一次性构造）
   │  ① 编译内置运算符接口表
   │  ② 编译 std 的 .yx 声明 → 谓词名集合
   ▼
TypeEnvProbe（trait，定义在 L2，src/frontend/core/parser/probe.rs）
   │  fn is_predicate(&self, name: &str) -> bool
   │  fn is_constraint(&self, name: &str) -> bool
   ▼
ParserState（parser_state.rs:46/54 处扩展，改为委派给 probe）
   ▼
declarations.rs:498-511 与 ast.rs:930 改为
   let is_predicate_app = |n: &str| state.probe().is_predicate(n);
   if state.probe().is_constraint(n) { ... }
```

**Per-file changes**:

| File                                                                                 | Change                                                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/probe.rs` (new)                                            | Defines `trait TypeEnvProbe`, providing four pure query methods: `is_predicate` / `is_constraint` / `is_type_var` / `is_builtin_type`. **The trait is defined in L2, and the implementation is supplied by L3**—this is legitimate dependency inversion                         |
| `src/frontend/core/parser/parser_state.rs:46-58`                                     | Retains `declare_predicate` / `is_predicate_name` (for accumulation during the current pass), and adds a `probe: Box<dyn TypeEnvProbe>` field and a `probe()` accessor                                                                                                          |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                        | Removes `n == "Terminates"`; changes to `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                                                         |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                        | Removes the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; changes to `state.probe().is_constraint(n)`                                                                                                                                                  |
| `src/frontend/core/parser/ast.rs:930`                                                | Removes the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` is the definition site for `Type` / `Expr` / `StmtKind` and must not hold an environment dependency—the function must instead accept a `&dyn TypeEnvProbe` parameter (see below) |
| All 5 parser construction points (`Parser::new` / `ParserState::new` in `parser.rs`) | Accept a `probe: Box<dyn TypeEnvProbe>` parameter. **The default implementation `NullProbe` (all queries return false)** ensures that parser unit tests remain unaffected                                                                                                       |

**The `extract_generic_param_names` signature issue**: `ast.rs:891`’s
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` is a free function
that receives no context. After the out-of-bounds call at `ast.rs:930` is removed, it needs an
`is_constraint` decision, so its signature must add `probe: &dyn TypeEnvProbe`. Callers (including
`parser/tests/ast.rs`) must be updated to pass `&NullProbe`.

**When `NameKind` is classified**: when `types.rs:162-165` constructs `Type::Name`, it uses
`state.probe()` to determine `kind`, fixing the classification once so that all later match arms no
longer perform string comparisons.

**This eliminates all 3 entries in RFC-039 routing table C**—2 of them involve only functions
internal to the parser under the definition in 2.4; the actual cross-layer references are the 2
locations `declarations.rs:509` and `ast.rs:930`, both covered by this section.

### 5.5 Merging `Expr::FnDef`

**Proposition: remove `Expr::FnDef` (`ast.rs:37-43`); function definitions uniformly use
`Expr::Lambda` + `StmtKind::Assign`.**

**This is already the active path**: `nud.rs:425-429` + `declarations.rs:462-479` (see 2.6). The
`FnDef` branch is a parallel implementation that never executes.

**Change list** (12 consumption sites + 2 exhaustive-match arms, listed individually):

| Location                             | Existing arm                                         | Target                                                                                                                                       |
| ------------------------------------ | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `spawn/placement.rs:122`             | `Expr::FnDef { body, .. } => self.check_block(body)` | Merge into the existing `Expr::Lambda` arm (`body` is also a `Box<Block>`; **delete this arm directly**)                                     |
| `spawn/analysis.rs:912`              | `Expr::FnDef { body, .. } => { ... }`                | Same; merge into the Lambda arm                                                                                                              |
| `formatter/handlers/expr.rs:43`      | `Expr::FnDef { ... }`                                | Format the Lambda in the `Assign` instead; remove this arm                                                                                   |
| `orchestrator.rs:1492`               | `if let Expr::FnDef { name, .. }`                    | Obtain the name from `StmtKind::Assign { target, .. }` instead                                                                               |
| `ir_gen.rs:4325`                     | `ast::Expr::FnDef { span, .. } => *span`             | Merge into the Lambda arm (`Lambda` also has a `span`)                                                                                       |
| `ir_gen.rs:5097`                     | `\| ast::Expr::FnDef { span, .. }`                   | Remove this or-pattern                                                                                                                       |
| `checker.rs:1642`                    | `if let ...::Expr::FnDef {`                          | Change the test to `Expr::Lambda`                                                                                                            |
| `checker/semantic_tokens.rs:1416`    | `Expr::FnDef {`                                      | Remove this arm                                                                                                                              |
| `inference/expressions.rs:3374`      | `...::Expr::FnDef {`                                 | Change the test to `Expr::Lambda`                                                                                                            |
| `inference/existential.rs:55`        | `\| Expr::FnDef { span, .. }`                        | Remove this or-pattern                                                                                                                       |
| `passes/dead_code.rs:305`            | `Expr::FnDef {`                                      | Change the test to `Expr::Lambda`                                                                                                            |
| `layers/ownership.rs:1207`           | `\| Expr::FnDef { span, .. }`                        | Remove this or-pattern                                                                                                                       |
| `layers/termination.rs:752`          | `Expr::FnDef {`                                      | Change the test to `Expr::Lambda`; update the comment at `termination.rs:2303` (“`fn(): Never` — FnDef.return_type is the bare return type”) |
| `ast.rs:1026`                        | `\| Expr::FnDef { span, .. }`                        | Remove this or-pattern                                                                                                                       |
| `pratt/mod.rs:47`                    | `Expr::FnDef { span, .. } => *span`                  | Remove this arm                                                                                                                              |
| `typecheck/tests/checker.rs:109-111` | Test constructs `Expr::FnDef`                        | Rewrite as `Expr::Lambda` + `Assign`                                                                                                         |
| `parser/tests/ast.rs:837-847`        | Test constructs `Expr::FnDef`                        | Remove this test                                                                                                                             |

**No behavior changes**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, and `span`,
structurally matching the first three fields of `FnDef`. The `name` field unique to `FnDef` has no
use at the `Expr` layer (the name is on `StmtKind::Assign.target`).

**Risk and mitigation**: non-exhaustive `if let` matches such as `checker.rs:1642` and
`inference/expressions.rs:3374` become impossible matches after the change, with no error or
warning. Mitigation: the type-table gate (5.7) gains a “zero-construction variant” check. After
`Expr::FnDef` is removed, reintroducing an `if let Expr::FnDef` fails compilation directly because
the variant no longer exists—**this is guaranteed at compile time and requires no gate**.

### 5.6 Disposition of `signature_params` and `NamedParen`

**`StmtKind::Assign.signature_params` (`ast.rs:244-245`) → delete.**

Rationale: its reason for existing (the comment at `ast.rs:244`) is “for typechecker
`classify_generic_params` use”—**a dedicated slot reserved by the statement AST for the type
checker**. However, `extracted_params` at `declarations.rs:500-511` in the same function has already
computed `value_params` (after type-position/constraint-position filtering), so carrying it in the
statement is duplicate transport.

Change: remove the field at `ast.rs:244-245` → remove the argument at `declarations.rs:472` → change
the source of `classify_generic_params` to an in-place recomputation (it can access `Lambda.params`
in `value: Option<Box<Expr>>`). **Net removal: one field declaration, one construction site, and one
transport path.**

> **Risk**: `signature_params` contains the **unfiltered** parameter list, while `Lambda.params` is
> **filtered** (the filter at `declarations.rs:500-511` removes type-position and
> constraint-position parameters). If `classify_generic_params` depends on the unfiltered version,
> the behavior will change after the refactor. **The actual input used by `classify_generic_params`
> must be verified in phase 1**; equivalence must not be assumed.

**`Type::NamedParen` (`ast.rs:533-540`) → retain the node, but move binder semantics out of it.**

The comment at `ast.rs:519-532` records its **sufficient reason for existence** (see 2.7): in
`(r: P(m))`, dropping `r` would leave the type checker to guess that “a free variable not in scope
in the constraint is the return formal parameter,” which would **silently substitute the undeclared
`m` as a binder** (the comment marks this as a “measured defect”).

Therefore, **it cannot simply be deleted**—that would reintroduce an already-fixed correctness
defect. The correct disposition is:

1. Retain `NamedParen { param, param_span, inner }` as a **pure syntax node**; `param`/`param_span`
   carry source facts only and do not bear type-layer binder semantics.
2. Move binder interpretation to `sema/`: the type checker **explicitly** extracts `param` as the
   binder when consuming `NamedParen`. This is “reading a syntax node’s field,” analogous to
   `ConstExpr` embedding an `Expr`—the syntax node supplies facts, and the semantic layer interprets
   them.
3. Rewrite the comment from “the type checker can only guess” to “this node supplies the binder
   name, and semantic interpretation takes place in `sema/`.”

**Why this is “migration” rather than “deletion”**: the syntax tree must represent source code
without loss. `r` genuinely exists in the source, so the AST cannot pretend it does not. The problem
is not the node’s existence, but the failure to define responsibility for semantic interpretation
clearly.

### 5.7 Build-Time Type-Table Gates

**The reference implementation has already been validated in this repository**: `build.rs:19-37`
invokes `parse_registry` / `validate` from `tools/code-tables` to perform uniqueness, range, and
Chinese-completeness validation over 145 error codes, directly `panic!`ing to reject compilation
when `is_ok()` is false; `build.rs:39-54` further compares the Markdown code table in RFC-013 item
by item and likewise `panic!`s on any mismatch. **RFC-013’s documentation and code therefore always
remain consistent**—`tools/code-tables/Cargo.toml:15-16` has only one dependency, `serde_json`, and
the validation logic is text-level parsing with no dependence on `syn`.

The type tables currently have **no** equivalent gate. This document adds a mechanism of the same
form.

**New crate**: `tools/type-tables` (its `Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). Four checks:

| #                                                                   | Check                                                                                                                                                                                                                                                                              | Inputs                                   | Failure condition                                                                                                                                                                                                                                                                                                                                                   | Remedy                                                                                                                |
| ------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| **T1 Construction-site breakdown**                                  | Scan the `pub enum Type` variant list in `src/frontend/core/parser/ast.rs`; under `src/`, count each variant’s construction sites in **three separate categories** (① parser-forward ② AST→AST reconstruction ③ reverse-bridge reconstruction; excluding `tests/` and `*/tests/*`) | Variant list + three-category hit tables | ① A variant has **zero forward construction** and is not registered; ② a registered variant gains a forward construction site; ③ a registered wildcard-arm fallback exemption point **is not hit**; ④ an **unregistered wildcard-arm fallback point exists** (fallback exemptions must be registered individually and must not be generalized to all wildcard arms) | Delete the variant; or register a rationale / remove an obsolete registration (see 5.2 and `REGISTERED_ZERO_FORWARD`) |
| **T2 Synonym table consistent with the repository-wide name lists** | The `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` at `ast.rs:838-843` + the LSP built-in type-name list at `src/lsp/world.rs:176`                                                                                                                       | Three tables                             | A type name exists in one table but is not recognized by another (compared according to each table’s own semantic mapping rules)                                                                                                                                                                                                                                    | `type-tables --fix` fills the gaps in both directions                                                                 |
| **T3 Built-in type names consistent with RFC-011**                  | The canonical-name set from `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                             | Two tables                               | The code-table ranges/names differ from the registry                                                                                                                                                                                                                                                                                                                | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --fix`                                                     |
| **T4 Exhaustiveness of `Type → MonoType`**                          | The `Type` variant list vs. the match arms in `types/lower.rs`                                                                                                                                                                                                                     | Two tables                               | A variant has no corresponding arm, or an arm has no corresponding variant                                                                                                                                                                                                                                                                                          | Add an arm / remove an arm                                                                                            |

**T4 implementation note**: Rust’s exhaustive match already guarantees the “variant → arm” direction
at compile time. T4’s value lies in the **opposite** direction—detecting stale arms in `lower.rs`
that refer to already-deleted variants. Such code fails to compile when the variant is deleted, so
T4 is in practice redundant. **T4 is therefore downgraded to a CI assertion (`cargo build` success
is sufficient) and does not enter `build.rs`’s `panic!` path.** T1–T3 are the three items that
genuinely require text parsing.

**T1’s two implementation difficulties and mitigations**:

1. Distinguishing **construction sites from match arms** requires syntactic analysis. The repository
   root `Cargo.toml` has **no `syn` dependency**, and `tools/code-tables` likewise has only
   `serde_json`. Mitigations:
   - **Preferred**: add `syn` to `tools/type-tables` (only that crate; the root crate remains
     unchanged). `syn` is standard practice, and the gate crate is independent of the compiler.
   - **Fallback**: reuse the existing line-level heuristic in `tools/extract_arm.py`: construction
     sites occur on the right-hand side of `=` or in function-argument position, while match arms
     occur on the left-hand side of `=>`. Approximate the distinction by counting occurrences of
     `=>`.
   - **Conservative starting point**: when T1 first ships, **report only and do not fail**
     (`cargo:warning`); take a full baseline and confirm the heuristic is correct before changing it
     to `panic!`. This is consistent with the approach of “CI gates may initially be red and need a
     baseline first.”
2. **Reverse bridges and AST→AST reconstruction points must be classified separately**. 2.2 records
   the two `MonoType → ast::Type` reconstructions at `bytecode.rs:2353-2390` and
   `function.rs:507-529`; 2.2.2 records the AST→AST reconstruction in `function.rs`’s two
   replacement functions (15 variants / 28 points, including 5 variants / 10 points from the
   13-variant list). If T1 conflates them with forward construction, `Int` / `Float` / `Bool` /
   `Char` / `Void` / `String` / `Bytes` and `Union` / `Option` / `Result` / `Sum` / `AssocType` will
   be judged to have construction sites and exempted from deletion, directly contradicting the
   disposition table in 5.2. T1’s report must list the three categories separately, and **fallback
   exemptions apply only to individually registered entries** (currently only `Void`’s parser error
   fallback at `types.rs:836`), never to all wildcard arms.

**Gate coverage**: add a `type_tables::validate(root, &entries)` call to `build.rs`, immediately
after the existing block at `build.rs:19-37`, following the
`panic!("类型表校验失败（{} 项 error ...）", ...)` pattern.

## Detailed Design

### 6.1 Impact on the Type System

| Affected area                                                | Assessment                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact.** Inference consumes `MonoType` throughout and does not touch `ast::Type` variants                                                                                                                                                                                                             |
| Solver (`types/solver.rs`)                                   | **No impact.** `pub fn unify` (`solver.rs:411`), `substitute` at `types/substitute.rs:112`, and `substitute` at `types/mono.rs:649` all operate on `MonoType`                                                                                                                                               |
| Constant evaluation (`types/eval/`)                          | **No impact.** `eval/const_eval.rs` (1,128 lines), `eval/dependent_types.rs` (`check_structural_termination` at `478`), and `unify` at `eval/reducer.rs:572` all operate on `MonoType`                                                                                                                      |
| Monomorphization (`middle/passes/mono/`)                     | **Changes required.** Eight locations—`mono/function.rs:358`, `362`, `451`, `456`, `554`, `558`, `618`, `623`—match `AstType::NamedStruct` / `AstType::AssocType`. If `AssocType` is deleted, all 8 must change together. **`mono_to_ast_type` at `function.rs:507-529` must also change** (see #47 in 6.3) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Changes required.** Matches on `ast::Type::NamedStruct` at `ir_gen.rs:1266` and `1399`; `let params: Vec<MonoType> = signature_params` at `ir_gen.rs:1380` is another consumer of `signature_params`, which must be checked as part of 5.6                                                                |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Changes required.** `type_id_to_monotype` at `codegen/bytecode.rs:492-495`, the encoding loop at `351-352`, and assembly at `632`                                                                                                                                                                         |
| Interpreter (`backends/interpreter/`)                        | **Changes required.** `type_table: Vec<ir::Type>` at `image.rs:44` changes to `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, and the change in element type affects the `{:?}` output format                                                                             |
| RFC-027 compile-time types                                   | **Changes required.** `Type::ConstExpr` is retained but marked **compile-time only**; it disappears after `Type → MonoType` (`types/lower.rs`) and is never visible to the IR                                                                                                                               |

**IR-layer type discipline (to be inherited by 04-ssa)**: after convergence, IR type annotations may
use only the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only variants** and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. This discipline must be
enforced by 07’s first-tier validator’s “type consistency” invariant. `ast::BinOp::Assign` /
`ast::UnOp::Deref` (2.5) are expression-side counterparts—they may appear inside `ConstExpr`, and
therefore must likewise be excluded from IR type annotations.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** The basis is:

1. The 13 deleted variants have no construction sites in the **parser-forward** direction (the one
   forward point for `Void` is an error fallback, registered as an individual exemption) → the
   parser does not produce them (the reverse-bridge and AST→AST reconstruction points are handled
   through 6.3’s #28 / #47 and the 10 points in 6.3.2).
2. `ir::Type` is an alias for `ast::Type` → changing it back to `MonoType` does not change the set
   of types, only the **carrier**.
3. The **loss** of `From<MonoType> for IrType` (8 variants → `Void`) no longer occurs after the
   change—this **may** change the behavior of some programs (types that were previously lost are now
   retained). **This is the only direction in which behavior can change, and it is a fix rather than
   a regression**, but it must be confirmed by 07’s third-tier corpus differential.

**Semantic change points requiring focused attention**:

| Location                | Current state                                                                                 | After convergence                         |
| ----------------------- | --------------------------------------------------------------------------------------------- | ----------------------------------------- |
| `bytecode.rs:2371-2376` | `Generic` other than `String`/`Bytes`/`Tuple` → `IrType::Void`                                | Retain the real `MonoType::Generic`       |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Retain the real `MonoType`                |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` fallback                                                                  | **Delete** (an exhaustive match suffices) |

> **Design judgment**: the fact that all 8 variants currently collapse to `Void` means that **all
> code paths depending on signature bytecode (the interpreter’s arity/type checks and the
> serialization format) currently see `Void` everywhere**. After convergence, these locations will
> see the real types. **This is the only possible source of behavioral change in this document, and
> its direction is “restore correct information.”** If the corpus differential reveals a difference,
> it should be treated as **an existing defect being exposed**, not as a regression introduced by
> this document—the method is to construct a `.yx` program containing a `Ref` parameter and compare
> `dump_bytecode` before and after the change.

### 6.3 Compiler Change List

| #   | File                                                                   | Lines                                  | Change                                                                                                                                                                                                                                           | Category                 |
| --- | ---------------------------------------------------------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------ |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472` | Delete the `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants                                                                                                                                          | Pure deletion            |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                              | Add a `kind: NameKind` field to `Name`                                                                                                                                                                                                           | Data flow                |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43`                                | Delete `Expr::FnDef`                                                                                                                                                                                                                             | Pure deletion            |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                              | Delete `Assign.signature_params`                                                                                                                                                                                                                 | Pure deletion            |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026`                                 | Remove the `Expr::FnDef` or-pattern                                                                                                                                                                                                              | With #3                  |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                              | Rewrite the `NamedParen` comment as “syntax supplies the binder name; semantic interpretation takes place in `sema/`”                                                                                                                            | Documentation            |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                        | Add `pub enum NameKind`                                                                                                                                                                                                                          | Addition                 |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                    | `trait TypeEnvProbe` + `NullProbe`                                                                                                                                                                                                               | Addition                 |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                | Add the `probe` field and accessor                                                                                                                                                                                                               | Data flow                |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                              | Use `state.probe()` to determine `NameKind`                                                                                                                                                                                                      | Data flow                |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                              | Update the comment (`Result`/`Option` use `Generic`, already consistent with the facts; only add the variant-removal note)                                                                                                                       | Documentation            |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                  | Delete the `_ => (Vec::new(), Type::Void)` fallback and return an explicit error instead                                                                                                                                                         | Behavior                 |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                              | Delete `n == "Terminates"` and use `state.probe().is_predicate(n)`                                                                                                                                                                               | Reverse dependency       |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                              | Delete the `typecheck::operator_interfaces::spec()` call                                                                                                                                                                                         | Reverse dependency       |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                  | Delete the `signature_params` argument                                                                                                                                                                                                           | With #4                  |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                   | Remove the `Expr::FnDef` arm                                                                                                                                                                                                                     | With #3                  |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                  | Retain (the point at which the parameter list is recovered; with 5.5, read the Lambda directly)                                                                                                                                                  | With #3                  |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814` | Delete 13 match arms (one for each deleted variant)                                                                                                                                                                                              | With #1                  |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                              | Remove synonyms (capitalization aliases, `i64`-style abbreviations, `"()"`)                                                                                                                                                                      | Convergence              |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                  | Retain the `DateTime` alias                                                                                                                                                                                                                      | Retain                   |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                    | Move `From<Type> for MonoType` (currently at `mono.rs:692-830`) to become the sole conversion point                                                                                                                                              | Addition                 |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                          | Register `i64`/`int` and similar spellings as equivalent forms of `Int`                                                                                                                                                                          | Convergence              |
| 23  | `src/middle/core/ir.rs`                                                | `3`                                    | Delete `pub use ... ast::Type`                                                                                                                                                                                                                   | Pure deletion            |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                  | Keep `FunctionCode.params` as `Vec<MonoType>` (already the case)                                                                                                                                                                                 | —                        |
| 25  | `src/middle/core/bytecode.rs`                                          | `812`, `814`                           | Change `BytecodeFunction.params` / `return_type` to `MonoType`                                                                                                                                                                                   | Convergence              |
| 26  | `src/middle/core/bytecode.rs`                                          | `854`                                  | Change `BytecodeModule.type_table` to `Vec<MonoType>`                                                                                                                                                                                            | Convergence              |
| 27  | `src/middle/core/bytecode.rs`                                          | `2339`                                 | Change the `type_table` conversion to `.collect()` (elements now have the same type)                                                                                                                                                             | With #26                 |
| 28  | `src/middle/core/bytecode.rs`                                          | `2352-2390`                            | **Delete the entire `From<MonoType> for IrType` impl**                                                                                                                                                                                           | Pure deletion            |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                   | Change `type_table` to `Vec<MonoType>`                                                                                                                                                                                                           | Convergence              |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 locations)              | Handle the `AstType::AssocType` arms according to the gate’s conclusion                                                                                                                                                                          | Pending                  |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`           | Type-table gates T1–T3                                                                                                                                                                                                                           | Gate                     |
| 32  | `spawn/placement.rs`                                                   | `122`                                  | Remove the `Expr::FnDef` arm                                                                                                                                                                                                                     | With #3                  |
| 33  | `spawn/analysis.rs`                                                    | `912`                                  | Same                                                                                                                                                                                                                                             | With #3                  |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                   | Same                                                                                                                                                                                                                                             | With #3                  |
| 35  | `orchestrator.rs`                                                      | `1492`                                 | Same; obtain the name from `Assign.target` instead                                                                                                                                                                                               | With #3                  |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                         | Same                                                                                                                                                                                                                                             | With #3                  |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                 | Same                                                                                                                                                                                                                                             | With #3                  |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                 | Same                                                                                                                                                                                                                                             | With #3                  |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                 | Same                                                                                                                                                                                                                                             | With #3                  |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                   | Same                                                                                                                                                                                                                                             | With #3                  |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                  | Same                                                                                                                                                                                                                                             | With #3                  |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                 | Same                                                                                                                                                                                                                                             | With #3                  |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                          | Same + comment update                                                                                                                                                                                                                            | With #3                  |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                              | Remove the `Expr::FnDef` test                                                                                                                                                                                                                    | Test                     |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                              | Rewrite as Lambda + Assign                                                                                                                                                                                                                       | Test                     |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`        | Remove the lower assertions for the 11 dead variants                                                                                                                                                                                             | Test                     |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                              | Remove the `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches from `mono_to_ast_type` and use `AstType::Name` + `mono.type_name()` uniformly. **Must be processed in the same batch as #1**                                   | Convergence              |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                           | Add `probe: &dyn TypeEnvProbe` to the `extract_generic_param_names` signature; delete the `typecheck::operator_interfaces::spec()` call and use `probe.is_constraint(name)` instead. Callers (including `parser/tests/ast.rs`) pass `&NullProbe` | Reverse dependency       |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                | Delete the local `name_used_as_type` and call the merged version in `ast.rs`; change the `is_predicate` short-circuit to read `NameKind::Predicate`                                                                                              | Duplicate implementation |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                  | Connect the `CONST_PARAM_TYPES` decision through `NameKind::Builtin` (see 5.3)                                                                                                                                                                   | Convergence              |

> **The gate itself must also be gated**: `tools/type-tables` and `code-tables` both require `mod`
> declaration wiring (the repository has already learned this lesson from 8 orphan test trees).
> Register the new crate in the root `Cargo.toml` at the same time.

### 6.4 Backward-Compatibility Strategy

**Constraint**: the `tests/yaoxiang/` corpus of 293 single-file `.yx` programs + the multi-file
corpus layer established in P2 (`tests/yaoxiang-multifile/`) + `src/std/tests/*.yx` + the standard
library **must all preserve behavior** (identical diagnostic codes and messages; ordering may be
normalized).

| Risk                                                                | Assessment                                                                                                                                                                                                                                                                                                            | Mitigation                                                                                                                                                                                                                                                                                                             |
| ------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source code contains type annotations such as `i64` / `int` / `f64` | After normalization to `Int`, `from_builtin_name` recognizes only canonical names                                                                                                                                                                                                                                     | **First task in phase 1**: scan the full corpus and count non-canonical type names. If the count is 0, the synonyms can be deleted directly; otherwise, move them into the lexer alias table as 5.3 describes (**accepted by the syntax layer and normalized by the lower layer**, with no impact on diagnostic codes) |
| Source code contains `DateTime`                                     | Retain the alias (`mono.rs:627`)                                                                                                                                                                                                                                                                                      | No impact                                                                                                                                                                                                                                                                                                              |
| `("()")` appears in type position                                   | The `LParen` branch beginning at `types.rs:167` handles tuples/parameter groups and produces a `Tuple`, not `Name{"()"}`                                                                                                                                                                                              | Before deleting `"()"`, count occurrences of `"()"` as a type name throughout the corpus                                                                                                                                                                                                                               |
| `dump_bytecode` output changes                                      | The `type_table` element type changes from `ir::Type` to `MonoType`, resulting in a different `{:?}` format                                                                                                                                                                                                           | **Permitted by the C3 criterion** (07 specifies that `dump_bytecode` is compared only in the C1/C2 phases). Record the output diff in phase 2 for manual review                                                                                                                                                        |
| Behavior changes after the lossy bridge is fixed                    | See 6.2                                                                                                                                                                                                                                                                                                               | Corpus differential + focused `dump_bytecode` comparison                                                                                                                                                                                                                                                               |
| Results change after `mono_to_ast_type` is rewritten                | `function.rs:507-529` is the exit point for monomorphized type substitution (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back to `AstType`). After the variants are removed, taking the `_` arm changes the result from `AstType::Int(64)` to a form such as `AstType::Name { name: "i64" }` | Phase 3 focused comparison: select corpus files containing generic substitution and compare the `type_name()` of the monomorphized products before and after the change. **Do not assume `{:?}` output equivalence**                                                                                                   |
| `.yx` source files themselves                                       | **Zero modifications**                                                                                                                                                                                                                                                                                                | This document requires no changes to any corpus or standard-library source file. This is a hard acceptance criterion                                                                                                                                                                                                   |

**Hard acceptance condition**: `git diff --stat tests/ src/std/` must be empty (except for additions
to the multi-file corpus layer and test baseline files).

## Implementation Essentials

The criteria uniformly use 07’s **C3: identical diagnostic codes and messages (ordering may be
normalized)**. The following is the phase breakdown for this workstream; see RFC-039 for the global
cross-document ordering.

| Phase                                         | Content                                                                                                                                                                                                                                                        | Acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Rollback point                         |
| --------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| **0. Gates first**                            | Create `tools/type-tables` and implement T1/T2/T3. The first version of T1 **reports only and does not fail**. Take a full baseline on **unmodified** code                                                                                                     | T1’s report accurately lists the 13 variants with **zero parser-forward construction** (including the classification of `AssocType`: 0 forward / 2 AST→AST, with D8 unchanged), and **separately lists the reverse-bridge reconstruction points from 2.2 and the AST→AST reconstruction points from 2.2.2**; the 13 zero-forward-construction variants and the 1 globally zero-construction variant (only `Enum`) are reported separately; T2/T3 are green or provide a repairable list | No code changes; no rollback required  |
| **1. Name normalization**                     | Count non-canonical type names in the corpus; remove synonyms from `from_builtin_name` (`mono.rs:620-642`) and move them into the lexer alias table. **At the same time, verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`** | C3: the full corpus’s diagnostic set is **identical item by item** to the phase 0 baseline                                                                                                                                                                                                                                                                                                                                                                                              | Separate commit; revertible            |
| **2. Bytecode type unification**              | Change `bytecode.rs:812`/`814`/`854` and `image.rs:44` to `MonoType`; delete `From<MonoType> for IrType` (`bytecode.rs:2352-2390`)                                                                                                                             | C3 + focused `dump_bytecode` comparison for every row in the table in 6.2                                                                                                                                                                                                                                                                                                                                                                                                               | Separate commit; revertible            |
| **3. Dead-variant deletion**                  | Delete the 13 variants according to the 5.2 table; remove match arms such as `mono.rs:699-708` and the `types.rs:836` fallback; **process `function.rs:507-529` (#47) and the 10 AST→AST points (6.3.2) in the same batch**; update `types/tests/mono.rs`      | C3 + `cargo build` fully green (exhaustive matches force completion of all arms) + focused generic-substitution comparison (last row of 6.4)                                                                                                                                                                                                                                                                                                                                            | Separate commit; revertible            |
| **4. Parser data flow**                       | Add `probe.rs`; change `parser_state.rs:46-58`; change `declarations.rs:498-511` and `ast.rs:930` (#48); determine `NameKind` at `types.rs:162-165`; consolidate the two `name_used_as_type*` implementations (#49)                                            | C3 + **new assertion**: `grep 'typecheck' src/frontend/core/parser/` must return 0                                                                                                                                                                                                                                                                                                                                                                                                      | Separate commit; revertible            |
| **5. Dead AST variants and dedicated fields** | Delete `Expr::FnDef` (16 locations) + `Assign.signature_params`; migrate `NamedParen` semantics                                                                                                                                                                | C3 + all 18 modules under `tests/integration/` green                                                                                                                                                                                                                                                                                                                                                                                                                                    | Two commits (FnDef / signature_params) |
| **6. Gates become hard failures**             | Change T1 from a warning to `panic!`; downgrade T4 to a CI assertion                                                                                                                                                                                           | `cargo build` **must fail** when a zero-construction variant is deliberately introduced (red test first)                                                                                                                                                                                                                                                                                                                                                                                | Separate commit                        |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Phases 3 and 5 must follow phase
2**—`bytecode.rs:2372-2373` consumes `IrType::String` / `IrType::Bytes`, so deleting the variants
first would break compilation. Phase 3’s #47 must be processed in the same batch as #1; otherwise,
`mono_to_ast_type` silently changes behavior.

**Dependency on downstream work**: [04-ssa.md](04-ssa.md) may begin only after all phases in this
document are complete. At that point, the element types of `BytecodeFunction.params`,
`FunctionCode.params`, and `type_table` are consistent, giving SSA type annotations a single landing
point.

### Mandatory Verification for Each Phase

| Phase | Required actions                                                                                                                                                      |
| ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`; manually verify the T1 report (**including the separate reverse-bridge reconstruction points**) |
| 1–5   | `cargo test`; full corpus differential over `tests/yaoxiang/` (07’s third tier); item-by-item comparison under the C3 criterion                                       |
| 2     | Additionally: construct one `.yx` sample for every variant in the table in 6.2 and compare `dump_bytecode` before and after the change                                |
| 3     | Additionally: for corpus files containing generic substitution, compare the `type_name()` of the monomorphized products before and after the change                   |
| 4     | Additionally: `rg 'typecheck' src/frontend/core/parser/` returns 0                                                                                                    |
| 6     | Additionally: deliberately write a zero-construction variant and confirm that `cargo build` fails (red → green)                                                       |

## Key Decisions and Rationale

### Decisions

| Decision                     | Resolution                                                                                                               | Rationale                                                                                                                                                                                                                                                 |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Target representation shape  | `Type` as the sole type structure + `MonoType` as the working representation; remove the `ir::Type` alias                | The alias at `ir.rs:3` is the direct cause of the mismatch between the `BytecodeFunction.params` and `FunctionCode.params` field types (2.1). After the alias is removed, the two field types are unified and the `From` bridge loses its reason to exist |
| Reject “unify on `MonoType`” | Not adopted                                                                                                              | It would create an L2→L3 reverse dependency (violating RFC-039’s layering) and require adding `Span` / `ConstExpr` back to `MonoType`. See the full argument in 5.1                                                                                       |
| Do not delete `NamedParen`   | Retain it as a pure syntax node and move binder semantics to `sema/`                                                     | `ast.rs:519-532` records that deletion would reintroduce an already-fixed correctness defect: `(r: P(m))` would silently substitute the undeclared `m` as a binder                                                                                        |
| Directory renaming           | **Do it** (D1), as a pure move batch at the end of P5/P6; this document’s paths are written against the pre-rename state | Decision of 2026-10-03 (ChenXu233); construction method clarified on 2026-10-04                                                                                                                                                                           |

### Rejected Alternatives

None of the four alternative approaches was adopted. **Retaining the three representations and
adding only a conversion layer** addresses none of the root causes; `ir::Type` would still not be an
independent type, and none of the dead variants, synonym table, or reverse dependencies would
disappear. **Generating the synonym table with macros** treats a symptom rather than the cause: once
`Type::Int(usize)` is removed, the macro has no reason to exist, and consistency would instead be
guaranteed by gate T2, whose correctness is more verifiable than macro expansion. **Deleting only
dead variants without unification** has the lowest cost but leaves the problem in place, with no
gate to prevent the next person from adding `Type::Int128(usize)` and growing the same class of dead
variant; if only one commit is possible, it should explicitly be defined as a subset of this
approach and deliver the T1 gate as well. **Unifying on `MonoType`** is addressed in 5.1.

### Benefits

- **Eliminates maintenance burden in 14 locations**. The 12 consumers of `Expr::FnDef` + 2
  exhaustive-match arms form a parallel path that never executes; removing it leaves `ir_gen.rs` /
  `ownership.rs` / `termination.rs` with one fewer branch each.
- **Eliminates one diverged duplicate implementation**. The two copies of `name_used_as_type` (2.4)
  have already diverged in their recursion over fields in a definition body; after consolidation,
  only one remains.
- **After the lossy bridge disappears, `IrType::Void` is no longer a “legal type value.”** This is
  the prerequisite for 07’s first-tier validator to make its “type consistency” invariant genuinely
  pass.
- **Gates make regression impossible**. T1 turns “adding a zero-construction variant” from today’s
  silent pass into a build failure—the existing `build.rs` mechanism in this repository proves that
  this approach works.
- **Zero source-code changes**. None of the 293 `.yx` corpus files or standard-library files require
  modification, which is the fundamental reason the C3 criterion can hold.

## Known Limitations and Risks

### Risks

- **Phase 2 may change behavior**. Once the lossy bridge is fixed, the 8 `MonoType` variants can
  expose their real types instead of `Void`. The direction is corrective, but it changes paths that
  depend on signature bytecode. This is the document’s largest uncertainty.
- **The classification of `AssocType` depends on gate accuracy**. If T1 uses a line-level heuristic
  (counting `=>`) rather than `syn`, it may misclassify. This is why manual verification is
  mandatory in phase 0.
- **If T1 omits reverse bridges, the disposition table and gate will contradict each other**. 2.2
  has proved that 7 “dead variants” have production reconstruction points in the reverse bridge. If
  T1 does not separate construction directions, those 7 variants will be judged to have construction
  sites and exempted from deletion—the gate would become an obstacle to the deletion work rather
  than a safeguard. | **2. Deleting `signature_params` carries semantic risk.** It carries the
  **unfiltered** parameter list, whereas `Lambda.params` is **filtered**. If
  `classify_generic_params` depends on the former, the refactor will change generic-parameter
  classification. **This must be verified first; equivalence must not be assumed** (the
  `signature_params` consumer at `ir_gen.rs:1380` must be checked as well). | **3. Phase 3 touches
  `middle/passes/mono/function.rs`.** Besides the 8 matches on `AssocType`, `mono_to_ast_type` at
  `507-529` is the exit point for monomorphized type substitution, and rewriting it may change the
  substitution result’s `type_name()` (last row of 6.4).
- **The number of non-canonical type names in the `.yx` corpus may be nonzero**. If corpus files use
  `i64` / `int` extensively, phase 1’s “delete synonyms directly” is infeasible and degrades to
  “move them into the lexer alias table,” reducing the convergence achieved by `from_builtin_name`.
- **The gate will initially be red**. If T1 launches with a direct `panic!`, it will immediately
  block compilation. It must first report only and establish a baseline.

### Open Questions

> **All open questions originally listed in this section have been adjudicated.** The item-by-item
> resolutions are in the
> [RFC-039 decision register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50). **This
> document leaves no pending items.**

## See Also

### Governing and Same-Batch Documents

- [RFC-039 Compiler Feature Routing Directory Design (Overview)](../../rfc/accepted/039-compiler-architecture.md)
  — Governing framework; four-layer model, criterion tiers, routing table C (3
  type-representation-related entries assigned to this document)
- [01-routing.md](01-routing.md) — Feature routing tables A/B/C, dependency-direction rules, target
  directory structure
- [04-ssa.md](04-ssa.md) — SSA conversion; this document’s prerequisite and downstream consumer
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — Front-end paradigm; convergence of the
  operator-change impact
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C3 category definition, diagnostic-set
  comparison specification, type-consistency invariant for the first-tier IR validator

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — Basis for the
  unified `Type::Generic` path
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — Built-in type table
  (comparison target for gate T3)
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — Example
  of a build-time gate in `build.rs`
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Origin of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

- `src/frontend/core/parser/ast.rs:427-541` — Definition of the 26 `ast::Type` variants
- `src/frontend/core/parser/ast.rs:519-540` — Explanation of `Type::NamedParen` binder semantics
  (evidence that deletion would reintroduce a defect)
- `src/frontend/core/parser/ast.rs:241-249` — The typechecker-specific field
  `StmtKind::Assign.signature_params`
- `src/frontend/core/parser/ast.rs:838-841` — `CONST_PARAM_TYPES`: the third copy of const generic
  parameter names
- `src/frontend/core/parser/ast.rs:843-885` — `name_used_as_type_in`: the duplicate of
  `declarations.rs:42` (already diverged)
- `src/frontend/core/parser/ast.rs:891`, `930` — `extract_generic_param_names` and its out-of-bounds
  call
- `src/frontend/core/parser/ast.rs:191-213`, `217-223` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — The parser represents every primitive
  type as `Type::Name`
- `src/frontend/core/parser/statements/types.rs:392-396` — `Result`/`Option` are not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:836` — The only forward construction point of
  `Type::Void` (error fallback)
- `src/frontend/core/parser/statements/declarations.rs:42-80` — The original `name_used_as_type`
- `src/frontend/core/parser/statements/declarations.rs:494-511` — Hard-coded `"Terminates"` and the
  `typecheck::operator_interfaces::spec()` call
- `src/frontend/core/parser/pratt/nud.rs:505-514` — Expression position is represented as an
  empty-body `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:466` — The `expr_to_params` arm that recovers the parameter
  list
- `src/frontend/core/types/mono.rs:618-643` — The `from_builtin_name` synonym table
- `src/frontend/core/types/mono.rs:692-830` — The `From<Type> for MonoType` implementation (12 of
  the 13 match arms are here)
- `src/frontend/core/types/mono.rs` — Definition of `MonoType` (approximately 22 variants, beginning
  at `mono.rs:167`; 1,077-line file)
- `src/frontend/core/types/const_data.rs:234-258`, `321-330` — `const_data::BinOp` /
  `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — Compile-time constant evaluation (1,128 lines)
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- `src/middle/core/ir.rs:3` — Evidence that `ir::Type` is an alias for `ast::Type`
- `src/middle/core/bytecode.rs:808-814` — The `Vec<ir::Type>` fields of `BytecodeFunction`
- `src/middle/core/bytecode.rs:2352-2390` — The lossy `From<MonoType> for IrType` bridge
- `src/middle/passes/codegen/bytecode.rs:32`, `275-278` — The `Vec<MonoType>` fields of
  `FunctionCode`
- `src/middle/passes/mono/function.rs:507-529` — `mono_to_ast_type`: the reverse bridge that
  reconstructs 7 “dead variants”
- `src/backends/interpreter/image.rs:44` — The interpreter module’s `type_table`
- `build.rs:19-37` — Build-time error-code registry gate (the model for the type-table gate)
- `build.rs:39-54` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — The gate crate depends only on `serde_json` (the root crate
  has no `syn`; `tools/type-tables` introducing `syn` is the first exception)
