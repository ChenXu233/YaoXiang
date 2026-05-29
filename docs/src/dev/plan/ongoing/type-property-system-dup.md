---
title: 类型属性系统实现设计 (Dup/Clone)
status: draft
created: 2026-05-29
---

# 类型属性系统实现设计

## 目标

在编译器类型系统中实现 `Dup` trait（隐式浅复制标记），补齐 trait 系统的递归检查能力。

## 核心设计

### Dup trait 定义

```rust
// 和 Clone、Debug 同级的 marker trait
// 没有方法——只做类型级标记
TraitDefinition {
    name: "Dup",
    methods: {},           // 空——marker trait
    parent_traits: vec!["Clone"],  // Dup 意味着可以 Clone
    generic_params: vec![],
    is_marker: true,
}
```

### 哪些类型是 Dup

| 类型 | Dup | 原因 |
|------|-----|------|
| Int, Float(32), Float(64) | ✅ | 原语 |
| Bool, Char | ✅ | 原语 |
| String, Bytes | ✅ | 内部已经是引用计数 |
| &T (ReadToken) | ✅ | 零大小，编译期概念 |
| &mut T (WriteToken) | ❌ | 线性，独占唯一 |
| struct | 自动推导 | 所有字段 Dup → struct Dup |
| Fn (闭包) | ❌ | 闭包捕获的环境可能非 Dup |
| Arc(T) | ✅ | Arc 本身可以浅复制 |

### Dup 和 Clone 的关系

```
Dup  →  Clone   （所有 Dup 类型自动实现 Clone）
Clone  ↛  Dup   （有 Clone 不一定有 Dup）
```

## 实现清单

### 1. trait_data.rs — 加 is_marker 字段

**文件**: `src/frontend/core/types/base/trait_data.rs`

```rust
pub struct TraitDefinition {
    pub name: String,
    pub methods: HashMap<String, TraitMethodSignature>,
    pub parent_traits: Vec<String>,
    pub generic_params: Vec<String>,
    pub span: Option<Span>,
    pub is_marker: bool,  // NEW: 无方法的标记 trait
}
```

`is_marker = true` 的 trait 不需要方法实现检查。编译器对 marker trait 的处理：
- 原语类型 → 自动注册 impl
- struct → auto-derive 递归检查字段
- 泛型约束 `T: Dup` → 和普通 trait 约束一样处理

### 2. std_traits.rs — 注册 Dup，移除 Send/Sync

**文件**: `src/frontend/core/typecheck/traits/std_traits.rs`

```rust
// 修改 STD_TRAITS（删 Send, Sync，加 Dup）
pub const STD_TRAITS: &[&str] = &[
    "Clone",
    "Dup",      // NEW
    "Equal",
    "Debug",
    "Iterator",
];

// 新增函数
fn add_dup_trait(trait_table: &mut TraitTable) {
    trait_table.add_trait(TraitDefinition {
        name: "Dup".to_string(),
        methods: HashMap::new(),
        parent_traits: vec!["Clone".to_string()],
        generic_params: vec![],
        span: None,
        is_marker: true,
    });
}

// init_primitive_impls 中为原语注册 Dup
// Int, Float, Bool, Char, String, Bytes 全部自动获得 Dup impl
```

### 3. solver.rs — 支持递归 struct 检查

**文件**: `src/frontend/core/typecheck/traits/solver.rs`

核心改动：`check_dup_trait` 方法必须递归进入 struct 字段。

```rust
fn check_dup_trait(&self, ty: &MonoType) -> bool {
    match ty {
        // 原语：自动 Dup
        MonoType::Int(_) | MonoType::Float(_) | MonoType::Bool 
        | MonoType::Char | MonoType::String | MonoType::Bytes => true,
        
        // Arc：自动 Dup（引用计数语义）
        MonoType::Arc(_) => true,
        
        // Ref（借用令牌）：&T Dup，&mut T 不 Dup
        MonoType::Ref { mutable: false, .. } => true,
        MonoType::Ref { mutable: true, .. } => false,
        
        // struct：递归检查所有字段
        MonoType::Struct(s) => {
            s.fields.iter().all(|(_, field_ty)| self.check_dup_trait(field_ty))
        }
        
        // Tuple：递归检查所有元素
        MonoType::Tuple(elems) => {
            elems.iter().all(|t| self.check_dup_trait(t))
        }
        
        // Enum：检查所有 variant 的所有字段
        MonoType::Enum(e) => {
            e.variants.iter().all(|v| 
                v.fields.iter().all(|(_, t)| self.check_dup_trait(t))
            )
        }
        
        // 其他一切：默认不 Dup
        _ => false,
    }
}
```

同样模式应用于 `check_clone_trait`——之前只认原语，现在也要递归 struct。

### 4. auto_derive.rs — 支持复杂类型和递归

**文件**: `src/frontend/core/typecheck/traits/auto_derive.rs`

当前 `can_auto_derive` 的致命问题：遇到 `List[Int]` 这样 `Type::Generic` 直接返回 false。

```rust
pub fn can_auto_derive(
    trait_table: &TraitTable,
    trait_name: &str,
    fields: &[StructField],
) -> bool {
    for field in fields {
        if !field_type_satisfies(trait_table, trait_name, &field.ty) {
            return false;
        }
    }
    true
}

// NEW: 递归检查字段类型是否满足 trait
fn field_type_satisfies(
    trait_table: &TraitTable,
    trait_name: &str,
    ty: &Type,
) -> bool {
    match ty {
        // 简单类型名 → 查 trait table
        Type::Name { name, .. } => {
            trait_table.has_impl(trait_name, name)
        }
        
        // 泛型类型 List(Int), Option(Point) → 检查内层
        Type::Generic { name, args, .. } => {
            // 容器本身实现 trait 且所有参数也实现
            if !trait_table.has_impl(trait_name, name) {
                return false;
            }
            args.iter().all(|arg| field_type_satisfies(trait_table, trait_name, arg))
        }
        
        // 元组 → 检查所有元素
        Type::Tuple(elems) => {
            elems.iter().all(|e| field_type_satisfies(trait_table, trait_name, e))
        }
        
        // 函数类型 → 函数不能 Dup（保守）
        Type::Fn { .. } => false,
        
        // 其他不可推导
        _ => false,
    }
}
```

### 5. resolution.rs — 完善 trait 解析

**文件**: `src/frontend/core/typecheck/traits/resolution.rs`

```rust
fn find_trait_definition(&self, name: &str) -> Option<String> {
    match name {
        "Clone" => Some("std::Clone".to_string()),
        "Dup" => Some("std::Dup".to_string()),     // NEW
        "Debug" => Some("std::fmt::Debug".to_string()),
        "Equal" => Some("std::cmp::Equal".to_string()),
        "Iterator" => Some("std::iter::Iterator".to_string()),
        _ => None,
    }
}
```

### 6. bounds.rs — Dup 约束支持

**文件**: `src/frontend/core/typecheck/inference/bounds.rs`

bounds checker 现有代码已经支持 `T: Clone` 模式。加上 `T: Dup` 后自动工作——它调 `trait_solver.check_trait(ty, "Dup")`。

唯一需要确保的：当 `check_trait` 失败时，对于 struct 类型，先尝试 auto-derive。

```rust
pub fn check_trait_bounds(&mut self, ty: &MonoType, bounds: &[String]) -> Result<()> {
    for bound in bounds {
        if !self.trait_solver.check_trait(ty, bound) {
            // 尝试 auto-derive
            if let MonoType::Struct(s) = ty {
                if can_auto_derive_for_monotype(&self.trait_table, bound, s) {
                    continue;  // auto-derive 通过
                }
            }
            return Err(TypeError::TraitBoundFailed { ... });
        }
    }
    Ok(())
}
```

### 7. mono.rs — MonoType 无需改动（目前）

`MonoType` 不需要加 `TypeFlags`。Dup 的判断完全通过 trait 系统——查一下 `trait_table.has_impl("Dup", type_name)` 就够了。这是类型检查时的操作，不是热路径。

未来如果性能需要，可以加一个 `Cache<TypeId, bool>` 缓存查表结果。现在不需要。

### 8. 清理 Send/Sync

**文件**: `src/frontend/core/typecheck/traits/std_traits.rs`
- `STD_TRAITS` 中删除 "Send", "Sync"
- 删除 `add_send_trait()`, `add_sync_trait()`

**文件**: `src/middle/passes/lifetime/send_sync.rs`
- 整个 checker 删除，或保留为 no-op（保守）
- `OwnershipChecker` 中移除 `send_sync_checker` 字段
- `mod.rs` 中移除 `SendSyncChecker` 的导入和调用

**文件**: `src/middle/passes/lifetime/error.rs`
- `OwnershipError::NotSend`, `NotSync` 变体删除（或保留但标记 deprecated）

## 实现顺序

1. **trait_data.rs** — 加 `is_marker` 字段（5 行改动）
2. **std_traits.rs** — 注册 Dup，删 Send/Sync，注册原语 dup impl（~50 行改动）
3. **solver.rs** — 递归 struct 检查（~30 行改动）
4. **auto_derive.rs** — 支持泛型参数检查（~50 行重写）
5. **resolution.rs** — 加 Dup 路径（1 行）
6. **bounds.rs** — auto-derive 集成（~10 行）
7. **清理 Send/Sync** — 删除相关代码

总改动量估计：~200 行。改动集中在 6 个文件的 trait 系统目录。

## 验证方式

```yaoxiang
# 测试 1：原语自动 Dup
x: Int = 42
y = x        # ✅ Int: Dup
print(x)     # ✅

# 测试 2：struct 自动推导
Point2D: Type = { x: Float, y: Float }
p = Point2D(1.0, 2.0)
q = p         # ✅ Point2D: Dup（两个字段都是 Float: Dup）
print(p)      # ✅

# 测试 3：含非 Dup 字段的 struct
Buffer: Type = { data: Array(Int), len: Int }
b = Buffer(...)
b2 = b        # ❌ Move（Array 不 Dup）
print(b)      # ❌ 已移动

# 测试 4：泛型约束
dup_use: (x: T: Dup) -> T = x  # ✅ T: Dup 约束
```

## 参考

- 探查缺口分析（类型系统集成缺口）
- 探查缺口分析（trait 系统缺口）
- [RFC-011 泛型系统设计](../../design/rfc/accepted/011-generic-type-system.md)
- [RFC-009 所有权模型 v9](../../design/rfc/accepted/009-ownership-model.md)
