---
title: "Состояние стандартной библиотеки"
---

# Стандартная библиотека (Std)

> **Состояние модулей**: есть пробелы (4 пункта требуют доработки)
> **Расположение**:`src/std/`
> **Последнее обновление**: 2026-06-01

---

## Обзор модулей

Стандартная библиотека предоставляет основные функциональные модули языка YaoXiang. Включает модули для IO, математики, строк, списков, словарей, файловой системы, сетевого взаимодействия, параллелизма и других.

**Объём кода**: 5 071 строка (14 подмодулей)

---

## Список функций

### std.io (379 строк) - ✅ Выполнено

| Функция | Сигнатура | Состояние |
|---------|-----------|-----------|
| `print` | `(...args) -> ()` | ✅ |
| `println` | `(...args) -> ()` | ✅ |
| `read_line` | `() -> String` | ✅ |
| `read_file` | `(path: String) -> String` | ✅ |
| `write_file` | `(path: String, content: String) -> Bool` | ✅ |
| `append_file` | `(path: String, content: String) -> Bool` | ✅ |
| `format_fallback` | `(value, type_name: String) -> String` | ✅ |

### std.math (301 строка) - ✅ Выполнено

| Функция | Сигнатура | Состояние |
|---------|-----------|-----------|
| `abs` | `(n: Int) -> Int` | ✅ |
| `max/min` | `(a: Int, b: Int) -> Int` | ✅ |
| `clamp` | `(value: Int, min: Int, max: Int) -> Int` | ✅ |
| `fabs/fmax/fmin` | Float-версии | ✅ |
| `pow` | `(base: Float, exp: Float) -> Float` | ✅ |
| `sqrt` | `(n: Float) -> Float` | ✅ |
| `floor/ceil/round` | `(n: Float) -> Float` | ✅ |
| `sin/cos/tan` | `(n: Float) -> Float` | ✅ |
| `PI/E/TAU` | Константы | ✅ |

### std.string (523 строки) - ✅ Выполнено

| Функция | Сигнатура | Состояние |
|---------|-----------|-----------|
| `split` | `(s: String, sep: String) -> List` | ✅ |
| `trim` | `(s: String) -> String` | ✅ |
| `upper/lower` | `(s: String) -> String` | ✅ |
| `replace` | `(s: String, old: String, new: String) -> String` | ✅ |
| `contains/starts_with/ends_with` | `(s: String, sub: String) -> Bool` | ✅ |
| `index_of` | `(s: String, sub: String) -> Int` | ✅ |
| `substring` | `(s: String, start: Int, end: Int) -> String` | ✅ |
| `is_empty/len` | `(s: String) -> Bool/Int` | ✅ |
| `chars` | `(s: String) -> List` | ✅ |
| `concat/repeat/reverse` | Строковые операции | ✅ |
| `format` | `(format: String, ...args) -> String` | ✅ |

### std.list (784 строки) - ✅ Выполнено

| Функция | Сигнатура | Состояние |
|---------|-----------|-----------|
| `push/pop/append/prepend` | Модификация списка | ✅ |
| `remove_at` | `(list: List, index: Int) -> Any` | ✅ |
| `reverse/concat` | Операции со списком | ✅ |
| `map/filter/reduce` | Функции высшего порядка | ✅ |
| `len/is_empty` | Информация о списке | ✅ |
| `get/set` | Доступ по индексу | ✅ |
| `first/last` | Крайние элементы | ✅ |
| `slice` | `(list: List, start: Int, end: Int) -> List` | ✅ |
| `contains/find_index` | Поиск | ✅ |
| `iter/next/has_next` | Протокол итератора | ✅ |

### std.dict (335 строк) - ✅ Выполнено

| Функция | Сигнатура | Состояние |
|---------|-----------|-----------|
| `get/set` | Доступ к словарю | ✅ |
| `has` | `(dict: Dict, key: Any) -> Bool` | ✅ |
| `keys/values/entries` | Получение коллекций | ✅ |
| `delete` | `(dict: Dict, key: Any) -> Dict` | ✅ |
| `len/is_empty` | Информация о словаре | ✅ |
| `merge` | `(a: Dict, b: Dict) -> Dict` | ✅ |

### std.convert (149 строк) - ✅ Выполнено

- ✅ `to_string` — Преобразование универсальных типов в строку
- ✅ Методы `to_string` для каждого типа: int, float, bool, char, string, list, dict, tuple, set, range

### std.os (1 023 строки) - ✅ Выполнено

- ✅ Операции с файлами: open, close, read, write, seek, tell, flush
- ✅ Операции с директориями: mkdir, rmdir, read_dir
- ✅ Проверка путей: remove, exists, is_file, is_dir
- ✅ Операции с файлами: copy, rename
- ✅ Переменные окружения: get_env, set_env
- ✅ Информация о процессе: args, chdir, getcwd

### std.time (507 строк) - ✅ Выполнено

- ✅ Получение времени: now, timestamp, timestamp_ms
- ✅ `sleep` — `(seconds: Float) -> Void`
- ✅ Форматирование: format_time, parse_time (в стиле strftime)
- ✅ Методы DateTime: year, month, day, hour, minute, second, weekday, to_string

### std.net (177 строк) - ⚠️ Заглушка

| Функция | Сигнатура | Состояние |
|---------|-----------|-----------|
| `http_get` | `(url: String) -> String` | ⚠️ Заглушка — возвращает `"GET: {url}"` |
| `http_post` | `(url: String, body: String) -> String` | ⚠️ Заглушка — возвращает `"POST {url}: {body}"` |
| `url_encode` | `(s: String) -> String` | ✅ |
| `url_decode` | `(s: String) -> String` | ✅ |

### std.concurrent (85 строк) - ✅ Базовая реализация

- ✅ `sleep` — `(millis: Int) -> Void`
- ✅ `thread_id` — `() -> String`
- ✅ `yield_now` — `() -> Void`

### std.ffi (265 строк) - ✅ Выполнено

- ✅ `native` — `(symbol: String) -> Never` (перехват на этапе компиляции)

### std.weak (45 строк) - ⚠️ Базовая реализация

- ✅ `weak_new` — `(arc) -> Weak`
- ✅ `weak_upgrade` — `(weak) -> Option`
- ⚠️ Отсутствует реализация trait `StdModule`, невозможно импортировать через `use std.weak`

### gen_interfaces (208 строк) - ✅ Выполнено

- ✅ Автоматическая генерация файлов интерфейсов `.yx`
- ✅ Поддержка записи в директорию, поиска файлов интерфейсов

---

## Покрытие тестами

**Всего 8 модульных тестов**, что крайне недостаточно:

| Модуль | Количество модульных тестов | Состояние |
|--------|---------------------------|-----------|
| io | 0 | ❌ Отсутствуют |
| math | 0 | ❌ Отсутствуют |
| string | 0 | ❌ Отсутствуют |
| list | 0 | ❌ Отсутствуют |
| dict | 0 | ❌ Отсутствуют |
| convert | 0 | ❌ Отсутствуют |
| os | 0 | ❌ Отсутствуют |
| time | 0 | ❌ Отсутствуют |
| net | 0 | ❌ Отсутствуют |
| concurrent | 0 | ❌ Отсутствуют |
| ffi | 2 | ✅ Базовое покрытие |
| gen_interfaces | 6 | ✅ Хорошее покрытие |

**Косвенное тестовое покрытие**:
- `tests/yx_runner.rs` покрывает часть функциональности через E2E-тесты
- `tests/integration/execution.rs` содержит базовые интеграционные тесты

---

## Обнаруженные проблемы

1. **Модуль net реализован как заглушка**: `http_get` и `http_post` возвращают имитационные строки
2. **Модуль weak неполон**: отсутствует реализация trait `StdModule`, невозможно импортировать через `use std.weak`
3. **os.chdir не выполняет переключение директорий**: только проверяет существование директории, не вызывает `std::env::set_current_dir()`
4. **string.len возвращает количество байтов**: `native_len` использует `s.len()`, возвращая количество байт, а не символов

---

## Оценка качества кода

| Измерение | Оценка | Пояснение |
|-----------|--------|-----------|
| Незавершённые задачи | 4 | Дополнить тесты, исправить баги, модуль weak, HTTP-заглушка |
| Тестовое покрытие | Критически недостаточно | Всего 8 модульных тестов |
| Качество документации | Хорошее | Каждый модуль имеет документационные комментарии на уровне модуля `//!` |
| Архитектура кода | Хорошая | Чёткое разделение на модули |

---

## Пункты для доработки

1. **Добавить модульные тесты для всех модулей** (наивысший приоритет)
2. **Исправить проблемы с `os.chdir` и `string.len`**
3. **Дополнить реализацию `StdModule` в модуле `weak`**
4. **Реализовать реальную HTTP-функциональность или явно пометить как заглушку**