---
title: 'RFC-031: Optimization Levels and Pass Manager'
status: 'Draft'
author: 'Chen Xu'
created: '2026-06-16'
updated: '2026-07-05'
---

# RFC-031: Optimization Levels and Pass Manager

> **Reference**:
>
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-028: JIT Compiler](./028-jit-compiler.md)
> - [RFC-018: LLVM AOT Compiler](../accepted/018-llvm-aot-compiler.md)

## Summary

This document proposes introducing an **optimization level system** and **Pass Manager** for
YaoXiang, changing compilation optimization from an "all or nothing" approach to configurable
optimization packages. Optimization levels (O0-O3) define different combinations of optimization
strategies, and the Pass Manager is responsible for executing optimization Passes in dependency
order. This document also defines a standard interface for optimization Passes, providing an
architectural foundation for future extensions (monomorphization, inlining, constant folding, etc.).

**Core Goal: Enable users to make explicit trade-offs between compilation speed, binary size, and
runtime performance.**

## Motivation

### Why Do We Need Optimization Levels?

Currently, the compiler has no optimization configuration, and all code goes through the same
processing pipeline. This leads to:

1. **Poor debugging experience**: Optimization is not needed during debugging, but there's no way to
   disable it
2. **No control over binary size**: Generic monomorphization can bloat binaries, but cannot be
   disabled
3. **Uncontrollable compilation speed**: Cannot choose "fast compilation" or "deep optimization"
   based on the scenario
4. **Unordered optimization Passes**: Future multiple optimization Passes have dependencies between
   them, requiring unified management

### Current Problems

```yaoxiang
# Current: All code goes through the same processing
# - During debugging: Optimization not needed, but cannot be disabled
# - In production: Optimization needed, but cannot configure depth
# - Generic functions: Generate multiple copies of code, but cannot control

identity: (T: Type) -> (x: T) -> T = (x) => x
x = identity(42)        # Will generate identity_Int
s = identity("hello")   # Will generate identity_String
# User cannot choose "no monomorphization" (type erasure mode)
```

### Value of Optimization Levels

| Scenario                  | Requirements                                      | Optimization Level |
| ------------------------- | ------------------------------------------------- | ------------------ |
| Development/Debugging     | Fast compilation, retain debug info               | O0                 |
| Daily Development         | Basic optimization, balance compilation speed     | O1                 |
| Testing/CI                | Standard optimization, verify production behavior | O2                 |
| Production Release        | Deep optimization, ultimate performance           | O3                 |
| Scripts/Rapid Prototyping | Auto-select (based on target platform)            | Auto               |

## Proposal

### Core Design

#### 1. Optimization Level Definition

```rust
/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OptLevel {
    /// O0: No optimization (debug mode)
    /// - Retains all debug information
    /// - No optimization transformations
    /// - Fastest compilation speed
    /// - Use case: development debugging, rapid iteration
    O0,

    /// O1: Basic optimization (default)
    /// - On-demand monomorphization (doesn't generate unused specializations)
    /// - Basic constant folding
    /// - Basic dead code elimination
    /// - Use case: daily development
    #[default]
    O1,

    /// O2: Standard optimization
    /// - On-demand monomorphization
    /// - Full constant folding
    /// - Full dead code elimination
    /// - Small function inlining
    /// - Tail call optimization
    /// - Use case: testing, CI, production release
    O2,

    /// O3: Aggressive optimization
    /// - Full monomorphization (pre-generate all possible type combinations)
    /// - Aggressive inlining
    /// - All optimization Passes
    /// - May increase compilation time and binary size
    /// - Use case: ultimate performance requirements
    O3,

    /// Auto: Automatic selection
    /// - Automatically selects optimization strategy based on target platform and available resources
    /// - Use case: scripts, rapid prototyping
    Auto,
}
```

#### 2. Optimization Pass Interface

```rust
/// Optimization Pass interface
pub trait OptimizationPass {
    /// Pass name (for logging and dependency declaration)
    fn name(&self) -> &str;

    /// Run the Pass
    fn run(&self, module: &mut ModuleIR, config: &PassConfig) -> PassResult;

    /// Other Passes that this Pass depends on (must run first)
    fn dependencies(&self) -> Vec<&str> {
        vec![]
    }

    /// Whether this Pass should run under the current configuration
    fn should_run(&self, config: &PassConfig) -> bool {
        true
    }
}

/// Pass configuration
#[derive(Debug, Clone)]
pub struct PassConfig {
    /// Optimization level
    pub opt_level: OptLevel,
    /// Whether to enable debug information
    pub debug_info: bool,
    /// Target platform
    pub target_platform: TargetPlatform,
}

/// Pass run result
#[derive(Debug, Default)]
pub struct PassResult {
    /// Whether the IR was modified
    pub changed: bool,
    /// Statistics
    pub stats: PassStats,
}

/// Pass statistics
#[derive(Debug, Default)]
pub struct PassStats {
    /// Number of functions inlined
    pub functions_inlined: usize,
    /// Number of functions monomorphized
    pub functions_monomorphized: usize,
    /// Amount of dead code removed
    pub dead_code_removed: usize,
    /// Number of constants folded
    pub constants_folded: usize,
}
```

#### 3. Pass Manager

```rust
/// Optimizer
pub struct Optimizer {
    /// Registered list of Passes (sorted by dependency order)
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl Optimizer {
    /// Create an optimizer based on optimization level
    pub fn for_opt_level(level: OptLevel) -> Self {
        let passes = Self::create_passes_for_level(level);
        Self { passes }
    }

    /// Create list of Passes for the specified level
    fn create_passes_for_level(level: OptLevel) -> Vec<Box<dyn OptimizationPass>> {
        match level {
            OptLevel::O0 => {
                vec![
                    // Debug mode: minimal optimization, only necessary cleanup
                    Box::new(ConstFoldPass::minimal()),
                ]
            }
            OptLevel::O1 => {
                vec![
                    // Basic optimization
                    Box::new(ConstFoldPass::basic()),
                    Box::new(MonomorphizePass::on_demand()),
                    Box::new(DcePass::basic()),
                ]
            }
            OptLevel::O2 => {
                vec![
                    // Standard optimization
                    Box::new(ConstFoldPass::full()),
                    Box::new(MonomorphizePass::on_demand()),
                    Box::new(DcePass::full()),
                    Box::new(InlinePass::small_functions()),
                    Box::new(TcoPass::new()),
                ]
            }
            OptLevel::O3 => {
                vec![
                    // Aggressive optimization
                    Box::new(ConstFoldPass::full()),
                    Box::new(MonomorphizePass::full()),
                    Box::new(InlinePass::aggressive()),
                    Box::new(DcePass::full()),
                    Box::new(TcoPass::new()),
                    // More aggressive optimizations...
                ]
            }
            OptLevel::Auto => {
                // Auto-select: determined by target platform
                Self::create_passes_for_level(OptLevel::O1)
            }
        }
    }

    /// Run all optimization Passes
    pub fn run(&self, module: &mut ModuleIR, config: &PassConfig) -> OptimizerResult {
        let mut total_stats = OptimizerStats::default();

        for pass in &self.passes {
            if !pass.should_run(config) {
                continue;
            }

            let result = pass.run(module, config);
            total_stats.merge(result.stats);
        }

        OptimizerResult {
            module: module.clone(),
            stats: total_stats,
        }
    }
}
```

### Examples

#### Command Line Usage

```bash
# Debug mode: no optimization
yaoxiang build --opt-level O0

# Daily development: basic optimization (default)
yaoxiang build

# Production release: standard optimization
yaoxiang build --opt-level O2

# Ultimate performance: aggressive optimization
yaoxiang build --opt-level O3

# Auto-select
yaoxiang build --opt-level Auto
```

#### Configuration File

```json
{
  "optimization_level": "O2",
  "mono": {
    "enabled": true,
    "strategy": "OnDemand"
  },
  "debug_info": false
}
```

#### API Usage

```rust
use yaoxiang::frontend::{Compiler, CompileConfig, OptLevel};

// Debug mode
let config = CompileConfig::new()
    .with_opt_level(OptLevel::O0);
let mut compiler = Compiler::with_config(config);

// Production mode
let config = CompileConfig::new()
    .with_opt_level(OptLevel::O2);
let mut compiler = Compiler::with_config(config);
```

### Syntax Changes

No syntax changes. Optimization levels are compiler configuration and do not affect language syntax.

## Detailed Design

### Optimization Level and Pass Mapping

| Pass                       | O0  | O1        | O2              | O3         | Description                                  |
| -------------------------- | --- | --------- | --------------- | ---------- | -------------------------------------------- |
| **Constant Folding**       | Min | Basic     | Full            | Full       | Compute constant expressions at compile time |
| **Monomorphization**       | ❌  | On-demand | On-demand       | Full       | Specialize generic functions                 |
| **Dead Code Elimination**  | ❌  | Basic     | Full            | Full       | Remove unreachable/unused code               |
| **Function Inlining**      | ❌  | ❌        | Small functions | Aggressive | Insert function body at call site            |
| **Tail Call Optimization** | ❌  | ❌        | ✅              | ✅         | Convert tail recursion to loops              |
| **Escape Analysis**        | ❌  | ❌        | ❌              | ✅         | Determine stack/heap allocation              |
| **Loop Optimization**      | ❌  | ❌        | ❌              | ✅         | Loop unrolling, invariant code motion        |

### Monomorphization Strategy

```rust
/// Monomorphization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MonoStrategy {
    /// No monomorphization — type erasure, generic functions have only one copy
    /// Advantages: smaller binary, faster compilation
    /// Disadvantages: dynamic dispatch overhead at runtime
    Erased,

    /// On-demand monomorphization — generate code only for actual type combinations used
    /// Advantages: zero-cost abstraction, no runtime overhead
    /// Disadvantages: binary may bloat
    #[default]
    OnDemand,

    /// Full monomorphization — pre-generate all possible type combinations
    /// Advantages: all calls resolved at compile time
    /// Disadvantages: slow compilation, large binary
    Full,
}

/// Monomorphization configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonoConfig {
    /// Whether to enable monomorphization
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Monomorphization strategy
    #[serde(default)]
    pub strategy: MonoStrategy,

    /// Whether to enable DCE (dead code elimination)
    #[serde(default = "default_true")]
    pub dce_enabled: bool,

    /// Maximum specialization depth (prevents infinite recursive generics)
    #[serde(default = "default_max_mono_depth")]
    pub max_depth: usize,
}

impl Default for MonoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: MonoStrategy::OnDemand,
            dce_enabled: true,
            max_depth: 100,
        }
    }
}
```

### Compilation Pipeline Integration

```rust
// src/frontend/pipeline.rs

impl Pipeline {
    fn run_ir_generation(
        &mut self,
        source_name: &str,
        source: &str,
        ast: &Module,
        type_result: &TypeCheckResult,
        phase_durations: &mut Vec<(CompilationPhase, u64)>,
    ) -> IRResult {
        let start = Instant::now();

        // 1. Generate base IR
        let mut ir = middle::generate_ir(ast, type_result)?;

        // 2. Run optimization Passes based on optimization level
        let optimizer = Optimizer::for_opt_level(self.config.optimization_level);
        let pass_config = PassConfig {
            opt_level: self.config.optimization_level,
            debug_info: self.config.generate_debug_info,
            target_platform: TargetPlatform::detect(),
        };

        let result = optimizer.run(&mut ir, &pass_config);

        let duration = start.elapsed().as_millis() as u64;
        phase_durations.push((CompilationPhase::Optimization, duration));

        IRResult::success(result.module)
    }
}
```

### Type System Impact

No direct impact. Optimization Passes operate at the IR level and do not affect the type system.

### Runtime Behavior

| Optimization Level | Runtime Behavior                            |
| ------------------ | ------------------------------------------- |
| O0                 | No optimization, retain all debug info      |
| O1                 | Basic optimization, retain basic debug info |
| O2                 | Standard optimization, no debug info        |
| O3                 | Aggressive optimization, no debug info      |

**Key Point: No runtime modifications needed**. Optimization Passes only affect the IR level and
code generation level. The runtime looks up and executes functions by name/ID and is unaware of the
optimization process.

### Compiler Changes

| Component                  | Changes                              |
| -------------------------- | ------------------------------------ |
| `frontend/config.rs`       | Add `OptLevel` enum and `MonoConfig` |
| `frontend/pipeline.rs`     | Integrate Pass Manager               |
| `middle/passes/optimizer/` | Add optimization Pass module         |
| `middle/passes/mono/`      | Refactor to standard Pass interface  |
| CLI                        | Add `--opt-level` parameter          |

### Backward Compatibility

- ✅ Fully backward compatible
- Default optimization level is O1, behavior is consistent with current
- Users can explicitly specify optimization level to override default

## Trade-offs

### Advantages

- **Flexibility**: Users can choose optimization strategies based on scenario
- **Extensibility**: Standard Pass interface, easy to add new optimizations
- **Predictability**: Clear behavior for each optimization level
- **Debugging friendly**: O0 mode retains complete debug information

### Disadvantages

- **Increased complexity**: Need to maintain multiple optimization levels
- **Larger test matrix**: Need to test behavior for each optimization level
- **Documentation burden**: Need to explain the meaning of each optimization level

## Alternative Approaches

| Approach                           | Why Not Chosen                                                                                |
| ---------------------------------- | --------------------------------------------------------------------------------------------- |
| Only on/off states                 | Cannot finely control optimization depth                                                      |
| Use GCC/LLVM-style `-O` numbers    | Inconsistent with YaoXiang's configuration system                                             |
| Independent switches for each Pass | Users need to understand details of each Pass, usage is complex                               |
| Defer to v2.0                      | Monomorphization is implemented but not integrated, need to resolve architecture issues first |

## Implementation Strategy

### Phase Division

1. **Phase 1 (current)**: Define optimization levels and Pass interface
2. **Phase 2**: Implement monomorphization Pass (based on existing `mono/` module)
3. **Phase 3**: Implement constant folding and dead code elimination Passes
4. **Phase 4**: Implement function inlining and tail call optimization Passes
5. **Phase 5**: Implement aggressive optimization Passes (escape analysis, loop optimization)

### Dependencies

- Depends on RFC-011 (generic system)'s monomorphization module
- Depends on RFC-028 (JIT compiler)'s optimization Pass interface
- Shares optimization Pass design with RFC-018 (LLVM AOT)

### Risks

- **Performance regression**: Optimization Passes may introduce bugs, causing performance
  degradation
- **Increased compilation time**: Optimization Passes increase compilation time
- **Binary bloat**: Monomorphization may significantly increase binary size

## Open Questions

- [ ] Should O3 level enable escape analysis by default? (@Chen Xu: needs performance test data)
- [ ] Is there a need for `Os` (optimize for size) and `Oz` (optimize for extreme size) levels?
- [ ] Should optimization levels affect the verbosity of debug information?
- [ ] How to handle circular dependencies between optimization Passes?

---

## Appendix A: Design Decision Records

| Decision                   | Decision                       | Date       | Recorder |
| -------------------------- | ------------------------------ | ---------- | -------- |
| Optimization level naming  | Use O0-O3 + Auto               | 2026-06-16 | Chen Xu  |
| Default optimization level | O1 (basic optimization)        | 2026-06-16 | Chen Xu  |
| Monomorphization strategy  | Support Erased/OnDemand/Full   | 2026-06-16 | Chen Xu  |
| Pass interface design      | trait + dependency declaration | 2026-06-16 | Chen Xu  |

---

## Appendix B: Glossary

| Term                       | Definition                                                                        |
| -------------------------- | --------------------------------------------------------------------------------- |
| **Optimization Pass**      | An independent module that performs one transformation on IR                      |
| **Monomorphization**       | Code generation strategy that specializes generic functions for concrete types    |
| **Constant Folding**       | Computing constant expressions at compile time                                    |
| **Dead Code Elimination**  | Removing unreachable or unused code from the program                              |
| **Function Inlining**      | Inserting function body at call site to avoid function call overhead              |
| **Tail Call Optimization** | Converting tail recursion to loops to avoid stack overflow                        |
| **Escape Analysis**        | Analyzing whether variables escape their scope to determine stack/heap allocation |

---

## References

- [Rust Compiler Optimizations](https://rustc-dev-guide.rust-lang.org/optimizations.html)
- [GCC Optimization Levels](https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html)
- [LLVM Pass Manager](https://llvm.org/docs/WritingAnLLVMNewPMPass.html)
- [V8 TurboFan Optimization Pipeline](https://v8.dev/docs/turbofan)

---

## Lifecycle and Future

This RFC defines the architecture design for optimization levels, providing a unified framework for
future optimization Passes.

**Relationship with Monomorphization**: Monomorphization is one of the optimization Passes and will
be the first Pass to be implemented after this RFC is accepted.
