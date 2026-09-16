---
title: 'RFC-014c: Поддержка рабочих пространств'
status: 'На рассмотрении'
author: 'Чэньсюй'
created: '2026-06-11'
updated: '2026-09-15'
group: 'rfc-014'
issue: '#113'
---

# RFC-014c: Поддержка рабочих пространств

> Этот RFC является под-RFC документа
> [RFC-014: Проект системы управления пакетами](../accepted/014-package-manager.md).

## Резолюция ревью от 2026-09-15

Следующие решения были приняты владельцем 2026-09-15:

1. **Сроки реализации перенесены вперёд**: workspace не зависит от сети и системы сборки (разрешение
   путей `{ workspace = "key" }` + общий lockfile полностью локальны), сроки сдвинуты **до**
   RFC-014b (общий порядок выполнения изменён на `3 → 3.5 → 6 → 4 → 5`). Сам репозиторий компилятора
   (компилятор + расширение vscode + wasm + docs) является первым пользователем.
2. **`[build]` уровня workspace**: не поддерживается. Придерживаемся принципа «корень только
   координирует, члены самодостаточны», объявления сборки записываются только в toml членов.
3. **Собственный lockfile члена**: не допускается, корневой `yaoxiang.lock` единственный.
4. **Вложенные workspace**: на начальном этапе не поддерживаются.

## Резюме

Определяет механизм workspace (рабочего пространства) в YaoXiang: совместное использование
зависимостей, ссылок на пути, единый lockfile и интеграцию с Cargo workspace при разработке
нескольких связанных пакетов.

## Мотивация

Когда проект растёт, код необходимо разбивать на несколько пакетов. Этим пакетам требуется:

- Взаимные ссылки (зависимости по пути)
- Общее использование версий внешних зависимостей (во избежание расхождения версий)
- Единый lockfile (для гарантии согласованности сборки)
- Взаимодействие с Cargo workspace (для частей FFI)

### Текущие проблемы

- Каждый проект независимо управляет зависимостями, совместное использование невозможно
- Отсутствует механизм автоматической замены путевых зависимостей при публикации
- Отсутствует интеграция с Cargo workspace

## Предложение

### Основной дизайн: координирующий слой + самодостаточные члены

Корневой workspace только координирует, каждый член полностью самодостаточен.

### Корневой yaoxiang.toml

```toml
# корневой yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**Корневой toml выполняет только три функции:**

1. Объявляет список членов (в виде словаря, key — имя члена, value — путь к toml)
2. Предоставляет общий lockfile (`yaoxiang.lock`)
3. Предоставляет общий каталог vendor (`.yaoxiang/vendor/`)

**Корневой toml не определяет dependencies.** Зависимости каждого члена записываются в его
собственном `yaoxiang.toml`.

### Членский yaoxiang.toml

```toml
# packages/core/yaoxiang.toml
[package]
name = "core"
version = "0.1.0"

[dependencies]
json = "^2.0.0"
utils = { workspace = "utils" }    # ссылка на члена workspace
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

### Структура workspace

```
my-workspace/
├── yaoxiang.toml              # конфигурация корня workspace
├── yaoxiang.lock              # общий lockfile
├── .yaoxiang/
│   └── vendor/                # общий каталог vendor
├── packages/
│   ├── core/
│   │   ├── yaoxiang.toml      # конфигурация пакета-члена
│   │   └── src/lib.yx
│   ├── utils/
│   │   ├── yaoxiang.toml
│   │   └── src/lib.yx
│   └── app/
│       ├── yaoxiang.toml
│       └── src/main.yx
└── Cargo.toml                 # опционально: общий Cargo workspace (FFI)
```

### Разрешение зависимостей

- Каждый член читает свой `[dependencies]`
- При разрешении зависимости всех членов объединяются, генерируется общий lockfile
- Конфликты версий сообщаются при генерации lockfile
- Один и тот же пакет в разных членах должен разрешаться в одну и ту же версию

### Ссылка на зависимость в workspace

`{ workspace = "member-name" }` ссылается на **key** из `[workspace.members]` (а не на
`[package].name` члена).

```toml
# корневой yaoxiang.toml
[workspace.members]
utils = "packages/utils/yaoxiang.toml"    # key = "utils"
```

```toml
# packages/app/yaoxiang.toml
[package]
name = "app"

[dependencies]
utils = { workspace = "utils" }   # ✅ ссылается на key "utils"
# даже если в packages/utils/yaoxiang.toml указано name = "my-utils"
```

**Почему используется key, а не name:**

- key контролируется workspace, стабилен и уникален
- `[package].name` — публичное имя, может изменяться при публикации
- key — это ключ BTreeMap, по своей природе уникален
- При публикации ссылки workspace заменяются на версионные зависимости, key не утекает в публичный
  API

### Путевые зависимости и публикация

При разработке используется ссылка workspace:

```toml
[dependencies]
utils = { workspace = "utils" }
```

При публикации автоматически заменяется на версионную зависимость:

```toml
[dependencies]
utils = "^0.2.0"
```

**Источник версии:** читается `[package].version` зависимого члена, добавляется префикс `^`.
Registry не проверяется — авторитетным источником версии является `yaoxiang.toml` члена, Registry —
лишь канал распространения.

Менеджер пакетов автоматически выполняет эту замену при `yaoxiang publish`.

### Интеграция с Cargo Workspace

Если в workspace есть FFI-пакеты, можно одновременно определить Cargo workspace:

```toml
# корневой Cargo.toml
[workspace]
members = ["packages/core/native", "packages/utils/native"]
```

```
my-workspace/
├── yaoxiang.toml          # YaoXiang workspace
├── Cargo.toml             # Cargo workspace (FFI часть)
├── packages/
│   ├── core/
│   │   ├── src/lib.yx     # код YaoXiang
│   │   └── native/
│   │       ├── Cargo.toml # код Rust FFI
│   │       └── src/lib.rs
│   └── utils/
│       ├── src/lib.yx
│       └── native/
│           ├── Cargo.toml
│           └── src/lib.rs
```

`yaoxiang build` автоматически обнаруживает это и вызывает `cargo build` для компиляции нативной
части.

### Команды CLI

| Команда                            | Функция                                                    |
| ---------------------------------- | ---------------------------------------------------------- |
| `yaoxiang workspace list`          | Список членов workspace                                    |
| `yaoxiang workspace add <path>`    | Добавление члена                                           |
| `yaoxiang workspace remove <name>` | Удаление члена                                             |
| `yaoxiang build`                   | Сборка всех членов (в топологическом порядке зависимостей) |
| `yaoxiang build core`              | Сборка указанного члена                                    |
| `yaoxiang test`                    | Запуск тестов всех членов                                  |

**Поведение `yaoxiang build`:** собирает все члены в топологическом порядке зависимостей. Если core
→ utils → app, порядок сборки: core → utils → app.

## Детальный дизайн

### Структура WorkspaceManifest

Корневой toml использует отдельный тип `WorkspaceManifest`, а не переиспользует `PackageManifest`:

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

**Логика обнаружения:** при загрузке toml, если присутствует секция `[workspace]`, файл парсится как
`WorkspaceManifest`, иначе — как `PackageManifest`.

### Ссылка на зависимость в workspace

Семантика `{ workspace = "member-name" }`:

- В `dependencies` ссылается на другой член workspace
- При разработке разрешается в локальный путь
- При публикации заменяется на версию из Registry
- Имя члена должно присутствовать в `[workspace.members]`

### Общий lockfile

- У workspace только один `yaoxiang.lock` (в корневом каталоге)
- Разрешение зависимостей всех членов объединяется в один lockfile
- Конфликты версий сообщаются при генерации lockfile с указанием источника конфликта

## Компромиссы

### Преимущества

- Единое управление многопакетным проектом
- Общий lockfile обеспечивает согласованность
- Удобство работы с путевыми зависимостями при разработке
- Бесшовная интеграция с Cargo workspace

### Недостатки

- Все члены должны использовать одинаковые версии внешних зависимостей (может быть слишком строго)
- Корневой toml не может иметь собственных зависимостей (ограничение дизайна)
- Интеграция с Cargo workspace увеличивает сложность

## Альтернативы

| Альтернатива                             | Почему не выбрана                                    |
| ---------------------------------------- | ---------------------------------------------------- |
| Независимые проекты + path-зависимости   | lockfile не единый, риск расхождения версий          |
| Аналог npm workspaces                    | У npm в workspace много проблем, не стоит копировать |
| Прямое переиспользование Cargo workspace | YaoXiang и Cargo — разные экосистемы пакетов         |

## Стратегия реализации

### Разбивка на этапы

| Этап     | Содержание                                                |
| -------- | --------------------------------------------------------- |
| Phase 6a | Парсинг `[workspace.members]` + WorkspaceManifest         |
| Phase 6b | Общий lockfile + объединённое разрешение зависимостей     |
| Phase 6c | `{ workspace = "name" }` ссылка на путевую зависимость    |
| Phase 6d | Автоматическая замена путевых зависимостей при публикации |
| Phase 6e | Интеграция с Cargo workspace                              |

### Зависимости

- Зависит от RFC-014 Phase 3 (глобальный кэш)
- Опционально зависит от RFC-014b (система сборки, для нативных членов)

## Открытые вопросы

- [x] Допускаются ли циклические зависимости между членами?→ **Нет.** Члены — независимые пакеты,
      циклы между пакетами являются ошибкой компиляции. (Решение RFC-029, 2026-07-30)
- [x] Поддерживается ли конфигурация `[build]` уровня workspace?→ Нет, члены самодостаточны
      (резолюция от 2026-09-15, п. 2)
- [x] Могут ли члены иметь собственный lockfile (перекрывающий корневой)?→ Нет, корневой lockfile
      единственный (резолюция от 2026-09-15, п. 3)
- [x] Поддерживаются ли вложенные workspace?→ На начальном этапе нет (резолюция от 2026-09-15, п. 4)

---

## Ссылки

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)
