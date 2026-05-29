---
title: コマンドラインインターフェース
description: パッケージマネージャーの全コマンド詳細説明
---

# コマンドラインインターフェース

## yaoxiang init

新しい YaoXiang プロジェクトを初期化します。

### 使い方

```bash
yaoxiang init <プロジェクト名>
```

### 引数

| 引数 | 説明 |
|------|------|
| プロジェクト名 | 新規プロジェクトの名前 |

### オプション

| オプション | 説明 |
|------|------|
| `--help` | ヘルプ情報を表示 |

### 例

```bash
# 新規プロジェクト作成
yaoxiang init my-project

# 結果：
# ✨ プロジェクト作成完了：my-project
#   my-project/yaoxiang.toml
#   my-project/yaoxiang.lock
#   my-project/src/main.yx
#   my-project/.gitignore
```

---

## yaoxiang add

プロジェクトに依存関係を追加します。

### 使い方

```bash
yaoxiang add <パッケージ名> [バージョン]
yaoxiang add <パッケージ名> --dev
```

### 引数

| 引数 | 説明 |
|------|------|
| パッケージ名 | 追加する依存関係名 |
| バージョン | バージョン番号（オプション、デフォルトは `*`） |

### オプション

| オプション | 説明 |
|------|------|
| `--dev`, `-D` | 開発用依存関係として追加 |

### 例

```bash
# 最新バージョンを追加
yaoxiang add http

# 指定バージョンを追加
yaoxiang add http 1.0.0

# バージョン範囲を指定して追加
yaoxiang add json ">=2.0.0"

# 開発用依存関係を追加
yaoxiang add test-utils --dev
yaoxiang add benchmark -D
```

---

## yaoxiang rm

プロジェクトから依存関係を削除します。

### 使い方

```bash
yaoxiang rm <パッケージ名>
yaoxiang rm <パッケージ名> --dev
```

### 引数

| 引数 | 説明 |
|------|------|
| パッケージ名 | 削除する依存関係名 |

### オプション

| オプション | 説明 |
|------|------|
| `--dev`, `-D` | 開発用依存関係を削除 |

### 例

```bash
# 通常の依存関係を削除
yaoxiang rm http

# 開発用依存関係を削除
yaoxiang rm test-utils --dev
```

---

## yaoxiang install

プロジェクトの依存関係をインストールします。

### 使い方

```bash
yaoxiang install
```

### 説明

- `yaoxiang.toml` の依存関係宣言を読み取る
- 全依存関係を `vendor` ディレクトリにダウンロード
- `yaoxiang.lock` のバージョンロックを生成/更新
- 依存関係のバージョン競合を検出

### 例

```bash
# 全依存関係をインストール
yaoxiang install

# 出力例：
# 📦 依存関係を解決中...
#   http (1.0.0) [インストール済み]
#   json (2.0.0) [キャッシュ済み]
# ✅ 依存関係のインストール完了、ロックファイル更新済み
```

---

## yaoxiang update

プロジェクトの依存関係を更新します。

### 使い方

```bash
yaoxiang update
yaoxiang update <パッケージ名>
```

### 引数

| 引数 | 説明 |
|------|------|
| パッケージ名 | 更新する特定の依存関係（オプション） |

### 説明

- 引数なし：全依存関係を更新
- 引数あり：指定した依存関係のみ更新

### 例

```bash
# 全依存関係を更新
yaoxiang update

# 出力例：
# 📦 依存関係を更新中...
#   http (0 → 1.1.0)
# ✅ 1件の依存関係を更新、ロックファイル更新済み

# 单一の依存関係を更新
yaoxiang update http
```

---

## yaoxiang list

プロジェクトの全依存関係を一覧表示します。

### 使い方

```bash
yaoxiang list
```

### 説明

ランタイム依存関係と開発用依存関係、および它们的版本と出所を顯示します。

### 例

```bash
# 依存関係を一覧表示
yaoxiang list

# 出力例：
# 📦 プロジェクト依存関係
#
# ランタイム依存関係:
#   http        1.0.0    registry
#   json        2.0.0    registry
#
# 開発用依存関係:
#   test-utils  0.5.0    registry
```