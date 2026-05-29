---
title: Closure Capture Model Implementation Design
status: draft
created: 2026-05-29
---

# Closure Capture Model Implementation Design

## Goal

Implement analysis of closure capture of external variables, capture mode selection, and IR generation.

## Core Rules

```
Variable Type    Closure Escapes    Capture Mode
─────────────────────────────────────────────────
Dup              Any                Copy (zero-cost, no side effects)
Non-Dup          Non-escaping       Automatic Borrow (&T or &mut T token)
Non-Dup          Escaping           Move (ownership transfer)
```

This set of rules is the **same logic** as automatic borrow selection for function calls. No new concepts introduced.

## Implementation Checklist

### Step 1: Escape Analysis

**File**: `src/frontend/core/typecheck/inference/expressions.rs` (or create new `capture.rs`)

Definition of closure "escape":

```rust
enum ClosureUsage {
    Inline,    // Called immediately or passed to synchronous function, non-escaping
    Escaping,  // spawn, return, stored on heap, stored globally
}
```

Escape determination rules:

```
lambda as argument to spawn { ... }              → Escaping
lambda as return value                           → Escaping
lambda assigned to external variable/field       → Escaping
lambda passed to function parameter (non-spawn)  → Inline (conservative)
lambda called immediately                        → Inline
```

**Conservative Principle**: When uncertain, treat as Escaping.

### Step 2: Captured Variable Analysis

**Traverse the closure body AST** to find references to variables in the closure's outer scope.

```rust
struct CaptureInfo {
    captures: Vec<CapturedVar>,
}

struct CapturedVar {
    name: String,           // Variable name
    usage: CaptureUsage,    // Usage pattern
}

enum CaptureUsage {
    Read,           // Read-only (only needs &T)
    Write,          // Read-write (needs &mut T)
    Move,           // Ownership transfer (non-Dup + escaping)
    DupCopy,        // Dup type direct copy
}
```

**Analysis Process**:

1. Traverse the lambda body AST
2. Record all `Expr::Var(name)` references
3. Filter: keep only variables in the closure's outer scope
4. Classify by usage pattern:
   - Assignment/call to mut method → Write
   - Read-only → Read
   - Moved to somewhere else → Move

### Step 3: Capture Mode Selection

```rust
fn determine_capture_mode(
    var: &CapturedVar,
    ty: &MonoType,
    usage: ClosureUsage,
    is_dup: bool,
) -> CaptureMode {
    match (is_dup, usage) {
        // Dup type: direct copy—simplest path
        (true, _) => CaptureMode::Copy,
        
        // Non-Dup + escaping → Move
        (false, ClosureUsage::Escaping) => CaptureMode::Move,
        
        // Non-Dup + non-escaping → automatic borrow
        (false, ClosureUsage::Inline) => match var.usage {
            CaptureUsage::Read => CaptureMode::Borrow,     // &T
            CaptureUsage::Write => CaptureMode::BorrowMut, // &mut T
            CaptureUsage::Move => CaptureMode::Move,
            CaptureUsage::DupCopy => unreachable!(),
        },
    }
}

enum CaptureMode {
    Copy,       // Direct value copy
    Borrow,     // &T token
    BorrowMut,  // &mut T token
    Move,       // Ownership transfer
}
```

**Key Scenarios**:

```yaoxiang
# 1. &T token passed to closure—Dup → Copy, zero-cost
threshold: &Float = &some_float
items.filter(|p| p.x > threshold)
# threshold: &Float → Dup → CaptureMode::Copy
# Compiler: copies token (zero size, zero runtime overhead)

# 2. Non-Dup value, closure non-escaping—automatic borrow
buffer: Buffer = ...
process(|b| b.read())
# buffer not Dup, closure non-escaping → CaptureMode::Borrow
# Compiler: automatically creates &Buffer token passed to closure

# 3. Closure escaping—Move
big_data: Data = ...
spawn { use(big_data) }
# big_data not Dup, spawn → Escaping → CaptureMode::Move
```

### Step 4: IR Generation

**File**: `src/middle/core/ir_gen.rs`

```rust
// Current (empty implementation)
Instruction::MakeClosure {
    dst: Operand::Local(result_reg),
    func: closure_name,
    env: Vec::new(),  // ← always empty
}

// Changed to
Instruction::MakeClosure {
    dst: Operand::Local(result_reg),
    func: closure_name,
    env: captured_vars,  // Vec<(Operand, CaptureMode)>
}
```

IR generation for each captured variable:

```rust
for captured in &captures {
    let src = self.lookup_local(&captured.name);
    match captured.mode {
        CaptureMode::Copy => {
            // Dup type: Mov instruction copy (zero-cost optimization see Step 5)
            instructions.push(Instruction::Move {
                dst: new_temp(),
                src,
            });
        }
        CaptureMode::Borrow => {
            // Automatic borrow: create ReadToken
            instructions.push(Instruction::Borrow {
                dst: new_temp(),
                src,
                mutable: false,
            });
        }
        CaptureMode::BorrowMut => {
            instructions.push(Instruction::Borrow {
                dst: new_temp(),
                src,
                mutable: true,
            });
        }
        CaptureMode::Move => {
            // Move: ownership transfer
            instructions.push(Instruction::Move {
                dst: new_temp(),
                src,
            });
        }
    }
}
```

### Step 5: ZST Optimization—Token Elimination

When `CaptureMode::Copy` is used for `&T`, the `&T` is a zero-sized type. `Instruction::Move` copies zero bytes of data → **needs to be eliminated in IR optimization pass**.

Two implementation approaches:

**Approach A: Skip at IR Generation**
```rust
CaptureMode::Copy if is_zero_sized_type(ty) => {
    // Generate no IR instruction
    // Closure body directly references outer variable (compile-time)
}
```

**Approach B: IR Optimization Pass**
```rust
// New ZstElimination pass:
// Scan all Move dst, src; if src type is ZST, delete that instruction
// Replace dst with src (aliasing)
```

**Recommended Approach A**—knowing it's ZST at generation time, no subsequent optimization needed.

### Step 6: Borrow Token Conflict Detection

After a closure captures a `&mut T` token, the original scope cannot use that token simultaneously:

```yaoxiang
tok = &mut point        # WriteToken created
closure = |x| {
    tok.shift(x, 0.0)   # tok borrowed by closure
}
tok.shift(1.0, 0.0)     # ❌ Compile error: tok's WriteToken already held by closure
```

This is covered by existing token conflict detection (RFC-009 v9 Section 2.6)—the borrow checker handles it in flow-sensitive liveness analysis.

## File Change Checklist

| # | File | Changes |
|---|------|---------|
| 1 | `typecheck/inference/capture.rs` (new) | Capture analysis + escape analysis + mode selection |
| 2 | `typecheck/inference/expressions.rs` | Lambda type inference calls capture analysis |
| 3 | `middle/core/ir_gen.rs` | MakeClosure env population, ZST skip |
| 4 | `middle/core/ir.rs` | May need Borrow instruction (if IR requires it) |
| 5 | `middle/passes/lifetime/mod.rs` | Register closure-related borrow checks (if new checks needed) |

Estimated total changes: ~300 lines.

## Implementation Order

1. **Capture analysis** (capture.rs)—pure AST traversal, returns captured variable list
2. **Escape analysis**—determines whether closure escapes
3. **Mode selection**—decides CaptureMode based on Dup/Non-Dup + Escaping/Non-escaping
4. **IR generation**—populate MakeClosure env
5. **ZST optimization**—Dup + ZST skip IR instruction

Steps 1-3 are pure type-checking layer (frontend). Steps 4-5 are IR generation layer (middle-end). Can be implemented separately.

## Verification Scenarios

```yaoxiang
# ✅ Scenario 1: Dup token copy (most critical scenario)
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}

# ✅ Scenario 2: Non-Dup automatic borrow
process_buffer: (buf: Buffer) -> Void = {
    transform(|b| b.read())  # buf non-escaping → &T borrow
}

# ✅ Scenario 3: Cross-task forced Move
spawn_worker: (data: Data) -> Void = {
    spawn { use(data) }  # escaping → Move
}

# ❌ Scenario 4: Borrow + subsequent use conflict
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf already borrowed by closure
}
```

## References

- [RFC-009 v9 Ownership Model](../../design/rfc/accepted/009-ownership-model.md) — Borrow token system
- [RFC-007 Function Syntax Unification](../../design/rfc/accepted/007-function-syntax-unification.md) — lambda definition
- Investigation Report: IR Generation Gap (MakeClosure env empty implementation)