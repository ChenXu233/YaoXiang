---
title: 'RFC-036: std.test テストフレームワークと yaoxiang test コマンド'
status: '承認済み'
author: '晨煦'
created: '2026-07-26'
updated: '2026-09-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test テストフレームワークと yaoxiang test コマンド

## 概要

YaoXiang に標準テストフレームワーク `std.test` モジュールと `yaoxiang test`
CLI サブコマンドを導入する。テストファイルは通常の `.yx` ファイルであり、`std.assert.assert` + exit
code によって合格/不合格を判定する。`std.test`
モジュールは純粋な YaoXiang で実装され、最初の dogfooding ライブラリである。`yaoxiang test`
は CLI ツールであり、コンパイラ機能ではない——parser、IR、バイトコード、エグゼキュータのいずれにも変更を加えない。

## 動機

### なぜテストフレームワークが必要なのか？

現在の YaoXiang のテストカバレッジは Rust 側の `#[test]` と `tests/`
統合テストに依存している。これは次のような問題を生む：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert /
   std.io）の単体テストが YaoXiang で記述できない
2. `#117 標準ライブラリ各モジュールの単体テストカバレッジ`
   が、利用可能なテスト基盤がないためにブロックされている
3. 言語機能の回帰テスト（例：RFC-032 spawn セマンティクスの変更）に自動化された手段がない

### 重要な制約

- **17 キーワード鉄則**：新しいキーワードや構文構造を導入しない
- **コンパイラ変更ゼロ**：parser、IR、バイトコード、エグゼキュータには触れない
- **セルフホスティング優先**：テストライブラリは YaoXiang で記述し、最初の dogfooding ライブラリとする

## アーキテクチャ

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI 層:  yaoxiang test [--filter --fail-fast --json ...]    │
│              │                                               │
│  発見層:    yaoxiang.toml を読み込み → [tool.test] patterns   │
│              デフォルト: tests/**/*.yx                        │
│              │                                               │
│  実行層:    各ファイルごとに: yaoxiang run <file>             │
│              exit code を確認 → 逐次実行                      │
│              │                                               │
│  報告層:    PASS/FAIL → 集計                                  │
│              --json / --verbose / --fail-fast をサポート      │
│                                                              │
│  検証層:    std.test (純粋な YaoXiang、セルフホスティング)     │
│              基盤: std.assert.assert                          │
│              診断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### 基本原則

1. **テストフレームワークはコンパイラ機能ではなく、CLI ツールである** — `yaoxiang run`
   はすでに「テスト実行」が可能であり、`yaoxiang test`
   は単にすべてのファイルを実行してレポートを表示するだけ
2. **コンパイラ変更ゼロ** — `@test`
   アノテーションスキャン、バイトコードメタデータセグメント、エグゼキュータの特殊エントリポイントは導入しない
3. **セルフホスティング** — `std.test` モジュールは純粋な YaoXiang で実装され、基盤として
   `std.assert.assert` を呼び出す
4. **テストファイルは通常の `.yx` ファイル** — exit code で合格/不合格を判定する

## 詳細設計

### 1. CLI 設計

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      テストファイルまたはディレクトリを指定（デフォルト: yaoxiang.toml から読み込み、未設定なら tests/）

Options:
  --filter <NAME>     ファイル名に <NAME> を含むテストのみ実行
  --fail-fast         最初の失敗で停止
  --verbose, -v       各テストの詳細な stdout/stderr を表示
  --list              テストファイルを列挙するだけで実行しない
  --no-progress       プログレスバーを表示しない（CI 向け）
  --json              JSON 形式で結果を出力（CI 統合用）
```

#### 出力形式

**デフォルト出力**：

```
Running 5 tests from 3 files...

tests/math_test.yx ........................ PASS (0.002s)
tests/list_test.yx ........................ PASS (0.001s)
tests/string_test.yx ...................... FAIL (0.003s)
  `-- Expected "hello", got "world"
      at tests/string_test.yx:12:5

Results: 2 passed, 1 failed, 0 skipped (0.006s)
```

**JSON 出力**（`--json`）：

```json
{
  "summary": { "total": 3, "passed": 2, "failed": 1, "skipped": 0, "time_secs": 0.006 },
  "tests": [
    { "file": "tests/math_test.yx", "passed": true, "time_secs": 0.002 },
    {
      "file": "tests/string_test.yx",
      "passed": false,
      "time_secs": 0.003,
      "error": "Expected \"hello\", got \"world\"",
      "exit_code": 1
    }
  ]
}
```

### 2. yaoxiang.toml 設定

RFC-015 の `[tool.*]` サードパーティ拡張規約に従い、`[tool.test]` の下に配置する：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
# 将来の拡張:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザー設定不要で即座に利用可能
- 単一ファイルモード（`yaoxiang test foo.yx`）は設定を読まずに直接実行
- 将来的に独立リポジトリへ分割する可能性あり（`[tool.test]` の位置は不変）

### 3. std.test モジュール（純粋な YaoXiang）

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library
// First dogfooding library: YaoXiang's test library written in YaoXiang

use std.assert

assert_eq = (a, b) => {
    assert.assert(a == b, f"Expected {b}, got {a}")
}

assert_ne = (a, b) => {
    assert.assert(a != b, f"Expected not equal to {b}, got {a}")
}

assert_true = (cond: Bool) => {
    assert.assert(cond, f"Expected true, got {cond}")
}

assert_false = (cond: Bool) => {
    assert.assert(cond == false, f"Expected false, got {cond}")
}
```

- 4 つのアサーション関数、すべて `f"..."` で診断情報を生成
- `assert_eq` / `assert_ne` は**注釈なしのパラメータ**（`Any`）を使用——2026-08-02 に実証：Any 上で
  `==`/`!=`
  と f-string 補間が正常に動作（Int/String ともに検証済み）、**generics システムに依存しない**。generics 対応後に注釈を補完可能
- `assert_true` / `assert_false` のパラメータは `Bool` 注釈付き；`assert_false` は `cond == false`
  で否定を表現（`not` 単項構文は #251 で権威付け進行中、安定後に移行可能）
- `std.test` は native コードに一切依存せず、純粋な YaoXiang で実装

### 4. 標準ライブラリのロード機構（重要な設計）

**Phase 1：バイナリへの埋め込み**

`std/test.yx`（および将来的に YaoXiang で記述されるすべての標準ライブラリモジュール）はビルド時にバイナリへ埋め込まれる：

```rust
// build.rs またはビルドスクリプトで自動生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // ソースコードテキスト
    // 将来的にさらに追加
];
```

モジュールシステム（RFC-029、2026-08-02 に完全実装）により、Registry は native モジュールとソースモジュールの両方を保持し、orchestrator が複数ファイル編成を担当する。`use std.test`
の解決順序：

1. まず Rust の native モジュールを検索（既存メカニズム、例：`std.assert`）
2. ヒットしない場合、埋め込まれた `STD_YX_FILES`
   を検索——ヒットすれば**仮想パス**（例：`<std>/test.yx`）をシードモジュールとして orchestrator に注入し、通常のフロントエンドパイプライン（parse
   → typecheck → IR）を経由
3. ヒットしない場合、ファイルシステム探索（ユーザーモジュール）へ

埋め込まれたソースモジュール内部の `use std.assert` は resolver によって通常通り native
registry へ解決される——native とソースモジュールは Registry に共存し、種類をまたいだ依存は自然に成立する。埋め込みモジュールは**オンデマンドでコンパイル**：import されたときのみパイプラインへ入る。

利点：

- 単一ファイルモードでも `use std.test` が動作する
- 標準ライブラリのバージョンがバイナリと厳密に紐付けられ、版数の不一致が発生しない
- ユーザーが標準ライブラリのパスを設定する必要がない

**将来：ファイルシステム上の標準ライブラリ**

YaoXiang のプロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式へ移行する。詳細は RFC-014 の更新を参照。

### 5. 発見と実行

**前提条件（2026-08-02 審査決議）**：CLI `run` を orchestrator に接続する。現状 CLI `run`
は単一ファイルパイプライン（`run_file_with_diagnostics`）を通るため、ユーザーモジュールのインポートを解析できない。一方
`yaoxiang test`
のサブプロセスモデルは CLI の能力を継承し、テストファイルがプロジェクトモジュールをインポートすることは中核シナリオである。したがって Phase
1 ではまず CLI `Run` のソースブランチを
`run_project`（orchestrator、ディレクトリ再帰探索）に委譲する；#247（use に沿ったオンデマンド発見）はその後、純粋なパフォーマンス最適化として重ね合わせる。import のない単一ファイルは orchestrator でも動作が等価であり、バイトコードブランチは不変。

**発見段階**：

1. `[PATHS]` が指定されていれば、指定されたパスをそのまま使用
2. そうでなければ `yaoxiang.toml` の `[tool.test].patterns` を読み込む
3. 設定がなければ、デフォルト `tests/**/*.yx`
4. `--filter` でフィルタリング（ファイル名に含まれるもの）

**実行段階**：

1. 各ファイルについて：`yaoxiang run --debug-info <file>` でサブプロセスを起動（`--debug-info`
   によりランタイムエラーに位置情報を付与——2026-08-02 に実証、stack trace は `file:line:col`
   を出力）
2. exit code を確認：0 なら PASS、0 以外なら FAIL
3. stdout/stderr をキャプチャしてレポートに使用
4. 逐次実行のみ（Phase 1）、将来的に `--parallel` をサポート
5. `--fail-fast` 指定時、最初の FAIL で即座に停止

### 6. テスト分離

テスト分離はプロセスレベルの境界によって自然に実現される：

- 各テストファイルは独立したサブプロセスで実行される
- 各サブプロセスは独立した Heap、Frame、NativeContext を持つ
- 一つのテストファイルのパニックは他のテストファイルに影響しない
- 追加の独立 Heap コンテキスト機構は不要

## 既存システムとの関係

| 項目                                                      | 関係                                                                                              |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                            | 不変、コンパイラ内部テストは引き続き Rust を使用                                                  |
| 既存の `.yx` 統合テスト（`tests/yaoxiang/`）              | `yaoxiang test` で発見・実行される                                                                |
| `std.assert.assert(cond)`                                 | 保持、`std.test` の基盤依存先                                                                     |
| モジュールシステム（RFC-029）                             | 埋め込みソースモジュールは Registry/orchestrator 経由で接続；CLI `run` の orchestrator 接続が前提 |
| `#200` リファクタリング（`io.println` → `assert.assert`） | `yaoxiang test` と完全に同じ方向                                                                  |
| `@` アノテーション                                        | 使用しない、`@test` は導入しない                                                                  |

## 実装戦略

### Phase 1：中核機能

変更範囲：

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` のソースブランチを `run_project`
  に委譲（複数ファイル実行の前提）
- `src/main.rs` — 新規 `Test` サブコマンド追加
- `src/std/test.yx` — 新規純粋 YaoXiang モジュール追加
- `build.rs` — `std/*.yx` をバイナリへ埋め込み
- orchestrator / Registry — 埋め込みソースから仮想パスで `.yx` モジュールをロードする機能をサポート
- RFC-015 設定解析 — `[tool.test]` セクション
- サブプロセス実行（`--debug-info`）+ レポート

成果物：

- `yaoxiang test` が基本的に利用可能
- `std.test` の 4 つのアサーション関数
- デフォルトの `tests/**/*.yx` 発見
- 逐次実行 + デフォルト出力形式

### Phase 2：整備

- `--filter` / `--fail-fast` / `--verbose` オプション
- `--json` 出力（CI 統合）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：発展

- `--parallel` 並列実行（spawn 並行モデルの完成に依存）
- `[tool.test].exclude` 設定
- より多くのアサーション関数（例：Float 用 `assert_approx_eq`）

## リスクと緩和

| リスク                                             | 確率 | 緩和                                                                       |
| -------------------------------------------------- | ---- | -------------------------------------------------------------------------- |
| `f"..."` の Any 上での補間失敗                     | なし | 2026-08-02 に実証済み（Int/String ともに正常）                             |
| サブプロセス起動のオーバーヘッドがテスト速度に影響 | 中   | Phase 1 は逐次実行で許容範囲；Phase 3 で並列化により緩和                   |
| `yaoxiang.toml` 設定解析が現行 CLI にない          | 低   | 単純な拡張であり、中核機能に影響しない                                     |
| CLI `run` の orchestrator 接続による挙動回帰       | 低   | import のない単一ファイルパスは等価；orchestrator は統合テストでカバー済み |
| `.yx` ソースファイルの埋め込みによるバイナリ肥大化 | 低   | `.yx` ソースファイルは極小のため無視可能                                   |

## 未解決問題

- [x] `std/test.yx` 内の `use std.assert`
      の参照は正しく解決されるか？——**解決済み（2026-08-02）**。モジュールシステム（RFC-029）実装後、native とソースモジュールは Registry 内で共存し、resolver が統一的に解決し、種類をまたいだ依存は自然に成立する
- [x] テスト出力の `f"..."` における generics の `to_string`
      は新たな型制約を導入するか？——**解決済み（2026-08-02）**。実証により、注釈なしパラメータ（Any）上で
      `==`/`!=` と f-string 補間がともに動作（Int/String 検証済み）、新たな制約は導入されない
- [x] `?` generics パラメータの実現可能性？——**解決済み（2026-08-02）**：`?`
      型構文は現状存在せず（かつ黙って飲み込まれる、別 issue で追跡中）、Phase
      1 のアサーション関数は注釈なしパラメータを使用し、generics システムに依存しない

## 設計判断記録

| 判断                   | 決定                                                                                          | 日付       | 理由                                                                         |
| ---------------------- | --------------------------------------------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------- |
| テストマーカー方式     | `@test` アノテーションを使用せず、テストファイルは通常の `.yx`                                | 2026-07-26 | コンパイラ変更ゼロ、サブプロセスがそのまま分離境界                           |
| アサーション方式       | `std.test` モジュールは純粋な YaoXiang 関数                                                   | 2026-07-26 | セルフホスティング、native コード不要                                        |
| テスト実行モデル       | サブプロセス `yaoxiang run <file>` + exit code                                                | 2026-07-26 | プロセスレベル分離、コンパイラ変更ゼロ                                       |
| 標準ライブラリのロード | 現状はバイナリ埋め込み、将来的にファイルシステム                                              | 2026-07-26 | バージョン固定、単一ファイルでも動作                                         |
| アサーション引数の型   | 注釈なしパラメータ（Any）、generics システムに非依存                                          | 2026-08-02 | `?` 型構文は存在しない；Any で比較・補間が実証済み                           |
| 複数ファイル実行       | CLI `run` を `run_project`（orchestrator）に委譲することを前提とする                          | 2026-08-02 | サブプロセスモデルが CLI 能力を継承；#247 は純粋なパフォーマンス最適化へ降格 |
| レポートのソース位置   | サブプロセスに `--debug-info` を付与                                                          | 2026-08-02 | stack trace が `file:line:col` を出力することを実証済み                      |
| 負方向テストの階層化   | 値レベルの反転汎用 / コンパイル失敗 runner 構造化マーカー（内部専用）/ ハード失敗は Result 化 | 2026-09-02 | #319 で確定；暗黙の [test:error] 規約を置換                                  |
| ファイル内複数テスト   | 値化標準モデル：テスト関数は Result を返し、スイートが per-test 判定を収集                    | 2026-09-02 | catch なし、非エントリ呼び出し（エントリは内部シーンのみ）                   |
| Error コード           | Error に機械可読な `code` フィールドを追加                                                    | 2026-09-02 | エラーコードアサーションを支える；コンパイル時コードは runner 比較で処理     |

## 改訂：テストモデル確定（2026-09-02、#319）

メンテナとの議論により以下 4 点を確定した。本文中と本節が矛盾する場合、本節を優先する。

### R1. 負方向テストの 3 階層確定（暗黙の [test:error] 規約を置換）

負方向テスト（「失敗が期待される」）は失敗の発生層によって分割し、それぞれ帰属させる：

1. **値レベルの反転（汎用、ユーザー向け）**：被験操作が `Result`
   を返し、テストは通常のアサーションで期待される失敗を表現する（`assert(r.is_err())`、エラーコードアサーション）。Result 化の進展（#301、#316）に伴い、コーパス中の
   `[test:error]`
   ファイルは段階的にファイル内アサーションへ移行し、ファイルヘッダーマーカーは消滅する。std.test に先行して
   `assert_not` / `assert_err` 関数ファミリーを追加（`!assert`
   の単項形式は not 構文実装後に提供予定、現状 `!` 接頭辞構文は存在しない——§3 の `assert_false` が
   `cond == false` を使うのと同じ制約）。
2. **コンパイル失敗（言語設計者の内部使用のみ）**：コンパイルは全ファイル単位の all-or-nothing であり、ファイル内で「この行はコンパイルされないはず」を表現できない。ファイルレベルの特殊マーカー（`[test:error]`）を保持し、runner が読み取って逆方向判定する。本リポジトリのコーパス専用であり、**ユーザーテストフレームワークの一部ではない**。マーカーは構造化された期待コードへ昇格：runner はヘッダ
   `期待: コンパイルエラー EXXXX` を解析し、コンパイラの stderr が出力する `[EXXXX]`
   と実コードを比較し、コード不一致 =
   FAIL（trybuild の stderr スナップショット思想の軽量版、コンパイラ変更ゼロ）。
3. **ランタイムハード失敗（Result 化に統合、独立機構は設けない）**：失敗しうる操作は言語の方向性に従って
   `Result` を返し（#301、#316）、テストは第 1 層で統一的に表現する。

### R2. ファイル内複数テスト：値化標準モデル

一つのテストファイルは複数のテストを含みうる。標準モデルは**値化**：テスト関数は `Result`
を返し、アサーション失敗は abort プロセスではなく `Err` 値で表現される；スイートは各テストの
`Result` を順次収集し、per-test 判定を集約レポートし、いずれかが Ok 以外ならファイルの exit
code は 0 以外になる。`std.test`
アサーションファミリーはこれに応じて**値セマンティクス**版を提供（Err 診断情報を返し、abort しない）；`std.assert.assert`
のプロセスレベル abort セマンティクスはランタイムガード用に保持し、テストアサーションには使用しない。catch 境界は採用しない（17 キーワード鉄則）、runner による関数単位のエントリ呼び出しも採用しない（コンパイル失敗などの内部シーンに限定、R1.2 を参照）。

### R3. Error 値への code フィールド追加

エラーコードアサーションを支える：`Error` を `Struct { message }` から機械可読な `code`
を含むよう拡張（native
`error_new_with_code`、std がコード定数をエクスポート）し、`assert(err.code == "E3017")`
を成立させる。コンパイル時エラーのコード検証はこの経路を通らない——コンパイル失敗時はプロセスが実行されないため、R1.2 の runner 比較が担う（コンパイラの stderr に既に
`[EXXXX]` が含まれる）。

### R4. レポートと性能の訂正

- 出力例の "Running 5 tests from 3 files" は旧モデルの残滓：ファイルレベルモデルでは 1 ファイル =
  1 テストプロセス；R2 実装後、per-test 判定はスイート内の収集から得られ、runner がテスト関数をスキャンするのではない
- 性能の主項目は 1 ファイルあたりの全コンパイル（185 ファイル実測 11.3s）であり、サブプロセス起動ではない；Phase
  3 の `--parallel` はコンパイルコストを解決せず、テストループのキャッシュは #251 /
  #293 スライスを参照
- アサーション失敗の位置情報が埋め込みモジュールを貫通する（実測
  `at std.test.assert_eq (ip: 9)`、ユーザーは自分のファイルの行番号を取得できない）問題は本 RFC の対象外——スタックフレームの帰属は #289 の UX 問題 +
  RFC-034 のデバッグメタデータ（`get_frames`）の設計範囲

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
  — 標準ライブラリのディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert アサーション機構](../review/030-assert-mechanism.md) — 基盤依存
- [Rust `#[test]` 機構](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計
