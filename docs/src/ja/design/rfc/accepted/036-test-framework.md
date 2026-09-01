---
title: 'RFC-036: std.test テストフレームワークと yaoxiang test コマンド'
status: '承認済み'
author: '晨煦'
created: '2026-07-26'
updated: '2026-08-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test テストフレームワークと yaoxiang test コマンド

## 概要

YaoXiang に標準テストフレームワーク `std.test` モジュールと `yaoxiang test`
CLI サブコマンドを導入する。テストファイルは通常の `.yx` ファイルであり、`std.assert.assert` + exit
code で合格/失敗を判定する。`std.test`
モジュールは純粋な YaoXiang で実装され、最初の dogfooding ライブラリとなる。`yaoxiang test`
は CLI ツールであり、コンパイラ機能ではない —
parser、IR、バイトコード、エグゼキュータのいずれの変更も伴わない。

## 動機

### なぜテストフレームワークが必要か？

現在、YaoXiang のテストカバレッジは Rust 側の `#[test]` と `tests/`
統合テストに依存している。これは次のような問題を意味する：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert /
   std.io）のユニットテストを YaoXiang で記述できない
2. `#117 標準ライブラリ各モジュールのユニットテストカバレッジ`
   が、利用可能なテストインフラがないためブロックされている
3. 言語機能の回帰テスト（例：RFC-032 spawn セマンティクスの変更）に自動化された手段がない

### 重要な制約

- **17 キーワード鉄則**：新しいキーワードや構文構造を導入しない
- **コンパイラ変更ゼロ**：parser、IR、バイトコード、エグゼキュータに触れない
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
│  実行層:    各ファイルに対して: yaoxiang run <file>            │
│              exit code をチェック → 逐次実行                  │
│              │                                               │
│  報告層:    PASS/FAIL → 集計                                  │
│              --json / --verbose / --fail-fast をサポート      │
│                                                              │
│  アサーション層: std.test (純粋な YaoXiang、セルフホスティング)│
│              基盤: std.assert.assert                          │
│              診断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### 中核となる原則

1. **テストフレームワークはコンパイラ機能ではなく、CLI ツールである** — `yaoxiang run`
   はすでに「テスト実行」を行える。`yaoxiang test`
   はすべてのファイルを実行してレポートを表示するだけのヘルパーである
2. **コンパイラ変更ゼロ** — `@test`
   アノテーションスキャン、バイトコードメタデータセグメント、エグゼキュータ特殊入口を導入しない
3. **セルフホスティング** — `std.test` モジュールは純粋な YaoXiang で実装され、基盤として
   `std.assert.assert` を呼び出す
4. **テストファイルは通常の `.yx` ファイル** — exit code で合格/失敗を判定する

## 詳細設計

### 1. CLI 設計

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      テストファイルまたはディレクトリを指定（デフォルト: yaoxiang.toml から読み込み、なければ tests/）

Options:
  --filter <NAME>     ファイル名に <NAME> を含むテストのみ実行
  --fail-fast         最初の失敗で停止
  --verbose, -v       各テストの詳細 stdout/stderr を表示
  --list              テストファイルを列挙するだけで実行しない
  --no-progress       プログレスバーを表示しない（CI 向け）
  --json              JSON 形式の結果を出力（CI 連携用）
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

`[tool.test]` の下に配置し、RFC-015 の `[tool.*]` サードパーティ拡張規約に準拠する：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
# 将来の拡張:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザーは設定なしですぐに使える
- 単一ファイルモード（`yaoxiang test foo.yx`）は直接実行され、設定は読み込まれない
- 将来的に独立したリポジトリに分割される可能性がある（`[tool.test]` の位置は不変）

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

- 4 つのアサーション関数、すべて `f"..."` で診断情報を作成
- `assert_eq` / `assert_ne` は**注釈なしのパラメータ**（`Any`）を使用 — 2026-08-02 実証：`==`/`!=`
  と f-string 補間は Any 上で正常に動作（Int/String ともに検証済み）、**ジェネリクスシステムに依存しない**。将来ジェネリクスが利用可能になれば注釈を追加可能
- `assert_true` / `assert_false` のパラメータ注釈は `Bool`；`assert_false` は `cond == false`
  で否定を表現する（`not` 単項構文は #251 で権威化進行中のため、安定後に移行可能）
- `std.test` はネイティブコードに依存せず、純粋な YaoXiang で実装されている

### 4. 標準ライブラリ読み込み機構（重要な設計）

**Phase 1：バイナリ埋め込み**

`std/test.yx`（および将来 YaoXiang で書かれるすべての標準ライブラリモジュール）はビルド時にバイナリに埋め込まれる：

```rust
// build.rs またはビルドスクリプト、自動的に生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // ソースコードテキスト
    // 将来さらに追加
];
```

モジュールシステム（RFC-029、2026-08-02 に完全実装）により接続点が提供される：Registry はネイティブモジュールとソースモジュールの両方を保持し、orchestrator が複数ファイルの編成を担当する。`use std.test`
の解決順序：

1. まず Rust ネイティブモジュールを検索（既存の仕組み、例えば `std.assert`）
2. ヒットしない場合、埋め込まれた `STD_YX_FILES` を検索 — ヒットすれば**仮想パス**（例えば
   `<std>/test.yx`）をシードモジュールとして orchestrator に注入し、通常のパイプライン（parse →
   typecheck → IR）を経由
3. ヒットしない場合、ファイルシステム探索（ユーザーモジュール）に進む

埋め込まれたソースモジュール内部の `use std.assert`
は resolver によって正常にネイティブレジストリに解決される — ネイティブとソースモジュールは Registry 内で共存し、種類をまたいだ依存関係が自然に成立する。埋め込まれたモジュールは**オンデマンドでコンパイル**される：import されたときのみパイプラインに入る。

利点：

- 単一ファイルモードでも `use std.test` が動作する
- 標準ライブラリのバージョンがバイナリと厳密に結合され、バージョンミスマッチが発生しない
- ユーザーに標準ライブラリパスの設定が不要

**将来：ファイルシステム標準ライブラリ**

YaoXiang のプロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式に変更される。詳細は RFC-014 の更新を参照。

### 5. 発見と実行

**前提条件（2026-08-02 レビュー決議）**：CLI `run` を orchestrator に接続する。現状の CLI `run`
は単一ファイルパイプライン（`run_file_with_diagnostics`）を通り、ユーザーモジュールのインポートを解析できない。一方
`yaoxiang test`
の子プロセスモデルは CLI 機能を継承し、テストファイルがプロジェクトモジュールをインポートするのは中心的なシナリオである。したがって Phase
1 ではまず CLI `Run` のソースブランチを
`run_project`（orchestrator、ディレクトリ再帰的探索）に委譲する；#247（use に沿ったオンデマンド探索）はその後、純粋なパフォーマンス最適化として重ね合わせる。import がない単一ファイルは orchestrator 経由でも動作が等価であり、バイトコードブランチは変わらない。

**発見段階**：

1. `[PATHS]` が指定されている場合、指定されたパスを直接使用
2. そうでなければ `yaoxiang.toml` の `[tool.test].patterns` を読み込む
3. 設定がない場合、デフォルト `tests/**/*.yx`
4. `--filter` でフィルタを適用（ファイル名を含む）

**実行段階**：

1. 各ファイルについて：`yaoxiang run --debug-info <file>` で子プロセスを起動（`--debug-info`
   はランタイムエラーにソースコード位置を付加する — 2026-08-02 実証でスタックトレースが
   `file:line:col` を出力）
2. exit code をチェック：0 が PASS、0 以外が FAIL
3. レポート用に stdout/stderr をキャプチャ
4. 逐次実行のみ（Phase 1）、将来 `--parallel` をサポート
5. `--fail-fast` の場合、最初の FAIL で即座に停止

### 6. テスト分離

テスト分離はプロセスレベルの境界によって自然に実現される：

- 各テストファイルは独立した子プロセスで実行される
- 各子プロセスは独立した Heap、Frame、NativeContext を持つ
- あるテストファイルのパニックが他のテストファイルに影響しない
- 追加の独立 Heap コンテキスト機構は不要

## 既存システムとの関係

| 項目                                                      | 関係                                                                                              |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                            | 変更なし、コンパイラ内部テストは Rust を継続使用                                                  |
| 既存の `.yx` 統合テスト（`tests/yaoxiang/`）              | `yaoxiang test` で発見・実行される                                                                |
| `std.assert.assert(cond)`                                 | 維持、`std.test` の基盤依存                                                                       |
| モジュールシステム（RFC-029）                             | 埋め込みソースモジュールは Registry/orchestrator 経由で接続；CLI `run` の orchestrator 接続が前提 |
| `#200` リファクタリング（`io.println` → `assert.assert`） | `yaoxiang test` と完全に同じ方向                                                                  |
| `@` アノテーション                                        | 使用しない、`@test` を導入しない                                                                  |

## 実装戦略

### Phase 1：コア機能

変更範囲：

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` のソースブランチを `run_project`
  に委譲（複数ファイル実行の前提）
- `src/main.rs` — 新しい `Test` サブコマンドを追加
- `src/std/test.yx` — 純粋な YaoXiang モジュールを追加
- `build.rs` — `std/*.yx` をバイナリに埋め込む
- orchestrator / Registry — 埋め込みソースから仮想パスで `.yx` モジュールを読み込みをサポート
- RFC-015 設定解析 — `[tool.test]` セクション
- 子プロセス実行（`--debug-info`）+ レポート

成果物：

- `yaoxiang test` が基本的に使用可能
- `std.test` の 4 つのアサーション関数
- デフォルト `tests/**/*.yx` の発見
- 逐次実行 + デフォルト出力形式

### Phase 2：完成

- `--filter` / `--fail-fast` / `--verbose` パラメータ
- `--json` 出力（CI 連携）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：高度化

- `--parallel` 並列実行（spawn 並行モデルの完成に依存）
- `[tool.test].exclude` 設定
- さらなるアサーション関数（Float 用の `assert_approx_eq` など）

## リスクと緩和

| リスク                                                 | 確率 | 緩和                                                                      |
| ------------------------------------------------------ | ---- | ------------------------------------------------------------------------- |
| `f"..."` の Any 上での補間失敗                         | なし | 2026-08-02 実証済み（Int/String ともに正常）                              |
| 子プロセス起動のオーバーヘッドがテスト速度に影響       | 中   | Phase 1 逐次実行で許容可能；Phase 3 並列で緩和                            |
| `yaoxiang.toml` 設定解析が現在の CLI にない            | 低   | 簡単な拡張で、コア機能に影響しない                                        |
| CLI run の orchestrator 接続による動作回帰             | 低   | import なし単一ファイルパスは等価；統合テストが orchestrator をカバー済み |
| `.yx` ソースファイルのバイナリ埋め込みによるサイズ増加 | 低   | `.yx` ソースファイルは非常に小さく、無視可能                              |

## 未解決問題

- [x] `std/test.yx` 内の `use std.assert`
      の参照が正しく解決できるか？——**解決済み（2026-08-02）**。モジュールシステム（RFC-029）実装後、ネイティブとソースモジュールは Registry 内で共存し、resolver が統一的に解決し、種類をまたいだ依存関係が自然に成立する
- [x] テスト出力における `f"..."` のジェネリック `to_string`
      が新しい型制約を導入するか？——**解決済み（2026-08-02）**。注釈なしのパラメータ（Any）上で
      `==`/`!=`
      と f-string 補間がともに動作することを実証（Int/String 検証済み）、新しい制約を導入しない
- [x] `?` ジェネリックパラメータの可否？——**解決済み（2026-08-02）**：`?`
      型構文は現時点で存在せず（静かに吸収される、別途 issue で追跡中）、Phase
      1 のアサーション関数は注釈なしパラメータを使用し、ジェネリクスシステムに依存しない

## 設計決定記録

| 決定                   | 決定内容                                                       | 日付       | 理由                                                                         |
| ---------------------- | -------------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------- |
| テストマーカー方式     | `@test` アノテーションを使用せず、テストファイルは通常の `.yx` | 2026-07-26 | コンパイラ変更ゼロ、子プロセスによる分離                                     |
| アサーション方式       | `std.test` モジュール純粋な YaoXiang 関数                      | 2026-07-26 | セルフホスティング、ネイティブコードなし                                     |
| テスト実行モデル       | 子プロセス `yaoxiang run <file>` + exit code                   | 2026-07-26 | プロセスレベル分離、コンパイラ変更ゼロ                                       |
| 標準ライブラリ読み込み | 現在バイナリ埋め込み、将来ファイルシステム                     | 2026-07-26 | バージョン結合、単一ファイル対応                                             |
| アサーション引数の型   | 注釈なしパラメータ（Any）、ジェネリクスシステムに依存しない    | 2026-08-02 | `?` 型構文が存在しない；Any で比較・補間可能と実証                           |
| 複数ファイル実行       | CLI `run` を `run_project`（orchestrator）に委譲を前提とする   | 2026-08-02 | 子プロセスモデルが CLI 機能を継承；#247 は純粋なパフォーマンス最適化に格下げ |
| レポートソース位置     | 子プロセスに `--debug-info` を付与                             | 2026-08-02 | 実証でスタックトレースが `file:line:col` を出力                              |

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
  — 標準ライブラリディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert アサーション機構](../review/030-assert-mechanism.md) — 基盤依存
- [Rust `#[test]` 機構](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計
