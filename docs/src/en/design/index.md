# YaoXiang Design Document

> The Tao gives birth to One, One gives birth to Two, Two gives birth to Three, Three gives birth to all things.

This directory contains design decisions, proposals, and discussions for the YaoXiang programming language.

## Core Design Principles

| Principle | Description |
|-----------|-------------|
| **Everything is a type** | Values, functions, and modules are all types; types are first-class citizens |
| **Natural syntax** | Python-like readability, close to natural language |
| **Ownership model** | Zero-cost abstraction, no GC, high performance |
| **Spawn model** | Synchronous syntax, asynchronous nature, automatic parallelism |
| **AI-friendly** | Strictly structured, clear AST |

## Design Document Structure

```
design/
├── index.md              # This index
├── deprecated/           # Deprecated (superseded by new design)
│   └── *.md
├── rejected/             # Rejected
│   └── *.md
├── rfc/
│   ├── draft/            # Draft (work in progress)
│   ├── review/           # Under review (open for discussion)
│   ├── accepted/         # Accepted (design passed)
│   ├── deprecated/       # Deprecated (superseded)
│   └── rejected/         # Rejected (not passed)
└── discussion/           # Design discussion area (open for discussion)
    └── *.md
```

## Accepted Design Proposals

| Document | Status | Description |
|----------|--------|-------------|
| [RFC-001 Concurrent Model Error Handling](./rfc/accepted/001-concurrent-model-error-handling.md) | ✅ Accepted | Error handling design in the concurrent model |
| [RFC-008 Runtime Concurrency Model](./rfc/accepted/008-runtime-concurrency-model.md) | ✅ Accepted | Spawn model and task scheduler design |
| [RFC-009 Ownership Model](./rfc/accepted/009-ownership-model.md) | ✅ Accepted | Ownership and borrowing system design |
| [RFC-010 Unified Type Syntax](./rfc/accepted/010-unified-type-syntax.md) | ✅ Accepted | Unified type definition syntax |
| [RFC-011 Generic Type System](./rfc/accepted/011-generic-type-system.md) | ✅ Accepted | Generic type system design |

> See the [`rfc/accepted/`](./rfc/accepted/) directory for the complete list.

## RFC Proposals

> RFC (Request for Comments) is the proposal process for new features and major changes.

### Active Proposals

| ID | Title | Status |
|----|-------|--------|
| RFC-003 | Version Planning | Under Review |
| RFC-016 | Quantum-Native Support | Draft |
| RFC-018 | LLVM AOT Compiler | Under Review |
| RFC-019 | Typed Homoiconicity | Draft |
| RFC-020 | Dynamic Module FFI | Draft |
| RFC-021 | Library-Driven FFI Extension | Under Review |
| RFC-022 | Hoare Logic Static Verification | Under Review |

### Accepted Proposals

| ID | Title | Status |
|----|-------|--------|
| RFC-001 | Concurrent Model Error Handling | Accepted |
| RFC-004 | Curried Multi-Position Binding | Accepted |
| RFC-006 | Documentation Site Optimization | Accepted |
| RFC-007 | Unified Function Syntax | Accepted |
| RFC-008 | Runtime Concurrency Model | Accepted |
| RFC-009 | Ownership Model | Accepted |
| RFC-010 | Unified Type Syntax | Accepted |
| RFC-011 | Generic Type System | Accepted |
| RFC-012 | f-string Template Strings | Accepted |
| RFC-013 | Error Code Specification | Accepted |
| RFC-014 | Package Manager | Accepted |
| RFC-015 | Configuration System | Accepted |
| RFC-017 | LSP Support | Accepted |
| RFC-023 | Closure Capture Model | Accepted |

### Rejected Proposals

| ID | Title | Status |
|----|-------|--------|
| RFC-002 | Cross-Platform IO (libuv) | Rejected |
| RFC-005 | Automated CVE Scanning | Rejected |

### RFC Template

Before submitting a new proposal, please refer to:
- [RFC_TEMPLATE.md](./rfc/RFC_TEMPLATE.md)
- [Full Example](./rfc/EXAMPLE_full_feature_proposal.md)

## Participating in Design Discussions

### RFC Lifecycle

RFC proposals have 5 statuses:

| Status | Meaning |
|--------|---------|
| Draft | Work in progress |
| Under Review | Open for discussion |
| Accepted | Design passed |
| Deprecated | Was accepted, superseded by new design |
| Rejected | Not passed |

Full lifecycle:
```
Draft → Under Review → Accepted → Deprecated (superseded)
                  ↓
              Rejected (not passed)
```

### Proposal Process

```
1. Draft the proposal (use RFC template)
   → Put in rfc/draft/

2. Submit for review
   → Move to rfc/review/, open community discussion

3. Core team review
   → Accept → Move to rfc/accepted/
   → Reject → Move to rfc/rejected/

4. Ongoing maintenance
   → Superseded → Move to rfc/deprecated/
```

### Design Principles

- **Clear boundaries**: Each design decision should have a clear scope of application
- **Practicality first**: Solve real problems, not hypothetical threats
- **User-visible behavior unchanged**: Never break userspace

## Code Examples

```yaoxiang
// Type definition
Point: Type = { x: Float, y: Float }
Result: Type(T, E) = { ok(T) | err(E) }

// Function definition
add: (a: Int, b: Int) -> Int = a + b

// Main function
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## Key Design Decisions

### 1. Type System

- **Unified type syntax**: Abolish `enum`, `struct`, `union`, unify with `Name: Type = {...}`
- **Constructors are types**: Eliminate the gap between "type" and "value"
- **Generic support**: Compile-time monomorphization, zero runtime overhead

### 2. Spawn Model

```yaoxiang
// Spawn model: sequential by default, spawn introduces dataflow parallelism

// Sequential execution by default
compute: (Int) -> Int = (n) => {
    a = heavy_calc(1)
    b = heavy_calc(2)  // Sequential, wait for a to complete
    c = heavy_calc(3)  // Sequential, wait for b to complete
    a + b + c
}

// Spawn block introduces dataflow parallelism
process: () -> Void = () => {
    spawn {
        users = fetch_users()   // Parallel
        posts = fetch_posts()   // Parallel
    }
    // Caller blocks synchronously waiting for results
    render(users, posts)
}
```

### 3. Error Handling

```yaoxiang
Result: Type(T, E) = { ok(T) | err(E) }

process: () -> Result(Data, Error) = {
    data = fetch_data()?      // ? operator propagates transparently
    transformed = transform(data)?
    save(transformed)?
}
```

## Related Resources

- [Tutorial](../tutorial/) - Learn to use YaoXiang
- [Reference Documentation](../reference/) - API and standard library
- [Language Specification](../reference/language-spec/index.md) - Complete language specification
- [GitHub Discussions](https://github.com/ChenXu233/YaoXiang/discussions)
- [Contribution Guide](../tutorial/contributing.md)

## Historical Archive

Historical documents from the design process have been moved to the [`docs/old/`](../../old/) directory, including:
- Early architecture design
- Deprecated proposals
- Outdated implementation plans