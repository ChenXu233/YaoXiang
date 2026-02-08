---
layout: page
is_download: true

title: TYPE THE UNIVERSE
description: "Select your platform to initiate sequence."
version: 0.1.0
install_command: "curl -fsSL https://yaoxiang.org/install.sh | sh"

download:
  latest_stable: "LATEST STABLE v{version}"
  quick_install: "QUICK INSTALL"
  copy: "COPY"
  copied: "COPIED!"
  supported: "Supported: Windows (PowerShell), macOS, Linux (x64/ARM64)"
  download_btn: "DOWNLOAD"
  coming_soon: "COMING SOON"
  checksum: "checksum / signatures"
  build_from_source:
    title: "Build from Source"
    description: "You can build YaoXiang from source using Cargo. Make sure you have Rust installed."
  nightly_builds:
    title: "Nightly Builds"
    description: "Bleeding edge builds are available for testing. Not recommended for production."
  github_actions: "Go to GitHub Actions"

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
