---
title: Руководство по интеграции CI
description: Интеграция yaoxiang check и yaoxiang format в конвейер CI/CD
---

# Руководство по интеграции CI

Интеграция инструментов статической проверки и форматирования YaoXiang в конвейер CI/CD для обеспечения качества кода.

## GitHub Actions

```yaml
name: YaoXiang CI

on:
  push:
    branches: [main, dev]
  pull_request:
    branches: [main]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install YaoXiang
        run: |
          curl -fsSL https://yaoxiang.dev/install.sh | sh
          echo "$HOME/.yaoxiang/bin" >> $GITHUB_PATH

      - name: Type check
        run: yaoxiang check --color never --no-progress

      - name: Format check
        run: yaoxiang format --dry-run .
```

## GitLab CI

```yaml
yaoxiang-check:
  image: rust:latest
  script:
    - curl -fsSL https://yaoxiang.dev/install.sh | sh
    - export PATH="$HOME/.yaoxiang/bin:$PATH"
    - yaoxiang check --color never --no-progress
    - yaoxiang format --dry-run .
  rules:
    - if: $CI_MERGE_REQUEST_IID
    - if: $CI_COMMIT_BRANCH == "main"
    - if: $CI_COMMIT_BRANCH == "dev"
```

## Коды выхода

| Код выхода | Значение | Поведение CI |
|--------|------|---------|
| `0` | Нет ошибок | Успешно |
| `1` | Проверка обнаружила ошибки | Сбой |
| `2` | Файл `.yx` не найден | Зависит от конфигурации |

## Разбор JSON-вывода

Используйте флаг `--json` для получения машиночитаемого вывода:

```bash
yaoxiang check --json | jq '.error_count'
```

## Лучшие практики

1. **Параметр пути**: `yaoxiang check` по умолчанию проверяет текущий каталог, но также можно указать путь: `yaoxiang check src/`
2. **Разделение проверки и форматирования**: запускайте `check` и `format --dry-run` отдельно — это упрощает локализацию проблем
3. **Используйте `--no-progress`**: в среде CI индикатор прогресса не нужен
4. **Используйте `--color never`**: во избежание загрязнения логов ANSI-кодами цветов
5. **Кэширование зависимостей**: используйте механизмы кэширования CI для ускорения сборки