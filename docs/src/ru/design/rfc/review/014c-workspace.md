---
title: 'RFC-014c: Поддержка рабочего пространства'
status: 'На рассмотрении'
author: '晨煦'
created: '2026-06-11'
updated: '2026-07-05'
group: 'rfc-014'
issue: '#113'
---

# RFC-014c: Поддержка рабочего пространства

> Данный RFC является под-RFC к
> [RFC-014: Проектирование системы управления пакетами](../accepted/014-package-manager.md).

## Резюме

Определяет механизм рабочего пространства (workspace) в YaoXiang: совместное использование
зависимостей, путевые ссылки, единый lockfile, интеграция с Cargo workspace при совместной
разработке нескольких связанных пакетов.

## Мотивация

По мере роста проекта код необходимо разбивать на несколько пакетов. Эти пакеты должны:

- ссылаться друг на друга (путевые зависимости)
- совместно использовать версии внешних зависимостей (во избежание расхождения версий)
- иметь единый lockfile (для обеспечения согласованности сборки)
- взаимодействовать с Cargo workspace (для FFI-частей)

### Текущие проблемы

- Каждый проект управляет зависимостями независимо, совместное использование невозможно
- Нет механизма автоматической замены путевых зависимостей при публикации
- Нет интеграции с Cargo workspace

## Предложение

### Основная концепция: координационный слой + самодостаточные участники

Корневой workspace выполняет только координацию, каждый участник полностью самодостаточен.

### Корневой yaoxiang.toml

```toml
# Корневой yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**Корневой toml выполняет только три задачи:**

1. Объявление списка участников (в виде словаря, где key — имя участника, value — путь к toml)
2. Предоставление общего lockfile (`yaoxiang.lock`)
3. Предоставление общего каталога vendor (`.yaoxiang/vendor/`)

**Корневой toml не определяет dependencies.** Зависимости каждого участника записываются в его
собственном `yaoxiang.toml`.

### yaoxiang.toml участника

```toml
# packages/core/yaoxiang.toml
[package]
name = "core"
version = "0.1.0"

[dependencies]
json = "^2.0.0"
utils = { workspace = "utils" }    # Ссылка на участника рабочего пространства
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
├── yaoxiang.toml              # Конфигурация корня рабочего пространства
├── yaoxiang.lock              # Общий lockfile
├── .yaoxiang/
│   └── vendor/                # Общий каталог vendor
├── packages/
│   ├── core/
│   │   ├── yaoxiang.toml      # Конфигурация пакета-участника
│   │   └── src/lib.yx
│   ├── utils/
│   │   ├── yaoxiang.toml
│   │   └── src/lib.yx
│   └── app/
│       ├── yaoxiang.toml
│       └── src/main.yx
└── Cargo.toml                 # Опционально: общий Cargo workspace (для FFI)
```

### Разрешение зависимостей

- Каждый участник читает свой собственный `[dependencies]`
- При разрешении зависимости всех участников объединяются, генерируется общий lockfile
- Конфликты версий сообщаются при генерации lockfile
- Один и тот же пакет у разных участников должен разрешаться в одну и ту же версию

### Ссылки на зависимости в workspace

`{ workspace = "member-name" }` ссылается на **key** в `[workspace.members]` (а не на
`[package].name` участника).

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
- key является ключом BTreeMap, по своей природе уникален
- При публикации ссылка workspace заменяется версионной зависимостью, key не утекает в публичный API

### Путевые зависимости и публикация

При разработке используется ссылка на рабочее пространство:

```toml
[dependencies]
utils = { workspace = "utils" }
```

При публикации автоматически заменяется на версионную зависимость:

```toml
[dependencies]
utils = "^0.2.0"
```

**Источник версии:** читается `[package].version` зависимого участника, добавляется префикс `^`.
Registry не проверяется — авторитетным источником версии является `yaoxiang.toml` участника,
Registry — лишь канал распространения.

Менеджер пакетов автоматически выполняет эту замену при выполнении `yaoxiang publish`.

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
├── Cargo.toml             # Cargo workspace (FFI-часть)
├── packages/
│   ├── core/
│   │   ├── src/lib.yx     # Код YaoXiang
│   │   └── native/
│   │       ├── Cargo.toml # Код Rust FFI
│   │       └── src/lib.rs
│   └── utils/
│       ├── src/lib.yx
│       └── native/
│           ├── Cargo.toml
│           └── src/lib.rs
```

`yaoxiang build` автоматически обнаруживает и вызывает `cargo build` для компиляции native-части.

### Команды CLI

| Команда                            | Функция                                                         |
| ---------------------------------- | --------------------------------------------------------------- |
| `yaoxiang workspace list`          | Список участников рабочего пространства                         |
| `yaoxiang workspace add <path>`    | Добавить участника                                              |
| `yaoxiang workspace remove <name>` | Удалить участника                                               |
| `yaoxiang build`                   | Собрать всех участников (в топологическом порядке зависимостей) |
| `yaoxiang build core`              | Собрать указанного участника                                    |
| `yaoxiang test`                    | Запустить тесты всех участников                                 |

**Поведение `yaoxiang build`:** собирает всех участников в топологическом порядке зависимостей. Если
core → utils → app, то порядок сборки: core → utils → app.

## Подробное проектирование

### Структура WorkspaceManifest

Корневой toml использует отдельный тип `WorkspaceManifest`, а не повторно использует
`PackageManifest`:

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

**Логика обнаружения:** при загрузке toml, если присутствует секция `[workspace]`, она разбирается
как `WorkspaceManifest`, иначе — как `PackageManifest`.

### Ссылки на зависимости в workspace

Семантика `{ workspace = "member-name" }`:

- в `dependencies` ссылается на другого участника рабочего пространства
- при разработке разрешается в локальный путь
- при публикации заменяется на версию из Registry
- имя участника должно присутствовать в `[workspace.members]`

### Совместное использование lockfile

- В рабочем пространстве только один `yaoxiang.lock` (в корневом каталоге)
- Разрешение зависимостей всех участников объединяется в один lockfile
- Конфликты версий сообщаются при генерации lockfile с указанием источника конфликта

## Компромиссы

### Преимущества

- Единое управление многопакетными проектами
- Общий lockfile обеспечивает согласованность
- Удобство работы с путевыми зависимостями при разработке
- Бесшовная интеграция с Cargo workspace

### Недостатки

- Все участники должны использовать одну и ту же версию внешней зависимости (может быть слишком
  строго)
- Корневой toml не может иметь собственных зависимостей (ограничение дизайна)
- Интеграция с Cargo workspace увеличивает сложность

## Альтернативы

| Альтернатива                                   | Почему не выбрана                                   |
| ---------------------------------------------- | --------------------------------------------------- |
| Независимые проекты + path-зависимости         | lockfile не объединён, риск расхождения версий      |
| Подобно npm workspaces                         | У npm много проблем с workspace, не стоит подражать |
| Прямое повторное использование Cargo workspace | YaoXiang и Cargo — разные экосистемы пакетов        |

## Стратегия реализации

### Этапы

| Этап     | Содержание                                                |
| -------- | --------------------------------------------------------- |
| Phase 6a | Разбор `[workspace.members]` + WorkspaceManifest          |
| Phase 6b | Общий lockfile + объединённое разрешение зависимостей     |
| Phase 6c | Путевые ссылки на зависимости `{ workspace = "name" }`    |
| Phase 6d | Автоматическая замена путевых зависимостей при публикации |
| Phase 6e | Интеграция с Cargo workspace                              |

### Зависимости

- Зависит от RFC-014 Phase 3 (глобальный кэш)
- Опционально зависит от RFC-014b (система сборки, для native-участников)

## Открытые вопросы

- [x] Разрешить ли циклические зависимости между участниками? → **Нет.** Участники — это независимые
      пакеты, циклы между пакетами являются ошибкой компиляции. (Решение RFC-029, 2026-07-30)
- [ ] Поддерживать ли конфигурацию `[build]` на уровне workspace?
- [ ] Могут ли участники иметь собственный lockfile (перекрывающий корневой)?
- [ ] Поддерживать ли вложенные workspace?

---

## Ссылки

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)
