# Спецификация системы типов

Настоящий документ определяет спецификацию системы типов языка программирования YaoXiang, включая базовые типы, составные типы, generics и trait.

---

## Глава 0: Теоретические основы

### 0.1 Соответствие Карри — Ховарда

Соответствие Карри — Ховарда (Curry-Howard correspondence) является теоретической основой системы типов YaoXiang. Оно раскрывает глубокую связь между системой типов языка программирования и математической логикой:

| Логика | Язык программирования |
|--------|------------------------|
| Высказывание \(P\) | Тип `Type` |
| Доказательство \(p: P\) | Программа `x: T = ...` |
| Импликация \(P \rightarrow Q\) | Функциональный тип `(P) -> Q` |
| Конъюнкция \(P \wedge Q\) | Тип-произведение `{ a: P, b: Q }` |
| Дизъюнкция \(P \vee Q\) | Тип-сумма `{ a(P) | b(Q) }` |
| Универсальная квантификация \(\forall x:T. P(x)\) | Generics `(T: Type) -> ...` |
| Истина \(\top\) | Пустой тип `{}` |
| Ложь \(\bot\) | `Void` / `Never` |
| Типовой универсум \(Type_n : Type_{n+1}\) | Иерархия вселенных (защита от парадокса Рассела) |
| Математическая индукция | Типовой `match` |

### 0.2 Типы как высказывания, программы как доказательства

В YaoXiang данное соответствие является основным принципом проектирования:

- **Тип — это логическое высказывание**. `Int` — высказывание "существует целое число", `fn(a: Int, b: Int) -> Int` — высказывание "при данных двух целых числах существует целое число".
- **Проверка типов — это верификация доказательства**. Когда программа проходит проверку типов, это равносильно конструктивному доказательству логического высказывания.
- **Завершающиеся типовые вычисления соответствуют корректной индуктивной дедукции**. Типовые семейства YaoXiang (такие как pattern matching `Add` на `Nat`) по сути являются кодированием математической индукции на уровне типов.

### 0.3 Влияние на дизайн языка

Соответствие Карри — Ховарда в YaoXiang проявляется следующим образом:

1. **Иерархия вселенных** (RFC-010): `Type₀ : Type₁ : Type₂ …` позволяет избежать логического парадокса, вызванного `Type: Type` (парадокс Жирара)
2. **Типовые семейства** (RFC-011): Паттерн-матчинг натуральных чисел `Nat(Zero/Succ)` соответствует индуктивному доказательству в аксиоматике Пеано
3. **Условные типы** (RFC-011): `If: (C: Bool, T: Type, E: Type) -> Type` соответствует case-дизъюнкции в логике
4. **Зависимые от значений типы** (RFC-011): `Vec: (n: Int) -> Type` соответствует конечной квантификации "для каждого целого n существует тип"

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
|------|----------|---------------------|
| `Type` | Метатип | 0 байт |
| `Void` | Пустое значение | 0 байт |
| `Bool` | Булево значение | 1 байт |
| `Int` | Знаковое целое | 8 байт |
| `Uint` | Беззнаковое целое | 8 байт |
| `Float` | Число с плавающей точкой | 8 байт |
| `String` | Строка UTF-8 | Переменный |
| `Char` | Символ Unicode | 4 байта |
| `Bytes` | Сырые байты | Переменный |

Целые с битовой шириной: `Int8`, `Int16`, `Int32`, `Int64`, `Int128`
Числа с плавающей точкой с битовой шириной: `Float32`, `Float64`

---

## Глава 3: Составные типы

### 3.1 Record type

**Унифицированный синтаксис**: `Name: Type = { field1: Type1, field2: Type2, ... }`

```
RecordType  ::= '{' FieldList? '}'
FieldList   ::= Field (',' Field)* ','?
Field       ::= Identifier ':' TypeExpr
            |  Identifier                 // Ограничение интерфейса
```

```yaoxiang
// Простой record type
Point: Type = { x: Float, y: Float }

// Пустой record type
Empty: Type = {}

// Record type с generics
Pair: (T: Type) -> Type = { first: T, second: T }

// Record type, реализующий интерфейсы
Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable
}
```

**Правила**:
- Record type определяется фигурными скобками `{}`
- После имени поля непосредственно следует двоеточие и тип
- Имена интерфейсов внутри тела типа означают реализацию этих интерфейсов

#### 3.1.1 Значения полей по умолчанию

Поля типа могут иметь значения по умолчанию, которые можно не указывать при конструировании:

```yaoxiang
// Поле со значением по умолчанию - необязательно при конструировании
Point: Type = {
    x: Float = 0,
    y: Float = 0
}

// Использование
Point()           // -> Point(x=0, y=0)
Point(x=1)       // -> Point(x=1, y=0)
Point(x=1, y=2) // -> Point(x=1, y=2)

// Поле без значения по умолчанию - обязательно при конструировании
Point2: Type = {
    x: Float,
    y: Float
}

// Использование
Point2(x=1, y=2) // Правильно
Point2()          // Ошибка
```

**Правила**:
- `field: Type = expression` -> есть значение по умолчанию, необязательно при конструировании
- `field: Type` -> нет значения по умолчанию, обязательно при конструировании

#### 3.1.2 Builtin binding

Внутри определения типа можно напрямую связывать методы:

```yaoxiang
// Способ 1: Связывание внешней функции
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]    // Связывание с позицией 0
}
// Вызов: p1.distance(p2) -> distance(p1, p2)

// Способ 2: Анонимная функция + позиционное связывание
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

### 3.2 Enum type (variant type)

```
EnumType    ::= '{' Variant ('|' Variant)* '}'
Variant     ::= Identifier (':' TypeExpr)?
```

**Синтаксис**: `Name: Type = { Variant1 | Variant2(params) | ... }`

```yaoxiang
// Варианты без параметров
Color: Type = { red | green | blue }

// Варианты с параметрами
Option: (T: Type) -> Type = { some(T) | none }

// Смешанный
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Вариант без параметров эквивалентен конструктору без параметров
Bool: Type = { true | false }
```

### 3.3 Interface type

```
InterfaceType ::= '{' FnField (',' FnField)* ','?
FnField       ::= Identifier ':' FnType
FnType        ::= '(' ParamTypes? ')' '->' TypeExpr
```

**Синтаксис**: Интерфейс — это record type, все поля которого являются функциональными типами

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

**Реализация интерфейса**: Тип реализует интерфейс, перечисляя имена интерфейсов в конце определения

```yaoxiang
// Тип, реализующий интерфейсы
Point: Type = {
    x: Float,
    y: Float,
    Drawable,        // Реализует интерфейс Drawable
    Serializable     // Реализует интерфейс Serializable
}
```

**Прямое присваивание интерфейсу**: Конкретный тип может напрямую присваиваться переменной интерфейсного типа (структурный subtype)

```yaoxiang
// Прямое присваивание (конкретный тип определяется во время компиляции -> вызов без накладных расходов)
d: Drawable = Circle(1)
d.draw(screen)        // После компиляции: прямой вызов circle_draw, без vtable

// Возврат из функции (конкретный тип невозможно определить во время компиляции -> вызов через vtable)
d: Drawable = get_shape()
d.draw(screen)        // Поиск метода через vtable

// Интерфейс как параметр функции
process: (d: Drawable) -> Void = d.draw(screen)
```

**Стратегия оптимизации компиляции**:

| Сценарий | Результат вывода | Способ вызова |
|----------|------------------|---------------|
| Прямое присваивание конкретного типа | Конкретный тип определён | Прямой вызов (ноль накладных расходов) |
| Возврат из функции | Неизвестен | vtable |
| Гетерогенная коллекция | Несколько типов | vtable |

### 3.4 Tuple type

```
TupleType   ::= '(' TypeList? ')'
TypeList    ::= TypeExpr (',' TypeExpr)* ','?
```

### 3.5 Function type

```
FnType      ::= '(' ParamList? ')' '->' TypeExpr
ParamList   ::= TypeExpr (',' TypeExpr)*
```

---

## Глава 4: Generics

### 4.1 Синтаксис параметров generics

```
GenericType     ::= Identifier '[' TypeArgList ']'
TypeArgList     ::= TypeExpr (',' TypeExpr)* ','?
GenericParams   ::= '[' Identifier (',' Identifier)* ']'
                 |  '[' Identifier ':' TypeBound (',' Identifier ':' TypeBound)* ']'
TypeBound       ::= Identifier
                 |  Identifier '+' Identifier ('+' Identifier)*
```

### 4.2 Определение generic types

```yaoxiang
// Базовый generic type
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

### 4.3 Type inference

```yaoxiang
// Компилятор автоматически выводит generic параметры
numbers: List(Int) = List(1, 2, 3)  // Компилятор выводит List(Int)
```

---

## Глава 5: Type constraint

### 5.1 Single constraint

```
ConstrainedType ::= '[' Identifier ':' TypeBound ']' TypeExpr
```

```yaoxiang
// Определение интерфейсного типа (как constraint)
Clone: Type = {
    clone: (Self) -> Self
}

// Использование constraint
clone: (T: Clone)(value: T) -> T = value.clone()
```

### 5.2 Multiple constraint

```yaoxiang
// Синтаксис множественных constraints
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

// Сортировка generic контейнера
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    result = list.clone()
    quicksort(&mut result)
    return result
}
```

### 5.3 Constraints на функциональные типы

```yaoxiang
// Constraints для функций высшего порядка
call_twice: (T: Type, F: Fn() -> T)(f: F) -> (T, T) = (f(), f())

compose: (A: Type, B: Type, C: Type, F: Fn(A) -> B, G: Fn(B) -> C)(a: A, f: F, g: G) -> C = g(f(a))
```

---

## Глава 6: Associated type

### 6.1 Определение associated type

```
AssociatedType ::= Identifier ':' TypeExpr
```

```yaoxiang
// Iterator trait (с использованием синтаксиса record type)
Iterator: (T: Type) -> Type = {
    Item: T,                    // Associated type
    next: (Self) -> Option(T),
    has_next: (Self) -> Bool
}

// Использование associated type
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

### 6.2 Generic associated type (GAT)

```yaoxiang
// Более сложный associated type
Container: (T: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(T),  // Associated type также generic
    iter: (Self) -> IteratorType
}
```

---

## Глава 7: Compile-time generics

### 7.1 Constraints на literal types

```
LiteralType   ::= Identifier ':' Int          // Compile-time константа
CompileTimeFn ::= '[' Identifier ':' Int ']' '(' Identifier ')' '->' TypeExpr
```

**Основной дизайн**: использование generic параметра `(n: Int)` + значения параметра `(n: n)` для различения compile-time констант и runtime значений.

```yaoxiang
// Compile-time факториал: параметр должен быть literal, известным во время компиляции
factorial: (n: Int)(n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

// Массив compile-time констант
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),      // Массив с размером, известным во время компиляции
    length: N
}

// Использование
arr: StaticArray(Int, factorial(5))  // Компилятор вычисляет factorial(5) = 120 во время компиляции
```

### 7.2 Массивы compile-time констант

```yaoxiang
// Использование для типов матриц
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows)
}

// Compile-time проверка размерностей
identity_matrix: (T: Add + Zero + One, N: Int)(size: N) -> Matrix(T, N, N) = {
    // ...
}
```

---

## Глава 8: Conditional type

### 8.1 If conditional type

```
IfType        ::= 'If' '[' BoolExpr ',' TypeExpr ',' TypeExpr ']'
```

```yaoxiang
// Типовой If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E
}

// Пример: compile-time ветвление
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

// Compile-time валидация
Assert: (C: Bool) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed")
}
```

### 8.2 Type family

```yaoxiang
// Compile-time преобразование типов
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String
}
```

---

## Глава 9: Type union и type intersection

### 9.1 Type union

```
TypeUnion     ::= TypeExpr '|' TypeExpr
```

### 9.2 Type intersection

```
TypeIntersection ::= TypeExpr '&' TypeExpr
```

**Синтаксис**: Type intersection `A & B` представляет тип, удовлетворяющий одновременно A и B

```yaoxiang
// Композиция интерфейсов = type intersection
DrawableSerializable: Type = Drawable & Serializable

// Использование type intersection
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

### 10.2 Специализация по платформе

```yaoxiang
// Enum типов платформ (определён в стандартной библиотеке)
Platform: Type = X86_64 | AArch64 | RISC_V | ARM | X86

// P — предопределённое имя generic параметра, представляющее текущую компилируемую платформу
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

```
// === Record type (фигурные скобки) ===

// Структура
Point: Type = { x: Float, y: Float }

// Enum (variant type)
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }
Status: Type = { pending | processing | completed }

// === Interface type (фигурные скобки, все поля — функции) ===

// Определение интерфейса
Serializable: Type = { serialize: () -> String }

// Тип, реализующий интерфейс
Point: Type = {
    x: Float,
    y: Float,
    Serializable    // Реализует интерфейс Serializable
}

// === Function type ===

Adder: Type = (Int, Int) -> Int
```

### A.2 Синтаксис generics

```
// Generic типы
List: (T: Type) -> Type = { data: Array(T), length: Int }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Generic функции
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = { ... }

// Type constraint
clone: (T: Clone)(value: T) -> T = value.clone()
combine: (T: Clone + Add)(a: T, b: T) -> T = body

// Associated type
Iterator: (T: Type) -> Type = { Item: T, next: () -> Option(T) }

// Compile-time generics
factorial: (n: Int)(n: n) -> Int = { ... }
StaticArray: (T: Type, N: Int) -> Type = { data: T(N), length: N }

// Conditional type
If: (C: Bool, T: Type, E: Type) -> Type = match C { True => T, False => E }

// Специализация функций
sum: (arr: Array(Int)) -> Int = { ... }
sum: (arr: Array(Float)) -> Float = { ... }
```