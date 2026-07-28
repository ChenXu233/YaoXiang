---
title: 'RFC-036: std.test テストフレームワークと yaoxiang test コマンド'
status: 'ドラフト'
author: '晨煦'
created: '2026-07-26'
updated: '2026-07-26'
issue: '#94, #95, #221'
---

# RFC-036: std.test テストフレームワークと yaoxiang test コマンド

## 概要

YaoXiang に標準テストフレームワーク `std.test` モジュールと `yaoxiang test` CLI サブコマンドを導入する。テストファイルは通常の `.yx` ファイルであり、`std.assert.assert` + exit code で合格/不合格を判定する。`std.test` モジュールは純粋な YaoXiang で実装されており最初のパ dogfooding ライブラリである。`yaoxiang test` は CLI ツールであり、コンパイラ機能ではない——parser、IR、バイトコード、エクゼキュータへの変更は一切ない。

## 動機

### なぜテストフレームワークが必要か？

現在の YaoXiang のテストカバレッジは Rust 側の `#[test]` と `tests/` 統合テストに依存している。这意味着：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert / std.io）のユニットテストは YaoXiang で記述できない
2. `#117 標準ライブラリの各モジュールユニットテストカバレッジ` がブロックされている——利用可能なテストインフラがないため
3. 言語機能の回帰テスト（RFC-032 spawn セマンティクス変更など）に自動化手段がない

### 重要な制約

- **17キーワード鉄則**：新しいキーワードや構文構造を導入しない
- **コンパイラ変更ゼロ**：parser、IR、バイトコード、エクゼキュータに触れない
- **セルフホスティング優先**：テストライブラリは YaoXiang で記述最初のパ dogfooding ライブラリ

## アーキテクチャ

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI層:    yaoxiang test [--filter --fail-fast --json ...]    │
│              │                                               │
│  発見層:    yaoxiang.toml を読み込む → [tool.test] patterns   │
│              デフォルト: tests/**/*.yx                        │
│              │                                               │
│  実行層:    各ファイルに対して: yaoxiang run <file>            │
│              exit code をチェック → 串行実行                  │
│              │                                               │
│  レポート層: PASS/FAIL → 集計                                  │
│              --json / --verbose / --fail-fast をサポート      │
│                                                              │
│  アサーション層: std.test (純粋 YaoXiang、セルフホスティング) │
│              下層: std.assert.assert                          │
│              診断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### コア原則

1. **テストフレームワークはコンパイラ機能ではなく、CLIツールである** — `yaoxiang run` はすでに「テストを実行」でき、`yaoxiang test` は単にすべてのファイルを実行し結果レポートを表示するだけである
2. **コンパイラ変更ゼロ** — `@test` アノテーションスキャン、バイトコードメタデータセクション、エクゼキュータ特殊エントリを導入しない
3. **セルフホスティング** — `std.test` モジュールは純粋な YaoXiang で実装されており、下層で `std.assert.assert` を呼び出す
4. **テストファイルは通常の `.yx` ファイルである** — exit code で合格/不合格を判定する

## 詳細設計

### 1. CLI 設計

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      テストファイルまたはディレクトリを指定（デフォルト: yaoxiang.toml から読み込む、または tests/）

Options:
  --filter <NAME>     ファイル名が <NAME> を含むテストのみを実行
  --fail-fast         最初の失敗で停止
  --verbose, -v       各テストの詳細な stdout/stderr を表示
  --list              テストファイルを一覧表示のみ（実行しない）
  --no-progress       進捗バーを表示しない（CI シナリオ）
  --json              JSON 形式で結果を出力（CI 統合用）
```

#### 出力フォーマット

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

`[tool.test]` の下に配置し、RFC-015 の `[tool.*]` サードパーティ拡張規則に従う：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
# 将来拡張可能:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザーはゼロ設定で箱から出してすぐに使用可能
- 単一ファイルモード（`yaoxiang test foo.yx`）は直接実行し、設定を読み込まない
- 将来は独立したリポジトリに分割する可能性（`[tool.test]` の位置は変更なし）

### 3. std.test モジュール（純粋 YaoXiang）

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library
// First dogfooding library: YaoXiang's test library written in YaoXiang

use std.assert

assert_eq: (a: ?, b: ?) -> Void = (a, b) => {
    assert.assert(a == b, f"Expected {b}, got {a}")
}

assert_ne: (a: ?, b: ?) -> Void = (a, b) => {
    assert.assert(a != b, f"Expected not equal to {b}, got {a}")
}

assert_true: (cond: Bool) -> Void = (cond) => {
    assert.assert(cond, f"Expected true, got {cond}")
}

assert_false: (cond: Bool) -> Void = (cond) => {
    assert.assert(!cond, f"Expected false, got {cond}")
}
```

- 4つのアサーション関数、すべて `f"..."` で診断情報を提供
- `assert_eq` / `assert_ne` の `?` 型パラメータは 型システム に依存
- `std.test` は native コードに依存せず、純粋な YaoXiang 実装

### 4. 標準ライブラリ読み込みメカニズム（重要な設計）

**Phase 1：バイナリへの埋め込み**

`std/test.yx`（および将来のすべての YaoXiang で書かれた標準ライブラリモジュール）はビルド時にバイナリに埋め込まれる：

```rust
// build.rs またはビルドスクリプト、自动生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // ソースコードテキスト
    // 将来更多
];
```

モジュールローダーが `use std.test` を解決する際：

1. まず Rust native モジュールを検索（既存のメカニズム、例：`std.assert`）
2. 見つからなければ、埋め込まれた `STD_YX_FILES` を検索し、`std/test.yx` のソースコードを見つける
3. そのソースコードをコンパイルしてモジュールシステムに登録する

優位性：

- 単一ファイルモードでも `use std.test` が動作する
- 標準ライブラリのバージョンがバイナリに厳密にバインドされ、バージョン不一致がない
- ユーザーが標準ライブラりのパスを設定する必要がない

**将来：ファイルシステム標準ライブラリ**

YaoXiang のプロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式に変更される。RFC-014 の更新を参照。

### 5. 発見と実行

**発見段階**：

1. `[PATHS]` が指定されていれば、指定されたパスを使用
2. それ以外は `yaoxiang.toml` の `[tool.test].patterns` を読み込む
3. 設定がなければ、デフォルト `tests/**/*.yx` を使用
4. `--filter` を適用（ファイル名に含まれているかでフィルタリング）

**実行段階**：

1. 各ファイルに対して：`yaoxiang run <file>` でサブプロセスを起動
2. exit code をチェック：0 なら PASS、0 以外なら FAIL
3. stdout/stderr をキャプチャしてレポートに使用
4. Phase 1 は串行実行のみ、将来 `--parallel` をサポート
5. `--fail-fast` の場合、最初の FAIL で即座に停止

### 6. テスト隔離

テスト隔離はプロセスの境界を越えて自然に実装される：

- 各テストファイルは独立したサブプロセスで実行される
- 各サブプロセスは独立した Heap、Frame、NativeContext を持つ
- あるテストファイルの panic は他のテストファイルに影響しない
- 追加の独立 Heap コンテキストメカニズムは不要

## 既存システムとの関係

| 項目                                          | 関係                              |
| --------------------------------------------- | --------------------------------- |
| Rust `#[test]`                                | 変更なし、コンパイラの内部テストは引き続き Rust を使用 |
| 既存の `.yx` 統合テスト（`tests/yaoxiang/`）  | `yaoxiang test` によって発見され実行される |
| `std.assert.assert(cond)`                     | 維持、`std.test` の下層がこれに依存する       |
| `#200` リファクタリング（`io.println` → `assert.assert`） | `yaoxiang test` と完全に同一の方向 |
| `@` アノテーション                                      | 使用しない、`@test` を導入しない            |

## 実装戦略

### Phase 1：コア機能

変更範囲：

- `src/main.rs` — 新規 `Test` サブコマンド追加
- `src/std/test.yx` — 新規純粋 YaoXiang モジュール
- `build.rs` — `std/*.yx` をバイナリに埋め込む
- モジュールローダー — 埋め込みソースから `.yx` モジュールを読み込みサポート
- RFC-015 設定解析 — `[tool.test]` セクション
- サブプロセス実行 + レポート

成果物：

- `yaoxiang test` の基本動作
- `std.test` の4つのアサーション関数
- デフォルト `tests/**/*.yx` 発見
- 串行実行 + デフォルト出力フォーマット

### Phase 2：改善

- `--filter` / `--fail-fast` / `--verbose` 引数
- `--json` 出力（CI 統合）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：上級

- `--parallel` 並行実行（spawn 並行モデルの改善に依存）
- `[tool.test].exclude` 設定
- 追加のアサーション関数（Float 用の `assert_approx_eq` など）

## リスクと緩和策

| リスク                                    | 確率 | 緩和                                        |
| --------------------------------------- | ---- | ------------------------------------------- |
| `f"..."` がジェネリック型での補間失敗           | 低   | `std.assert.assert` で基本型が動作することが既に検証済み |
| サブプロセス起動オーバーヘッドがテスト速度に影響              | 中   | Phase 1 の串行実行は許容可能；Phase 3 の並列で緩和  |
| `yaoxiang.toml` 設定解析が現在の CLI にない | 低   | シンプルな拡張、コア機能に影響しない                    |
| ジェネリック `?` が `std.test` で使用不可         | 低   | `Any` 型への降級または型特殊화로対応可能               |
| `.yx` ソースファイルをバイナリに埋め込むことでサイズ増加       | 低   | `.yx` ソースファイルは非常に小さく、無視可能                    |

## 開放問題

- [ ] `std/test.yx` 内の `use std.assert` の参照がモジュールローダーで正しく解決できるか？埋め込みソースモジュール間の依存関係を検証する必要がある
- [ ] テスト出力の `f"..."` におけるジェネリック `to_string` が新しい型制約を引き起こすか？検証が必要

## 設計意思決定記録

| 意思決定         | 決定                                      | 日付       | 理由                       |
| ------------ | ----------------------------------------- | ---------- | -------------------------- |
| テストマーク方式 | `@test` アノテーションを使用せず、テストファイルは通常の `.yx` | 2026-07-26 | コンパイラ変更ゼロ、サブプロセスで隔離 |
| アサーション方式     | `std.test` モジュールは純粋 YaoXiang 関数           | 2026-07-26 | セルフホスティング、native コードなし       |
| テスト実行モデル | サブプロセス `yaoxiang run <file>` + exit code  | 2026-07-26 | プロセスレベル隔離、コンパイラ変更ゼロ   |
| 標準ライブラリ読み込み   | 現時点ではバイナリに埋め込み、将来はファイルシステム              | 2026-07-26 | バージョンバインディング、単一ファイル使用可能       |
| ジェネリックアサーション     | `?` 型パラメータに依存                         | 2026-07-26 | 特殊化を導入せず、 型システムを信頼   |

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md) — 標準ライブラリディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert アサーションメカニズム](../review/030-assert-mechanism.md) — 下層依存
- [Rust `#[test]` メカニズム](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参照設計
