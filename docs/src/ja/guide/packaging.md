---
title: パッケージマネージャー
description: YaoXiang 公式パッケージマネージャーの使い方
---

# パッケージマネージャー

YaoXiang に組み込まれたパッケージマネージャーで、完全な依存関係管理機能を提供します。

## 概要

YaoXiang Package Manager (YPM) は宣言型依存関係管理を採用しています：

- `yaoxiang.toml` でプロジェクトの依存関係を宣言
- `yaoxiang.lock` で正確なバージョンをロックし、再現可能なビルドを確保
- 依存関係は `vendor` ディレクトリにダウンロードされる

## クイックスタート

```bash
# 新規プロジェクトの作成
yaoxiang init my-project
cd my-project

# 依存関係の追加
yaoxiang add http
yaoxiang add json

# 依存関係のインストール
yaoxiang install

# プロジェクトの実行
yaoxiang run src/main.yx
```

## プロジェクト構造

```
my-project/
├── yaoxiang.toml      # プロジェクトマニフェスト
├── yaoxiang.lock      # 依存関係ロックファイル
├── vendor/            # 依存関係ストレージ
└── src/
    └── main.yx
```

---

## init

新規プロジェクトを初期化します。

### 用法

```bash
yaoxiang init <name>
```

### パラメータ

| パラメータ | 型 | 説明 |
|------|------|------|
| `name` | string | プロジェクト名 |

### 説明

現在のディレクトリまたは指定されたパスに新しい YaoXiang プロジェクトを作成します。

### 作成されるファイル

- `yaoxiang.toml` - プロジェクトマニフェスト
- `yaoxiang.lock` - 依存関係ロックファイル
- `src/main.yx` - エントリーポイントファイル
- `.gitignore` - Git 除外設定

### 例

```bash
# 現在のディレクトリにプロジェクトを作成
yaoxiang init my-project

# 出力
# ✨ プロジェクトが作成されました：my-project
#   my-project/yaoxiang.toml
#   my-project/yaoxiang.lock
#   my-project/src/main.yx
#   my-project/.gitignore
```

---

## add

プロジェクトに依存関係を追加します。

### 用法

```bash
yaoxiang add <name> [version]
yaoxiang add <name> --dev
```

### パラメータ

| パラメータ | 型 | 説明 |
|------|------|------|
| `name` | string | パッケージ名 |
| `version` | string | バージョン番号（オプション、デフォルト `*`） |

### オプション

| オプション | 説明 |
|------|------|
| `--dev`, `-D` | 開発依存関係として追加 |

### 説明

プロジェクトの `yaoxiang.toml` ファイルに依存関係を追加し、`yaoxiang.lock` を更新します。

### バージョン指定

| 指定 | 説明 | 例 |
|------|------|------|
| `*` | 任意のバージョン | `http = "*"` |
| `1.0.0` | 正確なバージョン | `http = "1.0.0"` |
| `>=1.0.0` | 最低バージョン | `http = ">1.0.0"` |
| `~1.0.0` | 互換バージョン | `http = "~1.0.0"` |
| `^1.0.0` | caret バージョン | `http = "^1.0.0"` |

### 依存関係のソース

#### Registry（デフォルト）

```bash
yaoxiang add http
yaoxiang add http 1.0.0
```

#### Git リポジトリ

```bash
# マニフェストに以下のような設定が生成される
# http = { version = "1.0.0", git = "https://github.com/example/http" }
```

#### ローカルパス

```bash
# マニフェストに以下のような設定が生成される
# mylib = { version = "0.1.0", path = "./mylib" }
```

### 例

```bash
# 最新バージョンを追加
yaoxiang add http

# 指定バージョンを追加
yaoxiang add http 1.0.0

# バージョン範囲を追加
yaoxiang add json ">=2.0.0"

# 開発依存関係を追加
yaoxiang add test-utils --dev
yaoxiang add benchmark -D
```

---

## rm

プロジェクトから依存関係を削除します。

### 用法

```bash
yaoxiang rm <name>
yaoxiang rm <name> --dev
```

### パラメータ

| パラメータ | 型 | 説明 |
|------|------|------|
| `name` | string | パッケージ名 |

### オプション

| オプション | 説明 |
|------|------|
| `--dev`, `-D` | 開発依存関係を削除 |

### 説明

プロジェクトの `yaoxiang.toml` から指定された依存関係を削除し、`yaoxiang.lock` を更新します。

### 例

```bash
# 実行時依存関係を削除
yaoxiang rm http

# 開発依存関係を削除
yaoxiang rm test-utils --dev
```

---

## install

プロジェクトの依存関係をインストールします。

### 用法

```bash
yaoxiang install
```

### 説明

`yaoxiang.toml` 内の依存関係宣言を読み取り、以下の操作を実行します：

1. 依存関係のバージョンを解決
2. バージョン競合を検出
3. 依存関係を `vendor` ディレクトリにダウンロード
4. `yaoxiang.lock` を生成/更新

### 動作

- 依存関係がない場合は、メッセージを表示して終了
- `vendor` ディレクトリが既に存在する場合、キャッシュを確認して再利用
- バージョン競合が検出された場合は、エラーメッセージを表示して終了

### 例

```bash
# すべての依存関係をインストール
yaoxiang install

# 出力
# 📦 依存関係を解決中...
#   http (1.0.0) [インストール済み]
#   json (2.0.0) [キャッシュ済み]
# ✅ 依存関係がインストールされ、ロックファイルが更新されました
```

### ロックファイルの更新

`install` コマンドは `yaoxiang.lock` を更新します：

```toml
# yaoxiang.lock
[package]
version = 1

[package.http]
version = "1.0.0"
source = "registry"

[package.json]
version = "2.0.0"
source = "registry"
```

---

## update

プロジェクトの依存関係を更新します。

### 用法

```bash
yaoxiang update
yaoxiang update <name>
```

### パラメータ

| パラメータ | 型 | 説明 |
|------|------|------|
| `name` | string | パッケージ名（オプション） |

### 説明

### 全量更新

パラメータなしで実行すると、すべての依存関係を更新します：

1. 現在ロックされているバージョンをクリア
2. `vendor` ディレクトリ内の古いバージョンをクリーンアップ
3. すべての依存関係を再ダウンロード
4. `yaoxiang.lock` を更新

### 单一更新

パラメータ付きで実行すると、指定された依存関係のみを更新します：

1. `vendor` から古いバージョンを削除
2. 新しいバージョンを再ダウンロード
3. `yaoxiang.lock` の該当エントリを更新
4. 他の依存関係には影響しない

### 例

```bash
# すべての依存関係を更新
yaoxiang update

# 出力
# 📦 依存関係を更新中...
#   http (1.0.0 → 1.1.0)
#   json (2.0.0 → 2.1.0)
# ✅ 2件の依存関係が更新され、ロックファイルが更新されました

# 単一の依存関係を更新
yaoxiang update http

# 出力
# ✅ http が更新されました (1.0.0 → 1.1.0)
```

---

## list

プロジェクトの依存関係を一覧表示します。

### 用法

```bash
yaoxiang list
```

### 説明

プロジェクト内のすべての依存関係を表示します：

- 実行時依存関係（`[dependencies]` 由来）
- 開発依存関係（`[dev-dependencies]` 由来）
- 各依存関係のバージョンとソース

### 例

```bash
yaoxiang list

# 出力
# 📦 プロジェクト依存関係
#
# 実行時依存関係:
#   http        1.0.0    registry
#   json        2.0.0    registry
#
# 開発依存関係:
#   test-utils  0.5.0    registry
```

---

## 設定ファイル

### yaoxiang.toml

プロジェクトメタデータと依存関係を宣言するプロジェクトマニフェストファイル。

```toml
[package]
name = "my-project"
version = "0.1.0"
description = "プロジェクトの説明"
authors = ["作者 <email@example.com>"]
license = "MIT"

[dependencies]
http = "1.0.0"
json = "*"

[dev-dependencies]
test-utils = "0.5.0"
```

### yaoxiang.lock

パッケージマネージャーによって自動生成される依存関係ロックファイル。

```toml
# YaoXiang パッケージマネージャーによって自動生成

[package]
version = 1

[package.http]
version = "1.0.0"
source = "registry"
```

---

## コアコンセプト

### 実行時依存関係 vs 開発依存関係

- **実行時依存関係**（`[dependencies]`）：プロジェクトの実行に必要なパッケージ
- **開発依存関係**（`[dev-dependencies]`）：開発・テスト時のみ必要なパッケージ

### 依存関係のソース

| 型 | 設定例 | 説明 |
|------|----------|------|
| Registry | `http = "1.0.0"` | リモートパッケージレジストリから取得 |
| Git | `{ version = "1.0.0", git = "https://..." }` | Git リポジトリから取得 |
| Path | `{ version = "0.1.0", path = "./lib" }` | ローカルパスから取得 |

### ロックファイル

`yaoxiang.lock` はパッケージマネージャーによって自動生成されます。**必ずバージョン管理に含めてください**：

- チームメンバーが完全に同じバージョンの依存関係を使用することを保証
- CI ビルドの再現性を保証
- 「自分の環境では動く」問題を回避

### vendor ディレクトリ

依存関係がダウンロードされると `vendor` ディレクトリに保存されます：

- `yaoxiang install` と `yaoxiang update` によって自動管理
- 削除後に `install` を再実行すると再構築可能
- `.gitignore` に追加推奨、チーム成员が個別に管理

---

## よくある質問

### Q: 依存関係のバージョン競合が発生した場合は？

YPM は依存関係のバージョン競合を検出してエラー表示されます。解決策：

1. 依存関係のバージョン要件を調整
2. 依存関係の作者が修正するのを待つ
3. 競合する依存関係の移除を検討

### Q: プライベートパッケージを使用するには？

プライベートパッケージには、Git ソースを使用できます：

```bash
# Git URL 経由で追加
# yaoxiang.toml を手動で編集
[dependencies]
private-pkg = { version = "1.0.0", git = "https://github.com/org/private-pkg" }
```

### Q: vendor ディレクトリは削除できますか？

可能です。削除後に `yaoxiang install` を実行すると、すべての依存関係が再ダウンロードされます。

### Q: 特定のパッケージの情報を確認するには？

`yaoxiang list` ですべての依存関係を表示するか、`yaoxiang.toml` を確認してください。