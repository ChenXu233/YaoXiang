# Спецификация системы типов

Данный документ определяет спецификацию системы типов языка программирования YaoXiang, включая базовые типы, составные типы, обобщения и trait.

---

## Глава 0: Теоретические основы

### 0.1 Соответствие Карри — Ховарда

Соответствие Карри — Ховарда (Curry-Howard correspondence) является теоретической основой системы типов YaoXiang. Оно раскрывает глубокую связь между системой типов языка программирования и математической логикой:

| Логика | Язык программирования |
|--------|-----------------------|
| Высказывание \(P\) | Тип `Type` |
| Доказательство \(p: P\) | Программа `x: T = ...` |
| Импликация \(P \rightarrow Q\) | Функциональный тип `(P) -> Q` |
| Конъюнкция \(P \wedge Q\) | Тип-произведение `{ a: P, b: Q }` |
| Дизъюнкция \(P \vee Q\) | Тип-сумма `{ a(P) \| b(Q) }` |
| Универсальная квантификация \(\forall x:T. P(x)\) | Обобщения `(T: Type) -> ...` |
| Истина \(\top\) | Пустой тип `{}` |
| Ложь \(\bot\) | `Void` / `Never` |
| Тип вселенной \(Type_n : Type_{n+1}\) | Иерархия вселенных (защита от парадокса Рассела) |
| Математическая индукция | Типовый `match` |

### 0.2 Тип как высказывание, программа как доказательство

В YaoXiang данное соответствие является основным принципом проектирования:

- **Тип — это логическое высказывание**. `Int` — это высказывание «целое число существует», `fn(a: Int, b: Int) -> Int` — высказывание «для двух целых чисел существует целое число».
- **Проверка типов — это верификация доказательства**. Когда программа проходит проверку типов, это эквивалентно конструктивному доказательству логического высказывания.
- **Завершающиеся вычисления на уровне типов соответствуют корректным индуктивным рассуждениям**. Типовые семейства YaoXiang (такие как `Add` для `Nat` через сопоставление с образцом) по сути являются кодированием математической индукции на уровне типов.

### 0.3 Влияние на проектирование языка

Соответствие Карри — Ховарда в YaoXiang проявляется следующим образом:

1. **Иерархия вселенных** (RFC-010): `Type₀ : Type₁ : Type₂ …` позволяет избежать логических парадоксов (парадокс Жирара), возникающих при `Type: Type`
2. **Типовые семейства** (RFC-011): паттерн-матчинг на уровне типов для натуральных чисел `Nat(Zero/Succ)` соответствует индуктивному доказательству в аксиоматике Пеано
3. **Условные типы** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` соответствует case-дизъюнкции в логике
4. **Типы, зависящие от значений** (RFC-011): `Vec: (n: Int) -> Type` соответствует конечной квантификации «для каждого целого n существует тип»

---

## Глава 1: Классификация типов

### 1.1 Типовые выражения

```
TypeExpr    ::= PrimitiveType
              | StructType
              | EnumType
              | InterfaceType
              | TupleType
              | FnType
              | GenericType
              | TypeRef
              | TypeUnion
              | TypeIntersection
              | ConstrainedType
              | AssociatedType
```

---

## Глава 2: Базовые типы

### 2.1 Примитивные типы

| Тип | Описание | Размер по умолчанию |
|-----|----------|---------------------|
| `Type` | Метатип | 0 байт |
| `Void` | Пустое значение | 0 байт |
| `Bool` | Логическое значение | 1 байт |
| `Int` | Знаковое целое | 8 байт |
| `Uint` | Беззнаковое целое | 8 байт |
| `Float` | Число с плавающей запятой | 8 байт |
| `String` | Строка UTF-8 | Переменный |
| `Char` | Символ Unicode | 4 байта |
| `Bytes` | Сырые байты | Переменный |

Целые с указанием разрядности: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`
Числа с плавающей запятой с указанием разрядности: `Float32`, `Float64`

---

## Глава 3: Составные типы

### 3.1 Типы записи

**Унифицированный синтаксис**：`Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 // ограничение интерфейса
```

```yaoxiang
// Простой тип записи
Point: Type = { x: Float, y: Float }

// Пустой тип записи
Empty: Type = {}

// Тип записи с обобщениями
Pair: (T: Type) -> Type = { first: T, second: T }

// Тип записи, реализующий интерфейсы
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**Правила**:
- Типы записи определяются с помощью фигурных скобок `{}`
- После имени поля непосредственно следует двоеточие и тип
- Имена интерфейсов внутри тела типа означают их реализацию

#### 3.1.1 Значения полей по умолчанию

Для полей типа можно указать значения по умолчанию, которые при конструировании можно опустить:

```yaoxiang
// Поля со значениями по умолчанию — при конструировании необязательны
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Использование
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Поля без значений по умолчанию — при конструировании обязательны
Point2: Type = {
    x: Float,
    y: Float
}

// Использование
Point2(x=1, y=2) // Правильно
Point2()          // Ошибка
```

**Правила**:
- `field: Type = expression` -> есть значение по умолчанию, при конструировании необязательно
- `field: Type` -> нет значения по умолчанию, при конструировании обязательно

#### 3.1.2 Встроенные привязки

Внутри определения типа можно напрямую привязывать методы:

```yaoxiang
// Способ 1: привязка внешней функции
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // привязка к позиции 0
}
// Вызов: p1.distance(p2) -> distance(p1, p2)

// Способ 2: анонимная функция + привязка по позиции
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        return (dx * dx + dy * dy).sqrt()
    })
}
// Синтаксис: ((params) => body)[position]
// Вызов: p1.distance(p2) -> distance(p1, p2)
```

### 3.2 Перечислимые типы (типы вариантов)

```
EnumType    ::= '{' Variant ('|' Variant)* '}'
Variant     ::= Identifier (':' TypeExpr)?
```

**Синтаксис**：`Name: Type = { Variant1 | Variant2(params) | ... }`

```yaoxiang
// Варианты без параметров
Color: Type = { red | green | blue }

// Варианты с параметрами
Option: (T: Type) -> Type = { some(T) | none }

// Смешанный вариант
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Вариант без параметров эквивалентен конструктору без параметров
Bool: Type = { true | false }
```

### 3.3 Типы интерфейсов

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**Синтаксис**：Интерфейс — это тип записи, все поля которого являются функциональными типами

```yaoxiang
// Определение интерфейса
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// Пустой интерфейс
EmptyInterface: Type = {}
```

**Реализация интерфейса**：Тип реализует интерфейс, указывая имена интерфейсов в конце определения

```yaoxiang
// Тип, реализующий интерфейсы
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // реализует интерфейс Drawable
    Serializable     // реализует интерфейс Serializable
}
```

**Прямое присваивание интерфейсу**：Конкретный тип можно напрямую присвоить переменной типа интерфейса (структурный подтип)

```yaoxiang
// Прямое присваивание (конкретный тип определяется на этапе компиляции -> вызов без накладных расходов)
d: Drawable = Circle(1)
d.draw(screen)        // после компиляции: прямой вызов circle_draw, без vtable

// Возвращаемое значение функции (конкретный тип неизвестен на этапе компиляции -> вызов через vtable)
d: Drawable = get_shape()
d.draw(screen)        // поиск метода через vtable

// Интерфейс как параметр функции
process: (d: Drawable) -> Void = d.draw(screen)
```

**Стратегия оптимизации на этапе компиляции**:

| Ситуация | Результат вывода | Способ вызова |
|----------|------------------|---------------|
| Прямое присваивание конкретного типа | Конкретный тип определён | Прямой вызов (без накладных расходов) |
| Возвращаемое значение функции | Неизвестен | vtable |
| Гетерогенная коллекция | Несколько типов | vtable |

### 3.4 Кортежные типы

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.5 Функциональные типы

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= TypeExpr (',' TypeExpr)*
```

---

## Глава 4: Обобщения

### 4.1 Синтаксис обобщённых параметров

```
GenericType     ::= Identifier '[' TypeArgList ']'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
GenericParams   ::= '[' Identifier (',' Identifier)* ']'
                 |  '[' Identifier ':' TypeBound (',' Identifier ':' TypeBound)* ']'
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

### 4.2 Определение обобщённых типов

```yaoxiang
// Базовый обобщённый тип
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self
}

List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    get: (self: List(T), index: Int) -> Option(T)
}
```

### 4.3 Вывод типов

```yaoxiang
// Компилятор автоматически выводит обобщённые параметры
numbers: List(Int) = List(1, 2, 3)  // компилятор выводит List(Int)
```

---

## Глава 5: Ограничения типов

### 5.1 Одиночное ограничение

```
ConstrainedType ::= '[' Identifier ':' TypeBound ']' TypeExpr
```

```yaoxiang
// Определение интерфейсного типа (как ограничения)
Clone: Type = {
    clone: (Self) -> Self
}

// Использование ограничения
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 Множественные ограничения

```yaoxiang
// Синтаксис множественных ограничений
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// Сортировка обобщённого контейнера
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

### 5.3 Ограничения функционального типа

```yaoxiang
// Ограничения для функций высшего порядка
call_twice: (T: Type, F: Fn() -> T)(f: F) -> (T, T) = (f(), f())

compose: (A: Type, B: Type, C: Type, F: Fn(A) -> B, G: Fn(B) -> C)(a: A, f: F, g: G) -> C = g(f(a))
```

---

## Глава 6: Ассоциированные типы

### 6.1 Определение ассоциированных типов

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait (использование синтаксиса типов записи)
Iterator: (T: Type) -> Type = {
    Item: T,                    // ассоциированный тип
    next: (Self) -> Option(T),
    has_next: (Self) -> Bool
}

// Использование ассоциированного типа
collect: (T: Type, I: Iterator(T))(iter: I) -> List(T) = {
    result = List(T)()
    while iter.has_next() {
        if let Some(item) = iter.next() {
            result.push(item)
        }
    }
    return result
}
```

### 6.2 Обобщённые ассоциированные типы (GAT)

```yaoxiang
// Более сложные ассоциированные типы
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // ассоциированный тип тоже обобщённый
    iter: (Self) -> IteratorType
}
```

---

## Глава 7: Обобщения на этапе компиляции

### 7.1 Ограничения литеральных типов

```
LiteralType   ::= Identifier ':' Int          // константа времени компиляции
CompileTimeFn ::= '[' Identifier ':' Int ']' '(' Identifier ')' '->' TypeExpr
```

**Основной принцип проектирования**: использование обобщённого параметра `(n: Int)` + значение параметра `(n: n)` для различения констант времени компиляции и значений времени выполнения.

```yaoxiang
// Факториал времени компиляции: параметр должен быть литералом, известным на этапе компиляции
factorial: (n: Int)(n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// Массив констант времени компиляции
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // массив с размером, известным на этапе компиляции
    length: N
}

// Использование
arr: StaticArray(Int, factorial(5))  // компилятор вычисляет factorial(5) = 120 во время компиляции
```

### 7.2 Массивы констант времени компиляции

```yaoxiang
// Использование для типов матриц
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// Проверка размерностей времени компиляции
identity_matrix: (T: Add + Zero + One, N: Int)(size: N) -> Matrix(T, N, N) = {
    // ...
}
```

---

## Глава 8: Условные типы

### 8.1 Условный тип If

```
IfType        ::= 'If' '[' BoolExpr ',' TypeExpr ',' TypeExpr ']'
```

```yaoxiang
// Типовый If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E
}

// Пример: ветвление времени компиляции
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

// Проверка времени компиляции
Assert: (C: Bool) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed")
}
```

### 8.2 Типовые семейства

```yaoxiang
// Преобразование типов времени компиляции
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

---

## Глава 9: Объединение и пересечение типов

### 9.1 Объединение типов

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 Пересечение типов

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**Синтаксис**: пересечение типов `A & B` представляет тип, одновременно удовлетворяющий A и B

```yaoxiang
// Комбинация интерфейсов = пересечение типов
DrawableSerializable: Type = Drawable & Serializable

// Использование типа пересечения
process: (T: Drawable & Serializable)(item: T, screen: Surface) -> String = {
    item.draw(screen)
    return item.serialize()
}
```

---

## Глава 10: Перегрузка и специализация функций

### 10.1 Перегрузка функций

```yaoxiang
// Базовая специализация: использование перегрузки функций (компилятор выбирает автоматически)
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

// Универсальная реализация
sum: (T: Add)(arr: Array(T)) -> T = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

### 10.2 Специализация для платформы

```yaoxiang
// Перечисление типов платформ (определено в стандартной библиотеке)
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// P — предопределённое имя обобщённого параметра, представляющее текущую компилируемую платформу
sum: (P: X86_64)(arr: Array(Float)) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: (P: AArch64)(arr: Array(Float)) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

---

## Приложение: Краткий справочник определений типов

### A.1 Определения типов

```yaoxiang
// === Типы записи (фигурные скобки) ===

// Структура
Point: Type = { x: Float, y: Float }

// Перечисление (типы вариантов)
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }
Status: Type = { pending | processing | completed }

// === Типы интерфейсов (фигурные скобки, все поля — функции) ===

// Определение интерфейса
Serializable: Type = { serialize: () -> String }

// Тип, реализующий интерфейс
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // реализует интерфейс Serializable
}

// === Функциональные типы ===

Adder: Type = (Int, Int) -> Int
```

### A.2 Синтаксис обобщений

```yaoxiang
// Обобщённые типы
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Обобщённые функции
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = { ... }

// Ограничения типов
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// Ассоциированные типы
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// Обобщения времени компиляции
factorial: (n: Int)(n: n) -> Int = { ... }
StaticArray: (T: Type, N: Int) -> Type = { data: T(N), length: N }

// Условные типы
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// Специализация функций
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```