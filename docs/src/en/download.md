---
layout: page
is_download: true

title: 'TYPE THE UNIVERSE'
description: 'Choose your platform and start building worlds.'

download:
  latest_stable: 'Latest stable v{version}'
  quick_install: 'Quick Install'
  copy: 'Copy'
  copied: 'Copied!'
  supported: 'Supported platforms: Windows (PowerShell), macOS, Linux (x64/ARM64)'
  download_btn: 'Download'
  coming_soon: 'Coming soon'
  checksum: 'Checksum / Signature'
  build_from_source:
    title: 'Build from source'
    description: 'Build YaoXiang from source using Cargo. Make sure Rust is installed.'
  nightly_builds:
    title: 'Nightly builds'
    description:
      'The latest cutting-edge builds are now available. Recommended for testing; use with caution
      in production environments.'
  github_actions: 'Go to GitHub Actions'

versions:
  - version: 0.8.2
    latest: true
    # 一行命令安装（RFC-037 傻瓜渠道）；0.8.0 起随重组包格式生效
    install_command:
      'curl -fsSL
      https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh'
    downloads:
      - os: Windows
        arch: x64
        features: ['Command-line binary (legacy format, without libz3)', 'No installation required']
        links:
          - name: 'Windows x64 (.exe)'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.8.2/yaoxiang-x86_64-pc-windows-gnu.exe'
      - os: Windows
        arch: ARM64
        features: ['Command-line binary (legacy format, without libz3)', 'No installation required']
        links:
          - name: 'Windows ARM64 (.exe)'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.8.2/yaoxiang-aarch64-pc-windows-gnullvm.exe'
      - os: Linux
        arch: x64
        features:
          [
            'Command-line binary (legacy format, statically linked with Z3)',
            'No installation required',
          ]
        links:
          - name: 'Linux x64'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.8.2/yaoxiang-x86_64-unknown-linux-gnu'
      - os: Linux
        arch: ARM64
        features:
          [
            'Command-line binary (legacy format, statically linked with Z3)',
            'No installation required',
          ]
        links:
          - name: 'Linux ARM64'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.8.2/yaoxiang-aarch64-unknown-linux-gnu'
      - os: macOS
        arch: Intel / Apple Silicon
        features:
          [
            'Command-line binary (legacy format, statically linked with Z3)',
            'No installation required',
          ]
        links:
          - name: 'macOS Intel'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.8.2/yaoxiang-x86_64-apple-darwin'
          - name: 'macOS Apple Silicon'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.8.2/yaoxiang-aarch64-apple-darwin'
sidebar: false
---
