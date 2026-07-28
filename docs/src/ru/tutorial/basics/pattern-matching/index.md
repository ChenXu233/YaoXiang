---
title: Сопоставление с образцом
---

# Сопоставление с образцом

В [основах match](../control-flow/match.md) вы изучили базовое использование `match` — литералы, идентификаторы, подстановочный знак. Теперь углубимся в полные возможности сопоставления с образцом в YaoXiang.

## Полные типы образцов

Согласно грамматической спецификации, полное определение `Pattern`:

```
Pattern     ::= Literal       # Литеральный образец: 42, "hello"
            | Identifier      # Идентификаторный образец: захват значения
            | Wildcard        # Подстановочный знак: _
            | StructPattern   # Структурный образец: деконструкция записи
            | TuplePattern    # Кортежный образец: деконструкция кортежа
            | EnumPattern     # Перечислимый образец: деконструкция варианта
            | OrPattern       # Или-образец: pattern1 | pattern2
```

Вы уже изучили первые три базовых образца в предыдущей главе. Эта глава фокусируется на последних четырёх продвинутых образцах.

## Перечислимые образцы

Перечислимые образцы — наиболее часто используемая продвинутая возможность `match`. Они позволяют деконструировать варианты перечисления и извлекать внутренние данные.

### Базовое сопоставление с перечислениями

```yaoxiang
// Определение типа Result
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Функция использует match для обработки Result
handle: (result: Result(Int, String)) -> String = match result {
    ok(value) => "Успех! Полученное значение: {value}",
    err(msg) => "Ошибка: {msg}",
}

a = ok(42)
b = err("Таймаут соединения")

print(handle(a))  // Успех! Полученное значение: 42
print(handle(b))  // Ошибка: Таймаут соединения
```

### Тип Option

```yaoxiang
// Использование Option для избежания null
// Встроенный тип: Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "Есть значение: {n}",
    none => "Ничего нет",
}

print(describe(some(100)))  // Есть значение: 100
print(describe(none))       // Ничего нет
```

### Пользовательские перечисления

```yaoxiang
// Определение перечисления Color
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color, rgb: (Int, Int, Int) -> Color }

to_hex: (c: Color) -> String = match c {
    red => "#FF0000",
    green => "#00FF00",
    blue => "#0000FF",
    rgb(r, g, b) => "#{r.to_hex()}{g.to_hex()}{b.to_hex()}",
}

print(to_hex(red))                // #FF0000
print(to_hex(rgb(128, 128, 128))) // #808080
```

В `rgb(r, g, b)` переменные `r`, `g`, `b` являются идентификаторными образцами — они захватывают три значения внутри варианта `rgb`.

## Структурные образцы (деконструкция записей)

Структурные образцы позволяют напрямую извлекать интересующие поля из структуры:

```yaoxiang
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// Структурный образец для деконструкции
area: (shape: Rect) -> Float = match shape {
    { x: _, y: _, width: w, height: h } => w * h,
}

r = Rect(0.0, 0.0, 10.0, 20.0)
print(area(r))  // 200.0
```

`{ width: w, height: h }` означает «извлечь поле `width` из записи и привязать к переменной `w`, извлечь поле `height` и привязать к переменной `h`». `x: _` и `y: _` означают «эти поля существуют, но значение не важно».

**Сокращённая запись**: когда имя поля и имя переменной совпадают, можно сократить — компилятор автоматически деконструирует в одноимённую переменную:

```yaoxiang
describe_point: (p: Point) -> String = match p {
    { x: 0.0, y: 0.0 } => "Начало координат",
    { x, y } => "Координаты ({x}, {y})",
}

print(describe_point(Point(0.0, 0.0)))  // Начало координат
print(describe_point(Point(3.0, 4.0)))  // Координаты (3.0, 4.0)
```

## Кортежные образцы

Кортежные образцы деконструируют элементы кортежа:

```yaoxiang
Pair: Type = (Int, String)

first: (p: Pair) -> Int = match p {
    (n, _) => n,
}

second: (p: Pair) -> String = match p {
    (_, s) => s,
}

p = (42, "hello")
print(first(p))   // 42
print(second(p))  // "hello"
```

## Или-образец

С помощью `|` можно комбинировать несколько образцов и сопоставить с любым из них:

```yaoxiang
Token: Type = { number: (Int) -> Token, plus: () -> Token, minus: () -> Token, times: () -> Token, divide: () -> Token, eof: () -> Token }

// Объединение нескольких вариантов в "оператор"
is_operator: (t: Token) -> Bool = match t {
    plus | minus | times | divide => true,
    _ => false,
}

print(is_operator(plus))      // true
print(is_operator(number(5))) // false
```

## Охранные выражения (if-охранники)

Добавление `if условие` после ветки match позволяет, чтобы сопоставление срабатывало только когда образец совпал **и** условие выполнено:

```yaoxiang
Age: Type = { adult: (Int) -> Age, child: (Int) -> Age }

// Охранное выражение добавляет дополнительное условие
can_drive: (a: Age) -> Bool = match a {
    adult(n) if n >= 18 => true,
    adult(n) if n < 18 => false,
    child(_) => false,
}

print(can_drive(adult(20)))  // true
print(can_drive(adult(16)))  // false
```

Переменные в охранном выражении берутся из предшествующего образца — `adult(n) if n >= 18` сначала захватывает значение через `n`, затем проверяет `n >= 18`.

## Проверка исчерпывачности

Компилятор YaoXiang гарантирует, что `match` покрывает все возможные случаи. Если ветка пропущена, компилятор выдаст ошибку:

```yaoxiang
Direction: Type = { north: () -> Direction, south: () -> Direction, east: () -> Direction, west: () -> Direction }

// ✅ Правильно: все четыре направления покрыты
turn: (d: Direction) -> Direction = match d {
    north => east,
    east => south,
    south => west,
    west => north,
}

// ❌ Ошибка компиляции: отсутствует west
// broken: (d: Direction) -> Direction = match d {
//     north => east,
//     east => south,
//     south => west,
//     // west не обработан → ошибка компиляции
// }
```

Это важный механизм YaoXiang для предотвращения неожиданностей во время выполнения — как только добавляется новый вариант, компилятор напоминает обновить все места с `match`.

## Вложенные образцы

Настоящая мощь образцов раскрывается в **вложенности** — можно вложить один образец внутрь другого:

```yaoxiang
Expr: Type = { literal: (Int) -> Expr, add: (Expr, Expr) -> Expr, mul: (Expr, Expr) -> Expr }

// Вложенные образцы: внутри add ещё сопоставление с literal
simplify: (e: Expr) -> Expr = match e {
    add(literal(0), right) => right,  // 0 + x = x
    add(left, literal(0)) => left,    // x + 0 = x
    mul(literal(1), right) => right,  // 1 * x = x
    mul(left, literal(1)) => left,    // x * 1 = x
    other => other,
}

e = add(literal(0), literal(5))
print(simplify(e))  // literal(5)
```

В `add(literal(0), right)` внешний слой — перечислимый образец `add`, внутренний — литеральный образец `literal(0)` — два уровня вложенности, одно сопоставление.

## Итог

| Тип образца     | Синтаксис            | Назначение              |
| --------------- | -------------------- | ----------------------- |
| Литерал         | `42`, `"hi"`         | Точное сопоставление    |
| Идентификатор   | `x`                  | Захват сопоставленного  |
| Подстановочный  | `_`                  | Захват остатка          |
| Перечисление    | `ok(value)`          | Деконструкция варианта  |
| Структурный     | `{ x, y }`           | Деконструкция полей     |
| Кортежный       | `(a, b)`             | Деконструкция элементов |
| Или-образец     | `a \| b \| c`        | Сопоставление любого    |
| Охранное выр.   | `pattern if cond`    | Дополнительное условие  |

`match` + сопоставление с образцом = самый мощный инструмент управления потоком выполнения в YaoXiang. Освойте его, и вы напишете более безопасный и понятный код.
