---
title: "RFC-014: Дизайн системы управления пакетами"
status: "Принят"
author: "晨煦"
created: "2026-02-12"
updated: "2026-06-11"
group: "rfc-014"  # Настоящий RFC является общим документом для системы управления пакетами; дочерние RFC: 014a/014b/014c
issue: "#88"
impl: "48%"
impl_status: "partial"
---

# RFC-014: Дизайн системы управления пакетами (общий документ)

> **Дочерние RFC:**
> - [RFC-014a: Спецификация протокола Registry](../draft/014a-registry-protocol.md)
> - [RFC-014b: Система сборки и распространения бинарных файлов](../draft/014b-build-system.md)
> - [RFC-014c: Поддержка рабочих пространств](../draft/014c-workspace.md)

## Резюме

Проектирование системы управления пакетами для языка YaoXiang с поддержкой семантического версионирования, локальных зависимостей и зависимостей из GitHub, унифицированного синтаксиса импорта, файла конфигурации `yaoxiang.toml` и файла блокировки `yaoxiang.lock`.

## Мотивация

### Зачем нужна эта функциональность/изменение?

Управление пакетами — базовая инфраструктура экосистемы современного языка программирования. В настоящее время в языке YaoXiang отсутствуют:
- Механизм объявления зависимостей
- Возможности управления версиями
- Стандартные каналы распространения

### Текущая проблема

```
my-project/
├── src/
│   └── main.yx          # Код зависит от других модулей
├── lib/                  # Модули, скопированные вручную
│   ├── foo.yx
│   └── bar.yx
└── ???                   # Нет стандартного управления зависимостями
```

## Предложение

### Основной дизайн

**Многоуровневая архитектура**:
```
┌─────────────────────────────────────────────┐
│           Resolution Engine                  │ ← Разрешение зависимостей
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│            Global Cache                      │ ← ~/.yaoxiang/cache/
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│              Source Trait                    │ ← Расширяемые источники
├──────────┬──────────┬──────────┬────────────┤
│  Local   │   Git    │ Registry │   GitHub   │
│ (Локальный) │  (VCS)  │ (Открытый) │ (Release)│
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
├── yaoxiang.toml        # Конфигурация пакета
├── yaoxiang.lock        # Файл блокировки (генерируется автоматически)
├── src/
│   └── main.yx
└── .yaoxiang/
    └── vendor/              # Локальные зависимости
        ├── foo-1.2.3/
        └── bar-0.5.0/
```

## Подробный дизайн

### Формат файла конфигурации

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
foo = "1.2.3"           # Точная версия
bar = "^1.0.0"          # Совместимая версия
baz = "~1.2.0"          # Версия с патчами
qux = { git = "...", version = "0.5.0" }
local_pkg = { path = "./local-module" }

[dev-dependencies]
test-utils = "0.1.0"

[build]
strategy = "none"       # none | cargo | cmake | custom

[binaries]
"linux-x86_64" = { url = "...", sha256 = "..." }

[workspace.members]     # Только для корня рабочего пространства
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
    GitHub { owner: String, repo: String, ref_: GitRef },  // Собственный GitHub
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
    Workspace { member: String },  // Ссылка на члена рабочего пространства
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
    None,          // Чистый пакет .yx
    Cargo,         // Вызов cargo build
    Cmake,         // Вызов cmake
    Custom,        // Выполнение скрипта build.yx
    Precompiled,   // Использование предкомпилированного артефакта напрямую
}
```

### Дизайн CLI-команд

Применяется единый подход, объединяющий компилятор, менеджер пакетов и REPL в один CLI-инструмент:

#### Однофайловый режим vs проектный режим

| Команда | Однофайловый | Проектный | Описание |
|------|--------|---------|------|
| `yaoxiang run <file>` | ✅ | ✅ | Запуск файла/точки входа проекта |
| `yaoxiang build` | ❌ | ✅ | Сборка проекта |
| `yaoxiang build <file>` | ✅ | ✅ | Сборка отдельного файла |
| `yaoxiang init <name>` | ❌ | ✅ | Создание проекта |
| `yaoxiang add <dep>` | ❌ | ✅ | Добавление зависимости |
| `yaoxiang update` | ❌ | ✅ | Обновление зависимостей |
| `yaoxiang fmt` | ✅ | ✅ | Форматирование |
| `yaoxiang check` | ✅ | ✅ | Проверка типов |
| `yaoxiang` (без аргументов) | ✅ | ✅ | Прямой вход в REPL |

#### Подробное описание команд

| Команда | Функция | Пример |
|------|------|------|
| `yaoxiang` | Прямой вход в REPL | `yaoxiang` |
| `yaoxiang run <file>` | Запуск одного файла/проекта | `yaoxiang run main.yx` |
| `yaoxiang init <name>` | Создание нового проекта | `yaoxiang init my-app` |
| `yaoxiang build` | Сборка проекта | `yaoxiang build` |
| `yaoxiang build <file>` | Сборка отдельного файла | `yaoxiang build foo.yx` |
| `yaoxiang add <dep>` | Добавление зависимости | `yaoxiang add foo` |
| `yaoxiang add -D <dep>` | Добавление dev-зависимости | `yaoxiang add -D test` |
| `yaoxiang rm <dep>` | Удаление зависимости | `yaoxiang rm foo` |
| `yaoxiang update` | Обновление всех зависимостей | `yaoxiang update` |
| `yaoxiang update foo` | Обновление указанной зависимости | `yaoxiang update foo` |
| `yaoxiang install` | Установка всех зависимостей | `yaoxiang install` |
| `yaoxiang list` | Список зависимостей | `yaoxiang list` |
| `yaoxiang outdated` | Проверка устаревших зависимостей | `yaoxiang outdated` |
| `yaoxiang fmt` | Форматирование кода | `yaoxiang fmt` |
| `yaoxiang check` | Проверка типов | `yaoxiang check` |
| `yaoxiang clean` | Очистка артефактов сборки | `yaoxiang clean` |
| `yaoxiang task <name>` | Запуск пользовательской задачи | `yaoxiang task lint` |
| `yaoxiang publish` | Публикация пакета в Registry | `yaoxiang publish` |
| `yaoxiang publish --github` | Публикация и создание GitHub Release | `yaoxiang publish --github` |
| `yaoxiang yank <pkg>@<ver>` | Удаление опубликованной версии (необратимо) | `yaoxiang yank foo@1.2.3` |
| `yaoxiang login --registry <url>` | Аутентификация в Registry | `yaoxiang login --registry https://reg.example.com` |
| `yaoxiang login --github` | Аутентификация в GitHub | `yaoxiang login --github` |
| `yaoxiang logout --registry <url>` | Выход | `yaoxiang logout --registry https://reg.example.com` |
| `yaoxiang cache clean` | Очистка глобального кэша | `yaoxiang cache clean` |
| `yaoxiang workspace <cmd>` | Операции с рабочим пространством | `yaoxiang workspace list` |

#### Описание ограничений команд

```bash
# Однофайловый режим: yaoxiang.toml не требуется
yaoxiang run hello.yx   # ✅ Работает нормально
yaoxiang add foo        # ❌ Ошибка: не директория проекта

# Проектный режим: требуется yaoxiang.toml
cd my-project
yaoxiang run main.yx    # ✅ Запуск файла точки входа
yaoxiang build          # ✅ Сборка проекта
yaoxiang add foo        # ✅ Добавление зависимости
```

### Обратная совместимость

- ✅ Существующий синтаксис `use` полностью сохранён
- ✅ Существующая логика разрешения модулей не изменяется
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
- Пакеты Registry: номер версии неизменяем, кэш не устаревает
- Git-зависимости: кэшируются по tag/rev, если tag не меняется — кэш остаётся
- Ручная очистка: `yaoxiang cache clean`

### Аутентификация

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

- Приоритет переменных окружения: `$YX_GITHUB_TOKEN`, `$YX_REGISTRY_TOKEN`
- Токены никогда не записываются в `yaoxiang.toml` или `yaoxiang.lock`
- Права доступа к файлу: 600

### Семантика yank

Выполнение `yaoxiang yank foo@1.2.3` означает **удаление + блокировку номера версии**:

- Пакет полностью удаляется, восстановление невозможно
- Номер версии блокируется навсегда, повторная публикация той же версии невозможна
- Проекты, в существующих lockfile которых есть ссылка на эту версию, будут выдавать ошибку и потребуют обновления
- **Цель безопасности**: предотвращение атак на цепочку поставок в стиле npm (когда злоумышленник перехватывает контроль над удалённым номером версии для внедрения вредоносного кода)

### Протокол Registry

Подробности см. в [RFC-014a: Спецификация протокола Registry](../draft/014a-registry-protocol.md).

Основной дизайн: открытый протокол + адаптер. Официальный Registry как основной, GitHub Release/main ветка как вспомогательный, поддержка пользовательских Registry.

### Система сборки

Подробности см. в [RFC-014b: Система сборки и распространения бинарных файлов](../draft/014b-build-system.md).

Основной дизайн: декларативная конфигурация `[build]`, приоритет предкомпилированных бинарных файлов с резервным вариантом в виде исходного кода, поддержка стратегий cargo/cmake/custom.

### Рабочее пространство

Подробности см. в [RFC-014c: Поддержка рабочих пространств](../draft/014c-workspace.md).

Основной дизайн: объявление members в виде словаря, общий lockfile, зависимости по путям, интеграция с Cargo workspace.

## Компромиссы

### Преимущества

- Унифицированный синтаксис импорта, пользователю не нужно заботиться об источнике зависимости
- Детерминированная сборка, lock-файл гарантирует согласованность сборки
- Поддержка офлайн, после загрузки возможна автономная разработка
- Source trait упрощает дальнейшее расширение

### Недостатки

- Требуется дополнительное дисковое пространство (директория .yaoxiang/vendor)
- Конфликты версий требуют ручного разрешения пользователем

## Альтернативы

| Альтернатива | Почему не выбрана |
|------|-----------|
| Прямой доступ к GitHub в реальном времени | Сложно гарантировать безопасность и повторное использование кэша |
| Только глобальный кэш ($HOME/.yaoxiang) | Плохая изоляция, сложные конфликты версий |
| Поддержка только реестра | GitHub — текущая основная платформа для хостинга кода |

## Стратегия реализации

### Разделение на этапы

| Этап | Содержание | Статус |
|------|------|------|
| **Фаза 1** | Парсинг toml, локальные зависимости, генерация lock, базовые алгоритмы | ✅ Завершено |
| **Фаза 2** | Поддержка GitHub, управление .yaoxiang/vendor, инструмент загрузки | ✅ Завершено |
| **Фаза 3** | Глобальный кэш, замена semver crate, доработка CLI | Ожидает начала |
| **Фаза 3.5** | Перевод Source trait на async, интеграция async-trait | Ожидает начала |
| **Фаза 4** | Протокол Registry, publish, auth (RFC-014a) | Ожидает начала |
| **Фаза 5** | Система сборки, предкомпилированные бинарные файлы (RFC-014b) | Ожидает начала |
| **Фаза 6** | Поддержка рабочих пространств (RFC-014c) | Ожидает начала |

### Зависимости

- Нет предварительных зависимостей
- Требуется интеграция с `ModuleGraph` (`middle/passes/module/`)

### Риски

| Риск | Меры по смягчению |
|------|----------|
| Сложность алгоритма разрешения зависимостей | Сначала простая версия, затем добавление обнаружения конфликтов |
| Нестабильность загрузки из Git | Механизмы повторных попыток и кэширования |
| Проблемы с производительностью | Ленивая загрузка, инкрементальное разрешение |

## Открытые вопросы

- [x] Синтаксис условной компиляции для `dev-dependencies`? → Обрабатывается системой сборки в RFC-014b
- [x] Алгоритм проверки целостности (SHA-256 / BLAKE3)? → SHA-256
- [ ] `excludes` для исключения определённых файлов из загрузки?
- [ ] Соглашения об именовании пакетов (поддержка namespace, например `@org/pkg`)?
- [ ] Стратегия версионирования Registry API?

---

## Зависимости (необходимо добавить в Cargo.toml)

| Назначение | crate | Описание |
|------|-------|------|
| Семантическое версионирование | `semver` | Замена самописного парсера |
| HTTP-клиент | `reqwest` | Связь с Registry |
| SHA-256 | `sha2` | Проверка целостности |
| Сжатие | `flate2` + `tar` | Обработка формата пакетов |

---

## Ссылки

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)