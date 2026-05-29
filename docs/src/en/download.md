---
layout: page
is_download: true

title: TYPE THE UNIVERSE
description: "Choose your platform and start building the world."

download:
  latest_stable: "Latest Stable v{version}"
  quick_install: "Quick Install"
  copy: "Copy"
  copied: "Copied!"
  supported: "Supported platforms: Windows (PowerShell), macOS, Linux (x64/ARM64)"
  download_btn: "Download"
  coming_soon: "Coming Soon"
  checksum: "Checksum / Signatures"
  build_from_source:
    title: "Build from Source"
    description: "Build YaoXiang from source using Cargo. Ensure Rust is installed."
  nightly_builds:
    title: "Nightly Builds"
    description: "The latest cutting-edge version is available. Recommended for testing; use with caution in production."
  github_actions: "Go to GitHub Actions"

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