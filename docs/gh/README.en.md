# YaoXiang (爻象) Programming Language

> An experimental general-purpose programming language that integrates the power of type theory, ownership model, and natural syntax.
>
> Based on "Concurrent Model: All Things Work Together, and We Observe the Return"

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-v0.7.0--experimental-blue.svg)]()

> **Language: [中文](../README.md)**

---

## Introduction

YaoXiang (爻象) is an **experimental programming language under active development**, designed to explore the fusion of type theory, ownership models, and natural syntax.

> **Project Status: Experimental Validation**
> This is a research project for learning compiler development. The implementation is incomplete and not production-ready.

### Core Design Goals

| Goal | Description |
|------|-------------|
| **Everything is Type** | Values, functions, modules, generics — all are types. Types are first-class citizens |
| **Unified Syntax** | Everything is `name: type = value` — one rule covers all declarations |
| **Natural Syntax** | Python-like readability, close to natural language |
| **Ownership Model** | Move semantics + borrow tokens + ref sharing — no GC, no lifetime annotations |
| **Concurrent Model** | Synchronous syntax, asynchronous essence (design phase, not yet implemented) |
| **Value-Dependent Types** | Types can depend on values, enabling compile-time dimension verification |

### Code Example

```yaoxiang
# === Type Definitions (unified: name: type = value) ===

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

# === Functions ===

add: (a: Int, b: Int) -> Int = a + b

# Generic function
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# === Ownership Model ===

# Move (default): zero-copy ownership transfer
p1 = Point(1.0, 2.0)
p2 = p1              # Move, p1 no longer readable

# &T / &mut T tokens: zero-cost compile-time access proofs
p2.print()           # compiler auto-creates &Point token
p2.shift(1.0, 1.0)  # compiler auto-creates &mut Point token

# ref: shared ownership (compiler auto-selects Rc or Arc)
shared = ref p2      # cross-scope sharing

# clone(): explicit deep copy
backup = p2.clone()

# === Entry Point ===

main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

For more examples, see [docs/examples/](docs/examples/).

### Ownership Model

YaoXiang uses a five-level ownership gradient — no GC, no lifetime annotations:

```
&T / &mut T       Move            ref            clone()         unsafe
    |                |               |               |               |
borrow token      default         shared          deep copy       raw ptr
zero-cost         zero-copy       auto Rc/Arc     explicit        system-level
```

| Operation | Cost | When to Use |
|-----------|------|-------------|
| `&T` / `&mut T` | Zero (compile-time tokens) | Read-only or exclusive mutable access |
| Move | Zero (pointer move) | Default — assignment, function args, returns |
| `ref` | Low (Rc) / Medium (Arc) | Cross-scope shared ownership |
| `clone()` | Type-dependent | Explicit independent copy |
| `unsafe` + `*T` | Zero (raw memory) | FFI, system-level programming |

**Key design decisions:**
- No lifetime annotations (`'a`) — tokens are values managed by RAII
- No borrow checker — type properties (Dup/Linear) handle permissions naturally
- No GC — deterministic resource management
- Compiler auto-selects Rc (single-threaded) vs Arc (cross-thread) for `ref`

### Installation & Building

```bash
# Clone and build (development build)
git clone https://github.com/yaoxiang-lang/yaoxiang.git
cd yaoxiang
cargo build

# Run tests to see current status
cargo test

# Try the examples (some may not work)
cargo run --example hello
```

### Current Working Features

```bash
# Basic tokenization and parsing only
echo 'main: () -> Void = { print("Hello") }' | cargo run -- eval

# Build bytecode (partial implementation)
cargo run -- build docs/examples/hello.yx -o hello.42

# Dump bytecode for debugging
cargo run -- dump docs/examples/hello.yx
```

### Project Structure

```
yaoxiang/
├── Cargo.toml              # Project configuration
├── README.md               # This file (Chinese)
├── LICENSE                 # MIT License
├── src/                    # Source code
│   ├── main.rs             # CLI entry point
│   └── lib.rs              # Library entry point
├── docs/                   # Documentation
│   ├── src/
│   │   ├── design/         # Design documents
│   │   │   ├── rfc/        # RFC proposals
│   │   │   │   ├── accepted/   # Accepted RFCs
│   │   │   │   └── draft/      # Draft RFCs
│   │   │   └── manifesto.md    # Design manifesto
│   │   ├── reference/      # Language reference
│   │   │   └── language-spec/  # Language specification
│   │   ├── guide/          # User guides
│   │   ├── tutorial/       # Tutorials (zh/en)
│   │   ├── blog/           # Blog posts
│   │   └── dev/            # Developer docs
│   ├── examples/           # Example code
│   └── gh/                 # GitHub-specific docs (this file)
└── tests/                  # Tests
```

### Design Philosophy

YaoXiang's design philosophy can be summarized in five principles:

```
Everything is Type → Unified Abstraction → Type as Data → Runtime Available
Ownership Model → Zero-Cost Abstraction → No GC → High Performance
Python Syntax → Natural Language → Readability → Beginner-Friendly
Concurrent Model → Lazy Evaluation → Auto Parallel → Seamless Concurrency
Send/Sync → Compile-Time Check → Data Race → Thread Safety
```

### Comparison with Existing Languages

| Feature | YaoXiang | Rust | Python | TypeScript | Go |
|---------|----------|------|--------|------------|-----|
| Everything is Type | Yes | No | No | No | No |
| Auto Type Inference | Yes | Yes | Yes | Yes | Yes |
| Default Immutable | Yes | Yes | No | No | No |
| Ownership Model | Yes | Yes | No | No | No |
| Concurrent Model | Yes | No | No | No | No |
| Zero-Cost Abstraction | Yes | Yes | No | No | No |
| No GC | Yes | Yes | No | No | No |
| Compile-Time Thread Safety | Yes | Yes | No | No | No |
| Value-Dependent Types | Yes | No | No | No | No |
| Keyword Count | 17 | 51+ | 35 | 64+ | 25 |

> **Concurrent Model** = Synchronous Syntax + Lazy Evaluation + Implicit Parallel + Seamless Async (compiler auto-parallelizes, no manual thread management)

### Key RFCs

| RFC | Title | Description |
|-----|-------|-------------|
| [RFC-009](docs/src/design/rfc/accepted/009-ownership-model.md) | Ownership Model | Move + borrow tokens + ref — no GC, no lifetimes |
| [RFC-010](docs/src/design/rfc/accepted/010-unified-type-syntax.md) | Unified Type Syntax | Everything is `name: type = value` |
| [RFC-011](docs/src/design/rfc/accepted/011-generic-type-system.md) | Generic System | Value-dependent types with zero-cost abstraction |

### Roadmap

For detailed implementation status and future plans, see [Implementation Roadmap](docs/plan/IMPLEMENTATION-ROADMAP.md).

### Contributing

Contributions are welcome! Please read the [Contribution Guide](CONTRIBUTING.md).

### Community

- GitHub Issues: Feature suggestions, bug reports
- Discussions: Discussion and exchange

### License

This project uses the MIT License. See [LICENSE](LICENSE) for details.

### Acknowledgments

YaoXiang's design is inspired by the following projects and languages:

- **Rust** - Ownership model, zero-cost abstraction
- **Python** - Syntax style, readability
- **Idris/Agda** - Dependent types, type-driven development
- **TypeScript** - Type annotations, runtime types
- **MoonBit** - AI-friendly design

### Yes, It's Still an Experimental Project

Before you criticize, check this out:

- [YaoXiang Design Manifesto (Satirical Version)](docs/src/design/manifesto-wtf.md) - DeepSeek's Review

> "The One generates two, two generates three, three generates all things."
> — Tao Te Ching
>
> Types are like the Way, all things are born from them.

---

> **Main documentation: [中文](../README.md)**
