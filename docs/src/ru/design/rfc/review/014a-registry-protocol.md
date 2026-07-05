---
title: "RFC-014a: Спецификация протокола Registry"
status: "На рассмотрении"
author: "晨煦"
created: "2026-06-11"
updated: "2026-07-05"
group: "rfc-014"
---

# RFC-014a: Спецификация протокола Registry

> Данный RFC является под-RFC к [RFC-014: Проектирование системы управления пакетами](../accepted/014-package-manager.md).

## Аннотация

Определяет протокол Registry системы управления пакетами YaoXiang: дизайн открытого интерфейса, спецификацию официального Registry, слой адаптера GitHub, процедуры публикации/отзыва пакетов, модель аутентификации.

## Мотивация

Общий документ RFC-014 определяет архитектуру системы управления пакетами, но раздел Registry помечен лишь как «зарезервировано». Без протокола Registry пакеты невозможно распространять — это как проектирование тележки для покупок без магазина.

### Текущие проблемы

- `RegistrySource` представляет собой заглушку (`source/mod.rs:150-203`), `resolve` напрямую возвращает объявленную версию, `download` возвращает пустой путь
- HTTP-клиент отсутствует (нет зависимости `reqwest`)
- Механизм публикации пакетов отсутствует
- Аутентификация/авторизация отсутствует

## Предложение

### Ключевая идея: открытый протокол + слой адаптера

```
┌──────────────────────────────────────────┐
│         yaoxiang publish/install         │  ← Уровень CLI
└──────────────────┬───────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│          Registry Trait                  │  ← Уровень протокола (открытый интерфейс)
│  ┌─────────┬──────────┬────────────┐    │
│  │ .publish│ .search  │ .download  │    │
│  │ .yank   │ .info    │ .versions  │    │
│  └─────────┴──────────┴────────────┘    │
└──────────────────┬───────────────────────┘
                   │
        ┌──────────┼──────────┐
        ▼          ▼          ▼
   ┌─────────┐ ┌────────┐ ┌────────┐
   │ Офици-  │ │ Адап-  │ │ Пользо-│
   │ альный  │ │ тер    │ │ ватель-│
   │Registry │ │ GitHub │ │ ский   │
   │         │ │        │ │ Registry│
   └─────────┘ └────────┘ └────────┘
```

### Решение об асинхронной архитектуре

Trait `Source` единообразно переводится на async с полным переходом на tokio:

```rust
// Существующий (синхронный) → новый (асинхронный)
#[async_trait]
pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> SourceKind;

    async fn resolve(&self, spec: &DependencySpec) -> PackageResult<String>;
    async fn download(&self, spec: &DependencySpec, dest: &Path) -> PackageResult<ResolvedPackage>;
}
```

Все реализации (`LocalSource`, `GitSource`, `RegistrySource`) единообразно переводятся на async. Точка входа CLI запускается через `#[tokio::main]` или `Runtime::block_on`.

**Обоснование:**
- Registry требует HTTP-запросов, блокировка остановит весь процесс установки
- Параллельная загрузка множества зависимостей (`join_all`) существенно повышает скорость установки
- Git clone также является I/O-операцией, async для неё естественнее
- tokio уже присутствует в зависимостях проекта

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// Публикация пакета
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// Удаление опубликованной версии (необратимое, номер версии блокируется)
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// Получение информации о пакете
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// Получение списка доступных версий
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// Поиск пакетов
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// Загрузка указанной версии
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// Аутентификация
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### Приоритет источников (цепочка поиска по умолчанию)

Цепочка поиска по умолчанию при выполнении `yaoxiang add foo` (без флагов):

| Приоритет | Где искать | Пояснение |
|-----------|------------|-----------|
| 1 | Глобальный кеш | `~/.yaoxiang/cache/registry/foo-<ver>/` |
| 2 | Официальный Registry | Запрос версии → загрузка |
| 3 | Неудача | Возврат ошибки с подсказкой проверить имя пакета или сеть |

**Явное переопределение (минуя цепочку по умолчанию):**

| Флаг | Поведение |
|------|-----------|
| `--git <url>` | Пропуск Registry, прямой Git clone (приоритет у Release assets → fallback на tag/branch) |
| `--path <dir>` | Пропуск Registry, прямой поиск по локальному пути |
| `--registry <url>` | Пропуск официального Registry, использование указанного Registry |

### Официальный Registry

Официальный Registry, аналогичный crates.io, является основным каналом распространения пакетов.

**Конечные точки API:**

| Конечная точка | Метод | Пояснение |
|----------------|-------|-----------|
| `/api/v1/packages/{name}` | GET | Получение информации о пакете |
| `/api/v1/packages/{name}/versions` | GET | Получение списка версий |
| `/api/v1/packages/{name}/{version}` | GET | Загрузка пакета |
| `/api/v1/packages` | PUT | Публикация пакета |
| `/api/v1/packages/{name}/{version}/yank` | DELETE | Отзыв версии |
| `/api/v1/search?q={query}` | GET | Поиск пакетов |
| `/api/v1/login` | POST | Аутентификация |

### Интеграция с GitHub

При использовании GitHub в качестве источника пакетов применяется стратегия в стиле Go modules:

1. **Приоритет у Release assets**: проверка наличия на странице GitHub Release предсобранных артефактов для соответствующей платформы
2. **Fallback на ветку main**: при отсутствии Release — git clone

```toml
[dependencies]
# Базовая git-зависимость
foo = { git = "https://github.com/user/foo" }

# Указание версии (сопоставление по tag)
bar = { git = "https://github.com/user/bar", version = "^1.0.0" }

# Указание ветки
baz = { git = "https://github.com/user/baz", branch = "main" }

# Указание коммита
qux = { git = "https://github.com/user/qux", rev = "abc123" }

# Приватный репозиторий (используется токен GitHub из credentials.toml)
private = { git = "https://github.com/my-org/private-lib" }
```

### Формат пакета (.yxpkg)

```
foo-1.2.3.yxpkg (tar.gz)
├── yaoxiang.toml          # Метаданные пакета
├── src/                   # Исходный код
├── build/                 # Артефакты сборки (если есть)
│   └── native/
│       └── linux-x86_64/
│           └── libfoo.so
├── build.yx               # Скрипт сборки (если есть)
└── SHA256SUMS             # Контрольные суммы
```

### Процедура publish

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

Проверки перед публикацией:
1. В `yaoxiang.toml` должны присутствовать `name`, `version`, `description`
2. Номер версии не должен существовать
3. Запуск тестов (опционально, `--no-test` для пропуска)
4. Вычисление SHA-256 всех файлов
5. Упаковка в `.yxpkg` (tar.gz)
6. Загрузка в Registry

### Семантика yank

```bash
yaoxiang yank foo@1.2.3
```

**Удаление + блокировка номера версии:**

- Пакет полностью удаляется, восстановление невозможно
- Номер версии блокируется навсегда, повторная публикация с тем же номером невозможна
- Проекты с lockfile, ссылающимся на эту версию, получат ошибку и должны будут перейти на другую версию
- **Цель безопасности**: предотвращение атак на цепочку поставок в стиле npm. Злоумышленники перехватывали удалённые номера версий пакетов для внедрения вредоносного кода; блокировка номера версии при yank полностью закрывает этот вектор

### Модель аутентификации

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**Правило сопоставления:** `yaoxiang login --registry <url>` сопоставляется по URL с полем `url` в `[registries.*]`. При отсутствии совпадения создаётся новая запись (с автогенерируемым именем, например `reg-1`).

**Приоритет:** переменные окружения > файл конфигурации

| Переменная окружения | Назначение |
|----------------------|------------|
| `$YX_GITHUB_TOKEN` | Аутентификация GitHub |
| `$YX_REGISTRY_TOKEN` | Аутентификация Registry (для Registry по умолчанию) |
| `$YX_REGISTRY_URL` | Адрес Registry по умолчанию |

**Команды CLI:**

```bash
yaoxiang login --registry https://yxreg.example.com   # Сопоставление по URL или создание новой записи
yaoxiang login --github                                # GitHub OAuth или токен
yaoxiang logout --registry https://yxreg.example.com   # Удаление соответствующей записи
```

**Ограничения безопасности:**
- Токен никогда не записывается в `yaoxiang.toml` или `yaoxiang.lock`
- Права доступа к файлу `credentials.toml` — 600
- Для CI используются переменные окружения, для разработки — файл

## Детальное проектирование

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

        // Проверка SHA-256
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

| crate | Назначение |
|-------|------------|
| `reqwest` | HTTP-клиент |
| `sha2` | Проверка SHA-256 |
| `flate2` + `tar` | Обработка формата пакета |
| `async-trait` | Поддержка async trait |

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

    #[error("Сетевая ошибка: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("Ошибка проверки SHA-256: ожидалось {expected}, получено {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Недостаточно прав: {0}")]
    Forbidden(String),
}
```

## Компромиссы

### Преимущества

- Открытый протокол, не привязан к конкретному серверу
- GitHub как лёгкий канал распространения снижает порог входа
- Модель безопасности с блокировкой номера версии
- Стратегия установки с приоритетом предсобранных артефактов

### Недостатки

- Официальный Registry требует отдельной эксплуатации
- API GitHub имеет ограничения по частоте запросов
- Блокировка номера версии может приводить к «растрате» номеров версий

## Альтернативные варианты

| Вариант | Почему не выбран |
|---------|------------------|
| Только поддержка GitHub | Привязка к экосистеме GitHub, невозможность собственного Registry |
| crates.io в стиле Cargo | Избыточная сложность, на раннем этапе экосистемы YaoXiang не требуется |
| yank в стиле npm (только пометка) | Риск безопасности, известные случаи атак на цепочку поставок |

## Стратегия реализации

### Разбивка на этапы

| Этап | Содержание |
|------|------------|
| Phase 3.5 | Перевод Source trait на async + async-trait + миграция всех реализаций |
| Phase 4a | Registry trait + интеграция reqwest + локальный mock Registry |
| Phase 4b | Адаптер GitHub Release |
| Phase 4c | Команда publish + упаковка формата пакета |
| Phase 4d | Аутентификация + yank |

### Зависимости

- Зависит от RFC-014 Phase 3 (глобальный кеш, замена semver)
- Зависит от RFC-014b (система сборки, для обработки каталога `build/`)

## Открытые вопросы

- [ ] Нужна ли версионизация API Registry (`/api/v1/` против `/api/v2/`)?
- [ ] Поддерживать ли пространства имён в именах пакетов (например, `@org/pkg`)?
- [ ] Стратегия ограничения частоты запросов?
- [ ] Максимальный размер пакета?

---

## Ссылки

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)