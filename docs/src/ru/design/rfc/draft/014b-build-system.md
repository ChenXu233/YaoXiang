```markdown
---
title: "RFC-014b: Система сборки и распространение бинарных файлов"
status: "Черновик"
author: "Чэньсюй"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
---

# RFC-014b: Система сборки и распространение бинарных файлов

> Данный RFC является под-RFC к [RFC-014: Проектирование системы управления пакетами](../accepted/014-package-manager.md).

## Резюме

Определяет механизмы сборки системы управления пакетами YaoXiang: декларативная конфигурация сборки, стратегии сборки (cargo/cmake/custom/none), распространение прекомпилированных бинарных файлов, проверка системных зависимостей.

## Мотивация

Некоторые пакеты представляют собой чистый код `.yx` и не требуют сборки. Другие требуют компиляции FFI-привязок (с вызовом Cargo, CMake и т.д.). Необходим единый механизм, позволяющий авторам пакетов декларировать требования к сборке, а менеджеру пакетов — автоматически их обрабатывать.

### Текущие проблемы

- Отсутствие декларации конфигурации сборки (в `yaoxiang.toml` нет секции `[build]`)
- Отсутствие механизма распространения прекомпилированных бинарных файлов
- Сборка FFI-пакетов полностью зависит от ручных действий пользователя
- Отсутствие проверки системных зависимостей

## Предложение

### Основной дизайн: декларативная сборка + приоритет прекомпиляции

Автор пакета декларирует требования к сборке в `yaoxiang.toml`, а менеджер пакетов автоматически принимает решения на основе этой декларации.

### Стратегии сборки

```rust
enum BuildStrategy {
    None,          // 纯 .yx 包，无需构建
    Cargo,         // 调用 cargo build
    Cmake,         // 调用 cmake
    Custom,        // 执行 build.yx 脚本
    Precompiled,   // 直接用预编译产物
}
```

### Декларация сборки в yaoxiang.toml

```toml
[package]
name = "native-foo"
version = "1.0.0"

[build]
strategy = "cargo"              # 构建策略
script = "build.yx"            # 仅 strategy = "custom" 时使用

[build.cargo]
features = ["ffi"]             # cargo build --features ffi
target = "release"             # cargo build --release

[build.requirements]
cargo = ">= 1.70"              # 构建时需要的工具
cmake = ">= 3.20"

[build.platforms]              # 平台特定覆盖
"linux-x86_64" = { cargo-features = ["linux-ffi"] }
"windows-x86_64" = { cargo-features = ["win-ffi"] }
"aarch64-apple-darwin" = { cargo-features = ["mac-ffi"] }
```

### Дерево решений при установке

```
yaoxiang install foo
    │
    ├─ 1. 查 Registry/GitHub Release 有无当前平台预编译产物？
    │     → 有：下载，校验 SHA-256，直接安装（跳过构建）
    │     → 无：继续
    │
    ├─ 2. 下载源码包
    │
    ├─ 3. 读 yaoxiang.toml 的 [build] 段
    │     → strategy = "none"：直接安装
    │     → 其他：检查 requirements，执行构建
    │
    └─ 4. 安装到 vendor/
```

**Прекомпиляция в приоритете, исходный код — в качестве резерва.** Аналогично подходу wheel vs sdist в Python.

### Декларация прекомпилированных бинарных файлов

```toml
# yaoxiang.toml
[binaries]
"linux-x86_64" = { url = "releases/download/v1.0.0/foo-linux-x64.tar.gz", sha256 = "abc123" }
"windows-x86_64" = { url = "releases/download/v1.0.0/foo-win-x64.tar.gz", sha256 = "def456" }
"aarch64-apple-darwin" = { url = "releases/download/v1.0.0/foo-mac-arm.tar.gz", sha256 = "ghi789" }
```

**Условия пропуска сборки:**

1. В `[binaries]` есть запись для текущей платформы
2. Проверка SHA-256 прошла успешно
3. Загрузка выполнена успешно

Все три условия выполнены → пропустить сборку. Иначе → откат на сборку из исходного кода.

### Сборочный скрипт build.yx

Выполняется при `strategy = "custom"`:

```yx
# build.yx — 包的构建脚本
use std.os
use std.io

fn main() {
    let platform = os.platform()       # "linux", "windows", "macos"
    let arch = os.arch()               # "x86_64", "aarch64"

    if os.file_exists("Cargo.toml") {
        io.println("Building native extension via Cargo...")
        os.exec("cargo build --release")

        os.copy(
            "target/release/libfoo.so",
            "build/native/${platform}-${arch}/libfoo.so"
        )
    }

    io.println("Build complete!")
}
```

### Проверка системных зависимостей

Перед установкой автоматически проверяются все `[build.requirements]`; при неудовлетворении выдаётся ошибка:

```
Error: Build requirement not satisfied
  cargo >= 1.70 required, but cargo is not installed
  Install: https://rustup.rs
```

### Интеграция с Cargo Workspace

Если пакет содержит FFI-код, можно одновременно определить Cargo workspace:

```
my-package/
├── yaoxiang.toml          # YaoXiang 包配置
├── Cargo.toml             # Cargo workspace（FFI 部分）
├── src/
│   └── lib.yx             # YaoXiang 代码
└── native/
    ├── Cargo.toml          # Rust FFI 代码
    └── src/
        └── lib.rs
```

`yaoxiang build` автоматически обнаруживает это и вызывает `cargo build` для компиляции нативной части.

## Детальный дизайн

### Идентификация платформ

Используется формат `{os}-{arch}`:

| Идентификатор платформы | OS | Arch |
|----------|----|------|
| `linux-x86_64` | Linux | x86_64 |
| `linux-aarch64` | Linux | ARM64 |
| `windows-x86_64` | Windows | x86_64 |
| `aarch64-apple-darwin` | macOS | ARM64 (Apple Silicon) |
| `x86_64-apple-darwin` | macOS | x86_64 |

### Структура каталогов артефактов сборки

```
build/
└── native/
    ├── linux-x86_64/
    │   └── libfoo.so
    ├── windows-x86_64/
    │   └── foo.dll
    └── aarch64-apple-darwin/
        └── libfoo.dylib
```

### Полный жизненный цикл прекомпилированного пакета

```
开发者：
  1. 写 .yx 代码 + FFI 绑定
  2. 在 yaoxiang.toml 声明 [build] + [binaries]
  3. yaoxiang publish
     → 自动在 CI 上构建多平台二进制
     → 上传源码 + 预编译产物

用户：
  yaoxiang add native-foo
    → 检测到有预编译产物 → 直接下载（秒级）
    → 没有预编译产物 → 下载源码 + 执行构建（分钟级）
```

## Компромиссы

### Преимущества

- Декларативная конфигурация — пользователю не нужно разбираться в деталях сборки
- Приоритет прекомпиляции — установка выполняется очень быстро
- Поддержка нескольких платформ с автоматическим выбором
- Бесшовная интеграция с экосистемой Cargo

### Недостатки

- Для прекомпилированных артефактов требуется поддержка CI
- Многоплатформенная сборка усложняет публикацию
- Для скриптов build.yx необходим механизм песочницы

## Альтернативы

| Альтернатива | Почему не выбрана |
|------|-----------|
| Распространение только исходного кода | Пользователю нужно устанавливать инструментарий сборки — высокий порог входа |
| Бинарный формат наподобие Python wheel | Слишком сложно, на раннем этапе экосистемы YaoXiang не требуется |
| Отсутствие поддержки сборки FFI | Ограничивает возможности расширения языка |

## Стратегия реализации

### Разделение на этапы

| Этап | Содержание |
|------|------|
| Phase 5a | Разбор конфигурации `[build]` + перечисление `BuildStrategy` |
| Phase 5b | Проверка системных зависимостей |
| Phase 5c | Интеграция сборки Cargo |
| Phase 5d | Загрузка прекомпилированных бинарных файлов + проверка |
| Phase 5e | Выполнение скрипта build.yx |

### Зависимости

- Зависит от RFC-014a (протокол Registry, для загрузки прекомпилированных артефактов)
- Зависит от crate `sha2` (проверка целостности)

## Открытые вопросы

- [ ] Нужна ли песочница для скриптов build.yx?
- [ ] Каков максимально допустимый размер артефактов сборки?
- [ ] Поддерживать ли кросс-компиляцию (сборку Windows-артефактов на Linux)?
- [ ] Как поступать при несовместимости версий Cargo?

## Ссылки

- [Rust build.rs](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Python wheels](https://packaging.python.org/en/latest/guides/distributing-packages-using-setuptools/#wheels)
- [Go build constraints](https://pkg.go.dev/cmd/go#hdr-Build_constraints)
```