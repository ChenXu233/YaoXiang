---
title: 'YaoXiang のインストール'
description:
  '二層インストールチャネル——標準チャネルは展開するだけですぐ使える（Go/Zig
  モード）、簡単チャネルはワンライナー + バージョン管理（Rust/rustup モード）'
---

# YaoXiang のインストール

YaoXiang のコマンド面は**フロントドア/エンジン分離**です（RFC-037）：

- **`yx`** — フロントドア、日常のエントリポイント。バージョン管理を内蔵（`yx toolchain` /
  `yx self update`）、その他のコマンドはエンジンに透過的に渡されます
- **`yaoxiang-rs`**
  — エンジンで、コンパイル/実行/パッケージ管理/フォーマット/LSP を担当し、通常は直接呼び出しません

インストールは二層チャネルに分かれ、すべてのチャネルが同一のプロダクト構造（`bin/` +
`lib/yaoxiang/std/`）を共有します。

## 標準チャネル：展開するだけですぐ使える（Go/Zig モード）

[GitHub Releases](https://github.com/ChenXu233/YaoXiang/releases)
から対応するプラットフォームのパッケージをダウンロードし、任意のディレクトリに展開して、`bin/`
を PATH に追加します：

```sh
tar xzf yaoxiang-<バージョン>-<プラットフォーム>.tar.gz
export PATH="$PWD/yaoxiang-<バージョン>-<プラットフォーム>/bin:$PATH"
```

展開後すぐ使用できます：`yx --version`。パッケージにはすべての依存関係（Z3 共有ライブラリを含む）と閲覧可能な標準ライブラリソース
`lib/yaoxiang/std/` が含まれており、追加のステップは必要ありません。

## 簡単チャネル：ワンライナー（Rust/rustup モード）

### Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.ps1 | iex
```

スクリプトはツールチェーンを `~/.yaoxiang/`（Windows は
`%USERPROFILE%\.yaoxiang`）にインストールし、`yx` を PATH に追加します。

### Linux: apt リポジトリ

```sh
curl -fsSL <apt リポジトリ URL>/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/yaoxiang.gpg
echo "deb [signed-by=/usr/share/keyrings/yaoxiang.gpg] <apt リポジトリ URL> stable main" | sudo tee /etc/apt/sources.list.d/yaoxiang.list
sudo apt update && sudo apt install yaoxiang
```

apt インストールはシステムレベルでのインストール（`/usr/lib/yaoxiang/`、コマンドエントリ
`/usr/bin/yx`）で、`apt upgrade` でバージョンに追従します。リポジトリのメタデータは release
CI によって自動署名されます。

### Windows: Inno Setup ウィザード

[Releases](https://github.com/ChenXu233/YaoXiang/releases) から `YaoXiang-Setup-<バージョン>.exe`
をダウンロードし、ウィザードに従ってインストールします（自動 PATH 追加はオプション）。apt と同様にシステムレベルでのインストールです。

## バージョン管理（yx フロントドア内蔵）

```sh
yx toolchain install stable    # 最新安定版をインストール
yx toolchain install 0.7.14    # 指定バージョンをインストール
yx toolchain default 0.7.14    # デフォルトバージョンを設定
yx toolchain list              # インストール済みバージョンを一覧表示
yx toolchain update            # 最新安定版にアップグレード
yx self update                 # yx 本体を更新
```

複数バージョンは `~/.yaoxiang/versions/<バージョン>/`
に共存し、各バージョンは完全な自己完結型のツールチェーンツリー（エンジン、Z3、標準ライブラリは同じバージョンでロック）です。

### プロジェクトレベルのバージョンロック

プロジェクトのルートディレクトリに `yx-toolchain.toml`（先例：`rust-toolchain.toml`）を配置します：

```toml
toolchain = "0.7.14"
```

そのディレクトリで `yx`
コマンドを実行すると、pin されたバージョンが使用されます。異なるプロジェクトは異なるバージョンで作業できます。

## 環境変数

| 変数               | 説明                                                                            |
| ------------------ | ------------------------------------------------------------------------------- |
| `YAOXIANG_HOME`    | インストールルートを上書き、デフォルトは `~/.yaoxiang`（CI とコンテナ環境向け） |
| `YAOXIANG_VERSION` | install スクリプトが最新バージョンではなく指定バージョンをインストール          |

ミラーダウンロードは `~/.yaoxiang/settings.toml` で設定できます（ghproxy スタイルプレフィックス）：

```toml
mirror = "https://ghproxy.example.com"
```

## インストールの確認

```sh
yx --version        # フロントドアバージョン
yx run main.yx      # 任意の YaoXiang プログラム
```
