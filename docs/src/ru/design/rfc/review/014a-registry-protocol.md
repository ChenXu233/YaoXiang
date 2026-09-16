---
title: 'RFC-014a: Спецификация протокола Registry'
status: 'На рассмотрении'
author: 'Чэньсюй'
created: '2026-06-11'
updated: '2026-09-15'
group: 'rfc-014'
---

# RFC-014a: Спецификация протокола Registry

> Данный RFC является дочерним RFC для
> [RFC-014: Проектирование системы управления пакетами](../accepted/014-package-manager.md).

## Решение по итогам ревью от 2026-09-15

Следующие решения были приняты владельцем 2026-09-15; соответствующие разделы основного текста
сохраняются как полная спецификация «после запуска официального Registry»:

1. **Сокращение области применения (самый важный пункт данного документа)**: сервер официального
   Registry, аутентификация (login/logout), yank **откладываются на неопределённый срок**.
   Фактический результат Phase 4 = адаптер GitHub Release/Git + фиксация trait Registry + упаковка
   `.yxpkg` + `publish --github`. Причина: для холодного старта экосистемы достаточно канала
   git/GitHub (аналогично ранней Go); эксплуатация сервера Registry, учётные записи и
   противодействие злоупотреблениям на этапе отсутствия сторонних пакетов — чистый долг.
2. **`add` с голым именем пакета недоступно**: до запуска официального Registry команда
   `yaoxiang add <голое имя пакета>` выдаёт ошибку; добавление зависимости требует явного указания
   источника (`--git` / `--path`). Цепочка поиска по умолчанию из раздела «Приоритет источников»
   вступает в силу с момента запуска Registry.
3. **Унификация формата пакета**: `.yxpkg` содержит только исходный код
   (`yaoxiang.toml`/`src/`/`build.yx`/`SHA256SUMS`); каталог прекомпилированных артефактов
   `build/native/` удаляется; распространение бинарных файлов выполняется исключительно через
   внешние ссылки `[binaries]` согласно RFC-014b (Release/CDN), что предотвращает раздувание размера
   пакета и глобального кэша.
4. **Реализация диспетчеризации Source**: встроенные четыре источника (Local/Git/Registry/GitHub)
   представляют собой закрытое множество; на уровне реализации используется диспетчеризация через
   enum (во избежание ограничений Send у dyn-async и зависимости от `async-trait`); определение
   `Source` trait сохраняется на семантическом уровне — при будущем открытии сторонних Source
   подключение будет выполнено через объекты trait.
5. **Версионирование API**: префикс пути URL `/api/v1/` + заголовок ответа с версией протокола;
   критические изменения ведут к v2 сосуществующей версии, без изменений на месте.
6. **Ограничение скорости**: адаптер GitHub использует экспоненциальный откат + условный кэш
   запросов на базе ETag; политика ограничения скорости на стороне Registry откладывается вместе с
   официальным Registry.
7. **Лимит размера пакета**: пакет с исходным кодом — 20 МиБ (начальное значение, настраивается);
   для канала GitHub ограничения определяются платформой.

## Резюме

Определение протокола Registry системы управления пакетами YaoXiang: проектирование открытого
интерфейса, спецификация официального Registry, адаптер GitHub, процедуры публикации/отзыва пакета,
модель аутентификации.

## Мотивация

Общий документ RFC-014 определяет общую архитектуру системы управления пакетами, однако раздел
Registry помечен лишь как «зарезервировано». Без протокола Registry пакеты невозможно распространять
— это как спроектировать корзину покупателя без магазина.

### Текущие проблемы

- `RegistrySource` является заглушкой (`source/mod.rs:150-203`), `resolve` напрямую возвращает
  объявленную версию, `download` возвращает пустой путь
- Отсутствует HTTP-клиент (нет зависимости `reqwest`)
- Нет механизма публикации пакетов
- Нет аутентификации/авторизации

## Предложение

### Основная идея: открытый протокол + адаптер

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
   │Официальн│ │ GitHub │ │Пользов │
   │ Registry│ │ адаптер│ │ Registry│
   └─────────┘ └────────┘ └────────┘
```

### Решение об асинхронной архитектуре

`Source` trait повсеместно переводится на async, с полным переходом на tokio:

```rust
// Текущий (синхронный) → меняется на (асинхронный)
#[async_trait]
pub trait Source: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> SourceKind;

    async fn resolve(&self, spec: &DependencySpec) -> PackageResult<String>;
    async fn download(&self, spec: &DependencySpec, dest: &Path) -> PackageResult<ResolvedPackage>;
}
```

Все реализации (`LocalSource`, `GitSource`, `RegistrySource`) единообразно переводятся на async.
Точка входа CLI управляется через `#[tokio::main]` или `Runtime::block_on`.

**Обоснование:**

- Registry требует HTTP-запросов, блокировка заморозит весь процесс установки
- Параллельная загрузка нескольких зависимостей (`join_all`) существенно ускоряет установку
- Git clone также является I/O-операцией — async здесь естественнее
- tokio уже присутствует в зависимостях проекта

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// Публикация пакета
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// Удаление опубликованной версии (необратимо, номер версии блокируется)
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

Порядок поиска по умолчанию при выполнении `yaoxiang add foo` (без флагов):

| Приоритет | Поиск                | Описание                                            |
| --------- | -------------------- | --------------------------------------------------- |
| 1         | Глобальный кэш       | `~/.yaoxiang/cache/registry/foo-<ver>/`             |
| 2         | Официальный Registry | Запрос версии → загрузка                            |
| 3         | Отказ                | Ошибка с предложением проверить имя пакета или сеть |

**Явное переопределение (минуя цепочку по умолчанию):**

| flag               | Поведение                                                                                             |
| ------------------ | ----------------------------------------------------------------------------------------------------- |
| `--git <url>`      | Пропустить Registry, выполнить Git clone напрямую (приоритет Release assets → fallback на tag/branch) |
| `--path <dir>`     | Пропустить Registry, использовать локальный путь напрямую                                             |
| `--registry <url>` | Пропустить официальный Registry, использовать указанный Registry                                      |

### Официальный Registry

Официальный Registry по аналогии с crates.io является основным каналом распространения пакетов.

**Конечные точки API:**

| Конечная точка                           | Метод  | Описание            |
| ---------------------------------------- | ------ | ------------------- |
| `/api/v1/packages/{name}`                | GET    | Информация о пакете |
| `/api/v1/packages/{name}/versions`       | GET    | Список версий       |
| `/api/v1/packages/{name}/{version}`      | GET    | Загрузка пакета     |
| `/api/v1/packages`                       | PUT    | Публикация пакета   |
| `/api/v1/packages/{name}/{version}/yank` | DELETE | Отзыв версии        |
| `/api/v1/search?q={query}`               | GET    | Поиск пакетов       |
| `/api/v1/login`                          | POST   | Аутентификация      |

### Интеграция с GitHub

При использовании GitHub в качестве источника пакетов применяется стратегия в стиле Go modules:

1. **Приоритет Release assets**: проверка страницы GitHub Release на наличие прекомпилированных
   артефактов для целевой платформы
2. **Fallback на ветку main**: при отсутствии Release выполняется git clone

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

> Решение от 2026-09-15: только исходный код, каталог прекомпилированных артефактов `build/` удалён
> — бинарные файлы распространяются исключительно через внешние ссылки `[binaries]` согласно
> RFC-014b.

```
foo-1.2.3.yxpkg (tar.gz)
├── yaoxiang.toml          # Метаданные пакета
├── src/                   # Исходный код
├── build.yx               # Скрипт сборки (при наличии)
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

- Пакет полностью удаляется без возможности восстановления
- Номер версии остаётся занят навсегда, повторная публикация под тем же номером невозможна
- Существующие lockfile, ссылающиеся на эту версию, будут выдавать ошибку — требуется обновление до
  другой версии
- **Цель безопасности**: предотвращение атак на цепочку поставок в стиле npm. Злоумышленники ранее
  перехватывали номера версий удалённых пакетов для внедрения вредоносного кода; блокировка номера
  версии при yank полностью закрывает этот вектор

### Модель аутентификации

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**Правило сопоставления:** `yaoxiang login --registry <url>` сопоставляет поле `url` в разделе
`[registries.*]`. При отсутствии совпадения создаётся новая запись (имя генерируется автоматически,
например `reg-1`).

**Приоритет:** переменные среды > файл конфигурации

| Переменная среды     | Назначение                                          |
| -------------------- | --------------------------------------------------- |
| `$YX_GITHUB_TOKEN`   | Аутентификация GitHub                               |
| `$YX_REGISTRY_TOKEN` | Аутентификация Registry (для Registry по умолчанию) |
| `$YX_REGISTRY_URL`   | Адрес Registry по умолчанию                         |

**Команды CLI:**

```bash
yaoxiang login --registry https://yxreg.example.com   # Сопоставление по URL или создание новой записи
yaoxiang login --github                                # OAuth или токен GitHub
yaoxiang logout --registry https://yxreg.example.com   # Удаление соответствующей записи
```

**Ограничения безопасности:**

- Токены никогда не записываются в `yaoxiang.toml` или `yaoxiang.lock`
- Права доступа к файлу `credentials.toml` — 600
- Для CI используются переменные среды, для разработки — файл

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

| crate            | Назначение               |
| ---------------- | ------------------------ |
| `reqwest`        | HTTP-клиент              |
| `sha2`           | Проверка SHA-256         |
| `flate2` + `tar` | Обработка формата пакета |
| `async-trait`    | Поддержка async trait    |

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

- Открытый протокол, не привязан к конкретному серверу
- GitHub как легковесный канал распространения снижает порог входа
- Модель безопасности с блокировкой номера версии
- Стратегия установки с приоритетом прекомпиляции

### Недостатки

- Официальный Registry требует отдельной эксплуатации
- API GitHub имеет ограничения по скорости
- Блокировка номера версии может приводить к расходу номеров

## Альтернативы

| Вариант                              | Почему не выбран                                                    |
| ------------------------------------ | ------------------------------------------------------------------- |
| Только GitHub                        | Привязка к экосистеме GitHub, невозможность собственного Registry   |
| crates.io в стиле Cargo              | Излишне сложно, не требуется на начальном этапе экосистемы YaoXiang |
| yank в стиле npm (только маркировка) | Риск безопасности, известные случаи атак на цепочку поставок        |

## Стратегия реализации

### Разбиение на этапы

| Этап      | Содержание                                                     |
| --------- | -------------------------------------------------------------- |
| Phase 3.5 | Source trait на async + async-trait + миграция всех реализаций |
| Phase 4a  | Registry trait + интеграция reqwest + локальный мок Registry   |
| Phase 4b  | Адаптер GitHub Release                                         |
| Phase 4c  | Команда publish + упаковка формата пакета                      |
| Phase 4d  | Аутентификация + yank                                          |

### Зависимости

- Зависит от RFC-014 Phase 3 (глобальный кэш, замена semver)
- Зависит от RFC-014b (система сборки, для обработки каталога `build/`)

## Открытые вопросы

- [x] Требуется ли версионирование API Registry (`/api/v1/` против `/api/v2/`)? → URL `/api/v1/` +
      заголовок ответа с версией, критические изменения ведут к сосуществующей v2 (2026-09-15)
- [x] Поддерживаются ли пространства имён в именах пакетов (например, `@org/pkg`)? → На начальном
      этапе не поддерживаются, плоские имена пакетов (2026-09-15, см. общий документ)
- [x] Стратегия ограничения скорости? → Откат + кэш в адаптере GitHub; на стороне Registry —
      откладывается вместе с официальным Registry (2026-09-15)
- [x] Лимит размера пакета? → Пакет с исходным кодом 20 МиБ как начальное значение (2026-09-15,
      настраивается)

---

## Список литературы

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)
