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
> таргета `wasm32`.

> **Предупреждение о состоянии реализации (#56):** Из 4 функций модуля только `url_encode` /
> `url_decode` являются реальными реализациями. Функции `http_get` / `http_post` — это **заглушки,
> которые не отправляют никаких сетевых запросов**, а лишь склеивают аргументы в строку и возвращают
> её. Подробности см. в описании каждой функции ниже.

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

> **Заглушка, HTTP-клиент не подключён (#56).** Текущее поведение — склеить входной аргумент в
> строку вида `"GET: {url}"` и вернуть её. **Сетевые запросы не отправляются**, тело ответа не
> возвращается. Опора на эту функцию для реальных HTTP-вызовов приведёт к тихому сбою — в результате
> вы получите описательную строку, а не содержимое ответа.

- `url` — адрес запроса (неизменяемое заимствование)

Возвращает: строку вида `"GET: http://example.com"`. Ошибки: при отсутствии аргумента выбрасывает
`E6007`; при аргументе, не являющемся `String`, выбрасывает ошибку типа.

```yaoxiang
use std.assert
use std.net

main = {
    // Текущая реализация возвращает описательную строку, а не тело ответа
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

> **Заглушка, HTTP-клиент не подключён (#56).** Текущее поведение — склеить строку вида
> `"POST {url}: {body}"` и вернуть её. **Сетевые запросы не отправляются**.

- `url` — адрес запроса (неизменяемое заимствование)
- `body` — тело запроса (неизменяемое заимствование)

Возвращает: строку вида `"POST http://example.com: hello"`. Ошибки: при нехватке аргументов
выбрасывает `E6007`; при несоответствии типов аргументов выбрасывает ошибку типа.

```yaoxiang
use std.assert
use std.net

main = {
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

- `s` — кодируемая строка (неизменяемое заимствование)

Возвращает: закодированную строку. Пробел кодируется как `%20` (а не как `+`); зарезервированные
символы экранируются по RFC 3986; незарезервированные символы остаются без изменений.

Ошибки: при отсутствии аргумента выбрасывает `E6007`; при аргументе, не являющемся `String`,
выбрасывает ошибку типа.

```yaoxiang
use std.assert
use std.net

main = {
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

Процентное декодирование, взаимно обратное к `url_encode`.

- `s` — закодированная строка (неизменяемое заимствование)

Возвращает: декодированную строку. Недопустимые escape-последовательности остаются как есть.

Ошибки: при отсутствии аргумента выбрасывает `E6007`; при аргументе, не являющемся `String`,
выбрасывает ошибку типа.

```yaoxiang
use std.assert
use std.net

main = {
    assert(net.url_decode("a%20b") == "a b")

    // Идемпотентность при круговом обходе
    orig = "hello world & friends"
    assert(net.url_decode(net.url_encode(orig)) == orig)
}
```

## Связанные материалы

- [Справочник по кодам ошибок](../error-code/) — `E6007` общая ошибка времени выполнения
