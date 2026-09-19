---
title: 'std.concurrent'
description: 'Сон, уступка планировщику и идентификатор потока'
---

# std.concurrent

Вспомогательный модуль для параллелизма.

```yaoxiang
use std.concurrent
```

> Этот модуль зависит от потоков ОС и **не экспортируется** на целевой платформе `wasm32`.

## Обзор функций

<!-- stdlib:table:concurrent start -->

| Функция     | Сигнатура               |
| ----------- | ----------------------- |
| `sleep`     | `(millis: Int) -> Void` |
| `thread_id` | `() -> String`          |
| `yield_now` | `() -> Void`            |

<!-- stdlib:table:concurrent end -->

## Функции

### sleep

<!-- stdlib:sig:concurrent.sleep start -->

```yaoxiang
sleep: (millis: Int) -> Void
```

<!-- stdlib:sig:concurrent.sleep end -->

Блокирует текущий поток на указанное **количество миллисекунд**.

- `millis` — количество миллисекунд сна; если значение не `Int` или отсутствует, обрабатывается как
  `0` (без ошибки)

> **Внимание к единицам измерения**: [`std.time.sleep`](./time#sleep) работает с **секундами** и
> принимает дробные значения, тогда как данная функция работает с **миллисекундами**.
> `concurrent.sleep(1)` усыпляет на 1 миллисекунду, `time.sleep(1)` усыпляет на 1 секунду.

```yaoxiang
use std.concurrent

main: () -> Void = {
    concurrent.sleep(0)
    concurrent.sleep(1)
}
```

### thread_id

<!-- stdlib:sig:concurrent.thread_id start -->

```yaoxiang
thread_id: () -> String
```

<!-- stdlib:sig:concurrent.thread_id end -->

Возвращает строку-идентификатор текущего потока.

Возвращает: строку вида `ThreadId(1)`. Конкретное значение зависит от платформы и планировщика,
поэтому **следует использовать только для проверки наличия**, не полагаясь на конкретное содержимое
или формат.

```yaoxiang
use std.assert
use std.concurrent
use std.string

main: () -> Void = {
    tid = concurrent.thread_id()
    assert(string.len(tid) > 0)
}
```

### yield_now

<!-- stdlib:sig:concurrent.yield_now start -->

```yaoxiang
yield_now: () -> Void
```

<!-- stdlib:sig:concurrent.yield_now end -->

Добровольно уступает квант времени планировщика текущего потока, предоставляя возможность выполнения
другим потокам.

```yaoxiang
use std.concurrent

main: () -> Void = {
    concurrent.yield_now()
}
```

## Связанные материалы

- [`std.time.sleep`](./time#sleep) — посекундный сон
- [Спецификация языка: модель параллелизма](../language-spec/concurrency.md) — `spawn` и семантика
  spawn
