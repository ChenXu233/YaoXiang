# Спецификация стандартной библиотеки

Этот документ определяет спецификацию стандартной библиотеки языка программирования YaoXiang,
включая основную библиотеку, библиотеку ввода-вывода и математическую библиотеку.

---

## Глава 1: Основная библиотека

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

| Вариант       | Синтаксис            | Описание              |
| ------------- | -------------------- | --------------------- |
| `Option.some` | `Option.some(value)` | Значение присутствует |
| `Option.none` | `Option.none()`      | Значение отсутствует  |

**Общие методы**:

```yaoxiang
// Проверить, есть ли значение
is_some: (self: Option(T)) -> Bool
is_none: (self: Option(T)) -> Bool

// Получить значение (может вызвать панику)
unwrap: (self: Option(T)) -> T

// Получить значение или значение по умолчанию
unwrap_or: (self: Option(T), default: T) -> T

// Преобразовать значение
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

**Общие методы**:

```yaoxiang
// Проверить, успешно ли
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// Получить значение (может вызвать панику)
unwrap: (self: Result(T, E)) -> T

// Получить значение или значение по умолчанию
unwrap_or: (self: Result(T, E), default: T) -> T

// Преобразовать значение успеха
map: (R: Type) -> ((self: Result(T, E), f: (T) -> R) -> Result(R, E))

// Преобразовать значение ошибки
map_err: (F: Type) -> ((self: Result(T, E), f: (E) -> F) -> Result(T, F))
```

**Носитель ошибок и коды ошибок (#323 M4)**:

Носитель `Error` в модулях std содержит нормализованные коды ошибок, повторно использующие сегменты
E6xxx/E7xxx из RFC-013 (например, E6009 = Недопустимый шаг Range), образуя стабильный контракт между
версиями — программы могут программно проверять по коду, `yx explain E6009` позволяет просмотреть
документацию. Индекс кодов см. в главе RFC-013 «Согласование значений и кодов ошибок во время
выполнения».

```yaoxiang
// Форма значения Error: { code: String, message: String }

// Извлечь носитель Err (при Ok сообщает об ошибке выполнения)
unwrap_err: (T, E) -> ((self: Result(T, E)) -> E)

// Прочитать код ошибки / сообщение
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
        // Обработка ветки для недопустимого шага Range
        io.println(result.message(e))
    }
}
```

Пользовательское моделирование ошибок осуществляется через параметр обобщения E типа `Result(T, E)`
(набор пользовательских вариантов); `Error` в std — удобный резервный носитель, его система кодов не
ограничивает пользовательский тип E.

### 1.4 Распространение ошибок

```
ErrorPropagate ::= Expr '?'
```

Оператор `?` автоматически распространяет ошибки типа Result:

```
// При успехе возвращает значение, при неудаче возвращает err вверх по стеку
data = fetch_data()?

// Концептуально эквивалентная форма (примечание: сопоставление с деконструкцией
// вариантов через match ещё не реализовано — до поставки RFC-010b будет
// сообщать об ошибке компиляции E3008; `?` — единственный доступный в
// данный момент способ записи распространения ошибок)
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

### 1.5 Утверждения (std.assert)

Модуль `std.assert` предоставляет унифицированный механизм утверждений — `assert` времени выполнения
и уточняющий тип `Assert` времени компиляции являются двумя сторонами одной и той же примитивы.

```yaoxiang
// IsTrue: функция-мост от значения к типу
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, программа продолжается
    false => Never,    // ⊥, расхождение
}

// Assert: примитива уточняющего типа времени компиляции
Assert: (cond: Bool) -> Type = IsTrue(cond)

// assert: утверждение времени выполнения (значение-ввод Assert)
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

// Перегрузка Result
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**Диспетчеризация**:

| Условие                                                    | Поведение                                                                                                 |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Все свободные переменные cond известны во время компиляции | Компилятор вычисляет: true → стирание, false → ошибка компиляции                                          |
| Присутствуют свободные переменные времени выполнения       | Вставляется проверка во время выполнения, инжектируется чувствительное к потоку множество предположений Γ |

`assert(false, "msg")` эквивалентно raise — не требуется отдельное ключевое слово throw/raise.

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

// Логарифм
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
// Итератор диапазона (Range — официальный тип, идентичность во время выполнения —
// неизменяемая запись из трёх скаляров, без оболочки кортежа; выводит `1..10` /
// `1..10..2`, структурное равенство, именованные поля)
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}

// Использование (протокол итератора: std.range.iter/has_next/next, for
// диспетчеризируется через статические типы)
for i in 0..10 {
    print(i)
}

// Форма с шагом (две точки, без новых ключевых слов)
for i in 0..10..2 {
    print(i)
}
```

> **`Range(Int)` официально реализован** — доступны именованные поля `r.start`/`r.end`/`r.step`;
> `x in r` во время выполнения идёт через `std.range.contains` (проверка границ + выравнивание
> шага), конвейер доказательств распознаёт как утверждение об интервале
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0` (интервал остаётся интервалом, не
> материализуется). Литерал step=0 отвергается во время компиляции; динамический step=0 преобразован
> в Result: `std.range.iter` → `Result(Iterator, Error)`, `std.range.contains` →
> `Result(Bool, Error)`, в точке потребления используйте `?` для распространения по стеку вызовов
> или `result.unwrap` для явного разделения; раскрытие синтаксического сахара `for`/`in` происходит
> в ir_gen при распаковке, ветка Err (динамический step=0) явно завершается неудачей
> (`abort_invalid_step`), никогда не приводя к тихому бесконечному циклу. Синтаксис типов для
> инстанцирования интерфейсов (объявление `Iterator(Int)` в теле типа) и статическая диспетчеризация
> реализованы в фазах 1-2 RFC-011a: применение элемента тела типа `Iterator(Int)` запускает
> раскрытие замены `Self ↦ Range` и проверку полноты, после прохождения генерируется доказательство
> реализации. Динамическая диспетчеризация реализована в фазе 3: имя интерфейса существует как тип
> без инстанцирования (`List(Animal)`), конкретные значения, поступающие в позицию экзистенциального
> типа, автоматически оборачиваются как значение варианта, вызовы методов элемента
> диспетчеризируются по фактическому типу (§6). Поверхность протокола времени выполнения модуля
> std.range пока по-прежнему предоставляется собственными методами, миграция на диспетчеризацию
> через интерфейсы — последующая работа.

---

## Приложение: Индекс модулей стандартной библиотеки

| Модуль           | Описание                                                                                                               |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `std.assert`     | Механизм утверждений — assert времени выполнения + уточняющий тип Assert времени компиляции                            |
| `std.option`     | Тип Option                                                                                                             |
| `std.result`     | Тип Result                                                                                                             |
| `std.collection` | Типы коллекций, такие как List, Map                                                                                    |
| `std.string`     | Операции со строками                                                                                                   |
| `std.array`      | Операции с массивами                                                                                                   |
| `std.iterator`   | Итератор (поверхность протокола в настоящее время предоставляется `std.range`)                                         |
| `std.range`      | Итератор Range, предикаты интервалов, адаптеры                                                                         |
| `std.test`       | Библиотека утверждений для тестирования (семантика значений, RFC-036 §3) — первый чистый модуль dogfooding на YaoXiang |

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

| Модуль       | Описание                                                                                    |
| ------------ | ------------------------------------------------------------------------------------------- |
| `std.random` | Генерация случайных чисел                                                                   |
| `std.time`   | Время и дата                                                                                |
| `std.assert` | `Assert(C)` времени компиляции и `assert(x > 0)` времени выполнения унифицированы (RFC-030) |
| `std.regex`  | Регулярные выражения                                                                        |
