> **⚠️ Внимание: данный документ устарел и предназначен только для справки.**
>
> Описанное в документе больше не актуально, см. актуальную документацию.

# План реструктуризации каталога документации YaoXiang

## Обзор

**Цель**: реструктуризация каталога `docs/`, создание отдельных **раздела дизайна** и **каталога отслеживания планов реализации**

---

## Целевая структура каталогов

```
docs/
├── design/                    # ⭐ Раздел дизайна (новый основной каталог)
│   ├── README.md              # Индекс документации по дизайну
│   ├── manifesto.md           # Дизайнерский манифест
│   ├── language-spec.md       # Спецификация языка
│   ├── async-whitepaper.md    # Асинхронная белая книга
│   ├── 00-wtf.md              # Компромиссы в дизайне (FAQ)
│   └── 01-philosophy.md       # Философия дизайна (бывший "Взгляд на дизайн языка от человека, родившегося в 2006 году")
│
├── design/rfc/                # Проектирование в стиле RFC (опционально)
│   └── (будущие предложения)
│
├── design/discussion/         # Открытая зона обсуждения (черновики)
│   └── (документы дизайна на обсуждении)
│
├── plans/                     # ⭐ Планы реализации (перенесено из works/plans)
│   ├── README.md
│   ├── book-improvement.md
│   ├── stdlib-implementation.md
│   ├── test-organization.md
│   └── async/
│       ├── implementation-plan.md
│       └── threading-safety.md
│
├── implementation/            # ⭐ Отслеживание реализации (новое)
│   ├── README.md
│   ├── phase1/
│   │   └── type-check-inference.md
│   └── phase5/
│       ├── bytecode-generation.md
│       └── gap-analysis.md
│
├── architecture/              # Архитектурный дизайн (сохранено)
├── guides/                    # Руководства по использованию (сохранено)
├── examples/                  # Примеры кода (сохранено)
└── reference/                 # Справочная документация (сохранено)
```

---

## Описание обязанностей каталогов

| Каталог | Обязанность | Тип содержимого |
|---------|-------------|-----------------|
| `design/` | Завершённые обсуждения дизайнерских решений | Манифесты, спецификации, белые книги, компромиссы дизайна |
| `design/rfc/` | Проектирование в стадии предложений | RFC документы, черновики |
| `design/discussion/` | Дизайн на обсуждении | Открытые вопросы, черновики на обсуждении |
| `plans/` | Планы реализации | Дорожные карты реализации, разбиение задач |
| `implementation/` | Детали реализации (завершённой/текущей) | Технические детали, отчёты по фазам |

---

## Чек-лист миграции

### 1. Переместить в `design/`

| Старое расположение | Новое расположение |
|--------------------|--------------------|
| `docs/YaoXiang-design-manifesto.md` | `docs/design/manifesto.md` |
| `docs/YaoXiang-language-specification.md` | `docs/design/language-spec.md` |
| `docs/YaoXiang-async-whitepaper.md` | `docs/design/async-whitepaper.md` |
| `docs/YaoXiang-WTF.md` | `docs/design/00-wtf.md` |
| `docs/一个2006年出生者的语言设计观.md` | `docs/design/01-philosophy.md` |

### 2. Переместить `works/plans/` на корневой уровень

| Старое расположение | Новое расположение |
|--------------------|--------------------|
| `docs/plans/` | `docs/plans/` |

### 3. Переместить в `implementation/`

| Старое расположение | Новое расположение |
|--------------------|--------------------|
| `docs/works/phase/phase1/type-check-inference-rules.md` | `docs/implementation/phase1/type-check-inference.md` |
| `docs/works/phase/phase5/phase5-bytecode-generation.md` | `docs/implementation/phase5/bytecode-generation.md` |
| `docs/works/phase/phase5/phase5-implementation-gap-analysis.md` | `docs/implementation/phase5/gap-analysis.md` |

### 4. Оставить без изменений

| Каталог | Описание |
|---------|----------|
| `docs/architecture/` | Архитектурный дизайн уже вынесен, без изменений |
| `docs/guides/` | Руководства пользователя уже вынесены, без изменений |
| `docs/examples/` | Примеры кода, без изменений |
| `docs/works/old/` | Исторический архив, оставить или удалить |
| `docs/plans/async/` | Уже перенесено в `plans/async/` |

### 5. Опционально: обновить `docs/README.md`

Необходимо обновить индекс документации для отражения новой структуры каталогов.

---

## Шаги выполнения

### Шаг 1: Создание структуры каталогов

```bash
mkdir -p docs/design/discussion
mkdir -p docs/design/rfc
mkdir -p docs/plans/async
mkdir -p docs/implementation/phase1
mkdir -p docs/implementation/phase5
```

### Шаг 2: Перемещение документации по дизайну

```bash
# Перемещение в design/
mv docs/YaoXiang-design-manifesto.md docs/design/manifesto.md
mv docs/YaoXiang-language-specification.md docs/design/language-spec.md
mv docs/YaoXiang-async-whitepaper.md docs/design/async-whitepaper.md
mv docs/YaoXiang-WTF.md docs/design/00-wtf.md
mv "docs/一个2006年出生者的语言设计观.md" docs/design/01-philosophy.md

# Перемещение в design/discussion/ (опционально: для черновиков на обсуждении)
```

### Шаг 3: Перенос каталога plans

```bash
# Перенос works/plans на корневой уровень
mv docs/plans/* docs/plans/
rmdir docs/works/plans
```

### Шаг 4: Перемещение документации по реализации

```bash
# Перемещение в implementation/
mv docs/works/phase/phase1/type-check-inference-rules.md docs/implementation/phase1/type-check-inference.md
mv docs/works/phase/phase5/phase5-bytecode-generation.md docs/implementation/phase5/bytecode-generation.md
mv docs/works/phase/phase5/phase5-implementation-gap-analysis.md docs/implementation/phase5/gap-analysis.md
```

### Шаг 5: Обновление docs/README.md

Обновить индекс документации, добавить описание новых каталогов.

### Шаг 6: Очистка пустых каталогов

```bash
rmdir docs/works/phase/phase5
rmdir docs/works/phase/phase1
rmdir docs/works/phase
rmdir docs/works/old/archived
rmdir docs/works/old
```

---

## Обратная совместимость

⚠️ **Важно**: данная реструктуризация нарушит существующие ссылки, рекомендуется:

1. **Не удалять оригинальные файлы**, сначала создать символические ссылки или переместить и проверить
2. **Обновить все внутренние ссылки**: проверить относительные пути в `docs/**/*.md`
3. **Обновить конфигурацию IDE**: если существуют `.vscode` или другие конфигурации

---

## Ожидаемые преимущества

1. **Чёткое разграничение ответственности**: дизайн vs план vs реализация, границы чёткие
2. **Удобство доступа**: `design/` и `plans/` на корневом уровне, не нужно углубляться в `works/`
3. **Масштабируемость**: новые `design/rfc/` и `design/discussion/` поддерживают RFC-процесс
4. **Чёткая типизация документов**: завершённый дизайн, дизайн на обсуждении, планы реализации, отслеживание реализации — каждая категория на своём месте

---

## Примечания

- Подтвердить необходимость сохранения архивного содержимого каталога `works/`
- Проверить наличие других документов, ссылающихся на эти пути к файлам
- Рассмотреть необходимость создания шаблона RFC для `design/rfc/`