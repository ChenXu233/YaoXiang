---
title: 'パッケージマネージャー'
description: 'YaoXiang 公式パッケージマネージャーの使用チュートリアル'
---

# パッケージマネージャー

YaoXiang に内蔵されているパッケージマネージャーは、完全な依存関係管理機能を提供します。

## 概要

YaoXiang Package Manager (YPM) は宣言型の依存関係管理を採用しています：

- `yaoxiang.toml` でプロジェクトの依存関係を宣言
- `yaoxiang.lock` で正確なバージョンをロックし、ビルドの再現性を確保
- 依存関係は `vendor` ディレクトリにダウンロード

## クイックスタート

```bash
# 新規プロジェクトを作成
yx init my-project
cd my-project

# 依存関係を追加
yx add http
yx add json

# 依存関係をインストール
yx install

# プロジェクトを実行
yx run src/main.yx
```

## プロジェクト構造

```
my-project/
├── yaoxiang.toml      # プロジェクトマニフェスト
├── yaoxiang.lock      # 依存関係ロックファイル
├── vendor/            # 依存関係の保存先
└── src/
    └── main.yx
```

---

## init

新しいプロジェクトを初期化します。

### 使用法

```bash
yx init <name>
```

### 引数

| 引数   | 型     | 説明           |
| ------ | ------ | -------------- |
| `name` | string | プロジェクト名 |

### 説明

現在のディレクトリまたは指定されたパスに新しい YaoXiang プロジェクトを作成します。

### 作成されるファイル

- `yaoxiang.toml` - プロジェクトマニフェスト
- `yaoxiang.lock` - 依存関係ロックファイル
- `src/main.yx` - エントリファイル
- `.gitignore` - Git の無視設定

### 例

```bash
# 現在のディレクトリにプロジェクトを作成
yx init my-project

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

### 使用法

```bash
yx add <name> [version]
yx add <name> --dev
```

### 引数

| 引数      | 型     | 説明                                           |
| --------- | ------ | ---------------------------------------------- |
| `name`    | string | パッケージ名                                   |
| `version` | string | バージョン番号（オプション、デフォルトは `*`） |

### オプション

| オプション    | 説明                     |
| ------------- | ------------------------ |
| `--dev`, `-D` | 開発用依存関係として追加 |

### 説明

プロジェクトの `yaoxiang.toml` ファイルに依存関係を追加し、`yaoxiang.lock` を更新します。

### バージョン指定

| 指定方法  | 説明             | 例                |
| --------- | ---------------- | ----------------- |
| `*`       | 任意のバージョン | `http = "*"`      |
| `1.0.0`   | 正確バージョン   | `http = "1.0.0"`  |
| `>=1.0.0` | 最低バージョン   | `http = ">1.0.0"` |
| `~1.0.0`  | 互換バージョン   | `http = "~1.0.0"` |
| `^1.0.0`  | caret バージョン | `http = "^1.0.0"` |

### 依存関係の出所

#### Registry（デフォルト）

```bash
yx add http
yx add http 1.0.0
```

#### Git リポジトリ

```bash
# マニフェストには以下のような設定が生成されます
# http = { version = "1.0.0", git = "https://github.com/example/http" }
```

#### ローカルパス

```bash
# マニフェストには以下のような設定が生成されます
# mylib = { version = "0.1.0", path = "./mylib" }
```

### 例

```bash
# 最新バージョンを追加
yx add http

# 指定バージョンを追加
yx add http 1.0.0

# バージョン範囲を追加
yx add json ">=2.0.0"

# 開発用依存関係を追加
yx add test-utils --dev
yx add benchmark -D
```

---

## rm

プロジェクトから依存関係を削除します。

### 使用法

```bash
yx rm <name>
yx rm <name> --dev
```

### 引数

| 引数   | 型     | 説明         |
| ------ | ------ | ------------ |
| `name` | string | パッケージ名 |

### オプション

| オプション    | 説明                 |
| ------------- | -------------------- |
| `--dev`, `-D` | 開発用依存関係を削除 |

### 説明

プロジェクトの `yaoxiang.toml` から指定された依存関係を削除し、`yaoxiang.lock` を更新します。

### 例

```bash
# ランタイム依存関係を削除
yx rm http

# 開発用依存関係を削除
yx rm test-utils --dev
```

---

## install

プロジェクトの依存関係をインストールします。

### 使用法

```bash
yx install
```

### 説明

`yaoxiang.toml` の依存関係宣言を読み込み、以下の操作を実行します：

1. 依存関係のバージョンを解決
2. バージョン競合を検出
3. 依存関係を `vendor` ディレクトリにダウンロード
4. `yaoxiang.lock` を生成・更新

### 動作

- 依存関係がない場合はメッセージを表示して終了
- `vendor` ディレクトリが既に存在する場合は、キャッシュを確認して再利用
- バージョン競合を検出した場合はエラーメッセージを表示して終了

### 例

```bash
# すべての依存関係をインストール
yx install

# 出力
# 📦 依存関係を解析中...
#   http (1.0.0) [インストール済み]
#   json (2.0.0) [キャッシュ済み]
# ✅ 依存関係のインストールが完了し、ロックファイルが更新されました
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

### 使用法

```bash
yx update
yx update <name>
```

### 引数

| 引数   | 型     | 説明                       |
| ------ | ------ | -------------------------- |
| `name` | string | パッケージ名（オプション） |

### 説明

### 全件更新

引数を指定しない場合、すべての依存関係を更新します：

1. 現在ロックされているバージョンをクリア
2. `vendor` ディレクトリ内の旧バージョンを削除
3. すべての依存関係を再ダウンロード
4. `yaoxiang.lock` を更新

### 単一更新

引数を指定した場合、指定された依存関係のみを更新します：

1. 旧バージョンを `vendor` から削除
2. 新バージョンを再ダウンロード
3. `yaoxiang.lock` の対応するエントリを更新
4. 他の依存関係には影響しない

### 例

```bash
# すべての依存関係を更新
yx update

# 出力
# 📦 依存関係を更新中...
#   http (1.0.0 → 1.1.0)
#   json (2.0.0 → 2.1.0)
# ✅ 2 個の依存関係が更新され、ロックファイルが更新されました

# 単一の依存関係を更新
yx update http

# 出力
# ✅ http を更新しました (1.0.0 → 1.1.0)
```

---

## list

プロジェクトの依存関係を一覧表示します。

### 使用法

```bash
yx list
```

### 説明

プロジェクト内のすべての依存関係を表示します。含まれる情報：

- ランタイム依存関係（`[dependencies]` から）
- 開発用依存関係（`[dev-dependencies]` から）
- 各依存関係のバージョンと出所

### 例

```bash
yx list

# 出力
# 📦 プロジェクト依存関係
#
# ランタイム依存関係:
#   http        1.0.0    registry
#   json        2.0.0    registry
#
# 開発用依存関係:
#   test-utils  0.5.0    registry
```

---

## 設定ファイル

### yaoxiang.toml

プロジェクトマニフェストファイル。プロジェクトのメタデータと依存関係を宣言します。

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

依存関係ロックファイル。パッケージマネージャーによって自動生成されます。

```toml
# YaoXiang パッケージマネージャーによって自動生成

[package]
version = 1

[package.http]
version = "1.0.0"
source = "registry"
```

---

## 中核概念

### ランタイム依存関係と開発用依存関係

- **ランタイム依存関係** (`[dependencies]`)：プロジェクトの実行時に必須となるパッケージ
- **開発用依存関係** (`[dev-dependencies]`)：開発やテスト時にのみ必要なパッケージ

### 依存関係の出所

| 種類     | 設定例                                       | 説明                                   |
| -------- | -------------------------------------------- | -------------------------------------- |
| Registry | `http = "1.0.0"`                             | リモートのパッケージリポジトリから取得 |
| Git      | `{ version = "1.0.0", git = "https://..." }` | Git リポジトリから取得                 |
| Path     | `{ version = "0.1.0", path = "./lib" }`      | ローカルパスから取得                   |

### ロックファイル

`yaoxiang.lock`
はパッケージマネージャーによって自動生成されます。**必ずバージョン管理システムにコミットしてください**：

- チームメンバーが完全に同じ依存関係バージョンを使用することを保証
- CI ビルドの再現性を確保
- 「自分のマシンでは動く」問題を回避

### vendor ディレクトリ

ダウンロードされた依存関係は `vendor` ディレクトリに保存されます：

- `yx install` と `yx update` によって自動的に管理される
- 削除後に `install` を再実行することで再構築可能
- `.gitignore` に追加することを推奨（チームメンバーごとに独立して管理）

---

## よくある質問

### Q: 依存関係のバージョン競合が発生した場合はどうすればいいですか？

YPM は依存関係のバージョン競合を検出し、エラーを報告します。解決策：

1. 依存関係のバージョン要件を調整
2. 依存関係のパッケージ作者の修正を待つ
3. 競合している依存関係の削除を検討

### Q: プライベートパッケージはどのように使用しますか？

プライベートパッケージの場合、Git 出所を使用できます：

```bash
# Git URL 経由で追加
# yaoxiang.toml を手動で編集
[dependencies]
private-pkg = { version = "1.0.0", git = "https://github.com/org/private-pkg" }
```

### Q: vendor ディレクトリは削除できますか？

はい、削除可能です。削除後に `yx install` を実行すると、すべての依存関係が再ダウンロードされます。

### Q: 特定のパッケージの情報を確認するにはどうすればよいですか？

`yx list` ですべての依存関係を確認するか、`yaoxiang.toml` を確認してください。
