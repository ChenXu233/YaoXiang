---
layout: page
is_download: true

title: TYPE THE UNIVERSE
description: "选择你的平台，开始构建世界。"
version: 0.1.0
install_command: "curl -fsSL https://yaoxiang.org/install.sh | sh"

download:
  latest_stable: "最新稳定版 v{version}"
  quick_install: "快速安装"
  copy: "复制"
  copied: "已复制！"
  supported: "支持平台：Windows (PowerShell)、macOS、Linux (x64/ARM64)"
  download_btn: "下载"
  coming_soon: "即将推出"
  checksum: "校验文件 / 签名"
  build_from_source:
    title: "从源码构建"
    description: "使用 Cargo 从源码构建 YaoXiang，请确保已安装 Rust。"
  nightly_builds:
    title: "每夜构建"
    description: "最新前沿版本已可获取。推荐用于测试，生产环境请谨慎使用。"
  github_actions: "前往 GitHub Actions"

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
