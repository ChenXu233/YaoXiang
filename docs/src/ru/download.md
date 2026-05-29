---
layout: page
is_download: true

title: ТИПИРУЙ ВСЕЛЕННУЮ
description: "Выберите вашу платформу и начните создавать мир."

download:
  latest_stable: "Последняя стабильная версия v{version}"
  quick_install: "Быстрая установка"
  copy: "Копировать"
  copied: "Скопировано!"
  supported: "Поддерживаемые платформы: Windows (PowerShell), macOS, Linux (x64/ARM64)"
  download_btn: "Скачать"
  coming_soon: "Скоро"
  checksum: "Контрольная сумма / Подпись"
  build_from_source:
    title: "Сборка из исходников"
    description: "Сборка YaoXiang из исходного кода с помощью Cargo. Убедитесь, что Rust установлен."
  nightly_builds:
    title: "Ночные сборки"
    description: "Доступны самые свежие версии. Рекомендуется для тестирования. В продакшене используйте с осторожностью."
  github_actions: "Перейти к GitHub Actions"

versions:
  - version: 0.1.0
    latest: true
    install_command: "curl -fsSL https://yaoxiang.org/install.sh | sh"
    downloads:
      - os: Windows
        arch: x64
        features: ["MSI Installer", "Portable Zip"]
        links: 
          - name: "Installer (.msi)"
            url: "https://github.com/ChenXu233/YaoXiang/releases/download/v0.1.0/yaoxiang-v0.1.0-x86_64-pc-windows-msvc.msi"
          - name: "Portable (.zip)"
            url: "https://github.com/ChenXu233/YaoXiang/releases/download/v0.1.0/yaoxiang-v0.1.0-x86_64-pc-windows-msvc.zip"
      - os: Linux
        arch: x64 / ARM64
        features: ["Static Binary", ".tar.gz"]
        links: 
          - name: "Linux x64 (.tar.gz)"
            url: "https://github.com/ChenXu233/YaoXiang/releases/download/v0.1.0/yaoxiang-v0.1.0-x86_64-unknown-linux-musl.tar.gz"
          - name: "Linux ARM64 (.tar.gz)"
            url: "https://github.com/ChenXu233/YaoXiang/releases/download/v0.1.0/yaoxiang-v0.1.0-aarch64-unknown-linux-musl.tar.gz"
      - os: macOS
        arch: Apple Silicon / Intel
        features: ["Universal Binary"]
        links: 
          - name: "Universal (.tar.gz)"
            url: "https://github.com/ChenXu233/YaoXiang/releases/download/v0.1.0/yaoxiang-v0.1.0-universal-apple-darwin.tar.gz"
sidebar: false

---