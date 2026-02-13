---
title: Chapter 9: Move and Share
---

# Chapter 9: Move and Share

> **Chapter Goal**: Deeply understand Move, ref, and clone three sharing methods, master advanced usage of ownership

## 9.1 Three Sharing Methods Review

| Method | Syntax | Meaning | Thread Safe | Performance |
|--------|--------|---------|-------------|-------------|
| **Move** | `p2 = p` | Ownership transfer | - | Zero overhead |
| **Reference** | `ref p` | Share (Arc) | ✅ Safe | Medium |
| **Clone** | `.clone()` | Explicit copy | - | Depends |

## 9.2 Deep Dive: Move

### 9.2.1 Assignment = Move

```yaoxiang
# Assignment is move
original: String = "Hello"
copy = original              # Move! original becomes empty
```

### 9.2.2 Function Parameter = Move

```yaoxiang
# Function parameters are move by default
process: (data: Data) -> Void = {
    print(data)              # Use data
    # Function ends, data is released
}

main: () -> Void = {
    big_data: Data = load_data()
    process(big_data)       # Move! big_data becomes empty
}
```

### 9.2.3 Return Value = Move

```yaoxiang
# Function return value is also move
create_point: () -> Point = {
    p = Point(1.0, 2.0)
    return p                 # Move! p becomes empty, returns new value
}

main: () -> Void = {
    my_point = create_point()  # Receive return value
}
```

## 9.3 Deep Dive: ref

### 9.3.1 Create Reference

```yaoxiang
# ref keyword creates shared reference (Arc)
data: Point = Point(1.0, 2.0)
shared: ref Point = ref data    # Arc reference count

# ref automatically infers type
shared2 = ref data             # Automatically inferred as ref Point
```

### 9.3.2 Characteristics of References

```yaoxiang
data: Point = Point(1.0, 2.0)

# Create multiple references
ref1 = ref data
ref2 = ref data
ref3 = ref data

# Reference count = 3
# ref1, ref2, ref3 all point to the same data
```

### 9.3.3 Reference Count Auto-management

```yaoxiang
{
    data = Point(1.0, 2.0)    # Reference count = 1

    shared = ref data           # Reference count = 2

    # shared leaves scope, reference count = 1

}                               # data leaves scope, reference count = 0, automatically released
```

### 9.3.4 Concurrent Sharing

```yaoxiang
# ref = Arc, thread safe
shared_data = ref my_data

spawn(() => {
    # Use shared data in new task
    print(shared_data.x)
})

spawn(() => {
    # Another task
    print(shared_data.y)
})
```

## 9.4 Deep Dive: clone

### 9.4.1 Explicit Copy

```yaoxiang
# Clone = explicit copy
original: Point = Point(1.0, 2.0)
copy = original.clone()        # Create a new copy

# Now there are two independent Points
original.x = 0.0              # ✅ Doesn't affect copy
copy.x = 10.0                 # ✅ Doesn't affect original
```

### 9.4.2 When to Use Clone?

| Scenario | Use clone? |
|----------|-------------|
| Need to keep original value | ✅ Yes |
| Original value no longer needed | ❌ No, use Move |
| Need multiple copies | ✅ Yes |

```yaoxiang
# Scenario 1: Need to keep original value
data: Config = load_config()
backup = data.clone()          # ✅ Keep backup
process(data)                  # Process original data

# Scenario 2: Original value no longer needed
processed = data               # ✅ Just move
```

## 9.5 Three Methods Comparison

```yaoxiang
# Data
p: Point = Point(1.0, 2.0)

# === Method 1: Move ===
p2 = p          # p becomes empty, p2 is new owner
# print(p.x)     # ❌ Error! p is empty
# print(p2.x)    # ✅ Correct! 2.0

# === Method 2: ref ===
p: Point = Point(1.0, 2.0)  # Recreate
shared = ref p      # Share, p is still owner
# print(shared.x)   # ✅ Can access
# print(p.x)        # ✅ p is still owner

# === Method 3: clone ===
p: Point = Point(1.0, 2.0)  # Recreate
copy = p.clone()  # Create independent copy
# print(p.x)       # ✅ p still valid
# print(copy.x)    # ✅ copy is independent copy
```

## 9.6 Ownership Backflow

When function modifies parameter and returns, can "backflow" ownership:

```yaoxiang
# Ownership backflow example
p: Point = Point(1.0, 2.0)

# p is modified and returned
p = p.translate(10.0, 10.0)   # p.translate returns new Point

# Equivalent to:
# temp = p.translate(10.0, 10.0)
# p = temp                      # Backflow
```

**Chained calls**:

```yaoxiang
p: Point = Point(1.0, 2.0)

# Chained calls
p = p.translate(10.0, 10.0)
      .rotate(90)
      .scale(2.0)
```

## 9.7 Circular References

**Note**: Circular references may cause memory leaks!

```yaoxiang
# ❌ Circular reference (compiler will error)
a = Node("A")
b = Node("B")
a.child = ref b    # a -> b
b.child = ref a    # b -> a (circular!)
```

**Solution**: YaoXiang compiler detects cross-task cycles:

```yaoxiang
# ✅ Single task cycle (compiler allows, leak controllable)
{
    a = Node("A")
    b = Node("B")
    a.child = ref b
    b.child = ref a

    # Leak disappears after task ends
}
```

## 9.8 unsafe Mode

For low-level operations, can use `unsafe` to bypass checks:

```yaoxiang
# unsafe: raw pointer operation
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p       # Get raw pointer
    (*ptr).x = 0.0          # Directly modify memory
}
```

**Note**: `unsafe` requires you to guarantee safety!

## 9.9 Chapter Summary

| Concept | Syntax | Description |
|---------|--------|-------------|
| Move | `p2 = p` | Ownership transfer, zero overhead |
| Reference | `ref p` | Share (Arc), thread safe |
| Clone | `.clone()` | Explicit copy, use when needed |
| Backflow | `p = p.method()` | Chained calls |
| unsafe | `unsafe { ... }` | Raw pointer operations |

## 9.10 I Ching Introduction

> "The opposite is the movement of Tao; the weak is the use of Tao.
> All things in the world are born from existence; existence is born from non-existence."
> — Tao Te Ching
>
> Move and share are also the way of yin and yang:
> - **Move**: Yang is moving, transferring ownership
> - **ref**: Yin is still, maintaining sharing
> - **clone**: Change is transformation, creating new forms
>
> The three generate and restrain each other, cycling endlessly.
> Understanding the balance between "moving" and "still", one can wield the sword of ownership.
