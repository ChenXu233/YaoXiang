---
title: "Формат yaoxiang.toml"
description: Описание формата файла конфигурации проекта
---

# Формат yaoxiang.toml

`yaoxiang.toml` — это манифест проекта YaoXiang, объявляющий метаданные проекта и зависимости.

## Структура файла

```toml
[package]
name = "Название проекта"
version = "0.1.0"
description = "Описание проекта"
authors = ["Имя автора"]
license = "MIT"

[dependencies]
# Обычные зависимости

[dev-dependencies]
# Зависимости для разработки
```

## Секция package

| Поле | Тип | Обязательно | Описание |
|------|------|------|------|
| `name` | string | Да | Название проекта, должно соответствовать правилам именования (строчные буквы, цифры, дефисы) |
| `version` | string | Да | Семантический номер версии в соответствии со спецификацией semver |
| `description` | string | Нет | Краткое описание проекта |
| `authors` | array | Нет | Список авторов |
| `license` | string | Нет | Идентификатор лицензии |

### Пример

```toml
[package]
name = "my-awesome-app"
version = "1.2.3"
description = "Замечательное приложение"
authors = ["Иван Иванов <ivan@example.com>"]
license = "MIT"
```

## Объявление зависимостей

### Простая версия

```toml
[dependencies]
http = "1.0.0"
json = "*"
```

### Подробная конфигурация

```toml
[dependencies]
# Git-зависимость
http = { version = "1.0.0", git = "https://github.com/example/http" }

# Локальная зависимость по пути
utils = { version = "0.1.0", path = "./utils" }

# Git-зависимость с веткой
bleeding-edge = { git = "https://github.com/example/edge", branch = "main" }
```

### Описание полей зависимостей

| Поле | Тип | Описание |
|------|------|------|
| `version` | string | Номер версии или диапазон версий |
| `git` | string | Адрес Git-репозитория |
| `branch` | string | Имя ветки Git |
| `path` | string | Локальный относительный путь |

## Синтаксис номеров версий

| Синтаксис | Описание | Пример |
|------|------|------|
| `*` | Любая версия | `"*"` |
| `1.0.0` | Точная версия | `"1.0.0"` |
| `>=1.0.0` | Минимальная версия | `">=1.0.0"` |
| `<2.0.0` | Максимальная версия | `"<2.0.0"` |
| `>=1.0.0, <2.0.0` | Диапазон версий | `">=1.0.0, <2.0.0"` |
| `~1.0.0` | Совместимая версия | `"~1.0.0"` |
| `^1.0.0` | Caret-версия | `"^1.0.0"` |

## Полный пример

```toml
[package]
name = "web-server"
version = "0.1.0"
description = "Простой веб-сервер"
authors = ["Разработчик <dev@example.com>"]
license = "MIT"

[dependencies]
http = "1.0.0"
json = "2.0.0"
router = { version = "0.5.0", path = "./router" }

[dev-dependencies]
test-utils = "1.0.0"
benchmark = "0.1.0"
```