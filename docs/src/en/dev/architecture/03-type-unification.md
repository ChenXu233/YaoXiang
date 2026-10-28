# Type Representation Unification

> **Supporting design document.** This document supports
> [RFC-039 Compiler Architecture Refactor](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, acceptance criterion tiers, and execution phase order are defined in the main
> RFC-039 text; the role of each supporting document is described in the
> [Index for This Directory](index.md).

## Scope and Positioning

### Coverage

Consolidate the multiple parallel type representations currently in use into a single
representation, thereby eliminating the four categories of burden derived from them:

- **13 `ast::Type` variants with zero parser-forward construction sites** (among the 26 variants in
  `ast.rs:427-541`; see 2.2; the “forward” definition excludes reverse bridges and AST→AST
  reconstruction points)
- **One hand-written type-name synonym table** (`mono.rs:839-886`; see 2.3; WBS 6.1.2 has
  consolidated this into a 15-name resolution table)
- **The parser’s genuine reverse dependency on the typecheck layer** (2 locations; see 2.4)
- **One expression variant with zero production construction sites**, `Expr::FnDef` (`ast.rs:37-43`;
  see 2.6)

There are four concrete deliverables: a disposition table covering all 26 variants individually
(5.2), semantic typing for `Type::Name` through `NameKind` and elimination of the synonym table
(5.3), an injected `TypeEnvProbe` data flow (5.4), and generation-time type-table gates T1–T3 (5.7).

### Directory Renaming and This Document’s Paths

RFC-039 decision D1 calls for renaming the execution directories (`typecheck/`→`sema/`,
`middle/core/`→`middle/ir/`), **to be completed together with P5/P6 rather than in separate
phases**. This document’s 50-item change list and all acceptance greps are written against the
**pre-rename paths**—they are the implementation baseline for the P6 batches **before the rename
batch**; the directory rename at the end of each batch is a **pure relocation batch** (C1 zero-diff)
in a separate commit. The baseline becoming invalid after the rename commit is expected; use git
history as the bisection basis.

### Not Covered

- **How the directory rename is implemented.** `Type` is not moved out of `parser/ast.rs` in this
  phase; only `lower.rs` is added under `types/` as the single conversion point. The directory
  rename (`typecheck/`→`sema/`, etc.) is performed at the end of P5/P6 according to RFC-039 decision
  D1, in the form shown in the target directory structure in `01-routing.md`.
- **The four-layer model and dependency-direction rules.** See [01-routing.md](01-routing.md).
- **Tiering of equivalence criteria and checker implementation.** The definitions of C1–C6 and the
  three criterion layers (IR checker / normalized snapshots / corpus differential) are in
  [07-equivalence-oracle.md](07-equivalence-oracle.md). This document uses only **C3 (type
  representation convergence)** as its acceptance criterion.
- **SSA conversion.** See [04-ssa.md](04-ssa.md). This document is a prerequisite because of the
  reasons described below.
- **The global P1–P10 execution sequence.** See the main RFC-039 text and
  [02-stage-contract.md](02-stage-contract.md). This document defines only the phase breakdown for
  its own workstream (see “Implementation Essentials”).

### Division of Labor with RFC-039

RFC-039 is the overarching plan: it defines the four-layer model, criterion tiers, cross-document
phase order, and “why this refactor should happen now.” This document is the implementation plan for
**one part of RFC-039: type representation convergence**. It specifies which variants to remove,
which files to modify, the order of the changes, and the gates that prevent regressions.

All line numbers, variant lists, and construction-site counts in this document come from a static
audit of `src/`; each item provides `file:line` evidence. **This document does not record the
execution process; it records code facts and the dispositions derived from them.**

For cross-references with documents in the same batch, the “Target Design” and “Detailed Design”
chapters retain their `5.x` and `6.x` subsection numbering; all other chapters follow the
directory’s common style.

### Why This Must Precede SSA Conversion

`ir::Type` is not a third independent type representation; it is a `pub use` alias for `ast::Type`
(`ir.rs:3`). [04-ssa.md](04-ssa.md) will introduce register type annotations and new instructions to
the IR. If the representation is not consolidated beforehand, SSA type annotations will follow the
`ir::Type` alias and attach directly to the syntactic type’s 26 variants, thereby institutionalizing
a third representation.

The current capability boundary of `ir::Type` is defined by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) embeds a complete expression tree inside a type.
  An `Expr` cannot appear in an IR instruction’s type annotation.
- `ast::Type::Literal { name, base_type }` (`ast.rs:476-482`) carries a source name and `Span`. It
  is the syntactic form of a compile-time literal type, not a runtime type.

Adding SSA type annotations on top of the 26 variants would permanently entrench the 13 dead
variants and the two syntax-specific variants in the IR contract.

## Current State

Everything in this section is a statement of code fact, not a proposal. Each item provides
`file:line` evidence.

### 2.1 Three Parallel Representations and a Lossy Bridge

> **Status note (2026-10-28, WBS 6.2 implemented):** This section records the state **before** WBS
> 6.2 landed. WBS 6.2 (6.2.1 / 6.2.2 / 6.2.3) changed the four type carriers to use `MonoType`,
> removed the lossy bridge `From<MonoType> for IrType`, and removed the `ir::Type` re-export alias.
> All statements below concerning `ir::Type` or the lossy bridge are **pre-change facts**; items
> marked “(deleted in 6.2.2)” describe dispositions that are already in effect. Implementation
> details and measured results are in [09a](./09a-wbs-history.md) §P6 6.2.

The multiplicity of type representations is the common upstream cause of four independent defects:

| Downstream symptom                                                               | Causal relationship to type representation                                                                                                                                                                                                                                                                                                              |
| -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The same type text has multiple valid AST forms                                  | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but structurally different**, and the parser selects the latter, typecheck must supply a table to translate the former back                                                                           |
| Bytecode-layer signatures differ from IR-layer signatures                        | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only obtain the alias. Thus `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`), while `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) declares `params` as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding one builtin type name requires changes in multiple places, with no checks | Because the name-to-type mapping is scattered across `from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) and the string match in `bytecode.rs:2371-2376`; both locations duplicate the mapping without cross-checking it                                                                                                                    |
| The parser hard-codes `"Terminates"`                                             | Because the parser cannot determine whether something “is a type” or “is an operator interface,” it must embed type-layer knowledge in the syntax layer as a string literal (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                                 |

**The first three items are representation problems; the fourth is a downstream side effect of a
representation problem.** Doing only item 1 (removing dead variants) leaves the synonym table and
reverse dependency in place; doing only item 4 (injecting a callback into the parser) builds on an
inconsistent representation.

One of the three representations is an alias illusion:

| Name        | Definition                                | Size        | Role                                                    |
| ----------- | ----------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541` | 26 variants | Syntax-layer type                                       |
| `MonoType`  | `src/frontend/core/types/mono.rs`         | 1,077 lines | Semantic-layer type (after monomorphization)            |
| `ir::Type`  | `src/middle/core/ir.rs:3`                 | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

`ir.rs:3` is a one-line `pub use`, and `ir.rs:6` immediately performs
`use crate::frontend::core::typecheck::MonoType;`. The IR module therefore depends on two “type”
symbols, even though one is only an alias.

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
    _ => IrType::Void,          // ← Any other generic type silently becomes Void
},
// bytecode.rs:2378-2385
MonoType::Struct(_) | MonoType::Enum(_) | MonoType::Ref { .. }
    | MonoType::TypeVar(_) | MonoType::TypeRef(_) | MonoType::Union(_)
    | MonoType::Intersection(_) | MonoType::AssocType { .. } => IrType::Void,
```

All eight `MonoType` variants collapsed into `IrType::Void`, in addition to the `_ => IrType::Void`
fallback (`bytecode.rs:2386-2387`; its comment explicitly describes it as a “transitional branch
before deleting enum variants”).

Here, `IrType::Void` is a **legitimate type value, not an error signal**. Downstream code therefore
cannot distinguish “this function really returns `Void`” from “this function’s return type was lost
during bridging”—one reason why the first-layer “type consistency” invariant in 07 cannot currently
pass on this code.

At the same time, `IrType::String` / `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`) and
`ast::Type::Bytes` (`ast.rs:436`), which are precisely the dead variants targeted in 2.2—**both ends
of the lossy bridge point to dead code**.

**Current state (since 6.2.2):** The impl has been **removed entirely**, and decoding no longer
downgrades and reconstructs `MonoType` as `ast::Type`—all four carriers now uniformly use
`MonoType`: `BytecodeFunction.params` (`src/middle/core/bytecode.rs:814`) and `return_type` (same
file, `:816`), `BytecodeModule.type_table` (same file, `:856`), and `Image.type_table`
(`src/backends/interpreter/image.rs:44`); decoding now carries the values through unchanged
(`bytecode.rs:2315`, `:2316`, `:2346`). The collapse described above **no longer occurs**: those
eight `MonoType` variants retain their real types in signatures and type tables, and `IrType::Void`
is no longer used as a substitute for “information loss.” The `ir::Type` re-export alias
(`pub use ... ast::Type` at `src/middle/core/ir.rs:3`) has also been reduced to a private import
(`:7`): the IR layer no longer publicly re-exports the syntax-layer type. However, the IR layer
still has three carriers that directly hold `ast::Type` (`Instruction::Cast.target_type` /
`Instruction::TypeTest.ty` / `FunctionBody::TypeDecl.definition`), pending disposition in 6.3.

### 2.2 Parser-Forward Zero-Construction Variants

**Definition of the metric:** The construction-site scan covers all of `src/`, excluding `tests/`
and `*/tests/*`. **Construction site = a `Type::<variant>` path in expression position** (pattern
positions are not counted). Sites are divided into **three separate categories and must not be
conflated**: ① **parser-forward** (parser → AST); ② **AST→AST reconstruction** (the two replacement
functions in `middle/passes/mono/function.rs`); ③ **reverse bridges** (`From<MonoType> for IrType`
and `mono_to_ast_type`).

Of the 26 variants of `ast::Type`, the following 13 have **zero parser-forward construction sites**
(reconstruction sites in the other two categories are listed in 2.2.1 / 2.2.2):

| Variant                              | Definition       | Parser-forward construction sites                                                                                                                                                                                                                                                                                |
| ------------------------------------ | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Int(usize)`                         | `ast.rs:432`     | None (`mono.rs:699` is a match arm)                                                                                                                                                                                                                                                                              |
| `Float(usize)`                       | `ast.rs:433`     | None (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                              |
| `Char`                               | `ast.rs:434`     | None (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                              |
| `String`                             | `ast.rs:435`     | None (`mono.rs:702` is a match arm)                                                                                                                                                                                                                                                                              |
| `Bytes`                              | `ast.rs:436`     | None (`mono.rs:703` is a match arm)                                                                                                                                                                                                                                                                              |
| `Bool`                               | `ast.rs:437`     | None (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                              |
| `Void`                               | `ast.rs:438`     | `types.rs:836` (**the only forward site = error fallback** `_ => (Vec::new(), Type::Void)`, registered as an individual exemption; eliminated with 6.4 #12)                                                                                                                                                      |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | None (`mono.rs:747` is a match arm)                                                                                                                                                                                                                                                                              |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | None (`mono.rs:743` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                                   |
| `Option(Box<Type>)`                  | `ast.rs:455`     | None (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                                   |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | None (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                                   |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | None (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                                   |
| `AssocType { .. }`                   | `ast.rs:463-471` | None (`mono.rs:781-786`, `formatter/handlers/types.rs:84`, and `semantic_tokens.rs:230` are match arms); **2 production AST→AST reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3; **D8 remains unchanged** |

> **Definition-column line-anchor drift registration (P6 6.0 measurement, 2026-10-27):** The
> “Definition” column in this table and in the 5.2 table was written on 2026-10-05. Since then, 17
> lines were added before the `ast::Type` enum (`pub enum Type` moved from 427 to 444), so the
> **definition lines for all 26 variants shifted uniformly by +17** (for example, `Int` moved from
> 432 → 449, and `NamedParen` moved from 533-540 → 550-557). This round only **registers** the drift
> and does not alter the table body; all subsequent references use the `ast::Type` symbol anchor.

The root-cause chain can be fully verified:

1. **The parser represents all primitive types as `Type::Name { name }`**—the fallback arm in
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` / `Option` are not lowered into dedicated nodes**—the comment in `types.rs:392-396`
   explicitly states: “RFC-010: Result/Option are not lowered into dedicated AST nodes… They use the
   same `Type::Generic` path as user-defined generic types. The type representation remains
   `Generic{"Result"/"Option", args}`.”
3. **The only forward construction site for `Type::Void` is an error fallback**—`types.rs:836`
   contains `_ => (Vec::new(), Type::Void)` at the end of a match returning `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction sites: `types.rs:364`, `types.rs:571`, `types.rs:100`, and
`declarations.rs:573`; they are not dead variants.

**These 13 variants have construction sites in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
Therefore, this document uses the term **“zero parser-forward construction”**: it means neither
“zero references globally” nor “zero production construction”—11 of these variants do have
production construction sites in the other two categories. The tests themselves codify the behavior
under test: they assert exactly what these syntactic forms lower to, even though the parser never
emits them. When the variants are removed, these tests must be removed or rewritten accordingly.

#### 2.2.1 Metric Boundary (1): Reverse Bridges Reconstruct Seven of Them

The “zero forward construction” classification above applies only to the **parser → AST
forward-construction direction**. In the reverse direction, there are two production-grade
`MonoType → ast::Type` reconstructions that **actually construct seven of the variants in the table
above**:

| Reverse bridge                                                 | Location                                 | Reconstructed members of the 13-variant set                                                                              |
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
    // #299：String/Bytes 等容器类型现为 Generic，经 type_name 反解回 AstType
    MonoType::Generic { name, .. } if name == "String" => AstType::String,
    MonoType::Generic { name, .. } if name == "Bytes" => AstType::Bytes,
    _ => AstType::Name { name: mono.type_name(), span },
}
```

**This metric boundary directly affects disposition:**

- The `bytecode.rs` location disappears with item #28 in 6.3 (removing the entire
  `From<MonoType> for IrType` impl), so it adds no separate cost.
- `function.rs:507-529` **is not covered by any existing change item**. It is a `MonoType → AstType`
  type-name reverse resolver (marked `#299` in its comment), converting `String` / `Bytes` from
  `Generic` back into dedicated variants by name. After removing `ast::Type::String` / `Bytes`,
  these two guard branches must be rewritten; after removing `Int` / `Float` / `Bool` / `Char` /
  `Void`, the behavior of the `_` arm changes (some types that previously fell into `_` would
  silently switch to `AstType::Name`). This is item 47 added in 6.3.
- **Direct consequence for the gate:** A T1 gate that scans only forward construction sites would
  classify the seven variants above as having construction sites, thereby excluding them from the
  removable list. T1 must explicitly include reverse bridges in its metric, or its conclusions will
  contradict the disposition table in this document (see 5.7).

#### 2.2.2 Metric Boundary (2): AST→AST Reconstruction (the Third Category Added in This Revision)

`substitute_type_ast` (320-497) and `substitute_type_in_ast` (499-655) in
`middle/passes/mono/function.rs` perform **AST→AST** type-variable substitution by reconstructing an
existing AST type; they are **not** `MonoType → AST`. P6 T1 gate measurements (6.0.2 baseline,
2026-10-27) found **15 variants / 28 sites**, of which **5 variants / 10 sites** belong to the
13-variant list above:

| Variant     | AST→AST reconstruction sites |
| ----------- | ---------------------------- |
| `Union`     | `377`, `606`                 |
| `Option`    | `427`, `600`                 |
| `Result`    | `431`, `602`                 |
| `Sum`       | `467`, `632`                 |
| `AssocType` | `456`, `623`                 |

**When 6.3 removes these five variants, it must rewrite all 10 sites in the same batch** (registered
in 09 under 6.3.2); otherwise, the affected arms will no longer compile. **Implemented (WBS 6.3.2,
2026-10-29):** All 10 sites were removed. See the disposition table in [09a](./09a-wbs-history.md)
§P6 6.3.

### 2.3 Hand-Written Synonym Table

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`; measured at 841-867 / **13 arms**
when this section was written, and at **839-886 / 15 arms / 16 spellings** after WBS 6.1.2) uses a
string table to map type names to `MonoType`:

```
"Int" | "int" | "Int64" | "int64" | "i64"  => Some(MonoType::Int(64)),   // :843
"DateTime" | "datetime"                     => Some(MonoType::Int(64)),   // :850
"Int32" | "int32" | "i32"                   => Some(MonoType::Int(32)),   // :851
...
"Void" | "void" | "()"                      => Some(MonoType::Void),      // :863
```

**The sole reason this table exists is to bridge an inconsistency in the AST layer**: because
`Type::Int(64)` and `Type::Name { name: "Int" }` are semantically equivalent but structurally
different at the AST layer—and the parser emits only the latter—`from_builtin_name` must accept
casing variants, abbreviations, signed names, and the `DateTime` alias. The appearance of `"()"` on
the `Void` line is especially revealing: it exists to accommodate the `Type::Void` fallback in
`types.rs:836`.

The same knowledge about “which names are builtin type names” exists in seven copies throughout the
repository, with no consistency checking between them:

| #   | Location (**measured on 2026-10-27**; this section was written on 2026-10-05, so the original line numbers have drifted)             | Form                                                                                                                                                                                                                           |
| --- | ------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | `MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`, measured at 839-886 / **15 arms / 16 spellings**; after WBS 6.1.2) | String match, name → `MonoType`                                                                                                                                                                                                |
| 2   | `src/middle/core/bytecode.rs:2371-2376`                                                                                              | String match, duplicating `"String"` / `"Bytes"` / `"Tuple"`                                                                                                                                                                   |
| 3   | `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`, measured at 855-858)                                                         | `&[&str]` constant containing 15 const generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`)                                                                                       |
| 4   | `src/lsp/world.rs:182`                                                                                                               | LSP-side list of builtin type names (14 entries = the 15 names minus `()`; WBS 6.1.2)                                                                                                                                          |
| 5   | `ConstKind::from_ast_type_name` (`src/frontend/core/types/const_data.rs`, measured at 407)                                           | String match, name → `ConstKind`. **Its coverage differs from item 1**: it does not accept lowercase or the `i64` / `i32` family of aliases (registered by the P0 gate on 2026-10-06)                                          |
| 6   | Base-type size table in `eval_builtin` (`src/frontend/core/types/eval/const_eval.rs`, measured at 497-512)                           | String match, name → byte size. It previously contained a `"Uint"` arm (item 1 has no `Uint` mapping)—WBS 6.1.2 removed that arm, and `sizeof(Uint)` now reports an unknown type. **Dead arm, living evidence of table drift** |
| 7   | `to_type_id` (`src/middle/passes/codegen/bytecode.rs`, measured at 735-775; `"Set" => 25` at 762)                                    | String match, builtin name → serialized type ID. `"Set" => 25` contradicts the typecheck-side “Set is only a name” decision (`inference/expressions.rs:2917-2918`, decision #300)                                              |

> **Line-anchor drift registration (P6 6.0 measurement, 2026-10-27):** The line numbers for all
> seven copies in this section **have drifted** (item 1 by +223 lines and the others by 2–9 lines),
> but **the number of entries and their contents have not changed** (item 1 had 13 arms when this
> section was written, item 3 had 15 entries, and item 4 had 15 entries when written; after WBS
> 6.1.2, item 1 has 15 arms / 16 spellings and item 4 has 14 entries). All subsequent references use
> the **symbol anchor + repository path**. The line numbers in this table are provided only for
> locating the code. The arm-count discrepancy in item 1 and the drift evidence in items 5 / 6 / 7
> (the dead `Uint` arm and the contradiction between `Set` and the typecheck side) **still exist**;
> this section did not change code.

Item 3, `CONST_PARAM_TYPES`, is used by `extract_generic_param_names`
(`src/frontend/core/parser/ast.rs`, measured at 932) to determine const generic parameters. It
enumerates “type names usable as const parameters,” which has a **partially overlapping but
non-identical** relationship to the full set handled by `from_builtin_name`, and no mechanism keeps
the two synchronized. Item 4 is the fourth copy used for LSP syntax highlighting and hover
information and likewise has no checking; T2’s comparison scope includes it.

Items 5–7 were first registered by `check-concepts.py` (P0, [09](09-execution-wbs.md) §0.2.1) on
2026-10-06: item 5 is a second type-name synonym table; items 6 / 7 share the same root cause as
items 1–4 (string-keyed tables mapping builtin names to semantics / IDs), although their purposes
differ and they drift for the same reasons. The same gate also found 11 string-based
builtin-container-name dispatches on the typecheck / eval / middle sides (`"List" | "Vec" | "Array"`
×4, `"String" | "Bytes"` ×4, etc.). **These are outside both this section and the parser data-flow
scope of 5.4**; the list and disposition are registered in
[06-cleanup-inventory.md](06-cleanup-inventory.md) §G3 as a P6 extension verification item.

### 2.4 The Parser’s Reverse Dependency on the Type Layer

RFC-039 routing table C records three entries (`declarations.rs:27-80` calls
`is_type_param_annotation` / `name_used_as_type`, functions belonging to the type layer).
**Measurement differs from that description**: both functions are defined inside the parser, not in
the type layer.

- `is_type_param_annotation` is defined in
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the repository’s only `fn`
  definition)
- `name_used_as_type` is defined in `declarations.rs:42-80` (the repository’s only `fn` definition)
- `declare_predicate` / `is_predicate_name` are defined in
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

There are only **2 production-code locations** that truly reference `typecheck`, and both call the
same function, `crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Enclosing function                    | Purpose                                                                                      |
| --- | --------------------------------------------------------- | ------------------------------------- | -------------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature-parameter filtering closure | Determines constraint-position parameters (`T: Add`), which do not occupy runtime parameters |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`         | Determines constraint-position parameters and produces `GenericParamName` with `constraints` |

The second location is deeper inside the parser—**`parser/ast.rs` itself** (the entire
`ast.rs:847-948` block shares a file with `StmtKind` / `Expr`) directly references a typecheck
symbol. Routing table C records only the `declarations.rs` location.

Together with the hard-coding in `declarations.rs:498-499`, it forms the same decision:

```rust
// declarations.rs:498-499
// Extended as more built-in predicates are added (future open-set approach: pass predicate names down to
// the parser, or change to a form determinable at the syntax layer).
// The parser does not hold a type environment, so it looks in two places: hard-coded built-in predicates,
// plus the declarations parsed so far in this pass (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment explicitly describes this as the temporary form of a “future open-set approach.”
`n == "Terminates"` **embeds type-layer knowledge in the syntax layer as a string literal**.

**Why this is downstream of the type-representation problem:** The parser needs to ask “is this an
operator-interface name?” because it must distinguish whether `Type::Name` refers to a type or an
interface constraint. **`Type::Name { name: String }` uses an undifferentiated string to carry at
least four meanings** (concrete type / type variable / constraint name / predicate name). The syntax
layer cannot determine this by itself, so the decision is forced upward into the type layer.

#### Duplicate Implementation: Same Meaning, Divergent Behavior

There are two implementations of `name_used_as_type`. The comment in `ast.rs:843-846` acknowledges
this:

```rust
/// Whether the parameter name is treated as a type reference in the given type (`(N: Int) -> (n: N)`'s `N`).
///
/// Semantically equivalent to `name_used_as_type` in `declarations.rs`; implemented independently here to avoid
/// a cross-module dependency within the parser.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations **have already diverged**:

| Dimension               | `declarations.rs:42-80` `name_used_as_type`                               | `ast.rs:847-885` `name_used_as_type_in`              |
| ----------------------- | ------------------------------------------------------------------------- | ---------------------------------------------------- |
| Signature               | Takes an `is_predicate: &dyn Fn(&str) -> bool` callback                   | Takes no callback                                    |
| `Type::Generic` branch  | Short-circuits on `is_predicate(app_name)` and returns `false` (`:54-56`) | No short-circuit; directly recurses into `args`      |
| `Type::Struct { body }` | **Absent** (falls into `_ => false`)                                      | **Present** (`:871-879`; traverses `Field` / `Expr`) |
| `Type::NamedStruct`     | **Absent**                                                                | **Present** (`:880-882`; traverses `fields`)         |

The comment in `ast.rs:868-870` records the origin of the latter two branches: “These two branches
were previously missing; when a const generic parameter was referenced inside a definition body,
‘used as a type’ could not be detected, and annotation validation incorrectly reported a valid const
parameter as an unknown name.”

In other words: **the function was duplicated to avoid a cross-module dependency, but its fixes were
never merged back into the original.** The `ast.rs` version recursively traverses fields inside
definition bodies, while the `declarations.rs` version does not—the same source receives different
results at two decision points. This duplicate must be consolidated when 5.3 introduces `NameKind`.

### 2.5 Parallel Operator Enums

There are two operator enums. They do not reference each other, and **their members have already
diverged**:

| Concept          | Syntax layer                    | Constant-evaluation layer                                             | Difference                                                                                                                                     |
| ---------------- | ------------------------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Binary operators | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | The syntax layer uniquely has `Neq` (`:197`), `Range` (`:205`), and `Assign` (`:206`); the constant layer uses `Ne` (`:243`) rather than `Neq` |
| Unary operators  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | The syntax layer uniquely has `Deref` (`:222`); the constant layer uniquely has `BitNot` (`:329`)                                              |

Each also implements a complete `Display` (`const_data.rs:291-317`) and classification predicates
(`const_data.rs:260-289`: `is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise`).

**Why this is part of the type-representation problem:** `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) **are not types**—when they occur in type position, they fall into
`Type::ConstExpr` (`ast.rs:506`), the “compile-time-only variant.” This is directly related to the
IR-layer type discipline in 6.1: after convergence, `Type::ConstExpr` disappears once
`Type → MonoType`, and IR never sees it, while `Assign` / `Deref` continue to exist through paths
such as `ir_gen.rs`’s `BinOp::Assign => Ok(MonoType::Void)` (`inference/expressions.rs:875`).
Because the two enums have diverged, neither side alone can determine which operators may appear in
type position.

This phase does not merge the two enums (their ownership is discussed under “Changes to Operators”
in [05-frontend-paradigm.md](05-frontend-paradigm.md)), but the T1 gate’s scan scope should include
`ast::BinOp` / `ast::UnOp`, making newly introduced zero-construction operators equally visible.

### 2.6 `Expr::FnDef`: An Expression Variant with Zero Production Construction Sites

It is true that `Expr::FnDef` (`ast.rs:37-43`) has **zero production construction sites**—the
repository contains only two construction sites: `src/frontend/core/parser/tests/ast.rs:837` and
`src/frontend/core/typecheck/tests/checker.rs:111`, both in tests.

However, it has **far more than 4 consumption sites**. There are 12 production consumption sites:

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

There are also 2 exhaustive-match arms that exist solely for it: `ast.rs:1026` (`Expr::span()`) and
`pratt/mod.rs:47` (`expr_end_line`).

**The path actually taken by function definitions** is: `nud.rs:425-429` constructs `Expr::Lambda`,
and `declarations.rs:462-479` constructs `StmtKind::Assign` (placing the `Lambda` in `value`). In
other words, the `FnDef` branch is **a parallel path that never executes but is maintained at 14
locations**.

### 2.7 Syntax Nodes Storing Semantics on Behalf of the Type Layer

**First, `StmtKind::Assign` carries a typechecker-specific field.** The `Assign` variant at
`ast.rs:241-249` contains `signature_params: Vec<Param>` (`ast.rs:244-245`), and its comment states:

> `/// The first parameter group of the signature is preserved as-is (including parameter names), for typechecker classify_generic_params to use`

The statement AST reserves a dedicated slot for the type checker. Its construction site is
`declarations.rs:472`.

**Second, expression position is used as a parameter list.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a **Lambda with an
empty body**. The arm at `src/frontend/core/parser/pratt/led.rs:466`,
`Expr::Lambda { params, .. } => Some(params.clone())`, then “retrieves” it and uses it as a
parameter list.

In other words, a normal AST variant is used as a **temporary parameter-list carrier**, and another
module’s match arm restores its meaning.

**Third, `Type::NamedParen` carries the return-position binder name.** The comment at
`ast.rs:519-540` states explicitly:

> The return-position refinement in RFC-027 §3 relies on it to declare the **return formal parameter
> name** (called a binder here)… **Without it, the type checker can only guess** that “a free
> variable outside the scope of a constraint is the return value parameter,” so `(r: P(m))` would
> silently substitute the undeclared `m` as a binder as well (measured defect).

A syntax node is storing binder identity on behalf of the type layer.

## Target Design

### 5.1 Shape of the Target Representation

**Proposition: a single representation means using `Type` (formerly `ast::Type`) as the sole type
structure; demoting `MonoType` to the type checker’s internal working representation; and deleting
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

**Why not “merge everything into `MonoType`”** (this is the proposal most likely to be raised, and
it is rejected here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.), whereas
   `MonoType` does not. The C3 criterion in 07 is “identical diagnostic codes and
   messages”—diagnostic positions must remain traceable to source. Adding `Span` to `MonoType` would
   attach a mass of meaningless sentinel spans to monomorphized types.
2. `MonoType` contains `TypeVar` and substitution states (`types/substitute.rs:112`, `pub fn unify`
   at `types/solver.rs:411`). These are **intermediate states in the type checker’s solving
   process**, not types themselves. Putting them in the sole type structure would require the
   IR-construction layer to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027) that are lowered away in `MonoType`. Unifying on `MonoType`
   would require adding these syntactic forms back to it.
4. **The decisive counterargument:** Promoting `MonoType` to the sole representation would make the
   L2 syntax layer depend on the L3 semantic layer’s type. RFC-039 explicitly prohibits L2 from
   depending on L3. Eliminating reverse dependencies is one of this document’s objectives, so
   choosing a solution that creates one would be self-contradictory.

**Why `ir::Type` must be deleted directly rather than “retained”**:

The alias at `ir.rs:3` causes `BytecodeFunction.params` (`bytecode.rs:812`) and `return_type`
(`bytecode.rs:814`) to use `ir::Type`, while the serialization side uses `MonoType`
(`codegen/bytecode.rs:277-278`). The two field types are connected by
`file.type_table.into_iter().map(|t| t.into())` at `bytecode.rs:2339` and the lossy `From` bridge at
`bytecode.rs:2353-2390`.

**The sole reason the `From` bridge exists is that “the two sides use different field types.”**
After changing `bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, and `image.rs:44` uniformly
to `MonoType`, `From<MonoType> for IrType` has no reason to exist and can be removed entirely,
eliminating the reverse reconstruction recorded in 2.2.

**Explicit non-goals:** `Type` is not moved from `parser/ast.rs` to `sema/types/` in this phase.
Only `lower.rs` is added under `types/` as the sole conversion point, and the definition of `Type`
remains where it is. The directory rename is performed at the end of P6 under D1 as an independent
relocation batch; at that point, `Type` will live in the top-level `ast/type_.rs` domain, while
`sema/types/` will be the home of the `MonoType` working representation.

### 5.2 Per-Variant Disposition Table for All 26 Variants

> **Implemented (WBS 6.3, 2026-10-29):** All 13 “remove” items in this table **have been
> implemented**—`ast::Type` was consolidated from 26 variants to 13 variants (13 removed, 1
> migrated, 12 retained); the two type bridges (reverse bridge B, `mono_to_ast_type`, and the two
> AST→AST substitutions) were rewritten in the same batch. See the measured results, 10-site
> disposition table, and criteria in [09a](./09a-wbs-history.md) §P6 6.3.

> Disposition categories: **remove** (zero parser-forward construction), **retain** (forward
> construction exists), and **migrate** (semantics move to another node).

| #   | Variant                                                            | Definition line  | Disposition                                | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| --- | ------------------------------------------------------------------ | ---------------- | ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Retain (strengthened)**                  | The parser’s **sole** primitive-type exit (`types.rs:162-165`). After the 12 primitive-type variants are removed, all primitive types use it. A “semantic kind” annotation for names must be added (see 5.3)                                                                                                                                                                                                                                     |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Remove**                                 | Zero parser-forward construction (`mono.rs:699` is a match arm). Bit width is carried by the `Name` path in `from_builtin_name`                                                                                                                                                                                                                                                                                                                  |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Remove**                                 | Zero parser-forward construction (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                  |
| 4   | `Char`                                                             | `ast.rs:434`     | **Remove**                                 | Zero parser-forward construction (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                  |
| 5   | `String`                                                           | `ast.rs:435`     | **Remove**                                 | Zero parser-forward construction (`mono.rs:702` is a match arm). **Note**: `IrType::String` at `bytecode.rs:2372` consumes it, so this arm must be rewritten after removal                                                                                                                                                                                                                                                                       |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Remove**                                 | Zero parser-forward construction (`mono.rs:703-706` are match arms). As above, `bytecode.rs:2373` consumes it                                                                                                                                                                                                                                                                                                                                    |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Remove**                                 | Zero parser-forward construction (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                  |
| 8   | `Void`                                                             | `ast.rs:438`     | **Remove**                                 | The parser’s only forward construction site = the error fallback `types.rs:836` (`_ => (Vec::new(), Type::Void)`, registered as an individual exemption). The fallback is eliminated together with the 6.4 error-path refactor                                                                                                                                                                                                                   |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Retain**                                 | Has production consumers (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                                                                                                                                                                                                |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Retain**                                 | Has production construction site `types.rs:364`                                                                                                                                                                                                                                                                                                                                                                                                  |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Remove**                                 | Zero parser-forward construction (`mono.rs:743-746` are match arms); 2 AST→AST reconstruction sites (`function.rs:377`/`606`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                           |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Remove**                                 | Zero parser-forward construction (`mono.rs:747` is a match arm); zero across all three categories—the only globally zero-construction variant. Enums use `Struct` + `TypeBodyItem`; unrelated to `MonoType::Enum(_) => IrType::Void` at `mono.rs:2378`                                                                                                                                                                                           |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Retain**                                 | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:451-454` | **Retain**                                 | Consumed by `bytecode.rs:2366`; **field reshaping `params: Vec<Type>` → `Vec<Param>` (name and type colocated) follows in 6.8.1 (D54)**—the names in `Assign.signature_params` are carried over this way (see 6.5.2)                                                                                                                                                                                                                             |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Remove**                                 | Zero parser-forward construction (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites (`function.rs:427`/`600`) are rewritten in the same batch as 6.3.2. `types.rs:392-396` already states that the `Generic` path is used                                                                                                                                                                                                            |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Remove**                                 | Zero parser-forward construction (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites (`function.rs:431`/`602`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                               |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Retain**                                 | The actual representation of `Result` / `Option` / `String` / `Bytes` (`types.rs:397-399`)                                                                                                                                                                                                                                                                                                                                                       |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Remove**                                 | Zero parser-forward construction (the syntax layer has no `::` path that produces it); **2 production AST→AST reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3 (registered under 6.3.2). **D8 remains unchanged**: the metric is forward-oriented; if associated-type syntax is enabled in the future, a new proposal will define it again |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Remove**                                 | Zero parser-forward construction (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites (`function.rs:467`/`632`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                               |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Retain**                                 | Has production construction site `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                                                                                                                                                                                          |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Retain**                                 | Raw pointer type in unsafe blocks                                                                                                                                                                                                                                                                                                                                                                                                                |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Retain**                                 | Borrow notation                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Retain**                                 | Has production construction sites `types.rs:100`, `declarations.rs:573`; RFC-010                                                                                                                                                                                                                                                                                                                                                                 |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Retain (and mark as compile-time-only)** | Has production construction site `types.rs:160`. It is the only `Type` variant that embeds an `Expr`; the IR layer must not carry it (see 6.1)                                                                                                                                                                                                                                                                                                   |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Retain**                                 | RFC-004 currying terminator; `split_curry` depends on its existence                                                                                                                                                                                                                                                                                                                                                                              |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                                | The semantics (binder name) move to the type checker; the AST retains only syntax. See 5.6                                                                                                                                                                                                                                                                                                                                                       |

**Net effect: 26 → 13 variants** (13 removed, 1 migrated, 12 retained).

> **Reverse bridges and AST→AST reconstruction must be handled before variants are removed**:
> `function.rs:507-529` (`mono_to_ast_type`) recorded in 2.2 reconstructs seven of items 2–8 in this
> table; additionally, the AST→AST reconstruction recorded in 2.2.2 covers 5 variants / 10 sites
> within the 13-variant set (2 each for Union/Option/Result/Sum/AssocType). Both categories of
> reconstruction sites must be handled in the same batch as 6.3.1; otherwise, the result is either a
> silent behavior change (see 6.3 #47) or a direct compilation failure.

> **How `AssocType` is handled**: **D8 remains unchanged**, and it is removed directly according to
> D8 (item 18 in the 5.2 table). The **metric is parser-forward**: zero forward construction holds
> because the syntax layer has no path that produces `::` associated types, so D8 does not need to
> be changed and the variant does not enter the allowlist. Its **2 production AST→AST reconstruction
> sites** (`function.rs:456`’s `substitute_type_ast` and `function.rs:623`’s
> `substitute_type_in_ast`) were registered in 6.3.2 and are rewritten in the same batch as 6.3.
> T1’s three-category split (5.7) reproduces this conclusion (all 26 variants agree in the 6.0.1
> measurement).

### 5.3 `NameKind` and Elimination of the Synonym Table

**Problem:** `Type::Name { name: String }` uses an undifferentiated string to carry at least four
meanings—concrete type / type variable / constraint name (operator interface) / predicate name.
`src/frontend/core/parser/statements/declarations.rs:500-501` must determine whether something is a
predicate, the same file’s `510-512` must determine whether it is an operator interface,
`src/frontend/core/parser/ast.rs:971` must determine whether it is an operator interface, and the
two `name_used_as_type*` implementations (`name_used_as_type_in` at `ast.rs:864` and
`name_used_as_type` at `declarations.rs:42`) must determine whether it is a type reference—all
through string comparisons.

**Proposal: add a `kind` field to `Type::Name`, determined once by the parser.**

```rust
// Target form (illustrative; line numbers are after the change)
Name {
    name: String,
    kind: NameKind,   // New field
    span: Span,
}

enum NameKind {
    Builtin,      // Builtin type name: Int / Float / Bool / ... → delegated to from_builtin_name
    UserType,     // User-declared type name (including Generic's std.result / std.option)
    TypeVar,      // Type variable: T / K / V
    Constraint,   // Operator interface / constraint name: Add / Ord / ...
    Predicate,    // Compile-time predicate: Terminates / Sorted / ...
}
```

**Key design point: determining `NameKind` requires exactly the information the parser cannot
currently obtain** (the type environment, the declared operator-interface table, and the declared
predicate table). Therefore, this field **cannot be determined independently inside the parser**—it
must be determined by an injected environment. This leads directly to 5.4.

**Disposition of the synonym table (WBS 6.1.2 implemented, 2026-10-27; user-approved final
wording)**:

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) is **retained with
downgraded semantics**. Before the change, it was a disambiguator that bridged two equivalent AST
representations. After `Type::Int(64)` is removed, “Int” has only one representation path, and it
degenerates into the **sole name-to-type resolution table**.

It must not be removed because:

- It is the **sole point of contact** for standard-library type names and language builtin names
  (`src/std/*.rs` signature strings, `CONST_PARAM_TYPES`, and the LSP list all use it as their
  reference).
- The `DateTime` arm records a real fix (#338 collateral issue: the return value of `now()` could
  not be passed to `format_time`)—it is a **semantic alias**, not a casing alias such as
  `Int`/`int`, so it is retained separately until P10 S11 (issue #513).

**Disposition result** (final metric = 15 builtin names + the temporary `DateTime` alias):

| Category                                                                                                                                                                                                                                       | Before                              | Disposition                                                                                                                                                                                                    |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 15 builtin names                                                                                                                                                                                                                               | Mixed into 13 arms with aliases     | **One name per arm**: `Never` / `Void` / `()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` / `Char` / `String` / `Bytes`                                          |
| Casing and abbreviation aliases (`int` / `int64` / `i64` / `int32` / `i32` / `int16` / `i16` / `int8` / `i8` / `float` / `float64` / `f64` / `float32` / `f32` / `bool` / `char` / `string` / `str` / `bytes` / `void` / `never` / `datetime`) | Retained                            | **Remove** (22 spellings); **do not add a lexer alias table**—corpus measurement found 0 noncanonical names (see the design judgment at the end of this section), so syntax-layer compatibility is unnecessary |
| `()`                                                                                                                                                                                                                                           | Shared arm with `Void`              | **Retain**—`()` is a spelling of the `Void` type (standard-library signature strings use `() -> Void`), not another type                                                                                       |
| `DateTime`                                                                                                                                                                                                                                     | Shared arm with the lowercase alias | **Retain only the uppercase** temporary compatibility alias, with a note that its removal belongs to P10 S11 (issue #513)                                                                                      |

**Net effect:** `from_builtin_name` = **15 match arms / 16 spellings / 1 alias** (`()` shares an arm
with `Void`; there is also a temporary `DateTime` alias). After the 22 spellings are removed, a
noncanonical spelling in type position becomes a `TypeRef`, and
`TypeEnvironment::resolves_type_name` reports an unknown type—it is **no longer silently accepted**.
Previously, `add_builtin_types` also registered lowercase copies of `int` / `float` / `bool` /
`string` / `void` / `never` / `char` in the type table, effectively creating a second alias table;
these were removed in the same batch.

This acceptance surface has **two permanent gates**:

1. Positive and negative unit cases in `src/frontend/core/types/tests/mono.rs` (the expected
   `MonoType` for each of the 15 names, `None` for the 22 spellings, and `DateTime` → `Int(64)`);
2. **T2.5 “removed-spelling sentinels”** in `tools/type-tables --report`—if any of the 22 spellings
   is added back to the resolution table, the report immediately counts it as **unregistered drift**
   (there is no disposition that can be registered: that would be a regression).

**Consolidating the two `name_used_as_type*` implementations:** The duplicate implementations
recorded in 2.4 must be merged, or the semantic information provided by `NameKind` will be bypassed
by two independent string-comparison implementations. The merged implementation belongs in
`parser/ast.rs` (next to the `Type` data source), and `declarations.rs` should call it; the
`is_predicate` short-circuit should read `NameKind::Predicate`, and `Struct` / `NamedStruct` field
recursion should follow the newer, more complete implementation at `ast.rs:894-905`.

**Ownership of `CONST_PARAM_TYPES`:** The const generic parameter-name table `CONST_PARAM_TYPES`
(`src/frontend/core/parser/ast.rs`, measured at 855-858) partially overlaps the full set in
`from_builtin_name` (see 2.3). After `NameKind::Builtin` is introduced, the decision in
`extract_generic_param_names` (same file, measured at 932) can be changed to “`kind == Builtin` and
the name belongs to the accepted const-parameter subset,” explicitly connecting the two pieces of
knowledge. The const-parameter subset is validated by the T2 gate (see 5.7).

> **Design judgment:** If any `.yx` corpus file uses `i64` / `int` in a type annotation, normalizing
> it to `Int` would change the file’s **diagnostic position** (but not its diagnostic code). This is
> permitted under C3 (messages and order may be normalized), but the baseline must be recorded in
> phase 1. **Measurement (P6 6.0, 2026-10-27):** A full scan of `tests/yaoxiang` (316 `.yx` files),
> `tests/yaoxiang-multifile`, and `src/std` found **0 noncanonical names in actual type positions**;
> the 16 raw-token matches were adjudicated individually as 12 strings and 4 comments (both `i64` /
> `f64` occurrences were in comments). Therefore, name normalization in 6.1 **does not change the
> diagnostic position of any corpus file**, and **there is no need to regenerate the baseline**.
>
> **Retest (P6 6.1 closeout, 2026-10-28):** 6.1.1 rescanned the final **22** spellings—the
> 2026-10-27 measurement used a **narrow definition** (only the 14 integer/floating-point-family
> spellings, excluding namespace usage in `src/std`), hence its record of “16 raw-token matches = 12
> strings + 4 comments.” The retest covered 316 `.yx` files in `tests/yaoxiang`, 39 files in
> `tests/yaoxiang-multifile`, and 38 files in `src/std`, for a total of **393 files**, with matches
> on **308 lines**. Only **2 lines were type-position candidates**, and both were inside `//`
> comments (`tests/yaoxiang/06-compile-errors/runtime/int_overflow.yx:5`’s `i64::MAX`, and
> `tests/yaoxiang/99-demos/compare_rust_side_by_side.yx:17`’s `-> f64`), so there were still **0
> actual type positions**. The remaining matches were individually adjudicated as string literals /
> comments / `std.string` module names and namespace access / `Json` variant names / `void`
> literals; **none was a type annotation**. See the 6.1.1 entry in [09](09-execution-wbs.md) and the
> “P6 6.1 Name Normalization Closeout” section in 09a for the retest command. The conclusion is
> unchanged from 6.0: name normalization in 6.1 **does not change the diagnostic position of any
> corpus file**, and **there is no need to regenerate the baseline**.

### 5.4 Data Flow for Predicate and Type Names

**Current data flow** (closed loop inside the parser + 2 boundary crossings):

```
ast.rs (extract_generic_param_names, :930)  ─┐
declarations.rs:509                          ─┴─→ typecheck::operator_interfaces::spec()   ✗
declarations.rs
  ├─ declare_predicate()      ← parser_state.rs:46（parser 自持，仅本遍已解析的声明）
  ├─ n == "Terminates"        ← 硬编码字面量（declarations.rs:499）
  └─ （上述越界调用）
```

**Target data flow** (injected, with no boundary crossing):

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

**Changes by file**:

| File                                                                                | Change                                                                                                                                                                                                                                                       |
| ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `src/frontend/core/parser/probe.rs` (new)                                           | Define `trait TypeEnvProbe`, providing four pure query methods: `is_predicate` / `is_constraint` / `is_type_var` / `is_builtin_type`. **The trait is defined in L2 and implemented by L3**—this is legitimate dependency inversion                           |
| `src/frontend/core/parser/parser_state.rs:46-58`                                    | Retain `declare_predicate` / `is_predicate_name` for accumulation within the current pass; add a `probe: Box<dyn TypeEnvProbe>` field and a `probe()` accessor                                                                                               |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                       | Remove `n == "Terminates"`; use `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                                              |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                       | Remove the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; use `state.probe().is_constraint(n)`                                                                                                                                       |
| `src/frontend/core/parser/ast.rs:930`                                               | Remove the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` defines `Type` / `Expr` / `StmtKind` and must not hold an environment dependency—this function must accept a `&dyn TypeEnvProbe` parameter instead (see below) |
| All 5 parser construction sites (`Parser::new` / `ParserState::new` in `parser.rs`) | Accept a `probe: Box<dyn TypeEnvProbe>` parameter. **The default implementation, `NullProbe` (all queries return false),** ensures parser unit tests remain unaffected                                                                                       |

**Signature issue for `extract_generic_param_names`:** `ast.rs:891` defines
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` as a free function
that accepts no context. After the boundary-crossing call at `ast.rs:930` is removed, it still needs
`is_constraint` decisions, so its signature must gain `probe: &dyn TypeEnvProbe`. All callers,
including `parser/tests/ast.rs`, must pass `&NullProbe` accordingly.

**When `NameKind` is determined:** When `types.rs:162-165` constructs `Type::Name`, `state.probe()`
determines `kind` once and fixes it permanently, so subsequent match arms no longer perform string
comparisons.

**This eliminates all 3 entries in RFC-039 routing table C**—2 of them only concerned
parser-internal functions even under the 2.4 metric, while the 2 genuine cross-layer references,
`declarations.rs:509` and `ast.rs:930`, are both covered by this section.

### 5.5 Merging `Expr::FnDef`

**Proposition: remove `Expr::FnDef` (`ast.rs:37-43`) and represent function definitions uniformly
through `Expr::Lambda` + `StmtKind::Assign`.**

**This is the path already in use:** `nud.rs:425-429` + `declarations.rs:462-479` (see 2.6). The
`FnDef` branch is a parallel implementation that never executes.

**Change list** (12 consumption sites + 2 exhaustive-match arms, listed individually):

| Location                             | Existing arm                                         | Target                                                                                                                                       |
| ------------------------------------ | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `spawn/placement.rs:122`             | `Expr::FnDef { body, .. } => self.check_block(body)` | Merge into the existing `Expr::Lambda` arm (`body` is also `Box<Block>`, so **remove this arm directly**)                                    |
| `spawn/analysis.rs:912`              | `Expr::FnDef { body, .. } => { ... }`                | As above; merge into the Lambda arm                                                                                                          |
| `formatter/handlers/expr.rs:43`      | `Expr::FnDef { ... }`                                | Format the Lambda in `Assign` instead; remove this arm                                                                                       |
| `orchestrator.rs:1492`               | `if let Expr::FnDef { name, .. }`                    | Obtain the name from `StmtKind::Assign { target, .. }`                                                                                       |
| `ir_gen.rs:4325`                     | `ast::Expr::FnDef { span, .. } => *span`             | Merge into the Lambda arm (`Lambda` also has `span`)                                                                                         |
| `ir_gen.rs:5097`                     | `\| ast::Expr::FnDef { span, .. }`                   | Remove this or-pattern                                                                                                                       |
| `checker.rs:1642`                    | `if let ...::Expr::FnDef {`                          | Match `Expr::Lambda` instead                                                                                                                 |
| `checker/semantic_tokens.rs:1416`    | `Expr::FnDef {`                                      | Remove this arm                                                                                                                              |
| `inference/expressions.rs:3374`      | `...::Expr::FnDef {`                                 | Match `Expr::Lambda` instead                                                                                                                 |
| `inference/existential.rs:55`        | `\| Expr::FnDef { span, .. }`                        | Remove this or-pattern                                                                                                                       |
| `passes/dead_code.rs:305`            | `Expr::FnDef {`                                      | Match `Expr::Lambda` instead                                                                                                                 |
| `layers/ownership.rs:1207`           | `\| Expr::FnDef { span, .. }`                        | Remove this or-pattern                                                                                                                       |
| `layers/termination.rs:752`          | `Expr::FnDef {`                                      | Match `Expr::Lambda` instead; also update the comment at `termination.rs:2303` (“`fn(): Never` — FnDef.return_type is the bare return type”) |
| `ast.rs:1026`                        | `\| Expr::FnDef { span, .. }`                        | Remove this or-pattern                                                                                                                       |
| `pratt/mod.rs:47`                    | `Expr::FnDef { span, .. } => *span`                  | Remove this arm                                                                                                                              |
| `typecheck/tests/checker.rs:109-111` | Test constructs `Expr::FnDef`                        | Rewrite as `Expr::Lambda` + `Assign`                                                                                                         |
| `parser/tests/ast.rs:837-847`        | Test constructs `Expr::FnDef`                        | Remove this test                                                                                                                             |

**No behavior change:** `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, and `span`,
making it structurally identical to the first three fields of `FnDef`; the `name` field unique to
`FnDef` has no usable location at the `Expr` layer (the name is on `StmtKind::Assign.target`).

**Risk and mitigation:** Non-exhaustive `if let` matches such as `checker.rs:1642` and
`inference/expressions.rs:3374` become impossible to match after the refactor, without producing an
error or warning. Mitigation: the type-table gate (5.7) adds a “zero-construction variant” check.
After `Expr::FnDef` is removed, any new `if let Expr::FnDef` fails to compile because the variant no
longer exists—**this is guaranteed at compile time and requires no gate**.

### 5.6 Disposition of `signature_params` and `NamedParen`

**`StmtKind::Assign.signature_params` (`ast.rs:244-245`) → remove.**

The reason for its existence (the comment at `ast.rs:244`) is “for typechecker
`classify_generic_params` to use”—**a dedicated slot reserved by the statement AST for the type
checker**. However, `extracted_params` at `declarations.rs:500-511` has already calculated
`value_params` in the same function after performing type-position and constraint-position
filtering. Carrying them through the statement is therefore redundant.

Change: remove the field at `ast.rs:244-245` → remove the argument at `declarations.rs:472` → change
the source of `classify_generic_params` to recompute the values in place (it can obtain
`Lambda.params` from `value: Option<Box<Expr>>`). **Net removal: one field declaration, one
construction site, and one transport path.**

> **Risk:** `signature_params` is the **unfiltered original** list, whereas `Lambda.params` is
> **filtered** (the filter at `declarations.rs:500-511` removes type-position and
> constraint-position parameters). If `classify_generic_params` depends on the unfiltered version,
> the refactor changes behavior. **The actual input used by `classify_generic_params` must be
> verified in phase 1**; equivalence must not be assumed.

**`Type::NamedParen` (`ast.rs:533-540`) → retain the node, but move the binder semantics out of
it.**

The comments at `ast.rs:519-532` record its **sufficient reason for existing** (see 2.7): if `r` is
lost from `(r: P(m))`, the type checker can only guess that “a free variable outside the scope of a
constraint is the return-value parameter,” and will **silently substitute the undeclared `m` as a
binder as well** (marked as a “measured defect” in the comment).

Therefore, it **cannot simply be removed**—that would reintroduce a correctness defect that has
already been fixed. The correct disposition is:

1. Retain `NamedParen { param, param_span, inner }` as a **pure syntax node**; `param` /
   `param_span` carry source facts only and do not carry type-layer binder semantics.
2. Move binder interpretation into `sema/`: the type checker **explicitly** extracts `param` as the
   binder when consuming `NamedParen`. This is “reading a syntax node’s field,” the same category as
   embedding an `Expr` in `ConstExpr`: the syntax node provides facts, and the semantic layer
   interprets them.
3. Rewrite the comment from “the type checker can only guess” to “this node provides the binder
   name, and semantic interpretation belongs in `sema/`.”

**Why this is a “migration” rather than a “removal”**: The syntax tree must represent source code
losslessly. `r` really exists in the source, so the AST cannot pretend otherwise. The problem is not
the node’s existence, but the failure to assign responsibility for semantic interpretation
explicitly.

### 5.7 Generation-Time Type-Table Gates

**The reference implementation has already been validated in this repository**: `build.rs:19-37`
calls `parse_registry` / `validate` from `tools/code-tables`, performing uniqueness, section, and
zh-completeness checks for 145 error codes; if `is_ok()` is false, it directly executes `panic!` and
rejects the build. `build.rs:39-54` further compares the markdown code table in RFC-013 item by item
and likewise executes `panic!` on a mismatch. **RFC-013’s documentation and code therefore always
remain consistent**—`tools/code-tables/Cargo.toml:15-16` has only one `serde_json` dependency, and
the validation logic is text-level parsing with no dependency on syn.

The type tables currently have **no similar gate**. This document adds a mechanism in the same form.

**New crate**: `tools/type-tables` (`Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). Four checks:

| #                                                              | Check                                                                                                                                                                                                                                                                               | Input                           | Failure condition                                                                                                                                                                                                                                                                                                                                          | Remedy                                                                                                              |
| -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| **T1 Construction-site split**                                 | Scan the variant list of `pub enum Type` in `src/frontend/core/parser/ast.rs`; under `src/`, count each variant’s construction sites in **three separate categories** (① parser-forward ② AST→AST reconstruction ③ reverse-bridge reconstruction; exclude `tests/` and `*/tests/*`) | Variant list + three hit tables | ① A variant has **zero forward construction** and is not registered; ② A registered variant gains a forward construction site; ③ A registered wildcard-arm fallback exemption does not match; ④ An **unregistered wildcard-arm fallback site** exists (fallback exemptions must be registered individually and cannot be generalized to all wildcard arms) | Remove the variant; or register a rationale / remove the stale registration (see 5.2 and `REGISTERED_ZERO_FORWARD`) |
| **T2 Synonym table and repository-wide name-list consistency** | The `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` at `ast.rs:855` + the LSP builtin type-name list at `src/lsp/world.rs:182`                                                                                                                             | Three tables                    | A type name exists in one table but is not recognized by another, compared according to each table’s semantic mapping rules; **since WBS 6.1.2, T2.5 “removed-spelling sentinels” also reports an error if any of the 22 removed spellings is added back**                                                                                                 | `type-tables --fix` synchronizes both directions                                                                    |
| **T3 Builtin type names match RFC-011**                        | The canonical name set in `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                                | Two tables                      | The code-table ranges / names and the registry differ                                                                                                                                                                                                                                                                                                      | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --fix`                                                   |
| **T4 Exhaustiveness of `Type → MonoType`**                     | The `Type` variant list vs the match arms in `types/lower.rs`                                                                                                                                                                                                                       | Two tables                      | A variant has no corresponding arm, or an arm has no corresponding variant                                                                                                                                                                                                                                                                                 | Add an arm / remove an arm                                                                                          |

**Implementation note for T4:** Rust’s exhaustive matching already guarantees the “variant → arm”
direction at compile time. T4’s value lies in the **reverse** direction—detecting stale arms in
`lower.rs` that refer to removed variants. Such code would fail to compile when the variant was
removed, so T4 is effectively redundant. **T4 is therefore downgraded to a CI assertion
(`cargo build` success is sufficient) and is not placed on `build.rs`’s `panic!` path.** T1–T3 are
the three checks that genuinely require text parsing.

**T1’s two implementation difficulties and mitigations**:

1. **Distinguishing construction sites from match arms** requires syntax analysis. The
   repository-root `Cargo.toml` has **no `syn` dependency**, and neither does `tools/code-tables`.
   Mitigations:
   - **Preferred:** Add `syn` to `tools/type-tables` only (the root crate remains unchanged). `syn`
     is the standard approach, and the gate crate is independent of the compiler.
   - **Fallback:** Reuse the line-level heuristics in the existing `tools/extract_arm.py` Python
     tool—construction sites occur on the right-hand side of `=` or in function-argument position,
     while match arms occur on the left-hand side of `=>`. Use the number of `=>` occurrences as an
     approximation.
   - **Conservative starting point:** On its first release, T1 should **only report and not fail**
     (`cargo:warning`), establishing a full baseline and confirming the heuristic before changing it
     to `panic!`. This follows the same approach used when CI gates initially fail and require a
     baseline.
2. **Reverse bridges and AST→AST reconstruction sites must be categorized separately.** Section 2.2
   records the two `MonoType → ast::Type` reconstruction locations, `bytecode.rs:2353-2390` and
   `function.rs:507-529`; section 2.2.2 records the AST→AST reconstruction in the two replacement
   functions in `function.rs` (15 variants / 28 sites, including 5 variants / 10 sites within the
   13-variant set). If T1 conflates them with forward construction, `Int` / `Float` / `Bool` /
   `Char` / `Void` / `String` / `Bytes` and `Union` / `Option` / `Result` / `Sum` / `AssocType` are
   classified as having construction sites and exempted from removal, directly contradicting the
   disposition table in 5.2. T1’s report must separate the three construction-site categories, and
   **wildcard-arm exemptions must apply only to individually registered entries**—currently only the
   parser error fallback for `Void` at `types.rs:836`—and must not be generalized to all wildcard
   arms.

**Gate coverage:** Add a `type_tables::validate(root, &entries)` call to `build.rs`, immediately
after the existing block at `build.rs:19-37`, following the
`panic!("类型表校验失败（{} 项 error ...）", ...)` pattern.

## Detailed Design

### 6.1 Impact on the Type System

| Affected area                                                | Assessment                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact.** Inference consumes `MonoType` throughout and does not touch `ast::Type` variants                                                                                                                                                                                                                                                                                                                               |
| Solver (`types/solver.rs`)                                   | **No impact.** `pub fn unify` (`solver.rs:411`), `substitute` at `types/substitute.rs:112`, and `substitute` at `types/mono.rs:649` all operate on `MonoType`                                                                                                                                                                                                                                                                 |
| Constant evaluation (`types/eval/`)                          | **No impact.** `eval/const_eval.rs` (1,128 lines), `check_structural_termination` at `eval/dependent_types.rs:478`, and `unify` at `eval/reducer.rs:572` all operate on `MonoType`                                                                                                                                                                                                                                            |
| Monomorphization (`middle/passes/mono/`)                     | **Changed (WBS 6.3, 2026-10-29).** Eight locations in `mono/function.rs`—`358`, `362`, `451`, `456`, `554`, `558`, `618`, and `623`—match `AstType::NamedStruct` / `AstType::AssocType`. The disposition of `AssocType` is removal, and all 8 locations were updated accordingly. **`mono_to_ast_type` at `function.rs:507-529` was also changed** (see #47 in 6.3; it now uses `canonical_name` + structural reconstruction) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Needs changes.** The `ast::Type::NamedStruct` matches at `ir_gen.rs:1266` and `1399`; `let params: Vec<MonoType> = signature_params` at `ir_gen.rs:1380` is another consumption point for `signature_params` and must be verified together with 5.6                                                                                                                                                                         |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Needs changes.** `type_id_to_monotype` at `codegen/bytecode.rs:492-495`, the encoding loop at `351-352`, and assembly at `632`                                                                                                                                                                                                                                                                                              |
| Interpreter (`backends/interpreter/`)                        | **Needs changes.** `type_table: Vec<ir::Type>` at `image.rs:44` becomes `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, and the changed element type affects the `{:?}` output format                                                                                                                                                                                                       |
| RFC-027 compile-time types                                   | **Needs changes.** `Type::ConstExpr` is retained but marked **compile-time-only**; it disappears after `Type → MonoType` (`types/lower.rs`) and is never visible to IR                                                                                                                                                                                                                                                        |

**IR-layer type discipline (to be inherited by 04-ssa):** After convergence, IR type annotations may
use only the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only variants** and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. This rule should be
enforced by the first-layer validator’s “type consistency” invariant in 07. `ast::BinOp::Assign` /
`ast::UnOp::Deref` (2.5) are the expression-side counterparts—they may occur inside `ConstExpr`, so
they likewise must not propagate into IR type annotations.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** The basis is:

1. The 13 removed variants have **no construction sites in the parser-forward direction** (the one
   forward site for `Void` was an error fallback—**WBS 6.3 changed it to `Type::Name{"()"}` and
   removed the gate-exemption registration**, so there are now no forward construction sites) → the
   parser does not emit them. The reverse bridge and AST→AST reconstruction dispositions are covered
   by #28 / #47 in 6.3 and the 10 sites in 6.3.2, all of which have been implemented.
2. `ir::Type` is an alias for `ast::Type` → changing it back to `MonoType` does not change the set
   of types, only the **carrier**.
3. The **loss** in `From<MonoType> for IrType` (8 variants → `Void`) no longer occurs after the
   change—this **may** change the behavior of some programs because types that were previously lost
   are now retained. **This is the only direction in which behavior can change, and it is a fix
   rather than a regression**, but 07’s third-layer corpus differential must confirm that there are
   no regressions.

**Semantic change points requiring particular attention**:

| Location                | Before                                                                                        | After convergence                              |
| ----------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| `bytecode.rs:2371-2376` | `Generic` other than `String`/`Bytes`/`Tuple` → `IrType::Void`                                | Retain the real `MonoType::Generic`            |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Retain the real `MonoType`                     |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` fallback                                                                  | **Remove** (an exhaustive match is sufficient) |

> **Design judgment:** These 8 variants currently all collapse to `Void`, meaning that **every code
> path that depends on signature bytecode—the interpreter’s arity/type checks and the serialization
> format—currently sees `Void`**. After convergence, these locations will see the real types. **This
> is the only possible source of behavioral change in this document, and its direction is “restore
> correct information.”** If the corpus differential produces a difference, it should be treated as
> **exposing an existing defect**, not as a regression introduced by this document. To determine
> this, construct a `.yx` program containing a `Ref` parameter and compare `dump_bytecode` output
> before and after the change.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                      | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Category                          |
| --- | ---------------------------------------------------------------------- | ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472`    | Remove the `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants (**completed: WBS 6.3.1, 2026-10-29— all 13 variants removed; `ast::Type` consolidated to 13 variants**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Pure removal                      |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                                 | Add a `kind: NameKind` field to `Name`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Data flow                         |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43`                                   | Remove `Expr::FnDef`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Pure removal                      |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                                 | Remove `Assign.signature_params`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Pure removal                      |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026`                                    | Remove the `Expr::FnDef` or-pattern                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Follows #3                        |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                                 | Rewrite the `NamedParen` comment as “the syntax provides the binder name; semantic interpretation belongs in `sema/`”                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Documentation                     |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                           | Add `pub enum NameKind`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | New                               |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                       | Add `trait TypeEnvProbe` + `NullProbe`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | New                               |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                   | Add the `probe` field and accessor                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Data flow                         |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                                 | Determine `NameKind` through `state.probe()`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Data flow                         |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                                 | Update the comment (`Result`/`Option` use `Generic`, already consistent with the facts; only add a note about the removed variants)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Documentation                     |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                     | Remove the `_ => (Vec::new(), Type::Void)` fallback and replace it with an explicit error return—**WBS 6.3.1 changed the fallback to a let-else that explicitly returns `Type::Name{"()"}`** (`Void` has been removed; semantics and formatter output remain byte-for-byte identical); **the “replace it with an explicit error return” half remains part of the 6.4 error-path refactor** (this path is syntactically reachable; the `Name` arm in `semantic_tokens` records one additional dummy-span token; there are 0 corpus matches and therefore 0 drift)                                                                                                                                                                 | Behavior                          |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                                 | Remove `n == "Terminates"` and use `state.probe().is_predicate(n)`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Reverse dependency                |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                                 | Remove the `typecheck::operator_interfaces::spec()` call                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Reverse dependency                |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                     | Remove the `signature_params` argument                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Follows #4                        |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                      | Remove the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Follows #3                        |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                     | Retain (this is the parameter-list retrieval point; according to 5.5, change it to read the Lambda directly)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Follows #3                        |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814`    | Remove 13 match arms (one for each removed variant) (**completed: WBS 6.3.1, 2026-10-29**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Follows #1                        |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                                 | Remove synonyms (casing aliases, abbreviations such as `i64`, and `"()"`)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Consolidation                     |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                     | Retain the `DateTime` alias                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | Retain                            |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                       | Move `From<Type> for MonoType` (currently at `mono.rs:692-830`) to the sole conversion point                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | New                               |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                             | Register `i64`/`int` and similar spellings as equivalent forms of `Int`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Consolidation                     |
| 23  | `src/middle/core/ir.rs`                                                | Original `3` (now `7` as a private `use`) | Remove `pub use ... ast::Type` (**completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Pure removal                      |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                     | Keep `FunctionCode.params` as `Vec<MonoType>` (already is)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | —                                 |
| 25  | `src/middle/core/bytecode.rs`                                          | `814`, `816`                              | Change `BytecodeFunction.params` / `return_type` to `MonoType` (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Consolidation                     |
| 26  | `src/middle/core/bytecode.rs`                                          | `856`                                     | Change `BytecodeModule.type_table` to `Vec<MonoType>` (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Consolidation                     |
| 27  | `src/middle/core/bytecode.rs`                                          | `2346`                                    | Change the `type_table` conversion to `.collect()` (the elements already have the same type)—the actual implementation is to **remove the conversion and carry the values through unchanged** (**completed: WBS 6.2.1, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Follows #26                       |
| 28  | `src/middle/core/bytecode.rs`                                          | Original `2352-2390` (**removed**)        | **Remove** the entire `From<MonoType> for IrType` impl (**completed: WBS 6.2.2, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Pure removal                      |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                      | Change `type_table` to `Vec<MonoType>` (**completed: WBS 6.2.3, 2026-10-28**)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Consolidation                     |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 locations)                 | Handle the `AstType::AssocType` arms according to the gate conclusion—**the conclusion has been determined and implemented: WBS 6.3.2 removed all 5 variants / 10 sites registered in 03 §2.2.2 (`Union`/`Option`/`Result`/`Sum`/`AssocType`, 2 each)**, and both bridges (`substitute_type_ast` / `substitute_type_in_ast`) were rewritten in the same batch                                                                                                                                                                                                                                                                                                                                                                    | Completed (WBS 6.3.2, 2026-10-29) |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`              | Type-table gates T1–T3                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | Gate                              |
| 32  | `spawn/placement.rs`                                                   | `122`                                     | Remove the `Expr::FnDef` arm                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Follows #3                        |
| 33  | `spawn/analysis.rs`                                                    | `912`                                     | As above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Follows #3                        |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                      | As above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Follows #3                        |
| 35  | `orchestrator.rs`                                                      | `1492`                                    | As above; obtain the name from `Assign.target`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Follows #3                        |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                            | As above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Follows #3                        |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                    | As above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Follows #3                        |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                    | As above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Follows #3                        |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                    | As above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Follows #3                        |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                      | As above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Follows #3                        |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                     | As above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Follows #3                        |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                    | As above                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Follows #3                        |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                             | As above + update the comment                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Follows #3                        |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                                 | Remove the `Expr::FnDef` test                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Test                              |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                                 | Rewrite as Lambda + Assign                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Test                              |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`           | Remove the lower assertions for 11 dead variants                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Test                              |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                                 | Remove the `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches from `mono_to_ast_type` and use `AstType::Name` + `mono.type_name()` uniformly. **This must be handled in the same batch as #1**—**the metric has been corrected (completed: WBS 6.3.2, 2026-10-29)**: `mono.type_name()` is the lowercase internal name used for diagnostic display (`int64` / `void` / `string`). After 6.1.2 removes the lowercase spellings, it would become a `TypeRef` and **silently lose type identity**. The actual implementation therefore uses the newly added `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) + structural reconstruction (`Generic` / `Ref` / `Fn`) | Consolidation                     |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                              | Add `probe: &dyn TypeEnvProbe` to the `extract_generic_param_names` signature; remove the `typecheck::operator_interfaces::spec()` call and use `probe.is_constraint(name)`. Callers, including `parser/tests/ast.rs`, pass `&NullProbe`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Reverse dependency                |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                   | Remove the local `name_used_as_type` and call the consolidated implementation in `ast.rs`; change the `is_predicate` short-circuit to read `NameKind::Predicate`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Duplicate implementation          |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                     | Connect the `CONST_PARAM_TYPES` decision through `NameKind::Builtin` (see 5.3)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Consolidation                     |

> **The six completed 6.2 items (#23 / #25 / #26 / #27 / #28 / #29):** WBS 6.2, completed on
> 2026-10-28, handled all six in one batch—the four carriers were changed to `MonoType`
> (`bytecode.rs:814`, `:816`, `:856`; `image.rs:44`), the lossy bridge `From<MonoType> for IrType`
> and the `ir::Type` re-export alias were removed, and decoding was changed to carry values through
> unchanged (`bytecode.rs:2315`, `:2316`, `:2346`). **The line numbers for these items have been
> remeasured against the implemented state** (the original anchors `812` / `814` / `854` / `2339` /
> `2352-2390` were pre-change positions). See [09a](./09a-wbs-history.md) §P6 6.2 for measured
> results and gate outcomes.

> **The gates must themselves be gated**: `tools/type-tables` and `code-tables` likewise require
> explicit `mod` declarations (the repository has already learned this from eight orphaned test
> trees). Register the new crate in the root `Cargo.toml` when it is created.

### 6.4 Backward-Compatibility Strategy

**Constraint:** The 293 single-file `.yx` corpus files in `tests/yaoxiang/`, the multi-file corpus
layer established in P2 (`tests/yaoxiang-multifile/`), `src/std/tests/*.yx`, and the standard
library **must all retain identical behavior** (identical diagnostic codes and messages, with order
normalized where necessary).

| Risk                                                            | Assessment                                                                                                                                                                                                                                                                                                                   | Mitigation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Source code uses type annotations such as `i64` / `int` / `f64` | After normalization to `Int`, `from_builtin_name` recognizes only canonical names                                                                                                                                                                                                                                            | **Measured** (end of 5.3): 0 noncanonical names in actual type positions → 6.1.2 directly removes the 22 spellings and **does not add a lexer alias table**; the corpus diagnostic set remains unchanged                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Source code uses `DateTime`                                     | Retain only the uppercase temporary compatibility alias (`mono.rs:883`); it is not a builtin name                                                                                                                                                                                                                            | Removing it from the builtin-name table belongs to P10 S11 (issue #513); until then, `DateTime` remains synonymous with `Int`, with no behavior change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `("()")` appears in type position                               | It is not produced in `Name` form; it is the type spelling for `Void` (standard-library signature strings use `() -> Void`)                                                                                                                                                                                                  | **Retain:** `()` shares an arm with `Void` and is one of the final 15 names                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `dump_bytecode` output changes                                  | The `type_table` element type changes from `ir::Type` to `MonoType`, producing a different `{:?}` format                                                                                                                                                                                                                     | **Permitted by C3** (07 specifies that `dump_bytecode` is compared only in phases C1/C2). Record the output diff in phase 2 for manual review                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Behavior changes after the lossy bridge is fixed                | See 6.2                                                                                                                                                                                                                                                                                                                      | Corpus differential + targeted `dump_bytecode` comparison                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| The result of `mono_to_ast_type` changes after rewriting        | `function.rs:507-529` is the exit point for monomorphized type substitution (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back to `AstType`). After the variants are removed, using the `_` arm would change the result from `AstType::Int(64)` to something such as `AstType::Name { name: "i64" }` | Targeted comparison in phase 3: select a corpus containing generic substitution and compare the monomorphization artifacts’ `type_name()` before and after the change. **Do not assume that `{:?}` output is equivalent.** **The metric has been corrected (phase 3 measurement):** Equality must not be determined using `type_name()`—it is the lowercase internal name used for diagnostic display, and comparing `type_name()` can only show that both values are `int64`, not whether the canonical name is `Int64` or `i64`. Instead, compare **canonical-name round trips** (`from_builtin_name` ↔ `MonoType::canonical_name()` for every name) + structural comparison of the reconstructed artifacts. WBS 6.3 measurement: the dumps and md5 hashes of the `.42` artifacts for 4 generic-substitution programs are byte-for-byte identical to the pre-change versions |
| `.yx` source code itself                                        | **No modifications**                                                                                                                                                                                                                                                                                                         | This document requires no changes to any corpus or std source file. This is a hard acceptance requirement                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |

**Hard acceptance condition** (**metric revised on 2026-10-27, following decision ② from
2026-10-26**): `git diff --stat tests/yaoxiang tests/baselines src/std/` must be empty. **Directly
driven test files** under `tests/` may be modified when mechanically required by the change; each
such modification must be registered with **file / line count / rationale**, and **tests must not be
deleted or assertions weakened**. The original metric (`git diff --stat tests/ src/std/`) imposed a
zero-difference requirement on all of `tests/`; this revision **relaxes the requirement only by one
level for directly driven test files**. The zero-difference requirement remains in force for the
corpus layer (`tests/yaoxiang`), baseline layer (`tests/baselines`), and standard library
(`src/std/`). **Exception:** the 6.8.2 behavior-fix batch is handled in a separate round with its
own decision.

## Implementation Essentials

The criteria uniformly use 07’s **C3: identical diagnostic codes and messages (order may be
normalized)**. The following is the phase breakdown for this workstream; see RFC-039 for the global
ordering across documents.

| Phase                                         | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Rollback point                                    |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| **0. Gates first**                            | Create `tools/type-tables` and implement T1/T2/T3. The first version of T1 should **only report and not fail**. Establish a full baseline against the **unmodified** code                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | The T1 report accurately lists the 13 **parser-forward zero-construction** variants, including the characterization of `AssocType` (forward 0 / AST→AST 2, D8 unchanged), and **separately lists the reverse-bridge reconstruction sites from 2.2 and the AST→AST reconstruction sites from 2.2.2**. The 13 parser-forward zero-construction variants and the 1 global zero-construction variant (`Enum` only) are reported separately. T2/T3 are green or provide an actionable repair list | No code changes; no rollback required             |
| **1. Name normalization**                     | **Completed (WBS 6.1.2, 2026-10-27):** `from_builtin_name` (`mono.rs:839-886`) consolidated to 15 names + the temporary `DateTime` alias; 22 in-table spellings removed; the 7 lowercase copies in `add_builtin_types`, the `Uint` / `Int128` entries in the LSP list, and the `Uint` arm in `sizeof` cleaned up in the same batch; **no lexer alias table added** (0 noncanonical names measured in the corpus). **At the same time, verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`**                                                                                                                                                                                                                                                                                                                                                                                                           | C3: the full-corpus diagnostic set is item-for-item identical to the phase 0 baseline                                                                                                                                                                                                                                                                                                                                                                                                        | Separate commit; can be reverted                  |
| **2. Bytecode type unification**              | **Completed (WBS 6.2, 2026-10-28):** Four carriers changed to `MonoType`—`BytecodeFunction.params` (`bytecode.rs:814`) and `return_type` (`:816`), `BytecodeModule.type_table` (`:856`), and `Image.type_table` (`image.rs:44`); the lossy bridge `From<MonoType> for IrType` (originally `bytecode.rs:2352-2390`) and the `ir::Type` re-export alias (`pub use` at `ir.rs:3`) removed; decoding changed to carry values through unchanged (`bytecode.rs:2315`, `:2316`, `:2346`). The previous silent information loss where 8 `MonoType` variants collapsed to `Void` no longer occurs                                                                                                                                                                                                                                                                                                                                              | C3 + targeted `dump_bytecode` comparison (each item in the 6.2 table)—**measured fully green** (recorded in [09a](./09a-wbs-history.md) §P6 6.2)                                                                                                                                                                                                                                                                                                                                             | Separate commit; can be reverted                  |
| **3. Dead-variant removal**                   | **Completed (WBS 6.3, 2026-10-29):** Remove the 13 variants according to the 5.2 table (`ast::Type` 26 → 13); remove match arms such as `mono.rs:699-708` and the fallback at `types.rs:836` (changed to a let-else that explicitly returns `Type::Name{"()"}`); **handle `function.rs:507-529` (#47) and the 10 AST→AST sites (6.3.2) in the same batch**; update `types/tests/mono.rs`. **The #47 metric has been corrected:** the original wording, “use `AstType::Name` + `mono.type_name()` uniformly,” became invalid after 6.1. `type_name()` is the lowercase internal name used for diagnostic display (`int64` / `void` / `string`); after 6.1.2 removed the lowercase spellings, it would become a `TypeRef` and **silently lose type identity**. The actual implementation uses the newly added `MonoType::canonical_name()` (the strict inverse of `from_builtin_name`, with explicit width) + structural reconstruction | C3 + `cargo build` fully green (exhaustive matching forces all arms to be completed) + targeted generic-substitution comparison (last row of 6.4)—**measured fully green** (recorded in [09a](./09a-wbs-history.md) §P6 6.3)                                                                                                                                                                                                                                                                 | Separate commit; can be reverted                  |
| **4. Parser data flow**                       | Add `probe.rs`; change `parser_state.rs:46-58`; change `declarations.rs:498-511` and `ast.rs:930` (#48); determine `NameKind` at `types.rs:162-165`; consolidate the two `name_used_as_type*` implementations (#49)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | C3 + **new assertion**: `grep 'typecheck' src/frontend/core/parser/` must return 0                                                                                                                                                                                                                                                                                                                                                                                                           | Separate commit; can be reverted                  |
| **5. Dead AST variants and dedicated fields** | Remove `Expr::FnDef` (16 locations) + `Assign.signature_params`; migrate `NamedParen` semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | C3 + all 18 modules in `tests/integration/` green                                                                                                                                                                                                                                                                                                                                                                                                                                            | Split into two commits (FnDef / signature_params) |
| **6. Turn gates into hard failures**          | Change T1 from warning to `panic!`; downgrade T4 to a CI assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | `cargo build` **must fail** when a zero-construction variant is deliberately introduced (red test first)                                                                                                                                                                                                                                                                                                                                                                                     | Separate commit                                   |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Phases 3 and 5 must follow phase 2**—before phase
2 lands, reverse bridge A on the bytecode side (`From<MonoType> for IrType` in `bytecode.rs`)
consumes `IrType::String` / `IrType::Bytes`, so removing the variants first would break compilation.
That bridge was removed in 6.2.2. **The remaining rewrite surface when phase 3 removes the variants
is reverse bridge B** (`mono_to_ast_type` in `middle/passes/mono/function.rs`, which still resolves
`String` / `Bytes` by name) and the three `ast::Type` carriers in `middle/core/ir.rs`. Phase 3’s #47
must be handled in the same batch as #1; otherwise, `mono_to_ast_type` silently changes behavior.
**Phase 3 was completed on 2026-10-29 (WBS 6.3):** Reverse bridge B was changed to use
`canonical_name` + structural reconstruction. The **three `ast::Type` carriers in
`middle/core/ir.rs` were not changed**—6.3 only removes variants and does not alter the IR shape
(216 IR snapshots and 356 corpus baseline lines had 0 drift); carrier disposition remains in
phase 4.

**Dependency on downstream work**: [04-ssa.md](04-ssa.md) may begin only after all phases in this
document are complete. At that point, `BytecodeFunction.params`, `FunctionCode.params`, and
`type_table` all use the same element type, giving SSA type annotations a single landing point.

### Mandatory Verification for Each Phase

| Phase | Required                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`; manually verify the T1 report (**including the reverse-bridge reconstruction split**)                                                                                                                                                                                                                                                                                                                              |
| 1–5   | `cargo test`; full corpus differential over `tests/yaoxiang/` (07’s third layer); item-by-item C3 comparison                                                                                                                                                                                                                                                                                                                                                                             |
| 2     | Additionally: construct one `.yx` sample for every variant in the 6.2 table and compare `dump_bytecode` before and after the change                                                                                                                                                                                                                                                                                                                                                      |
| 3     | Additionally: for a corpus containing generic substitution, compare the monomorphization artifacts’ `type_name()` before and after the change—**the metric has been corrected (phase 3 measurement)**: `type_name()` is the lowercase internal name used for diagnostic display and is insufficient for equality. Instead, use **canonical-name round trips** (`from_builtin_name` ↔ `MonoType::canonical_name()` for every name) + structural comparison of the reconstructed artifacts |
| 4     | Additionally: `rg 'typecheck' src/frontend/core/parser/` must return 0                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 6     | Additionally: deliberately write a zero-construction variant and confirm that `cargo build` fails (red → green)                                                                                                                                                                                                                                                                                                                                                                          |

## Key Decisions and Rationale

### Decisions

| Decision                     | Resolution                                                                                                                          | Rationale                                                                                                                                                                                                                              |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Target representation shape  | Use `Type` as the sole type structure and `MonoType` as the working representation; delete the `ir::Type` alias                     | The alias at `ir.rs:3` directly causes the field types of `BytecodeFunction.params` and `FunctionCode.params` to differ (2.1). Once the alias is removed, both field types are unified and the `From` bridge loses its reason to exist |
| Reject “unify on `MonoType`” | Not adopted                                                                                                                         | It would create an L2→L3 reverse dependency, violating RFC-039’s layering, and would require adding `Span` / `ConstExpr` back to `MonoType`. See the full argument in 5.1                                                              |
| Do not remove `NamedParen`   | Retain it as a pure syntax node; move binder semantics to `sema/`                                                                   | `ast.rs:519-532` documents that removing it would reintroduce an already fixed correctness defect: `(r: P(m))` would silently treat the undeclared `m` as a binder                                                                     |
| Directory renaming           | **Do it** (D1), as a pure relocation batch at the end of P5/P6; the paths in this document’s main text reflect the pre-rename state | Decision of 2026-10-03 (ChenXu233); implementation method clarified on 2026-10-04                                                                                                                                                      |

### Rejected Alternatives

None of the four alternatives was adopted. **Retaining all three representations and adding only a
conversion layer** does not address any root cause: `ir::Type` remains non-independent, and the dead
variants, synonym table, and reverse dependency all remain. **Generating the synonym table with
macros** treats a symptom rather than the cause—after `Type::Int(usize)` is removed, the macro has
no reason to exist, and consistency should instead be guaranteed by gate T2, whose correctness is
more verifiable than macro expansion. **Removing only dead variants without unification** is the
least expensive option, but it leaves the problems in place and provides no gate to prevent the next
person from adding another `Type::Int128(usize)` and allowing the same dead variant to grow again;
if only one commit can be made, it should explicitly be recorded as a subset of this option and
deliver the T1 gate as well. **Unifying on `MonoType`** is discussed in 5.1.

### Benefits

- **Eliminates maintenance burden at 14 locations.** The 12 consumption sites and 2 exhaustive-match
  arms for `Expr::FnDef` form a parallel path that never executes; removing it eliminates one branch
  each from `ir_gen.rs`, `ownership.rs`, and `termination.rs`.
- **Eliminates one diverged duplicate implementation.** The two copies of `name_used_as_type` (2.4)
  already differ in field recursion inside definition bodies; after consolidation, only one remains.
- **After the lossy bridge disappears, `IrType::Void` is no longer a “legal type value.”** This is
  the prerequisite for 07’s first-layer validator to enforce the “type consistency” invariant
  successfully.
- **The gates make regression impossible.** T1 turns the silent acceptance of a newly introduced
  zero-construction variant into a build failure—the repository’s existing `build.rs` mechanism
  demonstrates that this path is viable.
- **No source-code changes.** The 293 `.yx` corpus files and the standard library require no
  modifications, which is the fundamental reason the C3 criterion can hold.

## Known Limitations and Risks

### Risks

- **Phase 2 may change behavior.** Fixing the lossy bridge means that 8 `MonoType` variants now
  expose their real types instead of `Void`. The direction is a fix, but it changes paths that
  depend on signature bytecode. This is the largest uncertainty in this document.
- **The characterization of `AssocType` depends on gate accuracy.** If T1 uses line-level heuristics
  (such as counting `=>`) instead of `syn`, it may misclassify sites. This is why phase 0 requires
  manual verification.
- **If T1 fails to count reverse bridges, the disposition table and gate contradict each other.**
  Section 2.2 proves that 7 “dead variants” have production reconstruction sites in reverse bridges.
  If T1 does not separate construction-site directions, those 7 variants are classified as having
  construction sites and exempted from removal, making the gate an obstacle to the removal work
  rather than a safeguard.
- **2. Removing `signature_params` carries semantic risk.** It carries the **unfiltered** parameter
  list, whereas `Lambda.params` is **filtered**. If `classify_generic_params` depends on the former,
  the refactor changes generic-classification results. **This must be verified first and cannot be
  assumed to be equivalent** (the `signature_params` consumption at `ir_gen.rs:1380` must also be
  verified).
- **3. Phase 3 touches `middle/passes/mono/function.rs`.** Besides the 8 `AssocType` matches,
  `mono_to_ast_type` at `507-529` is the exit point for monomorphized type substitution, and
  rewriting it may change the substitution result’s `type_name()` (last row of 6.4).
- **Noncanonical type names in the `.yx` corpus may not be zero.** If the corpus extensively uses
  `i64` / `int`, directly removing synonyms in phase 1 is not viable, and the alternative is to move
  them into a lexer alias table, reducing the scope of `from_builtin_name` consolidation.
  (**Measured as 0:** the 6.0 survey is recorded at the end of 5.3, so WBS 6.1.2 directly removed
  the aliases instead of taking the alias-table route.)
- **The gate may initially be red.** If T1 directly uses `panic!` on its first release, it
  immediately blocks the build. It must first run in report-only mode to establish the baseline.

### Open Questions

> **All open questions originally listed in this section have been decided.** See the
> [RFC-039 Decision Registry](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> individual decisions. **No unresolved items remain in this document.**

## See Also

### Parent and Peer Documents

- [RFC-039 Compiler Feature Routing Directory Design (Master Plan)](../../rfc/accepted/039-compiler-architecture.md)
  — Parent master plan; four-layer model, criterion tiers, and routing table C (the 3
  type-representation-related entries belong to this document)
- [01-routing.md](01-routing.md) — Feature routing tables A/B/C, dependency-direction rules, and
  target directory structure
- [04-ssa.md](04-ssa.md) — SSA conversion; this document is its prerequisite and it is a downstream
  consumer
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — Frontend paradigm; consolidation of the
  operator change surface
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — Definition of C3, diagnostic-set comparison
  specification, and the first-layer IR validator’s type-consistency invariant

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — Basis of the
  unified `Type::Generic` path
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — Builtin type table
  (comparison target for gate T3)
- [RFC-013 Error Code Specification](../../rfc/accepted/013-error-code-specification.md) — Example
  of a generation-time `build.rs` gate
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — Origin of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

- `src/frontend/core/parser/ast.rs:427-541` — Definition of the 26 `ast::Type` variants
- `src/frontend/core/parser/ast.rs:519-540` — Explanation of `Type::NamedParen` binder semantics
  (evidence that removing it would reintroduce the defect)
- `src/frontend/core/parser/ast.rs:241-249` — The typechecker-specific
  `StmtKind::Assign.signature_params` field
- `src/frontend/core/parser/ast.rs:838-841` — `CONST_PARAM_TYPES`: the third copy of const generic
  parameter names
- `src/frontend/core/parser/ast.rs:843-885` — `name_used_as_type_in`: duplicate implementation of
  `declarations.rs:42` (already diverged)
- `src/frontend/core/parser/ast.rs:891`, `930` — `extract_generic_param_names` and its
  boundary-crossing call
- `src/frontend/core/parser/ast.rs:191-213`, `217-223` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — Parser represents all primitive types as
  `Type::Name`
- `src/frontend/core/parser/statements/types.rs:392-396` — `Result`/`Option` are not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:836` — The only forward construction site for
  `Type::Void` (error fallback)
- `src/frontend/core/parser/statements/declarations.rs:42-80` — Original `name_used_as_type`
- `src/frontend/core/parser/statements/declarations.rs:494-511` — Hard-coded `"Terminates"` and the
  `typecheck::operator_interfaces::spec()` call
- `src/frontend/core/parser/pratt/nud.rs:505-514` — Expression position represented as an empty-body
  `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:466` — `expr_to_params` arm that retrieves the parameter
  list
- `src/frontend/core/types/mono.rs:839-886` — `from_builtin_name` builtin-name resolution table (15
  names + the temporary `DateTime` alias; WBS 6.1.2)
- `src/frontend/core/types/mono.rs:692-830` — `From<Type> for MonoType` implementation (12 of the 13
  match arms are here)
- `src/frontend/core/types/mono.rs` — `MonoType` definition (approximately 22 variants, starting at
  `mono.rs:167`; 1,077-line file)
- `src/frontend/core/types/const_data.rs:234-258`, `321-330` — `const_data::BinOp` /
  `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — Compile-time constant evaluation (1,128 lines)
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- `src/middle/core/ir.rs:3` — Evidence that `ir::Type` is an alias for `ast::Type`
- `src/middle/core/bytecode.rs:808-814` — `BytecodeFunction`’s `Vec<ir::Type>` fields
- `src/middle/core/bytecode.rs:2352-2390` — Lossy `From<MonoType> for IrType` bridge
- `src/middle/passes/codegen/bytecode.rs:32`, `275-278` — `FunctionCode`’s `Vec<MonoType>` fields
- `src/middle/passes/mono/function.rs:507-529` — `mono_to_ast_type`: reverse bridge that
  reconstructs 7 “dead variants”
- `src/backends/interpreter/image.rs:44` — Interpreter module’s `type_table`
- `build.rs:19-37` — Error-code registry generation-time gate (reference form for the type-table
  gate)
- `build.rs:39-54` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — Gate crate depends only on `serde_json` (the root crate has
  no `syn`; `tools/type-tables` introducing `syn` is the first exception)
