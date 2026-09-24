# Commit 提交ガイドライン

本文書は YaoXiang プロジェクトの Git コミット規範を定義したものであり、コミット履歴を明確で読みやすく、理解しやすいものにすることを目的としています。

---

## 目次

- [コミット形式](#コミット形式)
- [コミットタイプ](#コミットタイプ)
- [完全な Emoji 参照](#完全な-emoji-参照)
- [スコープ](#スコープ)
- [バージョン管理](#バージョン管理)
- [メッセージ規範](#メッセージ規範)
- [言語規範](#言語規範)
- [🔖 リリースコミット](#-リリースコミット)
- [例](#例)
- [Commit Template の使用](#commit-template-の使用)
- [よくある質問](#よくある質問)

---

## コミット形式

**非常に重要です！！！！絶対に忘れてはいけません！！！** すべてのコミットメッセージは次の形式に従います：

```
:emojiコード: type(scope): 主题（中国語）

[任意のボディコンテンツ]

[任意のフッター]
```

> ⚠️ **重要**: emoji コードを直接入力するのではなく、**emoji コード**（例：`:sparkles:`）を使用する必要があります。
>
> **中国語のコミットメッセージの使用を推奨**し、チーム内のコミュニケーションの一貫性を保ちます。

### 構成要素

| 部分       | 説明                             | 必須 |
| ---------- | -------------------------------- | ---- |
| emojiコード | 絵文字によるコミットタイプの識別 | ✅   |
| type       | コミットタイプ                   | ✅   |
| scope      | 影響範囲                         | ✅   |
| subject    | 簡潔な説明（中国語、50文字以内） | ✅   |
| body       | 詳細な説明（任意）               | ❌   |
| footer     | 破壊的変更またはIssue закрытие（任意） | ❌   |

---

## コミットタイプ

| emojiコード               | type     | 説明                   |
| ----------------------- | -------- | ---------------------- |
| :sparkles:              | feat     | 新機能                 |
| :bug:                   | fix      | バグ修正               |
| :memo:                  | docs     | ドキュメント変更のみ   |
| :lipstick:              | style    | コードフォーマット（機能に影響なし） |
| :recycle:               | refactor | リファクタリング       |
| :zap:                   | perf     | パフォーマンス最適化   |
| :white_check_mark:      | test     | テストの追加または変更 |
| :wrench:                | chore    | ビルドツール、補助ツールの変更 |
| :building_construction: | build    | ビルドシステム変更     |
| :rocket:                | ci       | CI 設定変更            |

---

## 完全な Emoji 参照

以下は gitmoji プロジェクトと一致する完全な emoji リストであり、コミット内容に応じて適切な emoji を選択できます：

| emoji | emoji コード                    | コミット説明                     |
| :---- | :---------------------------- | :-------------------------- |
| 🎨    | `:art:`                       | コード構造/フォーマットの改善   |
| ⚡️    | `:zap:` / `:racehorse:`       | パフォーマンス向上              |
| 🔥    | `:fire:`                      | コードまたはファイルの削除      |
| 🐛    | `:bug:`                       | バグ修正                       |
| 🚑    | `:ambulance:`                 | 重要パッチ                     |
| ✨    | `:sparkles:`                  | 新機能の導入                   |
| 📝    | `:memo:`                      | ドキュメント作成               |
| 🚀    | `:rocket:`                    | デプロイ機能                   |
| 💄    | `:lipstick:`                  | UI とスタイルファイルの更新    |
| 🎉    | `:tada:`                      | 初回コミット                   |
| ✅    | `:white_check_mark:`          | テスト追加                     |
| 🔒    | `:lock:`                      | セキュリティ問題の修正          |
| 🍎    | `:apple:`                     | macOS でのコンテンツの修正     |
| 🐧    | `:penguin:`                   | Linux でのコンテンツの修正     |
| 🏁    | `:checkered_flag:`            | Windows でのコンテンツの修正   |
| 🤖    | `:robot:`                     | Android でのコンテンツの修正    |
| 🍏    | `:green_apple:`               | iOS での問題の解決              |
| 🔖    | `:bookmark:`                  | リリース/バージョンタグ         |
| 🚨    | `:rotating_light:`            | linter 警告の削除              |
| 🚧    | `:construction:`              | 作業進行中                     |
| 💚    | `:green_heart:`               | CI ビルドの問題を修正          |
| ⬇️    | `:arrow_down:`                | 依存関係のダウングレード        |
| ⬆️    | `:arrow_up:`                  | 依存関係のアップグレード        |
| 📌    | `:pushpin:`                   | 依存関係を特定バージョンに固定   |
| 👷    | `:construction_worker:`       | CI ビルドシステムの追加         |
| 📈    | `:chart_with_upwards_trend:`  | 分析またはトラッキングコードの追加 |
| ♻️    | `:recycle:`                   | コードリファクタリング          |
| 🔨    | `:hammer:`                   | メジャーリファクタリング        |
| ➖    | `:heavy_minus_sign:`          | 依存関係を1つ削除               |
| 🐳    | `:whale:`                     | Docker 関連作業                 |
| ➕    | `:heavy_plus_sign:`           | 依存関係を1つ追加               |
| 🔧    | `:wrench:`                    | 設定ファイルの編集              |
| 🌐    | `:globe_with_meridians:`      | 国際化とローカライズ            |
| ✏️    | `:pencil2:`                   | typo の修正                    |
| 💩    | `:hankey:`                    | 改善が必要な悪いコードを書く     |
| ⏪️    | `:rewind:`                    | 変更を元に戻す                  |
| 🔀    | `:twisted_rightwards_arrows:` | ブランチのマージ               |
| 📦    | `:package:`                   | コンパイルされたファイルまたはパッケージの更新 |
| 👽    | `:alien:`                     | 外部 API の変更によりコードを更新 |
| 🚚    | `:truck:`                     | ファイルの移動または名前変更    |
| 📄    | `:page_facing_up:`            | ライセンスの追加または更新      |
| 💥    | `:boom:`                      | 破壊的変更の導入                |
| 🍱    | `:bento:`                     | アセットの追加または更新        |
| 👌    | `:ok_hand:`                   | コードレビューによる変更でコードを更新 |
| ♿️    | `:wheelchair:`                | アクセシビリティの向上          |
| 💡    | `:bulb:`                      | ソースコードのドキュメント      |
| 🍻    | `:beers:`                     | 酔っ払いながらコードを書く      |
| 💬    | `:speech_balloon:`            | テキストと文言の更新            |
| 🗃️    | `:card_file_box:`             | データベース関連の変更を実行    |
| 🔊    | `:loud_sound:`                | ログの追加                     |
| 🔇    | `:mute:`                      | ログの削除                     |
| 👥    | `:busts_in_silhouette:`       | コントリビューターの追加        |
| 🚸    | `:children_crossing:`         | ユーザー体験/使いやすさの改善    |
| 🏗️    | `:building_construction:`     | アーキテクチャの変更を行う      |
| 📱    | `:iphone:`                    | レスポンシブデザインに取り組む   |
| 🤡    | `:clown_face:`                | 物事をからかう                 |
| 🥚    | `:egg:`                       | イースターエッグを追加          |
| 🙈    | `:see_no_evil:`               | .gitignore ファイルの追加または更新 |
| 📸    | `:camera_flash:`              | スナップショットの追加または更新  |

---

## スコープ

スコープはプロジェクトの `src/` ディレクトリ構造に基づいており、**以下の定義済み scope を使用する必要があります**：

### トップレベルモジュール

| スコープ      | 対応ディレクトリ         | 説明                               |
| ----------- | ---------------- | ---------------------------------- |
| `frontend`  | `src/frontend/`  | フロントエンド：字句解析、構文解析、型チェック |
| `middle`    | `src/middle/`    | ミドル層：IR、最適化、モノホルフィズム       |
| `backends`  | `src/backends/`  | バックエンド：インタープリタ、ランタイム、REPL |
| `std`       | `src/std/`       | 標準ライブラリ                     |
| `formatter` | `src/formatter/` | コードフォーマッタ                 |
| `lsp`       | `src/lsp/`       | 言語サーバープロトコル             |
| `package`   | `src/package/`   | パッケージマネージャ               |
| `util`      | `src/util/`      | ユーティリティ：診断、キャッシュ、i18n      |

### フロントエンドサブモジュール

| スコープ      | 対応ディレクトリ                       | 説明         |
| ----------- | ------------------------------ | ------------ |
| `parser`    | `src/frontend/core/parser/`    | 構文解析器   |
| `lexer`     | `src/frontend/core/lexer/`     | 字句解析器   |
| `typecheck` | `src/frontend/core/typecheck/` | 型チェック   |
| `types`     | `src/frontend/core/types/`     | 型システム定義 |

### ミドル層サブモジュール

| スコープ         | 対応ディレクトリ                          | 説明               |
| -------------- | --------------------------------- | ------------------ |
| `codegen`      | `src/middle/passes/codegen/`      | コード生成（バイトコード） |
| `monomorphize` | `src/middle/passes/monomorphize/` | モノホルフィズム処理   |
| `lifetime`     | `src/middle/passes/lifetime/`     | ライフタイム解析       |

### バックエンドサブモジュール

| スコープ    | 対応ディレクトリ                    | 説明              |
| --------- | --------------------------- | ----------------- |
| `repl`    | `src/backends/dev/repl/`    | REPL 対話型コマンドライン |
| `shell`   | `src/backends/dev/shell.rs` | シェルコマンド処理    |
| `runtime` | `src/backends/runtime/`     | ランタイム実行エンジン   |

### ドキュメントスコープ

| スコープ   | 説明                |
| -------- | ------------------- |
| `docs`   | 汎用ドキュメント更新        |
| `design` | 言語設計仕様（RFC） |
| `plan`   | 実装計画ドキュメント        |

### その他のスコープ

| スコープ    | 説明                                 |
| --------- | ------------------------------------ |
| `build`   | ビルドシステム、Cargo 設定                 |
| `ci`      | CI/CD 設定（GitHub Actions）         |
| `test`    | テスト関連                             |
| `release` | リリース関連                             |
| `meta`    | プロジェクトメタ設定（.claude, .gitignore 等） |

---

## メッセージ規範

### バージョン管理

バージョン番号はプロジェクトルートの `Cargo.toml` の `version` フィールドで定義されています：

```toml
[package]
version = "0.7.2"
```

セマンティックバージョニング `MAJOR.MINOR.PATCH` を採用しています：

| バージョンタイプ  | 説明                        | 例          |
| --------- | --------------------------- | ------------- |
| **major** | 破壊的な更新、非互換の API 変更 | 0.7.2 → 1.0.0 |
| **minor** | 新機能、後方互換            | 0.7.2 → 0.8.0 |
| **patch** | バグ修正、後方互換          | 0.7.2 → 0.7.3 |

> ⚠️ リリース時に **dev ブランチで `Cargo.toml` のバージョン番号を更新し**、PR を介して main にマージすると CI が自動的に tag と Release を作成します。**tag を手動でプッシュせず**、否则 CI はリリースフローをスキップします。

---

## CI リリースフロー

リリースは GitHub Actions (`release.yml`) によって自動的に行われ、手順は以下の通りです：

```
1. dev ブランチで Cargo.toml の version フィールドを更新
2. cargo build で Cargo.lock を更新
3. リリース形式で commit（下記 🔖 リリースコミットを参照）
   - commit message には前回のリリースからのすべての変更を含める必要がある（つまり PR の全内容）
4. dev から main への PR を作成
5. PR を main にマージ
6. CI が自動的に検出：
   - Cargo.toml のバージョン番号を読み取り → "v{version}"
   - その tag が既に存在するかをチェック
   - 存在しない場合 → 完全なリリースフローをトリガー
   - 存在する場合 → スキップ（重複リリースなし）
7. CI が自動的に実行：
   - 並列：クロスプラットフォームビルド (Linux/Windows/macOS) + セキュリティ監査 + テスト
   - すべて成功後：tag の作成、パッケージビルド、GitHub Release の公開
```

### 重要ルール

| ルール                               | 説明                                                                |
| ---------------------------------- | ------------------------------------------------------------------- |
| **tag を手動でプッシュしない**                 | CI は tag の存在に応じてリリース可否を決定するため、手動プッシュ会导致 CI がスキップ |
| **バージョン番号は dev 上で bump する**             | リリース commit は dev 上で完了し、PR を介して main にマージ                      |
| **リリース commit には完全な changelog を含める** | commit message には今回のリリースの全変更内容を含める必要がある，这是因为它是 PR の説明元 |
| **main を dev にマージしない**           | PR マージ後、dev は自動的に同期され、逆マージは不要                          |

---

## メッセージ規範

### 言語規範

**中国語のコミットメッセージの使用を推奨**し、チーム内のコミュニケーションの一貫性を保ちます。

- Subject は中国語で、簡潔かつ明確に使用
- Body は中国語で詳細に説明してもよい
- 特殊な技術用語がある場合は、英単語を保持してもよい

### Subject（テーマ）

- 中国語で、簡潔かつ明確に使用
- 50文字を超えない
- 末尾に句点を付けない

### Body（ボディ）

- 変更理由と方法を詳細に説明
- 各行は72文字を超えない
- 要点を列挙するには - または * を使用

### Footer（フッター）

- **破壊的変更**: `BREAKING CHANGE:` で始める
- **Issue のクローズ**: `Closes #123` または `Fixes #456` を使用

---

## 例

### ✨ feat - 新機能

```
:sparkles: feat(parser): クロージャ構文解析サポートの追加

クロージャ式の解析を実装：
- |args| body 省略構文のサポート
- move 意味キャプチャのサポート
- クロージャ型推論の追加

Closes #42
```

### 🐛 fix - バグ修正

```
:bug: fix(repl): 複数行入力時にコンプリーターが機能しない問題を修正

SessionREPL が複数行モードでコンプライアンスを正しく登録せず、
Tab 補完がトリガーされない。

Fixes #128
```

### 📝 docs - ドキュメント更新

```
:memo: docs(design): 所有権モデルと型システム仕様の更新

RFC-009 と RFC-011 の最新の設計変更を同期。
```

### ♻️ refactor - リファクタリング

```
:recycle: refactor(typecheck): プリミティブ値型と Dup シャローコピーセマンティクスの分離

MonoType の値型とコピーセマンティクスを分離し、
match 分岐内の特殊ケースを排除。
```

### ⚡️ perf - パフォーマンス最適化

```
:zap: perf(types): 定数ジェネリクスの評価パフォーマンスを最適化

再帰評価に深度制限を追加（デフォルト 128），
悪意のある型式でスタックオーバーフローを回避。
```

### ✅ test - テスト

```
:white_check_mark: test(typecheck): scope VarInfo 可変性テストの補完

カバーするシナリオ：
- 不変バインディングの読み取り専用アクセス
- mut バインディングの可変性トラッキング
- スコープ間の可変性伝播
```

### 🔧 chore - 雑多な作業

```
:wrench: chore(build): rand, hashbrown, tempfile, ron, clap を bump

6つの本番依存を最新安定版にアップグレード。
```

### 🚀 ci - CI 設定

```
:rocket: ci: nightly ビルドの Rust バージョンが低すぎる問題を修正

RUST_TOOLCHAIN を 1.91.0 から 1.96.0 に更新し、
Cargo.toml の rust-version 要件に一致させる。
```

### 💄 style - フォーマット調整

```
:lipstick: style(frontend): cargo fmt フォーマットを適用

関数シグネチャの改行スタイルを統一。
```

---

---

## 🔖 リリースコミット

今回のコミットが**リリース（Release）**の場合、次の規範に従う必要があります：

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

```

### リリース要件

1. **メッセージヘッダー**: `:bookmark:` + `V<バージョン番号>` 形式を必ず使用
2. **バージョン番号**: セマンティックバージョニング規範に従う
3. **コンテンツの完全性**: 前回のリリースからの**すべての commit** 内容の紹介を含める必要がある
4. **タイプ別分類**: `feat`, `fix`, `refactor`, `chore` などのタイプ別に整理

### リリース例

```

:bookmark: V0.7.2: REPL の書き直しと型システムの改善

## 📦 バージョン情報

**发布日期:** 2026-06-01

**バージョン番号:** 0.7.1 → 0.7.2

---

## ✨ 新機能

- :sparkles: feat(typecheck): ジェネリクス型パラメータの自動推論を実装
- :sparkles: feat(typecheck): MonoType::Generic 構造化ジェネリクス表現を追加
- feat: CLI REPL コマンドを SessionREPL に接続

---

## ♻️ リファクタリング最適化

- :recycle: refactor(backends): tui_repl モジュールを削除し SessionREPL に書き直し
- :recycle: refactor(typecheck): scope 変数存储に VarInfo トラッキング可変性を導入
- :recycle: refactor(typecheck): プリミティブ値型と Dup シャローコピーセマンティクスを分離

---

## 🐛 バグ修正

- :bug: fix(repl): REPL 履歴のデフォルト設定を構成し、shell evaluate_code を修正
- :bug: fix(repl): コンプライアンスを登録し複数行入力を修正
- :bug: fix(repl): wrap_code の余分なセミコロンを削除して式値を保持

---

## ⚡ パフォーマンス最適化

- :zap: perf(types): 定数ジェネリクスの評価に再帰深度制限を追加

---

## 🔧 その他の変更

- :wrench: chore(build): rand, hashbrown, tempfile, ron, clap, owo-colors を bump
- :white_check_mark: test(typecheck): scope VarInfo 可変性テストを補完

### 参照テンプレート

リリースドキュメントは [`release.md`](release.md) テンプレート形式を参照して作成してください。

---

### 1. Commit Template の設定

```bash
# プロジェクトルートディレクトリで実行
git config commit.template .gitmessage.txt
```

### 2. Template ファイル

プロジェクトルートの `.gitmessage.txt` ファイル形式は次のとおりです：

```
# emojiコード type(scope): テーマ（中国語）
#
# ボディコンテンツ（任意）
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
# ✨ feat(db): バッチ削除 TODO 機能を追加
# 🐛 fix(provider): タイマーのバックグラウンド復元問題を修正
#
# リリース形式: 🔖 V1.0.0: リリースタイトル
```

---

## よくある質問

### Q: コミットタイプはどう選択すればよいですか？

- **feat**: ユーザーが見える機能の変更
- **fix**: ユーザーが報告した問題の修正
- **docs**: README、コメントなどのドキュメント
- **chore**: 依存関係の更新、設定ファイル
- **refactor**: 動作を変えないコード最適化

### Q: いつコミットを分割すべきですか？

- 各コミットは**1つのことだけ**を行う
- 関連する機能は一緒にコミットし、関連しないものは別々にコミット
- Atomic Commits の原則に従う

---

## 参考資料

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [Emoji 完全リスト](#完全な-emoji-参照)
- [release.md](release.md) - リリーステンプレート

---

> 💡 **ヒント**: コミットの原子性を保ち、説明をクリアにすることで、コードレビューと追跡がより効率的になります！