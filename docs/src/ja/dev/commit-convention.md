# Commit 提交ガイドライン

本文書は YaoXiang プロジェクトの Git コミット規範を定義したもので、コミット履歴を明確で読みやすく、理解しやすいものにすることを目的としています。

---

## 目次

- [コミット形式](#コミット形式)
- [コミットタイプ](#コミットタイプ)
- [完全 Emoji リファレンス](#完全-emoji-リファレンス)
- [スコープ](#スコープ)
- [バージョン管理](#バージョン管理)
- [メッセージ規範](#メッセージ規範)
- [言語規範](#言語規範)
- [🔖 リリースコミット](#-リリースコミット)
- [示例](#示例)
- [Commit Template の使用](#commit-template-の使用)
- [よくある質問](#よくある質問)

---

## コミット形式

**非常重要！！！！絶対に忘れないでください！！！**
すべてのコミットメッセージは以下の形式に従います：

```
:emojiコード: type(scope): 件名（日本語）

[オプションの本文]

[オプションのフッター]
```

> ⚠️
> **重要**: 直接的な emoji 文字ではなく、**emoji コード**（例：`:sparkles:`）を使用する必要があります。
>
> **日本語でのコミットメッセージを推奨します**。チーム間のコミュニケーションの一貫性を保つためです。

### 構成要素

| 要素        | 説明                                    | 必須 |
| ----------- | --------------------------------------- | ---- |
| emojiコード | コミットタイプを示す絵文字              | ✅   |
| type        | コミットタイプ                          | ✅   |
| scope       | 影響範囲                                | ✅   |
| subject     | 簡潔な説明（日本語、50文字以内）        | ✅   |
| body        | 詳細な説明（オプション）                | ❌   |
| footer      | 破壊的変更または 이슈終了（オプション） | ❌   |

---

## コミットタイプ

| emojiコード             | type     | 説明                                 |
| ----------------------- | -------- | ------------------------------------ |
| :sparkles:              | feat     | 新機能                               |
| :bug:                   | fix      | バグ修正                             |
| :memo:                  | docs     | ドキュメント変更のみ                 |
| :lipstick:              | style    | コードフォーマット（機能に影響なし） |
| :recycle:               | refactor | リファクタリング                     |
| :zap:                   | perf     | パフォーマンス最適化                 |
| :white_check_mark:      | test     | テストの追加または変更               |
| :wrench:                | chore    | ビルドツール、補助ツールの変更       |
| :building_construction: | build    | ビルドシステム変更                   |
| :rocket:                | ci       | CI 設定変更                          |

---

## 完全 Emoji リファレンス

以下は gitmoji プロジェクトと一致する完全な emoji リストで、コミット内容に応じて適切な emoji を選択できます：

| emoji | emoji コード                  | commit 説明                                    |
| :---- | :---------------------------- | :--------------------------------------------- |
| 🎨    | `:art:`                       | コード構造/フォーマット改善                    |
| ⚡️    | `:zap:` / `:racehorse:`       | パフォーマンス向上                             |
| 🔥    | `:fire:`                      | コードまたはファイルの削除                     |
| 🐛    | `:bug:`                       | バグ修正                                       |
| 🚑    | `:ambulance:`                 | 重要なパッチ                                   |
| ✨    | `:sparkles:`                  | 新機能の導入                                   |
| 📝    | `:memo:`                      | ドキュメント作成                               |
| 🚀    | `:rocket:`                    | 機能のデプロイ                                 |
| 💄    | `:lipstick:`                  | UI とスタイルファイルの更新                    |
| 🎉    | `:tada:`                      | 最初のコミット                                 |
| ✅    | `:white_check_mark:`          | テストの増加                                   |
| 🔒    | `:lock:`                      | セキュリティ問題の修正                         |
| 🍎    | `:apple:`                     | macOS でのコンテンツの修正                     |
| 🐧    | `:penguin:`                   | Linux でのコンテンツの修正                     |
| 🏁    | `:checkered_flag:`            | Windows でのコンテンツの修正                   |
| 🤖    | `:robot:`                     | Android でのコンテンツの修正                   |
| 🍏    | `:green_apple:`               | iOS での問題の解決                             |
| 🔖    | `:bookmark:`                  | リリース/バージョンチーム                      |
| 🚨    | `:rotating_light:`            | linter 警告の削除                              |
| 🚧    | `:construction:`              | 作業進行中                                     |
| 💚    | `:green_heart:`               | CI ビルド問題の修正                            |
| ⬇️    | `:arrow_down:`                | 依存関係のダウングレード                       |
| ⬆️    | `:arrow_up:`                  | 依存関係のアップグレード                       |
| 📌    | `:pushpin:`                   | 依存関係を特定バージョンに固定                 |
| 👷    | `:construction_worker:`       | CI ビルドシステムの追加                        |
| 📈    | `:chart_with_upwards_trend:`  | 分析またはトラッキングコードの追加             |
| ♻️    | `:recycle:`                   | コードリファクタリング                         |
| 🔨    | `:hammer:`                    | 大規模なリファクタリング                       |
| ➖    | `:heavy_minus_sign:`          | 依存関係を1つ削除                              |
| 🐳    | `:whale:`                     | Docker 関連作業                                |
| ➕    | `:heavy_plus_sign:`           | 依存関係を1つ追加                              |
| 🔧    | `:wrench:`                    | 設定ファイルの変更                             |
| 🌐    | `:globe_with_meridians:`      | 国際化とローカライズ                           |
| ✏️    | `:pencil2:`                   | typo の修正                                    |
| 💩    | `:hankey:`                    | 改善が必要な悪いコードを書く                   |
| ⏪️    | `:rewind:`                    | 変更の復元                                     |
| 🔀    | `:twisted_rightwards_arrows:` | ブランチのマージ                               |
| 📦    | `:package:`                   | コンパイルされたファイルまたはパッケージの更新 |
| 👽    | `:alien:`                     | 外部 API の変更によりコードを更新              |
| 🚚    | `:truck:`                     | ファイルの移動または名前変更                   |
| 📄    | `:page_facing_up:`            | ライセンスの追加または更新                     |
| 💥    | `:boom:`                      | breaking change の導入                         |
| 🍱    | `:bento:`                     | アセットの追加または更新                       |
| 👌    | `:ok_hand:`                   | コードレビューによりコードを更新               |
| ♿️    | `:wheelchair:`                | アクセシビリティの向上                         |
| 💡    | `:bulb:`                      | ソースコードのドキュメント化                   |
| 🍻    | `:beers:`                     | 酔っ払いながらコードを書く                     |
| 💬    | `:speech_balloon:`            | テキストと文字の更新                           |
| 🗃️    | `:card_file_box:`             | データベース関連の変更を実行                   |
| 🔊    | `:loud_sound:`                | ログの追加                                     |
| 🔇    | `:mute:`                      | ログの削除                                     |
| 👥    | `:busts_in_silhouette:`       | コントリビューターの追加                       |
| 🚸    | `:children_crossing:`         | ユーザー体験/ユーザビリティの改善              |
| 🏗️    | `:building_construction:`     | アーキテクチャ変更を実行                       |
| 📱    | `:iphone:`                    | レスポンシブデザインに取り組む                 |
| 🤡    | `:clown_face:`                | 物事をからかう                                 |
| 🥚    | `:egg:`                       | イースターエッグを追加                         |
| 🙈    | `:see_no_evil:`               | .gitignore ファイルの追加または更新            |
| 📸    | `:camera_flash:`              | スナップショットの追加または更新               |

---

## スコープ

スコープはプロジェクトの `src/`
ディレクトリ構造に基づいており、**以下の定義済み scope を使用する必要があります**：

### トップレベルモジュール

| スコープ    | 対応ディレクトリ | 説明                                             |
| ----------- | ---------------- | ------------------------------------------------ |
| `frontend`  | `src/frontend/`  | フロントエンド：字句解析、構文解析、型チェック   |
| `middle`    | `src/middle/`    | ミドルウェア：IR、最適化、モノモーフィズム       |
| `backends`  | `src/backends/`  | バックエンド：インタプリタ、ランタイム、REPL     |
| `std`       | `src/std/`       | 標準ライブラリ                                   |
| `formatter` | `src/formatter/` | コードフォーマッタ                               |
| `lsp`       | `src/lsp/`       | 言語サーバープロトコル                           |
| `package`   | `src/package/`   | パッケージマネージャー                           |
| `util`      | `src/util/`      | ユーティリティライブラリ：診断、キャッシュ、i18n |

### フロントエンドサブモジュール

| スコープ    | 対応ディレクトリ               | 説明           |
| ----------- | ------------------------------ | -------------- |
| `parser`    | `src/frontend/core/parser/`    | 構文解析器     |
| `lexer`     | `src/frontend/core/lexer/`     | 字句解析器     |
| `typecheck` | `src/frontend/core/typecheck/` | 型チェック     |
| `types`     | `src/frontend/core/types/`     | 型システム定義 |

### ミドルウェアサブモジュール

| スコープ       | 対応ディレクトリ                  | 説明                       |
| -------------- | --------------------------------- | -------------------------- |
| `codegen`      | `src/middle/passes/codegen/`      | コード生成（バイトコード） |
| `monomorphize` | `src/middle/passes/monomorphize/` | モノモーフィズム処理       |
| `lifetime`     | `src/middle/passes/lifetime/`     | ライフタイム解析           |

### バックエンドサブモジュール

| スコープ  | 対応ディレクトリ            | 説明                      |
| --------- | --------------------------- | ------------------------- |
| `repl`    | `src/backends/dev/repl/`    | REPL 対話型コマンドライン |
| `shell`   | `src/backends/dev/shell.rs` | シェルコマンド処理        |
| `runtime` | `src/backends/runtime/`     | ランタイム実行エンジン    |

### ドキュメントスコープ

| スコープ | 説明                 |
| -------- | -------------------- |
| `docs`   | 汎用ドキュメント更新 |
| `design` | 言語設計仕様（RFC）  |
| `plan`   | 実装計画ドキュメント |

### その他のスコープ

| スコープ  | 説明                                             |
| --------- | ------------------------------------------------ |
| `build`   | ビルドシステム、Cargo 設定                       |
| `ci`      | CI/CD 設定（GitHub Actions）                     |
| `test`    | テスト関連                                       |
| `release` | リリース関連                                     |
| `meta`    | プロジェクトメタ設定（.claude, .gitignore など） |

---

## メッセージ規範

### バージョン管理

バージョン番号はプロジェクトのルートディレクトリにある `Cargo.toml` の `version`
フィールドで定義されます：

```toml
[package]
version = "0.7.2"
```

セマンティックバージョニング `MAJOR.MINOR.PATCH` を採用します：

| バージョンタイプ | 説明                              | 例            |
| ---------------- | --------------------------------- | ------------- |
| **major**        | 非互換の API 変更を含む重大な更新 | 0.7.2 → 1.0.0 |
| **minor**        | 新機能、後方互換                  | 0.7.2 → 0.8.0 |
| **patch**        | バグ修正、後方互換                | 0.7.2 → 0.7.3 |

> ⚠️ リリース時に **dev ブランチで `Cargo.toml` のバージョン番号を更新します**
> PR を main にマージした後、CI が自動的に tag と Release を作成します。**手動で tag をプッシュしないでください**。さもなくば CI が release プロセスをスキップします。

---

## CI リリースプロセス

リリースは GitHub Actions（`release.yml`）により自動化され、プロセスは以下の通りです：

```
1. dev ブランチで Cargo.toml の version フィールドを更新
2. cargo build で Cargo.lock を更新
3. リリース形式で commit（下記 🔖 リリースコミットを参照）
   - commit message には前回リリースからのすべての変更を含める必要がある
   （つまり PR の完全な内容）
4. dev から main への PR を作成
5. PR を main にマージ
6. CI が自動的に検出：
   - Cargo.toml のバージョン番号を読み取り → "v{version}"
   - その tag が既に存在するかどうかを確認
   - 存在しない場合 → 完全な release プロセスをトリガー
   - 存在する場合 → スキップ（再发布なし）
7. CI が自動的に実行：
   - 並列処理：クロスプラットフォームビルド（Linux/Windows/macOS）
     + セキュリティ監査 + テスト
   - すべて成功后：tag の作成、パッケージ成果物の生成、GitHub Release の公開
```

### 重要なルール

| ルール                                            | 説明                                                                                   |
| ------------------------------------------------- | -------------------------------------------------------------------------------------- |
| **手動で tag をプッシュしない**                   | CI は tag の存在に基づいてリリース可否を決定するため、手動プッシュ会导致 CI がスキップ |
| **バージョン番号は dev 上で bump**                | リリース commit は dev 上で完了し、PR を介して main にマージ                           |
| **リリース commit には完全な changelog を含める** | commit message には本次リリースの全変更内容を含める必要がある。これは PR の説明元      |
| **main を dev にマージバックしない**              | PR マージ後、dev は自動的に同期されるため、リバースマージは不要                        |

---

## メッセージ規範

### 言語規範

**日本語でのコミットメッセージを推奨します**。チーム間のコミュニケーションの一貫性を保つためです。

- Subject は日本語で、簡潔明瞭
- Body は日本語で詳細に説明可能
- 特別な技術用語がある場合は英語のままでも可

### Subject（件名）

- 日本語で、簡潔明瞭
- 50文字以内
- 末尾に句点は不要

### Body（本文）

- 変更理由と方法を詳細に説明
- 各行は72文字以内
- 要点は - または * で列出

### Footer（フッター）

- **破壊的変更**: `BREAKING CHANGE:` で始める
- **Issue の終了**: `Closes #123` または `Fixes #456` を使用

---

## 示例

### ✨ feat - 新機能

```
:sparkles: feat(parser): クロージャ構文解析サポートを追加

クロージャ式の解析を実装：
- |args| body 短縮構文をサポート
- move 意味キャプチャをサポート
- クロージャ型推論を追加

Closes #42
```

### 🐛 fix - バグ修正

```
:bug: fix(repl): 複数行入力時に補完機能が無効化する問題を修正

SessionREPL は複数行モードで補完器を正しく登録せず、
Tab 補完がトリガーされない問題。

Fixes #128
```

### 📝 docs - ドキュメント更新

```
:memo: docs(design): 所有権モデルと型システム仕様を更新

RFC-009 と RFC-011 の最新設計変更を同期。
```

### ♻️ refactor - リファクタリング

```
:recycle: refactor(typecheck): プリミティブ値型と Dup 浅いコピーセマンティクスを分離

MonoType の値型とコピーセマンティクスを分離し、
match 分岐内の特殊ケースを解消。
```

### ⚡️ perf - パフォーマンス最適化

```
:zap: perf(types): const generic 評価パフォーマンスを最適化

再帰的評価に深度制限を追加（デフォルト 128）、
悪意のある型式によるスタックオーバーフローを防止。
```

### ✅ test - テスト

```
:white_check_mark: test(typecheck): scope VarInfo 可変性テストを補充

カバレッジシナリオ：
- 不変バインディングの読み取り専用アクセス
- mut バインディングの可変性追跡
- スコープ間の可変性伝播
```

### 🔧 chore - 雑多な作業

```
:wrench: chore(build): rand, hashbrown, tempfile, ron, clap を bump

6つの本番依存関係を最新安定版にアップグレード。
```

### 🚀 ci - CI 設定

```
:rocket: ci: nightly ビルドの Rust バージョンが低すぎる問題を修正

RUST_TOOLCHAIN を 1.91.0 から 1.96.0 に更新し、
Cargo.toml の rust-version 要件と一致させる。
```

### 💄 style - フォーマット調整

```
:lipstick: style(frontend): cargo fmt フォーマットを適用

関数シグネチャの改行スタイルを統一。
```

---

---

## 🔖 リリースコミット

本次コミットが**リリース（Release）**の場合、以下の規範に従う必要があります：

### リリースコミット形式

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

## 📝 コミット記録

| コミット | 説明 |
|:---:|------|
| `<hash>` | :bookmark: V<バージョン番号> |
| `<hash>` | <コミットメッセージ> |
```

### リリース要件

1. **メッセージヘッダー**: `:bookmark:` + `V<バージョン番号>` 形式を使用する必要があります
2. **バージョン番号**: セマンティックバージョニング仕様に従う
3. **内容の完全性**: 前回リリースからの**すべての commit** 内容紹介を含める必要があります
4. **タイプ別分類**: `feat`, `fix`, `refactor`, `chore` などのタイプ別に整理
5. **コミット記録**: 関連するすべてのコミットの hash と説明列出

### リリース例

```
:bookmark: V0.7.2: REPL 書き直しと型システム改善

## 📦 バージョン情報

**发布日期:** 2026-06-01

**バージョン番号:** 0.7.1 → 0.7.2

---

## ✨ 新機能

- :sparkles: feat(typecheck): ジェネリク 型パラメータの自動推論を実装
- :sparkles: feat(typecheck): MonoType::Generic 構造化ジェネリク 表現を追加
- feat: CLI REPL コマンドを SessionREPL に接続

---

## ♻️ リファクタリング最適化

- :recycle: refactor(backends): tui_repl モジュールを削除し SessionREPL に書き直し
- :recycle: refactor(typecheck): scope 変数存储に VarInfo 可変性追跡を導入
- :recycle: refactor(typecheck): プリミティブ値型と Dup 浅いコピーセマンティクスを分離

---

## 🐛 バグ修正

- :bug: fix(repl): REPL 履歴のデフォルト設定を構成、shell evaluate_code を修正
- :bug: fix(repl): 補完器を登録し複数行入力を修正
- :bug: fix(repl): wrap_code の余分なセミコロンを削除して式値を保持

---

## ⚡ パフォーマンス最適化

- :zap: perf(types): const generic 評価に再帰深度制限を追加

---

## 🔧 その他の変更

- :wrench: chore(build): rand, hashbrown, tempfile, ron, clap, owo-colors を bump
- :white_check_mark: test(typecheck): scope VarInfo 可変性テストを補充

---

## 📝 コミット記録

| コミット | 説明 |
|:---:|------|
| `f438aab` | :sparkles: feat(typecheck): ジェネリク 型パラメータの自動推論を実装 |
| `bf0c121` | :zap: perf(types): 再帰深度制限 |
| `6edac15` | feat: CLI REPL を SessionREPL に接続 |
| `02cf54f` | :sparkles: feat(typecheck): MonoType::Generic |
| `3160a28` | :recycle: refactor(typecheck): VarInfo 可変性追跡 |
| `f00a2a4` | :recycle: refactor(backends): tui_repl モジュールを削除 |
| `afe3e0c` | :bug: fix(repl): REPL 履歴とシェル修正 |
| `c4d2242` | :wrench: chore(build): 依存関係 bump |
```

### コミット記録の取得方法

```bash
# 前回リリースからのすべてのコミットを表示
git log --oneline <前回リリースコミット>..HEAD

# または最近 N 件のコミットを表示
git log --oneline -20
```

### 参照テンプレート

リリースドキュメントは [`release.md`](release.md) テンプレート形式を参照して作成してください。

---

### 1. Commit Template の設定

```bash
# プロジェクトルートディレクトリで実行
git config commit.template .gitmessage.txt
```

### 2. Template ファイル

プロジェクトルートディレクトリの `.gitmessage.txt` ファイル形式は以下通りです：

```
# emojiコード type(scope): 件名（日本語）
#
# 本文（オプション）
#
# フッター（オプション）
#
# Types: ✨feat, 🐛fix, 📝docs, 💄style, ♻️refactor, ⚡️perf, ✅test, 🔧chore, 🚀ci, 🔖release
# Scopes: frontend, parser, lexer, typecheck, types, middle, codegen,
#         monomorphize, lifetime, backends, repl, shell, runtime,
#         std, formatter, lsp, package, util, docs, design, plan,
#         build, ci, test, release, meta
#
# 例:
# ✨ feat(db): 一括削除功能的追加
# 🐛 fix(provider): タイマーのバックグラウンド回復問題を修正
#
# リリース形式: 🔖 V1.0.0: リリースタイトル
```

---

## よくある質問

### Q: コミットタイプはどのように選択すればよいですか？

- **feat**: ユーザーが目に見える機能変更
- **fix**: ユーザーが報告した問題の修正
- **docs**: README、コメントなどのドキュメント
- **chore**: 依存関係の更新、設定ファイル
- **refactor**: 動作を変えないコード最適化

### Q: いつコミットを分割すべきですか？

- 各コミットは**1つのことだけ**を行います
- 関連する機能は一緒にコミットし、関連しないものは別々にコミット
- Atomic Commits の原則に従います

---

## 参考資料

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [emoji.md](emoji.md) - Emoji 完全リスト
- [release.md](release.md) - リリーステンプレート

---

> 💡
> **ヒント**: コミットをアトミックに保ち、説明を明確にすることで、コードレビューと追跡がより効率的になります！
