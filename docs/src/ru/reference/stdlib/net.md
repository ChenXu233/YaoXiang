---
title: 'std.net'
description: 'HTTP-запросы и процентное кодирование/декодирование URL'
---

# std.net

Сетевой модуль.

```yaoxiang
use std.net
```

> Данный модуль зависит от сетевых возможностей операционной системы и **не экспортируется** для
> целевой платформы `wasm32`.

> **Предупреждение о состоянии реализации (#56)**: Из 4 функций данного модуля только `url_encode` /
> `url_decode` являются реальными реализациями. `http_get` / `http_post` — это **заглушки — они не
> отправляют никаких сетевых запросов**, а лишь склеивают параметры в строку и возвращают её.
> Подробности см. в описании каждой функции ниже.

## Обзор функций

<!-- stdlib:table:net start -->

| Функция      | Сигнатура                                 |
| ------------ | ----------------------------------------- |
| `http_get`   | `(url: &String) -> String`                |
| `http_post`  | `(url: &String, body: &String) -> String` |
| `url_encode` | `(s: &String) -> String`                  |
| `url_decode` | `(s: &String) -> String`                  |

<!-- stdlib:table:net end -->

## Функции

### http_get

<!-- stdlib:sig:net.http_get start -->

```yaoxiang
http_get: (url: &String) -> String
```

<!-- stdlib:sig:net.http_get end -->

> **Заглушка, HTTP-клиент не подключён (#56).** Текущее поведение: склеивает входной аргумент в
> строку `"GET: {url}"` и возвращает её, **не отправляя никаких сетевых запросов** и не возвращая
> тело ответа. Полагаться на эту функцию для реальных HTTP-вызовов нельзя — в результате будет
> получена описательная строка, а не содержимое ответа.

- `url` — адрес запроса (только для чтения)

Возвращает: строку вида `"GET: http://example.com"`. Ошибки: при отсутствии аргумента выбрасывает
`E6007`; при аргументе не типа `String` выбрасывает ошибку типа.

```yaoxiang
use std.assert
use std.net

main: () -> Void = {
    // 当前实现返回描述字符串，而非响应体
    r = net.http_get("http://example.com")
    assert(r == "GET: http://example.com")
}
```

### http_post

<!-- stdlib:sig:net.http_post start -->

```yaoxiang
http_post: (url: &String, body: &String) -> String
```

<!-- stdlib:sig:net.http_post end -->

> **Заглушка, HTTP-клиент не подключён (#56).** Текущее поведение: склеивает строку
> `"POST {url}: {body}"` и возвращает её, **не отправляя никаких сетевых запросов**.

- `url` — адрес запроса (только для чтения)
- `body` — тело запроса (только для чтения)

Возвращает: строку вида `"POST http://example.com: hello"`. Ошибки: при недостаточном числе
аргументов выбрасывает `E6007`; при несоответствии типов аргументов выбрасывает ошибку типа.

```yaoxiang
use std.assert
use std.net

main: () -> Void = {
    r = net.http_post("http://example.com", "hello")
    assert(r == "POST http://example.com: hello")
}
```

### url_encode

<!-- stdlib:sig:net.url_encode start -->

```yaoxiang
url_encode: (s: &String) -> String
```

<!-- stdlib:sig:net.url_encode end -->

Процентное кодирование (percent-encoding).

- `s` — кодируемая строка (только для чтения)

Возвращает: закодированную строку. Пробел кодируется как `%20` (не `+`); зарезервированные символы
экранируются по RFC 3986; незарезервированные символы остаются без изменений.

Ошибки: при отсутствии аргумента выбрасывает `E6007`; при аргументе не типа `String` выбрасывает
ошибку типа.

```yaoxiang
use std.assert
use std.net

main: () -> Void = {
    assert(net.url_encode("a b") == "a%20b")
    assert(net.url_encode("a&b=c?d") == "a%26b%3Dc%3Fd")
}
```

### url_decode

<!-- stdlib:sig:net.url_decode start -->

```yaoxiang
url_decode: (s: &String) -> String
```

<!-- stdlib:sig:net.url_decode end -->

Процентное декодирование, обратное к `url_encode`.

- `s` — закодированная строка (только для чтения)

Возвращает: декодированную строку. Недопустимые escape-последовательности сохраняются как есть.

Ошибки: при отсутствии аргумента выбрасывает `E6007`; при аргументе не типа `String` выбрасывает
ошибку типа.

```yaoxiang
use std.assert
use std.net

main: () -> Void = {
    assert(net.url_decode("a%20b") == "a b")

    // 往返一致
    orig = "hello world & friends"
    assert(net.url_decode(net.url_encode(orig)) == orig)
}
```

## Связанные материалы

- [Справочник по кодам ошибок](../error-code/) — `E6007` — общая ошибка времени выполнения
