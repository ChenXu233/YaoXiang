# YaoXiang — Быстрый старт

> Это руководство поможет вам быстро освоить язык программирования YaoXiang.
>
> **Примечание**: Примеры кода в данном документе основаны на спецификации языка YaoXiang. Если при реальном запуске вы столкнётесь с синтаксическими различиями, обратитесь к [спецификации языка](../design/language-spec.md).

## Установка

### Сборка из исходного кода (рекомендуется)

```bash
# Клонирование репозитория
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang

# Сборка (отладочная версия, для разработки и тестирования)
cargo build

# Сборка (релизная версия, рекомендуется для продакшена)
cargo build --release

# Запуск тестов
cargo test

# Просмотр версии
./target/debug/yaoxiang --version
# или
./target/release/yaoxiang --version
```

**Проверка успешной установки**:
```bash
./target/debug/yaoxiang --version
# Должно вывести что-то вроде: yaoxiang x.y.z
```

## Первая программа

Создайте файл `hello.yx`:

```yaoxiang
// hello.yx
use std.io

// 函数定义: name: (param: Type, ...) -> return_type = { return ... }  # 代码块必须显式 return
// 表达式形式: name: (param: Type, ...) -> return_type = expr           # 表达式直接返回值
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

Запуск:

```bash
./target/debug/yaoxiang hello.yx
# или с релизной версией
./target/release/yaoxiang hello.yx
```

Вывод:

```
Hello, YaoXiang!
```

## Основные концепции

### Переменные и типы

```yaoxiang
// 自动类型推断
x = 42  // 推断为 Int
name = "YaoXiang"  // 推断为 String
pi = 3.14159  // 推断为 Float
is_valid = true  // 推断为 Bool

// 显式类型注解（推荐使用类型集中约定）
count: Int = 100

// 默认不可变（安全特性）
x = 10
x = 20  // ❌ 编译错误！不可变

// 可变变量（需要显式声明）
mut counter = 0
counter = counter + 1  // ✅ OK
```

### Функции

```yaoxiang
// 函数定义语法
// 表达式形式：直接返回值，不需要 return
add: (a: Int, b: Int) -> Int = a + b

// 代码块形式：必须使用 return 返回值
// add: (a: Int, b: Int) -> Int = { return a + b }

// 调用
result = add(1, 2)  // result = 3

// 单参数函数（表达式形式）
inc: (x: Int) -> Int = x + 1
```

### Определения типов

YaoXiang использует унифицированный синтаксис `name: type = value`:

```yaoxiang
// 变量声明
x: Int = 42
name: String = "YaoXiang"

// 函数定义
add: (a: Int, b: Int) -> Int = a + b

// 类型定义（使用花括号）
Point: Type = { x: Float, y: Float }

// 使用类型
p: Point = Point(x=1.0, y=2.0)
p.x  // 1.0
p.y  // 2.0
```

#### Record type (запись)

```yaoxiang
// 结构体类型
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// 使用
p = Point(x=3.0, y=4.0)
r = Rect(x=0.0, y=0.0, width=10.0, height=20.0)
```

#### Определение interface type (интерфейса)

Интерфейс — это record type, все поля которого являются функциональными типами:

```yaoxiang
// 定义接口
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// 空接口
EmptyInterface: Type = {}
```

#### Методы типа

Используйте синтаксис `Type.method: (Type, ...) -> Return = ...` для определения методов типа:

```yaoxiang
// 类型定义
Point: Type = { x: Float, y: Float }

// 类型方法定义
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    "Point({self.x}, {self.y})"
}

// 使用方法（语法糖）
p = Point(x=1.0, y=2.0)
p.draw(screen)  // → Point.draw(p, screen)
str = p.serialize()  // → Point.serialize(p)
```

#### Автоматическое связывание (builtin binding)

Функции, объявленные с ключевым словом `pub`, автоматически привязываются к типам, определённым в том же файле:

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub 声明自动绑定到 Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// 使用
p1 = Point(x=3.0, y=4.0)
p2 = Point(x=1.0, y=2.0)

// 函数式调用
d = distance(p1, p2)  // 3.606...

// OOP 语法糖（自动绑定到 Point.distance）
d2 = p1.distance(p2)  // → distance(p1, p2)
```

#### Enum type (перечисление)

```yaoxiang
// 简单枚举
Color: Type = { red | green | blue }

// 带数据的枚举
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// 使用泛型
success: Result(Int, String) = ok(42)
failure: Result(Int, String) = err("not found")
```

#### Generic типы

```yaoxiang
// 泛型类型定义
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (List(T), T) -> Void
}

// 具体实例化
IntList: Type = List(Int)
StringList: Type = List(String)
```

### Управление потоком

```yaoxiang
// 条件表达式
if x > 0 {
    "positive"
} elif x == 0 {
    "zero"
} else {
    "negative"
}

// 循环
for i in 0..5 {
    print(i)
}

// while 循环
mut n = 0
while n < 5 {
    print(n)
    n = n + 1
}
```

### Списки и словари

```yaoxiang
// 列表
numbers = [1, 2, 3, 4, 5]
first = numbers[0]  // 1

// 字典
scores = {"Alice": 90, "Bob": 85}
alice_score = scores["Alice"]  // 90

// 添加元素
mut list = [1, 2, 3]
list.append(4)
```

### Pattern matching (сопоставление с образцом)

```yaoxiang
// match 表达式
result: Result(Int, String) = ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## Spawn-программирование (асинхронность)

Уникальная особенность YaoXiang: функции, помеченные `spawn`, автоматически получают асинхронные возможности.

```yaoxiang
// 定义并作函数（自动异步执行）
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

// 调用并作函数（自动并行，无需 await）
main: () -> Void = {
    // 两次调用自动并行执行
    user = fetch_user(1)  // 自动并行
    posts = fetch_posts()  // 自动并行

    // 当需要结果时自动等待
    print(user.name)
    print(posts.length)
}
```

## Система модулей

```yaoxiang
// 导入标准库
use std.io
use std.math

// 使用导入的函数
result = math.sqrt(16)  // 4.0
print("Hello!")
```

## Часто задаваемые вопросы

### В: Переменные по умолчанию неизменяемые — как их изменять?

```yaoxiang
// 使用 mut 关键字声明可变变量
mut x = 10
x = 20  // ✅ OK
```

### В: Как определить функцию?

```yaoxiang
// 完整形式（推荐）
add: (a: Int, b: Int) -> Int = a + b

// 简短形式（类型推断）
add = (a, b) => a + b
```

### В: Как обрабатывать ошибки?

```yaoxiang
// 使用 Result 类型
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// 模式匹配处理
result = risky_operation()
match result {
    ok(value) => print("Success: " + value)
    err(error) => print("Error: " + error)
}
```

## Следующие шаги

- 📖 Прочтите [Руководство YaoXiang](../YaoXiang-book.md), чтобы узнать об основных возможностях
- 📚 Ознакомьтесь со [Спецификацией языка](../YaoXiang-language-specification.md) для изучения полного синтаксиса
- 🏗️ Изучите [Документацию по архитектуре](../architecture/), чтобы понять детали реализации
- 💡 Прочтите [Манифест дизайна](../YaoXiang-design-manifesto.md) для понимания ключевых идей

## Связанные ресурсы

- [Репозиторий на GitHub](https://github.com/yourusername/yaoxiang)
- [Сообщить о проблеме](https://github.com/yourusername/yaoxiang/issues)
- [Руководство для контрибьюторов](../guides/dev/)