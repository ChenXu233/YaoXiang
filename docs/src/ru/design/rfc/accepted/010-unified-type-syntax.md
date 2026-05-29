```md
---
title: "RFC-010：Унифицированный синтаксис типов"
---

# RFC-010: Унифицированный синтаксис типов - модель name: type = value

> **Статус**: Принято
>
> **Автор**: Чэнь Сюй
>
> **Дата создания**: 2025-01-20
>
> **Последнее обновление**: 2026-03-21 (завершена реализация этапов 1-4, унифицированы Fn/TypeDef/MethodBind в Binding)

## Резюме

Данный RFC предлагает предельно минималистичную унифицированную модель синтаксиса типов: **всё есть `name: type = value`**.

В YaoXiang существует только одна форма объявления:

```
identifier : type = expression
```

Где `type` может быть любым тип-выражением, а `expression` — любым выражением-значением.
**Нет `fn`, нет `struct`, нет `trait`, нет `impl`, нет ключевого слова `type` в нижнем регистре (но есть `Type` как ключевое слово метатипа)**.

> **Ключевой дизайн**: `Type` сам по себе является generics типом. `(T: Type) -> Type` означает «тип, принимающий параметр типа T».

| Концепция    | Запись кода                                        |
|--------------|----------------------------------------------------|
| Переменная   | `x: Int = 42`                                      |
| Функция      | `add: (a: Int, b: Int) -> Int = a + b`             |
| Запись       | `Point: Type = { x: Float, y: Float }`             |
| Интерфейс    | `Drawable: Type = { draw: (Surface) -> Void }`     |
| generics тип | `List: (T: Type) -> Type = { data: Array(T), length: Int }` |
| generics тип | `Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }` |
| Метод        | `Point.draw: (self: Point, s: Surface) -> Void = ...` |
| generics функция | `map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))` |

**`Type` — единственное ключевое слово метатипа в языке**.
Оно используется для обозначения уровня типов, компилятор автоматически обрабатывает различение Type0, Type1, Type2... прозрачно для пользователя.

```yaoxiang
// Основной синтаксис: унификация + различение

// Переменная
x: Int = 42

// Функция (имена параметров в сигнатуре)
add: (a: Int, b: Int) -> Int = a + b

// Запись
Point: Type = {
    x: Float,
    y: Float,
    draw: (Surface) -> Void,
    serialize: () -> String
}

// Интерфейс (по сути запись, все поля которой — функции)
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// Определение методов (синтаксис Type.method)
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

// generics тип ((T: Type) -> Type = generics тип, принимающий параметры типа)
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int
}

Map: (K: Type, V: Type) -> Type = {
    keys: Array(K),
    values: Array(V)
}

// Использование
p: Point = Point(1.0, 2.0)
p.draw(screen)           // синтаксический сахар → Point.draw(p, screen)
s: Drawable = p           // структурный подтип: Point реализует Drawable
drawables: List(Drawable) = [p, r]
process_all(drawables)
```

## Мотивация

### Зачем нужна эта функциональность?

Текущая система типов содержит несколько разрозненных концепций:

- Синтаксис объявления переменных
- Синтаксис определения функций
- Синтаксис определения типов (различный)
- Синтаксис определения интерфейсов
- Синтаксис привязки методов

Отсутствие унификации между этими концепциями приводит к фрагментации синтаксиса и высокой стоимости обучения.

### Цели проектирования

1. **Предельная унификация**: одно синтаксическое правило покрывает все случаи
2. **Простота и элегантность**: симметричная эстетика `name: type = value`
3. **Без новых ключевых слов**: повторное использование существующих синтаксических элементов
4. **Теоретическая элегантность**: типы сами по себе являются значениями типа Type
5. **Дружественность к generics**: бесшовная интеграция с системой generics (RFC-011)

### Интеграция с системой generics

Унифицированная модель синтаксиса RFC-010 **естественно согласуется** с дизайном системы generics RFC-011, параметры generics легко интегрируются в унифицированную модель:

```yaoxiang
// Базовый generics (RFC-011 Phase 1)
List: (T: Type) -> Type = { data: Array(T), length: Int }

// generics функция (RFC-023 синтаксис: Type в сигнатуре можно опустить, автоматически выводится при вызове)
map: (: Type, R: Type) -> (( list: List(T), f: (T) -> R) -> List(R)) = ...

// Ограничения типа (RFC-011 Phase 2)
clone: (value: T) -> T = value.clone()  // Ограничение T: Clone переносится вместе с типом параметра

// Const generics (RFC-011 Phase 4)
Array: (T: Type, N: Int) -> Type = { data: Array(T, N), length: N }
```

**Зависимости**:

- RFC-011 Phase 1 (базовый generics) является **сильной зависимостью** RFC-010
- Без базового generics примеры generics из RFC-010 не скомпилируются
- Рекомендация: RFC-011 Phase 1 и RFC-010 реализовать синхронно

## Предложение

### Ключевой принцип: конструктор типов vs функции/переменные

**Это ключевой выбор дизайна, определяющий правила разрешения синтаксической неоднозначности:**

| Запись     | Значение              | Правило |
|------------|----------------------|---------|
| **`x: Type = ...`** | Конструктор типов | `: Type` явно объявляет → принудительно считать типом |
| **`f = ...`**        | Функция или переменная | Нет `: Type` → HM активно выводит как функцию/переменную |

**Почему так спроектировано?**

Синтаксис `{ ... }` сам по себе неоднозначен:

- `{ x: Float, y: Float }` может быть **литералом типа** (запись)
- `{ a = 1 + 1 }` может быть **блоком кода** (выполняет операторы, возвращает Void)

**Правила разрешения неоднозначности**:

- **Есть** `: Type` → принудительное разбор как конструктора типов, `{ ... }` — литерал типа
- **Нет** `: Type` → HM активно разбирает `{ ... }` как блок кода, выводит как function type

```yaoxiang
# ✅ Конструктор типов: есть : Type
Point: Type = { x: Float, y: Float }

# ✅ Функция: нет : Type, HM выводит как () -> Void
main = { println("Hello") }

# ❌ Ошибка: нет : Type, компилятор не может разобрать { ... } как тип
Point = { x: Float, y: Float }  // HM выводит как функцию, не как тип!
```

---

**Унифицированная модель: identifier : type = expression**

```
├── Переменная
│   └── x: Int = 42
│
├── Функция
│   └── add: (a: Int, b: Int) -> Int = a + b  # Нет : Type, HM выводит как функцию
│
├── Запись
│   └── Point: Type = { x: Float, y: Float }  # Обязательный возврат: Type
│
├── Интерфейс
│   └── Drawable: Type = { draw: (Surface) -> Void }  # Обязательный возврат: Type
│
├── generics тип
│   └── List: (T: Type) -> Type = { data: Array(T), length: Int }  # Обязательный возврат: Type
│
├── generics тип (несколько параметров)
│   └── Map: (K: Type, V: Type) -> Type = { keys: Array(K), values: Array(V) }  # Обязательный возврат: Type
│
├── Метод
│   └── Point.draw: (self: Point, surface: Surface) -> Void = ...
│
└── generics функция
    └── map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R))  # Не возвращает Type, HM выводит как функцию
```

### Уровни метатипа (внутри компилятора)

**Компилятор внутри** поддерживает космологический уровень `level: selfpointnum` (хранится как строка, теоретически бесконечно расширяемый).

| Уровень | Описание |
|---------|----------|
| `Type0` | Повседневные типы (`Int`, `Float`, `Point`) |
| `Type1` | Конструкторы типов (`List`, `Maybe`) |
| `Type2+` | Высшие конструкторы |

**Пользователи никогда не видят эти числа**, только `: Type`.

> **Изоморфизм Карри-Говарда**: существование космологических уровней — не деталь инженерной реализации, а необходимое условие логической согласованности. Изоморфизм Карри-Говарда отождествляет типы с высказываниями; если позволить `Type: Type` (то есть «тип типа тоже является типом»), возникнет парадокс, подобный «это высказывание ложно» — в системе типов это проявляется как парадокс Жирара. Иерархия `Type0 / Type1 / Type2…` в YaoXiang (накопленные вселенные в теории типов Мартина-Лёфа) гарантирует, что каждый тип принадлежит определённому уровню, а `Typeₙ : Typeₙ₊₁` образует никогда не замыкающуюся восходящую цепочку, фундаментально избегая парадоксов. Это означает, что система типов YaoXiang в смысле Карри-Говарда является **логически согласованной**.

### Определение синтаксиса

#### 1. Объявление переменных

```yaoxiang
// Основной синтаксис
x: Int = 42
name: String = "Alice"
flag: Bool = true

// Type inference (можно опустить)
y = 100  // Выводится как Int
```

#### 2. Определение функций

```yaoxiang
// Полный синтаксис (имена параметров объявляются в сигнатуре)
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// С именами параметров
greet: (name: String) -> String = {
    return "Hello, ${name}!"
}

// Несколько параметров
calc: (x: Float, y: Float, op: String) -> Float = {
    return match op {
        "+" -> x + y,
        "-" -> x - y,
        _ -> 0.0
    }
}

// Тело из нескольких строк
calc2: (x: Float, y: Float) -> Float = {
    if x > y {
        return x
    }
    return y
}
```

#### Правила возврата

Все функции должны явно использовать ключевое слово `return` для возврата значения (кроме функций, возвращающих `()`):

```yaoxiang
// Не-Void тип возврата - необходимо использовать return
add: (a: Int, b: Int) -> Int = {
    return a + b
}

// Void тип возврата - использование return опционально (обычно опускается)
print: (msg: String) -> Void = {
    // return не нужен
}

// Однострочное выражение (возврат значения напрямую, без return)
greet: (name: String) -> String = "Hello, ${name}!"

// Тело из нескольких строк - необходимо использовать return
max: (a: Int, b: Int) -> Int = {
    if a > b {
        return a
    } else {
        return b
    }
}
```

#### 3. Определение типов

Определение типов — ядро унифицированного синтаксиса YaoXiang, включает поля, значения по умолчанию, привязанные методы, реализации интерфейсов:

##### Базовые типы

**Запись**: список полей, типы полей могут быть любыми тип-выражениями.

```yaoxiang
Point: Type = {
    x: Float,
    y: Float
}
```

**Поля со значениями по умолчанию**: поля могут иметь значения по умолчанию, необязательны при конструировании.

```yaoxiang
Point: Type = {
    x: Float = 0,
    y: Float = 0
}
```

Использование:

```yaoxiang
Point() → Point(x=0, y=0)
Point(x=1) → Point(x=1, y=0)
Point(x=1, y=2) → Point(x=1, y=2)
```

**Поля без значений по умолчанию**: должны быть предоставлены при конструировании.

```yaoxiang
Point2: Type = {
    x: Float,
    y: Float
}
```

Использование:
```yaoxiang
Point2(x=1, y=2) //✓
Point2() //✗
Point2(x=1) //✗
```

##### Привязка методов

**Способ 1: прямая привязка внешней функции внутри определения типа**

```yaoxiang
distance: (a: Point, b: Point) -> Float = { ... }
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           // Привязка к позиции 0, после каррирования method: (b: Point) -> Float
}
// Вызов: p1.distance(p2) → distance(p1, p2)
```

**Способ 2: анонимная функция + привязка по позиции**

```yaoxiang
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
// Вызов: p1.distance(p2) → distance(p1, p2)
```

##### Реализация интерфейсов

**Имя интерфейса записывается в теле типа, компилятор автоматически проверяет его реализацию**

```yaoxiang
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

Point: Type = {
    x: Float,
    y: Float,
    Drawable,          // Реализует интерфейс Drawable
    Serializable      // Реализует интерфейс Serializable
}
```

##### Определение интерфейсов

**Интерфейс = запись, все поля которой являются функциями**

```yaoxiang
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// Пустой тип/пустой интерфейс
EmptyType: Type = {}
Empty: Type = {}
```

##### Определение методов (внешнее)

**Методы типа**: ассоциированы с конкретным типом (синтаксис Type.method)

```yaoxiang
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}
```

##### Привязка методов (внешняя)

Обычные методы можно привязать к типу с помощью синтаксиса `[position]` (подробный синтаксис см. RFC-004).

**Ручная привязка**:

```yaoxiang
// Явная привязка
Point.distance = distance[0]

// Указание позиции привязки
Point.transform = transform[1]  // this привязан к позиции 1
```

**Многопозиционная привязка**:

```yaoxiang
// Привязка нескольких позиций (автоматическое каррирование)
Point.transform = transform_points[0, 1]
// Вызов: p1.transform(p2)(2.0) → transform_points(p1, p2, 2.0)
```

**Обратная привязка** (метод типа → обычная функция):

```yaoxiang
// Метод типа в обычную функцию
draw_point: (p: Point, surface: Surface) -> Void = Point.draw
```

#### 4. Композиция интерфейсов

```yaoxiang
// Композиция интерфейсов = пересечение типов
DrawableSerializable: Type = Drawable & Serializable

// Использование типа пересечения
process: (T: Drawable & Serializable) -> ((item: T, screen: Surface) -> String) = {
    item.draw(screen)
    return item.serialize()
}
```

#### 5. generics типы

```yaoxiang
// Базовый generics (RFC-011 Phase 1)
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (T:Type)-((self: List(T), item: T) -> Void),
    get: (T:Type)->((self: List(T), index: Int) -> Maybe(T))
}

// Конкретная реализация (RFC-023 синтаксис)
IntList: Type = List(Int)

IntList.push = {
    self.data.append(item)
    self.length = self.length + 1
}

List.push = (type: Type) -> {
    return (self: List(type), item: type) -> {
        self.data.append(item)
        self.length = self.length + 1
    }
}

IntList.push(Int)(self, item)  // Пример вызова

// generics методы (RFC-023 синтаксис: параметры типа автоматически выводятся из точки вызова)
List.push: (self: List(T), item: T) -> Void = {
    self.data.append(item)
    self.length = self.length + 1
}

List.get: (self: List(T), index: Int) -> Maybe(T) = {
    if index >= 0 && index < self.length {
        return Maybe.Just(self.data[index])
    } else {
        return Maybe.Nothing
    }
}
```

#### 6. Синтаксис вызова generics

Вызов generics типов и generics функций унифицирован и использует синтаксис `()`. `[]` не используется ни в каком generics контексте.

**Основные правила**:

1. **`()` делает всё**: применение типа, вызов функции, конструирование значения — всё через `()`

```yaoxiang
# Аннотация типа
numbers: List(Int) = List(1, 2, 3)

# Пустой контейнер: T приходит слева
empty: List(Int) = List()

# Вызов generics функции — типы автоматически перетекают из параметров
strings = map(numbers, f)
// T=Int приходит из numbers: List(Int)
// R=String приходит из f: (Int) -> String
```

2. **Type слева, значение справа**: `name: type = value` — параметры Type объявляются слева, справа всегда конкретное значение. Для пустого контейнера `List()` параметр `T` должен быть получен из аннотации типа слева.

3. **Информация о типе пишется один раз** — при объявлении параметра, компилятор несёт её вместе:

```yaoxiang
numbers: List(Int) = List(1, 2, 3)  // Int пишется один раз слева
f: (Int) -> String = (x) => x.to_string()
strings = map(numbers, f)   // T=Int, R=String автоматически из numbers и f
```

4. **Конструирование значения выводит тип из элементов**:

```yaoxiang
x = List(1, 2, 3)       // Выводится как List(Int)
y = List("a", "b")      // Выводится как List(String)
z = List()              // ❌ Ошибка компиляции: невозможно вывести T
z: List(Int) = List()   // ✅ T=Int из аннотации слева
```

5. **Псевдонимы типов**:

```yaoxiang
IntList: Type = List(Int)
StringToInt: Type = (String) -> Int
Matrix3x3: Type = Matrix(Float, 3, 3)
```

> **Сравнение со старым синтаксисом**: `List[Int]` → `List(Int)`, `List[Int]()` → `List()`, `List[Int](1,2,3)` → `List(1,2,3)`.
> Старый синтаксис generics с `[]` полностью удалён. `[]` используется только для литералов массивов/списков и доступа по индексу.

### Примеры

#### Полный пример

```yaoxiang
// ======== 1. Определение интерфейсов ========

Drawable: Type = {
    draw: (self: Self, surface: Surface) -> Void,
    bounding_box: (self: Self) -> Rect
}

Serializable: Type = {
    serialize: (self: Self) -> String
}

Transformable: Type = {
    translate: (self: Self, dx: Float, dy: Float) -> Self,
    scale: (self: Self, factor: Float) -> Self
}

// ======== 2. Определение типов ========

Point: Type = {
    x: Float,
    y: Float,
    Drawable,
    Serializable,
    Transformable
}

Rect: Type = {
    x: Float,
    y: Float,
    width: Float,
    height: Float,
    Drawable,
    Serializable,
    Transformable
}

// ======== 3. Определение методов ========

// Методы Point
draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

bounding_box: (self: Point) -> Rect = {
    return Rect(self.x - 1, self.y - 1, 2, 2)
}

serialize: (self: Point) -> String = {
    return "Point(${self.x}, ${self.y})"
}

translate: (self: Point, dx: Float, dy: Float) -> Point = {
    return Point(self.x + dx, self.y + dy)
}

scale: (self: Point, factor: Float) -> Point = {
    return Point(self.x * factor, self.y * factor)
}

// Обычные методы (pub, автоматически привязываются к Point.distance)
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

// Методы Rect
draw: (self: Rect, surface: Surface) -> Void = {
    surface.draw_rect(self.x, self.y, self.width, self.height)
}

bounding_box: (self: Rect) -> Rect = self

serialize: (self: Rect) -> String = {
    return "Rect(${self.x}, ${self.y}, ${self.width}, ${self.height})"
}

translate: (self: Rect, dx: Float, dy: Float) -> Rect = {
    return Rect(self.x + dx, self.y + dy, self.width, self.height)
}

scale: (self: Rect, factor: Float) -> Rect = {
    return Rect(self.x * factor, self.y * factor, self.width * factor, self.height * factor)
}

// ======== 4. Привязка методов ========

Point.distance = distance[0]  // Привязка к позиции 0, после каррирования method: (p2: Point) -> Float
Point.transform = transform[1]  // Привязка к позиции 1, после каррирования method: (dx: Float, dy: Float) -> Point
Rect.transform = transform[1]  // Привязка к позиции 1, после каррирования method: (dx: Float, dy: Float) -> Rect

// ... и так далее, привязка остальных методов...

// ======== 5. Использование ========

// Создание экземпляров
p: Point = Point(1.0, 2.0)
r: Rect = Rect(0.0, 0.0, 10.0, 20.0)

// Вызов методов (синтаксический сахар)
p.draw(screen)
r.draw(screen)

// Вызов обычных методов (прямой вызов)
d: Float = distance(p, Point(0.0, 0.0))

// Цепочечные вызовы
p2: Point = p.translate(1.0, 1.0).scale(2.0)

// Присваивание интерфейсу
drawables: List(Drawable) = [p, r]
for d in drawables {
    d.draw(screen)
}

// generics функции (RFC-023 синтаксис: параметры типа опускаются при вызове, автоматически выводятся)
process_all: (items: List(T)) -> Void = {
    for item in items {
        print(item.serialize())
    }
}

process_all([p, r])
```

## Детальное проектирование

### Алгоритм проверки интерфейсов

```rust
fn check_type_implements_interface(
    typ: &Type,
    iface: &Type
) -> Result<(), TypeError> {
    // Для каждого поля интерфейса (поле-функция)
    for (field_name, iface_field) in &iface.fields {
        // Проверяем, есть ли у типа метод с таким же именем
        if let Some(method) = typ.methods.get(field_name) {
            // Проверяем совместимость сигнатуры метода
            // Поле интерфейса: (Surface) -> Void
            // Сигнатура метода: (Point, Surface) -> Void
            // Сравнение: после удаления параметра self должно совпадать
            if !method_signature_matches(method, iface_field.type_) {
                return Err(TypeError::MethodSignatureMismatch {
                    type_name: typ.name,
                    interface_name: iface.name,
                    method_name: field_name,
                });
            }
        } else {
            return Err(TypeError::MissingMethod {
                type_name: typ.name,
                interface_name: iface.name,
                method_name: field_name,
            });
        }
    }
    Ok(())
}
```

### Прямое присваивание интерфейса и оптимизация на этапе компиляции

Интерфейсные типы поддерживают прямое присваивание, компилятор автоматически выбирает оптимальную стратегию вызова на основе типа правого значения:

```yaoxiang
// Прямое присваивание конкретного типа → конкретный тип известен на этапе компиляции, вызов без накладных расходов
d: Drawable = Circle(1)
d.draw(screen)  // После компиляции: прямой вызов circle_draw(screen), без vtable

// Возврат из функции → конкретный тип нельзя определить на этапе компиляции, используется vtable
d: Drawable = get_shape()
d.draw(screen)  // Поиск метода через vtable

// Гетерогенная коллекция → используется vtable
shapes: List(Drawable) = [Circle(1), Rect(2, 3)]
for s in shapes {
    s.draw(screen)  // Поиск метода через vtable
}
```

**Стратегии оптимизации на этапе компиляции**:

| Сценарий                      | Результат вывода      | Способ вызова |
|-------------------------------|----------------------|---------------|
| `d: Drawable = Circle(1)`     | Конкретный тип Circle | Прямой вызов (ноль накладных расходов) |
| `d: Drawable = get_shape()`   | Неизвестен           | vtable |
| `shapes: List(Drawable) = [...]` | Гетерогенный     | vtable |

**Правила**:

1. Когда правое значение является конкретным конструктором типа и может быть определён на этапе компиляции, генерируется прямой вызов IR
2. Когда тип правого значения невозможно определить на этапе компиляции, происходит откат к механизму vtable
3. vtable как fallback гарантирует корректность динамического полиморфизма во время runtime

### Поддержка утиной типизации

```yaoxiang
// Пока есть одинаковые методы, можно присвоить типу интерфейса
CustomPoint: Type = {
    draw: (self: CustomPoint, surface: Surface) -> Void,
    x: Float,
    y: Float
}

custom: CustomPoint = CustomPoint(
    (self: CustomPoint, surface: Surface) => surface.plot(self.x, self.y),
    1.0,
    2.0
)
```

### Изменения синтаксиса

| Раньше                                                    | Теперь                                                    |
|-----------------------------------------------------------|-----------------------------------------------------------|
| `type Point = Point(x: Float, y: Float)`                  | `type Point = { x: Float, y: Float }`                     |
| `type Result(T, E) = ok(T) \| err(E)`                     | `Result: (T: Type, E: Type) -> Type = { ok: (T) -> Self, err: (E) -> Self }` |
| Требовалось ключевое слово `impl`                         | Ключевое слово не нужно, имя интерфейса пишется после тела типа |

## Объяснение дизайна синтаксиса: именованные функции по сути являются синтаксическим сахаром Lambda

### Ключевое понимание

**Именованные функции и Lambda-выражения — это одно и то же!** Единственное различие: именованная функция просто даёт имя Lambda-выражению.

```yaoxiang
// Эти два варианта по сути идентичны
add: (a: Int, b: Int) -> Int = a + b           // Именованная функция (рекомендуется)
add: (a: Int, b: Int) -> Int = (a, b) => a + b        // Lambda форма (полностью эквивалентно)
```

### Модель синтаксического сахара

```
// Именованная функция = Lambda + имя
name: (Params) -> ReturnType = body

// По сути эквивалентно
name: (Params) -> ReturnType = (params) => body
```

**Ключевой момент**: когда сигнатура полностью объявляет типы параметров, имена параметров в голове Lambda становятся избыточными и могут быть опущены.

### Правила области видимости параметров

**Параметры перекрывают внешние переменные**: область видимости параметров в сигнатуре перекрывает тело функции, приоритет у внутренней области выше.

```yaoxiang
x = 10  // Внешняя переменная

double: (x: Int) -> Int = x * 2  // ✅ Параметр x перекрывает внешний x, результат 20
```

### Гибкость размещения аннотаций

Аннотации типа можно ставить в любом из следующих мест, **достаточно аннотировать хотя бы в одном**:

| Место аннотации | Форма                               | Описание |
|-----------------|-------------------------------------|----------|
| Только в сигнатуре | `double: (x: Int) -> Int = x * 2` | ✅ Рекомендуется |
| Только в голове Lambda | `double = (x: Int) => x * 2` | ✅ Допустимо |
| Обе стороны     | `double: (x: Int) -> Int = (x) => x * 2` | ✅ Избыточно, но допустимо |

### Полный пример

```yaoxiang
// ✅ Рекомендуется: сигнатура полная, голова Lambda опущена
add: (a: Int, b: Int) -> Int = a + b
inc: (x: Int) -> Int = x + 1
main: () -> Void = { print("hi") }

// ✅ Допустимо: тип в голове Lambda
double = (x: Int) => x * 2

// ✅ Допустимо: обе стороны аннотированы
double: (x: Int) -> Int = (x) => x * 2
```

### Преимущества дизайна

| Характеристика | Преимущество |
|----------------|--------------|
| **Простота** | При полной сигнатуре не нужно повторно писать имена параметров |
| **Гибкость** | Сохранена Lambda форма, используйте любую |
| **Согласованность** | Единообразие с объявлением переменных `x: Int = 42` |
| **Наглядность** | `name: Type = body` напрямую соответствует «имя name, тип Type, значение body» |

## Компромиссы

### Преимущества

| Преимущество     | Описание |
|------------------|----------|
| Предельная унификация | Одно синтаксическое правило покрывает все случаи |
| Теоретическая элегантность | Идеально симметричная модель `name: type = value` |
| Без новых ключевых слов | Повторное использование существующих синтаксических элементов |
| Простота реализации | Компилятору нужно обрабатывать только одну форму объявления |
| Простота обучения | Одно правило позволяет писать любой код |
| Простота расширения | Новые возможности естественно вписываются в эту модель |

### Недостатки

| Недостаток       | Описание |
|------------------|----------|
| Соглашения об именовании | Методы должны следовать именованию `Type.method` |
| Многословность   | Полный синтаксис длиннее сокращённого, но может быть выведен |
| Кривая обучения   | Требуется понимание унифицированной модели |

### Меры по смягчению

```yaoxiang
// 1. Чёткие сообщения об ошибках
// Пример ошибки компиляции:
// Error: Point does not implement Serializable
//   Required method 'serialize: (self: Point) -> String' not found
//   Note: Define Point.serialize to implement Serializable

// 2. Type inference
// Тип можно опустить, компилятор выведет сам
Point.draw = (self: Point, surface: Surface) => surface.plot(self.x, self.y)

// 3. Подсказки IDE
// IDE автоматически предлагает недостающие методы
```

### Риски

| Риск             | Влияние                    | Меры по смягчению |
|------------------|---------------------------|-------------------|
| Сложность парсинга | Унифицированный синтаксис может увеличить сложность парсинга | Использовать рекурсивный нисходящий парсер |
| Накладные расходы на производительность | Поиск в vtable может иметь дополнительные накладные расходы | Оптимизация мономорфизации на этапе компиляции |

---

## Пасхалка 🎮: Источник языка

> ✨ **Type: Type = Type** ✨

```yaoxiang
// Попытка определить тип типа...
Type: Type = Type
```

**Предупреждение**: это **невыразимое** нечто!

```
╔══════════════════════════════════════════════════════════════╗
║                                                              ║
║   一生二，二生三，三生万物。                                   ║
║   易有太极，是生两仪。                                         ║
║                                                              ║
║   Type: Type = Type                                          ║
║   Это источник YaoXiang, граница языка.                      ║
║   Здесь компилятор безмолвствует, здесь философия пребывает.  ║
║                                                              ║
║   Благодарим за прикосновение к философской границе языка.   ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

> **Примечание**: Компилятор не может корректно обработать `Type: Type = Type` (это приводит к космологическому парадоксу Type0/Type1), но мы намеренно сохраняем эту «пасхалку» — когда вы попытаетесь её скомпилировать, получите дзэнское послание от создателя языка. Это не только техническая граница, но и дань уважения философии типов от YaoXiang.

---

## Приложения

### БНФ синтаксиса

```bnf
program ::= statement*

statement ::= declaration | expression

# Унифицированное объявление: name: Type = expression
declaration ::= identifier ':' type_expr '=' expression

# Тип-выражение
type_expr ::= identifier
       | identifier '(' type_expr (',' type_expr)* ')'      # Применение типа
       | '(' type_expr (',' type_expr)* ')' '->' type_expr       # Function type
       | '{' type_field* '}'                       # Запись/интерфейс
       | 'Type'                                    # Метатип

type_field ::= identifier ':' type_expr
             | identifier                           # Ограничение интерфейса

# generics параметры: как часть function type, например (T: Type, R: Type) -> (...)
# Не требуется отдельное БНФ правило — параметры : Type это обычные параметры функций

# Выражение
expression ::= literal
              | identifier
              | identifier '(' expression (',' expression)* ')'  # Вызов функции / вызов конструктора
              | '(' expression (',' expression)* ')'              # Кортеж
              | expression '.' identifier '(' arguments? ')'    # Вызов метода
              | lambda
              | '{' field ':' expression (',' field ':' expression)* '}'

arguments ::= expression (',' expression)*

lambda ::= '(' parameter_list? ')' '=>' block

block ::= expression | '{' expression* '}'
```

### Глоссарий

| Термин          | Определение |
|-----------------|-------------|
| Объявление      | Оператор присваивания формы `name: type = value` |
| Запись          | Тип `{ ... }` с именованными полями |
| Интерфейс       | Запись, все поля которой являются function type |
| generics тип    | Тип, определённый как `Name: (T: Type) -> Type = { ... }`, принимающий параметры типа |
| Метод типа      | Метод формы `Type.method`, ассоциированный с конкретным типом |
| generics функция | Функция, использующая синтаксис `(T: Type)`, параметры типа передаются как первая группа параметров |
| Метатип         | `Type`, единственный маркер уровня типов в языке |

---

## Жизненный цикл и судьба

```
┌─────────────┐
│   Черновик  │  ← Текущее состояние
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  На review  │  ← Открыто обсуждение сообщества и обратная связь
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  Принято    │    │  Отклонено  │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│(официальный│    │(оставлен на │
│   дизайн)   │    │   месте)    │
└─────────────┘    └─────────────┘
```