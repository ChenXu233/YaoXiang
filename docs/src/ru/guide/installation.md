---
title: 'Установка YaoXiang'
description:
  'Двухуровневые каналы установки — стандартный канал «распакуй и используй» (режим Go/Zig),
  упрощённый канал в одну строку + управление версиями (режим Rust/rustup)'
---

# Установка YaoXiang

Командный интерфейс YaoXiang использует **разделение front-door/engine** (RFC-037):

- **`yx`** — front-door, повседневная точка входа. Встроенное управление версиями (`yx toolchain` /
  `yx self update`), остальные команды передаются движку
- **`yaoxiang-rs`** — движок, отвечает за компиляцию/запуск/управление пакетами/форматирование/LSP,
  как правило не вызывается напрямую

Установка осуществляется через двухуровневые каналы; все каналы используют общую структуру продуктов
(`bin/` + `lib/yaoxiang/std/`).

## Стандартный канал: распакуй и используй (режим Go/Zig)

Загрузите архив для соответствующей платформы со
[страницы релизов GitHub](https://github.com/ChenXu233/YaoXiang/releases), распакуйте в любой
каталог и добавьте `bin/` в PATH:

```sh
tar xzf yaoxiang-<версия>-<платформа>.tar.gz
export PATH="$PWD/yaoxiang-<версия>-<платформа>/bin:$PATH"
```

После распаковки можно сразу использовать: `yx --version`. Архив содержит все зависимости (включая
разделяемую библиотеку Z3) и читаемые исходники стандартной библиотеки `lib/yaoxiang/std/`, никаких
дополнительных действий не требуется.

## Упрощённый канал: одна команда (режим Rust/rustup)

### Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.ps1 | iex
```

Скрипт устанавливает тулчейн в `~/.yaoxiang/` (для Windows: `%USERPROFILE%\.yaoxiang`) и добавляет
`yx` в PATH.

### Linux: репозиторий apt

```sh
curl -fsSL <адрес apt-репозитория>/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/yaoxiang.gpg
echo "deb [signed-by=/usr/share/keyrings/yaoxiang.gpg] <адрес apt-репозитория> stable main" | sudo tee /etc/apt/sources.list.d/yaoxiang.list
sudo apt update && sudo apt install yaoxiang
```

Установка через apt выполняется на системном уровне (`/usr/lib/yaoxiang/`, точка входа
`/usr/bin/yx`), обновление версий происходит через `apt upgrade`. Метаданные репозитория
автоматически подписываются и публикуются релизным CI.

### Windows: мастер установки Inno Setup

Загрузите `YaoXiang-Setup-<версия>.exe` со
[страницы релизов](https://github.com/ChenXu233/YaoXiang/releases) и установите с помощью мастера
(опционально с автоматическим добавлением в PATH). Аналогично apt — системная установка.

## Управление версиями (встроено в front-door `yx`)

```sh
yx toolchain install stable    # установить последнюю стабильную версию
yx toolchain install 0.7.14    # установить указанную версию
yx toolchain default 0.7.14    # задать версию по умолчанию
yx toolchain list              # список установленных версий
yx toolchain update            # обновить до последней стабильной версии
yx self update                 # обновить сам `yx`
```

Несколько версий сосуществуют в `~/.yaoxiang/versions/<версия>/`, каждая версия — это полное
самодостаточное дерево тулчейна (движок, Z3, стандартная библиотека зафиксированы одной версией).

### Закрепление версии на уровне проекта

Поместите в корень проекта файл `yx-toolchain.toml` (по аналогии с `rust-toolchain.toml`):

```toml
toolchain = "0.7.14"
```

Любая команда `yx`, выполненная в этом каталоге, будет использовать закреплённую версию — разные
проекты могут работать с разными версиями.

## Переменные окружения

| Переменная         | Описание                                                                           |
| ------------------ | ---------------------------------------------------------------------------------- |
| `YAOXIANG_HOME`    | Переопределяет корень установки, по умолчанию `~/.yaoxiang` (для CI и контейнеров) |
| `YAOXIANG_VERSION` | Скрипт install устанавливает указанную версию вместо последней                     |

Зеркало для загрузки можно настроить в `~/.yaoxiang/settings.toml` (префикс в стиле ghproxy):

```toml
mirror = "https://ghproxy.example.com"
```

## Проверка установки

```sh
yx --version        # версия front-door
yx run main.yx      # любая программа на YaoXiang
```
