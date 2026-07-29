---
layout: page
is_download: true

title: 'タイプ・ザ・ユニバース'
description: 'プラットフォームを選び、世界を構築し始める。'

download:
  latest_stable: '最新安定版 v{version}'
  quick_install: 'クイックインストール'
  copy: 'コピー'
  copied: 'コピーしました！'
  supported: '対応プラットフォーム：Windows (PowerShell)、macOS、Linux (x64/ARM64)'
  download_btn: 'ダウンロード'
  coming_soon: '近日公開'
  checksum: 'チェックサム / 署名'
  build_from_source:
    title: 'ソースからビルド'
    description: 'Cargoを使用してYaoXiangをソースからビルドするには、Rustがインストールされていることを確認してください。'
  nightly_builds:
    title: 'ナイトリービルド'
    description: '最新鋭のバージョンが利用可能です。テスト用途に推奨、本番環境での使用は注意してください。'
  github_actions: 'GitHub Actionsへ'

versions:
  - version: 0.1.0
    latest: true
    install_command: 'curl -fsSL https://yaoxiang.org/install.sh | sh'
    downloads:
      - os: Windows
        arch: x64
        features: ['MSI インストーラー', 'ポータブル Zip']
        links:
          - name: 'Installer (.msi)'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.1.0/yaoxiang-v0.1.0-x86_64-pc-windows-msvc.msi'
          - name: 'Portable (.zip)'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.1.0/yaoxiang-v0.1.0-x86_64-pc-windows-msvc.zip'
      - os: Linux
        arch: x64 / ARM64
        features: ['静的バイナリ', '.tar.gz']
        links:
          - name: 'Linux x64 (.tar.gz)'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.1.0/yaoxiang-v0.1.0-x86_64-unknown-linux-musl.tar.gz'
          - name: 'Linux ARM64 (.tar.gz)'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.1.0/yaoxiang-v0.1.0-aarch64-unknown-linux-musl.tar.gz'
      - os: macOS
        arch: Apple Silicon / Intel
        features: ['ユニバーサルバイナリ']
        links:
          - name: 'Universal (.tar.gz)'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.1.0/yaoxiang-v0.1.0-universal-apple-darwin.tar.gz'
sidebar: false
---
