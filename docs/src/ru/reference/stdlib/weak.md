---
title: 'std.weak'
description: 'Arc / Weak слабые ссылки'
---

# std.weak

Модуль слабых ссылок, используется совместно с `Arc` для разрыва циклических ссылок.

```yaoxiang
use std.weak
```

> Данный модуль зависит от атомарного подсчёта ссылок и **не экспортируется** на целевой платформе
> `wasm32`.

## Обзор функций

<!-- stdlib:table:weak start -->

| Функция   | Сигнатура                                    |
| --------- | -------------------------------------------- |
| `new`     | `(T: Type)(arc: Arc(T)) -> Weak(T)`          |
| `upgrade` | `(T: Type)(weak: Weak(T)) -> Option(Arc(T))` |

<!-- stdlib:table:weak end -->

## Функции

### new

<!-- stdlib:sig:weak.new start -->

```yaoxiang
new: (T: Type)(arc: Arc(T)) -> Weak(T)
```

<!-- stdlib:sig:weak.new end -->

Создаёт слабую ссылку, соответствующую `Arc`.

- `arc` — значение сильной ссылки; передаётся по значению, после вызова **перемещается**.

Возвращает: дескриптор `Weak`, указывающий на тот же блок выделения памяти, **не увеличивая**
счётчик сильных ссылок.

```yaoxiang
use std.assert
use std.weak

main: () -> Void = {
    // ref 创建 Arc[Int]
    p = ref 42

    // Arc → Weak 登记
    w = weak.new(p)
    assert(true)
}
```

### upgrade

<!-- stdlib:sig:weak.upgrade start -->

```yaoxiang
upgrade: (T: Type)(weak: Weak(T)) -> Option(Arc(T))
```

<!-- stdlib:sig:weak.upgrade end -->

Пытается повысить слабую ссылку до сильной.

- `weak` — дескриптор слабой ссылки.

Возвращает: `Option.some(Arc)`, если блок выделения памяти всё ещё жив; `Option.none()` — если
освобождён. **Не выбрасывает ошибку** — наличие цели выражается через `Option`.

```yaoxiang
use std.weak
use std.option

main: () -> Void = {
    p = ref 42
    w = weak.new(p)

    // upgrade：目标存活返回 some(v)，已释放返回 none()
    u = weak.upgrade(w)
    match u {
        some(v) => println("alive"),
        none() => println("dropped"),
    }
}
```

> **Предварительное замечание о деструктуризации вариантов**: деструктуризация вариантов `Option`
> требует наличия набора вариантов — после `use std.option` становятся доступны `match some(v)` /
> `none()` (см. спецификацию языка §2.8 match).

## Семантика

Слабая ссылка **не владеет**: наличие `Weak` не препятствует освобождению цели. Типичное применение
— разрыв циклических ссылок: родительский узел владеет `Arc`, указывающим на дочерний узел, а
дочерний узел владеет лишь `Weak`, указывающим обратно на родительский, — и цикл разрывается.

## См. также

- [Спецификация языка: система типов](../language-spec/type-system.md) — Семантика владения `Arc` /
  `Weak`
