---
title: Chapter 8: Who Owns the Data
---

# Chapter 8: Who Owns the Data

> **Chapter Goal**: Understand the concept of ownership, why data needs "owners", and the concept of empty state

## 8.1 A Problem

Suppose you have a book and lend it to a classmate:

```yaoxiang
# You have a book
book: Book = Book("《三体》")

# You lend the book to a classmate
book_given = book    # Book given to someone else
```

**Questions**:
- Can you still use this book?
- Can the classmate still use this book?
- Who is responsible for returning the book (releasing memory)?

This is the problem **ownership** solves!

## 8.2 What is Ownership?

**Ownership** means every piece of data has an "owner" (owner).

| Rule | Description |
|------|-------------|
| **Rule 1** | Every value has an owner |
| **Rule 2** | Only one owner at a time |
| **Rule 3** | When owner leaves scope, value is automatically cleared |

```yaoxiang
# Example: ownership transfer
p: Point = Point(1.0, 2.0)   # p is the owner of Point

p2 = p                         # ⚠️ Ownership transfer! p becomes empty

# print(p.x)  # ❌ Error! p is no longer the owner
# p2.x = 0.0  # ✅ Correct! p2 is now the owner
```

## 8.3 Why Ownership?

| Method | Advantages | Disadvantages |
|--------|------------|---------------|
| **Manual management** (C/C++) | Flexible | Easy to forget release, double free |
| **Garbage collection** (Java/Python) | Automatic | Has delay, high memory usage |
| **Ownership** (YaoXiang/Rust) | Automatic + safe | Steep learning curve |

**YaoXiang's ownership**:
- ✅ Automatic memory release (no manual management)
- ✅ No garbage collection overhead
- ✅ Memory safety guaranteed (no dangling pointers)

## 8.4 Move

**Move** is the transfer of ownership:

```yaoxiang
# Move example
source: String = "Hello"
target = source              # Move! source becomes empty

# source = "Hello"          # ❌ Error! source is already empty
print(target)                # ✅ Correct! target is the new owner
```

**Diagram**:

```
Before move:
┌─────────────┐     ┌─────────────────┐
│  source    │ ──▶ │   "Hello"       │
│  "Hello"   │     │   (string value)│
└─────────────┘     └─────────────────┘

After move:
┌─────────────┐     ┌─────────────────┐
│  source    │     │   (empty)       │  ← Becomes empty
│             │     └─────────────────┘
└─────────────┘
┌─────────────┐     ┌─────────────────┐
│  target    │ ──▶ │   "Hello"       │
│  "Hello"   │     │   (string value)│
└─────────────┘     └─────────────────┘
```

## 8.5 Empty State

**Empty state** is the state of a variable after being moved:

```yaoxiang
# Empty state example
counter: Int = 0

# First assignment
counter = 1
counter = 2

# After move, becomes empty
other = counter    # counter becomes empty

# Can reassign (reuse variable name)
counter = 100       # ✅ counter is reassigned
```

**Rules of empty state**:
- After being moved, variable enters empty state
- Empty state can be reassigned (reuse variable name)
- Cannot use variables in empty state

## 8.6 Scope and Ownership

```yaoxiang
scope_example: () -> Void = {
    # Create value
    value: String = "Hello, scope!"

    print(value)   # ✅ Can use

    # Function ends, value is automatically cleared
    # Owner's "life" has ended
}

# value no longer exists here
```

## 8.7 Why Move Instead of Copy?

**Performance**: Move only needs to change pointer, doesn't need to copy data!

```yaoxiang
# Move large object
large_list: List[String] = load_very_large_list()
other_list = large_list    # ⚠️ Move! Zero copy

# If copying, would be slow:
# other_list = large_list.clone()  # ❌ Unnecessary, poor performance
```

## 8.8 Three Sharing Methods

| Method | Keyword | Applicable Scenario | Performance |
|--------|---------|-------------------|-------------|
| **Move** | Default | Need unique owner | Zero overhead |
| **Reference** | `ref` | Need sharing | Medium (atomic operation) |
| **Clone** | `.clone()` | Need copy | Depends on type |

```yaoxiang
# Move (default)
p: Point = Point(1.0, 2.0)
p2 = p                    # p becomes empty, p2 is new owner

# Reference (sharing)
p: Point = Point(1.0, 2.0)
shared = ref p           # Share, Arc reference count

# Clone (copy)
p: Point = Point(1.0, 2.0)
p2 = p.clone()           # p and p2 are both independent owners
```

## 8.9 Chapter Summary

| Concept | Understanding |
|---------|---------------|
| Ownership | Each piece of data has a unique owner |
| Move | Transfer of ownership |
| Empty state | State of variable after being moved |
| Scope | Valid range of variables |

## 8.10 I Ching Introduction

> "Because it does not contend, nothing in the world can contend with it."
> — Tao Te Ching
>
> The way of ownership lies in "not contending":
> - Each value has only one owner
> - When transferring, old owner lets go, new owner takes over
> - Not much contending, not wildly contending
>
> Like the flowing of Taiji, yang moves, yin follows, life continues without end.
> The contention of ownership rests, the way of memory is born.
