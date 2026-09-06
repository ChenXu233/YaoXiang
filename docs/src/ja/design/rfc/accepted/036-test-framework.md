---
title: 'RFC-036: std.test テストフレームワークと yaoxiang test サブコマンド'
status: '承認済み'
author: '晨煦'
created: '2026-07-26'
updated: '2026-09-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test テストフレームワークと yaoxiang test サブコマンド

## 概要

YaoXiangに標準テストフレームワーク `std.test` モジュールと `yaoxiang test`
CLIサブコマンドを導入する。テストファイルは通常の `.yx` ファイルであり、子プロセスの exit
code で全体の成功/失敗を判定する。ファイル内部では複数のテスト関数をサポートし、アサーション失敗は
`Err` 値で表現される（値意味論）。スイートが per-test 判定を収集する（§7）。`std.test`
モジュールは純YaoXiangで実装され、初のdogfoodingライブラリである。`yaoxiang test`
はCLIツールであり、コンパイラ機能ではない。parser、IR、バイトコード、実行器には一切変更を加えない。

## 動機

### なぜテストフレームワークが必要か？

現在のYaoXiangのテストカバレッジはRust側の `#[test]` と `tests/`
統合テストに依存している。これは次のような問題がある：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert /
   std.io）の単体テストをYaoXiangで記述できない
2. `#117 標準ライブラリ各モジュールの単体テストカバレッジ`
   がブロックされている。使用可能なテストインフラが存在しないためである
3. 言語機能の回帰テスト（例：RFC-032 spawn セマンティクスの変更）の自動化手段がない

### 重要な制約

- **17キーワードの鉄則**：新しいキーワードや文法構造を導入しない
- **コンパイラ変更ゼロ**：parser、IR、バイトコード、実行器に触れない
- **セルフホスティング優先**：テストライブラリはYaoXiangで記述する。初のdogfoodingライブラリ

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

### 中核となる原則

1. **テストフレームワークはコンパイラ機能ではなく、CLIツールである** — `yaoxiang run`
   はすでに「テスト実行」が可能であり、`yaoxiang test`
   は単にすべてのファイルを実行してレポートを表示するものに過ぎない
2. **コンパイラ変更ゼロ** — `@test`
   注釈スキャン、バイトコードメタデータセグメント、実行器特殊入口を導入しない
3. **セルフホスティング** — `std.test` モジュールは純YaoXiangで実装され、基盤能力は `std.assert` /
   `std.result` に由来する
4. **テストファイルは通常の `.yx` ファイル** — ファイルは子プロセスとして実行され、exit
   code で全体の成功/失敗を判定する
5. **アサーション失敗は値であり、プロセスイベントではない** — テスト関数は `Result`
   を返し、アサーション失敗は `Err`
   で表現され、スイートが per-test 判定を一つずつ収集する（§7）。プロセスレベルの abort は runtime ガードにのみ属し、テストアサーションには使用しない

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
```

#### 出力フォーマット

**デフォルト出力**（per-test 判定はファイル内スイート収集による、§7 参照）：

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

- 失敗ファイルは `exit_code` と
  `stderr`（ANSI 除去後の子プロセス診断、CI のフォレンジック用）を追加で保持する。`--verbose` と
  `--json` の組み合わせ時はすべてのファイルが `stdout` / `stderr` を保持する
- `--no-progress`
  は進捗出力（ヘッダーと PASS 行）のみを抑制する。FAIL詳細とサマリーは常に出力され、失敗をサイレントにしない。`--list`
  はテストファイルパスを1行ずつ出力し、実行はしない
- ファイル内の per-test `tests`
  配列は §7 のスイート収集に由来し、値化モデルの実装に伴い有効になる（#319）

### 2. yaoxiang.toml 設定

`[tool.test]` の下に配置し、RFC-015の `[tool.*]` サードパーティ拡張規約に準拠する：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
# 未来可扩展:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザー設定不要で即座に利用可能
- 単一ファイルモード（`yaoxiang test foo.yx`）は直接実行され、設定は読み込まれない
- 将来的には独立リポジトリに分割される可能性がある（`[tool.test]` の位置は変更なし）

### 3. std.test モジュール（純YaoXiang）

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

- アサーション関数は**値意味論**：`Result(Void, String)` を返し、失敗は `Err(診断情報)`
  で表現され、プロセスを abort しない。§7 スイートはこれに基づき per-test 判定を収集する。`std.assert.assert`
  のプロセスレベル abort セマンティクスは runtime ガードのために保持され、テストアサーション経路には入らない。Ok ペイロードは
  `Void`（type-system.md 仕様の unit；`()` は空 Tuple であり、両者は混用しない — 2026-09-03 決定）
- **関数ファミリ 7 個（2026-09-03 配信済、abort 過渡版削除）**：値化 `assert_eq` / `assert_ne` /
  `assert_true` / `assert_false` + `assert_not` （assert_false と同一実装、`!assert` 換装用に確保）+
  `assert_err`（§8.1）
  - `assert_err_code`（§8.1 エラーコードアサーション）
- `assert_eq` / `assert_ne` は **Any 型の注釈付き引数** を使用 — `==`/`!=`
  と f-string 補間は Any 上で正常に動作し、generics システムに依存しない。引数は**明示的に注釈を付ける必要がある**ことに注意：無注釈引数は native ジェネリクス
  `&Result(T, E)` の呼び出しチェックを通過しない（R1 プローブ実証）
- `assert_false` / `assert_not` は `cond == false` で否定を表現する（`not`
  単項構文は未実装、安定後に移行可能；`!assert` 単項形態も同じ依存関係、§8.1 参照）
- ブロック本体 + 明示的 `return`
  形態：if 式の then 枝の型はチェックで破棄され、両枝 Result の if 式はチェックの盲点となるため、実装でこれを回避する
- `std.test` は native コードに一切依存せず、純YaoXiangで実装される

### 4. 標準ライブラリロード機構（重要設計）

**Phase 1：バイナリへの埋め込み**

`std/test.yx`（および将来 YaoXiang で書かれるすべての標準ライブラリモジュール）はビルド時にバイナリに埋め込まれる：

```rust
// build.rs 或构建脚本，自动生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // 源代码文本
    // 未来更多
];
```

モジュールシステム（RFC-029、2026-08-02 完全実装済）がアクセスポイントを提供する：Registry は native モジュールとソースモジュールの両方を保持し、orchestrator が複数ファイルの編成を担当する。`use std.test`
の解決順序：

1. まず Rust native モジュールを検索（既存メカニズム、例：`std.assert`）
2. ヒットしない場合、埋め込まれた `STD_YX_FILES`
   を検索 — ヒットすれば**仮想パス**（例：`<std>/test.yx`）をシードモジュールとして orchestrator に注入し、正常なフロントエンドパイプライン（parse
   → typecheck → IR）を経由する
3. ヒットしない場合、ファイルシステム検出（ユーザーモジュール）を経由する

埋め込まれたソースモジュール内部の `use std.assert` は resolver によって正常に native
registry へ解決される —
native とソースモジュールは Registry 内で共存し、種類を横断した依存関係が自然に成立する。埋め込みモジュールは**オンデマンドでコンパイル**される：import されたときにのみパイプラインに入る。

利点：

- 単一ファイルモードでも `use std.test` が動作する
- 標準ライブラリのバージョンとバイナリが厳密に紐付けられ、バージョンミスマッチが発生しない
- ユーザーが標準ライブラリのパスを設定する必要がない

**将来：ファイルシステム標準ライブラリ**

YaoXiang のプロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式に変更される。詳細は RFC-014 の更新を参照。

### 5. 検出と実行

**前提条件（2026-08-02 レビュー決議）**：CLI `run` を orchestrator に接続する。現状の CLI `run`
は単一ファイルパイプライン（`run_file_with_diagnostics`）を経由し、ユーザーモジュールのインポートを解析できない。一方、`yaoxiang test`
の子プロセスモデルは CLI 能力を継承し、テストファイルがプロジェクトモジュールをインポートすることは中核シナリオである。したがって Phase
1 ではまず CLI `Run` のソースコードブランチを
`run_project`（orchestrator、ディレクトリ再帰検出）に委譲する。#247（`use`
に沿ったオンデマンド検出）はその後に純粋なパフォーマンス最適化として重ねる。import なしの単一ファイルは orchestrator 経由でも等価な動作であり、バイトコードブランチは変わらない。

**検出フェーズ**：

1. `[PATHS]` が指定された場合、指定されたパスを直接使用する
2. それ以外の場合、`yaoxiang.toml` の `[tool.test].patterns` を読み込む
3. 設定がない場合、デフォルト `tests/**/*.yx`
4. `--filter` フィルタを適用する（ファイル名に含まれる）
5. 検出範囲がそのままテスト階層となる（§9）：デフォルト patterns は言語可用性コーパスのみをカバーする。ライブラリテスト層（例：`src/std/tests/`）は明示的パスまたはパッケージ設定によって検出され、デフォルトスキャンには混入しない

**実行フェーズ**：

1. 各ファイルに対して：`yaoxiang run --debug-info <file>` で子プロセスを起動する（`--debug-info`
   は runtime エラーにソースコード位置を付与する — 2026-08-02 に stack trace が `file:line:col`
   を出力することを実証）。ヘッダー `[test:runtime]` は子プロセスの `--runtime`
   モードを宣言する（2026-09-03 クロージング）
2. ヘッダーが `[test:ignore]: <理由>`
   のファイルは実行をスキップし、レポートの skipped に計上される（2026-09-03 クロージング）
3. exit code を確認：0 は PASS、0 以外は FAIL。`[test:error]`
   ファイルは逆判定し、§8.2 に従い予期されるコードと比較する
4. レポート用に stdout/stderr をキャプチャする
5. 直列実行のみ（Phase 1）、将来自 `--parallel` サポート予定
6. `--fail-fast` の場合、最初の FAIL 発生時点で即座に停止する

### 6. テスト分離

テスト分離はプロセスレベル境界によって自然に実現される：

- 各テストファイルは独立した子プロセスで実行される
- 各子プロセスは独立した Heap、Frame、NativeContext を有する
- あるテストファイルの panic は他のテストファイルに影響しない
- 追加の独立 Heap コンテキスト機構は不要

### 7. スイートと複数テスト（値化モデル）

テストファイルには複数のテストを含め得る。ファイル内構成（2026-09-03 実装済）：

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

main = {
    test.suite([
        ("push_grows_len", () => push_grows_len()),
        ("pop_returns_last", () => pop_returns_last()),
    ])
}
```

- 各テストは `Result(Void, String)` を返すゼロ引数関数である。アサーション失敗は `Err`
  で表現され（§3 値意味論アサーションファミリ）、プロセスを中断しない。後のテストは通常通り実行される
- `test.suite`
  が一つずつ呼び出し収集する：テストが Ok でない場合は名前と診断を記録し、Ok の場合は静かに通過する。すべて実行完了後にいずれかの Err があれば
  `std.assert.assert` で中止し、失敗詳細（`N of M test(s) failed` + 各項目
  `[FAIL] 名前: 診断`）を付与する — ファイル終了コードは 0 以外（§5 判定は不変）。ここでの abort はテストバイナリの runtime ガードであり、アサーション経路ではない。すべて Ok なら静かに 0 で終了する
- トップレベルテスト関数は**クロージャ形式で登録**される（`("name", () => test_fn())`）：トップレベル関数名を値として参照することは現在サポートされていない（IR 層制限、`E3006`） — クロージャ本体からのグローバル関数呼び出しは影響を受けない
- runner はファイルのみを見ず、関数レベルのスキャンを行わない：per-test 判定は完全にスイート内収集に由来し、ファイル内部構造は runner に対して透過的である — コンパイラ変更ゼロの原則に影響しない
- 明示的に不採用とする：プロセス内 catch 境界（17キーワードの鉄則）、runner による関数単位の入口呼び出し（§8.2 コンパイル失敗等の内部シナリオのみ）
- API 形態は決定済（2026-09-03、#319）：`suite(tests: List((String, () -> Result(Void, String)))) -> Void`。重複名は検出しない（名前はレポート表示のみ）。`--filter`
  はファイル名でフィルタリングし、スイート内のテスト名は認識しない

### 8. 負のテスト（予期される失敗）三層設計

負のテストは失敗発生層によって分割され、各層に配置される：

#### 8.1 値レベルの反転（汎用、ユーザ向け）

被測定操作が `Result` を返し、テストは通常のアサーションで予期される失敗を表現する：

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// 或一句封装（码只存在于 std Error 载体上，E 钉死为 Error）：
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code`
  は値意味論ファミリと共に配信済（2026-09-03、§3）。`!assert` 単項形態は `not`
  構文の実装後に提供予定（`assert_false` の `cond == false` と同じ制約）
- エラーコードアサーションは `Error` 値が機械可読な `code` フィールドを保持することに依存する — #323
  M4 で実装済：`Error = { code, message }`（native `error_new(code, message)`）。読み取りは
  `result.unwrap_err(r)` でキャリアを取得し、`result.code(e)` / `result.message(e)`
  アクセサを介する（設計段階で想定されていた `error_new_with_code`
  命名、コード定数のエクスポート、`err.code`
  フィールドアクセスはいずれも採用せず — 言語に Struct フィールドアクセスがなく、コード定数はエクスポートされていないため）
- Result 化の推進（#301、#316）に伴い、失敗する可能性のある操作は順次 `Result`
  を返すようになり、コーパス内のファイルレベル負マーカーはファイル内アサーションに移行する

#### 8.2 ファイルレベル負マーカー（言語設計者の内部使用のみ）

コンパイルは全ファイル全か無かで、ファイル内で「この行はコンパイルされないべき」を表現できない。runtime 失敗も同様にファイルレベル表現が必要である（スイートに必ず失敗するテストが含まれる場合など）。ファイルレベルの特殊マーカーを保持し、runner は予期されるカテゴリに従って**分岐判定**する（2026-09-03 決定、実装待ち）：

- `[test:error]` マーカーは runner によって読み取られる。`予期: コンパイルエラー EXXXX` /
  `予期: ランタイムエラー EXXXX` 行がカテゴリと期待されるコードを宣言する
- **コンパイルエラーカテゴリ**：runner は `yaoxiang check` を実行する — 失敗が必須で、出力に
  `[EXXXX]` を含む必要がある。コンパイル成功 = FAIL、拒否されたがコード不一致 =
  FAIL。構文エラー（E1xxx 解析段）とセマンティックエラー（E2xxx+）は独立カテゴリを設定しない — 期待コード自体が段階を固定する
- **ランタイムエラーカテゴリ**：二段階判定 — `check` は**成功必須**（compile-time には無罪）、`run`
  は失敗必須で出力に `[EXXXX]` を含む必要がある。compile-time に爆発 =
  FAIL（コンパイルエラーカテゴリと逆方向の重要判定で、「コンパイルが想定外に通過し、実行で偶然失敗する」漏検出を防ぐ）
- `予期:` 行のない `[test:error]`
  は exit≠0 判定にフォールバックする（未分類形態、新規コーパスでは非推奨）
- **本リポジトリのコーパスのみを対象とし、ユーザーテストフレームワークの一部ではない**。二重 runner 判定規約はクロージング済（2026-09-03、#319）：yx_runner（cargo
  test）と `yaoxiang test` は `src/util/test_markers.rs`
  を共有してヘッダーマーカー（`[test:error]`/`[test:ignore]`/`[test:runtime]`/`予期: EXXXX`、先頭 16 行）を解析する。06-compile-errors のディレクトリ規約は廃止。分岐判定実装後、レポート層はカテゴリ別カウント（behavior
  / compile-error / runtime-error / skipped）を同時に出力する

#### 8.3 ランタイムハード失敗（Result 化に帰属）

独立機構は設けない — 失敗する可能性のある操作は言語の方向性に従って `Result`
を返し（#301、#316）、テストは §8.1 表現に統一される。プロセスレベル abort（アサーション違反、runtime パラメータ誤りなど）は Result 化に伴い徐々に値に収束し、テストフレームワークは専用のセマンティクスを提供しない。（注意：§8.2 の「ランタイムエラーカテゴリ」マーカー判定は、runner が**まだ Result 化できない操作**に対してファイルレベルで検証するチャネルであり、本節のセマンティクス方向と矛盾しない — 後者は終点、前者は移行期のチャネルである）

### 9. テスト体系の階層化：言語コーパスとライブラリテスト（2026-09-03 決定）

テストは**被測定対象**によって 2 層に分割され、それぞれ帰属とメンテナが異なる。マーカーシステム（§8.2）とアサーションライブラリ（§3）は 2 層共通である：

**第 1 層：言語可用性コーパス（`tests/yaoxiang/`）**

- 被測定対象は**言語自体** — parser、type
  system、モジュール、並行性、ownership、compile-time 拒否、runtime セマンティクス。ディレクトリは言語仕様の章に従って構成される
- std はコーパス内では**アサーションツール**（`std.assert` /
  `std.test`）としてのみ機能し、決して被測定対象とはならない。ライブラリの API 挙動は言語可用性に属さない
- コーパス内は §8 に従い、振る舞いテスト / compile-time 拒否テスト /
  runtime 失敗テストの 3 カテゴリに分岐判定される

**第 2 層：ライブラリテスト（ライブラリと共に）**

- 被測定対象は**ライブラリの公開 API 契約**（例：`list.push` の振る舞い、`result.code`
  のセマンティクス）
- テストは**ライブラリ自身のパッケージ内**に記述される：std のパッケージは `src/std/`
  であり、yx レベルのテストは `src/std/tests/` に属する（実装本体と同じ場所に）。`std.test`
  自身のテストもそこに含まれ（std.test で std.test をテストする、セルフホスティングの閉ループ）
- 将来ユーザーパッケージも同一の慣例に従う：テストはパッケージ内にあり、パッケージの `[tool.test]`
  で検出される（RFC-014 パッケージ管理のテストレイアウトはこれにより予行演習される）
- デフォルト patterns には入らない（デフォルト `tests/**/*.yx`
  は言語層のみをカバー）。ライブラリテスト層は明示的パス（`yaoxiang test src/std/tests`）またはパッケージ設定によって検出される。CI は階層別に実行する

移行注記：現在 `tests/yaoxiang/07-std/` に所在するライブラリテストはこのモデルに従って
`src/std/tests/`
へ移行する。移行時に逐一選別する — 純粋な API 挙動のものは移行し、被測定対象が実際には言語境界であるもの（例：native
`&T` 自動借用）はコーパス層に残し、対応する仕様の章に分類する。

## 既存システムとの関係

| 項目                                                      | 関係                                                                                                      |
| --------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                            | 変更なし、コンパイラ内部テストは引き続き Rust を使用                                                      |
| 既存の `.yx` 統合テスト（`tests/yaoxiang/`）              | `yaoxiang test` で検出・実行される                                                                        |
| `std.assert.assert(cond)`                                 | runtime ガードのために保持。`std.test` 値意味論アサーションファミリは `std.result` ベースに変更（§3、§7） |
| モジュールシステム（RFC-029）                             | 埋め込みソースモジュールは Registry/orchestrator 経由でアクセス。CLI `run` の orchestrator 接続が前提     |
| `#200` リファクタリング（`io.println` → `assert.assert`） | `yaoxiang test` と完全に同じ方向性                                                                        |
| `@` 注釈                                                  | 使用せず、`@test` を導入しない                                                                            |

## 実装戦略

### Phase 1：中核機能

変更範囲：

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` のソースコードブランチを `run_project`
  に委譲（複数ファイル実行の前提）
- `src/main.rs` — `Test` サブコマンドを新規追加
- `src/std/test.yx` — 純YaoXiang モジュールを新規追加
- `build.rs` — `std/*.yx` をバイナリに埋め込み
- orchestrator / Registry — 埋め込みソースから仮想パスで `.yx` モジュールをロードすることをサポート
- RFC-015 設定解析 — `[tool.test]` セクション
- 子プロセス実行（`--debug-info`）+ レポート

成果物：

- `yaoxiang test` が基本的に利用可能
- `std.test` 4 つのアサーション関数
- デフォルト `tests/**/*.yx` 検出
- 直列実行 + デフォルト出力フォーマット

### Phase 2：完成

- `--filter` / `--fail-fast` / `--verbose` オプション
- `--json` 出力（CI 統合）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：拡張

- `--parallel` 並列実行（spawn 並行モデル整備に依存）
- `[tool.test].exclude` 設定
- さらなるアサーション関数（例：Float 用 `assert_approx_eq`）

## リスクと緩和策

| リスク                                                 | 確率 | 緩和策                                                                                                                                                                                                |
| ------------------------------------------------------ | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` の Any 上の補間が失敗                         | なし | 2026-08-02 実証済（Int/String ともに正常）                                                                                                                                                            |
| `yaoxiang.toml` 設定解析が現行 CLI にない              | 低   | 単純な拡張で、中核機能には影響しない                                                                                                                                                                  |
| CLI run の orchestrator 接続による挙動回帰             | 低   | import なしの単一ファイルパスは等価。統合テストで orchestrator をカバー済                                                                                                                             |
| `.yx` ソースファイルのバイナリ埋め込みによるサイズ増加 | 低   | `.yx` ソースファイルは非常に小さく、無視可能                                                                                                                                                          |
| テストループの耗时がコーパス成長に伴い増大             | 高   | 主要因はファイルごとのフルコンパイル（185 ファイル実測 11.3s）で、子プロセス起動ではない。`--parallel` はプロセス側のみを緩和し、コンパイルコストはテストループキャッシュ（#251/#293 スライス）が必要 |

## オープンな問題

- [x] `std/test.yx` 中の `use std.assert` 参照が正しく解決できるか？ —
      **解決済（2026-08-02）**。モジュールシステム（RFC-029）実装後、native とソースモジュールは Registry 内で共存し、resolver が統一的に解決し、種類横断の依存関係が自然に成立する
- [x] テスト出力の `f"..."` のジェネリック `to_string` が新しい type constraint を導入するか？ —
      **解決済（2026-08-02）**。無注釈引数（Any）上で `==`/`!=`
      と f-string 補間がともに動作することを実証（Int/String で検証通過）、新しい type
      constraint は導入されない
- [x] `?` ジェネリック引数の実現可能性？ — **解決済（2026-08-02）**：`?`
      型構文は現在存在せず（かつ静かに飲み込まれる、別途 issue で追跡中）、Phase
      1 のアサーション関数は無注釈引数を使用し、generics システムに依存しない

## 設計決定の記録

| 決定                       | 決定内容                                                                                                                                                                                 | 日付       | 根拠                                                                                                                                                                                      |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| テストマーカー方式         | `@test` 注釈を使用せず、テストファイルは通常の `.yx`                                                                                                                                     | 2026-07-26 | コンパイラ変更ゼロ、子プロセスがそのまま分離                                                                                                                                              |
| アサーション方式           | `std.test` モジュールの純YaoXiang 関数                                                                                                                                                   | 2026-07-26 | セルフホスティング、native コードなし                                                                                                                                                     |
| テスト実行モデル           | 子プロセス `yaoxiang run <file>` + exit code                                                                                                                                             | 2026-07-26 | プロセスレベル分離、コンパイラ変更ゼロ                                                                                                                                                    |
| 標準ライブラリロード       | 現在はバイナリ埋め込み、将来はファイルシステム                                                                                                                                           | 2026-07-26 | バージョン紐付け、単一ファイルで利用可能                                                                                                                                                  |
| アサーション引数型         | 無注釈引数（Any）、generics システムに依存しない                                                                                                                                         | 2026-08-02 | `?` 型構文は存在しない。Any で比較・補間可能と実証                                                                                                                                        |
| 複数ファイル実行           | CLI `run` を `run_project`（orchestrator）に委譲することを前提とする                                                                                                                     | 2026-08-02 | 子プロセスモデルが CLI 能力を継承。#247 は純粋なパフォーマンス最適化に格下げ                                                                                                              |
| レポートのソース位置       | 子プロセスに `--debug-info` を付与                                                                                                                                                       | 2026-08-02 | stack trace が `file:line:col` を出力することを実証。埋め込みモジュール（std.test）経由のフレーム帰属は本保証の対象外で、#289 + RFC-034 に属する                                          |
| 負テストの階層化           | 値レベルの反転は汎用 / コンパイル失敗は runner 構造化マーカー（内部のみ） / ハード失敗は Result 化に帰属                                                                                 | 2026-09-02 | #319 決定。暗黙の [test:error] 規約を置き換え                                                                                                                                             |
| ファイル内複数テスト       | 値化標準モデル：テスト関数は Result を返し、スイートが per-test 判定を収集                                                                                                               | 2026-09-02 | catch なし、入口呼び出しなし（入口は内部シナリオのみ）                                                                                                                                    |
| Error コード               | Error に機械可読な `code` フィールドを追加                                                                                                                                               | 2026-09-02 | エラーコードアサーションを支える。compile-time コードは runner で比較                                                                                                                     |
| アサーションライブラリ形態 | 値意味論ファミリ 7 関数を実装し、`Result(Void, String)` 契約を確立。abort 過渡版を削除                                                                                                   | 2026-09-03 | Void は仕様 unit（`()` は空 Tuple で混用しない）。ネスト位置の Any は剛性、無注釈引数は native ジェネリクスチェックを通過しない — 引数は明示的に注釈を付ける必要がある（R1 プローブ実証） |
| テスト体系の階層化         | 言語コーパス（`tests/yaoxiang/`）とライブラリテスト（ライブラリと共に、std → `src/std/tests/`）の 2 層。std はコーパス内でアサーションツールとしてのみ機能                               | 2026-09-03 | 被測定対象が帰属とメンテナを決定。ライブラリテストのパッケージ随伴レイアウトは RFC-014 の予行演習                                                                                         |
| 負マーカーの分岐判定       | `[test:error]` を予期されるカテゴリで分岐：コンパイルエラーカテゴリは `check` 失敗必須、ランタイムエラーカテゴリは `check` 成功必須 + `run` 失敗必須。レポートにカテゴリ別カウントを出力 | 2026-09-03 | カテゴリ混在判定は「コンパイルが想定外に通過し、実行で偶然失敗する」漏検出を生む。期待コードが段階を固定し、構文エラーは独立カテゴリを設けない                                            |

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
  — 標準ライブラリディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert アサーション機構](../review/030-assert-mechanism.md) — 基盤依存
- [Rust `#[test]` 機構](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計
