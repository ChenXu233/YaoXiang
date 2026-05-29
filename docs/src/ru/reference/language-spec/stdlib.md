# Спецификация стандартной библиотеки

Настоящий документ определяет спецификацию стандартной библиотеки языка программирования YaoXiang, включая основную библиотеку, библиотеку ввода-вывода и математическую библиотеку.

---

## Глава 1: Основная библиотека

### 1.1 Базовые типы

Стандартная библиотека предоставляет реализации следующих базовых типов:

| Тип | Модуль | Описание |
|------|------|------|
| `Option[T]` | `std.option` | Тип опционального значения |
| `Result[T, E]` | `std.result` | Тип обработки ошибок |
| `List[T]` | `std.collection` | Динамический массив |
| `Map[K, V]` | `std.collection` | Хеш-отображение |
| `String` | `std.string` | Строковый тип |
| `Array[T, N]` | `std.array` | Массив фиксированного размера |

### 1.2 Тип Option

```
Option: Type[T] = some(T) | none
```

**Конструкторы вариантов**:

| Вариант | Синтаксис | Описание |
|------|------|------|
| `some(T)` | `some(value)` | Значение присутствует |
| `none` | `none` | Значение отсутствует |

**Основные методы**:

```yaoxiang
// Проверить наличие значения
is_some: (self: Option[T]) -> Bool
is_none: (self: Option[T]) -> Bool

// Получить значение (может вызвать panic)
unwrap: (self: Option[T]) -> T

// Получить значение или значение по умолчанию
unwrap_or: (self: Option[T], default: T) -> T

// Отобразить значение
map: [R](self: Option[T], f: Fn(T) -> R) -> Option[R]
```

### 1.3 Тип Result

```
Result: Type[T, E] = ok(T) | err(E)
```

**Конструкторы вариантов**:

| Вариант | Синтаксис | Описание |
|------|------|------|
| `ok(T)` | `ok(value)` | Успешное значение |
| `err(E)` | `err(error)` | Значение ошибки |

**Основные методы**:

```yaoxiang
// Проверить успешность
is_ok: (self: Result[T, E]) -> Bool
is_err: (self: Result[T, E]) -> Bool

// Получить значение (может вызвать panic)
unwrap: (self: Result[T, E]) -> T

// Получить значение или значение по умолчанию
unwrap_or: (self: Result[T, E], default: T) -> T

// Отобразить успешное значение
map: [R](self: Result[T, E], f: Fn(T) -> R) -> Result[R, E]

// Отобразить значение ошибки
map_err: [F](self: Result[T, E], f: Fn(E) -> F) -> Result[T, F]
```

### 1.4 Распространение ошибок

```
ErrorPropagate ::= Expr '?'
```

Оператор `?` автоматически распространяет ошибки типа Result:

```
// При успехе возвращает значение, при неудаче возвращает err выше
data = fetch_data()?

// Эквивалентно
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

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

### 2.2 Операции с файлами

```yaoxiang
// Тип файла
File: Type = {
    path: String,
    read: (self: File) -> Result[String, Error],
    write: (self: File, content: String) -> Result[Void, Error],
    append: (self: File, content: String) -> Result[Void, Error],
    close: (self: File) -> Void
}

// Операции с файлами
open: (path: String) -> Result[File, Error]
create: (path: String) -> Result[File, Error]
delete: (path: String) -> Result[Void, Error]
```

### 2.3 Операции с каталогами

```yaoxiang
// Тип каталога
Dir: Type = {
    path: String,
    entries: (self: Dir) -> Result[List[String], Error],
    create: (self: Dir) -> Result[Void, Error],
    delete: (self: Dir) -> Result[Void, Error]
}

// Операции с каталогами
read_dir: (path: String) -> Result[Dir, Error]
create_dir: (path: String) -> Result[Void, Error]
delete_dir: (path: String) -> Result[Void, Error]
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

## Глава 4: Строковая библиотека

### 4.1 Строковые операции

```yaoxiang
// Длина строки
length: (s: String) -> Int

// Конкатенация строк
concat: (a: String, b: String) -> String

// Разделение строки
split: (s: String, delimiter: String) -> List[String]

// Поиск в строке
find: (s: String, pattern: String) -> Option[Int]
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
parse_int: (s: String) -> Result[Int, Error]
parse_float: (s: String) -> Result[Float, Error]
```

---

## Глава 5: Библиотека коллекций

### 5.1 Тип List

```yaoxiang
// Тип List
List: Type[T] = {
    data: Array[T],
    length: Int,
    push: [T](self: List[T], item: T) -> Void,
    pop: [T](self: List[T]) -> Option[T],
    get: [T](self: List[T], index: Int) -> Option[T],
    set: [T](self: List[T], index: Int, value: T) -> Void,
    insert: [T](self: List[T], index: Int, item: T) -> Void,
    remove: [T](self: List[T], index: Int) -> Option[T],
    clear: [T](self: List[T]) -> Void,
    contains: [T](self: List[T], item: T) -> Bool,
    sort: [T](self: List[T]) -> List[T],
    reverse: [T](self: List[T]) -> List[T],
    map: [T, R](self: List[T], f: Fn(T) -> R) -> List[R],
    filter: [T](self: List[T], predicate: Fn(T) -> Bool) -> List[T],
    reduce: [T, R](self: List[T], initial: R, f: Fn(R, T) -> R) -> R
}
```

### 5.2 Тип Map

```yaoxiang
// Тип Map
Map: Type[K, V] = {
    data: Array[(K, V)],
    length: Int,
    insert: [K, V](self: Map[K, V], key: K, value: V) -> Void,
    get: [K, V](self: Map[K, V], key: K) -> Option[V],
    remove: [K, V](self: Map[K, V], key: K) -> Option[V],
    contains_key: [K, V](self: Map[K, V], key: K) -> Bool,
    keys: [K, V](self: Map[K, V]) -> List[K],
    values: [K, V](self: Map[K, V]) -> List[V],
    clear: [K, V](self: Map[K, V]) -> Void
}
```

---

## Глава 6: Библиотека итераторов

### 6.1 Трейт Iterator

```yaoxiang
// Трейт Iterator
Iterator: Type[T] = {
    Item: T,
    next: (self: Self) -> Option[T],
    has_next: (self: Self) -> Bool,
    map: [R](self: Self, f: Fn(T) -> R) -> Iterator[R],
    filter: (self: Self, predicate: Fn(T) -> Bool) -> Iterator[T],
    collect: (self: Self) -> List[T],
    reduce: [R](self: Self, initial: R, f: Fn(R, T) -> R) -> R,
    for_each: (self: Self, f: Fn(T) -> Void) -> Void
}
```

### 6.2 Адаптеры итераторов

```yaoxiang
// Итератор диапазона
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator[Int]
}

// Использование
for i in 0..10 {
    print(i)
}

for i in 0..10 step 2 {
    print(i)
}
```

---

## Приложение: Индекс модулей стандартной библиотеки

### A.1 Основные модули

| Модуль | Описание |
|------|------|
| `std.option` | Тип Option |
| `std.result` | Тип Result |
| `std.collection` | Типы коллекций List, Map и др. |
| `std.string` | Строковые операции |
| `std.array` | Операции с массивами |
| `std.iterator` | Итераторы |

### A.2 Модули ввода-вывода

| Модуль | Описание |
|------|------|
| `std.io` | Стандартный ввод-вывод |
| `std.file` | Операции с файлами |
| `std.dir` | Операции с каталогами |

### A.3 Математические модули

| Модуль | Описание |
|------|------|
| `std.math` | Математические функции |
| `std.math.trig` | Тригонометрические функции |
| `std.math.log` | Логарифмические функции |

### A.4 Вспомогательные модули

| Модуль | Описание |
|------|------|
| `std.random` | Генерация случайных чисел |
| `std.time` | Дата и время |
| `std.regex` | Регулярные выражения |