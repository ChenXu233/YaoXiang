---
title: 'Сопоставление с образцом'
---

# Сопоставление с образцом

В [основах match](../control-flow/match.md) вы изучили базовое использование `match` — литералы,
идентификаторы, подстановочные знаки. Теперь мы углубимся в полные возможности сопоставления с
образцом в YaoXiang.

## Полный список типов образцов

Согласно спецификации синтаксиса, полное определение `Pattern`:

```
Pattern     ::= Literal       # 字面量模式：42, "hello"
            | Identifier      # 标识符模式：捕获值
            | Wildcard        # 通配符：_
            | StructPattern   # 结构体模式：解构记录
            | TuplePattern    # 元组模式：解构元组
            | EnumPattern     # 枚举模式：解构变体
            | OrPattern       # 或模式：pattern1 | pattern2
```

Вы уже изучили первые три базовых образца в предыдущей главе. Эта глава посвящена четырём оставшимся
продвинутым образцам.

## Образец перечисления

Образец перечисления — наиболее часто используемая продвинутая возможность `match`. Он позволяет
деконструировать варианты перечисления и извлекать внутренние данные.

### Базовое сопоставление перечисления

```yaoxiang
// 定义 Result 类型
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 函数使用 match 处理 Result
handle: (result: Result(Int, String)) -> String = match result {
    ok(value) => "成功！得到的值是: {value}",
    err(msg) => "出错啦: {msg}",
}

a = ok(42)
b = err("连接超时")

print(handle(a))  // 成功！得到的值是: 42
print(handle(b))  // 出错啦: 连接超时
```

### Тип Option

```yaoxiang
// 使用 Option 避免 null
// 内置类型: Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "有值: {n}",
    none => "什么也没有",
}

print(describe(some(100)))  // 有值: 100
print(describe(none))       // 什么也没有
```

### Пользовательское перечисление

```yaoxiang
// 定义颜色枚举
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

`r`, `g`, `b` в `rgb(r, g, b)` — это образцы идентификаторов — они захватывают три значения внутри
варианта `rgb`.

## Образец структуры (деконструкция записи)

Образец структуры позволяет напрямую извлекать нужные поля из структуры:

```yaoxiang
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// 结构体模式解构
area: (shape: Rect) -> Float = match shape {
    { x: _, y: _, width: w, height: h } => w * h,
}

r = Rect(0.0, 0.0, 10.0, 20.0)
print(area(r))  // 200.0
```

`{ width: w, height: h }` означает «извлечь поле `width` из записи и привязать к переменной `w`,
извлечь поле `height` и привязать к переменной `h`». `x: _` и `y: _` означают «эти поля существуют,
но их значения не важны».

**Сокращённая запись**: когда имя поля и имя переменной совпадают, можно использовать сокращение —
компилятор автоматически деконструирует в одноимённую переменную:

```yaoxiang
describe_point: (p: Point) -> String = match p {
    { x: 0.0, y: 0.0 } => "原点",
    { x, y } => "坐标 ({x}, {y})",
}

print(describe_point(Point(0.0, 0.0)))  // 原点
print(describe_point(Point(3.0, 4.0)))  // 坐标 (3.0, 4.0)
```

## Образец кортежа

Образец кортежа деконструирует отдельные элементы кортежа:

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

## Образец «или»

Используйте `|` для объединения нескольких образцов, чтобы соответствовать любому из них:

```yaoxiang
Token: Type = { number: (Int) -> Token, plus: () -> Token, minus: () -> Token, times: () -> Token, divide: () -> Token, eof: () -> Token }

// 将多个变体组合为"运算符"类
is_operator: (t: Token) -> Bool = match t {
    plus | minus | times | divide => true,
    _ => false,
}

print(is_operator(plus))      // true
print(is_operator(number(5))) // false
```

## Охранные выражения (if-страж)

Добавьте `if условие` после ветви сопоставления, чтобы ветвь срабатывала только тогда, когда образец
совпадает **и** условие выполняется:

```yaoxiang
Age: Type = { adult: (Int) -> Age, child: (Int) -> Age }

// 卫表达式附加额外条件
can_drive: (a: Age) -> Bool = match a {
    adult(n) if n >= 18 => true,
    adult(n) if n < 18 => false,
    child(_) => false,
}

print(can_drive(adult(20)))  // true
print(can_drive(adult(16)))  // false
```

Переменные в охранном выражении берутся из предыдущего образца — `adult(n) if n >= 18` сначала
захватывает значение в `n`, а затем проверяет `n >= 18`.

## Проверка полноты (exhaustiveness)

Компилятор YaoXiang гарантирует, что `match` охватывает все возможные случаи. Если ветвь пропущена,
компилятор выдаст ошибку:

```yaoxiang
Direction: Type = { north: () -> Direction, south: () -> Direction, east: () -> Direction, west: () -> Direction }

// ✅ 正确：四个方向全部覆盖
turn: (d: Direction) -> Direction = match d {
    north => east,
    east => south,
    south => west,
    west => north,
}

// ❌ 编译错误：缺少 west
// broken: (d: Direction) -> Direction = match d {
//     north => east,
//     east => south,
//     south => west,
//     // west 未处理 → 编译错误
// }
```

Это важный механизм YaoXiang для предотвращения неожиданностей во время выполнения — как только
добавляется новый вариант, компилятор напомнит вам обновить все места с `match`.

## Вложенные образцы

Истинная сила образцов раскрывается благодаря **вложенности** — вы можете вкладывать один образец в
другой:

```yaoxiang
Expr: Type = { literal: (Int) -> Expr, add: (Expr, Expr) -> Expr, mul: (Expr, Expr) -> Expr }

// 嵌套模式：在 add 内部再匹配 literal
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

В `add(literal(0), right)` внешний слой — это образец перечисления `add`, внутренний — литеральный
образец `literal(0)` — два уровня вложенности за одно сопоставление.

## Итоги

| Тип образца         | Синтаксис         | Назначение                          |
| ------------------- | ----------------- | ----------------------------------- |
| Литерал             | `42`, `"hi"`      | Точное совпадение значения          |
| Идентификатор       | `x`               | Захват совпавшего значения          |
| Подстановочный знак | `_`               | Универсальное совпадение            |
| Перечисление        | `ok(value)`       | Деконструкция варианта перечисления |
| Структура           | `{ x, y }`        | Деконструкция полей записи          |
| Кортеж              | `(a, b)`          | Деконструкция элементов кортежа     |
| Или                 | `a \| b \| c`     | Совпадение с одним из вариантов     |
| Охранное выражение  | `pattern if cond` | Дополнительная проверка условия     |

`match` + сопоставление с образцом = самый мощный инструмент управления потоком выполнения в
YaoXiang. Освоив его, вы будете писать более безопасный и понятный код.
