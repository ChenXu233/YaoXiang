# YaoXiang Design Documents

> One generates two, two generates three, three generates all things.

This directory contains design decisions, proposals, and discussions for the YaoXiang programming language.

## Core Design Principles

| Principle | Description |
|-----------|-------------|
| **Everything is Type** | Values, functions, modules are all types; types are first-class citizens |
| **Natural Syntax** | Python-like readability, close to natural language |
| **Ownership Model** | Zero-cost abstraction, no GC, high performance |
| **Bingzuo Model** | Synchronous syntax, asynchronous nature, automatic parallelism |
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
| [008-Concurrency Model](./accepted/008-runtime-concurrency-model.md) | ✅ Accepted | Bingzuo model and task scheduler design |

> See [`accepted/`](./accepted/) directory for the complete list.

## RFC Proposals

> RFC (Request for Comments) is the proposal process for new features and major changes.

### Active Proposals

| ID | Title | Status |
|----|-------|--------|
| RFC-003 | Version Planning | Pending Review |
| RFC-005 | Automated CVE Scanning | Pending Review |
| RFC-006 | Documentation Site Optimization | Pending Review |
| RFC-012 | f-string Template Strings | Pending Review |

### RFC Template

Before submitting a new proposal, please refer to:
- [RFC_TEMPLATE.md](./rfc/RFC_TEMPLATE.md)
- [Full Example](./rfc/EXAMPLE_full_feature_proposal.md)

## Participating in Design Discussions

### Proposal Process

```
1. Draft proposal (using RFC template)
   → Place in rfc/ directory

2. Community discussion
   → Discuss in corresponding rfc/REPO issue

3. Core team review
   → Accept → Move to accepted/
   → Reject → Move to archived/ or delete
```

### Design Principles

- **Clear Boundaries**: Each design decision should have a clear scope
- **Practical First**: Solve real problems, not imagined threats
- **Progressive Transparency**: Layered concurrency model design (L1-L3)
- **Never Break Userspace**: User-visible behavior must not change

## Code Examples

```yaoxiang
# Type definitions
type Point = { x: Float, y: Float }
type Result[T, E] = { ok(T) | err(E) }

# Function definitions
add: (a: Int, b: Int) -> Int = a + b

# Bingzuo functions (automatic concurrency)
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

# Main function
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

## Key Design Decisions

### 1. Type System

- **Unified Type Syntax**: Abolish `enum`, `struct`, `union`, use `type` instead
- **Constructors as Types**: Eliminate the gap between "type" and "value"
- **Generic Support**: Compile-time monomorphization, zero runtime overhead

### 2. Bingzuo Model

```yaoxiang
# Three-layer concurrency abstraction

# L1: @blocking sync (disable parallelism)
fetch: (String) -> JSON @blocking = (url) => { ... }

# L2: spawn explicit concurrency
process: () -> Void spawn = () => {
    users = fetch_users()
    posts = fetch_posts()  # Automatic parallel
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
    data = fetch_data()?      # ? operator transparent propagation
    transformed = transform(data)?
    save(transformed)?
}
```

## Related Resources

- [Tutorial](../tutorial/) - Learn YaoXiang
- [Reference](../reference/) - API and standard library
- [GitHub Discussions](https://github.com/ChenXu233/YaoXiang/discussions)
- [Contributing Guide](../tutorial/contributing.md)

## Historical Archives

Historical documents from the design process have been moved to [`docs/old/`](../../old/):
- Early architecture designs
- Abandoned proposals
- Outdated implementation plans
