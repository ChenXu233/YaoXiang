---
title: 'RFC-014c: Поддержка рабочих пространств'
status: 'На рассмотрении'
author: '晨煦'
created: '2026-06-11'
updated: '2026-07-05'
group: 'rfc-014'
issue: '#113'
---

# RFC-014c: Поддержка рабочих пространств

> Настоящий RFC является подчиненным RFC
> [RFC-014: Дизайн системы управления пакетами](../accepted/014-package-manager.md).

## Краткое описание

Определяет механизм рабочих пространств (workspace) для YaoXiang: совместное использование
зависимостей, ссылки на пути, унификация lockfile и интеграция с Cargo workspace при разработке
нескольких связанных пакетов.

## Мотивация

Когда проект растёт, код необходимо разделять на несколько пакетов. Эти пакеты должны:

- ссылаться друг на друга (путевые зависимости)
- совместно использовать версии внешних зависимостей (во избежание дрейфа версий)
- иметь единый lockfile (для обеспечения согласованности сборки)
- взаимодействовать с Cargo workspace (для FFI)

### Текущие проблемы

- Каждый проект управляет зависимостями независимо, без возможности совместного использования
- Отсутствует механизм автоматической замены путевых зависимостей при публикации
- Нет интеграции с Cargo workspace

## Предложение

### Основной дизайн: координационный слой + самодостаточные участники

Корневое workspace выполняет только координацию, каждый участник полностью самодостаточен.

### Корневой yaoxiang.toml

```toml
# Корневой yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**Корневой toml выполняет только три действия:**

1. Объявляет список участников (в виде словаря, где key — имя участника, value — путь к toml)
2. Предоставляет общий lockfile (`yaoxiang.lock`)
3. Предоставляет общую директорию vendor (`.yaoxiang/vendor/`)

**Корневой toml не определяет dependencies.** Зависимости каждого участника записываются в его
собственном `yaoxiang.toml`.

### Участник yaoxiang.toml

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
├── yaoxiang.toml              # Корневая конфигурация рабочего пространства
├── yaoxiang.lock              # Общий lockfile
├── .yaoxiang/
│   └── vendor/                # Общая директория vendor
├── packages/
│   ├── core/
│   │   ├── yaoxiang.toml      # Конфигурация участника
│   │   └── src/lib.yx
│   ├── utils/
│   │   ├── yaoxiang.toml
│   │   └── src/lib.yx
│   └── app/
│       ├── yaoxiang.toml
│       └── src/main.yx
└── Cargo.toml                 # Опционально: общий Cargo workspace (FFI)
```

### Разрешение зависимостей

- Каждый участник читает свой собственный `[dependencies]`
- При разрешении зависимости всех участников объединяются, генерируя общий lockfile
- Конфликты версий вызывают ошибку при генерации lockfile
- Один и тот же пакет в разных участниках должен разрешаться в одинаковую версию

### Ссылки на зависимости workspace

`{ workspace = "member-name" }` ссылается на **key** из `[workspace.members]` (не на
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

**Почему используется key вместо name:**

- key контролируется рабочим пространством, стабильный и уникальный
- `[package].name` — это публичное имя, которое может измениться при публикации
- key является ключом BTreeMap, естественно уникальным
- При публикации ссылки workspace заменяются на версионные зависимости, key не раскрывается в
  публичном API

### Путевые зависимости и публикация

При разработке используются ссылки рабочего пространства:

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
Registry не проверяется——авторитетным источником версии является `yaoxiang.toml` участника, Registry
— это лишь канал распространения.

Менеджер пакетов автоматически выполняет эту замену при `yaoxiang publish`.

### Интеграция с Cargo Workspace

Если в рабочем пространстве есть FFI пакеты, можно одновременно определить Cargo workspace:

```toml
# Корневой Cargo.toml
[workspace]
members = ["packages/core/native", "packages/utils/native"]
```

```
my-workspace/
├── yaoxiang.toml          # YaoXiang workspace
├── Cargo.toml             # Cargo workspace (FFI часть)
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

`yaoxiang build` автоматически определяет и вызывает `cargo build` для компиляции нативной части.

### CLI команды

| Команда                            | Функция                                 |
| ---------------------------------- | --------------------------------------- |
| `yaoxiang workspace list`          | Перечислить участников workspace        |
| `yaoxiang workspace add <path>`    | Добавить участника                      |
| `yaoxiang workspace remove <name>` | Удалить участника                       |
| `yaoxiang build`                   | Собрать всех участников (топологически) |
| `yaoxiang build core`              | Собрать указанного участника            |
| `yaoxiang test`                    | Запустить тесты всех участников         |

**Поведение `yaoxiang build`:** собирает всех участников в порядке топологической сортировки
зависимостей. Если core → utils → app, порядок сборки будет core → utils → app.

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

**Логика определения:** при загрузке toml, если есть секция `[workspace]`, парсится как
`WorkspaceManifest`, иначе как `PackageManifest`.

### Ссылки на зависимости workspace

Семантика `{ workspace = "member-name" }`:

- В `dependencies` ссылается на другого участника рабочего пространства
- При разработке разрешается в локальный путь
- При публикации заменяется на версию из Registry
- Имя участника должно существовать в `[workspace.members]`

### Совместное использование lockfile

- У рабочего пространства только один `yaoxiang.lock` (в корневой директории)
- Разрешение зависимостей всех участников объединяется в единый lockfile
- Конфликты версий вызывают ошибку при генерации lockfile с информацией об источнике конфликта

## Компромиссы

### Преимущества

- Единое управление мультипакетными проектами
- Общий lockfile обеспечивает согласованность
- Хороший опыт разработки с путевыми зависимостями
- Бесшовная интеграция с Cargo workspace

### Недостатки

- Все участники должны использовать одинаковые версии внешних зависимостей (может быть слишком
  строго)
- Корневой toml не может иметь собственные зависимости (ограничение дизайна)
- Интеграция с Cargo workspace усложняет систему

## Альтернативные решения

| Решение                                  | Почему не выбрано                                    |
| ---------------------------------------- | ---------------------------------------------------- |
| Независимые проекты + path зависимости   | lockfile не унифицирован, риск дрейфа версий         |
| Подобно npm workspaces                   | У npm много проблем с workspace, не стоит копировать |
| Прямое переиспользование Cargo workspace | YaoXiang и Cargo — разные экосистемы пакетов         |

## Стратегия реализации

### Фазы

| Фаза     | Содержание                                                |
| -------- | --------------------------------------------------------- |
| Phase 6a | Парсинг `[workspace.members]` + WorkspaceManifest         |
| Phase 6b | Общий lockfile + объединённое разрешение зависимостей     |
| Phase 6c | `{ workspace = "name" }` путевая ссылка                   |
| Phase 6d | Автоматическая замена путевых зависимостей при публикации |
| Phase 6e | Интеграция с Cargo workspace                              |

### Зависимости

- Зависит от RFC-014 Phase 3 (глобальный кэш)
- Опциональная зависимость от RFC-014b (система сборки, для нативных участников)

## Открытые вопросы

- [ ] Разрешены ли циклические зависимости между участниками?
- [ ] Поддерживается ли workspace-уровневая конфигурация `[build]`?
- [ ] Могут ли участники иметь собственный lockfile (переопределяющий корневой)?
- [ ] Поддерживаются ли вложенные workspace?

---

## Ссылки

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)
