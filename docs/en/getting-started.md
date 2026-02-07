---
title: Quick Start
description: Install and run your first YaoXiang program in 5 minutes
---

# YaoXiang Quick Start

This guide helps you get started with the YaoXiang programming language quickly.

## Installation

### Building from Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/yaoxiang-lang/yaoxiang.git
cd yaoxiang

# Build (debug version, for development testing)
cargo build

# Build (release version, recommended for production)
cargo build --release

# Run tests
cargo test

# Check version
./target/debug/yaoxiang --version
```

**Verify successful installation**:
```bash
./target/debug/yaoxiang --version
# Should output: yaoxiang x.y.z
```

## Your First Program

Create a file `hello.yx`:

```yaoxiang
# hello.yx
use std.io

# Function definition: name: (param: Type, ...) -> return_type = { ... }
main: () -> Void = {
    println("Hello, YaoXiang!")
}
```

Run it:

```bash
./target/debug/yaoxiang hello.yx
# or use release version
./target/release/yaoxiang hello.yx
```

Output:

```
Hello, YaoXiang!
```

## Basic Concepts

### Variables and Types

```yaoxiang
# Automatic type inference
x = 42                    # inferred as Int
name = "YaoXiang"        # inferred as String
pi = 3.14159              # inferred as Float
is_valid = true           # inferred as Bool

# Explicit type annotation (recommended using type centralization convention)
count: Int = 100

# Default immutable (safety feature)
x = 10
x = 20                    # ❌ Compile error! Immutable

# Mutable variables (requires explicit declaration)
mut counter = 0
counter = counter + 1     # ✅ OK
```

### Functions

```yaoxiang
# Function definition syntax
add: (a: Int, b: Int) -> Int = a + b

# Call
result = add(1, 2)        # result = 3

# Single parameter function
inc: (x: Int) -> Int = x + 1
```

## Next Steps

- 📖 Read [Tutorial](/en/tutorial/) to understand core features
- 📚 Check [Reference](/en/reference/) for complete API
- 💡 Check [Design Documents](/en/design/) for core philosophy
