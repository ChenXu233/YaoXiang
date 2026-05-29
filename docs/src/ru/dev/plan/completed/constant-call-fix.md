# Исправление проблемы вызова констант

## Обзор

Исправление проблемы отображения констант типа `std.math.PI` как `unit` при использовании.

## Текущее состояние

- **Проблема**: При использовании `PI` возвращается `unit` вместо ожидаемого числа с плавающей точкой
- **Причина**: Константы интерпретируются как вызовы функций без параметров, но FFI handler не выполняется корректно

## Анализ проблемы

В текущем коде:

```rust
// FFI регистрация
registry.register("std.math.PI", |_args| {
    Ok(RuntimeValue::Float(std::f64::consts::PI))
});
```

Однако вызовы констант (например, `PI`) могут компилироваться в другие инструкции, отличные от вызовов функций.

## Модули, требующие изменения

### 1. Компилятор - генерация кода

Файл: `src/middle/passes/codegen/`

Необходимо корректно распознавать ссылки на константы (например, `PI`) как вызовы native функций и генерировать соответствующие bytecode инструкции.

### 2. Интерпретатор/исполнитель

Файл: `src/backends/interpreter/executor.rs`

Обеспечить корректную маршрутизацию ссылок на константы к FFI handler.

## План реализации

### Вариант A: Регистрация имён констант в translator

```rust
// src/middle/passes/codegen/translator.rs
// Добавление констант в native_functions
native_functions.insert("std.math.PI".to_string());
native_functions.insert("std.math.E".to_string());
native_functions.insert("std.math.TAU".to_string());
```

### Вариант B: Использование специального префикса в FFI

Использование соглашения, например `__const__std.math.PI`, для различения констант и функций.

## Тестовые случаи

```yaoxiang
use std.math.*

// Ожидаемый вывод: 3.14159...
println(PI)

// Ожидаемый вывод: 2.71828...
println(E)
```

## Связанные файлы

- `src/middle/passes/codegen/translator.rs` — Генерация кода
- `src/backends/interpreter/executor.rs` — Интерпретатор
- `src/backends/interpreter/ffi.rs` — FFI регистрация