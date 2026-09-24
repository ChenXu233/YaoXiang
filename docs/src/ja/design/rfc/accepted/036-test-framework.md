---
title: 'RFC-036: std.test テストフレームワークと yaoxiang test コマンド'
status: '受け入れ済み'
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
code で全体のパス/失敗を判定する。ファイル内部は複数のテスト関数をサポートし、assertion の失敗は
`Err` 値で表現され（値セマンティクス）、スイートがテストごとの判定を収集する（§7）。`std.test`
モジュールは純粋な YaoXiang で実装され、最初の dogfooding ライブラリである。`yaoxiang test`
は CLI ツールであり、コンパイラ機能ではない——parser、IR、バイトコード、エグゼキュータのいずれにも変更を加えない。

## 動機

### なぜテストフレームワークが必要か？

現在、YaoXiang のテストカバレッジは Rust 側の `#[test]` と `tests/`
統合テストに依存している。これは次のような問題を引き起こす：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert /
   std.io）のユニットテストを YaoXiang で記述できない
2. テストインフラが利用できないため、標準ライブラリモジュールのユニットテストカバレッジがブロックされる
3. 言語機能の回帰テスト（例：RFC-032 spawn セマンティクスの変更）に自動化された手段がない

### 主な制約

- **17 キーワード鉄則**：新しいキーワードや構文構造を導入しない
- **コンパイラ変更ゼロ**：parser、IR、バイトコード、エグゼキュータに触れない
- **セルフホスティング優先**：テストライブラリは YaoXiang で記述し、最初の dogfooding ライブラリとする

## アーキテクチャ

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI 层:  yaoxiang test [--filter --fail-fast --json ...]    │
│              │                                               │
│  发现层:    读取 yaoxiang.toml → [tool.test] patterns         │
│              默认: tests/**/*.yx                              │
│              │                                               │
│  执行层:    对每个文件: yaoxiang run <file>                    │
│              检查 exit code → 串行执行                        │
│              │                                               │
│  报告层:    PASS/FAIL → 汇总                                  │
│              支持 --json / --verbose / --fail-fast            │
│                                                              │
│  断言层:    std.test (纯 YaoXiang，自举)                      │
│              底层: std.assert.assert                          │
│              诊断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### 基本原則

1. **テストフレームワークはコンパイラ機能ではなく、CLI ツールである** — `yaoxiang run`
   はすでに「テストを実行」でき、`yaoxiang test`
   はすべてのファイルを実行してレポートを表示するヘルパーに過ぎない
2. **コンパイラ変更ゼロ** — `@test`
   アノテーションスキャン、バイトコードメタデータセグメント、エグゼキュータ特殊入口を導入しない
3. **セルフホスティング** — `std.test` モジュールは純粋な YaoXiang で実装され、基盤機能は
   `std.assert` / `std.result` から取得する
4. **テストファイルは通常の `.yx` ファイル** — ファイルはサブプロセスとして実行され、exit
   code で全体のパス/失敗を判定する
5. **assertion の失敗は値であり、プロセスイベントではない** — テスト関数は `Result`
   を返し、assertion の失敗は `Err`
   で表現され、スイートがテストごとの判定を収集する（§7）；プロセスレベルの abort はランタイムガードのみに属し、テスト assertion には使用しない

## 詳細設計

### 1. CLI 設計

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      指定测试文件或目录（默认: 从 yaoxiang.toml 读取，否则 tests/）

Options:
  --filter <NAME>     只跑文件名包含 <NAME> 的测试
  --fail-fast         遇到第一个失败就停止
  --verbose, -v       显示每个测试的详细 stdout/stderr
  --list              只列出测试文件，不跑
  --no-progress       不显示进度输出（表头与 PASS 行）；FAIL 明细与汇总保留（CI 场景）
  --json              输出 JSON 格式结果（CI 集成用）
  --parallel          并行执行（每核一个 worker；与 [tool.test].parallel 取或）
```

#### 出力形式

**デフォルト出力**（per-test 判定来自文件内套件收集，见 §7）：

```
Running 3 test files...

tests/math_test.yx ........................ PASS (0.002s)
tests/list_test.yx ........................ FAIL (0.003s)
  `-- [FAIL] push_grows_len: Expected 3, got 2
  `-- [ ok ] pop_returns_last
Results: 2 files passed, 1 file failed, 0 skipped (0.006s)
Categories: 2 behavior, 0 compile-error, 0 runtime-error
```

**JSON 出力**（`--json`）：

```json
{
  "summary": {
    "total": 3,
    "passed": 2,
    "failed": 1,
    "skipped": 0,
    "by_kind": { "behavior": 3, "compile-error": 0, "runtime-error": 0, "invalid": 0 },
    "time_secs": 0.006
  },
  "files": [
    { "file": "tests/math_test.yx", "kind": "behavior", "passed": true, "time_secs": 0.002 },
    {
      "file": "tests/list_test.yx",
      "kind": "behavior",
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

- 失敗したファイルは追加で `exit_code` と `stderr`
  を携带する（ANSI 除去後のサブプロセス診断、CI のフォレンジック用）； `--verbose` と `--json`
  を組み合わせた場合、すべてのファイルが `stdout` / `stderr` を携带する
- `--no-progress`
  は進捗出力（表頭と PASS 行）のみを抑制する——FAIL 明細とサマリーは常に出力され、失敗を静かにしない；`--list`
  は 1 行に 1 つのテストファイルパスを出力し、実行しない
- 各ファイルに `kind`（behavior / compile-error / runtime-error /
  invalid、§8.2）を付加し、summary に `by_kind`
  の実行カウントを付加する（skipped を含まない固定 4 キー）；人間向けサマリーに `Categories:`
  カテゴリ分布行を付加
- `--parallel`
  下では人間向け進捗行は**完了順**でストリーム出力され（ブロック全体を交差させない）、JSON の
  `files` は `file` パスでソートされて出力が安定する（CI diff フレンドリ）
- ファイル内 per-test `tests` 配列は §7 スイート収集に由来し、値化モデルの実装で有効になる
- 公式 CI（`.github/workflows/ci.yml` の test job）はこれに従って消費する：cargo 側は
  `--test integration`（CLI 統合）と `--test yx_runner`（二重ルートコーパスガード）を実行し、その後
  `yaoxiang test --json --parallel` で階層別に実走——デフォルトモード（言語コーパス）と明示的な
  `src/std/tests`（ライブラリ層）がそれぞれ 1 つのレポートを生成する；サマリー表（total / passed /
  failed / skipped / time_secs と by_kind）を job summary に書き込み、失敗したファイルは `kind` /
  `exit_code` / `stderr` のフォレンジックを出力し、いずれかのスイートが非ゼロ終了すれば赤と判定

### 2. yaoxiang.toml 設定

`[tool.test]` 配下に配置し、RFC-015 の `[tool.*]` サードパーティ拡張規約に準拠する：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # 命中者从发现集剔除（--list 同样剔除）
parallel = true                   # 并行执行（与 --parallel flag 取或）
```

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザーはゼロ設定で即座に使い始められる
- `exclude` は `patterns` と同じ形式（リテラルパスまたは
  `root/**…`、いずれもパスプレフィックスでマッチング）；excluded はテストではないことを意味し、ランタイム挙動の検証が必要なフィクスチャは
  `yaoxiang run` を直接実行する
- **単ファイルモード（`yaoxiang test foo.yx`）は設定を一切読まずに直接実行する**——明示的な paths 下では
  `exclude`/`parallel` 設定キーはいずれも無効（フラグを除く）
- 将来的に別リポジトリに分割される可能性がある（`[tool.test]` の位置は不変）

### 3. std.test モジュール（純粋な YaoXiang）

```yaoxiang
// std/test.yx — 纯 YaoXiang 测试断言库（值语义标准形态，2026-09-03 落地）
// 第一个 dogfooding 库：YaoXiang 的测试库用 YaoXiang 写。

use std.result

assert_eq: (a: Any, b: Any) -> Result(Void, String) = (a, b) => {
    if a == b { return result.ok(void) }
    return result.err(f"Expected {b}, got {a}")
}

assert_ne: (a: Any, b: Any) -> Result(Void, String) = (a, b) => {
    if a != b { return result.ok(void) }
    return result.err(f"Expected not equal to {b}, got {a}")
}

assert_true: (cond: Bool) -> Result(Void, String) = (cond) => {
    if cond { return result.ok(void) }
    return result.err(f"Expected true, got {cond}")
}

// assert_not 与 assert_false 同体；assert_err / assert_err_code 见 §8.1
```

- assertion 関数は**値セマンティクス**：`Result(Void, String)` を返し、失敗は `Err(診断情報)`
  で表現され、プロセスは abort しない——§7 スイートがこれに基づいて per-test 判定を収集する。`std.assert.assert`
  のプロセスレベル abort セマンティクスはランタイムガード用に保持され、テスト assertion 経路には入らない。Ok ペイロードは
  `Void`（type-system.md で規範 unit と規定；`()` は空 Tuple で両者は混在しない——2026-09-03 決定）
- **関数族 7 個（2026-09-03 配信済み、abort 過渡版削除）**：値化 `assert_eq` / `assert_ne` /
  `assert_true` / `assert_false` + `assert_not`（assert_false と同体、`!assert` への換装用に確保）+
  `assert_err`（§8.1）
  - `assert_err_code`（§8.1 エラーコード assertion）
  - `assert_approx_eq(a: Float, b: Float, eps: Float)`（Phase 3、2026-09-07 配信）：`|a - b| <= eps`
    判定、eps は呼び出し側が**明示的に与える**——許容差はテスト契約の一部であり、隠れたデフォルトは設定しない；負の eps は宣言時点で Err、NaN は常に Err
- `assert_eq` / `assert_ne` は **Any 注釈パラメータ** を使用——`==`/`!=`
  と f-string 補間は Any 上で正常に動作し、ジェネリクスシステムに依存しない。**パラメータは明示的に注釈する必要がある**ことに注意：注釈なしパラメータは native ジェネリクス
  `&Result(T, E)` の呼び出しチェックを通過できない（R1 プローブで実証）
- `assert_false` / `assert_not` は `cond == false` で否定を表現する（`not`
  の単項構文は未配信のため、安定後に移行可能；`!assert` の単項形態もこれに依存、§8.1 参照）
- ブロック本体 + 明示的な `return`
  形式：if 式の then 腕の型がチェックで破棄されるため、両腕 Result の if 式はチェックの盲点であり、実装でこれを回避する
- `std.test` は native コードに一切依存せず、純粋な YaoXiang で実装される

### 4. 標準ライブラリ読み込み機構（重要な設計）

**Phase 1：バイナリ埋め込み**

`std/test.yx`（および将来的に YaoXiang で書かれるすべての標準ライブラリモジュール）はビルド時にバイナリに埋め込まれる：

```rust
// build.rs 或构建脚本，自动生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // 源代码文本
    // 未来更多
];
```

モジュールシステム（RFC-029、2026-08-02 完全配信済み）がアクセスポイントを提供：Registry は native モジュールとソースモジュールの両方を保持し、orchestrator が複数ファイルの編成を担当する。`use std.test`
の解析順序：

1. まず Rust の native モジュールを検索（既存メカニズム、例：`std.assert`）
2. ヒットしなければ、埋め込まれた `STD_YX_FILES`
   を検索——ヒットすれば**仮想パス**（例：`<std>/test.yx`）をシードモジュールとして orchestrator に注入し、通常のフロントエンドパイプライン（parse
   → typecheck → IR）を通る
3. ヒットしなければ、ファイルシステム探索（ユーザーモジュール）

埋め込まれたソースモジュール内部の `use std.assert` は resolver によって正常に native
registry まで解析される——native とソースモジュールは Registry 内で共存し、種類をまたぐ依存は自然に成立する。埋め込まれたモジュールは**オンデマンドでコンパイル**される：import されたときのみパイプラインに入る。

利点：

- 単ファイルモードでも `use std.test` が動作する
- 標準ライブラリのバージョンとバイナリが厳密に紐付けられ、バージョンミスマッチが発生しない
- ユーザーが標準ライブラリパスを設定する必要がない

**将来：ファイルシステム標準ライブラリ**

YaoXiang のプロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式に移行する。詳細は RFC-014 の更新を参照。

### 5. 発見と実行

**前提条件（2026-08-02 レビュー決議）**：CLI `run` を orchestrator に接続する。現状の CLI `run`
は単ファイルパイプライン（`run_file_with_diagnostics`）を通り、ユーザーモジュールのインポートを解析できない。一方
`yaoxiang test`
のサブプロセスモデルは CLI の能力を継承し、テストファイルがプロジェクトモジュールをインポートすることが中心的なシナリオとなる。したがって Phase
1 ではまず CLI `Run` のソースブランチを
`run_project`（orchestrator、ディレクトリ再帰的発見）に委譲する；use 経由のオンデマンド発見は、その後純粋なパフォーマンス最適化として重ねる。import なしの単ファイルは orchestrator 経由でも動作が等価であり、バイトコードブランチは変わらない。

**発見フェーズ**：

1. `[PATHS]` が指定されている場合、指定されたパスを直接使用する
2. そうでなければ `yaoxiang.toml` の `[tool.test].patterns` を読み取る
3. 設定がない場合、デフォルトで `tests/**/*.yx`
4. `--filter` を適用してフィルタリングする（ファイル名に含まれる）
5. 発見範囲はテスト階層化そのもの（§9）：デフォルトの patterns は言語利用可能性コーパスのみをカバーする；ライブラリテスト層（例：`src/std/tests/`）は明示的なパスまたはパッケージ設定で発見され、デフォルトスキャンには混ざらない

**実行フェーズ**：

1. 各ファイルはヘッダー指令に応じて分岐実行される（指令文法は §8.2 を参照、`src/util/test_markers.rs`
   で解析され、yx_runner と共有）：
   - 動作テスト：`yaoxiang run <file>`
     サブプロセス（ランタイムエラーはデフォルトでソース位置とスタックフレームを携带——debug_map はデフォルトで生成され、stack
     trace は `file:line:col` を出力する）；`// mode:` はサブプロセスの `--runtime` モードを宣言する
   - コンパイル時拒否種：単一ステップ `yaoxiang check <file>`
   - ランタイム失敗種：`check`（成功必須）+ `run`（失敗必須）の 2 ステップ
2. `// skip: <原因>` のファイルは実行をスキップし、レポートの skipped にカウントされる
3. 判定と期待コードの比較は §8.2 の判定マトリクスに従う；指令解析が失敗した場合、実行せず直接 FAIL（構築期拒否）
4. stdout/stderr をレポート用にキャプチャする
5. デフォルトは逐次実行；`--parallel`（または
   `[tool.test].parallel`）で利用可能なコア数に応じて worker プールを起動し、各ファイルは引き続き独立したサブプロセス——skip/invalid は発見順で先行処理、実行結果は完了順でストリーム出力、JSON はパスでソート（Phase
   3、2026-09-07 配信）
6. `--fail-fast`
   の場合、最初の FAIL でただちに新規ファイルのスケジュールを停止する；並列モードでは実行中のファイルは完了まで実行されカウントされる

### 6. テスト分離

テスト分離はプロセスレベルの境界によって自然に実現される：

- 各テストファイルは独立したサブプロセスで実行される
- 各サブプロセスは独立した Heap、Frame、NativeContext を有する
- あるテストファイルの panic は他のテストファイルに影響しない
- 追加の独立 Heap コンテキスト機構は不要
- **並列実行（Phase
  3）は分離境界を拡張しない**：サブプロセス間の作業ディレクトリ（CWD）は共有され、並列テストは CWD 内の同じパスのファイルを占有してはならない——ファイル I/O 系のテストは独立したファイル名を使用し、終了時にクリーンアップする

### 7. スイートと複数テスト（値化モデル）

1 つのテストファイルに複数のテストを含め得る。ファイル内の構成方式（2026-09-03 配信済み）：

```yaoxiang
// tests/list_test.yx
use std.list
use std.result
use std.test

push_grows_len: () -> Result(Void, String) = () => {
    xs = [1]
    extended = list.push(xs, 2)
    test.assert_eq(list.len(extended), 2)
}

pop_returns_last: () -> Result(Void, String) = () => {
    mut xs = [1, 2]
    last = list.pop(xs)
    test.assert_eq(last, 2)
}

main: () -> Void = {
    test.suite([
        ("push_grows_len", () => push_grows_len()),
        ("pop_returns_last", () => pop_returns_last()),
    ])
}
```

- 各テストは `Result(Void, String)` を返すゼロ引数関数；assertion の失敗は `Err`
  で表現され（§3 値セマンティクス assertion 族）、プロセスは中断しない——後続のテストは通常通り実行される
- `test.suite`
  は順次呼び出して収集する：あるテストが Ok でなければ名前と診断を記録し、Ok は静かに通過；すべて実行後、Err が 1 つでもあれば
  `std.assert.assert` で中止し、失敗明細（`N of M test(s) failed` + 各項目
  `[FAIL] 名前: 診断`）を付加する——ファイル終了コードは非 0（§5 判定は不変）。ここでの abort はテストバイナリのランタイムガードであり、assertion 経路ではない；すべて Ok なら静かに 0 で終了する
- トップレベルのテスト関数は**クロージャ形式**でリストされる（`("name", () => test_fn())`）：トップレベルの関数名を値として参照することは現在サポートされていない（IR 層の制限、`E3006`）——クロージャ本体からグローバル関数を呼び出すことには影響しない
- runner はファイルのみを見知り、関数レベルのスキャンは行わない：per-test 判定は完全にスイート内の収集に由来し、ファイル内部構造は runner に対して透過的である——コンパイラ変更ゼロの原則は影響を受けない
- 明示的に採用しない：プロセス内 catch 境界（17 キーワード鉄則）；runner からの関数ごとのエントリ呼び出し（§8.2 のコンパイル失敗などの内部シナリオのみ）
- API 形態は決定済み（2026-09-03）：`suite(tests: List((String, () -> Result(Void, String)))) -> Void`；重複名はチェックしない（名前はレポート表示用のみ）；`--filter`
  はファイル名でフィルタリングし、スイート内のテスト名は認識しない

### 8. ネガティブテスト（想定失敗）三層設計

ネガティブテストは失敗の発生層に従って分割され、それぞれ帰属する：

#### 8.1 値レベルの反転（汎用、ユーザ向け）

被験操作が `Result` を返し、テストは通常の assertion で想定失敗を表現する：

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// 或一句封装（码只存在于 std Error 载体上，E 钉死为 Error）：
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code`
  は値セマンティクス族と共に配信済み（2026-09-03、§3）；`!assert` 単項形態は `not`
  構文の配信後に提供予定（`assert_false` の `cond == false` と同じ制約）
- エラーコード assertion は `Error` 値が機械可読な `code`
  フィールドを携带することに依存——配信済み：`Error = { code, message }`（native
  `error_new(code, message)`）、読み取りは `result.unwrap_err(r)` でキャリアを取得 +
  `result.code(e)` / `result.message(e)` アクセサ（設計段階で予想されていた `error_new_with_code`
  命名、コード定数のエクスポート、`err.code`
  フィールドアクセスはいずれも採用せず——言語に Struct フィールドアクセスがなく、コード定数もエクスポートされていない）
- Result 化の推進に伴い、失敗し得る操作が順次 `Result`
  を返すようになり、コーパス内のファイルレベルのネガティブマークはファイル内 assertion に移行する

#### 8.2 ファイルレベルのネガティブディレクティブ（言語設計者内部使用限定）

コンパイルはファイル全体に対し全か無かであり、「この行はコンパイルされるべきでない」をファイル内で表現できない；ランタイム失敗もファイルレベルでの表現が必要（スイートに必ず失敗するテストが含まれる場合など）。ファイルヘッダで**構造化ディレクティブ**により期待を宣言し、runner は期待カテゴリに応じて**分岐判定**する（2026-09-03 分岐決定；2026-09-06 ディレクティブ文法決定・配信）。

ヘッダーディレクティブ文法（`// key: value`、先頭 16 行以内、厳密な token マッチ）：

```text
// expect: compile-error E1002 [E1003 ...]   编译期拒绝测试
// expect: runtime-error E6003 [...]         运行期失败测试
// skip: <原因>                              跳过执行，计入 skipped
// mode: embedded|standard|full              子进程 --runtime 模式（仅 run 步消费）
```

- `expect:` ディレクティブなし = 動作テスト。`expect:` は期待の**唯一の宣言**——2026-09-06 決定で
  `[test:error]` ブールマークと中文 `预期:`
  散文コード抽出を廃止：ブールマークと期待行という 2 つの緩く結合された事実は規律で一致を保たせるしかなく、必ずドリフトする；英語の厳格な token 文法により、runner は機械的に解析する（kind とコードは固定 token、コード後に余分な token があれば解析失敗）、解析失敗 = 実行せず直接 FAIL——ディレクティブ宣言エラーの静かな縮退経路は存在しない（構築期拒否）
- **コンパイルエラー種**：単一ステップ `check`——失敗が必須で、出力にすべての `[EXXXX]`
  を含む必要がある；コンパイル成功 =
  FAIL（報告されるべきエラーが報告されない）、拒否されたがコードが一致しない =
  FAIL。構文エラー（E1xxx 解析段階）と意味エラー（E2xxx+）は独立カテゴリを設けない——期待コード自体がフェーズを固定する
- **ランタイムエラー種**：2 ステップ判定——`check` が**成功**必須（コンパイル時に無実）、`run`
  が失敗必須で出力にすべての `[EXXXX]` を含む。コンパイル時に爆発 =
  FAIL（コンパイルエラー種と逆方向の重要な判定、「コンパイルが予期せず成功し、ランタイムが侥幸で失敗する」ことの漏出防止）
- 業界と同型：Rust compiletest の `//~ ERROR`、Go の `// ERROR "regexp"`、GCC の
  `dg-error`、Clang の `expected-error`
  はいずれも fixture コメント内で期待を宣言し、harness が双方向比較する；これらが行レベルアンカーを採用するのは、複数診断コンパイラが同ファイル内の複数の期待を区別する必要があるためで、本コンパイラは最初のエラーで停止し 1 ファイル 1 診断であるため、ファイルレベルはコンパイラの現実に同型——エラー回復が配信された後、文法に行アンカー形態を追加できる（Cranelift
  filetests のファイルヘッダディレクティブ + 関数レベル期待も同じ混合形態）
- 既知の表示に関する技術的負債：parse 段階の診断は現在 Debug 形式で出力される（`code: "E0012"`
  でなく `[E0012]`）、コードスキャンは両形式を受け入れる；診断表示が統一された後、厳格形式を取り戻す
- **本リポジトリのコーパスにのみ適用され、ユーザーテストフレームワークの一部ではない**；二重 runner 判定規約は収束済み（2026-09-03）：yx_runner（cargo
  test）と `yaoxiang test` は `src/util/test_markers.rs`
  でヘッダーディレクティブを解析し、06-compile-errors のディレクトリ規約は廃止。レポート層はカテゴリ数を提供：人間向けサマリーに
  `Categories:` 行を付加、JSON summary に `by_kind`（behavior / compile-error / runtime-error /
  invalid、skipped は別途カウント）、各ファイルに `kind` を付加

#### 8.3 ランタイムハード失敗（Result 化に集約）

独立した機構は設けない——失敗し得る操作は言語の方向性に従って `Result`
を返し、テストは §8.1 で統一的に表現する。プロセスレベルの abort（assertion 違反、ランタイムパラメータの誤配置など）は Result 化の推進に伴い順次値に収束し、テストフレームワークはこれ専用のセマンティクスを提供しない。（注意：§8.2 の「ランタイムエラー種」マーク判定は、runner による**まだ Result 化できない操作**のファイルレベル検証チャネルであり、本節の意味論的方向と矛盾しない——後者は終点、前者は移行期のチャネル）

### 9. テスト体系の階層化：言語テストコーパスとライブラリテスト（2026-09-03 決定）

テストは**被験対象**によって 2 層に分割され、それぞれ帰属とメンテナーが異なる；マークシステム（§8.2）と assertion ライブラリ（§3）は両層で共通：

**第一層：言語利用可能性コーパス（`tests/yaoxiang/`）**

- 被験対象は**言語そのもの**——パーサ、型システム、モジュール、並行、所有権、コンパイル時拒否、ランタイムセマンティクス；ディレクトリは言語仕様の章に従って構成される
- std はコーパス内では**assertion ツール**としてのみ使用され（`std.assert` /
  `std.test`）、決して被験対象とならない——ライブラリの API 振る舞いは言語利用可能性に属さない
- コーパス内では §8 に従って動作テスト / コンパイル時拒否テスト / ランタイム失敗テストの 3 種類に分岐判定される

**第二層：ライブラリテスト（ライブラリに同梱）**

- 被験対象は**ライブラリの公開 API 契約**（例：`list.push` の振る舞い、`result.code`
  のセマンティクス）
- テストは**ライブラリ自身のパッケージ内に記述**：std のパッケージは `src/std/`
  であり、その yx レベルのテストは `src/std/tests/`
  に属する（実装と同じ場所にある；ディレクトリは Rust ユニットテストと共存するが、ファイルタイプは重ならない）；`std.test`
  自身のテストもここに含まれる（std.test で std.test をテストし、セルフホスティングのクローズドループ）
- 将来のユーザーパッケージも同一の慣例に従う：テストはパッケージ内にあり、パッケージの `[tool.test]`
  で発見される（RFC-014 パッケージ管理のテストレイアウトのリハーサル）
- 発見はデフォルトの patterns には入らない（デフォルトの `tests/**/*.yx`
  は言語層のみをカバー）：ライブラリテスト層は明示的なパス（`yaoxiang test src/std/tests`）またはパッケージ設定で発見される；CI は階層別に実行される

移行注記：**移行済み（2026-09-06）**——元の `tests/yaoxiang/07-std/`
の 19 ファイルは個別に精査した結果、すべてライブラリテストであり（被験対象はすべて std モジュールの API 契約；`?`
伝播、自動借用、ジェネリクスインスタンス化などの言語特性はそこでは担体として機能するのみで被験対象ではない）、`src/std/tests/`
へ一括移行し 07-std ディレクトリを撤廃した；yx_runner は二重ルート発見（`tests/yaoxiang/` +
`src/std/tests/`）に変更し、デフォルト patterns はライブラリ層を含まない（統合テストがこの契約を固化）。言語コーパスには以降 std-API テストが含まれない。

## 既存システムとの関係

| 項目                                             | 関係                                                                                                            |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                   | 変更なし、コンパイラ内部テストは引き続き Rust を使用                                                            |
| 既存の `.yx` 統合テスト（`tests/yaoxiang/`）     | `yaoxiang test` で発見・実行される                                                                              |
| `std.assert.assert(cond)`                        | ランタイムガード用に保持；`std.test` の値セマンティクス assertion 族は `std.result` ベースに変更（§3、§7）      |
| モジュールシステム（RFC-029）                    | 埋め込みソースモジュールは Registry/orchestrator 経由でアクセス；CLI `run` を orchestrator に接続することが前提 |
| コーパス再構成（`io.println` → `assert.assert`） | `yaoxiang test` と完全に同じ方向性                                                                              |
| `@` アノテーション                               | 使用しない、`@test` を導入しない                                                                                |

## 実装戦略

### Phase 1：コア機能

変更範囲：

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` ソースブランチを `run_project`
  に委譲（複数ファイル実行の前提）
- `src/main.rs` — 新規 `Test` サブコマンド
- `src/std/test.yx` — 新規の純粋な YaoXiang モジュール
- `build.rs` — `std/*.yx` をバイナリに埋め込む
- orchestrator / Registry — 埋め込まれたソースを仮想パスで `.yx` モジュールとして読み込み対応
- RFC-015 設定解析 — `[tool.test]` セクション
- サブプロセス実行 + レポート

成果物：

- `yaoxiang test` が基本的に使用可能
- `std.test` 4 つの assertion 関数
- デフォルトの `tests/**/*.yx` 発見
- 逐次実行 + デフォルト出力形式

### Phase 2：完成

- `--filter` / `--fail-fast` / `--verbose` パラメータ
- `--json` 出力（CI 統合）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：高度化（2026-09-07 配信済み）

- `--parallel` 並列実行（worker プール + 各ファイル独立サブプロセス；`[tool.test].parallel`
  設定キーも同効）
- `[tool.test].exclude` 設定（プレフィックス一致で除去、`--list` でも同様に除去）
- `assert_approx_eq`（Float の明示的 eps assertion、§3）

## リスクと対策

| リスク                                                     | 確率 | 対策                                                                                                                                                                                            |
| ---------------------------------------------------------- | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` の Any 上での補間失敗                             | なし | 2026-08-02 に実証済み（Int/String ともに正常）                                                                                                                                                  |
| `yaoxiang.toml` 設定解析が現在の CLI に存在しない          | 低   | 単純な拡張で、コア機能に影響しない                                                                                                                                                              |
| CLI run を orchestrator に接続することで挙動の回帰が発生   | 低   | import なしの単ファイルパスは等価；orchestrator は統合テストでカバー済み                                                                                                                        |
| `.yx` ソースファイルをバイナリに埋め込むことでサイズが増加 | 低   | `.yx` ソースファイルは非常に小さく、無視できる                                                                                                                                                  |
| コーパスの成長に伴うテストループの所要時間                 | 高   | 主なコストはファイルごとのフルコンパイル（185 ファイル実測 11.3s）で、サブプロセス起動ではない；`--parallel` はプロセス側のみを緩和し、コンパイルコストはテストループのキャッシュスライスが必要 |

## 未解決問題

- [x] `std/test.yx` 内の `use std.assert`
      の参照は正しく解析されるか？——**解決済み（2026-08-02）**。モジュールシステム（RFC-029）の導入後、native とソースモジュールが Registry 内で共存し、resolver が統一的に解析するため、種類をまたぐ依存は自然に成立する
- [x] テスト出力の `f"..."` のジェネリック `to_string`
      は新しい型制約を導入するか？——**解決済み（2026-08-02）**。実証により、注釈なしパラメータ（Any）上で
      `==`/`!=` と f-string 補間がともに動作し（Int/String で検証済み）、新しい制約を導入しない
- [x] `?` ジェネリックパラメータの実現可能性は？——**解決済み（2026-08-02）**：`?`
      型構文は現状存在せず（かつ暗黙的に飲み込まれる、別途 issue で追跡中）、Phase
      1 の assertion 関数は注釈なしパラメータを使用し、ジェネリクスシステムに依存しない

## 設計決定記録

| 決定                       | 決定内容                                                                                                                                                                                                                                                                                      | 日付                     | 理由                                                                                                                                                                                                                                                           |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| テストマーク方式           | `@test` アノテーションを使用せず、テストファイルは通常の `.yx` とする                                                                                                                                                                                                                         | 2026-07-26               | コンパイラ変更ゼロ、サブプロセスで分離される                                                                                                                                                                                                                   |
| assertion 方式             | `std.test` モジュールは純粋な YaoXiang 関数                                                                                                                                                                                                                                                   | 2026-07-26               | セルフホスティング、native コードなし                                                                                                                                                                                                                          |
| テスト実行モデル           | サブプロセス `yaoxiang run <file>` + exit code                                                                                                                                                                                                                                                | 2026-07-26               | プロセスレベル分離、コンパイラ変更ゼロ                                                                                                                                                                                                                         |
| 標準ライブラリ読み込み     | 現状はバイナリ埋め込み、将来的にファイルシステムへ                                                                                                                                                                                                                                            | 2026-07-26               | バージョン紐付け、単ファイルで動作                                                                                                                                                                                                                             |
| assertion パラメータ型     | 注釈なしパラメータ（Any）、ジェネリクスシステムに依存しない                                                                                                                                                                                                                                   | 2026-08-02               | `?` 型構文が存在しない；Any は実証により比較・補間可能                                                                                                                                                                                                         |
| 複数ファイル実行           | 前提として CLI `run` を `run_project`（orchestrator）に委譲                                                                                                                                                                                                                                   | 2026-08-02               | サブプロセスモデルが CLI 能力を継承；use 経由の遅延発見は純粋なパフォーマンス最適化に退化する                                                                                                                                                                  |
| ソース位置の報告           | ランタイムエラーはデフォルトでソース位置を携带                                                                                                                                                                                                                                                | 2026-09-07               | debug_map はデフォルトで生成され、stack trace は `file:line:col` を出力する；埋め込みモジュール（std.test）を経由したフレームの帰属はこの保証の対象外で、RFC-034 に属する                                                                                      |
| ネガティブテストの階層化   | 値レベルの反転は汎用 / コンパイル失敗は runner の構造化マーク（内部のみ）/ ハード失敗は Result 化へ集約                                                                                                                                                                                       | 2026-09-02               | 値化モデルの決定；暗黙的な [test:error] 規約に代わる                                                                                                                                                                                                           |
| ファイル内複数テスト       | 値化標準モデル：テスト関数は Result を返し、スイートが per-test 判定を収集する                                                                                                                                                                                                                | 2026-09-02               | catch なし、エントリ呼び出しなし（エントリは内部シナリオのみ）                                                                                                                                                                                                 |
| Error コード               | Error に機械可読 `code` フィールドを追加                                                                                                                                                                                                                                                      | 2026-09-02               | エラーコード assertion のサポート；コンパイル時コードは runner で比較                                                                                                                                                                                          |
| assertion ライブラリの形態 | 値セマンティクス族 7 関数を配信、`Result(Void, String)` 契約；abort 過渡版を削除                                                                                                                                                                                                              | 2026-09-03               | Void は規範 unit（`()` は空 Tuple で混在しない）；ネスト位置の Any は剛性があり、注釈なしパラメータは native ジェネリクスチェックを通過しないため、パラメータは明示的に注釈する必要がある（R1 プローブで実証）                                                 |
| テスト体系の階層化         | 言語コーパス（`tests/yaoxiang/`）とライブラリテスト（ライブラリに同梱、std → `src/std/tests/`）の 2 階層；std はコーパス内では assertion ツールとしてのみ使用                                                                                                                                 | 2026-09-03               | 被験対象が帰属とメンテナーを決定する；ライブラリテストのパッケージレイアウトは RFC-014 のリハーサル                                                                                                                                                            |
| ネガティブマーク分岐判定   | 期待カテゴリで分岐：コンパイルエラー種は `check` 失敗必須、ランタイムエラー種は `check` 成功 + `run` 失敗必須；レポートにカテゴリ数を出力                                                                                                                                                     | 2026-09-03（09-06 配信） | カテゴリの混在判定は「コンパイルが予期せず成功し、ランタイムが侥幸で失敗する」ことを見落とす；期待コードでフェーズを固定し、構文エラーは独立カテゴリを設けない                                                                                                 |
| ヘッダーディレクティブ文法 | 期待は英語の構造化ディレクティブで宣言（`// expect:` / `// skip:` / `// mode:`、厳格な token 文法、解析失敗時は直接 FAIL）；`[test:error]` ブールマークと中文 `预期:` 散文コード抽出を廃止                                                                                                    | 2026-09-06               | 期待は fixture 内容の属性であり、in-fixture 宣言は業界と同型（compiletest / Go / GCC / Clang すべて同様）、中央リストは必ず腐る；ブールマーク + 期待行の二重事実は規律で結合されるが、これは欠陥面；構造化文法により runner は人手を介さずに機械的に判定できる |
| 並列実行モデル             | `--parallel` でコアごとに worker プールを起動（各ファイルは引き続き独立サブプロセス）、skip/invalid は発見順で先行処理、実行結果は完了順でストリーム出力、JSON はパスでソート；`--fail-fast` はスケジュールを停止（実行中のものは完了までカウント）；CWD は引き続き共有、分離境界は拡張しない | 2026-09-07               | サブプロセスモデルでの並列は OS スレッドスケジューリングの spawn であり、yx 層の並行性は不要；所要時間の主項目はファイルごとのフルコンパイル（リスク表）で、並列はプロセス側のみを緩和する——テストループのキャッシュスライスが主な緩和策                       |

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
  — 標準ライブラリのディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert 機構](./030-assert-mechanism.md) — 基盤依存
- [Rust `#[test]` 機構](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計
