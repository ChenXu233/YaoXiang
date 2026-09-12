---
title: 'Установка YaoXiang'
description: 'Двухуровневые каналы установки — стандартный канал: распакуй и используй (режим Go/Zig); упрощённый канал: одна команда + управление версиями (режим Rust/rustup)'
---

# Установка YaoXiang

Командный интерфейс YaoXiang построен по принципу **разделения на фронтенд/движок** (RFC-037):

- **`yx`** — фронтенд, повседневная точка входа. Встроенное управление версиями (`yx toolchain` /
  `yx self update`), остальные команды прозрачно передаются движку
- **`yaoxiang-rs`** — движок, отвечает за компиляцию/запуск/управление пакетами/форматирование/LSP,
  обычно не вызывается напрямую

Установка разделена на два канала, все каналы используют общую структуру артефактов (`bin/` +
`lib/yaoxiang/std/`).

## Стандартный канал: распакуй и используй (режим Go/Zig)

Загрузите архив для вашей платформы со
[страницы релизов GitHub](https://github.com/ChenXu233/YaoXiang/releases), распакуйте в любой
каталог и добавьте `bin/` в PATH:

```sh
tar xzf yaoxiang-<版本>-<平台>.tar.gz
export PATH="$PWD/yaoxiang-<版本>-<平台>/bin:$PATH"
```

Сразу после распаковки можно использовать: `yx --version`. Архив содержит все зависимости (включая
разделяемую библиотеку Z3) и читаемый исходный код стандартной библиотеки `lib/yaoxiang/std/`,
никаких дополнительных шагов не требуется.

## Упрощённый канал: одна команда (режим Rust/rustup)

### Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.ps1 | iex
```

Скрипт установит инструментарий в `~/.yaoxiang/` (для Windows — `%USERPROFILE%\.yaoxiang`) и добавит
`yx` в PATH.

### Linux: репозиторий apt

```sh
curl -fsSL <apt 仓库地址>/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/yaoxiang.gpg
echo "deb [signed-by=/usr/share/keyrings/yaoxiang.gpg] <apt 仓库地址> stable main" | sudo tee /etc/apt/sources.list.d/yaoxiang.list
sudo apt update && sudo apt install yaoxiang
```

Установка через apt выполняется на системном уровне (`/usr/lib/yaoxiang/`, точка входа команды —
`/usr/bin/yx`), `apt upgrade` следует за версиями. Метаданные репозитория автоматически
подписываются и публикуются CI релизов.

### Windows: мастер установки Inno Setup

Загрузите `YaoXiang-Setup-<版本>.exe` со
[страницы релизов](https://github.com/ChenXu233/YaoXiang/releases) и установите с помощью мастера
(опционально с автоматическим добавлением в PATH). Как и apt, выполняет системную установку.

## Управление версиями (встроено во фронтенд yx)

```sh
yx toolchain install stable    # 安装最新稳定版
yx toolchain install 0.7.14    # 安装指定版本
yx toolchain default 0.7.14    # 设置默认版本
yx toolchain list              # 列出已安装版本
yx toolchain update            # 升级到最新稳定版
yx self update                 # 更新 yx 本体
```

Несколько версий сосуществуют в `~/.yaoxiang/versions/<версия>/`, каждая версия — это полное
самодостаточное дерево инструментария (движок, Z3, стандартная библиотека заблокированы на одной
версии).

### Закрепление версии на уровне проекта

Поместите файл `yx-toolchain.toml` в корень проекта (по аналогии с `rust-toolchain.toml`):

```toml
toolchain = "0.7.14"
```

Любая команда `yx`, выполненная в этом каталоге, будет использовать закреплённую версию — разные
проекты могут работать на разных версиях.

## Переменные среды

| Переменная         | Описание                                                                           |
| ------------------ | ---------------------------------------------------------------------------------- |
| `YAOXIANG_HOME`    | Переопределяет корень установки, по умолчанию `~/.yaoxiang` (для CI и контейнеров) |
| `YAOXIANG_VERSION` | Скрипт install устанавливает указанную версию вместо последней                     |

Загрузку через зеркало можно настроить в `~/.yaoxiang/settings.toml` (префикс в стиле ghproxy):

```toml
mirror = "https://ghproxy.example.com"
```

## Проверка установки

```sh
yx --version        # 前门版本
yx run main.yx      # 任意 YaoXiang 程序
```
