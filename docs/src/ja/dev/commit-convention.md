# Commit 提交ガイドライン

本文書は YaoXiang プロジェクトの Git コミット規約を定義したもので、コミット履歴を明確で読みやすく、理解しやすいものに保つことを目的としています。

---

## 目次

- [コミット形式](#コミット形式)
- [コミットタイプ](#コミットタイプ)
- [Emoji 完全一覧](#emoji-完全一覧)
- [スコープ](#スコープ)
- [バージョン管理](#バージョン管理)
- [メッセージ規約](#メッセージ規約)
- [言語規約](#言語規約)
- [🔖 リリースコミット](#-リリースコミット)
- [使用 Commit Template](#使用-commit-template)
- [サンプル](#サンプル)
- [よくある質問](#よくある質問)

---

## コミット形式

**非常に重要！！！！！！絶対に忘れてはいけません！！！**
すべてのコミットメッセージは以下の形式に従ってください：

```
:emojiコード: type(scope): 主题（中文）

[可选的主体内容]

[可选的页脚]
```

> ⚠️ **重要**:
> **emoji コード**（例：`:sparkles:`）を使用する必要があります。直接 emoji 文字を入力してはいけません。
>
> **コミットメッセージは中国語の使用を推奨します**。チーム内の一貫したコミュニケーションを維持するためです。

### 構成要素

| 部分        | 説明                                | 必須 |
| ----------- | ----------------------------------- | ---- |
| emojiコード | コミットタイプを示す絵文字識別子    | ✅   |
| type        | コミットタイプ                      | ✅   |
| scope       | 影響範囲                            | ✅   |
| subject     | 簡潔な説明（中国語、50 文字以内）   | ✅   |
| body        | 詳細な説明（任意）                  | ❌   |
| footer      | 破壊的変更や Issue クローズ（任意） | ❌   |

---

## コミットタイプ

| emojiコード             | type     | 説明                                 |
| ----------------------- | -------- | ------------------------------------ |
| :sparkles:              | feat     | 新機能                               |
| :bug:                   | fix      | バグ修正                             |
| :memo:                  | docs     | ドキュメントのみの変更               |
| :lipstick:              | style    | コードフォーマット（機能に影響なし） |
| :recycle:               | refactor | リファクタリング                     |
| :zap:                   | perf     | パフォーマンス最適化                 |
| :white_check_mark:      | test     | テストの追加・修正                   |
| :wrench:                | chore    | ビルドツール・補助ツールの変更       |
| :building_construction: | build    | ビルドシステム変更                   |
| :rocket:                | ci       | CI 設定の変更                        |

---

## Emoji 完全一覧

以下は gitmoji プロジェクトと一致する完全な emoji リストです。コミット内容に応じて適切な emoji を選択できます：

| emoji | emoji コード                  | commit 説明                              |
| :---- | :---------------------------- | :--------------------------------------- |
| 🎨    | `:art:`                       | コード構造/フォーマットを改善            |
| ⚡️    | `:zap:` / `:racehorse:`       | パフォーマンス向上                       |
| 🔥    | `:fire:`                      | コードやファイルの削除                   |
| 🐛    | `:bug:`                       | バグ修正                                 |
| 🚑    | `:ambulance:`                 | 重要なパッチ                             |
| ✨    | `:sparkles:`                  | 新機能の導入                             |
| 📝    | `:memo:`                      | ドキュメントの記述                       |
| 🚀    | `:rocket:`                    | 機能のデプロイ                           |
| 💄    | `:lipstick:`                  | UI やスタイルファイルの更新              |
| 🎉    | `:tada:`                      | 初回コミット                             |
| ✅    | `:white_check_mark:`          | テストの追加                             |
| 🔒    | `:lock:`                      | セキュリティ問題の修正                   |
| 🍎    | `:apple:`                     | macOS 関連の修正                         |
| 🐧    | `:penguin:`                   | Linux 関連の修正                         |
| 🏁    | `:checkered_flag:`            | Windows 関連の修正                       |
| 🤖    | `:robot:`                     | Android 関連の修正                       |
| 🍏    | `:green_apple:`               | iOS 関連の修正                           |
| 🔖    | `:bookmark:`                  | リリース/バージョンタグ                  |
| 🚨    | `:rotating_light:`            | linter 警告の除去                        |
| 🚧    | `:construction:`              | 作業中（WIP）                            |
| 💚    | `:green_heart:`               | CI ビルド問題の修正                      |
| ⬇️    | `:arrow_down:`                | 依存関係のダウングレード                 |
| ⬆️    | `:arrow_up:`                  | 依存関係のアップグレード                 |
| 📌    | `:pushpin:`                   | 依存関係を特定バージョンに固定           |
| 👷    | `:construction_worker:`       | CI ビルドシステムの追加                  |
| 📈    | `:chart_with_upwards_trend:`  | 分析・追跡コードの追加                   |
| ♻️    | `:recycle:`                   | コードのリファクタリング                 |
| 🔨    | `:hammer:`                    | 大規模なリファクタリング                 |
| ➖    | `:heavy_minus_sign:`          | 依存関係の削除                           |
| 🐳    | `:whale:`                     | Docker 関連作業                          |
| ➕    | `:heavy_plus_sign:`           | 依存関係の追加                           |
| 🔧    | `:wrench:`                    | 設定ファイルの変更                       |
| 🌐    | `:globe_with_meridians:`      | 国際化とローカライズ                     |
| ✏️    | `:pencil2:`                   | typo の修正                              |
| 💩    | `:hankey:`                    | 改善が必要な糟糕代码                     |
| ⏪️    | `:rewind:`                    | 変更の取り消し                           |
| 🔀    | `:twisted_rightwards_arrows:` | マージ                                   |
| 📦    | `:package:`                   | コンパイル済みファイルやパッケージの更新 |
| 👽    | `:alien:`                     | 外部 API 変更に伴うコード更新            |
| 🚚    | `:truck:`                     | ファイルの移動・リネーム                 |
| 📄    | `:page_facing_up:`            | ライセンスの追加・更新                   |
| 💥    | `:boom:`                      | 破壊的変更の導入                         |
| 🍱    | `:bento:`                     | アセットの追加・更新                     |
| 👌    | `:ok_hand:`                   | コードレビューによる更新                 |
| ♿️    | `:wheelchair:`                | アクセシビリティの向上                   |
| 💡    | `:bulb:`                      | ソースコードのドキュメント化             |
| 🍻    | `:beers:`                     | 酔った勢いでコードを書く                 |
| 💬    | `:speech_balloon:`            | テキストや文言の更新                     |
| 🗃️    | `:card_file_box:`             | データベース関連の変更                   |
| 🔊    | `:loud_sound:`                | ログの追加                               |
| 🔇    | `:mute:`                      | ログの削除                               |
| 👥    | `:busts_in_silhouette:`       | 貢献者の追加                             |
| 🚸    | `:children_crossing:`         | ユーザー体験/ユーザビリティの改善        |
| 🏗️    | `:building_construction:`     | アーキテクチャ変更                       |
| 📱    | `:iphone:`                    | レスポンシブデザイン対応                 |
| 🤡    | `:clown_face:`                | 何かをからかう                           |
| 🥚    | `:egg:`                       | イースターエッグの追加                   |
| 🙈    | `:see_no_evil:`               | .gitignore ファイルの追加・更新          |
| 📸    | `:camera_flash:`              | スナップショットの追加・更新             |

---

## スコープ

スコープはプロジェクトの `src/`
ディレクトリ構造に基づき、**以下の定義済みスコープを使用する必要があります**：

### トップレベルモジュール

| スコープ    | 対応ディレクトリ | 説明                                         |
| ----------- | ---------------- | -------------------------------------------- |
| `frontend`  | `src/frontend/`  | フロントエンド：字句解析、構文解析、型検査   |
| `middle`    | `src/middle/`    | 中間層：IR、最適化、モノモーフィゼーション   |
| `backends`  | `src/backends/`  | バックエンド：インタプリタ、ランタイム、REPL |
| `std`       | `src/std/`       | 標準ライブラリ                               |
| `formatter` | `src/formatter/` | コードフォーマッタ                           |
| `lsp`       | `src/lsp/`       | 言語サーバープロトコル                       |
| `package`   | `src/package/`   | パッケージマネージャ                         |
| `util`      | `src/util/`      | ユーティリティ：診断、キャッシュ、i18n       |

### フロントエンドサブモジュール

| スコープ    | 対応ディレクトリ               | 説明           |
| ----------- | ------------------------------ | -------------- |
| `parser`    | `src/frontend/core/parser/`    | 構文パーサ     |
| `lexer`     | `src/frontend/core/lexer/`     | 字句解析器     |
| `typecheck` | `src/frontend/core/typecheck/` | 型検査         |
| `types`     | `src/frontend/core/types/`     | 型システム定義 |

### 中間層サブモジュール

| スコープ       | 対応ディレクトリ                  | 説明                       |
| -------------- | --------------------------------- | -------------------------- |
| `codegen`      | `src/middle/passes/codegen/`      | コード生成（バイトコード） |
| `monomorphize` | `src/middle/passes/monomorphize/` | モノモーフィゼーション     |
| `lifetime`     | `src/middle/passes/lifetime/`     | ライフタイム解析           |

### バックエンドサブモジュール

| スコープ  | 対応ディレクトリ            | 説明                      |
| --------- | --------------------------- | ------------------------- |
| `repl`    | `src/backends/dev/repl/`    | REPL インタラクティブ CLI |
| `shell`   | `src/backends/dev/shell.rs` | Shell コマンド処理        |
| `runtime` | `src/backends/runtime/`     | ランタイム実行エンジン    |

### ドキュメントスコープ

| スコープ | 説明                     |
| -------- | ------------------------ |
| `docs`   | 一般的なドキュメント更新 |
| `design` | 言語設計仕様（RFC）      |
| `plan`   | 実装計画ドキュメント     |

### その他のスコープ

| スコープ  | 説明                                             |
| --------- | ------------------------------------------------ |
| `build`   | ビルドシステム、Cargo 設定                       |
| `ci`      | CI/CD 設定（GitHub Actions）                     |
| `test`    | テスト関連                                       |
| `release` | リリース関連                                     |
| `meta`    | プロジェクトメタ設定（.claude, .gitignore など） |

---

## メッセージ規約

### バージョン管理

バージョン番号はプロジェクトルートディレクトリの `Cargo.toml` の `version`
フィールドで定義されています：

```toml
[package]
version = "0.7.2"
```

セマンティックバージョニング `MAJOR.MINOR.PATCH` を採用します：

| バージョンタイプ | 説明                                  | 例            |
| ---------------- | ------------------------------------- | ------------- |
| **major**        | メジャーアップデート、非互換 API 変更 | 0.7.2 → 1.0.0 |
| **minor**        | 新機能、後方互換あり                  | 0.7.2 → 0.8.0 |
| **patch**        | バグ修正、後方互換あり                | 0.7.2 → 0.7.3 |

> ⚠️ リリース時は **dev ブランチで `Cargo.toml`
> のバージョン番号を更新**し、PR を main にマージした後、CI が自動的に tag と Release を作成します。**手動で tag をプッシュしないでください**。そうしないと CI が release フローをスキップします。

---

## CI リリースフロー

リリースは GitHub Actions（`release.yml`）により自動実行されます。フローは以下の通りです：

```
1. dev ブランチで Cargo.toml の version フィールドを更新
2. cargo build で Cargo.lock を更新
3. リリース形式でコミット（後述の 🔖 リリースコミットを参照）
   - commit message には前回のリリース以降のすべての変更（PR の全内容）を含める必要があります
4. dev から main への PR を作成
5. PR を main にマージ
6. CI が自動検出：
   - Cargo.toml のバージョン番号を読み取り → "v{version}"
   - その tag が既に存在するかチェック
   - 存在しない → 完全な release フローをトリガー
   - 存在する → スキップ（重複リリースしない）
7. CI が自動実行：
   - 並列：クロスプラットフォームビルド（Linux/Windows/macOS）+ セキュリティ監査 + テスト
   - すべて成功後：tag 作成、アーティファクトパッケージ、GitHub Release の公開
```

### 重要なルール

| ルール                                          | 説明                                                                                         |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------- |
| **手動で tag をプッシュしない**                 | CI は tag の有無でリリースを判断するため、手動プッシュすると CI がスキップされます           |
| **バージョンは dev で bump**                    | リリースコミットは dev で完了し、PR で main にマージします                                   |
| **リリースコミットに完全な changelog を含める** | commit message には今回のリリースの全変更内容を含める必要があります（PR 説明の元になるため） |
| **main を dev にマージバックしない**            | PR マージ後、dev は自動同期されるため、逆マージは不要です                                    |

---

## メッセージ規約

### 言語規約

**コミットメッセージは中国語の使用を推奨します**。チーム内の一貫したコミュニケーションを維持するためです。

- Subject は中国語で簡潔明瞭に
- Body は中国語で詳細に説明可能
- 特別な技術用語は英語のままでも可

### Subject（タイトル）

- 中国語で簡潔明瞭に
- 50 文字以内
- 末尾に句点をつけない

### Body（本文）

- 変更の理由と方法を詳細に説明
- 1 行 72 文字以内
- 要点は `-` または `*` で列挙

### Footer（フッター）

- **破壊的変更**: `BREAKING CHANGE:` で始める
- **Issue クローズ**: `关闭 #123` または `修复 #456` を使用

---

## サンプル

### ✨ feat - 新機能

```
:sparkles: feat(parser): 添加闭包语法解析支持

实现闭包表达式解析：
- 支持 |args| body 简写语法
- 支持 move 语义捕获
- 添加闭包类型推断

关闭 #42
```

### 🐛 fix - バグ修正

```
:bug: fix(repl): 修复多行输入时补全器失效的问题

SessionREPL 在多行模式下未正确注册补全器，
导致 Tab 补全无法触发。

修复 #128
```

### 📝 docs - ドキュメント更新

```
:memo: docs(design): 更新所有权模型与类型系统规范

同步 RFC-009 和 RFC-011 的最新设计变更。
```

### ♻️ refactor - リファクタリング

```
:recycle: refactor(typecheck): 分离原语值类型与 Dup 浅拷贝语义

将 MonoType 中的值类型和拷贝语义解耦，
消除 match 分支中的特殊情况。
```

### ⚡️ perf - パフォーマンス最適化

```
:zap: perf(types): 优化 const generic 求值性能

为递归求值添加深度限制（默认 128），
避免恶意构造的类型表达式导致栈溢出。
```

### ✅ test - テスト

```
:white_check_mark: test(typecheck): 补充 scope VarInfo 可变性测试

覆盖场景：
- 不可变绑定的只读访问
- mut 绑定的可变性追踪
- 跨作用域的可变性传播
```

### 🔧 chore - その他

```
:wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap

升级 6 个生产依赖至最新稳定版本。
```

### 🚀 ci - CI 設定

```
:rocket: ci: 修复 nightly 构建 Rust 版本过低的问题

将 RUST_TOOLCHAIN 从 1.91.0 更新至 1.96.0，
匹配 Cargo.toml 中的 rust-version 要求。
```

### 💄 style - フォーマット調整

```
:lipstick: style(frontend): 应用 cargo fmt 格式化

统一函数签名的换行风格。
```

---

---

## 🔖 リリースコミット

今回のコミットが**リリース**である場合、以下の規約に従う必要があります：

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

## 📦 新規ファイル

- `<ファイルパス>` - <ファイル説明>

---



### リリース要件

1. **メッセージヘッダー**: `:bookmark:` + `V<バージョン番号>` 形式を使用する必要がある
2. **バージョン番号**: セマンティックバージョニング規約に従う
3. **内容の完全性**: 前回のリリース以降の**すべての commit** 内容を含む必要がある
4. **タイプ別分類**: `feat`, `fix`, `refactor`, `chore` などのタイプ別に整理

### リリースサンプル

```

:bookmark: V0.7.2: REPL 重写与类型系统改进

## 📦 バージョン情報

**发布日期:** 2026-06-01

**バージョン番号:** 0.7.1 → 0.7.2

---

## ✨ 新機能

- :sparkles: feat(typecheck): 实现泛型类型参数自动推断
- :sparkles: feat(typecheck): 添加 MonoType::Generic 结构化泛型表示
- feat: 接入 CLI REPL 命令到 SessionREPL

---

## ♻️ リファクタリング最適化

- :recycle: refactor(backends): 移除 tui_repl 模块，重写为 SessionREPL
- :recycle: refactor(typecheck): scope 变量存储引入 VarInfo 追踪可变性
- :recycle: refactor(typecheck): 分离原语值类型与 Dup 浅拷贝语义

---

## 🐛 バグ修正

- :bug: fix(repl): 配置默认 REPL 历史记录，修复 shell evaluate_code
- :bug: fix(repl): 注册补全器并修复多行输入
- :bug: fix(repl): 移除 wrap_code 中多余的分号以保留表达式值

---

## ⚡ パフォーマンス最適化

- :zap: perf(types): 为 const generic 求值添加递归深度限制

---

## 🔧 その他の変更

- :wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap, owo-colors
- :white_check_mark: test(typecheck): 补充 scope VarInfo 可变性テスト

### 参考テンプレート

リリースドキュメントは [`release.md`](release.md) テンプレート形式に従って作成してください。

---

### 1. Commit Template の設定

```bash
# プロジェクトルートディレクトリで実行
git config commit.template .gitmessage.txt
```

### 2. Template ファイル

プロジェクトルートディレクトリの `.gitmessage.txt` ファイル形式は以下の通りです：

```
# emoji代码 type(scope): 主题（中文）
#
# 主体内容（可选）
#
# 页脚（可选）
#
# Types: ✨feat, 🐛fix, 📝docs, 💄style, ♻️refactor, ⚡️perf, ✅test, 🔧chore, 🚀ci, 🔖release
# Scopes: frontend, parser, lexer, typecheck, types, middle, codegen,
#         monomorphize, lifetime, backends, repl, shell, runtime,
#         std, formatter, lsp, package, util, docs, design, plan,
#         build, ci, test, release, meta
#
# 示例:
# ✨ feat(db): 添加批量删除待办功能
# 🐛 fix(provider): 修复计时器后台恢复问题
#
# 发版格式: 🔖 V1.0.0: 发版标题
```

---

## よくある質問

### Q: コミットタイプはどのように選べばよいですか？

- **feat**: ユーザーが認識できる機能変更
- **fix**: ユーザーから報告された問題の修正
- **docs**: README、コメントなどのドキュメント
- **chore**: 依存関係の更新、設定ファイル
- **refactor**: 挙動を変えないコード最適化

### Q: いつコミットを分割すべきですか？

- 1 つのコミットで **1 つのこと** だけを行う
- 関連する機能は一緒に、そうでないものは分ける
- Atomic Commits の原則に従う

---

## 参考資料

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [Emoji 完全一覧](#emoji-完全一覧)
- [release.md](release.md) - リリーステンプレート

---

> 💡
> **ヒント**: コミットをアトミックに保ち、説明を明確にすることで、コードレビューやトレースバックがより効率的になります！
