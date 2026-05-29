# Проект расширения FFI

> **Статус**: ✅ Выполнено (все 10 шагов реализованы)
>
> **Дата реализации**: 2025 г.

## I. Предыстория и цели

### 1.1 Текущее состояние (до внедрения)

Текущая архитектура FFI:

```rust
type NativeHandler = fn(&[RuntimeValue]) -> Result<RuntimeValue, ExecutorError>;
```

**Проблемы**:
- Функции native не могут обращаться к heap, не могут возвращать List/Dict
- Функции native не могут вызывать пользовательские функции YaoXiang (высшие функции невозможны)
- В интерпретаторе разбросаны жёстко закодированные обработки исключений (len, dict_keys и др.)

### 1.2 Цели

1. ✅ Позволить функциям native обращаться к heap и возвращать List/Dict
2. ✅ Позволить функциям native вызывать функции YaoXiang (поддержка высших функций)
3. ✅ Унифицировать архитектуру, устранить жёсткое кодирование в интерпретаторе

---

## II. Общий дизайн

### 2.1 Определения основных типов

```rust
// Контекст выполнения - передаётся функциям native
pub struct NativeContext<'a> {
    /// Управление памятью heap
    pub heap: &'a mut Heap,
    /// Обратный вызов: для вызова функций YaoXiang (сценарии высших функций)
    pub call_fn: Option<&'a mut dyn FnMut(&RuntimeValue, &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError>>,
}

// Сигнатура функции Native изменена
pub type NativeHandler = fn(args: &[RuntimeValue], ctx: &mut NativeContext<'_>) -> Result<RuntimeValue, ExecutorError>;
```

> **Пояснение к реализации**: В финальной реализации используется обратный вызов `call_fn` замыканием вместо прямого хранения ссылки на `Interpreter`,
> это позволяет избежать проблем самореференции borrow checker'а Rust (Interpreter одновременно владеет heap и ffi).

### 2.2 Структура модулей

```
src/backends/interpreter/
├── ffi.rs          # Изменения: тип NativeHandler, соглашения о вызовах
└── executor.rs     # Изменения: конструкция Context при вызове native

src/std/
├── mod.rs          # Изменения: определение типа NativeHandler
├── io.rs           # Изменения: все сигнатуры функций
├── math.rs         # Изменения: все сигнатуры функций
├── string.rs       # Изменения: реализация доступа к heap
├── list.rs         # Изменения: реализация доступа к heap + высшие функции
├── dict.rs         # Изменения: реализация доступа к heap
└── ... другие модули   # Изменения: все сигнатуры функций
```

### 2.3 Процесс вызова

```
Код пользователя вызывает функцию native
    ↓
BytecodeExecutor выполняет CallNative/CallStatic
    ↓
Получение NativeHandler из FFIRegistry
    ↓
Конструкция NativeContext { heap, call_fn }
    ↓
Вызов handler(args, &mut ctx)
    ↓
Внутри handler можно:
  - Обращаться к ctx.heap для выделения/изменения List/Dict
  - Вызывать ctx.call_function() для выполнения пользовательских функций
    ↓
Возврат RuntimeValue
```

---

## III. Детальные шаги реализации

### Шаг 1: Изменение определений типов FFI

**Файл**: `src/std/mod.rs`

**Содержание изменений**:
1. Добавить определение структуры `NativeContext`
2. Изменить псевдоним типа `NativeHandler`
3. Изменить структуру `NativeExport` (необязательно)

**Критерии приёмки**:
- [x] Структура `NativeContext` содержит поля `heap` и `call_fn`
- [x] Тип `NativeHandler` имеет вид `fn(args: &[RuntimeValue], ctx: &mut NativeContext<'_>) -> Result<RuntimeValue, ExecutorError>`
- [x] Компиляция проходит

**Тестовый план**:
- Тест компиляции: `cargo check` проходит

---

### Шаг 2: Изменение FFI Registry

**Файл**: `src/backends/interpreter/ffi.rs`

**Содержание изменений**:
1. Изменить сигнатуру метода `register()`
2. Изменить метод `call()`, передавать ctx при вызове

**Критерии приёмки**:
- [x] `register(name, handler)` принимает handler с новой сигнатурой
- [x] `call(name, args, ctx)` передаёт ctx в handler
- [x] Компиляция проходит

**Тестовый план**:
- Тест компиляции: `cargo check` проходит

---

### Шаг 3: Изменение точек вызова в интерпретаторе

**Файл**: `src/backends/interpreter/executor.rs`

**Содержание изменений**:
1. Найти место обработки байткода `CallNative` (около строки 600)
2. Создать `NativeContext` перед вызовом функции native
3. Передать ctx в `ffi.call()`

**Критерии приёмки**:
- [x] При вызове функции native создаётся NativeContext
- [x] NativeContext содержит валидную ссылку на heap
- [x] NativeContext содержит обратный вызов call_fn (для сценариев высших функций)
- [x] Компиляция проходит

**Тестовый план**:
- Тест компиляции: `cargo check` проходит

---

### Шаг 4: Обновление модуля std.io

**Файл**: `src/std/io.rs`

**Содержание изменений**:
1. Обновить все сигнатуры функций native
2. Добавить параметр `ctx`

**Задействованные функции**:
- `native_print`
- `native_println`
- `native_read_line`
- `native_read_file`
- `native_write_file`
- `native_append_file`

**Критерии приёмки**:
- [x] Все сигнатуры функций соответствуют новому типу `NativeHandler`
- [x] Внутри функций ctx не используется (обратная совместимость)
- [x] Компиляция проходит

**Тестовый план**:
- [x] `std.io.print("test")` работает корректно
- [x] `std.io.println("test")` работает корректно

---

### Шаг 5: Обновление модуля std.math

**Файл**: `src/std/math.rs`

**Содержание изменений**:
1. Обновить все сигнатуры функций native
2. Добавить параметр `ctx`

**Задействованные функции**:
- `native_abs`, `native_max`, `native_min`, `native_clamp`
- `native_fabs`, `native_fmax`, `native_fmin`, `native_pow`
- `native_sqrt`, `native_floor`, `native_ceil`, `native_round`
- `native_sin`, `native_cos`, `native_tan`
- `native_pi`, `native_e`, `native_tau`

**Критерии приёмки**:
- [x] Все сигнатуры функций соответствуют новому типу
- [x] Компиляция проходит

**Тестовый план**:
- [x] `std.math.abs(-5)` возвращает 5
- [x] `std.math.sqrt(4)` возвращает 2

---

### Шаг 6: Реализация полной функциональности std.string

**Файл**: `src/std/string.rs`

**Содержание изменений**:
1. Изменить сигнатуры функций
2. Реализовать доступ к heap, возвращать настоящие List

**Задействованные функции**:

| Функция | Способ реализации |
|---------|-------------------|
| `split` | Использование ctx.heap для выделения List |
| `chars` | Использование ctx.heap для выделения List |
| `trim/upper/lower/replace` | Уже реализовано (heap не требуется) |
| `contains/starts_with/ends_with` | Уже реализовано (heap не требуется) |

**Критерии приёмки**:
- [x] `std.string.split("a,b", ",")` возвращает `["a", "b"]`
- [x] `std.string.chars("abc")` возвращает `["a", "b", "c"]`
- [x] Компиляция проходит

**Тестовый план**:
```yaoxiang
// Тест split
let result = std.string.split("hello,world", ",");
assert(std.list.len(result) == 2);

// Тест chars
let chars = std.string.chars("abc");
assert(std.list.len(chars) == 3);
```

---

### Шаг 7: Реализация полной функциональности std.list (с высшими функциями)

**Файл**: `src/std/list.rs`

**Содержание изменений**:
1. Изменить все сигнатуры функций
2. Реализовать доступ к heap
3. Реализовать вызов высших функций

**Задействованные функции**:

| Функция | Способ реализации |
|---------|-------------------|
| `push` | Использование ctx.heap для выделения нового List |
| `pop` | Получение элемента из heap |
| `prepend` | Использование ctx.heap для выделения нового List |
| `reverse` | Использование ctx.heap для выделения нового List |
| `concat` | Использование ctx.heap для выделения нового List |
| `map` | **Вызов пользовательской функции** |
| `filter` | **Вызов пользовательской функции** |
| `reduce` | **Вызов пользовательской функции** |
| `get/set/first/last/slice` | Доступ к heap |

**Ключевые моменты реализации высших функций**:
```rust
fn native_map(args: &[RuntimeValue], ctx: &mut NativeContext<'_>) -> Result<RuntimeValue, ExecutorError> {
    // args[0] - список, args[1] - пользовательская функция
    let list_handle = /* Извлечение из args[0] */;
    let func_value = /* Извлечение из args[1] */;

    // Получение элементов списка (clone для избежания конфликта заимствования)
    let items = match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(...)
    };

    // Вызов пользовательской функции для каждого элемента
    let mut result_items = Vec::new();
    for item in &items {
        let mapped = ctx.call_function(&func_value, &[item.clone()])?;
        result_items.push(mapped);
    }

    // Возврат нового списка
    let new_handle = ctx.heap.allocate(HeapValue::List(result_items));
    Ok(RuntimeValue::List(new_handle))
}
```

**Критерии приёмки**:
- [x] `std.list.push([1, 2], 3)` возвращает `[1, 2, 3]`
- [x] `std.list.pop([1, 2, 3])` возвращает `3` и остаток `[1, 2]`
- [x] `std.list.map([1, 2], x => x * 2)` возвращает `[2, 4]`
- [x] `std.list.filter([1, 2, 3], x => x > 1)` возвращает `[2, 3]`
- [x] `std.list.reduce([1, 2, 3], (acc, x) => acc + x, 0)` возвращает `6`
- [x] Компиляция проходит

**Тестовый план**:
```yaoxiang
// Тест push
let list1 = std.list.push([1, 2], 3);
assert(std.list.len(list1) == 3);

// Тест map
let doubled = std.list.map([1, 2, 3], x => x * 2);
assert(std.list.get(doubled, 0) == 2);

// Тест filter
let filtered = std.list.filter([1, 2, 3, 4], x => x > 2);
assert(std.list.len(filtered) == 2);

// Тест reduce
let sum = std.list.reduce([1, 2, 3], (acc, x) => acc + x, 0);
assert(sum == 6);
```

---

### Шаг 8: Реализация полной функциональности std.dict

**Файл**: `src/std/dict.rs`

**Содержание изменений**:
1. Изменить все сигнатуры функций
2. Реализовать доступ к heap
3. Поддержка ключей типа Any

**Задействованные функции**:

| Функция | Способ реализации |
|---------|-------------------|
| `get` | Получение Dict из heap, поиск ключа |
| `set` | Использование ctx.heap для выделения нового Dict |
| `has` | Получение Dict из heap, проверка ключа |
| `keys/values/entries` | Использование ctx.heap для выделения List |
| `delete` | Использование ctx.heap для выделения нового Dict |
| `merge` | Использование ctx.heap для слияния двух Dict |

**Критерии приёмки**:
- [x] `std.dict.get({a: 1}, "a")` возвращает `1`
- [x] `std.dict.set({a: 1}, "b", 2)` возвращает `{a: 1, b: 2}`
- [x] `std.dict.keys({a: 1, b: 2})` возвращает `["a", "b"]`
- [x] `std.dict.has({a: 1}, "a")` возвращает `true`
- [x] Компиляция проходит

**Тестовый план**:
```yaoxiang
// Тест get
let d = {name: "tom", age: 20};
assert(std.dict.get(d, "name") == "tom");

// Тест set
let d1 = {a: 1};
let d2 = std.dict.set(d1, "b", 2);
assert(std.dict.has(d2, "b") == true);

// Тест keys
let keys = std.dict.keys({x: 1, y: 2});
assert(std.list.len(keys) == 2);
```

---

### Шаг 9: Обновление других модулей std

**Задействованные файлы**:
- `src/std/net.rs`
- `src/std/time.rs`
- `src/std/os.rs`
- `src/std/concurrent.rs`
- `src/std/weak.rs`
- `src/std/ffi.rs` (при наличии тестового кода)

**Содержание изменений**:
- Обновить все сигнатуры функций native, добавить параметр ctx
- Функции, которым не требуется ctx, можно оставить без изменений

**Критерии приёмки**:
- [x] Все модули std компилируются
- [x] Существующий функционал не затронут

---

### Шаг 10: Очистка жёстко закодированного кода в интерпретаторе

**Файл**: `src/backends/interpreter/executor.rs`

**Код для удаления**:
- Специальная обработка `len()` (около строк 609-634)
- Специальная обработка `dict_keys()` (около строк 637-666)

**Внимание**:
- ✅ Сначала завершить шаги 6-8, убедиться в корректной работе функций std
- Затем заменить встроенный `len()` на `std.list.len()`
- Заменить встроенный `dict_keys()` на `std.dict.keys()`

> **Пояснение к реализации**: В реальной реализации, поскольку на этапе генерации IR компилятором генерируются вызовы с голыми именами `"len"` и `"dict_keys"`,
> мы дополнительно зарегистрировали в `register_all()` универсальные функции `builtin_len` и `builtin_dict_keys`,
> которые обрабатывают вычисление длины для List/Tuple/Array/Dict/String/Bytes и извлечение ключей словаря соответственно.

**Критерии приёмки**:
- [x] После удаления жёсткого кода `len()`, `len([1,2,3])` всё ещё работает (через зарегистрированный FFI builtin_len)
- [x] После удаления жёсткого кода `dict_keys()`, `dict_keys({a:1})` всё ещё работает (через зарегистрированный FFI builtin_dict_keys)
- [x] Компиляция проходит

---

## IV. Тестовый план

### 4.1 Модульные тесты

Добавить тесты в каталоге `src/std/`:

```rust
#[cfg(test)]
mod tests {
    // Тесты string
    #[test]
    fn test_split() { ... }

    // Тесты list
    #[test]
    fn test_push() { ... }
    #[test]
    fn test_map() { ... }

    // Тесты dict
    #[test]
    fn test_get() { ... }
}
```

### 4.2 Интеграционные тесты

Создать тестовый файл `tests/std_primitives.yx`:

```yaoxiang
// Тесты строк
let s1 = std.string.trim("  hello  ");
assert(s1 == "hello");

let s2 = std.string.split("a,b,c", ",");
assert(std.list.len(s2) == 3);

// Тесты списков
let l1 = std.list.push([1, 2], 3);
assert(std.list.len(l1) == 3);

let doubled = std.list.map([1, 2, 3], x => x * 2);
assert(std.list.get(doubled, 0) == 2);

// Тесты словарей
let d = std.dict.set({a: 1}, "b", 2);
assert(std.dict.has(d, "b") == true);

// Тесты высших функций
let filtered = std.list.filter([1, 2, 3, 4, 5], x => x > 2);
assert(std.list.len(filtered) == 3);

let sum = std.list.reduce([1, 2, 3, 4], (acc, x) => acc + x, 0);
assert(sum == 10);
```

### 4.3 Регрессионные тесты

Убедиться, что существующий функционал не затронут:

```bash
# Запуск существующих тестов
cargo test

# Запуск интеграционных тестов
cargo run -- tests/std_primitives.yx
```

---

## V. Риски и откат

### 5.1 Риски

| Риск | Влияние | Меры снижения |
|------|---------|---------------|
| Большой объём изменений | Возможны баги | Пошаговая реализация, компиляция на каждом шаге |
| Нарушение существующих функций native | Ошибки runtime | Обновление всех сигнатур модулей std |
| Сложность вызова высших функций | Высокая сложность реализации | Ориентир на существующую логику вызовов интерпретатора |

### 5.2 План отката

При возникновении проблем можно выполнить откат через git:

```bash
git checkout -- src/std/ src/backends/interpreter/ffi.rs src/backends/interpreter/executor.rs
```

---

## VI. Оценка времени

| Шаг | Расчётное время |
|-----|-----------------|
| Шаги 1-3 (ядро FFI) | 1-2 часа |
| Шаги 4-5 (обновление io/math) | 30 минут |
| Шаг 6 (полный string) | 30 минут |
| Шаг 7 (list + высшие функции) | 1-2 часа |
| Шаг 8 (dict) | 1 час |
| Шаги 9-10 (очистка) | 30 минут |
| **Итого** | **5-6 часов** |

---

## VII. Итоги

**Возможности после завершения**:

```yaoxiang
// Строки
std.string.split("a,b,c", ",")  // ["a", "b", "c"]
std.string.chars("hi")          // ["h", "i"]

// Списки
std.list.push([1,2], 3)         // [1, 2, 3]
std.list.map([1,2], x => x*2)   // [2, 4]
std.list.filter([1,2,3], x => x>1)  // [2, 3]
std.list.reduce([1,2,3], (a,x)=>a+x, 0)  // 6

// Словари
std.dict.get({a:1}, "a")       // 1
std.dict.keys({a:1, b:2})      // ["a", "b"]
```