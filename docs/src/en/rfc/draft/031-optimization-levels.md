---
title: 'RFC-031: Optimization Levels and Pass Manager'
status: 'Draft'
author: 'Chenxu'
created: '2026-06-16'
updated: '2026-07-05'
---

# RFC-031: Optimization Levels and Pass Manager

> **References**:
>
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-028: JIT Compiler](028-jit-compiler.md)
> - [RFC-018: LLVM AOT Compiler](../accepted/018-llvm-aot-compiler.md)

## Summary

This document proposes introducing an **optimization level system** and a **Pass manager** for
YaoXiang, transforming compilation optimization from an "all or nothing" approach into a
configurable set of optimization packages. Optimization levels (O0–O3) define different combinations
of optimization strategies, and the Pass manager is responsible for executing optimization Passes in
dependency order. This document also defines a standard interface for optimization Passes, providing
an architectural foundation for future extensions (monomorphization, inlining, constant folding,
etc.).

**Core goal: enable users to make explicit trade-offs between compilation speed, binary size, and
runtime performance.**

## Motivation

### Why do we need optimization levels?

The current compiler has no optimization configuration; all code goes through the same processing
pipeline. This leads to:

1. **Poor debugging experience**: Optimization is not needed during debugging, but it cannot be
   turned off.
2. **No control over binary size**: Generic monomorphization inflates binaries, but cannot be
   disabled.
3. **Uncontrollable compilation speed**: Cannot choose between "fast compilation" or "deep
   optimization" based on the scenario.
4. **Unordered optimization Passes**: Multiple future optimization Passes have dependencies on each
   other and need unified management.

### Current problems

```yaoxiang
# Currently: all code goes through the same processing
# - During debugging: no optimization needed, but cannot be turned off
# - In production: optimization needed, but depth cannot be configured
# - Generic functions: multiple copies are generated, but cannot be controlled

identity: (T: Type) -> (x: T) -> T = (x) => x
x = identity(42)        # Will generate identity_Int
s = identity("hello")   # Will generate identity_String
# User has no way to choose "no monomorphization" (type erasure mode)
```

### Value of optimization levels

| Scenario                 | Requirement                                       | Optimization Level |
| ------------------------ | ------------------------------------------------- | ------------------ |
| Development/debugging    | Fast compilation, preserve debug information      | O0                 |
| Daily development        | Basic optimization, balanced compilation speed    | O1                 |
| Testing/CI               | Standard optimization, verify production behavior | O2                 |
| Production release       | Deep optimization, ultimate performance           | O3                 |
| Scripts/rapid prototypes | Auto-select (based on target platform)            | Auto               |

## Proposal

### Core design

#### 1. Optimization level definition

```rust
/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OptLevel {
    /// O0: No optimization (debug mode)
    /// - Preserve all debug information
    /// - Perform no optimization transformations
    /// - Fastest compilation speed
    /// - Use case: development debugging, fast iteration
    O0,

    /// O1: Basic optimization (default)
    /// - On-demand monomorphization (do not generate unused specialized versions)
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
    /// - Automatically select optimization strategy based on target platform and available resources
    /// - Use case: scripts, rapid prototypes
    Auto,
}
```

#### 2. Optimization Pass interface

```rust
/// Optimization Pass interface
pub trait OptimizationPass {
    /// Pass name (used for logging and dependency declaration)
    fn name(&self) -> &str;

    /// Run the Pass
    fn run(&self, module: &mut ModuleIR, config: &PassConfig) -> PassResult;

    /// Which other Passes this Pass depends on running first
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
    /// Number of inlined functions
    pub functions_inlined: usize,
    /// Number of monomorphized functions
    pub functions_monomorphized: usize,
    /// Number of dead code removed
    pub dead_code_removed: usize,
    /// Number of constants folded
    pub constants_folded: usize,
}
```

#### 3. Pass manager

```rust
/// Optimizer
pub struct Optimizer {
    /// Registered Pass list (sorted by dependency order)
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl Optimizer {
    /// Create an optimizer for the given optimization level
    pub fn for_opt_level(level: OptLevel) -> Self {
        let passes = Self::create_passes_for_level(level);
        Self { passes }
    }

    /// Create the Pass list for the specified level
    fn create_passes_for_level(level: OptLevel) -> Vec<Box<dyn OptimizationPass>> {
        match level {
            OptLevel::O0 => {
                vec![
                    // Debug mode: minimum optimization, only necessary cleanup
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
                // Auto-select: based on target platform
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

#### Command line usage

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

#### Configuration file

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

#### API usage

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

### Syntax changes

No syntax changes. Optimization level is a compiler configuration and does not affect language
syntax.

## Detailed design

### Optimization level and Pass mapping

| Pass                       | O0      | O1        | O2              | O3         | Description                                  |
| -------------------------- | ------- | --------- | --------------- | ---------- | -------------------------------------------- |
| **Constant Folding**       | Minimal | Basic     | Full            | Full       | Compute constant expressions at compile time |
| **Monomorphization**       | ❌      | On demand | On demand       | Full       | Generic function specialization              |
| **Dead Code Elimination**  | ❌      | Basic     | Full            | Full       | Remove unused code                           |
| **Function Inlining**      | ❌      | ❌        | Small functions | Aggressive | Insert function body at the call site        |
| **Tail Call Optimization** | ❌      | ❌        | ✅              | ✅         | Convert tail recursion to loops              |
| **Escape Analysis**        | ❌      | ❌        | ❌              | ✅         | Decide stack/heap allocation                 |
| **Loop Optimization**      | ❌      | ❌        | ❌              | ✅         | Loop unrolling, invariant hoisting           |

### Monomorphization strategy

```rust
/// Monomorphization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MonoStrategy {
    /// No monomorphization — type erasure; generic functions have only one copy
    /// Pros: small binary, fast compilation
    /// Cons: dynamic dispatch overhead at runtime
    Erased,

    /// On-demand monomorphization — only generate code for actually used type combinations
    /// Pros: zero-cost abstraction, no runtime overhead
    /// Cons: binary may bloat
    #[default]
    OnDemand,

    /// Full monomorphization — pre-generate all possible type combinations
    /// Pros: all calls determined at compile time
    /// Cons: slow compilation, large binary
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

    /// Whether to enable DCE (Dead Code Elimination)
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

### Compilation pipeline integration

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

### Type system impact

No direct impact. Optimization Passes run at the IR layer and do not affect the type system.

### Runtime behavior

| Optimization Level | Runtime Behavior                              |
| ------------------ | --------------------------------------------- |
| O0                 | No optimization, preserve all debug info      |
| O1                 | Basic optimization, preserve basic debug info |
| O2                 | Standard optimization, no debug info          |
| O3                 | Aggressive optimization, no debug info        |

**Key point: no runtime modifications required**. Optimization Passes only affect the IR layer and
code generation layer; the runtime looks up execution by function name/ID and is unaware of the
optimization process.

### Compiler changes

| Component                  | Change                               |
| -------------------------- | ------------------------------------ |
| `frontend/config.rs`       | Add `OptLevel` enum and `MonoConfig` |
| `frontend/pipeline.rs`     | Integrate Pass manager               |
| `middle/passes/optimizer/` | Add optimization Pass module         |
| `middle/passes/mono/`      | Refactor to standard Pass interface  |
| CLI                        | Add `--opt-level` parameter          |

### Backward compatibility

- ✅ Fully backward compatible
- Default optimization level is O1, behavior consistent with the current implementation
- Users can explicitly specify the optimization level to override the default behavior

## Trade-offs

### Advantages

- **Flexibility**: Users can choose optimization strategies based on the scenario
- **Extensibility**: Standard Pass interface, easy to add new optimizations
- **Predictability**: Behavior of each optimization level is explicit
- **Debugging-friendly**: O0 mode preserves complete debug information

### Disadvantages

- **Increased complexity**: Multiple optimization levels need to be maintained
- **Larger test matrix**: Behavior of each optimization level needs to be tested
- **Documentation burden**: Need to explain the meaning of each optimization level

## Alternatives

| Plan                             | Why not chosen                                                                                            |
| -------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Only on/off two states           | Cannot finely control optimization depth                                                                  |
| Use GCC/LLVM-style `-O` numbers  | Inconsistent with YaoXiang's configuration system                                                         |
| Independent toggle for each Pass | Users need to understand each Pass's details, complex to use                                              |
| Defer to v2.0                    | Monomorphization is already implemented but not integrated; architecture issues need to be resolved first |

## Implementation strategy

### Phases

1. **Phase 1 (current)**: Define optimization levels and Pass interface
2. **Phase 2**: Implement the monomorphization Pass (based on the existing `mono/` module)
3. **Phase 3**: Implement constant folding and dead code elimination Passes
4. **Phase 4**: Implement function inlining and tail call optimization Passes
5. **Phase 5**: Implement aggressive optimization Passes (escape analysis, loop optimization)

### Dependencies

- Depends on RFC-011 (Generic Type System)'s monomorphization module
- Depends on RFC-028 (JIT Compiler)'s optimization Pass interface
- Shares the optimization Pass design with RFC-018 (LLVM AOT)

### Risks

- **Performance regression**: Optimization Passes may introduce bugs, causing performance
  degradation
- **Increased compilation time**: Optimization Passes increase compilation time
- **Binary bloat**: Monomorphization may significantly increase binary size

## Open questions

- [ ] Should O3 level enable escape analysis by default? (@Chenxu: requires performance test data)
- [ ] Do we need `Os` (optimize for size) and `Oz` (ultimate size optimization) levels?
- [ ] Should optimization level affect the verbosity of debug information?
- [ ] How to handle circular dependencies between optimization Passes?

---

## Appendix A: Design decision record

| Item                       | Decision                       | Date       | Recorder |
| -------------------------- | ------------------------------ | ---------- | -------- |
| Optimization level naming  | Use O0–O3 + Auto               | 2026-06-16 | Chenxu   |
| Default optimization level | O1 (basic optimization)        | 2026-06-16 | Chenxu   |
| Monomorphization strategy  | Support Erased/OnDemand/Full   | 2026-06-16 | Chenxu   |
| Pass interface design      | trait + dependency declaration | 2026-06-16 | Chenxu   |

---

## Appendix B: Glossary

| Term                       | Definition                                                                        |
| -------------------------- | --------------------------------------------------------------------------------- |
| **Optimization Pass**      | An independent module that performs a single transformation on the IR             |
| **Monomorphization**       | A code generation strategy that specializes generic functions into concrete types |
| **Constant Folding**       | Compute constant expressions at compile time                                      |
| **Dead Code Elimination**  | Remove unreachable or unused code from the program                                |
| **Function Inlining**      | Insert the function body at the call site to avoid function call overhead         |
| **Tail Call Optimization** | Convert tail recursion to a loop to avoid stack overflow                          |
| **Escape Analysis**        | Analyze whether a variable escapes its scope to decide stack/heap allocation      |

---

## References

- [Rust Compiler Optimizations](https://rustc-dev-guide.rust-lang.org/optimizations.html)
- [GCC Optimization Levels](https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html)
- [LLVM Pass Manager](https://llvm.org/docs/WritingAnLLVMNewPMPass.html)
- [V8 TurboFan Optimization Pipeline](https://v8.dev/docs/turbofan)

---

## Lifecycle and destination

This RFC defines the architectural design of optimization levels, providing a unified framework for
future optimization Passes.

**Relationship with monomorphization**: Monomorphization is one of the optimization Passes and will
be implemented as the first Pass after this RFC is accepted.
