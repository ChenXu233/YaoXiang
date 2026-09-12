---
title: 'パッケージマネージャー'
description: 'YaoXiang 公式パッケージマネージャー使用チュートリアル'
---

# パッケージマネージャー

YaoXiang 内蔵のパッケージマネージャーが、完全な依存関係管理機能を提供します。

## 概要

YaoXiang Package Manager (YPM) は宣言的な依存関係管理を採用しています：

- `yaoxiang.toml` でプロジェクトの依存関係を宣言
- `yaoxiang.lock` で正確なバージョンを固定し、ビルドの再現性を確保
- 依存関係を `vendor` ディレクトリにダウンロード

## クイックスタート

```bash
# 新しいプロジェクトを作成
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
├── vendor/            # 依存関係ストレージ
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

### パラメータ

| パラメータ | 型     | 説明           |
| ---------- | ------ | -------------- |
| `name`     | string | プロジェクト名 |

### 説明

現在のディレクトリまたは指定されたパスに新しい YaoXiang プロジェクトを作成します。

### 作成されるファイル

- `yaoxiang.toml` - プロジェクトマニフェスト
- `yaoxiang.lock` - 依存関係ロックファイル
- `src/main.yx` - エントリーファイル
- `.gitignore` - Git 無視設定

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

### パラメータ

| パラメータ | 型     | 説明                                         |
| ---------- | ------ | -------------------------------------------- |
| `name`     | string | パッケージ名                                 |
| `version`  | string | バージョン番号（オプション、デフォルト `*`） |

### オプション

| オプション    | 説明                   |
| ------------- | ---------------------- |
| `--dev`, `-D` | 開発依存関係として追加 |

### 説明

プロジェクトの `yaoxiang.toml` ファイルに依存関係を追加し、`yaoxiang.lock` を更新します。

### バージョン指定

| 指定      | 説明             | 例                |
| --------- | ---------------- | ----------------- |
| `*`       | 任意のバージョン | `http = "*"`      |
| `1.0.0`   | 正確なバージョン | `http = "1.0.0"`  |
| `>=1.0.0` | 最低バージョン   | `http = ">1.0.0"` |
| `~1.0.0`  | 互換バージョン   | `http = "~1.0.0"` |
| `^1.0.0`  | caret バージョン | `http = "^1.0.0"` |

### 依存関係ソース

#### Registry（デフォルト）

```bash
yx add http
yx add http 1.0.0
```

#### Git リポジトリ

```bash
# manifest に以下のような設定が生成されます
# http = { version = "1.0.0", git = "https://github.com/example/http" }
```

#### ローカルパス

```bash
# manifest に以下のような設定が生成されます
# mylib = { version = "0.1.0", path = "./mylib" }
```

### 例

```bash
# 最新バージョンを追加
yx add http

# 指定されたバージョンを追加
yx add http 1.0.0

# バージョン範囲を追加
yx add json ">=2.0.0"

# 開発依存関係を追加
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

### パラメータ

| パラメータ | 型     | 説明         |
| ---------- | ------ | ------------ |
| `name`     | string | パッケージ名 |

### オプション

| オプション    | 説明               |
| ------------- | ------------------ |
| `--dev`, `-D` | 開発依存関係を削除 |

### 説明

プロジェクトの `yaoxiang.toml` から指定された依存関係を削除し、`yaoxiang.lock` を更新します。

### 例

```bash
# ランタイム依存関係を削除
yx rm http

# 開発依存関係を削除
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

1. 依存関係のバージョンを解析
2. バージョンの競合を検出
3. 依存関係を `vendor` ディレクトリにダウンロード
4. `yaoxiang.lock` を生成/更新

### 動作

- 依存関係がない場合は、メッセージを表示して終了
- `vendor` ディレクトリが既に存在する場合は、キャッシュを確認して再利用
- バージョンの競合が検出された場合は、エラーメッセージを表示して終了

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

### パラメータ

| パラメータ | 型     | 説明                       |
| ---------- | ------ | -------------------------- |
| `name`     | string | パッケージ名（オプション） |

### 説明

### 全件更新

引数なしで、すべての依存関係を更新します：

1. 現在ロックされているバージョンをクリア
2. `vendor` ディレクトリの旧バージョンをクリーンアップ
3. すべての依存関係を再ダウンロード
4. `yaoxiang.lock` を更新

### 単一更新

引数を指定すると、指定された依存関係のみを更新します：

1. `vendor` から旧バージョンを削除
2. 新バージョンを再ダウンロード
3. `yaoxiang.lock` の対応するエントリを更新
4. 他の依存関係は影響を受けません

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
# ✅ http (1.0.0 → 1.1.0) を更新しました
```

---

## list

プロジェクトの依存関係を表示します。

### 使用法

```bash
yx list
```

### 説明

プロジェクト内のすべての依存関係を表示します。内容：

- ランタイム依存関係（`[dependencies]` から）
- 開発依存関係（`[dev-dependencies]` から）
- 各依存関係のバージョンとソース

### 例

```bash
yx list

# 出力
# 📦 プロジェクトの依存関係
#
# ランタイム依存関係:
#   http        1.0.0    registry
#   json        2.0.0    registry
#
# 開発依存関係:
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
description = "项目描述"
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
# 由 YaoXiang 包管理器自动生成

[package]
version = 1

[package.http]
version = "1.0.0"
source = "registry"
```

---

## コアコンセプト

### ランタイム依存関係 vs 開発依存関係

- **ランタイム依存関係** (`[dependencies]`)：プロジェクトの実行時に必須のパッケージ
- **開発依存関係** (`[dev-dependencies]`)：開発とテスト時にのみ必要なパッケージ

### 依存関係ソース

| 種類     | 設定例                                       | 説明                                 |
| -------- | -------------------------------------------- | ------------------------------------ |
| Registry | `http = "1.0.0"`                             | リモートパッケージリポジトリから取得 |
| Git      | `{ version = "1.0.0", git = "https://..." }` | Git リポジトリから取得               |
| Path     | `{ version = "0.1.0", path = "./lib" }`      | ローカルパスから取得                 |

### ロックファイル

`yaoxiang.lock`
はパッケージマネージャーによって自動生成されます。**必ずバージョン管理システムにコミットしてください**：

- チームメンバーがまったく同じ依存関係バージョンを使用することを保証
- CI ビルドの再現性を確保
- 「自分のマシンでは動作する」問題を回避

### vendor ディレクトリ

依存関係は `vendor` ディレクトリに格納されます：

- `yx install` と `yx update` によって自動管理
- 削除後 `install` を再実行して再構築可能
- `.gitignore` に追加することをお勧めします。チームメンバーごとに独立して管理

---

## よくある質問

### Q: 依存関係のバージョン競合が発生した場合の対処法は？

YPM は依存関係のバージョン競合を検出し、エラーを報告します。解決策：

1. 依存関係のバージョン要件を調整
2. 依存関係の作成者による修正を待つ
3. 競合している依存関係の削除を検討

### Q: プライベートパッケージの使用方法は？

プライベートパッケージの場合、Git ソースを使用できます：

```bash
# Git URL 経由で追加
# yaoxiang.toml を手動で編集
[dependencies]
private-pkg = { version = "1.0.0", git = "https://github.com/org/private-pkg" }
```

### Q: vendor ディレクトリを削除できますか？

はい。削除後 `yx install` を実行すると、すべての依存関係が再ダウンロードされます。

### Q: 特定のパッケージの情報を確認する方法は？

`yx list` を使用してすべての依存関係を確認するか、`yaoxiang.toml` を参照してください。
