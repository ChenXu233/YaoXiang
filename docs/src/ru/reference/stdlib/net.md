---
title: 'std.net'
description: 'HTTP-запросы и процентное кодирование URL'
---

# std.net

Модуль сети.

```yaoxiang
use std.net
```

> Данный модуль зависит от сетевых возможностей операционной системы и **не экспортируется** на
> целевой платформе `wasm32`.

HTTP-функции реализованы на основе синхронного блокирующего клиента (rustls TLS), перенаправления
отслеживаются по умолчанию (не более 5 раз). Блокировка действует только в потоке выполнения вызова,
что согласуется с явной моделью параллелизма [`spawn`](../../language-spec/concurrency).

## Обзор функций

<!-- stdlib:table:net start -->

| Функция      | Сигнатура                                                                                                 |
| ------------ | --------------------------------------------------------------------------------------------------------- |
| `http_get`   | `(url: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)`                |
| `http_post`  | `(url: &String, body: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)` |
| `url_encode` | `(s: &String) -> String`                                                                                  |
| `url_decode` | `(s: &String) -> String`                                                                                  |

<!-- stdlib:table:net end -->

## Функции

### http_get

<!-- stdlib:sig:net.http_get start -->

```yaoxiang
http_get: (url: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)
```

<!-- stdlib:sig:net.http_get end -->

Выполняет HTTP GET запрос и возвращает структурированный словарь ответа:

| Ключ      | Тип                    | Значение                                                                                                               |
| --------- | ---------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `status`  | `Int`                  | Код состояния HTTP (например, `200`, `404`)                                                                            |
| `headers` | `Dict(String, String)` | Заголовки ответа, ключи приведены к нижнему регистру; несколько значений с одинаковым именем объединяются через `", "` |
| `body`    | `String`               | Тело ответа (декодировано в UTF-8)                                                                                     |

- `url` — адрес запроса (неизменяемое заимствование)
- `headers` — необязательный словарь заголовков запроса; при пропуске отправляется `{}`
- `timeout_secs` — необязательное общее время ожидания в секундах; по умолчанию `30`

Ответы 4xx/5xx являются нормальными (код состояния передаётся в поле `status`) и **не** считаются
ошибкой; только сбои на транспортном уровне (ошибка DNS-разрешения, отказ в соединении, тайм-аут и
т. д.) приводят к выбросу `E6007`.

```yaoxiang
use std.net

// Схематический пример ответа (требуется сеть, не является исполняемым примером):
// resp = net.http_get("https://httpbin.org/get")
// status = resp["status"]        // 200
// body   = resp["body"]          // текст тела ответа
// ctype  = resp["headers"]["content-type"]
```

С заголовками запроса и пользовательским тайм-аутом:

```yaoxiang
use std.net

// net.http_get(
//     "https://api.example.com/v1/data",
//     { "Authorization": "Bearer token123" },
//     10,
// )
```

### http_post

<!-- stdlib:sig:net.http_post start -->

```yaoxiang
http_post: (url: &String, body: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)
```

<!-- stdlib:sig:net.http_post end -->

Выполняет HTTP POST запрос. Тело запроса отправляется в кодировке UTF-8, заголовок `Content-Length`
устанавливается автоматически. Структура словаря ответа такая же, как у [`http_get`](#http_get).

- `url` — адрес запроса (неизменяемое заимствование)
- `body` — тело запроса (неизменяемое заимствование)
- `headers` — необязательный словарь заголовков запроса; при пропуске отправляется `{}`
- `timeout_secs` — необязательное общее время ожидания в секундах; по умолчанию `30`

```yaoxiang
use std.net

// net.http_post(
//     "https://httpbin.org/post",
//     "payload=1&name=yx",
//     { "Content-Type": "application/x-www-form-urlencoded" },
// )
```

### url_encode

<!-- stdlib:sig:net.url_encode start -->

```yaoxiang
url_encode: (s: &String) -> String
```

<!-- stdlib:sig:net.url_encode end -->

Процентное кодирование (percent-encoding).

- `s` — кодируемая строка (неизменяемое заимствование)

Возвращает: закодированную строку. Пробелы кодируются как `%20` (а не `+`); зарезервированные
символы экранируются согласно RFC 3986; незарезервированные символы остаются без изменений.

Ошибки: при отсутствии аргумента выбрасывается `E6007`; при аргументе не типа `String` выбрасывается
ошибка типа.

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

- `s` — закодированная строка (неизменяемое заимствование)

Возвращает: декодированную строку. Недопустимые escape-последовательности сохраняются как есть.

Ошибки: при отсутствии аргумента выбрасывается `E6007`; при аргументе не типа `String` выбрасывается
ошибка типа.

```yaoxiang
use std.assert
use std.net

main: () -> Void = {
    assert(net.url_decode("a%20b") == "a b")

    // Идемпотентность при кодировании-декодировании
    orig = "hello world & friends"
    assert(net.url_decode(net.url_encode(orig)) == orig)
}
```

## Связанные материалы

- [Справочник по кодам ошибок](../error-code/) — `E6007` общая ошибка выполнения
