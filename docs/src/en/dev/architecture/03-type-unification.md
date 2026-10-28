# Unifying Type Representations

> **Ancillary design document**. This document accompanies
> [RFC-039 Compiler Architecture Refactoring](../../rfc/accepted/039-compiler-architecture.md). The
> four-layer model, tiered acceptance criteria, and execution-stage order are defined in the main
> body of RFC-039; the positioning of each ancillary document is described in the
> [index for this directory](index.md).

## Positioning and Scope

### Covered

Consolidate the multiple parallel type representations currently in use into one representation,
eliminating the four categories of burden derived from them:

- **13 `ast::Type` variants with zero forward construction sites in the parser** (among the 26
  variants in `ast.rs:427-541`, see 2.2; the “forward” criterion does not subtract reverse bridges
  or AST→AST reconstruction points)
- **A hand-written synonym table for type names** (`mono.rs:839-886`, see 2.3; WBS 6.1.2 has
  consolidated this into a 15-name resolution table)
- **The parser’s genuine reverse dependencies on the typecheck layer** (2 sites, see 2.4)
- **One expression variant, `Expr::FnDef`, with zero production construction sites**
  (`ast.rs:37-43`, see 2.6)

The concrete deliverables are four: a variant-by-variant disposition table for all 26 variants
(5.2), semantic typing of `Type::Name` through `NameKind` and elimination of the synonym table
(5.3), an injected `TypeEnvProbe` data flow (5.4), and type-table generation-time gates T1–T3 (5.7).

### Directory Renaming and This Document’s Path

RFC-039 decision D1 mandates renaming the implementation directories (`typecheck/`→`sema/`,
`middle/core/`→`middle/ir/`), to be completed **together with P5/P6 rather than in separate
stages**. This document’s 50-item change list and all acceptance greps are written against the
**pre-rename paths**—they form the construction baseline for the P6 batches **before the renaming
batch**; the directory rename at the end of each batch is a **pure relocation batch** (C1 zero-diff)
and is committed separately. The baseline becoming invalid after the rename commit is expected; use
git history as the bisection basis.

### Not Covered

- **How the directory rename is performed**. At this stage, `Type` does not move out of
  `parser/ast.rs`; only a new `lower.rs` is added under `types/` as the sole conversion point. The
  directory rename (`typecheck/`→`sema/`, etc.) is performed at the end of P5/P6 under RFC-039
  decision D1, in the form shown by the target directory structure in `01-routing.md`.
- **The four-layer model and dependency-direction rules**. See [01-routing.md](01-routing.md).
- **Tiering of equivalence criteria and validator implementation**. The definitions of C1–C6 and the
  three tiers of criteria—IR validators, normalized snapshots, and corpus differencing—are described
  in [07-equivalence-oracle.md](07-equivalence-oracle.md). This document cites only **C3
  (type-representation convergence)** as its acceptance criterion.
- **SSA conversion**. See [04-ssa.md](04-ssa.md). This document is its prerequisite, for the reasons
  described below.
- **The global P1–P10 execution sequence**. See the main body of RFC-039 and
  [02-stage-contract.md](02-stage-contract.md). This document provides only the stage breakdown for
  its own workstream (see “Implementation Essentials”).

### Division of Labor with RFC-039

RFC-039 is the overarching plan: the four-layer model, criterion tiers, cross-document stage order,
and the rationale for undertaking this refactoring now. This document is the construction plan for
**the type-representation convergence workstream** within RFC-039: which variants to delete, which
files to change, in what order, and which gates prevent regressions.

All line numbers, variant lists, and construction-site counts in this document were obtained through
static verification of `src/`, with each item cited as `file:line`. **This document does not record
the execution process; it records code facts and the dispositions derived from them.**

For ease of cross-referencing with documents in the same batch, the “Target Design” and “Detailed
Design” chapters retain their `5.x` and `6.x` subsection numbering; the remaining chapters follow
this directory’s unified style.

### Why This Must Precede SSA Conversion

`ir::Type` is not a third independent type representation; it is a `pub use` alias of `ast::Type`
(`ir.rs:3`). [04-ssa.md](04-ssa.md) is to introduce register type annotations and new instructions
into the IR. If the representation is not consolidated beforehand, SSA type annotations will flow
through the `ir::Type` alias directly onto the syntactic type’s 26 variants, effectively
institutionalizing a third representation.

The current capability boundary of `ir::Type` is defined by two variants:

- `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) embeds a complete expression tree in a type. An
  `Expr` cannot appear in an IR instruction’s type annotation.
- `ast::Type::Literal { name, base_type }` (`ast.rs:476-482`) carries a source name and `Span`. It
  is the syntactic form of a compile-time literal type, not a runtime type.

Applying SSA type annotations on top of the 26 variants would permanently cement the 13 dead
variants and the two syntax-only variants into the IR contract.

## Current State

Everything below is a statement of code fact and contains no proposals. Each item provides
`file:line` evidence.

### 2.1 Three Parallel Representations and a Lossy Bridge

The multiplicity of type representations is the common upstream cause of four independent defects:

| Downstream symptom                                                               | Causal relationship to type representations                                                                                                                                                                                                                                                                                                                      |
| -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The same type text has multiple valid AST forms                                  | Because `Type::Int(64)` (`src/frontend/core/parser/ast.rs:432`) and `Type::Name { name: "Int" }` (`ast.rs:428-431`) are **semantically equivalent but structurally different**, and the parser selects the latter, typecheck must restore the former through a table                                                                                             |
| Bytecode-layer signatures differ from IR-layer signatures                        | Because `ir::Type` is an alias rather than an independent type, `BytecodeFunction` (`src/middle/core/bytecode.rs:808`) can only obtain the alias. Consequently, `params` is declared as `Vec<ir::Type>` (`bytecode.rs:812`), while `FunctionCode` (`src/middle/passes/codegen/bytecode.rs:275`) declares `params` as `Vec<MonoType>` (`codegen/bytecode.rs:277`) |
| Adding one builtin type name requires changes in multiple places, with no checks | Because the name→type mapping is scattered across `from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) and the string match in `bytecode.rs:2371-2376`; each is written separately and the two are never cross-checked                                                                                                                                |
| The parser hardcodes `"Terminates"`                                              | Because the parser cannot determine “is this a type?” or “is this an operator interface?”, it must embed type-layer knowledge in the syntactic layer as a string literal (`src/frontend/core/parser/statements/declarations.rs:496`)                                                                                                                             |

**The first three are representation problems; the fourth is a downstream side effect of a
representation problem.** Doing only item 1—deleting dead variants—would leave the synonym table and
reverse dependency in place. Doing only item 4—injecting a callback into the parser—would build on
top of an inconsistent representation.

One of the three representations is an alias illusion:

| Name        | Definition                                | Size        | Role                                                    |
| ----------- | ----------------------------------------- | ----------- | ------------------------------------------------------- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541` | 26 variants | Syntactic-layer type                                    |
| `MonoType`  | `src/frontend/core/types/mono.rs`         | 1,077 lines | Semantic-layer type, after monomorphization             |
| `ir::Type`  | `src/middle/core/ir.rs:3`                 | —           | **`pub use crate::frontend::core::parser::ast::Type;`** |

`ir.rs:3` is a one-line `pub use`, and `ir.rs:6` then uses
`use crate::frontend::core::typecheck::MonoType;`. The IR module depends on two “type” symbols at
once, but one is only an alias.

The serialization side has a fourth type carrier:

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
fallback (`bytecode.rs:2386-2387`). The accompanying comment admits that this is a “transitional
branch before deleting enum variants.”

Here, `IrType::Void` is **a legitimate type value rather than an error signal**. Downstream code
therefore cannot distinguish “this function really returns `Void`” from “this function’s return type
was lost during bridging.” This is one reason the first-layer validator in 07 cannot currently make
its “type consistency” invariant pass.

At the same time, `IrType::String` and `IrType::Bytes` point to `ast::Type::String` (`ast.rs:435`)
and `ast::Type::Bytes` (`ast.rs:436`), which are precisely the dead variants to be deleted in
2.2—**both ends of the lossy bridge point to dead code**.

### 2.2 Parser-Forward Zero-Construction Variants

**Criterion**: The construction-site scan covers all of `src/`, excluding `tests/` and `*/tests/*`.
A **construction site is an expression-position `Type::<variant>` path**; pattern positions do not
count. Results are divided into **three categories that must not be mixed**: ① **parser-forward**
(parser → AST); ② **AST→AST reconstruction** (the two replacement functions in
`middle/passes/mono/function.rs`); ③ **reverse bridge** (`From<MonoType> for IrType` and
`mono_to_ast_type`).

Of the 26 variants of `ast::Type`, the following 13 have **zero construction sites in the
parser-forward direction**; see 2.2.1 and 2.2.2 for the other two reconstruction categories:

| Variant                              | Definition       | Parser-forward construction sites                                                                                                                                                                                                                                                                    |
| ------------------------------------ | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Int(usize)`                         | `ast.rs:432`     | None (`mono.rs:699` is a match arm)                                                                                                                                                                                                                                                                  |
| `Float(usize)`                       | `ast.rs:433`     | None (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                  |
| `Char`                               | `ast.rs:434`     | None (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                  |
| `String`                             | `ast.rs:435`     | None (`mono.rs:702` is a match arm)                                                                                                                                                                                                                                                                  |
| `Bytes`                              | `ast.rs:436`     | None (`mono.rs:703` is a match arm)                                                                                                                                                                                                                                                                  |
| `Bool`                               | `ast.rs:437`     | None (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                  |
| `Void`                               | `ast.rs:438`     | `types.rs:836` (**the sole forward site = error fallback** `_ => (Vec::new(), Type::Void)`, registered as an individual exemption; eliminated with item #12 in 6.4)                                                                                                                                  |
| `Enum(Vec<String>)`                  | `ast.rs:449`     | None (`mono.rs:747` is a match arm)                                                                                                                                                                                                                                                                  |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448`     | None (`mono.rs:743` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                       |
| `Option(Box<Type>)`                  | `ast.rs:455`     | None (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                       |
| `Result(Box<Type>, Box<Type>)`       | `ast.rs:456`     | None (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                       |
| `Sum(Vec<Type>)`                     | `ast.rs:472`     | None (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites, see 2.2.2                                                                                                                                                                                                                       |
| `AssocType { .. }`                   | `ast.rs:463-471` | None (`mono.rs:781-786`, `formatter/handlers/types.rs:84`, and `semantic_tokens.rs:230` are all match arms); **2 production AST→AST reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten with 6.3; **D8 remains unchanged** |

> **Definition-column line-anchor drift registration (P6 6.0 measurement, 2026-10-27)**: The
> “Definition” columns in this table and the table in 5.2 were written on 2026-10-05. Seventeen
> lines were subsequently inserted before the `ast::Type` enum (`pub enum Type` moved from 427 to
> 444), so **the definition lines of all 26 variants uniformly shift by +17** (for example, `Int`
> moves from 432 → 449, and `NamedParen` from 533-540 → 550-557). This round only **records** the
> drift and does not alter the table body; subsequent references use the `ast::Type` symbol anchor.

The root-cause chain can be fully verified:

1. **The parser represents every primitive type as `Type::Name { name }`**—the fallback arm in
   `src/frontend/core/parser/statements/types.rs:162-165` directly returns
   `Some(Type::Name { name, span: name_span })`.
2. **`Result` and `Option` are not lowered into dedicated nodes**—the comment in `types.rs:392-396`
   states explicitly: “RFC-010: Result/Option are not lowered into dedicated AST nodes… They follow
   the same `Type::Generic` path as user-defined generic types. The type representation remains
   `Generic{"Result"/"Option", args}`.”
3. **The sole forward construction point of `Type::Void` is an error fallback**—`types.rs:836`
   contains `_ => (Vec::new(), Type::Void)` at the end of a match returning `(Vec<Param>, Type)`.

`NamedStruct` (`ast.rs:443-447`), `Literal` (`ast.rs:476-482`), and `MetaType` (`ast.rs:497-504`)
have production construction sites: `types.rs:364`, `types.rs:571`, and `types.rs:100` plus
`declarations.rs:573`. They are not dead variants.

**These 13 variants have construction sites in test code**
(`src/frontend/core/types/tests/mono.rs:27-40`, `97`, `119-145`, `221`;
`src/frontend/core/typecheck/inference/tests/statements.rs:81`, `366`, `525`, `567`, `591`, `620`).
Therefore, this document uses the term “**zero forward construction in the parser**”: it means
neither “zero references globally” nor “zero production construction”—11 of these variants do have
production construction sites in the other two categories (reverse bridge / AST→AST reconstruction).
The tests themselves codify the behavior under test: they assert exactly what these syntactic forms
lower to as `MonoType`, even though the parser never produces them. These tests must be deleted or
rewritten when the variants are removed.

#### 2.2.1 Criterion Boundary (1): The Reverse Bridge Reconstructs Seven of Them

The table’s “zero forward construction” property holds only in the **parser → AST
forward-construction direction**. In the reverse direction, there are two production-grade
`MonoType → ast::Type` reconstructions that **actually construct seven of the variants in the table
above**:

| Reverse bridge                                                 | Location                                 | Reconstructed members of the 13-variant list                                                                             |
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
    // #299: String/Bytes and other container types are now Generic; reverse-resolve to AstType via type_name
    MonoType::Generic { name, .. } if name == "String" => AstType::String,
    MonoType::Generic { name, .. } if name == "Bytes" => AstType::Bytes,
    _ => AstType::Name { name: mono.type_name(), span },
}
```

**This criterion boundary directly affects the disposition**:

- The `bytecode.rs` site disappears with item #28 in 6.3, which deletes the entire
  `From<MonoType> for IrType` impl, and therefore adds no separate cost.
- `function.rs:507-529` **is not covered by any existing change item**. It is a `MonoType → AstType`
  type-name reverse resolver (annotated `#299`) that maps `String` and `Bytes` from `Generic` back
  to their dedicated variants by name. After deleting `ast::Type::String` and `Bytes`, these two
  guard branches must be rewritten. After deleting `Int`, `Float`, `Bool`, `Char`, and `Void`, the
  behavior of the `_` arm changes: some types that previously entered `_` will now silently take the
  `AstType::Name` path. This is item #47 added to 6.3.
- **Direct consequence for the gates**: A T1 implementation that scans only forward construction
  sites will classify the seven variants above as “having construction sites,” thereby excluding
  them from the deletion list. T1 must explicitly include the reverse bridge in its counting
  criterion; otherwise, the gate’s conclusions will conflict with the disposition table in this
  document (see 5.7).

#### 2.2.2 Criterion Boundary (2): AST→AST Reconstruction (the Third Category Added in This Revision)

`substitute_type_ast` (320-497) and `substitute_type_in_ast` (499-655) in
`middle/passes/mono/function.rs` are **AST→AST** type-variable substitutions that recreate an
existing AST type; they are **not** `MonoType → AST` conversions. The P6 T1 gate measurement (6.0.2
baseline, 2026-10-27) found **15 variants / 28 sites**, of which **5 variants / 10 sites** belong to
the 13-variant list above:

| Variant     | AST→AST reconstruction sites |
| ----------- | ---------------------------- |
| `Union`     | `377`, `606`                 |
| `Option`    | `427`, `600`                 |
| `Result`    | `431`, `602`                 |
| `Sum`       | `467`, `632`                 |
| `AssocType` | `456`, `623`                 |

**When 6.3 deletes these five variants, it must rewrite these 10 sites in the same batch** (already
registered in 6.3.2 of 09); otherwise, the affected arms will no longer compile.

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
different in the AST layer, and the parser produces only the latter, `from_builtin_name` must accept
capitalization variants, abbreviations, signed names, and the `DateTime` alias. The presence of
`"()"` on the `Void` line is especially revealing: it works in conjunction with the `Type::Void`
fallback in `types.rs:836`.

The same knowledge of “which names are builtin type names” exists in seven copies throughout the
repository, with no consistency checking whatsoever:

| #   | Location (**measured 2026-10-27**; this section was written on 2026-10-05, and the original line numbers have drifted)            | Form                                                                                                                                                                                                                                |
| --- | --------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs`, measured 839-886 / **15 arms / 16 spellings**; after WBS 6.1.2) | String match, name → `MonoType`                                                                                                                                                                                                     |
| 2   | `src/middle/core/bytecode.rs:2371-2376`                                                                                           | String match, copying `"String"`, `"Bytes"`, and `"Tuple"`                                                                                                                                                                          |
| 3   | `CONST_PARAM_TYPES` (`src/frontend/core/parser/ast.rs`, measured 855-858)                                                         | `&[&str]` constant containing 15 const generic parameter names (`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`)                                                                                            |
| 4   | `src/lsp/world.rs:182`                                                                                                            | Builtin type-name list on the LSP side (14 names = the 15 names minus `()`; WBS 6.1.2)                                                                                                                                              |
| 5   | `ConstKind::from_ast_type_name` (`src/frontend/core/types/const_data.rs`, measured 407)                                           | String match, name → `ConstKind`. **Coverage differs from item 1**: it does not accept lowercase or the `i64` / `i32` aliases (registered by the 2026-10-06 P0 gate)                                                                |
| 6   | Base-type size table for `eval_builtin` (`src/frontend/core/types/eval/const_eval.rs`, measured 497-512)                          | String match, name → byte size. It previously contained a `"Uint"` arm (item 1 has no `Uint` mapping)—WBS 6.1.2 removed that arm, and `sizeof(Uint)` now reports an unknown type. **A dead arm and living evidence of table drift** |
| 7   | `to_type_id` (`src/middle/passes/codegen/bytecode.rs`, measured 735-775; `"Set" => 25` at 762)                                    | String match, builtin name → serialized type ID. `"Set" => 25` contradicts the typecheck side’s “Set is a name other than a type” rule (`inference/expressions.rs:2917-2918`, decision 4 of #300)                                   |

> **Line-anchor drift registration (P6 6.0 measurement, 2026-10-27)**: The line numbers for all
> seven copies in this section **have drifted** (item 1 by +223 and the others by 2–9 lines each),
> but **the item counts and contents have not changed** (item 1 had 13 arms when this section was
> written, item 3 has 15 entries, and item 4 had 15 entries when written; after WBS 6.1.2, item 1
> has 15 arms / 16 spellings and item 4 has 14 entries). Subsequent references use the **symbol
> anchor + repository path**. The line numbers in this table are for location only. The arm-count
> drift and the live evidence in items 5–7—the dead `Uint` arm and the contradiction between `Set`
> and the typecheck side—**all remain**, and this section has not changed code.

Item 3, `CONST_PARAM_TYPES`, is used by `extract_generic_param_names`
(`src/frontend/core/parser/ast.rs`, measured 932) to identify const generic parameters. It
enumerates “type names that can be const parameters,” which **partially overlaps but is not
identical to** the full set handled by `from_builtin_name`, and no mechanism keeps the two
synchronized. Item 4 is the LSP’s fourth copy, used for syntax highlighting and hovers, and is
likewise unchecked; T2’s comparison scope includes it.

Items 5–7 were registered on the first run of `check-concepts.py` (P0, [09](09-execution-wbs.md)
§0.2.1) on 2026-10-06. Item 5 is a second type-name synonym table; items 6 and 7 share the same root
cause as items 1–4—string-keyed tables mapping builtin names to semantics or IDs—despite having
different uses and the same drift risk. The same gate also found 11 string dispatches for builtin
container names on the typecheck / eval / middle sides (`"List" | "Vec" | "Array"` ×4,
`"String" | "Bytes"` ×4, etc.). **These are outside both this section and the parser data-flow scope
in 5.4**; the inventory and disposition are registered in
[06-cleanup-inventory.md](06-cleanup-inventory.md) §G3 as P6 extended-verification items.

### 2.4 The Parser’s Reverse Dependency on the Type Layer

RFC-039 routing table C records three entries (`declarations.rs:27-80` calls
`is_type_param_annotation` / `name_used_as_type`, functions that belong to the type layer).
**Measurement differs from that description**: these two functions are defined inside the parser
itself, not in the type layer.

- `is_type_param_annotation` is defined in
  `src/frontend/core/parser/statements/declarations.rs:27-33` (the repository’s sole `fn`
  definition)
- `name_used_as_type` is defined in `declarations.rs:42-80` (the repository’s sole `fn` definition)
- `declare_predicate` / `is_predicate_name` are defined in
  `src/frontend/core/parser/parser_state.rs:46` / `54`, also inside the parser

There are exactly **2 production-code sites** that genuinely cross the layer boundary into
`typecheck`, and both call the same function:
`crate::frontend::core::typecheck::operator_interfaces::spec`:

| #   | Location                                                  | Enclosing function                    | Purpose                                                                                             |
| --- | --------------------------------------------------------- | ------------------------------------- | --------------------------------------------------------------------------------------------------- |
| 1   | `src/frontend/core/parser/statements/declarations.rs:509` | Signature-parameter filtering closure | Identifies constraint-position parameters (`T: Add`), which do not occupy runtime parameter slots   |
| 2   | `src/frontend/core/parser/ast.rs:930`                     | `extract_generic_param_names`         | Identifies constraint-position parameters and produces `GenericParamName` values with `constraints` |

The second site is deeper: **`parser/ast.rs` itself** (the entire `ast.rs:847-948` block, alongside
`StmtKind` and `Expr`) directly references a typecheck symbol. Routing table C records only the
`declarations.rs` site.

Together with the hardcoding in `declarations.rs:498-499`, this forms a single decision:

```rust
// declarations.rs:498-499
// Extended as subsequent builtin predicates are added (future open-set approach: pass predicate names down to
// the parser, or change to a form the syntax layer can determine).
// The parser does not hold a type environment, so it checks two places: hardcoded builtin predicates,
// plus declarations accumulated during this pass (`declare_predicate`).
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

The comment admits that this is a temporary form for a “future open-set approach.”
`n == "Terminates"` **embeds type-layer knowledge in the syntactic layer as a string literal**.

**Why this is downstream of the type-representation problem**: the parser needs to ask “is this an
operator-interface name?” because it must determine whether `Type::Name` refers to a type or an
interface constraint. **`Type::Name { name: String }` uses an undifferentiated string to carry at
least four meanings**—concrete type, type variable, constraint name, or predicate name. The
syntactic layer cannot decide this by itself, so the decision is forced up into the type layer.

#### Duplicate Implementation: Same Name and Meaning, Already Diverged

There are two implementations of `name_used_as_type`. The comment in `ast.rs:843-846` admits this:

```rust
/// Whether a parameter name is treated as a type reference in the given type (`(N: Int) -> (n: N)`).
///
/// Semantically equivalent to `name_used_as_type` in `declarations.rs`; independently implemented here to avoid
/// a cross-module dependency within the parser.
pub fn name_used_as_type_in(   // ast.rs:847
```

The two implementations **have already diverged**:

| Dimension               | `name_used_as_type` in `declarations.rs:42-80`                      | `name_used_as_type_in` in `ast.rs:847-885`           |
| ----------------------- | ------------------------------------------------------------------- | ---------------------------------------------------- |
| Signature               | Takes an `is_predicate: &dyn Fn(&str) -> bool` callback             | No callback                                          |
| `Type::Generic` branch  | Short-circuits to `false` after `is_predicate(app_name)` (`:54-56`) | No short-circuit; directly recurses into `args`      |
| `Type::Struct { body }` | **Absent** (falls into `_ => false`)                                | **Present** (`:871-879`, traverses `Field` / `Expr`) |
| `Type::NamedStruct`     | **Absent**                                                          | **Present** (`:880-882`, traverses `fields`)         |

The comment in `ast.rs:868-870` records the origin of the latter two branches: “These two branches
were previously missing, so a const generic parameter referenced within a definition body could not
be recognized as ‘used as a type’; annotation validation incorrectly reported valid const parameters
as unknown names.”

In other words, **a function copied to avoid a cross-module dependency did not receive subsequent
fixes**. The `ast.rs` version recursively traverses fields in definition bodies, while the
`declarations.rs` version does not; the same source construct therefore produces different results
at two decision points. This duplication must be consolidated when 5.3 introduces `NameKind`.

### 2.5 Parallel Operator Enums

There are two operator enums. They do not reference each other, and **their members have already
diverged**:

| Concept          | Syntactic layer                 | Constant-evaluation layer                                             | Difference                                                                                                                                    |
| ---------------- | ------------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Binary operators | `ast::BinOp` (`ast.rs:191-213`) | `const_data::BinOp` (`src/frontend/core/types/const_data.rs:234-258`) | The syntactic layer alone has `Neq` (`:197`), `Range` (`:205`), and `Assign` (`:206`); the constant layer uses `Ne` (`:243`) instead of `Neq` |
| Unary operators  | `ast::UnOp` (`ast.rs:217-223`)  | `const_data::UnOp` (`const_data.rs:321-330`)                          | The syntactic layer alone has `Deref` (`:222`); the constant layer alone has `BitNot` (`:329`)                                                |

Each also implements a complete `Display` (`const_data.rs:291-317`) and classification predicates
(`is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise` in `const_data.rs:260-289`).

**Why this is part of the type-representation problem**: `ast::BinOp::Assign` (`ast.rs:206`) and
`ast::UnOp::Deref` (`ast.rs:222`) **are not types**. When they occur in type-position expressions,
they fall into `Type::ConstExpr` (`ast.rs:506`), the “compile-time-only variant.” This is directly
related to the IR-layer type discipline in 6.1: after convergence, `Type::ConstExpr` disappears once
`Type → MonoType` has been applied and is never visible to the IR, while `Assign` and `Deref`
continue to exist through paths such as `ir_gen.rs`’s `BinOp::Assign => Ok(MonoType::Void)`
(`inference/expressions.rs:875`). The divergence between the two enums makes it impossible to
determine “which operators may appear in type positions?” from either side alone.

This stage does not consolidate the two enums—their ownership is covered by the “operator-change
surface” section of [05-frontend-paradigm.md](05-frontend-paradigm.md)—but the T1 gate’s scan set
should include `ast::BinOp` / `ast::UnOp`, making newly added zero-construction operators equally
visible.

### 2.6 `Expr::FnDef`: An Expression Variant with Zero Production Construction Sites

It is true that `Expr::FnDef` (`ast.rs:37-43`) has **zero production construction sites**—the
repository contains only two construction sites, `src/frontend/core/parser/tests/ast.rs:837` and
`src/frontend/core/typecheck/tests/checker.rs:111`, both in tests.

However, **there are far more consumption sites than four**. The measured production consumption
count is 12:

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

There are also 2 exhaustive match arms that exist only because of it: `ast.rs:1026` (`Expr::span()`)
and `pratt/mod.rs:47` (`expr_end_line`).

**The path actually taken by function definitions** is: `nud.rs:425-429` constructs `Expr::Lambda`,
and `declarations.rs:462-479` constructs `StmtKind::Assign` (placing the `Lambda` in `value`). In
other words, the `FnDef` branch is **a parallel path that never executes but is maintained at 14
locations**.

### 2.7 Syntax Nodes Store Semantics on Behalf of the Type Layer

**First, `StmtKind::Assign` carries a typechecker-specific field.** The `Assign` variant in
`ast.rs:241-249` contains `signature_params: Vec<Param>` (`ast.rs:244-245`), and the comment states
directly:

> `/// The first group of signature parameters is preserved as-is (including parameter names), for the typechecker’s classify_generic_params to use`

The statement AST reserves a dedicated slot for the type checker. Its construction site is
`declarations.rs:472`.

**Second, an expression position is treated as a parameter list.** `(a: Int, b: Int)` is parsed at
`src/frontend/core/parser/pratt/nud.rs:505-514` as
**`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**—a Lambda with an
**empty body**. The arm `Expr::Lambda { params, .. } => Some(params.clone())` at
`src/frontend/core/parser/pratt/led.rs:466` then “retrieves” it and uses it as a parameter list.

In other words, a normal AST variant is used as a **temporary parameter-list carrier**, and another
module’s match arm restores its meaning.

**Third, `Type::NamedParen` carries the return-position binder name.** The comment in
`ast.rs:519-540` states:

> The return-position refinement in RFC-027 §3 relies on it to declare the **return formal parameter
> name** (called a binder here)… **Without it, the type checker can only guess** that “a free
> variable in a constraint that is not in scope is the return formal parameter,” so `(r: P(m))`
> silently substitutes the undeclared `m` as a binder as well (observed defect).

A syntax node is storing binder identity on behalf of the type layer.

## Target Design

### 5.1 Shape of the Target Representation

**Position: a single representation means using `Type` (formerly `ast::Type`) as the sole type
structure; `MonoType` is demoted to the type checker’s internal working form; and the `ir::Type`
alias is deleted.**

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

**Why not “consolidate everything into `MonoType`”** (the proposal most likely to be raised, and
rejected here):

1. `Type` carries `Span` (`ast.rs:430`, `458`, `464`, `478`, `499`, `490`, `533-540`, etc.), whereas
   `MonoType` does not. C3 in 07 requires identical diagnostic codes and messages—diagnostic
   locations must remain traceable to the source. Adding `Span` to `MonoType` would burden
   monomorphized types with meaningless sentinel spans.
2. `MonoType` contains `TypeVar` and substitution states (`types/substitute.rs:112`; `pub fn unify`
   in `types/solver.rs:411`). These are **intermediate states of the type checker’s solving
   process**, not types themselves. Putting them into the sole type structure would force the IR
   construction layer to understand type variables.
3. `ast::Type::ConstExpr(Box<Expr>)` (`ast.rs:506`) and `ast::Type::Literal` (`ast.rs:476-482`) are
   **compile-time** concepts (RFC-027), lowered away in `MonoType`. Consolidating on `MonoType`
   would require adding these syntactic forms back to it.
4. **The decisive reverse argument**: promoting `MonoType` to the sole representation would make the
   L2 syntactic layer depend on the L3 semantic layer’s type. RFC-039 explicitly prohibits L2 from
   depending on L3. Eliminating reverse dependencies is one of this document’s objectives, so
   choosing that design would be self-contradictory.

**Why `ir::Type` must be deleted rather than retained**:

The alias in `ir.rs:3` causes `BytecodeFunction.params` (`bytecode.rs:812`) and `return_type`
(`bytecode.rs:814`) to use `ir::Type`, while the serialization side uses `MonoType` at
`codegen/bytecode.rs:277-278`. The two field types are connected by
`file.type_table.into_iter().map(|t| t.into())` at `bytecode.rs:2339` and the lossy `From` bridge at
`bytecode.rs:2353-2390`.

**The `From` bridge exists solely because “the field types on the two sides differ.”** Once
`bytecode.rs:812`, `bytecode.rs:814`, `bytecode.rs:854`, and `image.rs:44` are uniformly changed to
`MonoType`, `From<MonoType> for IrType` loses its reason to exist and can be deleted in full, also
eliminating the reverse reconstruction recorded in 2.2.

**Explicit non-goals**: Do not move `Type` from `parser/ast.rs` to `sema/types/` at this stage. Add
only `lower.rs` under `types/` as the sole conversion point, leaving the definition of `Type` in
place. The directory rename is performed at the end of P6 under D1 as an independent relocation
batch; at that point, `Type` will live in the top-level `ast/type_.rs` AST domain, while
`sema/types/` will be the home of the `MonoType` working representation.

### 5.2 Variant-by-Variant Disposition of the 26 Variants

> Disposition categories: **delete** (zero forward construction in the parser), **retain** (forward
> construction exists), and **migrate** (semantics move to another node).

| #   | Variant                                                            | Definition       | Disposition                            | Rationale                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| --- | ------------------------------------------------------------------ | ---------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `Name { name, span }`                                              | `ast.rs:428-431` | **Retain (strengthened)**              | The parser’s **sole** primitive-type output (`types.rs:162-165`). After deleting the 12 primitive-type variants, every primitive type will use it. A semantic kind for the name must be added (see 5.3)                                                                                                                                                                                                                                                       |
| 2   | `Int(usize)`                                                       | `ast.rs:432`     | **Delete**                             | Zero forward construction in the parser (`mono.rs:699` is a match arm). Bit width is carried through the `Name` path in `from_builtin_name`                                                                                                                                                                                                                                                                                                                   |
| 3   | `Float(usize)`                                                     | `ast.rs:433`     | **Delete**                             | Zero forward construction in the parser (`mono.rs:700` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                        |
| 4   | `Char`                                                             | `ast.rs:434`     | **Delete**                             | Zero forward construction in the parser (`mono.rs:701` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                        |
| 5   | `String`                                                           | `ast.rs:435`     | **Delete**                             | Zero forward construction in the parser (`mono.rs:702` is a match arm). **Note**: `IrType::String` at `bytecode.rs:2372` consumes it, so that arm must be rewritten after deletion                                                                                                                                                                                                                                                                            |
| 6   | `Bytes`                                                            | `ast.rs:436`     | **Delete**                             | Zero forward construction in the parser (`mono.rs:703-706` are match arms). As above, `bytecode.rs:2373` consumes it                                                                                                                                                                                                                                                                                                                                          |
| 7   | `Bool`                                                             | `ast.rs:437`     | **Delete**                             | Zero forward construction in the parser (`mono.rs:707` is a match arm)                                                                                                                                                                                                                                                                                                                                                                                        |
| 8   | `Void`                                                             | `ast.rs:438`     | **Delete**                             | The parser’s sole forward construction site = error fallback `types.rs:836` (`_ => (Vec::new(), Type::Void)`, registered as an individual exemption). The fallback is eliminated together with the error-path refactoring in 6.4                                                                                                                                                                                                                              |
| 9   | `Struct { body }`                                                  | `ast.rs:439-442` | **Retain**                             | Has production consumers (`mono.rs:709`, `checker.rs:3167`, etc.)                                                                                                                                                                                                                                                                                                                                                                                             |
| 10  | `NamedStruct { name, name_span, fields }`                          | `ast.rs:443-447` | **Retain**                             | Has production construction site `types.rs:364`                                                                                                                                                                                                                                                                                                                                                                                                               |
| 11  | `Union(Vec<(String, Option<Type>)>)`                               | `ast.rs:448`     | **Delete**                             | Zero forward construction in the parser (`mono.rs:743-746` are match arms); 2 AST→AST reconstruction sites (`function.rs:377`/`606`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                 |
| 12  | `Enum(Vec<String>)`                                                | `ast.rs:449`     | **Delete**                             | Zero forward construction in the parser (`mono.rs:747` is a match arm); zero across all three categories—the sole globally zero-construction variant. Enums use `Struct` + `TypeBodyItem`, unrelated to `mono.rs:2378`’s `MonoType::Enum(_) => IrType::Void`                                                                                                                                                                                                  |
| 13  | `Tuple(Vec<Type>)`                                                 | `ast.rs:450`     | **Retain**                             | Consumed by `bytecode.rs:2374`                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 14  | `Fn { params, return_type }`                                       | `ast.rs:451-454` | **Retain**                             | Consumed by `bytecode.rs:2366`; **field reshaping `params: Vec<Type>` → `Vec<Param>` (name and type together) follows 6.8.1 (D54)**—the names in `Assign.signature_params` are carried forward this way (see 6.5.2)                                                                                                                                                                                                                                           |
| 15  | `Option(Box<Type>)`                                                | `ast.rs:455`     | **Delete**                             | Zero forward construction in the parser (`mono.rs:770` is a match arm); 2 AST→AST reconstruction sites (`function.rs:427`/`600`) are rewritten in the same batch as 6.3.2. `types.rs:392-396` already explains that this uses the `Generic` path                                                                                                                                                                                                              |
| 16  | `Result(Box<Type>, Box<Type>)`                                     | `ast.rs:456`     | **Delete**                             | Zero forward construction in the parser (`mono.rs:771` is a match arm); 2 AST→AST reconstruction sites (`function.rs:431`/`602`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                     |
| 17  | `Generic { name, name_span, args }`                                | `ast.rs:457-461` | **Retain**                             | The actual representation of `Result` / `Option` / `String` / `Bytes` (`types.rs:397-399`)                                                                                                                                                                                                                                                                                                                                                                    |
| 18  | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **Delete**                             | Zero forward construction in the parser (the syntactic layer has no `::` path that produces it); **2 production AST→AST reconstruction sites**: `function.rs:456` (`substitute_type_ast`) and `function.rs:623` (`substitute_type_in_ast`), rewritten in the same batch as 6.3 (registered in 6.3.2). **D8 remains unchanged**: the criterion is forward construction; if associated-type syntax is enabled in the future, a new proposal will define it then |
| 19  | `Sum(Vec<Type>)`                                                   | `ast.rs:472`     | **Delete**                             | Zero forward construction in the parser (`mono.rs:814` is a match arm); 2 AST→AST reconstruction sites (`function.rs:467`/`632`) are rewritten in the same batch as 6.3.2                                                                                                                                                                                                                                                                                     |
| 20  | `Literal { name, name_span, base_type }`                           | `ast.rs:476-482` | **Retain**                             | Has production construction site `types.rs:571`; RFC-027 const generics                                                                                                                                                                                                                                                                                                                                                                                       |
| 21  | `Ptr(Box<Type>)`                                                   | `ast.rs:485`     | **Retain**                             | Raw pointer type inside an unsafe block                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 22  | `Ref { mutable, inner, span }`                                     | `ast.rs:488-492` | **Retain**                             | Borrow notation                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 23  | `MetaType { name_span, args }`                                     | `ast.rs:497-504` | **Retain**                             | Has production construction sites `types.rs:100` and `declarations.rs:573`; RFC-010                                                                                                                                                                                                                                                                                                                                                                           |
| 24  | `ConstExpr(Box<Expr>)`                                             | `ast.rs:506`     | **Retain (mark as compile-time only)** | Has production construction site `types.rs:160`. It is the only `Type` variant that embeds an `Expr`, and the IR layer must not carry it (see 6.1)                                                                                                                                                                                                                                                                                                            |
| 25  | `Paren(Box<Type>)`                                                 | `ast.rs:518`     | **Retain**                             | RFC-004 currying’s layer terminator; `split_curry` depends on its existence                                                                                                                                                                                                                                                                                                                                                                                   |
| 26  | `NamedParen { param, param_span, inner }`                          | `ast.rs:533-540` | **Migrate**                            | The semantics—the binder name—move to the type checker; the AST retains only the syntax. See 5.6                                                                                                                                                                                                                                                                                                                                                              |

**Net effect: 26 → 13 variants** (13 deleted, 1 migrated, 12 retained).

> **The reverse bridge and AST→AST reconstruction must be handled before deleting variants**:
> `function.rs:507-529` (`mono_to_ast_type`), recorded in 2.2, reconstructs seven of items 2–8 in
> this table; in addition, the AST→AST reconstruction recorded in 2.2.2 covers 5 variants / 10 sites
> within the 13-variant list (2 each for Union/Option/Result/Sum/AssocType). Both reconstruction
> categories must be handled in the same batch as 6.3.1; otherwise, behavior either changes silently
> (see #47 in 6.3) or compilation fails outright.

> **How to handle `AssocType`**: **D8 remains unchanged**, and the variant is deleted directly under
> D8 (item 18 in the 5.2 table). The **criterion is forward construction**: zero forward
> construction holds because the syntactic layer has no path that produces `::` associated types.
> Therefore, D8 does not need to be changed, and the variant does not enter the whitelist. The **2
> production AST→AST reconstruction sites** (`function.rs:456`, `substitute_type_ast`, and
> `function.rs:623`, `substitute_type_in_ast`) are already registered in 6.3.2 and are rewritten in
> the same batch as 6.3. The T1 gate’s three-category split in 5.7 has reproduced this conclusion
> (all 26 variants matched one-for-one in the 6.0.1 commit measurement).

### 5.3 `NameKind` and Elimination of the Synonym Table

**Problem**: `Type::Name { name: String }` uses an undifferentiated string to carry at least four
meanings—concrete type, type variable, constraint name (operator interface), or predicate name.
`src/frontend/core/parser/statements/declarations.rs:500-501` must determine “is this a predicate?”,
`510-512` in the same file must determine “is this an operator interface?”,
`src/frontend/core/parser/ast.rs:971` must determine “is this an operator interface?”, and the two
`name_used_as_type*` implementations—`name_used_as_type_in` in the same file (measured 864) and
`name_used_as_type` in `declarations.rs:42`—must determine “is this a type reference?”. All rely on
string comparison.

**Proposal: add a `kind` field to `Type::Name`, determined once by the parser.**

```rust
// Target form (illustrative; line numbers are after the changes)
Name {
    name: String,
    kind: NameKind,   // New
    span: Span,
}

enum NameKind {
    Builtin,      // Builtin type name: Int / Float / Bool / ... → delegated to from_builtin_name
    UserType,     // User-declared type name, including Generic forms such as std.result / std.option
    TypeVar,      // Type variable: T / K / V
    Constraint,   // Operator interface / constraint name: Add / Ord / ...
    Predicate,    // Compile-time predicate: Terminates / Sorted / ...
}
```

**Key design point: determining `NameKind` requires precisely the information the parser cannot
currently obtain**—the type environment, the declared operator-interface table, and the declared
predicate table. Therefore, this field **cannot be determined independently inside the parser**—it
must be determined by an injected environment. This leads directly to 5.4.

**Disposition of the synonym table (WBS 6.1.2 completed, user-approved on 2026-10-27)**:

`MonoType::from_builtin_name` (`src/frontend/core/types/mono.rs:839-886`) is **retained with reduced
semantics**. Before the change, it disambiguated two equivalent AST representations; after
`Type::Int(64)` is deleted, “Int” has only one representation path, and it degenerates into the
**sole name→type resolution table**.

Reasons it cannot be deleted:

- It is the **sole cut point** for standard-library type names and language builtin names (the
  signature strings in `src/std/*.rs`, `CONST_PARAM_TYPES`, and the LSP lists all use it as their
  reference).
- The `DateTime` arm records a real fix (#338 related issue: the return value of `now()` could not
  be passed to `format_time`)—it is a **semantic alias**, not the same kind of capitalization alias
  as `Int`/`int`, so it is retained separately until P10 S11 (issue #513).

**Disposition result** (final criterion = 15 builtin names + the temporary `DateTime` alias):

| Category                                                                                                                                                                                                                                               | Before                                          | Disposition                                                                                                                                                                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 15 builtin names                                                                                                                                                                                                                                       | Mixed into 13 arms with aliases                 | **One name per arm**: `Never` / `Void` / `()` / `Bool` / `Int` / `Int8` / `Int16` / `Int32` / `Int64` / `Float` / `Float32` / `Float64` / `Char` / `String` / `Bytes`                                                       |
| Capitalization and abbreviation aliases (`int` / `int64` / `i64` / `int32` / `i32` / `int16` / `i16` / `int8` / `i8` / `float` / `float64` / `f64` / `float32` / `f32` / `bool` / `char` / `string` / `str` / `bytes` / `void` / `never` / `datetime`) | Retained                                        | **Delete** (22 spellings); **do not establish a lexer alias table**—corpus measurements found zero noncanonical names (see the design judgment at the end of this section), so syntactic-layer compatibility is unnecessary |
| `()`                                                                                                                                                                                                                                                   | Shared arm with `Void`                          | **Retain**—`()` is the type spelling of `Void` (the standard-library signature string uses `() -> Void`), not another type                                                                                                  |
| `DateTime`                                                                                                                                                                                                                                             | `datetime` shared an arm with lowercase aliases | **Retain only the capitalized** temporary compatibility alias, noting that its migration belongs to P10 S11 (issue #513)                                                                                                    |

**Net effect**: `from_builtin_name` = **15 match arms / 16 spellings / 1 alias** (`()` shares an arm
with `Void`; there is also one temporary `DateTime` alias). After deleting the 22 spellings, a
noncanonical spelling in type position becomes a `TypeRef`, and
`TypeEnvironment::resolves_type_name` reports an unknown type—**it is no longer silently accepted**.
Previously, `add_builtin_types` also registered lowercase copies of `int` / `float` / `bool` /
`string` / `void` / `never` / `char` in the type table, constituting a second alias table; these
were deleted in the same batch.

This acceptance surface has **two permanent gates**:

1. Positive and negative unit cases in `src/frontend/core/types/tests/mono.rs` (the expected
   `MonoType` for each of the 15 names, `None` for all 22 spellings, and `DateTime` → `Int(64)`);
2. T2.5, the “deleted-spelling sentinel,” in `tools/type-tables --report`—if any of the 22 spellings
   is added back to the resolution table, the report immediately counts it as **unregistered drift**
   (there is no disposition that can be registered: that would be a regression).

**Consolidating the two `name_used_as_type*` implementations**: the duplicate implementations
recorded in 2.4 must be consolidated, otherwise the semantic information provided by `NameKind` will
be bypassed by two independent string-comparison implementations. The consolidated implementation
belongs in `parser/ast.rs` (next to the `Type` data), and `declarations.rs` must call it. The
`is_predicate` short-circuit changes to read `NameKind::Predicate`, and the `Struct` / `NamedStruct`
field recursion follows the newer, more complete version at `ast.rs:894-905`.

**Ownership of `CONST_PARAM_TYPES`**: The const generic parameter-name table `CONST_PARAM_TYPES`
(`src/frontend/core/parser/ast.rs`, measured 855-858) partially overlaps the full set handled by
`from_builtin_name` (see 2.3). After `NameKind::Builtin` is introduced, the check in
`extract_generic_param_names` (same file, measured 932) can change to “`kind == Builtin` and the
name is in the accepted const-parameter set,” explicitly connecting the two pieces of knowledge. The
definition of the const-parameter subset is validated by the T2 gate (see 5.7).

> **Design judgment**: If the `.yx` corpus contains type annotations such as `i64` or `int`,
> normalizing them to `Int` would change the **diagnostic locations** of those files (though not
> their diagnostic codes). C3 permits this (messages and order may be normalized), but the baseline
> must be recorded in stage 1. **Measurement (P6 6.0, 2026-10-27)**: a full scan of `tests/yaoxiang`
> (316 `.yx` files) + `tests/yaoxiang-multifile` + `src/std` found **zero noncanonical names in
> actual type positions**; 16 raw-token matches were adjudicated individually as 12 strings and 4
> comments (both instances of `i64` / `f64` were in comments). Therefore, 6.1’s name normalization
> **does not change diagnostic locations in any corpus**, and **there is no need to regenerate the
> baseline**.
>
> **Retest (P6 6.1 closure, 2026-10-28)**: 6.1.1 rescanned the finalized **22 spellings**—the
> 2026-10-27 result used a **narrow criterion** (only the 14 integer/floating-point-family
> spellings, excluding namespace use in `src/std`), which is why it recorded “16 raw-token matches =
> 12 strings + 4 comments.” The retest scope was 316 `.yx` files in `tests/yaoxiang` + 39 in
> `tests/yaoxiang-multifile` + 38 in `src/std` = **393 files**, with matches on **308 lines**; only
> **2 lines were type-position candidates, and both were inside `//` comments**
> (`tests/yaoxiang/06-compile-errors/runtime/int_overflow.yx:5`, `i64::MAX`;
> `tests/yaoxiang/99-demos/compare_rust_side_by_side.yx:17`, `-> f64`). Thus, **the number of actual
> type positions remains zero**. The remaining matches were individually adjudicated as string
> literals / comments / `std.string` module names and namespace access / `Json` variant names /
> `void` literals; **none was a type annotation**. See the 6.1.1 line in [09](09-execution-wbs.md)
> and 09a “P6 6.1 Name Normalization Closure” for the retest command. The conclusion is consistent
> with 6.0: 6.1’s name normalization **does not change diagnostic locations in any corpus**, and
> **there is no need to regenerate the baseline**.

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

| File                                                                                | Change                                                                                                                                                                                                                                                      |
| ----------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/frontend/core/parser/probe.rs` (new)                                           | Define the `trait TypeEnvProbe`, providing four pure query methods: `is_predicate` / `is_constraint` / `is_type_var` / `is_builtin_type`. **The trait is defined in L2 and implemented by L3**—this is legitimate dependency inversion                      |
| `src/frontend/core/parser/parser_state.rs:46-58`                                    | Retain `declare_predicate` / `is_predicate_name` for accumulation within the current pass; add a `probe: Box<dyn TypeEnvProbe>` field and a `probe()` accessor                                                                                              |
| `src/frontend/core/parser/statements/declarations.rs:498-499`                       | Delete `n == "Terminates"`; change to `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)`                                                                                                                                                       |
| `src/frontend/core/parser/statements/declarations.rs:508-511`                       | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(n)` call; change to `state.probe().is_constraint(n)`                                                                                                                                |
| `src/frontend/core/parser/ast.rs:930`                                               | Delete the `crate::frontend::core::typecheck::operator_interfaces::spec(name)` call. `ast.rs` defines `Type` / `Expr` / `StmtKind` and must not hold an environment dependency—the function must instead accept a `&dyn TypeEnvProbe` parameter (see below) |
| All 5 parser construction sites (`Parser::new` / `ParserState::new` in `parser.rs`) | Accept a `probe: Box<dyn TypeEnvProbe>` parameter. **The default implementation, `NullProbe` (everything returns false), ensures parser unit tests remain unaffected**                                                                                      |

**The `extract_generic_param_names` signature problem**:
`pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` at `ast.rs:891` is a
free function that accepts no context. After deleting the boundary-crossing call at `ast.rs:930`, it
needs the `is_constraint` check, so its signature must add `probe: &dyn TypeEnvProbe`. Callers,
including `parser/tests/ast.rs`, must pass `&NullProbe` accordingly.

**When `NameKind` is determined**: when `types.rs:162-165` constructs `Type::Name`, use
`state.probe()` to determine `kind`, fixing it once so that subsequent match arms no longer perform
string comparisons.

**This eliminates all 3 entries in RFC-039 routing table C**—2 of them involve only parser-internal
functions under the 2.4 criterion, while the actual cross-layer references are the 2 sites in
`declarations.rs:509` and `ast.rs:930`, both covered by this section.

### 5.5 Consolidating `Expr::FnDef`

**Position: delete `Expr::FnDef` (`ast.rs:37-43`); function definitions uniformly use
`Expr::Lambda` + `StmtKind::Assign`.**

**This is the path already in use**: `nud.rs:425-429` + `declarations.rs:462-479` (see 2.6). The
`FnDef` branch is a never-executed parallel implementation.

**Change list** (12 consumption sites + 2 exhaustive arms, listed individually):

| Location                             | Existing arm                                         | Target                                                                                                                                              |
| ------------------------------------ | ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `spawn/placement.rs:122`             | `Expr::FnDef { body, .. } => self.check_block(body)` | Merge into the existing `Expr::Lambda` arm (`body` is also a `Box<Block>`; **delete this arm directly**)                                            |
| `spawn/analysis.rs:912`              | `Expr::FnDef { body, .. } => { ... }`                | As above; merge into the Lambda arm                                                                                                                 |
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
| `layers/termination.rs:752`          | `Expr::FnDef {`                                      | Match `Expr::Lambda` instead; update the comment at `termination.rs:2303` (“`fn(): Never` — FnDef.return_type is the bare return type”) accordingly |
| `ast.rs:1026`                        | `\| Expr::FnDef { span, .. }`                        | Delete this or-pattern                                                                                                                              |
| `pratt/mod.rs:47`                    | `Expr::FnDef { span, .. } => *span`                  | Delete this arm                                                                                                                                     |
| `typecheck/tests/checker.rs:109-111` | Test constructs `Expr::FnDef`                        | Rewrite as `Expr::Lambda` + `Assign`                                                                                                                |
| `parser/tests/ast.rs:837-847`        | Test constructs `Expr::FnDef`                        | Delete this test                                                                                                                                    |

**No behavior change**: `Expr::Lambda` has `params: Vec<Param>`, `body: Box<Block>`, and `span`,
structurally identical to the first three fields of `FnDef`. The `name` field unique to `FnDef` has
no use at the `Expr` layer—the name is on `StmtKind::Assign.target`.

**Risk and mitigation**: non-exhaustive `if let` matches such as `checker.rs:1642` and
`inference/expressions.rs:3374` become impossible matches after the change, producing neither an
error nor a warning. Mitigation: the type-table gate in 5.7 adds a “zero-construction variant”
check. After `Expr::FnDef` is deleted, any new `if let Expr::FnDef` fails to compile because the
variant no longer exists. **This is a compile-time guarantee and requires no additional gate.**

### 5.6 Disposition of `signature_params` and `NamedParen`

**`StmtKind::Assign.signature_params` (`ast.rs:244-245`) → delete.**

The reason for its existence—the comment at `ast.rs:244`—is “for the typechecker’s
`classify_generic_params` to use”: **a dedicated slot reserved by the statement AST for the type
checker**. However, `extracted_params` in the same function at `declarations.rs:500-511` has already
computed `value_params` after filtering type-position and constraint-position parameters. Passing it
through the statement is redundant transport.

Change: delete the field at `ast.rs:244-245` → delete the argument at `declarations.rs:472` → change
the source of `classify_generic_params` to recompute it locally (it can obtain `Lambda.params` from
`value: Option<Box<Expr>>`). **Net result: one field declaration, one construction site, and one
transport path are removed.**

> **Risk**: `signature_params` is the **unfiltered** parameter list, whereas `Lambda.params` is
> **filtered**—the filter at `declarations.rs:500-511` removes type-position and constraint-position
> parameters. If `classify_generic_params` depends on the unfiltered version, the refactor changes
> behavior. **The actual list used by `classify_generic_params` must be verified in stage 1;
> equivalence must not be assumed.**

**`Type::NamedParen` (`ast.rs:533-540`) → retain the node, but move binder semantics out of it.**

The comment at `ast.rs:519-532` records its **sufficient reason** for existence (see 2.7): in
`(r: P(m))`, dropping `r` would leave the type checker to guess that “a free variable in a
constraint that is not in scope is the return formal parameter,” which would **silently substitute
the undeclared `m` as a binder as well** (the comment marks this as an “observed defect”).

Therefore, **it cannot simply be deleted**—that would reintroduce an already fixed correctness
defect. The correct disposition is:

1. Retain `NamedParen { param, param_span, inner }` as a **pure syntax node**. `param` /
   `param_span` carry source facts only and no longer bear type-layer binder semantics.
2. Move interpretation of the binder to `sema/`: the type checker **explicitly** extracts `param` as
   the binder when consuming `NamedParen`. This is “reading a syntax node’s field,” equivalent in
   nature to the `Expr` embedded in `ConstExpr`—the syntax node provides facts, and the semantic
   layer interprets them.
3. Rewrite the comment from “the type checker can only guess” to “this node provides the binder
   name; semantic interpretation is performed in `sema/`.”

**Why this is a migration rather than a deletion**: the syntax tree must represent source code
without loss. `r` genuinely exists in the source, so the AST cannot pretend otherwise. The problem
is not the node’s existence, but the failure to assign responsibility for semantic interpretation
explicitly.

### 5.7 Type-Table Generation-Time Gates

**The reference implementation has already been validated in this repository**: `build.rs:19-37`
calls `parse_registry` / `validate` from `tools/code-tables`, performing uniqueness, segment, and
Chinese-completeness checks over 145 error codes, and directly calls `panic!` to reject the build
when `is_ok()` is false. `build.rs:39-54` further compares the code table entry by entry against
RFC-013’s Markdown table and also calls `panic!` on a mismatch. **RFC-013’s documentation and code
therefore remain consistent at all times**—`tools/code-tables/Cargo.toml:15-16` has only one
`serde_json` dependency, and the validation logic is text-level parsing with no dependency on `syn`.

The type tables currently have **no** comparable gate. This document adds a mechanism of the same
form.

**New crate**: `tools/type-tables` (`Cargo.toml` depends only on `serde_json`, symmetric with
`code-tables`). Four checks:

| #                                                                 | Check                                                                                                                                                                                                                                                                  | Inputs                                   | Failure condition                                                                                                                                                                                                                                                                                                                                      | Remedy                                                                                                              |
| ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| **T1: construction-site split**                                   | Scan the variants of `pub enum Type` in `src/frontend/core/parser/ast.rs`; under `src/`, count each variant’s construction sites in **three categories** (① parser-forward ② AST→AST reconstruction ③ reverse-bridge reconstruction; exclude `tests/` and `*/tests/*`) | Variant list + three-category hit tables | ① A variant has **zero forward construction** and is not registered; ② a registered variant has a forward construction site; ③ a registered wildcard-arm fallback exemption does not match; ④ an **unregistered wildcard-arm fallback site** exists (fallback exemptions are registered individually and must not be generalized to all wildcard arms) | Delete the variant; or register a rationale / remove the stale registration (see 5.2 and `REGISTERED_ZERO_FORWARD`) |
| **T2: synonym tables consistent with repository-wide name lists** | The `from_builtin_name` match table in `types/mono.rs` + `CONST_PARAM_TYPES` at `ast.rs:855` + the LSP builtin type-name list at `src/lsp/world.rs:182`                                                                                                                | Three tables                             | A type name exists in one table but is not recognized by another, compared according to each table’s semantic mapping rules; **since WBS 6.1.2, T2.5, the “deleted-spelling sentinel,” also exists: adding back any of the 22 deleted spellings is an error**                                                                                          | Two-way reconciliation with `type-tables --fix`                                                                     |
| **T3: builtin type names consistent with RFC-011**                | The canonical-name set from `from_builtin_name` + the type table in `docs/src/rfc/accepted/011-generic-type-system.md`                                                                                                                                                 | Two tables                               | Code ranges or names do not match the registry                                                                                                                                                                                                                                                                                                         | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --fix`                                                   |
| **T4: exhaustiveness of `Type → MonoType`**                       | The list of `Type` variants vs the match arms in `types/lower.rs`                                                                                                                                                                                                      | Two tables                               | A variant has no corresponding arm, or an arm has no corresponding variant                                                                                                                                                                                                                                                                             | Add an arm / delete the arm                                                                                         |

**Implementation note for T4**: Rust’s exhaustive match already guarantees the “variant → arm”
direction at compile time. T4’s value is in the **reverse** direction—detecting stale arms in
`lower.rs` that point to deleted variants. Such code would fail to compile when the variant is
deleted, so T4 is effectively redundant. **T4 is therefore downgraded to a CI assertion
(`cargo build` succeeding is sufficient) and does not enter the `panic!` path in `build.rs`.** T1–T3
are the three checks that genuinely require text parsing.

**Two implementation difficulties in T1 and their mitigations**:

1. **Distinguishing construction sites from match arms** requires syntax analysis. The repository
   root `Cargo.toml` has **no `syn` dependency**, and neither does `tools/code-tables`. Mitigations:
   - **Preferred**: add `syn` to `tools/type-tables` only, leaving the root crate untouched. `syn`
     is standard practice, and the gate crate is independent of the compiler.
   - **Fallback**: reuse the existing `tools/extract_arm.py` line-level heuristic—the construction
     site is on the right-hand side of `=` or in an argument position, while a match arm is on the
     left-hand side of `=>`. Use the number of `=>` occurrences as an approximation.
   - **Conservative starting point**: when T1 first goes live, **report without failing**
     (`cargo:warning`), take a full baseline to confirm that the heuristic is correct, and only then
     change it to `panic!`. This is consistent with the handling of “CI gates may initially be red;
     establish a baseline first.”
2. **Reverse-bridge and AST→AST reconstruction sites must be classified separately**. Section 2.2
   records the two production `MonoType → ast::Type` reconstructions at `bytecode.rs:2353-2390` and
   `function.rs:507-529`; 2.2.2 records the AST→AST reconstruction in the two replacement functions
   in `function.rs` (15 variants / 28 sites, including 5 variants / 10 sites within the 13-variant
   list). If T1 mixes them with forward construction, `Int` / `Float` / `Bool` / `Char` / `Void` /
   `String` / `Bytes` and `Union` / `Option` / `Result` / `Sum` / `AssocType` are judged to “have
   construction sites” and exempted from deletion, directly contradicting the disposition table in
   5.2. T1’s report must separate the three construction categories, and **fallback exemptions apply
   only to individually registered entries**—currently only the parser error fallback for `Void` at
   `types.rs:836`—not to all wildcard arms.

**Gate coverage**: add a `type_tables::validate(root, &entries)` call to `build.rs`, immediately
after the existing block at `build.rs:19-37`, using the existing
`panic!("类型表校验失败（{} 项 error ...）", ...)` form.

## Detailed Design

### 6.1 Impact on the Type System

| Affected area                                                | Assessment                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Type inference (`inference/`)                                | **No impact**. Inference consumes `MonoType` throughout and does not interact with `ast::Type` variants                                                                                                                                                                                                     |
| Solver (`types/solver.rs`)                                   | **No impact**. `pub fn unify` (`solver.rs:411`), `substitute` in `types/substitute.rs:112`, and `substitute` in `types/mono.rs:649` all operate on `MonoType`                                                                                                                                               |
| Constant evaluation (`types/eval/`)                          | **No impact**. `eval/const_eval.rs` (1,128 lines), `eval/dependent_types.rs` (`check_structural_termination` at `478`), and `unify` in `eval/reducer.rs:572` all operate on `MonoType`                                                                                                                      |
| Monomorphization (`middle/passes/mono/`)                     | **Changes required**. Eight matches in `mono/function.rs:358`, `362`, `451`, `456`, `554`, `558`, `618`, and `623` handle `AstType::NamedStruct` / `AstType::AssocType`. If `AssocType` is deleted, all 8 must be updated. **`function.rs:507-529` (`mono_to_ast_type`) must also change** (see #47 in 6.3) |
| IR construction (`middle/core/ir_gen.rs`)                    | **Changes required**. The `ast::Type::NamedStruct` matches at `ir_gen.rs:1266` and `1399`; `ir_gen.rs:1380` contains `let params: Vec<MonoType> = signature_params` (another consumer of `signature_params`; this must be verified as part of 5.6)                                                          |
| Bytecode serialization (`middle/passes/codegen/bytecode.rs`) | **Changes required**. `type_id_to_monotype` at `codegen/bytecode.rs:492-495`, the encoding loop at `351-352`, and assembly at `632`                                                                                                                                                                         |
| Interpreter (`backends/interpreter/`)                        | **Changes required**. `image.rs:44` changes `type_table: Vec<ir::Type>` to `Vec<MonoType>`; `repl/eval.rs:369-370` formats type names by `type_table` index, and the change in element type affects the `{:?}` output format                                                                                |
| RFC-027 compile-time types                                   | **Changes required**. `Type::ConstExpr` is retained but marked **compile-time only**; it disappears after `Type → MonoType` in `types/lower.rs` and is never visible to the IR                                                                                                                              |

**IR-layer type discipline (to be inherited by 04-ssa)**: after convergence, IR type annotations may
use only the **resolved subset** of `MonoType`. `Type::ConstExpr` / `Type::Literal` /
`Type::MetaType` / `Type::Paren` / `Type::NamedParen` are **compile-time-only variants** and must
not appear in `BytecodeFunction`, instruction operands, or `type_table`. The first-layer validator
in 07 must enforce this through its “type consistency” invariant. `ast::BinOp::Assign` /
`ast::UnOp::Deref` (2.5) are analogous expression-side forms—they may occur inside `ConstExpr`, so
they likewise must not descend into IR type annotations.

### 6.2 Runtime Behavior

**This document does not change any runtime behavior.** The basis is:

1. The 13 deleted variants have no construction sites in the **parser-forward** direction (the one
   forward site for `Void` is an error fallback, registered as an individual exemption) → the parser
   does not produce them; the reverse bridge and AST→AST reconstruction are handled through #28 /
   #47 in 6.3 and the 10 sites in 6.3.2.
2. `ir::Type` is an alias of `ast::Type` → changing the carrier back to `MonoType` does not change
   the set of types, only the **carrier**.
3. The **loss** in `From<MonoType> for IrType` (8 variants → `Void`) no longer occurs after the
   change—this **may** change the behavior of some programs that previously lost types. **This is
   the only direction in which behavior can change, and it is a fix rather than a regression**, but
   corpus differencing in 07’s third tier must confirm that there is no regression.

**Semantic changes requiring particular attention**:

| Location                | Current state                                                                                 | After convergence                            |
| ----------------------- | --------------------------------------------------------------------------------------------- | -------------------------------------------- |
| `bytecode.rs:2371-2376` | `Generic` other than `String`/`Bytes`/`Tuple` → `IrType::Void`                                | Retain the real `MonoType::Generic`          |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | Retain the real `MonoType`                   |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` fallback                                                                  | **Delete**—an exhaustive match is sufficient |

> **Design judgment**: the fact that all 8 variants currently collapse to `Void` means that **every
> code path relying on signature bytecode—the interpreter’s arity/type checks, serialized formats,
> etc.—currently sees `Void` everywhere**. After convergence, these locations will see the real
> types. **This is the only possible source of behavioral change in this document, and its direction
> is “restoring correct information.”** If corpus differencing reveals a difference, it should be
> treated as **exposing an existing defect**, not as a regression introduced by this document.
> Method: construct a `.yx` program containing a `Ref` parameter, run `dump_bytecode` before and
> after the change, and compare the results.

### 6.3 Compiler Change List

| #   | File                                                                   | Line                                   | Change                                                                                                                                                                                                                                           | Category                 |
| --- | ---------------------------------------------------------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------ |
| 1   | `src/frontend/core/parser/ast.rs`                                      | `435-438`, `448-449`, `455-456`, `472` | Delete the `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` variants                                                                                                                                          | Pure deletion            |
| 2   | `src/frontend/core/parser/ast.rs`                                      | `428-431`                              | Add a `kind: NameKind` field to `Name`                                                                                                                                                                                                           | Data flow                |
| 3   | `src/frontend/core/parser/ast.rs`                                      | `37-43`                                | Delete `Expr::FnDef`                                                                                                                                                                                                                             | Pure deletion            |
| 4   | `src/frontend/core/parser/ast.rs`                                      | `244-245`                              | Delete `Assign.signature_params`                                                                                                                                                                                                                 | Pure deletion            |
| 5   | `src/frontend/core/parser/ast.rs`                                      | `1026`                                 | Delete the `Expr::FnDef` or-pattern                                                                                                                                                                                                              | Follows #3               |
| 6   | `src/frontend/core/parser/ast.rs`                                      | `519-532`                              | Change the `NamedParen` comment to “the syntax provides the binder name; semantic interpretation is performed in `sema/`”                                                                                                                        | Documentation            |
| 7   | `src/frontend/core/parser/ast.rs`                                      | After `795-814`                        | Add `pub enum NameKind`                                                                                                                                                                                                                          | New                      |
| 8   | `src/frontend/core/parser/probe.rs`                                    | New                                    | Add `trait TypeEnvProbe` + `NullProbe`                                                                                                                                                                                                           | New                      |
| 9   | `src/frontend/core/parser/parser_state.rs`                             | `46-58`                                | Add the `probe` field and accessor                                                                                                                                                                                                               | Data flow                |
| 10  | `src/frontend/core/parser/statements/types.rs`                         | `162-165`                              | Determine `NameKind` through `state.probe()`                                                                                                                                                                                                     | Data flow                |
| 11  | `src/frontend/core/parser/statements/types.rs`                         | `392-396`                              | Update the comment (`Result`/`Option` use `Generic`, already consistent with reality; only add an explanation of the variant deletion)                                                                                                           | Documentation            |
| 12  | `src/frontend/core/parser/statements/types.rs`                         | `836`                                  | Delete the `_ => (Vec::new(), Type::Void)` fallback; replace it with an explicit error return                                                                                                                                                    | Behavior                 |
| 13  | `src/frontend/core/parser/statements/declarations.rs`                  | `498-499`                              | Delete `n == "Terminates"`; change to `state.probe().is_predicate(n)`                                                                                                                                                                            | Reverse dependency       |
| 14  | `src/frontend/core/parser/statements/declarations.rs`                  | `508-511`                              | Delete the `typecheck::operator_interfaces::spec()` call                                                                                                                                                                                         | Reverse dependency       |
| 15  | `src/frontend/core/parser/statements/declarations.rs`                  | `472`                                  | Delete the `signature_params` argument                                                                                                                                                                                                           | Follows #4               |
| 16  | `src/frontend/core/parser/pratt/mod.rs`                                | `47`                                   | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                     | Follows #3               |
| 17  | `src/frontend/core/parser/pratt/led.rs`                                | `466`                                  | Retain (this is the parameter-list retrieval point; change it to read the Lambda directly under 5.5)                                                                                                                                             | Follows #3               |
| 18  | `src/frontend/core/types/mono.rs`                                      | `699-708`, `743-747`, `770-771`, `814` | Delete the 13 match arms, one for each deleted variant                                                                                                                                                                                           | Follows #1               |
| 19  | `src/frontend/core/types/mono.rs`                                      | `620-642`                              | Remove synonyms (capitalization aliases, `i64`-style abbreviations, `"()"`)                                                                                                                                                                      | Convergence              |
| 20  | `src/frontend/core/types/mono.rs`                                      | `627`                                  | Retain the `DateTime` alias                                                                                                                                                                                                                      | Retain                   |
| 21  | `src/frontend/core/types/lower.rs`                                     | New                                    | Move `From<Type> for MonoType` (currently at `mono.rs:692-830`) here as the sole conversion point                                                                                                                                                | New                      |
| 22  | `src/frontend/core/lexer/state.rs`                                     | Keyword table                          | Register `i64`/`int` and similar spellings as equivalent forms of `Int`                                                                                                                                                                          | Convergence              |
| 23  | `src/middle/core/ir.rs`                                                | `3`                                    | Delete `pub use ... ast::Type`                                                                                                                                                                                                                   | Pure deletion            |
| 24  | `src/middle/core/ir.rs`                                                | `678`                                  | Keep `FunctionCode.params` as `Vec<MonoType>` (already true)                                                                                                                                                                                     | —                        |
| 25  | `src/middle/core/bytecode.rs`                                          | `812`, `814`                           | Change `BytecodeFunction.params` / `return_type` to `MonoType`                                                                                                                                                                                   | Convergence              |
| 26  | `src/middle/core/bytecode.rs`                                          | `854`                                  | Change `BytecodeModule.type_table` to `Vec<MonoType>`                                                                                                                                                                                            | Convergence              |
| 27  | `src/middle/core/bytecode.rs`                                          | `2339`                                 | Change the `type_table` conversion to `.collect()` (the elements now have the same type)                                                                                                                                                         | Follows #26              |
| 28  | `src/middle/core/bytecode.rs`                                          | `2352-2390`                            | **Delete** the entire `From<MonoType> for IrType` impl                                                                                                                                                                                           | Pure deletion            |
| 29  | `src/backends/interpreter/image.rs`                                    | `44`                                   | Change `type_table` to `Vec<MonoType>`                                                                                                                                                                                                           | Convergence              |
| 30  | `src/middle/passes/mono/function.rs`                                   | `358`-`623` (8 sites)                  | Handle the `AstType::AssocType` arms according to the gate conclusion                                                                                                                                                                            | To be determined         |
| 31  | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | New / after `build.rs:19-37`           | Add type-table gates T1–T3                                                                                                                                                                                                                       | Gate                     |
| 32  | `spawn/placement.rs`                                                   | `122`                                  | Delete the `Expr::FnDef` arm                                                                                                                                                                                                                     | Follows #3               |
| 33  | `spawn/analysis.rs`                                                    | `912`                                  | As above                                                                                                                                                                                                                                         | Follows #3               |
| 34  | `formatter/handlers/expr.rs`                                           | `43`                                   | As above                                                                                                                                                                                                                                         | Follows #3               |
| 35  | `orchestrator.rs`                                                      | `1492`                                 | As above; obtain the name from `Assign.target`                                                                                                                                                                                                   | Follows #3               |
| 36  | `ir_gen.rs`                                                            | `4325`, `5097`                         | As above                                                                                                                                                                                                                                         | Follows #3               |
| 37  | `typecheck/checker.rs`                                                 | `1642`                                 | As above                                                                                                                                                                                                                                         | Follows #3               |
| 38  | `typecheck/checker/semantic_tokens.rs`                                 | `1416`                                 | As above                                                                                                                                                                                                                                         | Follows #3               |
| 39  | `typecheck/inference/expressions.rs`                                   | `3374`                                 | As above                                                                                                                                                                                                                                         | Follows #3               |
| 40  | `typecheck/inference/existential.rs`                                   | `55`                                   | As above                                                                                                                                                                                                                                         | Follows #3               |
| 41  | `typecheck/passes/dead_code.rs`                                        | `305`                                  | As above                                                                                                                                                                                                                                         | Follows #3               |
| 42  | `typecheck/layers/ownership.rs`                                        | `1207`                                 | As above                                                                                                                                                                                                                                         | Follows #3               |
| 43  | `typecheck/layers/termination.rs`                                      | `752`, `2303`                          | As above + comment update                                                                                                                                                                                                                        | Follows #3               |
| 44  | `parser/tests/ast.rs`                                                  | `837-847`                              | Delete the `Expr::FnDef` test                                                                                                                                                                                                                    | Test                     |
| 45  | `typecheck/tests/checker.rs`                                           | `109-111`                              | Rewrite as Lambda + Assign                                                                                                                                                                                                                       | Test                     |
| 46  | `types/tests/mono.rs`                                                  | `27-40`, `97`, `119-145`, `221`        | Delete the lower assertions for the 11 dead variants                                                                                                                                                                                             | Test                     |
| 47  | `src/middle/passes/mono/function.rs`                                   | `507-529`                              | Delete the `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` branches from `mono_to_ast_type`; use `AstType::Name` + `mono.type_name()` uniformly. **Must be processed in the same batch as #1**                                      | Convergence              |
| 48  | `src/frontend/core/parser/ast.rs`                                      | `891`, `930`                           | Add `probe: &dyn TypeEnvProbe` to the `extract_generic_param_names` signature; delete the `typecheck::operator_interfaces::spec()` call and use `probe.is_constraint(name)` instead. Callers, including `parser/tests/ast.rs`, pass `&NullProbe` | Reverse dependency       |
| 49  | `src/frontend/core/parser/statements/declarations.rs`                  | `42-80`                                | Delete this file’s `name_used_as_type`; call the consolidated version in `ast.rs`; change the `is_predicate` short-circuit to read `NameKind::Predicate`                                                                                         | Duplicate implementation |
| 50  | `src/frontend/core/parser/ast.rs`                                      | `912`                                  | Connect the `CONST_PARAM_TYPES` check through `NameKind::Builtin` (see 5.3)                                                                                                                                                                      | Convergence              |

> **The gate must itself be gated**: `tools/type-tables` and `code-tables` also require `mod`
> declaration wiring (the repository has already learned this lesson from 8 orphan test trees).
> Register the new crate in the root `Cargo.toml` at the same time.

### 6.4 Backward-Compatibility Strategy

**Constraint**: `tests/yaoxiang/` with 293 single-file `.yx` corpus files + the multi-file corpus
layer established in P2 (`tests/yaoxiang-multifile/`) + `src/std/tests/*.yx` + the standard library
**must all remain behaviorally unchanged** (identical diagnostic codes and messages; order may be
normalized).

| Risk                                                            | Assessment                                                                                                                                                                                                                                                                                                         | Mitigation                                                                                                                                                                                                                       |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Source code uses type annotations such as `i64` / `int` / `f64` | After normalization to `Int`, `from_builtin_name` recognizes only canonical names                                                                                                                                                                                                                                  | **Measured** (end of 5.3): zero noncanonical names in actual type positions → 6.1.2 directly deletes the 22 spellings and **does not establish a lexer alias table**; the corpus diagnostic set is unchanged                     |
| Source code uses `DateTime`                                     | Retain only the capitalized temporary compatibility alias (`mono.rs:883`); it is not a builtin name                                                                                                                                                                                                                | Migrating it out of the builtin-name table belongs to P10 S11 (issue #513); until then, `DateTime` and `Int` are synonymous, with no behavior change                                                                             |
| `("()")` appears in a type position                             | It is not a product of the `Name` form, but the type spelling of `Void` (the standard-library signature string uses `() -> Void`)                                                                                                                                                                                  | **Retain**: `()` shares an arm with `Void` and is one of the finalized 15 names                                                                                                                                                  |
| `dump_bytecode` output changes                                  | The element type of `type_table` changes from `ir::Type` to `MonoType`, producing a different `{:?}` format                                                                                                                                                                                                        | **Permitted by C3** (07 specifies that `dump_bytecode` is compared only during C1/C2). Record the output diff in stage 2 for manual review                                                                                       |
| Behavior changes after the lossy bridge is fixed                | See 6.2                                                                                                                                                                                                                                                                                                            | Corpus differencing + targeted `dump_bytecode` comparison                                                                                                                                                                        |
| The result of `mono_to_ast_type` changes after rewriting        | `function.rs:507-529` is the exit point for monomorphized type substitution (`substitute_type_in_ast:532-537` converts the substituted `MonoType` back to `AstType`). After deleting the variants, using the `_` arm can change a result from `AstType::Int(64)` to a form such as `AstType::Name { name: "i64" }` | Targeted comparison in stage 3: select corpus files containing generic substitution and compare the `type_name()` of the monomorphized artifacts before and after the change. **Do not assume that `{:?}` output is equivalent** |
| `.yx` source code itself                                        | **Zero changes**                                                                                                                                                                                                                                                                                                   | This document does not require changes to any corpus or standard-library source file. This is a hard acceptance criterion                                                                                                        |

**Hard acceptance condition** (**criterion revised on 2026-10-27, following decision ② from
2026-10-26**): `git diff --stat tests/yaoxiang tests/baselines src/std/` must be empty. **Directly
driven test files** under `tests/` may be modified when mechanically required, but each modification
must be registered with its **file / line count / rationale**, and **tests must not be deleted or
assertions weakened**. The original criterion (`git diff --stat tests/ src/std/`) applied a blanket
zero-difference requirement to all of `tests/`; this revision **relaxes the rule by one level for
directly driven test files only**. The zero-difference requirements for the corpus layer
(`tests/yaoxiang`), baseline layer (`tests/baselines`), and standard library (`src/std/`) remain
unchanged. **Exception**: the behavior-fix batch in 6.8.2 is a separate round and requires a
separate decision.

## Implementation Essentials

All criteria use 07’s **C3: identical diagnostic codes and messages, with order normalized**. The
following is the stage breakdown for this workstream; see RFC-039 for the global cross-document
ordering.

| Stage                                         | Content                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Acceptance criterion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Rollback point                         |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| **0. Gates first**                            | Create `tools/type-tables` and implement T1/T2/T3. The first version of T1 **reports without failing**. Take a full baseline against the **unmodified** code                                                                                                                                                                                                                                                                                                                                                                                         | The T1 report accurately lists the 13 variants with **zero forward construction in the parser**, including the characterization of `AssocType`: 0 forward / 2 AST→AST, with D8 unchanged, and **separately reports the reverse-bridge reconstruction sites from 2.2 and the AST→AST reconstruction sites from 2.2.2**. The 13 parser-forward zero-construction variants and the 1 globally zero-construction variant (`Enum` only) are reported separately. T2/T3 are green or provide a repairable list | No code changes; no rollback required  |
| **1. Name normalization**                     | **Completed (WBS 6.1.2, 2026-10-27)**: `from_builtin_name` (`mono.rs:839-886`) was consolidated to 15 names + the temporary `DateTime` alias, deleting 22 implemented spellings. The 7 lowercase copies in `add_builtin_types`, the `Uint` / `Int128` entries in the LSP list, and the `Uint` arm in `sizeof` were cleaned up in the same batch. **No lexer alias table was established** (corpus measurement found zero noncanonical names). **Simultaneously verify whether `classify_generic_params` uses `signature_params` or `Lambda.params`** | C3: the full corpus diagnostic set is **identical entry by entry** to the stage 0 baseline                                                                                                                                                                                                                                                                                                                                                                                                               | Separate commit; can be reverted       |
| **2. Bytecode type unification**              | Change `bytecode.rs:812`/`814`/`854` and `image.rs:44` to `MonoType`; delete `From<MonoType> for IrType` (`bytecode.rs:2352-2390`)                                                                                                                                                                                                                                                                                                                                                                                                                   | C3 + targeted `dump_bytecode` comparison for every row in the 6.2 table                                                                                                                                                                                                                                                                                                                                                                                                                                  | Separate commit; can be reverted       |
| **3. Delete dead variants**                   | Delete the 13 variants in the 5.2 table; remove the `mono.rs:699-708` and related match arms and the `types.rs:836` fallback; **process `function.rs:507-529` (#47) and the 10 AST→AST sites (6.3.2) in the same batch**; update `types/tests/mono.rs` in parallel                                                                                                                                                                                                                                                                                   | C3 + `cargo build` fully green (exhaustive matches force every arm to be completed) + targeted comparison of generic substitution (last row of 6.4)                                                                                                                                                                                                                                                                                                                                                      | Separate commit; can be reverted       |
| **4. Parser data flow**                       | Add `probe.rs`; change `parser_state.rs:46-58`, `declarations.rs:498-511`, and `ast.rs:930` (#48); determine `NameKind` at `types.rs:162-165`; consolidate the two `name_used_as_type*` implementations (#49)                                                                                                                                                                                                                                                                                                                                        | C3 + **new assertion**: `grep 'typecheck' src/frontend/core/parser/` must return 0                                                                                                                                                                                                                                                                                                                                                                                                                       | Separate commit; can be reverted       |
| **5. Dead AST variants and dedicated fields** | Delete `Expr::FnDef` (16 sites) + `Assign.signature_params`; migrate `NamedParen` semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                          | C3 + all 18 modules in `tests/integration/` green                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Two commits (FnDef / signature_params) |
| **6. Make gates strict**                      | Change T1 from warning to `panic!`; downgrade T4 to a CI assertion                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | `cargo build` **must fail** when a zero-construction variant is deliberately introduced (red test first)                                                                                                                                                                                                                                                                                                                                                                                                 | Separate commit                        |

**Dependency order**: 0 → 1 → 2 → 3 → 4 → 5 → 6. **Stages 3 and 5 must follow stage
2**—`bytecode.rs:2372-2373` consumes `IrType::String` / `IrType::Bytes`, so deleting the variants
first would break compilation. Stage 3’s #47 must be processed in the same batch as #1; otherwise,
`mono_to_ast_type` would silently change behavior.

**Dependency on downstream work**: after all stages in this document are complete,
[04-ssa.md](04-ssa.md) may begin. At that point, the element types of `BytecodeFunction.params`,
`FunctionCode.params`, and `type_table` are consistent, giving SSA type annotations a single
destination.

### Mandatory Validation for Each Stage

| Stage | Required action                                                                                                                                                  |
| ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 0     | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`; manually verify the T1 report (**including the reverse-bridge reconstruction-site split**) |
| 1–5   | `cargo test`; full-corpus differencing for `tests/yaoxiang/` (07’s third tier); compare C3 criteria entry by entry                                               |
| 2     | Additionally: construct one `.yx` sample for every variant in the 6.2 table and compare `dump_bytecode` output before and after the change                       |
| 3     | Additionally: for corpus files containing generic substitution, compare the `type_name()` of the monomorphized artifacts before and after the change             |
| 4     | Additionally: `rg 'typecheck' src/frontend/core/parser/` must return 0                                                                                           |
| 6     | Additionally: deliberately write a zero-construction variant and confirm that `cargo build` fails (red→green)                                                    |

## Key Decisions and Rationale

### Decisions

| Decision                     | Resolution                                                                                                         | Rationale                                                                                                                                                                                                                                      |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Target representation shape  | `Type` as the sole type structure + `MonoType` as the working representation; delete the `ir::Type` alias          | The alias at `ir.rs:3` directly causes the mismatch between the `BytecodeFunction.params` and `FunctionCode.params` field types (2.1). Once the alias is deleted, both field types are unified and the `From` bridge loses its reason to exist |
| Reject “unify on `MonoType`” | Not adopted                                                                                                        | It would introduce an L2→L3 reverse dependency in violation of the RFC-039 layering, and `MonoType` would need `Span` / `ConstExpr` added back. See the full argument in 5.1                                                                   |
| Do not delete `NamedParen`   | Retain it as a pure syntax node; migrate binder semantics to `sema/`                                               | `ast.rs:519-532` records that deletion would reintroduce a fixed correctness defect: `(r: P(m))` would silently treat the undeclared `m` as a binder                                                                                           |
| Directory rename             | **Do it** (D1), as a pure relocation batch at the end of P5/P6; this document’s paths reflect the pre-rename state | 2026-10-03 decision (ChenXu233); construction method clarified on 2026-10-04                                                                                                                                                                   |

### Rejected Alternatives

None of the four alternative approaches was adopted: **retaining all three representations and
adding only a conversion layer** does not address any root cause, while `ir::Type` remains a
non-independent type and the dead variants, synonym table, and reverse dependency all remain;
**using a macro to generate the synonym table** treats a symptom rather than the cause—once
`Type::Int(usize)` is deleted, the macro has no reason to exist, and consistency is better
guaranteed by gate T2 than by macro expansion correctness; **deleting only dead variants without
unifying the representation** has the lowest cost but leaves the problem in place, with no gate
preventing the next person from adding another `Type::Int128(usize)`—if only one commit can be
delivered, it must explicitly be recorded as a subset of this alternative, and the T1 gate must be
delivered with it; **unifying on `MonoType`** is addressed in 5.1.

### Benefits

- **Eliminates the maintenance burden at 14 locations**. The 12 consumption sites and 2 exhaustive
  arms for `Expr::FnDef` form a never-executed parallel path; deleting it removes one branch each
  from `ir_gen.rs`, `ownership.rs`, and `termination.rs`.
- **Eliminates one diverged duplicate implementation**. The two copies of `name_used_as_type` (2.4)
  already differ in recursion over definition-body fields; consolidation leaves only one.
- **After the lossy bridge disappears, `IrType::Void` is no longer a “legitimate type value.”** This
  is a prerequisite for the first-layer validator in 07 to make its “type consistency” invariant
  genuinely pass.
- **The gates make regression impossible**. T1 turns “adding a zero-construction variant” from a
  silently accepted condition into a build failure—the existing `build.rs` mechanism proves this
  path is viable.
- **Zero source-code changes**. The 293 `.yx` corpus files and the standard library require no
  modifications, which is the fundamental reason C3 can hold.

## Known Limitations and Risks

### Risks

- **Stage 2 may change behavior**. After the lossy bridge is fixed, the 8 `MonoType` variants expose
  their real types instead of `Void`. The direction is a fix, but it changes paths that depend on
  signature bytecode. This is the largest uncertainty in this document.
- **The characterization of `AssocType` depends on gate accuracy**. If T1 uses a line-level
  heuristic (counting `=>`) instead of `syn`, it may misclassify the variant. This is why stage 0
  requires manual verification.
- **If T1 misses reverse bridges, its conclusions conflict with the disposition table**. Section 2.2
  proves that 7 “dead variants” have production reconstruction sites in the reverse bridge. If T1
  does not separate construction directions, these 7 are judged to “have construction sites” and
  exempted from deletion—the gate would obstruct the deletion work rather than protect it. | **2.
  Deleting `signature_params` carries semantic risk**. It carries the **unfiltered** parameter list,
  whereas `Lambda.params` is **filtered**. If `classify_generic_params` depends on the former, the
  refactor changes generic classification. **This must be verified first; equivalence must not be
  assumed** (including the `signature_params` consumer at `ir_gen.rs:1380`). | **3. Stage 3 affects
  `middle/passes/mono/function.rs`**. Besides the 8 `AssocType` matches, `mono_to_ast_type` at
  `507-529` is the exit point for monomorphized type substitution, and rewriting it may change the
  substitution result’s `type_name()` (last row of 6.4).
- **Noncanonical type names in the `.yx` corpus may be nonzero**. If the corpus extensively uses
  `i64` / `int`, directly deleting synonyms in stage 1 is infeasible and the design must fall back
  to “move them into a lexer alias table,” reducing the scope of `from_builtin_name` convergence.
  **Measured value: 0**—the 6.0 survey is recorded at the end of 5.3, so WBS 6.1.2 directly deletes
  the aliases without taking the alias-table route.
- **The gate may initially be red**. If T1 calls `panic!` immediately on its first deployment, it
  will block compilation. It must first report without failing and establish a baseline.

### Open Questions

> **All open questions formerly listed in this section have been resolved.** See the
> [RFC-039 decision register](../../rfc/accepted/039-compiler-architecture.md) (D1–D50) for the
> individual decisions. **This document contains no remaining undetermined items.**

## See Also

### Parent and Same-Batch Documents

- [RFC-039 Compiler Feature Routing Directory Design (Master Plan)](../../rfc/accepted/039-compiler-architecture.md)
  — overarching plan; four-layer model, criterion tiers, and routing table C (3
  type-representation-related entries belong to this document)
- [01-routing.md](01-routing.md) — feature routing tables A/B/C, dependency-direction rules, and
  target directory structure
- [04-ssa.md](04-ssa.md) — SSA conversion; this document’s prerequisite and downstream consumer
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — frontend paradigm; consolidation of the
  operator change surface
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C3 category definition, diagnostic-set
  comparison specification, and the first-layer IR validator’s type-consistency invariant

### Other RFCs

- [RFC-010 Unified Type Syntax](../../rfc/accepted/010-unified-type-syntax.md) — basis for the
  unified `Type::Generic` path
- [RFC-011 Generic Type System](../../rfc/accepted/011-generic-type-system.md) — builtin type table
  (comparison target for gate T3)
- [RFC-013 Error-Code Specification](../../rfc/accepted/013-error-code-specification.md) — example
  of a generation-time gate in `build.rs`
- [RFC-027 Compile-Time Evaluation and Types](../../rfc/accepted/027-compile-time-evaluation-types.md)
  — origin of `Type::NamedParen` and `Type::ConstExpr`

### Code Locations

- `src/frontend/core/parser/ast.rs:427-541` — definition of the 26 `ast::Type` variants
- `src/frontend/core/parser/ast.rs:519-540` — explanation of `Type::NamedParen` binder semantics
  (evidence that deleting it would reintroduce a defect)
- `src/frontend/core/parser/ast.rs:241-249` — the typechecker-specific
  `StmtKind::Assign.signature_params` field
- `src/frontend/core/parser/ast.rs:838-841` — `CONST_PARAM_TYPES`: a third copy of const generic
  parameter names
- `src/frontend/core/parser/ast.rs:843-885` — `name_used_as_type_in`: the duplicate of
  `declarations.rs:42` that has already diverged
- `src/frontend/core/parser/ast.rs:891`, `930` — `extract_generic_param_names` and its
  boundary-crossing call
- `src/frontend/core/parser/ast.rs:191-213`, `217-223` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — the parser represents every primitive
  type as `Type::Name`
- `src/frontend/core/parser/statements/types.rs:392-396` — `Result`/`Option` are not lowered into
  dedicated AST nodes
- `src/frontend/core/parser/statements/types.rs:836` — the sole forward construction site of
  `Type::Void` (error fallback)
- `src/frontend/core/parser/statements/declarations.rs:42-80` — the original `name_used_as_type`
- `src/frontend/core/parser/statements/declarations.rs:494-511` — hardcoded `"Terminates"` and the
  `typecheck::operator_interfaces::spec()` call
- `src/frontend/core/parser/pratt/nud.rs:505-514` — an expression position is represented as an
  empty-body `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:466` — the `expr_to_params` arm retrieves the parameter
  list
- `src/frontend/core/types/mono.rs:839-886` — the `from_builtin_name` builtin-name resolution table
  (15 names + the temporary `DateTime` alias; WBS 6.1.2)
- `src/frontend/core/types/mono.rs:692-830` — the `From<Type> for MonoType` implementation (12 of
  the 13 match arms are here)
- `src/frontend/core/types/mono.rs` — the `MonoType` definition (approximately 22 variants,
  beginning at `mono.rs:167`; 1,077-line file)
- `src/frontend/core/types/const_data.rs:234-258`, `321-330` — `const_data::BinOp` /
  `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — compile-time constant evaluation (1,128 lines)
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- `src/middle/core/ir.rs:3` — evidence that `ir::Type` is an alias of `ast::Type`
- `src/middle/core/bytecode.rs:808-814` — the `Vec<ir::Type>` fields of `BytecodeFunction`
- `src/middle/core/bytecode.rs:2352-2390` — the lossy `From<MonoType> for IrType` bridge
- `src/middle/passes/codegen/bytecode.rs:32`, `275-278` — the `Vec<MonoType>` fields of
  `FunctionCode`
- `src/middle/passes/mono/function.rs:507-529` — `mono_to_ast_type`: the reverse bridge that
  reconstructs 7 “dead variants”
- `src/backends/interpreter/image.rs:44` — the interpreter module’s `type_table`
- `build.rs:19-37` — the error-code registry’s build-time gate (the model for the type-table gate)
- `build.rs:39-54` — RFC-013 code-table consistency comparison
- `tools/code-tables/Cargo.toml:15-16` — the gate crate depends only on `serde_json` (the root crate
  has no `syn`; `tools/type-tables` introducing `syn` is the first exception)
