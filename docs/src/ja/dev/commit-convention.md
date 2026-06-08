# Commit 提交ガイドライン

このドキュメントでは、YaoXiang プロジェクトの Git 提交規範を定義し、提交履歴を明確で読みやすく、理解しやすいものにすることを目的としています。

---

## 目次

- [提交フォーマット](#提交フォーマット)
- [提交タイプ](#提交タイプ)
- [完全な Emoji リファレンス](#完全な-emoji-リファレンス)
- [スコープ](#スコープ)
- [バージョニング](#バージョニング)
- [メッセージ規範](#メッセージ規範)
- [言語規範](#言語規範)
- [🔖 リリース提交](#-リリース提交)
- [例](#例)
- [Commit Template の使用方法](#commit-template-の使用方法)
- [よくある質問](#よくある質問)

---

## 提交フォーマット

**非常重要！！！！絶対に忘れないでください！！！**
すべての提交メッセージは次の形式に従います：

```
:emojiコード: type(scope): テーマ（日本語）

[任意のボディ内容]

[任意のフッター]
```

> ⚠️ **重要**: emoji 文字を直接入力せず、**emoji コード**（例：`:sparkles:`）を使用する必要があります。
> 
> **日本語での提交メッセージの使用をお勧めします**。チーム内のコミュニケーションの一貫性を保つためです。

### 構成要素

| 部分 | 説明 | 必須 |
|------|------|------|
| emojiコード | 提交タイプを識別する絵文字 | ✅ |
| type | 提交タイプ | ✅ |
| scope | 影響範囲 | ✅ |
| subject | 短い説明（日本語、50文字以内） | ✅ |
| body | 詳細説明（任意） | ❌ |
| footer | 破壊的変更または issue  закрытие（任意） | ❌ |

---

## 提交タイプ

| emojiコード | type | 説明 |
|-----------|------|------|
| :sparkles: | feat | 新機能 |
| :bug: | fix | バグ修正 |
| :memo: | docs | ドキュメント変更のみ |
| :lipstick: | style | コードフォーマット（機能に影響なし） |
| :recycle: | refactor | コードのリファクタリング |
| :zap: | perf | パフォーマンス最適化 |
| :white_check_mark: | test | テストの追加または変更 |
| :wrench: | chore | ビルドツール、補助ツールの変更 |
| :building_construction: | build | ビルドシステム変更 |
| :rocket: | ci | CI 設定変更 |

---

## 完全な Emoji リファレンス

以下は gitmoji プロジェクトと一致する完全な emoji リストです。提交内容に応じて適切な emoji を選択できます：

| emoji | emoji コード | commit 説明 |
| :---- | :---------------------------- | :--------------------------- |
| 🎨 | `:art:` | コード構造/フォーマットの改善 |
| ⚡️ | `:zap:` / `:racehorse:` | パフォーマンス向上 |
| 🔥 | `:fire:` | コードまたはファイルの削除 |
| 🐛 | `:bug:` | バグ修正 |
| 🚑 | `:ambulance:` | 重要なパッチ |
| ✨ | `:sparkles:` | 新機能の導入 |
| 📝 | `:memo:` | ドキュメント作成 |
| 🚀 | `:rocket:` | 機能のデプロイ |
| 💄 | `:lipstick:` | UI とスタイルファイルの更新 |
| 🎉 | `:tada:` | 初回提交 |
| ✅ | `:white_check_mark:` | テストの追加 |
| 🔒 | `:lock:` | セキュリティ問題の修正 |
| 🍎 | `:apple:` | macOS での修正 |
| 🐧 | `:penguin:` | Linux での修正 |
| 🏁 | `:checkered_flag:` | Windows での修正 |
| 🤖 | `:robot:` | Android での修正 |
| 🍏 | `:green_apple:` | iOS での問題対応 |
| 🔖 | `:bookmark:` | リリース/バージョントagging |
| 🚨 | `:rotating_light:` | linter 警告の削除 |
| 🚧 | `:construction:` | 作業進行中 |
| 💚 | `:green_heart:` | CI ビルドの問題修正 |
| ⬇️ | `:arrow_down:` | 依存関係のダウングレード |
| ⬆️ | `:arrow_up:` | 依存関係のアップグレード |
| 📌 | `:pushpin:` | 依存関係を特定のバージョンに固定 |
| 👷 | `:construction_worker:` | CI ビルドシステムの追加 |
| 📈 | `:chart_with_upwards_trend:` | 分析またはトラッキングコードの追加 |
| ♻️ | `:recycle:` | コードのリファクタリング |
| 🔨 | `:hammer:` | 大規模なリファクタリング |
| ➖ | `:heavy_minus_sign:` | 依存関係を1つ削除 |
| 🐳 | `:whale:` | Docker 関連作業 |
| ➕ | `:heavy_plus_sign:` | 依存関係を1つ追加 |
| 🔧 | `:wrench:` | 設定ファイルの変更 |
| 🌐 | `:globe_with_meridians:` | 国際化とローカライズ |
| ✏️ | `:pencil2:` | typo 修正 |
| 💩 | `:hankey:` | 改善が必要な悪いコードの記述 |
| ⏪️ | `:rewind:` | 変更の復元 |
| 🔀 | `:twisted_rightwards_arrows:` | ブランチのマージ |
| 📦 | `:package:` | コンパイル済みファイルまたはパッケージの更新 |
| 👽 | `:alien:` | 外部 API の変更に伴うコード更新 |
| 🚚 | `:truck:` | ファイルの移動または名前変更 |
| 📄 | `:page_facing_up:` | ライセンスの追加または更新 |
| 💥 | `:boom:` | 破壊的変更の導入 |
| 🍱 | `:bento:` | アセットの追加または更新 |
| 👌 | `:ok_hand:` | コードレビューによる変更 |
| ♿️ | `:wheelchair:` | アクセシビリティの向上 |
| 💡 | `:bulb:` | ソースコードの文書化 |
| 🍻 | `:beers:` | 酔っ払いながらコードを書く |
| 💬 | `:speech_balloon:` | テキストと文言の更新 |
| 🗃️ | `:card_file_box:` | データベース関連の処理実行 |
| 🔊 | `:loud_sound:` | ログの追加 |
| 🔇 | `:mute:` | ログの削除 |
| 👥 | `:busts_in_silhouette:` | コントリビューターの追加 |
| 🚸 | `:children_crossing:` | ユーザー体験/ユーザビリティの改善 |
| 🏗️ | `:building_construction:` | アーキテクチャ変更の実行 |
| 📱 | `:iphone:` | レスポンシブデザインに取り組む |
| 🤡 | `:clown_face:` | 物事をからかう |
| 🥚 | `:egg:` | イースターエッグの追加 |
| 🙈 | `:see_no_evil:` | .gitignore ファイルの追加または更新 |
| 📸 | `:camera_flash:` | スナップショットの追加または更新 |

---

## スコープ

スコープはプロジェクトの `src/` ディレクトリ構造に基づいており、**次の定義済み scope を使用する必要があります**：

### トップレベルモジュール

| スコープ | 対応ディレクトリ | 説明 |
|--------|----------|------|
| `frontend` | `src/frontend/` | フロントエンド：字句解析、構文解析、型チェック |
| `middle` | `src/middle/` | ミドル層：IR、最適化、モノモーフィゼーション |
| `backends` | `src/backends/` | バックエンド：インタープリター、ランタイム、REPL |
| `std` | `src/std/` | 標準ライブラリ |
| `formatter` | `src/formatter/` | コードフォーマッター |
| `lsp` | `src/lsp/` | 言語サーバープロトコル |
| `package` | `src/package/` | パッケージマネージャー |
| `util` | `src/util/` | ユーティリティライブラリ：診断、キャッシュ、i18n |

### フロントエンドサブモジュール

| スコープ | 対応ディレクトリ | 説明 |
|--------|----------|------|
| `parser` | `src/frontend/core/parser/` | 構文解析器 |
| `lexer` | `src/frontend/core/lexer/` | 字句解析器 |
| `typecheck` | `src/frontend/core/typecheck/` | 型チェック |
| `types` | `src/frontend/core/types/` | 型システム定義 |

### ミドル層サブモジュール

| スコープ | 対応ディレクトリ | 説明 |
|--------|----------|------|
| `codegen` | `src/middle/passes/codegen/` | コード生成（バイトコード） |
| `monomorphize` | `src/middle/passes/monomorphize/` | モノモーフィゼーション処理 |
| `lifetime` | `src/middle/passes/lifetime/` | ライフタイム解析 |

### バックエンドサブモジュール

| スコープ | 対応ディレクトリ | 説明 |
|--------|----------|------|
| `repl` | `src/backends/dev/repl/` | REPL 対話型コマンドライン |
| `shell` | `src/backends/dev/shell.rs` | シェルコマンド処理 |
| `runtime` | `src/backends/runtime/` | ランタイム実行エンジン |

### ドキュメントスコープ

| スコープ | 説明 |
|--------|------|
| `docs` | 汎用ドキュメント更新 |
| `design` | 言語設計仕様（RFC） |
| `plan` | 実装計画ドキュメント |

### その他のスコープ

| スコープ | 説明 |
|--------|------|
| `build` | ビルドシステム、Cargo 設定 |
| `ci` | CI/CD 設定（GitHub Actions） |
| `test` | テスト関連 |
| `release` | リリース関連 |
| `meta` | プロジェクトメタ設定（.claude, .gitignore 等） |

---

## メッセージ規範

### バージョニング

バージョン番号はプロジェクトのルートディレクトリ `Cargo.toml` の `version` フィールドで定義されています：

```toml
[package]
version = "0.7.2"
```

セマンティックバージョニング `MAJOR.MINOR.PATCH` を採用しています：

| バージョンタイプ | 説明 | 例 |
|----------|------|------|
| **major** | 下位互換性のない API 変更を含む大きな更新 | 0.7.2 → 1.0.0 |
| **minor** | 下位互換性のある新機能 | 0.7.2 → 0.8.0 |
| **patch** | 下位互換性のあるバグ修正 | 0.7.2 → 0.7.3 |

> ⚠️ リリース時は **dev ブランチで `Cargo.toml` のバージョン番号を更新し**、PR を通じて main にマージすると CI が自動的に tag と Release を作成します。**tag を手動でプッシュしないでください**。さもなくば CI が release フローをスキップします。

---

## CI リリースフロー

リリースは GitHub Actions（`release.yml`）によって自動化され、フローは次のとおりです：

```
1. dev ブランチで Cargo.toml の version フィールドを更新
2. cargo build で Cargo.lock を更新
3. リリースフォーマットに従って commit（下記 🔖 リリース提交参照）
   - commit message には前回のリリース以来的すべての変更を含める必要がある
     （つまり、PR の完全な内容）
4. dev から main へ PR を作成
5. PR を main にマージ
6. CI が自動的に検出：
   - Cargo.toml のバージョン番号を読み取り → "v{version}"
   - その tag が既に存在するかをチェック
   - 存在しない場合 → 完全なリリースフローをトリガー
   - 既に存在する場合 → スキップ（重複リリースなし）
7. CI が自動的に実行：
   - 並列：クロスプラットフォームビルド (Linux/Windows/macOS) 
     + セキュリティ監査 + テスト
   - すべて成功後：tag 作成、パッケージ作成、GitHub Release 公開
```

### 重要なルール

| ルール | 説明 |
|------|------|
| **tag を手動でプッシュしない** | CI は tag の存在に応じてリリース実行要不要を決める。手動 push は CI のスキップを引き起こす |
| **バージョン BUM は dev で行う** | リリース commit は dev で行い、PR を通じて main にマージする |
| **リリース commit には完全な changelog を含める** | commit message には今回のリリースのすべての変更内容を含める必要がある。これは PR の説明の元になる |
| **main を dev にマージバックしない** | PR マージ後、dev は自動的に同期される。逆マージは不要 |

---

## メッセージ規範

### 言語規範

**日本語での提交メッセージの使用をお勧めします**。チーム内のコミュニケーションの一貫性を保つためです。

- Subject は日本語で、簡潔明了に使用
- Body は日本語で詳細に説明可能
- 特殊な技術用語がある場合は、英語を保持可能

### Subject（テーマ）

- 日本語で簡潔明了に使用
- 50文字を超えない
- 末尾に句点を付けない

### Body（ボディ）

- 変更理由と方法を詳細に説明
- 各行は72文字を超えない
- 要点を列出には - または * を使用

### Footer（フッター）

- **破壊的変更**: `BREAKING CHANGE:` で始まる
- **Issue の关闭**: `关闭 #123` または `Fixes #456` を使用

---

## 例

### ✨ feat - 新機能

```
:sparkles: feat(parser): クロージャ構文解析サポートの追加

クロージャ式解析の実装：
- |args| body 省略構文のサポート
- move 意味キャプチャのサポート
- クロージャ型推論の追加

#42 をクローズ
```

### 🐛 fix - バグ修正

```
:bug: fix(repl): 複数行入力時に補完機能が動作しない問題の修正

SessionREPL は複数行モード時に補完器を正しく登録せず、
Tab 補完がトリガーされない問題。

#128 を修正
```

### 📝 docs - ドキュメント更新

```
:memo: docs(design): 所有権モデルと型システム仕様の更新

RFC-009 と RFC-011 の最新設計変更を同期。
```

### ♻️ refactor - リファクタリング

```
:recycle: refactor(typecheck): プリミティブ値型と Dup シャローコピー意味の分離

MonoType 内の値型とコピー意味を分離し、
match ブランチ内の特殊ケースを解消。
```

### ⚡️ perf - パフォーマンス最適化

```
:zap: perf(types): const ジェネリクス評価パフォーマンスの最適化

再帰評価に深さ制限を追加（デフォルト 128）、
悪意のある型式によるスタックオーバーフローを防止。
```

### ✅ test - テスト

```
:white_check_mark: test(typecheck): scope VarInfo 可変性テストの補足

カバレッジシナリオ：
- 不変バインディングの読み取り専用アクセス
- mut バインディング可変性トラッキング
- スコープ間可変性伝播
```

### 🔧 chore - 雑多

```
:wrench: chore(build): rand, hashbrown, tempfile, ron, clap を bump

6つの本番依存関係を最新安定版にアップグレード。
```

### 🚀 ci - CI 設定

```
:rocket: ci: nightly ビルド Rust バージョンが低すぎる問題の修正

RUST_TOOLCHAIN を 1.91.0 から 1.96.0 に更新し、
Cargo.toml の rust-version 要件と一致させる。
```

### 💄 style - フォーマット調整

```
:lipstick: style(frontend): cargo fmt フォーマットの適用

関数シグネチャの改行スタイルを統一。
```

---

---

## 🔖 リリース提交

本次の提交が**リリース（Release）**である場合は、以下の規範に従う必要があります：

### リリース提交フォーマット

```
:bookmark: V<バージョン番号>: <リリースタイトル>

## 📦 バージョン情報

**发布日期:** YYYY-MM-DD

**バージョン番号:** <旧バージョン> → <新バージョン>

---

## ✨ 新機能

### <機能モジュール>
- :sparkles: feat(<scope>): <機能説明>

---

## ♻️ リファクタリング最適化

- :recycle: refactor(<scope>): <リファクタリング説明>

---

## 🐛 バグ修正

- :bug: fix(<scope>): <修正説明>

---

## 🔧 その他の変更

- :wrench: chore: <変更説明>

---

## 📦 新規追加ファイル

- `<ファイルパス>` - <ファイル説明>

---

## 📝 提交記録

| 提交 | 説明 |
|:---:|------|
| `<hash>` | :bookmark: V<バージョン番号> |
| `<hash>` | <提交メッセージ> |
```

### リリース要件

1. **メッセージヘッダー**: `:bookmark:` + `V<バージョン番号>` 形式を使用する必要があります
2. **バージョン番号**: セマンティックバージョニング仕様に従う
3. **内容の完全性**: 前回のリリース以来的**すべての commit** 内容紹介を含める必要があります
4. **タイプ別分類**: `feat`, `fix`, `refactor`, `chore` などのタイプ別に整理
5. **提交記録**: 関連するすべての commit の hash と説明リスト

### リリース例

```
:bookmark: V0.7.2: REPL 書き直しと型システム改善

## 📦 バージョン情報

**发布日期:** 2026-06-01

**バージョン番号:** 0.7.1 → 0.7.2

---

## ✨ 新機能

- :sparkles: feat(typecheck): ジェネリクス型パラメータ自動推論の実装
- :sparkles: feat(typecheck): MonoType::Generic 構造化ジェネリクス表現の追加
- feat: CLI REPL コマンドを SessionREPL に接続

---

## ♻️ リファクタリング最適化

- :recycle: refactor(backends): tui_repl モジュールを削除し SessionREPL に書き直し
- :recycle: refactor(typecheck): scope 変数存储に VarInfo トラッキング可变性を導入
- :recycle: refactor(typecheck): プリミティブ値型と Dup シャローコピー意味の分離

---

## 🐛 バグ修正

- :bug: fix(repl): REPL 履歴のデフォルト設定と shell evaluate_code の修正
- :bug: fix(repl): 補完器の登録と複数行入力の修正
- :bug: fix(repl): wrap_code の余分なセミコロンを削除して式値を保持

---

## ⚡ パフォーマンス最適化

- :zap: perf(types): const ジェネリクス評価に再帰深さ制限を追加

---

## 🔧 その他の変更

- :wrench: chore(build): rand, hashbrown, tempfile, ron, clap, owo-colors を bump
- :white_check_mark: test(typecheck): scope VarInfo 可変性テストの補足

---

## 📝 提交記録

| 提交 | 説明 |
|:---:|------|
| `f438aab` | :sparkles: feat(typecheck): ジェネリクス型パラメータ自動推論の実装 |
| `bf0c121` | :zap: perf(types): 再帰深さ制限 |
| `6edac15` | feat: CLI REPL を SessionREPL に接続 |
| `02cf54f` | :sparkles: feat(typecheck): MonoType::Generic |
| `3160a28` | :recycle: refactor(typecheck): VarInfo 可変性トラッキング |
| `f00a2a4` | :recycle: refactor(backends): tui_repl モジュールの削除 |
| `afe3e0c` | :bug: fix(repl): REPL 履歴と shell 修正 |
| `c4d2242` | :wrench: chore(build): 依存関係 bump |
```

### 提交記録の取得方法

```bash
# 前回のリリース以来的すべての提交を表示
git log --oneline <前回のリリースcommit>..HEAD

# または最近 N 件の提交を表示
git log --oneline -20
```

### リファレンステンプレート

リリースドキュメントは [`release.md`](release.md) テンプレートフォーマットを参照して作成してください。

---

### 1. Commit Template の設定

```bash
# プロジェクトルートディレクトリで実行
git config commit.template .gitmessage.txt
```

### 2. Template ファイル

プロジェクトルートディレクトリの `.gitmessage.txt` ファイル形式は次のとおりです：

```
# emojiコード type(scope): テーマ（日本語）
#
# ボディ内容（任意）
#
# フッター（任意）
#
# Types: ✨feat, 🐛fix, 📝docs, 💄style, ♻️refactor, ⚡️perf, ✅test, 🔧chore, 🚀ci, 🔖release
# Scopes: frontend, parser, lexer, typecheck, types, middle, codegen,
#         monomorphize, lifetime, backends, repl, shell, runtime,
#         std, formatter, lsp, package, util, docs, design, plan,
#         build, ci, test, release, meta
#
# 例:
# ✨ feat(db): バッチ削除 Todo 機能の追加
# 🐛 fix(provider): タイマーonetask バックグラウンド復元問題の修正
#
# リリース形式: 🔖 V1.0.0: リリースタイトル
```

---

## よくある質問

### Q: 提交タイプはどうやって選択しますか？

- **feat**: ユーザーが目にする機能の更改
- **fix**: ユーザーが報告した問題の修正
- **docs**: README、コメントなどのドキュメント
- **chore**: 依存関係更新、設定ファイル
- **refactor**: 動作を変えないコード最適化

### Q: いつ提交を分割すべきですか？

- 各提交は**1つのことだけ**を行う
- 関連する機能は一緒に提交し、関連しないものは分ける
- Atomic Commits の原則に従う

---

## 参考資料

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [emoji.md](emoji.md) - Emoji 完全リスト
- [release.md](release.md) - リリーステンプレート

---

> 💡 **ヒント**: 提交の原子性を保ち、説明を明確にすることで、コードレビューと追跡がより効率的になります！