# コミットガイドライン

本文書は YaoXiang プロジェクトの Git コミット規約を定義し、コミット履歴を明確で可読性が高く、理解しやすいものにするためのものです。

---

## 目次

- [コミットフォーマット](#コミットフォーマット)
- [コミットタイプ](#コミットタイプ)
- [完全 emoji リファレンス](#完全-emoji-リファレンス)
- [スコープ](#スコープ)
- [バージョン管理](#バージョン管理)
- [メッセージ規約](#メッセージ規約)
- [言語規約](#言語規約)
- [🔖 リリースコミット](#-リリースコミット)
- [例](#例)
- [Commit Template の使用](#commit-template-の使用)
- [よくある質問](#よくある質問)

---

## コミットフォーマット

**非常に重要！！！！！！忘れてはいけません！！！**
すべてのコミットメッセージは以下のフォーマットに従ってください：

```
:emoji代码: type(scope): 主题（中文）

[可选的主体内容]

[可选的页脚]
```

> ⚠️ **重要**:
> **emoji 文字**を直接入力するのではなく、**emoji コード**（例：`:sparkles:`）を使用する必要があります。
>
> **中文コミットメッセージの使用を推奨**します。チーム内のコミュニケーションの一貫性を保つためです。

### 構成要素

| 部分        | 説明                                          | 必須 |
| ----------- | --------------------------------------------- | ---- |
| emojiコード | コミットタイプを示す絵文字識別子              | ✅   |
| type        | コミットタイプ                                | ✅   |
| scope       | 影響範囲                                      | ✅   |
| subject     | 簡潔な説明（中文、50 文字以内）               | ✅   |
| body        | 詳細説明（オプション）                        | ❌   |
| footer      | 破壊的変更または Issue クローズ（オプション） | ❌   |

---

## コミットタイプ

| emoji コード            | type     | 説明                                 |
| ----------------------- | -------- | ------------------------------------ |
| :sparkles:              | feat     | 新機能                               |
| :bug:                   | fix      | バグ修正                             |
| :memo:                  | docs     | ドキュメントのみの変更               |
| :lipstick:              | style    | コードフォーマット（機能に影響なし） |
| :recycle:               | refactor | コードリファクタリング               |
| :zap:                   | perf     | パフォーマンス最適化                 |
| :white_check_mark:      | test     | テストの追加または修正               |
| :wrench:                | chore    | ビルドツール、補助ツールの変更       |
| :building_construction: | build    | ビルドシステム変更                   |
| :rocket:                | ci       | CI 設定変更                          |

---

## 完全 emoji リファレンス

以下は gitmoji プロジェクトと一致する完全な emoji リストです。コミット内容に応じて適切な emoji を選択できます：

| emoji | emoji コード                  | commit 説明                                |
| :---- | :---------------------------- | :----------------------------------------- |
| 🎨    | `:art:`                       | コード構造/コードフォーマットの改善        |
| ⚡️    | `:zap:` / `:racehorse:`       | パフォーマンスの向上                       |
| 🔥    | `:fire:`                      | コードやファイルの削除                     |
| 🐛    | `:bug:`                       | バグの修正                                 |
| 🚑    | `:ambulance:`                 | 重要なパッチ                               |
| ✨    | `:sparkles:`                  | 新機能の導入                               |
| 📝    | `:memo:`                      | ドキュメントの作成                         |
| 🚀    | `:rocket:`                    | 機能のデプロイ                             |
| 💄    | `:lipstick:`                  | UI とスタイルファイルの更新                |
| 🎉    | `:tada:`                      | 初めてのコミット                           |
| ✅    | `:white_check_mark:`          | テストの追加                               |
| 🔒    | `:lock:`                      | セキュリティ問題の修正                     |
| 🍎    | `:apple:`                     | macOS 環境での修正                         |
| 🐧    | `:penguin:`                   | Linux 環境での修正                         |
| 🏁    | `:checkered_flag:`            | Windows 環境での修正                       |
| 🤖    | `:robot:`                     | Android での特定の問題の修正               |
| 🍏    | `:green_apple:`               | iOS での特定の問題の解決                   |
| 🔖    | `:bookmark:`                  | リリース/バージョンタグ                    |
| 🚨    | `:rotating_light:`            | linter 警告の除去                          |
| 🚧    | `:construction:`              | 作業進行中                                 |
| 💚    | `:green_heart:`               | CI ビルド問題の修正                        |
| ⬇️    | `:arrow_down:`                | 依存関係のダウングレード                   |
| ⬆️    | `:arrow_up:`                  | 依存関係のアップグレード                   |
| 📌    | `:pushpin:`                   | 依存関係を特定のバージョンに固定           |
| 👷    | `:construction_worker:`       | CI ビルドシステムの追加                    |
| 📈    | `:chart_with_upwards_trend:`  | 分析や追跡コードの追加                     |
| ♻️    | `:recycle:`                   | コードリファクタリング                     |
| 🔨    | `:hammer:`                    | 大規模リファクタリング                     |
| ➖    | `:heavy_minus_sign:`          | 依存関係の削減                             |
| 🐳    | `:whale:`                     | Docker 関連作業                            |
| ➕    | `:heavy_plus_sign:`           | 依存関係の追加                             |
| 🔧    | `:wrench:`                    | 設定ファイルの変更                         |
| 🌐    | `:globe_with_meridians:`      | 国際化とローカリゼーション                 |
| ✏️    | `:pencil2:`                   | タイポ修正                                 |
| 💩    | `:hankey:`                    | 改善が必要な悪いコードの作成               |
| ⏪️    | `:rewind:`                    | 変更の復元                                 |
| 🔀    | `:twisted_rightwards_arrows:` | ブランチのマージ                           |
| 📦    | `:package:`                   | コンパイルされたファイルやパッケージの更新 |
| 👽    | `:alien:`                     | 外部 API の変更によるコード更新            |
| 🚚    | `:truck:`                     | ファイルの移動または名前変更               |
| 📄    | `:page_facing_up:`            | ライセンスの追加または更新                 |
| 💥    | `:boom:`                      | 破壊的変更の導入                           |
| 🍱    | `:bento:`                     | アセットの追加または更新                   |
| 👌    | `:ok_hand:`                   | コードレビューによるコード更新             |
| ♿️    | `:wheelchair:`                | アクセシビリティの向上                     |
| 💡    | `:bulb:`                      | ソースコードのドキュメント化               |
| 🍻    | `:beers:`                     | 酔ったコード書き                           |
| 💬    | `:speech_balloon:`            | テキストと文字の更新                       |
| 🗃️    | `:card_file_box:`             | データベース関連の変更                     |
| 🔊    | `:loud_sound:`                | ログの追加                                 |
| 🔇    | `:mute:`                      | ログの削除                                 |
| 👥    | `:busts_in_silhouette:`       | 貢献者の追加                               |
| 🚸    | `:children_crossing:`         | ユーザー体験/ユーザビリティの改善          |
| 🏗️    | `:building_construction:`     | アーキテクチャの変更                       |
| 📱    | `:iphone:`                    | レスポンシブデザインへの取り組み           |
| 🤡    | `:clown_face:`                | 物事を嘲笑する                             |
| 🥚    | `:egg:`                       | イースターエッグの追加                     |
| 🙈    | `:see_no_evil:`               | .gitignore ファイルの追加または更新        |
| 📸    | `:camera_flash:`              | スナップショットの追加または更新           |

---

## スコープ

スコープはプロジェクトの `src/`
ディレクトリ構造に基づき、**以下の定義済みスコープを使用する必要があります**：

### 最上位モジュール

| スコープ    | 対応するディレクトリ | 説明                                             |
| ----------- | -------------------- | ------------------------------------------------ |
| `frontend`  | `src/frontend/`      | フロントエンド：字句解析、構文解析、型検査       |
| `middle`    | `src/middle/`        | ミドル：IR、最適化、単相化                       |
| `backends`  | `src/backends/`      | バックエンド：インタプリタ、ランタイム、REPL     |
| `std`       | `src/std/`           | 標準ライブラリ                                   |
| `formatter` | `src/formatter/`     | コードフォーマッタ                               |
| `lsp`       | `src/lsp/`           | 言語サーバープロトコル                           |
| `package`   | `src/package/`       | パッケージマネージャ                             |
| `util`      | `src/util/`          | ユーティリティライブラリ：診断、キャッシュ、i18n |

### フロントエンドサブモジュール

| スコープ    | 対応するディレクトリ           | 説明           |
| ----------- | ------------------------------ | -------------- |
| `parser`    | `src/frontend/core/parser/`    | 構文解析器     |
| `lexer`     | `src/frontend/core/lexer/`     | 字句解析器     |
| `typecheck` | `src/frontend/core/typecheck/` | 型検査         |
| `types`     | `src/frontend/core/types/`     | 型システム定義 |

### 中間層サブモジュール

| スコープ       | 対応するディレクトリ              | 説明                       |
| -------------- | --------------------------------- | -------------------------- |
| `codegen`      | `src/middle/passes/codegen/`      | コード生成（バイトコード） |
| `monomorphize` | `src/middle/passes/monomorphize/` | 単相化処理                 |
| `lifetime`     | `src/middle/passes/lifetime/`     | ライフタイム分析           |

### バックエンドサブモジュール

| スコープ  | 対応するディレクトリ        | 説明                                |
| --------- | --------------------------- | ----------------------------------- |
| `repl`    | `src/backends/dev/repl/`    | REPL インタラクティブコマンドライン |
| `shell`   | `src/backends/dev/shell.rs` | Shell コマンド処理                  |
| `runtime` | `src/backends/runtime/`     | ランタイム実行エンジン              |

### ドキュメントスコープ

| スコープ | 説明                 |
| -------- | -------------------- |
| `docs`   | 汎用ドキュメント更新 |
| `design` | 言語設計仕様 (RFC)   |
| `plan`   | 実装計画ドキュメント |

### その他のスコープ

| スコープ  | 説明                                            |
| --------- | ----------------------------------------------- |
| `build`   | ビルドシステム、Cargo 設定                      |
| `ci`      | CI/CD 設定 (GitHub Actions)                     |
| `test`    | テスト関連                                      |
| `release` | リリース関連                                    |
| `meta`    | プロジェクトメタ設定 (.claude, .gitignore など) |

---

## メッセージ規約

### バージョン管理

バージョン番号はプロジェクトルートディレクトリの `Cargo.toml` の `version`
フィールドで定義されています：

```toml
[package]
version = "0.7.2"
```

セマンティックバージョニング `MAJOR.MINOR.PATCH` を採用しています：

| バージョンタイプ | 説明                          | 例            |
| ---------------- | ----------------------------- | ------------- |
| **major**        | 重大な更新、非互換の API 変更 | 0.7.2 → 1.0.0 |
| **minor**        | 新機能、後方互換              | 0.7.2 → 0.8.0 |
| **patch**        | バグ修正、後方互換            | 0.7.2 → 0.7.3 |

> ⚠️ リリース時は **dev ブランチで `Cargo.toml`
> のバージョン番号を更新**し、PR を介して main にマージした後、CI が自動的に tag と Release を作成します。**手動で tag をプッシュしないでください**。さもないと CI が release フローをスキップします。

---

## CI リリースフロー

リリースは GitHub Actions (`release.yml`) によって自動的に完了し、フローは以下の通りです：

```
1. 在 dev 分支上更新 Cargo.toml 的 version 字段
2. cargo build 更新 Cargo.lock
3. 按发版格式 commit（见下方 🔖 发版提交）
   - commit message 必须包含自上次发版以来的所有变更（即 PR 的完整内容）
4. 从 dev 创建 PR 到 main
5. 合并 PR 到 main
6. CI 自动检测：
   - 读取 Cargo.toml 版本号 → "v{version}"
   - 检查该 tag 是否已存在
   - 不存在 → 触发完整 release 流程
   - 已存在 → 跳过（不会重复发布）
7. CI 自动执行：
   - 并行：跨平台构建 (Linux/Windows/macOS) + 安全审计 + 测试
   - 全部通过后：创建 tag、打包产物、发布 GitHub Release
```

### 重要なルール

| ルール                                          | 説明                                                                                                             |
| ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| **手動で tag をプッシュしない**                 | CI は tag の存在有無によって公開するかどうかを決定します。手動で tag をプッシュすると CI がスキップされます      |
| **バージョンは dev で bump**                    | リリースコミットは dev で完了し、PR を介して main にマージされます                                               |
| **リリースコミットに完全な changelog を含める** | コミットメッセージには今回のリリースのすべての変更内容を含める必要があります。これは PR の説明のソースであるため |
| **main を dev にマージしない**                  | PR マージ後 dev は自動的に同期されるため、逆マージは不要です                                                     |

---

## メッセージ規約

### 言語規約

**中文コミットメッセージの使用を推奨**します。チーム内のコミュニケーションの一貫性を保つためです。

- 件名は中文を使用し、簡潔明瞭に
- ボディは中文で詳細説明が可能
- 特別な技術用語がある場合は、英文を保持可能

### サブジェクト（件名）

- 中文を使用し、簡潔明瞭
- 長さは 50 文字以内
- 末尾に句点を付けない

### ボディ（本文）

- 変更の理由と方法を詳細に説明
- 各行は 72 文字以内
- `-` または `*` を使用して要点をリストアップ

### フッター

- **破壊的変更**: `BREAKING CHANGE:` で始める
- **Issue をクローズ**: `关闭 #123` または `修复 #456` を使用

---

## 例

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

今回のコミットが**リリース（Release）**の場合、以下の規約に従う必要があります：

### リリースコミットフォーマット

```
:bookmark: V<版本号>: <发版标题>

## 📦 版本信息

**发布日期:** YYYY-MM-DD

**版本号:** <旧版本> → <新版本>

---

## ✨ 新功能

### <功能模块>
- :sparkles: feat(<scope>): <功能描述>

---

## ♻️ 重构优化

- :recycle: refactor(<scope>): <重构描述>

---

## 🐛 Bug 修复

- :bug: fix(<scope>): <修复描述>

---

## 🔧 其他变更

- :wrench: chore: <变更描述>

---

## 📦 新增文件

- `<文件路径>` - <文件说明>

---



### リリース要件

1. **メッセージヘッダー**: `:bookmark:` + `V<バージョン番号>` フォーマットを使用する必要がある
2. **バージョン番号**: セマンティックバージョニング規約に従う
3. **コンテンツの完全性**: 前回のリリース以降の**すべてのコミット**内容紹介を含める必要がある
4. **タイプ別に分類**: `feat`, `fix`, `refactor`, `chore` などのタイプ別に整理

### リリース例

```

:bookmark: V0.7.2: REPL 重写与类型系统改进

## 📦 版本信息

**发布日期:** 2026-06-01

**版本号:** 0.7.1 → 0.7.2

---

## ✨ 新功能

- :sparkles: feat(typecheck): 实现泛型类型参数自动推断
- :sparkles: feat(typecheck): 添加 MonoType::Generic 结构化泛型表示
- feat: 接入 CLI REPL 命令到 SessionREPL

---

## ♻️ 重构优化

- :recycle: refactor(backends): 移除 tui_repl 模块，重写为 SessionREPL
- :recycle: refactor(typecheck): scope 变量存储引入 VarInfo 追踪可变性
- :recycle: refactor(typecheck): 分离原语值类型与 Dup 浅拷贝语义

---

## 🐛 Bug 修复

- :bug: fix(repl): 配置默认 REPL 历史记录，修复 shell evaluate_code
- :bug: fix(repl): 注册补全器并修复多行输入
- :bug: fix(repl): 移除 wrap_code 中多余的分号以保留表达式值

---

## ⚡ 性能优化

- :zap: perf(types): 为 const generic 求值添加递归深度限制

---

## 🔧 其他变更

- :wrench: chore(build): bump rand, hashbrown, tempfile, ron, clap, owo-colors
- :white_check_mark: test(typecheck): 补充 scope VarInfo 可变性测试

### 参考テンプレート

リリースドキュメントは [`release.md`](release.md)
テンプレートのフォーマットに従って作成してください。

---

### 1. Commit Template の設定

```bash
# 在项目根目录执行
git config commit.template .gitmessage.txt
```

### 2. Template ファイル

プロジェクトルートディレクトリの `.gitmessage.txt` ファイルフォーマットは以下の通りです：

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

### Q: コミットタイプの選び方は？

- **feat**: ユーザーから見える機能の変更
- **fix**: ユーザーが報告した問題の修正
- **docs**: README、コメントなどのドキュメント
- **chore**: 依存関係の更新、設定ファイル
- **refactor**: 動作を変更しないコード最適化

### Q: いつコミットを分割すべきか？

- 各コミットは一つのことだけを行う
- 関連機能は一緒にコミットし、無関係なものは分ける
- Atomic Commits の原則に従う

---

## 参考資料

- [Conventional Commits](https://www.conventionalcommits.org/)
- [gitmoji](https://gitmoji.carloscuesta.me/)
- [emoji.md](emoji.md) - Emoji 完全リスト
- [release.md](release.md) - リリーステンプレート

---

> 💡
> **ヒント**: コミットのアトミック性と明確な説明を維持し、コードレビューと遡及をより効率的にしましょう！
