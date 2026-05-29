# Type Default Values and Builtin Bindings Implementation Plan

> **Status: Core Implemented** — Phase 1-4 core framework + legacy enhancements complete, all 1499 tests passing.

## Overview

This plan implements two core features of YaoXiang language, corresponding to RFC-004 (curried multi-position bindings) and RFC-010 (unified type syntax):

1. **Type Default Value Initialization** (RFC-010): Type fields support default values, construction is optional
2. **Builtin Bindings** (RFC-004): Bind methods directly within type definition bodies (referencing external functions or anonymous functions), supporting precise control of parameter bindings via position indices

### Core Syntax (RFC-010 Unified Model)

```yaoxiang
Point: Type = {
    x: Float = 0,                                    # field + default value
    y: Float = 0,                                    # field + default value
    distance = distance[0],                          # external function binding (RFC-004 position syntax)
    norm: ((p: Point) -> Float)[0] = ((p) => ...)    # anonymous function binding
}
```

### Core Data Structure Changes

```
Type definition body fields:
├── Field declaration: field: Type
├── Field default value: field: Type = expression          (RFC-010)
├── External function binding: field = function[position]  (RFC-004)
└── Anonymous function binding: field: ((params) -> Return)[position] = ((params) => body)  (RFC-004)
```

### Modules Involved

| Module | File | Responsibility |
|--------|------|----------------|
| AST | `ast.rs` | `StructField`, `TypeBodyBinding`, `BindingKind` |
| Parser | `declarations.rs` | `parse_struct_type()` - four field types parsing |
| Type System | `mono.rs` | `StructType.field_has_default` |
| Semantic Analysis | `checking/mod.rs` | `check_type_def`, `check_field_default`, `check_field_binding` |
| IR Generation | `ir_gen.rs` | `CreateStruct`, default value filling, binding method call forwarding |
| Bytecode | `bytecode.rs` | `CreateStruct` bytecode encoding/decoding |
| Runtime | `executor.rs` | `CreateStruct` execution |

---

## Implementation Steps

### Phase 1: Parser Enhancement ✅

#### 1.1 Extend Type Field AST

**Goal**: Add new field types to distinguish normal fields, default value fields, and binding fields

**Acceptance Criteria**:
- [x] Parsing `Point: Type = { x: Float = 0 }` generates correct AST
- [x] Parsing `Point: Type = { distance = distance[0] }` generates Binding node
- [x] Parsing `Point: Type = { distance: ((a, b) => Float)[0] = ((a, b) => a + b) }` generates AnonBinding node

**Implementation Notes**:
- `StructField` adds new `default: Option<Box<Expr>>` field
- New `TypeBodyBinding` struct and `BindingKind` enum (`External` / `Anonymous`)
- `Type::Struct` changes from tuple variant to struct variant `Type::Struct { fields, bindings }`
- `parse_struct_type()` completely rewritten, supports four field types parsing
- New helper functions: `parse_optional_binding_positions()`, `parse_binding_positions()`, `extract_fn_type_info()`

---

### Phase 2: Semantic Analysis Enhancement ✅

#### 2.1 Default Value Field Type Checking

**Goal**: Verify that the default value expression type matches the field type

**Acceptance Criteria**:
- [x] `x: Float = 0` passes type checking (supports Int → Float implicit numeric promotion)
- [x] `x: Float = "str"` reports type mismatch (String ≠ Float)
- [x] `x: Int = 1.0` reports error (Float cannot be assigned to Int, no reverse promotion)

**Implementation Notes**:
- `BodyChecker::check_type_def()` → iterates through struct fields and bindings for checking
- `check_field_default()` → type infers the default value expression and unifies with field type
  - Supports Int → Float implicit numeric promotion (complies with RFC-010 `x: Float = 0` usage)
  - Other type mismatches (e.g., String → Float, Float → Int) report `type_mismatch` error
- `StructType` adds `field_has_default: Vec<bool>` field, propagated at all type transformation points

#### 2.2 Binding Field Semantic Checking (RFC-004 Type Safety)

**Goal**: Verify that bound referenced functions exist, position indices are valid, and types match

**Acceptance Criteria**:
- [x] `distance = distance[0]` passes verification for `distance: (a: Point, b: Point) -> Float`
- [x] `distance = distance[5]` reports index out of bounds error for 2-argument function
- [x] `distance = distance[0]` reports type mismatch for `distance: (a: String, b: String)`
- [x] Position index list non-empty verification

**Implementation Notes**:
- `check_field_binding(type_name, binding, span)` receives type name for verification
- External binding verification flow (RFC-004 §Type Checking Rules):
  1. Position index list is non-empty
  2. Find the polymorphic type of the function, instantiate to monomorphic function type
  3. Verify each position index < function parameter count
  4. Verify that the parameter type at the binding position is compatible with the current type (via `unify`)
- When the function is not found in the current scope, deep checking is skipped (function may be in outer scope or defined later)

#### 2.3 Anonymous Function Binding Semantic Checking

**Goal**: Verify anonymous function binding parameter positions and types

**Acceptance Criteria**:
- [x] Position index non-empty verification
- [x] Position index within anonymous function parameter range
- [x] Bound position parameter type matches current type

**Implementation Notes**:
- Anonymous binding verification: position non-empty + position < parameter count + bound position parameter type unifies with type name

---

### Phase 3: Code Generation Enhancement ✅

#### 3.1 Default Value Initialization Code Generation

**Goal**: Generate default constructor and constructor calls with default value overrides

**Acceptance Criteria**:
- [x] `Point()` generates code that calls default values
- [x] `Point(1.0)` only overrides x, y uses default value
- [x] `Point(1.0, 2.0)` overrides all fields

**Implementation Notes**:
- New `CreateStruct` IR instruction (`ir.rs`) and bytecode instruction (`bytecode.rs`)
- New `Opcode::CreateStruct = 0x79`
- `generate_struct_constructor_ir()` rewritten: load all parameters → `CreateStruct` → `Ret`
- Default value filling at call site: in `generate_expr_ir` `Expr::Call` branch, detect struct constructor calls, generate default value expression IR for missing parameters
- `translate_create_struct()` translator + bytecode decoder implementation
- Interpreter allocates `HeapValue::Tuple` in `CreateStruct` and creates `RuntimeValue::Struct`

#### 3.2 Binding Method Code Generation (RFC-004 Parameter Reordering)

**Goal**: Generate correct function call forwarding for binding method calls

**Acceptance Criteria**:
- [x] `p1.distance(p2)` + `distance = distance[0]` → generates `distance(p1, p2)` call
- [x] `p1.distance(p2)` + `distance = distance[1]` → generates `distance(p2, p1)` call
- [x] Multi-position binding `transform = transform[0, 1]` forwards correctly

**Implementation Notes**:
- New `BindingInfo` struct (records original function name + binding position list)
- New `type_bindings: HashMap<String, HashMap<String, BindingInfo>>` (type name → method name → binding info)
- `register_type_bindings()` extracts binding info from `Type::Struct { bindings }` during constructor IR generation phase
- Method call IR generation enhancement (`Expr::Call` + `FieldAccess` branches):
  1. Derive object type name (via `get_expr_struct_type_name()`)
  2. Look up binding info for that type
  3. If binding exists: reorder parameters according to RFC-004 position rules
     - Create parameter slots of size `total_params = positions.len() + method_args.len()`
     - Put obj at binding position
     - Fill remaining positions with method parameters in order
  4. Call original function name (not method name)
- Anonymous function bindings use `type_name.__anon_method_name` naming convention

#### 3.3 Dynamic Field Index Resolution

**Goal**: `resolve_field_index()` dynamically finds field index from type information

**Acceptance Criteria**:
- [x] No longer relies on hardcoding (original x→0, y→1, z→2)
- [x] Precise lookup by field name from `struct_definitions`

**Implementation Notes**:
- `resolve_field_index(expr, field_name)` rewritten:
  1. Derive the struct type name of the expression via `get_expr_struct_type_name(expr)`
  2. Find the field list of that type from `struct_definitions`, match field name and return index
  3. Fallback: iterate through all struct definitions to find field name (when type inference is unavailable)
- New `get_expr_struct_type_name(expr)` helper method:
  - Variable: look up from `type_result.local_var_types`, `bindings`, `local_var_types`
  - Constructor call: `Point(...)` directly returns `"Point"`
- New `mono_type_to_struct_name(mono_type)` helper method:
  - `MonoType::TypeRef(name)` → `Some(name)`
  - `MonoType::Struct(st)` → `Some(st.name)`

---

### Phase 4: Runtime Support ✅

#### 4.1 Default Value Expression Evaluation

**Goal**: Runtime correctly evaluates default value expressions

**Acceptance Criteria**:
- [x] Simple literal defaults `0`, `"hello"` evaluate correctly
- [x] Expression defaults `x: Int = 1 + 2` evaluate correctly to 3

**Implementation Notes**:
- Default values are evaluated at call site IR generation phase (via `generate_expr_ir` processing default value expressions)
- Supports arbitrary expressions as defaults (literals, arithmetic expressions, function calls, etc.)
- Interpreter performs actual struct creation and field initialization via `CreateStruct` bytecode

---

## Test Plan

### Unit Tests

| Test Category | Test Content | Status |
|--------------|--------------|--------|
| Parser tests | Various field syntax parsing | ✅ |
| Type checking | Default value type matching (including Int→Float promotion) | ✅ |
| Type checking | Binding position validity (out of bounds, type matching) | ✅ |
| Type checking | Anonymous function binding position verification | ✅ |
| Code generation | Default value generation | ✅ |
| Code generation | Binding method call forwarding | ✅ |

### Integration Tests

| Test Case | Expected Result | Status |
|-----------|-----------------|--------|
| `Point: Type = { x: Float = 0, y: Float = 0 }` + `Point()` | Construction succeeds | ✅ |
| `Point(1.0, 2.0)` positional parameter construction | Fields correctly assigned | ✅ |
| `Point(1.0)` partial parameters + defaults | x=1.0, y=0 | ✅ |
| Binding method call (RFC-004 position reordering) | Parameters correctly forwarded | ✅ |

### Regression Tests

- [x] All 1464 lib tests pass
- [x] All 30 integration tests pass
- [x] All 5 runtime tests pass
- Existing type definition syntax unaffected
- Existing binding syntax unaffected
- Existing constructor call syntax unaffected

---

## Risks and Dependencies

### Dependencies

- **RFC-004** (curried multi-position bindings): `[position]` position binding syntax, parameter reordering rules
- **RFC-010** (unified type syntax): `name: Type = { ... }` unified model, field default value syntax

### Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Parsing ambiguity | `field = value` could be assignment or binding | Distinguish based on right side of `=` syntax (function reference + position vs Lambda) |
| Incomplete type inference | `resolve_field_index` fallback iteration may be imprecise | Prioritize using type checking results |

---

## Milestones

1. **M1**: ✅ Parser supports all field types (default value fields, external bindings, anonymous bindings)
2. **M2**: ✅ Semantic checking for field rules (default value type checking + numeric promotion, binding position out of bounds + type matching verification)
3. **M3**: ✅ Code generation supports default values (`CreateStruct` instruction, call site default value filling)
4. **M4**: ✅ Binding method code generation (RFC-004 parameter reordering, `BindingInfo` + `type_bindings` mapping)
5. **M5**: ✅ Dynamic field index resolution (`resolve_field_index` lookup from `struct_definitions`)

### Optional Future Enhancements

The following features are all implemented (test coverage in `binding_enhancements` test module):

| Feature | Corresponding RFC | Status | Description |
|---------|------------------|--------|-------------|
| Named parameter construction | RFC-010 | ✅ Implemented | `Point(x=1, y=2)` - parser detects `name=expr` pattern and separates to `named_args`; IR generator reorders parameters by field name |
| Negative index bindings | RFC-004 | ✅ Implemented | `func[-1]` position type changed to `Vec<i64>`, parser supports `Minus` + `IntLiteral` pattern |
| Default binding | RFC-004 | ✅ Implemented | `Type.method = function` (no position) generates `BindingKind::DefaultExternal`, IR generator defaults position to 0 |
| External binding statement | RFC-004 | ✅ Implemented | `Point.distance = distance[0]` parsed as `StmtKind::ExternalBindingStmt`, IR generator registers to `type_bindings` |
| Interface constraints | RFC-010 | ✅ Implemented | `StructType` adds `interfaces: Vec<String>` field, parser recognizes uppercase identifiers as interface constraints |
| Anonymous function IR generation | RFC-004 | ✅ Implemented | `generate_anon_binding_ir` method generates independent `FunctionIR`, registered to module-level `anon_function_irs` |