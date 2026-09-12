---
title: 'YaoXiangのインストール'
description:
  2層のインストールチャネル —
  標準チャネルは解凍してすぐ使用（Go/Zigモード）、簡単チャネルは1行コマンド +
  バージョン管理（Rust/rustupモード）
---

# YaoXiangのインストール

YaoXiangのコマンド面は**フロントドア/エンジン分離**（RFC-037）になっています：

- **`yx`** — フロントドア、日常的なエントリポイント。バージョン管理を内蔵（`yx toolchain` /
  `yx self update`）、その他のコマンドはエンジンに透過
- **`yaoxiang-rs`**
  — エンジン、コンパイル/実行/パッケージ管理/フォーマット/LSPを担当、通常は直接呼び出さない

インストールは2層のチャネルに分かれており、すべてのチャネルが同じ成果物構造（`bin/` +
`lib/yaoxiang/std/`）を共有します。

## 標準チャネル：解凍してすぐ使用（Go/Zigモード）

[GitHub Releases](https://github.com/ChenXu233/YaoXiang/releases)から対応プラットフォームのアーカイブをダウンロードし、任意のディレクトリに解凍して、`bin/`
をPATHに追加します：

```sh
tar xzf yaoxiang-<バージョン>-<プラットフォーム>.tar.gz
export PATH="$PWD/yaoxiang-<バージョン>-<プラットフォーム>/bin:$PATH"
```

解凍後すぐに使用できます：`yx --version`。アーカイブにはすべての依存関係（Z3共有ライブラリを含む）と閲覧可能な標準ライブラリのソースコード
`lib/yaoxiang/std/` が含まれており、追加の手順は一切不要です。

## 簡単チャネル：1行コマンド（Rust/rustupモード）

### Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.ps1 | iex
```

スクリプトはツールチェーンを `~/.yaoxiang/`（Windowsでは
`%USERPROFILE%\.yaoxiang`）にインストールし、`yx` をPATHに追加します。

### Linux: aptリポジトリ

```sh
curl -fsSL <aptリポジトリアドレス>/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/yaoxiang.gpg
echo "deb [signed-by=/usr/share/keyrings/yaoxiang.gpg] <aptリポジトリアドレス> stable main" | sudo tee /etc/apt/sources.list.d/yaoxiang.list
sudo apt update && sudo apt install yaoxiang
```

aptインストールはシステムレベルのフラットインストール（`/usr/lib/yaoxiang/`、コマンドエントリ
`/usr/bin/yx`）で、`apt upgrade` でバージョンに追従します。リポジトリのメタデータはrelease
CIによって自動的に署名・公開されます。

### Windows: Inno Setupウィザード

[Releases](https://github.com/ChenXu233/YaoXiang/releases)から `YaoXiang-Setup-<バージョン>.exe`
をダウンロードし、ウィザードに従ってインストールします（オプションで自動的にPATHに追加）。aptと同様、システムレベルのフラットインストールです。

## バージョン管理（yxフロントドア内蔵）

```sh
yx toolchain install stable    # 最新の安定版をインストール
yx toolchain install 0.7.14    # 指定バージョンをインストール
yx toolchain default 0.7.14    # デフォルトバージョンを設定
yx toolchain list              # インストール済みバージョンを一覧表示
yx toolchain update            # 最新の安定版にアップグレード
yx self update                 # yx本体を更新
```

複数バージョンは `~/.yaoxiang/versions/<バージョン>/`
に共存し、各バージョンは自己完結した完全なツールチェーンツリー（エンジン、Z3、標準ライブラリが同バージョンでロック）です。

### プロジェクトレベルのバージョンロック

プロジェクトのルートディレクトリに `yx-toolchain.toml`（先例：`rust-toolchain.toml`）を配置します：

```toml
toolchain = "0.7.14"
```

そのディレクトリ下で `yx`
コマンドを実行すると、pinで指定されたバージョンが使用されます — 異なるプロジェクトで異なるバージョンを使用できます。

## 環境変数

| 変数               | 説明                                                                           |
| ------------------ | ------------------------------------------------------------------------------ |
| `YAOXIANG_HOME`    | インストールルートを上書き、デフォルトは `~/.yaoxiang`（CIとコンテナ環境向け） |
| `YAOXIANG_VERSION` | installスクリプトが最新版ではなく指定バージョンをインストール                  |

ミラーダウンロードは `~/.yaoxiang/settings.toml` で設定できます（ghproxyスタイルプレフィックス）：

```toml
mirror = "https://ghproxy.example.com"
```

## インストールの確認

```sh
yx --version        # フロントドアのバージョン
yx run main.yx      # 任意のYaoXiangプログラム
```
