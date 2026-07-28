---
title: 'RFC-014: Дизайн системы управления пакетами'
status: 'Принято'
author: 'Чэньсюй'
created: '2026-02-12'
updated: '2026-06-11'
group: 'rfc-014' # Этот RFC является общим планом системы управления пакетами, под-RFC: 014a/014b/014c
issue: '#88'
impl: '48%'
impl_status: 'частично'
---

# RFC-014: Дизайн системы управления пакетами (общий план)

> **Под-RFC:**
>
> - [RFC-014a: Спецификация протокола Registry](../draft/014a-registry-protocol.md)
> - [RFC-014b: Система сборки и распространения бинарников](../draft/014b-build-system.md)
> - [RFC-014c: Поддержка workspace](../draft/014c-workspace.md)

## Резюме

Проектирование системы управления пакетами языка YaoXiang с поддержкой семантического
версионирования, локальных и GitHub-зависимостей, унифицированного синтаксиса импорта,
конфигурационного файла `yaoxiang.toml` и файла блокировки `yaoxiang.lock`.

## Мотивация

### Почему необходима эта функциональность/изменение?

Управление пакетами является инфраструктурой экосистемы современных языков программирования. В
настоящее время язык YaoXiang не имеет:

- механизма объявления зависимостей
- возможностей управления версиями
- стандартных каналов распространения

### Текущая проблема

```
my-project/
├── src/
│   └── main.yx          # 代码依赖其他模块
├── lib/                  # 手动复制的模块
│   ├── foo.yx
│   └── bar.yx
└── ???                   # 没有标准依赖管理
```

## Предложение

### Основной дизайн

**Многоуровневая архитектура:**

```
┌─────────────────────────────────────────────┐
│           Resolution Engine                  │ ← 依赖解析
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│            Global Cache                      │ ← ~/.yaoxiang/cache/
└─────────────────┬───────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│              Source Trait                    │ ← 可扩展源
├──────────┬──────────┬──────────┬────────────┤
│  Local   │   Git    │ Registry │   GitHub   │
│  (本地)  │  (VCS)   │  (开放)  │ (Release)  │
└──────────┴──────────┴──────────┴────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────┐
│           Vendor Directory                   │ ← .yaoxiang/vendor/
└─────────────────────────────────────────────┘
```

**Механизм расширения**: для добавления нового типа Source достаточно реализовать trait, без
необходимости изменения движка разрешения.

### Пример

```bash
# 1. 创建项目
yaoxiang init my-project

# 2. 编辑 yaoxiang.toml 添加依赖
[dependencies]
foo = "^1.0.0"
bar = { git = "https://github.com/user/bar", version = "0.5.0" }

# 3. 安装依赖
yaoxiang add foo

# 4. 代码中使用
use foo;
use bar.baz;
```

### Структура проекта

```
my-project/
├── yaoxiang.toml        # 包配置
├── yaoxiang.lock        # 锁定文件（自动生成）
├── src/
│   └── main.yx
└── .yaoxiang/
    └── vendor/              # 本地依赖
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
foo = "1.2.3"           # 精确版本
bar = "^1.0.0"          # 兼容版本
baz = "~1.2.0"          # 补丁版本
qux = { git = "...", version = "0.5.0" }
local_pkg = { path = "./local-module" }

[dev-dependencies]
test-utils = "0.1.0"

[build]
strategy = "none"       # none | cargo | cmake | custom

[binaries]
"linux-x86_64" = { url = "...", sha256 = "..." }

[workspace.members]     # 仅工作空间根
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

Порядок разрешения зависит от рабочего режима (наличие `yaoxiang.toml`).

#### Режим проекта (с yaoxiang.toml)

```
use foo.bar.baz;

查找顺序:
0. 嵌入二进制                          (std/*.yx — 编译时嵌入，版本绑定，仅 std.* 命名空间)
1. ./.yaoxiang/std/foo/bar/baz.yx     (项目级标准库 — 存在时全局标准库完全失效)
2. ./.yaoxiang/vendor/*/src/foo/bar/baz.yx  (vendor/)
3. ./src/foo/bar/baz.yx                       (本地模块)
4. ~/.yaoxiang/cache/foo/<ver>/src/foo/bar/baz.yx  (全局缓存)
5. $YXPATH/foo/bar/baz.yx                     (全局路径，预留)
```

**Правила режима проекта**:

- Встроенный бинарник действует только для пространства имён `std.*`, имеет наивысший приоритет
- При наличии проектной стандартной библиотеки (`.yaoxiang/std/`) глобальная стандартная библиотека
  полностью пропускается — обеспечение детерминированности сборки
- Проект может управлять стандартной библиотекой как зависимостью через `yaoxiang add std@1.0.1`,
  фиксируя версию

#### Режим одного файла (без yaoxiang.toml)

```
use foo.bar.baz;

查找顺序:
0. 嵌入二进制                          (std/*.yx — 编译时嵌入，版本绑定)
1. <yaoxiang-install-dir>/yx/<version>/std/foo/bar/baz.yx  (全局标准库)
2. ./src/foo/bar/baz.yx                                       (本地模块)
3. $YXPATH/foo/bar/baz.yx                                     (全局路径，预留)
```

**Правила режима одного файла**:

- Нет проектных зависимостей, стандартная библиотека загружается напрямую из глобального пути
- Глобальный путь стандартной библиотеки привязан к версии компилятора:
  `<install-dir>/yx/<version>/std/`

### Структура каталога установки стандартной библиотеки

#### Глобальная стандартная библиотека

```
<yaoxiang-install-dir>/
├── yx/                          # YaoXiang 语言目录
│   ├── 1.0.1/                   # 版本目录
│   │   ├── std/
│   │   │   ├── test.yx          # 纯 YaoXiang 标准库模块
│   │   │   ├── math.yx          # 未来自举模块
│   │   │   └── ...
│   │   └── ...
│   └── 1.1.0/
│       └── std/
│           └── ...
└── bin/
    └── yaoxiang                 # 编译器二进制
```

#### Проектная стандартная библиотека

Проект может добавить стандартную библиотеку как зависимость через `yaoxiang add std@1.0.1`,
хранящуюся в `.yaoxiang/std/`:

```
my-project/
├── yaoxiang.toml
├── yaoxiang.lock
├── .yaoxiang/
│   ├── std/                     # 项目级标准库（存在时全局标准库失效）
│   │   ├── test.yx
│   │   ├── math.yx
│   │   └── ...
│   └── vendor/                  # 其他依赖
│       └── ...
├── src/
│   └── main.yx
```

**Ключевые моменты дизайна**:

- Встроенный бинарник как уровень совместимости: пока стандартная библиотека на файловой системе не
  полностью реализована, модули стандартной библиотеки сначала предоставляются через встроенный
  бинарник
- Изоляция каталогов версий: `yx/<version>/std/` позволяет сосуществовать разным версиям стандартной
  библиотеки без взаимного влияния
- Проектная стандартная библиотека перекрывает глобальную: обеспечение детерминированности сборки,
  независимо от изменений глобального окружения
- При отсутствии yaoxiang.toml (режим одного файла) происходит откат на глобальную стандартную
  библиотеку
- Наличие `.yaoxiang/std/` означает "проектная стандартная библиотека включена", глобальная
  стандартная библиотека больше не участвует

### Основные структуры данных

```rust
// 依赖来源（可扩展）
enum Source {
    Local { path: PathBuf },
    Git { url: Url, version: Option<VersionConstraint> },
    Registry { registry: String, namespace: Option<String> },
    GitHub { owner: String, repo: String, ref_: GitRef },  // GitHub 原生
}

enum GitRef {
    Tag(String),
    Branch(String),
    Rev(String),
    DefaultBranch,
}

// 依赖声明
enum DependencySpec {
    Version(VersionConstraint),
    Git { url: Url, version: Option<VersionConstraint> },
    Local { path: PathBuf },
    Workspace { member: String },  // 工作空间成员引用
}

// 解析后的依赖
struct ResolvedDependency {
    name: String,
    version: Version,
    source: Source,
    integrity: Option<String>,
    checksum: Option<String>,  // SHA-256
}

// 构建策略
enum BuildStrategy {
    None,          // 纯 .yx 包
    Cargo,         // 调用 cargo build
    Cmake,         // 调用 cmake
    Custom,        // 执行 build.yx 脚本
    Precompiled,   // 直接用预编译产物
}
```

### Дизайн команд CLI

Применяется унифицированный подход, объединяющий компилятор, пакетный менеджер и REPL в единый
инструмент CLI:

#### Режим одного файла vs Режим проекта

| Команда                     | Один файл | Режим проекта | Описание                         |
| --------------------------- | --------- | ------------- | -------------------------------- |
| `yaoxiang run <file>`       | ✅        | ✅            | Запуск файла/точки входа проекта |
| `yaoxiang build`            | ❌        | ✅            | Сборка проекта                   |
| `yaoxiang build <file>`     | ✅        | ✅            | Сборка одного файла              |
| `yaoxiang init <name>`      | ❌        | ✅            | Создание проекта                 |
| `yaoxiang add <dep>`        | ❌        | ✅            | Добавление зависимости           |
| `yaoxiang update`           | ❌        | ✅            | Обновление зависимостей          |
| `yaoxiang fmt`              | ✅        | ✅            | Форматирование                   |
| `yaoxiang check`            | ✅        | ✅            | Проверка типов                   |
| `yaoxiang` (без параметров) | ✅        | ✅            | Прямой вход в REPL               |

#### Подробное описание команд

| Команда                            | Функция                                     | Пример                                               |
| ---------------------------------- | ------------------------------------------- | ---------------------------------------------------- |
| `yaoxiang`                         | Прямой вход в REPL                          | `yaoxiang`                                           |
| `yaoxiang run <file>`              | Запуск одного файла/проекта                 | `yaoxiang run main.yx`                               |
| `yaoxiang init <name>`             | Создание нового проекта                     | `yaoxiang init my-app`                               |
| `yaoxiang build`                   | Сборка проекта                              | `yaoxiang build`                                     |
| `yaoxiang build <file>`            | Сборка одного файла                         | `yaoxiang build foo.yx`                              |
| `yaoxiang add <dep>`               | Добавление зависимости                      | `yaoxiang add foo`                                   |
| `yaoxiang add -D <dep>`            | Добавление dev-зависимости                  | `yaoxiang add -D test`                               |
| `yaoxiang rm <dep>`                | Удаление зависимости                        | `yaoxiang rm foo`                                    |
| `yaoxiang update`                  | Обновление всех зависимостей                | `yaoxiang update`                                    |
| `yaoxiang update foo`              | Обновление указанной зависимости            | `yaoxiang update foo`                                |
| `yaoxiang install`                 | Установка всех зависимостей                 | `yaoxiang install`                                   |
| `yaoxiang list`                    | Список зависимостей                         | `yaoxiang list`                                      |
| `yaoxiang outdated`                | Проверка устаревших зависимостей            | `yaoxiang outdated`                                  |
| `yaoxiang fmt`                     | Форматирование кода                         | `yaoxiang fmt`                                       |
| `yaoxiang check`                   | Проверка типов                              | `yaoxiang check`                                     |
| `yaoxiang clean`                   | Очистка артефактов сборки                   | `yaoxiang clean`                                     |
| `yaoxiang task <name>`             | Запуск пользовательской задачи              | `yaoxiang task lint`                                 |
| `yaoxiang publish`                 | Публикация пакета в Registry                | `yaoxiang publish`                                   |
| `yaoxiang publish --github`        | Публикация и создание GitHub Release        | `yaoxiang publish --github`                          |
| `yaoxiang yank <pkg>@<ver>`        | Удаление опубликованной версии (необратимо) | `yaoxiang yank foo@1.2.3`                            |
| `yaoxiang login --registry <url>`  | Аутентификация в Registry                   | `yaoxiang login --registry https://reg.example.com`  |
| `yaoxiang login --github`          | Аутентификация в GitHub                     | `yaoxiang login --github`                            |
| `yaoxiang logout --registry <url>` | Выход                                       | `yaoxiang logout --registry https://reg.example.com` |
| `yaoxiang cache clean`             | Очистка глобального кэша                    | `yaoxiang cache clean`                               |
| `yaoxiang workspace <cmd>`         | Операции с workspace                        | `yaoxiang workspace list`                            |

#### Описание ограничений команд

```bash
# 单文件模式：不需要 yaoxiang.toml
yaoxiang run hello.yx   # ✅ 正常工作
yaoxiang add foo        # ❌ 报错：不是项目目录

# 项目模式：需要 yaoxiang.toml
cd my-project
yaoxiang run main.yx    # ✅ 运行入口文件
yaoxiang build          # ✅ 构建项目
yaoxiang add foo        # ✅ 添加依赖
```

### Обратная совместимость

- ✅ Существующий синтаксис `use` полностью сохранён
- ✅ Существующая логика разрешения модулей не изменяется
- ✅ Новый каталог `.yaoxiang/vendor` не влияет на существующие проекты

### Глобальный кэш

Все загруженные зависимости кэшируются в `~/.yaoxiang/cache/`, каталог vendor проекта копируется из
кэша.

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
- Git-зависимости: кэшируются по tag/rev, если tag не изменён — не устаревает
- `yaoxiang cache clean` ручная очистка

### Аутентификация

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

- Приоритет переменных среды: `$YX_GITHUB_TOKEN`, `$YX_REGISTRY_TOKEN`
- Токен никогда не записывается в `yaoxiang.toml` или `yaoxiang.lock`
- Права доступа к файлу 600

### Семантика yank

`yaoxiang yank foo@1.2.3` выполняет **удаление + блокировку номера версии**:

- Пакет полностью удаляется, восстановление невозможно
- Номер версии навсегда заблокирован, повторная публикация того же номера версии невозможна
- Проекты с существующими lockfile, ссылающимися на эту версию, будут получать ошибку, требуется
  обновление
- **Цель безопасности**: предотвращение атак на цепочку поставок в стиле npm (злоумышленник
  перехватывает удалённый номер версии для внедрения вредоносного кода)

### Протокол Registry

См. подробности в [RFC-014a: Спецификация протокола Registry](../draft/014a-registry-protocol.md).

Основной дизайн: открытый протокол + уровень адаптера. Официальный Registry как основной, GitHub
Release/main ветка как вспомогательный, поддержка пользовательских Registry.

### Система сборки

См. подробности в
[RFC-014b: Система сборки и распространения бинарников](../draft/014b-build-system.md).

Основной дизайн: декларативная конфигурация `[build]`, приоритет предкомпиляции/исходный код как
запасной вариант, поддержка стратегий cargo/cmake/custom.

### Workspace

См. подробности в [RFC-014c: Поддержка workspace](../draft/014c-workspace.md).

Основной дизайн: объявление members в форме словаря, общий lockfile, зависимости по путям,
интеграция с Cargo workspace.

## Компромиссы

### Преимущества

- Унифицированный синтаксис импорта, пользователю не нужно думать об источнике зависимости
- Детерминированная сборка, файл блокировки гарантирует согласованность сборки
- Поддержка офлайн, после загрузки возможна автономная разработка
- Source trait удобен для последующего расширения

### Недостатки

- Требуется дополнительное дисковое пространство (каталог `.yaoxiang/vendor`)
- Конфликты версий требуют ручного разрешения пользователем

## Альтернативы

| Подход                                    | Почему не выбран                                                 |
| ----------------------------------------- | ---------------------------------------------------------------- |
| Прямой доступ к GitHub в реальном времени | Безопасность и повторное использование кэша трудно гарантировать |
| Глобальный кэш ($HOME/.yaoxiang)          | Плохая изоляция, сложные конфликты версий                        |
| Поддержка только реестра                  | GitHub — ведущая платформа для хостинга кода в настоящее время   |

## Стратегия реализации

### Разбивка по фазам

| Фаза          | Содержание                                                             | Статус       |
| ------------- | ---------------------------------------------------------------------- | ------------ |
| **Phase 1**   | Парсинг toml, локальные зависимости, генерация lock, базовые алгоритмы | ✅ Завершено |
| **Phase 2**   | Поддержка GitHub, управление `.yaoxiang/vendor`, инструменты загрузки  | ✅ Завершено |
| **Phase 3**   | Глобальный кэш, замена `semver` crate, доработка CLI                   | Ожидается    |
| **Phase 3.5** | Изменение Source trait на async, интеграция `async-trait`              | Ожидается    |
| **Phase 4**   | Протокол Registry, publish, auth (RFC-014a)                            | Ожидается    |
| **Phase 5**   | Система сборки, предкомпилированные бинарники (RFC-014b)               | Ожидается    |
| **Phase 6**   | Поддержка workspace (RFC-014c)                                         | Ожидается    |

### Зависимости

- Нет предварительных зависимостей
- Требует интеграции с `ModuleGraph` (`middle/passes/module/`)

### Риски

| Риск                                        | Меры по снижению                                                          |
| ------------------------------------------- | ------------------------------------------------------------------------- |
| Сложность алгоритма разрешения зависимостей | Сначала реализовать простую версию, затем добавить обнаружение конфликтов |
| Нестабильность загрузки Git                 | Механизмы повторных попыток и кэширования                                 |
| Проблемы с производительностью              | Ленивая загрузка, инкрементальное разрешение                              |

## Открытые вопросы

- [x] Условный синтаксис компиляции для `dev-dependencies`? → Унифицировано системой сборки в
      RFC-014b
- [x] Алгоритм проверки целостности (SHA-256 / BLAKE3)? → SHA-256
- [ ] Исключение определённых файлов из загрузки через `excludes`?
- [ ] Соглашения об именовании пакетов (поддержка namespace, например `@org/pkg`)?
- [ ] Стратегия версионирования API Registry?

---

## Зависимости (для добавления в Cargo.toml)

| Назначение                    | crate            | Описание                   |
| ----------------------------- | ---------------- | -------------------------- |
| Семантическое версионирование | `semver`         | Замена самописного парсера |
| HTTP-клиент                   | `reqwest`        | Коммуникация с Registry    |
| SHA-256                       | `sha2`           | Проверка целостности       |
| Сжатие                        | `flate2` + `tar` | Обработка формата пакета   |

---

## Ссылки

- [Cargo Dependency Resolution](https://doc.rust-lang.org/cargo/)
- [Go Modules](https://go.dev/ref/mod)
- [PEP 440: Version Identification](https://peps.python.org/pep-0440/)
