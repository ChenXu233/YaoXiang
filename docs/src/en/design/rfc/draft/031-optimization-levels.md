---
title: "RFC-031: Optimization Level and Pass Manager"
status: "Draft"
author: "晨煦"
created: "2026-06-16"
---

# RFC-031: Optimization Level and Pass Manager

> **References**:
> - [RFC-011: Generic Type System Design](../accepted/011-generic-type-system.md)
> - [RFC-028: JIT Compiler](./028-jit-compiler.md)
> - [RFC-018: LLVM AOT Compiler](../accepted/018-llvm-aot-compiler.md)

## Summary

This document proposes introducing an **optimization level system** and a **Pass manager** to YaoXiang, transforming compilation optimization from an "all-or-nothing" approach into a configurable optimization package. Optimization levels (O0–O3) define different combinations of optimization strategies, and the Pass manager is responsible for executing optimization passes in dependency order. This document also defines a standard interface for optimization passes, providing an architectural foundation for future extensions (monomorphization, inlining, constant folding, etc.).

**Core goal: enable users to make explicit trade-offs between compilation speed, binary size, and runtime performance.**

## Motivation

### Why do we need optimization levels?

The current compiler has no optimization configuration; all code goes through the same processing pipeline. This causes:

1. **Poor debugging experience**: Debugging does not require optimization, but it cannot be turned off
2. **No control over binary size**: Generic monomorphization bloats the binary, but cannot be disabled
3. **Uncontrollable compilation speed**: Cannot choose "fast compile" or "deep optimization" based on the scenario
4. **Disordered optimization passes**: Future optimization passes have dependencies on each other and need unified management

### Current problems

```yaoxiang
# Currently: all code goes through the same processing
# - During debugging: optimization is not needed, but cannot be turned off
# - In production: optimization is needed, but depth cannot be configured
# - Generic functions: multiple copies of code are generated, but cannot be controlled

identity: (T: Type) -> (x: T) -> T = (x) => x
x = identity(42)        # generates identity_Int
s = identity("hello")   # generates identity_String
# Users cannot choose "do not monomorphize" (type erasure mode)
```

### The value of optimization levels

| Scenario | Requirement | Optimization Level |
|----------|-------------|--------------------|
| Development & debugging | Fast compile, preserve debug info | O0 |
| Daily development | Basic optimization, balanced compile speed | O1 |
| Testing / CI | Standard optimization, validate production behavior | O2 |
| Production release | Deep optimization, peak performance | O3 |
| Scripts / rapid prototyping | Automatic selection (based on target platform) | Auto |

## Proposal

### Core design

#### 1. Optimization level definition

```rust
/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OptLevel {
    /// O0: No optimization (debug mode)
    /// - Preserve all debug info
    /// - Perform no optimization transforms
    /// - Fastest compile speed
    /// - Use case: development debugging, rapid iteration
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
    /// - All optimization passes
    /// - May increase compile time and binary size
    /// - Use case: peak performance requirements
    O3,

    /// Auto: Automatic selection
    /// - Automatically choose optimization strategy based on target platform and available resources
    /// - Use case: scripts, rapid prototyping
    Auto,
}
```

#### 2. Optimization pass interface

```rust
/// Optimization pass interface
pub trait OptimizationPass {
    /// Pass name (used for logging and dependency declaration)
    fn name(&self) -> &str;

    /// Run the pass
    fn run(&self, module: &mut ModuleIR, config: &PassConfig) -> PassResult;

    /// Which other passes must run before this one
    fn dependencies(&self) -> Vec<&str> {
        vec![]
    }

    /// Whether this pass should run under the current configuration
    fn should_run(&self, config: &PassConfig) -> bool {
        true
    }
}

/// Pass configuration
#[derive(Debug, Clone)]
pub struct PassConfig {
    /// Optimization level
    pub opt_level: OptLevel,
    /// Whether to enable debug info
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
    /// Number of functions inlined
    pub functions_inlined: usize,
    /// Number of functions monomorphized
    pub functions_monomorphized: usize,
    /// Number of dead code blocks removed
    pub dead_code_removed: usize,
    /// Number of constants folded
    pub constants_folded: usize,
}
```

#### 3. Pass manager

```rust
/// Optimizer
pub struct Optimizer {
    /// Registered pass list (sorted by dependency order)
    passes: Vec<Box<dyn OptimizationPass>>,
}

impl Optimizer {
    /// Create an optimizer for a given optimization level
    pub fn for_opt_level(level: OptLevel) -> Self {
        let passes = Self::create_passes_for_level(level);
        Self { passes }
    }

    /// Create the pass list for a given level
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
                // Automatic selection: decide based on target platform
                Self::create_passes_for_level(OptLevel::O1)
            }
        }
    }

    /// Run all optimization passes
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

#### Command-line usage

```bash
# Debug mode: no optimization
yaoxiang build --opt-level O0

# Daily development: basic optimization (default)
yaoxiang build

# Production release: standard optimization
yaoxiang build --opt-level O2

# Peak performance: aggressive optimization
yaoxiang build --opt-level O3

# Automatic selection
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

No syntax changes. The optimization level is a compiler configuration and does not affect language syntax.

## Detailed design

### Optimization level to pass mapping

| Pass | O0 | O1 | O2 | O3 | Description |
|------|----|----|----|----|-------------|
| **Constant folding** | Minimal | Basic | Full | Full | Evaluate constant expressions at compile time |
| **Monomorphization** | ❌ | On-demand | On-demand | Full | Generic function specialization |
| **Dead code elimination** | ❌ | Basic | Full | Full | Remove unused code |
| **Function inlining** | ❌ | ❌ | Small functions | Aggressive | Insert function body at call site |
| **Tail call optimization** | ❌ | ❌ | ✅ | ✅ | Convert tail recursion to loops |
| **Escape analysis** | ❌ | ❌ | ❌ | ✅ | Decide stack/heap allocation |
| **Loop optimization** | ❌ | ❌ | ❌ | ✅ | Loop unrolling, hoisting invariants |

### Monomorphization strategy

```rust
/// Monomorphization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MonoStrategy {
    /// No monomorphization — type erasure; generic functions have only one copy of code
    /// Pros: small binary, fast compile
    /// Cons: runtime dynamic dispatch overhead
    Erased,

    /// On-demand monomorphization — generate code only for actually used type combinations
    /// Pros: zero-cost abstraction, no runtime overhead
    /// Cons: binary may bloat
    #[default]
    OnDemand,

    /// Full monomorphization — pre-generate all possible type combinations
    /// Pros: all calls determined at compile time
    /// Cons: slow compile, large binary
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

    /// Maximum specialization depth (prevent infinite recursive generics)
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

        // 2. Run optimization passes based on optimization level
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

### Impact on the type system

No direct impact. Optimization passes run at the IR layer and do not affect the type system.

### Runtime behavior

| Optimization level | Runtime behavior |
|--------------------|------------------|
| O0 | No optimization, preserve all debug info |
| O1 | Basic optimization, preserve basic debug info |
| O2 | Standard optimization, no debug info |
| O3 | Aggressive optimization, no debug info |

**Key point: the runtime does not need to be modified.** Optimization passes only affect the IR layer and code generation layer; the runtime looks up execution by function name/ID and is unaware of the optimization process.

### Compiler changes

| Component | Change |
|-----------|--------|
| `frontend/config.rs` | Add `OptLevel` enum and `MonoConfig` |
| `frontend/pipeline.rs` | Integrate pass manager |
| `middle/passes/optimizer/` | Add optimization pass module |
| `middle/passes/mono/` | Refactor to standard pass interface |
| CLI | Add `--opt-level` parameter |

### Backward compatibility

- ✅ Fully backward compatible
- Default optimization level is O1, behavior matches current
- Users can explicitly specify an optimization level to override the default

## Trade-offs

### Advantages

- **Flexibility**: users can choose optimization strategy based on the scenario
- **Extensibility**: standard pass interface, easy to add new optimizations
- **Predictability**: clear behavior for each optimization level
- **Debug-friendly**: O0 mode preserves complete debug info

### Disadvantages

- **Increased complexity**: multiple optimization levels to maintain
- **Larger test matrix**: behavior at each optimization level needs testing
- **Documentation burden**: the meaning of each optimization level must be explained

## Alternatives

| Option | Why not chosen |
|--------|----------------|
| Only on/off states | Cannot finely control optimization depth |
| Use GCC/LLVM-style `-O` numbers | Inconsistent with YaoXiang's configuration system |
| Independent toggle per optimization pass | Users need to understand the details of each pass, complex to use |
| Defer to v2.0 | Monomorphization is implemented but not integrated; the architectural problem must be solved first |

## Implementation strategy

### Phases

1. **Phase 1 (current)**: Define optimization levels and pass interface
2. **Phase 2**: Implement monomorphization pass (based on existing `mono/` module)
3. **Phase 3**: Implement constant folding and dead code elimination passes
4. **Phase 4**: Implement function inlining and tail call optimization passes
5. **Phase 5**: Implement aggressive optimization passes (escape analysis, loop optimization)

### Dependencies

- Depends on RFC-011 (Generic Type System)'s monomorphization module
- Depends on RFC-028 (JIT Compiler)'s optimization pass interface
- Shares optimization pass design with RFC-018 (LLVM AOT)

### Risks

- **Performance regression**: optimization passes may introduce bugs that degrade performance
- **Increased compile time**: optimization passes add to compile time
- **Binary bloat**: monomorphization may significantly increase binary size

## Open questions

- [ ] Should O3 enable escape analysis by default? (@晨煦: needs performance test data)
- [ ] Do we need `Os` (optimize for size) and `Oz` (extreme size optimization) levels?
- [ ] Should optimization level affect the verbosity of debug info?
- [ ] How to handle circular dependencies between optimization passes?

---

## Appendix A: Design decision records

| Decision | Resolution | Date | Recorder |
|----------|------------|------|----------|
| Optimization level naming | Use O0–O3 + Auto | 2026-06-16 | 晨煦 |
| Default optimization level | O1 (basic optimization) | 2026-06-16 | 晨煦 |
| Monomorphization strategy | Support Erased / OnDemand / Full | 2026-06-16 | 晨煦 |
| Pass interface design | trait + dependency declaration | 2026-06-16 | 晨煦 |

---

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| **Optimization pass** | An independent module that performs one transformation on the IR |
| **Monomorphization** | A code generation strategy that specializes generic functions into concrete types |
| **Constant folding** | Evaluating constant expressions at compile time |
| **Dead code elimination** | Removing unreachable or unused code from the program |
| **Function inlining** | Inserting a function body at the call site to avoid call overhead |
| **Tail call optimization** | Converting tail recursion to a loop to avoid stack overflow |
| **Escape analysis** | Analyzing whether a variable escapes its scope to decide stack/heap allocation |

---

## References

- [Rust Compiler Optimizations](https://rustc-dev-guide.rust-lang.org/optimizations.html)
- [GCC Optimization Options](https://gcc.gnu.org/onlinedocs/gcc/Optimize-Options.html)
- [LLVM Pass Manager](https://llvm.org/docs/WritingAnLLVMNewPMPass.html)
- [V8 TurboFan Optimization Pipeline](https://v8.dev/docs/turbofan)

---

## Lifecycle and destination

This RFC defines the architectural design of optimization levels and provides a unified framework for subsequent optimization passes.

**Relationship to monomorphization**: monomorphization is one of the optimization passes and will be the first implemented pass once this RFC is accepted.