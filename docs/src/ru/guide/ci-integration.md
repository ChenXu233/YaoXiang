---
title: Руководство по интеграции CI
description: Интеграция yaoxiang check и yaoxiang format в конвейеры CI/CD
---

# Руководство по интеграции CI

Интеграция инструментов статической проверки и форматирования YaoXiang в конвейеры CI/CD для обеспечения качества кода.

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

## Коды завершения

| Код завершения | Значение              | Действие CI |
| -------------- | --------------------- | ----------- |
| `0`            | Ошибок нет            | Успешно     |
| `1`            | Обнаружены ошибки     | Ошибка      |
| `2`            | Файлы `.yx` не найдены | Зависит от конфигурации |

## Разбор вывода в формате JSON

Используйте `--json` для получения вывода в машиночитаемом формате:

```bash
yaoxiang check --json | jq '.error_count'
```

## Рекомендации

1. **Параметры пути**: `yaoxiang check` по умолчанию проверяет текущий каталог, также можно указать путь: `yaoxiang check src/`
2. **Разделение проверки и форматирования**: запускайте `check` и `format --dry-run` отдельно для удобства поиска проблем
3. **Используйте `--no-progress`**: в среде CI индикатор прогресса не нужен
4. **Используйте `--color never`**: избегайте ANSI-кодов цвета в логах
5. **Кэшируйте зависимости**: используйте механизм кэширования CI для ускорения сборки
