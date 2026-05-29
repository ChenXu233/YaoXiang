---
title: Проектирование реализации системы атрибутов типа (Dup/Clone)
status: draft
created: 2026-05-29
---

# Проектирование реализации системы атрибутов типа

## Цель

Реализовать в компиляторе `Dup` trait (маркер неявного поверхностного копирования), дополнив рекурсивные проверки в системе trait'ов.

## Основной дизайн

### Определение Dup trait

```rust
// Trait того же уровня, что Clone, Debug
// Без методов — только типовая маркировка
TraitDefinition {
    name: "Dup",
    methods: {},           // пусто — marker trait
    parent_traits: vec!["Clone"],  // Dup означает возможность Clone
    generic_params: vec![],
    is_marker: true,
}
```

### Какие типы являются Dup

| Тип | Dup | Причина |
|------|-----|---------|
| Int, Float(32), Float(64) | ✅ | Примитив |
| Bool, Char | ✅ | Примитив |
| String, Bytes | ✅ | Внутренняя реализация уже с подсчётом ссылок |
| &T (ReadToken) | ✅ | Нулевой размер, понятие времени компиляции |
| &mut T (WriteToken) | ❌ | Линейный, эксклюзивно уникальный |
| struct | автовывод | Все поля Dup → struct Dup |
| Fn (замыкание) | ❌ | Захваченное замыканием окружение может быть не Dup |
| Arc(T) | ✅ | Arc сам по себе может быть поверхностно скопирован |

### Связь между Dup и Clone

```
Dup  →  Clone   (все типы Dup автоматически реализуют Clone)
Clone  ↛  Dup   (есть Clone не означает наличие Dup)
```

## Список реализации

### 1. trait_data.rs — добавление поля is_marker

**Файл**: `src/frontend/core/types/base/trait_data.rs`

```rust
pub struct TraitDefinition {
    pub name: String,
    pub methods: HashMap<String, TraitMethodSignature>,
    pub parent_traits: Vec<String>,
    pub generic_params: Vec<String>,
    pub span: Option<Span>,
    pub is_marker: bool,  // NEW: marker trait без методов
}
```

Trait с `is_marker = true` не требует проверки реализации методов. Обработка marker trait компилятором:
- Примитивные типы → автоматическая регистрация impl
- struct → auto-derive с рекурсивной проверкой полей
- Обобщённые ограничения `T: Dup` → обрабатываются как обычные ограничения trait

### 2. std_traits.rs — регистрация Dup, удаление Send/Sync

**Файл**: `src/frontend/core/typecheck/traits/std_traits.rs`

```rust
// Изменение STD_TRAITS (удалить Send, Sync, добавить Dup)
pub const STD_TRAITS: &[&str] = &[
    "Clone",
    "Dup",      // NEW
    "Equal",
    "Debug",
    "Iterator",
];

// Новая функция
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

// В init_primitive_impls регистрация Dup для примитивов
// Int, Float, Bool, Char, String, Bytes все автоматически получают Dup impl
```

### 3. solver.rs — поддержка рекурсивной проверки struct

**Файл**: `src/frontend/core/typecheck/traits/solver.rs`

Ключевое изменение: метод `check_dup_trait` должен рекурсивно проверять поля struct.

```rust
fn check_dup_trait(&self, ty: &MonoType) -> bool {
    match ty {
        // Примитивы: автоматически Dup
        MonoType::Int(_) | MonoType::Float(_) | MonoType::Bool 
        | MonoType::Char | MonoType::String | MonoType::Bytes => true,
        
        // Arc: автоматически Dup (семантика подсчёта ссылок)
        MonoType::Arc(_) => true,
        
        // Ref (заёмные токены): &T Dup, &mut T не Dup
        MonoType::Ref { mutable: false, .. } => true,
        MonoType::Ref { mutable: true, .. } => false,
        
        // struct: рекурсивная проверка всех полей
        MonoType::Struct(s) => {
            s.fields.iter().all(|(_, field_ty)| self.check_dup_trait(field_ty))
        }
        
        // Tuple: рекурсивная проверка всех элементов
        MonoType::Tuple(elems) => {
            elems.iter().all(|t| self.check_dup_trait(t))
        }
        
        // Enum: проверка всех variant всех полей
        MonoType::Enum(e) => {
            e.variants.iter().all(|v| 
                v.fields.iter().all(|(_, t)| self.check_dup_trait(t))
            )
        }
        
        // Всё остальное: по умолчанию не Dup
        _ => false,
    }
}
```

Та же схема применяется к `check_clone_trait` — ранее проверялись только примитивы, теперь требуется рекурсия для struct.

### 4. auto_derive.rs — поддержка сложных типов и рекурсии

**Файл**: `src/frontend/core/typecheck/traits/auto_derive.rs`

Критическая проблема текущего `can_auto_derive`: при встрече `List[Int]` как `Type::Generic` сразу возвращается false.

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

// NEW: рекурсивная проверка поля типа на удовлетворение trait
fn field_type_satisfies(
    trait_table: &TraitTable,
    trait_name: &str,
    ty: &Type,
) -> bool {
    match ty {
        // Простое имя типа →查询 trait table
        Type::Name { name, .. } => {
            trait_table.has_impl(trait_name, name)
        }
        
        // Обобщённые типы List(Int), Option(Point) → проверка внутреннего типа
        Type::Generic { name, args, .. } => {
            // Контейнер реализует trait и все параметры тоже
            if !trait_table.has_impl(trait_name, name) {
                return false;
            }
            args.iter().all(|arg| field_type_satisfies(trait_table, trait_name, arg))
        }
        
        // Tuple → проверка всех элементов
        Type::Tuple(elems) => {
            elems.iter().all(|e| field_type_satisfies(trait_table, trait_name, e))
        }
        
        // Функциональный тип → функции не могут быть Dup (консервативно)
        Type::Fn { .. } => false,
        
        // Остальное нельзя вывести
        _ => false,
    }
}
```

### 5. resolution.rs — улучшение разрешения trait

**Файл**: `src/frontend/core/typecheck/traits/resolution.rs`

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

### 6. bounds.rs — поддержка ограничений Dup

**Файл**: `src/frontend/core/typecheck/inference/bounds.rs`

Существующий код bounds checker уже поддерживает схему `T: Clone`. Добавление `T: Dup` работает автоматически — вызывается `trait_solver.check_trait(ty, "Dup")`.

Единственное, что нужно обеспечить: при неудаче `check_trait` для struct-типа сначала попробовать auto-derive.

```rust
pub fn check_trait_bounds(&mut self, ty: &MonoType, bounds: &[String]) -> Result<()> {
    for bound in bounds {
        if !self.trait_solver.check_trait(ty, bound) {
            // Попытка auto-derive
            if let MonoType::Struct(s) = ty {
                if can_auto_derive_for_monotype(&self.trait_table, bound, s) {
                    continue;  // auto-derive прошёл
                }
            }
            return Err(TypeError::TraitBoundFailed { ... });
        }
    }
    Ok(())
}
```

### 7. mono.rs — MonoType не требует изменений (на данный момент)

`MonoType` не нуждается в добавлении `TypeFlags`. Определение Dup полностью через систему trait'ов — достаточно вызова `trait_table.has_impl("Dup", type_name)`. Это операция во время проверки типов, не горячий путь.

В будущем при необходимости производительности можно добавить кэш `Cache<TypeId, bool>` для результатов запросов. Сейчас не требуется.

### 8. Очистка Send/Sync

**Файл**: `src/frontend/core/typecheck/traits/std_traits.rs`
- Удалить "Send", "Sync" из `STD_TRAITS`
- Удалить `add_send_trait()`, `add_sync_trait()`

**Файл**: `src/middle/passes/lifetime/send_sync.rs`
- Удалить весь checker, или оставить как no-op (консервативно)
- Убрать поле `send_sync_checker` из `OwnershipChecker`
- Убрать импорт и вызов `SendSyncChecker` из `mod.rs`

**Файл**: `src/middle/passes/lifetime/error.rs`
- Удалить варианты `OwnershipError::NotSend`, `NotSync` (или оставить с пометкой deprecated)

## Порядок реализации

1. **trait_data.rs** — добавление поля `is_marker` (5 строк изменений)
2. **std_traits.rs** — регистрация Dup, удаление Send/Sync, регистрация примитивных dup impl (~50 строк изменений)
3. **solver.rs** — рекурсивная проверка struct (~30 строк изменений)
4. **auto_derive.rs** — поддержка проверки обобщённых параметров (~50 строк переработки)
5. **resolution.rs** — добавление пути для Dup (1 строка)
6. **bounds.rs** — интеграция auto-derive (~10 строк)
7. **Очистка Send/Sync** — удаление связанного кода

Общий объём изменений: ~200 строк. Изменения сконцентрированы в 6 файлах в директории системы trait'ов.

## Способ проверки

```yaoxiang
# Тест 1: примитивы автоматически Dup
x: Int = 42
y = x        # ✅ Int: Dup
print(x)     # ✅

# Тест 2: struct автоматический вывод
Point2D: Type = { x: Float, y: Float }
p = Point2D(1.0, 2.0)
q = p         # ✅ Point2D: Dup (оба поля Float: Dup)
print(p)      # ✅

# Тест 3: struct с не-Dup полем
Buffer: Type = { data: Array(Int), len: Int }
b = Buffer(...)
b2 = b        # ❌ Move (Array не Dup)
print(b)      # ❌ Уже перемещён

# Тест 4: обобщённые ограничения
dup_use: (x: T: Dup) -> T = x  # ✅ Ограничение T: Dup
```

## Ссылки

- Анализ пробелов исследования (пробелы интеграции системы типов)
- Анализ пробелов исследования (пробелы системы trait'ов)
- [RFC-011 Дизайн системы обобщённых типов](../../design/rfc/accepted/011-generic-type-system.md)
- [RFC-009 Модель владения v9](../../design/rfc/accepted/009-ownership-model.md)