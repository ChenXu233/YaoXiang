---
layout: page
is_download: true

title: 'TYPE THE UNIVERSE'
description: '选择你的平台，开始构建世界。'

download:
  latest_stable: '最新稳定版 v{version}'
  quick_install: '快速安装'
  copy: '复制'
  copied: '已复制！'
  supported: '支持平台：Windows (PowerShell)、macOS、Linux (x64/ARM64)'
  download_btn: '下载'
  coming_soon: '即将推出'
  checksum: '校验文件 / 签名'
  build_from_source:
    title: '从源码构建'
    description: '使用 Cargo 从源码构建 YaoXiang，请确保已安装 Rust。'
  nightly_builds:
    title: '每夜构建'
    description: '最新前沿版本已可获取。推荐用于测试，生产环境请谨慎使用。'
  github_actions: '前往 GitHub Actions'

versions:
  - version: 0.7.14
    latest: true
    # 一行命令安装（RFC-037 傻瓜渠道）；0.8.0 起随重组包格式生效
    install_command: 'curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh'
    downloads:
      - os: Windows
        arch: x64
        features: ['命令行二进制（旧格式，不含 libz3）', '免安装']
        links:
          - name: 'Windows x64 (.exe)'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-x86_64-pc-windows-gnu.exe'
      - os: Windows
        arch: ARM64
        features: ['命令行二进制（旧格式，不含 libz3）', '免安装']
        links:
          - name: 'Windows ARM64 (.exe)'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-aarch64-pc-windows-gnullvm.exe'
      - os: Linux
        arch: x64
        features: ['命令行二进制（旧格式，静态链接 Z3）', '免安装']
        links:
          - name: 'Linux x64'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-x86_64-unknown-linux-gnu'
      - os: Linux
        arch: ARM64
        features: ['命令行二进制（旧格式，静态链接 Z3）', '免安装']
        links:
          - name: 'Linux ARM64'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-aarch64-unknown-linux-gnu'
      - os: macOS
        arch: Intel / Apple Silicon
        features: ['命令行二进制（旧格式，静态链接 Z3）', '免安装']
        links:
          - name: 'macOS Intel'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-x86_64-apple-darwin'
          - name: 'macOS Apple Silicon'
            url: 'https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/yaoxiang-aarch64-apple-darwin'
sidebar: false
---
