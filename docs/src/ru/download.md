---
layout: page
is_download: true

title: 'ВВЕДИ ТИП ВСЕЛЕННОЙ'
description: 'Выберите свою платформу и начните создавать миры.'

download:
  latest_stable: 'Последняя стабильная версия v{version}'
  quick_install: 'Быстрая установка'
  copy: 'Копировать'
  copied: 'Скопировано!'
  supported: 'Поддерживаемые платформы: Windows (PowerShell), macOS, Linux (x64/ARM64)'
  download_btn: 'Скачать'
  coming_soon: 'Скоро появится'
  checksum: 'Файлы контрольных сумм / Подписи'
  build_from_source:
    title: 'Сборка из исходного кода'
    description:
      'Соберите YaoXiang из исходного кода с помощью Cargo. Убедитесь, что Rust установлен.'
  nightly_builds:
    title: 'Ночные сборки'
    description:
      'Доступны самые свежие передовые версии. Рекомендуется для тестирования, используйте с
      осторожностью в продакшене.'
  github_actions: 'Перейти в GitHub Actions'

versions:
  - version: 0.7.14
    latest: true
    # Установка одной командой (канал для чайников из RFC-037); начиная с версии 0.8.0 вступает в силу новый формат пакета
    install_command:
      'curl -fsSL
      https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh'
    downloads:
      - os: Windows
        arch: x64
        features: ['Бинарник командной строки (старый формат, без libz3)', 'Не требует установки']
        links:
          - name: 'Windows x64 (.exe)'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-x86_64-pc-windows-gnu.exe'
      - os: Windows
        arch: ARM64
        features: ['Бинарник командной строки (старый формат, без libz3)', 'Не требует установки']
        links:
          - name: 'Windows ARM64 (.exe)'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-aarch64-pc-windows-gnullvm.exe'
      - os: Linux
        arch: x64
        features:
          [
            'Бинарник командной строки (старый формат, статически слинкован с Z3)',
            'Не требует установки',
          ]
        links:
          - name: 'Linux x64'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-x86_64-unknown-linux-gnu'
      - os: Linux
        arch: ARM64
        features:
          [
            'Бинарник командной строки (старый формат, статически слинкован с Z3)',
            'Не требует установки',
          ]
        links:
          - name: 'Linux ARM64'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-aarch64-unknown-linux-gnu'
      - os: macOS
        arch: Intel / Apple Silicon
        features:
          [
            'Бинарник командной строки (старый формат, статически слинкован с Z3)',
            'Не требует установки',
          ]
        links:
          - name: 'macOS Intel'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-x86_64-apple-darwin'
          - name: 'macOS Apple Silicon'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-aarch64-apple-darwin'
sidebar: false
---
