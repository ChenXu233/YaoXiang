# Спецификация стандартной библиотеки

Этот документ определяет спецификацию стандартной библиотеки языка программирования YaoXiang, включая базовую библиотеку, библиотеку ввода-вывода и математическую библиотеку.

---

## Глава 1: Базовая библиотека

### 1.1 Базовые типы

Стандартная библиотека предоставляет реализации следующих базовых типов:

| Тип | Модуль | Описание |
|------|------|------|
| `Option(T)` | `std.option` | тип опционального значения |
| `Result(T, E)` | `std.result` | тип обработки ошибок |
| `List(T)` | `std.collection` | динамический массив |
| `Map(K, V)` | `std.collection` | хеш-отображение |
| `String` | `std.string` | строковый тип |
| `Array(T, N)` | `std.array` | массив фиксированного размера |

### 1.2 Тип Option

```
Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
```

**Конструкторы вариантов**:

| Вариант | Синтаксис | Описание |
|------|------|------|
| `Option.some` | `Option.some(value)` | значение присутствует |
| `Option.none` | `Option.none()` | значение отсутствует |

**Основные методы**:

```yaoxiang
// проверка наличия значения
is_some: (self: Option(T)) -> Bool
is_none: (self: Option(T)) -> Bool

// получение значения (может вызвать panic)
unwrap: (self: Option(T)) -> T

// получение значения или значения по умолчанию
unwrap_or: (self: Option(T), default: T) -> T

// отображение значения
map: (R: Type) -> ((self: Option(T), f: (T) -> R) -> Option(R))
```

### 1.3 Тип Result

```
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }
```

**Конструкторы вариантов**:

| Вариант | Синтаксис | Описание |
|------|------|------|
| `Result.ok` | `Result.ok(value)` | значение успеха |
| `Result.err` | `Result.err(error)` | значение ошибки |

**Основные методы**:

```yaoxiang
// проверка успеха
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// получение значения (может вызвать panic)
unwrap: (self: Result(T, E)) -> T

// получение значения или значения по умолчанию
unwrap_or: (self: Result(T, E), default: T) -> T

// отображение значения успеха
map: (R: Type) -> ((self: Result(T, E), f: (T) -> R) -> Result(R, E))

// отображение значения ошибки
map_err: (F: Type) -> ((self: Result(T, E), f: (E) -> F) -> Result(T, F))
```

### 1.4 Распространение ошибок

```
ErrorPropagate ::= Expr '?'
```

Оператор `?` автоматически распространяет ошибки типа Result:

```
// возвращает значение при успехе, возвращает err вверх при неудаче
data = fetch_data()?

// эквивалентно
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```


### 1.5 Утверждения (std.assert)

Модуль `std.assert` предоставляет единый механизм утверждений — runtime `assert` и уточняющий тип compile-time `Assert` являются двумя сторонами одной и той же примитивы.

```yaoxiang
// IsTrue: мостовая функция от значения к типу
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, программа продолжается
    false => Never,    // ⊥, расходимость
}

// Assert: уточняющий тип compile-time примитива
Assert: (cond: Bool) -> Type = IsTrue(cond)

// assert: утверждение runtime (вводящее значение Assert)
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

// Перегрузка Result
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**Диспетчеризация**:

| Условие | Поведение |
|------|------|
| все свободные переменные cond известны compile-time | компилятор вычисляет, true → стирается, false → ошибка компиляции |
| существуют runtime свободные переменные | вставляется runtime check, внедряется чувствительное к потоку множество предположений Γ |

`assert(false, "msg")` эквивалентно raise — нет необходимости в отдельном ключевом слове throw/raise.

---

## Глава 2: Библиотека ввода-вывода

### 2.1 Стандартный ввод-вывод

```yaoxiang
// стандартный вывод
print: (msg: String) -> Void
println: (msg: String) -> Void

// стандартный ввод
read_line: () -> String
read_char: () -> Char
```

### 2.2 Файловые операции

```yaoxiang
// тип файла
File: Type = {
    path: String,
    read: (self: File) -> Result(String, Error),
    write: (self: File, content: String) -> Result(Void, Error),
    append: (self: File, content: String) -> Result(Void, Error),
    close: (self: File) -> Void
}

// файловые операции
open: (path: String) -> Result(File, Error)
create: (path: String) -> Result(File, Error)
delete: (path: String) -> Result(Void, Error)
```

### 2.3 Операции с каталогами

```yaoxiang
// тип каталога
Dir: Type = {
    path: String,
    entries: (self: Dir) -> Result(List(String), Error),
    create: (self: Dir) -> Result(Void, Error),
    delete: (self: Dir) -> Result(Void, Error)
}

// операции с каталогами
read_dir: (path: String) -> Result(Dir, Error)
create_dir: (path: String) -> Result(Void, Error)
delete_dir: (path: String) -> Result(Void, Error)
```

---

## Глава 3: Математическая библиотека

### 3.1 Основные математические функции

```yaoxiang
// абсолютное значение
abs: (x: Int) -> Int
abs: (x: Float) -> Float

// максимум и минимум
max: (a: Int, b: Int) -> Int
min: (a: Int, b: Int) -> Int
max: (a: Float, b: Float) -> Float
min: (a: Float, b: Float) -> Float

// возведение в степень
pow: (base: Float, exp: Float) -> Float
sqrt: (x: Float) -> Float

// логарифм
log: (x: Float) -> Float
log2: (x: Float) -> Float
log10: (x: Float) -> Float
```

### 3.2 Тригонометрические функции

```yaoxiang
// тригонометрические функции
sin: (x: Float) -> Float
cos: (x: Float) -> Float
tan: (x: Float) -> Float

// обратные тригонометрические функции
asin: (x: Float) -> Float
acos: (x: Float) -> Float
atan: (x: Float) -> Float
atan2: (y: Float, x: Float) -> Float
```

### 3.3 Константы

```yaoxiang
// математические константы
pi: Float = 3.141592653589793
e: Float = 2.718281828459045
```

---

## Глава 4: Библиотека строк

### 4.1 Операции со строками

```yaoxiang
// длина строки
length: (s: String) -> Int

// конкатенация строк
concat: (a: String, b: String) -> String

// разделение строки
split: (s: String, delimiter: String) -> List(String)

// поиск в строке
find: (s: String, pattern: String) -> Option(Int)
contains: (s: String, pattern: String) -> Bool

// замена в строке
replace: (s: String, old: String, new: String) -> String

// обрезка строки
trim: (s: String) -> String
trim_left: (s: String) -> String
trim_right: (s: String) -> String
```

### 4.2 Преобразование строк

```yaoxiang
// преобразование типов
to_string: (x: Int) -> String
to_string: (x: Float) -> String
to_string: (x: Bool) -> String

// разбор
parse_int: (s: String) -> Result(Int, Error)
parse_float: (s: String) -> Result(Float, Error)
```

---

## Глава 5: Библиотека коллекций

### 5.1 Тип List

```yaoxiang
// тип List
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
// тип Map
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
// итератор диапазона
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}

// использование
for i in 0..10 {
    print(i)
}

for i in 0..10 step 2 {
    print(i)
}
```

---

## Приложение: Индекс модулей стандартной библиотеки

| Модуль | Описание |
|------|------|
| `std.assert` | механизм утверждений — runtime assert + уточняющий тип compile-time Assert |
| `std.option` | тип Option |
| `std.result` | тип Result |
| `std.collection` | типы коллекций List, Map и т.д. |
| `std.string` | операции со строками |
| `std.array` | операции с массивами |
| `std.iterator` | итератор |

### A.2 Модули ввода-вывода

| Модуль | Описание |
|------|------|
| `std.io` | стандартный ввод-вывод |
| `std.file` | файловые операции |
| `std.dir` | операции с каталогами |

### A.3 Математические модули

| Модуль | Описание |
|------|------|
| `std.math` | математические функции |
| `std.math.trig` | тригонометрические функции |
| `std.math.log` | логарифмические функции |

### A.4 Утилитарные модули
| Модуль | Описание |
|------|------|
| `std.random` | генерация случайных чисел |
| `std.time` | дата и время |
| `std.assert` | унификация compile-time `Assert(C)` и runtime `assert(x > 0)` (RFC-030) |
| `std.regex` | регулярные выражения |