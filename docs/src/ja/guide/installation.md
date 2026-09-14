---
title: 'YaoXiang のインストール'
description:
  '二層のインストールチャネル——標準チャネルは展開するだけで使える（Go/Zig
  モード）、簡単チャネルはワンライナー + バージョン管理（Rust/rustup モード）'
---

# YaoXiang のインストール

YaoXiang のコマンド面は**フロントドア / エンジン分離**（RFC-037）です。

- **`yx`** — フロントドア、日常的なエントリポイント。バージョン管理を内蔵（`yx toolchain` /
  `yx self update`）、その他のコマンドはエンジンに透過的に渡します
- **`yaoxiang-rs`** — エンジン、コンパイル / 実行 / パッケージ管理 / フォーマット /
  LSP を担当し、通常は直接呼び出しません

インストールは二層チャネルに分かれており、すべてのチャネルで同じ成果物構造（`bin/` +
`lib/yaoxiang/std/`）を共有します。

## 標準チャネル：展開するだけで使える（Go/Zig モード）

[GitHub Releases](https://github.com/ChenXu233/YaoXiang/releases)
から対応プラットフォームのアーカイブをダウンロードし、任意のディレクトリに展開して `bin/`
を PATH に追加します。

```sh
tar xzf yaoxiang-<バージョン>-<プラットフォーム>.tar.gz
export PATH="$PWD/yaoxiang-<バージョン>-<プラットフォーム>/bin:$PATH"
```

展開後すぐに使用できます：`yx --version`。アーカイブにはすべての依存関係（Z3 共有ライブラリを含む）と、閲覧可能な標準ライブラリのソースコード
`lib/yaoxiang/std/` が同梱されており、追加の手順は一切不要です。

## 簡単チャネル：ワンライナー（Rust/rustup モード）

### Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/ChenXu233/YaoXiang/main/scripts/install/install.ps1 | iex
```

スクリプトはツールチェーンを `~/.yaoxiang/`（Windows の場合は
`%USERPROFILE%\.yaoxiang`）にインストールし、`yx` を PATH に追加します。

### Linux：apt リポジトリ

```sh
curl -fsSL <apt リポジトリ URL>/KEY.gpg | sudo gpg --dearmor -o /usr/share/keyrings/yaoxiang.gpg
echo "deb [signed-by=/usr/share/keyrings/yaoxiang.gpg] <apt リポジトリ URL> stable main" | sudo tee /etc/apt/sources.list.d/yaoxiang.list
sudo apt update && sudo apt install yaoxiang
```

apt インストールはシステムレベルでのプレーンインストール（`/usr/lib/yaoxiang/`、コマンドのエントリは
`/usr/bin/yx`）で、`apt upgrade` でバージョンに追従します。リポジトリのメタデータは release
CI により自動的に署名されて公開されます。

### Windows：Inno Setup ウィザード

[Releases](https://github.com/ChenXu233/YaoXiang/releases) から `YaoXiang-Setup-<バージョン>.exe`
をダウンロードし、ウィザードに従ってインストールします（任意の自動 PATH 追加）。apt と同様にシステムレベルでのプレーンインストールです。

## バージョン管理（yx フロントドアに内蔵）

```sh
yx toolchain install stable    # 最新の安定版をインストール
yx toolchain install 0.7.14    # 指定バージョンをインストール
yx toolchain default 0.7.14    # デフォルトバージョンを設定
yx toolchain list              # インストール済みバージョンを一覧表示
yx toolchain update            # 最新の安定版にアップグレード
yx self update                 # yx 本体を更新
```

複数バージョンは `~/.yaoxiang/versions/<バージョン>/`
に共存し、各バージョンは完全な自己完結型のツールチェーンツリー（エンジン、Z3、標準ライブラリが同じバージョンでロック）です。

### プロジェクトレベルのバージョン固定

プロジェクトのルートディレクトリに `yx-toolchain.toml`（先例：`rust-toolchain.toml`）を配置します。

```toml
toolchain = "0.7.14"
```

そのディレクトリ配下で `yx`
コマンドを実行すると、pin されたバージョンが使用されます。プロジェクトごとに異なるバージョンで作業できます。

## 環境変数

| 変数               | 説明                                                                            |
| ------------------ | ------------------------------------------------------------------------------- |
| `YAOXIANG_HOME`    | インストールルートを上書き、デフォルトは `~/.yaoxiang`（CI / コンテナ環境向け） |
| `YAOXIANG_VERSION` | install スクリプトで最新版ではなく指定バージョンをインストールする              |

## ネットワークとミラー（制限のあるネットワーク）

GitHub に直接接続できないネットワークでは、`yx`
はダウンロードミラーをサポートします。`<インストールルート>/settings.toml`（デフォルトは
`~/.yaoxiang/settings.toml`）で ghproxy 形式の URL プレフィックスを設定します。

```toml
mirror = "https://ghproxy.example.com"
```

有効範囲（`yx` は完全な GitHub URL をこのプレフィックスの後ろに連結します）：

- `yx toolchain install` / `yx toolchain update`——リリースパッケージ、`.sha256`
  サイドカー、バージョン照会 API
- `yx self update`——同上

注意：

- ワンライナーインストールスクリプト（`install.sh` /
  `install.ps1`）はミラー設定を読み取らず、GitHub に直接接続します
- ダウンロードに失敗した場合、`yx`
  はエラーメッセージでミラー設定を促します。ミラー自体にアクセスできない場合もネットワークエラーとして報告されます

## インストールの確認

```sh
yx --version        # フロントドアのバージョン
yx run main.yx      # 任意の YaoXiang プログラム
```
