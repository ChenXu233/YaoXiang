# Спецификация стандартной библиотеки

Настоящий документ определяет спецификацию стандартной библиотеки языка программирования YaoXiang,
включая ядро, библиотеку ввода-вывода и математическую библиотеку.

---

## Глава 1: Ядро

### 1.1 Базовые типы

Стандартная библиотека предоставляет реализации следующих базовых типов:

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

| Вариант       | Синтаксис            | Описание      |
| ------------- | -------------------- | ------------- |
| `Option.some` | `Option.some(value)` | Есть значение |
| `Option.none` | `Option.none()`      | Нет значения  |

**Часто используемые методы**:

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

| Вариант      | Синтаксис           | Описание          |
| ------------ | ------------------- | ----------------- |
| `Result.ok`  | `Result.ok(value)`  | Успешное значение |
| `Result.err` | `Result.err(error)` | Значение ошибки   |

**Часто используемые методы**:

```yaoxiang
// Проверка успешности
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// Получение значения (может вызвать panic)
unwrap: (self: Result(T, E)) -> T

// Получение значения или значения по умолчанию
unwrap_or: (self: Result(T, E), default: T) -> T

// Отображение успешного значения
map: (R: Type) -> ((self: Result(T, E), f: (T) -> R) -> Result(R, E))

// Отображение значения ошибки
map_err: (F: Type) -> ((self: Result(T, E), f: (E) -> F) -> Result(T, F))
```

**Носитель ошибки и коды ошибок (#323 M4)**:

Носитель `Error` модулей std несёт нормализованный код ошибки; коды повторно используют сегменты
E6xxx/E7xxx из RFC-013 (например, E6009 = недопустимый шаг Range) как стабильный контракт между
версиями — программы могут программно ветвиться по коду, а `yaoxiang explain E6009` выдаёт
документацию. Указатель кодов см. в разделе RFC-013 «Значения ошибок времени выполнения и сквозные
коды».

```yaoxiang
// Форма значения Error: { code: String, message: String }

// Извлечение носителя Err (ошибка времени выполнения при Ok)
unwrap_err: (T, E) -> ((self: Result(T, E)) -> E)

// Чтение кода / сообщения ошибки
code: (self: Error) -> String
message: (self: Error) -> String
```

**Пример ветвления по коду**:

```yaoxiang
use std.range
use std.result

r = range.iter(1..10..0)      // step=0 → Err(Error)
if result.is_err(r) {
    e = result.unwrap_err(r)
    if result.code(e) == "E6009" {
        // Ветвь обработки: недопустимый шаг Range
        io.println(result.message(e))
    }
}
```

Пользовательское моделирование ошибок идёт через обобщённый параметр E типа `Result(T, E)`
(пользовательский набор вариантов); std-овский `Error` — удобный резервный носитель, чья система
кодов не ограничивает пользовательский тип E.

### 1.4 Распространение ошибок

```
ErrorPropagate ::= Expr '?'
```

Оператор `?` автоматически распространяет ошибки типа Result:

```
// При успехе возвращает значение, при неудаче — поднимает err вверх
data = fetch_data()?

// Эквивалентно
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

### 1.5 Утверждения (std.assert)

Модуль `std.assert` предоставляет единый механизм утверждений — утверждение времени выполнения
`assert` и уточняющий тип времени компиляции `Assert` суть две стороны одной сущности.

```yaoxiang
// IsTrue: мостовая функция от значения к типу
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, программа продолжается
    false => Never,    // ⊥, расходимость
}

// Assert: примитив уточнения типа на этапе компиляции
Assert: (cond: Bool) -> Type = IsTrue(cond)

// assert: утверждение времени выполнения (value introduction для Assert)
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

// Перегрузка для Result
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**Диспетчеризация (dispatch)**:

| Условие                                                    | Поведение                                                                                          |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Все свободные переменные cond известны на этапе компиляции | Компилятор вычисляет: true → стирается, false → ошибка компиляции                                  |
| Имеются свободные переменные времени выполнения            | Вставляется проверка времени выполнения, инжектируется потоко-чувствительный набор предположений Γ |

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

## Глава 4: Библиотека строк

### 4.1 Операции со строками

```yaoxiang
// Длина строки
length: (s: String) -> Int

// Конкатенация строк
concat: (a: String, b: String) -> String

// Разбиение строки
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
// Итератор по диапазону (Range — полноценный тип, идентичность времени выполнения —
// неизменяемая запись из трёх скаляров; больше не использует обёртку Tuple;
// печатает `1..10` / `1..10..2`, структурное равенство, именованные поля)
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}

// Использование (протокол итератора: std.range.iter/has_next/next, for —
// через статическую диспетчеризацию типов)
for i in 0..10 {
    print(i)
}

// Форма с шагом (две точки, без новых ключевых слов)
for i in 0..10..2 {
    print(i)
}
```

> **`Range(Int)` официально утверждён** — доступны именованные поля `r.start`/`r.end`/`r.step`;
> `x in r` во время выполнения проходит через `std.range.contains` (проверка границ + согласование
> шага), и конвейер доказательства распознаёт предикат интервала
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0` (интервал остаётся интервалом, не
> материализуется). Литерал step=0 отвергается на этапе компиляции; динамический step=0 обёрнут в
> Result: `std.range.iter` → `Result(Iterator, Error)`, `std.range.contains` →
> `Result(Bool, Error)`; точка потребления распространяет ошибку по стеку через `?` либо явно
> ветвится через `result.unwrap`; сахар `for`/`in` раскрывается в ir_gen, ветвь Err (динамический
> step=0) явно завершается неудачей (`abort_invalid_step`), никогда не превращаясь в тихий
> бесконечный цикл. Инстанцирование интерфейса (объявление элемента тела типа `Iterator(Int)`) и
> статическая диспетчеризация уже введены в рамках фаз 1–2 RFC-011a: применение элемента тела типа
> `Iterator(Int)` запускает подстановку `Self ↦ Range` с проверкой полноты, после прохождения
> которой генерируется доказательство реализации. Динамическая диспетчеризация введена в рамках фазы
> 3: если имя интерфейса присутствует как тип без инстанцирования (`List(Animal)`), конкретное
> значение в позиции экзистенциального типа автоматически оборачивается в вариантное значение,
> вызовы методов элементов диспетчеризируются по фактическому типу (§6). Поверхность протокола
> времени выполнения модуля `std.range` пока по-прежнему обеспечивается встроенными методами;
> миграция на диспетчеризацию через интерфейс остаётся последующей работой.

---

## Приложение: Указатель модулей стандартной библиотеки

| Модуль           | Описание                                                                                                   |
| ---------------- | ---------------------------------------------------------------------------------------------------------- |
| `std.assert`     | Механизм утверждений — runtime assert + уточняющий тип компиляции Assert                                   |
| `std.option`     | Тип Option                                                                                                 |
| `std.result`     | Тип Result                                                                                                 |
| `std.collection` | Типы коллекций List, Map и др.                                                                             |
| `std.string`     | Операции со строками                                                                                       |
| `std.array`      | Операции с массивами                                                                                       |
| `std.iterator`   | Итераторы (поверхность протокола в настоящее время обеспечивается `std.range`)                             |
| `std.range`      | Итератор Range, предикаты и адаптеры интервалов                                                            |
| `std.test`       | Библиотека тестовых утверждений (семантика значений, RFC-036 §3) — первый чисто-YaoXiang dogfooding-модуль |

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

### A.4 Утилитарные модули

| Модуль       | Описание                                                                              |
| ------------ | ------------------------------------------------------------------------------------- |
| `std.random` | Генерация случайных чисел                                                             |
| `std.time`   | Время и дата                                                                          |
| `std.assert` | Единые `Assert(C)` на этапе компиляции и `assert(x > 0)` времени выполнения (RFC-030) |
| `std.regex`  | Регулярные выражения                                                                  |
