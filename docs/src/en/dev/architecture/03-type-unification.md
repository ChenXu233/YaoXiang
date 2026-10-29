# Unification of Type Representations

> **Companion design document**. This article accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance-criteria tiering, and execution-phase ordering are defined in the
> main text of RFC-039; the positioning of each companion document is described in the
> [index for this directory](index.md).

## Positioning and Scope

### Coverage

Consolidate the multiple parallel type representations currently in use into a single
representation, eliminating the four categories of resulting burden:

- **13 `ast::Type` variants with zero forward construction by the parser** (among the 26 variants in
  `ast.rs:427-541`, see 2.2; the “forward” scope excludes reverse bridges and AST→AST reconstruction
  points)
- **A handwritten table of type-name synonyms** (`mono.rs:839-886`, see 2.3; WBS 6.1.2 has
  consolidated this into a 15-name resolution table)
- **The parser's genuine reverse dependency on the typechecking layer** (2 locations, see 2.4)
- **One expression variant with zero production construction**, `Expr::FnDef` (`ast.rs:37-43`, see
  2.6)

The concrete deliverables are four: a per-variant disposition table for all 26 variants (5.2),
semantic typing for `Type::Name` through `NameKind` and elimination of the synonym table (5.3), an
injectable data flow via `TypeEnvProbe` (5.4), and build-time type-table gates T1–T3 (5.7).

### Directory Renaming and This Document's Path

RFC-039 decision D1 mandates renaming the implementation directories (`typecheck/`→`sema/`,
`middle/core/`→`middle/ir/`) **together with P5/P6, without staging the work separately**. This
document's 50-item change list and all acceptance greps are written against the **current pre-rename
paths**—they constitute the construction baseline for each P6 batch **before the rename batch**; the
directory rename performed at the end of each batch is a **pure move batch** (C1 zero-diff)
committed separately. The baseline becoming invalid after the rename commit is expected; use git
history as the bisection basis.

### Out of Scope

- **How the directory rename is implemented**. `Type` is not moved out of `parser/ast.rs` in this
  phase; only a new `lower.rs` is added under `types/` as the sole conversion point. The directory
  rename (`typecheck/`→`sema/`, etc.) follows RFC-039 decision D1 at the end of P5/P6, in the form
  shown by the target directory structure in `01-routing.md`.
- **The four-layer model and dependency-direction rules**. See [01-routing.md](01-routing.md).
- **Tiering of equivalence criteria and validator implementation**. The definitions of C1–C6 and the
  three tiers of criteria—IR validators, normalized snapshots, and corpus differential
  testing—appear in [07-equivalence-oracle.md](07-equivalence-oracle.md). This document references
  only **C3 (type-representation convergence)** as its acceptance criterion.
- **SSA conversion**. See [04-ssa.md](04-ssa.md). This document is a prerequisite for that work, for
  the reasons described below.
- **The global P1–P10 execution sequence**. See the main text of RFC-039 and
  [02-stage-contract.md](02-stage-contract.md). This document only defines the phase breakdown for
  its own workstream (see “Implementation Essentials”).

### Division of Labor with RFC-039

RFC-039 is the overarching blueprint: it defines the four-layer model, criteria tiering,
cross-document phase ordering, and the rationale for undertaking this refactoring now. This document
is the construction plan for **the type-representation convergence item in RFC-039**: which variants
to remove, which files to change, in what order, and which gates prevent regression.

All line numbers, variant lists, and construction-point counts in this document were obtained
through static verification of `src/`. Each item cites `file:line`. **This document does not record
the execution process; it records code facts and the dispositions derived from them.**

For ease of cross-referencing with documents from the same batch, the “Target Design” and “Detailed
Design” chapters retain the `5.x` and `6.x` subsection numbering; the remaining chapters follow this
directory's standard format.

### Why This Must Precede SSA Conversion

`ir::Type` is not a third independent type representation; it is a `pub use` alias of `ast::Type`
(`ir.rs:3`). [04-ssa.md](04-ssa.md) is to introduce register type annotations and new instructions
to the IR. If the representations are not consolidated beforehand, SSA type annotations will
propagate through the `ir::Type` alias directly onto the 26-variant syntactic type, effectively
institutionalizing a third representation.

The current capability boundary of `ir::Type` is fixed by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) embeds a complete expression tree inside a type.
  It is impossible for an `Expr` to appear in an IR instruction's type annotation.
- `ast::Type::Literal { name, base_type }` (`ast.rs:476-482`) carries a source name and `Span`. It
  is the syntactic form of a compile-time literal type, not a runtime type.

Applying SSA type annotations on top of the 26 variants would permanently codify these 13 dead
variants and two syntax-specific variants into the IR contract.

## Current State

Everything in this section is a statement of code fact, not a proposal. Each item provides
`file:line` evidence.

### 2.1 Three Parallel Representations and a Lossy Bridge

> **Status note (2026-10-28, WBS 6.2 implemented)**: this section describes the state **before** WBS
> 6.2 landed. WBS 6.2 (6.2.1 / 6.2.2 / 6.2.3) changed all four type carriers to hold `MonoType`,
> deleted the lossy `From<MonoType> for IrType` bridge, and removed the `ir::Type` re-export alias.
> Every statement below concerning `ir::Type` or the lossy bridge describes the **pre-change**
> state; locations marked “(deleted in 6.2.2)” indicate an already implemented disposition. See
> [09a](./09a-wbs-history.md) §P6 6.2 for implementation details and measured results.

The multiplicity of type representations is the common upstream cause of four independent defects:

| Downstream symptom                                                  | Causal relationship to type representation                                                                                                                                                                                                                                                                                                              |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The same type text has multiple valid AST forms                     | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but structurally different**, the parser chooses the latter, forcing typechecking to supply a table that translates the latter back into the former                                                   |
| Bytecode-layer signatures disagree with IR-layer signatures         | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only obtain the alias. Thus `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`), while `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) declares `params` as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding one built-in type name requires multiple uncoordinated edits | Because the name-to-type mapping is scattered across `from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) and a string match in `bytecode.rs:2371-2376`, each location duplicates the mapping without cross-checking                                                                                                                         |
| The parser hardcodes `"Terminates"`                                 | Because the parser cannot determine whether a name denotes a type or an operator interface, it must embed type-layer knowledge in the syntactic layer as a string literal (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                                   |

**The first three are representation problems; the fourth is a downstream side effect of a
representation problem.** Implementing only item 1 (removing dead variants) leaves the synonym table
and reverse dependency in place; implementing only item 4 (injecting a callback into the parser)
builds on top of representations that remain inconsistent.

One of the three representations is an alias illusion:

| Name        | Definition                                | Size        | Role                                                    |
| ----------- | ----------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541` | 26 variants | Syntactic-layer type                                    |
| `MonoType`  | `src/frontend/core/types/mono.rs`         | 1,077 lines | Semantic-layer type (after monomorphization)            |
| `ir::Type`  | `src/middle/core/ir.rs:3`                 | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

`ir.rs:3` is a one-line `pub use`, and `ir.rs:6` then contains
`use crate::frontend::core::typecheck::MonoType;`. The IR module depends on two “type” symbols
simultaneously, but one is only an alias.

The serialization side has a fourth type carrier:

| Carrier                         | Element type    | Location                                   |
| ------------------------------- | --------------- | ------------------------------------------ |
| `BytecodeModule.type_table`     | `Vec<ir::Type>` | `src/middle/core/bytecode.rs:854`          |
| `FunctionCode.type_table`       | `Vec<MonoType>` | `src/middle/passes/codegen/bytecode.rs:32` |
| Interpreter module `type_table` | `Vec<ir::Type>` | `src/backends/interpreter/image.rs:44`     |

**Lossy bridging (pre-change fact; this impl was deleted in 6.2.2).** `From<MonoType> for IrType`
(previously at `src/middle/core/bytecode.rs:2353-2390`) downgraded `MonoType` to `ast::Type`, and
its **information loss was silent**:

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

All eight `MonoType` variants collapsed into `IrType::Void`, together with an `_ => IrType::Void`
fallback (`bytecode.rs:2386-2387`; the comment itself acknowledges that this is a “transitional
branch before deleting enum variants”).

Here, `IrType::Void` is **a valid type value rather than an error signal**. Downstream code
therefore cannot distinguish “this function genuinely returns `Void`” from “this function's return
type was lost during bridging.” This is one reason why the first-layer “type consistency” invariant
in 07 cannot currently pass on the existing code.

At the same time, `IrType::String` and `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`)
and `ast::Type::Bytes` (`ast.rs:436`), which are precisely the dead variants targeted in 2.2—**both
ends of the lossy bridge point to dead code**.

**Current state (since 6.2.2)**: the impl has been **deleted in its entirety**, and decoding no
longer downgrades and reconstructs `MonoType` as `ast::Type`—all four carriers now uniformly hold
`MonoType`: `BytecodeFunction.params` (`src/middle/core/bytecode.rs:814`) and `return_type` (same
file, `:816`), `BytecodeModule.type_table` (same file, `:856`), and `Image.type_table`
(`src/backends/interpreter/image.rs:44`). Decoding now copies values unchanged (`bytecode.rs:2315`,
`:2316`, `:2346`). The collapse described above **no longer occurs**: those eight `MonoType`
variants retain their real types in signatures and type tables, and `IrType::Void` no longer doubles
as a substitute for “information loss.” The `ir::Type` re-export alias (`pub use ... ast::Type` at
`src/middle/core/ir.rs:3`) has also been reduced to a private import (`:7`)—the IR layer no longer
re-exports the syntactic-layer type externally; however, three IR-layer carriers still directly hold
`ast::Type` (`Instruction::Cast.target_type` / `Instruction::TypeTest.ty` /
`FunctionBody::TypeDecl.definition`), pending disposition in 6.3.

### 2.2 Parser-Forward Variants with Zero Construction

**Definition of scope**: the construction-point scan covers all of `src/`, excluding `tests/` and
`*/tests/*`. **A construction point is an expression-position `Type::<variant>` path** (pattern
positions are not counted), and results must be **listed in three separate categories without mixing
them**: ① **parser-forward** (parser → AST); ② **AST→AST reconstruction** (the two replacement
functions in `middle/passes/mono/function.rs`); ③ **reverse bridge** (`From<MonoType> for IrType`
and `mono_to_ast_type`).

Of the 26 variants of `ast::Type`, the following 13 have **zero construction in the parser-forward
direction** (see 2.2.1 / 2.2.2 for the other two reconstruction categories):

| Variant                              | Definition       | Parser-forward construction points                                                                                                                                                                                                                                                                                    |
| ------------------------------------ | ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Int(usize)`                         | `ast.rs:432`     | None (`mono.rs:699` is a match arm)                                                                                                                                                                                                                                                                                   |
| `Float(usize)`                       | `ast.rs:433`     | None (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                   |
| `Char`                               | `ast.rs:434`     | None (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                   |
| `String`                             | `ast.rs:435`     | None (`mono.rs:702` is a match arm)                                                                                                                                                                                                                                                                                   |
| `Bytes`                              | `ast.rs:436`     | None (`mono.rs:703` is a match arm)                                                                                                                                                                                                                                                                                   |
| `Bool`                               | `ast.rs:437`     | None (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                   |
| `Void`                               | `ast.rs:438`     | `types.rs:836` (**the sole forward point = error fallback** `_ => (Vec::new(), Type::Void)`, individually registered as an exemption; eliminated with 6.4 #12)                                                                                                                                                        |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | None (`mono.rs:747` is a match arm)                                                                                                                                                                                                                                                                                   |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | None (`mono.rs:743` is a match arm); 2 AST→AST reconstruction points, see 2.2.2                                                                                                                                                                                                                                       |
| `Option(Box<Type>)`                  | `ast.rs:455`     | None (`mono.rs:770` is a match arm); 2 AST→AST reconstruction points, see 2.2.2                                                                                                                                                                                                                                       |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | None (`mono.rs:771` is a match arm); 2 AST→AST reconstruction points, see 2.2.2                                                                                                                                                                                                                                       |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | None (`mono.rs:814` is a match arm); 2 AST→AST reconstruction points, see 2.2.2                                                                                                                                                                                                                                       |
| `AssocType { .. }`                   | `ast.rs:463-471` | None (`mono.rs:781-786`, `formatter/handlers/types.rs:84`, and `semantic_tokens.rs:230` are all match arms); **2 AST→AST production reconstruction points**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3; **D8 remains unchanged** |

> **Definition-column line-anchor drift registry (P6 6.0 measurement, 2026-10-27)**: the
> “Definition” columns in this table and in 5.2 were written on 2026-10-05. Seventeen lines were
> subsequently added before the `ast::Type` enum (`pub enum Type` moved from 427 to 444), so **the
> definition lines of all 26 variants uniformly shifted by +17** (for example, `Int` moved from 432
> → 449 and `NamedParen` from 533-540 → 550-557). This round only **records** the drift and does not
> alter the table body; subsequent references use the `ast::Type` symbol anchor.

The causal chain can be verified in full:

1. **The parser represents every primitive type as `Type::Name { name }`**—the fallback arm at
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered into dedicated nodes**—the comment at `types.rs:392-396`
   explicitly states: “RFC-010: Result/Option are not lowered into dedicated AST nodes… they follow
   the same `Type::Generic` path as user-defined generic types. The representation remains
   `Generic{"Result"/"Option", args}`.”
3. **The sole parser-forward construction point of `Type::Void` is an error
   fallback**—`_ => (Vec::new(), Type::Void)` at `types.rs:836`, at the end of a match returning
   `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction points: `types.rs:364`, `types.rs:571`, `types.rs:100`, and
`declarations.rs:573`. They are not dead variants.

**These 13 variants do have construction points in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
The statement in this document is therefore “**zero parser-forward construction**”: it means neither
“zero references globally” nor “zero production construction”—11 of them do have production
construction points in the other two categories. These tests themselves codify the behavior under
test: they assert exactly what these syntactic forms lower into as `MonoType`, even though the
parser never produces them. When deleting variants, these tests must be deleted or rewritten
accordingly.

#### 2.2.1 Scope Boundary (1): The Reverse Bridge Reconstructs 7 of Them

The table's “zero forward construction” property holds only in the **parser → AST
forward-construction direction**. In the reverse direction, there are two production-grade
`MonoType → ast::Type` reconstructions that **actually construct 7 of the variants in the table
above**:

| Reverse bridge                                                 | Location                                 | Reconstructed members of the 13-variant list                                                                             |
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
    // #299: container types such as String/Bytes are now Generic and are converted back to AstType via type_name
    MonoType::Generic { name, .. } if name == "String" => AstType::String,
    MonoType::Generic { name, .. } if name == "Bytes" => AstType::Bytes,
    _ => AstType::Name { name: mono.type_name(), span },
}
```

**This scope boundary directly affects disposition**:

- The `bytecode.rs` location disappears together with 6.3 item #28 (deletion of the entire
  `From<MonoType> for IrType` impl), so it adds no extra cost.
- `function.rs:507-529` **is not covered by any existing change item**. It is a `MonoType → AstType`
  type-name reverse resolver (annotated `#299`) that converts `String` / `Bytes` from `Generic` back
  into dedicated variants by name. After `ast::Type::String` / `Bytes` are removed, these two guard
  branches must be rewritten; after `Int` / `Float` / `Bool` / `Char` / `Void` are removed, the
  behavior of the `_` arm changes—some types that previously fell into `_` will silently switch to
  `AstType::Name`. This is the 47th supplementary item in 6.3.
- **Direct consequence for the gates**: a T1 implementation that scans only forward construction
  points would classify the seven variants above as having construction points and therefore exclude
  them from the deletion list. T1 must explicitly include reverse bridges in its counting scope;
  otherwise, its conclusions contradict the disposition table in this document (see 5.7).

#### 2.2.2 Scope Boundary (2): AST→AST Reconstruction (the Third Category Added in This Revision)

`substitute_type_ast` (320-497) and `substitute_type_in_ast` (499-655) in
`middle/passes/mono/function.rs` perform **AST→AST** type-variable substitution (recreating an
existing AST type, **not** converting `MonoType → AST`). The P6 T1 gate measurement (6.0.2 baseline,
2026-10-27) found **15 variants / 28 points**, of which **5 variants / 10 points** belong to the
list of 13 above:

| Variant     | AST→AST reconstruction points |
| ----------- | ----------------------------- |
| `Union`     | `377`, `606`                  |
| `Option`    | `427`, `600`                  |
| `Result`    | `431`, `602`                  |
| `Sum`       | `467`, `632`                  |
| `AssocType` | `456`, `623`                  |

**When 6.3 deletes these five variants, all ten points must be rewritten in the same batch** (09's
6.3.2 records them); otherwise, those arms will no longer compile after the variants are deleted.
**Implemented (WBS 6.3.2, 2026-10-29)**: all ten points were deleted, with the dispositions recorded
in [09a](./09a-wbs-history.md) §P6 6.3.

### 2.3 Handwritten Synonym Table

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`; measured at 841-867 / **13 arms**
when this section was written, and **839-886 / 15 arms / 16 spellings** after WBS 6.1.2) uses a
string table to map type names to `MonoType`:

```
"Int" | "int" | "Int64" | "int64" | "i64"  => Some(MonoType::Int(64)),   // :843
"DateTime" | "datetime"                     => Some(MonoType::Int(64)),   // :850
"Int32" | "int32" | "i32"                   => Some(MonoType::Int(32)),   // :851
...
"Void" | "void" | "()"                      => Some(MonoType::Void),      // :863
```

**The sole reason this table exists is to bridge the AST layer's inconsistency**: because
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent at the AST layer but
structurally different—and the parser produces only the latter—`from_builtin_name` must accept case
variants, abbreviations, signed names, and the `DateTime` alias. The presence of `"()"` on the
`Void` line is especially revealing: it works in conjunction with the `Type::Void` fallback at
`types.rs:836`.

The same knowledge—which names are built-in type names—exists in seven copies throughout the
repository, with no consistency checking whatsoever:

| #   | Location (**measured 2026-10-27**; this section was written on 2026-10-05, and the original line numbers have drifted)            | Form                                                                                                                                                                                                                           |
| --- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | `MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`, measured 839-886 / **15 arms / 16 spellings**; after WBS 6.1.2) | String match, name → `MonoType`                                                                                                                                                                                                |
| 2   | `src/middle/core/bytecode.rs:2371-2376`                                                                                           | String match, copying `"String"` / `"Bytes"` / `"Tuple"`                                                                                                                                                                       |
| 3   | `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`, measured 855-858)                                                         | `&[&str]` constant containing 15 const-generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`)                                                                                       |
| 4   | `src/lsp/world.rs:182`                                                                                                            | LSP-side list of built-in type names (14 entries = the 15 names without `()`; WBS 6.1.2)                                                                                                                                       |
| 5   | `ConstKind::from_ast_type_name` (`src/frontend/core/types/const_data.rs`, measured 407)                                           | String match, name → `ConstKind`. **Coverage disagrees with item 1**: it does not accept lowercase or the `i64` / `i32` alias families (registered by the P0 gate on 2026-10-06)                                               |
| 6   | Base-type size table for `eval_builtin` (`src/frontend/core/types/eval/const_eval.rs`, measured 497-512)                          | String match, name → byte size. It previously contained a `"Uint"` arm (item 1 has no `Uint` mapping)—WBS 6.1.2 removed that arm, and `sizeof(Uint)` now reports an unknown type. **Dead arm; living evidence of table drift** |
| 7   | `to_type_id` (`src/middle/passes/codegen/bytecode.rs`, measured 735-775; `"Set" => 25` at 762)                                    | String match, built-in name → serialized type ID. `"Set" => 25` contradicts the typechecking side's “Set is excluded from names” decision (`inference/expressions.rs:2917-2918`, decision #300)                                |

> **Line-anchor drift registry (P6 6.0 measurement, 2026-10-27)**: the line numbers of all seven
> copies in this section **have drifted** (item 1 by +223 and the others by 2–9 lines each), but
> **the entry counts and contents have not changed** (when this section was written, item 1 had 13
> arms and item 3 had 15 entries; after WBS 6.1.2, item 1 has 15 arms / 16 spellings and item 4 has
> 14 entries). Subsequent references use the **symbol anchor + repository path**; the line numbers
> in this table are for location only. The arm-count drift in item 1 and the drift evidence in items
> 5–7 (the dead `Uint` arm and the contradiction between `Set` and the typechecking side) **all
> remain**, and this section does not modify code.

`CONST_PARAM_TYPES` in item 3 is used by `extract_generic_param_names`
(`src/frontend/core/parser/ast.rs`, measured 932) to identify const-generic parameters. It
enumerates names “usable as const parameters,” which **partially overlaps but is not identical to**
the full set handled by `from_builtin_name`, and no mechanism keeps them synchronized. Item 4 is the
fourth copy, used for LSP syntax highlighting and hover; it likewise has no validation (the
comparison scope of T2 includes it).

Items 5–7 were registered by `check-concepts.py` (P0, [09](09-execution-wbs.md) §0.2.1) on its first
run on 2026-10-06: item 5 is a second type-name synonym table, while items 6 and 7 share the same
root cause as items 1–4—string-keyed tables mapping built-in names to semantics or IDs—with
different uses but the same drift risk. The same gate also found 11 locations in the typecheck,
eval, and middle layers that dispatch built-in container names as strings
(`"List" | "Vec" | "Array"` ×4, `"String" | "Bytes"` ×4, etc.). **These are not covered by this
section or by the parser data flow in 5.4**; the list and disposition are recorded in
[06-cleanup-inventory.md](06-cleanup-inventory.md) §G3 as P6 expansion verification items.

### 2.4 The Parser's Reverse Dependency on the Type Layer

RFC-039 routing table C records three entries (`declarations.rs:27-80` calls
`is_type_param_annotation` / `name_used_as_type`, both of which belong to the type layer).
**Measurement differs from that description**: these two functions are defined inside the parser
itself, not in the type layer.

- `is_type_param_annotation` is defined at
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the only `fn` definition in the
  repository)
- `name_used_as_type` is defined at `declarations.rs:42-80` (the only `fn` definition in the
  repository)
- `declare_predicate` / `is_predicate_name` are defined at
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

There are exactly **two production-code references** that genuinely cross into `typecheck`, and both
call the same function, `crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Enclosing function                    | Purpose                                                                                                         |
| --- | --------------------------------------------------------- | ------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature-parameter filtering closure | Determines whether a constrained-position parameter (`T: Add`) occupies a runtime parameter position            |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`         | Determines whether a constrained-position parameter exists and produces a `GenericParamName` with `constraints` |

The second reference is deeper—**`parser/ast.rs` itself** (the entire `ast.rs:847-948` block
alongside `StmtKind`/`Expr`) directly references a typecheck symbol. Routing table C records only
the `declarations.rs` location.

Together with the hardcoding at `declarations.rs:498-499`, it forms the same decision:

```rust
// declarations.rs:498-499
// Expanded as more built-in predicates are added (the future open-set approach: pass predicate names down to
// the parser, or change them into a form determinable by the syntactic layer).
// The parser does not hold the type environment, so it checks two locations: hardcoded built-in predicates,
// plus declarations accumulated during the current pass (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment itself acknowledges this temporary form of the “future open-set approach.”
`n == "Terminates"` **embeds type-layer knowledge in the syntactic layer as a string literal**.

**Why this is downstream of the type-representation problem**: the parser needs to ask whether a
name is an operator-interface name because it must distinguish whether `Type::Name` denotes a type
or an interface constraint. **`Type::Name { name: String }` uses an undifferentiated string to carry
at least four meanings** (concrete type / type variable / constraint name / predicate name). The
syntactic layer cannot determine the meaning itself, so the authority is pushed upward to the type
layer.

#### Duplicate Implementation: Same Name and Meaning, Behavior Already Diverged

`name_used_as_type` has two implementations. The comment at `ast.rs:843-846` acknowledges this:

```rust
/// Whether a parameter name is treated as a type reference in the given type (the `N` in `(N: Int) -> (n: N)`).
///
/// Semantically equivalent to `name_used_as_type` in `declarations.rs`; implemented independently here to avoid
/// an internal cross-module parser dependency.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations **have already diverged**:

| Dimension               | `name_used_as_type` in `declarations.rs:42-80`                      | `name_used_as_type_in` in `ast.rs:847-885`           |
| ----------------------- | ------------------------------------------------------------------- | ---------------------------------------------------- |
| Signature               | Takes an `is_predicate: &dyn Fn(&str) -> bool` callback             | No callback                                          |
| `Type::Generic` branch  | Short-circuits to `false` after `is_predicate(app_name)` (`:54-56`) | No short-circuit; directly recurses into `args`      |
| `Type::Struct { body }` | **Absent** (falls through to `_ => false`)                          | **Present** (`:871-879`, traverses `Field` / `Expr`) |
| `Type::NamedStruct`     | **Absent**                                                          | **Present** (`:880-882`, traverses `fields`)         |

The comment at `ast.rs:868-870` records the origins of the latter two branches: “Previously these
two branches were missing, so a const-generic parameter referenced in a definition body could not be
detected as ‘used as a type,’ and annotation validation incorrectly reported a valid const parameter
as an unknown name.”

In other words, **the function copied to avoid a cross-module dependency did not have its fixes
backported to the original**. The `ast.rs` version recursively traverses fields in a definition
body, while the `declarations.rs` version does not—the same source receives different results at the
two decision points. This duplicate must be consolidated when 5.3 introduces `NameKind`.

### 2.5 Parallel Operator Enums

There are two operator enums that do not reference each other and **whose members have already
diverged**:

| Concept         | Syntactic layer                 | Constant-evaluation layer                                             | Difference                                                                                                                                     |
| --------------- | ------------------------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Binary operator | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | The syntactic layer alone has `Neq` (`:197`), `Range` (`:205`), and `Assign` (`:206`); the constant layer uses `Ne` (`:243`) rather than `Neq` |
| Unary operator  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | The syntactic layer alone has `Deref` (`:222`); the constant layer alone has `BitNot` (`:329`)                                                 |

Each also implements a complete `Display` (`const_data.rs:291-317`) and classification predicates
(`const_data.rs:260-289`: `is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise`).

**Why this is part of the type-representation problem**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) **are not types**—when they occur in expression position within a
type, they fall into `Type::ConstExpr` (`ast.rs:506`), the “compile-time-only variant.” This is
directly related to the IR-layer type discipline introduced in 6.1: after convergence,
`Type::ConstExpr` disappears following `Type → MonoType`, and the IR can never observe it, while
`Assign` / `Deref` continue to exist through paths such as `ir_gen.rs`'s
`BinOp::Assign => Ok(MonoType::Void)` (`inference/expressions.rs:875`). The divergence between the
enums means that “which operators may occur in type position” cannot be determined from either side
alone.

This phase does not merge the two enums (their ownership is covered under “changes affecting
operators” in [05-frontend-paradigm.md](05-frontend-paradigm.md)), but the T1 gate's scan scope
should include `ast::BinOp` / `ast::UnOp` so that newly introduced zero-construction operators are
equally visible.

### 2.6 `Expr::FnDef`: An Expression Variant with Zero Production Construction

It is true that `Expr::FnDef` (`ast.rs:37-43`) has **zero production construction**—the only two
construction sites in the entire repository are `src/frontend/core/parser/tests/ast.rs:837` and
`src/frontend/core/typecheck/tests/checker.rs:111`, both in tests.

However, **there are far more than four consumption sites**. The measured number of production
consumption sites is 12:

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

There are also 2 exhaustive-match arms that must exist because of it: `ast.rs:1026` (`Expr::span()`)
and `pratt/mod.rs:47` (`expr_end_line`).

**The path actually taken by function definitions** is: `nud.rs:425-429` constructs `Expr::Lambda`,
and `declarations.rs:462-479` constructs `StmtKind::Assign` (placing the `Lambda` in `value`). In
other words, the `FnDef` branch is **a parallel path that never executes but is maintained at 14
locations**.

### 2.7 Syntax Nodes Storing Semantics on Behalf of the Type Layer

**First, `StmtKind::Assign` carries a typechecker-only field.** The `Assign` variant at
`ast.rs:241-249` contains `signature_params: Vec<Param>` (`ast.rs:244-245`), and its comment states:

> `/// The first group of signature parameters is preserved as-is (including parameter names), for typechecker classify_generic_params to use`

The statement AST reserves a dedicated slot for the type checker. Construction point:
`declarations.rs:472`.

**Second, expression position is treated as a parameter list.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a **Lambda with an
empty body**. The `Expr::Lambda { params, .. } => Some(params.clone())` arm at
`src/frontend/core/parser/pratt/led.rs:466` then “retrieves” it and uses it as a parameter list.

In other words, a normal AST variant is used as a **temporary parameter-list carrier**, after which
another module's match arm restores its meaning.

**Third, `Type::NamedParen` stores the return-position binder's name.** The comment at
`ast.rs:519-540` explicitly states:

> RFC-027 §3's return-position refinement relies on it to declare the **return formal parameter's
> name** (called a binder here)… **Without it, the type checker can only guess** that “a free
> variable in a constraint that is not in scope is the return formal parameter,” causing `(r: P(m))`
> to silently substitute the undeclared `m` as though it were a binder (measured defect).

A syntax node is storing binder identity on behalf of the type layer.

## Target Design

### 5.1 Form of the Target Representation

**Claim: a single representation means using `Type` (formerly `ast::Type`) as the sole type
structure; demoting `MonoType` to the type checker's internal working representation; and deleting
the `ir::Type` alias.**

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

**Why not “merge into `MonoType`”** (this is the most likely proposal and is rejected here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.), whereas
   `MonoType` does not. C3 in 07 requires “identical diagnostic codes and messages”—diagnostic
   position information must remain traceable to the source. Adding `Span` to `MonoType` would
   attach a collection of meaningless sentinel spans to every monomorphized type.
2. `MonoType` contains `TypeVar` and substitution states (`types/substitute.rs:112`, and
   `pub fn unify` at `types/solver.rs:411`). These are **intermediate states of the type checker's
   solving process**, not types themselves. Putting them into the sole type structure would require
   the IR-construction layer to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027) that are lowered away in `MonoType`. Unifying on `MonoType`
   would require adding these syntactic forms back to it.
4. **The decisive counterargument**: elevating `MonoType` to the sole representation would make the
   L2 syntactic layer depend on the L3 semantic layer's types. RFC-039 explicitly prohibits L2 from
   depending on L3. Eliminating reverse dependencies is one of this document's objectives, so
   choosing a solution that introduces one would be self-contradictory.

**Why `ir::Type` must be deleted directly rather than “retained”**:

The alias at `ir.rs:3` causes `BytecodeFunction.params` (`bytecode.rs:812`) and `return_type`
(`bytecode.rs:814`) to use `ir::Type`, while the serialization side uses `MonoType` at
`codegen/bytecode.rs:277-278`. The two field types are connected through
`file.type_table.into_iter().map(|t| t.into())` at `bytecode.rs:2339` and the lossy `From` bridge at
`bytecode.rs:2353-2390`.

**The sole reason the `From` bridge exists is that “the field types on the two sides differ.”** Once
`bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, and `image.rs:44` are uniformly changed to
`MonoType`, `From<MonoType> for IrType` has no reason to exist and can be deleted in full,
eliminating the reverse reconstruction recorded in 2.2.

**Explicit non-goals**: `Type` is not moved from `parser/ast.rs` to `sema/types/` in this phase.
Only a new `lower.rs` is added under `types/` as the sole conversion point, and the definition of
`Type` remains in place. The directory rename is executed at the end of P6 under D1 as an
independent move batch—at that point, `Type` will live in top-level `ast/type_.rs` (the AST
top-level domain), while `sema/types/` will be the home of the `MonoType` working representation.

### 5.2 Per-Variant Disposition Table for All 26 Variants

> **Implemented (WBS 6.3, 2026-10-29)**: all 13 “delete” entries in this table **have been
> implemented**—`ast::Type` has converged from 26 variants to 13 (13 deleted, 1 migrated, 12
> retained); both type bridges (reverse bridge B, `mono_to_ast_type`, and the two AST→AST
> substitutions) were **rewritten in the same batch**. See [09a](./09a-wbs-history.md) §P6 6.3 for
> measured results, the ten-point disposition table, and the criteria.

> Disposition categories: **delete** (zero parser-forward construction), **retain** (forward
> construction exists), and **migrate** (semantics are relocated to another node).

| #   | Variant                                                            | Definition line  | Disposition                                | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| --- | ------------------------------------------------------------------ | ---------------- | ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Retain (strengthened)**                  | The parser's **sole** primitive-type exit (`types.rs:162-165`). After the 12 primitive-type variants are removed, all primitive types use it. A “semantic kind of name” annotation must be added (see 5.3)                                                                                                                                                                                                                                                     |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:699` is a match arm). Bit width is carried by the `Name` path through `from_builtin_name`                                                                                                                                                                                                                                                                                                                           |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:702` is a match arm). **Note**: `bytecode.rs:2372` consumes it through `IrType::String`; that arm must be rewritten after deletion                                                                                                                                                                                                                                                                                  |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:703-706` is a match arm). `bytecode.rs:2373` consumes it in the same way                                                                                                                                                                                                                                                                                                                                            |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                                |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                                 | The sole parser-forward construction point is the error fallback `types.rs:836` (`_ => (Vec::new(), Type::Void)`, individually registered as an exemption). The fallback is eliminated together with the error-path rework in 6.4                                                                                                                                                                                                                              |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Retain**                                 | Has production consumers (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                                                                                                                                                                                                              |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Retain**                                 | Has production construction point `types.rs:364`                                                                                                                                                                                                                                                                                                                                                                                                               |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:743-746` is a match arm); 2 AST→AST reconstruction points (`function.rs:377`/`606`) rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                            |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:747` is a match arm); zero across all three categories (the only globally zero-construction variant). Enums use `Struct` + `TypeBodyItem`; unrelated to `MonoType::Enum(_) => IrType::Void` at `mono.rs:2378`                                                                                                                                                                                                       |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Retain**                                 | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:451-454` | **Retain**                                 | Consumed by `bytecode.rs:2366`; **field reshaping `params: Vec<Type>` → `Vec<Param>` (name and type colocated) follows in 6.8.1 (D54)**—the names for `Assign.signature_params` are thereby carried forward (see 6.5.2)                                                                                                                                                                                                                                        |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:770` is a match arm); 2 AST→AST reconstruction points (`function.rs:427`/`600`) rewritten in the same batch as 6.3.2. `types.rs:392-396` already documents the `Generic` path                                                                                                                                                                                                                                       |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:771` is a match arm); 2 AST→AST reconstruction points (`function.rs:431`/`602`) rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Retain**                                 | The actual representation of `Result` / `Option` / `String` / `Bytes` (`types.rs:397-399`)                                                                                                                                                                                                                                                                                                                                                                     |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                                 | Zero parser-forward construction (the syntactic layer has no `::` associated-type path); **2 AST→AST production reconstruction points**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3 (registered in 6.3.2). **D8 remains unchanged**: the criterion is forward construction; if associated-type syntax is enabled in the future, a new proposal at that time will define it |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                                 | Zero parser-forward construction (`mono.rs:814` is a match arm); 2 AST→AST reconstruction points (`function.rs:467`/`632`) rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                                |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Retain**                                 | Has production construction point `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                                                                                                                                                                                                       |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Retain**                                 | Raw pointer type inside an unsafe block                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Retain**                                 | Borrow notation                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Retain**                                 | Has production construction points `types.rs:100` and `declarations.rs:573`; RFC-010                                                                                                                                                                                                                                                                                                                                                                           |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Retain (and mark as compile-time-only)** | Has production construction point `types.rs:160`. It is the only variant in `Type` that embeds an `Expr`, and the IR layer must not carry it (see 6.1)                                                                                                                                                                                                                                                                                                         |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Retain**                                 | RFC-004 currying terminator; `split_curry` depends on its existence                                                                                                                                                                                                                                                                                                                                                                                            |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                                | Semantics (the binder name) move to the type checker; the AST retains only syntax. See 5.6                                                                                                                                                                                                                                                                                                                                                                     |

**Net effect: 26 → 13 variants** (13 deleted, 1 migrated, 12 retained).

> **Reverse bridges and AST→AST reconstruction must be handled before variants are deleted**:
> `function.rs:507-529` (`mono_to_ast_type`) recorded in 2.2 reconstructs seven of items 2–8 in this
> table; in addition, the AST→AST reconstruction recorded in 2.2.2 accounts for 5 variants / 10
> points within the 13-item list (Union/Option/Result/Sum/AssocType, 2 points each). Both categories
> must be handled in the same batch as 6.3.1; otherwise, behavior either changes silently (see 6.3
> #47) or the code no longer compiles.

> **How `AssocType` is handled**: **D8 remains unchanged**, and it is deleted directly under D8
> (item 18 in the 5.2 table). The **criterion is parser-forward construction**: zero forward
> construction holds because the syntactic layer has no path that produces `::` associated types.
> Therefore, D need not be changed and the variant does not enter the whitelist. The **2 AST→AST
> production reconstruction points** (`function.rs:456`, `substitute_type_ast`, and
> `function.rs:623`, `substitute_type_in_ast`) were registered in 6.3.2 and rewritten in the same
> batch as 6.3. The three-category split of the T1 gate (5.7) reproduced this conclusion (all 26
> variants matched in the 6.0.1 measurement).

### 5.3 `NameKind` and Elimination of the Synonym Table

**Problem**: `Type::Name { name: String }` uses an undifferentiated string to carry at least four
meanings—concrete type / type variable / constraint name (operator interface) / predicate name.
`src/frontend/core/parser/statements/declarations.rs:500-501` must determine whether something is a
predicate; `510-512` in the same file must determine whether it is an operator interface;
`src/frontend/core/parser/ast.rs:971` must determine whether it is an operator interface; and both
`name_used_as_type_in` in the same file (measured at 864) and `name_used_as_type` at
`declarations.rs:42` must determine whether it is a type reference. All of these rely on string
comparison.

**Proposal: add a `kind` field to `Type::Name`, determined once by the parser.**

```rust
// Target form (illustrative; line numbers are post-change)
Name {
    name: String,
    kind: NameKind,   // New
    span: Span,
}

enum NameKind {
    Builtin,      // Built-in type name: Int / Float / Bool / ... → handled by from_builtin_name
    UserType,     // User-declared type name (including Generic's std.result / std.option)
    TypeVar,      // Type variable: T / K / V
    Constraint,   // Operator interface / constraint name: Add / Ord / ...
    Predicate,    // Compile-time predicate: Terminates / Sorted / ...
}
```

**Key design point: determining `NameKind` requires exactly the information the parser cannot
currently obtain** (the type environment, the declared operator-interface table, and the declared
predicate table). Therefore, this field **cannot be determined independently inside the parser**—it
must be determined through an injected environment. This connects directly to 5.4.

**Disposition of the synonym table (WBS 6.1.2 implemented, final wording by the user on
2026-10-27)**:

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) **is retained, with
reduced semantics**. Before the change, it was a disambiguator that bridged two equivalent AST
representations. After `Type::Int(64)` is removed, “Int” has only one representation path, and the
function degenerates into the **sole name-to-type resolution table**.

It must be retained for these reasons:

- It is the **sole cut point** for standard-library type names and language built-ins (the signature
  strings in `src/std/*.rs`, `CONST_PARAM_TYPES`, and the LSP list all use it as their reference).
- The `DateTime` arm records a real fix (#338 related issue: `now()`'s return value could not be
  passed to `format_time`). It is a **semantic alias**, not the same kind of case alias as
  `Int`/`int`, and is therefore retained separately through P10 S11 (issue #513).

**Disposition result** (final criterion = 15 built-in names + the temporary `DateTime` alias):

| Category                                                                                                                                                                                                                                     | Before                                      | Disposition                                                                                                                                                                                                   |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 15 built-in names                                                                                                                                                                                                                            | Crowded into 13 arms and mixed with aliases | **One name per arm**: `Never` / `Void` / `()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` / `Char` / `String` / `Bytes`                                         |
| Case and abbreviation aliases (`int` / `int64` / `i64` / `int32` / `i32` / `int16` / `i16` / `int8` / `i8` / `float` / `float64` / `f64` / `float32` / `f32` / `bool` / `char` / `string` / `str` / `bytes` / `void` / `never` / `datetime`) | Retained                                    | **Deleted** (22 spellings), **with no lexer alias table**—corpus measurement found 0 noncanonical names (see the design judgment at the end of this section), so syntactic-layer compatibility is unnecessary |
| `()`                                                                                                                                                                                                                                         | On the same arm as `Void`                   | **Retain**—`()` is a spelling of the `Void` type (standard-library signature strings use `() -> Void`), not another type                                                                                      |
| `DateTime`                                                                                                                                                                                                                                   | `datetime` and lowercase aliases on one arm | Retain **only the uppercase** temporary compatibility alias, noting that its removal belongs to P10 S11 (issue #513)                                                                                          |

**Net effect**: `from_builtin_name` = **15 match arms / 16 spellings / 1 alias** (`()` shares an arm
with `Void`; there is also the temporary `DateTime` alias). After deleting the 22 spellings, a
noncanonical spelling in type position becomes a `TypeRef`, and
`TypeEnvironment::resolves_type_name` reports an unknown type—it is **no longer silently accepted**.
Previously, `add_builtin_types` also registered seven lowercase duplicate spellings—`int` / `float`
/ `bool` / `string` / `void` / `never` / `char`—in the type table, effectively constituting a second
alias table; these were deleted in the same batch.

This acceptance surface has **two permanent gates**:

1. Positive and negative unit cases in `src/frontend/core/types/tests/mono.rs` (the expected
   `MonoType` for each of the 15 names, `None` for the 22 spellings, and `DateTime` → `Int(64)`);
2. **T2.5, the “deleted spelling sentinel,”** in `tools/type-tables --report`—if any of the 22
   spellings is added back to the resolution table, the report immediately counts it as
   **unregistered drift** (there is no disposition to register: that would be a rollback).

**Consolidating the two `name_used_as_type*` implementations**: the two duplicate implementations
recorded in 2.4 must be merged into one; otherwise, the semantic information provided by `NameKind`
can be bypassed by two independent string-comparison implementations. The merged implementation is
placed in `parser/ast.rs` (next to the `Type` data), and `declarations.rs` calls it. The
`is_predicate` short-circuit reads `NameKind::Predicate`, and `Struct` / `NamedStruct` field
recursion follows the `ast.rs:894-905` version, which is newer and more complete.

**Ownership of `CONST_PARAM_TYPES`**: `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`,
measured 855-858), the table of const-generic parameter names, partially overlaps the full set
handled by `from_builtin_name` (see 2.3). After `NameKind::Builtin` is introduced, the decision in
`extract_generic_param_names` (same file, measured 932) can be changed to “`kind == Builtin` and the
name is in the accepted const-parameter set,” explicitly connecting the two sources of knowledge;
the subset defining const parameters is validated by the T2 gate (see 5.7).

> **Design judgment**: if any `.yx` corpus file contains a type annotation such as `i64` / `int`,
> normalizing it to `Int` changes that file's **diagnostic position** (but not its diagnostic code).
> This is permitted under C3 (messages and ordering may be normalized), but the baseline must be
> recorded in phase 1. **Measurement (P6 6.0, 2026-10-27)**: a full scan of `tests/yaoxiang` (316
> `.yx` files), `tests/yaoxiang-multifile`, and `src/std` found **0 noncanonical names in actual
> type positions**; 16 raw token matches were adjudicated individually as 12 strings and 4 comments
> (both `i64` / `f64` occurrences were in comments). Therefore, 6.1's name normalization **does not
> change the diagnostic position of any corpus file**, and **there is no need to regenerate the
> baseline**.
>
> **Retest (P6 6.1 closure, 2026-10-28)**: 6.1.1 rescanned using the final set of **22** spellings.
> The 2026-10-27 measurement used a **narrow scope** (counting only the 14
> integer/floating-point-family spellings and excluding namespace usage in `src/std`, hence the “16
> raw token matches = 12 strings + 4 comments”). The retest scope was 316 `.yx` files in
> `tests/yaoxiang` + 39 in `tests/yaoxiang-multifile` + 38 in `src/std` = **393 files**, with
> matches on **308 lines**; **only 2 lines were type-position candidates, and both were in `//`
> comments** (`tests/yaoxiang/06-compile-errors/runtime/int_overflow.yx:5`, `i64::MAX`;
> `tests/yaoxiang/99-demos/compare_rust_side_by_side.yx:17`, `-> f64`) ⇒ **there are still 0 actual
> type positions**; all other matches were individually adjudicated as string literals / comments /
> `std.string` module names and namespace accesses / the `Json` variant name / the `void` literal,
> and **none was a type annotation**. See the 6.1.1 line in [09](09-execution-wbs.md) and 09a's “P6
> 6.1 Name Normalization Closure” for the retest command. As with 6.0, 6.1's name normalization
> **does not change the diagnostic position of any corpus file**, and **there is no need to
> regenerate the baseline**.

### 5.4 Data Flow for Predicate and Type Names

**Current data flow** (a closed loop inside the parser + 2 boundary crossings):

```
ast.rs (extract_generic_param_names, :930)  ─┐
declarations.rs:509                          ─┴─→ typecheck::operator_interfaces::spec()   ✗
declarations.rs
  ├─ declare_predicate()      ← parser_state.rs:46（parser 自持，仅本遍已解析的声明）
  ├─ n == "Terminates"        ← 硬编码字面量（declarations.rs:499）
  └─ （上述越界调用）
```

**Target data flow** (injected, without boundary crossings):

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

| File                                                                                                          | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/probe.rs` (new)                                                                     | Define the `trait TypeEnvProbe`, providing four pure query methods: `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`. **The trait is defined in L2 and implemented by L3**—this is legal dependency inversion. **The implemented method names are authoritative** (the original design's `is_type_var` / `is_builtin_type` were not adopted: a type variable's identity is determined structurally by its declaration site, not by its name, so that criterion is omitted; built-in-name classification is instead provided by `kind_of`, and `knows_builtin_types()` declares whether the probe holds the built-in type-name table) |
| `src/frontend/core/parser/parser_state.rs:46-58`                                                              | Retain `declare_predicate` / `is_predicate_name` (for accumulation during the current pass); add a `probe: &'a dyn TypeEnvProbe` field and a `probe()` accessor (**borrowed rather than boxed**). **The implemented state also adds a `kind_of()` accessor**—predicates declared during the current pass take priority, and all remaining cases are delegated to the probe, making this the sole entry point for name classification                                                                                                                                                                                                                     |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                                                 | Remove `n == "Terminates"`; use `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                                                 | Remove the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; use `state.probe().is_constraint(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `src/frontend/core/parser/ast.rs:930`                                                                         | Remove the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` defines `Type` / `Expr` / `StmtKind` and must not hold an environment dependency—this function must instead accept a `&dyn TypeEnvProbe` parameter (see below)                                                                                                                                                                                                                                                                                                                                                                                             |
| All **40** parser construction sites (`ParserState::new` is the sole constructor; **4 production + 36 test**) | Accept a `probe: &'a dyn TypeEnvProbe` parameter. **The default implementation, `NullProbe` (all results false),** ensures parser unit tests remain unaffected; the default production probe is `BuiltinProbe` (the compiler's built-in closed set), with injection seams `parse_with_probe` / `parse_expression_with_probe`. (**The original design's “5 construction sites” was a pre-survey estimate; the measured number is 40**—`Parser::new` does not exist, and `ParserState::new` is the only constructor)                                                                                                                                       |

**The `extract_generic_param_names` signature problem**:
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` at `ast.rs:891` is a
free function that receives no context. After the boundary-crossing call at `ast.rs:930` is removed,
it requires an `is_constraint` decision, so its signature must add `probe: &dyn TypeEnvProbe`.
Callers, including `parser/tests/ast.rs`, must likewise pass `&NullProbe`.

**When `NameKind` is determined**: when `types.rs:162-165` constructs `Type::Name`, it uses
`state.probe()` to determine `kind`, fixing it once so that all later match arms no longer perform
string comparisons.

**This eliminates all three entries in RFC-039 routing table C**—two of them already involved only
parser-internal functions under the scope in 2.4, while the genuine cross-layer references are the
two locations `declarations.rs:509` and `ast.rs:930`, both covered by this section.

### 5.5 Consolidating `Expr::FnDef`

**Claim: delete `Expr::FnDef` (`ast.rs:37-43`) and route all function definitions through
`Expr::Lambda` + `StmtKind::Assign`.**

**This is already the active path**: `nud.rs:425-429` + `declarations.rs:462-479` (see 2.6). The
`FnDef` branch is a parallel implementation that never executes.

**Change list** (12 consumption sites + 2 exhaustive-match arms, listed individually):

| Location                             | Existing arm                                         | Target                                                                                                                                              |
| ------------------------------------ | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `spawn/placement.rs:122`             | `Expr::FnDef { body, .. } => self.check_block(body)` | Merge into the existing `Expr::Lambda` arm (`body` is likewise `Box<Block>`, so **delete this arm directly**)                                       |
| `spawn/analysis.rs:912`              | `Expr::FnDef { body, .. } => { ... }`                | Merge into the Lambda arm in the same way                                                                                                           |
| `formatter/handlers/expr.rs:43`      | `Expr::FnDef { ... }`                                | Format the Lambda inside `Assign`; delete this arm                                                                                                  |
| `orchestrator.rs:1492`               | `if let Expr::FnDef { name, .. }`                    | Obtain the name from `StmtKind::Assign { target, .. }`                                                                                              |
| `ir_gen.rs:4325`                     | `ast::Expr::FnDef { span, .. } => *span`             | Merge into the Lambda arm (`Lambda` also carries `span`)                                                                                            |
| `ir_gen.rs:5097`                     | `\| ast::Expr::FnDef { span, .. }`                   | Delete this or-pattern                                                                                                                              |
| `checker.rs:1642`                    | `if let ...::Expr::FnDef {`                          | Match `Expr::Lambda` instead                                                                                                                        |
| `checker/semantic_tokens.rs:1416`    | `Expr::FnDef {`                                      | Delete this arm                                                                                                                                     |
| `inference/expressions.rs:3374`      | `...::Expr::FnDef {`                                 | Match `Expr::Lambda` instead                                                                                                                        |
| `inference/existential.rs:55`        | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                              |
| `passes/dead_code.rs:305`            | `Expr::FnDef {`                                      | Match `Expr::Lambda` instead                                                                                                                        |
| `layers/ownership.rs:1207`           | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                              |
| `layers/termination.rs:752`          | `Expr::FnDef {`                                      | Match `Expr::Lambda` instead; update the comment at `termination.rs:2303` (“`fn(): Never` — FnDef.return_type is the bare return type”) in parallel |
| `ast.rs:1026`                        | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                              |
| `pratt/mod.rs:47`                    | `Expr::FnDef { span, .. } => *span`                  | Delete this arm                                                                                                                                     |
| `typecheck/tests/checker.rs:109-111` | Test constructs `Expr::FnDef`                        | Rewrite as `Expr::Lambda` + `Assign`                                                                                                                |
| `parser/tests/ast.rs:837-847`        | Test constructs `Expr::FnDef`                        | Delete this test                                                                                                                                    |

**No behavior changes**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, and `span`,
structurally identical to the first three fields of `FnDef`. The `name` field unique to `FnDef` has
no usable location at the `Expr` layer—the name resides on `StmtKind::Assign.target`.

**Risks and mitigation**: non-exhaustive `if let` matches such as `checker.rs:1642` and
`inference/expressions.rs:3374` become impossible to match after the change, without an error or
warning. Mitigation: the type-table gate (5.7) adds a “zero-construction variant” check. After
`Expr::FnDef` is deleted, any attempt to reintroduce `if let Expr::FnDef` fails to compile because
the variant no longer exists. **This is guaranteed at compile time and requires no additional
gate.**

### 5.6 Disposition of `signature_params` and `NamedParen`

**`StmtKind::Assign.signature_params` (`ast.rs:244-245`) → delete.**

Rationale: its reason for existing (the comment at `ast.rs:244`) is “for typechecker
`classify_generic_params` to use”—a **dedicated slot reserved by the statement AST for the type
checker**. However, `extracted_params` in the same function at `declarations.rs:500-511` has already
computed `value_params` (with type-position and constraint-position filtering complete). Passing it
through the statement duplicates that work.

Change: remove the field at `ast.rs:244-245` → remove the argument at `declarations.rs:472` → change
the source of `classify_generic_params` data to an in-place recomputation (it can obtain
`Lambda.params` from `value: Option<Box<Expr>>`). **Net result: one field declaration, one
construction site, and one transport path are removed.**

> **Risk**: `signature_params` is the **unfiltered original** list, whereas `Lambda.params` is
> **filtered** (the filter at `declarations.rs:500-511` removes type-position and
> constraint-position parameters). If `classify_generic_params` depends on the unfiltered version,
> behavior will change after the refactor. **Whether `classify_generic_params` actually uses each
> version must be verified in phase 1; equivalence must not be assumed.**

**`Type::NamedParen` (`ast.rs:533-540`) → retain the node, but remove binder semantics from it.**

The comment at `ast.rs:519-532` records its **sufficient reason for existing** (see 2.7): if `r` is
lost from `(r: P(m))`, the type checker can only guess that “a free variable in a constraint that is
not in scope is the return formal parameter,” and will **silently substitute the undeclared `m` as
though it were a binder** (the comment labels this a “measured defect”).

Therefore, **it cannot simply be deleted**—that would reintroduce a correctness defect that has
already been fixed. The correct disposition is:

1. Retain `NamedParen { param, param_span, inner }` as a **pure syntax node**, with
   `param`/`param_span` carrying only source facts and no longer bearing type-layer binder
   semantics.
2. Move binder interpretation into `sema/`: the type checker **explicitly** extracts `param` as the
   binder when consuming `NamedParen`. This is “reading a syntax node's field,” the same category as
   embedding an `Expr` in `ConstExpr`—the syntax node provides facts, while the semantic layer
   interprets them.
3. Rewrite the comment from “the type checker can only guess” to “this node provides the binder
   name; semantic interpretation occurs inside `sema/`.”

**Why this is a “migration” rather than a “deletion”**: the syntax tree must represent source code
without loss. `r` genuinely exists in the source, and the AST cannot pretend otherwise. The problem
is not the node's existence, but the failure to define responsibility for semantic interpretation
explicitly.

### 5.7 Build-Time Type-Table Gates

**The reference implementation has already been validated in this repository**: `build.rs:19-37`
calls `parse_registry` / `validate` from `tools/code-tables` to verify uniqueness, section
placement, and Chinese-language completeness for 145 error codes, and directly `panic!`s to reject
the build when `is_ok()` is false. `build.rs:39-54` further compares the RFC-013 markdown code table
entry by entry, also `panic!`ing on a mismatch. **RFC-013 documentation and code therefore remain
consistent**—`tools/code-tables/Cargo.toml:15-16` has only a `serde_json` dependency, and the
validation logic performs text-level parsing without relying on syn.

The type tables currently have **no** comparable gate. The mechanism introduced here can directly
follow the same pattern.

**New crate**: `tools/type-tables` (`Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). Four checks:

| #                                                                | Check                                                                                                                                                                                                                                                                               | Input                                       | Failure condition                                                                                                                                                                                                                                                                                                                                                | Remedy                                                                                                            |
| ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **T1 Construction-point categorization**                         | Scan the `pub enum Type` variant list in `src/frontend/core/parser/ast.rs`; under `src/`, count each variant's construction points in **three separate categories** (① parser-forward ② AST→AST reconstruction ③ reverse-bridge reconstruction; excluding `tests/` and `*/tests/*`) | Variant list + three categorized hit tables | ① A variant has **zero forward construction** and is not registered; ② a registered variant gains a forward construction point; ③ a registered wildcard-arm fallback exemption point is **not hit**; ④ an **unregistered wildcard-arm fallback point** exists (fallback exemptions are registered individually and must not be generalized to all wildcard arms) | Delete the variant; or register a rationale / remove a stale registration (see 5.2 and `REGISTERED_ZERO_FORWARD`) |
| **T2 Synonym table consistency with repository-wide name lists** | The `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` at `ast.rs:855` + the LSP built-in type-name list at `src/lsp/world.rs:182`                                                                                                                            | Three tables                                | A type name exists in one table but is not recognized by another (comparison follows each table's semantic mapping rules); **since WBS 6.1.2, T2.5, the “deleted spelling sentinel,” additionally reports an error if any of the 22 deleted spellings is added back**                                                                                            | `type-tables --fix` synchronizes in both directions                                                               |
| **T3 Built-in type names consistent with RFC-011**               | The canonical-name set in `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                                | Two tables                                  | The code range or names disagree with the registry                                                                                                                                                                                                                                                                                                               | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --fix`                                                 |
| **T4 Exhaustiveness of `Type → MonoType`**                       | The `Type` variant list vs. the match arms in `types/lower.rs`                                                                                                                                                                                                                      | Two tables                                  | A variant has no corresponding arm, or an arm has no corresponding variant                                                                                                                                                                                                                                                                                       | Add an arm / delete an arm                                                                                        |

**Implementation note for T4**: Rust's exhaustive match already guarantees the “variant → arm”
direction at compile time. T4's value lies in the **reverse** direction—detecting stale arms in
`lower.rs` that refer to deleted variants. Such code would fail to compile when the variant is
deleted, so T4 is effectively redundant. **T4 is therefore downgraded to a CI assertion**
(`cargo build` succeeding is sufficient) and is not placed on `build.rs`'s `panic!` path. **T1–T3
are the three items that genuinely require text parsing.**

**Two implementation difficulties in T1 and their mitigations**:

1. **Distinguishing construction points from match arms** requires syntax analysis. The repository
   root `Cargo.toml` has **no `syn` dependency**, and `tools/code-tables` likewise has only
   `serde_json`. Mitigations:
   - **Preferred**: introduce `syn` into `tools/type-tables` (only that crate; the root crate
     remains unchanged). `syn` is standard practice, and the gate crate is independent of the
     compiler.
   - **Fallback**: reuse the existing `tools/extract_arm.py` line-level heuristic (an existing
     Python tool in the repository)—a construction point occurs on the right side of `=` or in a
     function-argument position, while a match arm occurs on the left side of `=>`. Use the number
     of `=>` occurrences as an approximation.
   - **Conservative starting point**: when T1 first launches, **report without failing**
     (`cargo:warning`); take a full baseline to confirm the heuristic before changing it to
     `panic!`. This follows the established handling of “CI gates are initially red and require a
     baseline.”
2. **Reverse bridges and AST→AST reconstruction points must be categorized separately.** Section 2.2
   records the two production `MonoType → ast::Type` reconstructions at `bytecode.rs:2353-2390` and
   `function.rs:507-529`; section 2.2.2 records the AST→AST reconstructions in the two replacement
   functions in `function.rs` (15 variants / 28 points, including 5 variants / 10 points from the
   13-item list). If T1 mixes them with forward construction, `Int` / `Float` / `Bool` / `Char` /
   `Void` / `String` / `Bytes` and `Union` / `Option` / `Result` / `Sum` / `AssocType` will be
   judged as having construction points and exempted from deletion, directly contradicting the
   disposition table in 5.2. The T1 report must list the three construction categories separately,
   and **fallback exemptions must apply only to individually registered entries** (currently only
   `Void`'s parser error fallback at `types.rs:836`), never to all wildcard arms.

**Gate coverage**: add a `type_tables::validate(root, &entries)` call in `build.rs`, immediately
after the existing block at `build.rs:19-37`, following the
`panic!("Type table validation failed ({} error items ...)", ...)` pattern.

## Detailed Design

### 6.1 Impact on the Type System

| Affected area                                                | Assessment                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact**. Inference consumes `MonoType` throughout and never touches `ast::Type` variants                                                                                                                                                                                                                                                                                                                                    |
| Solver (`types/solver.rs`)                                   | **No impact**. `pub fn unify` (`solver.rs:411`), `types/substitute.rs:112`'s `substitute`, and `types/mono.rs:649`'s `substitute` all operate on `MonoType`                                                                                                                                                                                                                                                                       |
| Constant evaluation (`types/eval/`)                          | **No impact**. `eval/const_eval.rs` (1,128 lines), `eval/dependent_types.rs` (`check_structural_termination` at `478`), and `eval/reducer.rs:572`'s `unify` all operate on `MonoType`                                                                                                                                                                                                                                             |
| Monomorphization (`middle/passes/mono/`)                     | **Changed (WBS 6.3, 2026-10-29)**. Eight locations in `mono/function.rs`—`358`, `362`, `451`, `456`, `554`, `558`, `618`, and `623`—match `AstType::NamedStruct` / `AstType::AssocType`. The disposition of `AssocType` is deletion, and all eight locations were updated in parallel. **`mono_to_ast_type` at `function.rs:507-529` was also changed** (see #47 in 6.3; it now uses `canonical_name` plus structural write-back) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Requires changes**. `ast::Type::NamedStruct` matches at `ir_gen.rs:1266` and `1399`; `let params: Vec<MonoType> = signature_params` at `ir_gen.rs:1380` is another consumer of `signature_params` that must be checked in 5.6                                                                                                                                                                                                   |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Requires changes**. `type_id_to_monotype` at `codegen/bytecode.rs:492-495`, the encoding loop at `351-352`, and assembly at `632`                                                                                                                                                                                                                                                                                               |
| Interpreter (`backends/interpreter/`)                        | **Requires changes**. `image.rs:44`'s `type_table: Vec<ir::Type>` becomes `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, and the element-type change affects the `{:?}` output format                                                                                                                                                                                                          |
| RFC-027 compile-time types                                   | **Requires changes**. `Type::ConstExpr` is retained but marked **compile-time-only**; it disappears after `Type → MonoType` (`types/lower.rs`) and can never be observed by the IR                                                                                                                                                                                                                                                |

**IR-layer type discipline (to be inherited by 04-ssa)**: after convergence, IR type annotations may
only use the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only** variants and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. This rule must be enforced
by 07's first-layer “type consistency” invariant. `ast::BinOp::Assign` / `ast::UnOp::Deref` (2.5)
are analogous expression-side forms—they may occur inside `ConstExpr`, and therefore likewise must
not descend into IR type annotations.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** The basis is:

1. The 13 deleted variants have **no construction points in the parser-forward direction** (`Void`'s
   single forward point was an error fallback—**WBS 6.3 changed that fallback to `Type::Name{"()"}`
   and removed the gate exemption registration**, so there are now no forward construction points) →
   the parser does not produce them (the reverse bridge and AST→AST reconstruction dispositions in
   6.3 #28 / #47 and the 10 points in 6.3.2 have all been implemented).
2. `ir::Type` is an alias of `ast::Type` → changing it back to `MonoType` does not change the set of
   types, only the **carrier**.
3. The **loss** of `From<MonoType> for IrType` (8 variants → `Void`) no longer occurs after the
   change— this **may** change the behavior of some programs, because types that were previously
   lost are now retained. **This is the only direction of behavioral change, and it is a fix rather
   than a regression**, but 07's third-layer corpus differential testing must confirm that there are
   no regressions.

**Semantic change points requiring focused attention**:

| Location                | Current state                                                                                 | After convergence                              |
| ----------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `bytecode.rs:2371-2376` | `Generic` other than `String`/`Bytes`/`Tuple` → `IrType::Void`                                | Retain the real `MonoType::Generic`            |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Retain the real `MonoType`                     |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` fallback                                                                  | **Delete** (an exhaustive match is sufficient) |

> **Design judgment**: the fact that all eight variants currently collapse to `Void` means that
> **every code path that depends on signature bytecode—interpreter arity/type checks and
> serialization formats—currently sees `Void`**. After convergence, these locations will see the
> real types. **This is the only possible source of behavioral change in this document, and its
> direction is “restore correct information.”** If corpus differential testing finds a difference,
> it should be treated as **exposing a pre-existing defect**, not as a regression introduced by this
> document. To determine this, construct a `.yx` program containing a `Ref` parameter and compare
> `dump_bytecode` output before and after the change.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                      | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Category                          |
| --- | ---------------------------------------------------------------------- | ----------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472`    | Delete the `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants (**completed: WBS 6.3.1, 2026-10-29**—all 13 variants deleted and `ast::Type` converged to 13 variants)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Pure deletion                     |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                                 | Add a `kind: NameKind` field to `Name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Data flow                         |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43`                                   | Delete `Expr::FnDef`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Pure deletion                     |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                                 | Delete `Assign.signature_params`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Pure deletion                     |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026`                                    | Delete the `Expr::FnDef` or-pattern                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Follows #3                        |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                                 | Rewrite the `NamedParen` comment as “syntax provides the binder name; semantic interpretation occurs in `sema/`”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Documentation                     |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                           | Add `pub enum NameKind`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | New                               |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                       | `trait TypeEnvProbe` + `NullProbe` (**completed: WBS 6.4.1, 2026-10-29**—the implemented state also includes `BuiltinProbe` and `NameKind`; the trait methods are `is_constraint` / `is_predicate` / `kind_of` / `knows_builtin_types`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | New                               |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                   | Add the `probe` field and accessors (**completed: WBS 6.4.2, 2026-10-29**—the field is `&'a dyn TypeEnvProbe`, and the accessors are `probe()` and `kind_of()`; `ParserState::new` is the sole constructor, and **all 40 construction sites are wired up**: 4 production + 36 test)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Data flow                         |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                                 | Use `state.probe()` to determine `NameKind` (**not done**: 6.4 introduced only the **classification enum + probe queries** and did **not** change the structure of `Type::Name`—fielding it would affect every production/test construction site and require the reverse bridge `mono_to_ast_type` to faithfully write back `kind`, triggering regeneration of IR snapshots and corpus baselines, which conflicts with this batch's C3 zero-drift scope; fielding `NameKind` is deferred until a second consumer appears)                                                                                                                                                                                                                                                         | Data flow                         |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                                 | Update the comment (`Result`/`Option` use `Generic`, already consistent with reality; only add the variant-deletion note)—**not done**: 6.4 did not modify `types.rs` (the change surface in that file belongs to the fallback rework in 6.3), so this line is deferred with #10                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Documentation                     |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                     | Delete the `_ => (Vec::new(), Type::Void)` fallback and return an explicit error instead—**WBS 6.3.1 changed the fallback to a let-else explicit return of `Type::Name{"()"}`** (the `Void` variant was deleted; semantics and formatter output remain byte-for-byte identical). **The “return an explicit error” half was not done**—the original plan deferred it to the error-path rework in 6.4, but **6.4 did not take over that path** (6.4 only injected type-name dependencies and did not modify `types.rs`) ⇒ **moved to 6.5** (registered under the 6.5 line “next-round contract inputs” in 09). This path is syntactically reachable (the `Name` arm of `semantic_tokens` records one additional dummy-span token; there are 0 corpus hits and therefore zero drift) | Behavior                          |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                                 | Delete `n == "Terminates"` and use `state.probe().is_predicate(n)` (**completed: WBS 6.4.3, 2026-10-29**—the implemented state reads `NameKind::Predicate` through `ParserState::kind_of`: predicates declared during the current pass take priority, and all others are delegated to the injected probe)                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Reverse dependency                |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                                 | Delete the `typecheck::operator_interfaces::spec()` call (**completed: WBS 6.4.3, 2026-10-29**—now uses `state.probe().is_constraint(n)`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Reverse dependency                |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                     | Delete the `signature_params` argument—**not done**: belongs to 6.5.2 “with #4,” and is deleted together with `Assign.signature_params`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Follows #4                        |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                      | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Follows #3                        |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                     | Retain (this is the parameter-list retrieval point; under 5.5, read the Lambda directly)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Follows #3                        |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814`    | Delete the match arms for the 13 deleted variants (one per variant) (**completed: WBS 6.3.1, 2026-10-29**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Follows #1                        |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                                 | Remove synonyms (case aliases, `i64`-style abbreviations, `"()"`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Convergence                       |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                     | Retain the `DateTime` alias                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Retain                            |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                       | Move `From<Type> for MonoType` (currently at `mono.rs:692-830`) to the sole conversion point                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | New                               |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                             | Register `i64`/`int` and similar spellings as equivalent forms of `Int`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Convergence                       |
| 23  | `src/middle/core/ir.rs`                                                | Original `3` (now a private `use` at `7`) | Delete `pub use ... ast::Type` (**completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Pure deletion                     |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                     | Keep `FunctionCode.params` as `Vec<MonoType>` (already the case)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | —                                 |
| 25  | `src/middle/core/bytecode.rs`                                          | `814`, `816`                              | Change `BytecodeFunction.params` / `return_type` to `MonoType` (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Convergence                       |
| 26  | `src/middle/core/bytecode.rs`                                          | `856`                                     | Change `BytecodeModule.type_table` to `Vec<MonoType>` (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Convergence                       |
| 27  | `src/middle/core/bytecode.rs`                                          | `2346`                                    | Change the `type_table` conversion to `.collect()` (the elements are now the same type)—the actual change is to **delete the conversion and copy the values unchanged** (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Follows #26                       |
| 28  | `src/middle/core/bytecode.rs`                                          | Original `2352-2390` (**deleted**)        | **Delete** the entire `From<MonoType> for IrType` impl (**completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Pure deletion                     |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                      | Change `type_table` to `Vec<MonoType>` (**completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Convergence                       |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 locations)                 | Handle the `AstType::AssocType` arms according to the gate conclusion—**the conclusion has been made and implemented: WBS 6.3.2 deleted all 5 variants / 10 points registered in 03 §2.2.2 (`Union`/`Option`/`Result`/`Sum`/`AssocType`, 2 each)**, and both bridges (`substitute_type_ast` / `substitute_type_in_ast`) were rewritten in the same batch                                                                                                                                                                                                                                                                                                                                                                                                                          | Completed (WBS 6.3.2, 2026-10-29) |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`              | Type-table gates T1–T3                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Gate                              |
| 32  | `spawn/placement.rs`                                                   | `122`                                     | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Follows #3                        |
| 33  | `spawn/analysis.rs`                                                    | `912`                                     | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #3                        |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                      | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #3                        |
| 35  | `orchestrator.rs`                                                      | `1492`                                    | Same, but obtain the name from `Assign.target`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Follows #3                        |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                            | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #3                        |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                    | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #3                        |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                    | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #3                        |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                    | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #3                        |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                      | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #3                        |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                     | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #3                        |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                    | Same                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #3                        |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                             | Same + comment update                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Follows #3                        |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                                 | Delete the `Expr::FnDef` test                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Test                              |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                                 | Rewrite as Lambda + Assign                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Test                              |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`           | Delete the lower assertions for the 11 dead variants                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Test                              |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                                 | Delete the `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches from `mono_to_ast_type` and uniformly use `AstType::Name` + `mono.type_name()`. **This must be handled in the same batch as #1**—**the criterion has been corrected (completed: WBS 6.3.2, 2026-10-29)**: `mono.type_name()` is the lowercase internal name used for diagnostic display (`int64` / `void` / `string`). After 6.1.2 removes lowercase spellings, it becomes a `TypeRef` and **silently loses type identity**. The actual change therefore uses the newly added `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) plus structural write-back (`Generic` / `Ref` / `Fn`)                                                                | Convergence                       |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                              | Add `probe: &dyn TypeEnvProbe` to the `extract_generic_param_names` signature; delete the `typecheck::operator_interfaces::spec()` call and use `probe.is_constraint(name)`. Callers, including `parser/tests/ast.rs`, pass `&NullProbe` (**completed: WBS 6.4.3, 2026-10-29**—the implemented signature is at `ast.rs:927`; the 3 production callers in `ir_gen.rs` pass `SemanticProbe`, while the others pass `&NullProbe` / `&BuiltinProbe`)                                                                                                                                                                                                                                                                                                                                  | Reverse dependency                |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                   | Delete this file's `name_used_as_type` and call the consolidated version in `ast.rs`; change the `is_predicate` short-circuit to read `NameKind::Predicate` (**completed: WBS 6.4.4, 2026-10-29**—the consolidated implementation is at `ast.rs:848` and covers header detection + predicate short-circuit + `Struct`/`NamedStruct` recursion)                                                                                                                                                                                                                                                                                                                                                                                                                                    | Duplicate implementation          |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                     | Connect the `CONST_PARAM_TYPES` decision through `NameKind::Builtin` (see 5.3) (**completed: WBS 6.4.5, 2026-10-29**—dual-source wiring: `CONST_PARAM_TYPES.contains(name) && (kind_of == Builtin \|\| !knows_builtin_types())`; a probe without the built-in name table falls back to the single-table criterion used before 6.4)                                                                                                                                                                                                                                                                                                                                                                                                                                                | Convergence                       |

> **The six lines implemented in 6.2 (#23 / #25 / #26 / #27 / #28 / #29)**: WBS 6.2 (completed on
> 2026-10-28) implemented all six lines in one batch—four carriers changed to `MonoType`
> (`bytecode.rs:814`, `:816`, `:856`; `image.rs:44`), the lossy `From<MonoType> for IrType` bridge
> and the `ir::Type` re-export alias were deleted, and decoding began copying values unchanged
> (`bytecode.rs:2315`, `:2316`, `:2346`). **The line numbers for these entries have been retaken
> against the implemented state** (the original anchors `812` / `814` / `854` / `2339` / `2352-2390`
> are pre-change positions). See [09a](./09a-wbs-history.md) §P6 6.2 for measured results and gate
> outcomes.

> **The gates must themselves be gated**: `tools/type-tables`, like `code-tables`, requires a `mod`
> declaration to connect it (the repository has already learned this from 8 orphaned test trees).
> When creating the new crate, register it in the root `Cargo.toml` at the same time.

### 6.4 Backward-Compatibility Strategy

**Constraint**: `tests/yaoxiang/` with 293 `.yx` corpus files (single-file), the multi-file corpus
layer established in P2 (`tests/yaoxiang-multifile/`), `src/std/tests/*.yx`, and the standard
library **must all retain identical behavior** (identical diagnostic codes and messages; ordering
may be normalized).

| Risk                                                                       | Assessment                                                                                                                                                                                                                                                                                                        | Mitigation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| -------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source code contains type annotations such as `i64` / `int` / `f64`        | After normalization to `Int`, `from_builtin_name` recognizes only canonical names                                                                                                                                                                                                                                 | **Measured** (at the end of 5.3): 0 noncanonical names in actual type positions → 6.1.2 directly deleted the 22 spellings and **introduced no lexer alias table**; the corpus diagnostic set remains unchanged                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Source code contains `DateTime`                                            | Retain only the uppercase temporary compatibility alias (`mono.rs:883`); it is not a built-in name                                                                                                                                                                                                                | Removing it from the built-in name table belongs to P10 S11 (issue #513); until then, `DateTime` and `Int` are synonymous, with no behavior change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `("()")` occurs in type position                                           | It is not a product of the `Name` form but a spelling of `Void` (standard-library signature strings use `() -> Void`)                                                                                                                                                                                             | **Retain**: `()` shares an arm with `Void` and is one of the final 15 names                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `dump_bytecode` output changes                                             | The element type of `type_table` changes from `ir::Type` to `MonoType`, producing a different `{:?}` format                                                                                                                                                                                                       | **Permitted by C3** (07 specifies that `dump_bytecode` is compared only during C1/C2). The phase-2 output diff must nevertheless be recorded for manual review                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Behavior changes after the lossy bridge is fixed                           | See 6.2                                                                                                                                                                                                                                                                                                           | Corpus differential testing + targeted `dump_bytecode` comparison                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| The type-substitution result changes after `mono_to_ast_type` is rewritten | `function.rs:507-529` is the output of monomorphized type substitution (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back into `AstType`). After the variants are deleted, the `_` arm can change the result from `AstType::Int(64)` to something such as `AstType::Name { name: "i64" }` | Targeted phase-3 comparison: select corpus files containing generic substitution and compare the monomorphized products' `type_name()` before and after the change. **Do not assume that `{:?}` output is equivalent.** **The criterion has been corrected (phase-3 measurement)**: equality must not be judged using `type_name()`—it is the lowercase internal display name, and comparing `type_name()` can reveal only that both are “`int64`,” not whether the canonical name is `Int64` or `i64`. Instead, compare **canonical-name round trips** (`from_builtin_name` ↔ `MonoType::canonical_name()`, mutually inverse for every name) plus the structure of the write-back product. WBS 6.3 measurement: the dumps and `.42` product md5 values for 4 generic-substitution programs are byte-for-byte identical to the pre-change state |
| `.yx` source files themselves                                              | **Zero modifications**                                                                                                                                                                                                                                                                                            | This document does not require changes to any corpus or standard-library source file. This is a hard acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |

**Hard acceptance condition** (**criterion revised on 2026-10-27, following the 2026-10-26 decision
②**): `git diff --stat tests/yaoxiang tests/baselines src/std/` must be empty. **Directly driven
test files** under `tests/` may be modified when mechanically required by the change, but each
modification must be registered with **file / line count / rationale**, and **tests must not be
deleted or assertions weakened**. The original criterion (`git diff --stat tests/ src/std/`) imposed
a blanket zero-diff requirement over all of `tests/`; this revision relaxes the requirement by one
level for directly driven test files only. The zero-diff requirements for the corpus layer
(`tests/yaoxiang`), baseline layer (`tests/baselines`), and standard library (`src/std/`) remain
unchanged. **Exception**: the 6.8.2 behavior-fix batch receives a separate round of adjudication.

## Implementation Essentials

The criterion used throughout is 07's **C3: identical diagnostic codes and messages (ordering may be
normalized)**. The following is this document's phase breakdown; the global ordering across
documents is defined in RFC-039.

| Phase                                         | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Rollback point                         |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| **0. Gates first**                            | Create `tools/type-tables` and implement T1/T2/T3. The first version of T1 **reports without failing**. Take a full baseline against the **unmodified** code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | The T1 report accurately lists the 13 **parser-forward zero-construction** variants, including the classification of `AssocType`: 0 forward / 2 AST→AST, with D8 unchanged, and **separately lists the reverse-bridge reconstruction points from 2.2 and the AST→AST reconstruction points from 2.2.2**. The 13 parser-forward zero-construction variants are reported separately from the 1 globally zero-construction variant (`Enum` only). T2/T3 are green or provide a repair list | No code changes; no rollback required  |
| **1. Name normalization**                     | **Completed (WBS 6.1.2, 2026-10-27)**: `from_builtin_name` (`mono.rs:839-886`) converged to 15 names + the temporary `DateTime` alias, deleting 22 in-implementation spellings; `add_builtin_types`' 7 lowercase duplicates, the LSP list's `Uint` / `Int128`, and `sizeof`'s `Uint` arm were cleaned up in the same batch; **no lexer alias table was introduced** (corpus measurement found 0 noncanonical names). **In parallel, verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`**                                                                                                                                                                                                                                                                                                                                                                                                       | C3: every corpus diagnostic and the phase-0 baseline are **identical entry by entry**                                                                                                                                                                                                                                                                                                                                                                                                   | Separate commit; revertible            |
| **2. Bytecode type unification**              | **Completed (WBS 6.2, 2026-10-28)**: all four carriers changed to `MonoType`—`BytecodeFunction.params` (`bytecode.rs:814`) and `return_type` (`:816`), `BytecodeModule.type_table` (`:856`), and `Image.type_table` (`image.rs:44`); deleted the lossy `From<MonoType> for IrType` bridge (originally `bytecode.rs:2352-2390`) and the `ir::Type` re-export alias (the `pub use` at `ir.rs:3` became a private `use`); decoding now copies values unchanged (`bytecode.rs:2315`, `:2316`, `:2346`), and the previous silent loss of information when eight `MonoType` categories collapsed to `Void` no longer occurs                                                                                                                                                                                                                                                                                                           | C3 + targeted `dump_bytecode` comparison (each row of the 6.2 table)—**measured fully green** (record in [09a](./09a-wbs-history.md) §P6 6.2)                                                                                                                                                                                                                                                                                                                                           | Separate commit; revertible            |
| **3. Dead-variant deletion**                  | **Completed (WBS 6.3, 2026-10-29)**: delete the 13 variants in the 5.2 table (`ast::Type` 26 → 13); remove match arms such as `mono.rs:699-708` and the `types.rs:836` fallback (changed to a let-else explicit return of `Type::Name{"()"}`); **handle `function.rs:507-529` (#47) and the 10 AST→AST points (6.3.2) in the same batch**; update `types/tests/mono.rs` in parallel. **The criterion for #47 has been corrected**: the original wording, “uniformly use `AstType::Name` + `mono.type_name()`,” became invalid after 6.1 because `type_name()` is the lowercase internal diagnostic-display name (`int64` / `void` / `string`), and after 6.1.2 removes lowercase spellings it becomes a `TypeRef` and **silently loses type identity**. The actual implementation uses the newly added `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) plus structural write-back | C3 + `cargo build` fully green (exhaustive matches force every arm to be completed) + targeted generic-substitution comparison (final row of 6.4)—**measured fully green** (record in [09a](./09a-wbs-history.md) §P6 6.3)                                                                                                                                                                                                                                                              | Separate commit; revertible            |
| **4. Parser data flow**                       | **Completed (WBS 6.4, 2026-10-29)**: add `probe.rs` (`TypeEnvProbe` + `NullProbe` + `BuiltinProbe` + `NameKind`); modify `parser_state.rs` (add the `probe` field and `probe()` / `kind_of()` accessors; **all 40 construction sites are wired up** = 4 production + 36 test); delete the 2 parser-side `operator_interfaces::spec()` calls and change the 1 middle-side and 1 driver-side call to the narrow `is_constraint_name` entry point (#48); consolidate the two `name_used_as_type*` implementations into the single implementation at `ast.rs:848` (#49); connect `CONST_PARAM_TYPES` through the dual-source `NameKind::Builtin` wiring (#50). **Not done**: add a `kind` field to `Type::Name` at `types.rs:162-165` (#10; fielding it conflicts with this batch's C3 scope), and change the `types.rs:836` fallback to an explicit error return (#12, moved to 6.5)                                               | C3 + **new assertion**: `grep 'typecheck' src/frontend/core/parser/` must return 0—**both greps measured (`typecheck` / `operator_interfaces`) with 0 hits**; corpus 356/356, IR snapshot drift 0 across 216 entries, `cargo test --all` exit 0 (lib 2868 → 2875 passed, +7)                                                                                                                                                                                                            | Separate commit; revertible            |
| **5. Dead AST variants and dedicated fields** | Delete `Expr::FnDef` (16 locations) + `Assign.signature_params`; migrate the semantics of `NamedParen`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | C3 + all 18 modules under `tests/integration/` green                                                                                                                                                                                                                                                                                                                                                                                                                                    | Two commits (FnDef / signature_params) |
| **6. Convert gates to hard failures**         | Change T1 from a warning to `panic!`; downgrade T4 to a CI assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | `cargo build` **must fail** when a zero-construction variant is deliberately introduced (red test first)                                                                                                                                                                                                                                                                                                                                                                                | Separate commit                        |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Phases 3 and 5 must follow phase 2**—before phase
2 lands, the bytecode-side reverse bridge A (`bytecode.rs`'s `From<MonoType> for IrType`) consumes
`IrType::String` / `IrType::Bytes`, so deleting the variants first would break compilation. That
bridge was deleted in 6.2.2, and **the remaining rewrite surface when phase 3 deletes the variants
is reverse bridge B** (`middle/passes/mono/function.rs`'s `mono_to_ast_type`, which still resolves
`String` / `Bytes` by name) and the three `ast::Type` carriers in `middle/core/ir.rs`. Phase 3's #47
must be handled in the same batch as #1; otherwise, `mono_to_ast_type` silently changes behavior.
**Phase 3 was completed on 2026-10-29 (WBS 6.3)**: reverse bridge B now uses `canonical_name` plus
structural write-back; the three `ast::Type` carriers in `middle/core/ir.rs` **were not
changed**—6.3 only deleted variants and did not alter the IR shape (all 216 IR snapshots and all 356
corpus baseline lines remained drift-free), so carrier disposition is deferred to phase 4.

**Downstream dependency**: [04-ssa.md](04-ssa.md) may begin only after all phases in this document
are complete. At that point, the element types of `BytecodeFunction.params`, `FunctionCode.params`,
and `type_table` are consistent, giving SSA type annotations a unique destination.

### Mandatory Validation for Each Phase

| Phase | Required execution                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`; manually verify the T1 report (**including the reverse-bridge reconstruction points**)                                                                                                                                                                                                                                                                                                                                   |
| 1–5   | `cargo test`; full-corpus differential testing over `tests/yaoxiang/` (07's third layer); entry-by-entry C3 comparison                                                                                                                                                                                                                                                                                                                                                                         |
| 2     | Additionally: construct one `.yx` sample for each variant in the 6.2 table and compare `dump_bytecode` before and after the change                                                                                                                                                                                                                                                                                                                                                             |
| 3     | Additionally: for corpus files containing generic substitution, compare the monomorphized products' `type_name()` before and after the change—**the criterion has been corrected (phase-3 measurement)**: `type_name()` is the lowercase internal diagnostic-display name and is insufficient for equality; instead, compare **canonical-name round trips** (`from_builtin_name` ↔ `MonoType::canonical_name()`, mutually inverse for every name) plus the structure of the write-back product |
| 4     | Additionally: `rg 'typecheck' src/frontend/core/parser/` must return 0                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 6     | Additionally: deliberately write a zero-construction variant and confirm that `cargo build` fails (red → green)                                                                                                                                                                                                                                                                                                                                                                                |

## Key Decisions and Rationale

### Decisions

| Decision                     | Resolution                                                                                                                     | Rationale                                                                                                                                                                                                                                |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Target representation form   | `Type` as the sole type structure + `MonoType` as the working representation; delete the `ir::Type` alias                      | The alias at `ir.rs:3` directly causes the field-type mismatch between `BytecodeFunction.params` and `FunctionCode.params` (2.1). After deleting the alias, both field types are unified and the `From` bridge loses its reason to exist |
| Reject “unify on `MonoType`” | Not adopted                                                                                                                    | It would introduce an L2→L3 reverse dependency (violating RFC-039's layering) and require adding `Span` / `ConstExpr` back to `MonoType`. See 5.1 for the full argument                                                                  |
| Do not delete `NamedParen`   | Retain it as a pure syntax node and migrate binder semantics into `sema/`                                                      | `ast.rs:519-532` records that deletion would reintroduce an already-fixed correctness defect: `(r: P(m))` would silently treat the undeclared `m` as a binder                                                                            |
| Directory rename             | **Do it** (D1), executed as a pure move batch at the end of P5/P6; this document's body paths use the current pre-rename paths | Decisions of 2026-10-03 (ChenXu233) and 2026-10-04 supplement on implementation method                                                                                                                                                   |

### Rejected Alternatives

None of the four alternative approaches was adopted: **retaining all three representations and
adding only a conversion layer** resolves none of the root causes, and `ir::Type` would still not be
an independent type, while the dead variants, synonym table, and reverse dependency would all
remain. **Using a macro to generate the synonym table** treats a symptom rather than the cause—once
`Type::Int(usize)` is removed, the macro has no reason to exist, and consistency should instead be
guaranteed by the T2 gate, whose correctness is more verifiable than macro expansion. **Deleting
only the dead variants without unification** has the lowest cost but leaves the problem in place,
with no gate preventing the next person from adding `Type::Int128(usize)` and recreating the same
dead-variant pattern; if only one commit can be made, it should explicitly be recorded as a subset
of this approach and include the T1 gate. **Unifying on `MonoType`** is addressed in 5.1.

### Benefits

- **Eliminates maintenance burden at 14 locations**. `Expr::FnDef`'s 12 consumption sites + 2
  exhaustive-match arms form a parallel path that never executes; deleting it removes a branch from
  each of `ir_gen.rs`, `ownership.rs`, and `termination.rs`.
- **Eliminates one diverged duplicate implementation**. The two copies of `name_used_as_type` (2.4)
  already differ in recursion through definition-body fields; after consolidation, only one remains.
- **After the lossy bridge disappears, `IrType::Void` is no longer a “valid type value.”** This is a
  prerequisite for 07's first-layer “type consistency” invariant to pass genuinely.
- **The gates make regression impossible**. T1 turns “adding a zero-construction variant” from a
  silent pass into a build failure—the existing `build.rs` mechanism in this repository proves that
  this approach works.
- **Zero source changes**. The 293 `.yx` corpus files and the standard library require no
  modifications, which is the fundamental reason the C3 criterion can hold.

## Known Limitations and Risks

### Risks

- **Phase 2 may cause behavioral changes**. After the lossy bridge is fixed, eight `MonoType`
  variants expose their real types rather than `Void`. The direction is corrective, but paths that
  depend on signature bytecode will change. This is the largest uncertainty in this document.
- **The classification of `AssocType` depends on gate accuracy**. If T1 uses a line-level heuristic
  (counting `=>`) rather than `syn`, it may misclassify the variant. This is why manual verification
  is mandatory in phase 0.
- **If T1 fails to count reverse bridges, the disposition table and gate will contradict each
  other**. Section 2.2 proves that seven “dead variants” have production reconstruction points in
  reverse bridges. If T1 does not categorize construction-point direction, those seven will be
  judged as having construction points and exempted from deletion—the gate would obstruct the
  deletion work rather than protect it. | **2. Deleting `signature_params` carries semantic risk**.
  It carries the **unfiltered** parameter list, whereas `Lambda.params` is **filtered**. If
  `classify_generic_params` depends on the former, the change will alter generic-classification
  results. **This must be verified first and must not be assumed to be equivalent** (the
  `signature_params` consumer at `ir_gen.rs:1380` must be checked in parallel). | **3. Phase 3
  affects `middle/passes/mono/function.rs`**. In addition to the eight `AssocType` matches,
  `507-529`'s `mono_to_ast_type` is the output of monomorphized type substitution, and rewriting it
  may change the substitution result's `type_name()` (final row of 6.4).
- **Noncanonical type names in the `.yx` corpus may not be zero**. If the corpus uses `i64` / `int`
  extensively, phase 1's “directly delete synonyms” approach is infeasible and must regress to “move
  them into a lexer alias table,” reducing the scope of `from_builtin_name`'s convergence.
  (**Measured as 0**: the 6.0 survey at the end of 5.3 found none, so WBS 6.1.2 directly deleted the
  aliases rather than taking the alias-table route.)
- **The gates will initially be red**. If T1 fails immediately on its first launch with `panic!`, it
  will block compilation. It must first report without failing so that a baseline can be
  established.

### Open Questions

> **All open questions originally listed in this section have been adjudicated.** See the
> [RFC-039 decision registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> individual decisions. **This document contains no unresolved items.**

## See Also

### Governing and Peer Documents

- [RFC-039 Compiler Feature Routing Catalog Design (Master Blueprint)](../../rfc/accepted/039-compiler-architecture.md)
  — governing blueprint; four-layer model, criteria tiering, routing table C (3
  type-representation-related entries are assigned to this document)
- [01-routing.md](01-routing.md) — feature routing tables A/B/C, dependency-direction rules, target
  directory structure
- [04-ssa.md](04-ssa.md) — SSA conversion; this document's prerequisite and downstream consumer
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — frontend paradigm; consolidation of the
  operator-change surface
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C3 category definition, diagnostic-set
  comparison specification, type-consistency invariant for the first-layer IR validator

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — basis of the
  unified `Type::Generic` path
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — built-in type table
  (comparison target for gate T3)
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — example
  of a build-time gate
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — origins of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

- `src/frontend/core/parser/ast.rs:427-541` — definition of the 26-variant `ast::Type`
- `src/frontend/core/parser/ast.rs:519-540` — explanation of `Type::NamedParen` binder semantics
  (evidence that deletion would reintroduce a defect)
- `src/frontend/core/parser/ast.rs:241-249` — typechecker-only `StmtKind::Assign.signature_params`
  field
- `src/frontend/core/parser/ast.rs:838-841` — `CONST_PARAM_TYPES`: the third copy of the
  const-generic parameter names
- `src/frontend/core/parser/ast.rs:843-885` — `name_used_as_type_in`: the duplicate of
  `declarations.rs:42` (already diverged)
- `src/frontend/core/parser/ast.rs:891`, `930` — `extract_generic_param_names` and its
  boundary-crossing call
- `src/frontend/core/parser/ast.rs:191-213`, `217-223` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — parser representation of every primitive
  type as `Type::Name`
- `src/frontend/core/parser/statements/types.rs:392-396` — `Result`/`Option` are not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:836` — sole parser-forward construction point of
  `Type::Void` (error fallback)
- `src/frontend/core/parser/statements/declarations.rs:42-80` — original `name_used_as_type`
- `src/frontend/core/parser/statements/declarations.rs:494-511` — hardcoded `"Terminates"` and the
  `typecheck::operator_interfaces::spec()` call
- `src/frontend/core/parser/pratt/nud.rs:505-514` — expression position represented as an empty-body
  `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:466` — `expr_to_params` arm retrieves the parameter list
- `src/frontend/core/types/mono.rs:839-886` — `from_builtin_name` built-in name resolution table (15
  names + temporary `DateTime` alias; WBS 6.1.2)
- `src/frontend/core/types/mono.rs:692-830` — `From<Type> for MonoType` implementation (12 of the 13
  match arms are here)
- `src/frontend/core/types/mono.rs` — definition of `MonoType` (approximately 22 variants, beginning
  at `mono.rs:167`; 1,077-line file)
- `src/frontend/core/types/const_data.rs:234-258`, `321-330` — `const_data::BinOp` /
  `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — compile-time constant evaluation (1,128 lines)
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- `src/middle/core/ir.rs:3` — evidence that `ir::Type` is an alias of `ast::Type`
- `src/middle/core/bytecode.rs:808-814` — `Vec<ir::Type>` fields of `BytecodeFunction`
- `src/middle/core/bytecode.rs:2352-2390` — lossy `From<MonoType> for IrType` bridge
- `src/middle/passes/codegen/bytecode.rs:32`, `275-278` — `Vec<MonoType>` fields of `FunctionCode`
- `src/middle/passes/mono/function.rs:507-529` — `mono_to_ast_type`: reverse bridge reconstructing 7
  “dead variants”
- `src/backends/interpreter/image.rs:44` — interpreter module's `type_table`
- `build.rs:19-37` — build-time error-code registry gate (model for the type-table gate)
- `build.rs:39-54` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — gate crate depends only on `serde_json` (the root crate has
  no `syn`; introducing `syn` in `tools/type-tables` is the first exception)
