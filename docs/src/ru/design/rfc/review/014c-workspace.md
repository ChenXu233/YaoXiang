---
title: "RFC-014c: Поддержка рабочих пространств"
status: "На рассмотрении"
author: "晨煦"
created: "2026-06-11"
updated: "2026-07-05"
group: "rfc-014"
issue: "#113"
---

# RFC-014c: Поддержка рабочих пространств

> Данный RFC является поддокументом [RFC-014: Дизайн системы управления пакетами](../accepted/014-package-manager.md).

## Краткое описание

Определение механизма рабочих пространств (workspace) в YaoXiang: общий доступ к зависимостям при совместной разработке нескольких связанных пакетов, ссылки на пути, унифицированный lockfile и интеграция с Cargo workspace.

## Мотивация

Когда проект растёт, код необходимо разделять на несколько пакетов. Эти пакеты должны:

- ссылаться друг на друга (зависимости через пути)
- совместно использовать версии внешних зависимостей (во избежание дрейфа версий)
- иметь унифицированный lockfile (для обеспечения согласованности сборки)
- интегрироваться с Cargo workspace (для FFI)

### Текущие проблемы

- Каждый проект независимо управляет зависимостями, без возможности совместного использования
- Отсутствует механизм автоматической замены path-зависимостей при публикации
- Отсутствует интеграция с Cargo workspace

## Предложение

### Основной дизайн: Координирующий слой + самодостаточные члены

Корневое workspace выполняет только координацию, каждый член полностью самодостаточен.

### Корневой yaoxiang.toml

```toml
# Корневой yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**Корневой toml выполняет только три действия:**

1. Объявляет список членов (в виде словаря, key — имя члена, value — путь к toml)
2. Предоставляет общий lockfile (`yaoxiang.lock`)
3. Предоставляет общую директорию vendor (`.yaoxiang/vendor/`)

**Корневой toml не определяет dependencies.** Зависимости каждого члена записываются в его собственном `yaoxiang.toml`.

### yaoxiang.toml члена

```toml
# packages/core/yaoxiang.toml
[package]
name = "core"
version = "0.1.0"

[dependencies]
json = "^2.0.0"
utils = { workspace = "utils" }    # ссылка на члена рабочего пространства
regex = "^1.0.0"
```

```toml
# packages/utils/yaoxiang.toml
[package]
name = "utils"
version = "0.2.0"

[dependencies]
regex = "^1.0.0"
```

### Структура рабочего пространства

```
my-workspace/
├── yaoxiang.toml              # Корневая конфигурация рабочего пространства
├── yaoxiang.lock              # Общий lockfile
├── .yaoxiang/
│   └── vendor/                # Общая директория vendor
├── packages/
│   ├── core/
│   │   ├── yaoxiang.toml      # Конфигурация члена пакета
│   │   └── src/lib.yx
│   ├── utils/
│   │   ├── yaoxiang.toml
│   │   └── src/lib.yx
│   └── app/
│       ├── yaoxiang.toml
│       └── src/main.yx
└── Cargo.toml                 # Опционально: общее Cargo workspace (FFI)
```

### Разрешение зависимостей

- Каждый член читает свой собственный раздел `[dependencies]`
- При разрешении зависимости всех членов объединяются, генерируется общий lockfile
- Конфликты версий обнаруживаются при генерации lockfile
- Один и тот же пакет в разных членах должен разрешаться в одну и ту же версию

### Ссылки на зависимости workspace

`{ workspace = "member-name" }` ссылается на **key** из `[workspace.members]` (не на `[package].name` члена).

```toml
# Корневой yaoxiang.toml
[workspace.members]
utils = "packages/utils/yaoxiang.toml"    # key = "utils"
```

```toml
# packages/app/yaoxiang.toml
[package]
name = "app"

[dependencies]
utils = { workspace = "utils" }   # ✅ Ссылка на key "utils"
# Даже если в packages/utils/yaoxiang.toml указано name = "my-utils"
```

**Почему используется key, а не name:**

- key контролируется рабочим пространством, стабилен и уникален
- `[package].name` — это публичное имя, которое может измениться при публикации
- key является ключом BTreeMap, естественно уникален
- При публикации ссылки workspace заменяются на версионные зависимости, key не попадает в публичный API

### Path-зависимости и публикация

При разработке используются ссылки рабочего пространства:

```toml
[dependencies]
utils = { workspace = "utils" }
```

При публикации автоматически заменяются на версионные зависимости:

```toml
[dependencies]
utils = "^0.2.0"
```

**Источник версии:** читается `[package].version` зависимого члена, добавляется префикс `^`. Проверка Registry не выполняется — авторитетным источником версии является yaoxiang.toml члена, Registry — лишь канал дистрибуции.

Менеджер пакетов автоматически выполняет эту замену при `yaoxiang publish`.

### Интеграция с Cargo Workspace

Если в рабочем пространстве есть FFI-пакеты, можно одновременно определить Cargo workspace:

```toml
# Корневой Cargo.toml
[workspace]
members = ["packages/core/native", "packages/utils/native"]
```

```
my-workspace/
├── yaoxiang.toml          # YaoXiang workspace
├── Cargo.toml             # Cargo workspace (часть FFI)
├── packages/
│   ├── core/
│   │   ├── src/lib.yx     # YaoXiang код
│   │   └── native/
│   │       ├── Cargo.toml # Rust FFI код
│   │       └── src/lib.rs
│   └── utils/
│       ├── src/lib.yx
│       └── native/
│           ├── Cargo.toml
│           └── src/lib.rs
```

`yaoxiang build` автоматически обнаруживает и вызывает `cargo build` для компиляции native-части.

### CLI-команды

| Команда | Функция |
|------|------|
| `yaoxiang workspace list` | Вывести список членов рабочего пространства |
| `yaoxiang workspace add <path>` | Добавить члена |
| `yaoxiang workspace remove <name>` | Удалить члена |
| `yaoxiang build` | Собрать всех членов (в порядке топологической сортировки зависимостей) |
| `yaoxiang build core` | Собрать указанного члена |
| `yaoxiang test` | Запустить тесты всех членов |

**Поведение `yaoxiang build`:** собирает всех членов в порядке топологической сортировки зависимостей. Если core → utils → app, порядок сборки: core → utils → app.

## Детальный дизайн

### Структура WorkspaceManifest

Корневой toml использует отдельный тип `WorkspaceManifest`, не переиспользуя `PackageManifest`:

```rust
struct WorkspaceManifest {
    workspace: WorkspaceConfig,
}

struct WorkspaceConfig {
    members: BTreeMap<String, String>,  // key -> путь к toml
}

struct Workspace {
    root: PathBuf,
    manifest: WorkspaceManifest,
    members: Vec<WorkspaceMember>,
    lock: LockFile,
}

struct WorkspaceMember {
    name: String,           // key из [workspace.members]
    root: PathBuf,
    manifest: PackageManifest,
}
```

**Логика обнаружения:** при загрузке toml, если присутствует секция `[workspace]`, парсится как `WorkspaceManifest`, иначе — как `PackageManifest`.

### Ссылки на зависимости workspace

Семантика `{ workspace = "member-name" }`:

- В `dependencies` ссылается на другого члена рабочего пространства
- При разработке разрешается в локальный путь
- При публикации заменяется на версию из Registry
- Имя члена должно существовать в `[workspace.members]`

### Совместное использование lockfile

- Рабочее пространство имеет только один `yaoxiang.lock` (в корневой директории)
- Зависимости всех членов объединяются при разрешении в один lockfile
- Конфликты версий обнаруживаются при генерации lockfile с информацией об источниках конфликта

## Компромиссы

### Преимущества

- Унифицированное управление мультипакетными проектами
- Общий lockfile обеспечивает согласованность
- Хороший опыт разработки с path-зависимостями
- Бесшовная интеграция с Cargo workspace

### Недостатки

- Все члены должны использовать одинаковые версии внешних зависимостей (может быть слишком строго)
- Корневой toml не может иметь собственных зависимостей (ограничение дизайна)
- Интеграция с Cargo workspace добавляет сложности

## Альтернативные решения

| Решение | Почему не выбрано |
|------|-----------|
| Независимые проекты + path-зависимости | lockfile не унифицирован, риск дрейфа версий |
| Похоже на npm workspaces | Проблемы npm workspaces многочисленны, не стоит копировать |
| Прямое переиспользование Cargo workspace | YaoXiang и Cargo — разные экосистемы пакетов |

## Стратегия реализации

### Разделение на фазы

| Фаза | Содержание |
|------|------|
| Phase 6a | Парсинг `[workspace.members]` + WorkspaceManifest |
| Phase 6b | Общий lockfile + объединённое разрешение зависимостей |
| Phase 6c | `{ workspace = "name" }` ссылка на path-зависимость |
| Phase 6d | Автоматическая замена path-зависимостей при публикации |
| Phase 6e | Интеграция с Cargo workspace |

### Зависимости

- Зависит от RFC-014 Phase 3 (глобальный кэш)
- Опциональная зависимость от RFC-014b (система сборки, для native-членов)

## Открытые вопросы

- [ ] Допускаются ли циклические зависимости между членами?
- [ ] Поддерживается ли workspace-уровневая конфигурация `[build]`?
- [ ] Могут ли члены иметь собственные lockfile (переопределяющие корневой)?
- [ ] Поддерживаются ли вложенные workspace?

---

## Ссылки

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)