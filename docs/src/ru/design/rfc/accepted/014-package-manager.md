---
title: "RFC-014: Дизайн системы управления пакетами"
status: "Принято"
author: "Чэньсюй"
created: "2026-02-12"
updated: "2026-06-11"
group: "rfc-014"  # Данный RFC является общим документом для системы управления пакетами, под-RFC: 014a/014b/014c
---

# RFC-014: Дизайн системы управления пакетами (общий документ)

> **Под-RFC:**
> - [RFC-014a: Спецификация протокола Registry](../draft/014a-registry-protocol.md)
> - [RFC-014b: Система сборки и распространение бинарных файлов](../draft/014b-build-system.md)
> - [RFC-014c: Поддержка рабочего пространства](../draft/014c-workspace.md)

## Резюме

Дизайн системы управления пакетами языка YaoXiang с поддержкой семантического версионирования, локальных зависимостей и зависимостей с GitHub, унифицированного синтаксиса импорта, конфигурационного файла `yaoxiang.toml` и файла блокировки `yaoxiang.lock`.

## Мотивация

### Зачем нужна эта функциональность/изменение?

Управление пакетами — это инфраструктура, лежащая в основе экосистемы современных языков программирования. В настоящее время в языке YaoXiang отсутствуют:
- Механизм объявления зависимостей
- Возможности управления версиями
- Стандартные каналы распространения

### Текущие проблемы

```
my-project/
├── src/
│   └── main.yx          # код зависит от других модулей
├── lib/                  # модули, скопированные вручную
│   ├── foo.yx
│   └── bar.yx
└── ???                   # нет стандартного управления зависимостями
```

## Предложение

### Основной дизайн

**Многоуровневая архитектура**:
```
┌─────────────────────────────────────────────┐
│           Resolution Engine                  │ ← разрешение зависимостей
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│            Global Cache                      │ ← ~/.yaoxiang/cache/
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│              Source Trait                    │ ← расширяемые источники
├──────────┬──────────┬──────────┬────────────┤
│  Local   │   Git    │ Registry │   GitHub   │
│ (локал.) │  (VCS)   │ (открыт.)│ (Release)  │
└──────────┴──────────┴──────────┴────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│           Vendor Directory                   │ ← .yaoxiang/vendor/
└─────────────────────────────────────────────┘
```

**Механизм расширения**: для добавления нового типа Source достаточно реализовать trait, без изменения движка разрешения.

### Пример

```bash
# 1. Создание проекта
yaoxiang init my-project

# 2. Редактирование yaoxiang.toml для добавления зависимостей
[dependencies]
foo = "^1.0.0"
bar = { git = "https://github.com/user/bar", version = "0.5.0" }

# 3. Установка зависимостей
yaoxiang add foo

# 4. Использование в коде
use foo;
use bar.baz;
```

### Структура проекта

```
my-project/
├── yaoxiang.toml        # конфигурация пакета
├── yaoxiang.lock        # файл блокировки (генерируется автоматически)
├── src/
│   └── main.yx
└── .yaoxiang/
    └── vendor/              # локальные зависимости
        ├── foo-1.2.3/
        └── bar-0.5.0/
```

## Детальный дизайн

### Формат конфигурационного файла

**yaoxiang.toml**:
```toml
[package]
name = "my-package"
version = "0.1.0"
description = "A short description"
license = "MIT"
authors = ["Your Name <you@example.com>"]
repository = "https://github.com/you/my-package"
keywords = ["cli", "utility"]

[dependencies]
foo = "1.2.3"           # точная версия
bar = "^1.0.0"          # совместимая версия
baz = "~1.2.0"          # версия с патчами
qux = { git = "...", version = "0.5.0" }
local_pkg = { path = "./local-module" }

[dev-dependencies]
test-utils = "0.1.0"

[build]
strategy = "none"       # none | cargo | cmake | custom

[binaries]
"linux-x86_64" = { url = "...", sha256 = "..." }

[workspace.members]     # только в корне рабочего пространства
core = "packages/core/yaoxiang.toml"
```

**yaoxiang.lock**:
```toml
version = 1

[[package]]
name = "foo"
version = "1.2.3"
source = "git"
resolved = "https://github.com/user/foo?tag=v1.2.3"
integrity = "sha256-xxxx"
```

### Порядок разрешения модулей

```
use foo.bar.baz;

Порядок поиска:
1. ./.yaoxiang/vendor/*/src/foo/bar/baz.yx  (vendor/)
2. ./src/foo/bar/baz.yx                     (локальный модуль)
3. ~/.yaoxiang/cache/foo/<ver>/src/foo/bar/baz.yx  (глобальный кэш)
4. $YXPATH/foo/bar/baz.yx                   (глобальный путь, зарезервировано)
5. $YXLIB/std/foo/bar/baz.yx                (стандартная библиотека)
```

### Основные структуры данных

```rust
// Источник зависимости (расширяемый)
enum Source {
    Local { path: PathBuf },
    Git { url: Url, version: Option<VersionConstraint> },
    Registry { registry: String, namespace: Option<String> },
    GitHub { owner: String, repo: String, ref_: GitRef },  // нативный GitHub
}

enum GitRef {
    Tag(String),
    Branch(String),
    Rev(String),
    DefaultBranch,
}

// Объявление зависимости
enum DependencySpec {
    Version(VersionConstraint),
    Git { url: Url, version: Option<VersionConstraint> },
    Local { path: PathBuf },
    Workspace { member: String },  // ссылка на член рабочего пространства
}

// Разрешённая зависимость
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
    checksum: Option<String>,  // SHA-256
}

// Стратегия сборки
enum BuildStrategy {
    None,          // чистый .yx пакет
    Cargo,         # вызов cargo build
    Cmake,         # вызов cmake
    Custom,        // выполнение скрипта build.yx
    Precompiled,   // непосредственное использование прекомпилированного продукта
}
```

### Дизайн CLI-команд

Используется унифицированный подход, объединяющий компилятор, менеджер пакетов и REPL в единый CLI-инструмент:

#### Режим одиночного файла vs. режим проекта

| Команда | Одиночный файл | Режим проекта | Описание |
|------|--------|---------|------|
| `yaoxiang run <file>` | ✅ | ✅ | запуск файла/точки входа проекта |
| `yaoxiang build` | ❌ | ✅ | сборка проекта |
| `yaoxiang build <file>` | ✅ | ✅ | сборка одиночного файла |
| `yaoxiang init <name>` | ❌ | ✅ | создание проекта |
| `yaoxiang add <dep>` | ❌ | ✅ | добавление зависимости |
| `yaoxiang update` | ❌ | ✅ | обновление зависимостей |
| `yaoxiang fmt` | ✅ | ✅ | форматирование |
| `yaoxiang check` | ✅ | ✅ | проверка типов |
| `yaoxiang` (без аргументов) | ✅ | ✅ | прямой вход в REPL |

#### Подробное описание команд

| Команда | Функция | Пример |
|------|------|------|
| `yaoxiang` | прямой вход в REPL | `yaoxiang` |
| `yaoxiang run <file>` | запуск одиночного файла/проекта | `yaoxiang run main.yx` |
| `yaoxiang init <name>` | создание нового проекта | `yaoxiang init my-app` |
| `yaoxiang build` | сборка проекта | `yaoxiang build` |
| `yaoxiang build <file>` | сборка одиночного файла | `yaoxiang build foo.yx` |
| `yaoxiang add <dep>` | добавление зависимости | `yaoxiang add foo` |
| `yaoxiang add -D <dep>` | добавление dev-зависимости | `yaoxiang add -D test` |
| `yaoxiang rm <dep>` | удаление зависимости | `yaoxiang rm foo` |
| `yaoxiang update` | обновление всех зависимостей | `yaoxiang update` |
| `yaoxiang update foo` | обновление указанной зависимости | `yaoxiang update foo` |
| `yaoxiang install` | установка всех зависимостей | `yaoxiang install` |
| `yaoxiang list` | список зависимостей | `yaoxiang list` |
| `yaoxiang outdated` | проверка устаревших зависимостей | `yaoxiang outdated` |
| `yaoxiang fmt` | форматирование кода | `yaoxiang fmt` |
| `yaoxiang check` | проверка типов | `yaoxiang check` |
| `yaoxiang clean` | очистка продуктов сборки | `yaoxiang clean` |
| `yaoxiang task <name>` | выполнение пользовательской задачи | `yaoxiang task lint` |
| `yaoxiang publish` | публикация пакета в Registry | `yaoxiang publish` |
| `yaoxiang publish --github` | публикация с созданием GitHub Release | `yaoxiang publish --github` |
| `yaoxiang yank <pkg>@<ver>` | удаление опубликованной версии (необратимо) | `yaoxiang yank foo@1.2.3` |
| `yaoxiang login --registry <url>` | аутентификация в Registry | `yaoxiang login --registry https://reg.example.com` |
| `yaoxiang login --github` | аутентификация в GitHub | `yaoxiang login --github` |
| `yaoxiang logout --registry <url>` | выход | `yaoxiang logout --registry https://reg.example.com` |
| `yaoxiang cache clean` | очистка глобального кэша | `yaoxiang cache clean` |
| `yaoxiang workspace <cmd>` | операции с рабочим пространством | `yaoxiang workspace list` |

#### Описание ограничений команд

```bash
# Режим одиночного файла: yaoxiang.toml не требуется
yaoxiang run hello.yx   # ✅ работает нормально
yaoxiang add foo        # ❌ ошибка: это не директория проекта

# Режим проекта: требуется yaoxiang.toml
cd my-project
yaoxiang run main.yx    # ✅ запуск файла точки входа
yaoxiang build          # ✅ сборка проекта
yaoxiang add foo        # ✅ добавление зависимости
```

### Обратная совместимость

- ✅ Существующий синтаксис `use` полностью сохранён
- ✅ Существующая логика разрешения модулей не изменена
- ✅ Добавление директории .yaoxiang/vendor не влияет на существующие проекты

### Глобальный кэш

Все загруженные зависимости кэшируются в `~/.yaoxiang/cache/`, директория vendor проекта копируется из кэша.

```
~/.yaoxiang/
├── cache/
│   ├── registry/
│   │   └── foo-1.2.3/
│   ├── git/
│   │   └── github.com-user-bar-abc123/
│   └── binaries/
│       └── foo-1.2.3-linux-x86_64.tar.gz
├── credentials.toml
└── config.toml
```

```toml
# ~/.yaoxiang/config.toml
[cache]
dir = "~/.yaoxiang/cache"
max_size = "2GB"
ttl = "30d"
```

Правила инвалидации кэша:
- Пакеты Registry: номер версии неизменяем, никогда не устаревает
- Git-зависимости: кэшируются по tag/rev, если tag не изменён — кэш валиден
- `yaoxiang cache clean` — ручная очистка

### Аутентификация

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

- Приоритет у переменных окружения: `$YX_GITHUB_TOKEN`, `$YX_REGISTRY_TOKEN`
- Токен никогда не записывается в `yaoxiang.toml` или `yaoxiang.lock`
- Права доступа к файлу 600

### Семантика yank

Команда `yaoxiang yank foo@1.2.3` выполняет **удаление + блокировку номера версии**:

- Пакет полностью удаляется, восстановление невозможно
- Номер версии занят навсегда, повторная публикация с тем же номером версии невозможна
- Проекты, в существующих lockfile которых имеется ссылка на эту версию, будут выдавать ошибку и требовать обновления
- **Цель безопасности**: предотвращение атак на цепочку поставок в стиле npm (когда злоумышленник перехватывает удалённый номер версии и внедряет вредоносный код)

### Протокол Registry

Подробности см. в [RFC-014a: Спецификация протокола Registry](../draft/014a-registry-protocol.md).

Основной дизайн: открытый протокол + адаптерный слой. Официальный Registry является основным, GitHub Release/main ветка — вспомогательным, поддерживаются пользовательские Registry.

### Система сборки

Подробности см. в [RFC-014b: Система сборки и распространение бинарных файлов](../draft/014b-build-system.md).

Основной дизайн: декларативная конфигурация `[build]`, приоритет прекомпиляции/исходный код как запасной вариант, поддержка стратегий cargo/cmake/custom.

### Рабочее пространство

Подробности см. в [RFC-014c: Поддержка рабочего пространства](../draft/014c-workspace.md).

Основной дизайн: объявление members в виде словаря, общий lockfile, зависимости по путям, интеграция с Cargo workspace.

## Компромиссы

### Преимущества

- Унифицированный синтаксис импорта, пользователю не нужно заботиться об источнике зависимости
- Детерминированная сборка, файл блокировки гарантирует согласованность сборки
- Поддержка офлайн, после загрузки возможна автономная разработка
- Trait Source облегчает последующее расширение

### Недостатки

- Требуется дополнительное дисковое пространство (директория .yaoxiang/vendor)
- Конфликты версий требуют ручного разрешения пользователем

## Альтернативные варианты

| Вариант | Почему не выбран |
|------|-----------|
| Обращение к GitHub в реальном времени | Безопасность и переиспользование кэша трудно гарантировать |
| Только глобальный кэш ($HOME/.yaoxiang) | Плохая изоляция, сложные конфликты версий |
| Поддержка только реестра | GitHub — текущая主流 платформа для хостинга кода |

## Стратегия реализации

### Этапы

| Этап | Содержание | Статус |
|------|------|------|
| **Phase 1** | парсинг toml, локальные зависимости, генерация lock, базовые алгоритмы | ✅ Завершено |
| **Phase 2** | поддержка GitHub, управление .yaoxiang/vendor, инструменты загрузки | ✅ Завершено |
| **Phase 3** | глобальный кэш, замена semver crate, доработка CLI | К началу |
| **Phase 3.5** | Source trait переход на async, интеграция async-trait | К началу |
| **Phase 4** | протокол Registry, publish, auth (RFC-014a) | К началу |
| **Phase 5** | система сборки, прекомпилированные бинарники (RFC-014b) | К началу |
| **Phase 6** | поддержка рабочего пространства (RFC-014c) | К началу |

### Зависимости

- Без предварительных зависимостей
- Требуется интеграция с `ModuleGraph` (`middle/passes/module/`)

### Риски

| Риск | Меры по снижению |
|------|----------|
| Сложный алгоритм разрешения зависимостей | Сначала реализовать простую версию, затем добавить обнаружение конфликтов |
| Нестабильность загрузки из Git | Механизмы повтора и кэширования |
| Проблемы с производительностью | Ленивая загрузка, инкрементальное разрешение |

## Открытые вопросы

- [x] Синтаксис условной компиляции для `dev-dependencies`? → Обрабатывается системой сборки RFC-014b
- [x] Алгоритм проверки целостности (SHA-256 / BLAKE3)? → SHA-256
- [ ] `excludes` — исключение определённых файлов из загрузки?
- [ ] Соглашение об именовании пакетов (поддержка namespace, например `@org/pkg`)?
- [ ] Стратегия версионирования Registry API?

---

## Зависимости (необходимо добавить в Cargo.toml)

| Назначение | crate | Описание |
|------|-------|------|
| Семантическое версионирование | `semver` | замена ручного парсера |
| HTTP-клиент | `reqwest` | взаимодействие с Registry |
| SHA-256 | `sha2` | проверка целостности |
| Сжатие | `flate2` + `tar` | обработка формата пакетов |

---

## Ссылки

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)