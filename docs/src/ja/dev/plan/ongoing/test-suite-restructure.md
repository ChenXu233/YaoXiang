# テストスイート再構築計画

> 状態：計画
> 分岐：refactor/test-suite
> 日付：2026-05-10

## 一、再構築の理由

### 現状の問題

1. **1752個のテストが全て成功したが、実際のバグを検出できなかった**
   - match式が実行時に0を返す（ir_genがMatchノードを処理していない）
   - リスト内包表記が0を返す（ir_genがListCompノードを処理していない）
   - `x: Int = 42` 型注釈付き変数宣言のパースに失敗

2. **統合テストはコンパイル成功のみ検証し、実行時出力の正しさを検証していない**
   - `tests/integration/interpreter.rs`は`assert!(result.is_ok())`のみ実行
   - `tests/integration/execution.rs`は完全にコメントアウトされている

3. **E2E .yxファイルの体系がない**
   - 新旧混在：`closure_test.yx`（旧）と`spec_features_test.yx`（新）が同ディレクトリ
   - 命名規則なし：`closure_test.yx`、`closure_test2.yx`、`mut_param_test.yx`
   - カバー計画なし：言語仕様書の章対応マッピングがない

4. **インラインテストが断片化**
   - `src/frontend/typecheck/tests/`に23ファイルあり、多くのテストが重複
   - scopeテストが4ファイルに分散
   - inferテストが3ファイルに分散
   - `typecheck_fixes.rs`は歴史的パッチの残骸の可能性

5. **Codegenテストが孤立**
   - 全て手書きのIRで、parser→typecheck→ir_genの完全パイプラインを経由しない
   - 「手書きIRがバイトコードに翻訳できるか」をテストしており、「ソースコードのコンパイル結果が正しいか」をテストしていない

### 再構築目標

1. **三層テスト体系の確立** — 各層が明確な責任とカバー基準を持つ
2. **E2Eテストのベンチマーク兼用** — 各.yxテストファイルで実行時間を測定可能
3. **内部テストの正規化** — 統一されたテスト規約、命名、アサーションスタイル
4. **言語仕様書の主要パスのカバー** — 言語仕様書で定義された構文機能が対応するテストを持つことを保証

---

## 二、三層テスト体系

### 第一層：E2E .yxテストスイート（tests/yaoxiang/）

言語仕様書の章별로構成され、各ファイルが1つの構文機能に対応。

```
tests/yaoxiang/
├── 00-smoke/                 # スモークテスト
│   └── hello.yx
│
├── 01-basics/                # 基本構文（仕様書 第2/4/5章）
│   ├── variables.yx          # 変数宣言 + 型推論
│   ├── typed_vars.yx         # 型注釈付き変数 x: Int = 42
│   ├── operators.yx          # 全演算子
│   ├── literals.yx           # 全リテラル
│   └── comments.yx           # コメント
│
├── 02-functions/             # 関数（仕様書 第6章）
│   ├── definitions.yx        # name: (params) -> Ret = ...
│   ├── lambdas.yx            # ラムダ式
│   ├── closures.yx           # 高階関数
│   └── generics.yx           # ジェネリクス関数
│
├── 03-control-flow/          # 制御フロー（仕様書 第4/5章）
│   ├── if_else.yx
│   ├── while.yx
│   ├── for.yx
│   ├── match.yx
│   └── list_comp.yx          # リスト内包表記
│
├── 04-types/                 # 型システム（仕様書 第3章）
│   ├── structs.yx            # Point: Type = { x: Float, y: Float }
│   ├── enums.yx              # Color: Type = { red | green | blue }
│   └── generics.yx           # Option: (T: Type) -> Type = ...
│
├── 05-data-structures/       # コレクション型（仕様書 §2.6）
│   ├── lists.yx
│   ├── tuples.yx
│   └── dicts.yx
│
├── 06-modules/               # モジュールシステム（仕様書 第7章）
│   ├── imports.yx
│   └── lib/
│
└── 07-errors/                # エラー処理（仕様書 第9章、未実装機能はマーク）
    ├── result.yx
    └── option.yx
```

**ファイル規約**：

```yaoxiang
// 01-basics/variables.yx
// カバー: 仕様書 §5.2 変数宣言, §6.2 型推論
// 検証: 基本宣言、型推論、可変性
// 分岐: refactor/test-suite
// 状態: ✅ 実行可能

use std.io

main = {
    x = 42
    io.println(x)
    // expect: 42

    s = "hello"
    io.println(s)
    // expect: hello

    io.println("ALL TESTS PASSED")
}
```

**アサーション機構**：Rustテストフレームワークがstdoutをキャプチャし、各.yxファイルの出力に`ALL TESTS PASSED`文字列が出現することを検証。

**ベンチマーク拡張**：`.yx`テストファイルは自然に応用力ベンチマークとなる—実行時間を測定可能。将来的には`criterion`でラップし、パフォーマンスリグレッションを追跡可能。

### 第二層：統合テスト（tests/integration/）

完全コンパイル+実行パイプラインをテストし、出力値を検証。

| 現行ファイル | 操作 | 説明 |
|---------|------|------|
| `interpreter.rs` | 再構築 | ソースコードコンパイル→実行→出力値アサーションに変更 |
| `execution.rs` | 再構築（コメント解除） | stack overflowを修正し、実際の.yxファイルを実行 |
| `codegen.rs` | 保持 | バイトコードシリアライズ/デシリアライズ |
| `codegen_extended.rs` | 保持 | opcode/メタデータテスト |
| `fstring.rs` | 保持 | 実行検証を補完 |
| `backends.rs` | 保持 | RuntimeValue型テスト |

**補完**：`tests/yx_runner.rs` — `tests/yaoxiang/`下の全.yxファイルを自動発見・実行。

### 第三層：ユニットテスト（src/*/tests/）

単一モジュールの内部ロジックをテストし、プライベートAPIにアクセス可能。

#### 3.1 Lexerテスト（src/frontend/core/lexer/tests/）

11ファイル → デバッグ用1ファイルを削除し、10ファイルを保持。

| 操作 | ファイル | |
|------|------|
| 削除 | `debug_lexer.rs` — デバッグ専用 | |
| 保持 | `basic.rs`, `comments.rs`, `keywords.rs`, `literals.rs`, `operators.rs` | |
| 保持 | `delimiters.rs`, `errors.rs`, `fstring.rs` | |
| 保持 | `rfc004_lexer.rs`, `rfc010_lexer.rs` | |

#### 3.2 Parserテスト（src/frontend/core/parser/tests/）

13ファイル → 審査後に微調整。

| 操作 | ファイル | |
|------|------|
| 保持 | `basic.rs`, `fn_def.rs`, `syntax_validation.rs`, `old_syntax_rejection.rs` | |
| 保持 | `boundary.rs`, `concurrency.rs`, `fstring.rs` | |
| 保持 | `ref_test.rs`, `unsafe_ptr.rs`, `state.rs` | |
| 審査 | `binding_enhancements.rs` — fn_defとの重複を確認 | |

#### 3.3 Typecheckテスト（src/frontend/typecheck/tests/)

**最大問題エリア**：23ファイル → 12ファイルに統合。

| 操作 | 元ファイル | 対象ファイル |
|------|--------|--------|
| 統合 | `infer.rs` + `inference.rs` + `types.rs` | `type_inference.rs` |
| 統合 | `scope.rs` + `shadowing.rs` + `use_scope.rs` + `use_block_scope.rs` | `scoping.rs` |
| 統合 | `visibility.rs` + `pub_bind.rs` | `visibility.rs` |
| 審査 | `typecheck_fixes.rs` | 歴史的パッチテストのみであれば対応ファイルに統合後削除 |
| 保持 | `basic.rs`, `check.rs` | — |
| 保持 | `constraint.rs`, `concurrency.rs`, `fstring.rs` | — |
| 保持 | `gat.rs`, `ref_test.rs`, `result_try.rs` | — |
| 保持 | `semantic_tokens.rs`, `traits.rs`, `type_constructor_rules.rs` | — |

#### 3.4 Middle/Codegenテスト（src/middle/passes/tests/)

| ディレクトリ | 操作 | |
|------|------|
| `codegen/` | 既存を保持、**統合型codegenテストを補完**（ソースからIRまでコンパイル結果の構造検証） | |
| `lifetime/` | 変更なし | |
| `mono/` | 変更なし | |
| `module/` | 変更なし | |

## 三、テスト標準ドキュメント

同ディレクトリに`TEST_STANDARD.md`を作成：

### 命名規則

```
用途        パターン                    例
─────────────────────────────────────────────────────
テストモジュール mod_<説明>_tests          mod_parser_basic_tests
テスト関数    test_<機能>_<シナリオ>      test_parse_fn_def_no_params
E2Eファイル   <章>-<機能>.yx              01-basics-variables.yx
```

### アサーション規則

- E2E `.yx`ファイル：末尾に`ALL TESTS PASSED`を出力
- 統合テスト：stdoutが期待値を含むことを検証
- ユニットテスト：データ構造体のフィールド値を検証し、`assert!(result.is_ok())`を唯一のassertにしない

### コメント規則

```
// E2Eファイルヘッダー：
// カバー: 仕様書 §X.X
// 検証: 一文での説明
// 分岐: refactor/test-suite
// 状態: ✅ 実行可能 / ⚠️ 修正待ち / 🔴 未実装
```

### 未実装機能の処理

- 存在しない機能のE2E `.yx`：書かず、実装後に補完
- ユニットテストで未実装機能を参照：`#[ignore]`でマークし、コメントに「XXX実装後に有効化」と記載

---

## 四、実行計画

### Phase 0：準備作業

- [ ] `dev`から分岐`refactor/test-suite`を作成
- [ ] `typecheck_fixes.rs`と`binding_enhancements.rs`を審査し、削除候補を決定
- [ ] `tests/integration/execution.rs`のstack overflow問題を審査

### Phase 1：E2Eテストフレームワーク

- [ ] `tests/yx_runner.rs`を作成 — `tests/yaoxiang/**/*.yx`を自動発見・実行
- [ ] `tests/yaoxiang/`の新規ディレクトリ構造を作成
- [ ] 00-smokeスモークテストを作成
- [ ] 01-basics層を作成（現在実行可能な構文）
- [ ] 02-functions層を作成

### Phase 2：実行時バグ修正 + 対応テスト

- [ ] match式を修正（ir_genにMatch処理を追加）
- [ ] リスト内包表記を修正（ir_genにListComp処理を追加）
- [ ] `x: Int = 42`変数型注釈を修正
- [ ] 上記修正に対応する.yx E2Eテストを補完

### Phase 3：統合テスト再構築

- [ ] `tests/integration/interpreter.rs`を再構築（実行時出力値を検証）
- [ ] `tests/integration/execution.rs`を再構築（stack overflowを修正）
- [ ] 統合型codegenテストを補完（ソースからIRまで）

### Phase 4：インラインテスト統合

- [ ] typecheckテストを23→12に統合
- [ ] `debug_lexer.rs`を削除
- [ ] parserテストの重複を審査

### Phase 5：テスト標準ドキュメントの作成

- [ ] `tests/yaoxiang/`ルートディレクトリに`TEST_STANDARDS.md`を作成

---

## 五、検証方法

```bash
# 全テスト
cargo test

# E2Eテスト
cargo test --test yx_runner

# ユニットテスト
cargo test --lib

# .yxファイルの手動実行
cargo run -- run tests/yaoxiang/01-basics/variables.yx

# ベンチマーク実行
cargo bench
```

---

## 六、関連ファイル一覧

### 新規作成ファイル
- `tests/yx_runner.rs` — E2Eテストランナー
- `tests/yaoxiang/TEST_STANDARDS.md` — テスト標準
- `tests/yaoxiang/00-smoke/hello.yx`
- `tests/yaoxiang/01-basics/variables.yx`
- `tests/yaoxiang/01-basics/typed_vars.yx`
- `tests/yaoxiang/01-basics/operators.yx`
- `tests/yaoxiang/01-basics/literals.yx`
- `tests/yaoxiang/01-basics/comments.yx`
- `tests/yaoxiang/02-functions/definitions.yx`
- `tests/yaoxiang/02-functions/lambdas.yx`
- `tests/yaoxiang/02-functions/closures.yx`
- `tests/yaoxiang/03-control-flow/if_else.yx`
- `tests/yaoxiang/03-control-flow/while.yx`
- `tests/yaoxiang/03-control-flow/for.yx`
- `tests/yaoxiang/03-control-flow/match.yx`
- `tests/yaoxiang/05-data-structures/lists.yx`
- `tests/yaoxiang/05-data-structures/tuples.yx`
- `tests/yaoxiang/06-modules/imports.yx`
- `tests/yaoxiang/06-modules/lib/math.yx`

### 削除ファイル
- `tests/yaoxiang/closure_test.yx`
- `tests/yaoxiang/closure_test2.yx`
- `tests/yaoxiang/list_test.yx`
- `tests/yaoxiang/mut_param_test.yx`
- `tests/yaoxiang/mut_param_error_test.yx`
- `tests/yaoxiang/impl_status_test.yx`
- `tests/yaoxiang/spec_basics_test.yx`
- `tests/yaoxiang/spec_features_test.yx`
- `tests/yaoxiang/spec_functions_test.yx`
- `tests/yaoxiang/spec_types_test.yx`
- `src/frontend/core/lexer/tests/debug_lexer.rs`（確認待ち）

### 変更ファイル
- `tests/integration/interpreter.rs` — 再構築
- `tests/integration/execution.rs` — 再構築
- `src/frontend/core/ir_gen.rs` — matchとlistcompの修正
- `src/frontend/typecheck/` — `x: Int = 42`の修正
- `src/frontend/typecheck/tests/` — 23→12ファイル統合