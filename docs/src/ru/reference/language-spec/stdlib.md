# Спецификация стандартной библиотеки

Данный документ определяет спецификацию стандартной библиотеки языка программирования YaoXiang,
включая ядро, библиотеку ввода-вывода и математическую библиотеку.

---

## Глава 1: Ядро

### 1.1 Базовые типы

Стандартная библиотека предоставляет реализации следующих базовых типов:

| Тип            | Модуль           | Описание                      |
| -------------- | ---------------- | ----------------------------- |
| `Option(T)`    | `std.option`     | Тип опционального значения    |
| `Result(T, E)` | `std.result`     | Тип для обработки ошибок      |
| `List(T)`      | `std.collection` | Динамический массив           |
| `Map(K, V)`    | `std.collection` | Хеш-отображение               |
| `String`       | `std.string`     | Строковый тип                 |
| `Array(T, N)`  | `std.array`      | Массив фиксированного размера |

### 1.2 Тип Option

```
Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
```

**Конструкторы вариантов**:

| Вариант       | Синтаксис            | Описание          |
| ------------- | -------------------- | ----------------- |
| `Option.some` | `Option.some(value)` | Содержит значение |
| `Option.none` | `Option.none()`      | Пустое значение   |

**Основные методы**:

```yaoxiang
// Проверка наличия значения
is_some: (self: Option(T)) -> Bool
is_none: (self: Option(T)) -> Bool

// Получение значения (может вызвать panic)
unwrap: (self: Option(T)) -> T

// Получение значения или значения по умолчанию
unwrap_or: (self: Option(T), default: T) -> T

// Отображение значения
map: (R: Type) -> ((self: Option(T), f: (T) -> R) -> Option(R))
```

### 1.3 Тип Result

```
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }
```

**Конструкторы вариантов**:

| Вариант      | Синтаксис           | Описание        |
| ------------ | ------------------- | --------------- |
| `Result.ok`  | `Result.ok(value)`  | Значение успеха |
| `Result.err` | `Result.err(error)` | Значение ошибки |

**Основные методы**:

```yaoxiang
// Проверка успешности
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// Получение значения (может вызвать panic)
unwrap: (self: Result(T, E)) -> T

// Получение значения или значения по умолчанию
unwrap_or: (self: Result(T, E), default: T) -> T

// Отображение значения успеха
map: (R: Type) -> ((self: Result(T, E), f: (T) -> R) -> Result(R, E))

// Отображение значения ошибки
map_err: (F: Type) -> ((self: Result(T, E), f: (E) -> F) -> Result(T, F))
```

### 1.4 Error propagation

```
ErrorPropagate ::= Expr '?'
```

Оператор `?` автоматически распространяет ошибки типа Result:

```
// При успехе возвращает значение, при неудаче возвращает err вверх по стеку
data = fetch_data()?

// Эквивалентно
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

### 1.5 Утверждения (std.assert)

Модуль `std.assert` предоставляет унифицированный механизм утверждений — runtime `assert` и
compile-time уточняющий тип `Assert` являются двумя сторонами одной и той же сущности.

```yaoxiang
// IsTrue: мост от значения к типу
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, программа продолжается
    false => Never,    // ⊥, расходится
}

// Assert: примитив уточняющего типа compile-time
Assert: (cond: Bool) -> Type = IsTrue(cond)

// assert: утверждение runtime (value introducer для Assert)
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

// Перегрузка для Result
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**Диспетчеризация**:

| Условие                                             | Поведение                                                                     |
| --------------------------------------------------- | ----------------------------------------------------------------------------- |
| Все свободные переменные cond известны compile-time | Компилятор вычисляет, true → стирается, false → ошибка компиляции             |
| Существуют свободные переменные runtime             | Вставляется runtime check, инъектируется flow-sensitive набор предположений Γ |

`assert(false, "msg")` эквивалентно raise — отдельные ключевые слова throw/raise не нужны.

---

## Глава 2: Библиотека ввода-вывода

### 2.1 Стандартный ввод-вывод

```yaoxiang
// Стандартный вывод
print: (msg: String) -> Void
println: (msg: String) -> Void

// Стандартный ввод
read_line: () -> String
read_char: () -> Char
```

### 2.2 Файловые операции

```yaoxiang
// Тип файла
File: Type = {
    path: String,
    read: (self: File) -> Result(String, Error),
    write: (self: File, content: String) -> Result(Void, Error),
    append: (self: File, content: String) -> Result(Void, Error),
    close: (self: File) -> Void
}

// Файловые операции
open: (path: String) -> Result(File, Error)
create: (path: String) -> Result(File, Error)
delete: (path: String) -> Result(Void, Error)
```

### 2.3 Операции с каталогами

```yaoxiang
// Тип каталога
Dir: Type = {
    path: String,
    entries: (self: Dir) -> Result(List(String), Error),
    create: (self: Dir) -> Result(Void, Error),
    delete: (self: Dir) -> Result(Void, Error)
}

// Операции с каталогами
read_dir: (path: String) -> Result(Dir, Error)
create_dir: (path: String) -> Result(Void, Error)
delete_dir: (path: String) -> Result(Void, Error)
```

---

## Глава 3: Математическая библиотека

### 3.1 Основные математические функции

```yaoxiang
// Абсолютное значение
abs: (x: Int) -> Int
abs: (x: Float) -> Float

// Максимум и минимум
max: (a: Int, b: Int) -> Int
min: (a: Int, b: Int) -> Int
max: (a: Float, b: Float) -> Float
min: (a: Float, b: Float) -> Float

// Возведение в степень
pow: (base: Float, exp: Float) -> Float
sqrt: (x: Float) -> Float

// Логарифмы
log: (x: Float) -> Float
log2: (x: Float) -> Float
log10: (x: Float) -> Float
```

### 3.2 Тригонометрические функции

```yaoxiang
// Тригонометрические функции
sin: (x: Float) -> Float
cos: (x: Float) -> Float
tan: (x: Float) -> Float

// Обратные тригонометрические функции
asin: (x: Float) -> Float
acos: (x: Float) -> Float
atan: (x: Float) -> Float
atan2: (y: Float, x: Float) -> Float
```

### 3.3 Константы

```yaoxiang
// Математические константы
pi: Float = 3.141592653589793
e: Float = 2.718281828459045
```

---

## Глава 4: Библиотека строк

### 4.1 Операции со строками

```yaoxiang
// Длина строки
length: (s: String) -> Int

// Конкатенация строк
concat: (a: String, b: String) -> String

// Разделение строки
split: (s: String, delimiter: String) -> List(String)

// Поиск в строке
find: (s: String, pattern: String) -> Option(Int)
contains: (s: String, pattern: String) -> Bool

// Замена в строке
replace: (s: String, old: String, new: String) -> String

// Обрезка строки
trim: (s: String) -> String
trim_left: (s: String) -> String
trim_right: (s: String) -> String
```

### 4.2 Преобразование строк

```yaoxiang
// Преобразование типов
to_string: (x: Int) -> String
to_string: (x: Float) -> String
to_string: (x: Bool) -> String

// Парсинг
parse_int: (s: String) -> Result(Int, Error)
parse_float: (s: String) -> Result(Float, Error)
```

---

## Глава 5: Библиотека коллекций

### 5.1 Тип List

```yaoxiang
// Тип List
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (T: Type) -> ((self: List(T), item: T) -> Void),
    pop: (T: Type) -> ((self: List(T)) -> Option(T)),
    get: (T: Type) -> ((self: List(T), index: Int) -> Option(T)),
    set: (T: Type) -> ((self: List(T), index: Int, value: T) -> Void),
    insert: (T: Type) -> ((self: List(T), index: Int, item: T) -> Void),
    remove: (T: Type) -> ((self: List(T), index: Int) -> Option(T)),
    clear: (T: Type) -> ((self: List(T)) -> Void),
    contains: (T: Type) -> ((self: List(T), item: T) -> Bool),
    sort: (T: Type) -> ((self: List(T)) -> List(T)),
    reverse: (T: Type) -> ((self: List(T)) -> List(T)),
    map: (T: Type, R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (T: Type) -> ((self: List(T), predicate: (T) -> Bool) -> List(T)),
    reduce: (T: Type, R: Type) -> ((self: List(T), initial: R, f: (R, T) -> R) -> R)
}
```

### 5.2 Тип Map

```yaoxiang
// Тип Map
Map: (K: Type, V: Type) -> Type = {
    data: Array((K, V)),
    length: Int,
    insert: (K: Type, V: Type) -> ((self: Map(K, V), key: K, value: V) -> Void),
    get: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Option(V)),
    remove: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Option(V)),
    contains_key: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Bool),
    keys: (K: Type, V: Type) -> ((self: Map(K, V)) -> List(K)),
    values: (K: Type, V: Type) -> ((self: Map(K, V)) -> List(V)),
    clear: (K: Type, V: Type) -> ((self: Map(K, V)) -> Void)
}
```

---

## Глава 6: Библиотека итераторов

### 6.1 Iterator trait

```yaoxiang
// Iterator trait
Iterator: (T: Type) -> Type = {
    Item: T,
    next: () -> Option(T),
    has_next: () -> Bool,
    map: (R: Type) -> ((f: (T) -> R) -> Iterator(R)),
    filter: (predicate: (T) -> Bool) -> Iterator(T),
    collect: () -> List(T),
    reduce: (R: Type) -> ((initial: R, f: (R, T) -> R) -> R),
    for_each: (f: (T) -> Void) -> Void
}
```

### 6.2 Адаптеры итераторов

```yaoxiang
// Итератор по диапазону (#302: Range — официальный тип, runtime-идентичность — неизменяемая запись с тремя скалярами,
// больше не использует оболочку Tuple; выводит `1..10` / `1..10..2`, структурное равенство, именованные поля)
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}

// Использование (протокол итератора: std.range.iter/has_next/next, for через статическую диспетчеризацию типов)
for i in 0..10 {
    print(i)
}

// Форма со step (две точки, без новых ключевых слов)
for i in 0..10..2 {
    print(i)
}
```

> **#302**: `Range(Int)` официально реализован — доступны именованные поля
> `r.start`/`r.end`/`r.step`; `x in r` runtime идёт через `std.range.contains` (проверка границ +
> выравнивание шага), что доказывает, что конвейер распознаёт предикат интервала
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0` (интервал остаётся интервалом, не
> материализуется). Литерал step=0 отклоняется compile-time, динамический step=0 — runtime ошибка
> (после внедрения будущей системы ошибок повышается до Result, #301). Инстанцирование интерфейса
> (объявление тела типа `Iterator(Int)`) — синтаксис типа и статическая диспетчеризация реализованы
> в рамках фаз 1-2 RFC-011a (#307): элемент применения типа `Iterator(Int)` в теле типа запускает
> подстановку `Self ↦ Range` и проверку полноты, после прохождения которой генерируется
> доказательство реализации. Динамическая диспетчеризация реализована в рамках фазы 3: если имя
> интерфейса не инстанцировано, тип существует (`List(Animal)`), конкретные значения в позиции
> экзистенциального типа автоматически оборачиваются в варианты, вызовы методов элементов
> диспетчеризуются по фактическому типу (§6). Протокольная сторона runtime модуля `std.range` в
> настоящее время обеспечивается нативными методами, миграция на диспетчеризацию интерфейса —
> дальнейшая работа.

---

## Приложение: Указатель модулей стандартной библиотеки

| Модуль           | Описание                                                                            |
| ---------------- | ----------------------------------------------------------------------------------- |
| `std.assert`     | Механизм утверждений — runtime assert + compile-time уточняющий тип Assert          |
| `std.option`     | Тип Option                                                                          |
| `std.result`     | Тип Result                                                                          |
| `std.collection` | Коллекции List, Map и др.                                                           |
| `std.string`     | Операции со строками                                                                |
| `std.array`      | Операции с массивами                                                                |
| `std.iterator`   | Итератор (протокольная сторона в настоящее время предоставляется `std.range`, #302) |
| `std.range`      | Итератор Range и предикаты интервалов, адаптеры (#302)                              |

### A.2 Модуль IO

| Модуль     | Описание               |
| ---------- | ---------------------- |
| `std.io`   | Стандартный ввод-вывод |
| `std.file` | Файловые операции      |
| `std.dir`  | Операции с каталогами  |

### A.3 Математические модули

| Модуль          | Описание                   |
| --------------- | -------------------------- |
| `std.math`      | Математические функции     |
| `std.math.trig` | Тригонометрические функции |
| `std.math.log`  | Логарифмические функции    |

### A.4 Утилитарные модули

| Модуль       | Описание                                                            |
| ------------ | ------------------------------------------------------------------- |
| `std.random` | Генерация случайных чисел                                           |
| `std.time`   | Время и дата                                                        |
| `std.assert` | Единые compile-time `Assert(C)` и runtime `assert(x > 0)` (RFC-030) |
| `std.regex`  | Регулярные выражения                                                |
