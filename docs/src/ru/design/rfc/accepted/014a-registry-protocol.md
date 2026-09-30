---
title: 'RFC-014a: Спецификация протокола Registry'
status: 'Принято'
author: 'Чэньсюй'
created: '2026-06-11'
updated: '2026-09-29'
group: 'rfc-014'
---

# RFC-014a: Спецификация протокола Registry

> Этот RFC является под-RFC для
> [RFC-014: Дизайн системы управления пакетами](../accepted/014-package-manager.md).

## Решение проверки 2026-09-15

Следующие решения были приняты владельцем 2026-09-15, соответствующие разделы основного текста
сохраняются как полная спецификация «после запуска официального Registry»:

1. **Сокращение области (самое важное в этом документе)**: сервер официального Registry,
   аутентификация (login/logout), yank **отложены на неопределённый срок**. Фактический результат
   Phase 4 = адаптер GitHub Release/Git + утверждение Registry trait + упаковка `.yxpkg` +
   `publish --github`. Причина: для холодного старта экосистемы достаточно каналов git/GitHub (так
   же было у Go на раннем этапе); стоимость эксплуатации сервера Registry, системы аккаунтов и
   борьбы со злоупотреблениями на этапе отсутствия сторонних пакетов — чистый долг.
2. **Голое имя пакета для add недоступно**: до запуска официального Registry
   `yaoxiang add <голое имя пакета>` будет возвращать ошибку, добавление зависимостей требует явного
   источника (`--git` / `--path`). Цепочка поиска по умолчанию «Приоритет источников» ниже вступает
   в силу с момента запуска Registry.
3. **Унификация формата пакета**: `.yxpkg` содержит только исходный код
   (`yaoxiang.toml`/`src/`/`build.yx`/`SHA256SUMS`), каталог скомпилированных артефактов
   `build/native/` удаляется; распространение бинарных файлов всегда идёт через внешние ссылки
   `[binaries]` из RFC-014b (Release/CDN), чтобы избежать раздувания размера пакета и глобального
   кэша.
4. **Реализация диспетчеризации Source**: встроенные четыре источника (Local/Git/Registry/GitHub) —
   закрытый набор, на уровне реализации используется enum для диспетчеризации (чтобы избежать
   ограничений Send для dyn-async и зависимости `async-trait`); определение trait `Source`
   сохраняется на семантическом уровне, если в будущем откроется сторонний Source, он будет
   подключён через объект trait.
5. **Версионирование API**: URL-путь `/api/v1/` + заголовок ответа несёт версию протокола; ломающие
   изменения повышают до v2 со сосуществованием, без изменений на месте.
6. **Ограничение скорости**: адаптер GitHub — экспоненциальный откат + кэш условных запросов ETag;
   стратегия ограничения скорости на стороне Registry откладывается вместе с официальным Registry.
7. **Лимит размера пакета**: пакет с исходным кодом 20 МиБ (начальное значение, настраиваемое);
   канал GitHub управляется самой платформой.

## Резюме

Определяет протокол Registry системы управления пакетами YaoXiang: дизайн открытого интерфейса,
спецификация официального Registry, адаптер GitHub, процесс публикации/отзыва пакетов, модель
аутентификации.

## Мотивация

Общий документ RFC-014 определяет общую архитектуру системы управления пакетами, но раздел Registry
отмечен только как «зарезервировано». Без протокола Registry пакеты не могут распространяться — это
как спроектировать корзину покупок без магазина.

### Текущая проблема

- `RegistrySource` — это заглушка (`source/mod.rs:150-203`), `resolve` напрямую возвращает
  заявленную версию, `download` возвращает пустой путь
- Нет HTTP-клиента (нет зависимости `reqwest`)
- Нет механизма публикации пакетов
- Нет аутентификации/авторизации

## Предложение

### Основной дизайн: открытый протокол + адаптер

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
   │Офици-   │ │ GitHub │ │Пользо- │
   │альный   │ │адаптер │ │ватель- │
   │Registry │ │        │ │ский    │
   │         │ │        │ │Registry│
   └─────────┘ └────────┘ └────────┘
```

### Решение об асинхронной архитектуре

`Source` trait единообразно переводится на async, полностью принимая tokio:

```rust
// 现有（同步）→ 改为（异步）
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

**Причины:**

- Registry требует HTTP-запросов, блокировка заморозит весь процесс установки
- Параллельное скачивание нескольких зависимостей (`join_all`) значительно ускоряет установку
- Git clone тоже I/O-операция, async естественнее
- tokio уже в зависимостях проекта

### Registry Trait

```rust
#[async_trait]
trait Registry: Send + Sync {
    /// 发布包
    async fn publish(&self, package: &PackageManifest, artifact: &Path) -> PackageResult<()>;

    /// 删除已发布版本（不可恢复，版本号锁死）
    async fn yank(&self, name: &str, version: &Version) -> PackageResult<()>;

    /// 查询包信息
    async fn info(&self, name: &str) -> PackageResult<PackageInfo>;

    /// 查询可用版本列表
    async fn versions(&self, name: &str) -> PackageResult<Vec<Version>>;

    /// 搜索包
    async fn search(&self, query: &str) -> PackageResult<Vec<PackageSummary>>;

    /// 下载指定版本
    async fn download(&self, name: &str, version: &Version) -> PackageResult<PathBuf>;

    /// 认证
    async fn authenticate(&self, credentials: &Credentials) -> PackageResult<()>;
}
```

### Приоритет источников (цепочка поиска по умолчанию)

Порядок поиска по умолчанию при `yaoxiang add foo` (без флагов):

| Приоритет | Поиск                | Описание                                                     |
| --------- | -------------------- | ------------------------------------------------------------ |
| 1         | Глобальный кэш       | `~/.yaoxiang/cache/registry/foo-<ver>/`                      |
| 2         | Официальный Registry | Запрос версий → скачивание                                   |
| 3         | Ошибка               | Сообщить об ошибке, предложить проверить имя пакета или сеть |

**Явное переопределение (минуя цепочку по умолчанию):**

| flag               | Поведение                                                                                   |
| ------------------ | ------------------------------------------------------------------------------------------- |
| `--git <url>`      | Пропустить Registry, напрямую Git clone (приоритет Release assets → fallback на tag/branch) |
| `--path <dir>`     | Пропустить Registry, напрямую использовать локальный путь                                   |
| `--registry <url>` | Пропустить официальный Registry, использовать указанный Registry                            |

### Официальный Registry

Официальный Registry аналогичен crates.io и является основным каналом распространения пакетов.

**Конечные точки API:**

| Конечная точка                           | Метод  | Описание            |
| ---------------------------------------- | ------ | ------------------- |
| `/api/v1/packages/{name}`                | GET    | Информация о пакете |
| `/api/v1/packages/{name}/versions`       | GET    | Список версий       |
| `/api/v1/packages/{name}/{version}`      | GET    | Скачать пакет       |
| `/api/v1/packages`                       | PUT    | Опубликовать пакет  |
| `/api/v1/packages/{name}/{version}/yank` | DELETE | Отозвать версию     |
| `/api/v1/search?q={query}`               | GET    | Поиск пакетов       |
| `/api/v1/login`                          | POST   | Аутентификация      |

### Интеграция с GitHub

При использовании GitHub в качестве источника пакетов применяется стратегия в стиле Go modules:

1. **Приоритет Release assets**: проверить страницу GitHub Release на наличие скомпилированных
   артефактов для соответствующей платформы
2. **Fallback на ветку main**: если Release нет — git clone

```toml
[dependencies]
# 基本 git 依赖
foo = { git = "https://github.com/user/foo" }

# 指定版本（匹配 tag）
bar = { git = "https://github.com/user/bar", version = "^1.0.0" }

# 指定分支
baz = { git = "https://github.com/user/baz", branch = "main" }

# 指定 commit
qux = { git = "https://github.com/user/qux", rev = "abc123" }

# 私有仓库（使用 credentials.toml 中的 GitHub token）
private = { git = "https://github.com/my-org/private-lib" }
```

### Формат пакета (.yxpkg)

> Решение от 2026-09-15: содержит только исходный код, скомпилированные артефакты `build/` удалены —
> бинарные файлы всегда распространяются через внешние ссылки `[binaries]` в RFC-014b.

```
foo-1.2.3.yxpkg (tar.gz)
├── yaoxiang.toml          # метаданные пакета
├── src/                   # исходный код
├── build.yx               # сценарий сборки (если есть)
└── SHA256SUMS             # контрольные суммы
```

### Процесс публикации

```bash
# 发布到官方 Registry
yaoxiang publish

# 发布到指定 Registry
yaoxiang publish --registry my-company

# 同时创建 GitHub Release
yaoxiang publish --github

# 干跑
yaoxiang publish --dry-run
```

Проверки перед публикацией:

1. `yaoxiang.toml` должен содержать `name`, `version`, `description`
2. Номер версии не должен существовать
3. Запустить тесты (опционально, `--no-test` для пропуска)
4. Вычислить SHA-256 всех файлов
5. Упаковать в `.yxpkg` (tar.gz)
6. Загрузить в Registry

### Семантика yank

```bash
yaoxiang yank foo@1.2.3
```

**Удаление + блокировка номера версии:**

- Пакет полностью удалён, невосстановимо
- Номер версии занят навсегда, нельзя повторно опубликовать ту же версию
- Проекты, у которых lockfile ссылается на эту версию, получат ошибку и должны будут обновиться до
  другой версии
- **Цель безопасности**: предотвращение атак на цепочку поставок в стиле npm. Злоумышленники
  захватывали номера версий удалённых пакетов для внедрения вредоносного кода; блокировка номера
  версии в yank полностью закрывает этот путь.

### Модель аутентификации

```toml
# ~/.yaoxiang/credentials.toml
[github]
token = "ghp_xxxx"

[registries.my-company]
url = "https://yxreg.my-company.com"
token = "xxx"
```

**Правило сопоставления:** `yaoxiang login --registry <url>` сопоставляет по URL поле `url` в
`[registries.*]`. Если совпадения нет, создаётся новая запись (имя генерируется автоматически,
например `reg-1`).

**Приоритет:** переменные окружения > конфигурационный файл

| Переменная окружения | Назначение                                          |
| -------------------- | --------------------------------------------------- |
| `$YX_GITHUB_TOKEN`   | Аутентификация GitHub                               |
| `$YX_REGISTRY_TOKEN` | Аутентификация Registry (для Registry по умолчанию) |
| `$YX_REGISTRY_URL`   | Адрес Registry по умолчанию                         |

**Команды CLI:**

```bash
yaoxiang login --registry https://yxreg.example.com   # 按 URL 匹配或新建
yaoxiang login --github                                # GitHub OAuth 或 token
yaoxiang logout --registry https://yxreg.example.com   # 删除匹配的条目
```

**Ограничения безопасности:**

- Токен никогда не записывается в `yaoxiang.toml` или `yaoxiang.lock`
- Права на файл `credentials.toml` — 600
- Для CI используются переменные окружения, для разработки — файл

## Подробный дизайн

### Реализация RegistrySource

Заменить существующую заглушку (`source/mod.rs:150-203`):

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

        // SHA-256 校验
        let actual_hash = sha256_hex(&bytes);
        // ... 解压到 dest ...

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

| crate             | Назначение                                                                                                      |
| ----------------- | --------------------------------------------------------------------------------------------------------------- |
| `reqwest`         | HTTP-клиент                                                                                                     |
| `sha2`            | SHA-256 проверка                                                                                                |
| `flate2` + `tar`  | Обработка формата пакета                                                                                        |
| ~~`async-trait`~~ | Отменено — по решению 4 применяется enum-диспетчеризация + нативный async fn in trait, зависимость не требуется |

### Типы ошибок

```rust
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("包 '{0}' 不存在")]
    PackageNotFound(String),

    #[error("版本 '{0}' 不存在")]
    VersionNotFound(String),

    #[error("版本 '{0}' 已被占用")]
    VersionAlreadyExists(String),

    #[error("认证失败: {0}")]
    AuthFailed(String),

    #[error("网络错误: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("SHA-256 校验失败: 期望 {expected}, 实际 {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("权限不足: {0}")]
    Forbidden(String),
}
```

## Компромиссы

### Преимущества

- Открытый протокол, не привязан к конкретному серверу
- GitHub как лёгкий канал распространения снижает порог входа
- Модель безопасности с блокировкой номера версии
- Стратегия установки с приоритетом прекомпиляции

### Недостатки

- Официальный Registry требует отдельной эксплуатации
- У GitHub API есть ограничения скорости
- Блокировка номера версии может приводить к «растрате» номеров версий

## Альтернативы

| Вариант                         | Почему не выбран                                               |
| ------------------------------- | -------------------------------------------------------------- |
| Поддержка только GitHub         | Привязка к экосистеме GitHub, невозможно создать свой Registry |
| crates.io в стиле Cargo         | Слишком сложно, не нужно на раннем этапе экосистемы YaoXiang   |
| yank в стиле npm (только метка) | Риск безопасности, известные случаи атак на цепочку поставок   |

## Стратегия реализации

### Разбивка на этапы

| Этап      | Содержание                                                                                                                                                                                                                                                                       | Статус                                                                     |
| --------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Phase 3.5 | Перевод диспетчеризации Source на enum (решение 4) + нативный async fn in trait + миграция всех реализаций                                                                                                                                                                       | ✅ Завершён                                                                |
| Phase 4a  | Адаптер GitHub (исходный «Registry trait + reqwest + локальный mock» сокращён по решению 1: маршрутизация git-зависимостей github.com через `GitHubSource`, разбор API + скачивание артефактов `.yxpkg` + git fallback; экспоненциальный откат + условный кэш ETag по решению 6) | ✅ Завершён                                                                |
| Phase 4b  | Формат пакета `.yxpkg` (tar.gz + SHA256SUMS + лимит 20 МиБ, решения 3/7; детерминированная упаковка, принудительная проверка при распаковке)                                                                                                                                     | ✅ Завершён                                                                |
| Phase 4c  | Команда publish (`--dry-run` полный локальный прогон; `--github` проверка дубликатов → проверка tag → Release → загрузка артефактов; замена ссылок workspace 6d при упаковке)                                                                                                    | ✅ Завершён                                                                |
| Phase 4d  | Аутентификация (login/logout/credentials.toml) + yank                                                                                                                                                                                                                            | Отложено на неопределённый срок (вместе с официальным Registry, решение 1) |

**Заметки по реализации (2026-09-29)**:

- HTTP-стек: reqwest (rustls, без кросс-компиляции OpenSSL) + собственный tokio current_thread
  рантайм пакетного менеджера (`package::runtime::drive`); POST-запросы (создание Release/загрузка
  артефактов) не подвергаются автоматическим повторам — они не идемпотентны, повтор при 5xx может
  привести к дублированию.
- Резолвинг целевого репозитория для publish: `[package].repository` в приоритете, fallback на
  `git remote origin`; требуется, чтобы tag уже существовал (семантика как в Cargo: publish не
  создаёт tag).
- Прогон тестов перед публикацией (шаг 3 контрольного списка выше) подключён (2026-09-30, вместе с
  RFC-014b): по умолчанию запускаются тесты, обнаруженные `[tool.test]`, при провале публикация
  прерывается, `--no-test` пропускает.
- `credentials.toml` и команды `login`/`logout`/`yank` отложены вместе с официальным Registry;
  сейчас аутентификация — только переменная окружения `$YX_GITHUB_TOKEN` (правило приоритета без
  изменений: переменная окружения > конфигурационный файл).

### Зависимости

- Зависит от RFC-014 Phase 3 (глобальный кэш, подстановка semver)
- Зависит от RFC-014b (система сборки, для обработки каталога `build/`)

## Открытые вопросы

- [x] Нужно ли версионировать Registry API (`/api/v1/` vs `/api/v2/``)? → URL `/api/v1/` + заголовок
      версии в ответе, ломающие изменения повышают до v2 со сосуществованием (2026-09-15)
- [x] Поддерживать ли namespace в именах пакетов (например, `@org/pkg`)? → На начальном этапе не
      поддерживается, плоские имена пакетов (2026-09-15, см. общий документ)
- [x] Стратегия ограничения скорости? → Адаптер GitHub — откат + кэш; на стороне Registry —
      откладывается вместе с официальным Registry (2026-09-15)
- [x] Лимит размера пакета? → Пакет с исходным кодом 20 МиБ как начальное значение (2026-09-15,
      настраиваемо)

---

## Ссылки

- [crates.io API](https://crates.io/)
- [Go Module Proxy Protocol](https://go.dev/ref/mod#module-proxy)
- [npm Registry API](https://github.com/npm/registry/blob/main/docs/REGISTRY-API.md)
- [GitHub Packages](https://docs.github.com/en/packages)
