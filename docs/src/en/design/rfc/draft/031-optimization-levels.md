---
title: "RFC-031: Optimization Levels and Pass Manager"
status: "Draft"
author: "Chenxu"
created: "2026-06-16"
updated: "2026-07-05"
---

# RFC-031: Optimization Levels and Pass Manager

> **References**:
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-028: JIT Compiler](./028-jit-compiler.md)
> - [RFC-018: LLVM AOT Compiler](../accepted/018-llvm-aot-compiler.md)

## Summary

This document proposes introducing an **optimization level system** and a **Pass Manager** for YaoXiang, transforming compilation optimization from an "all-or-nothing" approach into configurable optimization packages. Optimization levels (O0–O3) define different combinations of optimization strategies, and the Pass Manager is responsible for executing optimization Passes in dependency order. This document also defines a standard interface for optimization Passes, providing an architectural foundation for future extensions (monomorphization, inlining, constant folding, etc.).

**Core Goal: Enable users to make explicit trade-offs between compilation speed, binary size, and runtime performance.**

## Motivation

### Why do we need optimization levels?

The current compiler has no optimization configuration; all code goes through the same processing pipeline. This causes:

1. **Poor debugging experience**: Optimization is not needed during debugging, but cannot be disabled
2. **No control over binary size**: Generic monomorphization can bloat the binary, but cannot be disabled
3. **Uncontrollable compilation speed**: Cannot choose "fast compilation" or "deep optimization" based on the scenario
4. **Unordered optimization Passes**: Future optimization Passes have dependencies among each other and require unified management

### Current Problems

```yaoxiang
# Current: all code goes through the same processing
# - During debugging: optimization is not needed, but cannot be disabled
# - During production: optimization is needed, but depth cannot be configured
# - Generic functions: generate multiple code versions, but cannot be controlled

identity: (T: Type) -> (x: T) -> T = (x) => x
x = identity(42)        # will generate identity_Int
s = identity("hello")   # will generate identity_String
# Users cannot choose "do not monomorphize" (type erasure mode)
```

### The Value of Optimization Levels

| Scenario | Need | Optimization Level |
|------|------|----------|
| Development debugging | Fast compilation, preserve debug info | O0 |
| Daily development | Basic optimization, balance compilation speed | O1 |
| Testing/CI | Standard optimization, verify production behavior | O2 |
| Production release | Deep optimization, peak performance | O3 |
| Scripts/quick prototypes | Auto-select (based on target platform) | Auto |

## Proposal

### Core Design

#### 1. Optimization Level Definition

```rust
/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OptLevel {
    /// O0: No optimization (debug mode)
    /// - Preserve all debug info
    /// - Perform no optimization transformations
    /// - Fastest compilation speed
    /// - Use case: development debugging, fast iteration
    O0,

    /// O1: Basic optimization (default)
    /// - On-demand monomorphization (do not generate unused specializations)
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
    /// - Use case: extreme performance requirements
    O3,

    /// Auto: Automatic selection
    /// - Automatically select optimization strategy based on target platform and available resources
    /// - Use case: scripts, quick prototypes
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

    /// Which other Passes must run before this Pass
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
    /// Whether debug info is enabled
    pub debug_info: bool,
    /// Target platform
    pub target_platform: TargetPlatform,
}

/// Pass execution result
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
    /// Number of removed dead code items
    pub dead_code_removed: usize,
    /// Number of folded constants
    pub constants_folded: usize,
}
```

#### 3. Pass Manager

```rust
/// Optimizer
pub struct Optimizer {
    /// Registered Pass list (sorted by dependency order)
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl Optimizer {
    /// Create an optimizer for a given optimization level
    pub fn for_opt_level(level: OptLevel) -> Self {
        let passes = Self::create_passes_for_level(level);
        Self { passes }
    }

    /// Create the Pass list for a given level
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
                // Auto-select: decide based on target platform
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

# Peak performance: aggressive optimization
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

### Optimization Level to Pass Mapping

| Pass | O0 | O1 | O2 | O3 | Description |
|------|----|----|----|----|------|
| **Constant Folding** | Minimal | Basic | Full | Full | Compute constant expressions at compile-time |
| **Monomorphization** | ❌ | On-demand | On-demand | Full | Generic function specialization |
| **Dead Code Elimination** | ❌ | Basic | Full | Full | Remove unused code |
| **Function Inlining** | ❌ | ❌ | Small functions | Aggressive | Insert function body at call site |
| **Tail Call Optimization** | ❌ | ❌ | ✅ | ✅ | Convert tail recursion to loop |
| **Escape Analysis** | ❌ | ❌ | ❌ | ✅ | Decide stack/heap allocation |
| **Loop Optimization** | ❌ | ❌ | ❌ | ✅ | Loop unrolling, hoisting invariants |

### Monomorphization Strategy

```rust
/// Monomorphization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MonoStrategy {
    /// No monomorphization — type erasure; generic functions have only one copy
    /// Pros: small binary, fast compilation
    /// Cons: runtime dynamic dispatch overhead
    Erased,

    /// On-demand monomorphization — only generate code for actually used type combinations
    /// Pros: zero-cost abstraction, no runtime overhead
    /// Cons: binary may bloat
    #[default]
    OnDemand,

    /// Full monomorphization — pre-generate all possible type combinations
    /// Pros: all calls resolved at compile-time
    /// Cons: slow compilation, large binary
    Full,
}

/// Monomorphization configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonoConfig {
    /// Whether monomorphization is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Monomorphization strategy
    #[serde(default)]
    pub strategy: MonoStrategy,

    /// Whether DCE (dead code elimination) is enabled
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

No direct impact. Optimization Passes operate at the IR layer and do not affect the type system.

### Runtime Behavior

| Optimization Level | Runtime Behavior |
|----------|-----------|
| O0 | No optimization, preserve all debug info |
| O1 | Basic optimization, preserve basic debug info |
| O2 | Standard optimization, no debug info |
| O3 | Aggressive optimization, no debug info |

**Key point: no runtime changes required**. Optimization Passes only affect the IR layer and code generation layer; the runtime looks up execution by function name/ID and is unaware of the optimization process.

### Compiler Changes

| Component | Change |
|------|------|
| `frontend/config.rs` | Add `OptLevel` enum and `MonoConfig` |
| `frontend/pipeline.rs` | Integrate the Pass Manager |
| `middle/passes/optimizer/` | Add optimization Pass module |
| `middle/passes/mono/` | Refactor into the standard Pass interface |
| CLI | Add `--opt-level` parameter |

### Backward Compatibility

- ✅ Fully backward compatible
- Default optimization level is O1, behavior consistent with the current state
- Users can explicitly specify an optimization level to override the default behavior

## Trade-offs

### Pros

- **Flexibility**: Users can choose optimization strategies based on the scenario
- **Extensibility**: Standard Pass interface makes it easy to add new optimizations
- **Predictability**: Behavior of each optimization level is clearly defined
- **Debug-friendly**: O0 mode preserves full debug info

### Cons

- **Increased complexity**: Multiple optimization levels need to be maintained
- **Larger test matrix**: Behavior of each optimization level needs testing
- **Documentation burden**: Need to explain the meaning of each optimization level

## Alternatives

| Approach | Why Not Chosen |
|------|--------------|
| Only an on/off toggle | Cannot finely control optimization depth |
| Using GCC/LLVM-style `-O` numbers | Inconsistent with YaoXiang's configuration system |
| Independent on/off switch for each Pass | Users need to understand the details of each Pass, which is complex to use |
| Defer to v2.0 | Monomorphization is already implemented but not integrated; the architecture must be resolved first |

## Implementation Strategy

### Phased Rollout

1. **Phase 1 (current)**: Define optimization levels and Pass interface
2. **Phase 2**: Implement monomorphization Pass (based on the existing `mono/` module)
3. **Phase 3**: Implement constant folding and dead code elimination Passes
4. **Phase 4**: Implement function inlining and tail call optimization Passes
5. **Phase 5**: Implement aggressive optimization Passes (escape analysis, loop optimization)

### Dependencies

- Depends on the monomorphization module from RFC-011 (Generic System)
- Depends on the optimization Pass interface from RFC-028 (JIT Compiler)
- Shares the optimization Pass design with RFC-018 (LLVM AOT)

### Risks

- **Performance regression**: Optimization Passes may introduce bugs that cause performance degradation
- **Increased compilation time**: Optimization Passes add to compilation time
- **Binary bloat**: Monomorphization may cause significant increase in binary size

## Open Questions

- [ ] Should O3 enable escape analysis by default? (@Chenxu: performance test data needed)
- [ ] Do we need `Os` (optimize for size) and `Oz` (aggressively optimize for size) levels?
- [ ] Should optimization levels affect the verbosity of debug info?
- [ ] How to handle circular dependencies between optimization Passes?

---

## Appendix A: Design Decision Record

| Decision | Resolution | Date | Recorder |
|------|------|------|--------|
| Optimization level naming | Use O0–O3 + Auto | 2026-06-16 | Chenxu |
| Default optimization level | O1 (basic optimization) | 2026-06-16 | Chenxu |
| Monomorphization strategy | Support Erased/OnDemand/Full | 2026-06-16 | Chenxu |
| Pass interface design | trait + dependency declaration | 2026-06-16 | Chenxu |

---

## Appendix B: Glossary

| Term | Definition |
|------|------|
| **Optimization Pass** | An independent module that performs one transformation on the IR |
| **Monomorphization** | A code generation strategy that specializes generic functions into concrete-type versions |
| **Constant Folding** | Computing constant expressions at compile-time |
| **Dead Code Elimination** | Removing unreachable or unused code from a program |
| **Function Inlining** | Inserting a function body at its call site to avoid function call overhead |
| **Tail Call Optimization** | Converting tail recursion into a loop to avoid stack overflow |
| **Escape Analysis** | Analyzing whether a variable escapes its scope to decide stack/heap allocation |

---

## References

- [Rust Compiler Optimizations](https://rustc-dev-guide.rust-lang.org/optimizations.html)
- [GCC Optimization Levels](https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html)
- [LLVM Pass Manager](https://llvm.org/docs/WritingAnLLVMNewPMPass.html)
- [V8 TurboFan Optimization Pipeline](https://v8.dev/docs/turbofan)

---

## Lifecycle and Destination

This RFC defines the architectural design of optimization levels, providing a unified framework for future optimization Passes.

**Relationship with monomorphization**: Monomorphization is one of the optimization Passes and will be implemented as the first Pass after this RFC is accepted.