---
title: 'RFC-014a: Спецификация протокола Registry'
status: 'На рассмотрении'
author: '晨煦'
created: '2026-06-11'
updated: '2026-07-05'
group: 'rfc-014'
---

# RFC-014a: Спецификация протокола Registry

> Настоящий RFC является подчинённым документом
> [RFC-014: Дизайн системы управления пакетами](../accepted/014-package-manager.md).

## Краткое описание

Определение протокола Registry для системы управления пакетами YaoXiang: дизайн открытых
интерфейсов, спецификация официального Registry, адаптер для GitHub, процесс публикации/отзыва
пакетов, модель аутентификации.

## Мотивация

В общей спецификации RFC-014 определена общая архитектура системы управления пакетами, однако раздел
Registry лишь обозначен как «зарезервированный». Без протокола Registry пакеты невозможно
распространять — это всё равно что спроектировать тележку для покупок без магазина.

### Текущие проблемы

- `RegistrySource` является заглушкой (`source/mod.rs:150-203`), `resolve` напрямую возвращает
  объявленную версию, `download` возвращает пустой путь
- Отсутствует HTTP-клиент (нет зависимости `reqwest`)
- Отсутствует механизм публикации пакетов
- Отсутствует аутентификация/авторизация

## Предложение

### Основной дизайн: открытый протокол + адаптерный слой

```
┌──────────────────────────────────────────┐
│         yaoxiang publish/install         │  ← CLI слой
└──────────────────┬───────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│          Registry Trait                  │  ← Протокольный слой (открытый интерфейс)
│  ┌─────────┬──────────┬────────────┐    │
│  │ .publish│ .search  │ .download  │    │
│  │ .yank   │ .info    │ .versions  │    │
│  └─────────┴──────────┴────────────┘    │
└──────────────────┬───────────────────────┘
                   │
        ┌──────────┼──────────┐
        ▼          ▼          ▼
   ┌─────────┐ ┌────────┐ ┌────────┐
   │ Официальный│ │ GitHub │ │ Пользовательский│
   │ Registry│ │ Адаптер│ │ Registry│
   └─────────┘ └────────┘ └────────┘
```

### Решение об асинхронной архитектуре

`Source` trait унифицированно меняется на async, полный переход на tokio:

```rust
// Существующий (синхронный) → Изменённый (асинхронный)
#[async_trait]
pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> SourceKind;

    async fn resolve(&self, spec: &DependencySpec) -> PackageResult<String>;
    async fn download(&self, spec: &DependencySpec, dest: &Path) -> PackageResult<ResolvedPackage>;
}
```

Все реализации (`LocalSource`, `GitSource`, `RegistrySource`) унифицированно меняются на async.
CLI入口通过 `#[tokio::main]` 或 `Runtime::block_on` 驱动。

**Обоснование:**

- Registry требует HTTP-запросов, блокировка парализует весь процесс установки
- Параллельная загрузка нескольких зависимостей (`join_all`) значительно ускоряет установку
- Git clone тоже операция ввода-вывода, async более естественен
- tokio уже есть в зависимостях проекта

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// Публикация пакета
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// Удаление опубликованной версии (необратимо, номер версии заблокирован)
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// Запрос информации о пакете
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// Запрос списка доступных версий
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// Поиск пакетов
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// Скачивание указанной версии
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// Аутентификация
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### Приоритет источников (цепочка поиска по умолчанию)

Порядок поиска по умолчанию при `yaoxiang add foo` (без флагов):

| Приоритет | Поиск                | Описание                                                       |
| --------- | -------------------- | -------------------------------------------------------------- |
| 1         | Глобальный кэш       | `~/.yaoxiang/cache/registry/foo-<ver>/`                        |
| 2         | Официальный Registry | Запрос версии → Скачивание                                     |
| 3         | Сбой                 | Сообщение об ошибке, предложение проверить имя пакета или сеть |

**Явное переопределение (обход цепочки по умолчанию):**

| Флаг               | Поведение                                                                                    |
| ------------------ | -------------------------------------------------------------------------------------------- |
| `--git <url>`      | Пропустить Registry, напрямую Git clone (приоритет: Release assets → fallback на tag/branch) |
| `--path <dir>`     | Пропустить Registry, напрямую использовать локальный путь                                    |
| `--registry <url>` | Пропустить официальный Registry, использовать указанный Registry                             |

### Официальный Registry

Официальный Registry по аналогии с crates.io является основным каналом распространения пакетов.

**API-endpoints:**

| Endpoint                                 | Метод  | Описание                   |
| ---------------------------------------- | ------ | -------------------------- |
| `/api/v1/packages/{name}`                | GET    | Запрос информации о пакете |
| `/api/v1/packages/{name}/versions`       | GET    | Запрос списка версий       |
| `/api/v1/packages/{name}/{version}`      | GET    | Скачивание пакета          |
| `/api/v1/packages`                       | PUT    | Публикация пакета          |
| `/api/v1/packages/{name}/{version}/yank` | DELETE | Отзыв версии               |
| `/api/v1/search?q={query}`               | GET    | Поиск пакетов              |
| `/api/v1/login`                          | POST   | Аутентификация             |

### Интеграция с GitHub

При использовании GitHub в качестве источника пакетов применяется стратегия в стиле Go modules:

1. **Приоритет Release assets**: проверка наличия на странице GitHub Release предкомпилированных
   артефактов для целевой платформы
2. **Fallback на main ветку**: при отсутствии Release выполняется git clone

```toml
[dependencies]
# Базовая git-зависимость
foo = { git = "https://github.com/user/foo" }

# Указание версии (совпадение с тегом)
bar = { git = "https://github.com/user/bar", version = "^1.0.0" }

# Указание ветки
baz = { git = "https://github.com/user/baz", branch = "main" }

# Указание commit
qux = { git = "https://github.com/user/qux", rev = "abc123" }

# Приватный репозиторий (с использованием GitHub token из credentials.toml)
private = { git = "https://github.com/my-org/private-lib" }
```

### Формат пакета (.yxpkg)

```
foo-1.2.3.yxpkg (tar.gz)
├── yaoxiang.toml          # Метаданные пакета
├── src/                   # Исходный код
├── build/                 # Результаты сборки (если есть)
│   └── native/
│       └── linux-x86_64/
│           └── libfoo.so
├── build.yx               # Скрипт сборки (если есть)
└── SHA256SUMS             # Контрольные суммы
```

### Процесс публикации

```bash
# Публикация в официальный Registry
yaoxiang publish

# Публикация в указанный Registry
yaoxiang publish --registry my-company

# Одновременное создание GitHub Release
yaoxiang publish --github

# Пробный прогон
yaoxiang publish --dry-run
```

Валидация перед публикацией:

1. `yaoxiang.toml` должен содержать `name`, `version`, `description`
2. Номер версии не должен существовать
3. Запуск тестов (опционально, `--no-test` для пропуска)
4. Вычисление SHA-256 для всех файлов
5. Упаковка в `.yxpkg` (tar.gz)
6. Загрузка в Registry

### Семантика yank

```bash
yaoxiang yank foo@1.2.3
```

**Удаление + блокировка номера версии:**

- Пакет полностью удаляется, восстановление невозможно
- Номер версии блокируется навсегда, нельзя повторно опубликовать ту же версию
- Проекты с lockfile, ссылающимися на эту версию, выдают ошибку — требуется обновление до другой
  версии
- **Цель безопасности**: предотвращение атак на цепочку поставок в стиле npm. Злоумышленники
  регистрировали удалённые номера версий пакетов для внедрения вредоносного кода; yank с блокировкой
  номера версии полностью закрывает эту лазейку.

### Модель аутентификации

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**Правила сопоставления:** `yaoxiang login --registry <url>` сопоставляет по URL поле `url` в
`[registries.*]`. Если совпадение не найдено, создаётся новая запись (с автогенерируемым именем,
например `reg-1`).

**Приоритет:** переменные окружения > конфигурационный файл

| Переменная окружения | Назначение                                          |
| -------------------- | --------------------------------------------------- |
| `$YX_GITHUB_TOKEN`   | Аутентификация GitHub                               |
| `$YX_REGISTRY_TOKEN` | Аутентификация Registry (для Registry по умолчанию) |
| `$YX_REGISTRY_URL`   | Адрес Registry по умолчанию                         |

**CLI-команды:**

```bash
yaoxiang login --registry https://yxreg.example.com   # Сопоставление по URL или создание
yaoxiang login --github                                # GitHub OAuth или token
yaoxiang logout --registry https://yxreg.example.com   # Удаление сопоставленной записи
```

**Ограничения безопасности:**

- Token никогда не записывается в `yaoxiang.toml` или `yaoxiang.lock`
- Файл `credentials.toml` с правами доступа 600
- В CI-сценариях используются переменные окружения, при разработке — файл

## Детальный дизайн

### Реализация RegistrySource

Замена существующей заглушки (`source/mod.rs:150-203`):

```rust
pub struct RegistrySource {
    client: reqwest::Client,
    base_url: String,
}

#[async_trait]
impl Source for RegistrySource {
    fn name(&self) -> &str { "registry" }
    fn kind(&self) -> SourceKind { SourceKind::Registry }

    async fn resolve(&self, spec: &DependencySpec) -> PackageResult<String> {
        let url = format!("{}/api/v1/packages/{}/versions", self.base_url, spec.name);
        let versions: Vec<Version> = self.client.get(&url).send().await?.json().await?;
        let req = parse_version_req(&spec.version)?;
        select_best(&req, &versions)
            .map(|v| v.to_string())
            .ok_or(PackageError::DependencyNotFound(spec.name.clone()))
    }

    async fn download(&self, spec: &DependencySpec, dest: &Path) -> PackageResult<ResolvedPackage> {
        let version = self.resolve(spec).await?;
        let url = format!("{}/api/v1/packages/{}/{}/download", self.base_url, spec.name, version);
        let bytes = self.client.get(&url).send().await?.bytes().await?;

        // SHA-256 проверка
        let actual_hash = sha256_hex(&bytes);
        // ... распаковка в dest ...

        Ok(ResolvedPackage {
            name: spec.name.clone(),
            version,
            source_kind: SourceKind::Registry,
            source_url: self.base_url.clone(),
            local_path: dest.to_path_buf(),
            checksum: Some(actual_hash),
        })
    }
}
```

### Зависимости

| crate            | Назначение                |
| ---------------- | ------------------------- |
| `reqwest`        | HTTP-клиент               |
| `sha2`           | SHA-256 проверка          |
| `flate2` + `tar` | Обработка формата пакетов |
| `async-trait`    | Поддержка async trait     |

### Типы ошибок

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Пакет '{0}' не существует")]
    PackageNotFound(String),

    #[error("Версия '{0}' не существует")]
    VersionNotFound(String),

    #[error("Версия '{0}' уже занята")]
    VersionAlreadyExists(String),

    #[error("Ошибка аутентификации: {0}")]
    AuthFailed(String),

    #[error("Ошибка сети: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Ошибка проверки SHA-256: ожидалось {expected}, получено {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Недостаточно прав: {0}")]
    Forbidden(String),
}
```

## Компромиссы

### Преимущества

- Открытый протокол, отсутствие привязки к конкретному серверу
- GitHub как лёгкий канал дистрибуции, снижение порога входа
- Модель безопасности с блокировкой номеров версий
- Стратегия установки с приоритетом предкомпилированных артефактов

### Недостатки

- Официальный Registry требует отдельной эксплуатации
- GitHub API имеет ограничения скорости
- Блокировка номеров версий может привести к их нерациональному расходованию

## Альтернативные варианты

| Вариант                           | Почему не выбран                                                               |
| --------------------------------- | ------------------------------------------------------------------------------ |
| Только GitHub                     | Зависимость от экосистемы GitHub, невозможность создания собственного Registry |
| crates.io в стиле Cargo           | Слишком сложно, на начальном этапе экосистема YaoXiang в этом не нуждается     |
| yank в стиле npm (только пометка) | Риски безопасности, известны случаи атак на цепочку поставок                   |

## Стратегия реализации

### Разделение на этапы

| Этап      | Содержание                                                    |
| --------- | ------------------------------------------------------------- |
| Phase 3.5 | Source trait → async + async-trait + миграция всех реализаций |
| Phase 4a  | Registry trait + интеграция reqwest + локальный Registry mock |
| Phase 4b  | Адаптер GitHub Release                                        |
| Phase 4c  | Команда publish + упаковка в формат пакетов                   |
| Phase 4d  | Аутентификация + yank                                         |

### Зависимости

- Зависит от RFC-014 Phase 3 (глобальный кэш, замена semver)
- Зависит от RFC-014b (система сборки, для обработки директории `build/`)

## Открытые вопросы

- [ ] Нужна ли версионификация API Registry (`/api/v1/` vs `/api/v2/`)?
- [ ] Поддержка namespace в именах пакетов (например, `@org/pkg`)?
- [ ] Стратегия ограничения скорости?
- [ ] Лимит размера пакета?

---

## Ссылки

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)
