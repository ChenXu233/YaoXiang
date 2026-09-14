---
layout: page
is_download: true

title: 'TYPE THE UNIVERSE'
description: 'プラットフォームを選んで、世界を構築し始めましょう。'

download:
  latest_stable: '最新安定版 v{version}'
  quick_install: 'クイックインストール'
  copy: 'コピー'
  copied: 'コピーしました！'
  supported: '対応プラットフォーム：Windows (PowerShell)、macOS、Linux (x64/ARM64)'
  download_btn: 'ダウンロード'
  coming_soon: '近日公開'
  checksum: 'チェックサムファイル / 署名'
  build_from_source:
    title: 'ソースからビルド'
    description:
      'Cargo を使用してソースから YaoXiang をビルドします。Rust
      がインストールされていることを確認してください。'
  nightly_builds:
    title: 'ナイトリービルド'
    description: '最新の最先端バージョンが利用可能です。テスト用途に推奨され、本番環境では慎重にご利用ください。'
  github_actions: 'GitHub Actions へ'

versions:
  - version: 0.7.14
    latest: true
    # ワンラインインストール（RFC-037 簡単チャネル）；0.8.0 以降は再構築されたパッケージ形式で有効
    install_command:
      'curl -fsSL
      https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh'
    downloads:
      - os: Windows
        arch: x64
        features: ['コマンドラインバイナリ（旧形式、libz3 を含まない）', 'インストール不要']
        links:
          - name: 'Windows x64 (.exe)'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-x86_64-pc-windows-gnu.exe'
      - os: Windows
        arch: ARM64
        features: ['コマンドラインバイナリ（旧形式、libz3 を含まない）', 'インストール不要']
        links:
          - name: 'Windows ARM64 (.exe)'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-aarch64-pc-windows-gnullvm.exe'
      - os: Linux
        arch: x64
        features: ['コマンドラインバイナリ（旧形式、Z3 を静的リンク）', 'インストール不要']
        links:
          - name: 'Linux x64'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-x86_64-unknown-linux-gnu'
      - os: Linux
        arch: ARM64
        features: ['コマンドラインバイナリ（旧形式、Z3 を静的リンク）', 'インストール不要']
        links:
          - name: 'Linux ARM64'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-aarch64-unknown-linux-gnu'
      - os: macOS
        arch: Intel / Apple Silicon
        features: ['コマンドラインバイナリ（旧形式、Z3 を静的リンク）', 'インストール不要']
        links:
          - name: 'macOS Intel'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-x86_64-apple-darwin'
          - name: 'macOS Apple Silicon'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-aarch64-apple-darwin'
sidebar: false
---
