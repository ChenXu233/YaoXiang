# YaoXiang (爻象) Programming Language

> AI-assisted compiler development exploration.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-v0.7.8-blue.svg)]()

> 🌐 **Language** | [中文](../../README.md)
>
> ❤️ **Docs** | [Docs Website](https://chenxu233.github.io/YaoXiang/)

## Introduction

YaoXiang (爻象) is **an experimental programming language under active development**, designed to explore the fusion of type theory, ownership models, and natural syntax.

> **Project Status: Experimental Validation**
> This is a research project for learning compiler development. The implementation is incomplete and not production-ready.

### Core Design Goals

| Goal | Description |
|------|-------------|
| **Everything is Type** | Values, functions, modules, generics — all are types; types are first-class citizens |
| **Unified Syntax** | Everything is `name: type = value` — one rule covers all declarations |
| **Natural Syntax** | Python-like readability, close to natural language |
| **Ownership Model** | Move semantics + borrow tokens + ref sharing — no GC, no lifetime annotations |
| **Concurrency Model** | Synchronous syntax, asynchronous essence (design phase, not yet implemented) |
| **Value-Dependent Types** | Types can depend on values, enabling compile-time dimension verification |

## Code Examples

```yaoxiang
# ═══════════ Type Definitions (unified syntax: name: type = value) ═══════════

# Record type
Point: Type = {
    x: Float,
    y: Float,
}

# Generic type
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

# Interface (record where all fields are function types)
Drawable: Type = {
    draw: (Surface) -> Void,
}

# ═══════════ Functions ═══════════

add: (a: Int, b: Int) -> Int = a + b

# Generic function
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# ═══════════ Ownership Model ═══════════

# Move (default): zero-copy ownership transfer
p1 = Point(1.0, 2.0)
p2 = p1              # Move, p1 no longer readable

# &T / &mut T borrow tokens: zero-cost compile-time access permissions
p2.print()           # compiler auto-creates &Point token
p2.shift(1.0, 1.0)  # compiler auto-creates &mut Point token

# ref: shared ownership (compiler auto-selects Rc or Arc)
shared = ref p2      # cross-scope sharing

# clone(): explicit deep copy
backup = p2.clone()

# ═══════════ Method Definitions ═══════════

Point.draw: (self: &Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx
    self.y = self.y + dy
}

# ═══════════ Entry Point ═══════════

main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## Type System

### Unified Syntax Model

YaoXiang has only one declaration form: **`identifier : type = expression`**

| Concept | Notation |
|---------|----------|
| Variable | `x: Int = 42` |
| Function | `add: (a: Int, b: Int) -> Int = a + b` |
| Record type | `Point: Type = { x: Float, y: Float }` |
| Interface | `Drawable: Type = { draw: (Surface) -> Void }` |
| Generic type | `List: (T: Type) -> Type = { data: Array(T), length: Int }` |
| Method | `Point.draw: (self: &Point, s: Surface) -> Void = ...` |

**No `fn`, `struct`, `trait`, `impl` keywords.** `Type` is the only metatype keyword in the language.

See [RFC-010: Unified Type Syntax](docs/src/design/rfc/accepted/010-unified-type-syntax.md).

### Generics & Value-Dependent Types

YaoXiang's generic system supports **types depending on values**, enabling compile-time dimension verification:

```yaoxiang
# Matrix type: dimensions determined at compile time
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# Compile-time computation: factorial(3) = 6
vec: Vec(factorial(3)) = Vec(6)()

# Compile-time dimension verification: mismatched dimensions caught at compile time
# multiply(matrix_2x3, matrix_4x2)  # Compile error: 3 != 4
```

See [RFC-011: Generic Type System](docs/src/design/rfc/accepted/011-generic-type-system.md).

## Installation & Building

```bash
# Clone the project
git clone https://github.com/yaoxiang-lang/yaoxiang.git
cd yaoxiang

# One-command Z3 dependency install (pure Rust, auto-downloads prebuilt package)
cd tools/setup-z3 && cargo run && cd ../..

# Build
cargo build

# Run tests
cargo test

# Try examples
cargo run --example hello
```

> Z3 is the SMT solving module used by the compiler for compile-time predicate proving (RFC-027). `tools/setup-z3` automatically downloads a prebuilt package matching your platform from GitHub Releases to `.z3/`, and writes `.cargo/config.toml`. After first run, `cargo build` works directly. See [RFC-027](docs/src/design/rfc/accepted/027-compile-time-evaluation-types.md).

### Development Environment Setup

We use `pre-commit` to run project checks before each commit (cross-platform). The repo includes `.pre-commit-config.yaml`, which runs `cargo fmt` and `cargo clippy`.

Recommended installation (using `pipx` to avoid polluting global site-packages):

```bash
python3 -m pip install --user pipx
python3 -m pipx ensurepath
pipx install pre-commit
pre-commit install
```

Quick install (without `pipx`):

```bash
python -m pip install --user pre-commit
pre-commit install
```

Run checks locally:

```bash
pre-commit run --all-files
```

## Comparison with Existing Languages

> ⚠️ YaoXiang is in the experimental stage. The "Current" column reflects the real state, and the "Goal" column indicates the design direction.

| Dimension | YaoXiang Current | YaoXiang Goal | Rust | Python | Go | Java | TypeScript |
|-----------|-----------------|---------------|------|--------|----|------|------------|
| Production Ready | ❌ Experimental | ✅ Stable Release | ✅ | ✅ | ✅ | ✅ | ✅ |
| Type Safety | 🚧 Refinement Types + Compile-time Proofs | ✅ Refinement Types + Compile-time Proofs | ✅ | ❌ Dynamic | ❌ Weak | ✅ Strong | 🚧 Has Escape Hatches |
| Runtime Performance | 🚧 Not Measured | ✅ Close to Rust | ✅ | ❌ Slow | ✅ Fast | ✅ Fast | ❌ Slow |
| Learning Curve | 🚧 To Be Verified | ✅ Smooth | ❌ Steep | ✅ Smooth | ✅ Smooth | ✅ Medium | ✅ Medium |
| Dev Experience | 🚧 Basic LSP | ✅ Mature | ✅ rust-analyzer | ✅ Mature | ✅ Mature | ✅ Mature | ✅ Mature |
| Package Management / Ecosystem | ❌ None | ✅ Unified Package Manager | ✅ crates.io | ✅ PyPI | ✅ Standard Library | ✅ Maven | ✅ npm |
| Memory Management | ✅ Ownership Model | ✅ Low-Cost Ownership | ✅ Ownership | ❌ GC | ❌ GC | ❌ GC | ❌ GC |
| Concurrency | 🚧 In Design | ✅ Safe Concurrency | ✅ Send+Sync | ❌ GIL | ✅ goroutine | 🚧 Thread Model | ❌ Single-threaded |
| Compile Speed | 🚧 Fast Check/new | 💥 Very Slow (compile-time proofs) / Incremental Compilation | 💥 Slow / Incremental Compilation | ✅ No Compilation | ✅ Fast | ✅ Fast | 🚧 tsc Slow |
| Startup Speed | 🚧 Interpreter Init + JIT Wait | ✅ Interpreter Mode / AOT Dual Mode | ✅ No Warmup | 🚧 Interpreter Init + JIT Wait | ✅ No Warmup | ❌ JVM Warmup | 🚧 Interpreter Init + JIT Wait |
| Generics / Polymorphism | 🚧 Basic Implementation | ✅ Full Generics | ✅ Trait System | ✅ Duck Typing | ❌ None | ✅ Erasure-based | ✅ Structural |

## Core RFCs

| RFC | Title | Description |
|-----|-------|-------------|
| [RFC-009](docs/src/design/rfc/accepted/009-ownership-model.md) | Ownership Model | Move + borrow tokens + ref — no GC, no lifetimes |
| [RFC-010](docs/src/design/rfc/accepted/010-unified-type-syntax.md) | Unified Type Syntax | Everything is `name: type = value` |
| [RFC-011](docs/src/design/rfc/accepted/011-generic-type-system.md) | Generic System | Value-dependent types, zero-cost abstraction |

## Contributing

Contributions are welcome! Please read the [Contribution Guide](CONTRIBUTING.md).

## Community

- GitHub Issues: Feature requests, bug reports
- Discussions: Discussion and exchange

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).

## Acknowledgments

YaoXiang's design is inspired by the following projects and languages:

- **Rust** — Ownership model, zero-cost abstraction
- **Python** — Syntax style, readability
- **Idris/Agda** — Dependent types, type-driven development
- **TypeScript** — Type annotations, runtime types
- **MoonBit** — AI-friendly design

## Yes, It's Still an Experimental Project

Before you criticize, check this out:

- [YaoXiang Design Manifesto (Satirical Version)](docs/src/design/manifesto-wtf.md) — DeepSeek's Review

> "The One generates two, two generates three, three generates all things."
> — Tao Te Ching
>
> Types are like the Way, all things are born from them.

## 🌟 Star History

<div align="center">
<a href="https://www.star-history.com/?type=date&repos=ChenXu233%2FYaoXiang">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=ChenXu233/YaoXiang&type=date&theme=dark&legend=top-left&sealed_token=wdeU56ITEYJrILAq17aZ5ciE-iqMUTIMhwkf3fvcrGbRz5Ejbm8pRO_Ef8EYVh8vrEGjwcPvDatnTcyNTSetcCPA88yg8Eia_OTa9dNHUVCTeIamCziUCE25ckxdpmGdLjKsS8ZZc2HWXvqhWAezVmpPtMLtc5p92_PX1MFCCtqppFmAndlJV-Ml8Q_C" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=ChenXu233/YaoXiang&type=date&legend=top-left&sealed_token=wdeU56ITEYJrILAq17aZ5ciE-iqMUTIMhwkf3fvcrGbRz5Ejbm8pRO_Ef8EYVh8vrEGjwcPvDatnTcyNTSetcCPA88yg8Eia_OTa9dNHUVCTeIamCziUCE25ckxdpmGdLjKsS8ZZc2HWXvqhWAezVmpPtMLtc5p92_PX1MFCCtqppFmAndlJV-Ml8Q_C" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=ChenXu233/YaoXiang&type=date&legend=top-left&sealed_token=wdeU56ITEYJrILAq17aZ5ciE-iqMUTIMhwkf3fvcrGbRz5Ejbm8pRO_Ef8EYVh8vrEGjwcPvDatnTcyNTSetcCPA88yg8Eia_OTa9dNHUVCTeIamCziUCE25ckxdpmGdLjKsS8ZZc2HWXvqhWAezVmpPtMLtc5p92_PX1MFCCtqppFmAndlJV-Ml8Q_C" />
 </picture>
</a>
</div>
