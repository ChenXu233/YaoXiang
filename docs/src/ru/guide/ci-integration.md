---
title: 'Руководство по интеграции CI'
description: 'Интеграция yx check и yx format в конвейер CI/CD'
---

# Руководство по интеграции CI

Интеграция инструментов статической проверки и форматирования YaoXiang в конвейер CI/CD для
обеспечения качества кода.

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
          curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
          echo "$HOME/.yaoxiang/bin" >> $GITHUB_PATH

      - name: Type check
        run: yx check --color never --no-progress

      - name: Format check
        run: yx format --dry-run .
```

## GitLab CI

```yaml
yaoxiang-check:
  image: rust:latest
  script:
    - curl -fsSL
      https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
    - export PATH="$HOME/.yaoxiang/bin:$PATH"
    - yx check --color never --no-progress
    - yx format --dry-run .
  rules:
    - if: $CI_MERGE_REQUEST_IID
    - if: $CI_COMMIT_BRANCH == "main"
    - if: $CI_COMMIT_BRANCH == "dev"
```

## Коды выхода

| Код выхода | Значение                                                                                     | Поведение CI    |
| ---------- | -------------------------------------------------------------------------------------------- | --------------- |
| `0`        | Ошибок нет                                                                                   | Успех           |
| `1`        | При проверке обнаружены ошибки; либо есть предупреждения при использовании `--deny-warnings` | Ошибка          |
| `2`        | Файлы `.yx` не найдены                                                                       | По конфигурации |

## Разбор JSON-вывода

Используйте `--json` для получения машиночитаемого вывода:

```bash
yx check --json | jq '.error_count'
```

## Рекомендации

1. **Параметр пути**: `yx check` по умолчанию проверяет текущий каталог, также можно указать путь:
   `yx check src/`
2. **Разделяйте проверку и форматирование**: запускайте `check` и `format --dry-run` отдельно для
   удобства локализации проблем
3. **Используйте `--no-progress`**: в среде CI индикатор прогресса не нужен
4. **Используйте `--color never`**: чтобы ANSI-коды цветов не загрязняли логи
5. **Строгий режим**: добавьте `--deny-warnings`, чтобы предупреждения также приводили к ошибке CI
6. **Кэшируйте зависимости**: используйте механизмы кэширования CI для ускорения сборки
