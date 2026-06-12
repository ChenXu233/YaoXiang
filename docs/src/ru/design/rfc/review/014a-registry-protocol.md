```markdown
---
title: "RFC-014a: Спецификация протокола Registry"
status: "На рассмотрении"
author: "晨煦"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
---

# RFC-014a: Спецификация протокола Registry

> Данный RFC является под-RFC к [RFC-014: Проектирование системы управления пакетами](../accepted/014-package-manager.md).

## Резюме

Определяет протокол Registry системы управления пакетами YaoXiang: проектирование открытого интерфейса, спецификацию официального Registry, адаптер GitHub, процессы публикации и отзыва пакетов, модель аутентификации.

## Мотивация

Основной RFC-014 определяет общую архитектуру системы управления пакетами, однако раздел Registry помечен лишь как «зарезервировано». Без протокола Registry пакеты невозможно распространять — это похоже на проектирование тележки для покупок без магазина.

### Текущие проблемы

- `RegistrySource` является заглушкой (`source/mod.rs:150-203`): `resolve` напрямую возвращает объявленную версию, `download` возвращает пустой путь
- HTTP-клиент отсутствует (нет зависимости `reqwest`)
- Механизм публикации пакетов отсутствует
- Аутентификация/авторизация отсутствует

## Предложение

### Основное проектирование: открытый протокол + слой адаптеров

```
┌──────────────────────────────────────────┐
│         yaoxiang publish/install         │  ← слой CLI
└──────────────────┬───────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│          Трейт Registry                  │  ← слой протокола (открытый интерфейс)
│  ┌─────────┬──────────┬────────────┐    │
│  │ .publish│ .search  │ .download  │    │
│  │ .yank   │ .info    │ .versions  │    │
│  └─────────┴──────────┴────────────┘    │
└──────────────────┬───────────────────────┘
                   │
        ┌──────────┼──────────┐
        ▼          ▼          ▼
   ┌─────────┐ ┌────────┐ ┌─────────┐
   │Официальн│ │Адаптер │ │Пользов. │
   │Registry │ │ GitHub │ │Registry │
   └─────────┘ └────────┘ └─────────┘
```

### Решение об асинхронной архитектуре

Трейт `Source` повсеместно переводится на `async`, с полным переходом на tokio:

```rust
// Текущий (синхронный) → становится (асинхронным)
#[async_trait]
pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> SourceKind;

    async fn resolve(&self, spec: &DependencySpec) -> PackageResult<String>;
    async fn download(&self, spec: &DependencySpec, dest: &Path) -> PackageResult<ResolvedPackage>;
}
```

Все реализации (`LocalSource`, `GitSource`, `RegistrySource`) единообразно переводятся на `async`. Точка входа CLI управляется через `#[tokio::main]` или `Runtime::block_on`.

**Обоснование:**
- Registry требует HTTP-запросов, блокировка заморозит весь процесс установки
- Параллельная загрузка множества зависимостей (`join_all`) существенно повышает скорость установки
- Git clone также является I/O-операцией, `async` подходит здесь естественнее
- tokio уже присутствует в зависимостях проекта

### Трейт Registry

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// Опубликовать пакет
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// Удалить опубликованную версию (невосстановимо, номер версии блокируется)
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// Получить информацию о пакете
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// Получить список доступных версий
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// Поиск пакетов
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// Загрузить указанную версию
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// Аутентификация
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### Приоритет источников (цепочка поиска по умолчанию)

Порядок поиска по умолчанию для `yaoxiang add foo` (без флагов):

| Приоритет | Источник | Описание |
|-----------|----------|----------|
| 1 | Глобальный кэш | `~/.yaoxiang/cache/registry/foo-<ver>/` |
| 2 | Официальный Registry | запрос версии → загрузка |
| 3 | Ошибка | сообщение об ошибке, предложение проверить имя пакета или сеть |

**Явное переопределение (минуя цепочку по умолчанию):**

| Флаг | Поведение |
|------|-----------|
| `--git <url>` | Пропустить Registry, выполнить `git clone` напрямую (с приоритетом Release assets → fallback на tag/branch) |
| `--path <dir>` | Пропустить Registry, использовать локальный путь |
| `--registry <url>` | Пропустить официальный Registry, использовать указанный Registry |

### Официальный Registry

Официальный Registry подобен crates.io и является основным каналом распространения пакетов.

**Конечные точки API:**

| Конечная точка | Метод | Описание |
|----------------|-------|----------|
| `/api/v1/packages/{name}` | GET | Получить информацию о пакете |
| `/api/v1/packages/{name}/versions` | GET | Получить список версий |
| `/api/v1/packages/{name}/{version}` | GET | Загрузить пакет |
| `/api/v1/packages` | PUT | Опубликовать пакет |
| `/api/v1/packages/{name}/{version}/yank` | DELETE | Отозвать версию |
| `/api/v1/search?q={query}` | GET | Поиск пакетов |
| `/api/v1/login` | POST | Аутентификация |

### Интеграция с GitHub

При использовании GitHub в качестве источника пакетов применяется стратегия в стиле Go modules:

1. **Приоритет Release assets**: проверить страницу GitHub Release на наличие прекомпилированных артефактов для целевой платформы
2. **Fallback на ветку main**: при отсутствии Release выполняется `git clone`

```toml
[dependencies]
# Базовая git-зависимость
foo = { git = "https://github.com/user/foo" }

# Указание версии (сопоставление с тегом)
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
├── build/                 # Артефакты сборки (при наличии)
│   └── native/
│       └── linux-x86_64/
│           └── libfoo.so
├── build.yx               # Скрипт сборки (при наличии)
└── SHA256SUMS             # Контрольные суммы
```

### Процесс публикации (publish)

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
1. `yaoxiang.toml` должен содержать `name`, `version`, `description`
2. Номер версии не должен существовать
3. Запуск тестов (опционально, пропускается с `--no-test`)
4. Вычисление SHA-256 для всех файлов
5. Упаковка в `.yxpkg` (tar.gz)
6. Загрузка в Registry

### Семантика yank

```bash
yaoxiang yank foo@1.2.3
```

**Удаление + блокировка номера версии:**

- Пакет полностью удаляется, восстановление невозможно
- Номер версии занимается навсегда, повторная публикация того же номера невозможна
- Проекты, имеющие ссылки на эту версию в lockfile, получат ошибку и должны будут обновиться до другой версии
- **Цель безопасности**: предотвращение атак на цепочку поставок в стиле npm. Злоумышленники ранее перехватывали номера удалённых версий пакетов для внедрения вредоносного кода; блокировка номера версии при yank полностью закрывает этот вектор атаки

### Модель аутентификации

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**Правило сопоставления:** `yaoxiang login --registry <url>` сопоставляет URL с полем `url` в секции `[registries.*]`. Если совпадение не найдено — создаётся новая запись (с автоматически сгенерированным именем, например `reg-1`).

**Приоритет:** переменные окружения > файл конфигурации

| Переменная окружения | Назначение |
|----------------------|------------|
| `$YX_GITHUB_TOKEN` | Аутентификация GitHub |
| `$YX_REGISTRY_TOKEN` | Аутентификация Registry (для Registry по умолчанию) |
| `$YX_REGISTRY_URL` | Адрес Registry по умолчанию |

**Команды CLI:**

```bash
yaoxiang login --registry https://yxreg.example.com   # Сопоставление по URL или создание новой
yaoxiang login --github                                # GitHub OAuth или токен
yaoxiang logout --registry https://yxreg.example.com   # Удаление соответствующей записи
```

**Ограничения безопасности:**
- Токен никогда не записывается в `yaoxiang.toml` или `yaoxiang.lock`
- Права доступа к файлу `credentials.toml` — 600
- В CI используются переменные окружения, в разработке — файл

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
| `async-trait` | Поддержка async в трейтах |

### Типы ошибок

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Пакет '{0}' не найден")]
    PackageNotFound(String),

    #[error("Версия '{0}' не найдена")]
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

- Открытый протокол, отсутствие привязки к конкретному серверу
- GitHub в качестве лёгкого канала распространения снижает порог входа
- Модель безопасности с блокировкой номера версии
- Стратегия установки с приоритетом прекомпилированных артефактов

### Недостатки

- Официальный Registry требует отдельной эксплуатации
- API GitHub имеет ограничения по частоте запросов
- Блокировка номера версии может приводить к «потере» номеров

## Альтернативы

| Решение | Причина отказа |
|---------|----------------|
| Поддержка только GitHub | Зависимость от экосистемы GitHub, невозможность собственного Registry |
| crates.io в стиле Cargo | Избыточная сложность, не нужна на начальном этапе экосистемы YaoXiang |
| yank в стиле npm (только пометка) | Риск безопасности, известные случаи атак на цепочку поставок |

## Стратегия реализации

### Разделение на этапы

| Этап | Содержание |
|------|------------|
| Phase 3.5 | Перевод трейта Source на async + async-trait + миграция всех реализаций |
| Phase 4a | Трейт Registry + интеграция reqwest + локальный мок Registry |
| Phase 4b | Адаптер GitHub Release |
| Phase 4c | Команда publish + упаковка формата пакета |
| Phase 4d | Аутентификация + yank |

### Зависимости

- Зависит от RFC-014 Phase 3 (глобальный кэш, замена semver)
- Зависит от RFC-014b (система сборки, для обработки каталога `build/`)

## Открытые вопросы

- [ ] Нужна ли версионизация Registry API (`/api/v1/` против `/api/v2/`)?
- [ ] Поддерживать ли namespace для имён пакетов (например, `@org/pkg`)?
- [ ] Стратегия ограничения частоты запросов?
- [ ] Максимальный размер пакета?

---

## Ссылки

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)
```