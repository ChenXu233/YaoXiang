# Стандартная библиотека

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

| Вариант       | Синтаксис            | Описание              |
| ------------- | -------------------- | --------------------- |
| `Option.some` | `Option.some(value)` | Значение присутствует |
| `Option.none` | `Option.none()`      | Значение отсутствует  |

**Часто используемые методы**:

```yaoxiang
// Проверка наличия значения
is_some: (self: Option(T)) -> Bool
is_none: (self: Option(T)) -> Bool

// Извлечение значения (может вызвать panic)
unwrap: (self: Option(T)) -> T

// Извлечение значения или значения по умолчанию
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

**Часто используемые методы**:

```yaoxiang
// Проверка успеха
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// Извлечение значения (может вызвать panic)
unwrap: (self: Result(T, E)) -> T

// Извлечение значения или значения по умолчанию
unwrap_or: (self: Result(T, E), default: T) -> T

// Отображение значения успеха
map: (R: Type) -> ((self: Result(T, E), f: (T) -> R) -> Result(R, E))

// Отображение значения ошибки
map_err: (F: Type) -> ((self: Result(T, E), f: (E) -> F) -> Result(T, F))
```

**Носитель ошибки и коды ошибок (#323 M4)**:

Носитель `Error` типа `Err` в модулях std содержит нормализованные коды ошибок; коды повторно
используют сегменты E6xxx/E7xxx из RFC-013 (например, E6009 = недопустимый шаг Range) как стабильный
межверсионный контракт — программы могут принимать решения по коду, а `yx explain E6009` выводит
документацию. Индекс кодов см. в разделе RFC-013 «Значения ошибок времени выполнения и сквозные
коды».

```yaoxiang
// Форма значения Error: { code: String, message: String }

// Извлечение носителя Err (при Ok — ошибка времени выполнения)
unwrap_err: (T, E) -> ((self: Result(T, E)) -> E)

// Чтение кода / сообщения ошибки
code: (self: Error) -> String
message: (self: Error) -> String
```

**Пример принятия решения по коду**:

```yaoxiang
use std.range
use std.result

r = range.iter(1..10..0)      // step=0 → Err(Error)
if result.is_err(r) {
    e = result.unwrap_err(r)
    if result.code(e) == "E6009" {
        // Обработка ветки недопустимого шага Range
        io.println(result.message(e))
    }
}
```

Пользовательское моделирование ошибок осуществляется через параметр-дженерик E типа `Result(T, E)`
(пользовательский набор вариантов); `Error` в std — это удобный резервный носитель, его система
кодов не ограничивает пользовательские типы E.

### 1.4 Распространение ошибок

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

Модуль `std.assert` предоставляет унифицированный механизм утверждений — утверждение времени
выполнения `assert` и уточняющий тип времени компиляции `Assert` являются двумя сторонами одной и
той же примитивы.

```yaoxiang
// IsTrue: мостовая функция от значения к типу
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, программа продолжается
    false => Never,    // ⊥, расходится
}

// Assert: примитива уточняющего типа времени компиляции
Assert: (cond: Bool) -> Type = IsTrue(cond)

// assert: утверждение времени выполнения (знаковое введение Assert)
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

// Перегрузка Result
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**Диспетчеризация**:

| Условие                                                    | Поведение                                                                              |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Все свободные переменные cond известны во время компиляции | Компилятор вычисляет: true → стирается, false → ошибка компиляции                      |
| Присутствуют свободные переменные времени выполнения       | Вставляется проверка времени выполнения, инжектируется предположение с учётом потока Γ |

`assert(false, "msg")` эквивалентно raise — отдельное ключевое слово throw/raise не требуется.

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

### 3.1 Базовые математические функции

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

## Глава 4: Библиотека для работы со строками

### 4.1 Операции со строками

```yaoxiang
// Длина строки
length: (s: String) -> Int

// Конкатенация строк
concat: (a: String, b: String) -> String

// Разделение строки
split: (s: String, delimiter: String) -> List(String)

// Поиск подстроки
find: (s: String, pattern: String) -> Option(Int)
contains: (s: String, pattern: String) -> Bool

// Замена подстроки
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

// Разбор
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

### 6.1 Трейт Iterator

```yaoxiang
// Трейт Iterator
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
// Итератор диапазона (Range — полноценный тип, среда выполнения представлена
// неизменяемой записью из трёх скаляров, без оболочки Tuple; выводит
// `1..10` / `1..10..2`, структурное равенство, именованные поля)
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}

// Использование (протокол итератора: std.range.iter/has_next/next,
// for использует статическую диспетчеризацию типов)
for i in 0..10 {
    print(i)
}

// Форма со step (две точки, без новых ключевых слов)
for i in 0..10..2 {
    print(i)
}
```

> **`Range(Int)` официально реализован** — доступны именованные поля `r.start` / `r.end` / `r.step`;
> `x in r` во время выполнения проходит через `std.range.contains` (проверка границ + выравнивание
> шага), проверочный конвейер распознаёт предикат диапазона
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0` (диапазон остаётся диапазоном, без
> материализации). Литерал step=0 отвергается во время компиляции; динамический step=0 уже
> представлен как Result: `std.range.iter` → `Result(Iterator, Error)`, `std.range.contains` →
> `Result(Bool, Error)`; точка потребления распространяет ошибку по стеку вызовов через `?` или явно
> ветвится через `result.unwrap`; сахар `for`/`in` раскрывается в ir_gen, при этом ветка Err
> (динамический step=0) явно завершается неудачей (`abort_invalid_step`) и никогда не приводит к
> тихому бесконечному циклу. Инстанцирование интерфейса (объявление `Iterator(Int)` в теле типа) —
> синтаксис типа и статическая диспетчеризация реализованы согласно RFC-011a, этапы 1-2: применение
> `Iterator(Int)` в теле типа запускает раскрытие с заменой `Self ↦ Range` и проверку полноты, после
> прохождения которой генерируется доказательство реализации. Динамическая диспетчеризация
> реализована согласно этапу 3: имя интерфейса без инстанцирования существует как тип
> (`List(Animal)`), конкретные значения на позиции экзистенциального типа автоматически
> оборачиваются как значение варианта, вызовы методов элементов диспетчеризуются по фактическому
> типу (§6). Среда выполнения протокола модуля `std.range` пока по-прежнему обеспечивается
> встроенными методами; миграция на диспетчеризацию через интерфейсы — дальнейшая работа.

---

## Приложение: индекс модулей стандартной библиотеки

| Модуль           | Описание                                                                                                       |
| ---------------- | -------------------------------------------------------------------------------------------------------------- |
| `std.assert`     | Механизм утверждений — assert времени выполнения + уточняющий тип Assert времени компиляции                    |
| `std.option`     | Тип Option                                                                                                     |
| `std.result`     | Тип Result                                                                                                     |
| `std.collection` | Коллекции: List, Map и др.                                                                                     |
| `std.string`     | Операции со строками                                                                                           |
| `std.array`      | Операции с массивами                                                                                           |
| `std.iterator`   | Итераторы (текущий протокол предоставляется модулем `std.range`)                                               |
| `std.range`      | Итератор Range, предикаты диапазона и адаптеры                                                                 |
| `std.test`       | Библиотека тестовых утверждений (семантика значений, RFC-036 §3) — первый чистый модуль dogfooding на YaoXiang |

### A.2 Модули ввода-вывода

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

### A.4 Служебные модули

| Модуль       | Описание                                                                             |
| ------------ | ------------------------------------------------------------------------------------ |
| `std.random` | Генерация случайных чисел                                                            |
| `std.time`   | Дата и время                                                                         |
| `std.assert` | Единые `Assert(C)` времени компиляции и `assert(x > 0)` времени выполнения (RFC-030) |
| `std.regex`  | Регулярные выражения                                                                 |
