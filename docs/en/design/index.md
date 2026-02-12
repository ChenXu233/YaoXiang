# YaoXiang Design Documents

> 「道生一，一生二，二生三，三生万物。」

This directory contains design decisions, proposals, and discussions for the YaoXiang programming language.

## Core Design Principles

| Principle | Description |
|-----------|-------------|
| **Everything is a Type** | Values, functions, and modules are all types; types are first-class citizens |
| **Natural Syntax** | Python-like readability, close to natural language |
| **Ownership Model** | Zero-cost abstraction, no GC, high performance |
| **Concurrency Model** | Synchronous syntax, asynchronous essence, automatic parallelism |
| **AI-Friendly** | Strictly structured, clear AST |

## Design Document Structure

```
design/
├── index.md              # This index
├── accepted/             # Accepted design proposals
│   └── *.md
├── rfc/                  # RFC proposals (pending review)
│   ├── *.md
│   └── RFC_TEMPLATE.md
└── discussion/           # Design discussion area (open discussion)
    └── *.md
```

## Accepted Design Proposals

| Document | Status | Description |
|----------|--------|-------------|
| [008-Concurrency Model](./accepted/008-runtime-concurrency-model.md) | ✅ Stable | Concurrency model and task scheduler design |

> See the [`accepted/`](./accepted/) directory for the complete list.

## RFC Proposals

> RFC (Request for Comments) is the proposal process for new features and major changes.

### Active Proposals

| ID | Title | Status |
|----|-------|--------|
| RFC-003 | Version Planning | Pending Review |
| RFC-005 | Automated CVE Scanning | Pending Review |
| RFC-006 | Documentation Site Optimization | Pending Review |
| RFC-012 | f-string Template Strings | Pending Review |

### RFC Templates

Before submitting a new proposal, please refer to:
- [RFC_TEMPLATE.md](./rfc/RFC_TEMPLATE.md)
- [Full Example](./rfc/EXAMPLE_full_feature_proposal.md)

## Participating in Design Discussions

### Proposal Process

```
1. Draft proposal (using RFC template)
   → Place in rfc/ directory

2. Community discussion
   → Discuss in corresponding issue in rfc/REPO

3. Core team review
   → Accept → Move to accepted/
   → Reject → Move to archived/ or delete
```

### Design Principles

- **Clear Boundaries**: Each design decision should have a clear scope of application
- **Practicality First**: Solve real problems, not hypothetical threats
- **Progressive Transparency**: Layered concurrency model design (L1-L3)
- **User-Visible Behavior Invariant**: Never break userspace

## Code Examples

```yaoxiang
# Type Definitions
type Point = { x: Float, y: Float }
type Result[T, E] = { ok(T) | err(E) }

# Function Definitions
add: (a: Int, b: Int) -> Int = a + b

# Concurrent Functions (Automatic Concurrency)
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

# Main Function
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## Key Design Decisions

### 1. Type System

- **Unified Type Syntax**: Abolish `enum`, `struct`, `union`, unify with `type`
- **Constructors as Types**: Bridge the gap between "types" and "values"
- **Generic Support**: Compile-time monomorphization, zero runtime overhead

### 2. Concurrency Model

```yaoxiang
# Three-layer concurrency abstraction

# L1: @blocking sync (disable parallelism)
fetch: (String) -> JSON @blocking = (url) => { ... }

# L2: spawn explicit concurrency
process: () -> Void spawn = () => {
    users = fetch_users()
    posts = fetch_posts()  # Automatic parallelism
}

# L3: Fully transparent (default)
compute: (Int) -> Int = (n) => {
    a = heavy_calc(1)  # System automatically analyzes dependencies
    b = heavy_calc(2)
    c = heavy_calc(3)
    a + b + c
}
```

### 3. Error Handling

```yaoxiang
type Result[T, E] = ok(T) | err(E)

process: () -> Result[Data, Error] = {
    data = fetch_data()?      # ? operator transparently propagates
    transformed = transform(data)?
    save(transformed)?
}
```

## Related Resources

- [Tutorial](../tutorial/) - Learn to use YaoXiang
- [Reference Documentation](../reference/) - API and standard library
- [GitHub Discussions](https://github.com/ChenXu233/YaoXiang/discussions)
- [Contributing Guide](../tutorial/contributing.md)

## Historical Archives

Historical documents from the design process have been moved to the [`docs/old/`](../../old/) directory, including:
- Early architecture designs
- Discarded proposals
- Outdated implementation plans
