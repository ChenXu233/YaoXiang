# RFC-011 Generic System - Unimplemented Feature List

> **Created**: 2026-02-03
> **Last Updated**: 2026-02-04
> **Status**: In Progress
> **Based on RFC**: [RFC-011 Generic System Design](../accepted/011-generic-type-system.md)

## Abstract

This document records the implemented and unimplemented feature modules in the RFC-011 generic system design. Based on analysis of the compiler implementation, it identifies the current capability boundaries of the system and areas requiring improvement.

---

## Implementation Status Overview

| Phase | Feature Module | Status | Completion | Key Files |
|-------|---------------|--------|------------|-----------|
| Phase 1 | Basic Generics | ✅ Partial Implementation | 70% | `src/middle/passes/mono/mod.rs` |
| Phase 2 | Type Constraint System | ⚠️ Basic Structure | 30% | `src/frontend/type_level/` |
| Phase 3 | Associated Types | ⚠️ Basic Structure | 5% | `src/frontend/typecheck/gat/` |
| Phase 4 | Compile-time Generics | ⚠️ Basic Structure | 40% | `src/frontend/type_level/const_generics/` |
| Phase 5 | Conditional Types | ✅ Basic Implementation | 65% | `src/frontend/type_level/type_match.rs` |
| - | Function Overloading Specialization | ✅ Implemented | 75% | `src/frontend/typecheck/overload.rs` |
| - | Platform-specific Optimization | ⚠️ Basic Implementation | 50% | `src/middle/passes/mono/platform_specializer.rs` |
| - | Full DCE | ✅ Partial Implementation | 90% | `src/middle/passes/mono/` |

---

## Unimplemented Feature Details

### 1. Function Overloading Specialization Mechanism

#### 1.1 Feature Description

RFC-011 design uses **function overloading** for specialization:
```yaoxiang
# Concrete type specialization
sum: (arr: Array[Int]) -> Int = (arr) => {
    native_sum_int(arr.data, arr.length)
}

sum: (arr: Array[Float]) -> Float = (arr) => {
    simd_sum_float(arr.data, arr.length)
}

# Generic implementation (automatic selection)
sum: [T](arr: Array[T]) -> T = (arr) => { ... }
```

#### 1.2 Current Status

- ✅ Data structure supports overloading (`instance.rs`)
- ✅ Overload resolver module exists (`overload.rs`)
- ✅ Type environment supports overload candidate storage
- ✅ Function call overload resolution integration (`expressions.rs`)
- ⚠️ Generic fallback integration (incomplete)

#### 1.3 Required Implementation

```
src/frontend/typecheck/overload.rs              # ✅ Overload resolver (complete)
src/frontend/typecheck/mod.rs                  # ✅ Type environment extension (complete)
src/middle/passes/mono/instance.rs             # ✅ Instantiation ID extension (complete)
src/frontend/typecheck/inference/expressions.rs  # ✅ Overload resolution integration (complete)
src/frontend/typecheck/checking/mod.rs          # ✅ BodyChecker extension (complete)
```

#### 1.4 Acceptance Criteria

- [x] Can parse different type signatures of functions with the same name (data structure support)
- [x] Automatically selects best match based on actual argument types at call site (integrated)
- [x] Compilation error: ambiguous call or no matching definition (implemented)
- [x] Integration with generics system: generics as fallback (complete)

---

### 2. Platform-specific Optimization

#### 2.1 Feature Description

RFC-011 design supports platform specialization via predefined generic parameter `P` (without `#[cfg]`):
```yaoxiang
# Generic implementation (available on all platforms)
sum: [T: Add](arr: Array[T]) -> T = { ... }

# Platform specialization: P is a predefined generic parameter, representing current platform
sum: [P: X86_64](arr: Array[Float]) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: [P: AArch64](arr: Array[Float]) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

#### 2.2 Current Status

- ✅ `platform_info.rs` implemented (80%)
  - TargetPlatform: X86_64, AArch64, RiscV64, Arm, X86, Wasm32
  - PlatformDetector: detects from target triple/env variables
  - Predefined generic parameter `P` support

- ✅ `platform_specializer.rs` implemented (50%)
  - PlatformConstraint: `[P: X86_64]` constraint
  - PlatformSpecializer: platform specialization selection
  - Supports registration and selection of multi-platform specialization versions

- ❌ No `#[cfg]` attribute parsing (RFC design does not use this approach)
- ❌ Platform specialization integration with monomorphizer (pending)
- ❌ Platform-aware code generation (pending)

#### 2.3 Required Implementation

```
src/frontend/core/parser/attr.rs                    # Optional: attribute parsing (RFC design does not use #[cfg])
src/middle/passes/mono/platform_info.rs             # ✅ Implemented
src/middle/passes/mono/platform_specializer.rs      # ✅ Implemented
src/middle/passes/mono/mod.rs                       # Modification: integrate platform specialization
```

#### 2.4 Acceptance Criteria

- [x] Can detect target platform (X86_64, AArch64, etc.)
- [x] Predefined generic parameter `P` recognition
- [ ] Platform specialization correctly integrated with monomorphizer
- [ ] Only generate specialization code matching current platform
- [ ] Automatically select specialization version at compile time based on target platform

---

### 3. Dead Code Elimination (DCE) Complete Implementation

#### 3.1 Feature Description

RFC-011 designed multi-level DCE:

1. **Instantiation graph analysis**: Build generic instantiation dependency graph, perform reachability analysis from entry points
2. **Use-point analysis**: Only instantiate generics actually called
3. **Cross-module DCE**: Analyze inter-module dependencies, eliminate unused exports
4. **LLVM-level DCE**: Utilize LLVM's optimization passes

```rust
// Compiler internals: build generic instantiation dependency graph
struct InstantiationGraph {
    nodes: HashMap<InstanceKey, InstanceNode>,
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}
```

#### 3.2 Current Status

- ✅ Basic monomorphizer exists (`mono/mod.rs`)
- ✅ Basic on-demand specialization implemented
- ✅ Instantiation graph construction complete (`instantiation_graph.rs`)
- ✅ Complete reachability analysis complete (`reachability.rs`)
- ⚠️ Cross-module DCE basic implementation (needs production environment validation)
- ✅ Code bloat control complete (`dce.rs`)

#### 3.3 Required Implementation

```rust
// New modules
src/middle/passes/mono/instantiation_graph.rs      # Instantiation graph construction
src/middle/passes/mono/reachability.rs              # Reachability analysis
src/middle/passes/mono/cross_module_dce.rs          # Cross-module DCE
src/middle/passes/mono/code_bloat_control.rs        # Code bloat control
```

#### 3.4 Acceptance Criteria

- [x] Build complete instantiation dependency graph
- [x] Reachability analysis from main entry point
- [x] Eliminate unused generic instantiations
- [x] Cross-module dependency analysis (production environment validation)
- [x] Code bloat threshold control
- [x] Statistics output (detailed + JSON format)

---

### 4. Complete Trait System

#### 4.1 Feature Description

RFC-011 designed a type constraint system (similar to Rust Trait):

```yaoxiang
# Trait definition
type Clone = { clone: (Self) -> Self }
type Add = { add: (Self, Self) -> Self }

# Using constraints
clone: [T: Clone](value: T) -> T = value.clone()
combine: [T: Clone + Add](a: T, b: T) -> T = a.clone() + b
```

#### 4.2 Current Status

- ⚠️ Type-level computation module exists (`type_level/`)
- ⚠️ Basic `Some`/`None` wrappers implemented
- ❌ No Trait definition syntax parsing
- ❌ No Trait implementation verification
- ❌ No Trait inheritance/derivation
- ❌ Constraint solver incomplete

#### 4.3 Required Implementation

```
src/frontend/core/parser/trait_def.rs               # New: Trait definition parsing
src/frontend/typecheck/trait_resolution.rs          # New: Trait constraint solving
src/frontend/typecheck/trait_impl.rs                # New: Trait implementation checking
src/frontend/type_level/trait_bounds.rs             # New: Trait bounds representation
```

#### 4.4 Acceptance Criteria

- [ ] Can parse `type TraitName = { ... }` syntax
- [ ] Can parse `[T: Trait]` constraint syntax
- [ ] Verify if a type satisfies Trait constraints
- [ ] Support multiple constraints `[T: A + B]`
- [ ] Error messages indicate missing Trait implementations

---

### 5. Associated Types (GAT)

#### 5.1 Feature Description

```yaoxiang
# Associated type definition
type Iterator[T] = {
    Item: T,                           # Associated type
    next: (Self) -> Option[T],
    has_next: (Self) -> Bool,
}

# Using associated types
collect: [T, I: Iterator[T]](iter: I) -> List[T] = { ... }
```

#### 5.2 Current Status

- ❌ No associated type parsing
- ❌ No associated type constraint checking
- ❌ No GAT support

#### 5.3 Required Implementation

```
src/frontend/type_level/associated_types.rs         # New: associated type representation
src/frontend/typecheck/gat_check.rs                 # New: GAT type checking
```

#### 5.4 Acceptance Criteria

- [ ] Can parse Trait definitions with associated types
- [ ] Can use associated types as constraints
- [ ] Type checking correctly resolves associated types
- [ ] Support generic associated types

---

### 6. Compile-time Generics Complete Implementation

#### 6.1 Feature Description

```yaoxiang
# Compile-time constant parameters
type Array[T, N: Int] = { data: T[N] }

# Compile-time function: using literal type constraints
factorial: [n: Int](n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

# Compile-time computation (compiler computes at compile time)
SIZE: Int = factorial(5)  # 120
```

#### 6.2 Current Status

- ⚠️ `const_generics/` module exists
- ⚠️ Basic `GenericSize` representation
- ⚠️ Basic constant expression evaluation
- ❌ No literal type parameter parsing `[n: Int](n: n)`
- ❌ No compile-time function instantiation
- ❌ No compile-time dimension validation
- ⚠️ `static_assert` implemented via conditional types standard library (see 7. Conditional Types)

#### 6.3 Required Implementation

```
src/frontend/core/parser/literal_param.rs           # New: literal type parameter parsing
src/frontend/typecheck/const_eval.rs               # New: compile-time expression evaluation
src/middle/passes/mono/compile_time_monomorphization.rs  # New: compile-time generic specialization
```

#### 6.4 Acceptance Criteria

- [ ] Can parse `[n: Int](n: n)` literal type parameter syntax
- [ ] Can parse `[N: Int]` compile-time generic parameters
- [ ] Evaluate function calls with literal type parameters at compile time
- [ ] Support compile-time generic instantiation
- [ ] Note: `Assert` implemented via standard library using conditional types (see 7.4 Acceptance Criteria)

---

### 7. Conditional Types Complete Implementation

#### 7.1 Feature Description

```yaoxiang
# Type-level If
type If[C: Bool, T, E] = match C {
    True => T,
    False => E,
}

# Type family
type Add[A: Nat, B: Nat] = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}
```

#### 7.2 Current Status

- ✅ `type_families.rs` implemented (60%)
  - Bool type family: `True`, `False`
  - Nat type family: `Zero`, `Succ[N]`
  - Conditional types: `IsTrue`, `IsFalse`, `IsZero`, `IsSucc`
  - TypeFamily trait for unified handling

- ✅ `type_match.rs` implemented (70%)
  - MatchPattern: literal/constructor/tuple/wildcard patterns
  - PatternMatcher: pattern matching engine
  - MatchType: complete type matching
  - PatternBuilder: fluent API for pattern building

- ✅ `type_eval.rs` implemented (65%)
  - If condition evaluation: `If<True, Int, String> => Int`
  - Nat operations: `Add`, `Sub`, `Mul`, `Div`, `Mod`, `Eq`, `Lt`
  - Caching, cycle detection, dependency tracking
  - Conditional combinations: `And`, `Or`, `Not`

- ⚠️ `conditional_types.rs` exists (basic framework)

- ❌ Complete integration with type normalizer (pending)
- ❌ Standard library `Assert` implementation (pending)

#### 7.3 Required Implementation

```
src/frontend/type_level/type_match.rs               # ✅ Implemented
src/frontend/type_level/type_families.rs            # ✅ Implemented
src/frontend/typecheck/type_eval.rs                 # ✅ Implemented
src/frontend/type_level/evaluation/mod.rs          # Modification: integrate evaluator
```

#### 7.4 Acceptance Criteria

- [x] Support `If[C, T, E]` conditional types
- [x] Support Bool type family (True, False)
- [x] Support Nat type family (Zero, Succ)
- [x] Support type-level match expressions
- [x] Compile-time type computation (If, Nat operations)
- [ ] Complete integration with type normalizer
- [ ] Standard library `Assert` implementation (compile-time assertion)
  ```yaoxiang
  type Assert[C: Bool] = match C {
      True => Void,
      False => compile_error("Assertion failed"),
  }
  ```

---

### 8. Inline Optimization Combined with Specialization

#### 8.1 Feature Description

RFC-011 design integrates function overloading with inline optimization naturally:

```yaoxiang
sum: (arr: Array[Int]) -> Int = (arr) => {
    native_sum_int(arr.data, arr.length)
}

# Compiler automatically selects and inlines at usage
result = sum(int_arr)  # => native_sum_int(int_arr.data, int_arr.length)
```

#### 8.2 Current Status

- ❌ No specialization + inline coordination
- ❌ Optimizer lacks specialization awareness
- ❌ Inline decisions do not consider specialization

#### 8.3 Required Implementation

```
src/middle/optimizer/specialization_aware_inlining.rs  # New: specialization-aware inlining
src/middle/passes/opt/size_analysis.rs                 # New: function size analysis
```

#### 8.4 Acceptance Criteria

- [ ] Specialized code can be further inlined
- [ ] Small specialization bodies automatically inlined at call sites
- [ ] Generated code equivalent to hand-optimized version

---

## Priority Ranking (Updated 2026-02-04)

| Priority | Feature | Estimated Duration | Dependencies | Status |
|----------|---------|-------------------|--------------|--------|
| **P0** | Full DCE | 1 week | Basic monomorphizer | 90% - Finishing |
| **P1** | Function Overloading Specialization Integration | 2 weeks | Overload resolver | 75% - Enhancing |
| **P2** | Conditional Types Integration | 2 weeks | Type normalizer | 65% - Mid-term |
| **P3** | Platform Specialization Integration | 2 weeks | Monomorphizer | 50% - Mid-term |
| **P4** | Compile-time Generics Complete | 3 weeks | Phase 4 | 40% - Mid-term |
| **P5** | Complete Trait System | 4 weeks | Phase 2 | 10% - Long-term |
| **P6** | Associated Types | 4 weeks | Trait system | 5% - Long-term |
| **P7** | Specialization-aware Inlining | 2 weeks | P1 + Optimizer | 0% - Long-term |
| **P8** | Macro Substitution | 3 weeks | Generics + Trait | 0% - Long-term |

### Next Steps

**Short-term (1-2 weeks)**:
1. Complete DCE finishing work
2. Integrate conditional types into type normalizer
3. Integrate platform specialization into monomorphizer

**Mid-term (1 month)**:
1. Enhance function overloading and generics integration
2. Enhance compile-time generics (literal parameters)
3. Start Trait system basic implementation

**Long-term (2-3 months)**:
1. Associated Types (GAT)
2. Specialization-aware inlining
3. Macro substitution capability

---

## Implementation Suggestions

### Short-term Goals (1-2 months)

1. **Complete Basic DCE**
   - Build instantiation graph
   - Implement reachability analysis
   - This will eliminate most code bloat from unused code

2. **Implement Function Overloading Specialization**
   - This is the core feature of RFC-011
   - Supports subsequent specialization optimizations

### Mid-term Goals (3-4 months)

1. **Complete Trait System**
   - Supports generic constraints
   - Provides foundation for standard library

2. **Compile-time Generics**
   - Supports compile-time computation
   - Enables static array optimization
   - No `const` keyword needed

### Long-term Goals (5-6 months)

1. **Conditional Types**
   - Type-level programming
   - More powerful generic capabilities

2. **Platform Specialization**
   - SIMD optimization
   - Architecture-specific code

---

## Related File List

### Implemented Modules (Partial/Basic)

| File Path | Status | Description |
|-----------|--------|-------------|
| `src/middle/passes/mono/mod.rs` | ⚠️ 70% | Monomorphizer main body |
| `src/middle/passes/mono/function.rs` | ⚠️ 70% | Function monomorphization |
| `src/middle/passes/mono/type_mono.rs` | ⚠️ 50% | Type monomorphization |
| `src/middle/passes/mono/closure.rs` | ⚠️ 50% | Closure monomorphization |
| `src/middle/passes/mono/platform_info.rs` | ✅ 80% | Platform information detection |
| `src/middle/passes/mono/platform_specializer.rs` | ✅ 50% | Platform specializer |
| `src/frontend/type_level/mod.rs` | ⚠️ 40% | Type-level computation entry |
| `src/frontend/type_level/conditional_types.rs` | ⚠️ 35% | Conditional types framework |
| `src/frontend/type_level/const_generics/mod.rs` | ⚠️ 40% | Compile-time generics framework |
| `src/frontend/type_level/evaluation/compute.rs` | ⚠️ 30% | Type-level computation |
| `src/frontend/type_level/type_match.rs` | ✅ 70% | Type-level match |
| `src/frontend/type_level/type_families.rs` | ✅ 60% | Type families (Bool/Nat) |
| `src/frontend/typecheck/type_eval.rs` | ✅ 65% | Compile-time type evaluator |
| `src/frontend/typecheck/gat/mod.rs` | ⚠️ 5% | GAT basic structure |
| `src/frontend/typecheck/traits/mod.rs` | ⚠️ 10% | Trait basic structure |

### Modules Needing New/Enhanced Implementation

| File Path | Function | Status |
|-----------|----------|--------|
| `src/frontend/core/parser/overload.rs` | Function overloading parsing | Exists |
| `src/frontend/core/parser/trait_def.rs` | Trait definition parsing | ❌ Not implemented |
| `src/frontend/core/parser/literal_param.rs` | Literal type parameter parsing | ❌ Not implemented |
| `src/frontend/core/parser/attr.rs` | Attribute parsing (optional) | ❌ Not implemented |
| `src/frontend/typecheck/trait_resolution.rs` | Trait constraint solving | ⚠️ Partial |
| `src/frontend/typecheck/trait_impl.rs` | Trait implementation checking | ⚠️ Partial |
| `src/frontend/typecheck/const_eval.rs` | Compile-time expression evaluation | ⚠️ Partial |
| `src/frontend/type_level/associated_types.rs` | Associated types | ❌ Not implemented |
| `src/middle/passes/mono/instantiation_graph.rs` | Instantiation graph | Exists |
| `src/middle/passes/mono/reachability.rs` | Reachability analysis | Exists |
| `src/middle/passes/mono/cross_module_dce.rs` | Cross-module DCE | Exists |
| `src/middle/optimizer/specialization_aware_inlining.rs` | Specialization-aware inlining | ❌ Not implemented |

---

## Appendix: RFC-011 Design Review

### Core Feature List (Updated 2026-02-04)

| Feature | RFC Design | Current Implementation | Gap | Priority |
|---------|------------|----------------------|-----|----------|
| Basic generics `[T]` | ✅ | ✅ 70% | Needs enhancement | P1 |
| Type inference | ✅ | ⚠️ Basic | Needs expansion | P1 |
| Type constraints (Trait) | ✅ | ⚠️ 10% | Needs implementation | P2 |
| Associated types (GAT) | ✅ | ⚠️ 5% | Needs implementation | P4 |
| Compile-time generics | ✅ | ⚠️ 40% | Needs enhancement | P3 |
| Conditional types | ✅ | ✅ 65% | Conditional types framework complete | P2 |
| Function specialization | ✅ | ✅ 75% | Overloading mechanism complete | P1 |
| Platform specialization | ✅ | ⚠️ 50% | Basic structure complete | P2 |
| Full DCE | ✅ | ⚠️ 90% | Near completion | P0 |
| Macro substitution | ✅ | ❌ 0% | Needs implementation | P5 |
| Specialization-aware inlining | ✅ | ❌ 0% | Needs implementation | P5 |

> **Update Note (2026-02-04)**:
> - Conditional types from 35% → **65%**: `type_match.rs`, `type_families.rs`, `type_eval.rs` implemented
> - Platform specialization from 0% → **50%**: `platform_info.rs`, `platform_specializer.rs` implemented
> - Associated types from 0% → **5%**: `src/frontend/typecheck/gat/` basic structure created

### Dependency Graph

```
Basic generics ([T])
    │
    ├──> Type constraint system (Trait)
    │        │
    │        ├──> Associated types (GAT)
    │        │
    │        └──> Trait inheritance
    │
    ├──> Compile-time generics
    │        │
    │        ├──> Literal type parameters
    │        │
    │        └──> Compile-time computation
    │
    └──> Conditional types
             │
             └──> Type-level programming

Function overloading specialization ─────────────> Platform-specific optimization
                             │
                             └──> Specialization-aware inlining

Full DCE ──────────────────> Cross-module DCE
                             │
                             └──> Code bloat control
```