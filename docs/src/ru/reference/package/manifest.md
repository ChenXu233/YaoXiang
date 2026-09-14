---
title: 'Формат yaoxiang.toml'
description: 'Описание формата файла конфигурации проекта'
---

# Формат yaoxiang.toml

`yaoxiang.toml` — это файл манифеста проекта YaoXiang, в котором объявляются метаданные проекта и
зависимости.

## Структура файла

```toml
[package]
name = "项目名称"
version = "0.1.0"
description = "项目描述"
authors = ["作者名"]
license = "MIT"

[dependencies]
# 普通依赖

[dev-dependencies]
# 开发依赖
```

## Секция package

| Поле          | Тип    | Обязательно | Описание                                                                                |
| ------------- | ------ | ----------- | --------------------------------------------------------------------------------------- |
| `name`        | string | да          | Имя проекта, должно соответствовать правилам именования (строчные буквы, цифры, дефисы) |
| `version`     | string | да          | Семантический номер версии, соответствует спецификации semver                           |
| `description` | string | нет         | Краткое описание проекта                                                                |
| `authors`     | array  | нет         | Список авторов                                                                          |
| `license`     | string | нет         | Идентификатор лицензии                                                                  |

### Пример

```toml
[package]
name = "my-awesome-app"
version = "1.2.3"
description = "一个很棒的应用"
authors = ["张三 <zhangsan@example.com>"]
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
# Git 依赖
http = { version = "1.0.0", git = "https://github.com/example/http" }

# 本地路径依赖
utils = { version = "0.1.0", path = "./utils" }

# 带分支的 Git 依赖
bleeding-edge = { git = "https://github.com/example/edge", branch = "main" }
```

### Описание полей зависимостей

| Поле      | Тип    | Описание                     |
| --------- | ------ | ---------------------------- |
| `version` | string | Номер версии или диапазон    |
| `git`     | string | Адрес Git-репозитория        |
| `branch`  | string | Имя ветки Git                |
| `path`    | string | Локальный относительный путь |

## Синтаксис номеров версий

| Синтаксис         | Описание            | Пример              |
| ----------------- | ------------------- | ------------------- |
| `*`               | Любая версия        | `"*"`               |
| `1.0.0`           | Точная версия       | `"1.0.0"`           |
| `>=1.0.0`         | Минимальная версия  | `">=1.0.0"`         |
| `<2.0.0`          | Максимальная версия | `"<2.0.0"`          |
| `>=1.0.0, <2.0.0` | Диапазон версий     | `">=1.0.0, <2.0.0"` |
| `~1.0.0`          | Совместимая версия  | `"~1.0.0"`          |
| `^1.0.0`          | Версия caret        | `"^1.0.0"`          |

## Полный пример

```toml
[package]
name = "web-server"
version = "0.1.0"
description = "一个简单的 Web 服务器"
authors = ["开发者 <dev@example.com>"]
license = "MIT"

[dependencies]
http = "1.0.0"
json = "2.0.0"
router = { version = "0.5.0", path = "./router" }

[dev-dependencies]
test-utils = "1.0.0"
benchmark = "0.1.0"
```
