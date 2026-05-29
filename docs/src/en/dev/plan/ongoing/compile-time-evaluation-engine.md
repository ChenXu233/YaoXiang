# Compile-Time Evaluation (CTE) Engine and Hoare Logic Static Verification — Implementation Plan

> **Task**: Implement compile-time evaluation engine + Hoare logic static verification pipeline to support value-dependent types and compile-time dimension verification
> **Based on RFCs**: RFC-010 (Unified Type Syntax), RFC-011 (Generics/Value-Dependent Types), RFC-022 (Hoare Logic Static Verification)
> **Date**: 2026-05-10
> **Status**: In Design
> **Target Milestones**:
> - M1: Constant folding + purity analysis skeleton
> - M2: Pure function compile-time evaluation + termination checking
> - M3: Type-level computation (`If`/`Assert`/`match` type families)
> - M4: Hoare logic verification (`//!` specification parsing → VC generation → SMT integration)

---

## Abstract

YaoXiang's value-dependent types (RFC-011) require types to depend on compile-time-known values (e.g., `Vec(factorial(5))` → `Vec(120)`), while Hoare logic static verification (RFC-022) requires compile-time specification checking for pure functions. Both share the same core requirement: **safely execute/analyze pure functions at compile time**.

This plan proposes a **unified Compile-Time Evaluation (CTE) Engine**, abstracting purity analysis, termination checking, and expression evaluation into common infrastructure that serves two consumers: type-level evaluation and Hoare logic verification.

---

## Core Design Principles

1. **Reuse ownership system for purity analysis**: YaoXiang's `&mut` is the side-effect marker — borrow checking already tells us what will be modified
2. **Pure functions are compile-time evaluable**: No `const fn` keyword needed; the compiler infers purity automatically
3. **Termination proof = cornerstone of type safety**: Evaluation in type positions must prove termination (`decreases` specification), otherwise the type system becomes undecidable
4. **Partial evaluation over full evaluation**: Evaluate with whatever parameters are known; leave the rest for runtime
5. **Compile-time evaluation and Hoare logic share the interpreter**: The same expression evaluation core supports two types of consumers
6. **Dual-mode evaluation**: Concrete evaluation (known parameters → produce `CTValue`) and symbolic evaluation (unknown parameters → produce SMT expressions) share the same interpreter framework, differing only in the evaluation environment

---

## Architecture Overview

```
                           Source File + //! Specifications
                                   ↓
                           ┌──────────────┐
                           │   Parser      │
                           │ (recognizes //! comments)│
                           └──────┬───────┘
                                   ↓
                           ┌──────────────┐
                           │  Type Checker │
                           │ • collects specs    │
                           │ • discovers value deps│
                           └──────┬───────┘
                                   ↓
              ┌────────────────────┴────────────────────┐
              ↓                                         ↓
   ┌──────────────────────┐                ┌──────────────────────┐
   │   CTE Engine         │                │  Hoare Logic Verifier│
   │                      │                │                      │
   │  ┌────────────────┐  │                │  1. Collect //! specs  │
   │  │  Purity Analyzer│  │                │  2. Generate VCs      │
   │  │  (based on ownership)│  │                │  3. SMT solver (Z3)  │
   │  └───────┬────────┘  │                │  4. Counterexample report│
   │  ┌───────┴────────┐  │                └──────────┬───────────┘
   │  │  Termination Checker│  │                           │
   │  │  (decreases)    │  │                           ↓
   │  └───────┬────────┘  │                ┌──────────────────────┐
   │  ┌───────┴────────┐  │                │  Verification Result  │
   │  │  AST Interpreter│  │                │  • Pass → cache       │
   │  │                │  │                │  • Fail → block release│
   │  │ ┌────────────┐ │  │                └──────────────────────┘
   │  │ │ Concrete Eval│ │  │                       ↑
   │  │ │ env: all known│ │  │                       │
   │  │ │ → CTValue   │ │  │  Shared interpreter framework  │
   │  │ └────────────┘ │  │  (AST traversal/inlining/loop unrolling)  │
   │  │ ┌────────────┐ │  │                       │
   │  │ │ Symbolic Eval│─┼──┼───────────────────────┘
   │  │ │ env: partial known│ │  │  (consumed by Hoare logic)
   │  │ │ → SMTExpr   │ │  │
   │  │ └────────────┘ │  │
   │  └───────┬────────┘  │
   │          ↓           │
   │  Embed results in type/monomorphize│
   └──────────────────────┘
```

---

## 1. Compile-Time Values (CTValue)

The core data type for compile-time computation. Designed as a compiler-internal IR value, distinct from runtime values.

```rust
/// Compile-time evaluation result
enum CTValue {
    /// Integer (covers all compile-time uses of Bool)
    Int(i64),

    /// Floating-point number
    Float(f64),

    /// String (error messages, type names, etc.)
    String(SmolStr),

    /// Type reference — the core of type-level computation
    /// In YaoXiang, types themselves are "values" at the Type1 layer
    Type(TypeId),

    /// Heterogeneous tuple
    Tuple(Vec<CTValue>),

    /// Homogeneous array
    Array(Vec<CTValue>),

    /// Structured value
    Struct {
        type_id: TypeId,
        fields: HashMap<SmolStr, CTValue>,
    },

    /// Unevaluated function reference (preserved during partial evaluation)
    /// When all parameters are known, inline and evaluate; otherwise keep as runtime call
    Thunk {
        func: FunctionId,
        known_args: Vec<CTValue>,
        unknown_params: Vec<ParamId>,
    },
}
```

**Key design**: `CTValue::Type(TypeId)` makes types first-class compile-time values. For `If(C, T, E)`, C evaluates to `CTValue::Bool`, and T/E evaluate to `CTValue::Type`.

---

## 2. Subsystem 1: Purity Analyzer

### 2.1 Design Philosophy

Reuse YaoXiang's ownership system (RFC-009); side effects are naturally expressed by type signatures:

| Parameter Pattern | Meaning | Compile-Time Evaluable? |
|-------------------|---------|--------------------------|
| `x: T` (owned) | Takes ownership, freely modifiable | ✅ |
| `x: &T` (shared reference) | Read-only | ✅ |
| `x: &mut T` (exclusive reference) | Mutable | ⚠️ depends on T's origin |
| I/O calls | External side effects | ❌ |
| Calling impure function | Transitive | ❌ |

### 2.2 Algorithm

```
analyze_purity(func: FunctionId, ctx: &mut PurityContext) -> PurityResult:
    // 1. Fast path: already annotated
    if ctx.has_purity_annotation(func):
        return ctx.get_annotation(func)

    // 2. Check for direct side effects
    for op in func.body.operations():
        match op:
            Call(callee, _) if is_io_operation(callee):
                return Impure("IO operation")
            Call(callee, args) where has_mut_arg(args):
                if arg_escapes_function(args):
                    return Impure("mutation of external state via &mut")
            Call(callee, _):
                // Transitivity: callee must also be pure
                if analyze_purity(callee, ctx).is_impure():
                    return Impure("calls impure function: {callee}")

    // 3. Default to pure
    return Pure
```

### 2.3 No Explicit Purity Annotations

**Design decision: Do not provide explicit annotations like `//! pure`.**

The ownership system (RFC-009) already expresses side-effect information through type signatures — `&mut T` means mutation, I/O operations mean external side effects. The compiler is capable of inferring purity automatically.

If the compiler misidentifies a pure function as impure, that's a compiler bug and should be fixed in the compiler, not patched by users. Providing an "trust me, this function is pure" annotation would only mask the real problem.

> *"Don't write compatibility, fallback, temporary, backup, or pattern-specific code. Let problems surface directly."*

### 2.4 Relationship with RFC-022

The purity analyzer serves both:
- **CTE**: Impure functions cannot be used in type positions
- **Hoare logic**: Specification expressions (`requires`/`ensures` right-hand sides) must be pure function calls

---

## 3. Subsystem 2: Termination Checker

### 3.1 Design Philosophy

Compile-time evaluation in type positions must guarantee termination; otherwise, the type system becomes undecidable. YaoXiang uses `//! decreases` specifications to prove termination.

```
//! decreases: <expr>
```

Where `<expr>` is a well-founded value with a lower bound (typically a natural number of `Int` type).

### 3.2 Algorithm

```
check_termination(func: FunctionId, ctx: &mut TermContext) -> TermResult:
    // 1. Find decreases specification
    let decreases_expr = find_decreases_spec(func)
        .or_else(|| infer_decreases(func))

    match decreases_expr:
        None if has_recursive_call(func):
            return TermError::NoDecreasesAnnotation
        None:
            return TermOk  // No recursion, no proof needed

        Some(decreases):
            // 2. Verify each recursive call site
            for call in func.recursive_calls():
                let dec_at_call = eval_decreases_at(call, decreases)
                let dec_at_entry = eval_decreases_at(func.entry, decreases)

                if !strictly_less_than(dec_at_call, dec_at_entry):
                    return TermError::NotDecreasing {
                        at: call.location,
                        expected_less_than: dec_at_entry,
                        actual: dec_at_call,
                    }

            // 3. Verify lower bound
            if !has_lower_bound(decreases):
                return TermError::NoLowerBound

            return TermOk
```

### 3.3 Automatic Inference

Some obvious termination cases need no annotation:

```yaoxiang
// No decreases needed — compiler sees loop has known upper bound n
sum: (arr: Array(Int, n)) -> Int = {
    s = 0; i = 0
    while i < n { s += arr[i]; i += 1 }
    return s
}
```

Cases requiring annotation:
```yaoxiang
// Must annotate decreases — recursive call n-1
factorial: (n: Int) -> Int = {
    //! decreases: n
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
```

### 3.4 Relationship with RFC-022

The termination checker serves both:
- **CTE**: decreases is the admission gate for compile-time evaluation
- **Hoare logic**: decreases variants of loop invariants (`/*! decreases: n - i !*/`) are also verified by the termination checker

---

## 4. Subsystem 3: AST Interpreter

### 4.1 Design Philosophy

The interpreter is based on AST traversal, maintaining an evaluation environment (variable name → CTValue mapping). The core capability is **partial evaluation**: evaluate with known parameters, preserve unknowns.

```
eval(expr: &Expr, env: &mut EvalEnv) -> EvalResult<CTValue>:
    match expr:
        // Literal → direct conversion
        Literal(lit) => lit.into_ctvalue()

        // Variable → lookup environment
        Variable(name) => env.get(name).ok_or(NotInScope)

        // Binary operation
        BinaryOp(l, op, r) =>
            let lv = eval(l, env)?; let rv = eval(r, env)?
            apply_op(op, lv, rv)

        // Conditional branch
        If(cond, then, else) =>
            match eval(cond, env)? {
                Bool(true) => eval(then, env),
                Bool(false) => eval(else, env),
                _ => Err(ExpectedBool),
            }

        // Function call — core logic
        Call(func, args) =>
            let known_args = args.filter_map(|a| eval(a, env).ok())
            if known_args.len() == args.len():
                // All args known → inline and evaluate
                inline_and_eval(func, known_args, env)
            else if known_args.len() > 0:
                // Partial known → partial evaluation (produce monomorphic code)
                partial_eval(func, known_args, env)
            else:
                // All unknown → Thunk
                CTValue::Thunk { func, known_args: vec![], unknown_params: args }

        // Pattern matching
        Match(scrutinee, arms) =>
            let val = eval(scrutinee, env)?
            for arm in arms:
                if arm.pattern.matches(val):
                    return eval(arm.body, env.with_bindings(arm.bindings))
            Err(NoMatch)

        // Code block
        Block(stmts) =>
            for stmt in stmts[..len-1]:
                eval(stmt, env)?
            eval(stmts.last(), env)

        // Loop
        While(cond, body) =>
            let mut result = CTValue::Void
            while eval(cond, env)? == Bool(true):
                check_step_limit()?  // Prevent infinite loops
                result = eval(body, env)?
            result
```

### 4.2 Inline Evaluation

When all parameters are known, the interpreter inlines the function body into the evaluation context:

```
inline_and_eval(func, args, env):
    // 1. Check purity
    purity.check(func)?

    // 2. Check cache
    if let Some(cached) = cache.get(func, args):
        return cached

    // 3. Create inline environment
    let mut inline_env = env.child()
    for (param, arg) in func.params.zip(args):
        inline_env.bind(param.name, arg)

    // 4. Evaluate function body
    let result = eval(&func.body, &mut inline_env)?

    // 5. Cache result
    cache.insert(func, args, result.clone())
    result
```

### 4.3 Step Limit

Compile-time evaluation must have a hard limit to prevent unexpected timeouts even with `decreases`:

```
const MAX_EVAL_STEPS: u64 = 1_000_000;  // One million step hard limit

struct EvalEnv {
    variables: HashMap<SmolStr, CTValue>,
    step_count: u64,
    step_limit: u64,
}
```

### 4.4 Dual-Mode Evaluation: Concrete vs Symbolic

The interpreter core framework (AST traversal, inlining, pattern matching) is unified, but the **evaluation environment** determines the two modes:

#### 4.4.1 Concrete Evaluation

**Consumer**: CTE Engine → type-level evaluation, monomorphization

**Characteristics**:
- All variables in the environment have concrete `CTValue`
- Function call parameters all known → inline evaluation
- Output: `CTValue` (concrete value or type reference)
- Failure = compile error

```
// Scenario: Vec(factorial(5))
// env = { factorial → Function(...) }
eval(Call("factorial", [Literal(5)]), env):
    → inline_and_eval(factorial, [CTValue::Int(5)], env)
    → CTValue::Int(120)
// Type substitution: Vec(120)
```

#### 4.4.2 Symbolic Evaluation

**Consumer**: Hoare logic verifier → SMT solving

**Characteristics**:
- Environment contains **symbolic variables** (e.g., function parameters `n`, `arr`, unknown at compile time)
- Known sub-expressions evaluate to concrete values; unknown parts preserved as SMT symbols
- Function calls are not inlined — instead expanded into logical formulas
- Output: `SMTExpr` (first-order logic expression), handed to Z3
- Failure = verification failure (not a compile error)

```
// Scenario: Verify max's ensures:
//   //! ensures: GreaterOrEqual(result, arr[0..n]) = result >= forall arr[i]
// env = { result → Symbol("result"), arr → Symbol("arr"), n → Symbol("n") }
eval(BinaryOp(Variable("result"), GtEq, Call("arr_max", [Symbol("arr"), Symbol("n")]))):
    // result is symbolic → preserve
    // arr_max(arr, n) is pure but parameters unknown → expand to logical definition
    → SMTExpr::Forall(i in 0..n, Symbol("result") >= Symbol("arr")[i])
// Hand to Z3: ∀arr, n, result. (n > 0 ∧ ...) → result >= arr[0] ∧ ... ∧ result >= arr[n-1]
```

#### 4.4.3 Key Differences Between the Two Modes

| Dimension | Concrete Evaluation | Symbolic Evaluation |
|-----------|---------------------|---------------------|
| Environment | `HashMap<Name, CTValue>` | `HashMap<Name, SMTTerm>` |
| Unknown variable | Error | Preserve as symbol |
| Function call | Inline + evaluate body | Expand to logical definition (don't execute) |
| Loop | Actual iteration (with step limit) | Convert to loop invariant VC |
| Output type | `Result<CTValue, CTError>` | `Result<SMTExpr, SMError>` |
| Failure semantics | Compile error | Verification failure (can downgrade to Runtime Check) |
| Performance | Fast (direct computation) | Slow (SMT solving) |

#### 4.4.4 Shared Interpreter Framework

Both modes share the same AST traversal skeleton:

```rust
/// Interpreter trait: concrete and symbolic evaluation each implement their own
trait Interpreter {
    type Value;       // CTValue or SMTExpr
    type Error;       // CTError or SMError

    fn eval_literal(&mut self, lit: &Literal) -> Result<Self::Value, Self::Error>;
    fn eval_variable(&mut self, name: &str) -> Result<Self::Value, Self::Error>;
    fn eval_binary_op(&mut self, op: BinOp, l: Self::Value, r: Self::Value) -> Result<Self::Value, Self::Error>;
    fn eval_call(&mut self, func: FunctionId, args: &[Expr]) -> Result<Self::Value, Self::Error>;
    fn eval_if(&mut self, cond: &Expr, then: &Expr, else_: &Expr) -> Result<Self::Value, Self::Error>;
    fn eval_match(&mut self, scrutinee: &Expr, arms: &[MatchArm]) -> Result<Self::Value, Self::Error>;
    fn eval_while(&mut self, cond: &Expr, body: &Expr) -> Result<Self::Value, Self::Error>;
}

/// Unified AST traverser, delegating to concrete implementation
fn eval_ast<I: Interpreter>(interp: &mut I, expr: &Expr) -> Result<I::Value, I::Error> {
    match expr {
        Expr::Literal(lit) => interp.eval_literal(lit),
        Expr::Variable(name) => interp.eval_variable(name),
        Expr::BinaryOp { op, left, right } => {
            let l = eval_ast(interp, left)?;
            let r = eval_ast(interp, right)?;
            interp.eval_binary_op(*op, l, r)
        }
        Expr::Call { func, args } => interp.eval_call(*func, args),
        Expr::If { cond, then, else_ } => interp.eval_if(cond, then, else_),
        // ... remaining AST nodes handled similarly
    }
}
```

**Key insight**: Concrete and symbolic evaluation have identical AST traversal logic; the differences are only:
- **What represents values** (`CTValue` vs `SMTExpr`)
- **How function calls are handled** (inline and execute vs expand logically)
- **How unknown variables are handled** (error vs preserve as symbol)

---

## 5. Integration with Other Compiler Passes

### 5.1 CTE Usage in Type Checking

```
1. Type annotation position
   Vec(factorial(5))        → CTE::eval(factorial(5)) → CTValue::Int(120)
   Type substituted to Vec(120)

2. Generic value parameters
   Array(Int, factorial(3)) → CTE::eval(factorial(3)) → CTValue::Int(6)
   Instantiated to Array(Int, 6)

3. Assert type
   Assert(N > 0)            → CTE::eval(N > 0) → CTValue::Bool(true/false)
   True → Void, False → compile_error("N must be > 0")

4. If conditional type
   If(C, T, E)              → CTE::eval(C) → CTValue::Bool(b)
   True → T, False → E

5. Match type family
   AsString(Int)            → match Int { Int => String, ... } → String
```

### 5.2 Interaction with Monomorphization

```
Monomorphization uses CTE results at the following positions:

1. Known generic value parameters → generate concrete instances
   push method of List(Int) → generates push_List_Int

2. Known value-dependent types → expand to concrete types
   Matrix(Float, 3, 3).data → Array(Array(Float, 3), 3)

3. Partial evaluation → generate monomorphic code
   map(Int, String) → generates map_Int_String, with T=Int, R=String already fixed
```

### 5.3 Interaction with Hoare Logic Verifier

```
The verifier uses CTE at the following positions:

1. Partial evaluation of specification expressions
   //! requires: n > 0 && factorial(n) < MAX
   CTE::eval(factorial(n)) → if n known at compile time → constant
                           → if n unknown → preserve as symbol, hand to SMT

2. Specification condition simplification
   //! ensures: result >= 0 && result < n
   CTE attempts to simplify known sub-expressions, reducing SMT solving burden

3. Specification type instantiation
   NonEmpty(n) = n > 0
   CTE expands specification types into Boolean expressions
```

---

## 6. Hoare Logic Static Verification (RFC-022 Implementation Design)

### 6.1 Specification Parsing

`//!` and `/*! ... !*/` are recognized by the parser as special comment nodes attached to the AST:

```rust
struct SpecAnnotation {
    kind: SpecKind,        // Requires | Ensures | Invariant | Decreases
    name: Option<SmolStr>, // Specification name (optional user naming)
    spec_type: TypeExpr,   // Specification type expression
    expr: Expr,            // Boolean expression
    span: Span,
}

enum SpecKind {
    Requires,
    Ensures,
    Invariant,
    Decreases,
}
```

### 6.2 Verification Condition Generation (VCGen)

Uses Weakest Precondition (WP) calculus:

```
generate_vc(func: FunctionId) -> Vec<VerificationCondition>:
    let requires = collect_requires(func)
    let ensures = collect_ensures(func)
    let invariants = collect_invariants(func)
    let decreases = collect_decreases(func)

    let mut vcs = Vec::new()

    // VC1: Precondition consistency
    vcs.push(VC::PreconditionConsistency(requires))

    // VC2: Postcondition verification (for each execution path)
    for path in func.paths():
        let wp = compute_wp(path.body, ensures)
        vcs.push(VC::Postcondition {
            path: path.id,
            formula: implies(requires, wp),
        })

    // VC3: Loop invariants
    for (loop_, invariant) in invariants:
        // Holds before entering loop
        vcs.push(VC::InvariantEntry { loop_, invariant })
        // Preserved on each iteration
        vcs.push(VC::InvariantPreservation { loop_, invariant })
        // Implies postcondition after exit
        vcs.push(VC::InvariantExit { loop_, invariant, post: ensures })

    vcs
```

### 6.3 SMT Solver Integration

```
┌─────────────┐     SMT-LIB format      ┌───────────┐
│  VC Generator │ ──────────────────→ │  Z3 Solver │
└─────────────┘                       └─────┬─────┘
                                            │
                          ┌─────────────────┴──────────────┐
                          ↓                                ↓
                      unsat                            sat
                          ↓                                ↓
                    ┌──────────┐                   ┌──────────────┐
                    │Verification passed│           │ Extract counterexample│
                    │  Cache result  │                   │ Convert to readable format│
                    └──────────┘                   └──────┬───────┘
                                                         ↓
                                                  ┌──────────────┐
                                                  │  Compile error report  │
                                                  │ • Input values       │
                                                  │ • Violated spec      │
                                                  └──────────────┘
```

### 6.4 Build Modes

| Mode | Behavior | CLI |
|------|----------|-----|
| **Debug Build** | Parse specs, generate VCs, call Z3 to prove; verification must pass before Release Build | `yaoxiangc --debug` |
| **Release Build** | Ignore all `//!` comments, zero overhead, clear verification cache | `yaoxiangc --release` |
| **Runtime Checks** | Downgrade specs to `assert` statements, panic on violation | `yaoxiangc --runtime-checks` |

---

## 7. Implementation Phases

### Phase 1: Constant Folding + Purity Analysis Skeleton

**Goal**: Establish CTE infrastructure, support basic compile-time evaluation

**Content**:
- [ ] Define `CTValue` enum and `EvalEnv` struct
- [ ] Implement basic `eval()` paths: literals, variables, binary ops, conditionals, blocks
- [ ] Implement first version of purity analyzer: identify I/O calls as impure, default others to pure
- [ ] Insert CTE call sites in type checker (type annotation positions)
- [ ] Constant folding: `1 + 2 * 3` computed to `7` at compile time
- [ ] Dead branch elimination: `if true { ... } else { ... }` → directly take then branch
- [ ] Unit tests: literal evaluation, simple expressions, constant folding

**Deliverable**: `src/middle/cte/` module, containing `value.rs`, `eval.rs`, `purity.rs`

### Phase 2: Pure Function Compile-Time Evaluation + Termination Checking

**Goal**: Support complete evaluation of pure functions at compile time

**Content**:
- [ ] Implement function inline evaluation: known all parameters → expand function body and evaluate
- [ ] Implement `//! decreases` parsing and termination verification
- [ ] Implement recursive function compile-time evaluation (with step limit)
- [ ] Implement evaluation result caching (Memoization)
- [ ] Refine purity analyzer: use ownership info to identify `&mut` side effects
- [ ] Partial evaluation: code generation optimization when some parameters are known
- [ ] Integration test: `factorial(5)` evaluates to `120` in type position

**Deliverable**: `src/middle/cte/interpreter.rs`, `src/middle/cte/termination.rs`

### Phase 3: Type-Level Computation

**Goal**: Support `If`/`Assert`/`match` type families

**Content**:
- [ ] Implement type-level operations for `CTValue::Type(TypeId)`
- [ ] Implement `If: (C: Bool, T: Type, E: Type) -> Type` conditional type evaluation
- [ ] Implement `Assert(C)` → `True → Void, False → compile_error`
- [ ] Implement type-level `match`: `AsString: (T: Type) -> Type = match T { ... }`
- [ ] Complete instantiation of value-dependent types: `Matrix(Float, 3, 3)` → concrete type
- [ ] Compile-time dimension verification: matrix multiplication dimension mismatch → compile error
- [ ] Integration with monomorphization (mono pass)

**Deliverable**: `src/middle/cte/type_level.rs`, updated `src/middle/passes/mono/`

### Phase 4: Hoare Logic Static Verification

**Goal**: Complete specification parsing, VC generation, SMT verification pipeline

**Content**:
- [ ] Parser extension: recognize `//!` and `/*! ... !*/` as specification nodes
- [ ] Specification type definitions (`NonEmpty`, `Sorted`, `GreaterOrEqual`, etc. standard library spec types)
- [ ] User-defined specification type support
- [ ] VC generator: Weakest Precondition calculus
- [ ] Z3 SMT solver integration (via `z3` crate)
- [ ] SMT-LIB format translation
- [ ] Counterexample extraction and readable reporting
- [ ] Debug/Release/RuntimeChecks build mode switching
- [ ] Integration tests: verify specs for `max`, `binary_search`, and other functions

**Deliverable**: `src/middle/verification/` module

---

## 8. Module Structure

```
src/middle/
├── cte/                          # Compile-Time Evaluation Engine
│   ├── mod.rs                    # CTE entry point, coordinates three subsystems
│   ├── value.rs                  # CTValue definition + basic operations
│   ├── eval.rs                   # Unified AST traverser (Interpreter trait + eval_ast)
│   ├── concrete.rs               # Concrete evaluation implementation (ConcreteInterpreter → CTValue)
│   ├── symbolic.rs               # Symbolic evaluation implementation (SymbolicInterpreter → SMTExpr)
│   ├── env.rs                    # EvalEnv (evaluation environment + step limit)
│   ├── purity.rs                 # Purity analyzer
│   ├── termination.rs            # Termination checker (decreases verification)
│   ├── type_level.rs             # Type-level computation (If/Assert/match type families)
│   └── cache.rs                  # Evaluation result cache
│
├── verification/                 # Hoare Logic Static Verification
│   ├── mod.rs                    # Verification entry point
│   ├── spec_parser.rs            # //! specification parsing
│   ├── spec_types.rs             # Built-in specification type definitions
│   ├── vcgen.rs                  # Verification condition generation (WP calculus)
│   ├── smt.rs                    # Z3 SMT solver interface
│   └── counterexample.rs         # Counterexample formatting
│
└── passes/
    └── mono/                     # Existing monomorphization (enhanced CTE integration)
        └── ...                   # Uses CTE results for instantiation
```

---

## 9. Key Design Decision Log

| Decision | Options | Chosen | Rationale |
|----------|---------|--------|-----------|
| Purity determination method | Explicit annotation vs auto-inference vs both | **Auto-inference** | Ownership system already provides sufficient info; no explicit annotation escape hatch |
| Compile-time evaluator | Restricted subset vs full language | **Full language (with step limit)** | Consistent with unified type syntax's "everything is `name: type = value`" |
| Termination proof | Mandatory annotation vs auto-inference | **Mandatory for type positions, auto elsewhere** | Type position non-termination = undecidable = compile error; can be lenient elsewhere |
| VC generation | WP calculus vs SP calculus | **WP calculus** | Simpler and more direct; clearer error localization |
| SMT solver | Z3 vs CVC5 vs custom | **Z3** | Most mature, best Rust binding, largest community |
| Caching strategy | No cache vs cross-module cache | **LRU cache + incremental invalidation** | Compile-time evaluation results are deterministic pure functions, naturally cacheable |
| Build mode | Unified mode vs Debug/Release separation | **Debug verifies → Release zero overhead** | Verification cost is high; Release should not bear it |

---

## 10. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Z3 integration complexity | Phase 4 delay | Use mature `z3` crate; start with simple arithmetic, expand gradually |
| Compile-time evaluation timeout | Poor user experience | Step limit + clear timeout error messages + suggestions to simplify expressions |
| Purity misjudgment | Inconsistency between compile-time and runtime evaluation | Ownership system provides strong guarantees; if misjudged, it's a compiler bug to fix |
| SMT verification failure hard to debug | Users don't understand why spec doesn't hold | Counterexample extraction + concrete input value display + execution path highlighting |
| Significant compile time increase | CI slowdown | Incremental verification + module-level cache + verification result files (like `.o` files) |

---

## 11. Cross-References to Existing RFCs

| RFC | Relationship | How This Plan Satisfies |
|-----|--------------|------------------------|
| RFC-010 §Unified type syntax | CTValue needs to support all type expressions | `CTValue::Type(TypeId)` + `CTValue::Struct` coverage |
| RFC-011 §4.2 Compile-time computation | Core mechanism for value-dependent types | Phase 2/3 implementation |
| RFC-011 §6 Type-level computation | `If`/`Assert`/`match` type families | Phase 3 implementation |
| RFC-011 §Termination checking mechanism | decreases specification | Phase 2 termination checker implementation |
| RFC-022 §1 Specification comment syntax | `//!` parsing + specification types | Phase 4 implementation |
| RFC-022 §3 Verification mechanism | VC generation + SMT integration | Phase 4 VCGen + SMT modules |
| RFC-009 §Ownership model | Foundation for purity analysis | Phase 1/2 reuse ownership info |

---

## References

- [RFC-010: Unified Type Syntax](../design/rfc/accepted/010-unified-type-syntax.md)
- [RFC-011: Generic Type System Design](../design/rfc/accepted/011-generic-type-system.md)
- [RFC-022: Hoare Logic Static Verification](../design/rfc/draft/022-hoare-logic-static-verification.md)
- [RFC-009: Ownership Model](../design/rfc/accepted/009-ownership-model.md)
- [Z3 Prover](https://github.com/Z3Prover/z3)
- [SMT-LIB Standard](https://smtlib.cs.uiowa.edu/)
- [Weakest Precondition Calculus (Dijkstra)](https://en.wikipedia.org/wiki/Predicate_transformer_semantics)