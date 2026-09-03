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
CLI サブコマンドを導入する。テストファイルは通常の `.yx` ファイルであり、サブプロセスの exit
code で全体の合格/不合格を判定する。ファイル内部では複数のテスト関数をサポートする——アサーション失敗は
`Err` 値で表現され（値意味論）、スイートが per-test 判定を収集する（§7）。`std.test`
モジュールは純粋な YaoXiang で実装され、初の dogfooding ライブラリとなる。`yaoxiang test`
は CLI ツールであり、コンパイラの機能ではない——parser、IR、バイトコード、エグゼキュータのいずれにも変更を加えない。

## 動機

### なぜテストフレームワークが必要か？

現在の YaoXiang のテストカバレッジは Rust 側の `#[test]` と `tests/`
統合テストに依存している。これは以下のことを意味する：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert /
   std.io）のユニットテストを YaoXiang で記述できない
2. `#117 標準ライブラリ各モジュールのユニットテストカバレッジ`
   がブロックされている。テストインフラが利用できないためである
3. 言語機能の回帰テスト（例：RFC-032 spawn 意味論変更）に自動化された手段がない

### 重要な制約

- **17 キーワード鉄則**：新しいキーワードや構文構造を導入しない
- **コンパイラ変更ゼロ**：parser、IR、バイトコード、エグゼキュータには触れない
- **セルフホスティング優先**：テストライブラリは YaoXiang で記述する、初の dogfooding ライブラリ

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
│  実行層:    各ファイルに対して: yaoxiang run <file>           │
│              exit code を確認 → 逐次実行                      │
│              │                                               │
│  レポート層: PASS/FAIL → 集計                                 │
│              --json / --verbose / --fail-fast をサポート      │
│                                                              │
│  アサーション層: std.test（純粋な YaoXiang、セルフホスティング）│
│              基盤: std.assert.assert                          │
│              診断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### 中核原則

1. **テストフレームワークはコンパイラの機能ではなく、CLI ツールである** — `yaoxiang run`
   はすでに「テストを実行する」ことができる。`yaoxiang test`
   は単にすべてのファイルを実行してレポートを表示するヘルパーである
2. **コンパイラ変更ゼロ** — `@test`
   アノテーションスキャン、バイトコードメタデータセグメント、エグゼキュータ特殊入口を導入しない
3. **セルフホスティング** — `std.test` モジュールは純粋な YaoXiang で実装され、基盤機能は
   `std.assert` / `std.result` から得られる
4. **テストファイルは通常の `.yx` ファイルである** — ファイルはサブプロセスとして実行され、exit
   code で全体の合格/不合格を判定する
5. **アサーション失敗は値であり、プロセスイベントではない** — テスト関数は `Result`
   を返し、アサーション失敗は `Err`
   で表現される。スイートが per-test 判定を収集する（§7）；プロセスレベルの abort はランタイムガードにのみ属し、テストアサーションには使用しない

## 詳細設計

### 1. CLI 設計

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      テストファイルまたはディレクトリを指定（デフォルト: yaoxiang.toml から読み取り、なければ tests/）

Options:
  --filter <NAME>     ファイル名に <NAME> を含むテストのみ実行
  --fail-fast         最初の失敗で停止
  --verbose, -v       各テストの詳細な stdout/stderr を表示
  --list              テストファイルの一覧のみ表示し、実行しない
  --no-progress       進捗出力を表示しない（ヘッダーと PASS 行）；FAIL 明細と集計は保持（CI シナリオ）
  --json              JSON 形式で結果を出力（CI 統合用）
```

#### 出力フォーマット

**デフォルト出力**（per-test 判定はファイル内スイート収集から取得、§7 参照）：

```
Running 3 test files...

tests/math_test.yx ........................ PASS (0.002s)
tests/list_test.yx ........................ FAIL (0.003s)
  `-- [FAIL] push_grows_len: Expected 3, got 2
  `-- [ ok ] pop_returns_last
Results: 2 files passed, 1 file failed, 0 skipped (0.006s)
```

**JSON 出力**（`--json`）：

```json
{
  "summary": { "total": 3, "passed": 2, "failed": 1, "skipped": 0, "time_secs": 0.006 },
  "files": [
    { "file": "tests/math_test.yx", "passed": true, "time_secs": 0.002 },
    {
      "file": "tests/list_test.yx",
      "passed": false,
      "time_secs": 0.003,
      "exit_code": 1,
      "stderr": "error [E1024]: one is not two",
      "tests": [
        { "name": "push_grows_len", "passed": false, "error": "Expected 3, got 2" },
        { "name": "pop_returns_last", "passed": true }
      ]
    }
  ]
}
```

- 失敗ファイルは追加で `exit_code` と `stderr`
  を携带する（ANSI 除去後のサブプロセス診断、CI フォレンジック用）； `--verbose` と `--json`
  の組み合わせ時、すべてのファイルが `stdout` / `stderr` を携带する
- `--no-progress`
  は進捗出力（ヘッダーと PASS 行）のみを抑制する——FAIL 明細と集計は常に出力され、失敗を静かにしない；`--list`
  は各行に 1 つのテストファイルパスを出力し、実行しない
- ファイル内 per-test `tests`
  配列は §7 スイート収集から取得され、値化モデルの実装とともに有効化される（#319）

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

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザーはゼロコンフィグで即座に使える
- 単一ファイルモード（`yaoxiang test foo.yx`）は設定を無視して直接実行する
- 将来的に別リポジトリに分割される可能性がある（`[tool.test]` の位置は不变）

### 3. std.test モジュール（純粋な YaoXiang）

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library
// First dogfooding library: YaoXiang's test library written in YaoXiang

use std.result

assert_eq = (a, b) => {
    if a == b { result.ok(()) } else { result.err(f"Expected {b}, got {a}") }
}

assert_ne = (a, b) => {
    if a != b { result.ok(()) } else { result.err(f"Expected not equal to {b}, got {a}") }
}

assert_true = (cond: Bool) => {
    if cond { result.ok(()) } else { result.err(f"Expected true, got {cond}") }
}

assert_false = (cond: Bool) => {
    if cond == false { result.ok(()) } else { result.err(f"Expected false, got {cond}") }
}
```

- アサーション関数は**値意味論**： `Result((), String)` を返し、失敗は `Err(診断情報)`
  で表現される、プロセスを abort させない——§7 スイートがこれに基づいて per-test 判定を収集する。`std.assert.assert`
  のプロセスレベル abort 意味論はランタイムガード用に保持され、テストアサーション経路には入らない
- 移行説明：Phase 1 で実装される 4 関数は
  `std.assert.assert`（abort 意味論）に基づいており、セルフホスティングへの過渡的実装である；値意味論ファミリが本 RFC の標準形式であり、実装後に置換される（#319）
- `assert_eq` / `assert_ne` は**型注釈なしパラメータ**（`Any`）を使用——2026-08-02 実証：`==`/`!=`
  と f-string 補間が Any 上で正常に動作する（Int/String ともに検証済み）、**ジェネリクスシステムに依存しない**。将来的にジェネリクスが利用可能になれば型注釈を追加可能
- `assert_false` は `cond == false` で否定を表現する（`not` 単項構文は未実装、安定後に移行可能；
  `!assert` 単項形式も同様の制約に依存する、§8.1 参照）
- エラーコードアサーション（`assert(err.code == "E3017")`）は、`Error` 値が機械可読な `code`
  フィールドを持つことに依存する（§8.1）
- `std.test` は native コードに一切依存せず、純粋な YaoXiang で実装される

### 4. 標準ライブラリロード機構（重要な設計）

**Phase 1：バイナリへの埋め込み**

`std/test.yx`（および将来 YaoXiang で記述されるすべての標準ライブラリモジュール）はビルド時にバイナリに埋め込まれる：

```rust
// build.rs またはビルドスクリプトで自動生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // ソースコードテキスト
    // 将来追加
];
```

モジュールシステム（RFC-029、2026-08-02 に完全に実装済み）が接続点を提供する：Registry は native モジュールとソースモジュールの両方を保持し、orchestrator が複数ファイルの編成を担当する。`use std.test`
の解決順序：

1. まず Rust native モジュールを検索（既存の仕組み、例えば `std.assert`）
2. ヒットしない場合、埋め込まれた `STD_YX_FILES`
   を検索——ヒットすれば**仮想パス**（例：`<std>/test.yx`）をシードモジュールとして orchestrator に注入し、通常のフロントエンドパイプライン（parse
   → typecheck → IR）を経由する
3. ヒットしない場合、ファイルシステム検索（ユーザーモジュール）

埋め込みソースモジュール内部の `use std.assert` は resolver によって正常に native
registry に解決される——native モジュールとソースモジュールは Registry 内で共存し、種類を跨いだ依存関係が自然に成立する。埋め込みモジュールは**オンデマンドでコンパイルされる**：import された時にのみパイプラインに入る。

利点：

- 単一ファイルモードでも `use std.test` が機能する
- 標準ライブラリのバージョンがバイナリと厳密にバインドされ、バージョンの不一致が発生しない
- ユーザーが標準ライブラリパスを設定する必要がない

**将来：ファイルシステム標準ライブラリ**

YaoXiang のプロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式に変更される。詳細は RFC-014 の更新を参照。

### 5. 発見と実行

**前提条件（2026-08-02 審査決議）**：CLI `run` が orchestrator に接続する。現状の CLI `run`
は単一ファイルパイプライン（`run_file_with_diagnostics`）を経由し、ユーザーモジュールインポートを解析できない。一方、`yaoxiang test`
のサブプロセスモデルは CLI 能力を継承し、テストファイルがプロジェクトモジュールをインポートすることは中核的なシナリオである。したがって Phase
1 ではまず CLI `Run` のソースブランチを
`run_project`（orchestrator、ディレクトリ再帰探索）に委譲する；#247（`use`
に沿ったオンデマンド発見）はその後、純粋なパフォーマンス最適化として重ね合わせる。import を持たない単一ファイルは orchestrator 経由でも動作が等価であり、バイトコードブランチは不变。

**発見フェーズ**：

1. `[PATHS]` が指定されている場合、指定されたパスを直接使用する
2. そうでなければ `yaoxiang.toml` の `[tool.test].patterns` を読み取る
3. 設定がない場合、デフォルト `tests/**/*.yx`
4. `--filter` によるフィルタリングを適用（ファイル名に含まれる）

**実行フェーズ**：

1. 各ファイルに対して：`yaoxiang run --debug-info <file>` でサブプロセスを起動する（`--debug-info`
   によりランタイムエラーにソース位置が付属する——2026-08-02 実証 stack trace は `file:line:col`
   を出力）；ヘッダー `[test:runtime]` はサブプロセス `--runtime`
   モードを宣言する（2026-09-03 収束）
2. ヘッダー `[test:ignore]: <理由>`
   のファイルは実行をスキップし、レポートの skipped にカウントされる（2026-09-03 収束）
3. exit code を確認：0 なら PASS、0 以外なら FAIL；`[test:error]`
   ファイルは逆判定し、§8.2 に従って予期されたコードと比較する
4. レポート用に stdout/stderr をキャプチャする
5. 逐次実行のみ（Phase 1）、将来 `--parallel` をサポート予定
6. `--fail-fast` の場合、最初の FAIL で直ちに停止する

### 6. テスト分離

テスト分離はプロセスレベルの境界によって自然に実現される：

- 各テストファイルは独立したサブプロセスで実行される
- 各サブプロセスは独立した Heap、Frame、NativeContext を有する
- あるテストファイルのパニックが他のテストファイルに影響することはない
- 追加の独立 Heap コンテキスト機構は不要

### 7. スイートと複数テスト（値化モデル）

1 つのテストファイルに複数のテストを含めることができる。ファイル内の組織方法：

```yaoxiang
// tests/list_test.yx
use std.test
use std.list

push_grows_len = () => {
    xs = []
    list.append(xs, 1)
    test.assert_eq(list.len(xs), 1)
}

pop_returns_last = () => {
    xs = [1, 2]
    test.assert_eq(list.pop(xs), 2)
}

main = {
    test.suite([
        ("push_grows_len", push_grows_len),
        ("pop_returns_last", pop_returns_last),
    ])
}
```

- 各テストは `Result((), String)` を返すゼロ引数関数である；アサーション失敗は `Err`
  で表現される（§3 値意味論アサーションファミリ）、プロセスを中断しない——後続のテストは通常通り実行される
- `test.suite`
  は 1 つずつ呼び出して収集する：あるテストが Ok でない場合、そのテストの名前と診断情報を出力し、Ok は静かにする
- ファイルの exit code：スイート全 Ok → 0；いずれかが Err → 0 以外（§5 実行フェーズの exit
  code 判定は不变）
- runner はファイルのみを見、関数レベルのスキャンを行わない：per-test 判定は完全にスイート内の収集に由来し、ファイル内部の構造は runner に対して透明である——コンパイラ変更ゼロ原則は影響を受けない
- 明示的に採用しないもの：プロセス内 catch 境界（17 キーワード鉄則）；runner による関数ごとの入口呼び出し（§8.2 のコンパイル失敗などの内部シナリオのみ）
- `test.suite` の具体的な API 形式（シグネチャ、重複名の処理、`--filter`
  とスイート名の相互作用）は実装詳細であり、#319 で実装時に決定する

### 8. ネガティブテスト（予期された失敗）の 3 層設計

ネガティブテストは失敗が発生する層に従って分割され、各層に帰属する：

#### 8.1 値レベルの逆判定（汎用、ユーザ向け）

被テスト操作が `Result` を返し、テストは通常のアサーションで予期された失敗を表現する：

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
test.assert_eq(result.err_code(r), "E3017")
```

- std.test に `assert_not` / `assert_err` 関数ファミリを追加する；`!assert`
  単項形式は not 構文の実装後に提供する（`assert_false` の `cond == false` と同様の制約）
- エラーコードアサーションは、`Error` 値の機械可読 `code`
  フィールド拡張に依存する：`Struct { message }` から `{ code, message }` へ拡張し（native
  `error_new_with_code`、std がコード定数をエクスポート）、 `err.code == "E3017"`
  をアサート可能にする
- Result 化の進展（#301、#316）に伴い、失敗する可能性のある操作は順次 `Result`
  を返すようになり、テストコーパス中のファイルレベルのネガティブマーカーはファイル内アサーションに移行される

#### 8.2 コンパイル失敗（言語設計者の内部使用のみ）

コンパイルは全ファイル全か無かであり、ファイル内で「この行はコンパイルされるべきではない」を表現できない。ファイルレベルの特殊マーカーを保持する：

- `[test:error]` マーカーは runner によって読み取られ、逆判定される（run の exit code が 0 以外 =
  PASS）
- マーカーは**構造化された予期コード**にアップグレードされる：runner はヘッダーの
  `预期: 编译错误 EXXXX`（訳注：原文中文）を解析し、コンパイラの stderr 出力の `[EXXXX]`
  実際コードと比較し、コードが一致しない場合 = FAIL（trybuild
  stderr スナップショット思想の軽量版、コンパイラ変更ゼロ）
- **本リポジトリのコーパス専用であり、ユーザーテストフレームワークの一部ではない**；デュアル runner 判定規約は既に収束している（2026-09-03、#319）：yx_runner（cargo
  test）と `yaoxiang test` は `src/util/test_markers.rs`
  を共有し、ヘッダーマーカー（`[test:error]`/`[test:ignore]`/`[test:runtime]`/`预期: EXXXX`、先頭 16 行）を解析する。06-compile-errors のディレクトリ規約は廃止

#### 8.3 ランタイムハード失敗（Result 化に帰属）

独立した仕組みは設けない——失敗する可能性のある操作は言語設計の方向に従って `Result`
を返し（#301、#316）、テストは§8.1 で統一的に表現される。プロセスレベルの abort（アサーション違反、ランタイム引数エラーなど）は Result 化の進展に伴い徐々に値に収束し、テストフレームワークはこれに専用の意味論を提供しない。

## 既存システムとの関係

| 項目                                                      | 関係                                                                                                |
| --------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                            | 不変。コンパイラ内部テストは Rust を継続使用                                                        |
| 既存の `.yx` 統合テスト（`tests/yaoxiang/`）              | `yaoxiang test` によって発見・実行される                                                            |
| `std.assert.assert(cond)`                                 | ランタイムガード用に保持；`std.test` 値意味論アサーションファミリは `std.result` に基づく（§3、§7） |
| モジュールシステム（RFC-029）                             | 埋め込みソースモジュールは Registry/orchestrator 経由で接続；CLI `run` の orchestrator 接続が前提   |
| `#200` リファクタリング（`io.println` → `assert.assert`） | `yaoxiang test` と完全に同じ方向性                                                                  |
| `@` アノテーション                                        | 使用しない。`@test` を導入しない                                                                    |

## 実装戦略

### Phase 1：中核機能

変更範囲：

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` ソースブランチが `run_project`
  に委譲（複数ファイル実行の前提）
- `src/main.rs` — 新規 `Test` サブコマンド
- `src/std/test.yx` — 新規純粋 YaoXiang モジュール
- `build.rs` — `std/*.yx` をバイナリに埋め込み
- orchestrator / Registry — 埋め込みソースからの仮想パスによる `.yx` モジュールロードをサポート
- RFC-015 設定解析 — `[tool.test]` セクション
- サブプロセス実行（`--debug-info`）+ レポート

成果物：

- `yaoxiang test` が基本的に使用可能
- `std.test` の 4 つのアサーション関数
- デフォルト `tests/**/*.yx` 発見
- 逐次実行 + デフォルト出力フォーマット

### Phase 2：完成

- `--filter` / `--fail-fast` / `--verbose` パラメータ
- `--json` 出力（CI 統合）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：上級

- `--parallel` 並列実行（spawn 並行モデル完善に依存）
- `[tool.test].exclude` 設定
- より多くのアサーション関数（例：`assert_approx_eq`（Float 用））

## リスクと緩和

| リスク                                                       | 確率 | 緩和                                                                                                                                                                                                          |
| ------------------------------------------------------------ | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` の Any 上での補間失敗                               | なし | 2026-08-02 に実証済み（Int/String ともに正常）                                                                                                                                                                |
| `yaoxiang.toml` 設定解析が現在の CLI にない                  | 低   | 単純な拡張であり、中核機能に影響しない                                                                                                                                                                        |
| CLI run の orchestrator 接続による動作回帰                   | 低   | import なしの単一ファイルパスは等価；orchestrator は統合テストでカバー済み                                                                                                                                    |
| `.yx` ソースファイルをバイナリに埋め込むことによるサイズ増加 | 低   | `.yx` ソースファイルは非常に小さく、無視可能                                                                                                                                                                  |
| テストループの所要時間がコーパス増加に伴い増大               | 高   | 主要項目はファイルごとの完全コンパイル（185 ファイル実測 11.3s）であり、サブプロセス起動ではない；`--parallel` はプロセス側のみ緩和し、コンパイルコストはテストループキャッシュ（#251/#293 スライス）を要する |

## 未解決問題

- [x] `std/test.yx` 中の `use std.assert`
      参照が正しく解決できるか？——**解決済み（2026-08-02）**。モジュールシステム（RFC-029）実装後、native モジュールとソースモジュールは Registry 内で共存し、resolver が統一的に解決し、種類を跨いだ依存関係が自然に成立する
- [x] テスト出力の `f"..."` のジェネリック `to_string`
      が新しい型制約を導入するか？——**解決済み（2026-08-02）**。型注釈なしパラメータ（Any）上で
      `==`/`!=`
      と f-string 補間がともに動作することを実証（Int/String 検証済み）、新しい制約を導入しない
- [x] `?` ジェネリックパラメータの実現可能性？——**解決済み（2026-08-02）**：`?`
      型構文は現在のところ存在せず（かつ暗黙的に吞まれる、別途 issue で追跡中）、Phase
      1 のアサーション関数は型注釈なしパラメータを使用し、ジェネリクスシステムに依存しない

## 設計決定記録

| 決定                     | 決定                                                                                                | 日付       | 理由                                                                                                                                           |
| ------------------------ | --------------------------------------------------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| テストマーカー方式       | `@test` アノテーションを使用せず、テストファイルは通常の `.yx`                                      | 2026-07-26 | コンパイラ変更ゼロ、サブプロセスがそのまま隔離                                                                                                 |
| アサーション方式         | `std.test` モジュール純粋 YaoXiang 関数                                                             | 2026-07-26 | セルフホスティング、native コードなし                                                                                                          |
| テスト実行モデル         | サブプロセス `yaoxiang run <file>` + exit code                                                      | 2026-07-26 | プロセスレベル隔離、コンパイラ変更ゼロ                                                                                                         |
| 標準ライブラリロード     | 現在はバイナリ埋め込み、将来ファイルシステム                                                        | 2026-07-26 | バージョンバインディング、単一ファイル対応                                                                                                     |
| アサーションパラメータ型 | 型注釈なしパラメータ（Any）、ジェネリクスシステムに非依存                                           | 2026-08-02 | `?` 型構文が存在しない；Any で比較・補間可能を実証                                                                                             |
| 複数ファイル実行         | CLI `run` が `run_project`（orchestrator）に委譲を前提とする                                        | 2026-08-02 | サブプロセスモデルが CLI 能力を継承；#247 は純粋なパフォーマンス最適化に降格                                                                   |
| ソース位置レポート       | サブプロセスに `--debug-info` を付与                                                                | 2026-08-02 | stack trace が `file:line:col` を出力することを実証；埋め込みモジュール（std.test）経由のフレーム帰属は本保証の対象外、#289 + RFC-034 に属する |
| ネガティブテスト分层     | 値レベル逆判定汎用 / コンパイル失敗 runner 構造化マーカー（内部のみ）/ ハード失敗は Result 化に帰属 | 2026-09-02 | #319 で決定；暗黙の [test:error] 規約に取って代わる                                                                                            |
| ファイル内複数テスト     | 値化標準モデル：テスト関数が Result を返し、スイートが per-test 判定を収集                          | 2026-09-02 | catch なし、入口呼び出しなし（入口は内部シナリオのみ）                                                                                         |
| Error コード             | Error に機械可読 `code` フィールドを追加                                                            | 2026-09-02 | エラーコードアサーションをサポート；コンパイル時コードは runner 比較を経由                                                                     |

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
  — 標準ライブラリディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert アサーション機構](../review/030-assert-mechanism.md) — 基盤依存
- [Rust `#[test]` 機構](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計
