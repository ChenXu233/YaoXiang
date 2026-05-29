# Commit 提交ガイドライン

本文書は YaoXiang プロジェクトの Git 提交規範を定義したもので、提交履歴を明確で読みやすく、理解しやすいものにすることを目的としています。

---

## 目次

- [提交フォーマット](#提交フォーマット)
- [提交タイプ](#提交タイプ)
- [完全な Emoji 参照](#完全な-emoji-参照)
- [スコープ](#スコープ)
- [バージョンマネジメント](#バージョンマネジメント)
- [メッセージ規範](#メッセージ規範)
- [言語規範](#言語規範)
- [🔖 リリース提交](#-リリース提交)
- [例](#例)
- [Commit Template の使用](#commit-template-の使用)
- [よくある質問](#よくある質問)

---

## 提交フォーマット

**非常に重要です！！！！忘れないでください！！！**
すべての提交メッセージは次のフォーマットに従います：

```
:emojiコード: type(scope): テーマ（日本語）

[任意のボディコンテンツ]

[任意のフッター]
```

> ⚠️ **重要**: 直接 emoji 文字を入力するのではなく、**emoji コード**（例：`:sparkles:`）を使用する必要があります。
>
> **日本語の提交メッセージを使用することを推奨します**。チーム内のコミュニケーションの一貫性を保ちます。

### 構成要素

| 部位 | 説明 | 必須 |
|------|------|------|
| emojiコード | 提交タイプを識別する絵文字 | ✅ |
| type | 提交タイプ | ✅ |
| scope | 影響範囲 | ✅ |
| subject | 簡潔な説明（日本語、50文字以内） | ✅ |
| body | 詳細説明（任意） | ❌ |
| footer | 破壊的変更またはイシューのクローズ（任意） | ❌ |

---

## 提交タイプ

| emojiコード | type | 説明 |
|-----------|------|------|
| :sparkles: | feat | 新機能 |
| :bug: | fix | バグ修正 |
| :memo: | docs | ドキュメント変更のみ |
| :lipstick: | style | コードフォーマット（機能に影響なし） |
| :recycle: | refactor | リファクタリング |
| :zap: | perf | パフォーマンス最適化 |
| :white_check_mark: | test | テストの追加または変更 |
| :wrench: | chore | ビルドツール、補助ツールの変更 |
| :building_construction: | build | ビルドシステム変更 |
| :rocket: | ci | CI 設定変更 |

---

## 完全な Emoji 参照

以下は gitmoji プロジェクトと一致する完全な emoji リストで、提交内容に応じて適切な emoji を選択できます：

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
| 🍎 | `:apple:` | macOS での問題の修正 |
| 🐧 | `:penguin:` | Linux での問題の修正 |
| 🏁 | `:checkered_flag:` | Windows での問題の修正 |
| 🤖 | `:robot:` | Android での問題の修正 |
| 🍏 | `:green_apple:` | iOS での問題の解決 |
| 🔖 | `:bookmark:` | リリース/バージョンタグ |
| 🚨 | `:rotating_light:` | linter 警告の削除 |
| 🚧 | `:construction:` | 作業進行中 |
| 💚 | `:green_heart:` | CI ビルドの問題修正 |
| ⬇️ | `:arrow_down:` | 依存関係の下位移行 |
| ⬆️ | `:arrow_up:` | 依存関係の上位移行 |
| 📌 | `:pushpin:` | 依存関係を特定バージョンに固定 |
| 👷 | `:construction_worker:` | CI ビルドシステムの追加 |
| 📈 | `:chart_with_upwards_trend:` | 分析または追跡コードの追加 |
| ♻️ | `:recycle:` | コードのリファクタリング |
| 🔨 | `:hammer:` | 大規模なリファクタリング |
| ➖ | `:heavy_minus_sign:` | 依存関係を1つ削除 |
| 🐳 | `:whale:` | Docker 関連作業 |
| ➕ | `:heavy_plus_sign:` | 依存関係を追加 |
| 🔧 | `:wrench:` | 設定ファイルの変更 |
| 🌐 | `:globe_with_meridians:` | 国際化とローカライズ |
| ✏️ | `:pencil2:` | typo の修正 |
| 💩 | `:hankey:` | 改善が必要な拙劣なコード |
| ⏪️ | `:rewind:` | 変更の復元 |
| 🔀 | `:twisted_rightwards_arrows:` | ブランチのマージ |
| 📦 | `:package:` | コンパイルされたファイルまたはパッケージの更新 |
| 👽 | `:alien:` | 外部 API の変更に伴うコード更新 |
| 🚚 | `:truck:` | ファイルの移動または名前変更 |
| 📄 | `:page_facing_up:` | ライセンスの追加または更新 |
| 💥 | `:boom:` | 破壊的変更の導入 |
| 🍱 | `:bento:` | アセットの追加または更新 |
| 👌 | `:ok_hand:` | コードレビューに伴う変更の更新 |
| ♿️ | `:wheelchair:` | アクセシビリティ向上 |
| 💡 | `:bulb:` | ソースコードのドキュメント化 |
| 🍻 | `:beers:` | はっぴいきんのコード作成 |
| 💬 | `:speech_balloon:` | テキストと文言の更新 |
| 🗃️ | `:card_file_box:` | データベース関連の変更の実行 |
| 🔊 | `:loud_sound:` | ログの追加 |
| 🔇 | `:mute:` | ログの削除 |
| 👥 | `:busts_in_silhouette:` | 貢献者の追加 |
| 🚸 | `:children_crossing:` | ユーザー体験/ユーザビリティの改善 |
| 🏗️ | `:building_construction:` | アーキテクチャ変更の実行 |
| 📱 | `:iphone:` | レスポンシブデザインに取り組む |
| 🤡 | `:clown_face:` | 物事のmock |
| 🥚 | `:egg:` | イースターエッグの追加 |
| 🙈 | `:see_no_evil:` | .gitignore ファイルの追加または更新 |
| 📸 | `:camera_flash:` | スナップショットの追加または更新 |

---

## スコープ

プロジェクト構造に基づき、以下のスコープを使用することを推奨します：

### コードスコープ

| スコープ | 説明 |
|--------|------|
| `frontend` | フロントエンドモジュール：字句解析、構文解析、型チェック |
| `parser` | 構文解析器 |
| `lexer` | 字句解析器 |
| `typecheck` | 型チェック |
| `middle` | ミドル層：IR、オプティマイザ |
| `codegen` | コード生成器 |
| `monomorphize` | 単態化処理 |
| `lifetime` | ライフタイム解析 |
| `vm` | 仮想マシン：命令実行、スタックフレーム、オペコード |
| `executor` | 実行器 |
| `frames` | スタックフレーム管理 |
| `instructions` | 命令セット |
| `runtime` | ランタイム：メモリ管理、スケジューラ |
| `memory` | メモリ管理 |
| `scheduler` | タスクスケジューリング |
| `std` | 標準ライブラリ |
| `concurrent` | 並行ライブラリ |
| `io` | IO ライブラリ |
| `net` | ネットワークライブラリ |
| `util` | ユーティリティライブラリ：診断、キャッシュ、Span |
| `cache` | キャッシュ管理 |
| `diagnostic` | 診断情報 |

### ドキュメントスコープ

| スコープ | 説明 |
|--------|------|
| `docs` | 汎用ドキュメント更新 |
| `architecture` | アーキテクチャ設計ドキュメント |
| `design` | 言語設計仕様 |
| `plan` | 実装計画ドキュメント |
| `guides` | ガイドドキュメント |
| `tutorial` | チュートリアルドキュメント |
| `examples` | サンプルコード |

### その他のスコープ

| スコープ | 説明 |
|--------|------|
| `build` | ビルドシステム、依存関係管理 |
| `ci` | CI/CD 設定 |
| `test` | テスト関連 |
| `chore` | 雑多なタスク |
| `release` | リリース関連 |
| `meta` | プロジェクトメタ設定（例：.claude, cargo 設定）|

---

## メッセージ規範

### バージョンマネジメント

**すべての提交前に必ずバージョンを bump してください**：

| バージョンタイプ | 更新場所 | 説明 |
|----------|----------|------|
| **major** | `pubspec.yaml` (version) + `release_v*.md` | 後方互換性のない API 変更を含む大きな更新 |
| **minor** | `pubspec.yaml` (version) | 新機能、後方互換性あり |
| **patch** | `pubspec.yaml` (version) | バグ修正、後方互換性あり |

### バージョン番号フォーマット

セマンティックバージョニング `MAJOR.MINOR.PATCH` を採用：

```
# メジャーバージョン (breaking changes)
1.0.0 -> 2.0.0

# マイナーバージョン (new features)
1.0.0 -> 1.1.0

# パッチバージョン (bug fixes)
1.0.0 -> 1.0.1
```

### 提交フロー

```bash
# 1. コード変更後、まずバージョンを bump
# semantic_release ツールを使用してバージョンを自動管理
npx semantic-release

# または手動で更新
# pubspec.yaml の version フィールドを編集

# 2. コードを提交（バージョン変更は次の release時に自動生成）
git add .
git commit -m ":tada: Release v1.0.0"
git push
```

> 💡 バージョンの bump と Changelog 生成は CI により自動実行されます。提交時はコード変更にバージョン更新が含まれていることを確認してください。

---

## メッセージ規範

### 言語規範

**日本語の提交メッセージを使用することを推奨します**。チーム内のコミュニケーションの一貫性を保ちます。

- Subject は日本語で、簡潔かつ明確
- Body は日本語で詳しく説明可能
- 特殊な技術用語がある場合は、英語をそのまま使用可能

### Subject（テーマ）

- 日本語で、簡潔かつ明確
- 50文字を超えない
- 末尾に句点を付けない

### Body（ボディ）

- 変更理由と方法を詳しく説明
- 各行は72文字以内
- 要点を列出するには - または * を使用

### Footer（フッター）

- **破壊的変更**: `BREAKING CHANGE:` で始まる
- **イシューのクローズ**: ` закрыт #123` または `修復 #456` を使用

---

## 例

### ✨ feat - 新機能

```
:sparkles: feat(db): 批量削除功能的追加

バッチ削除功能的追加：
- TodoRepository に batchDelete メソッドを追加
- 削除確認ダイアログを追加
- 複数選択操作対応の UI 更新

 закрыт #42
```

### 🐛 fix - バグ修正

```
:bug: fix(provider): ポモドーロタイマーがバックグラウンドで回復できない問題の修正

アプリがバックグラウンドから回復した時、ポモドーロタイマーが続きません。
initState に状態回復ロジックを追加しました。

修復 #128
```

### 📝 docs - ドキュメント更新

```
:memo: docs: README の新機能説明を更新

以下の章を追加：
- フォーカスモード
- データ統計
- BGM
```

### ♻️ refactor - リファクタリング

```
:recycle: refactor(ui): 共通のガラス風コンテナコンポーネントを抽出

再利用可能な GlassContainer コンポーネントを作成、
複数の画面間のコード重複を削減。
```

### ⚡️ perf - パフォーマンス最適化

```
:zap: perf(db): 完了済みTodoクエリのパフォーマンスを最適化

completed_at フィールドにインデックスを追加、
メモリ内でフィルタリングする代わりに WHERE 句を使用して完了済みTodoをフィルタリング。

最適化前：45ms
最適化後：12ms
```

### ✅ test - テスト

```
:white_check_mark: test(provider): TodoProvider ユニットテストを追加

カバーするシナリオ：
- Todo の追加
- 完了状態の切り替え
- Todo の削除
```

### 🔧 chore - 雑多

```
:wrench: chore: Flutter バージョンを 3.19.0 に更新

最小 Flutter バージョンの要件を引き上げ、互換性のある依存関係を更新。
```

### 🚀 ci - CI 設定

```
:rocket: ci: GitHub Actions テストワークフローを追加

以下のステップを含むワークフローを作成：
- ユニットテスト
- 統合テスト
- コードカバレッジ
```

### 💄 style - フォーマット調整

```
:lipstick: style(todo_item): dart fix を使用してコードをフォーマット

自動フォーマット修正を適用、
コードスタイルの一貫性を保つ。
```

---

---

## 🔖 リリース提交

本次の提交が**リリース（Release）**の場合、以下の規範に従う必要があります：

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

1. **メッセージヘッダー**: `:bookmark:` + `V<バージョン番号>` フォーマットを必ず使用
2. **バージョン番号**: セマンティックバージョニング規範に従う
3. **コンテンツの完全性**: 前回リリース以降の変更点を**すべて**含める必要がある
4. **タイプ別分類**: `feat`, `fix`, `refactor`, `chore` などのタイプ別に整理
5. **提交記録**: 関連するすべての提交の hash と説明を含む

### リリース例

```
:bookmark: V0.8.0: タスク統計とデータ分析功能の追加

## 📦 バージョン情報

**发布日期:** 2025-01-15

**バージョン番号:** 0.7.0 → 0.8.0

---

## ✨ 新機能

### 統計機能
- :sparkles: feat(statistics): フォーカス時間統計ページの追加
- :sparkles: feat(statistics): 日別/週別データ可視化グラフの追加

### Todo 強化
- :sparkles: feat(todo): タスク優先度フィルタリング機能の追加

---

## ♻️ リファクタリング最適化

- :recycle: refactor(db): データベースクエリパフォーマンスの最適化
- :recycle: refactor(provider): 状態管理ロジックのリファクタリング

---

## 🐛 バグ修正

- :bug: fix(todo): タスクリストのスワイプ遅延問題の修正

---

## 🔧 その他の変更

- :wrench: chore: 依存関係バージョンを最新版に更新
- :memo: docs: README インストール説明の更新

---

## 📦 新規追加ファイル

- `lib/screens/statistics/statistics.dart` - 統計ページ
- `lib/widgets/chart/data_chart.dart` - チャートコンポーネント

---

## 📝 提交記録

| 提交 | 説明 |
|:---:|------|
| `abc1234` | :bookmark: V0.8.0 |
| `def5678` | :sparkles: feat(statistics): フォーカス時間統計の追加 |
| `ghi9012` | :sparkles: feat(todo): 優先度フィルタリングの追加 |
| `jkl3456` | :recycle: refactor(db): クエリ最適化 |
| `mno7890` | :bug: fix(todo): スワイプ遅延の修正 |
```

### 提交記録の取得方法

```bash
# 前回リリース以降のすべての提交を表示
git log --oneline <前回リリースcommit>..HEAD

# または最近の N 件の提交を表示
git log --oneline -20
```

### 参照テンプレート

リリースドキュメントは [`release.md`](release.md) テンプレートフォーマットを参照して作成してください。

---

### 1. Commit Template の設定

```bash
# プロジェクトルートディレクトリで実行
git config commit.template .gitmessage.txt
```

### 2. Template ファイル

プロジェクトルートディレクトリの `.gitmessage.txt` ファイルの形式は次の通りです：

```
# emojiコード type(scope): テーマ（日本語）
#
# ボディコンテンツ（任意）
#
# フッター（任意）
#
# Types: ✨feat, 🐛fix, 📝docs, 💄style, ♻️refactor, ⚡️perf, ✅test, 🔧chore, 🚀ci, 🔖release
# Scopes: core, db, ui, screen, widget, provider, repo, i18n, router, dep
#
# 例:
# ✨ feat(db): 批量削除功能的追加
# 🐛 fix(provider): タイマー回復問題の修正
#
# リリースフォーマット: 🔖 V1.0.0: リリースタイトル
```

---

## よくある質問

### Q: 提交タイプはどう選択すればよいですか？

- **feat**: ユーザーが目に見える機能変更
- **fix**: ユーザーから報告された問題の修正
- **docs**: README、コメントなどのドキュメント
- **chore**: 依存関係の更新、設定ファイル
- **refactor**: 動作を変えないコード最適化

### Q: いつ提交を分割すべきですか？

- 各提交は**1つのことだけ**を行う
- 関連する機能は一緒に提交し、関係ないものは別々に提交
- Atomic Commits の原則に従う

---

## 参考資料

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [emoji.md](emoji.md) - Emoji 完全リスト
- [release.md](release.md) - リリーステンプレート

---

> 💡 **ヒント**: 提交の原子性を保ち、描述を明確にすることで、コードレビューと振り返りがより効率的になります！