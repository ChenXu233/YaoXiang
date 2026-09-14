# Спецификация стандартной библиотеки

Данный документ определяет спецификацию стандартной библиотеки языка программирования YaoXiang,
включая базовую библиотеку, библиотеку ввода-вывода и математическую библиотеку.

---

## Глава 1: Базовая библиотека

### 1.1 Базовые типы

Стандартная библиотека предоставляет следующие реализации базовых типов:

| Тип            | Модуль           | Описание                      |
| -------------- | ---------------- | ----------------------------- |
| `Option(T)`    | `std.option`     | Тип опционального значения    |
| `Result(T, E)` | `std.result`     | Тип обработки ошибок          |
| `List(T)`      | `std.collection` | Динамический массив           |
| `Map(K, V)`    | `std.collection` | Хеш-отображение               |
| `String`       | `std.string`     | Строковый тип                 |
| `Array(T, N)`  | `std.array`      | Массив фиксированного размера |

### 1.2 Тип Option

```
Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
```

**Конструкторы вариантов**:

| Вариант       | Синтаксис            | Описание             |
| ------------- | -------------------- | -------------------- |
| `Option.some` | `Option.some(value)` | Содержит значение    |
| `Option.none` | `Option.none()`      | Не содержит значения |

**Основные методы**:

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

| Вариант      | Синтаксис           | Описание          |
| ------------ | ------------------- | ----------------- |
| `Result.ok`  | `Result.ok(value)`  | Успешное значение |
| `Result.err` | `Result.err(error)` | Значение ошибки   |

**Основные методы**:

```yaoxiang
// Проверка успешности
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// Извлечение значения (может вызвать panic)
unwrap: (self: Result(T, E)) -> T

// Извлечение значения или значения по умолчанию
unwrap_or: (self: Result(T, E), default: T) -> T

// Отображение успешного значения
map: (R: Type) -> ((self: Result(T, E), f: (T) -> R) -> Result(R, E))

// Отображение значения ошибки
map_err: (F: Type) -> ((self: Result(T, E), f: (E) -> F) -> Result(T, F))
```

**Носитель Error и коды ошибок (#323 M4)**:

Носитель `Error` из модулей std в `Err` несёт нормализованный код ошибки. Коды повторно используют
сегменты E6xxx/E7xxx из RFC-013 (например, E6009 = недопустимый шаг Range), что обеспечивает
стабильный контракт между версиями — программа может программно проверять коды, а `yx explain E6009`
выводит документацию. Индекс кодов см. в разделе RFC-013 «Согласованность значений и кодов ошибок
runtime».

```yaoxiang
// Форма значения Error: { code: String, message: String }

// Извлечение носителя Err (при Ok возвращает ошибку runtime)
unwrap_err: (T, E) -> ((self: Result(T, E)) -> E)

// Чтение кода / сообщения ошибки
code: (self: Error) -> String
message: (self: Error) -> String
```

**Пример проверки по коду**:

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

Моделирование пользовательских ошибок осуществляется через обобщённый параметр E типа `Result(T, E)`
(пользовательский набор вариантов). `Error` в std — это удобный резервный носитель, чья система
кодов не ограничивает пользовательский тип E.

### 1.4 Распространение ошибок (error propagation)

```
ErrorPropagate ::= Expr '?'
```

Оператор `?` автоматически распространяет ошибки типа Result:

```
// При успехе возвращает значение, при неудаче — возвращает err наверх
data = fetch_data()?

// Концептуально эквивалентная форма (внимание: деструктуризация вариантов
// через match ещё не реализована — до поставки RFC-039 будет выдаваться
// ошибка компиляции E3008; `?` — единственный доступный сейчас синтаксис
// распространения ошибок)
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

### 1.5 Утверждения (std.assert)

Модуль `std.assert` предоставляет единый механизм утверждений — runtime-`assert` и compile-time
уточняющий тип `Assert` являются двумя сторонами одной и той же примитивы.

```yaoxiang
// IsTrue: мостовая функция «значение → тип»
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, программа продолжается
    false => Never,    // ⊥, расходимость
}

// Assert: примитива уточняющего типа compile-time
Assert: (cond: Bool) -> Type = IsTrue(cond)

// assert: runtime-утверждение (value-introducer для Assert)
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

// Перегрузка для Result
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**Диспетчеризация (dispatch)**:

| Условие                                                | Поведение                                                         |
| ------------------------------------------------------ | ----------------------------------------------------------------- |
| Все свободные переменные cond известны на compile-time | Компилятор вычисляет: true → стирается, false → ошибка компиляции |
| Присутствуют runtime-свободные переменные              | Вставляется runtime-проверка, инжектируется flow-sensitive Γ      |

`assert(false, "msg")` эквивалентно raise — отдельные ключевые слова throw/raise не нужны.

---

## Глава 2: Библиотека ввода-вывода (IO)

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

### 4.1 Операции над строками

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

// Разбор (парсинг)
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

### 6.1 Trait Iterator

```yaoxiang
// Trait Iterator
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
// Итератор диапазона (Range — полноценный тип, runtime-идентичность —
// неизменяемая запись из трёх скаляров, без обёртки Tuple; печатается
// как `1..10` / `1..10..2`, структурное равенство, именованные поля)
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}

// Использование (протокол итератора: std.range.iter/has_next/next,
// for выполняет статическую диспетчеризацию по типам)
for i in 0..10 {
    print(i)
}

// Форма с шагом (две точки, без новых ключевых слов)
for i in 0..10..2 {
    print(i)
}
```

> **`Range(Int)` официально реализован** — доступны именованные поля `r.start`/`r.end`/`r.step`;
> `x in r` в runtime идёт через `std.range.contains` (проверка границ + выравнивание шага),
> доказывающий конвейер распознаёт предикат интервала
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0` (интервал сохраняет интервал, без
> материализации). Литерал step=0 отвергается на этапе компиляции; динамический step=0 уже
> Result-изирован: `std.range.iter` → `Result(Iterator, Error)`, `std.range.contains` →
> `Result(Bool, Error)`. В местах потребления используется `?` для распространения по стеку вызовов
> или `result.unwrap` для явного ветвления; сахар `for`/`in` разворачивается и распаковывается в
> ir_gen, ветка Err (динамический step=0) явно завершается неудачей (`abort_invalid_step`), никогда
> не уходит в тихий бесконечный цикл. Инстанциация интерфейса (объявление `Iterator(Int)` в теле
> типа) и синтаксис статической диспетчеризации реализованы согласно RFC-011a, этапы 1-2: применение
> `Iterator(Int)` в теле типа запускает подстановку `Self ↦ Range` и проверку полноты, после
> прохождения генерируется доказательство реализации. Динамическая диспетчеризация реализована
> согласно этапу 3: неинстанцированное имя интерфейса существует как тип (`List(Animal)`),
> конкретные значения в позиции экзистенциального типа автоматически оборачиваются как value
> variant, вызовы методов элементов диспетчеризуются по фактическому типу (§6). Runtime-протокольная
> поверхность модуля `std.range` по-прежнему предоставляется собственными методами; миграция на
> диспетчеризацию через интерфейс — будущая работа.

---

## Приложение: Индекс модулей стандартной библиотеки

| Модуль           | Описание                                                                                                 |
| ---------------- | -------------------------------------------------------------------------------------------------------- |
| `std.assert`     | Механизм утверждений — runtime assert + compile-time уточняющий тип Assert                               |
| `std.option`     | Тип Option                                                                                               |
| `std.result`     | Тип Result                                                                                               |
| `std.collection` | Коллекционные типы List, Map и др.                                                                       |
| `std.string`     | Операции над строками                                                                                    |
| `std.array`      | Операции над массивами                                                                                   |
| `std.iterator`   | Итераторы (протокольная поверхность сейчас предоставляется `std.range`)                                  |
| `std.range`      | Итератор Range, предикаты интервалов, адаптеры                                                           |
| `std.test`       | Библиотека тестовых утверждений (value-семантика, RFC-036 §3) — первый чистый YaoXiang dogfooding-модуль |

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

| Модуль       | Описание                                                                |
| ------------ | ----------------------------------------------------------------------- |
| `std.random` | Генерация случайных чисел                                               |
| `std.time`   | Время и дата                                                            |
| `std.assert` | Унификация compile-time `Assert(C)` и runtime `assert(x > 0)` (RFC-030) |
| `std.regex`  | Регулярные выражения                                                    |
